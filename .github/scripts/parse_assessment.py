#!/usr/bin/env python3
"""
Evidence Checkpoint Engine (ECE) — Assessment Output Parser & Gate Evaluator
Minimal explicit gate-contract validator for ECE reusable GitHub Actions workflow.

This helper validates the deterministic assessment JSON emitted by ECE against
the structural and semantic requirements of docs/gov-00/schemas/assessment-output.schema.json.
It strictly binds CLI process exit code, JSON exit_code, verdict, dimensions, and
gate acceptance policy without inferring verification from missing defaults or
unauthenticated self-claims.
"""

import html
import json
import os
import sys

ALLOWED_VERDICTS = [
    "ASSESSED_PROVISIONAL_PASS",
    "ASSESSED_LIMITATIONS",
    "ASSESSED_FAIL",
    "INSUFFICIENT_EVIDENCE",
    "REFUSED",
]

ALLOWED_INTEGRITY_STATUSES = [
    "VALID",
    "INVALID",
    "CORRUPT",
    "TAMPERED",
    "NOT_EVALUATED",
]

ALLOWED_CI_STATUSES = [
    "VERIFIED_PASS",
    "FAILED",
    "INCOMPLETE_SKIPPED",
    "ATTEMPT_MISMATCH",
    "UNVERIFIED_RUN",
    "NOT_APPLICABLE",
]

ALLOWED_AVAILABILITY_STATUSES = [
    "AVAILABLE",
    "PARTIALLY_AVAILABLE",
    "EXPIRED",
    "UNREACHABLE",
    "NOT_PROVIDED",
    "NOT_EVALUATED",
]

ALLOWED_REVIEW_STATUSES = [
    "INDEPENDENT_REVIEW_PASS",
    "STATIC_PASS_WITH_LIMITATIONS",
    "SELF_REPORT_ONLY",
    "REVIEW_FAILED",
    "UNREVIEWED",
    "NOT_EVALUATED",
]

ALLOWED_REVIEW_TIERS = ["NONE", "SELF_CLAIM", "STATIC_AUDIT", "INDEPENDENT_FULL"]

ALLOWED_AUTH_STATUSES = [
    "EXPLICITLY_AUTHORIZED",
    "AUTHORIZATION_PENDING",
    "REJECTED",
    "REVOKED",
]

ALLOWED_FRESHNESS_STATUSES = [
    "CURRENT_HEAD",
    "HISTORICAL_BASELINE",
    "STALE_DIVERGED",
    "UNKNOWN_HEAD",
    "NOT_EVALUATED",
]

REQUIRED_TOP_LEVEL_KEYS = [
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
]

REQUIRED_DIMENSIONS = [
    "evidence_integrity",
    "ci_execution",
    "evidence_availability",
    "review_status",
    "authorization",
    "freshness",
]


