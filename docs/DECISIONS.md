# Architecture Decisions and Governance Log

| Decision ID | Area | Current Status / Resolution | Rationale and Context |
|---|---|---|---|
| ADR-001 | Repository Identity | **RESOLVED: `ErenTahiroglu/Evidence-Checkpoint-Engine`** | Standalone repository separate from AI-Toolchain-Manager to avoid self-verification coupling. |
| ADR-002 | Repository Visibility | **RESOLVED: PUBLIC** | Repository is publicly readable on GitHub to enable transparent verification and reference across public projects. |
| ADR-003 | Licensing | **RESOLVED: PolyForm Noncommercial 1.0.0** | Adopted official PolyForm Noncommercial License 1.0.0. Required Notice: `Copyright (c) 2026 Eren Tahiroğlu`. Commercial use is restricted unless separately authorized. |
| ADR-004 | Branch Governance | **RESOLVED: `main` and `development`** | Exactly two long-lived branches. `main` is the authorized baseline; `development` is the working branch. Initial bootstrap commit on `main` is a baseline placeholder, not a validated release. |
| ADR-005 | Verifier Architecture | **PROPOSED (GOV-00)** | Standalone verification core + CLI with optional read-only GitHub adapter. No database, daemon, or GUI verifier in MVP. |
| ADR-006 | Runtime Language | **OPEN (Pending GOV-00)** | Comparative evaluation between Rust (memory safety, native packaging), Go (standard library, toolchain simplicity), and Python (rapid fixture testing). Decision gated on GOV-00. |
| ADR-007 | Target Platforms | **OPEN (Pending GOV-00)** | Cross-platform compatibility goals (Linux, macOS, Windows) to be tested and bounded on CI before claiming support. |
| ADR-008 | Data Serialization | **OPEN (Pending GOV-00)** | Evaluation of RFC 8785 JSON Canonicalization Scheme (JCS) for deterministic hashing and cross-platform verification. |
| ADR-009 | Multi-Dimensional Trust | **PROPOSED (GOV-00)** | Discard single 1D PASS/FAIL model in favor of six explicit dimensions: Evidence Integrity, CI Execution, Evidence Availability, Review Status, Authorization, and Freshness. |
| ADR-010 | External Consumer Contract | **PROPOSED (GOV-06)** | Read-only static output contract for external consumers such as AI-Toolchain-Manager (#33, #34). External tools cannot mint or upgrade verifier trust. |
| ADR-011 | Contributor Policy | **DEFERRED (Post-Bootstrap)** | No CLA, DCO, or automated contributor acceptance introduced during bootstrap. Future contributions require explicit IP and redistribution review. |

---

## Next Architectural Milestone

The next actionable milestone is **GOV-00: Architecture, requirements, technology comparison, and threat model**, which will formally resolve ADR-006, ADR-007, and ADR-008. No functional verification code is authorized before GOV-00 acceptance.
