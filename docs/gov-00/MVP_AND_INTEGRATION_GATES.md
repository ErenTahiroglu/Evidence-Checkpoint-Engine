# MVP Reference Architecture and Integration Gates

> **Milestone:** `GOV-00 R1`
> **Status:** `PROPOSED IMPLEMENTATION GATES / REMEDIATED DRAFT FOR REVIEW`
> **MVP Path:** `BOOTSTRAP-00` $\to$ `GOV-00` $\to$ `GOV-01` $\to$ `GOV-02` (Pilot)

---

## 1. Minimal Viable Product (MVP) Scope

The goal of the MVP is to prove the core verification thesis in the smallest auditable surface area before expanding into network collection, archival, or GUI integrations.

```
+---------------------------------------------------------------------------------------+
|                                    MVP BOUNDARY                                       |
|                                                                                       |
|  [Untrusted JSON Checkpoint Payload]                                                  |
|            |                                                                          |
|            v (Standard I/O pipe or file input)                                        |
|  +---------------------------------------------------------------------------------+  |
|  |                     Evidence-Checkpoint-Engine Core (GOV-01)                    |  |
|  |  1. Bounded Parser (max 10MB payload size, recursion depth <= 32)               |  |
|  |  2. Draft 2020-12 Schema Validator (additionalProperties: false)                |  |
|  |  3. RFC 8785 Canonicalizer (UTF-16 code-unit sorting, ECMAScript floats)        |  |
|  |  4. Content Hash Verifier (SHA-256)                                             |  |
|  |  5. 6-Dimensional State Evaluator with strict fail-closed precedence            |  |
|  |  6. Deterministic Output Generator & Exit Code Contract                         |  |
|  +---------------------------------------------------------------------------------+  |
|            |                                                                          |
|            v (Deterministic stdout JSON / Exit Code 0..4)                             |
|  [Machine-Assessed Assessment Record]                                                 |
+---------------------------------------------------------------------------------------+
```

---

## 2. Capability Allocation: Immediate vs Deferred

### What Belongs in GOV-01 (Offline Core & CLI):
- Bounded input reading from local files or standard input (`stdin`).
- Strict schema validation against `checkpoint-input.schema.json` (Draft 2020-12).
- Exact RFC 8785 canonical byte generation and SHA-256 digest comparison.
- Evaluator for the 6 orthogonal trust dimensions with strict precedence rules.
- Deterministic exit code mapping (0=Pass, 1=Limitations, 2=Fail, 3=Refusal/Insufficient, 4=Execution error).
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

1. **Decoupled Process Boundary (Subprocess Pipe):**
   - Communication occurs via **standard input/output pipes** (`stdin` JSON manifest $\to$ `stdout` JSON assessment record) or **versioned local files**.
   - No shared memory, in-process C-ABI FFI, background daemons, listening HTTP servers, or persistent network sockets are introduced.
2. **Zero Inversion of Authority:**
   - The consumer UI (Tauri desktop app) is an **untrusted display terminal**.
   - The UI **cannot** mint new verifications, promote baselines, or override a verifier `ASSESSED_FAIL` verdict.
3. **Fail-Closed Display:**
   - If the engine emits an unknown schema version, unparseable output, or missing trust dimension, the consumer UI must display an explicit error banner rather than a default green status.

---

## 4. Cross-Platform Target Matrix and Architecture Alignment

Evidence-Checkpoint-Engine must eventually run independently across Windows, macOS, and Linux.

### 4.1 Target Priority Matrix

| Platform | Architecture | Priority | Binary Format | Verification Status |
|---|---|---|---|---|
| **Windows 11** | `x86_64` | **Mandatory** | PE32+ executable (`.exe`) | Specification only; pending GOV-01 native CI |
| **macOS** | Apple Silicon (`arm64`) | **Mandatory** | Mach-O 64-bit executable | Specification only; pending GOV-01 native CI |
| **macOS** | Intel (`x86_64`) | **Compatibility Target** | Mach-O 64-bit executable | Specification only; pending GOV-01 native CI |
| **Linux** | `x86_64` | **Mandatory Headless Target** | ELF 64-bit (static via MUSL) | Specification only; pending GOV-01 native CI |
| **Linux** | `aarch64` | **Secondary** | ELF 64-bit (static via MUSL) | Deferred / Cross-compile |
| **Windows** | `arm64` | **Deferred** | PE32+ executable | Deferred to future milestone |

### 4.2 Architecture-Matched GitHub-Hosted Runners (Planned for GOV-01)

