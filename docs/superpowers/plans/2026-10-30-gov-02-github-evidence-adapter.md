# GOV-02 GitHub Read-Only Evidence Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a minimal, read-only Rust GitHub REST API adapter for `Evidence-Checkpoint-Engine` that binds immutable repository identity, commit SHA, workflow run ID, run attempt, exact attempt jobs against a trusted policy, and artifact metadata to distinguish provider observations from untrusted developer claims.

**Architecture:** Extend the `ece` crate with a new `github` module containing `models.rs` (typed GitHub API DTOs), `client.rs` (bounded read-only HTTP client with rate limit/error handling and streaming artifact digest verification), `policy.rs` (trusted check policy and evaluation), `evaluator.rs` (conservative evidence comparison and assessment generation), and CLI commands (`verify-github-evidence` and `assess-github-run`). Unit and mock tests run offline; a real GitHub API pilot runs via GitHub Actions CI against historical run `40969401836`.

**Tech Stack:** Rust 2021, `reqwest` (with `blocking`, `json`, `rustls-tls`), `serde`, `serde_json`, `sha2`, `clap`, `chrono`, `uuid`.

**Spec:** Issue #4; authorized in conversation by owner; requirements in `docs/gov-00/schemas/` and GOV-02 prompt.

## Global Constraints

- Repository: `ErenTahiroglu/Evidence-Checkpoint-Engine`, REST ID `1432704795`
- Branch: `development` only; `main` (SHA `bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02`) remains strictly frozen
- Offline verification (`verify-envelope`, `inspect-untrusted-payload`) must remain 100% offline with zero network access
- All acceptance-relevant testing MUST execute through GitHub Actions across 4 native OS runners (`ubuntu-24.04`, `macos-26`, `windows-2025`, `macos-26-intel`); no local Mac tests
- Read-only operations only: zero write scopes, no issue creation/commenting, no PR mutation
- Zero secret logging or token persistence; credentials must not appear in stdout, stderr, or committed files
- Bounded response parsing (max 5 MiB for API responses, max 10 MiB for artifact downloads)
- Exit codes: 0=clean pass (all dimensions satisfied), 1=limitations/warnings (e.g. CI verified but human authorization pending), 2=verification failure, 3=refused/insufficient evidence, 4=operational error
- Preserve all 77 existing GOV-01 unit and integration tests

## Review Focus

1. **Repository Identity Spoofing (RT-01):** Provider-returned numeric repository ID must match the expected REST ID (`1432704795`). A repository name match alone is rejected fail-closed.
2. **Commit / Head SHA Substitution (RT-01):** The requested commit SHA must exist in the expected repo and match the workflow run's `head_sha`.
3. **Run Attempt Confusion (RT-02):** Jobs must be fetched strictly from the specified run attempt endpoint (`/attempts/{attempt_number}/jobs`), never mixed across attempts.
4. **Job Conclusion Integrity & Policy Enforcement (RT-02):** Every job required by the trusted policy must be present, completed, and successful. Missing, skipped, cancelled, or failed jobs fail closed.
5. **No Elevation of Authority:** Provider-observed CI pass does NOT satisfy independent code review or human authorization. `authorization.status` remains `AUTHORIZATION_PENDING`.

---

## File Structure

```
ece/
  Cargo.toml               — Add reqwest dependency
  Cargo.lock               — Updated lockfile
  src/
    lib.rs                 — Re-export verifier and github modules
    main.rs                — Add verify-github-evidence and assess-github-run CLI commands
    verifier/              — Existing offline verifier (unchanged)
    github/
      mod.rs               — GitHub adapter public API and exports
      models.rs            — Typed GitHub API response DTOs
      client.rs            — GitHubProvider trait, HttpGitHubClient, MockGitHubProvider, error types
      policy.rs            — TrustedCheckPolicy, CheckPolicyEvaluation
      evaluator.rs         — Core evidence comparison & assessment generation
  tests/
    github_adapter_mock.rs — 16+ mock-provider and error-handling tests (offline)
    integration_negative.rs — Existing 16 negative tests preserved
    integration_positive.rs — Existing 13 positive tests preserved
    release_output_validation.rs — Existing 4 release validation tests preserved
.github/
  workflows/
    build-and-test.yml     — Preserve locked 4-OS build/clippy/test, add pilot CLI test step
```

---

## Tasks

### Task 1: Add `reqwest` dependency and establish `github/models.rs`

**Files:**
- Modify: `ece/Cargo.toml`
- Create: `ece/src/github/mod.rs`
- Create: `ece/src/github/models.rs`
- Modify: `ece/src/lib.rs`

