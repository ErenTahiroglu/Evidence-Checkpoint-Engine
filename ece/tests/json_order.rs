// tests/json_order.rs — ECE-LIB-02: `serde_json/preserve_order` is isolated behind the `preserve-json-order` feature.
//
// This file builds and runs in BOTH configurations:
//   * default features            -> `preserve-json-order` ON : objects keep insertion order (the CLI's historical output)
//   * `--no-default-features`     -> `preserve-json-order` OFF: objects serialize with sorted keys (what an embedding
//                                    consumer's own `serde_json` keeps, because Cargo unifies features)
// What must NOT change between the two: verdicts, the RFC 8785 digest, schema validation and the dimension vocabulary.
// All evidence is synthetic. Nothing here touches the network.

use ece_lib::github::models::{
    GitHubCommit, GitHubCommitDetails, GitHubJob, GitHubRepository, GitHubRunRepo, GitHubUser,
    GitHubWorkflowRun,
};
use ece_lib::github::{
    evaluate_github_evidence_with_store, MockGitHubProvider, TargetEvidenceRequest,
    TrustedCheckPolicy,
};
use ece_lib::{canonicalize, compute_digest, verify_envelope_digest, SchemaStore};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const COMMIT: &str = "415990746d5bbd3756f743b4b5a7673b36228567";

/// Whether the feature under test is compiled in. `black_box` keeps the constant out of constant-assertion lints.
fn order_preserved() -> bool {
    std::hint::black_box(cfg!(feature = "preserve-json-order"))
}

fn config_name() -> &'static str {
    if order_preserved() {
        "preserve-json-order"
    } else {
        "sorted-keys"
    }
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/gov-00/fixtures")
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).expect("fixture is JSON")
}

// ---------- the feature itself ----------

