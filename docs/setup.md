# Setup Guide

## 1. Local UI prototype

The initial implementation is intentionally runnable without a build framework.

### Prerequisites

- Node.js 20 or newer
- npm 10 or newer
- A modern desktop browser

### Run

```bash
cd kubera-ai
npm install
npm run dev
```

The command serves `src/` on `0.0.0.0` so it can be opened in a local browser or preview environment. The prototype persists demo records in browser `localStorage`; use **Settings → Reset demo data** to restore the seed state.

### Quality checks

```bash
npm run check
```

This performs syntax checks for the frontend ES modules. It does not require a network connection after dependencies are installed.

## 2. Tauri desktop prerequisites

The production architecture requires the following on the target host:

- Rust toolchain through `rustup` (stable)
- Tauri CLI (`cargo install tauri-cli --version '^2'`)
- Platform dependencies:
  - **macOS:** Xcode Command Line Tools
  - **Windows:** Microsoft C++ Build Tools and WebView2 Runtime
  - **Linux:** WebKitGTK and Tauri system packages for the selected distribution
- An OpenCode account/session if the selected availability-limited OpenCode free model asks for authentication; KUBERA does not require an OpenAI-compatible endpoint or a general-purpose local model

Verify installation:

```bash
rustc --version
cargo --version
cargo tauri --version
```

> The current Arena workspace can format/parse the Rust sources, but lacks the GTK/GDK native libraries required for a Linux Tauri build and the MSVC linker required for a Windows cross-build. Use a platform-ready host or the Windows GitHub Actions runner for native validation.

## 3. Sidecar installation

KUBERA packages two target-specific binaries under `src-tauri/binaries/`:

```text
opencode-cli-<target-triple>
officecli-<target-triple>
```

Examples of target triples include `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, and `x86_64-unknown-linux-gnu`.

Before building UI integrations, manually verify on the target OS:

1. `opencode serve` starts on loopback and reports healthy.
2. A session can be created and produces SSE events.
3. OfficeCLI creates an openable one-slide PPTX.
4. OfficeCLI merges a simple DOCX template with `{{PLACEHOLDER}}` values.
5. OfficeCLI creates an openable XLSX.

Do not proceed to end-to-end development if these proofs fail.

## 4. OpenCode free-model configuration

KUBERA owns the OpenCode project configuration at runtime; the checked-in policy is [`src-tauri/opencode.kubera.json`](../src-tauri/opencode.kubera.json). It uses the official OpenCode provider rather than a direct third-party API client:

| Role | Model | Policy |
|---|---|---|
| Governed manager / agentic reasoning | `opencode/deepseek-v4.1-flash-free` | Explicitly selected both in the sidecar configuration and in each HTTP prompt request |
| Lightweight OpenCode work | `opencode/glm-5.3-flash-free` | Reserved by `small_model`; it is not a silent retry path for a manager run |

Free-model availability can rotate or be rate-limited. If the primary model is unavailable, KUBERA records a failed governed run instead of switching providers or models without an auditable user decision. Start a new run after a user-approved policy change.

If OpenCode requests authentication for its free-service endpoint, complete that authentication through the official OpenCode flow for the desktop user's account. Do not paste provider secrets into a requirement, model prompt, audit event, or `opencode.kubera.json`; a future credential-settings surface must use OS credential storage or Tauri Stronghold.

The sidecar remains loopback-only and denies every general shell, filesystem, browser/web, nested-agent, skill, and MCP-style tool. KUBERA’s typed SQLite and OfficeCLI commands remain the only paths to governed local records and files.

### Approved DOCX templates

Desktop template import copies a user-selected `.docx`, `.xlsx`, or `.pptx` into KUBERA's local template root after a 4 MiB limit and OOXML package check. DOCX import records `{{placeholder}}` tokens without exposing a source path. During a document-worker merge, values must exactly match the inspected token set. The manager workflow currently supports these trusted report tokens: `{{client_name}}`, `{{project_count}}`, `{{pipeline_value}}`, `{{at_risk_count}}`, `{{requirement}}`, and `{{report_title}}`. Keep each token in one Word text run; split Word runs are rejected rather than merged ambiguously.

### Explicit approved delivery

KUBERA never stores outbound credentials in SQLite or exposes them to OpenCode, agents, or the renderer. A user must first approve a registered artifact in the Report Center and then deliberately select a delivery channel.

- **SMTP email attachment:** set `KUBERA_SMTP_HOST`, `KUBERA_SMTP_PORT` (optional; defaults to `587`), `KUBERA_SMTP_USERNAME`, `KUBERA_SMTP_PASSWORD`, and `KUBERA_SMTP_FROM` in the desktop host environment.
- **Slack file upload:** set `KUBERA_SLACK_BOT_TOKEN` in the desktop host environment. The user must enter a Slack channel ID (`C…`, `G…`, or `D…`) in the delivery form. The bot needs permission to upload files to that destination.

KUBERA records only delivery metadata and state. It never records SMTP passwords, Slack tokens, provider diagnostics, or arbitrary webhook URLs. Delivery remains a target-host acceptance gate until credentials and provider policy are verified.

## 5. Run the future desktop app

Once `src-tauri` is implemented and Rust is installed:

```bash
npm install
cd src-tauri
cargo tauri dev
```

Expected boot sequence:

1. SQLite migration/seed.
2. Report/template directory creation.
3. OpenCode sidecar spawn at `127.0.0.1:4096`.
4. `/global/health` polling.
5. Engine state changes from `Starting` to `Ready`.

## 6. Package the host platform

```bash
cd src-tauri
cargo tauri build --bundles nsis
```

Before shipping, run the demo script twice from a clean launch and open every generated Office artifact in a native Office-compatible application.

## Troubleshooting

| Symptom | Check |
|---|---|
| UI preview is blank | Run `npm run check`; ensure the server root is `src/` |
| Engine stays Starting | Sidecar binary name/target triple, port conflict, sidecar stdout, `/global/health` |
| No provider output | Base URL, model, key, provider-compatible OpenCode configuration |
| Document invalid | Re-run controlled OfficeCLI sample; inspect adapter arguments and report path policy |
| Scheduled job time is wrong | Verify stored timezone and cron library’s field/timezone behavior |
| Report cannot open | Confirm it exists beneath the report root and has a registered deliverable ID |