def validate_assessment_structure(data, cli_rc):
    """
    Validates the structural and semantic integrity of the parsed assessment dictionary.
    Returns (is_valid: bool, error_msg: str, sanitized_data: dict).
    """
    if not isinstance(data, dict):
        return False, "Assessment root is not a valid JSON object", {}

    # 1. Check all required top-level keys
    for key in REQUIRED_TOP_LEVEL_KEYS:
        if key not in data:
            return False, f"Missing required top-level key: '{key}'", data

    # 2. Validate types of essential top-level structures
    if not isinstance(data.get("schema_version"), str):
        return False, "Top-level 'schema_version' must be a string", data
    if not isinstance(data.get("assessment_id"), str):
        return False, "Top-level 'assessment_id' must be a string", data
    if not isinstance(data.get("evaluator_auth"), dict):
        return False, "Top-level 'evaluator_auth' must be an object", data
    if not isinstance(data.get("engine"), dict):
        return False, "Top-level 'engine' must be an object", data
    if not isinstance(data.get("evaluation_timestamp"), str):
        return False, "Top-level 'evaluation_timestamp' must be a string", data
    if not isinstance(data.get("evaluated_target"), dict):
        return False, "Top-level 'evaluated_target' must be an object", data
    if not isinstance(data.get("dimensions"), dict):
        return False, "Top-level 'dimensions' must be an object", data
    if not isinstance(data.get("derived_summary"), dict):
        return False, "Top-level 'derived_summary' must be an object", data
    if not isinstance(data.get("claims_breakdown"), dict):
        return False, "Top-level 'claims_breakdown' must be an object", data
    if not isinstance(data.get("checks_performed"), list):
        return False, "Top-level 'checks_performed' must be an array", data
    if not isinstance(data.get("checks_omitted"), list):
        return False, "Top-level 'checks_omitted' must be an array", data

    # 3. Validate exit_code: MUST be present, MUST be genuine int (not bool), in [0..4]
    raw_exit_code = data.get("exit_code")
    if raw_exit_code is None:
        return False, "Top-level 'exit_code' must be present", data
    if not isinstance(raw_exit_code, int) or isinstance(raw_exit_code, bool):
        return False, f"Top-level 'exit_code' must be a genuine integer (got {type(raw_exit_code).__name__})", data
    if raw_exit_code not in (0, 1, 2, 3, 4):
        return False, f"Top-level 'exit_code' value {raw_exit_code} is outside valid range [0..4]", data

    # 4. Strict agreement between process CLI RC and assessment exit_code
    if raw_exit_code != cli_rc:
        return (
            False,
            f"Contradiction: CLI process return code ({cli_rc}) != assessment JSON exit_code ({raw_exit_code})",
            data,
        )

    # 5. Validate dimensions
    dimensions = data["dimensions"]
    for dim_name in REQUIRED_DIMENSIONS:
        if dim_name not in dimensions or not isinstance(dimensions[dim_name], dict):
            return False, f"Missing or non-object required dimension '{dim_name}'", data

    # 5a. evidence_integrity
    ev_int = dimensions["evidence_integrity"]
    if "status" not in ev_int or ev_int["status"] not in ALLOWED_INTEGRITY_STATUSES:
        return False, f"Dimension 'evidence_integrity' has invalid or missing status: {ev_int.get('status')}", data

    # 5b. ci_execution
    ci_exec = dimensions["ci_execution"]
    if "status" not in ci_exec or ci_exec["status"] not in ALLOWED_CI_STATUSES:
        return False, f"Dimension 'ci_execution' has invalid or missing status: {ci_exec.get('status')}", data

    # 5c. evidence_availability
    ev_avail = dimensions["evidence_availability"]
    if "status" not in ev_avail or ev_avail["status"] not in ALLOWED_AVAILABILITY_STATUSES:
        return False, f"Dimension 'evidence_availability' has invalid or missing status: {ev_avail.get('status')}", data

    # 5d. review_status
    rev = dimensions["review_status"]
    if "status" not in rev or rev["status"] not in ALLOWED_REVIEW_STATUSES:
        return False, f"Dimension 'review_status' has invalid or missing status: {rev.get('status')}", data
    if "tier" not in rev or rev["tier"] not in ALLOWED_REVIEW_TIERS:
        return False, f"Dimension 'review_status' has invalid or missing tier: {rev.get('tier')}", data

    # 5e. authorization
    auth = dimensions["authorization"]
    if "status" not in auth or auth["status"] not in ALLOWED_AUTH_STATUSES:
        return False, f"Dimension 'authorization' has invalid or missing status: {auth.get('status')}", data

    # 5f. freshness
    fresh = dimensions["freshness"]
    if "status" not in fresh or fresh["status"] not in ALLOWED_FRESHNESS_STATUSES:
        return False, f"Dimension 'freshness' has invalid or missing status: {fresh.get('status')}", data

    # 6. Validate derived_summary
    derived = data["derived_summary"]
    raw_verdict = derived.get("verdict")
    if not isinstance(raw_verdict, str) or raw_verdict not in ALLOWED_VERDICTS:
        return False, f"Derived summary has invalid or missing verdict: '{raw_verdict}'", data
    if not isinstance(derived.get("blocking_reasons"), list):
        return False, "Derived summary 'blocking_reasons' must be an array", data
    if not isinstance(derived.get("warnings"), list):
        return False, "Derived summary 'warnings' must be an array", data

    # 7. Semantic compatibility between verdict and exit_code
    if raw_verdict == "ASSESSED_PROVISIONAL_PASS" and raw_exit_code != 0:
        return False, f"Verdict ASSESSED_PROVISIONAL_PASS is incompatible with exit_code {raw_exit_code}", data
    if raw_verdict == "ASSESSED_LIMITATIONS" and raw_exit_code != 1:
        return False, f"Verdict ASSESSED_LIMITATIONS is incompatible with exit_code {raw_exit_code}", data
    if raw_verdict == "ASSESSED_FAIL" and raw_exit_code != 2:
        return False, f"Verdict ASSESSED_FAIL is incompatible with exit_code {raw_exit_code}", data
    if raw_verdict in ("INSUFFICIENT_EVIDENCE", "REFUSED") and raw_exit_code not in (2, 3, 4):
        return False, f"Verdict {raw_verdict} is incompatible with exit_code {raw_exit_code}", data

    return True, "", data


