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
- An OpenAI-compatible provider endpoint/key, or a reachable local Ollama instance

Verify installation:

```bash
rustc --version
cargo --version
cargo tauri --version
```

> The current Arena workspace exposes Node but not a Rust toolchain. The UI source can be developed and tested here; compile the Tauri core on a machine with Rust installed.

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

## 4. Provider configuration

The Settings screen accepts:

| Setting | Example | Notes |
|---|---|---|
| Provider base URL | `https://api.example.com/v1` | OpenAI-compatible endpoint |
| Model | `gpt-4o-mini` or local model name | Must support the required tool/JSON behavior |
| API key | masked | Never send to prompts/audit/activity logs |
| Timezone | `Asia/Calcutta` | Used to display schedule next runs |

Production should store the API key in OS credential storage or Tauri Stronghold. A plain app-data config file is not acceptable for a production secret.

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
