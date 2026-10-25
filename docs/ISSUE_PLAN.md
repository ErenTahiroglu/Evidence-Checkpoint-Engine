# GitHub Issues Plan and Execution Order

> **Status: PLANNING SPECIFICATION.**
> Issues outlined here are created during repository initialization and tracked within this repository.

## Issue Inventory and Dependency Graph

```
           +-------------------------------------------------------+
           | [GOV-00] Architecture, Tech Comparison & Threat Model | (NEXT ACTIONABLE GATE)
           +-------------------------------------------------------+
                   |                                       |
                   v                                       v
      +-------------------------+             +-------------------------+
      | [SEC-01] Identity &     |             | [SEC-02] Untrusted Data |
      | False-Verification Risk |             | & Credential Isolation  |
      +-------------------------+             +-------------------------+
                   |                                       |
                   v                                       |
      +-------------------------+                          |
      | [GOV-01] Offline Core   |<-------------------------+
      | & Deterministic CLI     |
      +-------------------------+
                   |
                   v
      +-------------------------+
      | [GOV-02] GitHub         |
      | Read-Only Adapter       |
      +-------------------------+
             /           \
            v             v
+------------------+ +------------------+
| [GOV-03] CI      | | [GOV-06] Version |
| Integration      | | Consumer Contract|
+------------------+ +------------------+
        |                     |
        v                     v
+------------------+ +------------------+
| [GOV-04] Archive | | [GOV-05] Agent   |
| & Trust Anchors  | | & Client Handoff |
+------------------+ +------------------+
```

---

## Detailed Issue Specifications

### 1. `[GOV-00] Architecture decision, data model, trust state dimensions and threat model`
- **Scope:** Complete architectural definition, technology evaluation (Rust vs Go vs Python), serialization standard (RFC 8785 JCS), schema specifications, and Red Team risk register baseline.
- **Dependencies:** BOOTSTRAP-00 completion.
- **Acceptance Criteria:** Written ADR resolving runtime language; schema definition for checkpoint contracts; multi-dimensional state model approved; complete threat model baseline.
- **Security Constraints:** Strictly no functional verifier code implementation in this phase.
- **Explicit Non-Goals:** Building production verifier binary or deploying CI automation.
- **Required Evidence:** Documentation pull request, comparative benchmark/audit matrix, independent review log.
- **Independent Review Requirement:** Formal architectural review by independent reviewer prior to owner authorization.
- **Current Status:** `PLANNED` (Next actionable architecture gate).

### 2. `[GOV-01] Offline deterministic verification core and CLI`
- **Scope:** Standalone library and CLI implementing local file evaluation, strict schema validation, deterministic hashing, and exit code contracts.
- **Dependencies:** GOV-00 acceptance.
- **Acceptance Criteria:** Deterministic evaluation of static test fixtures; negative tests rejecting corrupt, tampered, or schema-violating inputs; predictable exit codes.
- **Security Constraints:** Strictly offline execution; zero network access; size-bounded parsing.
- **Explicit Non-Goals:** GitHub API communication or network polling.
- **Required Evidence:** Automated test suite with 100% pass on negative fixture matrix; CLI verification logs.
- **Independent Review Requirement:** Red Team review of input parsers and hash routines.
- **Current Status:** `BLOCKED` (Awaiting GOV-00).

### 3. `[GOV-02] GitHub read-only evidence adapter`
- **Scope:** Read-only GitHub adapter retrieving repository identity, commit SHA, workflow runs/attempts, job statuses, and artifact metadata.
- **Dependencies:** GOV-01 acceptance.
- **Acceptance Criteria:** Accurate correlation of repo ID + commit SHA; distinction between run attempts; detection of skipped/cancelled checks.
- **Security Constraints:** Strictly read-only operations; no write tokens; fail-closed on API throttling or unknown responses.
- **Explicit Non-Goals:** Creating PR comments, modifying branch rules, or writing issues.
- **Required Evidence:** One-repository real-world pilot log compared against manual ground-truth inspection.
- **Independent Review Requirement:** Security review of GitHub API boundary and error handling.
- **Current Status:** `BLOCKED` (Awaiting GOV-01).

### 4. `[GOV-03] CI integration and security constraints`
- **Scope:** Reusable GitHub Actions workflow and CLI invocation recipes for integrating checkpoint checks into CI pipelines.
- **Dependencies:** GOV-02 acceptance.
- **Acceptance Criteria:** Automated verification runs within CI without ambient privilege escalation; proper error surfacing in CI summaries.
- **Security Constraints:** Minimal token permissions (`contents: read`); pinned action hashes; no auto-merge authorization.
- **Explicit Non-Goals:** Self-certifying production releases.
- **Required Evidence:** CI workflow run logs demonstrating clean PASS on valid commits and clear FAIL on invalid fixtures.
- **Independent Review Requirement:** Least-privilege workflow audit.
- **Current Status:** `BLOCKED` (Awaiting GOV-02).

