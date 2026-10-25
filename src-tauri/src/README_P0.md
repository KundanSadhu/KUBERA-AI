# P0 Runtime Slice

This source slice moves KUBERA from a UI-only workforce demonstration toward an executable, local-first desktop runtime.

## Implemented boundaries

1. **SQLite durability and reconciliation**
   - Requirements, tasks, agent runs, accepted handoffs, deliverables, schedules, and audit events are typed Tauri/SQLite records.
   - The Tauri bootstrap hydrates the renderer from SQLite rather than browser `localStorage`.
   - On desktop restart, incomplete `running` agent runs are marked `cancelled` with `desktop_restart`, agents are released to `idle`, and unfinished `in_progress` tasks without an output file return to `ready` for governed retry. Completed files/review states are not rewritten.

2. **One bounded OpenCode manager session**
   - The desktop adapter starts an atomic manager run, creates a loopback-only OpenCode session, stores its session ID in SQLite, sends a bounded planning prompt, and completes the run with an output summary.
   - Raw OpenCode SSE is forwarded across the Tauri event boundary as `opencode-event`; the renderer never connects to loopback itself.
   - OpenCode runs from the KUBERA app-data directory with a project-local restricted configuration. The HTTP prompt request separately disables shell, read/write/edit, browser/web research, skills, MCP-style tool use, and nested-agent tools. KUBERA keeps all governed data/file actions behind typed commands.

3. **Typed OfficeCLI PPTX creation**
   - Only an active W-2 document task can request a deck.
   - KUBERA owns the report-root path, generated file name, title/slide/bullet limits, OfficeCLI argument sequence, 45-second process limits, explicit close/flush, non-empty output validation, and SQLite deliverable registration.
   - No renderer or agent receives a general shell command or arbitrary filesystem path.

## Runtime lifecycle

The manager and analyst/document/reviewer task lifecycle mirrors into SQLite. Analysts claim atomic runs; their source-linked handoffs are persisted only after the peer gate accepts them. The document worker starts a governed run before the typed PPTX request, and Sentinel finalizes the delivery state.

## Validation status

- JavaScript syntax checks and deterministic workforce tests run in this workspace.
- Rust P0 sources have been parsed/formatted with `rustfmt`.
- A native Tauri/Windows build, live pinned OpenCode session, and real OfficeCLI deck must still be executed on a Windows host or the Windows GitHub Actions runner before this source slice is represented as shipping installer parity.

### Local smoke evidence (October 25, 2026)

- The pinned Linux OpenCode `v1.19.4` binary started on loopback, returned `{ "healthy": true, "version": "1.19.4" }` from `/global/health`, created a real `/session` record, and emitted the initial `server.connected` SSE payload from `/global/event`.
- A text completion was not possible in this workspace because no model provider credentials/default model are configured. The desktop path correctly times out/falls back rather than fabricating model output.
- The pinned Linux OfficeCLI `v1.0.152` binary passed `--version`, `create`, and `close` against a real PPTX container. Its `add ... --type slide` invocation then failed in this Linux host with a missing `System.Linq.Expressions, Version=10.0.0.0` assembly, so no slide-bearing deck was registered. KUBERA's typed adapter treats this as an OfficeCLI failure and does not register a deliverable. The pinned Windows OfficeCLI binary remains the required final smoke target.
