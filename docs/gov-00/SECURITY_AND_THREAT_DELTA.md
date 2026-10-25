# Security Architecture, Threat Delta, and Adversarial Traceability

> **Milestone:** `GOV-00 R2`
> **Status:** `PROPOSED SECURITY SPECIFICATION / REMEDIATED CONTRACT DRAFT`
> **Traceability:** Aligns with [SEC-01 (Issue #9)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) and [SEC-02 (Issue #10)](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10)

---

## 1. Primary-Source Security Research & Standards Classification

To avoid reinventing existing standards or introducing premature complexity, candidate security technologies are evaluated and classified under five explicit tiers:
`USE NOW`, `REUSE LATER`, `OPTIONAL`, `NOT REQUIRED`, or `INCOMPATIBLE OR INSUFFICIENT`.

| Standard / Technology | Classification | Rationale and Context |
|---|---|---|
| **JSON Schema Draft 2020-12** | **USE NOW** | Core input, envelope, and output contract enforcement. Enables strict structural validation, format constraints, conditional schemas (`allOf`/`if`/`then`), and `additionalProperties: false` to block parameter injection. |
| **RFC 8785 (JCS)** | **USE NOW** | Mathematical foundation for deterministic JSON hashing. Essential for content-addressed verification across heterogeneous platforms. |
| **Git Object Verification (SHA-1/SHA-256)** | **USE NOW** | Binding full 40-character (SHA-1) and 64-character (SHA-256) commit SHAs and tree hashes directly to repository identity. |
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
| **RT-01** | **P0** | **Identity Confusion / Branch Pointer Movement:** Old verified status reused on a different repo or moved branch. | Write access to a fork or branch with identical name. | Cryptographically and structurally bind immutable GitHub Repo ID + full commit SHA + Git tree hash. | `docs/gov-00/fixtures/input_invalid_wrong_repo_id.json` | GOV-01 |
| **RT-02** | **P0** | **Forged CI Run / Superseded Attempt:** Referencing an older run or passing off attempt 1 when attempt 2 failed. | Triggering workflow re-runs; manipulating PR checkout refs. | Bind workflow path + Run ID + Run Attempt. Fail closed on skips, cancellations, or attempt mismatches. | `docs/gov-00/fixtures/input_invalid_wrong_run_attempt.json` and `input_invalid_missing_ci_jobs.json` | GOV-01 |
| **RT-03** | **P0** | **Self-Certification Injection:** Agent or contributor writes `verified=true` or `accepted=true` into input manifest. | Submitting candidate manifest payload. | Enforce `additionalProperties: false`; reject any input containing authority assertions. | `docs/gov-00/fixtures/input_invalid_self_accepted_injection.json` | GOV-01 |
| **RT-04** | **P0** | **Hostile Input Parser Exploits:** Oversized payloads, extreme recursion, or malformed JSON crash verifier or execute code. | Supplying malformed or hostile input files. | Size-bounded parsing (max 10MB), bounded recursion depth (max 32 levels), memory-safe runtime (Rust). | `docs/gov-00/fixtures/input_malformed_syntax.txt` | GOV-01 |
| **RT-05** | **P1** | **Expired Artifact Trust Decay:** Artifact expires or is deleted from GitHub, but UI shows unconditional green status. | Retention window elapses or storage provider purges artifact. | Explicit `Evidence Availability` dimension. Degrades to `EXPIRED` when storage provider indicates object purged; isolates expiration from cross-repo substitution. | `docs/gov-00/fixtures/input_invalid_expired_artifact.json` | GOV-02 |
| **RT-06** | **P1** | **History Rewrite / Privileged Tampering:** Repository administrator rewrites Git history or replaces past commits. | Stolen admin PAT or compromised maintainer account. | Acknowledge that local hash chains cannot prevent full repository rewrites. Require external trust anchors and offsite archives. | Archival round-trip integrity test (GOV-04). | GOV-04 |
| **RT-07** | **P1** | **Decompression Attacks (Zip Bombs, Traversal):** Evidence bundles contain zip bombs, path traversal (`../`), or symlinks. | Supplying archive evidence files. | Reject symlinks/hardlinks, reject path traversal characters before extraction, enforce uncompressed size/ratio caps. | Adversarial zip bomb fixture test. | GOV-04 |
| **RT-08** | **P1** | **Malicious Workflow Injection:** CI workflow definition altered in a PR branch to mock verifier outputs. | Modifying `.github/workflows/` files in PR. | Verify workflow definition in the protected base branch; pin all third-party Actions to immutable commit SHAs. | Pinned action audit check. | GOV-03 |
| **RT-09** | **P1** | **Secret and Private Path Leakage:** Sensitive tokens or workstation paths embedded in evidence records or public issues. | Misconfigured logging or raw log ingestion. | Enforce redaction schemas; refuse ingestion of raw system logs; enforce `.gitignore` rules. | Pre-commit regex scanner. | GOV-01 |
| **RT-10** | **P1** | **Concurrency & Replay Attacks:** Concurrent agents create conflicting checkpoints or replay an earlier valid checkpoint. | Multiple agent sessions writing concurrently. | Monotonic checkpoint sequencing, deterministic timestamp/SHA binding, replay detection, explicit conflict rejection. | Replay sequence test fixture. | GOV-05 |
| **RT-11** | **P2** | **Compromised AI Agent Prompt Injection:** Embedded prompt injection in commit messages or issues attempts to hijack agent behavior. | Attacker submits crafted text in PR description or commit log. | Strict data/instruction separation: input manifests are parsed strictly as passive structured data, never fed into agent system prompts as instructions. | Prompt injection fixture test. | GOV-05 |
| **RT-12** | **P2** | **Stale Historical Baseline Display:** Consumer UI displays an older accepted baseline as if it validates current HEAD. | User viewing outdated dashboard without revalidating HEAD. | Explicit `Freshness` dimension comparing evaluated SHA directly against the live remote branch HEAD. | HEAD divergence test fixture. | GOV-06 |
| **RT-13** | **P1** | **64-Bit Integer Precision Loss & Out-of-Bounds Values:** JSON parser rounds IDs exceeding $2^{53}-1$ or permits numbers beyond uint64::MAX. | Ingesting large numeric IDs via IEEE-754 floats or unvalidated strings. | Schema regex `^[1-9][0-9]{0,19}$` enforces canonical non-zero string formatting; semantic validation strictly checks `val <= uint64::MAX`. | `input_invalid_uint64_overflow_edge.json`, `input_invalid_uint64_twenty_nines.json`, `input_invalid_id_leading_zero.json` | GOV-01 |
| **RT-14** | **P0** | **PR Head vs Synthetic Merge Ref Confusion:** Verifier checks GitHub's transient `refs/pull/X/merge` instead of the actual immutable PR commit. | PR branch head changes while synthetic merge is evaluated. | Explicitly bind and verify `head_sha` and `actual_checkout_sha`; reject synthetic merge ref for final acceptance baselines. | Merge ref mismatch test fixture. | GOV-02 |
| **RT-15** | **P0** | **Cross-Repository CI Provenance Substitution:** Checkpoint pairs valid repository ID with a real GitHub Actions Run ID belonging to an entirely different repository. | Obtaining valid Run ID from external public repo (e.g. AI-Toolchain-Manager). | Authoritative repository scope verification; adapter validates that `run_id` belongs to the exact declared repository. | `docs/gov-00/fixtures/input_invalid_cross_repo_ci_substitution.json` | GOV-02 |
| **RT-16** | **P1** | **Git Hash Algorithm & Length Mismatch:** Checkpoint declares `sha1` but supplies 64-character hash, or declares `sha256` but supplies 40-character hash. | Accidental confusion or malicious length substitution. | Conditional JSON Schema validation (`allOf`/`if`/`then`) strictly binding declared algorithm to exact hex length. | `docs/gov-00/fixtures/input_invalid_git_sha1_with_sha256_length.json` and `input_invalid_git_sha256_with_sha1_length.json` | GOV-01 |
| **RT-17** | **P0** | **Untrusted Producer CI Check Omission:** Developer or agent manifest defines its own `expected_jobs` array, omitting required security or lint checks. | Submitting self-serving check list without independent policy reference. | Independent check authority: mandatory `expected_policy_ref` and `expected_policy_digest` in `ci_evidence`; verifier matches against independently approved policy baseline. | `input_invalid_ci_missing_policy.json` and `input_invalid_ci_untrusted_policy_digest.json` | GOV-02 |
| **RT-18** | **P1** | **Untrusted Storage Expiration Declaration:** Producer attempts to self-declare an artifact expired via untrusted payload field. | Submitting unverified `observed_availability` declaration. | Stripped `observed_availability` from input schema; availability is strictly evaluated by engine storage interaction. | `docs/gov-00/fixtures/input_invalid_expired_artifact.json` | GOV-02 |
| **RT-19** | **P0** | **Synthetic Mock Authority Confusion:** Synthetic or mock assessment output claims clean provisional pass or human authorization. | Fabricating output records or passing test mocks into production decision pipelines. | Conditional schema constraints binding `is_synthetic_mock: true` to non-pass verdicts and `human_authorized.authorized: false`. | `output_invalid_mock_claiming_pass.json` and `output_invalid_synthetic_claiming_human_auth.json` | GOV-01 |
| **RT-20** | **P0** | **Envelope Tampering / Digest Mismatch:** Payload content modified while outer envelope digest remains unchanged. | Altering candidate metadata after digest calculation. | RFC 8785 canonical hash verification over `payload` compared against `envelope_manifest_digest`. | `docs/gov-00/fixtures/envelope_invalid_tampered_digest.json` | GOV-01 |

---

## 3. Threat-to-Test Traceability Matrix

| Threat ID | Security Issue | Implementation Milestone | Negative Test Vector / Fixture File | Verification Method |
|---|---|---|---|---|
| **RT-01** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `docs/gov-00/fixtures/input_invalid_wrong_repo_id.json` | Programmatic verification rejects non-matching repo ID. |
| **RT-02** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `docs/gov-00/fixtures/input_invalid_wrong_run_attempt.json` | CI evaluator rejects superseded/invalid run attempts. |
| **RT-03** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `docs/gov-00/fixtures/input_invalid_self_accepted_injection.json` | Schema validator rejects undeclared authority fields. |
| **RT-04** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-01 | `docs/gov-00/fixtures/input_malformed_syntax.txt` | Parser exits with non-zero code on malformed input. |
| **RT-05** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-02 | `docs/gov-00/fixtures/input_invalid_expired_artifact.json` | Evidence availability dimension reports `EXPIRED`. |
| **RT-07** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-04 | Decompression test fixtures (zip bomb / traversal) | Archive extractor refuses paths containing `../` or symlinks. |
| **RT-09** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-01 | Pre-commit regex scan | Linter and schema reject secrets and absolute paths. |
| **RT-13** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `input_invalid_uint64_overflow_edge.json`<br>`input_invalid_id_leading_zero.json` | Schema regex rejects formatting; semantic check rejects $> 2^{64}-1$. |
| **RT-14** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-02 | Mismatched checkout SHA fixture | Adapter checks `actual_checkout_sha == commit_sha`. |
| **RT-15** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-02 | `docs/gov-00/fixtures/input_invalid_cross_repo_ci_substitution.json` | Cross-repository provenance check rejects mismatched Run ID. |
| **RT-16** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `input_invalid_git_sha1_with_sha256_length.json`<br>`input_invalid_git_sha256_with_sha1_length.json` | Schema conditional logic rejects 40/64-char algorithm mismatch. |
| **RT-17** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-02 | `input_invalid_ci_missing_policy.json`<br>`input_invalid_ci_untrusted_policy_digest.json` | Schema requires policy ref/digest; evaluator checks policy root. |
| **RT-18** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-02 | `docs/gov-00/fixtures/input_invalid_expired_artifact.json` | Schema disallows producer declaration of availability. |
| **RT-19** | [SEC-02 #10](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/10) | GOV-01 | `output_invalid_mock_claiming_pass.json`<br>`output_invalid_synthetic_claiming_human_auth.json` | Conditional schema rejects mock authority elevation. |
| **RT-20** | [SEC-01 #9](https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/issues/9) | GOV-01 | `docs/gov-00/fixtures/envelope_invalid_tampered_digest.json` | Envelope verifier rejects mismatched canonical digest. |

---

## 4. Residual Risks and Unresolved Security Boundaries

1. **Compromised Account Authority (RT-06 / RT-10):** If the repository owner's GitHub account or workstation is fully compromised, an attacker can push malicious code or sign unauthorized decisions. Mitigation requires multi-factor authentication, commit signing (GPG/SSH), and future offsite trust anchors.
2. **Third-Party Action Supply Chain (RT-08):** Pinned commit hashes protect against tag mutation, but do not prevent malicious commits if the upstream action repository is backdoored. Mitigation requires maintaining local action mirrors or using minimal, audited actions.
3. **Draft Status:** These defensive controls are currently architectural specifications. **No code implementation or automated test runner has been executed in GOV-00.**
