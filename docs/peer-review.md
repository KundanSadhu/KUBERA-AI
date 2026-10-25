# Peer Review and Delivery Gates

## Why peer review exists

A worker saying “done” is not a sufficient quality signal. KUBERA adds a narrow, evidence-based peer gate before a document worker can use analyst output.

## Analyst cross-review

| Producer | Handoff | Peer reviewer | Checks |
|---|---|---|---|
| Maya — Market Analyst | `market_analysis` | W-1 — Operations Analyst | Handoff type, findings array, cited record IDs |
| W-1 — Operations Analyst | `pipeline_metrics` | Maya — Market Analyst | Handoff type, numeric pipeline total, cited record IDs |

An invalid handoff is rejected, downstream work stays blocked, and the audit log records the failure.

## File review

Sentinel is not a business strategist. Sentinel validates:

- expected extension and report registration;
- non-empty output file in desktop mode;
- supported OfficeCLI parse/render result where available;
- source-record provenance attached to the artifact.

## Human approval

Automated gates confirm structural correctness. A business user remains the final approver before using, sharing, or delivering a generated file externally.
