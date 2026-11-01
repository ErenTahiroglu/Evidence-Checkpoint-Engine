// client.rs — Bounded read-only GitHub REST API client and provider trait.

use super::models::{
    GitHubArtifact, GitHubArtifactsResponse, GitHubCommit, GitHubJob, GitHubJobsResponse,
    GitHubRepository, GitHubWorkflowRun,
};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::time::Duration;
use thiserror::Error;

pub const DEFAULT_GITHUB_API_BASE: &str = "https://api.github.com";
pub const GITHUB_API_VERSION: &str = "2022-11-28";
pub const MAX_RESPONSE_BYTES: u64 = 5 * 1024 * 1024; // 5 MiB
pub const MAX_ARTIFACT_DOWNLOAD_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum GitHubClientError {
    #[error("GitHub API rate limit exceeded (reset at {reset_at:?}): {message}")]
    RateLimited {
        reset_at: Option<u64>,
        message: String,
    },

    #[error("GitHub resource not found: {resource}")]
    NotFound { resource: String },

    #[error("GitHub API unauthorized or forbidden: {0}")]
    Unauthorized(String),

    #[error("GitHub API HTTP error (status {status}): {message}")]
    HttpError {
        status: u16,
        message: String,
        rate_limit_reset: Option<u64>,
    },

    #[error("HTTP request timed out: {0}")]
    Timeout(String),

    #[error("Network connection error: {0}")]
    NetworkError(String),

    #[error("Invalid JSON response from GitHub API: {0}")]
    InvalidJson(String),

    #[error("Response body exceeded size limit of {0} bytes")]
    SizeLimitExceeded(usize),

    #[error("Empty response from GitHub API")]
    EmptyResponse,

    #[error(
        "Incomplete pagination: retrieved {retrieved} items, \
         expected {total_count} (page limit {page_limit})"
    )]
    IncompletePagination {
        retrieved: usize,
        total_count: usize,
        page_limit: u32,
    },
}

pub trait GitHubProvider: Send + Sync {
    fn get_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<GitHubRepository, GitHubClientError>;
    fn get_commit(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> Result<GitHubCommit, GitHubClientError>;
    fn get_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<GitHubWorkflowRun, GitHubClientError>;
    fn get_attempt_jobs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        attempt: u32,
    ) -> Result<Vec<GitHubJob>, GitHubClientError>;
    fn get_artifacts(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<Vec<GitHubArtifact>, GitHubClientError>;
    fn download_artifact_digest(
        &self,
        owner: &str,
        repo: &str,
        artifact_id: u64,
        max_bytes: usize,
    ) -> Result<String, GitHubClientError>;
}

pub struct HttpGitHubClient {
    base_url: String,
    token: Option<String>,
    client: Client,
}

impl HttpGitHubClient {
    pub fn new(token: Option<String>) -> Result<Self, GitHubClientError> {
        Self::new_with_base_url(DEFAULT_GITHUB_API_BASE, token)
    }

    pub fn new_with_base_url(
        base_url: impl Into<String>,
        token: Option<String>,
    ) -> Result<Self, GitHubClientError> {
        // Fall back to GITHUB_TOKEN or GH_TOKEN env if token is None
        let resolved_token = token
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .or_else(|| std::env::var("GH_TOKEN").ok());
        Self::build_client(base_url, resolved_token, false)
    }

    /// Explicit unauthenticated constructor.
    ///
    /// Guarantees:
    /// - Never reads GITHUB_TOKEN or GH_TOKEN from environment.
    /// - Never attaches an Authorization header.
    /// - Uses the fixed GitHub API base URL (https://api.github.com).
    /// - Prevents automatic redirection outside the explicitly approved GitHub API host.
    /// - Preserves standard timeouts, headers, and response size limits.
    pub fn new_unauthenticated() -> Result<Self, GitHubClientError> {
        Self::new_unauthenticated_with_base_url(DEFAULT_GITHUB_API_BASE)
    }

    /// Explicit unauthenticated constructor with custom base URL (for testing).
    pub fn new_unauthenticated_with_base_url(
        base_url: impl Into<String>,
    ) -> Result<Self, GitHubClientError> {
        Self::build_client(base_url, None, true)
    }

