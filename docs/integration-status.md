# Integration Status

## Implemented in source

| Capability | Status | Notes |
|---|---|---|
| Vanilla desktop-oriented UI | Working browser prototype | Missions, workforce, reports, records, schedules, activity, settings, local persistence |
| Deterministic local workforce | Working and tested browser runtime | Manager selects records; Maya/W-1 produce real computed source-linked handoffs; W-2 builds an artifact plan; Sentinel validates provenance |
| Tauri v2 shell | Scaffolded | Configuration, window, CSP, shell plugin, IPC registration |
| SQLite schema and seed | Implemented in Rust source | Bundled `rusqlite` schema covers records, workforce, handoffs, deliverables, schedules, audit |
| SQLite repository commands | Implemented in Rust source | Read records, create requirements/schedules/runs, handoffs, deliverables, audit |
| OpenCode sidecar lifecycle | Implemented in Rust source | Loopback spawn attempt, five-second health polling, status/stop commands |
| OpenCode HTTP bridge | Implemented in Rust and JS source | Tauri session/message commands plus same-origin development proxy and SSE client; pin/smoke-test the selected OpenCode version on host |
| Tauri IPC client | Implemented in JS source | Isolated adapter for replacing browser localStorage |
| Office action policy | Implemented in JS source | Renderer-side typed validation mirror; Rust OfficeCLI process adapter remains next slice |
| Cron dispatcher | Prepared | Schedule persistence + validation boundary; live tokio scheduler remains next slice |

## Not verifiable in this workspace

The Arena workspace does not have a Rust toolchain, OpenCode sidecar binary, or OfficeCLI binary. Therefore, the Rust modules are source-complete but **not compiled or executed here**. The browser UI is the running development surface.

## Host-machine verification order

1. Install Rust/Tauri prerequisites.
2. Run `cargo check` and resolve version-specific Tauri shell/capability details.
3. Confirm SQLite migration and seed data in the desktop app-data folder.
4. Set `KUBERA_OPENCODE_BIN` to a pinned OpenCode executable and run the sidecar health command.
5. Validate the OpenCode session/SSE contract against the pinned sidecar version.
6. Add the typed Rust OfficeCLI adapter and manually verify valid PPTX/DOCX/XLSX outputs.
7. Enable persistent tokio scheduler after cron/timezone validation.

## Important safety boundary

Do not replace the restricted Office contract with a generic `exec` command to make integration easier. The browser and Rust policies must remain capability-scoped, path-constrained, and audit-first.
