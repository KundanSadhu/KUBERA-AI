# Integration Status

## Implemented in source

| Capability | Status | Notes |
|---|---|---|
| Vanilla desktop-oriented UI | Working browser prototype | Missions, workforce, reports, records, schedules, activity, settings, local persistence |
| Deterministic local workforce | Working and tested browser runtime | Manager selects records; Maya/W-1 produce real computed source-linked handoffs; W-2 builds an artifact plan; Sentinel validates provenance |
| Tauri v2 shell | Implemented in source | Configuration, window, CSP, IPC registration, SQLite bootstrap and restart reconciliation |
| SQLite schema and repository commands | Implemented in Rust source | Durable records, tasks, agent runs, accepted handoffs, deliverables, schedules, and audit events |
| OpenCode sidecar lifecycle | Implemented in Rust source | Loopback-only spawn, five-second health polling, status/stop commands, project-owned restricted config |
| OpenCode free-model policy | Implemented and contract-tested in source | Manager requests explicitly force `opencode/deepseek-v4.1-flash-free`; `opencode/glm-5.3-flash-free` is reserved for lightweight OpenCode work; no silent mid-run switching |
| OpenCode HTTP/SSE bridge | Implemented in Rust and JS source | Tauri session/message commands and raw SSE forwarding; renderer never reaches loopback itself |
| Tauri IPC client | Implemented in JS source | Desktop adapter hydrates SQLite and persists atomic task/run/handoff lifecycles, client/project creation, schedules, and typed report actions |
| Desktop client/project CRUD | Implemented in Rust and JS source | Typed inputs validate lengths/status/value, enforce the project-to-client foreign key, return durable records, and append audit events |
| Approved template import and DOCX merge | Implemented in Rust and JS source | User-selected DOCX/XLSX/PPTX bytes are capped at 4 MiB, validated as OOXML packages, written only under KUBERA's template root, registered in SQLite, and audited. A DOCX merge accepts only the exact inspected `{{placeholder}}` set, escapes XML values, confines the output to report root, then invokes OfficeCLI package validation |
| Governed OfficeCLI actions | Implemented in Rust source | Only W-2 can create typed PPTX, DOCX, or XLSX artifacts; backend owns report path, bounded content schema, CLI arguments, timeout, close/validation, and registration |
| In-app Office outline preview | Implemented in Rust and JS source | Report Center asks only for a registered deliverable UUID; backend re-canonicalizes it and returns a 64 KiB, 20-second bounded `OfficeCLI view outline` response, never a path or raw CLI capability |
| Native report actions | Implemented in Rust and JS source | Open/reveal accepts only an existing SQLite deliverable UUID, canonicalizes it beneath the report root, uses platform-native open behavior, and appends an audit event |
| Cron dispatcher | Implemented in Rust and JS source | Five/six/seven-field cron parsing with IANA timezones, durable next-run advancement, audit event, renderer subscription before worker start, and a one-at-a-time scheduled mission queue |

## Not verifiable in this workspace

Arena can parse/format the Rust source but cannot complete a native Tauri check because the host lacks GTK/GDK development libraries; cross-checking a Windows target also lacks the MSVC linker. The pinned Linux OpenCode `v1.19.4` sidecar did pass health, session creation, and global SSE smoke checks. Its official free-model configuration was resolved into a real `plan` session with all prohibited tools denied, but a bounded 120-second request did not produce a completion in this sandbox, so successful inference is **not** claimed. The pinned Linux OfficeCLI can create/close a container but its add-slide operation has a runtime assembly defect. Native Windows build, authenticated/available free-model completion, and a slide-bearing Windows deck remain host/CI acceptance gates.

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
