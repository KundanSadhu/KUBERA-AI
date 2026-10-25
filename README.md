# KUBERA AI

KUBERA AI is a local-first desktop business-operations workspace. A user states a business goal; a controlled manager agent deploys a small, capability-limited team of specialist subagents; KUBERA produces a real Office artifact and stores it in a traceable Report Center.

> **File first, chat second.** The product outcome is an openable PPTX, DOCX, or XLSX—not a long chat response.

## Current implementation status

This repository contains the implementation starting point for the KUBERA vertical slice:

- A running vanilla HTML/CSS/JS interface with Missions, Workforce, Reports, Records, Settings, activity events, and browser-local demo persistence.
- A source layout prepared for the intended Tauri v2/Rust core.
- Detailed product, agent, backend, security, and deployment documentation in [`docs/`](docs/).
- A working deterministic local agent runtime for UI development: it selects business records, produces source-linked analyst handoffs, builds an artifact plan, and validates provenance. It is deliberately separate from the production OpenCode + OfficeCLI bridge.

The Rust/Tauri desktop bridge is scaffolded as source architecture but cannot be compiled in this workspace until the Rust toolchain is installed. See [`docs/setup.md`](docs/setup.md).

## Demo prompt

```text
Build a Q3 pipeline deck for Acme Corp
```

In the UI prototype this deploys:

1. **Asha — Manager** to create a bounded workforce plan.
2. **Maya — Market Analyst** to produce opportunity/risk findings.
3. **W-1 — Operations Analyst** to produce pipeline metrics.
4. **W-2 — Document Worker** to produce the final presentation artifact.
5. **Sentinel — Reviewer** to validate the result.

## Repository map

```text
kubera-ai/
├── docs/                 # Product and engineering documentation
├── src/                  # Static vanilla frontend implementation
├── src-tauri/            # Planned Tauri/Rust desktop core
├── package.json          # Lightweight local preview scripts
├── LICENSE               # Apache-2.0
└── CONTRIBUTING.md
```

## Run the UI prototype

```bash
npm install
npm run dev
```

Open the local URL printed by the server. The prototype uses `localStorage` only, so it is safe to reset via **Settings → Reset demo data**.

## Planned desktop execution

Once Rust and Tauri prerequisites are installed:

```bash
npm install
cd src-tauri
cargo tauri dev
```

The desktop implementation will replace browser-local persistence with SQLite, run OpenCode as a loopback-only sidecar, and expose only constrained OfficeCLI operations.

## What KUBERA is not

- Not a general chatbot.
- Not a coding agent or unrestricted shell agent.
- Not a hidden recursive swarm of agents.
- Not a cloud CRM or a multi-user platform in the vertical slice.

## Documentation

| Document | Purpose |
|---|---|
| [Architecture](docs/architecture.md) | Component boundaries, data flow, sidecars, and security model |
| [Working model](docs/working.md) | End-to-end mission and schedule behavior |
| [Setup](docs/setup.md) | Local development and packaging setup |
| [Backend](docs/backend.md) | SQLite schema, Tauri command contract, and Rust implementation plan |
| [Tech stack](docs/tech-stack.md) | Technology decisions and version/pinning rules |
| [Agents](docs/agents.md) | Manager/subagent workforce, handoffs, prompts, and guardrails |
| [Features](docs/features-list.md) | Scope, acceptance criteria, and roadmap |
| [Problems & solutions](docs/problems-solutions.md) | Risk register and mitigations |
| [Security](docs/security.md) | Threat model and least-privilege controls |
| [Demo script](docs/demo-script.md) | Repeatable acceptance demo |
| [Integration status](docs/integration-status.md) | What is running, source-implemented, and still needs host validation |
| [Markus-inspired workforce](docs/markus-inspired-workforce.md) | Team board, peer-review gate, workspaces, and patrol adaptation |
| [Peer review](docs/peer-review.md) | Structured-handoff and final-artifact delivery gates |
| [Agent workflow test](docs/agent-workflow-test.md) | Automated and manual verification of the workforce path |

## License

Apache License 2.0. See [`LICENSE`](LICENSE).
