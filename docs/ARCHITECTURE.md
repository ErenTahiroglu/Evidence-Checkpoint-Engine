# Architecture Proposal and Trust Boundaries (GOV-00)

> **Status: PROPOSED / BOOTSTRAP ONLY — NOT A WORKING VERIFIER.**
> The verifier is not implemented in this repository bootstrap. No checkpoint has been validated.

## Architectural Purpose

`Evidence-Checkpoint-Engine` is designed as a standalone, evidence-bound verification engine reusable across GitHub repositories and autonomous AI development agents. It establishes technical truth by evaluating verifiable evidence rather than trusting unverified claims, developer assertions, green badges, or agent self-reports.

## Component Boundaries

The architecture separates responsibilities into four clear boundaries:

```
+-----------------------------------------------------------------------+
|                     Independent Verification Core                     |
|  - Deterministic offline evidence validation & hash comparison        |
|  - Versioned machine-readable contracts (JSON schemas)                |
|  - Standalone Library and CLI interface                               |
|  - Conservative, explainable multi-dimensional outcomes               |
+-----------------------------------------------------------------------+
                                   ^
                                   | (Bounded, read-only data)
+-----------------------------------------------------------------------+
|                       GitHub Read-Only Adapter                        |
|  - Binds immutable Repo ID and full Git Commit SHA                    |
|  - Distinguishes Workflow Run ID and Run Attempt                     |
|  - Compares expected vs observed CI check suites / jobs               |
|  - Verifies artifact metadata and bounded byte downloads              |
+-----------------------------------------------------------------------+
                                   ^
                                   | (Untrusted candidate input)
+----------------------------------+------------------------------------+
|        Agent Integration         |   Future AI-Toolchain-Manager      |
|  - Host-neutral instructions     |  - Narrow read-only consumer       |
|  - Thin Claude / Codex adapters  |  - Versioned output contract       |
|  - No model/API/IDE lock-in      |  - No GUI verifier authority       |
|  - NO agent self-certification   |  - Cannot invent trusted results   |
+----------------------------------+------------------------------------+
```

### 1. Independent Verification Core
- **Deterministic evidence assessment:** Pure, reproducible evaluation without side effects or implicit ambient trust.
- **Versioned machine-readable contracts:** Explicit schema versions for inputs, evidence records, and verifier assessments.
- **Offline validation capability:** Operates entirely offline on static fixtures, local files, and cached records without network access or credentials.
- **Library and CLI interface:** Embeddable as a library or executable via CLI with predictable exit codes and machine-readable output (stdout JSON).
- **Conservative, explainable outcomes:** Fail-closed design; any gap, ambiguity, or conflict produces an explicit diagnostic rather than an optimistic PASS.

### 2. GitHub Read-Only Adapter
- **Exact identity binding:** Binds immutable GitHub repository ID and full 40-character commit SHA together.
- **Execution provenance:** Distinguishes workflow definition, run ID, and run attempt (rejecting forged or superseded runs).
- **Check verification:** Verifies observed CI jobs against explicitly expected job lists; skipped, missing, cancelled, or blocked checks never silently become PASS.
- **Artifact integrity:** Validates artifact metadata, size limits, and digest matching.
- **Least privilege:** Strictly read-only; no repository write access, issue mutation, or administrative permissions required.

### 3. Agent Integration
- **Host-neutral operating guidelines:** Common operating rules applicable to any AI agent harness.
- **Thin adaptations:** Small adapters for Claude, Codex, or other environments if justified.
- **No mandatory dependencies:** Zero dependence on any single AI model provider, proprietary API, or IDE.
- **Invariant:** **No agent may certify its own work as independently accepted.** Agent assertions remain untrusted claims.

