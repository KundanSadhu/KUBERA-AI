# CI/CD and Windows Desktop Delivery

## Pipeline overview

KUBERA has two GitHub Actions workflows:

| Workflow | Trigger | Purpose |
|---|---|---|
| `CI` | Pull requests, pushes to `main`, manual runs | Checks JavaScript syntax, repository whitespace, deterministic workforce behavior, and the desktop SQLite persistence contract |
| `Windows Desktop` | Pull requests, pushes to `main`, version tags, manual runs | Verifies frontend/workforce tests and Rust formatting, compile-checks the Windows Tauri core, downloads pinned sidecars, builds a Windows NSIS installer, uploads it as an artifact, and publishes tagged releases |

## Windows build contents

The Windows workflow downloads and verifies via SHA-256 manifest:

- OpenCode `v1.19.4`, from the upstream Windows x64 baseline archive.
- OfficeCLI `v1.0.152`, from the upstream Windows x64 executable release.

It renames these binaries to Tauri’s required external-binary convention:

```text
src-tauri/binaries/opencode-cli-x86_64-pc-windows-msvc.exe
src-tauri/binaries/officecli-x86_64-pc-windows-msvc.exe
```

Tauri embeds them into the Windows desktop package. The temporary sidecars and their generated manifest are build artifacts; they are not committed.

## Automatic behavior

- A pull request validates source and attempts a Windows installer build.
- A push to `main` builds a Windows NSIS installer and stores it in the GitHub Actions run artifacts.
- A pushed version tag such as `v0.1.0` builds the installer and attaches the `.exe` plus `.sha256` checksum to a GitHub Release.

## Publish a Windows release

```bash
git checkout main
git pull --ff-only
git tag -a v0.1.0 -m "KUBERA AI v0.1.0"
git push origin v0.1.0
```

The `Windows Desktop` workflow creates release notes and uploads the NSIS installer automatically.

## Security controls

- Workflows use GitHub Actions’ ephemeral `GITHUB_TOKEN`; no deploy key is stored in workflow YAML.
- The release step is restricted to version tags.
- Sidecars are pinned in workflow environment variables, not downloaded as unversioned `latest` releases.
- The workflow records source URLs and SHA-256 digests in `sidecar-manifest.json`.
- GitHub Actions artifacts retain the installer/checksum according to the repository’s Actions retention policy.

## Updating sidecars

Edit `OPENCODE_VERSION` and `OFFICECLI_VERSION` in `.github/workflows/windows-desktop.yml`, then run the workflow manually. Validate a generated installer and an end-to-end PPTX/DOCX/XLSX mission before tagging a release.

## Current limitation

Rust source formatting and Cargo-manifest parsing are checked in this workspace. A native Linux Tauri compile requires unavailable GTK/GDK system libraries, and a Windows cross-check requires the Microsoft Windows SDK/linker. The Windows workflow therefore remains the authoritative native compilation, sidecar, and installer verification environment.

Before a release tag, verify one packaged Windows mission end-to-end: SQLite state survives a restart, the loopback OpenCode manager session receives an event, and OfficeCLI produces a nonempty `.pptx` under the KUBERA report root.
