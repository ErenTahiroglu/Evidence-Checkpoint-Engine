# GOV-01 Implementation Notes: Minimal Offline Verification Core & CLI

> **Milestone:** `GOV-01`
> **Target Classification:** `GOV_01_READY_FOR_INDEPENDENT_REDTEAM`
> **Date:** `2026-10-29`
> **Repository:** `ErenTahiroglu/Evidence-Checkpoint-Engine` (REST ID: `1432704795`)
> **Authorized Branch:** `development`
> **Starting SHA:** `226966392628d9782618347492f563ff536cf5b0`
> **Frozen Main SHA:** `bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02`

---

## 1. Minimal Software Architecture

The implementation follows ADR-006 (Rust) and the standalone CLI subprocess architecture with controlled stdin/stdout. A single cohesive crate (`ece`) is organized as:

```
ece/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Re-exports verifier public API
│   ├── main.rs             # CLI interface (clap, stdin/file reader, stdout JSON, stderr diagnostics)
│   └── verifier/
│       ├── mod.rs          # Verifier library entry points (verify_envelope, inspect_payload)
│       ├── parser.rs       # Bounded input parser (10 MiB, UTF-8, duplicate keys, nesting depth <= 32)
│       ├── schema.rs       # Offline Draft 2020-12 validator with local in-memory $ref resolution
│       ├── canonicalize.rs # RFC 8785 JCS canonicalization (UTF-16 key sort, ECMAScript float rules)
│       ├── digest.rs       # SHA-256 over canonical UTF-8 bytes and envelope_manifest_digest comparison
│       ├── semantic.rs     # uint64 checks, git SHA consistency, job status/conclusion, self-injection guard
│       ├── assessment.rs   # Six-dimensional assessment builder and exit code determination
│       └── error.rs        # VerifierError and ErrorCode enumerations
└── tests/
    ├── integration_positive.rs  # Positive test fixtures & standard vector assertions
    └── integration_negative.rs  # Adversarial fixtures, tampered digests, syntax & bounds rejections
```

---

## 2. Minimal Dependency Choices & Justifications

To preserve security, cross-platform reproducibility, and minimal supply chain risk:

| Crate | Purpose | Justification |
|---|---|---|
| `serde` (v1) | Serialization/deserialization traits | De facto Rust standard for typed serialization |
| `serde_json` (v1, `preserve_order`) | JSON AST and value representation | Standard JSON handling; streaming deserializer for duplicate-key checks |
| `jsonschema` (v0.29, no default features) | JSON Schema Draft 2020-12 validation | Compliant with Draft 2020-12; supports custom `Retrieve` trait for offline `$ref` resolution |
| `sha2` (v0.10) | SHA-256 hashing | Pure Rust implementation of FIPS 180-4 standard |
| `clap` (v4, `derive`) | CLI argument parsing | Synchronous, deterministic command-line parser |
| `chrono` (v0.4, `clock`) | UTC ISO-8601 timestamps | Accurate timestamping for assessment output |
| `uuid` (v4) | Unique assessment IDs (`eval-<uuid>`) | Standard v4 UUID generation |
| `hex` (v0.4) | Hexadecimal encoding/decoding | Constant-time hex string conversions |
| `thiserror` (v1) | Structured error definitions | Clean, deterministic error enums |

---

## 3. Safe Bounded Input Parsing

All input is treated as untrusted and evaluated under strict limits:
1. **Size Bound:** Max 10 MiB (`10 * 1024 * 1024` bytes). Oversized inputs fail closed immediately (`INPUT_TOO_LARGE`).
2. **UTF-8 Validation:** Checked before JSON parsing (`INVALID_UTF8`).
3. **Duplicate Key Rejection:** Streaming JSON parsing detects duplicate keys in objects and rejects them fail-closed (`DUPLICATE_KEY`).
4. **Nesting Depth Bound:** AST traversal validates maximum depth $\le 32$ (`NESTING_TOO_DEEP`).
5. **No Execution:** Inputs are parsed strictly as inert JSON data; no shell execution, code evaluation, or script execution occurs.

