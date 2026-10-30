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

        let client = Client::builder()
            .default_headers(default_headers)
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| GitHubClientError::NetworkError(e.to_string()))?;

        // Fall back to GITHUB_TOKEN or GH_TOKEN env if token is None
        let resolved_token = token
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .or_else(|| std::env::var("GH_TOKEN").ok());

        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: resolved_token,
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
