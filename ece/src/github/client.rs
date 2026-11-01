// client.rs — Bounded read-only GitHub REST API client and provider trait.

use super::models::{GitHubArtifact, GitHubCommit, GitHubJob, GitHubRepository, GitHubWorkflowRun};
use thiserror::Error;

// The live HTTP implementation lives in `http_client` and is compiled only with the `github-http` feature.
// Its public items keep their historical paths (`client::HttpGitHubClient`, `client::MAX_RESPONSE_BYTES`, ...).
#[cfg(feature = "github-http")]
pub use super::http_client::{
    HttpGitHubClient, DEFAULT_GITHUB_API_BASE, GITHUB_API_VERSION, MAX_ARTIFACT_DOWNLOAD_BYTES,
    MAX_RESPONSE_BYTES,
};

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
