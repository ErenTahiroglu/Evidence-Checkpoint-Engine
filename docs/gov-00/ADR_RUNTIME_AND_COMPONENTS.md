# Architectural Decision Record: Runtime Selection and Component Boundaries

- **ADR ID:** `ADR-006`
- **Milestone:** `GOV-00 R1`
- **Status:** `PROPOSED (Provisional Recommendation; Awaiting Independent Red Team Review and Owner Approval)`
- **Date:** `2026-10-25`
- **Author:** `Principal Software Architect & Security Architect`
- **Decision Owner:** `Eren Tahiroglu`

---

## 1. Context and Problem Statement

`Evidence-Checkpoint-Engine` requires an independent verification engine that:
1. Evaluates evidence against cryptographic and structural invariants without ambient trust.
2. Ingests untrusted, potentially adversarial inputs (manifests, zip archives, JSON files) from third-party developers, CI logs, and autonomous AI agents.
3. Operates deterministically across Linux, macOS, and Windows.
4. Distributes as a lightweight, zero-dependency, standalone CLI and embeddable component.
5. Integrates into external consumers (such as the Tauri-based [AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager)) via a strict read-only boundary without granting elevated native privileges.
6. Is sustainable for a single primary maintainer.

The initial architecture proposal left the implementation runtime open. This ADR provides a structured, evidence-backed evaluation of **Rust**, **Go**, and **Python**, with **TypeScript/Node.js** as a fourth evaluated candidate.

---

## 2. Evaluation Criteria and Weighting Rationale

The weighting reflects the engine's core role as a **security and trust anchor**. Because the engine evaluates untrusted inputs to decide whether software is acceptable, memory safety, cross-platform determinism, and minimal supply-chain surface area outrank rapid dynamic prototyping speed.

| Criterion | Weight | Rationale |
|---|---|---|
| **1. Memory Safety & Hostile Input Hardening** | **20%** | The engine processes untrusted JSON, compressed archives, and hex digests. Vulnerabilities (buffer overflows, use-after-free, unhandled panics) undermine the verifier. |
| **2. Deterministic Cross-Platform CLI Distribution** | **20%** | Must produce single, zero-dependency native binaries for Linux, macOS, and Windows without requiring end users to manage language runtimes. |
| **3. Standards Conformance (RFC 8785 JCS & Schema Draft 2020-12)** | **15%** | Deterministic canonicalization and modern JSON schema validation are non-negotiable foundations for content-addressed verification. |
| **4. Process Boundary & Consumer Integration** | **15%** | Must integrate with external hosts (e.g. Tauri/Rust desktop app or CLI wrappers) without compromising process isolation or introducing signal conflicts. |
| **5. Supply-Chain Exposure & Dependency Auditing** | **10%** | Minimal dependency tree, reproducible builds, and tight lockfile pinning to prevent supply-chain poisoning. |
| **6. Single Maintainer Burden & Development Velocity** | **10%** | The project is maintained by a single primary developer. Long-term maintenance cost (refactoring safety, regression prevention) must be balanced with initial velocity. |
| **7. Offline Determinism & Execution Isolation** | **10%** | Zero ambient network requirements, static linking, and bounded resource utilization. |
| **Total** | **100%** | |

---

## 3. Candidate Evaluation

### Candidate A: Rust

- **Memory Safety & Safety Boundaries:**
  - Rust provides compile-time enforcement of spatial and temporal memory safety in safe code without garbage collection overhead.
  - *Realistic Security Caveat:* Compile-time memory safety does **not** eliminate every parser vulnerability. Safe Rust code can still suffer from logic errors, unbounded recursion (stack overflow), quadratic-time algorithmic complexity (ReDoS / algorithmic DoS), and unhandled panics resulting in denial-of-service.
  - Furthermore, third-party dependencies frequently contain `unsafe` blocks. Any Rust implementation must enforce strict dependency auditing (`cargo-deny`, `cargo-audit`, `cargo-geiger`) and fuzzing (`cargo-fuzz`).
- **Cross-Platform Distribution & OS Dependencies:**
  - Compiles to standalone native binaries (ELF on Linux, Mach-O on macOS, PE on Windows).
  - *OS Dependency Caveat:* Native compilation does not automatically imply zero external operating system dependencies. Standard dynamic targets link against platform system libraries (e.g., `libSystem.dylib` on macOS, MSVCRT / `ntdll.dll` on Windows, `glibc` on Linux). Fully zero-dependency static binaries are achievable on Linux via MUSL (`x86_64-unknown-linux-musl`), but macOS and Windows binaries retain standard platform dynamic dependencies.
