# Trust Architecture and Multi-Dimensional State Model

> **Milestone:** `GOV-00 R2`
> **Status:** `PROPOSED ARCHITECTURE / REMEDIATED CONTRACT DRAFT`
> **Applicability:** Standalone Verification Core and Downstream Consumers

---

## 1. Core Trust Invariants

Traditional engineering verification often suffers from a fatal flaw: collapsing complex, multi-faceted verification properties into a single one-dimensional `PASS` or `FAIL` badge. This leads to severe security failures, such as:
- Mistaking a green CI run on an outdated branch for production approval.
- Treating developer-authored PR descriptions as proof of independent review.
- Assuming an artifact that was valid 90 days ago is still verifiable when GitHub has expired its storage.
- Permitting autonomous AI agents to self-certify their own code by setting `verified=true`.

`Evidence-Checkpoint-Engine` eliminates these failure modes by enforcing **strict authority tiering** and a **six-dimensional orthogonal state model**.

---

## 2. The Four Claim and Authority Tiers

Every statement within the checkpoint ecosystem belongs to exactly one authority tier:

```
+-------------------------------------------------------------------------+
| [TIER 1: SOURCE_DECLARED]                                              |
| Declared by: Developer, AI Agent, or Automated Bot                     |
| Authority: ZERO. Treated strictly as untrusted candidate input.         |
| Examples: Manifest metadata, claimed commit SHA, claimed run ID.        |
+-------------------------------------------------------------------------+
                                    |
                                    v (Programmatic Verification)
+-------------------------------------------------------------------------+
| [TIER 2: MACHINE_ASSESSED]                                              |
| Determined by: Independent Verification Core / Read-Only Adapter        |
| Authority: Cryptographic & Structural Proof only.                       |
| Examples: RFC 8785 byte digest matches, commit SHA exists, CI run matches|
+-------------------------------------------------------------------------+
                                    |
                                    v (Independent Technical Audit)
+-------------------------------------------------------------------------+
| [TIER 3: INDEPENDENTLY_REVIEWED]                                        |
| Performed by: Independent Reviewer / Red Team Auditor                   |
| Authority: Scoped technical evaluation of implementation & residual risk|
| Examples: Static documentation pass, adversarial parser audit sign-off. |
+-------------------------------------------------------------------------+
                                    |
                                    v (Governance Decision)
+-------------------------------------------------------------------------+
| [TIER 4: HUMAN_AUTHORIZED]                                              |
| Decided by: Designated Repository Owner (Eren Tahiroglu)                 |
| Authority: Sole authority to accept, baseline, or promote to main.      |
| Examples: Formal promotion decision recorded on an immutable SHA.       |
+-------------------------------------------------------------------------+
```

### Invariant Rules:
1. **No Self-Elevation:** An entity at Tier 1 can never assign a Tier 2, Tier 3, or Tier 4 status. Source-declared review claims or authorization claims are rejected as unauthorized inputs.
2. **No Automatic Promotion:** Machine assessment (Tier 2) and independent review (Tier 3) are necessary prerequisites, but they **never automatically confer Human Authorization (Tier 4)**.
3. **No GUI Authority:** External consumer interfaces (such as AI-Toolchain-Manager) have zero authority to mint, upgrade, or bypass any tier.
4. **No Authenticity from Output JSON Alone:** An assessment output JSON document can be syntactically valid while completely fabricated. Consumers must verify the **evaluator authentication context** (e.g. executed via a verified local subprocess pipe or authenticated via a detached cryptographic signature) before trusting machine-assessed evidence.

---

## 3. The Six Independent Verification Dimensions

Rather than a single binary status, an evaluation outcome is represented as a tuple of six independent dimensions:

$$\text{Checkpoint Assessment} = \langle D_{\text{integrity}}, D_{\text{ci}}, D_{\text{availability}}, D_{\text{review}}, D_{\text{authorization}}, D_{\text{freshness}} \rangle$$

---

### Dimension A: Evidence Integrity ($D_{\text{integrity}}$)
Evaluates whether input data, schemas, canonical bytes, and reference hashes are internally sound and untampered.

- **Allowed States (Enforced Schema Enum):**
  - `VALID`: Schema matches Draft 2020-12, RFC 8785 canonicalization succeeds, calculated digests match references.
  - `INVALID`: Schema syntax error, invalid regex pattern, or mismatched cryptographic digest.
  - `CORRUPT`: Unparseable bytes, truncated JSON, or decoding failure.
  - `TAMPERED`: Structural collision, duplicate keys detected, or envelope tampering.
  - `NOT_EVALUATED`: Check bypassed or skipped due to upstream parser error.