def parse_and_evaluate_gate(
    assessment_input,
    cli_rc,
    gate_mode="strict",
    independent_auth_verified=False,
):
    """
    Parses and evaluates an assessment string or dict against gate policy.

    Parameters:
    - assessment_input: JSON string or parsed dict
    - cli_rc: integer process exit code from CLI execution
    - gate_mode: "strict" or "observation"
    - independent_auth_verified: boolean indicating if an independently authenticated
      review and owner authorization trust path exists. Within GOV-03 offline CI boundary,
      this is strictly False.

    Returns dict containing all derived outputs, acceptance flags, and gate decisions.
    """
    # Normalize cli_rc
    try:
        cli_rc = int(cli_rc)
    except (ValueError, TypeError):
        cli_rc = 4

    data = None
    parse_error = None

    if isinstance(assessment_input, dict):
        data = assessment_input
    elif isinstance(assessment_input, str):
        if not assessment_input.strip():
            parse_error = "Assessment output is empty"
        else:
            try:
                data = json.loads(assessment_input)
            except Exception as e:
                parse_error = f"Failed to parse assessment JSON: {e}"
    else:
        parse_error = f"Unsupported assessment_input type: {type(assessment_input).__name__}"

    if parse_error is not None:
        is_valid = False
        error_msg = parse_error
        data = {}
    else:
        is_valid, error_msg, data = validate_assessment_structure(data, cli_rc)

    # Extract verified or fallback values
    derived = data.get("derived_summary", {}) if isinstance(data, dict) else {}
    dimensions = data.get("dimensions", {}) if isinstance(data, dict) else {}

    if is_valid:
        verdict = derived.get("verdict")
        exit_code = data.get("exit_code")
        contradiction = False
    else:
        verdict = "REFUSED" if cli_rc != 0 else "OPERATIONAL_ERROR"
        exit_code = cli_rc
        contradiction = (cli_rc != 0 and data.get("exit_code") == 0)

    ci_exec = dimensions.get("ci_execution", {}) if isinstance(dimensions, dict) else {}
    ci_status = ci_exec.get("status", "UNVERIFIED_RUN")

    # Checkpoint Acceptance Determination:
    # Full checkpoint acceptance requires:
    # 1. Structural and semantic assessment validity (is_valid == True)
    # 2. No contradiction between process RC and assessment exit_code
    # 3. Process cli_rc == 0 and exit_code == 0
    # 4. Engine verdict == ASSESSED_PROVISIONAL_PASS
    # 5. CI execution status == VERIFIED_PASS
    # 6. Supported independent review and owner authorization trust path.
    # Within GOV-03 offline CI gate scope, author-supplied review/authorization
    # claims cannot be independently authenticated (independent_auth_verified is False).
    # Therefore, checkpoint_accepted remains False.
    checkpoint_accepted = False
    if (
        is_valid
        and not contradiction
        and cli_rc == 0
        and exit_code == 0
        and verdict == "ASSESSED_PROVISIONAL_PASS"
        and ci_status == "VERIFIED_PASS"
        and independent_auth_verified
    ):
        checkpoint_accepted = True

    # Gate Decision Enforcement:
    # - Strict mode: requires genuine full checkpoint acceptance.
    # - Observation mode: succeeds if verifier executed without operational error (cli_rc != 4)
    #   and emitted a structurally valid, non-contradictory assessment.
    if gate_mode == "strict":
        gate_success = checkpoint_accepted
    elif gate_mode == "observation":
        gate_success = (cli_rc != 4 and is_valid and not contradiction)
    else:
        gate_success = False

    return {
        "is_valid_assessment": is_valid,
        "parse_error": error_msg if not is_valid else None,
        "verdict": verdict,
        "exit_code": exit_code,
        "cli_rc": cli_rc,
        "contradiction": contradiction,
        "ci_status": ci_status,
        "checkpoint_accepted": checkpoint_accepted,
        "gate_success": gate_success,
        "dimensions": dimensions,
        "derived_summary": derived,
        "raw_data": data,
    }


