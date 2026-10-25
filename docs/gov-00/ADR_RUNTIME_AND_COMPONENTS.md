# Architectural Decision Record: Runtime Selection and Component Boundaries

- **ADR ID:** `ADR-006`
- **Milestone:** `GOV-00`
- **Status:** `PROPOSED (Provisional Recommendation; Awaiting Independent Red Team Review and Owner Approval)`
- **Date:** `2026-10-25`
- **Author:** `Principal Software Architect & Security Architect`
- **Decision Owner:** `Eren Tahiroğlu`

---

## 1. Context and Problem Statement

`Evidence-Checkpoint-Engine` requires an independent verification engine that:
1. Evaluates evidence against cryptographic and structural invariants without ambient trust.
2. Ingests untrusted, potentially adversarial inputs (manifests, zip archives, JSON files) from third-party developers, CI logs, and autonomous AI agents.
3. Operates deterministically across Linux, macOS, and Windows.
4. Distributes as a lightweight, zero-dependency, standalone CLI and embeddable library.
5. Integrates into external consumers (such as the Tauri-based [AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager)) via a strict read-only boundary without granting elevated native privileges.
6. Is sustainable for a single primary maintainer.

The initial architecture proposal left the implementation runtime open. This ADR provides a structured, evidence-backed evaluation of **Rust**, **Go**, and **Python**, with **TypeScript/Node.js** as a fourth evaluated candidate.

---

## 2. Evaluation Criteria and Weighting Rationale

The weighting reflects the engine's core role as a **security and trust anchor**. Because the engine evaluates untrusted inputs to decide whether software is acceptable, memory safety, cross-platform determinism, and minimal supply-chain surface area outrank rapid dynamic prototyping speed.

| Criterion | Weight | Rationale |
|---|---|---|
| **1. Memory Safety & Hostile Input Hardening** | **20%** | The engine processes untrusted JSON, compressed archives, and hex digests. Vulnerabilities (buffer overflows, use-after-free, unhandled panic panics) undermine the verifier. |
| **2. Deterministic Cross-Platform CLI Distribution** | **20%** | Must produce single, zero-dependency native binaries for Linux, macOS, and Windows without requiring end users to manage language runtimes. |
| **3. Standards Conformance (RFC 8785 JCS & Schema Draft 2020-12)** | **15%** | Deterministic canonicalization and modern JSON schema validation are non-negotiable foundations for content-addressed verification. |
| **4. Library Embeddability & Consumer Integration** | **15%** | Must embed into external hosts (e.g. Tauri/Rust desktop app or CLI wrappers) without runtime collisions, signal hijacking, or heavy IPC daemons. |
| **5. Supply-Chain Exposure & Dependency Auditing** | **10%** | Minimal dependency tree, reproducible builds, and tight lockfile pinning to prevent supply-chain poisoning. |
| **6. Single Maintainer Burden & Development Velocity** | **10%** | The project is maintained by a single primary developer. Long-term maintenance cost (refactoring safety, regression prevention) must be balanced with initial velocity. |
| **7. Offline Determinism & Execution Isolation** | **10%** | Zero ambient network requirements, static linking, and bounded resource utilization. |
| **Total** | **100%** | |

---

## 3. Candidate Evaluation

### Candidate A: Rust

- **Memory Safety:** Industry-leading spatial and temporal memory safety enforced at compile time via the borrow checker without garbage collection overhead. Robust against parser exploits.
- **Cross-Platform Distribution:** First-class compilation to standalone, statically linked binaries (MUSL on Linux, native Mach-O on macOS, native PE on Windows). Zero runtime or interpreter installation required.
- **Standards Conformance:**
  - *RFC 8785 (JCS):* Available via vetted crates such as `serde_jcs` and `jcs` implementing exact UTF-16 code-unit property sorting and ECMAScript number serialization.
  - *JSON Schema Draft 2020-12:* Implemented via `jsonschema` (stranger6667) and `boon` with full test-suite compliance.
