// tests/github_adapter_mock.rs — Negative and regression test suite for GitHub evidence adapter.

use ece_lib::github::client::{GitHubClientError, HttpGitHubClient};
use ece_lib::github::models::{
    GitHubArtifact, GitHubCommit, GitHubCommitDetails, GitHubJob, GitHubRepository,
    GitHubRunRepo, GitHubUser, GitHubWorkflowRun,
};
use ece_lib::github::policy::TrustedCheckPolicy;
use ece_lib::github::{
    evaluate_github_evidence, DeclaredArtifactSpec, MockGitHubProvider, TargetEvidenceRequest,
};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

fn resolve_schema_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("docs/gov-00/schemas"),
        PathBuf::from("../docs/gov-00/schemas"),
        PathBuf::from("../../docs/gov-00/schemas"),
    ];

    for c in &candidates {
        if c.join("assessment-output.schema.json").exists() {
            return c.clone();
        }
    }

    panic!("Could not find schema directory containing assessment-output.schema.json");
}

fn sample_repo(id: u64) -> GitHubRepository {
    GitHubRepository {
        id,
        name: "Evidence-Checkpoint-Engine".to_string(),
        full_name: "ErenTahiroglu/Evidence-Checkpoint-Engine".to_string(),
        owner: GitHubUser {
            login: "ErenTahiroglu".to_string(),
            id: 149682795,
        },
        default_branch: Some("main".to_string()),
    }
}

fn sample_commit(sha: &str) -> GitHubCommit {
    GitHubCommit {
        sha: sha.to_string(),
        commit: GitHubCommitDetails {
            message: Some(
                "ci(gov-01): enable workflow_dispatch and trigger fresh CI run".to_string(),
            ),
            author: None,
        },
    }
}

fn sample_run(
    head_sha: &str,
    run_id: u64,
    attempt: u32,
    status: &str,
    conclusion: Option<&str>,
) -> GitHubWorkflowRun {
    GitHubWorkflowRun {
        id: run_id,
        name: Some("Build and Test".to_string()),
        head_sha: head_sha.to_string(),
        path: Some(".github/workflows/build-and-test.yml".to_string()),
        run_attempt: attempt,
        event: Some("push".to_string()),
        status: status.to_string(),
        conclusion: conclusion.map(|s| s.to_string()),
        repository: Some(GitHubRunRepo {
            id: 1432704795,
            full_name: "ErenTahiroglu/Evidence-Checkpoint-Engine".to_string(),
        }),
    }
}

fn sample_jobs() -> Vec<GitHubJob> {
    vec![
        GitHubJob {
            id: 123782057785,
            name: "Windows x64".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            started_at: None,
            completed_at: None,
        },
        GitHubJob {
            id: 123782058130,
            name: "macOS Intel x64".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            started_at: None,
            completed_at: None,
        },
        GitHubJob {
            id: 123782058145,
            name: "Linux x64".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            started_at: None,
            completed_at: None,
        },
        GitHubJob {
            id: 123782058201,
            name: "macOS ARM64".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            started_at: None,
            completed_at: None,
        },
    ]
}

fn pilot_target() -> TargetEvidenceRequest {
    TargetEvidenceRequest {
        owner: "ErenTahiroglu".to_string(),
        repo: "Evidence-Checkpoint-Engine".to_string(),
        expected_repo_id: "1432704795".to_string(),
        commit_sha: "415990746d5bbd3756f743b4b5a7673b36228567".to_string(),
        run_id: 40969401836,
        run_attempt: 1,
        expected_workflow_path: Some(".github/workflows/build-and-test.yml".to_string()),
        checkpoint_id: Some("ece-gov-02-pilot-checkpoint".to_string()),
        declared_phase: Some("GOV-02".to_string()),
        declared_producer: Some("Eren Tahiroğlu".to_string()),
        declared_artifacts: Vec::new(),
    }
}

