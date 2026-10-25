# Technology Stack

## Chosen stack

| Layer | Choice | Reason |
|---|---|---|
| Desktop shell | Tauri v2 | Small local desktop package, native integration, Rust backend |
| Renderer | Vanilla HTML, CSS, ES modules | Fast iteration; no framework or frontend build complexity |
| Agent runtime | OpenCode in serve mode | Headless session API and streaming events through a locally bundled sidecar |
| Office artifacts | OfficeCLI | Constrained creation/merge/render surface without an Office installation |
| Persistence | SQLite via `rusqlite` with `bundled` feature | Local-first, zero external DB configuration |
| Scheduler | `cron` + `chrono-tz` in a bounded Rust worker | IANA-timezone recurring missions while the desktop app is open; each tick persists before emitting |
| Process control | `tauri-plugin-shell` | Launch known external sidecars with minimal capability grants |
| Local UI preview | Node static server | Lets UI develop before a Rust/Tauri host is available |
| Icons/fonts | Inline SVG and system fonts | No asset pipeline or CDN dependency |

## Frontend modules

```text
src/
├── index.html
├── app.css
├── main.js             # Bootstrap, event delegation, router
├── store.js            # Tiny pub/sub state store
├── api.js              # Browser mock now; future Tauri wrappers
├── orchestration.js    # Mission/workforce lifecycle
├── seed.js             # Prototype seed state
├── formatters.js       # Dates, currency, escaping
└── views/
    ├── shell.js
    ├── missions.js
    ├── workforce.js
    ├── report-center.js
    ├── records.js
    ├── settings.js
    └── activity.js
```

## Rust crates to pin after proof

| Crate / tool | Role |
|---|---|
| `tauri` v2 | Desktop runtime |
| `tauri-plugin-shell` | Bundled sidecar lifecycle |
| `rusqlite` with `bundled` | SQLite storage |
| `cron` | Cron expression parsing and next-occurrence calculation |
| `chrono`, `chrono-tz` | IANA timezone-safe schedule calculation |
| `serde`, `serde_json` | Command/data serialization |
| `uuid` | Stable IDs |
| `reqwest` | Local sidecar health polling |
| `thiserror` or `anyhow` | Error boundaries |

## Version and compatibility policy

- Pin OpenCode and OfficeCLI versions in a release manifest.
- Record sidecar versions in the diagnostics view.
- Validate exact session, health, SSE, and restricted-tool APIs against the pinned OpenCode build.
- Pin Rust crate minor versions after the first successful host build.
- Test one target OS completely before claiming cross-platform support.
- Bundle binaries using Tauri’s required target-triple suffix.

## Explicitly avoided in the vertical slice

- React/Vue/Svelte build chain.
- Remote Postgres/Redis/backend API.
- Generic shell tools for agents.
- Python/LibreOffice conversion pipelines.
- Heavy icon/font/CDN asset dependencies.
- External task queue or microservice architecture.

## Environment variables for development

No environment variable is required for the browser UI prototype. The future desktop app may read provider configuration from Settings/secure storage rather than an environment variable.

For a local development convenience only, a host may define values such as:

```text
KUBERA_PROVIDER_BASE_URL=
KUBERA_PROVIDER_MODEL=
KUBERA_PROVIDER_API_KEY=
```

These must never be committed, injected into frontend bundles, or logged. Production UI configuration should use an encrypted/local secure store.

## Why not a generic agent framework?

The differentiation is reliable business output, not agent complexity. KUBERA needs a durable task graph, capability restriction, source-linked handoffs, Office-file validity, and a report center. A narrow orchestration module plus OpenCode sessions is more inspectable than adding a broad autonomous-agent framework.
