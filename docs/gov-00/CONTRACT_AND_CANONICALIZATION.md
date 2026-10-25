# Contract Specification and Canonical Serialization (RFC 8785)

> **Milestone:** `GOV-00 R1`
> **Status:** `PROPOSED SPECIFICATION / REMEDIATED DRAFT FOR REVIEW`
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

GitHub numeric identifiers can exceed this boundary:
- Workflow Run IDs (e.g. `40144852713` currently fits in 53 bits, but increases monotonically across all GitHub Actions).
- GitHub Artifact IDs and Database Node IDs.
- Potential 64-bit integer timestamps or internal hashes.

If an unhardened JSON parser parses a 64-bit integer exceeding $2^{53}-1$ as an IEEE-754 double, the value is silently rounded to the nearest representable float, **altering the numeric identifier without warning and invalidating cryptographic digests**.

### 3.2 Canonical Decimal String Specification
To eliminate ambiguity, precision loss, and representation mismatch across all platforms:
1. **String Type:** All external GitHub numeric identifiers (`github_rest_id`, `run_id`, `artifact_id`) **must be encoded as JSON strings**, never JSON numbers.
2. **Canonical Pattern:** Identifiers must strictly conform to regex `^[1-9][0-9]{0,19}$`.
   - **No leading zeros:** Strings like `"0123"` are strictly forbidden.
   - **Non-zero:** System IDs are positive non-zero integers.
   - **Upper bound:** Maximum 20 digits, bounded by the maximum unsigned 64-bit integer ($\text{uint64::MAX} = 18{,}446{,}744{,}073{,}709{,}551{,}615$). Values exceeding $\text{uint64::MAX}$ fail schema validation.
3. **No Intermediate Float Parsing:** Downstream consumers (including JavaScript/TypeScript UIs) must parse these fields as strings or native `BigInt` types, never via `Number()` or standard floating-point `JSON.parse` conversions.

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

## 5. Normative Digest Calculation: The Outer Envelope Protocol

### 5.1 Rejection of In-Document Exclusion
Previous drafts considered two hashing mechanisms: an external envelope `{payload, digest}` versus an in-document field exclusion (calculating a digest over a document while omitting `integrity.envelope_manifest_digest`).

**Normative Decision in R1:** The in-document exclusion mechanism is **formally rejected**. It introduces:
- Risk of ambiguous field exclusion paths across nested schemas.
- Brittle parser edge-cases if serializers omit or reorder the excluded property.
- Silent fallback bugs where unauthenticated documents are mistaken for authenticated documents.