#[test]
fn test_01_correct_repository_and_run() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(sample_jobs()));
    provider.artifacts_result = Some(Ok(Vec::new()));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 1); // ASSESSED_LIMITATIONS
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_LIMITATIONS"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["status"],
        "VERIFIED_PASS"
    );
    assert_eq!(
        output.assessment["claims_breakdown"]["machine_assessed"]["repo_id_matched"],
        true
    );
    assert_eq!(
        output.assessment["claims_breakdown"]["machine_assessed"]["sha_verified"],
        true
    );
}

#[test]
fn test_02_wrong_repository_id() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(999999999))); // Wrong repo ID!
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(sample_jobs()));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2); // ASSESSED_FAIL
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output.assessment["claims_breakdown"]["machine_assessed"]["repo_id_matched"],
        false
    );
}

#[test]
fn test_03_wrong_sha() {
    let schema_dir = resolve_schema_dir();
    let mut target = pilot_target();
    target.commit_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(
        "415990746d5bbd3756f743b4b5a7673b36228567",
    )));
    provider.run_result = Some(Ok(sample_run(
        "415990746d5bbd3756f743b4b5a7673b36228567",
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(sample_jobs()));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output.assessment["claims_breakdown"]["machine_assessed"]["sha_verified"],
        false
    );
}

#[test]
fn test_04_wrong_run_attempt() {
    let schema_dir = resolve_schema_dir();
    let mut target = pilot_target();
    target.run_attempt = 2; // Declares attempt 2
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1, // Attempt 1!
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(sample_jobs()));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["status"],
        "ATTEMPT_MISMATCH"
    );
}

#[test]
fn test_05_missing_required_job() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut jobs = sample_jobs();
    jobs.retain(|j| j.name != "Windows x64"); // Missing Windows x64

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(jobs));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["status"],
        "FAILED"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["unresolved_jobs"],
        serde_json::json!(["Windows x64"])
    );
}

#[test]
fn test_06_failed_required_job() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut jobs = sample_jobs();
    for j in &mut jobs {
        if j.name == "Linux x64" {
            j.conclusion = Some("failure".to_string());
        }
    }

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(jobs));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["status"],
        "FAILED"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["unresolved_jobs"],
        serde_json::json!(["Linux x64"])
    );
}

#[test]
fn test_07_skipped_or_cancelled_required_job() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut jobs = sample_jobs();
    for j in &mut jobs {
        if j.name == "macOS ARM64" {
            j.conclusion = Some("cancelled".to_string());
        }
    }

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(jobs));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["status"],
        "INCOMPLETE_SKIPPED"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["unresolved_jobs"],
        serde_json::json!(["macOS ARM64"])
    );
}

#[test]
fn test_08_in_progress_run() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "in_progress", // Run in progress
        None,
    )));
    provider.jobs_result = Some(Ok(sample_jobs()));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["status"],
        "UNVERIFIED_RUN"
    );
}

#[test]
fn test_09_untrusted_empty_policy() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::new("empty-untrusted-policy", vec![]); // Zero required checks

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(sample_jobs()));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 3); // INSUFFICIENT_EVIDENCE
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "INSUFFICIENT_EVIDENCE"
    );
    assert_eq!(
        output.assessment["dimensions"]["ci_execution"]["status"],
        "UNVERIFIED_RUN"
    );
}

#[test]
fn test_10_pagination_in_http_client() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tcp listener");
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        // Page 1
        if let Ok((mut stream, _)) = listener.accept() {
            let mut req_buf = [0u8; 1024];
            let _ = stream.read(&mut req_buf);
            let b = concat!(
                r#"{"total_count":4,"jobs":["#,
                r#"{"id":1,"name":"j1","status":"completed","conclusion":"success"},"#,
                r#"{"id":2,"name":"j2","status":"completed","conclusion":"success"}]}"#
            );
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: \
                 application/json\r\n\r\n{}",
                b.len(),
                b
            );
            let _ = stream.write_all(resp.as_bytes());
        }

        // Page 2
        if let Ok((mut stream, _)) = listener.accept() {
            let mut req_buf = [0u8; 1024];
            let _ = stream.read(&mut req_buf);
            let b = concat!(
                r#"{"total_count":4,"jobs":["#,
                r#"{"id":3,"name":"j3","status":"completed","conclusion":"success"},"#,
                r#"{"id":4,"name":"j4","status":"completed","conclusion":"success"}]}"#
            );
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: \
                 application/json\r\n\r\n{}",
                b.len(),
                b
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    let client = HttpGitHubClient::new_with_base_url(
        format!("http://127.0.0.1:{port}"),
        None,
    )
    .unwrap();
    use ece_lib::github::GitHubProvider;
    let jobs = client.get_attempt_jobs("o", "r", 123, 1).expect("fetches paginated jobs");
    assert_eq!(jobs.len(), 4);
    assert_eq!(jobs[0].name, "j1");
    assert_eq!(jobs[3].name, "j4");
}

