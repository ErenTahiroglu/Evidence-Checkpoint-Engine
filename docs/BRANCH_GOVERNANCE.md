# Branch Governance Policy

> **Status: AUTHORIZED BASELINE FOR BOOTSTRAP-00.**
> This repository strictly maintains exactly two long-lived branches: `main` and `development`.

## The Two-Branch Model

```
[main]        (Initial Bootstrap Commit) -----------------------------------> [Next Authorized Baseline]
                      \                                                              ^
                       \                                                             | (Explicit promotion:
                        \                                                            |  SHA + Tests + Red Team +
                         v                                                           |  Owner Authorization)
[development] (Initial Bootstrap Commit) ---> [GOV-00] ---> [GOV-01] ... ------------+
```

### 1. `main` Branch
- **Purpose:** Represents the last **independently reviewed and explicitly authorized** production baseline.
- **Restrictions:**
  - Strictly **no ordinary feature development** or direct daily commits.
  - Strictly **no automatic promotion** from CI, scripts, or agent workflows.
  - Strictly **no forced history rewrites** (`git push --force` is prohibited).
  - The initial commit on `main` is explicitly a **BOOTSTRAP BASELINE**, not a validated functional software release.

### 2. `development` Branch
- **Purpose:** The active integration branch for all ordinary implementation work, engineering phases, test suites, Red Team preparation, phase evidence, and documentation changes.
- **Workflow:**
  - Work is tracked via phase issues (GOV-XX / SEC-XX).
  - Commits land on `development` with clear evidence records.
  - The local development working tree normally stays on `development`.

### 3. Bootstrap Initialization Rule
The initial repository is initialized via an authorized one-time sequence:
1. Prepare and validate the documentation-only bootstrap.
2. Create the initial commit on `main` with message: `chore: bootstrap evidence checkpoint engine repository`.
3. Push `main` to origin.
4. Create `development` from that exact commit and push `development` to origin.
5. Verify both remote branches reference the exact same commit SHA.
6. Leave the local working tree on `development`.
7. Do not create phase branches, release branches, or unnecessary tags.

---

## Promotion to `main` Criteria

Promoting changes from `development` to `main` is a formal governance event requiring all of the following:

1. **Explicit Candidate SHA:** A specific, immutable 40-character commit SHA on `development`.
2. **Actual Test Execution Evidence:** Full test suite execution logs, recording exact CI workflow run ID and attempt, with zero unresolved skips or failures.
3. **Independent Red Team Evaluation:** Completed threat-model delta assessment and adversarial testing report from an independent reviewer.
4. **Documented Residual Risks:** All known residual risks, limitations, and security caveats documented with explicit severity ratings.
5. **Explicit Owner Authorization:** Written, affirmative authorization by the repository owner (`Eren Tahiroğlu`).

---

## Branch Protection & Ruleset Recommendations

Branch protection rules and repository rulesets are **not** programmatically modified during the bootstrap phase to prevent unauthorized permission alterations. The following recommended settings are recorded for subsequent user evaluation in GitHub repository settings:

- **Target branch: `main`**
  - Require pull request before merging (no direct pushes).
  - Require status checks to pass before merging.
  - Do not allow force pushes.
  - Do not allow branch deletions.
- **Target branch: `development`**
  - Prevent accidental deletion.
  - Do not allow force pushes.