def sanitize_cell(val):
    """Sanitizes text for safe inclusion inside Markdown table cells."""
    s = str(val) if val is not None else ""
    s = html.escape(s)
    return s.replace("|", "&#124;").replace("\n", " ").replace("\r", "")


def sanitize_block(val):
    """Sanitizes free-text blocks for Markdown inclusion."""
    s = str(val) if val is not None else ""
    return html.escape(s).replace("\r", "")


def format_step_summary(res, target_info):
    """
    Builds the structured six-dimensional Markdown summary.
    """
    target_info = target_info or {}
    gate_mode = target_info.get("gate_mode", "strict")
    target_repo_id = target_info.get("target_repo_id", "")
    target_owner = target_info.get("target_owner", "")
    target_repo = target_info.get("target_repo", "")
    target_commit = target_info.get("target_commit", "")
    target_run_id = target_info.get("target_run_id", "")
    target_attempt = target_info.get("target_attempt", "1")
    policy_selection = target_info.get("policy_selection", "")
    verifier_sha = target_info.get("verifier_sha", "unknown")
    parser_sha = target_info.get("gate_parser_sha", "unknown")

    verdict = res["verdict"]
    exit_code = res["exit_code"]
    cli_rc = res["cli_rc"]
    accepted = "true" if res["checkpoint_accepted"] else "false"
    ci_status = res["ci_status"]
    dimensions = res["dimensions"]
    derived = res["derived_summary"]

    ci_exec = dimensions.get("ci_execution", {}) if isinstance(dimensions, dict) else {}
    observed_jobs = ci_exec.get("observed_jobs_count", 0)
    expected_jobs = ci_exec.get("expected_jobs_count", 0)

    summary = []
    summary.append("## Evidence Checkpoint Engine — Verification Summary\n")
    summary.append("| Parameter | Value |")
    summary.append("| :--- | :--- |")
    summary.append(f"| **Gate Mode** | `{sanitize_cell(gate_mode)}` |")
    summary.append(f"| **Checkpoint Accepted** | `{sanitize_cell(accepted)}` |")
    summary.append(f"| **Engine Verdict** | **`{sanitize_cell(verdict)}`** (Exit Code `{sanitize_cell(exit_code)}`, Process `{sanitize_cell(cli_rc)}`) |")
    summary.append(f"| **Target Repository** | `{sanitize_cell(target_owner)}/{sanitize_cell(target_repo)}` (ID: `{sanitize_cell(target_repo_id)}`) |")
    summary.append(f"| **Target Commit SHA** | `{sanitize_cell(target_commit)}` |")
    summary.append(f"| **Target Workflow Run** | Run `{sanitize_cell(target_run_id)}` (Attempt `{sanitize_cell(target_attempt)}`) |")
    summary.append(f"| **Trusted Check Policy** | `{sanitize_cell(policy_selection)}` |")
    summary.append(f"| **ECE Verifier Source** | `{sanitize_cell(verifier_sha)}` |")
    summary.append(f"| **ECE Gate Parser Source** | `{sanitize_cell(parser_sha)}` |\n")

    summary.append("### Six-Dimensional Trust Status\n")
    summary.append("| Dimension | Status | Key Information |")
    summary.append("| :--- | :--- | :--- |")

    # 1. Evidence Integrity
    ev_int = dimensions.get("evidence_integrity", {}) if isinstance(dimensions, dict) else {}
    status_int = ev_int.get("status", "NOT_EVALUATED")
    details_int = ev_int.get("details", "")
    summary.append(f"| **Evidence Integrity** | `{sanitize_cell(status_int)}` | {sanitize_cell(details_int)} |")

    # 2. CI Execution
    summary.append(f"| **CI Execution** | `{sanitize_cell(ci_status)}` | {sanitize_cell(observed_jobs)}/{sanitize_cell(expected_jobs)} policy jobs observed completed with success |")

    # 3. Evidence Availability
    ev_avail = dimensions.get("evidence_availability", {}) if isinstance(dimensions, dict) else {}
    status_avail = ev_avail.get("status", "NOT_PROVIDED")
    summary.append(f"| **Evidence Availability** | `{sanitize_cell(status_avail)}` | Provider artifacts verification |")

    # 4. Independent Review
    rev = dimensions.get("review_status", {}) if isinstance(dimensions, dict) else {}
    status_rev = rev.get("status", "UNREVIEWED")
    tier_rev = rev.get("tier", "NONE")
    summary.append(f"| **Independent Review** | `{sanitize_cell(status_rev)}` | Tier: `{sanitize_cell(tier_rev)}` |")

    # 5. Human Authorization
    auth = dimensions.get("authorization", {}) if isinstance(dimensions, dict) else {}
    status_auth = auth.get("status", "AUTHORIZATION_PENDING")
    summary.append(f"| **Human Authorization** | `{sanitize_cell(status_auth)}` | Owner decision pending |")

    # 6. Freshness
    fresh = dimensions.get("freshness", {}) if isinstance(dimensions, dict) else {}
    status_fresh = fresh.get("status", "HISTORICAL_BASELINE")
    summary.append(f"| **Freshness** | `{sanitize_cell(status_fresh)}` | Head SHA unpinned |")

    blocking = derived.get("blocking_reasons", []) if isinstance(derived, dict) else []
    warnings = derived.get("warnings", []) if isinstance(derived, dict) else []

    if blocking and isinstance(blocking, list):
        summary.append("\n#### Blocking Reasons")
        for b in blocking:
            summary.append(f"- ⚠️ {sanitize_block(b)}")

    if warnings and isinstance(warnings, list):
        summary.append("\n#### Warnings & Invariants")
        for w in warnings:
            summary.append(f"- ℹ️ {sanitize_block(w)}")

    if not res["is_valid_assessment"] and res.get("parse_error"):
        summary.append("\n#### Validation Failure")
        summary.append(f"- 🛑 `{sanitize_block(res['parse_error'])}`")

    return "\n".join(summary) + "\n"


