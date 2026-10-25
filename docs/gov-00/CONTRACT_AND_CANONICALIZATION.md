# Contract Specification and Canonical Serialization (RFC 8785)

> **Milestone:** `GOV-00`
> **Status:** `PROPOSED SPECIFICATION / DRAFT FOR REVIEW`
> **Primary References:** [RFC 8785 (JCS)](https://www.rfc-editor.org/rfc/rfc8785.html), [RFC 7493 (I-JSON)](https://www.rfc-editor.org/rfc/rfc7493.html), [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)

---

## 1. The Necessity of Canonical Serialization

A hash over JSON data is meaningless unless the serialization process is **100% deterministic**. Standard JSON serializers in different programming languages introduce variations in:
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

## 3. The 64-Bit Integer Precision Boundary

### The Problem
JavaScript's `Number` type is an IEEE-754 double-precision float. The maximum safe integer is:
$$\text{Number.MAX\_SAFE\_INTEGER} = 2^{53} - 1 = 9{,}007{,}199{,}254{,}740{,}991$$

GitHub numeric identifiers can exceed this boundary:
- Workflow Run IDs (e.g. `40144852713` currently fits in 53 bits, but increases monotonically across all GitHub Actions).
- GitHub Artifact IDs and Database Node IDs.
- Potential 64-bit integer timestamps or internal hashes.

If a JSON parser in JavaScript or a dynamic consumer parses a 64-bit integer exceeding $2^{53}-1$, the value is silently rounded to the nearest representable float, **altering the numeric identifier without warning and invalidating cryptographic digests**.

### The Solution: Canonical Decimal Strings
To ensure absolute determinism across all platforms and downstream consumers, **all GitHub numeric identifiers must be encoded as canonical decimal strings in the input and output schemas**:
- Repository REST ID: `"1432704795"` (String pattern: `^[0-9]{1,20}$`)
- Workflow Run ID: `"40144852713"` (String pattern: `^[0-9]{1,20}$`)
- Artifact ID: `"12345678"` (String pattern: `^[0-9]{1,20}$`)

This guarantees zero precision loss regardless of the parsing language.

---

## 4. Digest Calculation & The Envelope Protocol

### The Self-Referential Digest Trap
A document cannot contain its own digest inside itself if the digest calculation includes the entire document:
$$\text{Digest} = H(\text{Document including Digest}) \implies \text{Impossible recursive dependency}$$

### The Canonical Envelope Protocol
To authenticate a manifest while avoiding self-referential paradoxes, `Evidence-Checkpoint-Engine` defines an explicit two-stage envelope structure:

```
+-------------------------------------------------------------+
| Checkpoint Manifest Payload                                 |
| (All metadata, repository info, source SHA, CI checks)      |
+-------------------------------------------------------------+
                              |
                              v  (1. RFC 8785 Canonicalize)
                    [Canonical UTF-8 Bytes]
                              |
                              v  (2. SHA-256 Digest)
                  "envelope_manifest_digest"
                              |
                              v
+-------------------------------------------------------------+
| Checkpoint Envelope (Outer Document)                        |
| - payload: { ... canonicalized manifest payload ... }       |
| - digest: "sha256:abc123..."                                |
| - signature (optional future): "sig:..."                    |
+-------------------------------------------------------------+
```

Alternatively, when hashing a standalone document containing an `integrity` block, the verifier calculates the digest over the document **excluding the `integrity.envelope_manifest_digest` field**.

---

## 5. Distinction Between Digest, Signature, and Authority

The verifier strictly distinguishes these concepts:

| Concept | What It Actually Proves | What It Does NOT Prove |
|---|---|---|
| **Content Digest (SHA-256)** | Byte equality against a reference. | Authorship, authenticity, or correctness. |
| **Authenticated Digest** | An identified source published this hash. | That the underlying software is secure or tested. |
| **Cryptographic Signature** | The private key holder signed these bytes. | That the signer was authorized or not compromised. |
| **Verified Identity** | The key belongs to a specific entity. | That tests passed or code is safe. |
| **Human Approval** | A human made a policy decision. | That the decision was wise or verified by tests. |
| **Software Correctness** | Formal mathematical proof. | (Cannot be inferred from any signature or hash alone). |

---

## 6. Git Hash Agility: SHA-1 vs SHA-256

- Standard Git historically uses 40-character hexadecimal **SHA-1** hashes for commits, trees, and blobs.
- Git is transitioning to **SHA-256** (Object Format 2).
- The verifier schema explicitly models `git_hash_algorithm: "sha1" | "sha256"` and binds the algorithm directly to the commit SHA.
- **Critical Rule:** The engine **never equates a 40-character Git commit hash with a SHA-256 content digest**.

---

## 7. Authoritative Test Vectors (RFC 8785 Section 3)

The following test vectors from RFC 8785 Section 3 will be verified in GOV-01:

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