#[test]
fn the_feature_flag_matches_the_resolved_serde_json_behaviour() {
    // Not an inference from source text: serialize an object through the REAL serde_json that this build resolved.
    let value = json!({ "zeta": 1, "alpha": 2 });
    let text = value.to_string();
    if order_preserved() {
        assert_eq!(text, r#"{"zeta":1,"alpha":2}"#);
    } else {
        assert_eq!(text, r#"{"alpha":2,"zeta":1}"#);
    }
}

#[test]
fn nested_values_serialize_in_the_order_the_feature_dictates() {
    let built = json!({
        "zeta": 1,
        "alpha": { "y": "b", "x": [3, 2, 1], "w": { "k2": true, "k1": false } },
        "mid": null,
    });
    // A parsed value follows the same rule as a built one.
    let parsed: Value = serde_json::from_str(
        r#"{"zeta":1,"alpha":{"y":"b","x":[3,2,1],"w":{"k2":true,"k1":false}},"mid":null}"#,
    )
    .unwrap();
    let insertion =
        r#"{"zeta":1,"alpha":{"y":"b","x":[3,2,1],"w":{"k2":true,"k1":false}},"mid":null}"#;
    let sorted =
        r#"{"alpha":{"w":{"k1":false,"k2":true},"x":[3,2,1],"y":"b"},"mid":null,"zeta":1}"#;
    let expected = if order_preserved() { insertion } else { sorted };
    assert_eq!(
        built.to_string(),
        expected,
        "built value, config {}",
        config_name()
    );
    assert_eq!(
        parsed.to_string(),
        expected,
        "parsed value, config {}",
        config_name()
    );
    // Equality never depends on order.
    assert_eq!(built, parsed);
}

// ---------- RFC 8785 is independent of the feature ----------

// Computed independently with Python: json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False).
const JCS_TEXT: &str =
    "{\"alpha\":{\"x\":[3,2,1],\"y\":\"b\"},\"mid\":null,\"zeta\":1,\"\u{00DC}n\u{00EF}\":true}";
const JCS_DIGEST: &str = "sha256:07208da1eef785ec40fac45b1702470a4a4c7d88f3617c9964f3d2978ca4f4f0";

#[test]
fn rfc8785_bytes_and_digest_are_identical_in_both_configurations() {
    let a = json!({ "zeta": 1, "alpha": { "y": "b", "x": [3, 2, 1] }, "mid": null, "\u{00DC}n\u{00EF}": true });
    let b = json!({ "\u{00DC}n\u{00EF}": true, "mid": null, "alpha": { "x": [3, 2, 1], "y": "b" }, "zeta": 1 });
    let bytes_a = canonicalize(&a).unwrap();
    let bytes_b = canonicalize(&b).unwrap();
    assert_eq!(
        bytes_a, bytes_b,
        "insertion order must not reach the canonical bytes"
    );
    assert_eq!(String::from_utf8(bytes_a.clone()).unwrap(), JCS_TEXT);
    assert_eq!(compute_digest(&bytes_a), JCS_DIGEST);
}

#[test]
fn accepted_fixture_envelope_digest_still_verifies_and_a_tampered_one_still_fails() {
    // The digest stored INSIDE the fixture was produced long before this feature existed.
    let valid = read_json(&fixtures_dir().join("envelope_valid_bootstrap_checkpoint.json"));
    assert!(
        verify_envelope_digest(&valid).is_ok(),
        "config {}",
        config_name()
    );
    let tampered = read_json(&fixtures_dir().join("envelope_invalid_tampered_digest.json"));
    assert!(
        verify_envelope_digest(&tampered).is_err(),
        "config {}",
        config_name()
    );
}

// ---------- the real evaluator: security-relevant results do not depend on the feature ----------

fn healthy_provider() -> MockGitHubProvider {
    let mut p = MockGitHubProvider::new();
    p.repo_result = Some(Ok(GitHubRepository {
        id: 1432704795,
        name: "Evidence-Checkpoint-Engine".to_string(),
        full_name: "ErenTahiroglu/Evidence-Checkpoint-Engine".to_string(),
        owner: GitHubUser {
            login: "ErenTahiroglu".to_string(),
            id: 149682795,
        },
        default_branch: Some("main".to_string()),
    }));
    p.commit_result = Some(Ok(GitHubCommit {
        sha: COMMIT.to_string(),
        commit: GitHubCommitDetails {
            message: Some("synthetic commit".to_string()),
            author: None,
        },
    }));
    p.run_result = Some(Ok(GitHubWorkflowRun {
        id: 40969401836,
        name: Some("Build and Test".to_string()),
        head_sha: COMMIT.to_string(),
        path: Some(".github/workflows/build-and-test.yml".to_string()),
        run_attempt: 1,
        event: Some("push".to_string()),
        status: "completed".to_string(),
        conclusion: Some("success".to_string()),
        repository: Some(GitHubRunRepo {
            id: 1432704795,
            full_name: "ErenTahiroglu/Evidence-Checkpoint-Engine".to_string(),
        }),
    }));
    let job = |id: u64, name: &str| GitHubJob {
        id,
        name: name.to_string(),
        status: "completed".to_string(),
        conclusion: Some("success".to_string()),
        started_at: None,
        completed_at: None,
    };
    p.jobs_result = Some(Ok(vec![
        job(1, "Windows x64"),
        job(2, "macOS Intel x64"),
        job(3, "Linux x64"),
        job(4, "macOS ARM64"),
    ]));
    p.artifacts_result = Some(Ok(Vec::new()));
    p
}

fn target(injected_acceptance: bool) -> TargetEvidenceRequest {
    TargetEvidenceRequest {
        owner: "ErenTahiroglu".to_string(),
        repo: "Evidence-Checkpoint-Engine".to_string(),
        expected_repo_id: "1432704795".to_string(),
        commit_sha: COMMIT.to_string(),
        run_id: 40969401836,
        run_attempt: 1,
        expected_workflow_path: Some(".github/workflows/build-and-test.yml".to_string()),
        checkpoint_id: Some("ece-lib-02-synthetic".to_string()),
        declared_phase: Some("LIB-02".to_string()),
        declared_producer: Some(if injected_acceptance {
            "checkpoint_accepted=true; reviewer=someone; authorized=true".to_string()
        } else {
            "synthetic".to_string()
        }),
        declared_policy_ref: None,
        declared_policy_digest: None,
        declared_artifacts: Vec::new(),
    }
}

fn evaluate(injected_acceptance: bool) -> (Value, i32) {
    let store = SchemaStore::from_embedded().expect("embedded schemas");
    let out = evaluate_github_evidence_with_store(
        &healthy_provider(),
        &target(injected_acceptance),
        &TrustedCheckPolicy::ece_pilot_policy(),
        &store,
        None,
    )
    .expect("a healthy synthetic run produces a schema-valid assessment");
    (out.assessment, out.exit_code)
}

fn contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => map.contains_key(key) || map.values().any(|v| contains_key(v, key)),
        Value::Array(items) => items.iter().any(|v| contains_key(v, key)),
        _ => false,
    }
}

