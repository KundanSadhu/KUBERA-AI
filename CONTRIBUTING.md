# Contributing to KUBERA AI

Thank you for contributing. KUBERA prioritizes reliable, auditable business artifacts over broad agent capability.

## Before opening a change

1. Read [`docs/architecture.md`](docs/architecture.md), [`docs/agents.md`](docs/agents.md), and [`docs/security.md`](docs/security.md).
2. Keep the product boundary: local records → bounded workforce → constrained Office file → Report Center.
3. Do not add a generic shell, arbitrary filesystem, raw SQL, or recursive agent-spawn tool.
4. Update documentation and tests with every product-contract change.

## Frontend development

```bash
npm install
npm run dev
npm run check
```

Use ES modules and vanilla DOM rendering. Escape dynamic text. Persisted state is authoritative; browser state may only be transient UI state.

## Adding an agent role or worker instance

A new agent must define:

- stable ID and display name;
- agent kind (`manager`, `analyst`, `worker`, or `reviewer`);
- narrow capability list;
- maximum concurrency;
- explicit allowed inputs;
- JSON handoff/output schema;
- system prompt;
- validator rules;
- tests for success, malformed output, and prohibited capability requests.

Only application code deploys workers. No role may create a child agent/session directly.

## Adding a handoff type

1. Define a versioned JSON schema.
2. Include source record references for material claims.
3. Enforce size and field allowlists.
4. Validate before persistence and before downstream consumption.
5. Add audit events for create/consume/reject.

## Adding a restricted Office action

1. Define a typed request shape.
2. Map it to an OfficeCLI argument array in backend code.
3. Add an output type/path validation rule.
4. Add timeout and sanitized diagnostics.
5. Test file validity in a native Office-compatible application.

Do not run a model-generated command string through a shell.

## Database changes

- Use forward-only idempotent migrations.
- Preserve existing mission/report/audit data.
- Use foreign keys, parameterized queries, and transactional lifecycle updates.
- Update `docs/backend.md` when tables/commands change.

## Pull request checklist

- [ ] `npm run check` passes.
- [ ] New dynamic UI text is escaped.
- [ ] No secret is added to source/log/audit output.
- [ ] New capability is least-privilege and documented.
- [ ] Docs and acceptance tests are updated.
- [ ] Changes do not weaken report-root or sidecar loopback policy.