### 5.2 The Outer Envelope Protocol (Single Normative Mechanism)
All evidence checkpoints submitted for cryptographic verification must use the **External Canonical Envelope**:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "envelope_version": "0.1.0-draft",
  "payload": {
    "schema_version": "0.1.0-draft",
    "checkpoint_id": "ece-bootstrap-00-baseline",
    "phase": "BOOTSTRAP-00",
    "timestamp": "2026-10-25T03:37:58Z",
    "repository": {
      "owner": "ErenTahiroglu",
      "name": "Evidence-Checkpoint-Engine",
      "github_rest_id": "1432704795"
    },
    "source": {
      "commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02",
      "git_hash_algorithm": "sha1",
      "branch": "development"
    },
    "producer": {
      "declared_by": "Eren Tahiroglu",
      "role": "developer"
    },
    "integrity": {
      "canonicalization_algorithm": "RFC8785",
      "digest_algorithm": "sha256"
    }
  },
  "envelope_manifest_digest": "sha256:d5a1b3..."
}
```

### 5.3 Deterministic Preimage Specification
1. **Preimage Input:** The exact JSON object value associated with the outer envelope's `payload` key.
2. **Field Inclusions / Exclusions:** 100% of the `payload` fields are included. Zero field exclusions are performed.
3. **Serialization:** The `payload` object is serialized into bytes using RFC 8785 (JCS).
4. **Preimage Bytes:** The raw UTF-8 bytes output by RFC 8785 canonicalization.
5. **Digest Algorithm:** SHA-256.
6. **Digest String Format:** `"sha256:<64-lowercase-hexadecimal-characters>"`.
7. **Schema Version Binding:** The inner payload explicitly declares `"schema_version": "0.1.0-draft"`. If the schema version is unrecognized, the verifier refuses processing before digest evaluation.
8. **Unknown Fields:** The payload schema enforces `additionalProperties: false`. Unknown fields result in immediate schema rejection.
9. **Future Cryptographic Signatures:** In later milestones (GOV-04), cryptographic signatures sign the raw 32-byte binary SHA-256 digest of the canonical `payload` bytes, directly referencing `envelope_manifest_digest`.

---

## 6. Git Hash Agility: SHA-1 versus SHA-256

### 6.1 Algorithmic Invariants
Git repositories support two object hash algorithms: **SHA-1** (default) and **SHA-256** (Object Format 2). The verifier schema explicitly models both, binding algorithm to character length:

| Declared Algorithm | Valid Identifier Length | Hexadecimal Regex Pattern |
|---|---|---|
| `sha1` | Exactly 40 characters | `^[0-9a-f]{40}$` |
| `sha256` | Exactly 64 characters | `^[0-9a-f]{64}$` |

### 6.2 Schema Enforcement
The schema uses conditional sub-schemas (`allOf` with `if`/`then`) to enforce that:
- When `source.git_hash_algorithm` is `"sha1"`, both `commit_sha` and optional `tree_hash` must match `^[0-9a-f]{40}$`.
- When `source.git_hash_algorithm` is `"sha256"`, both `commit_sha` and optional `tree_hash` must match `^[0-9a-f]{64}$`.
- Any mismatch (e.g. `sha1` with 64 hex characters or `sha256` with 40 hex characters) fails schema validation immediately.

### 6.3 Separation from Artifact Digests
- **Git Commit SHA:** Formed by hashing Git header `commit <size>\0` followed by commit tree, parents, author, and committer information.
- **Artifact SHA-256:** Formed by hashing raw file bytes directly.
- The verifier **never equates a 40-character Git SHA with a 64-character artifact hash**, nor assumes that a syntactically valid hash proves Git object existence or repository ownership.

---

## 7. Artifact Digest Binding

For all build outputs and downloadable archives in `artifact_evidence`:
- When `digest_algorithm` is `"sha256"`, `digest_value` must be exactly 64 lowercase hexadecimal characters (`^[0-9a-f]{64}$`).
- When `digest_algorithm` is `"sha512"`, `digest_value` must be exactly 128 lowercase hexadecimal characters (`^[0-9a-f]{128}$`).
- Any attempt to declare `sha256` with a 128-char string or `sha512` with a 64-char string is rejected at schema level.

---

## 8. Authoritative Test Vectors (RFC 8785 Section 3)

The following test vectors from RFC 8785 Section 3 serve as the baseline for canonicalization verification in GOV-01:

### Vector 1: Property Sorting (UTF-16 Code Units)
- **Input JSON:**
  ```json
  {"\u20ac": "Euro", "b": 2, "a": 1, "\ud83d\ude00": "Grinning Face"}
  ```
- **Canonical JCS Output (UTF-8 bytes):**
  ```json
  {"a":1,"b":2,"\u20ac":"Euro","\ud83d\ude00":"Grinning Face"}
  ```
- **Rationale:** `a` (U+0061) < `b` (U+0062) < `\u20ac` (U+20AC) < `\ud83d\ude00` (High surrogate U+D83D).

### Vector 2: Number Formatting and Zero
- **Input JSON:**
  ```json
  {"zero": -0, "float": 1.000000, "exp": 1e+20, "small": 0.0000001}
  ```
- **Canonical JCS Output:**
  ```json
  {"exp":1e+20,"float":1,"small":1e-7,"zero":0}
  ```
- **Rationale:** `-0` converts to `0`; `1.000000` converts to `1`; `0.0000001` formats as `1e-7`.
