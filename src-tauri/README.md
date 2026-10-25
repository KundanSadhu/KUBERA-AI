# Tauri Desktop Core — Scaffold Status

This directory is intentionally a small compile-first shell for the KUBERA desktop host.

## Implemented in source

- Tauri v2 configuration pointing to the static vanilla frontend.
- Loopback-only CSP allowance and external binary declarations for `opencode-cli` and `officecli`.
- Bundled SQLite schema/seed source covering clients, projects, workforce, runs, handoffs, deliverables, schedules, templates, and audit events.
- Narrow Tauri commands for records, requirements, context retrieval, agent runs, handoffs, deliverables, schedules, and audit history.
- Canonical report/template path policy and non-empty output validation helpers.
- OpenCode development sidecar lifecycle: loopback spawn attempt, health polling, status, and stop commands.
- Browser-side Tauri IPC/OpenCode/SSE adapter modules ready to replace the prototype adapter.

## Next implementation slices

1. Add the Rust toolchain on the host and run `cargo check` / `cargo tauri dev` to validate exact Tauri shell capability APIs.
2. Replace the browser prototype adapter in `src/api.js` with narrow `invoke` wrappers and hydrate UI state from SQLite.
3. Add the typed Rust OfficeCLI action adapter; never provide generic shell execution.
4. Validate the pinned OpenCode session/SSE contract against a real bundled binary.
5. Add persistent tokio cron dispatch and restart reconciliation after the ad-hoc mission loop works.

See `../docs/backend.md` for the full command contract and safety requirements.
