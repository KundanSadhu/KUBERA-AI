# Markus-Inspired Workforce Improvements

KUBERA borrows the useful operating ideas of a visible AI workforce while preserving its own business-document focus and local-first safety boundary.

## What KUBERA adopts

| Workforce idea | KUBERA adaptation |
|---|---|
| Requirement becomes a team | Asha turns every business goal into a bounded, persisted workforce plan |
| Specialist tasks run in parallel | Maya and W-1 perform independent analysis concurrently when dependencies allow |
| Every worker has a workspace | Each task records a logical isolated workspace path and receives only scoped context/handoffs |
| Peer review gates work | Analysts cross-review each other’s structured handoffs before W-2 can create a document |
| Visible task board | Workforce view adds Queued, Working, Peer review, Delivered, and Approved columns |
| Progress is inspectable | Task status, reviewer, workspace, handoff source count, and audit events are durable UI data |
| Deliveries require a gate | Sentinel validates the artifact and provenance; human file acceptance remains the business gate for production |
| Standing watch | Persistent schedules/patrol rows repeat a business mission against current local records |

## What KUBERA deliberately does not copy

- No unrestricted self-spawning hierarchy.
- No generic shell/code-edit/browser agent tool.
- No opaque agent-to-agent chat as a source of truth.
- No cloud-first architecture, external accounts, or a broad agent marketplace in MVP.
- No claim that a draft is a finished Office artifact before OfficeCLI validates it.

## Current local execution path

```text
Requirement
  ↓
Asha — Manager
  ├── Maya — Market Analysis ─────── reviewed by W-1 ─┐
  └── W-1 — Pipeline Metrics ─────── reviewed by Maya ├─ accepted handoffs
                                                       ↓
                                      W-2 — Document Worker
                                                       ↓
                                      Sentinel — File Reviewer
                                                       ↓
                                      Report Center
```

The local deterministic runtime executes this structure against live browser-record data today. The desktop OpenCode runtime will use the same task graph, handoff schema, peer-review gate, and audit model once its sidecar and OfficeCLI adapter are installed.

## Peer-review policy

1. An analyst completes a typed handoff and moves to `review`, not immediately to approved.
2. The designated peer verifies required fields and source record references.
3. KUBERA marks the handoff `accepted` only after peer review succeeds.
4. The document worker remains blocked until all required handoffs are accepted.
5. Sentinel checks the final artifact’s file/type/provenance policy.
6. In production, the user still accepts or rejects the business document.

## Product result

This produces the useful feeling of a company at work—specialists, dependencies, peer checks, delivery board, and patrol—without sacrificing KUBERA’s core rule: every agent is constrained, every source is traceable, and the deliverable is a business file.