- **Standards Conformance:**
  - *RFC 8785 (JCS):* Available via crates such as `serde_jcs` and `jcs` implementing exact UTF-16 code-unit property sorting and ECMAScript number serialization.
  - *JSON Schema Draft 2020-12:* Implemented via `jsonschema` (stranger6667) and `boon` with schema test-suite compliance.
- **Process Boundaries & Integration:**
  - Can produce both standalone CLI executables and native Rust libraries.
  - Rust's enum sum types cleanly model multi-dimensional states, though cross-dimensional business logic invariants require runtime validation.
- **Trade-offs:** Steeper initial learning curve and longer compile times compared to Go or Python.

### Candidate B: Go

- **Memory Safety:** Memory-safe managed language with garbage collection. Spatial memory safety is maintained, though nil-pointer dereferences and data races can cause runtime panics if unhandled.
- **Cross-Platform Distribution:** Built-in cross-compilation (`GOOS`/`GOARCH`) creates single self-contained static binaries easily.
- **Standards Conformance:**
  - *RFC 8785 (JCS):* Excellent reference implementation `github.com/gowebpki/jcs` by Anders Rundgren (co-author of RFC 8785).
  - *JSON Schema Draft 2020-12:* Available via `github.com/santhosh-tekuri/jsonschema/v5`.
- **Process Boundary & Integration:**
  - Excellent as a standalone CLI subprocess.
  - However, embedding Go as an in-process C-shared library (`-buildmode=c-shared`) inside an external application brings the full Go runtime (GC, M:N goroutine scheduler, signal handlers) into the host process, creating well-documented signal handling conflicts and memory overhead.
- **Supply Chain & Fuzzing:** Built-in fuzzing (`testing.F`) since Go 1.18. Small standard library minimizes third-party dependencies.
- **Trade-offs:** Error handling with deep nested schemas requires extensive boilerplate; lack of tagged unions makes multi-state transitions harder to structure concisely.

### Candidate C: Python

- **Memory Safety:** Memory-safe language runtime, but relies on C extensions (CPython) that have historical CVEs. No compile-time memory guarantees.
- **Cross-Platform Distribution:** Interpreted language. Producing standalone executables requires PyInstaller or PyOxidizer, packaging an entire virtual environment. This leads to large binaries (40MB+), platform glibc incompatibilities, slow startup latency, and extraction security vulnerabilities.
- **Standards Conformance:**
  - *RFC 8785 (JCS):* Packages like `canonicaljson` exist, but Python's standard `json.dumps(sort_keys=True)` sorts by Unicode code points rather than UTF-16 code units and differs on float serialization.
  - *JSON Schema Draft 2020-12:* Excellent support via the `jsonschema` package.
- **Process Boundary & Integration:** High overhead when spawning Python interpreters from desktop applications.
- **Supply Chain & Maintenance:** PyPI packaging ecosystem has high risk of dependency confusion and typosquatting. Runtime dynamic typing increases regression risk.
- **Trade-offs:** Rapid scripting velocity, but unsuitable for a hardened, self-contained, low-footprint verifier binary. Preserved strictly for offline test scripting.

### Candidate D: TypeScript / Node.js

- **Memory Safety:** Memory-safe managed language, but vulnerable to prototype pollution attacks when parsing untrusted JSON documents without defensive object creation (`Object.create(null)`).
- **Cross-Platform Distribution:** Requires bundling Node.js runtime (via `pkg`, Bun, or Node Single Executable Applications - SEA). Produces large binaries and complex cross-compilation toolchains.
- **Standards Conformance:** Native ECMAScript number serialization aligns with JCS; AJV provides top-tier Draft 2020-12 validation.
- **Supply Chain:** npm ecosystem has the highest frequency of dependency compromises and deep transitive trees.
- **Trade-offs:** High supply-chain surface area and runtime bundling overhead make it inappropriate for a foundational verification engine.

---

## 4. Evaluative Scoring Matrix (Engineering Judgment)

> **Methodology Note:**
> The following table represents a qualitative engineering judgment and heuristic scoring model based on the architectural constraints of this repository, rather than an experimentally measured empirical benchmark.