> **CRITICAL RUNNER CORRECTION (R1):**
> GitHub Actions retired `macos-13` in December 2025. Furthermore, `macos-latest` points to Apple Silicon (ARM64), not Intel. To prevent architecture mismatch, the planned CI matrix specifies explicit, verified runner labels from the current GitHub runner matrix:

| Target Platform & Arch | Explicit Runner Label | Runner Architecture | Notes |
|---|---|---|---|
| **Windows 11 (`x86_64`)** | `windows-2025` (or `windows-latest`) | `x86_64` | Windows Server 2025 / Windows 11 host environment |
| **macOS Apple Silicon (`arm64`)** | `macos-26` / `macos-15` (or `macos-latest`) | `arm64` (Apple Silicon M1/M2/M3) | Native ARM64 execution; default for `macos-latest` |
| **macOS Intel (`x86_64`)** | `macos-26-intel` / `macos-15-intel` | `x86_64` (Intel) | Dedicated Intel x86_64 runner for backwards compatibility |
| **Linux (`x86_64`)** | `ubuntu-24.04` (or `ubuntu-latest`) | `x86_64` | Ubuntu 24.04 LTS runner |

*Note on Windows ARM64:* Windows ARM64 is not currently available as a standard free GitHub-hosted runner. It is explicitly deferred to later milestones rather than silently claimed as supported.

*Note on CI Scope:* **No CI workflow file may be created in GOV-00 R1.** These runner definitions establish the planned verification matrix for GOV-01.

### 4.3 Evidence Hierarchy for Platform Support
Pursuant to repository governance, no platform is classified as "verified" until native execution evidence is produced. The six verification levels are:
1. **Source Compatibility:** Code compiles on a developer's local machine.
2. **Successful Native Compilation:** GitHub Actions runner compiles target binary with zero warnings.
3. **Passing Unit Tests:** Offline unit and schema tests pass on the target runner.
4. **Real CLI Behavior:** CLI process executes, parses stdin/stdout, and emits expected exit codes on the native OS.
5. **Distribution Compatibility:** Packaged binary runs on clean target OS installations without missing dynamic libraries (e.g. MSVCRT on Windows, libSystem on macOS).
6. **Consumer Integration:** End-to-end execution verified when invoked as a child process from AI-Toolchain-Manager.

### 4.4 Realistic Platform-Specific Considerations
- **CLI Paths and Arguments:** Windows uses backslashes (`\`) for file paths and requires checking `PATHEXT` (e.g. `ece.exe`), whereas POSIX systems use forward slashes (`/`). Schema paths and internal canonical identifiers always use normalized POSIX forward slashes.
- **Line Endings:** Windows tools frequently emit CRLF (`\r\n`), while Linux/macOS emit LF (`\n`). RFC 8785 canonicalization normalizes all internal whitespace; however, file readers must handle CRLF transparently without altering canonical payload bytes.
- **Console Encoding & Unicode:** Windows console output often defaults to legacy code pages (e.g. CP437, CP1252), causing mojibake when printing UTF-8. The CLI must explicitly set console output to UTF-8 (Code Page 65001) on Windows.
- **File Permissions:** POSIX executable bit (`chmod +x`) is required on Linux/macOS; Windows relies on file extensions (`.exe`).
- **Filesystem Case Sensitivity:** Linux filesystems (ext4) are case-sensitive; macOS (APFS) and Windows (NTFS) default to case-preserving case-insensitive. Manifest paths must strictly match exact file casing to avoid cross-platform checkout errors.
- **Binary Distribution & Signing:** macOS executables require Apple codesigning and Gatekeeper notarization for unprompted execution; Windows executables require Authenticode signing to avoid SmartScreen warnings. These are planned for GOV-04.

---

## 5. Entrance and Exit Criteria for GOV-01

### Entrance Criteria:
1. `GOV-00 R1` remediation package independently reviewed and confirmed by Red Team.
2. The project owner (`Eren Tahiroglu`) formally authorizes `ADR-006` runtime selection in Issue #2.
3. Issue #3 (`GOV-01`) is unblocked and transitioned to active development.

### Exit Criteria:
1. Core library and CLI compiled with zero warnings on `development`.
2. 100% pass on negative test fixtures (rejecting malformed, tampered, or mismatched inputs).
3. Exact byte-level match against RFC 8785 Section 3 canonicalization test vectors.
4. Independent code review completed; zero ambient network calls verified in sandbox.
5. Explicit owner authorization before promotion to `main`.