---

## 4. Offline JSON Schema Validation (Draft 2020-12)

- Bundled Draft 2020-12 schemas loaded strictly from local disk:
  - `checkpoint-envelope.schema.json`
  - `checkpoint-input.schema.json`
  - `assessment-output.schema.json`
- **Zero Ambient Network Requests:** A custom `LocalRetriever` struct implementing `jsonschema::Retrieve` maps schema `$id` URIs and relative filenames directly to pre-loaded in-memory schema objects. If an unmapped URI is referenced, resolution fails closed with an error; no HTTP requests are ever attempted.

---

## 5. RFC 8785 JCS Canonicalization and Envelope Digest

- Implementation in `canonicalize.rs` strictly follows RFC 8785:
  - Object keys sorted lexicographically by **UTF-16 code units** (ensuring correct surrogate-pair ordering for non-BMP code points).
  - Whitespace eliminated outside string literals.
  - Floats serialized according to ECMAScript `JSON.stringify()` rules; negative zero (`-0.0`) canonicalizes as `0`.
  - String escaping restricted to `\"`, `\\`, and lowercase `\u00xx` control characters; solidus (`/`) is never escaped.
- **Preimage Scope:** Digest is computed strictly over `RFC8785(payload)`. Outer envelope keys (`envelope_version`, `envelope_manifest_digest`) are strictly excluded from the preimage.
- **Bootstrap Verification:**
  - Canonical UTF-8 length: exactly **653 bytes**.
  - Canonical SHA-256 digest: `sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d`.

---

## 6. Semantic Validation Rules

Beyond schema syntax, `semantic.rs` enforces:
1. `schema_version`: Must strictly match `"0.1.0-draft"`.
2. `uint64` Parsing: String identifiers (`github_rest_id`, `run_id`, `artifact_id`) are parsed directly as unsigned 64-bit integers with explicit overflow rejection ($\le 18{,}446{,}744{,}073{,}709{,}551{,}615$). Leading zeros are rejected.
3. Git Hash Agility: Enforces 40-character hex for `sha1` and 64-character hex for `sha256`.
4. CI Job Logic: Completed jobs require non-null `conclusion`; queued/in-progress jobs require absent or null `conclusion`.
5. Expected Jobs: Duplicate expected job names are rejected; unobserved expected jobs are recorded in `unresolved_jobs`.
6. Self-Injection Defense: Prevents AI agents from elevating their assertions to independent human reviews.

---

## 7. Six-Dimensional Assessment Output & Exit Code Contract

The engine enforces strict separation between machine-evaluated facts and untrusted claims:

| Exit Code | Meaning | Offline MVP Applicability |
|---|---|---|
| `0` | Clean Pass (`ASSESSED_PROVISIONAL_PASS`) | **Never emitted offline** (requires authenticated GitHub CI, review, and owner authorization) |
| `1` | Limitations / Warnings (`ASSESSED_LIMITATIONS`) | Emitted on valid envelope and matching digest (with clear warnings that external facts remain unverified) |
| `2` | Verification Failure (`ASSESSED_FAIL`) | Emitted on confirmed tamper/digest mismatch |
| `3` | Refused / Invalid / Insufficient (`REFUSED`) | Emitted on malformed JSON, schema violation, semantic error, or `inspect-untrusted-payload` mode |
| `4` | Operational / Execution Error | Emitted on missing files, schema loading failures, or I/O errors |

---

## 8. Cross-Platform GitHub Actions Matrix

The repository-local workflow `.github/workflows/build-and-test.yml` tests across:
- `ubuntu-24.04` (Linux x64)
- `macos-26` (macOS Apple Silicon ARM64)
- `windows-2025` (Windows x64)
- `macos-26-intel` (macOS Intel x64)

Actions are pinned by immutable commit SHAs with `permissions: contents: read`.
