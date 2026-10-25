# Security Architecture, Threat Delta, and Adversarial Traceability

> **Milestone:** `GOV-00`
> **Status:** `PROPOSED SECURITY SPECIFICATION / DRAFT FOR REVIEW`
> **Traceability:** Aligns with [SEC-01 (Issue #9)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) and [SEC-02 (Issue #10)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10)

---

## 1. Primary-Source Security Research & Standards Classification

To avoid reinventing existing standards or introducing premature complexity, candidate security technologies are evaluated and classified under five explicit tiers:
`USE NOW`, `REUSE LATER`, `OPTIONAL`, `NOT REQUIRED`, or `INCOMPATIBLE OR INSUFFICIENT`.

| Standard / Technology | Classification | Rationale and Context |
|---|---|---|
| **JSON Schema Draft 2020-12** | **USE NOW** | Core input/output contract enforcement. Enables strict structural validation, format constraints, and `additionalProperties: false` to block parameter injection. |
| **RFC 8785 (JCS)** | **USE NOW** | Mathematical foundation for deterministic JSON hashing. Essential for content-addressed verification across heterogeneous platforms. |
| **Git Object Verification (SHA-1/SHA-256)** | **USE NOW** | Binding full 40-character commit SHAs and tree hashes directly to repository identity. |
| **GitHub Actions Least Privilege (`permissions: contents: read`)** | **USE NOW** | Strict elimination of write tokens in CI runners to prevent token exfiltration and unauthorized branch mutations. |
| **SLSA Provenance (v1.0)** | **REUSE LATER** | Standard for build integrity. Valuable for downstream binary distribution in later phases, but premature during core engine bootstrapping. |
| **in-toto Attestation Framework** | **REUSE LATER** | Multi-step supply chain attestation model. Standard envelope structures (`I-TOTO-STATEMENT`) will be evaluated in GOV-04 for archival bundling. |
| **GitHub Artifact Attestations** | **REUSE LATER** | Built-in GitHub Sigstore integration for Actions builds. Documents build provenance, but does not prove code correctness or test sufficiency. Evaluated for GOV-03. |
| **Sigstore (Cosign / Rekor)** | **OPTIONAL** | Keyless signing and transparency logs. Strong external trust anchor candidate for GOV-04; deferred to avoid premature SaaS/OIDC complexity in MVP. |
| **OpenID Connect (OIDC) Federated Auth** | **REUSE LATER** | Tokenless authentication from GitHub Actions to external trust anchors. Deferred to GOV-03/GOV-04. |
| **Proprietary SaaS Verification Services** | **NOT REQUIRED** | Engine must operate completely offline without external SaaS dependencies. |
| **Static Passwords / Long-Lived Personal Access Tokens (PATs)** | **INCOMPATIBLE** | High-risk credential leakage vector; strictly prohibited in verifier workflows. |

---

## 2. Threat Model Delta and Expanded Threat Register

This register extends the baseline in [docs/THREAT_MODEL.md](../THREAT_MODEL.md) with detailed preconditions, likelihood rationale, and earliest implementation gates.

| ID | Sev | Threat Scenario & Vector | Preconditions & Attacker Capability | Mitigation & Defensive Control | Negative Adversarial Test Fixture | Milestone Gate |
|---|---|---|---|---|---|---|
| **RT-01** | **P0** | **Identity Confusion / Branch Pointer Movement:** Old verified status reused on a different repo or moved branch. | Write access to a fork or branch with identical name. | Cryptographically and structurally bind immutable GitHub Repo ID + full 40-character commit SHA + Git tree hash. | `docs/gov-00/fixtures/input_invalid_wrong_repo_id.json` | GOV-01 |
| **RT-02** | **P0** | **Forged CI Run / Superseded Attempt:** Referencing an older run or passing off attempt 1 when attempt 2 failed. | Triggering workflow re-runs; manipulating PR checkout refs. | Bind workflow path + Run ID + Run Attempt. Fail closed on skips, cancellations, or attempt mismatches. | `docs/gov-00/fixtures/input_invalid_wrong_run_attempt.json` and `input_invalid_missing_ci_jobs.json` | GOV-01 |
| **RT-03** | **P0** | **Self-Certification Injection:** Agent or contributor writes `verified=true` or `accepted=true` into input manifest. | Submitting candidate manifest payload. | Enforce `additionalProperties: false`; reject any input containing authority assertions. | `docs/gov-00/fixtures/input_invalid_self_accepted_injection.json` | GOV-01 |
| **RT-04** | **P0** | **Hostile Input Parser Exploits:** Oversized payloads, extreme recursion, or malformed JSON crash verifier or execute code. | Supplying malformed or hostile input files. | Size-bounded parsing (max 10MB), bounded recursion depth (max 32 levels), memory-safe runtime (Rust). | `docs/gov-00/fixtures/input_malformed_syntax.txt` | GOV-01 |
| **RT-05** | **P1** | **Expired Artifact Trust Decay:** Artifact expires or is deleted from GitHub, but UI shows unconditional green status. | Time expiration (90 days) or manual artifact deletion. | Explicit `Evidence Availability` dimension. Degrades to `EXPIRED` if artifact cannot be retrieved. | `docs/gov-00/fixtures/input_invalid_expired_artifact.json` | GOV-02 |
| **RT-06** | **P1** | **History Rewrite / Privileged Tampering:** Repository administrator rewrites Git history or replaces past commits. | Stolen admin PAT or compromised maintainer account. | Acknowledge that local hash chains cannot prevent full repository rewrites. Require external trust anchors and offsite archives. | Archival round-trip integrity test (GOV-04). | GOV-04 |
| **RT-07** | **P1** | **Decompression Attacks (Zip Bombs, Traversal):** Evidence bundles contain zip bombs, path traversal (`../`), or symlinks. | Supplying archive evidence files. | Reject symlinks/hardlinks, reject path traversal characters before extraction, enforce uncompressed size/ratio caps. | Adversarial zip bomb fixture test. | GOV-04 |
| **RT-08** | **P1** | **Malicious Workflow Injection:** CI workflow definition altered in a PR branch to mock verifier outputs. | Modifying `.github/workflows/` files in PR. | Verify workflow definition in the protected base branch; pin all third-party Actions to immutable commit SHAs. | Pinned action audit check. | GOV-03 |
| **RT-09** | **P1** | **Secret and Private Path Leakage:** Sensitive tokens or workstation paths embedded in evidence records or public issues. | Misconfigured logging or raw log ingestion. | Enforce redaction schemas; refuse ingestion of raw system logs; enforce `.gitignore` rules. | Regex secret scanner in pre-commit hooks. | GOV-01 |
| **RT-10** | **P1** | **Concurrency & Replay Attacks:** Concurrent agents create conflicting checkpoints or replay an earlier valid checkpoint. | Multiple agent sessions writing concurrently. | Monotonic checkpoint sequencing, deterministic timestamp/SHA binding, replay detection, explicit conflict rejection. | Replay sequence test fixture. | GOV-05 |
| **RT-11** | **P2** | **Compromised AI Agent Prompt Injection:** Embedded prompt injection in commit messages or issues attempts to hijack agent behavior. | Attacker submits crafted text in PR description or commit log. | Strict data/instruction separation: input manifests are parsed strictly as passive structured data, never fed into agent system prompts as instructions. | Prompt injection fixture test. | GOV-05 |
| **RT-12** | **P2** | **Stale Historical Baseline Display:** Consumer UI displays an older accepted baseline as if it validates current HEAD. | User viewing outdated dashboard without revalidating HEAD. | Explicit `Freshness` dimension comparing evaluated SHA directly against the live remote branch HEAD. | HEAD divergence test fixture. | GOV-06 |
| **RT-13** | **P1** | **64-Bit Integer Precision Loss:** JavaScript or dynamic JSON parser rounds GitHub IDs exceeding $2^{53}-1$. | Ingesting large numeric IDs via IEEE-754 floats. | Mandate canonical decimal strings (`^[0-9]{1,20}$`) for all repository, run, and artifact IDs. | Integer boundary test fixture ($2^{53}+1$). | GOV-01 |
| **RT-14** | **P0** | **PR Head vs Synthetic Merge Ref Confusion:** Verifier checks GitHub's transient `refs/pull/X/merge` instead of the actual immutable PR commit. | PR branch head changes while synthetic merge is evaluated. | Explicitly bind and verify `head_sha` and `actual_checkout_sha`; reject synthetic merge ref for final acceptance baselines. | Merge ref mismatch test fixture. | GOV-02 |

---

## 3. Threat-to-Test Traceability Matrix

| Threat ID | Security Issue | Implementation Milestone | Negative Test Vector / Fixture File | Verification Method |
|---|---|---|---|---|
| **RT-01** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `docs/gov-00/fixtures/input_invalid_wrong_repo_id.json` | Programmatic verification rejects non-matching repo ID. |
| **RT-02** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `docs/gov-00/fixtures/input_invalid_missing_ci_jobs.json` | CI evaluator rejects missing expected checks. |
| **RT-03** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `docs/gov-00/fixtures/input_invalid_self_accepted_injection.json` | Schema validator rejects undeclared authority fields. |
| **RT-04** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-01 | `docs/gov-00/fixtures/input_malformed_syntax.txt` | Parser exits with non-zero code on malformed input. |
| **RT-05** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-02 | `docs/gov-00/fixtures/input_invalid_expired_artifact.json` | Evidence availability dimension reports `EXPIRED`. |
| **RT-07** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-04 | Decompression test fixtures (zip bomb / traversal) | Archive extractor refuses paths containing `../` or symlinks. |
| **RT-09** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-01 | Secret-bearing negative fixtures | Schema and linter reject raw tokens and absolute paths. |
| **RT-13** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `docs/gov-00/schemas/checkpoint-input.schema.json` | Schema regex `^[0-9]{1,20}$` enforces string representation. |
| **RT-14** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-02 | Mismatched checkout SHA fixture | Adapter checks `actual_checkout_sha == commit_sha`. |

---

## 4. Residual Risks and Unresolved Security Boundaries

1. **Compromised Account Authority (RT-06 / RT-10):** If the repository owner's GitHub account or workstation is fully compromised, an attacker can push malicious code or sign unauthorized decisions. Mitigation requires multi-factor authentication, commit signing (GPG/SSH), and future offsite trust anchors.
2. **Third-Party Action Supply Chain (RT-08):** Pinned commit hashes protect against tag mutation, but do not prevent malicious commits if the upstream action repository is backdoored. Mitigation requires maintaining local action mirrors or using minimal, audited actions.
3. **Draft Status:** These defensive controls are currently architectural specifications. **No code implementation or automated test runner has been executed in GOV-00.**