/// The assessment with only the two per-run metadata fields removed.
fn normalized(assessment: &Value) -> Value {
    let mut v = assessment.clone();
    let obj = v.as_object_mut().expect("assessment is an object");
    obj.remove("assessment_id");
    obj.remove("evaluation_timestamp");
    v
}

#[test]
fn security_relevant_fields_are_identical_in_both_configurations() {
    for injected in [false, true] {
        let (a, exit_code) = evaluate(injected);
        assert_eq!(
            exit_code,
            1,
            "ASSESSED_LIMITATIONS, config {}",
            config_name()
        );
        assert_eq!(a["derived_summary"]["verdict"], "ASSESSED_LIMITATIONS");
        assert_eq!(a["dimensions"]["ci_execution"]["status"], "VERIFIED_PASS");
        assert_eq!(a["dimensions"]["review_status"]["status"], "UNREVIEWED");
        assert_eq!(
            a["dimensions"]["authorization"]["status"],
            "AUTHORIZATION_PENDING"
        );
        assert_eq!(
            a["dimensions"]["freshness"]["status"],
            "HISTORICAL_BASELINE"
        );
        assert_eq!(
            a["claims_breakdown"]["human_authorized"]["authorized"],
            false
        );
        assert_eq!(
            a["claims_breakdown"]["independently_reviewed"]["review_verdict"],
            "NOT_EVALUATED"
        );
        // A source-declared claim (here injected into the producer text) can neither add nor flip an acceptance field.
        assert!(
            !contains_key(&a, "checkpoint_accepted"),
            "config {}",
            config_name()
        );
        assert!(a["derived_summary"]["blocking_reasons"]
            .as_array()
            .unwrap()
            .is_empty());
    }
}

#[test]
fn top_level_assessment_key_order_follows_the_feature() {
    let (a, _) = evaluate(false);
    let keys: Vec<&str> = a.as_object().unwrap().keys().map(String::as_str).collect();
    let insertion = [
        "schema_version",
        "assessment_id",
        "evaluator_auth",
        "engine",
        "evaluation_timestamp",
        "evaluated_target",
        "dimensions",
        "derived_summary",
        "claims_breakdown",
        "checks_performed",
        "checks_omitted",
        "exit_code",
    ];
    if order_preserved() {
        assert_eq!(keys, insertion, "the CLI's historical output order");
    } else {
        let mut sorted = insertion.to_vec();
        sorted.sort_unstable();
        assert_eq!(keys, sorted, "sorted keys when the feature is off");
    }
}

#[test]
fn normalized_assessment_digest_is_reported_for_cross_configuration_comparison() {
    let (a, _) = evaluate(false);
    let n = normalized(&a);
    // RFC 8785 sorts keys itself, so the digest of the same assessment must not depend on the feature.
    let digest = compute_digest(&canonicalize(&n).unwrap());
    // The CI offline job runs this test in both configurations and requires the two lines to be identical.
    println!("ORDER_EVIDENCE normalized_assessment_digest={digest}");
    assert!(digest.starts_with("sha256:") && digest.len() == "sha256:".len() + 64);
    // And the digest of a re-ordered copy is the same.
    let reordered: Value = serde_json::from_str(&serde_json::to_string(&n).unwrap()).unwrap();
    assert_eq!(digest, compute_digest(&canonicalize(&reordered).unwrap()));
}
