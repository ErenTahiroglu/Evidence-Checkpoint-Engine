# Red Team Threat Model and Risk Register

> **Status: ARCHITECTURAL BASELINE / DRAFT FOR GOV-00.**
> This document enumerates falsifiable threats and required defensive controls. No verification code has yet run against these threats.

## Threat Register

| Threat ID | Priority | Threat Scenario / Failure Mode | Attacker Capability & Vector | Required Mitigation & Negative Tests |
|---|---|---|---|---|
| **RT-01** | **P0** | **Identity Confusion / Branch Pointer Movement:** An attacker reuses an old verified status on a different repository, modified branch, or substituted commit. | Write access to a branch or control over a fork with identical branch name. | Cryptographically and structurally bind immutable GitHub Repo ID + full 40-character commit SHA + Git tree hash. Automated tests for wrong-repo and moved-HEAD rejection. |
| **RT-02** | **P0** | **Forged CI Success / Superseded Attempt:** An attacker references a successful run of an older workflow or an earlier attempt of a failed re-run. | Triggering workflow re-runs; manipulating PR merge ref commits. | Bind workflow definition file + Run ID + Run Attempt. Fail closed if run attempt does not match the exact evaluation target or if any expected check job is missing, skipped, or cancelled. |
| **RT-03** | **P0** | **Self-Certification / Agent Elevation:** An autonomous AI agent or malicious contributor writes `verified=true` or `accepted=true` into an input manifest or PR body. | Supplying structured metadata to the verifier or CI inbox. | Strict claim separation: user/agent inputs are treated as `SOURCE_DECLARED`. Verifier rejects any input that pre-asserts verifier-derived status. Reviewer and owner records require independent cryptographically bound records. |
| **RT-04** | **P0** | **Untrusted Payload Execution:** Malicious input manifests, CI logs, or archive artifacts exploit parser vulnerabilities to execute arbitrary code or exfiltrate environment secrets. | Crafting malicious JSON payloads, malformed strings, or polyglot files. | Strict schema enforcement, size-bounded input parsers, zero script execution, inert data handling, memory-safe runtime (evaluated in GOV-00), no network access during offline validation. |
| **RT-05** | **P1** | **Expired Artifact Trust Decay:** An artifact expires or is deleted from GitHub storage, but the dashboard continues to display an unconditional green status. | Time expiration (GitHub Actions default retention: 90 days) or deliberate artifact deletion. | Explicit `Evidence Availability` dimension. If reference artifacts cannot be fetched or verified by digest, degrade status to `INSUFFICIENT_EVIDENCE` or `EXPIRED` rather than preserving a stale PASS. |
| **RT-06** | **P1** | **History Rewrite / Privileged Tampering:** An attacker with repository administrative privileges rewrites git history or git commit objects. | Administrative GitHub credentials or stolen PAT. | Recognize that internal hash chains alone do not prevent complete repository rewrites. Plan for external trust anchors and offsite archival (GOV-04). |
| **RT-07** | **P1** | **Archive Decompression Attacks:** Evidence packages contain zip bombs, directory traversal (`../`), symlinks, or oversized recursive archives. | Supplying malicious zip or tar evidence bundles. | Strict decompression bounds (max uncompressed size, max file count, max depth), reject symlinks/hardlinks, reject path traversal characters before extraction. |
| **RT-08** | **P1** | **Malicious Workflow Injection:** CI workflow definition is modified on a branch to bypass tests or mock verifier outputs. | Modifying `.github/workflows/` files in a pull request. | Inspect workflow definition in the base branch; pin all third-party GitHub Actions to full immutable commit hashes; fail closed on unapproved workflow changes. |
| **RT-09** | **P1** | **Secret and Private Path Leakage:** Sensitive tokens, API keys, or private machine absolute paths are incorporated into verification outputs or published issues. | Misconfigured environment logging or raw log ingestion. | Enforce redaction schemas; refuse ingestion of raw system logs; test with secret-bearing negative fixtures; enforce `.gitignore` rules. |
| **RT-10** | **P1** | **Concurrency & Replay Attacks:** Concurrent agents create conflicting checkpoints or replay an earlier valid checkpoint to overwrite newer findings. | Multiple agent sessions writing to the same issue or inbox. | Monotonic checkpoint sequencing, deterministic timestamp/SHA binding, replay detection, explicit conflict rejection (no last-write-wins). |
| **RT-11** | **P1** | **Account / Machine Compromise:** The developer workstation or GitHub account credential is breached. | Credential compromise. | Acknowledge common-control limitation; maintain multi-factor authorization; separate review authority from author identity; plan for offsite trust anchors. |
| **RT-12** | **P2** | **Stale Historical Baseline Display:** A consumer UI displays an older accepted baseline as if it validates the current branch tip. | User viewing outdated checkpoint dashboard without revalidating HEAD. | Explicit `Freshness` dimension comparing evaluated SHA directly against the live remote branch HEAD; display clear visual warnings for non-head evaluations. |

---

## Non-Negotiable Hard Stops

1. **No Automatic Writes:** The verifier will not automatically post comments, open issues, or modify repository settings without explicit human authorization.
2. **No Ambient Privileges:** The core engine operates offline without requiring network permissions or credentials.
3. **No Code Execution:** Untrusted inputs, evidence files, and archived fixtures are data, not code. They will never be executed, evaluated in a shell, or passed to dynamic interpreters.
4. **No Claim of Magic Immunity:** Neither GitHub Artifact Attestations nor cryptographic hashes prove that code is bug-free or secure; they prove only origin and byte equality.
