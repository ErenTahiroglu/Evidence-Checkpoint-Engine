# GOV-00 Comprehensive Architecture Report (Revision R1)

> **Milestone:** `GOV-00 R1` — Independent Red Team Findings Remediation
> **Status:** `PROPOSED / READY FOR INDEPENDENT RED TEAM REVIEW`
> **Date:** `2026-10-25`
> **Repository:** `ErenTahiroglu/Evidence-Checkpoint-Engine` (REST ID: `1432704795`)
> **Target Branch:** `development` (Frozen baseline: `main` at `bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02`)
> **Expected Development Baseline:** `8cd92784792f6ab9ab45e4b65920bf0f2f55b6a9`
> **Primary Tracking Issue:** [#2 (GOV-00)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/2)

---

## 1. Executive Summary

The `Evidence-Checkpoint-Engine` project was conceived to solve a critical vulnerability in modern autonomous AI-assisted software engineering: **the collapse of evidence into unverified assertions**. When automated agents, CI status badges, or developer PR comments self-declare that a checkpoint is "verified," they introduce severe risks of identity confusion, replayed past successes, and silent omissions.

Milestone **GOV-00 R1** remediates independent Red Team findings identified on the initial GOV-00 proposal. This report synthesizes:
1. **Technology Selection & Trust Boundary (ADR-006):** Evaluates Rust, Go, Python, and TypeScript, provisionally recommending **Rust** as a standalone CLI subprocess with controlled standard I/O pipes. Softens absolute safety claims and evaluates the subprocess trust boundary against in-process Tauri embedding.
2. **Canonical Serialization & Integrity:** Selects the **Outer Envelope Protocol** as the single normative standard, enforces canonical decimal strings for 64-bit IDs, binds Git object hashes to exact hex lengths, and defines strict UTC date-time validation.
3. **Multi-Dimensional Trust Model:** Defines the six-dimensional orthogonal state space, eliminates phantom enum states (`NOT_EVALUATED_OFFLINE`), defines strict cross-dimensional fail-closed precedence, and binds exit codes directly to verification verdicts (0..4).
4. **Input/Output Contracts:** Declarative JSON Schemas (Draft 2020-12) strictly separating unverified source claims (`checkpoint-input.schema.json`) from authenticated machine-assessed findings (`assessment-output.schema.json`).
5. **Security & Threat Model Delta:** Expands the threat register to 17 concrete attack classes (RT-01 through RT-17), including cross-repository CI substitution, algorithm/length mismatch, and check omission risk.
6. **Cross-Platform Target Matrix:** Corrects obsolete runner definitions by replacing retired `macos-13` with `windows-2025`, `macos-26`, `macos-26-intel`, and `ubuntu-24.04`, while explicitly deferring Windows ARM64.
7. **Consumer Boundary:** Isolates [AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager) (#32–#35) as a strictly read-only consumer with zero verifier authority.

> **CRITICAL BOUNDARY:**
> This milestone is **strictly architecture and documentation**. No verification engine, CLI binary, API collector, GitHub Actions workflow, or background service has been implemented.

---

## 2. Architectural Blueprint & Component Boundaries

The engine is structured as a **modular monolith** with clear internal separation of concerns:

```
+-------------------------------------------------------------------------------+
|                      Evidence-Checkpoint-Engine (Standalone)                  |
|                                                                               |
|  +-------------------------------------------------------------------------+  |
|  |                    1. Offline Verification Core                         |  |
|  |  - Input parser (size-bounded max 10MB, depth-bounded max 32)           |  |
|  |  - JSON Schema Draft 2020-12 validation (additionalProperties: false)   |  |
|  |  - RFC 8785 JCS canonicalization engine                                 |  |
|  |  - SHA-256 digest calculation & normative outer envelope verification   |  |
|  |  - Multi-dimensional trust state evaluator with fail-closed precedence |  |
|  +-------------------------------------------------------------------------+  |
|                                       ^                                       |
|                                       | (Internal boundary)                   |
|  +-------------------------------------------------------------------------+  |
|  |                    2. Standalone CLI & Subprocess Interface             |  |
|  |  - Structured stdin/stdout JSON pipe                                    |  |
|  |  - Deterministic exit code contract (0=Pass, 1=Limits, 2=Fail, 3=Refuse)|  |
|  |  - Zero ambient network access required                                 |  |
|  +-------------------------------------------------------------------------+  |
|                                       ^                                       |
|                                       | (Optional extension - GOV-02)         |
|  +-------------------------------------------------------------------------+  |
|  |                    3. GitHub Read-Only Adapter                          |  |
|  |  - Immutable Repo ID + commit SHA binding                               |  |
|  |  - Workflow Run ID + Run Attempt matching                               |  |
|  |  - Independent check authority policy verification                      |  |
|  |  - Strictly read-only tokens; fail-closed on API throttling             |  |
|  +-------------------------------------------------------------------------+  |
+-------------------------------------------------------------------------------+
                                        |
                                        v (Decoupled read-only pipe)
+-------------------------------------------------------------------------------+
|                        Downstream Consumer Boundary                           |
|  - AI-Toolchain-Manager (Tauri / Rust frontend - ATM-GOV-01 #33, ATM-GOV-02 #34) |
|  - Subprocess pipe invocation; strictly read-only display terminal            |
|  - Zero authority to mint, promote, or override                               |
+-------------------------------------------------------------------------------+
```

---

## 3. Technology Selection Summary (ADR-006)

Full details are documented in [ADR_RUNTIME_AND_COMPONENTS.md](ADR_RUNTIME_AND_COMPONENTS.md).

- **Provisional Recommendation:** **Rust**
  - Score: **4.375 / 5.00**
  - Rationale: High spatial and temporal memory safety in safe code; produces compact, standalone native binaries; excellent Draft 2020-12 schema and RFC 8785 canonicalization crates; robust enum sum types.
  - Caveats: Compile-time memory safety does not eliminate algorithmic DoS or logic bugs; third-party `unsafe` dependencies require strict auditing; operating system dynamic library dependencies exist on macOS and Windows.
- **Process Boundary Recommendation:** Separately versioned CLI subprocess invoked via standard input/output pipes. In-process Tauri embedding is deferred to avoid shared address space and panic risks.
- **Status:** Remains **PROPOSED** pending formal repository owner approval.

---

## 4. Trust Dimensions & Authority Separation Summary

Full details are documented in [TRUST_AND_STATE_MODEL.md](TRUST_AND_STATE_MODEL.md).

### The Four Authority Tiers:
1. `SOURCE_DECLARED`: Unverified statements by developers or AI agents (untrusted input).
2. `MACHINE_ASSESSED`: Cryptographic and structural properties evaluated by the verifier core.
3. `INDEPENDENTLY_REVIEWED`: Scoped audit findings signed by an independent reviewer.
4. `HUMAN_AUTHORIZED`: Formal governance decisions made exclusively by the owner.

### The Six Orthogonal Dimensions:
- **$D_{\text{integrity}}$ (Evidence Integrity):** `VALID` | `INVALID` | `CORRUPT` | `TAMPERED` | `NOT_EVALUATED`
- **$D_{\text{ci}}$ (CI Execution):** `VERIFIED_PASS` | `FAILED` | `INCOMPLETE_SKIPPED` | `ATTEMPT_MISMATCH` | `UNVERIFIED_RUN` | `NOT_APPLICABLE`
- **$D_{\text{availability}}$ (Evidence Availability):** `AVAILABLE` | `PARTIALLY_AVAILABLE` | `EXPIRED` | `UNREACHABLE` | `NOT_PROVIDED`
- **$D_{\text{review}}$ (Review Status):** `INDEPENDENT_REVIEW_PASS` | `STATIC_PASS_WITH_LIMITATIONS` | `SELF_REPORT_ONLY` | `REVIEW_FAILED` | `UNREVIEWED`
- **$D_{\text{authorization}}$ (Authorization):** `EXPLICITLY_AUTHORIZED` | `AUTHORIZATION_PENDING` | `REJECTED` | `REVOKED`
- **$D_{\text{freshness}}$ (Freshness):** `CURRENT_HEAD` | `HISTORICAL_BASELINE` | `STALE_DIVERGED` | `UNKNOWN_HEAD`

### Precedence Rule:
$$\text{FAILED} > \text{REFUSED} > \text{UNKNOWN / INSUFFICIENT\_EVIDENCE} > \text{ASSESSED\_LIMITATIONS} > \text{ASSESSED\_PROVISIONAL\_PASS}$$

---

## 5. Canonical Serialization (RFC 8785) & Outer Envelope Standard

Full details are documented in [CONTRACT_AND_CANONICALIZATION.md](CONTRACT_AND_CANONICALIZATION.md).

- **Standard:** RFC 8785 JSON Canonicalization Scheme (JCS).
- **Normative Mechanism:** Outer Canonical Envelope `{payload, envelope_manifest_digest}`. Zero field exclusions are performed on the payload. In-document field exclusion is formally rejected.
- **Canonical Decimal Strings:** All GitHub IDs (`github_rest_id`, `run_id`, `artifact_id`) are encoded as non-zero strings matching `^[1-9][0-9]{0,19}$` up to uint64::MAX to eliminate $2^{53}-1$ float rounding.
- **Git Hash Agility:** Strict binding between `git_hash_algorithm` and hexadecimal character length (SHA-1 = 40 hex chars; SHA-256 = 64 hex chars).
- **Date-Time Enforcement:** Strict UTC ISO-8601 regex pattern paired with format assertion.

---

## 6. Threat Model Highlights & Adversarial Fixtures

Full details are documented in [SECURITY_AND_THREAT_DELTA.md](SECURITY_AND_THREAT_DELTA.md).

- **Seventeen Concrete Threats Analyzed:** RT-01 through RT-17 covering identity confusion, forged runs, self-certification, parser attacks, expired artifacts, history rewrites, integer rounding, cross-repo CI substitution, algorithm/length mismatches, and check omission risk.
- **Twelve Test Fixtures Prepared:** Located in `docs/gov-00/fixtures/`, providing verifiable test vectors for positive bootstrap validation and negative fail-closed verification.

---

## 7. Cross-Platform Strategy & Planned CI Matrix

Full details are documented in [MVP_AND_INTEGRATION_GATES.md](MVP_AND_INTEGRATION_GATES.md).

- **Target Platforms:** Windows 11 (`x86_64`, Mandatory), macOS Apple Silicon (`arm64`, Mandatory), macOS Intel (`x86_64`, Compatibility), Linux (`x86_64`, Mandatory Headless). Windows ARM64 is deferred.
- **Architecture-Matched Runners (Planned for GOV-01):** `windows-2025`, `macos-26`, `macos-26-intel`, `ubuntu-24.04`.
- **Scope Restriction:** Zero CI workflows created in GOV-00 R1.

---

## 8. Unresolved Architectural Decisions (Owner Action Required)

The following items are formal recommendations requiring owner authorization:
1. **Approval of ADR-006:** Confirm Rust as the primary implementation language for GOV-01.
2. **Approval of Draft Contract Schemas:** Formally baseline `checkpoint-input.schema.json` and `assessment-output.schema.json`.
3. **Authorization to Unblock GOV-01:** Upon satisfactory completion of independent Red Team review, authorize the opening of Issue #3 for implementation.