| Evaluation Criteria | Weight | Rust | Go | Python | TypeScript/Node |
|---|---|---|---|---|---|
| 1. Memory Safety & Hostile Inputs | 20% | **4.5** (0.90) | 4.0 (0.80) | 3.0 (0.60) | 3.0 (0.60) |
| 2. Deterministic CLI Distribution | 20% | **4.5** (0.90) | **5.0** (1.00) | 2.0 (0.40) | 2.0 (0.40) |
| 3. Standards Conformance (JCS/Schema) | 15% | 4.0 (0.60) | **4.5** (0.675) | 4.0 (0.60) | **5.0** (0.75) |
| 4. Process Boundary & Integration | 15% | **4.5** (0.675) | 3.5 (0.525) | 2.0 (0.30) | 2.0 (0.30) |
| 5. Supply-Chain Security & Pinning | 10% | **4.5** (0.45) | 4.0 (0.40) | 2.0 (0.20) | 1.0 (0.10) |
| 6. Single Maintainer Burden & Refactoring | 10% | 4.0 (0.40) | **4.5** (0.45) | 3.0 (0.30) | 3.0 (0.30) |
| 7. Offline Determinism & Zero Runtime | 10% | **4.5** (0.45) | 4.5 (0.45) | 2.0 (0.20) | 2.0 (0.20) |
| **Weighted Total Score** | **100%** | **4.375 / 5.00** | **4.250 / 5.00** | **2.600 / 5.00** | **2.650 / 5.00** |

*(Scale: 1 = Inadequate, 3 = Acceptable with notable trade-offs, 5 = Exemplary / Best in Class)*

---

## 5. Architectural Evaluation: In-Process Embedding vs Subprocess CLI Boundary

A critical architectural question is how the verifier should integrate with downstream consumers like [AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager) (a Tauri/Rust application).

### 5.1 Option 1: In-Process Library Embedding (Crate / FFI)
- **Concept:** AI-Toolchain-Manager imports `evidence-checkpoint-engine` as a Rust library crate directly into its Tauri backend process.
- **Advantages:** Zero inter-process communication overhead; direct typed struct passing.
- **Security & Trust Disadvantages:**
  - **Shared Address Space:** The verifier shares memory with the entire Tauri application, including third-party webview plugins, system tray handlers, and IPC bridges. A vulnerability or memory bug anywhere in the host process could inspect or tamper with verifier state.
  - **Shared Panic Handlers:** An unhandled panic in the verifier can crash the entire desktop application, or conversely, host crashes can terminate in-flight verifications unpredictably.
  - **Coupled Release Cycles:** Updating the verifier requires rebuilding and redeploying the entire desktop application.

### 5.2 Option 2: Separately Versioned CLI Subprocess via Controlled Pipe (Recommended)
- **Concept:** AI-Toolchain-Manager invokes the verifier as a distinct, standalone child executable via a controlled standard input/output pipe (`stdin` JSON manifest $\to$ `stdout` JSON assessment record).
- **Advantages:**
  - **Strict Physical Trust Boundary:** The verifier executes in its own isolated operating system process with a private virtual address space.
  - **Fail-Closed Isolation:** If the verifier encounters an invalid input, malformed syntax, or panic, the child process exits cleanly with an error exit code (e.g. `2`, `3`, or `4`), leaving the host application completely intact.
  - **Decoupled Versioning & Auditing:** The verifier binary is versioned, signed, and tested independently of any GUI shell. Third-party auditors can verify the exact standalone binary from shell scripts without Tauri.
  - **Negligible Overhead:** Evidence checkpoints are evaluated at milestone transitions (human timescales), where process spawn latency (~5–15 ms) is completely imperceptible.

### 5.3 Architectural Recommendation
For initial milestones (GOV-01 and GOV-02), **the verifier must be built and distributed as a standalone CLI executable operated over a strictly controlled standard I/O pipe**. In-process library embedding is deferred and should not be used if it weakens the physical isolation of the verification engine.

---

## 6. Important Unknowns and Proof-of-Concept Requirements

Before implementing the production engine in GOV-01, the following proof-of-concept checks must be validated in an isolated fixture test:
1. **RFC 8785 Interoperability:** Confirm that `serde_jcs` produces byte-exact matching hashes against standard test vectors published in RFC 8785 Section 3.
2. **Draft 2020-12 Completeness:** Verify that the chosen Rust schema validator correctly enforces `additionalProperties: false` across nested schemas and handles large integer string formats.
3. **Binary Footprint & Compilation Target:** Test static compilation for `x86_64-unknown-linux-musl`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, and `x86_64-pc-windows-msvc`.

---

## 7. Conditions for Changing This Recommendation

This recommendation should be revisited if:
1. Independent Red Team review reveals a fatal flaw in Rust's RFC 8785 or JSON Schema Draft 2020-12 crate ecosystem.
2. The project owner explicitly decides against Rust due to development bandwidth constraints, preferring Go's simpler syntax and built-in cross-compilation.
3. Target platform compilation benchmarks reveal unacceptable maintenance friction across Apple Silicon, Intel macOS, and Windows.

---

## 8. Owner Decision Gate

> **GOVERNANCE NOTICE:**
> This ADR constitutes an engineering recommendation and remains **PROPOSED**. Pursuant to repository governance, **only the repository owner (`Eren Tahiroglu`) may formally approve this ADR** following independent Red Team review. No code implementation may begin until formal acceptance is recorded in Issue #2.