- **Library Embeddability:** Native Rust crate embedding with zero overhead; can compile to static/dynamic C libraries (`cdylib`, `staticlib`) for Python/C FFI. Directly interoperates with Tauri desktop hosts (AI-Toolchain-Manager) via in-process read-only bindings without spawning daemons.
- **Supply Chain & Fuzzing:** `cargo-audit`, `cargo-deny`, `cargo-fuzz` (libFuzzer/AFL) integration are industry best practices for security research.
- **Trade-offs:** Steeper initial development curve; compile times can be significant. However, maintenance cost is low because the compiler catches refactoring bugs and concurrency races.

### Candidate B: Go

- **Memory Safety:** Memory-safe managed language with garbage collection. Spatial memory safety is maintained, though nil-pointer dereferences and data races can cause runtime panics if unhandled.
- **Cross-Platform Distribution:** Built-in cross-compilation (`GOOS`/`GOARCH`) creates single self-contained static binaries easily.
- **Standards Conformance:**
  - *RFC 8785 (JCS):* Excellent reference implementation `github.com/gowebpki/jcs` by Anders Rundgren (co-author of RFC 8785).
  - *JSON Schema Draft 2020-12:* Available via `github.com/santhosh-tekuri/jsonschema/v5`.
- **Library Embeddability:** Can produce C-shared libraries (`-buildmode=c-shared`), but embedding a Go runtime (garbage collector, M:N scheduler, signal handling) inside an external Rust/Tauri application introduces known signal conflicts, memory overhead, and complex lifecycle management.
- **Supply Chain & Fuzzing:** Built-in fuzzing (`testing.F`) since Go 1.18. Small standard library minimizes third-party dependencies.
- **Trade-offs:** Embedding inside Rust/Tauri hosts is awkward; error handling with deep nested schemas requires extensive boilerplate; lack of sum types (tagged unions) makes multi-dimensional state transitions harder to enforce at compile time.

### Candidate C: Python

- **Memory Safety:** Memory-safe language runtime, but relies on C extensions (CPython) that have historical CVEs. No compile-time memory guarantees.
- **Cross-Platform Distribution:** Interpreted language. Producing standalone executables requires PyInstaller or PyOxidizer, which package an entire virtual environment and shared libraries. This leads to 40MB+ binaries, platform glibc incompatibilities, slow startup latency, and extraction security vulnerabilities.
- **Standards Conformance:**
  - *RFC 8785 (JCS):* Packages like `canonicaljson` exist, but Python's standard `json.dumps(sort_keys=True)` sorts by Unicode code points rather than UTF-16 code units and differs on float serialization.
  - *JSON Schema Draft 2020-12:* Excellent support via the `jsonschema` package.
- **Library Embeddability:** Hard to embed cleanly into non-Python desktop applications without bundling Python or using IPC sockets.
- **Supply Chain & Maintenance:** PyPI packaging ecosystem has high risk of dependency confusion and typosquatting. Runtime dynamic typing increases regression risk during refactoring.
- **Trade-offs:** Rapid scripting velocity, but unsuitable for a hardened, self-contained, low-footprint verifier binary.

### Candidate D: TypeScript / Node.js

- **Memory Safety:** Memory-safe managed language, but vulnerable to prototype pollution attacks when parsing untrusted JSON documents without defensive object creation (`Object.create(null)`).
- **Cross-Platform Distribution:** Requires bundling Node.js runtime (via `pkg`, Bun, or Node Single Executable Applications - SEA). Produces large binaries and complex cross-compilation toolchains.
- **Standards Conformance:** Native ECMAScript number serialization aligns with JCS; AJV provides top-tier Draft 2020-12 validation.
- **Supply Chain:** npm ecosystem has the highest frequency of dependency compromises and deep transitive trees.
- **Trade-offs:** High supply-chain surface area and runtime bundling overhead make it inappropriate for a foundational verification engine.

---

## 4. Weighted Decision Matrix

