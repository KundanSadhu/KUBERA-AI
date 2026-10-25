# KUBERA AI Architecture

## Purpose

KUBERA AI turns structured local business records into auditable Office deliverables. Its architecture separates desktop persistence and process control from high-level business orchestration, and it treats agents as bounded workers—not as an unrestricted shell.

## Design principles

1. **File first:** a PPTX, DOCX, or XLSX is the principal product output.
2. **Local records are memory:** durable SQLite records are retrieved at mission time.
3. **Controlled workforce:** a manager proposes a bounded task graph; application code validates and deploys it.
4. **Least capability:** no agent receives generic shell, SQL, network, or arbitrary filesystem access.
5. **Traceability:** record reads, handoffs, document actions, and validation are audit events.
6. **DB is truth:** the renderer refetches durable data after writes rather than treating browser state as authoritative.

## Component diagram

```text
┌───────────────────────────────────────────────────────────────────────┐
│ Tauri v2 desktop window                                                │
│                                                                       │
│  Vanilla presentation layer                                            │
│  - index.html / app.css / main.js                                      │
│  - Missions, Workforce, Reports, Records, Settings, Activity          │
│                                                                       │
│  JS orchestration layer                                                │
│  - manager-plan validation                                             │
│  - task graph and dependency dispatch                                  │
│  - context selection + audit writes                                    │
│  - structured handoff validation                                       │
│  - OpenCode session/SSE routing                                        │
└───────────────────────────────┬───────────────────────────────────────┘
                                │ Tauri IPC + loopback HTTP/SSE
┌───────────────────────────────▼───────────────────────────────────────┐
│ Rust core (src-tauri)                                                  │
│ - SQLite migrations/repositories                                       │
│ - canonical report/template paths                                      │
│ - settings and native open/reveal                                      │
│ - OpenCode sidecar process lifecycle                                   │
│ - cron scheduler                                                        │
└───────────────┬──────────────────────────────────────────┬────────────┘
                │ loopback only                             │ application data
┌───────────────▼─────────────────────┐      ┌──────────────▼───────────┐
│ OpenCode sidecar                     │      │ SQLite + files             │
│ - manager / agent sessions           │      │ - kubera.db                │
│ - global SSE stream                  │      │ - reports/YYYY-MM/         │
│ - restricted KUBERA Office tool      │      │ - templates/               │
└───────────────┬─────────────────────┘      └──────────────────────────┘
                │ typed allowlisted actions
┌───────────────▼─────────────────────┐
│ OfficeCLI                            │
│ create · add · set · merge · dump    │
│ render                               │
└─────────────────────────────────────┘
```

## Responsibility boundaries

| Layer | Owns | Must not own |
|---|---|---|
| Vanilla UI | Rendering, user input, navigation, transient spinners | Raw SQL, secrets, arbitrary paths, agent policy |
| JS orchestration | Prompt construction, manager-plan validation, agent dispatch, handoff lifecycle | Direct OS process control or unbounded agent spawning |
| Rust core | SQLite, filesystem checks, scheduler, sidecar lifecycle, native file actions | Business interpretation and prompt design |
| OpenCode | One model session and streamed events | Database access, unrestricted shell/filesystem access |
| Office adapter | Typed OfficeCLI action mapping and output checks | Generic command execution |
| OfficeCLI | Valid Office-file creation/rendering/merge | Business-record lookup and authorization |

## Managed subagent topology

```text
Requirement
  ↓
Asha — Manager (plan only)
  ├─ Maya — Market Analyst ───────┐
  └─ W-1 — Operations Analyst ────┼─ accepted structured handoffs
                                  ↓
                         W-2 — Document Worker
                                  ↓
                         Sentinel — Reviewer
                                  ↓
                         Report Center artifact
```

The manager can propose at most four child tasks. Only application code can create them. Specialists cannot deploy child agents. The initial global worker concurrency budget is two sessions, and each named agent has a single active run.

## Mission data flow

1. The user creates a `requirement`.
2. Asha returns a JSON-only manager plan with a record selector, roles, dependencies, and required handoff types.
3. JS validates the plan against the local role/capability allowlist.
4. Rust repositories retrieve the scoped client/project context and log the reads.
5. Child tasks and `agent_runs` rows are persisted.
6. Independent analyst tasks receive minimal trusted context and return schema-checked handoffs.
7. W-2 receives accepted handoffs plus required source facts and can use only the Office adapter.
8. Sentinel verifies output existence, extension, size, and optional OfficeCLI dump/render result.
9. The deliverable, audit event, task completion, and Report Center row are committed transactionally.

## Persistence layout

```text
<KUBERA app data>/
├── kubera.db
├── reports/YYYY-MM/<timestamp>_<safe-name>.pptx
├── templates/<template-id>.docx
├── config.json                 # non-secret defaults only
└── logs/sidecar.log
```

Only `reports/` and `templates/` are writable through the application’s file surface. Inputs are canonicalized; arbitrary absolute paths and path traversal are rejected.

## Sidecar lifecycle

1. Rust opens/migrates SQLite and resolves the app-data directory.
2. It spawns the bundled `opencode-cli` at `127.0.0.1:4096`.
3. It polls `/global/health` for up to 30 seconds.
4. The UI remains in `Starting engine…` state until healthy.
5. The JS layer creates sessions and consumes SSE events filtered by session ID.
6. Rust terminates the child gracefully on application exit.

The deployed app must bind OpenCode to loopback only and limit its Tauri CSP to required local HTTP/SSE endpoints.

## Production/prototype boundary

The first UI implementation can simulate engine activity in a browser so that layout and workflow can be developed quickly. That simulation is not the production trust boundary. Production replaces it with the Rust repository commands, real sidecar health checks, OpenCode sessions, SQLite, and the restricted OfficeCLI adapter described above.

## Architectural acceptance checks

- A document producer never gets a generic shell command.
- An analyst cannot write a report file.
- A manager cannot recursively create workers.
- Every material handoff cites source record IDs.
- Report paths are generated by the backend, not by a model.
- Restart preserves requirements, tasks, runs, handoffs, schedules, and deliverables.