def run_tests():
    """
    Executes the focused regression suite directly against parse_and_evaluate_gate.
    """
    print("=" * 64)
    print("Executing ECE Gate & Assessment Parser Test Suite (GOV-03 R2)")
    print("=" * 64)

    # Base valid assessment fixture for testing
    def make_valid_fixture(verdict="ASSESSED_PROVISIONAL_PASS", exit_code=0):
        return {
            "schema_version": "0.1.0-draft",
            "assessment_id": "test_assessment_12345",
            "evaluator_auth": {
                "execution_context": "LOCAL_SUBPROCESS_PIPE",
                "is_synthetic_mock": False,
            },
            "engine": {
                "name": "Evidence-Checkpoint-Engine",
                "version": "0.1.0",
                "canonicalization_standard": "RFC8785",
            },
            "evaluation_timestamp": "2026-10-30T12:00:00Z",
            "evaluated_target": {
                "repository_id": "1432704795",
                "repository_owner": "ErenTahiroglu",
                "repository_name": "Evidence-Checkpoint-Engine",
                "target_commit_sha": "415990746d5bbd3756f743b4b5a7673b36228567",
                "evaluated_checkpoint_id": "cp_01",
            },
            "dimensions": {
                "evidence_integrity": {
                    "status": "VALID",
                    "digest_match": True,
                    "details": "All files match sha256 digests",
                },
                "ci_execution": {
                    "status": "VERIFIED_PASS",
                    "expected_jobs_count": 4,
                    "observed_jobs_count": 4,
                    "unresolved_jobs": [],
                },
                "evidence_availability": {
                    "status": "AVAILABLE",
                    "expired_references": [],
                },
                "review_status": {
                    "status": "UNREVIEWED",
                    "tier": "NONE",
                    "scope": "UNREVIEWED_BASELINE",
                },
                "authorization": {
                    "status": "AUTHORIZATION_PENDING",
                    "decision_by": None,
                },
                "freshness": {
                    "status": "HISTORICAL_BASELINE",
                    "live_head_sha": None,
                },
            },
            "derived_summary": {
                "verdict": verdict,
                "blocking_reasons": [] if verdict == "ASSESSED_PROVISIONAL_PASS" else ["Limitations noted"],
                "warnings": [],
            },
            "claims_breakdown": {
                "source_declared": {},
                "machine_assessed": {},
                "independently_reviewed": {},
            },
            "checks_performed": ["ci_jobs", "integrity"],
            "checks_omitted": [],
            "exit_code": exit_code,
        }

    tests_run = 0

    # Test 1: Missing exit_code with actual process exit 0
    fixture1 = make_valid_fixture()
    del fixture1["exit_code"]
    res1_strict = parse_and_evaluate_gate(fixture1, cli_rc=0, gate_mode="strict")
    res1_obs = parse_and_evaluate_gate(fixture1, cli_rc=0, gate_mode="observation")
    assert not res1_strict["is_valid_assessment"], "Missing exit_code must invalidate assessment"
    assert not res1_strict["checkpoint_accepted"], "Missing exit_code must not be accepted"
    assert not res1_strict["gate_success"], "Strict gate must fail on missing exit_code"
    assert not res1_obs["gate_success"], "Observation mode must fail on missing exit_code"
    tests_run += 1
    print("Test 1 Passed: Missing exit_code with actual process exit 0 rejected")

    # Test 2: JSON boolean exit_code (e.g. exit_code: false or true)
    fixture2 = make_valid_fixture()
    fixture2["exit_code"] = False
    res2 = parse_and_evaluate_gate(fixture2, cli_rc=0, gate_mode="strict")
    assert not res2["is_valid_assessment"], "Boolean exit_code must be rejected"
    assert not res2["checkpoint_accepted"], "Boolean exit_code must not be accepted"
    assert not res2["gate_success"], "Strict gate must fail on boolean exit_code"
    tests_run += 1
    print("Test 2 Passed: JSON boolean exit_code rejected (isinstance bool handled correctly)")

    # Test 3: Incomplete assessment dimensions (missing review_status & authorization)
    fixture3 = make_valid_fixture()
    del fixture3["dimensions"]["review_status"]
    del fixture3["dimensions"]["authorization"]
    res3 = parse_and_evaluate_gate(fixture3, cli_rc=0, gate_mode="strict")
    assert not res3["is_valid_assessment"], "Incomplete dimensions must invalidate assessment"
    assert not res3["checkpoint_accepted"], "Incomplete dimensions must not be accepted"
    assert not res3["gate_success"], "Gate must fail on incomplete dimensions"
    tests_run += 1
    print("Test 3 Passed: Incomplete assessment dimensions rejected")

    # Test 4: Malformed JSON string
    res4_strict = parse_and_evaluate_gate("{ not valid json at all", cli_rc=0, gate_mode="strict")
    res4_obs = parse_and_evaluate_gate("{ not valid json at all", cli_rc=0, gate_mode="observation")
    assert not res4_strict["is_valid_assessment"], "Malformed JSON must invalidate assessment"
    assert not res4_strict["checkpoint_accepted"], "Malformed JSON must not be accepted"
    assert not res4_strict["gate_success"], "Strict gate must fail on malformed JSON"
    assert not res4_obs["gate_success"], "Observation mode must fail on malformed JSON"
    tests_run += 1
    print("Test 4 Passed: Malformed JSON rejected")

    # Test 5: Actual process exit 4 with JSON claiming exit 0
    fixture5 = make_valid_fixture(verdict="ASSESSED_PROVISIONAL_PASS", exit_code=0)
    res5_strict = parse_and_evaluate_gate(fixture5, cli_rc=4, gate_mode="strict")
    res5_obs = parse_and_evaluate_gate(fixture5, cli_rc=4, gate_mode="observation")
    assert res5_strict["contradiction"], "Exit code contradiction must be flagged"
    assert not res5_strict["is_valid_assessment"], "Contradiction must invalidate assessment"
    assert not res5_strict["checkpoint_accepted"], "Contradictory exit must not be accepted"
    assert not res5_strict["gate_success"], "Strict gate must fail on CLI 4 vs JSON 0"
    assert not res5_obs["gate_success"], "Observation mode must fail on CLI 4 operational error"
    tests_run += 1
    print("Test 5 Passed: Actual process exit 4 with JSON exit 0 flagged as contradiction and rejected")

    # Test 6: Unsupported verdict string (e.g. 'VERIFIED' or 'CUSTOM_VERDICT')
    fixture6 = make_valid_fixture()
    fixture6["derived_summary"]["verdict"] = "VERIFIED"
    res6 = parse_and_evaluate_gate(fixture6, cli_rc=0, gate_mode="strict")
    assert not res6["is_valid_assessment"], "Unsupported verdict 'VERIFIED' must be rejected"
    assert not res6["checkpoint_accepted"], "Unsupported verdict must not be accepted"
    assert not res6["gate_success"], "Strict gate must fail on unsupported verdict"
    tests_run += 1
    print("Test 6 Passed: Unsupported verdict ('VERIFIED') rejected")

    # Test 7: Provisional pass lacking independently authenticated review/owner authorization
    # Under GOV-03 offline CI boundary, independent_auth_verified is False.
    # Therefore, checkpoint_accepted MUST be False and strict gate MUST fail.
    fixture7 = make_valid_fixture(verdict="ASSESSED_PROVISIONAL_PASS", exit_code=0)
    fixture7["dimensions"]["review_status"]["status"] = "INDEPENDENT_REVIEW_PASS"
    fixture7["dimensions"]["authorization"]["status"] = "EXPLICITLY_AUTHORIZED"
    res7_strict = parse_and_evaluate_gate(fixture7, cli_rc=0, gate_mode="strict", independent_auth_verified=False)
    res7_obs = parse_and_evaluate_gate(fixture7, cli_rc=0, gate_mode="observation", independent_auth_verified=False)
    assert res7_strict["is_valid_assessment"], "Structurally complete assessment is valid"
    assert not res7_strict["checkpoint_accepted"], "Provisional assessment without external trust path cannot be accepted"
    assert not res7_strict["gate_success"], "Strict gate must fail closed without independent authorization path"
    assert res7_obs["gate_success"], "Observation mode succeeds in observing and reporting valid provisional pass"
    tests_run += 1
    print("Test 7 Passed: Provisional pass lacking independently authenticated review/authorization fails strict gate")

    # Test 8: Historical ASSESSED_LIMITATIONS, exit 1, in both observation and strict modes
    fixture8 = make_valid_fixture(verdict="ASSESSED_LIMITATIONS", exit_code=1)
    res8_strict = parse_and_evaluate_gate(fixture8, cli_rc=1, gate_mode="strict")
    res8_obs = parse_and_evaluate_gate(fixture8, cli_rc=1, gate_mode="observation")
    assert res8_strict["is_valid_assessment"], "Historical pilot assessment is valid"
    assert not res8_strict["checkpoint_accepted"], "Historical exit 1 must not be accepted"
    assert not res8_strict["gate_success"], "Strict gate must fail closed on historical exit 1"
    assert res8_obs["gate_success"], "Observation mode must succeed on valid historical exit 1"
    assert res8_obs["exit_code"] == 1, "Exit code must be 1"
    assert res8_obs["verdict"] == "ASSESSED_LIMITATIONS", "Verdict must be ASSESSED_LIMITATIONS"
    tests_run += 1
    print("Test 8 Passed: Historical ASSESSED_LIMITATIONS exit 1 rejected in strict and reported in observation")

    # Test 9: CLI exit 1 vs JSON exit 0 contradiction
    fixture9 = make_valid_fixture(verdict="ASSESSED_PROVISIONAL_PASS", exit_code=0)
    res9 = parse_and_evaluate_gate(fixture9, cli_rc=1, gate_mode="strict")
    assert res9["contradiction"], "Contradiction must be flagged"
    assert not res9["is_valid_assessment"], "Contradiction must invalidate assessment"
    assert not res9["checkpoint_accepted"], "Contradiction must not be accepted"
    assert not res9["gate_success"], "Strict gate must fail on contradiction"
    tests_run += 1
    print("Test 9 Passed: CLI exit 1 vs JSON exit 0 flagged as contradiction and rejected")

    # Test 10: Missing top-level key (e.g. missing evaluator_auth)
    fixture10 = make_valid_fixture()
    del fixture10["evaluator_auth"]
    res10 = parse_and_evaluate_gate(fixture10, cli_rc=0, gate_mode="strict")
    assert not res10["is_valid_assessment"], "Missing evaluator_auth must invalidate assessment"
    tests_run += 1
    print("Test 10 Passed: Missing required top-level key rejected")

    # Test 11: Incompatible verdict / exit_code pair (ASSESSED_FAIL with exit_code 0)
    fixture11 = make_valid_fixture(verdict="ASSESSED_FAIL", exit_code=0)
    res11 = parse_and_evaluate_gate(fixture11, cli_rc=0, gate_mode="strict")
    assert not res11["is_valid_assessment"], "Incompatible verdict and exit_code must invalidate assessment"
    tests_run += 1
    print("Test 11 Passed: Incompatible verdict and exit_code rejected")

    # Test 12: Markdown sanitization safety
    payload = "<script>alert(1)</script> | pipe \n newline"
    clean_cell = sanitize_cell(payload)
    clean_block = sanitize_block(payload)
    assert "<script>" not in clean_cell, "HTML must be escaped in cell"
    assert "&lt;script&gt;" in clean_cell, "HTML escape must be present in cell"
    assert "|" not in clean_cell, "Pipe must be converted in cell"
    assert "&#124;" in clean_cell, "Pipe escape entity must be present in cell"
    assert "\n" not in clean_cell, "Newlines must be stripped from table cell"
    assert "<script>" not in clean_block, "HTML must be escaped in block"
    tests_run += 1
    print("Test 12 Passed: Markdown table cell and text block sanitization verified")

    # Test 13: Structured summary formatting with verifier and gate parser SHAs
    summary_test = format_step_summary(
        res8_obs,
        {
            "gate_mode": "observation",
            "target_repo_id": "1432704795",
            "target_owner": "ErenTahiroglu",
            "target_repo": "Evidence-Checkpoint-Engine",
            "target_commit": "415990746d5bbd3756f743b4b5a7673b36228567",
            "target_run_id": "40969401836",
            "target_attempt": "1",
            "policy_selection": "ece-pilot-four-runner-matrix",
            "verifier_sha": "fbea48be57adfe8bbf1dbd75c8c425eb7cf9f86c",
            "gate_parser_sha": "63ed53f73d9ece39062272d8f61d4616f7f824e8",
        },
    )
    assert "| **ECE Verifier Source** | `fbea48be57adfe8bbf1dbd75c8c425eb7cf9f86c` |" in summary_test
    assert "| **ECE Gate Parser Source** | `63ed53f73d9ece39062272d8f61d4616f7f824e8` |" in summary_test
    tests_run += 1
    print("Test 13 Passed: Structured step summary formats both verifier and parser SHAs")

    print("=" * 64)
    print(f"All {tests_run}/{tests_run} ECE Gate & Assessment Parser Tests PASSED.")
    print("=" * 64)
    return 0


