# AI-Toolchain-Manager External Integration Contract (Draft)

> **Status: ARCHITECTURAL SPECIFICATION / DRAFT ONLY.**
> No functional API or integration is implemented in this bootstrap phase.

This document defines the strict boundary and interaction rules between the standalone `Evidence-Checkpoint-Engine` and external consumers, specifically [AI-Toolchain-Manager](https://github.com/ErenTahiroglu/AI-Toolchain-Manager) ([Issue #32](https://github.com/ErenTahiroglu/AI-Toolchain-Manager/issues/32)).

## Architectural Separation of Concerns

1. **Independent Verification Authority:**
   - `Evidence-Checkpoint-Engine` owns the verification logic, schemas, and evidence evaluation.
   - External consumers are strictly **read-only consumers** of verifier outputs.
2. **Zero GUI Authority:**
   - External GUIs, dashboards, or desktop applications (e.g. Tauri/WebView) have **zero authority to mint, promote, or override** verification states.
   - An external UI cannot turn an unverified claim (`SOURCE_DECLARED`) into a verified fact (`MACHINE_ASSESSED`).
3. **Transport Mechanism:**
   - Primary transports are offline versioned JSON files or CLI standard output (`stdout` JSON).
   - No local daemon, background service, open socket, or unauthenticated IPC is permitted in MVP.

---

## Consumer Processing Flow

```
[Untrusted Metadata / Artifacts]
              |
              v
[Evidence-Checkpoint-Engine CLI / Core]
              |
              | (Deterministic Evaluation against Contract Schema)
              v
[Versioned Assessment Output (JSON)]
              |
              v
[AI-Toolchain-Manager Read-Only Adapter] (ATM-GOV-02 #34)
              |
              v
[Checkpoint Center View] (ATM-GOV-01 #33)
```

1. **Submission:** Source claims are supplied as untrusted input.
2. **Evaluation:** The standalone engine executes deterministic checks and emits a versioned assessment document.
3. **Consumption:** AI-Toolchain-Manager ingests the assessment document strictly read-only and displays the multi-dimensional status.
4. **Publication (Design Only):** Any future publishing of checkpoints to GitHub Issues (ATM-GOV-03 #35) requires separate explicit human authorization and operates outside the verification core.

---

## Consumer Rejection Rules

External consumers MUST reject or display explicit error warnings for any verifier output that exhibits:

- **Unknown or Unsupported Schema Version:** Engine or schema versions not recognized by the consumer.
- **Identity Mismatches:** Discrepancies between expected repository ID/name and evaluated repository ID/name.
- **Commit SHA Discrepancies:** Evaluation not matching the exact 40-character commit SHA under review.
- **Missing or Expired Evidence:** Referenced artifacts or check runs that cannot be retrieved or have expired.
- **Hostile Payload Indicators:** Input size violations, malicious URLs, or directory traversal attempts.
- **Missing Trust Anchors:** Inability to verify required signatures or integrity hashes.

When rejection occurs, consumers must display an explanatory diagnostic rather than falling back to an unverified or optimistic display.