- **Contradiction Behavior:** If calculated digest does not match declared digest, state is immediately `INVALID`.

---

### Dimension B: CI Execution ($D_{\text{ci}}$)
Evaluates what actually executed in automated CI environments versus independently approved policy expectations.

- **Allowed States (Enforced Schema Enum):**
  - `VERIFIED_PASS`: Workflow definition, Run ID, and exact Run Attempt match; 100% of expected checks executed with `conclusion: success`.
  - `FAILED`: One or more observed checks failed or timed out.
  - `INCOMPLETE_SKIPPED`: Any check in the expected list was skipped, cancelled, neutral, or blocked.
  - `ATTEMPT_MISMATCH`: The submitted evidence references an older or mismatched workflow run attempt.
  - `UNVERIFIED_RUN`: Workflow run cannot be correlated with the exact commit SHA or repository ID.
  - `NOT_APPLICABLE`: Phase does not require CI checks (e.g. documentation-only bootstrap).
- **Independent Expected Checks Authority:** The developer's candidate manifest may propose expected checks, but the authoritative check list is determined by an independently reviewed policy baseline (`expected_policy_ref` and `expected_policy_digest`). An offline core unable to reach policy evidence cannot assign `VERIFIED_PASS`.
- **Synthetic Merge vs PR Head:** Distinguishes between the PR head commit and GitHub's synthetic merge ref (`refs/pull/X/merge`). A workflow run on a synthetic merge commit proves that the merge result compiled, but does not prove approval of the unmerged branch head.

---

### Dimension C: Evidence Availability ($D_{\text{availability}}$)
Evaluates whether supporting evidence (artifacts, test logs, source trees) remains currently accessible and verifiable.

- **Allowed States (Enforced Schema Enum):**
  - `AVAILABLE`: All referenced artifacts and logs are currently retrievable and their byte digests match.
  - `PARTIALLY_AVAILABLE`: Non-critical secondary logs are missing, but primary evidence is present.
  - `EXPIRED`: GitHub artifact retention window (e.g., 90 days) has elapsed and artifact is deleted from live storage.
  - `UNREACHABLE`: Network failure, rate-limiting, or permission denial prevents fetching live evidence.
  - `NOT_PROVIDED`: Manifest does not reference external artifacts.
- **Distinction Between Expiration, Inaccessibility, and Corruption:**
  - An expired artifact is an expected operational lifecycle event, not evidence corruption.
  - A generic HTTP 404 or connection error is **not** automatically definitive proof of expiration (it could indicate network outage, permission denial, or rate-limiting, resolving to `UNREACHABLE`).
  - True `EXPIRED` status requires authoritative confirmation that the retention duration elapsed and the storage provider purged the object.
  - If the original artifact expired but an independently verified offsite archive exists, the availability status is `AVAILABLE` (with `is_archive: true`).
- **No Untrusted Expiration Declarations:** Producer input manifests cannot declare `observed_availability`. Expiration is strictly an engine-observed status ($D_{\text{availability}} = \text{EXPIRED}$) based on authoritative storage provider interaction.

---

### Dimension D: Review Status ($D_{\text{review}}$)
Evaluates whether an independent human or automated auditor has reviewed the candidate baseline.

- **Allowed States (Enforced Schema Enum):**
  - `INDEPENDENT_REVIEW_PASS`: An independent technical reviewer has inspected the code/evidence and found no unaddressed blocking risks.
  - `STATIC_PASS_WITH_LIMITATIONS`: Static review passed, but runtime reproduction or independent local build was not executed.
  - `SELF_REPORT_ONLY`: Only the author or creating agent reviewed their own work.
  - `REVIEW_FAILED`: Independent review discovered critical unmitigated vulnerabilities or rule violations.
  - `UNREVIEWED`: No review record attached.

---

### Dimension E: Authorization ($D_{\text{authorization}}$)
Evaluates whether the repository owner has granted governance approval for the checkpoint.

- **Allowed States (Enforced Schema Enum):**
  - `EXPLICITLY_AUTHORIZED`: The repository owner has provided written, verifiable authorization for this exact SHA.
  - `AUTHORIZATION_PENDING`: Candidate is undergoing evaluation; no owner decision recorded.
  - `REJECTED`: Owner has explicitly rejected the checkpoint.
  - `REVOKED`: A previously authorized baseline was revoked due to newly discovered security issues.
- **Authority Boundary:** Only the owner (`Eren Tahiroglu`) can transition this dimension to `EXPLICITLY_AUTHORIZED`.

---

### Dimension F: Freshness ($D_{\text{freshness}}$)
Evaluates whether the checkpoint applies to the current tip of the target branch or is a historical reference.

