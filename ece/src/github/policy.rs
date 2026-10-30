// policy.rs — Independently trusted check policy and job evaluation.

use super::models::GitHubJob;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedCheckPolicy {
    pub policy_id: String,
    pub policy_digest: String,
    pub required_jobs: Vec<String>,
    #[serde(default = "default_true")]
    pub allow_extra_jobs: bool,
    #[serde(default)]
    pub target_repo_id: Option<String>,
    #[serde(default)]
    pub target_workflow_path: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyStatus {
    VerifiedPass,
    Failed,
    IncompleteSkipped,
    MissingJobs,
    InProgress,
    AmbiguousDuplicated,
    UntrustedPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckPolicyEvaluation {
    pub status: PolicyStatus,
    pub expected_jobs_count: usize,
    pub observed_jobs_count: usize,
    pub passed_jobs: Vec<String>,
    pub failed_jobs: Vec<String>,
    pub skipped_or_cancelled_jobs: Vec<String>,
    pub missing_jobs: Vec<String>,
    pub in_progress_jobs: Vec<String>,
    pub conflicting_jobs: Vec<String>,
    pub details: Vec<String>,
}

impl TrustedCheckPolicy {
    pub fn new(
        policy_id: impl Into<String>,
        required_jobs: Vec<String>,
        target_repo_id: Option<String>,
        target_workflow_path: Option<String>,
    ) -> Self {
        let policy_id = policy_id.into();
        let digest_input = format!(
            "{}:{:?}:{:?}:{:?}",
            policy_id, required_jobs, target_repo_id, target_workflow_path
        );
        let hash = Sha256::digest(digest_input.as_bytes());
        let policy_digest = hex::encode(hash);

        Self {
            policy_id,
            policy_digest,
            required_jobs,
            allow_extra_jobs: true,
            target_repo_id,
            target_workflow_path,
        }
    }

    /// Pilot policy enforcing the four native CI runner jobs.
    pub fn ece_pilot_policy() -> Self {
        Self::new(
            "ece-pilot-four-runner-matrix",
            vec![
                "Linux x64".to_string(),
                "macOS ARM64".to_string(),
                "Windows x64".to_string(),
                "macOS Intel x64".to_string(),
            ],
            Some("1432704795".to_string()),
            Some(".github/workflows/build-and-test.yml".to_string()),
        )
    }

    pub fn validate_target_scope(
        &self,
        repo_id: &str,
        workflow_path: Option<&str>,
    ) -> Result<(), String> {
        if let Some(ref expected_repo) = self.target_repo_id {
            if expected_repo != repo_id {
                return Err(format!(
                    "Policy target repo mismatch: expected repo ID '{}', got '{}'",
                    expected_repo, repo_id
                ));
            }
        }
        if let Some(ref expected_wf) = self.target_workflow_path {
            if let Some(actual_wf) = workflow_path {
                if expected_wf != actual_wf {
                    return Err(format!(
                        "Policy target workflow mismatch: expected '{}', got '{}'",
                        expected_wf, actual_wf
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn evaluate_jobs(&self, observed_jobs: &[GitHubJob]) -> CheckPolicyEvaluation {
        if self.required_jobs.is_empty() {
            return CheckPolicyEvaluation {
                status: PolicyStatus::UntrustedPolicy,
                expected_jobs_count: 0,
                observed_jobs_count: observed_jobs.len(),
                passed_jobs: Vec::new(),
                failed_jobs: Vec::new(),
                skipped_or_cancelled_jobs: Vec::new(),
                missing_jobs: Vec::new(),
                in_progress_jobs: Vec::new(),
                conflicting_jobs: Vec::new(),
                details: vec!["Trusted check policy defines zero required jobs".to_string()],
            };
        }

        let mut seen = std::collections::HashSet::new();
        for req in &self.required_jobs {
            if !seen.insert(req) {
                return CheckPolicyEvaluation {
                    status: PolicyStatus::UntrustedPolicy,
                    expected_jobs_count: self.required_jobs.len(),
                    observed_jobs_count: observed_jobs.len(),
                    passed_jobs: Vec::new(),
                    failed_jobs: Vec::new(),
                    skipped_or_cancelled_jobs: Vec::new(),
                    missing_jobs: Vec::new(),
                    in_progress_jobs: Vec::new(),
                    conflicting_jobs: vec![req.clone()],
                    details: vec![format!(
                        "Trusted check policy contains duplicate required job '{req}'"
                    )],
                };
            }
        }

        let mut passed_jobs = Vec::new();
        let mut failed_jobs = Vec::new();
        let mut skipped_or_cancelled_jobs = Vec::new();
        let mut missing_jobs = Vec::new();
        let mut in_progress_jobs = Vec::new();
        let mut conflicting_jobs = Vec::new();
        let mut details = Vec::new();

        for req in &self.required_jobs {
            let matches: Vec<&GitHubJob> =
                observed_jobs.iter().filter(|j| &j.name == req).collect();

            if matches.is_empty() {
                missing_jobs.push(req.clone());
                details.push(format!(
                    "Required job '{req}' was NOT observed in run attempt"
                ));
                continue;
            }

            // Reject ambiguous duplicate executions in attempt
            if matches.len() > 1 {
                conflicting_jobs.push(req.clone());
                details.push(format!(
                    "Job '{req}' has ambiguous duplicate executions in attempt \
                     (observed {} instances)",
                    matches.len()
                ));
                continue;
            }

            let job = matches[0];
            if job.status != "completed" {
                in_progress_jobs.push(req.clone());
                details.push(format!(
                    "Job '{req}' is not completed (status='{}')",
                    job.status
                ));
            } else {
                match job.conclusion.as_deref() {
                    Some("success") => {
                        passed_jobs.push(req.clone());
                    }
                    Some("failure") | Some("timed_out") => {
                        failed_jobs.push(req.clone());
                        details.push(format!(
                            "Job '{req}' failed with conclusion '{:?}'",
                            job.conclusion
                        ));
                    }
                    Some("cancelled") | Some("skipped") | Some("action_required") => {
                        skipped_or_cancelled_jobs.push(req.clone());
                        details.push(format!(
                            "Job '{req}' was incomplete (conclusion='{:?}')",
                            job.conclusion
                        ));
                    }
                    other => {
                        failed_jobs.push(req.clone());
                        details.push(format!(
                            "Job '{req}' had unexpected conclusion '{:?}'",
                            other
                        ));
                    }
                }
            }
        }

        let status = if !conflicting_jobs.is_empty() {
            PolicyStatus::AmbiguousDuplicated
        } else if !failed_jobs.is_empty() {
            PolicyStatus::Failed
        } else if !missing_jobs.is_empty() {
            PolicyStatus::MissingJobs
        } else if !skipped_or_cancelled_jobs.is_empty() {
            PolicyStatus::IncompleteSkipped
        } else if !in_progress_jobs.is_empty() {
            PolicyStatus::InProgress
        } else if passed_jobs.len() == self.required_jobs.len() {
            PolicyStatus::VerifiedPass
        } else {
            PolicyStatus::Failed
        };

        CheckPolicyEvaluation {
            status,
            expected_jobs_count: self.required_jobs.len(),
            observed_jobs_count: observed_jobs.len(),
            passed_jobs,
            failed_jobs,
            skipped_or_cancelled_jobs,
            missing_jobs,
            in_progress_jobs,
            conflicting_jobs,
            details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_job(name: &str, status: &str, conclusion: Option<&str>) -> GitHubJob {
        GitHubJob {
            id: 1,
            name: name.to_string(),
            status: status.to_string(),
            conclusion: conclusion.map(|s| s.to_string()),
            started_at: None,
            completed_at: None,
        }
    }

    #[test]
    fn test_policy_all_pass() {
        let policy = TrustedCheckPolicy::ece_pilot_policy();
        let jobs = vec![
            make_job("Linux x64", "completed", Some("success")),
            make_job("macOS ARM64", "completed", Some("success")),
            make_job("Windows x64", "completed", Some("success")),
            make_job("macOS Intel x64", "completed", Some("success")),
        ];

        let eval = policy.evaluate_jobs(&jobs);
        assert_eq!(eval.status, PolicyStatus::VerifiedPass);
        assert_eq!(eval.passed_jobs.len(), 4);
        assert_eq!(eval.missing_jobs.len(), 0);
    }

    #[test]
    fn test_policy_missing_job() {
        let policy = TrustedCheckPolicy::ece_pilot_policy();
        let jobs = vec![
            make_job("Linux x64", "completed", Some("success")),
            make_job("macOS ARM64", "completed", Some("success")),
        ];

        let eval = policy.evaluate_jobs(&jobs);
        assert_eq!(eval.status, PolicyStatus::MissingJobs);
        assert_eq!(eval.missing_jobs.len(), 2);
    }

    #[test]
    fn test_policy_failed_job() {
        let policy = TrustedCheckPolicy::ece_pilot_policy();
        let jobs = vec![
            make_job("Linux x64", "completed", Some("success")),
            make_job("macOS ARM64", "completed", Some("success")),
            make_job("Windows x64", "completed", Some("failure")),
            make_job("macOS Intel x64", "completed", Some("success")),
        ];

        let eval = policy.evaluate_jobs(&jobs);
        assert_eq!(eval.status, PolicyStatus::Failed);
        assert_eq!(eval.failed_jobs, vec!["Windows x64"]);
    }

    #[test]
    fn test_policy_skipped_job() {
        let policy = TrustedCheckPolicy::ece_pilot_policy();
        let jobs = vec![
            make_job("Linux x64", "completed", Some("success")),
            make_job("macOS ARM64", "completed", Some("skipped")),
            make_job("Windows x64", "completed", Some("success")),
            make_job("macOS Intel x64", "completed", Some("success")),
        ];

        let eval = policy.evaluate_jobs(&jobs);
        assert_eq!(eval.status, PolicyStatus::IncompleteSkipped);
        assert_eq!(eval.skipped_or_cancelled_jobs, vec!["macOS ARM64"]);
    }

    #[test]
    fn test_policy_in_progress_job() {
        let policy = TrustedCheckPolicy::ece_pilot_policy();
        let jobs = vec![
            make_job("Linux x64", "completed", Some("success")),
            make_job("macOS ARM64", "in_progress", None),
            make_job("Windows x64", "completed", Some("success")),
            make_job("macOS Intel x64", "completed", Some("success")),
        ];

        let eval = policy.evaluate_jobs(&jobs);
        assert_eq!(eval.status, PolicyStatus::InProgress);
        assert_eq!(eval.in_progress_jobs, vec!["macOS ARM64"]);
    }

    #[test]
    fn test_policy_conflicting_duplicates() {
        let policy = TrustedCheckPolicy::ece_pilot_policy();
        let jobs = vec![
            make_job("Linux x64", "completed", Some("success")),
            make_job("Linux x64", "completed", Some("failure")),
            make_job("macOS ARM64", "completed", Some("success")),
            make_job("Windows x64", "completed", Some("success")),
            make_job("macOS Intel x64", "completed", Some("success")),
        ];

        let eval = policy.evaluate_jobs(&jobs);
        assert_eq!(eval.status, PolicyStatus::AmbiguousDuplicated);
        assert_eq!(eval.conflicting_jobs, vec!["Linux x64"]);
    }

    #[test]
    fn test_policy_identical_duplicates() {
        let policy = TrustedCheckPolicy::ece_pilot_policy();
        let jobs = vec![
            make_job("Linux x64", "completed", Some("success")),
            make_job("Linux x64", "completed", Some("success")),
            make_job("macOS ARM64", "completed", Some("success")),
            make_job("Windows x64", "completed", Some("success")),
            make_job("macOS Intel x64", "completed", Some("success")),
        ];

        let eval = policy.evaluate_jobs(&jobs);
        assert_eq!(eval.status, PolicyStatus::AmbiguousDuplicated);
        assert_eq!(eval.conflicting_jobs, vec!["Linux x64"]);
    }

    #[test]
    fn test_policy_duplicate_required_in_definition() {
        let policy = TrustedCheckPolicy::new(
            "bad-policy",
            vec!["Linux x64".to_string(), "Linux x64".to_string()],
            None,
            None,
        );
        let jobs = vec![make_job("Linux x64", "completed", Some("success"))];

        let eval = policy.evaluate_jobs(&jobs);
        assert_eq!(eval.status, PolicyStatus::UntrustedPolicy);
        assert_eq!(eval.conflicting_jobs, vec!["Linux x64"]);
    }

    #[test]
    fn test_policy_target_scope_validation() {
        let policy = TrustedCheckPolicy::ece_pilot_policy();
        assert!(policy
            .validate_target_scope("1432704795", Some(".github/workflows/build-and-test.yml"))
            .is_ok());

        let err_repo = policy
            .validate_target_scope("99999", Some(".github/workflows/build-and-test.yml"))
            .unwrap_err();
        assert!(err_repo.contains("Policy target repo mismatch"));

        let err_wf = policy
            .validate_target_scope("1432704795", Some(".github/workflows/other.yml"))
            .unwrap_err();
        assert!(err_wf.contains("Policy target workflow mismatch"));
    }
}
