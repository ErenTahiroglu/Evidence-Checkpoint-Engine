// tests/offline_library.rs — ECE-LIB-01: the offline embeddable library boundary.
//
// Nothing here imports the HTTP client, so this file builds and runs with `--no-default-features`
// (no `github-http` feature, no `reqwest`) as well as with the default features. All evidence is synthetic.

use ece_lib::github::models::{
    GitHubCommit, GitHubCommitDetails, GitHubJob, GitHubRepository, GitHubRunRepo, GitHubUser,
    GitHubWorkflowRun,
};
use ece_lib::github::{
    evaluate_github_evidence, evaluate_github_evidence_with_store, GitHubClientError,
    MockGitHubProvider, TargetEvidenceRequest, TrustedCheckPolicy,
};
use ece_lib::{
    SchemaStore, VerifierError, EMBEDDED_ASSESSMENT_SCHEMA, EMBEDDED_ENVELOPE_SCHEMA,
    EMBEDDED_INPUT_SCHEMA,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

const COMMIT: &str = "415990746d5bbd3756f743b4b5a7673b36228567";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn schema_dir() -> PathBuf {
    repo_root().join("docs/gov-00/schemas")
}

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .replace("\r\n", "\n")
}

fn file_store() -> SchemaStore {
    SchemaStore::load(&schema_dir()).expect("file-based schema store")
}

fn embedded_store() -> SchemaStore {
    SchemaStore::from_embedded().expect("embedded schema store")
}

// ---------- synthetic evidence ----------

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
            message: Some("synthetic commit".to_string()),
            author: None,
        },
    }
}

fn sample_run(head_sha: &str, attempt: u32) -> GitHubWorkflowRun {
    GitHubWorkflowRun {
        id: 40969401836,
        name: Some("Build and Test".to_string()),
        head_sha: head_sha.to_string(),
        path: Some(".github/workflows/build-and-test.yml".to_string()),
        run_attempt: attempt,
        event: Some("push".to_string()),
        status: "completed".to_string(),
        conclusion: Some("success".to_string()),
        repository: Some(GitHubRunRepo {
            id: 1432704795,
            full_name: "ErenTahiroglu/Evidence-Checkpoint-Engine".to_string(),
        }),
    }
}

fn job(id: u64, name: &str, conclusion: &str) -> GitHubJob {
    GitHubJob {
        id,
        name: name.to_string(),
        status: "completed".to_string(),
        conclusion: Some(conclusion.to_string()),
        started_at: None,
        completed_at: None,
    }
}

fn sample_jobs() -> Vec<GitHubJob> {
    vec![
        job(1, "Windows x64", "success"),
        job(2, "macOS Intel x64", "success"),
        job(3, "Linux x64", "success"),
        job(4, "macOS ARM64", "success"),
    ]
}

fn target() -> TargetEvidenceRequest {
    TargetEvidenceRequest {
        owner: "ErenTahiroglu".to_string(),
        repo: "Evidence-Checkpoint-Engine".to_string(),
        expected_repo_id: "1432704795".to_string(),
        commit_sha: COMMIT.to_string(),
        run_id: 40969401836,
        run_attempt: 1,
        expected_workflow_path: Some(".github/workflows/build-and-test.yml".to_string()),
        checkpoint_id: Some("ece-lib-01-synthetic".to_string()),
        declared_phase: Some("LIB-01".to_string()),
        declared_producer: Some("synthetic".to_string()),
        declared_policy_ref: None,
        declared_policy_digest: None,
        declared_artifacts: Vec::new(),
    }
}

fn healthy_provider() -> MockGitHubProvider {
    let mut p = MockGitHubProvider::new();
    p.repo_result = Some(Ok(sample_repo(1432704795)));
    p.commit_result = Some(Ok(sample_commit(COMMIT)));
    p.run_result = Some(Ok(sample_run(COMMIT, 1)));
    p.jobs_result = Some(Ok(sample_jobs()));
    p.artifacts_result = Some(Ok(Vec::new()));
    p
}