- **Allowed States (Enforced Schema Enum):**
  - `CURRENT_HEAD`: The evaluated commit SHA matches the exact current remote branch HEAD.
  - `HISTORICAL_BASELINE`: The evaluated commit SHA was an authorized past baseline, but the branch has advanced.
  - `STALE_DIVERGED`: The commit is on a diverged branch or obsolete fork.
  - `UNKNOWN_HEAD`: Remote HEAD cannot be determined (e.g., during offline verification).

---

## 4. Derived Summary Verdict Formulation and Precedence Policy

A summary verdict is emitted for convenience, but it is strictly a **conservative deterministic function** of the six underlying dimensions:

$$\text{Verdict} = f(D_{\text{integrity}}, D_{\text{ci}}, D_{\text{availability}}, D_{\text{review}}, D_{\text{authorization}}, D_{\text{freshness}})$$

### 4.1 Strict Precedence Hierarchy
When individual dimensions yield conflicting signals (e.g. one dimension is `FAILED`, another `UNKNOWN_HEAD`, and another historically `EXPLICITLY_AUTHORIZED`), the engine enforces a strict fail-closed precedence order:

$$\text{FAILED} > \text{REFUSED} > \text{UNKNOWN / INSUFFICIENT\_EVIDENCE} > \text{ASSESSED\_LIMITATIONS} > \text{ASSESSED\_PROVISIONAL\_PASS}$$

1. **Failure Dominance:** If **any** critical dimension is `INVALID`, `CORRUPT`, `TAMPERED`, `FAILED`, `REVIEW_FAILED`, or `REJECTED`, the overall verdict is strictly **`ASSESSED_FAIL`**. Historical authorization or valid integrity hashes never override an active failure.
2. **Refusal on Hostile / Malformed Inputs:** If the input payload violates the schema, contains illegal injection fields, or exceeds parser bounds, the verdict is **`REFUSED`**.
3. **Insufficient Evidence on Missing / Unreachable Facts:** If no dimension has failed, but one or more required dimensions cannot be verified (e.g. $D_{\text{availability}} = \text{UNREACHABLE}$, $D_{\text{freshness}} = \text{UNKNOWN_HEAD}$, or $D_{\text{ci}} = \text{UNVERIFIED_RUN}$), the verdict is **`INSUFFICIENT_EVIDENCE`**. Missing external information is never converted into a pass.
4. **Limitations on Scoped Restrictions:** If all dimensions are valid, but non-blocking limitations exist (e.g. $D_{\text{review}} = \text{STATIC_PASS_WITH_LIMITATIONS}$ or $D_{\text{authorization}} = \text{AUTHORIZATION_PENDING}$), the verdict is **`ASSESSED_LIMITATIONS`**.
5. **Clean Pass:** Only when 100% of dimensions are affirmatively satisfied without warnings or limitations is **`ASSESSED_PROVISIONAL_PASS`** emitted.

### 4.2 Summary Verdict Matrix

| Summary Verdict | Dimension Invariants Required | Exit Code |
|---|---|---|
| **`ASSESSED_PROVISIONAL_PASS`** | $D_{\text{integrity}} = \text{VALID} \land D_{\text{ci}} \in \{\text{VERIFIED_PASS}, \text{NOT_APPLICABLE}\} \land D_{\text{availability}} \in \{\text{AVAILABLE}, \text{NOT_PROVIDED}\} \land D_{\text{review}} = \text{INDEPENDENT_REVIEW_PASS} \land D_{\text{authorization}} = \text{EXPLICITLY_AUTHORIZED} \land D_{\text{freshness}} = \text{CURRENT_HEAD}$ | `0` |
| **`ASSESSED_LIMITATIONS`** | Integrity is valid, no dimension failed, but non-blocking limitations exist ($D_{\text{review}} = \text{STATIC_PASS_WITH_LIMITATIONS}$ or $D_{\text{authorization}} = \text{AUTHORIZATION_PENDING}$ or $D_{\text{freshness}} = \text{HISTORICAL_BASELINE}$). | `1` |
| **`INSUFFICIENT_EVIDENCE`** | Missing required evidence, $D_{\text{availability}} \in \{\text{EXPIRED}, \text{UNREACHABLE}\}$, $D_{\text{ci}} = \text{UNVERIFIED_RUN}$, or $D_{\text{freshness}} = \text{UNKNOWN_HEAD}$. | `3` |
| **`ASSESSED_FAIL`** | Any critical dimension failed: $D_{\text{integrity}} \in \{\text{INVALID}, \text{CORRUPT}, \text{TAMPERED}\} \lor D_{\text{ci}} \in \{\text{FAILED}, \text{ATTEMPT_MISMATCH}, \text{INCOMPLETE_SKIPPED}\} \lor D_{\text{review}} = \text{REVIEW_FAILED} \lor D_{\text{authorization}} \in \{\text{REJECTED}, \text{REVOKED}\}$. | `2` |
| **`REFUSED`** | Malformed input syntax, schema rejection, hostile payload detected, or unresolvable version conflict. | `3` |

