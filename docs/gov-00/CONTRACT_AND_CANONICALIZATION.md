# Contract Specification, Canonical Serialization (RFC 8785) & Two-Schema Envelope

> **Milestone:** `GOV-00 R2`
> **Status:** `PROPOSED SPECIFICATION / REMEDIATED CONTRACT DRAFT`
> **Primary References:** [RFC 8785 (JCS)](https://www.rfc-editor.org/rfc/rfc8785.html), [RFC 7493 (I-JSON)](https://www.rfc-editor.org/rfc/rfc7493.html), [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)

---

## 1. The Necessity of Canonical Serialization

A hash over JSON data is meaningless unless the serialization process is **100% deterministic**. Standard JSON serializers across programming languages introduce variations in:
- Whitespace and line endings (`\r\n` vs `\n`).
- Key ordering in object dictionaries (hash-table order vs insertion order vs alphabetical order).
- Floating-point representations (`1.0` vs `1`, `1e-5` vs `0.00001`, `-0` vs `0`).
- String escaping (`\/` vs `/`, uppercase vs lowercase hex in `\u000a` vs `\u000A`).
- Unicode representation (code-point sorting vs UTF-16 code-unit sorting).

`Evidence-Checkpoint-Engine` mandates **RFC 8785: JSON Canonicalization Scheme (JCS)** as its sole canonicalization standard.

---

## 2. RFC 8785 Canonicalization Rules

Any verifier implementation in this engine must strictly enforce the following RFC 8785 requirements:

### 2.1 Whitespace
- No whitespace is permitted outside of string literals.
- No newlines, tabs, or spaces between object keys, colons, or array elements (e.g. `{"a":1,"b":2}`, not `{"a": 1, "b": 2}`).

### 2.2 Object Property Sorting
- Object member keys **must be sorted lexicographically based on their UTF-16 code units**.
- *Critical Distinction:* UTF-16 code-unit sorting is NOT identical to Unicode code-point sorting for characters outside the Basic Multilingual Plane (BMP, code points > `U+FFFF`). Characters outside the BMP are encoded as surrogate pairs in UTF-16. JCS requires sorting by the high surrogate followed by the low surrogate.

### 2.3 Character Escaping and Encoding
- Input and output must be valid **UTF-8 without a Byte Order Mark (BOM)**.
- Lone surrogates (`\uD800` through `\uDFFF`) are invalid UTF-8 and **must be rejected fail-closed**.
- String escaping is restricted to:
  - Quotation mark (`\"`)
  - Reverse solidus (`\\`)
  - Control characters `U+0000` through `U+001F` (using `\b`, `\f`, `\n`, `\r`, `\t`, or two-hex-digit lowercase unicode escapes `\u00xx`).
- Solidus (`/`) **must NOT be escaped**.

### 2.4 Number Serialization and IEEE-754 Limits
- Number formatting must follow ECMAScript (JavaScript) `JSON.stringify()` number serialization rules.
- Negative zero (`-0` or `-0.0`) **must serialize as `0`**.
- Exponential notation rules: exponents must use lowercase `e`, no leading zeros in the exponent, and `+` must not be used (e.g., `1e20`, not `1e+20`).

---

## 3. Canonical Decimal Identifiers & The 64-Bit Integer Boundary

### 3.1 The Floating-Point Ingestion Risk
JavaScript's `Number` type is an IEEE-754 double-precision float. The maximum safe integer is:
$$\text{Number.MAX\_SAFE\_INTEGER} = 2^{53} - 1 = 9{,}007{,}199{,}254{,}740{,}991$$

GitHub numeric identifiers frequently exceed or monotonically approach this boundary:
- Workflow Run IDs (e.g. `40144852713` currently fits in 53 bits, but increases monotonically across all GitHub Actions).
- GitHub Artifact IDs and Database Node IDs.
- High-range integer timestamps or internal hashes.

If an unhardened JSON parser parses a 64-bit integer exceeding $2^{53}-1$ as an IEEE-754 double, the value is silently rounded to the nearest representable float, **altering the numeric identifier without warning and invalidating cryptographic digests**.

### 3.2 Canonical Decimal String Specification & Validation Roles
To eliminate ambiguity, precision loss, and representation mismatch across all platforms:
1. **String Type:** All external GitHub numeric identifiers (`github_rest_id`, `run_id`, `artifact_id`) **must be encoded as JSON strings**, never JSON numbers.
2. **Lexical Regex Role (`^[1-9][0-9]{0,19}$`):**
   - Strictly enforces canonical formatting: positive integer, non-zero, no leading zeros (e.g., `"0123"` is rejected), no negative signs (`"-1"`), no decimal points (`"1.0"`), no exponents (`"1e5"`), no whitespace (`" 123 "`), and maximum 20 digits.
   - **Scope Clarification:** The lexical regex alone **does not** enforce the exact numerical upper bound of $2^{64}-1$. 20-digit strings like `"18446744073709551616"` ($2^{64}$) and `"99999999999999999999"` match the regex pattern.
3. **Mandatory Semantic Bound (`uint64::MAX`):**
   - The engine implementation contract mandates an explicit semantic range check $[1, 18{,}446{,}744{,}073{,}709{,}551{,}615]$.
   - Any value exceeding $\text{uint64::MAX}$ must be rejected fail-closed during semantic verification.
   - Tested by explicit adversarial fixtures: `input_invalid_uint64_overflow_edge.json` ($2^{64}$) and `input_invalid_uint64_twenty_nines.json`.
4. **No Intermediate Float Parsing:** Consumers and engine runtimes must parse these identifiers directly into 64-bit unsigned integers or strings, never converting through IEEE-754 floating-point representations.

---

## 4. Date-Time Validation Semantics & UTC Enforcement

In JSON Schema Draft 2020-12, the `format` keyword functions as an informative annotation by default unless a validator explicitly enables assertion mode. Furthermore, standard ISO-8601 strings allow arbitrary local timezone offsets (e.g. `+03:00`, `-05:00`).

To ensure strict, fail-closed validation:
1. **Enforced Assertion:** Validator implementations in GOV-01 must explicitly enable format assertion mode (`validate_formats = true`).
2. **Pattern Backup:** To guarantee rejection even when a third-party validator runs in annotation-only mode, all timestamp fields in `checkpoint-input.schema.json` and `assessment-output.schema.json` enforce the following explicit regex:
   ```regex
   ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?Z$
   ```
3. **Strict UTC Semantics:**
   - Only timestamps ending in literal uppercase `Z` are accepted.
   - Non-UTC offset notations (such as `+00:00`, `-07:00`, or lowercase `z`) are rejected.
   - Nonexistent calendar dates (e.g., `2026-02-30T12:00:00Z`) must be rejected fail-closed during semantic verification.

---

## 5. The Two-Schema Architecture and Outer Canonical Envelope

### 5.1 Clear Two-Schema Separation
`Evidence-Checkpoint-Engine` defines two distinct, formal Draft 2020-12 schemas:
1. **`checkpoint-input.schema.json` (Inner Payload Schema):**
   Defines the structure of the checkpoint metadata payload (repository identity, commit SHA, Git hash algorithm, producer, governance, and CI evidence).
2. **`checkpoint-envelope.schema.json` (Outer Canonical Envelope Schema):**
   Wraps the checkpoint input payload with its RFC 8785 content-addressed SHA-256 digest:
   ```json
   {
     "$schema": "https://json-schema.org/draft/2020-12/schema",
     "$id": "https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/schemas/v0.1.0/checkpoint-envelope.schema.json",
     "title": "Evidence Checkpoint Canonical Envelope Contract (Draft R2)",
     "type": "object",
     "additionalProperties": false,
     "required": ["envelope_version", "payload", "envelope_manifest_digest"],
     "properties": {
       "envelope_version": { "type": "string", "enum": ["0.1.0-draft"] },
       "payload": { "$ref": "checkpoint-input.schema.json" },
       "envelope_manifest_digest": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
     }
   }
   ```

### 5.2 Verifier Entry Point Semantics
1. **Primary Verifier Entry Point (Production Verification):**
   - **Requires Envelope:** Accepts only documents conforming to `checkpoint-envelope.schema.json`.
   - **Rejection of Bare Payloads:** If a caller submits a bare payload directly to the primary verifier entry point, the engine rejects the request fail-closed with Exit Code `3` (`REFUSED`), requiring an authenticated envelope.
2. **Declaration / Linting Entry Point (Candidate Inspection):**
   - Can inspect a bare candidate payload (`checkpoint-input.schema.json`) for syntax and schema conformance.
   - Emits an explicit warning that the input is an unauthenticated, unverified candidate declaration (`SOURCE_DECLARED`).

### 5.3 Deterministic Preimage and Digest Specification
1. **Preimage Input:** The exact JSON object value associated with the outer envelope's `payload` key.
2. **Field Inclusions / Exclusions:** 100% of the `payload` fields are included. Zero field exclusions are performed. Outer envelope fields (`envelope_version`, `envelope_manifest_digest`) are strictly external to `payload` and are never part of the preimage.
3. **Serialization:** The `payload` object is serialized into raw UTF-8 bytes using RFC 8785 (JCS).
4. **Digest Calculation:** SHA-256 is computed over the RFC 8785 bytes.
5. **Digest String Format:** `"sha256:<64-lowercase-hexadecimal-characters>"`.
6. **Comparison:** Constant-time comparison between the computed digest string and `envelope_manifest_digest`. Any discrepancy fails closed with `ASSESSED_FAIL` ($D_{\text{integrity}} = \text{INVALID}$, Exit Code `2`).
7. **No Self-Referential / Recursive Hashing:** The digest is never computed over the envelope itself, eliminating recursive hashing paradoxes.
8. **Reproducible Test Vectors:**
   - Positive Synthetic Envelope: [`docs/gov-00/fixtures/envelope_valid_bootstrap_checkpoint.json`](fixtures/envelope_valid_bootstrap_checkpoint.json) contains the verified RFC 8785 digest:
     `sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d`.
   - Negative Tampered Envelope: [`docs/gov-00/fixtures/envelope_invalid_tampered_digest.json`](fixtures/envelope_invalid_tampered_digest.json).
   - **Content Consistency vs. Provenance:** A matching envelope digest establishes deterministic cryptographic content consistency between the envelope manifest and its payload. It does **not** establish author identity, origin provenance, or evaluation authority without external cryptographic signatures or trusted attestations.

---

## 6. Authoritative CI-Policy Provenance & Expected Checks

### 6.1 The Policy Provenance Hierarchy
A major vulnerability in naive provenance manifests is allowing the producer to dictate its own `expected_jobs`. To establish true independent verification, the engine distinguishes five levels of policy evidence:

```
+-------------------------------------------------------------+
| 1. Developer-Proposed Policy Reference                      |
| (Declared path/ID in manifest: SOURCE_DECLARED, untrusted)  |
+-------------------------------------------------------------+
                              |
                              v (Matched against Root of Trust)
+-------------------------------------------------------------+
| 2. Independently Trusted Policy Identity                    |
| (Pre-configured approved policy reference in repository)     |
+-------------------------------------------------------------+
                              |
                              v (Content Digest Verification)
+-------------------------------------------------------------+
| 3. Authenticated Policy Contents                            |
| (SHA-256 digest matches expected_policy_digest)             |
+-------------------------------------------------------------+
                              |
                              v (Extract Required Check Names)
+-------------------------------------------------------------+
| 4. Authoritative Expected Checks List                       |
| (Mandatory checks required for verification pass)           |
+-------------------------------------------------------------+
                              |
                              v (Compare against CI Observations)
+-------------------------------------------------------------+
| 5. Actual CI Observation Evaluation                         |
| (100% of expected checks must have status: completed and    |
| conclusion: success on the exact checkout SHA)              |
+-------------------------------------------------------------+
```

### 6.2 Trust Boundary Invariants
1. **Producer Digest is Not Authority:** Supplying `expected_policy_digest` in the manifest does **not** authenticate the policy. The verifier must verify that the policy digest matches a locally pinned or authoritatively approved baseline.
2. **Missing Policy Rejection:** In `checkpoint-input.schema.json`, `expected_policy_ref` and `expected_policy_digest` are **mandatory** whenever `ci_evidence` is provided. Submitting CI checks without policy references fails schema validation (`input_invalid_ci_missing_policy.json`).
3. **Offline Policy Handling:** If the verifier operates offline and cannot resolve the trusted policy root, it cannot assign `VERIFIED_PASS`. It fails closed to `INSUFFICIENT_EVIDENCE` (Exit Code `3`).
4. **Untrusted Policy Rejection:** If the policy digest does not match the approved baseline, the verifier emits `ASSESSED_FAIL` (Exit Code `2`, `input_invalid_ci_untrusted_policy_digest.json`).

---

## 7. Git Hash Agility: SHA-1 versus SHA-256

Git repositories support two object hash algorithms: **SHA-1** (default) and **SHA-256** (Object Format 2). The verifier schema explicitly models both, binding algorithm to character length:

| Declared Algorithm | Valid Identifier Length | Hexadecimal Regex Pattern |
|---|---|---|
| `sha1` | Exactly 40 characters | `^[0-9a-f]{40}$` |
| `sha256` | Exactly 64 characters | `^[0-9a-f]{64}$` |

The schema uses conditional sub-schemas (`allOf` with `if`/`then`) to enforce that:
- When `source.git_hash_algorithm` is `"sha1"`, both `commit_sha` and optional `tree_hash` must match `^[0-9a-f]{40}$`.
- When `source.git_hash_algorithm` is `"sha256"`, both `commit_sha` and optional `tree_hash` must match `^[0-9a-f]{64}$`.
- Any mismatch fails schema validation immediately (fixtures: `input_invalid_git_sha1_with_sha256_length.json` and `input_invalid_git_sha256_with_sha1_length.json`).

---

## 8. Artifact Digest Binding

For all build outputs and downloadable archives in `artifact_evidence`:
- When `digest_algorithm` is `"sha256"`, `digest_value` must be exactly 64 lowercase hexadecimal characters (`^[0-9a-f]{64}$`).
- When `digest_algorithm` is `"sha512"`, `digest_value` must be exactly 128 lowercase hexadecimal characters (`^[0-9a-f]{128}$`).
- Producer manifests cannot declare `observed_availability`; artifact availability is strictly an engine-observed dimension ($D_{\text{availability}}$).
