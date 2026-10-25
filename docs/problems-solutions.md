# Problems and Solutions

This register documents the known delivery, reliability, and product risks for KUBERA AI. A risk belongs here only when it has an explicit prevention, detection, and recovery path.

| Problem / risk | Why it matters | Prevention | Detection and recovery |
|---|---|---|---|
| OfficeCLI does not run on target OS | No valid Office deliverables; core value fails | Prove PPTX/DOCX/XLSX operations during first 45 minutes; pin binary/version | Stop UI work, inspect binary dependencies/arguments, repair native path before continuing |
| OpenCode API/plugin contract changes | Session, SSE, or restricted tool bridge may break | Pin a known version; isolate integration in one adapter | Health/session contract test at startup; show sidecar diagnostics |
| Manager proposes an unsafe task graph | Could create role confusion or unbounded work | JSON schema, allowlisted roles/capabilities, max 4 tasks, depth 1, cycle check | Reject plan, retry once with validation feedback, use deterministic workforce fallback |
| Recursive agent spawning | Cost, instability, loss of auditability | Only JS orchestration can create child tasks/sessions | Reject any manager/specialist spawn request; log `deployment_rejected` |
| Two agents work on same task | Duplicate output or inconsistent status | Atomic agent/task claim transaction and one active run per agent | Recover stale run at startup; mark duplicate claims impossible at repository layer |
| Analyst invents a claim | Misleading board deck/report | Require typed handoffs and source record IDs for material claims | Reject handoff or flag missing evidence; block downstream task |
| Handoff becomes prompt injection | Downstream worker may follow untrusted text | Delimit handoffs as data; validate schema; remove tool/path fields | Reject invalid fields and log sanitized failure |
| Worker writes outside report root | Local data integrity/security issue | Backend generates path, canonicalizes, and uses fixed action allowlist | Reject path and fail task; no deliverable row is created |
| Model invokes generic shell | Arbitrary command risk | Do not expose shell tool; use typed Office adapter with argument array execution | Reject unsupported action; audit tool-policy failure |
| Planner/manager JSON malformed | Mission cannot be dispatched | Provider JSON mode, local parser/schema, one retry | Use deterministic single-worker or analyst→document fallback |
| Provider latency feels like freeze | User loses confidence | Optimistic status, task-level spinner, SSE status summaries | Timeout with retry button and preserved diagnostic run record |
| Sidecar is not healthy | No agent sessions | Loopback health poll before enabling submit | UI blocks mission start and shows start/log diagnostics |
| API key leaks into UI or logs | Credential compromise | Mask UI, secret storage, log scrubber, no key in prompt context | Rotate key, purge local logs, verify settings command contract |
| SQLite write is interrupted | Broken mission state/audit | Transactions for related task/run/handoff changes; foreign keys on | Reopen DB, detect unfinished runs, return agents to idle safely |
| Crash leaves agent marked busy | Future work cannot dispatch | Startup reconciliation identifies non-terminal runs | Mark abandoned run failed/cancelled with recovery audit, release agent |
| Cron timezone mismatch | Mission fires at wrong business time | Store timezone per schedule; verify scheduler capability | Label UTC if unavoidable; calculate/display next run explicitly |
| App is closed at scheduled time | User expects unattended job | State MVP boundary clearly in UI/docs | Show missed run/last-run state; Phase 2 background service |
| DOCX placeholders remain unresolved | Client report is incomplete | Template placeholder manifest and mapping preflight | Mark task failed; show missing keys; preserve source template |
| Final file exists but is corrupt | Report Center shows unusable file | Use OfficeCLI supported commands; validate extension/size/dump-render | Do not approve; preserve tool diagnostics for retry |
| Scope creep | Five-hour slice fails to ship | Enforce local records + workforce + Office files + schedule boundary | Move nonessential capabilities to roadmap |

## Incident response policy

1. **Contain:** stop dispatching affected tasks; do not retry blindly.
2. **Preserve evidence:** retain sanitized run summaries, OfficeCLI exit data, and audit events.
3. **Protect data:** never delete a generated file or template as an automated recovery step.
4. **Recover deterministically:** use the defined retry/fallback path, not a new improvised agent/tool.
5. **Inform the user:** show a concise error with a next action such as Retry, Open diagnostics, or Update settings.

## Hard stop conditions

Stop the build and fix the blocker before adding UI polish if any of the following is true:

- A sample PPTX opens with an Office repair prompt.
- Output paths cannot be reliably constrained.
- The OpenCode sidecar cannot be health-checked or shut down cleanly.
- A manager can create agents without orchestration-layer validation.
- Audit events cannot establish which business records supplied a final file.
