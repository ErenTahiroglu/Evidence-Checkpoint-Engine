# GOV-01 Offline Checkpoint Verifier Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a minimal, offline Rust CLI (`ece`) that verifies evidence checkpoint envelopes via RFC 8785 JCS canonicalization and SHA-256 digest comparison, with six-dimensional assessment output and documented exit codes.

**Architecture:** A single Rust crate (`ece`) with a `verifier` library module (bounded parsing, schema validation, JCS canonicalization, digest verification, semantic checks, six-dimensional assessment) and a `main.rs` CLI entry point that reads from file or stdin, calls the library, and writes structured JSON to stdout / diagnostics to stderr.

**Tech Stack:** Rust stable, `serde_json`, `jsonschema` (Draft 2020-12), `jcs` (RFC 8785), `sha2`, `clap` (CLI arg parsing), `chrono` (UTC timestamps), `uuid` (assessment IDs).

**Spec:** `docs/gov-01-spec.md` (inline in this plan's parent commit message); authoritative contract in `docs/gov-00/CONTRACT_AND_CANONICALIZATION.md`, `docs/gov-00/GOV_00_R2_CONTRACT_CORRECTION.md`, and the three schemas in `docs/gov-00/schemas/`.

## Global Constraints

- Repository: `ErenTahiroglu/Evidence-Checkpoint-Engine`, REST ID `1432704795`
- Branch: `development` only; `main` (SHA `bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02`) must not change
- Rust edition: 2021; stable toolchain; single crate (no workspace unless forced)
- Max input: 10 MiB; max nesting depth: 32
- Digest preimage: RFC 8785 canonicalize(`payload` object only — never outer envelope fields)
- Expected bootstrap digest: `sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d` over 653 UTF-8 bytes
- Fixture path: `docs/gov-00/fixtures/` — read-only; do not modify
- Schema path: `docs/gov-00/schemas/` — loaded from disk at runtime, no network
- License: PolyForm Noncommercial 1.0.0 — preserve unchanged
- No async runtime, no database, no remote API, no GUI
- Exit codes: 0=clean pass, 1=limitations/warnings, 2=digest/verification failure, 3=refused/malformed/insufficient, 4=internal error
- Assessment output: always to stdout (JSON); diagnostics to stderr
- `is_synthetic_mock` must be `false` in real engine output; `ASSESSED_PROVISIONAL_PASS` forbidden in offline-only MVP
- No force-push, no `main` modification

## Review Focus

1. **Duplicate-key rejection before canonicalization:** A hostile input with duplicate JSON keys must be rejected before JCS processing begins; failure to reject means the engine may silently drop one key and hash a different document than the attacker supplied.
2. **uint64 overflow on 20-digit strings:** String `"18446744073709551616"` (2^64) matches the schema regex but must be rejected by the semantic parser; a float-conversion path would silently pass it.
3. **Digest exit-code agreement:** The `exit_code` field in the JSON output and the OS process exit code must always agree; an output claiming `ASSESSED_LIMITATIONS` with OS exit 2 is a contract violation.
4. **Envelope outer fields excluded from preimage:** Including `envelope_version` or `envelope_manifest_digest` in the JCS preimage produces a different hash; the test for the known digest pins this exactly.
5. **Self-declared acceptance injection:** A payload with `human_authorized: true` or a verdict of `ASSESSED_PROVISIONAL_PASS` in the input must not cause the engine to emit that verdict; the engine computes its own verdict independently.

---

## File Structure

```
ece/
  Cargo.toml
  Cargo.lock           (committed)
  src/
    main.rs            — CLI: arg parsing, I/O, calls verifier, writes output/stderr, exits
    verifier/
      mod.rs           — Public API: VerifierInput, VerificationResult, verify_envelope(), inspect_payload()
      parser.rs        — Bounded UTF-8 input reading, size/depth guard, duplicate-key JSON parse
      schema.rs        — Load schemas from disk, Draft 2020-12 validation
      canonicalize.rs  — RFC 8785 JCS implementation (pure Rust, no network)
      digest.rs        — SHA-256 over canonical bytes, prefix formatting
      semantic.rs      — uint64 checks, git SHA consistency, job status, duplicate jobs, self-injection guard
      assessment.rs    — Six-dimensional assessment builder, verdict determination, exit code
      error.rs         — VerifierError enum with explicit error classes
tests/
  integration_positive.rs
  integration_negative.rs
.github/
  workflows/
    build-and-test.yml — Cross-platform CI: windows-2025, macos-26, ubuntu-24.04
```

---

### Task 1: Crate scaffolding + error types + bounded parser

**Files:**
- Create: `ece/Cargo.toml`
- Create: `ece/src/main.rs` (stub only — arg dispatch shell, no logic)
- Create: `ece/src/verifier/mod.rs`
- Create: `ece/src/verifier/error.rs`
- Create: `ece/src/verifier/parser.rs`
- Create: `tests/integration_negative.rs` (size/depth/utf8/duplicate-key/malformed tests only)

**Interfaces:**
- Produces:
  - `VerifierError` enum (in `error.rs`): `InputTooLarge`, `NestingTooDeep`, `InvalidUtf8`, `DuplicateKey { key: String }`, `MalformedJson { detail: String }`, `SchemaViolation { details: Vec<String> }`, `DigestMismatch { expected: String, computed: String }`, `SemanticError { code: ErrorCode, detail: String }`, `InternalError(String)`.
  - `ErrorCode` enum: `UnsupportedVersion`, `Uint64Overflow`, `GitAlgorithmLengthMismatch`, `SelfInjectionAttempt`, `DuplicateJobName`, `InvalidJobStatusConclusion`, `MissingExpectedJob`, `LeadingZeroIdentifier`.
  - `parse_bounded_json(input: &[u8]) -> Result<serde_json::Value, VerifierError>` — checks size ≤ 10 MiB, validates UTF-8, then parses with duplicate-key detection, then checks nesting ≤ 32.

- [ ] **Step 1: Write failing tests** in `tests/integration_negative.rs`

```rust
// Test: input > 10 MiB → VerifierError::InputTooLarge
// Test: valid UTF-8 but nesting > 32 → VerifierError::NestingTooDeep  
// Test: invalid UTF-8 bytes → VerifierError::InvalidUtf8
// Test: JSON with duplicate key → VerifierError::DuplicateKey
// Test: malformed JSON syntax → VerifierError::MalformedJson
// (Use fixture: docs/gov-00/fixtures/input_malformed_syntax.txt)
```

- [ ] **Step 2: Run tests — expect compile failure (types not defined)**

```
cargo test --test integration_negative 2>&1 | head -30
```

- [ ] **Step 3: Create `ece/Cargo.toml`**

```toml
[package]
name = "ece"
version = "0.1.0"
edition = "2021"
description = "Evidence Checkpoint Engine — offline verifier CLI"
license = "LicenseRef-PolyForm-Noncommercial-1.0.0"

[[bin]]
name = "ece"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["preserve_order"] }
jsonschema = { version = "0.29", default-features = false }
sha2 = "0.10"
clap = { version = "4", features = ["derive"] }
chrono = { version = "0.4", features = ["clock"], default-features = false }
uuid = { version = "1", features = ["v4"] }
hex = "0.4"

[dev-dependencies]
# none needed beyond std

[profile.release]
strip = true
opt-level = "z"
```

- [ ] **Step 4: Create `ece/src/verifier/error.rs`** with `VerifierError` and `ErrorCode` as specified in Interfaces above.

- [ ] **Step 5: Create `ece/src/verifier/parser.rs`**

  `parse_bounded_json(input: &[u8]) -> Result<serde_json::Value, VerifierError>`:
  1. If `input.len() > 10 * 1024 * 1024` → `InputTooLarge`.
  2. `std::str::from_utf8(input)` → `InvalidUtf8` on error.
  3. Custom duplicate-key-detecting deserializer: use `serde_json` with a map deserializer that tracks keys and errors on duplicates → `DuplicateKey`.
  4. After parse, walk the `Value` tree to check nesting depth ≤ 32 → `NestingTooDeep`.
  5. On invalid JSON syntax → `MalformedJson`.

  For duplicate-key detection, use `serde_json::from_str` with a custom `serde::de::Visitor` that deserializes maps into `indexmap::IndexMap` tracking duplicates, **or** use `serde_json`'s raw value approach with a custom deserializer.

  **Simpler approach:** deserialize with `serde_json::Deserializer::from_str(...).deserialize_any(DuplicateCheckVisitor)` where the visitor errors on repeated keys during object parsing.

- [ ] **Step 6: Create stub `ece/src/verifier/mod.rs`** — re-export `error`, declare `parser` module.

- [ ] **Step 7: Create stub `ece/src/main.rs`** — `fn main() { println!("stub"); }` just to compile.

- [ ] **Step 8: Run tests — expect failures on missing logic, not compile errors**

```
cargo test --test integration_negative -- --nocapture 2>&1
```

- [ ] **Step 9: Implement `parser.rs` fully** until all 5 parser tests pass.

- [ ] **Step 10: Run tests — all 5 parser tests must pass**

```
cargo test --test integration_negative 2>&1
```

- [ ] **Step 11: `cargo fmt --all -- --check` and `cargo clippy --locked --all-targets -- -D warnings`**

- [ ] **Step 12: Commit**

```
git add ece/ tests/integration_negative.rs
git commit -m "feat(gov-01): crate scaffold, error types, bounded parser"
```

---

### Task 2: Schema validation (Draft 2020-12, local $ref)

**Files:**
- Create: `ece/src/verifier/schema.rs`
- Modify: `ece/src/verifier/mod.rs` — add `schema` module
- Modify: `tests/integration_negative.rs` — add schema rejection tests

**Interfaces:**
- Consumes: `parse_bounded_json` from Task 1, `serde_json::Value`
- Produces:
  - `SchemaStore` struct with `fn load(schema_dir: &Path) -> Result<SchemaStore, VerifierError>`
  - `fn validate_payload(store: &SchemaStore, payload: &Value) -> Result<(), VerifierError>`
  - `fn validate_envelope(store: &SchemaStore, envelope: &Value) -> Result<(), VerifierError>`
  - `fn validate_assessment(store: &SchemaStore, assessment: &Value) -> Result<(), VerifierError>`
  - Schema dir determined at runtime: path relative to binary or override via `ECE_SCHEMA_DIR` env var; default is `docs/gov-00/schemas/` relative to CWD.

- [ ] **Step 1: Write failing schema tests** (append to `tests/integration_negative.rs`)

```rust
// Test: unsupported schema_version → SchemaViolation
// Test: missing required field (e.g., missing "integrity") → SchemaViolation
// Test: sha1 commit with 64-char hash → SchemaViolation  
// Test: sha256 commit with 40-char hash → SchemaViolation
// Test: valid bootstrap payload → Ok(())
// Test: local $ref resolved without network (no panics, timeout)
```

- [ ] **Step 2: Run — expect failures**

- [ ] **Step 3: Implement `schema.rs`**

  Use `jsonschema` crate. Load `checkpoint-input.schema.json`, `checkpoint-envelope.schema.json`, `assessment-output.schema.json` from disk. Configure `jsonschema` with:
  - `with_draft(jsonschema::Draft::Draft202012)`
  - Local `$ref` resolution: register `checkpoint-input.schema.json` as a local resource by its `$id` URI so that when envelope schema references it, resolution succeeds offline. Use `jsonschema::options().with_resource(id, schema_json)`.
  - **No** network retrieval: do not call any HTTP client; the `jsonschema` crate's default is to attempt network for unresolved `$id` URIs, so explicitly register both schemas.

- [ ] **Step 4: Run schema tests — all pass**

- [ ] **Step 5: `cargo fmt --all -- --check && cargo clippy --locked --all-targets -- -D warnings`**

- [ ] **Step 6: Commit**

```
git add ece/src/verifier/schema.rs ece/src/verifier/mod.rs tests/integration_negative.rs
git commit -m "feat(gov-01): Draft 2020-12 schema validation with local ref resolution"
```

---

### Task 3: RFC 8785 JCS canonicalization

**Files:**
- Create: `ece/src/verifier/canonicalize.rs`
- Modify: `ece/src/verifier/mod.rs`
- Create: `tests/integration_positive.rs` (canonicalization tests)

**Interfaces:**
- Consumes: `serde_json::Value`
- Produces:
  - `fn canonicalize(value: &Value) -> Result<Vec<u8>, VerifierError>` — returns UTF-8 bytes of JCS-canonical JSON

- [ ] **Step 1: Write JCS test cases** in `tests/integration_positive.rs`

```rust
// Test: bootstrap payload canonicalizes to exactly 653 bytes
// Test: bootstrap payload digest == "sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d"
// Test: object keys sorted by UTF-16 code units (not Unicode codepoints)
// Test: negative zero serializes as "0"  
// Test: integer 1 serializes as "1" not "1.0"
// Test: no whitespace between tokens
// Test: non-BMP character key sorting (surrogate pair order)
// Test: solidus "/" not escaped
// Test: control chars \u0000-\u001F use lowercase \uXXXX
```

- [ ] **Step 2: Run — expect failures**

- [ ] **Step 3: Implement `canonicalize.rs`**

  Implement JCS directly (do not rely on an external `jcs` crate — implement it cleanly):
  
  ```rust
  pub fn canonicalize(value: &serde_json::Value) -> Result<Vec<u8>, VerifierError>
  ```
  
  Algorithm:
  - `Null` → `b"null"`
  - `Bool(true)` → `b"true"`, `Bool(false)` → `b"false"`
  - `Number`: if representable as i64/u64 with no fractional part → decimal integer bytes; else use ECMAScript number serialization rules (shortest representation, lowercase `e`, no `+` in exponent, negative zero → `0`)
  - `String(s)`: emit `"`, then for each char: if `\`, emit `\\`; if `"`, emit `\"`; if code point < 0x20, emit `\uXXXX` (lowercase, exactly 4 hex digits); else emit UTF-8 bytes. Never escape `/`.
  - `Array(items)`: `[` + items joined by `,` + `]`
  - `Object(map)`: sort keys by their UTF-16 encoding (each key → UTF-16 code units → sort lexicographically); emit `{` + `"key":value` pairs joined by `,` + `}`
  
  UTF-16 sort key for a string key: `key.encode_utf16().collect::<Vec<u16>>()`.
  
  For the `Number` ECMAScript serialization: use `serde_json`'s `f64()` representation. If `f64` is negative zero, emit `0`. Otherwise, use Rust's `ryu` or format via `serde_json` — the simplest correct path is: for integers (f64 with no fractional part, within i64 range), serialize as integer. For other floats, use `format!("{}", value.as_f64().unwrap())` and then apply ECMAScript rules (strip trailing zeros from mantissa but keep at least one digit, use `e` not `E`, etc.).
  
  **Simpler and safer:** Use `serde_json::Value::Number` — call `n.as_i64()` first (integers), then `n.as_u64()` (unsigned), then `n.as_f64()` for floats. This avoids float serialization edge cases for the schema values we handle.

- [ ] **Step 4: Run — all canonicalization tests must pass including 653-byte length and exact digest**

- [ ] **Step 5: `cargo fmt && cargo clippy -D warnings`**

- [ ] **Step 6: Commit**

```
git add ece/src/verifier/canonicalize.rs ece/src/verifier/mod.rs tests/integration_positive.rs
git commit -m "feat(gov-01): RFC 8785 JCS canonicalization with UTF-16 key sort"
```

---

### Task 4: SHA-256 digest computation and envelope verification

**Files:**
- Create: `ece/src/verifier/digest.rs`
- Modify: `ece/src/verifier/mod.rs`
- Modify: `tests/integration_positive.rs` — add envelope verification tests
- Modify: `tests/integration_negative.rs` — add tampered digest test

**Interfaces:**
- Consumes: `canonicalize()` from Task 3
- Produces:
  - `fn compute_digest(canonical_bytes: &[u8]) -> String` — returns `"sha256:<64 lowercase hex>"`
  - `fn verify_envelope_digest(envelope: &Value) -> Result<String, VerifierError>` — extracts `payload`, canonicalizes it, computes digest, compares to `envelope_manifest_digest`; on mismatch returns `DigestMismatch { expected, computed }`.

- [ ] **Step 1: Write digest tests**

```rust
// Positive: verify_envelope_digest on bootstrap envelope → Ok("sha256:157aaebc...")
// Positive: compute_digest on 653 canonical bytes == expected
// Negative: tampered digest fixture → DigestMismatch
// Negative: envelope where payload is modified after digest was computed → DigestMismatch
```

- [ ] **Step 2: Implement `digest.rs`** using `sha2::Sha256`, `hex::encode`.

- [ ] **Step 3: Run tests — all pass**

- [ ] **Step 4: `cargo fmt && cargo clippy -D warnings`**

- [ ] **Step 5: Commit**

```
git add ece/src/verifier/digest.rs ece/src/verifier/mod.rs tests/
git commit -m "feat(gov-01): SHA-256 envelope digest verification"
```

---

### Task 5: Semantic validation checks

**Files:**
- Create: `ece/src/verifier/semantic.rs`
- Modify: `ece/src/verifier/mod.rs`
- Modify: `tests/integration_negative.rs` — semantic check tests

**Interfaces:**
- Consumes: `serde_json::Value` of parsed payload, `VerifierError` / `ErrorCode` from Task 1
- Produces:
  - `fn run_semantic_checks(payload: &Value) -> Result<Vec<String>, VerifierError>` — returns list of non-fatal warnings, or `Err(VerifierError::SemanticError{..})` on fatal violations.
  
  Checks implemented (in order):
  1. `schema_version == "0.1.0-draft"` — else `UnsupportedVersion`
  2. `repository.github_rest_id` — parse as u64 strictly (no float), check ≤ `u64::MAX` — else `Uint64Overflow`. Also check not leading-zero (already regex-guarded, but double-check) — else `LeadingZeroIdentifier`.
  3. `ci_evidence.run_id` (if present) — same u64 parse + `Uint64Overflow` / `LeadingZeroIdentifier`
  4. `ci_evidence` `git_hash_algorithm` vs `commit_sha` length: sha1→40 chars, sha256→64 chars — else `GitAlgorithmLengthMismatch`
  5. `ci_evidence.expected_jobs` — check for duplicate names — else `DuplicateJobName`
  6. `ci_evidence.observed_jobs` — for each job with `status == "completed"`, `conclusion` must be present and not null; for non-completed, `conclusion` must be absent or null — else `InvalidJobStatusConclusion`
  7. `ci_evidence.expected_jobs` vs `ci_evidence.observed_jobs` names: any expected job not observed → warning (not fatal), stored in `unresolved_jobs`
  8. `ci_evidence.expected_policy_ref` must be present when `ci_evidence` is present — (schema enforces this, but add semantic warning if policy digest doesn't match a known baseline → produces `UNVERIFIED_POLICY` warning)
  9. **Self-injection guard:** if input contains `claims_breakdown.human_authorized.authorized == true` as a field in the *payload* (shouldn't exist there; it's an output field) — no structural issue. But check: if `producer.role == "agent"` AND payload contains any field named `assessment_id` or `verdict` at top level → `SelfInjectionAttempt`. Also if any observed_job `conclusion == "self_accepted"` or similar — emit warning.
  10. Source `commit_sha` length consistency with `git_hash_algorithm` (belt-and-suspenders on top of schema).

- [ ] **Step 1: Write semantic tests** (append to `tests/integration_negative.rs`)

```rust
// Test: github_rest_id "18446744073709551616" (u64 overflow) → SemanticError(Uint64Overflow)
// Test: github_rest_id "99999999999999999999" (20 nines) → SemanticError(Uint64Overflow)
// Test: leading-zero "01432704795" rejected by schema already, but semantic also → (schema catches first)
// Test: sha1 algorithm with 64-char hash → SemanticError(GitAlgorithmLengthMismatch)  [fixture: input_invalid_git_sha1_with_sha256_length.json]
// Test: sha256 algorithm with 40-char hash → SemanticError(GitAlgorithmLengthMismatch)  [fixture: input_invalid_git_sha256_with_sha1_length.json]
// Test: duplicate expected_job names → SemanticError(DuplicateJobName)
// Test: completed job with no conclusion → SemanticError(InvalidJobStatusConclusion)
// Test: queued job with conclusion "success" → SemanticError(InvalidJobStatusConclusion)  [fixture: input_invalid_mismatched_sha.json]
// Test: missing expected job → returns warning in checks_omitted, not SemanticError
// Test: unsupported schema version → SemanticError(UnsupportedVersion)  [fixture: input_invalid_unsupported_schema_version.json]
```

- [ ] **Step 2: Implement `semantic.rs`** — pure functions, no I/O, deterministic.

- [ ] **Step 3: Run tests — all pass**

- [ ] **Step 4: `cargo fmt && cargo clippy -D warnings`**

- [ ] **Step 5: Commit**

```
git add ece/src/verifier/semantic.rs ece/src/verifier/mod.rs tests/integration_negative.rs
git commit -m "feat(gov-01): semantic validation: uint64, git SHA, job status, self-injection"
```

---

### Task 6: Six-dimensional assessment builder and verdict logic

**Files:**
- Create: `ece/src/verifier/assessment.rs`
- Modify: `ece/src/verifier/mod.rs` — expose `verify_envelope()` and `inspect_payload()` public API
- Modify: `tests/integration_positive.rs` — assessment output tests

**Interfaces:**
- Consumes: results from Tasks 1–5 (all verifier modules)
- Produces:
  - `pub fn verify_envelope(input_bytes: &[u8], schema_dir: &Path) -> (serde_json::Value, i32)` — returns `(assessment_json, exit_code)`
  - `pub fn inspect_payload(input_bytes: &[u8], schema_dir: &Path) -> (serde_json::Value, i32)` — diagnostic only, always exit 3
  - `fn build_assessment(...)` — internal builder
  - Assessment JSON structure matches `assessment-output.schema.json` exactly.

  Assessment verdict rules (for offline MVP):
  - Fatal parse/schema/semantic error → verdict `"REFUSED"`, exit 3
  - Digest mismatch → verdict `"ASSESSED_FAIL"`, exit 2  
  - All structural checks pass, digest matches, but no authenticated CI/review/auth evidence → verdict `"ASSESSED_LIMITATIONS"`, exit 1
  - `is_synthetic_mock` always `false` in real engine output
  - `execution_context` always `"LOCAL_SUBPROCESS_PIPE"`
  - `ASSESSED_PROVISIONAL_PASS` (exit 0) is NOT achievable in offline MVP — always at least exit 1 due to unverified external facts
  - Six dimension statuses for a structurally valid envelope with matching digest:
    - `evidence_integrity`: `"VALID"`, `digest_match: true`
    - `ci_execution`: `"UNVERIFIED_RUN"` (source-declared, not authenticated)
    - `evidence_availability`: `"NOT_EVALUATED"` (offline)
    - `review_status`: `"NOT_EVALUATED"` (offline)
    - `authorization`: `"AUTHORIZATION_PENDING"` (offline)
    - `freshness`: `"NOT_EVALUATED"` (offline)

- [ ] **Step 1: Write assessment tests**

```rust
// Positive: verify_envelope on bootstrap envelope → exit 1, verdict "ASSESSED_LIMITATIONS", digest_match true
// Positive: inspect_payload on valid input → exit 3, prominent UNTRUSTED warning in output
// Negative: verify_envelope on tampered digest → exit 2, verdict "ASSESSED_FAIL"
// Negative: verify_envelope on malformed JSON → exit 3, verdict "REFUSED"
// Positive: assessment output validates against assessment-output.schema.json
// Positive: is_synthetic_mock is false in real output
// Positive: exit_code field in JSON matches actual return value
// Negative: self_accepted injection fixture → exit 3
// Positive: cross-repo fixture → exit 1 with warning (unverified run, offline cannot authenticate)
```

- [ ] **Step 2: Implement `assessment.rs`**

  `build_assessment()` takes structured results from all prior modules and builds the full output JSON. Use `serde_json::json!{}` macro for construction.

  Key fields:
  - `assessment_id`: `uuid::Uuid::new_v4().to_string()`
  - `evaluation_timestamp`: `chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()`
  - `evaluator_auth.execution_context`: `"LOCAL_SUBPROCESS_PIPE"`
  - `evaluator_auth.is_synthetic_mock`: `false`
  - `engine.name`: `"Evidence-Checkpoint-Engine"`, `version`: env!("CARGO_PKG_VERSION"), `canonicalization_standard`: `"RFC8785"`
  - `claims_breakdown.human_authorized.authorized`: `false` (offline cannot verify)
  - `checks_omitted`: always includes at minimum `"Live GitHub REST API authentication"`, `"CI workflow run cryptographic verification"`, `"Independent reviewer identity verification"`

- [ ] **Step 3: Run all tests — all pass**

- [ ] **Step 4: `cargo fmt && cargo clippy -D warnings`**

- [ ] **Step 5: Commit**

```
git add ece/src/verifier/assessment.rs ece/src/verifier/mod.rs tests/integration_positive.rs
git commit -m "feat(gov-01): six-dimensional assessment builder and verdict logic"
```

---

### Task 7: CLI entry point and exit code contract

**Files:**
- Modify: `ece/src/main.rs` (full implementation replacing stub)
- Modify: `tests/integration_positive.rs` — CLI invocation tests via `std::process::Command`
- Modify: `tests/integration_negative.rs` — CLI negative path tests

**Interfaces:**
- Consumes: `verifier::verify_envelope()` and `verifier::inspect_payload()` from Task 6
- Produces: CLI binary `ece` with these subcommands:
  - `ece verify-envelope --input <file|->` → assessment JSON to stdout, diagnostics to stderr, exits per code contract
  - `ece inspect-untrusted-payload --input <file|->` → always exit 3, prominent warning
  - `-` means read from stdin
  - `--schema-dir <path>` optional override for `ECE_SCHEMA_DIR`

- [ ] **Step 1: Write CLI integration tests** (using `std::process::Command` to invoke the binary)

```rust
// Test: ece verify-envelope --input docs/gov-00/fixtures/envelope_valid_bootstrap_checkpoint.json → exit 1
// Test: ece verify-envelope --input docs/gov-00/fixtures/envelope_invalid_tampered_digest.json → exit 2
// Test: ece verify-envelope --input - (pipe malformed JSON) → exit 3
// Test: ece verify-envelope --input nonexistent.json → exit 4
// Test: ece inspect-untrusted-payload --input docs/gov-00/fixtures/input_valid_bootstrap_checkpoint.json → exit 3
// Test: stdout is valid JSON on all above
// Test: stderr contains diagnostic text on failure paths
// Test: exit_code in JSON output matches OS exit code
```

- [ ] **Step 2: Implement `main.rs`**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ece", about = "Evidence Checkpoint Engine — offline verifier")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(long, env = "ECE_SCHEMA_DIR")]
    schema_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    VerifyEnvelope { #[arg(long)] input: String },
    InspectUntrustedPayload { #[arg(long)] input: String },
}
```

  Read input: if `input == "-"`, read all stdin bytes (respecting size limit); else read file. On I/O error → stderr diagnostic + exit 4.
  
  Call appropriate verifier function. Write `serde_json::to_string_pretty(&assessment)` to stdout. Write diagnostic summary to stderr (one line: `ECE: verdict=X exit=N`). Call `std::process::exit(exit_code)`.
  
  On Windows: set console to UTF-8 before any output. Use `#[cfg(windows)]` block calling `SetConsoleOutputCP(65001)` via `windows-sys` or just rely on `RUST_LOG` being off and write raw UTF-8 bytes.

- [ ] **Step 3: Run CLI tests**

```
cargo build --locked
cargo test --test integration_positive
cargo test --test integration_negative
```

- [ ] **Step 4: `cargo fmt && cargo clippy -D warnings && git diff --check`**

- [ ] **Step 5: Commit**

```
git add ece/src/main.rs tests/
git commit -m "feat(gov-01): CLI entry point, stdin/file input, documented exit codes"
```

---

### Task 8: GitHub Actions cross-platform CI workflow

**Files:**
- Create: `.github/workflows/build-and-test.yml`

**Interfaces:**
- Produces: CI workflow that builds and tests on `windows-2025`, `macos-26`, `ubuntu-24.04`, optional `macos-26-intel`

- [ ] **Step 1: Verify runner labels** against current GitHub documentation (already verified in plan: `macos-26`, `windows-2025`, `ubuntu-24.04` confirmed available)

- [ ] **Step 2: Write `.github/workflows/build-and-test.yml`**

```yaml
name: Build and Test

on:
  push:
    branches: [development]
  pull_request:
    branches: [development]

permissions:
  contents: read

jobs:
  build-and-test:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-24.04
            name: Linux x64
          - os: macos-26
            name: macOS ARM64
          - os: windows-2025
            name: Windows x64
          - os: macos-26-intel
            name: macOS Intel x64

    runs-on: ${{ matrix.os }}
    name: ${{ matrix.name }}

    steps:
      - name: Checkout repository
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683  # v4.2.2

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@a54c7afa936d8b73f8f87b2f63c4b6b9c5acbef6  # stable
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Cache Cargo registry
        uses: actions/cache@d4323d4dc3a21f9e4b1815c5b619a8d64e34e32a  # v4.2.2
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-${{ hashFiles('ece/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-

      - name: Check formatting
        working-directory: ece
        run: cargo fmt --all -- --check

      - name: Compiler check
        working-directory: ece
        run: cargo check --locked --all-targets

      - name: Clippy
        working-directory: ece
        run: cargo clippy --locked --all-targets -- -D warnings

      - name: Run tests
        working-directory: ece
        run: cargo test --locked

      - name: Build release binary
        working-directory: ece
        run: cargo build --locked --release

      - name: Smoke test CLI (Unix)
        if: runner.os != 'Windows'
        working-directory: ece
        run: |
          ./target/release/ece verify-envelope \
            --input ../docs/gov-00/fixtures/envelope_valid_bootstrap_checkpoint.json \
            --schema-dir ../docs/gov-00/schemas
          echo "Exit code: $?"

      - name: Smoke test CLI (Windows)
        if: runner.os == 'Windows'
        working-directory: ece
        run: |
          .\target\release\ece.exe verify-envelope `
            --input ..\docs\gov-00\fixtures\envelope_valid_bootstrap_checkpoint.json `
            --schema-dir ..\docs\gov-00\schemas
          Write-Host "Exit code: $LASTEXITCODE"
        shell: pwsh
```

  **Note on third-party action SHA pins:** The SHAs above (`actions/checkout@11bd71901...`, `dtolnay/rust-toolchain@a54c7...`, `actions/cache@d4323d4...`) must be verified immutable commit SHAs from the respective repository tags at time of implementation. Verify with `git ls-remote https://github.com/actions/checkout refs/tags/v4.2.2` before committing.

- [ ] **Step 3: Verify action SHAs** (run `git ls-remote` for each action tag)

- [ ] **Step 4: `git diff --check`**

- [ ] **Step 5: Commit**

```
git add .github/workflows/build-and-test.yml
git commit -m "ci(gov-01): cross-platform build and test workflow (windows-2025, macos-26, ubuntu-24.04)"
```

---

### Task 9: Documentation updates and pre-push verification

**Files:**
- Modify: `docs/gov-00/GOV_00_R2_CONTRACT_CORRECTION.md` — correct "1,228 bytes" to "653 bytes"
- Modify: `docs/gov-00/CONTRACT_AND_CANONICALIZATION.md` — correct byte count
- Create: `docs/gov-01/GOV_01_IMPLEMENTATION_NOTES.md` — brief notes on crate structure, dependency choices, known limitations

**Interfaces:**
- None (documentation only)

- [ ] **Step 1: Correct the 1,228-byte figure in `GOV_00_R2_CONTRACT_CORRECTION.md`** — replace with 653 bytes (independently verified by Python calculation above)

- [ ] **Step 2: Same correction in `CONTRACT_AND_CANONICALIZATION.md`** if present

- [ ] **Step 3: Create `docs/gov-01/GOV_01_IMPLEMENTATION_NOTES.md`** with:
  - Crate structure
  - Dependency justifications
  - What the engine proves vs. what remains unverified
  - Known limitations (offline only, no GitHub API, ASSESSED_PROVISIONAL_PASS not achievable)

- [ ] **Step 4: Full pre-push verification**

```bash
cd ece
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

- [ ] **Step 5: Commit**

```
git add docs/
git commit -m "docs(gov-01): correct 1228→653 byte count, add implementation notes"
```

---

### Task 10: Final integration pass and push

- [ ] **Step 1: Run complete test suite one final time**

```
cd ece && cargo test --locked 2>&1
```

- [ ] **Step 2: Verify Cargo.lock is committed** (`git ls-files ece/Cargo.lock`)

- [ ] **Step 3: Verify `main` SHA unchanged** (`git rev-parse origin/main`)

- [ ] **Step 4: Push `development`**

```
git push origin development
```

- [ ] **Step 5: Record exact final SHA**

```
git rev-parse HEAD
```

- [ ] **Step 6: Confirm `main` still frozen**

```
git rev-parse origin/main
# Must equal: bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02
```

---