#[test]
fn test_11_http_403_429_rate_limited() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Err(GitHubClientError::RateLimited {
        reset_at: Some(1730000000),
        message: "Rate limit exceeded".to_string(),
    }));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 3); // INSUFFICIENT_EVIDENCE
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "INSUFFICIENT_EVIDENCE"
    );
}

#[test]
fn test_12_http_404_not_found() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Err(GitHubClientError::NotFound {
        resource: "repo".to_string(),
    }));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 3);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "INSUFFICIENT_EVIDENCE"
    );
}

#[test]
fn test_13_http_500_internal_error() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Err(GitHubClientError::HttpError {
        status: 500,
        message: "Internal Server Error".to_string(),
        rate_limit_reset: None,
    }));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 3);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "INSUFFICIENT_EVIDENCE"
    );
}

#[test]
fn test_14_truncated_response_json() {
    let schema_dir = resolve_schema_dir();
    let target = pilot_target();
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Err(GitHubClientError::InvalidJson(
        "unexpected EOF at line 1 column 24".to_string(),
    )));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 3);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "INSUFFICIENT_EVIDENCE"
    );
}

#[test]
fn test_15_unavailable_or_expired_artifact() {
    let schema_dir = resolve_schema_dir();
    let mut target = pilot_target();
    target.declared_artifacts.push(DeclaredArtifactSpec {
        artifact_name: "release-binary.zip".to_string(),
        artifact_id: Some(12345),
        expected_digest: None,
        verify_bytes: false,
    });
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(sample_jobs()));
    provider.artifacts_result = Some(Ok(vec![GitHubArtifact {
        id: 12345,
        name: "release-binary.zip".to_string(),
        size_in_bytes: 5000,
        archive_download_url: None,
        expired: true, // Expired!
        created_at: None,
        expires_at: None,
        digest: None,
    }]));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output.assessment["dimensions"]["evidence_availability"]["status"],
        "EXPIRED"
    );
}

#[test]
fn test_16_invalid_artifact_digest() {
    let schema_dir = resolve_schema_dir();
    let mut target = pilot_target();
    target.declared_artifacts.push(DeclaredArtifactSpec {
        artifact_name: "release-binary.zip".to_string(),
        artifact_id: Some(12345),
        expected_digest: Some(
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        ),
        verify_bytes: true,
    });
    let policy = TrustedCheckPolicy::ece_pilot_policy();

    let mut provider = MockGitHubProvider::new();
    provider.repo_result = Some(Ok(sample_repo(1432704795)));
    provider.commit_result = Some(Ok(sample_commit(&target.commit_sha)));
    provider.run_result = Some(Ok(sample_run(
        &target.commit_sha,
        target.run_id,
        1,
        "completed",
        Some("success"),
    )));
    provider.jobs_result = Some(Ok(sample_jobs()));
    provider.artifacts_result = Some(Ok(vec![GitHubArtifact {
        id: 12345,
        name: "release-binary.zip".to_string(),
        size_in_bytes: 5000,
        archive_download_url: None,
        expired: false,
        created_at: None,
        expires_at: None,
        digest: None,
    }]));
    provider.download_digest_result = Some(Ok(
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
    ));

    let output = evaluate_github_evidence(&provider, &target, &policy, &schema_dir)
        .expect("valid evaluation");
    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output.assessment["derived_summary"]["verdict"],
        "ASSESSED_FAIL"
    );
}
