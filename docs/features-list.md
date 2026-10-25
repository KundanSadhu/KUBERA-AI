# Feature List and Scope

## MVP: required vertical slice

### Mission and workforce

- [x] Requirement input for plain-language business goals.
- [x] Manager-led, bounded subagent plan in the implementation prototype.
- [x] Named Market Analyst, W-1 Operations Analyst, W-2 Document Worker, and Reviewer states.
- [x] Visual dependency/handoff timeline.
- [x] Activity/audit-style feed showing record selection, deployment, handoffs, and final validation.
- [ ] Production OpenCode sessions and SSE routing.
- [ ] Production SQLite persistence and restart reconciliation.

### Business records

- [x] Clients table: name, industry, status, owner.
- [x] Projects table: client, stage, value, at-risk flag.
- [x] Sortable/readable local UI views.
- [x] Browser-prototype client/project creation forms with local persistence.
- [x] Prototype template-library metadata import.
- [x] Rust SQLite schema, seed, and repository-command source implementation.
- [ ] Desktop-validated SQLite CRUD repositories (requires Rust host compilation).
- [ ] Desktop template-file copy/placeholder inspection.

### Office deliverables

- [x] Report Center with PPTX/DOCX/XLSX type badges and artifact metadata.
- [x] Simulated document-result registration for UI workflow development.
- [ ] Restricted OfficeCLI adapter.
- [ ] Valid PPTX board deck generated from real local records.
- [ ] DOCX template merge with `{{placeholder}}` validation.
- [ ] XLSX with formulas/chart where stable.
- [ ] In-app file render/preview.

### Scheduling

- [x] Schedule list, enabled state, cron/timezone/next-run UI.
- [x] Browser prototype persistence of schedule rows.
- [ ] Rust cron scheduling while the Tauri app is open.
- [ ] Schedule-fired event creates a real agent mission.

### UX and quality

- [x] Three-pane business workspace.
- [x] Engine state display and Settings screen.
- [x] Empty states, completion banner, toasts, and reset-demo flow.
- [x] Responsive desktop-focused layout with accessible status labels.
- [ ] Native file Open/Reveal actions.
- [ ] Tauri package build and installer.

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

1. Human file approval/rejection queue.
2. Real record editing/imports and richer source relationships.
3. OS background scheduling/notifications.
4. In-app preview using OfficeCLI render output.
5. Email/Slack delivery after explicit approval.
6. Approved external-research connector with citations.
7. Multiple worker instances and queue balancing.
8. Template/skill import-export bundles.
9. Secure multi-user sync and remote storage.
