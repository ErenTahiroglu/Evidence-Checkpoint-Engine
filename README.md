# Evidence Checkpoint Engine

> **Status: BOOTSTRAP / ARCHITECTURE ONLY — NOT A WORKING VERIFIER.**
> No checkpoint has been independently validated by this repository. No production authorization is implied.

Independent, evidence-bound verification for GitHub engineering checkpoints. The planned tool is reusable across repositories and AI agents, including (later) [AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager).

## Goals

- Verify claims against **exact repository identity, commit SHA, workflow/run/attempt, expected jobs, artifact evidence, and reviewer scope**.
- Separate **source claims**, **machine-verifiable facts**, **independent review**, and **human authorization**.
- Record ambiguous, missing, expired, conflicted, or unsupported evidence without fabricating PASS.
- Offer a standalone library + CLI and optional read-only GitHub adapter; GUI/server/daemon not required.
- Make historical evidence retrievable without mistaking GitHub Issues or transient CI artifacts for immutable proof.

## Scope / Non-Goals

No executable verification implementation is included in this bootstrap. Do not run untrusted code, mutate other repositories or Issues, install agents, or modify credentials/configuration. GitHub Issue bodies and artifacts are untrusted inputs.

The implementation language and runtime remain open architectural decisions to be evaluated in GOV-00.

## Branches

- `main`: last **independently reviewed and explicitly authorized** baseline after first real acceptance. Initial documentation-only bootstrap does **not** constitute a working release.
- `development`: all normal engineering changes; promotion to `main` requires exact-SHA tests, independent review, and owner authorization. No automatic promotion.
- Both branches initially point to the same bootstrap commit; initial `main` is a baseline placeholder, not a validated verifier.

See [branch governance](docs/BRANCH_GOVERNANCE.md) and [setup guide](docs/SETUP_TR.md).

## Documentation

- [Architecture and trust boundaries](docs/ARCHITECTURE.md)
- [Branch governance policy](docs/BRANCH_GOVERNANCE.md)
- [Red Team risk register / threat model](docs/THREAT_MODEL.md)
- [Phase roadmap](docs/ROADMAP.md)
- [Decisions and open questions](docs/DECISIONS.md)
- [AI-Toolchain-Manager integration contract (draft)](docs/INTEGRATION_CONTRACT.md)
- [GitHub Issue migration plan](docs/ISSUE_PLAN.md)
- [Initial sources and references](docs/REFERENCES.md)
- [Setup guide (TR)](docs/SETUP_TR.md)
- [Licensing details](docs/LICENSING.md)

## Relationship to AI-Toolchain-Manager

The standalone core will be developed in this repository. The manager's planned read-only evidence viewer and verified dashboard are tracked separately in [#33](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/33) and [#34](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/34). The potential issue publisher [#35](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/35) is **design-only**, not permission to write.

## Licensing

This project is licensed under the **PolyForm Noncommercial License 1.0.0**.

Required Notice: `Copyright (c) 2026 Eren Tahiroğlu`

The repository is public and source-available for permitted noncommercial purposes. Commercial use is not authorized under the public license without separate explicit agreement. See [LICENSE.md](LICENSE.md) and [docs/LICENSING.md](docs/LICENSING.md) for full details.