- [ ] Add `reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }` to `ece/Cargo.toml`.
- [ ] Implement `models.rs` with `GitHubRepository`, `GitHubUser`, `GitHubCommit`, `GitHubCommitDetails`, `GitHubWorkflowRun`, `GitHubJobsResponse`, `GitHubJob`, `GitHubArtifactsResponse`, `GitHubArtifact`.
- [ ] Implement unit tests in `models.rs` verifying JSON deserialization of sample GitHub REST API responses.
- [ ] Expose `pub mod github;` in `lib.rs`.

### Task 2: Implement `github/policy.rs` (Trusted Check Policy)

**Files:**
- Create: `ece/src/github/policy.rs`
- Modify: `ece/src/github/mod.rs`

- [ ] Define `TrustedCheckPolicy` with `policy_id`, `policy_digest`, `required_jobs`, and `allow_extra_jobs`.
- [ ] Provide a default pilot policy constructor (`TrustedCheckPolicy::ece_pilot_policy()`) with required jobs: `["Linux x64", "macOS ARM64", "Windows x64", "macOS Intel x64"]`.
- [ ] Implement `evaluate_jobs(&self, observed_jobs: &[GitHubJob]) -> CheckPolicyEvaluation`.
- [ ] Enforce fail-closed rules: missing job -> `Missing`, skipped job -> `IncompleteSkipped`, failed job -> `Failed`, duplicate conflicting job -> `AmbiguousDuplicated`.
- [ ] Write comprehensive unit tests for `TrustedCheckPolicy` in `policy.rs`.

### Task 3: Implement `github/client.rs` (Bounded HTTP Client & Provider Trait)

**Files:**
- Create: `ece/src/github/client.rs`
- Modify: `ece/src/github/mod.rs`

- [ ] Define `GitHubClientError` with variants: `HttpError`, `RateLimited`, `NotFound`, `Unauthorized`, `Timeout`, `NetworkError`, `InvalidJson`, `SizeLimitExceeded`.
- [ ] Define `GitHubProvider` trait with methods: `get_repository`, `get_commit`, `get_workflow_run`, `get_attempt_jobs`, `get_artifacts`, `download_artifact_digest`.
- [ ] Implement `HttpGitHubClient`:
  - Enforces official base URL `https://api.github.com` (allows overriding for test servers).
  - Sends headers: `User-Agent: Evidence-Checkpoint-Engine-GOV-02`, `X-GitHub-Api-Version: 2022-11-28`, `Accept: application/vnd.github+json`.
  - Supports optional Bearer token from env (`GITHUB_TOKEN`, `GH_TOKEN`) or constructor.
  - Implements bounded reading (max 5 MiB for JSON responses).
  - Implements pagination for jobs and artifacts.
  - Implements bounded streaming SHA-256 computation for artifact archives (max 10 MiB) without extraction or writing to disk.
- [ ] Implement `MockGitHubProvider` for fast, offline testing of all API responses and error states.
- [ ] Write unit tests for client error classification and mock responses.

### Task 4: Implement `github/evaluator.rs` (Evidence Comparison & Assessment Builder)

**Files:**
- Create: `ece/src/github/evaluator.rs`
- Modify: `ece/src/github/mod.rs`

- [ ] Define input target struct `TargetEvidenceRequest` (owner, repo, expected_repo_id, commit_sha, run_id, run_attempt, expected_workflow_path, declared_artifacts).
- [ ] Implement `evaluate_github_evidence(provider: &dyn GitHubProvider, target: &TargetEvidenceRequest, policy: &TrustedCheckPolicy, schema_dir: &Path) -> Result<VerificationOutput, VerifierError>`.
- [ ] Implement check sequence:
  1. Fetch repository -> compare `repo.id` against `expected_repo_id` (numeric match).
  2. Fetch commit -> verify commit exists and SHA matches.
  3. Fetch workflow run -> verify `run.repository.id == repo.id`, `run.head_sha == commit_sha`, `run.run_attempt == expected_run_attempt`.
  4. Fetch attempt jobs -> evaluate against `policy`.
  5. Fetch artifacts -> check presence, size, expiration, and optional digest verification.
- [ ] Build 6-dimensional assessment compliant with `assessment-output.schema.json`:
  - `dimensions.ci_execution`: `VERIFIED_PASS`, `FAILED`, `INCOMPLETE_SKIPPED`, `ATTEMPT_MISMATCH`, or `UNVERIFIED_RUN`.
  - `dimensions.freshness`: `HISTORICAL_BASELINE`.
  - `dimensions.review_status`: `UNREVIEWED` (preserving review trust boundary).
  - `dimensions.authorization`: `AUTHORIZATION_PENDING` (`authorized: false`, preserving owner authorization boundary).
  - `derived_summary.verdict`: `ASSESSED_LIMITATIONS` (exit 1) on clean CI pass (due to pending review/authorization), `ASSESSED_FAIL` (exit 2) on CI/binding failure, `INSUFFICIENT_EVIDENCE` (exit 3) on API/rate-limit/not-found failure.