### 4. Future AI-Toolchain-Manager Integration
- **Separate integration boundary:** AI-Toolchain-Manager remains a separate consumer repository ([AI-Toolchain-Manager #32](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/32)).
- **Versioned external contract:** Communication occurs via versioned static contracts ([INTEGRATION_CONTRACT.md](INTEGRATION_CONTRACT.md)).
- **No GUI-dependent verifier:** The verifier runs headlessly; it never depends on GUI state or desktop runtimes.
- **No GUI authority:** The manager frontend or dashboard cannot invent, upgrade, or override verification statuses.

### Explicit Bootstrap Boundaries
In this bootstrap phase:
- **No validator, collector, dashboard, daemon, network service, database, or signing infrastructure is implemented.**
- The implementation language and runtime remain open decisions pending evaluation in GOV-00.

---

## Evidence and Trust Invariants

The engine enforces fifteen non-negotiable invariants:

1. **Content vs Author Identity:** An exact Git SHA identifies Git content; it does not prove author identity or technical correctness.
2. **Repository & Commit Binding:** Repository identity (including immutable GitHub repo ID) and full commit SHA must be cryptographically and structurally bound together.
3. **Run ID and Attempt:** CI workflow run ID and run attempt must be strictly distinguished; attempt 1 success does not validate attempt 2 failure or vice-versa.
4. **CI Success vs Coverage:** A green CI status proves only that configured assertions did not fail; it does not prove test coverage or correctness.
5. **Fail-Closed Checks:** Skipped, missing, cancelled, or blocked checks cannot silently become PASS.
6. **Artifact Hashes:** Artifact hashes prove byte equality relative to a reference; they do not prove independent authenticity or safety.
7. **Issues as Coordination:** GitHub Issues, PR descriptions, and comments are mutable coordination records, never immutable proof.
8. **Claims vs Facts:** Developer and agent assertions are strictly distinct from independently checked evidence.
9. **Technical Review vs Authorization:** Independent technical review is distinct from human owner authorization.
10. **Production Acceptance:** Production acceptance is distinct from all preceding technical states (declared, assessed, reviewed).
11. **Non-Transitive Acceptance:** Accepted historical commits must not automatically confer acceptance on newer branch heads.
12. **Explicit Missing Evidence:** Expired, missing, or unavailable evidence must be represented explicitly in output models.
13. **Untrusted Input:** External reports, Issues, logs, archives, and repository contents are untrusted input.
14. **No Self-Declared PASS:** A verifier must never use user-supplied or agent-supplied `verified=true` or `accepted=true` as an authoritative result.
15. **No Verification from Absence:** Lack of evidence or failure to inspect must never be represented as successful verification.

---

## Claim Taxonomy

Claims in the checkpoint system belong to four strictly separated tiers:

| Claim Tier | Authority | Meaning |
|---|---|---|
| `SOURCE_DECLARED` | Developer / Agent / Manifest | Unverified statement; untrusted input. |
| `MACHINE_ASSESSED` | Verification Core / Adapter | Programmatically validated against defined rules and evidence bounds. |
| `INDEPENDENTLY_REVIEWED` | Independent Auditor / Red Team | Human or separate auditor review within a documented, bounded scope. |
| `HUMAN_AUTHORIZED` | Repository Owner | Explicit governance authorization to accept or promote a specific baseline SHA. |

---

## Multi-Dimensional Verification Model

The engine rejects simplistic one-dimensional PASS/FAIL outcomes. A checkpoint status is evaluated across six independent dimensions:

1. **Evidence Integrity:** Are hashes, signatures, and formats structurally sound and valid?
2. **CI Execution:** Did required checks run on the exact commit SHA and run attempt without skips?
3. **Evidence Availability:** Are referenced artifacts, logs, and evidence records currently accessible?
4. **Review Status:** Has independent technical evaluation been performed for this scope?
5. **Authorization:** Has the repository owner explicitly authorized this exact state?
6. **Freshness:** Does the evidence correspond to the current branch head, or is it a historical baseline?

---

## Technology Evaluation (Pending GOV-00 ADR)

The runtime technology decision is deferred to GOV-00:

| Language Candidate | Strengths | Trade-offs |
|---|---|---|
| **Rust** | Memory safety, zero-dependency native binaries, aligns with AI-Toolchain-Manager core | Higher implementation overhead, longer compile cycles |
| **Go** | Fast compilation, clean standard library, cross-compilation simplicity | Runtime overhead, garbage collection, dependency auditing |
| **Python** | Rapid prototyping, extensive testing libraries | Interpreter dependencies, distribution packaging burden |

Canonical serialization must follow strict rules (evaluating RFC 8785 JSON Canonicalization Scheme - JCS) to ensure deterministic hash generation across platforms.