def main():
    if "--test" in sys.argv:
        return run_tests()

    cli_rc_str = os.environ.get("CLI_RC", "4")
    try:
        cli_rc = int(cli_rc_str)
    except (ValueError, TypeError):
        cli_rc = 4

    gate_mode = os.environ.get("GATE_MODE", "strict")
    assessment_file = os.environ.get("ASSESSMENT_FILE", "verifier_output/assessment.json")
    github_output = os.environ.get("GITHUB_OUTPUT", "")
    github_step_summary = os.environ.get("GITHUB_STEP_SUMMARY", "")

    target_info = {
        "gate_mode": gate_mode,
        "target_repo_id": os.environ.get("TARGET_REPO_ID", ""),
        "target_owner": os.environ.get("TARGET_OWNER", ""),
        "target_repo": os.environ.get("TARGET_REPO", ""),
        "target_commit": os.environ.get("TARGET_COMMIT_SHA", ""),
        "target_run_id": os.environ.get("TARGET_RUN_ID", ""),
        "target_attempt": os.environ.get("TARGET_RUN_ATTEMPT", "1"),
        "policy_selection": os.environ.get("POLICY_SELECTION", ""),
        "verifier_sha": os.environ.get("ECE_VERIFIER_SHA", "unknown"),
        "gate_parser_sha": os.environ.get("ECE_PARSER_SHA", "unknown"),
    }

    raw_json_str = ""
    if os.path.exists(assessment_file) and os.path.getsize(assessment_file) > 0:
        try:
            with open(assessment_file, "r", encoding="utf-8") as f:
                raw_json_str = f.read()
        except Exception as e:
            print(f"Error reading assessment file: {e}", file=sys.stderr)
            raw_json_str = ""
    else:
        print(f"Assessment file '{assessment_file}' missing or empty", file=sys.stderr)

    res = parse_and_evaluate_gate(
        raw_json_str,
        cli_rc=cli_rc,
        gate_mode=gate_mode,
        independent_auth_verified=False,
    )

    # Write Step Outputs
    if github_output:
        with open(github_output, "a", encoding="utf-8") as out:
            out.write(f"verdict={res['verdict']}\n")
            out.write(f"exit_code={res['exit_code']}\n")
            out.write(f"cli_rc={res['cli_rc']}\n")
            out.write(f"ci_status={res['ci_status']}\n")
            out.write(f"checkpoint_accepted={'true' if res['checkpoint_accepted'] else 'false'}\n")
            out.write(f"is_valid_assessment={'true' if res['is_valid_assessment'] else 'false'}\n")

    # Write GITHUB_STEP_SUMMARY
    if github_step_summary:
        summary_md = format_step_summary(res, target_info)
        with open(github_step_summary, "a", encoding="utf-8") as s:
            s.write(summary_md)

    if not res["is_valid_assessment"] and res.get("parse_error"):
        print(f"Assessment validation failed: {res['parse_error']}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