### 5. `[GOV-04] Long-term archive, evidence recovery and trust anchors`
- **Scope:** Specification and implementation of durable evidence archival, integrity preservation, expiry tracking, and restoration verification.
- **Dependencies:** GOV-03 acceptance.
- **Acceptance Criteria:** Exported archives restore to identical byte digests; expired evidence is explicitly flagged; tamper detection is verified.
- **Security Constraints:** Cryptographic binding of archives; handling hostile zip/tar inputs (prevent traversal, bombs).
- **Explicit Non-Goals:** Building a centralized database or hosting a proprietary cloud service.
- **Required Evidence:** Archive round-trip test results and adversarial payload rejection tests.
- **Independent Review Requirement:** Cryptographic and archive security review.
- **Current Status:** `BLOCKED` (Awaiting GOV-03).

### 6. `[GOV-05] Cross-agent integration and handoff`
- **Scope:** Host-neutral operating instructions, prompt guidelines, and thin client adapters for AI agents (Claude, Codex, etc.).
- **Dependencies:** GOV-01 acceptance.
- **Acceptance Criteria:** Clear execution instructions allowing any standard AI agent to invoke the verifier CLI; strict prevention of agent self-certification.
- **Security Constraints:** Agents cannot override verifier assessments or supply self-signed `accepted=true` assertions.
- **Explicit Non-Goals:** Model-specific IDE plugins or proprietary assistant dependencies.
- **Required Evidence:** Multi-agent invocation transcripts and boundary enforcement test logs.
- **Independent Review Requirement:** Review of prompt boundaries and agent authorization constraints.
- **Current Status:** `BLOCKED` (Awaiting GOV-01).

### 7. `[GOV-06] Versioned external integration contract`
- **Scope:** Machine-readable interface contract, schema stability tests, and rejection test suite for external consumers like AI-Toolchain-Manager.
- **Dependencies:** GOV-01 acceptance.
- **Acceptance Criteria:** Versioned output schema; test suite verifying that consumer GUIs cannot alter or invent verifier states.
- **Security Constraints:** Read-only ingestion; explicit refusal of malformed, unknown, or hostile payloads.
- **Explicit Non-Goals:** Modifying AI-Toolchain-Manager codebase or creating desktop interfaces here.
- **Required Evidence:** Schema compatibility test reports and fixture validation logs.
- **Independent Review Requirement:** Consumer boundary and integration audit.
- **Current Status:** `BLOCKED` (Awaiting GOV-01).

### 8. `[SEC-01] Cross-repository identity and false-verification attack classes`
- **Scope:** Security analysis and adversarial tests targeting repository identity confusion, commit spoofing, superseded run attempts, and stale PASS reuse.
- **Dependencies:** BOOTSTRAP-00 completion.
- **Acceptance Criteria:** Test suite proving rejection of spoofed repository IDs, mismatched commit trees, and superseded run attempts.
- **Security Constraints:** Continuous tracking of attack vectors; all P0/P1 vectors must have concrete automated negative tests.
- **Explicit Non-Goals:** Penetration testing unrelated external GitHub infrastructure.
- **Required Evidence:** Adversarial test execution reports with reproducible fixtures.
- **Independent Review Requirement:** Red Team review and sign-off.
- **Current Status:** `DECISION PENDING` (Baseline established; active during all phases).

### 9. `[SEC-02] Untrusted files, archives, credentials and evidence preservation`
- **Scope:** Hardening against malicious files, zip bombs, directory traversal, credential exfiltration, and untrusted environment leaks.
- **Dependencies:** BOOTSTRAP-00 completion.
- **Acceptance Criteria:** Rigorous input boundaries (file size, recursion limits, path sanitization); verified exclusion of credentials from evidence outputs.
- **Security Constraints:** No raw environment dumps; zero secrets committed; fail-closed decompression.
- **Explicit Non-Goals:** Antivirus scanner implementation.
- **Required Evidence:** Fuzzing and adversarial fixture execution logs.
- **Independent Review Requirement:** Red Team penetration assessment of parser and file handler routines.
- **Current Status:** `DECISION PENDING` (Baseline established; active during all phases).

---

## Parent Coordination

- Architecture parent in AI-Toolchain-Manager: [#32](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/32)
- Future consumer milestones in AI-Toolchain-Manager: [#33](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/33), [#34](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/34), [#35](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/35)
