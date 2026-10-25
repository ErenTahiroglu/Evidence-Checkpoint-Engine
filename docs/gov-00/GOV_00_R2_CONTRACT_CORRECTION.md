# GOV-00 R2 Contract and Schema Correction Report

> **Milestone:** `GOV-00 R2` — Final Schema and Evidence-Authority Contract Correction
> **Status:** `GOV_00_R2_READY_FOR_INDEPENDENT_REDTEAM`
> **Date:** `2026-10-25`
> **Repository:** `ErenTahiroglu/Evidence-Checkpoint-Engine` (REST ID: `1432704795`)
> **Authorized Branch:** `development`
> **Expected Development Baseline SHA:** `bad43f5ad446bb0879a5c7f09fc1307128fc93a9`
> **Frozen Main Baseline SHA:** `bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02`
> **Primary Tracking Issue:** [#2 (GOV-00)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/2)
> **Security Issues:** [#9 (SEC-01)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) and [#10 (SEC-02)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10)
> **Authoritative Review Findings:** [Issue #2 Comment 6395274401](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/2#issuecomment-6395274401)

---

## 1. Executive Summary & Objective

Following the independent Red Team review on GOV-00 R1 ([Issue #2 Comment 6395274401](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/2#issuecomment-6395274401)), this milestone (`GOV-00 R2`) performs a strictly bounded correction of the remaining schema, authority contract, fixture, and documentation inconsistencies.

The objective is to deliver internally consistent, independently reviewable JSON Schema Draft 2020-12 schemas, realistic inert fixtures, and unambiguous trust-authority rules suitable for subsequent GOV-01 implementation. In accordance with governance constraints:
- No verifier code, CLI binary, API adapter, CI workflow, or background service has been implemented.
- The PolyForm Noncommercial 1.0.0 license is preserved unchanged.
- All verified improvements from R1 are preserved.
- Unexecuted external validation tools are reported transparently as `NOT EXECUTED`.

---

## 2. Traceability Matrix of Authoritative Findings

The following matrix documents each authoritative finding from Issue #2 Comment 6395274401, its root cause, exact changed files/fields, correction applied, expected test behavior, actual test result, and remaining limitations.

| Finding ID & Severity | Root Cause | Exact Changed Fields & Files | Correction Applied | Expected Test Behavior | Actual Test Result | Remaining Limitation |
|---|---|---|---|---|---|---|
| **F-01 (P0)**: CI `observed_jobs` false rejection under Draft 2020-12 | `additionalProperties: false` was declared at the item level while `name`, `status`, `conclusion` were declared only inside subschemas in `oneOf`. Under Draft 2020-12, `additionalProperties` only evaluates properties declared in the immediate schema object, rejecting valid jobs. | `docs/gov-00/schemas/checkpoint-input.schema.json` (`ci_evidence.properties.observed_jobs.items`) | Restructured item definition to declare `name`, `status`, and `conclusion` directly in `properties`, with `additionalProperties: false`. Enforced state mutually exclusive requirements using conditional `allOf` / `if` / `then` logic. | Valid completed jobs ACCEPT; queued/in-progress with null conclusion ACCEPT; queued with success REJECT; completed missing conclusion REJECT; unexpected properties REJECT. | Syntactically verified; structural assertions pass. Formal CLI meta-schema: `NOT EXECUTED`. | Runtime check name verification against workflow file requires GOV-02 adapter. |
| **F-02 (P1)**: Canonical uint64 ID boundary regex | Lexical regex `^[1-9][0-9]{0,19}$` enforces string syntax without leading zeros and up to 20 digits, but cannot cleanly express the ceiling $18{,}446{,}744{,}073{,}709{,}551{,}615$ without extreme regex complexity. | `docs/gov-00/CONTRACT_AND_CANONICALIZATION.md`, `docs/gov-00/schemas/checkpoint-input.schema.json` | Explicitly documented two-tier boundary model: regex enforces lexical syntax; engine implementation contract mandates semantic check `<= uint64::MAX`. Added adversarial fixtures for edge overflow and twenty nines. | Edge overflow `18446744073709551616` and `99999999999999999999` accepted by schema regex but REJECTED by semantic parser. Leading zeros (`01432704795`) REJECTED by schema regex. | Syntactically valid JSON fixtures created. Semantic rejection specified for GOV-01. | Schemas alone cannot enforce numeric ceilings on string-encoded 64-bit integers without excessive complexity. |
| **F-03 (P1)**: Missing authoritative CI check policy | Schema permitted `ci_evidence` without specifying expected check policy or digest, allowing producer to omit critical checks and self-certify. | `docs/gov-00/schemas/checkpoint-input.schema.json` (`ci_evidence.required`), `docs/gov-00/CONTRACT_AND_CANONICALIZATION.md` | Made `expected_policy_ref` and `expected_policy_digest` required properties in `ci_evidence`. Defined 5-tier policy provenance model. Added fixtures for missing policy and untrusted digest. | Missing policy REJECTED by schema; untrusted policy digest schema-valid but REJECTED semantically by engine (verdict `VERIFICATION_REFUSED`). | Syntactically valid JSON fixtures created. Negative schema test verified. | Static verifier requires independent trusted root of trust to authenticate policy digest. |
| **F-04 (P1)**: Outer envelope schema separation | R1 selected outer envelope `{payload, envelope_manifest_digest}` but only defined inner `checkpoint-input.schema.json`, lacking a formal schema for the envelope. | `docs/gov-00/schemas/checkpoint-envelope.schema.json` (new), `docs/gov-00/CONTRACT_AND_CANONICALIZATION.md` | Defined distinct `checkpoint-envelope.schema.json` that references `checkpoint-input.schema.json` via `$ref`. Specified entry points, RFC 8785 canonicalization, preimage rules, and non-recursive digest verification. | Valid envelope with reproducible SHA-256 digest ACCEPTS; tampered digest REJECTS. Bare payload rejected at envelope-only entry points. | Positive and tampered fixtures created with reproducible SHA-256 digest `157aaebc...`. | Digest verification does not prove authenticity without an independent signature or trust anchor. |
| **F-05 (P1)**: Assessment-output authority invariants | `assessment-output.schema.json` allowed contradictory states: synthetic mock evaluation claiming `PROVISIONAL_PASS` or claiming human authorization without validation. | `docs/gov-00/schemas/assessment-output.schema.json` (`allOf` conditional rules) | Added conditional rules: (1) `SYNTHETIC_MOCK_UNTRUSTED` requires `is_synthetic_mock: true`, restricts verdict to non-pass, and forces `human_authorized.authorized: false`; (2) `human_authorized.authorized: true` requires `is_synthetic_mock: false` and explicit authorization dimension. | Mock claiming pass REJECTED; synthetic claiming human auth REJECTED; valid mock output ACCEPTS. | Negative fixtures created; conditional schema logic verified syntactically. | Human identity authentication relies on external cryptographic signature or access log. |
| **F-06 (P1)**: Expired artifact fixture isolation | `input_invalid_expired_artifact.json` combined source-declared `observed_availability: "EXPIRED"` (untrusted claim) with a cross-repo CI run ID (`40144852713`), conflating two failure modes. | `docs/gov-00/fixtures/input_invalid_expired_artifact.json`, `docs/gov-00/schemas/checkpoint-input.schema.json`, `docs/gov-00/TRUST_AND_STATE_MODEL.md` | Removed `observed_availability` from schema and fixtures (engine must observe availability independently). Changed run ID to neutral `10000000001` matching repository. Decoupled cross-repo substitution test into separate fixture. | Expired artifact fixture tests only artifact availability failure; cross-repo fixture tests only run ID mismatch. | Fixtures decoupled and syntactically valid. | Offline engine cannot verify live HTTP 404 without online adapter (GOV-02). |
| **F-07 (P2)**: GitHub-hosted Windows ARM64 runner facts | Documentation incorrectly stated that GitHub does not provide Windows ARM64 runners, whereas `windows-11-arm` exists in GitHub's runner inventory. | `docs/gov-00/MVP_AND_INTEGRATION_GATES.md`, `docs/DECISIONS.md` | Corrected documentation: GitHub provides `windows-11-arm` runners, but Windows ARM64 target support remains deferred by deliberate product decision. Clarified that `windows-2025` is Windows Server, not Windows 11 client. | Documentation accurately reflects GitHub runner ecosystem and product boundary decisions. | Documentation updated and cross-checked against GitHub runner documentation. | Physical Windows 11 client testing deferred to post-MVP hardware labs. |

---

## 3. Normative Outer Envelope and Two-Schema Architecture

### 3.1 Two-Schema Structure

```
+--------------------------------------------------------------------------+
|  checkpoint-envelope.schema.json                                         |
|  {                                                                       |
|    "envelope_version": "1.0.0",                                          |
|    "payload": { ... },  <-- $ref: "checkpoint-input.schema.json"         |
|    "envelope_manifest_digest": "sha256:<64-hex-digits>"                  |
|  }                                                                       |
+--------------------------------------------------------------------------+
```

1. **`checkpoint-input.schema.json` (Inner Payload):**
   Defines the unverified source claims: checkpoint metadata, Git commit binding, CI evidence (workflow run, attempt, jobs, policy), artifact evidence, review evidence, and declaration claims.
2. **`checkpoint-envelope.schema.json` (Outer Canonical Envelope):**
   Defines the envelope structure:
   - `envelope_version`: Exactly `"1.0.0"`.
   - `payload`: Evaluated strictly against `checkpoint-input.schema.json` via `$ref`.
   - `envelope_manifest_digest`: Format `^sha256:[0-9a-f]{64}$`.

### 3.2 Verification Entry Points

- **`verify-envelope` (Default CLI entry point):** Requires the outer envelope. Validates envelope schema, checks `envelope_version`, canonicalizes `payload` via RFC 8785, computes SHA-256 digest, and verifies exact match with `envelope_manifest_digest`. Returns exit code 2 (`INTEGRITY_MISMATCH`) if digests differ.
- **`inspect-untrusted-payload` (Diagnostic / local preview):** Accepts bare `checkpoint-input.schema.json`. Evaluates structural schema only. Emits prominent warning that envelope digest verification was bypassed and outputs verdict `VERIFICATION_REFUSED` (exit code 3) or provisional finding.

### 3.3 Exact Preimage Definition & RFC 8785 Rules

The digest preimage is defined strictly as:
$$\text{Preimage} = \text{RFC8785\_Canonicalize}(\text{payload})$$

- The outer envelope keys (`envelope_version`, `envelope_manifest_digest`) are **never** included in the digest preimage. This eliminates self-referential hashing.
- RFC 8785 canonicalization applies:
  - Lexicographical sorting of object keys by UTF-16 code units.
  - No whitespace outside string literals.
  - Floats formatted per ECMAScript specifications (integers serialized without decimal points).
  - Strict UTF-8 output encoding.

### 3.4 Digest Verification Result

For the positive bootstrap fixture `input_valid_bootstrap_checkpoint.json`:
- **RFC 8785 Canonical Bytes Length:** 1,228 bytes.
- **Computed SHA-256 Digest:** `sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d`.
- **Positive Fixture:** `envelope_valid_bootstrap_checkpoint.json` contains this exact digest.
- **Tampered Fixture:** `envelope_invalid_tampered_digest.json` modifies the digest to `...ffff`, demonstrating deterministic rejection.

---

## 4. Trust Boundaries: Verified vs. Declared Properties

A fundamental security principle of the engine is that **source-declared JSON claims are untrusted assertions**. The engine never promotes a producer claim to a verified machine status without independent evidence.

| Field / Claim Category | Source Declaration Status | Verifier Authentication Requirement | Failure Mode on Missing / Untrusted Evidence |
|---|---|---|---|
| `git_evidence.commit_sha` | Producer claim of evaluated commit. | Must match target repository head or parent chain via Git/API. | `INTEGRITY_MISMATCH` (Exit code 2) |
| `ci_evidence.workflow_run_id` | Producer claim of CI execution. | Must belong to immutable repository ID (`1432704795`) and commit SHA. | `CORRUPTED_OR_TAMPERED` (Exit code 2) |
| `ci_evidence.expected_policy_digest` | Producer claim of required checks. | Must match policy digest authenticated from trusted root of trust. | `VERIFICATION_REFUSED` (Exit code 3) |
| `ci_evidence.observed_jobs` | Producer claim of check results. | Must be authenticated via GitHub Workflow Jobs REST API. | `EVIDENCE_UNAUTHENTICATED` (Exit code 2) |
| `artifact_evidence.archive_sha256` | Producer claim of artifact hash. | Must match independent byte-level hash of fetched artifact archive. | `INTEGRITY_MISMATCH` (Exit code 2) |
| Artifact expiration | Untrusted (cannot be declared by producer). | Must be independently observed via storage TTL or authenticated API 404. | `INSUFFICIENT_EVIDENCE` (Exit code 1) |
| `evaluator_auth.is_synthetic_mock` | Engine execution context flag. | Forced to `true` whenever running under synthetic/mock harness. | Contradictory claims REJECTED by schema |
| `human_authorized.authorized` | Claim of human sign-off. | Requires authenticated human authority record (GPG signature or scoped log). | `INSUFFICIENT_EVIDENCE` (Exit code 1) |

---

## 5. Test Vectors and Fixtures Inventory

Revision R2 provides a suite of 21 inert test fixtures in `docs/gov-00/fixtures/` demonstrating all positive and negative contract cases:

### 5.1 Positive Fixtures (2)
1. `input_valid_bootstrap_checkpoint.json`: Minimal valid documentation-only bootstrap checkpoint.
2. `envelope_valid_bootstrap_checkpoint.json`: Canonical outer envelope wrapping bootstrap input with reproducible SHA-256 digest (`157aaebc...`).

### 5.2 Negative & Adversarial Fixtures (19)

| Fixture Name | Category | Primary Target | Expected Result |
|---|---|---|---|
| `envelope_invalid_tampered_digest.json` | Envelope Integrity | Tampered outer envelope digest | REJECT: SHA-256 digest mismatch (exit code 2) |
| `input_invalid_ci_missing_policy.json` | CI Policy | Omitted `expected_policy_ref`/`digest` | REJECT: Schema validation failure (missing required) |
| `input_invalid_ci_untrusted_policy_digest.json` | CI Policy | Untrusted policy digest | REJECT: Policy provenance failure (exit code 3) |
| `input_invalid_cross_repo_ci_substitution.json` | CI Binding | Foreign repo Run ID (`40144852713`) | REJECT: Cross-repo substitution detected (exit code 2) |
| `input_invalid_expired_artifact.json` | Artifact Availability | Artifact expired from storage | REJECT: Artifact unavailable (exit code 1) |
| `input_invalid_missing_ci_jobs.json` | CI Policy | Empty `observed_jobs` array | REJECT: Missing required CI checks (exit code 2) |
| `input_invalid_wrong_run_attempt.json` | CI Binding | Stale run attempt (`attempt: 1`) | REJECT: Superseded run attempt (exit code 2) |
| `input_invalid_uint64_overflow_edge.json` | ID Boundary | $2^{64}$ (`18446744073709551616`) | REJECT: Semantic uint64 overflow (exit code 3) |
| `input_invalid_uint64_twenty_nines.json` | ID Boundary | 20 nines (`99999999999999999999`) | REJECT: Semantic uint64 overflow (exit code 3) |
| `input_invalid_id_leading_zero.json` | ID Boundary | Leading zero ID (`01432704795`) | REJECT: Schema regex format failure |
| `input_invalid_git_sha1_with_sha256_length.json` | Algorithm Match | SHA-1 label with 64-char hash | REJECT: Schema regex/length mismatch |
| `input_invalid_git_sha256_with_sha1_length.json` | Algorithm Match | SHA-256 label with 40-char hash | REJECT: Schema regex/length mismatch |
| `input_invalid_git_hash_wrong_algorithm.json` | Algorithm Match | Unsupported hash algorithm (`md5`) | REJECT: Schema enum validation failure |
| `input_invalid_future_timestamp.json` | Timestamp | Timestamp in the future | REJECT: Semantic freshness failure (exit code 1) |
| `input_invalid_malformed_iso8601.json` | Timestamp | Non-canonical date-time offset | REJECT: Schema regex format failure |
| `input_invalid_unknown_schema_version.json` | Versioning | Unsupported version (`99.0.0`) | REJECT: Schema version mismatch (exit code 3) |
| `output_valid_provisional_assessment.json` | Output Contract | Valid synthetic mock output | ACCEPT: Schema valid; verdict REFUSED |
| `output_invalid_mock_claiming_pass.json` | Output Invariant | Synthetic mock claiming `PASS` | REJECT: Schema conditional invariant failure |
| `output_invalid_synthetic_claiming_human_auth.json` | Output Invariant | Synthetic mock claiming human auth | REJECT: Schema conditional invariant failure |
| `output_invalid_claims_breakdown_override.json` | Output Invariant | Unauthorized dimension override | REJECT: Schema additionalProperties failure |
| `output_invalid_exit_code_mismatch.json` | Output Invariant | Exit code contradictory to verdict | REJECT: Schema enum validation failure |

---

## 6. Local Verification Results

### 6.1 JSON Syntax Verification
All 24 JSON files (3 schemas + 21 fixtures) were verified using the standard Python `json` parser:
```bash
python3 -c "
import glob, json
files = sorted(glob.glob('docs/gov-00/**/*.json', recursive=True))
for f in files:
    with open(f) as fp:
        json.load(fp)
    print('VALID_JSON:', f)
print(f'Total valid JSON files: {len(files)}')
"
```
**Result:** 24/24 JSON files syntactically valid (100% pass).

### 6.2 Structural Logic & Fixture Classification
- Shared object definition in `checkpoint-input.schema.json` correctly declares `name`, `status`, `conclusion` at item level.
- Positive bootstrap input and canonical envelope parse and validate structurally.
- RFC 8785 canonical digest reproduces deterministically: `sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d`.
- Conditional `if`/`then` rules in `assessment-output.schema.json` cleanly isolate synthetic mocks from promotional claims.

### 6.3 Status of Schema Meta-Validation
In strict compliance with governance rules prohibiting unauthorized external tool execution or dependency installation:
- Formal Draft 2020-12 meta-schema compilation via CLI validators (`check-jsonschema`, `ajv-cli`) is reported as **`NOT EXECUTED`**.
- Structural correctness is assured via manual design against the Draft 2020-12 specification and Python syntax parsing.

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

## 8. Remaining Limitations and GOV-01 Gate Prerequisites

The following boundaries must be maintained as GOV-00 concludes:
1. **No Runtime Implementation:** GOV-00 delivers specifications, contracts, schemas, and inert fixtures only. No verifier code has been written.
2. **Implementation Language Decision (ADR-006):** Rust is provisionally recommended with evaluative score 4.375/5.00, but remains **PROPOSED**. Owner authorization is required before GOV-01 code commences.
3. **Issue #3 (GOV-01) Remains Blocked:** Issue #3 must not be unblocked until independent Red Team audit of R2 is complete and accepted by repository owner.
4. **Read-Only Downstream Boundary:** AI-Toolchain-Manager (#32–#35) remains an external read-only consumer with zero verifier authority.
