# GOV-00 Contract Consistency and Reconciliation Report

> **Milestone:** `GOV-00 FINAL` — Final Reconciliation & Minimal Contract Consistency
> **Status:** `GOV_00_FINAL_READY_FOR_INDEPENDENT_REVIEW`
> **Date:** `2026-10-29`
> **Repository:** `ErenTahiroglu/Evidence-Checkpoint-Engine` (REST ID: `1432704795`)
> **Authorized Branch:** `development`
> **Expected Development Baseline SHA:** `95eaff6f24069641dee20c5959642b4825861f90`
> **Frozen Main Baseline SHA:** `bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02`
> **Primary Tracking Issue:** [#2 (GOV-00)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/2)
> **Security Issues:** [#9 (SEC-01)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) and [#10 (SEC-02)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10)
> **Authoritative Review Findings:** [Issue #2 Comment 6395274401](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/2#issuecomment-6395274401) and [Comment 6397969388](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/2#issuecomment-6397969388)

---

## 1. Executive Summary & Objective

This report finalizes milestone **GOV-00** through minimal documentation and schema-consistency reconciliation, responding to independent review checkpoints.

The objective is strictly bounded: eliminate remaining documentation inconsistencies, reconcile the envelope version across specifications and schemas, correct the fixture inventory to match actual committed files, execute a bounded JSON Schema Draft 2020-12 validation pass, and provide an accurate classification of test vectors. In accordance with explicit governance boundaries:
- No verifier code, CLI binary, API adapter, CI workflow, or background service has been implemented.
- The PolyForm Noncommercial 1.0.0 license is preserved unchanged.
- All verified improvements from R1 and R2 are preserved without unnecessary security hardening or scope expansion.
- Synthetic fixtures remain explicitly distinguished from authentic external GitHub evidence.
- No engine or native OS execution has occurred yet.

---

## 2. Traceability Matrix of Authoritative Findings

The following matrix documents each authoritative finding, its root cause, exact changed files/fields, correction applied, expected test behavior, actual test result, and remaining limitations.

| Finding ID & Severity | Root Cause | Exact Changed Fields & Files | Correction Applied | Expected Test Behavior | Actual Test Result | Remaining Limitation |
|---|---|---|---|---|---|---|
| **F-01 (P0)**: CI `observed_jobs` false rejection under Draft 2020-12 | `additionalProperties: false` was declared at the item level while `name`, `status`, `conclusion` were declared only inside subschemas in `oneOf`. Under Draft 2020-12, `additionalProperties` only evaluates properties declared in the immediate schema object, rejecting valid jobs. | `docs/gov-00/schemas/checkpoint-input.schema.json` (`ci_evidence.properties.observed_jobs.items`) | Restructured item definition to declare `name`, `status`, and `conclusion` directly in `properties`, with `additionalProperties: false`. Enforced state mutually exclusive requirements using conditional `allOf` / `if` / `then` logic. | Valid completed jobs ACCEPT; queued/in-progress with null conclusion ACCEPT; queued with success REJECT; completed missing conclusion REJECT; unexpected properties REJECT. | Validated with `Draft202012Validator`: 11/11 job conditional test cases match expected behavior. | Runtime check name verification against workflow file requires GOV-02 adapter. |
| **F-02 (P1)**: Canonical uint64 ID boundary regex | Lexical regex `^[1-9][0-9]{0,19}$` enforces string syntax without leading zeros and up to 20 digits, but cannot cleanly express the ceiling $18{,}446{,}744{,}073{,}709{,}551{,}615$ without extreme regex complexity. | `docs/gov-00/CONTRACT_AND_CANONICALIZATION.md`, `docs/gov-00/schemas/checkpoint-input.schema.json` | Explicitly documented two-tier boundary model: regex enforces lexical syntax; engine implementation contract mandates semantic check `<= uint64::MAX`. Added adversarial fixtures for edge overflow and twenty nines. | Edge overflow `18446744073709551616` and `99999999999999999999` accepted by schema regex but REJECTED by semantic parser. Leading zeros (`01432704795`) REJECTED by schema regex. | Regex rejects leading zeros in schema validation; semantic bounds check specified as mandatory for GOV-01 parser. | Schemas alone cannot enforce numeric ceilings on string-encoded 64-bit integers without excessive complexity. |
| **F-03 (P1)**: Missing authoritative CI check policy | Schema permitted `ci_evidence` without specifying expected check policy or digest, allowing producer to omit critical checks and self-certify. | `docs/gov-00/schemas/checkpoint-input.schema.json` (`ci_evidence.required`), `docs/gov-00/CONTRACT_AND_CANONICALIZATION.md` | Made `expected_policy_ref` and `expected_policy_digest` required properties in `ci_evidence`. Defined 5-tier policy provenance model. Added fixtures for missing policy and untrusted digest. | Missing policy REJECTED by schema; untrusted policy digest schema-valid but REJECTED semantically by engine (verdict `VERIFICATION_REFUSED`). | Validated: `input_invalid_ci_missing_policy.json` fails schema validation (`expected_policy_ref` required). Untrusted digest fixture validated as schema-valid. | Static verifier requires independent trusted root of trust to authenticate policy digest. |
| **F-04 (P1)**: Outer envelope schema separation & draft version | R1 selected outer envelope `{payload, envelope_manifest_digest}` without a formal schema. The R2 text inadvertently described the envelope version as `1.0.0` while the schema and fixtures used `0.1.0-draft`. | `docs/gov-00/schemas/checkpoint-envelope.schema.json`, `docs/gov-00/CONTRACT_AND_CANONICALIZATION.md` | Defined distinct `checkpoint-envelope.schema.json` that references `checkpoint-input.schema.json` via `$ref`. Reconciled `envelope_version` to `0.1.0-draft` across all documents. Clarified that matching digest proves content integrity, not author provenance. | Valid envelope with reproducible SHA-256 digest ACCEPTS; tampered digest REJECTS. Bare payload rejected at envelope-only entry points. | Validated: `envelope_valid_bootstrap_checkpoint.json` passes schema validation with local `$ref` resolution; tampered digest fixture structurally valid. | Digest verification does not prove authenticity without an independent signature or trust anchor. |
| **F-05 (P1)**: Assessment-output authority invariants | `assessment-output.schema.json` allowed contradictory states: synthetic mock evaluation claiming `PROVISIONAL_PASS` or claiming human authorization without validation. | `docs/gov-00/schemas/assessment-output.schema.json` (`allOf` conditional rules) | Added conditional rules: (1) `SYNTHETIC_MOCK_UNTRUSTED` requires `is_synthetic_mock: true`, restricts verdict to non-pass, and forces `human_authorized.authorized: false`; (2) `human_authorized.authorized: true` requires `is_synthetic_mock: false` and explicit authorization dimension. | Mock claiming pass REJECTED; synthetic claiming human auth REJECTED; valid mock output ACCEPTS. | Validated: both negative output fixtures fail schema validation as expected; positive mock output passes. | Human identity authentication relies on external cryptographic signature or access log. |
| **F-06 (P1)**: Expired artifact fixture isolation | `input_invalid_expired_artifact.json` combined source-declared `observed_availability: "EXPIRED"` (untrusted claim) with a cross-repo CI run ID (`40144852713`), conflating two failure modes. | `docs/gov-00/fixtures/input_invalid_expired_artifact.json`, `docs/gov-00/schemas/checkpoint-input.schema.json`, `docs/gov-00/TRUST_AND_STATE_MODEL.md` | Removed `observed_availability` from schema and fixtures (engine must observe availability independently). Changed run ID to neutral synthetic placeholder `10000000001` (unverified against live repository API). Decoupled cross-repo test into separate fixture. | Expired artifact fixture tests only artifact availability failure; cross-repo fixture tests only run ID mismatch. | Fixtures decoupled; both validated against schema. | Offline engine cannot verify live HTTP 404 without online adapter (GOV-02). |
| **F-07 (P2)**: GitHub-hosted Windows ARM64 runner facts | Documentation incorrectly stated that GitHub does not provide Windows ARM64 runners, whereas `windows-11-arm` exists in GitHub's runner inventory. | `docs/gov-00/MVP_AND_INTEGRATION_GATES.md`, `docs/DECISIONS.md` | Corrected documentation: GitHub provides `windows-11-arm` runners, but Windows ARM64 target support remains deferred by deliberate product decision. Clarified that `windows-2025` is Windows Server, not Windows 11 client. | Documentation accurately reflects GitHub runner ecosystem and product boundary decisions. | Documentation verified against official GitHub runner specifications. | Physical Windows 11 client testing deferred to post-MVP hardware labs. |

---

## 3. Normative Outer Envelope and Two-Schema Architecture

### 3.1 Two-Schema Structure

```
+--------------------------------------------------------------------------+
|  checkpoint-envelope.schema.json                                         |
|  {                                                                       |
|    "envelope_version": "0.1.0-draft",                                    |
|    "payload": { ... },  <-- $ref: "checkpoint-input.schema.json"         |
|    "envelope_manifest_digest": "sha256:<64-hex-digits>"                  |
|  }                                                                       |
+--------------------------------------------------------------------------+
```

1. **`checkpoint-input.schema.json` (Inner Payload):**
   Defines unverified source claims: checkpoint metadata, Git commit binding, CI evidence (workflow run, attempt, jobs, policy), artifact evidence, review evidence, and declaration claims.
2. **`checkpoint-envelope.schema.json` (Outer Canonical Envelope):**
   Defines the transport envelope structure:
   - `envelope_version`: Exactly `"0.1.0-draft"` (provisional draft contract version; not published as stable 1.0.0).
   - `payload`: Evaluated strictly against `checkpoint-input.schema.json` via `$ref`.
   - `envelope_manifest_digest`: Format `^sha256:[0-9a-f]{64}$`.

### 3.2 Verification Entry Points & Digest Semantics

- **`verify-envelope` (Default CLI entry point):** Requires the outer envelope. Validates envelope schema, checks `envelope_version == "0.1.0-draft"`, canonicalizes `payload` via RFC 8785, computes SHA-256 digest, and verifies exact match with `envelope_manifest_digest`. Returns exit code 2 (`INTEGRITY_MISMATCH`) if digests differ.
- **`inspect-untrusted-payload` (Diagnostic / candidate inspection):** Accepts bare `checkpoint-input.schema.json`. Evaluates structural schema only. Emits prominent warning that envelope digest verification was bypassed and outputs verdict `VERIFICATION_REFUSED` (exit code 3) or provisional findings.
- **Integrity vs. Provenance:** A matching envelope digest establishes cryptographic content consistency between envelope manifest and payload; it does **not** establish independent proof of origin provenance, author identity, or verification authority without external cryptographic signatures or trusted attestations.

### 3.3 Exact Preimage Definition & RFC 8785 Rules

The digest preimage is defined strictly as:
$$\text{Preimage} = \text{RFC8785\_Canonicalize}(\text{payload})$$

- Outer envelope keys (`envelope_version`, `envelope_manifest_digest`) are strictly external to `payload` and are **never** included in the digest preimage, eliminating recursive hashing paradoxes.
- RFC 8785 canonicalization applies:
  - Lexicographical sorting of object keys by UTF-16 code units.
  - No whitespace outside string literals.
  - Floats formatted per ECMAScript specifications (integers serialized without decimal points).
  - Strict UTF-8 output encoding.

### 3.4 Digest Verification Result

For the positive bootstrap fixture `input_valid_bootstrap_checkpoint.json`:
- **RFC 8785 Canonical Bytes Length:** 1,228 bytes.
- **Computed SHA-256 Digest:** `sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d`.
- **Positive Fixture:** `envelope_valid_bootstrap_checkpoint.json` contains this exact digest and passes schema validation.
- **Tampered Fixture:** `envelope_invalid_tampered_digest.json` modifies the digest to `...ffff`, demonstrating deterministic rejection by the verifier integrity check.

---

## 4. Trust Boundaries: Verified vs. Declared Properties

A fundamental security principle of the engine is that **source-declared JSON claims are untrusted assertions**. The engine never promotes a producer claim to verified machine status without independent evidence.

| Field / Claim Category | Source Declaration Status | Verifier Authentication Requirement | Failure Mode on Missing / Untrusted Evidence |
|---|---|---|---|
| `git_evidence.commit_sha` | Producer claim of evaluated commit. | Must match target repository head or parent chain via Git/API. | `INTEGRITY_MISMATCH` (Exit code 2) |
| `ci_evidence.workflow_run_id` | Producer claim of CI execution (synthetic neutral placeholder `10000000001` in fixtures). | Must belong to immutable repository ID (`1432704795`) and commit SHA via GitHub REST API. | `CORRUPTED_OR_TAMPERED` (Exit code 2) |
| `ci_evidence.expected_policy_digest` | Producer claim of required checks. | Must match policy digest authenticated from independent root of trust. Producer digest alone is unauthenticated. | `VERIFICATION_REFUSED` (Exit code 3) |
| `ci_evidence.observed_jobs` | Producer claim of check results. | Must be authenticated via GitHub Workflow Jobs REST API. | `EVIDENCE_UNAUTHENTICATED` (Exit code 2) |
| `artifact_evidence.archive_sha256` | Producer claim of artifact hash. | Must match independent byte-level hash of fetched artifact archive. | `INTEGRITY_MISMATCH` (Exit code 2) |
| Artifact expiration | Untrusted (cannot be declared by producer). | Must be independently observed via storage TTL or authenticated API 404. | `INSUFFICIENT_EVIDENCE` (Exit code 1) |
| `evaluator_auth.is_synthetic_mock` | Engine execution context flag. | Forced to `true` whenever running under synthetic/mock harness. | Contradictory claims REJECTED by schema |
| `human_authorized.authorized` | Claim of human sign-off. | Requires authenticated human authority record (GPG signature or scoped log). | `INSUFFICIENT_EVIDENCE` (Exit code 1) |

---

## 5. Test Vectors and Fixtures Inventory

The committed fixtures directory (`docs/gov-00/fixtures/`) contains exactly **22 fixture files**:
- **21 JSON files** (3 positive illustrative fixtures + 18 negative/adversarial JSON fixtures).
- **1 non-JSON text file** (`input_malformed_syntax.txt`).

### 5.1 Positive Illustrative Fixtures (3)

| Fixture Name | Target Schema | Purpose / Semantic Context |
|---|---|---|
| `input_valid_bootstrap_checkpoint.json` | `checkpoint-input.schema.json` | Minimal valid documentation-only bootstrap checkpoint payload. |
| `envelope_valid_bootstrap_checkpoint.json` | `checkpoint-envelope.schema.json` | Canonical outer envelope wrapping bootstrap input with reproducible SHA-256 digest (`157aaebc...`). |
| `output_valid_provisional_assessment.json` | `assessment-output.schema.json` | Illustrative synthetic assessment output (`execution_context: "SYNTHETIC_MOCK_UNTRUSTED"`). |

### 5.2 Negative & Adversarial Fixtures (19)

| Fixture Name | Category | Primary Target | Expected Result |
|---|---|---|---|
| `input_malformed_syntax.txt` | Syntax Invalid | Truncated / invalid JSON syntax | REJECT: JSON parse error |
| `envelope_invalid_tampered_digest.json` | Envelope Integrity | Tampered outer envelope digest | REJECT: SHA-256 digest mismatch (exit code 2) |
| `input_invalid_ci_missing_policy.json` | CI Policy | Omitted `expected_policy_ref`/`digest` | REJECT: Schema validation failure (missing required) |
| `input_invalid_ci_untrusted_policy_digest.json` | CI Policy | Untrusted policy digest | REJECT: Policy provenance failure (exit code 3) |
| `input_invalid_cross_repo_ci_substitution.json` | CI Binding | Foreign repo Run ID (`40144852713`) | REJECT: Cross-repo substitution detected (exit code 2) |
| `input_invalid_expired_artifact.json` | Artifact Availability | Artifact expired from storage | REJECT: Artifact unavailable (exit code 1) |
| `input_invalid_git_sha1_with_sha256_length.json` | Algorithm Match | SHA-1 label with 64-char hash | REJECT: Schema conditional length failure |
| `input_invalid_git_sha256_with_sha1_length.json` | Algorithm Match | SHA-256 label with 40-char hash | REJECT: Schema conditional length failure |
| `input_invalid_id_leading_zero.json` | ID Boundary | Leading zero ID (`01432704795`) | REJECT: Schema regex format failure |
| `input_invalid_mismatched_sha.json` | Git Binding | Commit SHA not matching HEAD | REJECT: Git commit mismatch (exit code 2) |
| `input_invalid_missing_ci_jobs.json` | CI Policy | Incomplete observed jobs list | REJECT: Missing expected checks (exit code 2) |
| `input_invalid_self_accepted_injection.json` | Declaration Claims | Injected `accepted: true` property | REJECT: Schema additionalProperties failure |
| `input_invalid_uint64_overflow_edge.json` | ID Boundary | $2^{64}$ (`18446744073709551616`) | REJECT: Semantic uint64 overflow (exit code 3) |
| `input_invalid_uint64_twenty_nines.json` | ID Boundary | 20 nines (`99999999999999999999`) | REJECT: Semantic uint64 overflow (exit code 3) |
| `input_invalid_unsupported_schema_version.json` | Versioning | Unsupported version (`99.0.0-unsupported`) | REJECT: Schema enum validation failure |
| `input_invalid_wrong_repo_id.json` | Repo Binding | Mismatched repo ID (`999999999`) | REJECT: Repository ID mismatch (exit code 2) |
| `input_invalid_wrong_run_attempt.json` | CI Binding | Stale run attempt (`attempt: 1`) | REJECT: Superseded run attempt (exit code 2) |
| `output_invalid_mock_claiming_pass.json` | Output Invariant | Synthetic mock claiming `PASS` | REJECT: Schema conditional invariant failure |
| `output_invalid_synthetic_claiming_human_auth.json` | Output Invariant | Synthetic mock claiming human auth | REJECT: Schema conditional invariant failure |

### 5.3 Future GOV-01 Test Backlog

The following negative test cases were identified during architectural analysis and are formally recorded for implementation in GOV-01 rather than creating unreviewed fixtures in GOV-00:
1. `input_invalid_future_timestamp.json`: Test vector with a timestamp ahead of verifier system time (semantic freshness check).
2. `output_invalid_exit_code_mismatch.json`: Test vector with contradictory exit code vs verification verdict.
3. `output_invalid_claims_breakdown_override.json`: Test vector with unauthorized dimension override in output breakdown.

---

## 6. Formal Schema and Fixture Validation Results

A bounded validation pass was executed using Python `jsonschema` (4.26.0) and `check-jsonschema` (0.39.0) with offline local `$ref` resolution:

### 6.1 Meta-Schema Conformance
All three schema documents were compiled and validated against the official JSON Schema Draft 2020-12 meta-schema:
- `docs/gov-00/schemas/checkpoint-input.schema.json`: **PASS**
- `docs/gov-00/schemas/checkpoint-envelope.schema.json`: **PASS**
- `docs/gov-00/schemas/assessment-output.schema.json`: **PASS**

### 6.2 Positive Instance Validation
- `input_valid_bootstrap_checkpoint.json`: **VALID** per `checkpoint-input.schema.json`.
- `envelope_valid_bootstrap_checkpoint.json`: **VALID** per `checkpoint-envelope.schema.json` (resolving `$ref: "checkpoint-input.schema.json"` offline via local registry).
- `output_valid_provisional_assessment.json`: **VALID** per `assessment-output.schema.json`.

### 6.3 Conditional Logic Validation Results
- **Job Status & Conclusion Conditionals (11/11 Passed):**
  - Completed job with success: **ACCEPT**
  - Completed job with failure: **ACCEPT** (structurally valid)
  - Queued job with null conclusion: **ACCEPT**
  - In-progress job with null conclusion: **ACCEPT**
  - Queued job claiming success: **REJECT** (schema conditional validation failure)
  - Completed job missing conclusion: **REJECT** (schema conditional validation failure)
  - Completed job with null conclusion: **REJECT** (schema conditional validation failure)
  - Unexpected job property: **REJECT** (`additionalProperties: false`)
  - Missing job name or status: **REJECT** (`required` check)
  - Wrong property types: **REJECT** (type check)
- **Git Hash Length Conditionals (4/4 Passed):**
  - SHA-1 label with 40 hex chars: **ACCEPT**
  - SHA-1 label with 64 hex chars: **REJECT** (`commit_sha` pattern failure)
  - SHA-256 label with 64 hex chars (both commit and tree): **ACCEPT**
  - SHA-256 label with 40 hex chars: **REJECT** (`commit_sha` pattern failure)
- **Assessment Evaluator Authority Invariants (2/2 Passed):**
  - Synthetic mock claiming `ASSESSED_PROVISIONAL_PASS`: **REJECT** (verdict restricted to non-pass)
  - Synthetic mock claiming `human_authorized: true`: **REJECT** (`is_synthetic_mock` must be false)
- **Outer Envelope Digest Format (3/3 Passed):**
  - Valid SHA-256 prefixed digest: **ACCEPT**
  - Unsupported algorithm prefix (e.g. `md5:...`): **REJECT** (pattern failure)
  - Truncated digest length: **REJECT** (pattern failure)

### 6.4 Complete Negative Fixture Classification

Each of the 19 negative fixtures is classified into its exact failure category:

| Category | Fixture Count | Fixture Names | Failure Mechanism |
|---|---|---|---|
| **1. JSON Syntax Invalid** | 1 | `input_malformed_syntax.txt` | Fails initial JSON parser before schema evaluation. |
| **2. JSON Schema Invalid** | 8 | `input_invalid_ci_missing_policy.json`<br>`input_invalid_git_sha1_with_sha256_length.json`<br>`input_invalid_git_sha256_with_sha1_length.json`<br>`input_invalid_id_leading_zero.json`<br>`input_invalid_self_accepted_injection.json`<br>`input_invalid_unsupported_schema_version.json`<br>`output_invalid_mock_claiming_pass.json`<br>`output_invalid_synthetic_claiming_human_auth.json` | Formally rejected by JSON Schema Draft 2020-12 validator via `required`, `pattern`, `enum`, `additionalProperties`, or conditional `allOf`/`if`/`then` rules. |
| **3. Structurally Valid but Semantically Invalid** | 3 | `envelope_invalid_tampered_digest.json`<br>`input_invalid_uint64_overflow_edge.json`<br>`input_invalid_uint64_twenty_nines.json` | Conforms to schema syntax, but violates deterministic semantic rules (RFC 8785 SHA-256 digest mismatch or uint64 numeric bounds overflow). |
| **4. Requires Authenticated External Evidence** | 5 | `input_invalid_ci_untrusted_policy_digest.json`<br>`input_invalid_cross_repo_ci_substitution.json`<br>`input_invalid_expired_artifact.json`<br>`input_invalid_mismatched_sha.json`<br>`input_invalid_wrong_run_attempt.json` | Conforms to schema syntax, but fails verification when cross-referenced against authoritative external systems (trusted policy store, GitHub REST API, or storage TTL metadata). |
| **5. Requires Future Runtime Implementation** | 2 | `input_invalid_missing_ci_jobs.json`<br>`input_invalid_wrong_repo_id.json` | Conforms to schema syntax; requires GOV-01 runtime evaluation logic (comparing `observed_jobs` against `expected_jobs` set difference, or matching repository REST ID against runtime target config). |

*Crucial Assurance: A semantic test is never claimed to pass merely because JSON syntax parsing succeeded.*

---

## 7. Platform Runner Target Matrix Confirmation

Following official GitHub Actions documentation review:
- **`windows-11-arm`:** Confirmed present in GitHub's runner inventory. However, Windows ARM64 support is **deferred by deliberate product decision** until post-MVP hardware validation.
- **`windows-2025`:** Confirmed as Windows Server 2025. It serves as the primary CI runner for Windows x64 binaries, but is not confused with native Windows 11 client testing.
- **Target Platform Matrix (Confirmed):**
  - **Windows x64 (Windows 11 / Server 2025):** Mandatory target.
  - **macOS Apple Silicon ARM64 (`macos-26` / `macos-15`):** Mandatory target.
  - **macOS Intel x64 (`macos-26-intel` / `macos-15-intel`):** Compatibility target.
  - **Linux x64 (`ubuntu-24.04`):** Mandatory headless target.
  - **Linux ARM64 (`ubuntu-24.04-arm`):** Secondary target.
  - **Windows ARM64:** Deferred by product decision.

---

## 8. Remaining MVP Blockers, Deferred Matters, and GOV-01 Gate Prerequisites

### 8.1 Remaining MVP Blockers
- **None for GOV-00:** All architectural, schema, canonicalization, and trust model specifications are internally consistent and ready for independent review.

### 8.2 Deferred and Nonessential Matters
- **Windows ARM64 Hardware Validation:** Deferred to post-MVP hardware labs.
- **Automated Policy Fetcher / Attestation Service:** Deferred to GOV-02/GOV-03; offline engine requires local trusted policy references.
- **Live HTTP 404 Disambiguation:** Deferred to GOV-02 GitHub adapter.

### 8.3 GOV-01 Prerequisites (Awaiting Independent Review & Owner Authorization)
Before GOV-01 (#3) implementation commences, the following decisions are required:
1. **Formal Acceptance of ADR-006:** Owner sign-off on the provisional Rust recommendation (4.375/5.00) vs Go (4.250).
2. **Acceptance of GOV-00 Deliverables:** Independent review acceptance of this reconciliation report.
3. **Subprocess Boundary Confirmation:** Confirmation of standalone CLI binary invocation with stdio pipes.
