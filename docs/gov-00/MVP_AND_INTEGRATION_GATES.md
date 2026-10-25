# MVP Reference Architecture and Integration Gates

> **Milestone:** `GOV-00`
> **Status:** `PROPOSED IMPLEMENTATION GATES / DRAFT FOR REVIEW`
> **MVP Path:** `BOOTSTRAP-00` $\to$ `GOV-00` $\to$ `GOV-01` $\to$ `GOV-02` (Pilot)

---

## 1. Minimal Viable Product (MVP) Scope

The goal of the MVP is to prove the core verification thesis in the smallest auditable surface area before expanding into network collection, archival, or GUI integrations.

```
+---------------------------------------------------------------------------------------+
|                                    MVP BOUNDARY                                       |
|                                                                                       |
|  [Untrusted JSON Manifest]                                                            |
|            |                                                                          |
|            v                                                                          |
|  +---------------------------------------------------------------------------------+  |
|  |                     Evidence-Checkpoint-Engine Core (GOV-01)                    |  |
|  |  1. Bounded Parser (max 10MB, depth <= 32)                                      |  |
|  |  2. Draft 2020-12 Schema Validator (additionalProperties: false)                |  |
|  |  3. RFC 8785 Canonicalizer (UTF-16 code-unit sorting, ECMAScript floats)        |  |
|  |  4. Content Hash Verifier (SHA-256)                                             |  |
|  |  5. 6-Dimensional State Evaluator                                               |  |
|  |  6. Exit Code Mapper & Assessment Output Generator                              |  |
|  +---------------------------------------------------------------------------------+  |
|            |                                                                          |
|            v (Deterministic stdout JSON / Exit Code)                                  |
|  [Machine-Assessed Assessment Record]                                                 |
+---------------------------------------------------------------------------------------+
```

---

## 2. Capability Allocation: Immediate vs Deferred

### What Belongs in GOV-01 (Offline Core & CLI):
- Bounded input reading from local files or stdin.
- Strict schema validation against `checkpoint-input.schema.json`.
- Exact RFC 8785 canonical byte generation and SHA-256 digest comparison.
- Evaluator for the 6 orthogonal trust dimensions.
- Explicit exit codes:
  - `0`: Assessment complete, all evaluated dimensions pass.
  - `1`: Assessment complete, but limitations or warnings detected.
  - `2`: Evidence verification failed (mismatched digests, missing expected jobs).
  - `3`: Malformed input or schema rejection.
  - `4`: Internal or unhandled operational error.
- Comprehensive negative test fixture suite executing 100% offline.

### What Must Wait for Later Gates:
- **Live GitHub API Collection:** Deferred to `GOV-02` (prevents network coupling during core verification).
- **GitHub Actions Reusable Workflow Integration:** Deferred to `GOV-03`.
- **Durable Archival, Sigstore Signing & Offsite Anchors:** Deferred to `GOV-04`.
- **Agent Prompts & Tool Adapters (Claude/Codex):** Deferred to `GOV-05`.
- **Consumer Interface & Cross-Project Verification (AI-Toolchain-Manager #34):** Deferred to `GOV-06`.
- **Automated Issue Publishing (AI-Toolchain-Manager #35):** Remains design-only; no automated write authority.

---

## 3. Downstream Consumer Integration (AI-Toolchain-Manager)

The integration boundary with [AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager) is strictly governed by the following rules:

1. **Decoupled Transport:**
   - In MVP, communication occurs via **offline versioned JSON files** or **piping CLI standard output (`stdout` JSON)**.
   - No background daemons, listening HTTP servers, or persistent network sockets are introduced.
2. **Zero Inversion of Authority:**
   - The consumer UI (Tauri desktop app) is an **untrusted display terminal**.
   - The UI **cannot** mint new verifications, promote baselines, or override a verifier `ASSESSED_FAIL` verdict.
3. **Fail-Closed Display:**
   - If the engine emits an unknown schema version, unparseable output, or missing trust dimension, the consumer UI must display an explicit error banner rather than a default green status.

---

## 4. Platform Support and Test Matrix

The engine targets Linux, macOS, and Windows. However, pursuant to repository integrity rules, **no platform compatibility claims are made until compiled binaries and test suites execute on each platform in CI**.

| Target Platform | Architecture | Binary Format | Planned CI Runner | Status |
|---|---|---|---|---|
| **Linux** | `x86_64` | ELF (Statically linked via MUSL) | `ubuntu-latest` | Planned for GOV-01 |
| **Linux** | `aarch64` | ELF (Statically linked via MUSL) | Cross-compiled | Planned for GOV-01 |
| **macOS** | `aarch64` (Apple Silicon) | Mach-O 64-bit | `macos-latest` | Planned for GOV-01 |
| **macOS** | `x86_64` (Intel) | Mach-O 64-bit | `macos-13` | Planned for GOV-01 |
| **Windows** | `x86_64` | PE32+ executable | `windows-latest` | Planned for GOV-01 |

---

## 5. Entrance and Exit Criteria for GOV-01

### Entrance Criteria:
1. `GOV-00` documentation package independently reviewed by Red Team.
2. The project owner (`Eren Tahiroğlu`) formally authorizes `ADR-006` runtime selection in Issue #2.
3. Issue #3 (`GOV-01`) is unblocked and transitioned to active development.

### Exit Criteria:
1. Core library and CLI compiled with zero warnings on `development`.
2. 100% pass on negative test fixtures (rejecting malformed, tampered, or mismatched inputs).
3. Exact byte-level match against RFC 8785 Section 3 canonicalization test vectors.
4. Independent code review completed; zero ambient network calls verified in sandbox.
5. Explicit owner authorization before promotion to `main`.