    fn build_client(
        base_url: impl Into<String>,
        token: Option<String>,
        is_unauthenticated: bool,
    ) -> Result<Self, GitHubClientError> {
        let base_url_str = base_url.into().trim_end_matches('/').to_string();

        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Evidence-Checkpoint-Engine-GOV-02"),
        );
        default_headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static(GITHUB_API_VERSION),
        );
        default_headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );

        let mut builder = Client::builder()
            .default_headers(default_headers)
            .timeout(Duration::from_secs(15));

        if is_unauthenticated {
            let parsed_base = reqwest::Url::parse(&base_url_str)
                .map_err(|e| GitHubClientError::NetworkError(format!("Invalid base URL: {e}")))?;
            let approved_host = parsed_base.host_str().unwrap_or("").to_string();
            let approved_port = parsed_base.port_or_known_default();

            let redirect_policy = reqwest::redirect::Policy::custom(move |attempt| {
                let target_url = attempt.url();
                let target_host = target_url.host_str().unwrap_or("");
                let target_port = target_url.port_or_known_default();

                if target_host == approved_host && target_port == approved_port {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            });
            builder = builder.redirect(redirect_policy);
        }

        let client = builder
            .build()
            .map_err(|e| GitHubClientError::NetworkError(e.to_string()))?;

        Ok(Self {
            base_url: base_url_str,
            token,
            client,
        })
    }

    fn send_get(&self, url: &str) -> Result<reqwest::blocking::Response, GitHubClientError> {
        let mut req = self.client.get(url);
        if let Some(ref tok) = self.token {
            req = req.bearer_auth(tok);
        }

        let resp = req.send().map_err(|e| {
            if e.is_timeout() {
                GitHubClientError::Timeout(e.to_string())
            } else {
                GitHubClientError::NetworkError(e.to_string())
            }
        })?;

        let status = resp.status().as_u16();

        // Extract rate limit reset header if present
        let rate_limit_reset = resp
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let rate_limit_remaining = resp
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        if status == 429 || (status == 403 && rate_limit_remaining == Some(0)) {
            return Err(GitHubClientError::RateLimited {
                reset_at: rate_limit_reset,
                message: format!("GitHub API rate limit exceeded (HTTP {status})"),
            });
        }

        if status == 404 {
            return Err(GitHubClientError::NotFound {
                resource: url.to_string(),
            });
        }

        if status == 401 || status == 403 {
            return Err(GitHubClientError::Unauthorized(format!(
                "HTTP {status} authorization error"
            )));
        }

        if (300..=399).contains(&status) {
            return Err(GitHubClientError::NetworkError(format!(
                "HTTP redirect to {} outside approved host refused",
                resp.headers()
                    .get("location")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("unknown")
            )));
        }

        if status >= 400 {
            return Err(GitHubClientError::HttpError {
                status,
                message: format!("HTTP error from GitHub API: {status}"),
                rate_limit_reset,
            });
        }

        Ok(resp)
    }

    fn read_bounded_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, GitHubClientError> {
        let resp = self.send_get(url)?;
        let mut buffer = Vec::new();
        let mut reader = resp.take(MAX_RESPONSE_BYTES + 1);

        reader
            .read_to_end(&mut buffer)
            .map_err(|e| GitHubClientError::NetworkError(e.to_string()))?;

        if buffer.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(GitHubClientError::SizeLimitExceeded(
                MAX_RESPONSE_BYTES as usize,
            ));
        }

        if buffer.is_empty() {
            return Err(GitHubClientError::EmptyResponse);
        }

        serde_json::from_slice(&buffer).map_err(|e| GitHubClientError::InvalidJson(e.to_string()))
    }
}

