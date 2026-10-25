# Trust Architecture and Multi-Dimensional State Model

> **Milestone:** `GOV-00`
> **Status:** `PROPOSED ARCHITECTURE / DRAFT FOR REVIEW`
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
| Decided by: Designated Repository Owner (Eren Tahiroğlu)                 |
| Authority: Sole authority to accept, baseline, or promote to main.      |
| Examples: Formal promotion decision recorded on an immutable SHA.       |
+-------------------------------------------------------------------------+
```

### Invariant Rules:
1. **No Self-Elevation:** An entity at Tier 1 can never assign a Tier 2, Tier 3, or Tier 4 status.
2. **No Automatic Promotion:** Machine assessment (Tier 2) and independent review (Tier 3) are necessary prerequisites, but they **never automatically confer Human Authorization (Tier 4)**.
3. **No GUI Authority:** External consumer interfaces (such as AI-Toolchain-Manager) have zero authority to mint, upgrade, or bypass any tier.

---

## 3. The Six Independent Verification Dimensions

Rather than a single binary status, an evaluation outcome is represented as a tuple of six independent dimensions:

$$\text{Checkpoint Assessment} = \langle D_{\text{integrity}}, D_{\text{ci}}, D_{\text{availability}}, D_{\text{review}}, D_{\text{authorization}}, D_{\text{freshness}} \rangle$$

---

### Dimension A: Evidence Integrity ($D_{\text{integrity}}$)
Evaluates whether input data, schemas, canonical bytes, and reference hashes are internally sound and untampered.

- **Allowed States:**
  - `VALID`: Schema matches Draft 2020-12, RFC 8785 canonicalization succeeds, calculated digests match references.
  - `INVALID`: Schema syntax error, invalid regex pattern, or mismatched cryptographic digest.
  - `CORRUPT`: Unparseable bytes, truncated JSON, or decoding failure.
  - `TAMPERED`: Structural collision, duplicate keys detected, or envelope tampering.
  - `NOT_EVALUATED`: Check bypassed or skipped due to upstream parser error.
- **Contradiction Behavior:** If calculated digest does not match declared digest, state is immediately `INVALID`.
- **Transitions:** Evaluated once per assessment snapshot; deterministic.

---

### Dimension B: CI Execution ($D_{\text{ci}}$)
Evaluates what actually executed in automated CI environments versus independently approved policy expectations.

- **Allowed States:**
  - `VERIFIED_PASS`: Workflow definition, Run ID, and exact Run Attempt match; 100% of expected checks executed with `conclusion: success`.
  - `FAILED`: One or more observed checks failed or timed out.
  - `INCOMPLETE_SKIPPED`: Any check in the expected list was skipped, cancelled, neutral, or blocked.
  - `ATTEMPT_MISMATCH`: The submitted evidence references an older or mismatched workflow run attempt.
  - `UNVERIFIED_RUN`: Workflow run cannot be correlated with the exact commit SHA or repository ID.
  - `NOT_APPLICABLE`: Phase does not require CI checks (e.g. documentation-only bootstrap).
- **Contradiction Behavior:** A green check conclusion on a skipped test fails closed to `INCOMPLETE_SKIPPED`.

---

### Dimension C: Evidence Availability ($D_{\text{availability}}$)
Evaluates whether supporting evidence (artifacts, test logs, source trees) remains currently accessible and verifiable.

- **Allowed States:**
  - `AVAILABLE`: All referenced artifacts and logs are currently retrievable and their byte digests match.
  - `PARTIALLY_AVAILABLE`: Non-critical secondary logs are missing, but primary evidence is present.
  - `EXPIRED`: GitHub artifact retention window (e.g., 90 days) has elapsed and artifact is deleted.
  - `UNREACHABLE`: Network failure, rate-limiting, or permission denial prevents fetching.
  - `NOT_PROVIDED`: Manifest does not reference external artifacts.
- **Expiration Behavior:** Artifacts inevitably expire. A historical `AVAILABLE` status degrades to `EXPIRED` when rechecked after retention elapses. This does **not** retroactively rewrite the historical fact of past execution, but it alerts consumers that live byte verification is no longer possible without offsite archives.

---

### Dimension D: Review Status ($D_{\text{review}}$)
Evaluates whether an independent human or automated auditor has reviewed the candidate baseline.

- **Allowed States:**
  - `INDEPENDENT_REVIEW_PASS`: An independent technical reviewer has inspected the code/evidence and found no unaddressed blocking risks.
  - `STATIC_PASS_WITH_LIMITATIONS`: Static review passed, but runtime reproduction or independent local build was not executed (e.g. `BOOTSTRAP_00_STATIC_REVIEW_PASS_WITH_LIMITATIONS`).
  - `SELF_REPORT_ONLY`: Only the author or creating agent reviewed their own work.
  - `REVIEW_FAILED`: Independent review discovered critical unmitigated vulnerabilities or rule violations.
  - `UNREVIEWED`: No review record attached.
- **Contradiction Behavior:** An agent report claiming "independent pass" is classified as `SELF_REPORT_ONLY` unless verified by an independent credential/signature.

---

### Dimension E: Authorization ($D_{\text{authorization}}$)
Evaluates whether the repository owner has granted governance approval for the checkpoint.

- **Allowed States:**
  - `EXPLICITLY_AUTHORIZED`: The repository owner has provided written, verifiable authorization for this exact SHA.
  - `AUTHORIZATION_PENDING`: Candidate is undergoing evaluation; no owner decision recorded.
  - `REJECTED`: Owner has explicitly rejected the checkpoint.
  - `REVOKED`: A previously authorized baseline was revoked due to newly discovered security issues.
- **Authority Boundary:** Only the owner (`Eren Tahiroğlu`) can transition this dimension to `EXPLICITLY_AUTHORIZED`.

---

### Dimension F: Freshness ($D_{\text{freshness}}$)
Evaluates whether the checkpoint applies to the current tip of the target branch or is a historical reference.

- **Allowed States:**
  - `CURRENT_HEAD`: The evaluated commit SHA matches the exact current remote branch HEAD.
  - `HISTORICAL_BASELINE`: The evaluated commit SHA was an authorized past baseline, but the branch has advanced.
  - `STALE_DIVERGED`: The commit is on a diverged branch or obsolete fork.
  - `UNKNOWN_HEAD`: Remote HEAD cannot be determined (e.g., during offline verification).
- **Contradiction Behavior:** A historical baseline commit can never claim `CURRENT_HEAD`.

---

## 4. Derived Summary Verdict Formulation

A summary verdict is emitted for convenience, but it is strictly a **conservative deterministic function** of the six underlying dimensions:

$$\text{Verdict} = f(D_{\text{integrity}}, D_{\text{ci}}, D_{\text{availability}}, D_{\text{review}}, D_{\text{authorization}}, D_{\text{freshness}})$$

| Summary Verdict | Conditions Required |
|---|---|
| **`ASSESSED_PROVISIONAL_PASS`** | $D_{\text{integrity}} = \text{VALID} \land D_{\text{ci}} \in \{\text{VERIFIED_PASS}, \text{NOT_APPLICABLE}\} \land D_{\text{availability}} = \text{AVAILABLE} \land D_{\text{review}} = \text{INDEPENDENT_REVIEW_PASS} \land D_{\text{authorization}} = \text{EXPLICITLY_AUTHORIZED} \land D_{\text{freshness}} = \text{CURRENT_HEAD}$ |
| **`ASSESSED_LIMITATIONS`** | Integrity is valid, but one or more non-critical dimensions have scoped limits (e.g., $D_{\text{review}} = \text{STATIC_PASS_WITH_LIMITATIONS}$ or $D_{\text{authorization}} = \text{AUTHORIZATION_PENDING}$). |
| **`INSUFFICIENT_EVIDENCE`** | Missing required evidence, $D_{\text{availability}} \in \{\text{EXPIRED}, \text{UNREACHABLE}\}$, or unverified CI runs. |
| **`ASSESSED_FAIL`** | Any critical dimension is explicitly failed: $D_{\text{integrity}} \in \{\text{INVALID}, \text{CORRUPT}, \text{TAMPERED}\} \lor D_{\text{ci}} = \text{FAILED} \lor D_{\text{review}} = \text{REVIEW_FAILED} \lor D_{\text{authorization}} = \text{REJECTED}$. |
| **`REFUSED`** | Malformed input, schema rejection, hostile payload detected, or unresolvable security conflict. |

**The Golden Invariant:** A summary verdict can **never** be `ASSESSED_PROVISIONAL_PASS` if any single dimension has an unresolved gap, skip, or pending state.

---

## 5. Concrete Edge-Case Scenario Resolutions

| Scenario | Dimensions Resolution | Derived Verdict & System Behavior |
|---|---|---|
| **1. Historically accepted, but no longer current** | $D_{\text{auth}} = \text{AUTHORIZED}$, $D_{\text{freshness}} = \text{HISTORICAL_BASELINE}$ | Output displays `HISTORICAL_BASELINE`. UI shows historical acceptance banner with warning that live HEAD has advanced. |
| **2. Historically accepted, but evidence later expired** | $D_{\text{auth}} = \text{AUTHORIZED}$, $D_{\text{avail}} = \text{EXPIRED}$ | Historical acceptance fact is preserved, but live byte revalidation is flagged as `EXPIRED`. Output directs user to long-term archives. |
| **3. CI green, but no independent review** | $D_{\text{ci}} = \text{VERIFIED_PASS}$, $D_{\text{review}} = \text{UNREVIEWED}$ | Output flags `ASSESSED_LIMITATIONS`. Green CI is explicitly prevented from masquerading as approved software. |
| **4. Independently reviewed, but not owner authorized** | $D_{\text{review}} = \text{INDEPENDENT_REVIEW_PASS}$, $D_{\text{auth}} = \text{AUTHORIZATION_PENDING}$ | Output is `ASSESSED_LIMITATIONS`. Merge/promotion to `main` remains blocked. |
| **5. Correct commit SHA, but wrong repository ID** | $D_{\text{integrity}} = \text{INVALID}$, $D_{\text{ci}} = \text{UNVERIFIED_RUN}$ | Fails immediately with exit code non-zero. Attack class RT-01 rejected. |
| **6. Correct workflow run, but wrong attempt** | $D_{\text{ci}} = \text{ATTEMPT_MISMATCH}$ | Fails immediately. Attack class RT-02 rejected. |
| **7. Verified evidence with open security blockers** | $D_{\text{integrity}} = \text{VALID}$, $D_{\text{review}} = \text{REVIEW_FAILED}$ | Fails closed: `ASSESSED_FAIL`. Cannot be promoted. |
| **8. Offline operation with unreachable GitHub API** | $D_{\text{integrity}} = \text{VALID}$, $D_{\text{freshness}} = \text{UNKNOWN_HEAD}$, $D_{\text{avail}} = \text{UNREACHABLE}$ | Evaluates all static local properties deterministically. Reports external API status as `NOT_EVALUATED_OFFLINE` without fabricating failure or success. |
| **9. Missing expected jobs in CI** | $D_{\text{ci}} = \text{INCOMPLETE_SKIPPED}$ | Rejects candidate. Attack class RT-02 rejected. |
| **10. Valid content hash, but unauthenticated producer** | $D_{\text{integrity}} = \text{VALID}$, $D_{\text{review}} = \text{SELF_REPORT_ONLY}$ | Content hash verifies data integrity only; producer identity remains untrusted claim (`SOURCE_DECLARED`). |