---

## 5. Exit Code Semantics: Verification Verdict vs Execution Success

A critical ambiguity in earlier revisions was whether process exit codes describe **engine execution success** or the **verification verdict**.

**Normative Decision in R1:** The CLI exit code directly reflects the **Verification Verdict**:
- **`0`**: Clean verification pass (`ASSESSED_PROVISIONAL_PASS`).
- **`1`**: Assessment completed with limitations or warnings (`ASSESSED_LIMITATIONS`).
- **`2`**: Verification failure on evaluated evidence (`ASSESSED_FAIL`).
- **`3`**: Input schema rejection, refusal, or unverifiable evidence (`REFUSED` / `INSUFFICIENT_EVIDENCE`).
- **`4`**: Engine runtime execution error (I/O crash, unhandled panic, OS resource exhaustion).

This allows automated scripts, CI gates, and downstream consumers to distinguish clean passes from limitations, failures, and syntax rejections using standard POSIX exit status inspection.

---

## 6. Concrete Edge-Case Scenario Resolutions

| Scenario | Dimensions Resolution | Derived Verdict & Exit Code |
|---|---|---|
| **1. Historically accepted, but branch has advanced** | $D_{\text{auth}} = \text{EXPLICITLY_AUTHORIZED}$, $D_{\text{freshness}} = \text{HISTORICAL_BASELINE}$ | `ASSESSED_LIMITATIONS` (Exit `1`). Output preserves historical acceptance fact but warns that live HEAD has advanced. |
| **2. Historically accepted, but GitHub artifact expired** | $D_{\text{auth}} = \text{EXPLICITLY_AUTHORIZED}$, $D_{\text{avail}} = \text{EXPIRED}$ | `INSUFFICIENT_EVIDENCE` (Exit `3`) if live re-verification requested without offsite archive. |
| **3. CI green, but no independent review** | $D_{\text{ci}} = \text{VERIFIED_PASS}$, $D_{\text{review}} = \text{UNREVIEWED}$ | `ASSESSED_LIMITATIONS` (Exit `1`). Green CI cannot masquerade as audited software. |
| **4. Independently reviewed, but owner authorization pending** | $D_{\text{review}} = \text{INDEPENDENT_REVIEW_PASS}$, $D_{\text{auth}} = \text{AUTHORIZATION_PENDING}$ | `ASSESSED_LIMITATIONS` (Exit `1`). Promotion to `main` remains blocked. |
| **5. Cross-repository CI provenance substitution** | $D_{\text{integrity}} = \text{VALID}$ (syntax only), $D_{\text{ci}} = \text{UNVERIFIED_RUN}$ | `ASSESSED_FAIL` (Exit `2`). Run ID 40144852713 does not belong to ECE repo. |
| **6. Correct workflow run, but wrong attempt** | $D_{\text{ci}} = \text{ATTEMPT_MISMATCH}$ | `ASSESSED_FAIL` (Exit `2`). Rejects superseded or mismatched runs. |
| **7. Verified evidence with open security blockers** | $D_{\text{integrity}} = \text{VALID}$, $D_{\text{review}} = \text{REVIEW_FAILED}$ | `ASSESSED_FAIL` (Exit `2`). |
| **8. Offline operation with unreachable GitHub API** | $D_{\text{integrity}} = \text{VALID}$, $D_{\text{freshness}} = \text{UNKNOWN_HEAD}$, $D_{\text{avail}} = \text{UNREACHABLE}$ | `INSUFFICIENT_EVIDENCE` (Exit `3`). Evaluates static local properties; does not invent PASS or FAIL. (Eliminates phantom `NOT_EVALUATED_OFFLINE` state). |
| **9. Missing expected jobs in CI** | $D_{\text{ci}} = \text{INCOMPLETE_SKIPPED}$ | `ASSESSED_FAIL` (Exit `2`). |
| **10. Valid content hash, but unauthenticated producer** | $D_{\text{integrity}} = \text{VALID}$, $D_{\text{review}} = \text{SELF_REPORT_ONLY}$ | `ASSESSED_LIMITATIONS` (Exit `1`). Hash proves byte integrity; producer claims remain untrusted. |
