# Security and Privacy Model

## Security posture

KUBERA is local-first, but it handles business data, Office files, provider credentials, and model output. The product uses least privilege: agents see only scoped record context and the one constrained capability required for their task.

## Primary controls

| Area | Required control |
|---|---|
| OpenCode service | Bind to `127.0.0.1`; do not expose on LAN/WAN |
| Renderer | Narrow Tauri CSP; no broad remote script/connect sources |
| Database | Parameterized repository queries; no raw model-controlled SQL |
| Subagents | Manager plan allowlist; depth-one deployment; capability-bound roles |
| Handoffs | Typed schema, size limits, source record references, data delimiters |
| Office actions | Typed allowlist mapped to OfficeCLI argument arrays; never shell interpolation |
| Filesystem | Backend-generated output paths; canonical report/template roots only |
| Secrets | Masked UI, never in prompts/audits/logs; OS secure store/Stronghold in production |
| Rendering | Escape dynamic text; do not insert model output as HTML |
| Audit | Immutable append-style record reads, deployment, handoff, file, and validation events |

## Threats and required behavior

### Prompt injection in local records or handoffs

Client/project data can contain instruction-like text. KUBERA encloses source data and handoffs in a clearly marked trusted-data envelope and tells workers to treat it as data only. Handoff schemas reject fields that look like tools, paths, SQL, credentials, or executable instructions.

### Uncontrolled multi-agent behavior

The manager returns a plan but has no spawn API. JS validates the plan before creating child tasks and rejects unknown roles, cycles, deep nesting, or more than four children. Specialists have no deployment capability.

### Filesystem escape

Agents never submit final absolute paths. The backend derives a safe filename and canonical report-root path, rejects `..`, and verifies the final location before registering a deliverable.

### Generic execution

No agent gets a shell. The document worker gets only a typed KUBERA Office tool whose backend maps expected actions to OfficeCLI process arguments.

### Credential leakage

API keys never enter agent prompts, audit details, activity UI, or source-controlled configuration. Production uses secure OS-backed storage or Stronghold. Sidecar logs are scrubbed before display.

## Data minimization

- Retrieve only the records needed for a task.
- Send only required source fields to each subagent.
- Persist concise sanitized run summaries, not raw model transcripts by default.
- Retain audit evidence sufficient to prove provenance.
- Keep generated reports inside the user’s local app-data directory unless the user explicitly exports or delivers them in a future feature.

## Security acceptance checks

- Unknown role/capability in a manager plan is rejected.
- A specialist cannot request an Office action not in its capability set.
- Any output outside reports/templates is rejected.
- A record value cannot cause a downstream tool call.
- Audit entries include no provider secrets.
- Sidecar has no non-loopback listener.
