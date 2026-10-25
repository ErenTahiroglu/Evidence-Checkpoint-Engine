# Engineering Roadmap and Milestone Gates

> **Status Labels:** `PLANNED`, `IN_PROGRESS`, `READY_FOR_REDTEAM`, `ACCEPTED_FOR_SCOPE`, `BLOCKED`.
> Issue closure and roadmap labels represent operational coordination, not cryptographic proof of correctness.

## Phased Execution Matrix

| Gate | Scope & Deliverables | Entry Dependencies | Acceptance & Exit Criteria |
|---|---|---|---|
| **BOOTSTRAP-00** | Initial repository setup, PolyForm Noncommercial 1.0.0 licensing, branch governance (`main` + `development`), planning docs, issue initialization. | Initial approved brief & checksum verification. | Clean repository, matching remote branch SHAs, verified documentation, no code execution. (**CURRENT**) |
| **GOV-00** | Architecture specification, technology evaluation ADR (Rust vs Go vs Python), JCS canonical serialization evaluation, threat model baseline. | BOOTSTRAP-00 | Formal ADR sign-off, schema draft, independent architecture review. Strictly no verifier code prior to this gate. |
| **GOV-01** | Deterministic offline verification core, CLI interface, schema validation, exit code specifications, negative test suite. | GOV-00 | 100% pass on negative & adversarial test fixtures; offline determinism verified; independent code review. |
| **GOV-02** | GitHub read-only evidence adapter, exact repository ID + commit SHA binding, run/attempt distinction, job list comparison. | GOV-01 | Real-world single-repository pilot validation against manual truth; fail-closed behavior on API errors; no write operations. |
| **GOV-03** | Reusable CI workflow integration, least-privilege token boundary (`contents: read`), check enforcement. | GOV-02 | Tested in isolated CI runs; green on valid fixtures, red on forged/tampered checks; zero ambient write privileges. |
| **GOV-04** | Long-term evidence archive, serialization, retention tracking, expiration flags, restoration validation. | GOV-03 | Byte-exact round-trip archive restoration; expired evidence explicitly identified; hostile archive rejection tested. |
| **GOV-05** | Host-neutral agent operating guidelines, thin Claude/Codex client adapters, agent boundary enforcement. | GOV-01 | Proof that agents cannot self-certify or inject `accepted=true` assertions into verification outcomes. |
| **GOV-06** | Versioned external integration contract and consumer test suite for AI-Toolchain-Manager (#33, #34). | GOV-01 | Verification that consumer UI cannot modify or fabricate verifier states; fail-closed rejection of unknown versions. |
| **SEC-01** | Adversarial tests for identity confusion, commit spoofing, superseded run attempts, and stale PASS reuse. | Continuous (from BOOTSTRAP-00) | All P0/P1 attack vectors covered by automated regression tests in CI; signed Red Team report. |
| **SEC-02** | Adversarial tests for hostile inputs (zip bombs, directory traversal, oversized payloads, credential leaks). | Continuous (from BOOTSTRAP-00) | Comprehensive parser fuzzing and negative payload rejection verified; zero secrets permitted in evidence. |

---

## MVP Cut

The initial Minimum Viable Product focuses on:
`BOOTSTRAP-00` -> `GOV-00` -> `GOV-01` -> tightly bounded `GOV-02` pilot.

Durable archival infrastructure (GOV-04), multi-agent handoffs (GOV-05), and broad consumer dashboards (GOV-06) remain planned future expansions and will not be pre-implemented before the core verification engine is proven.
