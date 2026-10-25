# Backend Specification

## Purpose

The Tauri/Rust backend is a deliberately small local core. It owns durable data, filesystem policy, OS integration, sidecar lifecycle, and scheduling. Business interpretation, manager prompts, and high-level workflow orchestration stay in JavaScript so they are easy to evolve without Rust compile churn.

## Modules

```text
src-tauri/src/
├── main.rs        # Tauri entry point
├── lib.rs         # app/state registration
├── db.rs          # SQLite connection, transactions, repositories
├── schema.rs      # migrations and idempotent seed data
├── commands.rs    # narrow Tauri command handlers
├── files.rs       # report/template roots, canonicalization, open/reveal
├── sidecar.rs     # OpenCode child lifecycle and health checks
├── scheduler.rs   # cron registration and next-run calculation
└── settings.rs    # allowlisted non-secret settings / secure secret adapter
```

## Storage policy

Application data contains:

```text
kubera.db
reports/YYYY-MM/<generated-file>
templates/<imported-template>
logs/sidecar.log
```

The backend, not an agent or renderer, resolves every final report path. It rejects absolute paths, traversal, disallowed extensions, and any canonicalized path outside the app data root.

## Core tables

| Table | Purpose |
|---|---|
| `clients`, `projects` | Durable CRM-lite source records |
| `requirements` | User business goals and final mission state |
| `tasks` | Manager-approved work graph; dependencies and capability requirement |
| `agents` | Named roster, capability set, concurrency limit, availability |
| `agent_runs` | One persisted attempt for each OpenCode session |
| `agent_handoffs` | Validated structured outputs between subagents |
| `deliverables` | Registered Office artifacts in Report Center |
| `schedules` | Prompt, cron, timezone, enablement, last/next run |
| `audit_log` | Source reads, deployment, tool actions, validation, failures |
| `templates` | Imported Office template metadata and placeholder manifest |
| `settings` | Non-secret key/value configuration |

Use bundled SQLite, foreign keys, parameterized queries, and transactions. Timestamps are UTC Unix milliseconds; the UI converts them for display.

## Required command contract

Do not expose raw SQL commands to user/model-controlled input. Use explicit repository commands.

| Command | Purpose |
|---|---|
| `bootstrap_app` | Migrate DB, seed if empty, create directories, start sidecar/schedules |
| `get_engine_status` | Return state/version/model/sanitized error |
| `list_clients`, `list_projects` | Safe source-record views |
| `get_context_for_mission` | Retrieve scoped records and atomically write data-read audit events |
| `create_requirement` | Persist a mission in planning state |
| `create_task` | Persist a validated manager child task |
| `list_agents` | Return approved roster and availability |
| `create_agent_run` | Persist queued session attempt before sidecar call |
| `claim_agent_for_task` | Atomically claim one idle agent and ready task |
| `complete_agent_run` | Complete run, release agent, and write audit event |
| `record_agent_handoff` | Validate/store a source-linked handoff |
| `list_agent_runs`, `list_agent_handoffs` | Power workforce UI |
| `update_task_status` | Enforce status transitions and release dependencies |
| `register_deliverable` | Validate report root and insert output metadata/audit |
| `list_deliverables`, `open_deliverable`, `reveal_deliverable` | Report Center |
| `import_template`, `list_templates` | Approved template lifecycle |
| `create_schedule`, `set_schedule_enabled`, `list_schedules` | Persistent cron schedules |
| `list_audit_events` | Activity and traceability |
| `get_setting`, `set_setting` | Strictly allowlisted settings contract |

## Atomic dispatch algorithm

A safe `claim_agent_for_task` transaction must:

1. Check that the task is `ready` and all dependencies are approved/accepted.
2. Check that the selected named agent is idle and below concurrency limit.
3. Change agent status to `busy`.
4. Set task status to `in_progress` and increment attempts.
5. Set agent-run status to `running` and started time.
6. Write an audit entry.
7. Commit or roll back all changes together.

On every terminal completion/failure/cancellation path, a transaction releases the agent to `idle`, records the agent-run result, and advances/fails the task.

## Sidecar contract

Rust spawns an external bundled binary using the Tauri shell plugin with a fixed argument vector, conceptually:

```rust
sidecar("opencode-cli")
  .args(["serve", "--port", "4096", "--hostname", "127.0.0.1"])
  .spawn()
```

The app polls the pinned OpenCode health endpoint before enabling missions. Process stdout/stderr is captured to an application log. On shutdown, Rust terminates the child process gracefully and forcibly only if necessary.

## Office adapter contract

The OpenCode integration must map typed action objects to an OfficeCLI process argument array. It may expose only expected verbs such as `create`, `add`, `set`, `merge`, `render`, and `dump`.

The adapter must:

- validate requested action against task capability and output type;
- generate a safe output filename/path internally;
- disallow shell interpolation;
- enforce a timeout;
- capture sanitized stdout/stderr;
- verify existence and non-zero size before registering deliverable;
- optionally use `dump`/`render` for stronger validation.

## Scheduler contract

Rust loads enabled schedules at startup, registers cron jobs, and emits `schedule-fired` with a schedule ID. JS then creates an ordinary requirement through the same mission path. Store timezone with every schedule; label a schedule as UTC if the chosen cron implementation cannot honor its local zone.

## Implementation order

1. Add `Cargo.toml`, `tauri.conf.json`, capabilities, and state initialization.
2. Implement migrations/seed data and read-only repositories.
3. Implement report/template root canonicalization.
4. Implement command tests for data, task/runs, handoffs, and deliverables.
5. Implement sidecar health/spawn/shutdown.
6. Connect JS orchestration to repository calls.
7. Add cron events after the ad-hoc mission loop is reliable.