struct Scenario {
    name: &'static str,
    provider: MockGitHubProvider,
    target: TargetEvidenceRequest,
    policy: TrustedCheckPolicy,
    expected_exit: i32,
    expected_verdict: &'static str,
}

fn scenarios() -> Vec<Scenario> {
    let pilot = TrustedCheckPolicy::ece_pilot_policy;
    let mut out = vec![Scenario {
        name: "valid historical observation",
        provider: healthy_provider(),
        target: target(),
        policy: pilot(),
        expected_exit: 1,
        expected_verdict: "ASSESSED_LIMITATIONS",
    }];

    let mut p = healthy_provider();
    p.repo_result = Some(Ok(sample_repo(999999999)));
    out.push(Scenario {
        name: "wrong numeric repository id",
        provider: p,
        target: target(),
        policy: pilot(),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    let mut t = target();
    t.commit_sha = "a".repeat(40);
    out.push(Scenario {
        name: "wrong source sha",
        provider: healthy_provider(),
        target: t,
        policy: pilot(),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    let mut t = target();
    t.run_attempt = 2;
    out.push(Scenario {
        name: "wrong run attempt",
        provider: healthy_provider(),
        target: t,
        policy: pilot(),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    let mut p = healthy_provider();
    let mut jobs = sample_jobs();
    jobs.retain(|j| j.name != "Windows x64");
    p.jobs_result = Some(Ok(jobs));
    out.push(Scenario {
        name: "missing required job",
        provider: p,
        target: target(),
        policy: pilot(),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    let mut p = healthy_provider();
    let mut jobs = sample_jobs();
    jobs[2].conclusion = Some("failure".to_string());
    p.jobs_result = Some(Ok(jobs));
    out.push(Scenario {
        name: "failed required job",
        provider: p,
        target: target(),
        policy: pilot(),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    let mut p = healthy_provider();
    let mut jobs = sample_jobs();
    jobs[3].conclusion = Some("cancelled".to_string());
    p.jobs_result = Some(Ok(jobs));
    out.push(Scenario {
        name: "cancelled required job",
        provider: p,
        target: target(),
        policy: pilot(),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    let mut p = healthy_provider();
    let mut jobs = sample_jobs();
    jobs.push(job(99, "Linux x64", "success"));
    p.jobs_result = Some(Ok(jobs));
    out.push(Scenario {
        name: "duplicate contradictory job evidence",
        provider: p,
        target: target(),
        policy: pilot(),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    out.push(Scenario {
        name: "mutated policy: empty required jobs",
        provider: healthy_provider(),
        target: target(),
        policy: TrustedCheckPolicy::new("mutated-empty-policy", vec![], None, None),
        expected_exit: 3,
        expected_verdict: "INSUFFICIENT_EVIDENCE",
    });

    out.push(Scenario {
        name: "mutated policy: wrong workflow scope",
        provider: healthy_provider(),
        target: target(),
        policy: TrustedCheckPolicy::new(
            "mutated-scope-policy",
            vec!["Linux x64".to_string()],
            Some("1432704795".to_string()),
            Some(".github/workflows/other.yml".to_string()),
        ),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    let mut p = healthy_provider();
    let mut run = sample_run(COMMIT, 1);
    run.path = Some(".github/workflows/injected.yml".to_string());
    p.run_result = Some(Ok(run));
    out.push(Scenario {
        name: "wrong workflow identity",
        provider: p,
        target: target(),
        policy: pilot(),
        expected_exit: 2,
        expected_verdict: "ASSESSED_FAIL",
    });

    let mut p = healthy_provider();
    p.repo_result = Some(Err(GitHubClientError::NotFound {
        resource: "repo".to_string(),
    }));
    out.push(Scenario {
        name: "provider error",
        provider: p,
        target: target(),
        policy: pilot(),
        expected_exit: 3,
        expected_verdict: "INSUFFICIENT_EVIDENCE",
    });

    // Injected source claims: they are only reported as source-declared, never promoted.
    let mut t = target();
    t.declared_producer = Some("independent reviewer approved this checkpoint".to_string());
    t.declared_phase = Some("ACCEPTED".to_string());
    out.push(Scenario {
        name: "injected source/reviewer claims",
        provider: healthy_provider(),
        target: t,
        policy: pilot(),
        expected_exit: 1,
        expected_verdict: "ASSESSED_LIMITATIONS",
    });

    out
}

/// The evaluator stamps a fresh UUID and wall-clock time into every assessment. They carry no verification meaning,
/// so they are the only fields removed before two assessments are compared.
fn normalize(mut assessment: Value) -> Value {
    let obj = assessment.as_object_mut().expect("assessment is an object");
    let id = obj.remove("assessment_id").expect("assessment_id present");
    assert!(
        id.as_str().unwrap().starts_with("ece-eval-"),
        "unexpected assessment id shape"
    );
    obj.remove("evaluation_timestamp")
        .expect("evaluation_timestamp present");
    assessment
}

// ---------- embedded schema store ----------

#[test]
fn embedded_schema_texts_are_the_accepted_schema_files() {
    for (name, embedded) in [
        ("checkpoint-input.schema.json", EMBEDDED_INPUT_SCHEMA),
        ("checkpoint-envelope.schema.json", EMBEDDED_ENVELOPE_SCHEMA),
        ("assessment-output.schema.json", EMBEDDED_ASSESSMENT_SCHEMA),
    ] {
        let on_disk = read_text(&schema_dir().join(name));
        assert_eq!(
            embedded.replace("\r\n", "\n"),
            on_disk,
            "{name}: the embedded copy differs from the accepted file"
        );
        // Each is a real JSON Schema document with an $id (so local $ref resolution can register it).
        let v: Value = serde_json::from_str(embedded).expect("embedded schema is JSON");
        assert!(
            v.get("$id").and_then(|x| x.as_str()).is_some(),
            "{name}: no $id"
        );
    }
}

#[test]
fn embedded_and_file_schemas_agree_on_every_synthetic_fixture() {
    let fixtures = repo_root().join("docs/gov-00/fixtures");
    let file = file_store();
    let embedded = embedded_store();

    let mut checked = 0usize;
    let mut rejected = 0usize;
    let mut names: Vec<_> = std::fs::read_dir(&fixtures)
        .expect("fixtures dir")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();

    for name in names {
        let doc: Value = match serde_json::from_str(&read_text(&fixtures.join(&name))) {
            Ok(v) => v,
            Err(_) => continue, // syntactically broken documents never reach a validator
        };
        let (a, b) = if name.starts_with("input_") {
            (file.validate_payload(&doc), embedded.validate_payload(&doc))
        } else if name.starts_with("envelope_") {
            (
                file.validate_envelope(&doc),
                embedded.validate_envelope(&doc),
            )
        } else if name.starts_with("output_") {
            (
                file.validate_assessment(&doc),
                embedded.validate_assessment(&doc),
            )
        } else {
            continue;
        };
        // Same accept/reject decision and the same error details.
        assert_eq!(
            format!("{a:?}"),
            format!("{b:?}"),
            "{name}: embedded and file-based schemas disagree"
        );
        checked += 1;
        if a.is_err() {
            rejected += 1;
        }
    }
    assert!(
        checked >= 15,
        "expected the synthetic fixtures, checked {checked}"
    );
    assert!(rejected >= 5, "expected rejecting fixtures, saw {rejected}");
}

#[test]
fn embedded_store_accepts_the_valid_and_rejects_the_invalid_reference_fixtures() {
    let fixtures = repo_root().join("docs/gov-00/fixtures");
    let embedded = embedded_store();
    let load = |n: &str| -> Value { serde_json::from_str(&read_text(&fixtures.join(n))).unwrap() };

    assert!(embedded
        .validate_payload(&load("input_valid_bootstrap_checkpoint.json"))
        .is_ok());
    assert!(embedded
        .validate_envelope(&load("envelope_valid_bootstrap_checkpoint.json"))
        .is_ok());
    assert!(embedded
        .validate_assessment(&load("output_valid_provisional_assessment.json"))
        .is_ok());

    for bad in [
        "input_invalid_unsupported_schema_version.json",
        "input_invalid_git_sha1_with_sha256_length.json",
        "input_invalid_git_sha256_with_sha1_length.json",
        "input_invalid_id_leading_zero.json",
        "input_invalid_self_accepted_injection.json",
    ] {
        assert!(
            embedded.validate_payload(&load(bad)).is_err(),
            "{bad} must be rejected by the embedded schema"
        );
    }
    for bad in [
        "output_invalid_mock_claiming_pass.json",
        "output_invalid_synthetic_claiming_human_auth.json",
    ] {
        assert!(
            embedded.validate_assessment(&load(bad)).is_err(),
            "{bad} must be rejected by the embedded schema"
        );
    }
}

#[test]
fn file_based_load_is_preserved_and_still_fails_closed() {
    // A missing directory is a schema I/O error, not a silent fallback to the embedded copies.
    let missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ece-lib-01-no-such-schema-dir");
    match SchemaStore::load(&missing) {
        Err(VerifierError::SchemaIo(_)) => {}
        Err(other) => panic!("expected SchemaIo, got {other:?}"),
        Ok(_) => panic!("a missing schema directory must not load"),
    }
}

#[test]
fn an_incompatible_output_schema_makes_the_evaluator_fail_closed() {
    // Same idea as the CLI smoke test: a store whose assessment schema the engine can never satisfy yields no output.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ece-lib-01-incompatible-schemas");
    std::fs::create_dir_all(&dir).unwrap();
    for name in [
        "checkpoint-input.schema.json",
        "checkpoint-envelope.schema.json",
    ] {
        std::fs::copy(schema_dir().join(name), dir.join(name)).unwrap();
    }
    let id = serde_json::from_str::<Value>(EMBEDDED_ASSESSMENT_SCHEMA).unwrap()["$id"]
        .as_str()
        .unwrap()
        .to_string();
    let incompatible = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": id,
        "type": "object",
        "required": ["mandatory_field_engine_will_never_emit"],
        "properties": { "mandatory_field_engine_will_never_emit": { "type": "string" } }
    });
    std::fs::write(
        dir.join("assessment-output.schema.json"),
        serde_json::to_vec(&incompatible).unwrap(),
    )
    .unwrap();

    let store = SchemaStore::load(&dir).expect("the incompatible set still compiles");
    let result = evaluate_github_evidence_with_store(
        &healthy_provider(),
        &target(),
        &TrustedCheckPolicy::ece_pilot_policy(),
        &store,
        None,
    );
    let _ = std::fs::remove_dir_all(&dir);
    match result {
        Err(VerifierError::OutputSchemaViolation { .. }) => {}
        Err(other) => panic!("expected OutputSchemaViolation, got {other:?}"),
        Ok(_) => panic!("an assessment the output schema rejects must not be returned"),
    }
}

// ---------- the real evaluator with the in-memory provider ----------

#[test]
fn real_evaluator_with_embedded_schemas_and_mock_provider() {
    let store = embedded_store();
    let out = evaluate_github_evidence_with_store(
        &healthy_provider(),
        &target(),
        &TrustedCheckPolicy::ece_pilot_policy(),
        &store,
        None,
    )
    .expect("valid synthetic evaluation");

    let a = &out.assessment;
    assert_eq!(out.exit_code, 1);
    assert_eq!(a["derived_summary"]["verdict"], "ASSESSED_LIMITATIONS");
    assert_eq!(a["dimensions"]["ci_execution"]["status"], "VERIFIED_PASS");
    // A positive CI fact never becomes review, authorization or acceptance.
    assert_eq!(a["dimensions"]["review_status"]["status"], "UNREVIEWED");
    assert_eq!(
        a["dimensions"]["authorization"]["status"],
        "AUTHORIZATION_PENDING"
    );
    assert_eq!(
        a["claims_breakdown"]["human_authorized"]["authorized"],
        false
    );
    assert_eq!(
        a["claims_breakdown"]["independently_reviewed"]["review_verdict"],
        "NOT_EVALUATED"
    );
    assert_eq!(
        a["dimensions"]["evidence_integrity"]["status"],
        "NOT_EVALUATED"
    );
    assert_eq!(
        a["dimensions"]["freshness"]["status"],
        "HISTORICAL_BASELINE"
    );
    assert!(a["checks_omitted"].as_array().unwrap().iter().any(|c| c
        .as_str()
        .unwrap()
        .contains("Independent human authorization")));
}

#[test]
fn every_scenario_fails_conservatively_and_both_entry_points_agree() {
    let embedded = embedded_store();
    let dir = schema_dir();
    for s in scenarios() {
        let by_store =
            evaluate_github_evidence_with_store(&s.provider, &s.target, &s.policy, &embedded, None)
                .unwrap_or_else(|e| panic!("{}: store entry point failed: {e:?}", s.name));
        let by_path = evaluate_github_evidence(&s.provider, &s.target, &s.policy, &dir, None)
            .unwrap_or_else(|e| panic!("{}: path entry point failed: {e:?}", s.name));

        assert_eq!(by_store.exit_code, s.expected_exit, "{}: exit code", s.name);
        assert_eq!(
            by_store.assessment["derived_summary"]["verdict"], s.expected_verdict,
            "{}: verdict",
            s.name
        );
        assert_eq!(
            by_path.exit_code, by_store.exit_code,
            "{}: exit parity",
            s.name
        );
        assert_eq!(
            normalize(by_path.assessment),
            normalize(by_store.assessment.clone()),
            "{}: the path-based wrapper and the store entry point disagree",
            s.name
        );

        // No scenario ever yields a verdict above ASSESSED_LIMITATIONS, human approval, or an authorization.
        let a = &by_store.assessment;
        assert_ne!(
            a["derived_summary"]["verdict"], "ASSESSED_PASS",
            "{}",
            s.name
        );
        assert_eq!(
            a["claims_breakdown"]["human_authorized"]["authorized"],
            false
        );
        assert_eq!(
            a["dimensions"]["authorization"]["status"], "AUTHORIZATION_PENDING",
            "{}",
            s.name
        );
        // A provider failure halts before review is considered (NOT_EVALUATED); otherwise it is UNREVIEWED.
        // Never an approved or reviewed state.
        let review = a["dimensions"]["review_status"]["status"].as_str().unwrap();
        assert!(
            review == "UNREVIEWED" || review == "NOT_EVALUATED",
            "{}: unexpected review status {review}",
            s.name
        );
        if s.expected_exit != 1 {
            assert_ne!(
                a["dimensions"]["ci_execution"]["status"], "VERIFIED_PASS",
                "{}: a failed or unverified observation must not report a CI pass",
                s.name
            );
        }
    }
}

#[test]
fn injected_source_claims_are_reported_as_claims_only() {
    let mut t = target();
    t.declared_producer = Some("independent reviewer approved this checkpoint".to_string());
    t.declared_phase = Some("ACCEPTED".to_string());
    let out = evaluate_github_evidence_with_store(
        &healthy_provider(),
        &t,
        &TrustedCheckPolicy::ece_pilot_policy(),
        &embedded_store(),
        None,
    )
    .unwrap();
    let a = &out.assessment;
    assert_eq!(
        a["claims_breakdown"]["source_declared"]["claimed_producer"],
        "independent reviewer approved this checkpoint"
    );
    assert_eq!(
        a["claims_breakdown"]["independently_reviewed"]["reviewer"],
        Value::Null
    );
    assert_eq!(
        a["dimensions"]["review_status"]["reviewer_identity"],
        Value::Null
    );
    assert_eq!(a["dimensions"]["authorization"]["decision_by"], Value::Null);
    assert_eq!(a["derived_summary"]["verdict"], "ASSESSED_LIMITATIONS");
}

#[test]
fn per_run_ids_and_timestamps_are_not_evaluation_semantics() {
    let store = embedded_store();
    let run = || {
        evaluate_github_evidence_with_store(
            &healthy_provider(),
            &target(),
            &TrustedCheckPolicy::ece_pilot_policy(),
            &store,
            None,
        )
        .unwrap()
    };
    let (one, two) = (run(), run());
    assert_ne!(
        one.assessment["assessment_id"], two.assessment["assessment_id"],
        "every evaluation gets a fresh id"
    );
    assert_eq!(one.exit_code, two.exit_code);
    // Everything else is identical, so the volatile fields are the only difference.
    assert_eq!(normalize(one.assessment), normalize(two.assessment));
}

// ---------- the offline feature boundary ----------

#[test]
fn offline_modules_contain_no_http_client_token_lookup_or_socket_code() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in [
        "github/client.rs",
        "github/evaluator.rs",
        "github/models.rs",
        "github/policy.rs",
        "verifier/schema.rs",
        "verifier/assessment.rs",
        "verifier/semantic.rs",
        "verifier/parser.rs",
        "verifier/digest.rs",
        "verifier/canonicalize.rs",
        "verifier/error.rs",
        "lib.rs",
    ] {
        let text = read_text(&src.join(file));
        for forbidden in [
            "GITHUB_TOKEN",
            "GH_TOKEN",
            "reqwest",
            "std::net",
            "std::env",
            "bearer_auth",
            "std::process",
        ] {
            assert!(
                !text.contains(forbidden),
                "{file} must not mention `{forbidden}`: the live client belongs to http_client.rs only"
            );
        }
    }

    // The one module that does is compiled only with the feature.
    let http = read_text(&src.join("github/http_client.rs"));
    assert!(http.contains("GITHUB_TOKEN") && http.contains("reqwest"));
    let github_mod = read_text(&src.join("github/mod.rs"));
    assert!(
        github_mod.contains("#[cfg(feature = \"github-http\")]\npub mod http_client;"),
        "http_client must be gated by the github-http feature"
    );
    let client = read_text(&src.join("github/client.rs"));
    assert!(
        client.contains("#[cfg(feature = \"github-http\")]\npub use super::http_client::{"),
        "the re-export of the HTTP client must be gated by the github-http feature"
    );

    // Cargo wiring: optional dependency, opt-in only through the named feature, CLI requires it.
    let manifest = read_text(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
    assert!(manifest.contains("github-http = [\"dep:reqwest\"]"));
    assert!(manifest.contains("], optional = true }"));
    assert!(manifest.contains("required-features = [\"github-http\"]"));
}

#[cfg(not(feature = "github-http"))]
#[test]
fn offline_build_has_the_http_feature_disabled() {
    // Marker: this test only exists in a `--no-default-features` build, so a green run proves that build ran the
    // whole offline suite above (including the real evaluator with a mock provider and the embedded schemas).
    assert!(!cfg!(feature = "github-http"));
}

#[cfg(feature = "github-http")]
#[test]
fn default_build_still_exposes_the_http_client_for_the_cli() {
    // Existing behaviour is preserved in the normal build: the live client type is still reachable at its old paths.
    fn assert_provider<T: ece_lib::github::GitHubProvider>() {}
    assert_provider::<ece_lib::github::HttpGitHubClient>();
    assert_provider::<ece_lib::github::client::HttpGitHubClient>();
}
