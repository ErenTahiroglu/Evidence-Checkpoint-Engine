# Architecture Decisions and Governance Log

| Decision ID | Area | Current Status / Resolution | Rationale and Context |
|---|---|---|---|
| ADR-001 | Repository Identity | **RESOLVED: `ErenTahiroglu/Evidence-Checkpoint-Engine`** | Standalone repository separate from AI-Toolchain-Manager to avoid self-verification coupling. |
| ADR-002 | Repository Visibility | **RESOLVED: PUBLIC** | Repository is publicly readable on GitHub to enable transparent verification and reference across public projects. |
| ADR-003 | Licensing | **RESOLVED: PolyForm Noncommercial 1.0.0** | Adopted official PolyForm Noncommercial License 1.0.0. Required Notice: `Copyright (c) 2026 Eren Tahiroğlu`. Commercial use is restricted unless separately authorized. |
| ADR-004 | Branch Governance | **RESOLVED: `main` and `development`** | Exactly two long-lived branches. `main` is the authorized baseline; `development` is the working branch. Initial bootstrap commit on `main` is a baseline placeholder, not a validated release. |
| ADR-005 | Verifier Architecture | **PROPOSED (GOV-00 R2)** | Modular monolith core + CLI with optional read-only GitHub adapter. Detailed in [docs/gov-00/GOV_00_ARCHITECTURE_REPORT.md](gov-00/GOV_00_ARCHITECTURE_REPORT.md). |
| ADR-006 | Runtime Language | **PROPOSED: Rust (Provisional Recommendation)** | Evaluative scoring (4.375/5.00) favors Rust over Go (4.250), Python (2.600), and TypeScript (2.650). Recommends standalone CLI subprocess over in-process Tauri embedding for physical trust isolation. See [docs/gov-00/ADR_RUNTIME_AND_COMPONENTS.md](gov-00/ADR_RUNTIME_AND_COMPONENTS.md). Awaiting owner approval. |
| ADR-007 | Target Platforms | **PROPOSED: Windows 11, macOS, Linux** | Mandatory targets: Windows 11 (x86_64), macOS (Apple Silicon arm64), Linux (x86_64). Compatibility target: macOS (Intel x86_64). Windows ARM64 deferred by product decision (though runner `windows-11-arm` exists on GitHub). Planned runners: `windows-2025` (Windows Server), `macos-26`, `macos-26-intel`, `ubuntu-24.04`. See [docs/gov-00/MVP_AND_INTEGRATION_GATES.md](gov-00/MVP_AND_INTEGRATION_GATES.md). |
| ADR-008 | Data Serialization | **PROPOSED: RFC 8785 (JCS) & Two-Schema Envelope** | Strict UTF-16 key sorting, ECMAScript float formatting, two-schema outer envelope (`checkpoint-envelope.schema.json` wrapping `checkpoint-input.schema.json` via `$ref`), canonical decimal strings `^[1-9][0-9]{0,19}$` with mandatory semantic `<= uint64::MAX` check. See [docs/gov-00/CONTRACT_AND_CANONICALIZATION.md](gov-00/CONTRACT_AND_CANONICALIZATION.md). |
| ADR-009 | Multi-Dimensional Trust | **PROPOSED: 6-Dimensional Model** | Six orthogonal dimensions (Integrity, CI Execution, Availability, Review, Authorization, Freshness). Invariants prevent synthetic mocks from claiming passes or fabricated human authorization. Mandatory trusted root for expected-check policy. See [docs/gov-00/TRUST_AND_STATE_MODEL.md](gov-00/TRUST_AND_STATE_MODEL.md). |
| ADR-010 | External Consumer Contract | **PROPOSED: Read-Only Subprocess Pipe** | Downstream consumers (AI-Toolchain-Manager #33, #34) invoke verifier as a distinct child process and consume stdout JSON strictly read-only. See [docs/gov-00/MVP_AND_INTEGRATION_GATES.md](gov-00/MVP_AND_INTEGRATION_GATES.md). |
| ADR-011 | Contributor Policy | **DEFERRED (Post-Bootstrap)** | No CLA, DCO, or automated contributor acceptance introduced during bootstrap. Future contributions require explicit IP and redistribution review. |

---

## Architectural Governance Milestone

Milestone **GOV-00 R2: Final Schema and Evidence-Authority Contract Correction** has delivered corrected Draft 2020-12 schemas, two-schema envelope definitions, tightened evaluation invariants, and isolated adversarial test vectors under [docs/gov-00/](gov-00/) and [docs/gov-00/GOV_00_R2_CONTRACT_CORRECTION.md](gov-00/GOV_00_R2_CONTRACT_CORRECTION.md). All proposed decisions remain pending independent Red Team review and formal owner authorization before GOV-01 implementation.