impl GitHubProvider for HttpGitHubClient {
    fn get_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<GitHubRepository, GitHubClientError> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        self.read_bounded_json(&url)
    }

    fn get_commit(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> Result<GitHubCommit, GitHubClientError> {
        let url = format!(
            "{}/repos/{}/{}/commits/{}",
            self.base_url, owner, repo, commit_sha
        );
        self.read_bounded_json(&url)
    }

    fn get_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<GitHubWorkflowRun, GitHubClientError> {
        let url = format!(
            "{}/repos/{}/{}/actions/runs/{}",
            self.base_url, owner, repo, run_id
        );
        self.read_bounded_json(&url)
    }

    fn get_attempt_jobs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        attempt: u32,
    ) -> Result<Vec<GitHubJob>, GitHubClientError> {
        let mut all_jobs = Vec::new();
        let mut page = 1;
        let page_limit = 10;

        let total_count = loop {
            let url = format!(
                "{}/repos/{}/{}/actions/runs/{}/attempts/{}/jobs?per_page=100&page={}",
                self.base_url, owner, repo, run_id, attempt, page
            );

            let resp: GitHubJobsResponse = self.read_bounded_json(&url)?;
            let total = resp.total_count;
            let count_on_page = resp.jobs.len();

            all_jobs.extend(resp.jobs);

            if all_jobs.len() >= total {
                break total;
            }

            if count_on_page == 0 || page >= page_limit {
                return Err(GitHubClientError::IncompletePagination {
                    retrieved: all_jobs.len(),
                    total_count: total,
                    page_limit,
                });
            }

            page += 1;
        };

        if all_jobs.len() != total_count {
            return Err(GitHubClientError::IncompletePagination {
                retrieved: all_jobs.len(),
                total_count,
                page_limit,
            });
        }

        Ok(all_jobs)
    }

    fn get_artifacts(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<Vec<GitHubArtifact>, GitHubClientError> {
        let mut all_artifacts = Vec::new();
        let mut page = 1;
        let page_limit = 10;

        let total_count = loop {
            let url = format!(
                "{}/repos/{}/{}/actions/runs/{}/artifacts?per_page=100&page={}",
                self.base_url, owner, repo, run_id, page
            );

            let resp: GitHubArtifactsResponse = self.read_bounded_json(&url)?;
            let total = resp.total_count;
            let count_on_page = resp.artifacts.len();

            all_artifacts.extend(resp.artifacts);

            if all_artifacts.len() >= total {
                break total;
            }

            if count_on_page == 0 || page >= page_limit {
                return Err(GitHubClientError::IncompletePagination {
                    retrieved: all_artifacts.len(),
                    total_count: total,
                    page_limit,
                });
            }

            page += 1;
        };

        if all_artifacts.len() != total_count {
            return Err(GitHubClientError::IncompletePagination {
                retrieved: all_artifacts.len(),
                total_count,
                page_limit,
            });
        }

        Ok(all_artifacts)
    }

    fn download_artifact_digest(
        &self,
        owner: &str,
        repo: &str,
        artifact_id: u64,
        max_bytes: usize,
    ) -> Result<String, GitHubClientError> {
        let limit = (max_bytes as u64).min(MAX_ARTIFACT_DOWNLOAD_BYTES);
        let url = format!(
            "{}/repos/{}/{}/actions/artifacts/{}/zip",
            self.base_url, owner, repo, artifact_id
        );

        let resp = self.send_get(&url)?;
        let mut hasher = Sha256::new();
        let mut reader = resp.take(limit + 1);
        let mut buffer = [0u8; 8192];
        let mut total_read = 0;

        loop {
            let n = reader
                .read(&mut buffer)
                .map_err(|e| GitHubClientError::NetworkError(e.to_string()))?;
            if n == 0 {
                break;
            }
            total_read += n;
            if total_read as u64 > limit {
                return Err(GitHubClientError::SizeLimitExceeded(limit as usize));
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hex::encode(hasher.finalize()))
    }
}

/// In-memory mock GitHub provider for offline testing and adversarial simulation.
#[derive(Default)]
pub struct MockGitHubProvider {
    pub repo_result: Option<Result<GitHubRepository, GitHubClientError>>,
    pub commit_result: Option<Result<GitHubCommit, GitHubClientError>>,
    pub run_result: Option<Result<GitHubWorkflowRun, GitHubClientError>>,
    pub jobs_result: Option<Result<Vec<GitHubJob>, GitHubClientError>>,
    pub artifacts_result: Option<Result<Vec<GitHubArtifact>, GitHubClientError>>,
    pub download_digest_result: Option<Result<String, GitHubClientError>>,
}

impl MockGitHubProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl GitHubProvider for MockGitHubProvider {
    fn get_repository(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<GitHubRepository, GitHubClientError> {
        self.repo_result.clone().unwrap_or_else(|| {
            Err(GitHubClientError::NotFound {
                resource: "repo".into(),
            })
        })
    }

    fn get_commit(
        &self,
        _owner: &str,
        _repo: &str,
        _commit_sha: &str,
    ) -> Result<GitHubCommit, GitHubClientError> {
        self.commit_result.clone().unwrap_or_else(|| {
            Err(GitHubClientError::NotFound {
                resource: "commit".into(),
            })
        })
    }

    fn get_workflow_run(
        &self,
        _owner: &str,
        _repo: &str,
        _run_id: u64,
    ) -> Result<GitHubWorkflowRun, GitHubClientError> {
        self.run_result.clone().unwrap_or_else(|| {
            Err(GitHubClientError::NotFound {
                resource: "run".into(),
            })
        })
    }

    fn get_attempt_jobs(
        &self,
        _owner: &str,
        _repo: &str,
        _run_id: u64,
        _attempt: u32,
    ) -> Result<Vec<GitHubJob>, GitHubClientError> {
        self.jobs_result.clone().unwrap_or_else(|| {
            Err(GitHubClientError::NotFound {
                resource: "jobs".into(),
            })
        })
    }

    fn get_artifacts(
        &self,
        _owner: &str,
        _repo: &str,
        _run_id: u64,
    ) -> Result<Vec<GitHubArtifact>, GitHubClientError> {
        self.artifacts_result
            .clone()
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    fn download_artifact_digest(
        &self,
        _owner: &str,
        _repo: &str,
        _artifact_id: u64,
        _max_bytes: usize,
    ) -> Result<String, GitHubClientError> {
        self.download_digest_result.clone().unwrap_or_else(|| {
            Err(GitHubClientError::NotFound {
                resource: "artifact_zip".into(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;

    fn run_mock_server<F>(handler: F) -> (String, thread::JoinHandle<()>)
    where
        F: FnOnce(TcpStream) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let addr = listener.local_addr().expect("local addr");
        let url = format!("http://{}", addr);
        let handle = thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                handler(stream);
            }
        });
        (url, handle)
    }

    fn read_request_lines(stream: &mut TcpStream) -> (String, Vec<String>) {
        let mut reader = BufReader::new(stream);
        let mut req_line = String::new();
        reader.read_line(&mut req_line).expect("read request line");
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read header line");
            if line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }
            headers.push(line.trim().to_string());
        }
        (req_line, headers)
    }

    #[test]
    fn test_anonymous_client_sends_no_auth_header_despite_ambient_env() {
        std::env::set_var("GITHUB_TOKEN", "test-ambient-token-abc");
        std::env::set_var("GH_TOKEN", "test-ambient-gh-token-xyz");

        let auth_header_seen = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&auth_header_seen);

        let (server_url, server_handle) = run_mock_server(move |mut stream| {
            let (_req_line, headers) = read_request_lines(&mut stream);
            for h in headers {
                let lower = h.to_ascii_lowercase();
                if lower.starts_with("authorization:") {
                    flag.store(true, Ordering::SeqCst);
                }
            }
            let body = r#"{"id":1413039335,"name":"AI-Toolchain-Manager","full_name":"ErenTahiroglu/AI-Toolchain-Manager","owner":{"login":"ErenTahiroglu","id":149682795}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.github+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).expect("write response");
            stream.flush().expect("flush response");
        });

        let client = HttpGitHubClient::new_unauthenticated_with_base_url(&server_url)
            .expect("create unauthenticated client");
        let repo = client
            .get_repository("ErenTahiroglu", "AI-Toolchain-Manager")
            .expect("fetch repo");

        server_handle.join().expect("join server");

        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("GH_TOKEN");

        assert_eq!(repo.id, 1413039335);
        assert_eq!(repo.name, "AI-Toolchain-Manager");
        assert!(
            !auth_header_seen.load(Ordering::SeqCst),
            "Unauthenticated client MUST NOT send an Authorization header"
        );
    }

    #[test]
    fn test_authenticated_client_preserves_token() {
        let auth_header_seen = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&auth_header_seen);

        let (server_url, server_handle) = run_mock_server(move |mut stream| {
            let (_req_line, headers) = read_request_lines(&mut stream);
            for h in headers {
                let lower = h.to_ascii_lowercase();
                if lower.starts_with("authorization: bearer explicit-token") {
                    flag.store(true, Ordering::SeqCst);
                }
            }
            let body = r#"{"id":1413039335,"name":"AI-Toolchain-Manager","full_name":"ErenTahiroglu/AI-Toolchain-Manager","owner":{"login":"ErenTahiroglu","id":149682795}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.github+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).expect("write response");
            stream.flush().expect("flush response");
        });

        let client = HttpGitHubClient::new_with_base_url(
            &server_url,
            Some("explicit-token".to_string()),
        )
        .expect("create authenticated client");
        let repo = client
            .get_repository("ErenTahiroglu", "AI-Toolchain-Manager")
            .expect("fetch repo");

        server_handle.join().expect("join server");

        assert_eq!(repo.id, 1413039335);
        assert!(
            auth_header_seen.load(Ordering::SeqCst),
            "Authenticated client MUST send Authorization header with the explicit token"
        );
    }

    #[test]
    fn test_rest_api_parsing_commit_and_workflow_run() {
        let (server_url, server_handle) = run_mock_server(|mut stream| {
            let (req_line, _) = read_request_lines(&mut stream);
            let body = if req_line.contains("/commits/") {
                r#"{"sha":"6c324d04b6e6837e3833ea53ce3f411a5b0ce289","commit":{"message":"test msg"}}"#
            } else if req_line.contains("/actions/runs/") {
                r#"{"id":41195527653,"head_sha":"6c324d04b6e6837e3833ea53ce3f411a5b0ce289","run_attempt":1,"status":"completed","conclusion":"success"}"#
            } else {
                r#"{}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.github+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).expect("write response");
            stream.flush().expect("flush response");
        });

        let client = HttpGitHubClient::new_unauthenticated_with_base_url(&server_url)
            .expect("create unauthenticated client");
        let commit = client
            .get_commit(
                "ErenTahiroglu",
                "AI-Toolchain-Manager",
                "6c324d04b6e6837e3833ea53ce3f411a5b0ce289",
            )
            .expect("fetch commit");

        server_handle.join().expect("join server");
        assert_eq!(commit.sha, "6c324d04b6e6837e3833ea53ce3f411a5b0ce289");
    }

    #[test]
    fn test_network_errors_and_unauthorized_remain_fail_closed() {
        // Test 1: connection refused on unreachable port
        let client_bad_port =
            HttpGitHubClient::new_unauthenticated_with_base_url("http://127.0.0.1:1")
                .expect("create client with bad port");
        let err = client_bad_port
            .get_repository("ErenTahiroglu", "AI-Toolchain-Manager")
            .unwrap_err();
        assert!(matches!(err, GitHubClientError::NetworkError(_)));

        // Test 2: 404 Not Found
        let (server_url_404, server_handle_404) = run_mock_server(|mut stream| {
            let _ = read_request_lines(&mut stream);
            let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        let client_404 = HttpGitHubClient::new_unauthenticated_with_base_url(&server_url_404)
            .expect("client 404");
        let err_404 = client_404
            .get_repository("ErenTahiroglu", "AI-Toolchain-Manager")
            .unwrap_err();
        server_handle_404.join().expect("join 404 server");
        assert!(matches!(err_404, GitHubClientError::NotFound { .. }));

        // Test 3: 401 Unauthorized
        let (server_url_401, server_handle_401) = run_mock_server(|mut stream| {
            let _ = read_request_lines(&mut stream);
            let response = "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        let client_401 = HttpGitHubClient::new_unauthenticated_with_base_url(&server_url_401)
            .expect("client 401");
        let err_401 = client_401
            .get_repository("ErenTahiroglu", "AI-Toolchain-Manager")
            .unwrap_err();
        server_handle_401.join().expect("join 401 server");
        assert!(matches!(err_401, GitHubClientError::Unauthorized(_)));
    }

    #[test]
    fn test_unsupported_cross_host_redirect_is_refused() {
        let (server_url, server_handle) = run_mock_server(|mut stream| {
            let _ = read_request_lines(&mut stream);
            let response = "HTTP/1.1 302 Found\r\nLocation: http://evil.example.com/escaped\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).expect("write redirect");
            stream.flush().expect("flush redirect");
        });

        let client = HttpGitHubClient::new_unauthenticated_with_base_url(&server_url)
            .expect("create unauthenticated client");
        let err = client
            .get_repository("ErenTahiroglu", "AI-Toolchain-Manager")
            .unwrap_err();

        server_handle.join().expect("join server");

        match err {
            GitHubClientError::NetworkError(msg) => {
                assert!(
                    msg.contains("redirect") || msg.contains("refused"),
                    "Expected redirect refusal message, got: {msg}"
                );
            }
            other => panic!("Expected NetworkError on cross-host redirect, got: {other:?}"),
        }
    }
}
