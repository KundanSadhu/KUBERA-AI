# How KUBERA Works

## Product loop

```text
Business goal → Manager plan → Specialist work → Structured handoffs
             → Document worker → File validation → Report Center
             → Optional recurring schedule
```

The user submits a result-oriented business requirement. KUBERA does not make agent prose the primary result. It creates a mission, coordinates a bounded team, and finishes with a locally stored Office artifact.

## Ad-hoc mission workflow

### 1. Capture a requirement

The user enters a goal such as:

> Build a Q3 pipeline deck for Acme Corp.

KUBERA creates a durable `requirements` row in `planning` state. The UI shows a mission card immediately; it does not claim the document is complete.

### 2. Manager plans the workforce

Asha — Manager receives only the requirement, output constraints, and approved role catalog. Asha returns a JSON plan. For a board deck, the default plan is:

| Agent | Task | Output |
|---|---|---|
| Maya — Market Analyst | Identify evidence-backed opportunity and risk themes | `market_analysis` handoff |
| W-1 — Operations Analyst | Calculate pipeline totals, stage mix, and risk count | `pipeline_metrics` handoff |
| W-2 — Document Worker | Create the board deck | PPTX |
| Sentinel — Reviewer | Validate the PPTX | approval/failure |

The manager never opens a session for another agent itself. KUBERA validates the plan and creates the child tasks.

### 3. Select business context

The orchestrator turns a validated selector such as `client_name: Acme Corp` into safe repository calls. It retrieves only necessary client and project fields and logs the IDs and purpose in `audit_log`.

### 4. Deploy ready specialists

Maya and W-1 have no dependency on each other, so they may start in parallel. The scheduler sets an agent `busy` only through an atomic claim. The MVP uses a two-session global concurrency limit and one active run per named agent.

### 5. Peer-review and validate handoffs

A specialist returns a schema-limited object rather than raw chat history. Each material claim or metric is tied to source record IDs. The output moves to **Peer review**, not immediately to approved:

- W-1 reviews Maya’s `market_analysis` structure and cited records.
- Maya reviews W-1’s `pipeline_metrics` type, numeric total, and cited records.
- Only accepted handoffs release W-2’s blocked document task.

Invalid handoffs are rejected and block downstream work. The Workforce Team Board makes queued, working, peer-review, delivered, and approved states visible.

### 6. Build the file

W-2 receives accepted handoffs, trusted source data, a document contract, and the restricted Office tool. W-2 can create only the assigned Office file under the KUBERA report directory.

### 7. Review and publish

Sentinel/system validation checks that the artifact is inside the report directory, has the expected type, is non-empty, and passes a supported OfficeCLI parse/render check. On success, KUBERA stores a `deliverables` row and marks the mission done. The Report Center exposes **Open** and **Reveal** actions.

## State machine

```text
requirement: planning → running → done | failed

task: blocked → ready → in_progress → review → approved
                                └──────────────→ failed

agent run: queued → running → succeeded | failed | cancelled
```

- `blocked`: waits for declared upstream handoffs.
- `ready`: has all accepted dependencies and can be claimed.
- `in_progress`: an assigned named agent owns the task.
- `review`: a final Office artifact exists and is being checked.
- `approved`: validation passed.

## Handoff contract

A handoff contains data, evidence, and a known type—not executable instructions.

```json
{
  "handoff_type": "pipeline_metrics",
  "total_pipeline_value": 365000,
  "project_count": 2,
  "at_risk_count": 1,
  "stage_breakdown": [
    {"stage": "Proposal", "count": 1, "value": 240000},
    {"stage": "Negotiation", "count": 1, "value": 125000}
  ],
  "evidence_record_ids": ["seed-project-acme-expansion", "seed-project-acme-analytics"]
}
```

The downstream document worker sees only accepted, relevant handoffs. It never treats handoff text as a new tool instruction.

## Scheduled missions

A schedule saves requirement text, output type, cron expression, timezone, enabled flag, last run, and next run.

1. The desktop renderer subscribes to `schedule-fired`, then starts one Rust scheduler worker for the open app session.
2. Rust parses the schedule against its IANA timezone, atomically advances `last_run`/`next_run`, appends an audit event, and emits `schedule-fired`.
3. JS queues the event behind any already-running mission, then creates a normal requirement from the stored text and selected deliverable type.
4. The mission repeats against the current local records using the same manager and peer-review gates as an ad-hoc mission.
5. No cron callback invokes an LLM, OfficeCLI process, shell, or arbitrary filesystem path.

**MVP limit:** the desktop app must remain open. KUBERA does not claim background execution after exit, and missed offline occurrences are not replayed as an unbounded backlog.

## What users see

- **Missions:** input bar, task status, schedule rows, and completion banners.
- **Workforce:** manager plan, named agent runs, dependency state, accepted handoff badges, retry/cancel where supported.
- **Reports:** the final PPTX/DOCX/XLSX with Open and Reveal actions.
- **Records:** local client and project source tables. In desktop mode, the existing client/project forms call typed SQLite create commands with validation, foreign-key checks, and audit events.
- **Activity:** compact audit and engine events; never a wall of model chat.

## Definition of a completed mission

A mission is complete only when all required tasks are approved, every agent is released to idle, the document exists under the report root, a deliverable record exists, and the report is visible after application restart.