- [ ] Run through `validate_and_wrap` to guarantee fail-closed output schema validation.

### Task 5: Build Comprehensive Mock Negative Test Suite (`tests/github_adapter_mock.rs`)

**Files:**
- Create: `ece/tests/github_adapter_mock.rs`

- [ ] Test 1: Correct repository, commit, run, and attempt -> `VERIFIED_PASS` (exit 1 `ASSESSED_LIMITATIONS`).
- [ ] Test 2: Wrong repository ID -> fails closed (exit 2 `ASSESSED_FAIL`).
- [ ] Test 3: Wrong commit SHA -> fails closed (exit 2 `ASSESSED_FAIL`).
- [ ] Test 4: Wrong run attempt -> fails closed (exit 2 `ATTEMPT_MISMATCH`).
- [ ] Test 5: Missing required job -> fails closed (`MissingJobs`, exit 2).
- [ ] Test 6: Failed required job -> fails closed (`FAILED`, exit 2).
- [ ] Test 7: Skipped or cancelled required job -> fails closed (`INCOMPLETE_SKIPPED`, exit 2).
- [ ] Test 8: In-progress workflow run -> fails closed (`RunInProgress`, exit 2).
- [ ] Test 9: Untrusted / empty expected-check policy -> fails closed (`INSUFFICIENT_EVIDENCE`, exit 3).
- [ ] Test 10: Multi-page job pagination -> properly gathers all jobs across pages.
- [ ] Test 11: HTTP 403 / 429 Rate limited -> `INSUFFICIENT_EVIDENCE` (exit 3).
- [ ] Test 12: HTTP 404 Not Found -> `INSUFFICIENT_EVIDENCE` (exit 3).
- [ ] Test 13: HTTP 500 Internal Error -> `INSUFFICIENT_EVIDENCE` (exit 3).
- [ ] Test 14: Truncated / malformed JSON response -> `INSUFFICIENT_EVIDENCE` (exit 3).
- [ ] Test 15: Artifact not provided / expired -> `NOT_PROVIDED` / `EXPIRED`.
- [ ] Test 16: Invalid artifact digest -> fails closed (exit 2).

### Task 6: Add CLI Subcommands (`main.rs`)

**Files:**
- Modify: `ece/src/main.rs`

- [ ] Add `VerifyGithubEvidence` and `AssessGithubRun` subcommands to `Cli`.
- [ ] For `VerifyGithubEvidence`: reads input checkpoint envelope/payload (validates offline first), extracts repository and CI parameters, calls `evaluate_github_evidence`.
- [ ] For `AssessGithubRun`: directly takes command line parameters for owner, repo, expected_repo_id, commit_sha, run_id, run_attempt, policy.
- [ ] Handles output and errors consistently with GOV-01: stdout receives validated assessment JSON; stderr receives diagnostic info; exits with matching code.

### Task 7: Update CI Workflow and Add Real GitHub Pilot Step

**Files:**
- Modify: `.github/workflows/build-and-test.yml`

- [ ] Verify `cargo check --locked`, `cargo clippy --locked`, `cargo fmt --all -- --check`, `cargo test --locked`.
- [ ] Add step in `build-and-test.yml`: `Real GitHub API Pilot (Read-Only)`:
  - Invokes `./target/release/ece assess-github-run` with:
    - `--owner ErenTahiroglu`
    - `--repo Evidence-Checkpoint-Engine`
    - `--expected-repo-id 1432704795`
    - `--commit-sha 415990746d5bbd3756f743b4b5a7673b36228567`
    - `--run-id 40969401836`
    - `--run-attempt 1`
  - Passes `GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}` to avoid public rate-limiting in GitHub Actions runners.
  - Verifies exit code `1` (`ASSESSED_LIMITATIONS`), captures stdout assessment JSON, and checks that `ci_execution.status` is `VERIFIED_PASS`.

### Task 8: Push to `development`, Verify 4-OS CI, and Prepare GOV-02 Handoff

**Files:**
- Review git diff, verify commit message format: `feat(gov-02): add read-only GitHub evidence adapter`.
- Push to remote `origin/development`.
- Monitor GitHub Actions run across all 4 OS runners.
- Document exact results, compare pilot output against ground truth, and update Issue #4.
- Hand off with target classification `GOV_02_READY_FOR_INDEPENDENT_REDTEAM`.