| Evaluation Criteria | Weight | Rust | Go | Python | TypeScript/Node |
|---|---|---|---|---|---|
| 1. Memory Safety & Hostile Inputs | 20% | **5** (1.00) | 4 (0.80) | 3 (0.60) | 3 (0.60) |
| 2. Deterministic CLI Distribution | 20% | **5** (1.00) | 5 (1.00) | 2 (0.40) | 2 (0.40) |
| 3. Standards Conformance (JCS/Schema) | 15% | 4 (0.60) | **5** (0.75) | 4 (0.60) | 5 (0.75) |
| 4. Library Embeddability & Tauri/Consumer | 15% | **5** (0.75) | 2 (0.30) | 2 (0.30) | 2 (0.30) |
| 5. Supply-Chain Security & Pinning | 10% | **5** (0.50) | 4 (0.40) | 2 (0.20) | 1 (0.10) |
| 6. Single Maintainer Burden & Refactoring | 10% | 4 (0.40) | 4 (0.40) | 3 (0.30) | 3 (0.30) |
| 7. Offline Determinism & Zero Runtime | 10% | **5** (0.50) | 5 (0.50) | 2 (0.20) | 2 (0.20) |
| **Weighted Total Score** | **100%** | **4.75 / 5.00** | **4.15 / 5.00** | **2.60 / 5.00** | **2.65 / 5.00** |

*(Scale: 1 = Inadequate, 3 = Acceptable with notable trade-offs, 5 = Exemplary / Best in Class)*

---

## 5. Architectural Recommendation

### Primary Recommendation: **Rust**
1. **Unmatched Security Posture:** Eliminates entire classes of memory safety vulnerabilities when parsing untrusted, oversized, or malformed fixtures.
2. **Zero-Dependency Native Artifact:** Produces compact, self-contained binaries (~5–10 MB) runnable on Linux, macOS, and Windows without external prerequisites.
3. **Seamless Downstream Integration:** Directly exposes both a CLI interface and a native Rust crate interface. The downstream consumer ([AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager)) is a Tauri/Rust application; a Rust core can be integrated read-only as an in-process library without IPC sockets, background daemons, or runtime collisions.
4. **Compile-Time State Invariant Enforcement:** Rust's enum sum types allow the six-dimensional trust state machine to be enforced at compile time, eliminating invalid state combinations.

### Runner-Up: **Go**
Go is a strong, viable alternative if compile times or Rust's initial learning curve become prohibitive. Its built-in cross-compilation and standard library JCS reference implementation (`gowebpki/jcs`) are excellent. However, Go was not selected as primary because embedding Go into external Rust/Tauri runtimes requires complex C-bridge wrappers with dual-runtime overhead.

### Rejection of Python & TypeScript
- Python is rejected for the core verification engine due to distribution packaging fragility (bundling interpreters), execution latency, and lack of clean embeddability. Python is preserved exclusively as an optional fixture generation and test-scripting tool.
- TypeScript/Node.js is rejected due to prototype-pollution risks in untrusted JSON parsing, npm supply-chain vulnerabilities, and large packaged executable sizes.

---

## 6. Important Unknowns and Proof-of-Concept Requirements

Before implementing the production engine in GOV-01, the following proof-of-concept checks must be validated in an isolated fixture test:
1. **RFC 8785 Interoperability:** Confirm that `serde_jcs` produces byte-exact matching hashes against standard test vectors published in RFC 8785 Section 3.
2. **Draft 2020-12 Completeness:** Verify that the chosen Rust schema validator correctly enforces `additionalProperties: false` across nested schemas and handles large integer string formats.
3. **Binary Footprint & Compilation Target:** Test static compilation for `x86_64-unknown-linux-musl`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`.

---

## 7. Conditions for Changing This Recommendation

This recommendation should be revisited if:
1. Independent Red Team review reveals a fatal flaw in Rust's RFC 8785 or JSON Schema Draft 2020-12 crate ecosystem.
2. The project owner explicitly decides against Rust due to development bandwidth constraints, preferring Go's simpler syntax.
3. Downstream consumer integration strategy pivots away from in-process library embedding to an exclusively decoupled CLI pipe over standard I/O.

---

## 8. Owner Decision Gate

> **GOVERNANCE NOTICE:**
> This ADR constitutes a technical recommendation. Pursuant to repository governance, **only the repository owner (`Eren Tahiroğlu`) may approve this ADR** following independent Red Team review. No code implementation may begin until formal acceptance is recorded in Issue #2.
