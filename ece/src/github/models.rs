// models.rs — Strongly-typed DTOs for GitHub REST API responses.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRepository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: GitHubUser,
    #[serde(default)]
    pub default_branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCommit {
    pub sha: String,
    pub commit: GitHubCommitDetails,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCommitDetails {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub author: Option<GitHubCommitAuthor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCommitAuthor {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubWorkflowRun {
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    pub head_sha: String,
    #[serde(default)]
    pub path: Option<String>,
    pub run_attempt: u32,
    #[serde(default)]
    pub event: Option<String>,
    pub status: String,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub repository: Option<GitHubRunRepo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRunRepo {
    pub id: u64,
    pub full_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubJobsResponse {
    pub total_count: usize,
    pub jobs: Vec<GitHubJob>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubJob {
    pub id: u64,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubArtifactsResponse {
    pub total_count: usize,
    pub artifacts: Vec<GitHubArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubArtifact {
    pub id: u64,
    pub name: String,
    pub size_in_bytes: u64,
    #[serde(default)]
    pub archive_download_url: Option<String>,
    #[serde(default)]
    pub expired: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub digest: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_repository() {
        let json = r#"{
            "id": 1432704795,
            "name": "Evidence-Checkpoint-Engine",
            "full_name": "ErenTahiroglu/Evidence-Checkpoint-Engine",
            "owner": {
                "login": "ErenTahiroglu",
                "id": 149682795
            },
            "default_branch": "main"
        }"#;

        let repo: GitHubRepository = serde_json::from_str(json).expect("valid json");
        assert_eq!(repo.id, 1432704795);
        assert_eq!(repo.name, "Evidence-Checkpoint-Engine");
        assert_eq!(repo.full_name, "ErenTahiroglu/Evidence-Checkpoint-Engine");
        assert_eq!(repo.owner.login, "ErenTahiroglu");
        assert_eq!(repo.default_branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_deserialize_workflow_run() {
        let json = r#"{
            "id": 40969401836,
            "name": "Build and Test",
            "head_sha": "415990746d5bbd3756f743b4b5a7673b36228567",
            "path": ".github/workflows/build-and-test.yml",
            "run_attempt": 1,
            "event": "push",
            "status": "completed",
            "conclusion": "success",
            "repository": {
                "id": 1432704795,
                "full_name": "ErenTahiroglu/Evidence-Checkpoint-Engine"
            }
        }"#;

        let run: GitHubWorkflowRun = serde_json::from_str(json).expect("valid json");
        assert_eq!(run.id, 40969401836);
        assert_eq!(run.run_attempt, 1);
        assert_eq!(run.status, "completed");
        assert_eq!(run.conclusion.as_deref(), Some("success"));
        assert_eq!(run.repository.as_ref().map(|r| r.id), Some(1432704795));
    }

    #[test]
    fn test_deserialize_attempt_jobs() {
        let json = r#"{
            "total_count": 2,
            "jobs": [
                {
                    "id": 123782057785,
                    "name": "Windows x64",
                    "status": "completed",
                    "conclusion": "success",
                    "started_at": "2026-10-29T22:54:56Z",
                    "completed_at": "2026-10-29T22:56:49Z"
                },
                {
                    "id": 123782058145,
                    "name": "Linux x64",
                    "status": "completed",
                    "conclusion": "success",
                    "started_at": "2026-10-29T22:54:56Z",
                    "completed_at": "2026-10-29T22:56:15Z"
                }
            ]
        }"#;

        let resp: GitHubJobsResponse = serde_json::from_str(json).expect("valid json");
        assert_eq!(resp.total_count, 2);
        assert_eq!(resp.jobs.len(), 2);
        assert_eq!(resp.jobs[0].name, "Windows x64");
        assert_eq!(resp.jobs[0].conclusion.as_deref(), Some("success"));
    }

    #[test]
    fn test_deserialize_artifacts() {
        let json = r#"{
            "total_count": 1,
            "artifacts": [
                {
                    "id": 99887766,
                    "name": "ece-release-x86_64",
                    "size_in_bytes": 1048576,
                    "expired": false,
                    "created_at": "2026-10-29T22:56:00Z"
                }
            ]
        }"#;

        let resp: GitHubArtifactsResponse = serde_json::from_str(json).expect("valid json");
        assert_eq!(resp.total_count, 1);
        assert_eq!(resp.artifacts[0].id, 99887766);
        assert_eq!(resp.artifacts[0].name, "ece-release-x86_64");
        assert_eq!(resp.artifacts[0].size_in_bytes, 1048576);
        assert!(!resp.artifacts[0].expired);
    }
}
