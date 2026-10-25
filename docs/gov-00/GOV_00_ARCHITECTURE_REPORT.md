# GOV-00 Comprehensive Architecture Report

> **Milestone:** `GOV-00` — Architecture, Trust Model, Technology Selection & Security Contract
> **Status:** `PROPOSED / READY FOR INDEPENDENT RED TEAM REVIEW`
> **Date:** `2026-10-25`
> **Repository:** `ErenTahiroglu/Evidence-Checkpoint-Engine` (REST ID: `1432704795`)
> **Target Branch:** `development` (Frozen baseline: `main` at `bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02`)
> **Primary Tracking Issue:** [#2 (GOV-00)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/2)

---

## 1. Executive Summary

The `Evidence-Checkpoint-Engine` project was conceived to solve a critical vulnerability in modern autonomous AI-assisted software engineering: **the collapse of evidence into unverified assertions**. When automated agents, CI status badges, or developer PR comments self-declare that a checkpoint is "verified," they introduce severe risks of identity confusion, replayed past successes, and silent omissions.

Milestone **GOV-00** establishes the formal architectural foundation for a standalone, evidence-bound verification engine. This report synthesizes primary-source research into:
1. **Technology Selection:** Comparative evaluation of Rust, Go, Python, and TypeScript, provisionally recommending **Rust** as the primary runtime.
2. **Canonical Serialization & Integrity:** Complete specification of RFC 8785 (JCS) enforcement, 64-bit integer precision boundaries, and the two-stage envelope digest protocol.
3. **Multi-Dimensional Trust Model:** Replacement of the naive 1D PASS/FAIL model with a **six-dimensional orthogonal state space** and four separated authority tiers.
4. **Input/Output Contracts:** Declarative JSON Schemas (Draft 2020-12) strictly separating unverified source claims (`checkpoint-input.schema.json`) from machine-assessed findings (`assessment-output.schema.json`).
5. **Security & Threat Model Delta:** Expansion of the Red Team threat register (RT-01 through RT-14), mapped to concrete adversarial test fixtures and tracking issues [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) and [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10).
6. **Consumer Boundary:** Clear isolation preserving [AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager) (#32–#35) as a strictly read-only consumer with zero verifier authority.

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
|  |  - Input parser (size-bounded, depth-bounded)                           |  |
|  |  - JSON Schema Draft 2020-12 validation (additionalProperties: false)   |  |
|  |  - RFC 8785 JCS canonicalization engine                                 |  |
|  |  - SHA-256 digest calculation & envelope verification                   |  |
|  |  - Multi-dimensional trust state evaluator                              |  |
|  +-------------------------------------------------------------------------+  |
|                                       ^                                       |
|                                       | (Internal boundary)                   |
|  +-------------------------------------------------------------------------+  |
|  |                    2. Standalone CLI & Library Interface                |  |
|  |  - Structured stdout JSON output                                        |  |
|  |  - Deterministic exit code contract (0..4)                              |  |
|  |  - Zero ambient network access required                                 |  |
|  +-------------------------------------------------------------------------+  |
|                                       ^                                       |
|                                       | (Optional extension - GOV-02)         |
|  +-------------------------------------------------------------------------+  |
|  |                    3. GitHub Read-Only Adapter                          |  |
|  |  - Immutable Repo ID + commit SHA binding                               |  |
|  |  - Workflow Run ID + Run Attempt matching                               |  |
|  |  - Expected vs observed CI job comparison                              |  |
|  |  - Strictly read-only tokens; fail-closed on API throttling             |  |
|  +-------------------------------------------------------------------------+  |
+-------------------------------------------------------------------------------+
                                        |
                                        v (Decoupled read-only pipe)
+-------------------------------------------------------------------------------+
|                        Downstream Consumer Boundary                           |
|  - AI-Toolchain-Manager (Tauri / Rust frontend - ATM-GOV-01 #33, ATM-GOV-02 #34) |
|  - Strictly read-only consumer; zero authority to mint, promote, or override   |
+-------------------------------------------------------------------------------+
```

---

## 3. Technology Selection Summary (ADR-006)

Full details are documented in [ADR_RUNTIME_AND_COMPONENTS.md](ADR_RUNTIME_AND_COMPONENTS.md).

- **Provisional Recommendation:** **Rust**
  - Score: **4.75 / 5.00**
  - Rationale: Compile-time memory safety eliminates parser vulnerabilities against hostile inputs; produces zero-dependency, self-contained native binaries for Linux, macOS, and Windows; directly interoperates with Tauri-based downstream consumers via in-process library bindings without runtime collisions.
- **Runner-Up:** **Go**
  - Score: **4.15 / 5.00**
  - Rationale: Excellent standard library and cross-compiler, but runtime embedding inside external desktop applications (Tauri/Rust) introduces signal handling and GC overhead.
- **Rejected:** **Python** (distribution packaging fragility, dynamic typing, cannot embed cleanly) and **TypeScript/Node.js** (prototype pollution risks, npm supply-chain exposure).

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

---

## 5. Canonical Serialization (RFC 8785) & Integer Boundaries

Full details are documented in [CONTRACT_AND_CANONICALIZATION.md](CONTRACT_AND_CANONICALIZATION.md).

- **Standard:** RFC 8785 JSON Canonicalization Scheme (JCS).
- **Key Sorting:** UTF-16 code units (lexicographical order), strictly handling surrogate pairs for code points $> \text{U+FFFF}$.
- **Whitespace & Escaping:** Zero whitespace; strict minimal escaping (`\"`, `\\`, control chars); solidus `/` unescaped.
- **The 64-Bit Integer Boundary:** Because JavaScript and dynamic parsers round integers exceeding $2^{53}-1$, all GitHub IDs (`github_rest_id`, `run_id`, `artifact_id`) are mandated as **canonical decimal strings** matching regex `^[0-9]{1,20}$`.
- **Envelope Protocol:** Two-stage hashing eliminates self-referential digest paradoxes.

---

## 6. Threat Model Highlights & Adversarial Fixtures

Full details are documented in [SECURITY_AND_THREAT_DELTA.md](SECURITY_AND_THREAT_DELTA.md).

- **Fourteen Concrete Threats Analyzed:** RT-01 through RT-14 covering identity confusion, forged runs, self-certification, parser attacks, expired artifacts, history rewrites, and integer rounding.
- **Nine Test Fixtures Prepared:** Located in `docs/gov-00/fixtures/`, providing verifiable test vectors for positive validation and negative fail-closed verification.

---

## 7. Evidence Retention & Trust Anchors Evaluation

1. **Short-Term Retention (< 90 Days):** Relies on GitHub Actions artifact storage; explicitly tracked via the `Evidence Availability` dimension to flag expirations honestly.
2. **Medium-Term Retention (Active Milestones):** Small, sanitized, RFC 8785 canonicalized JSON manifests committed directly to Git in the `development` branch.
3. **Long-Term Tamper-Resistance (> 1 Year):** Acknowledges that a compromised GitHub repository account can rewrite Git history. Long-term defense requires **offsite trust anchors** (e.g. periodically publishing merkle tree roots or detached GPG signatures to an independent offsite location or immutable public timestamping service).

---

## 8. Unresolved Architectural Decisions (Owner Action Required)

The following items are formal recommendations requiring owner authorization:
1. **Approval of ADR-006:** Confirm Rust as the primary implementation language for GOV-01.
2. **Approval of Draft Contract Schemas:** Formally baseline `checkpoint-input.schema.json` and `assessment-output.schema.json`.
3. **Authorization to Unblock GOV-01:** Upon satisfactory completion of independent Red Team review, authorize the opening of Issue #3 for implementation.
