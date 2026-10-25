# Feature List and Scope

## MVP: required vertical slice

### Mission and workforce

- [x] Requirement input for plain-language business goals.
- [x] Manager-led, bounded subagent plan in the implementation prototype.
- [x] Named Market Analyst, W-1 Operations Analyst, W-2 Document Worker, and Reviewer states.
- [x] Visual dependency/handoff timeline.
- [x] Activity/audit-style feed showing record selection, deployment, handoffs, and final validation.
- [x] Source-implemented OpenCode sessions and SSE routing (loopback-only; host free-model completion remains an acceptance gate).
- [x] Source-implemented SQLite persistence and restart reconciliation (desktop host validation remains required).

### Business records

- [x] Clients table: name, industry, status, owner.
- [x] Projects table: client, stage, value, at-risk flag.
- [x] Sortable/readable local UI views.
- [x] Browser-prototype client/project creation forms with local persistence.
- [x] Prototype template-library metadata import.
- [x] Rust SQLite schema, seed, and repository-command source implementation.
- [x] Source-implemented desktop SQLite client/project creation with typed validation, client foreign-key checks, and audit events (native host proof remains required).
- [x] Desktop-approved Office template copy and DOCX placeholder inspection. The renderer submits bounded file bytes from a user-selected file; Rust validates the OOXML package, writes only under the template root, records it in SQLite, and audits the import.

### Office deliverables

- [x] Report Center with PPTX/DOCX/XLSX type badges and artifact metadata.
- [x] Simulated document-result registration for UI workflow development.
- [x] Restricted typed OfficeCLI adapter for PPTX, plain DOCX, and tabular XLSX requests; renderer/agents never receive a general command or output path.
- [ ] Valid PPTX board deck generated from real local records (Windows OfficeCLI acceptance gate).
- [x] Source-implemented DOCX template merge with exact approved `{{placeholder}}` set validation, XML escaping, template/report-root confinement, and OfficeCLI package validation (host OfficeCLI acceptance proof remains required).
- [x] Bounded XLSX table generation from local project rows (formula/chart authoring remains a later stable-syntax slice).
- [x] In-app bounded text-outline preview for registered Office artifacts through a typed OfficeCLI `view … outline` path (full visual rendering remains a later slice).

### Scheduling

- [x] Schedule list, enabled state, cron/timezone/next-run UI.
- [x] Browser prototype persistence of schedule rows.
- [x] Rust in-process cron scheduling while the Tauri app is open, with IANA timezone parsing, durable next-run advancement, and audit records.
- [x] Schedule-fired event is queued by the desktop renderer and starts the normal bounded manager mission (never a second simultaneous manager).

### UX and quality

- [x] Three-pane business workspace.
- [x] Engine state display and Settings screen.
- [x] Empty states, completion banner, toasts, and reset-demo flow.
- [x] Responsive desktop-focused layout with accessible status labels.
- [x] Native file Open/Reveal actions for SQLite-registered report IDs only; no renderer-provided path or command is accepted.
- [ ] Tauri package build and installer (Windows host/CI acceptance gate).

## Acceptance criteria

A release demo passes when:

1. A board-deck request produces an openable PPTX without a repair prompt.
2. A client-report request merges a local-data map into a DOCX template.
3. A persisted schedule shows a correct next run and can trigger while the app is open.
4. Restart preserves records, reports, missions, agent runs, handoffs, and schedules.
5. Audit history proves which records and handoffs contributed to the final file.
6. Workforce view proves that the manager deployed named subagents, W-2 created the artifact, and recursive spawning was impossible.

## Explicitly out of scope for MVP

- General browser, terminal, or code-edit tools for agents.
- External market research; Market Analyst uses local business context only.
- Email, Slack, Teams, or external delivery.
- Background execution after the window closes.
- Multi-user authentication and collaboration.
- Native multi-provider support beyond an OpenAI-compatible endpoint.
- Arbitrary MCP/server import.
- Marketplace/skills bundles.
- Full CRM/ERP scope.

## Phase 2

1. [x] Local human file approval/rejection queue for registered deliverables, with pending/approved/rejected state, required rejection feedback, and durable audit events (external delivery remains separate and disabled).
2. [~] Local client/project editing plus bounded all-or-nothing project CSV import are implemented with stable source IDs, client foreign keys, and audit events; richer source relationships remain pending.
3. [~] Open-app desktop notification flow is implemented for approved/failed mission review outcomes. The user selected this safer mode; OS task registration/background execution after app exit remains deferred pending Windows host acceptance.
4. [~] Source-implemented visual Office preview: a registered deliverable ID is rendered through typed `OfficeCLI view … html -o …`, bounded to 2 MiB, deleted after read, and displayed only in a scriptless/same-originless iframe. Host OfficeCLI render acceptance remains required.
5. [~] Source-implemented SMTP attachment and Slack Bot file-upload delivery after an explicit, current human approval. Credentials stay only in host environment variables; typed recipients, 8 MiB attachment bound, delivery records, and audit events are enforced. Target-host credential/provider acceptance remains required.
6. [~] Citation-only Crossref connector is source-implemented: a human-triggered fixed HTTPS endpoint with strict query/response bounds, redirect denial, local citation persistence, and audit events. Agents receive no browser or connector capability; host network acceptance remains required.
7. [~] Source-implemented fixed pool: two Market Analysts, two Operations Analysts, and two Document Workers. The manager stays globally serial; once planning completes, concurrent specialist missions use deterministic least-load assignment while preserving fixed roles, capability limits, and typed W-2/W-2B Office permissions. Native multi-mission host acceptance remains required.
8. [~] Approved template bundles are implemented: one Office template plus declarative placeholder metadata can be exported/downloaded and re-imported after ZIP, OOXML, metadata, and path validation. Executable skill/prompt/connector bundles remain deliberately disabled because they would violate the bounded-agent safety model.
9. [~] Self-hosted/local-account/SQLite-server pilot architecture is documented in `docs/self-hosted-sync.md`. Remote sync is intentionally not enabled until TLS deployment, password/session implementation, workspace authorization, conflict handling, backups, and independent authentication review are completed.
