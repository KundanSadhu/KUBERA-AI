use reqwest::blocking::Client;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::Mutex,
    thread,
    time::Duration,
};
use tauri::{AppHandle, Manager};

use crate::models::EngineStatus;

const PRIMARY_FREE_MODEL: &str = "opencode/deepseek-v4.1-flash-free";
const FALLBACK_FREE_MODEL: &str = "opencode/glm-5.3-flash-free";

pub struct SidecarState {
    child: Mutex<Option<Child>>,
    status: Mutex<EngineStatus>,
}

impl Default for SidecarState {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            status: Mutex::new(EngineStatus::offline("Engine has not been started.")),
        }
    }
}

#[derive(Deserialize)]
struct HealthPayload {
    healthy: bool,
    version: Option<String>,
}

impl SidecarState {
    pub fn current_status(&self) -> EngineStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| EngineStatus::offline("Engine state lock unavailable."))
    }

    pub fn start(&self, app: &AppHandle) -> Result<EngineStatus, String> {
        let mut child_guard = self
            .child
            .lock()
            .map_err(|_| "Sidecar process lock unavailable.")?;
        if child_guard.is_some() {
            return Ok(self.current_status());
        }
        *self
            .status
            .lock()
            .map_err(|_| "Sidecar status lock unavailable.")? = EngineStatus::starting();

        let binary = match resolve_binary(app) {
            Ok(binary) => binary,
            Err(error) => {
                *self
                    .status
                    .lock()
                    .map_err(|_| "Sidecar status lock unavailable.")? =
                    EngineStatus::offline(error.clone());
                return Err(error);
            }
        };
        let workdir = app_data_dir(app)?;
        prepare_restricted_project_config(&workdir)?;
        let child = Command::new(&binary)
            // OpenCode resolves project configuration per working directory.
            // KUBERA owns this otherwise-empty directory, never the product
            // source tree or an arbitrary user workspace.
            .current_dir(&workdir)
            .args(["serve", "--port", "4096", "--hostname", "127.0.0.1"])
            .spawn()
            .map_err(|error| {
                let message = format!(
                    "Could not start OpenCode binary '{}': {error}",
                    binary.display()
                );
                if let Ok(mut status) = self.status.lock() {
                    *status = EngineStatus::offline(message.clone());
                }
                message
            })?;
        *child_guard = Some(child);
        drop(child_guard);

        let status = wait_for_health();
        if status.state != "ready" {
            // Do not retain a dead process: a later explicit Start must be
            // allowed to retry with repaired credentials or a repaired binary.
            if let Ok(mut guard) = self.child.lock() {
                if let Some(mut child) = guard.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
        *self
            .status
            .lock()
            .map_err(|_| "Sidecar status lock unavailable.")? = status.clone();
        Ok(status)
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut child_guard = self
            .child
            .lock()
            .map_err(|_| "Sidecar process lock unavailable.")?;
        if let Some(mut child) = child_guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        *self
            .status
            .lock()
            .map_err(|_| "Sidecar status lock unavailable.")? =
            EngineStatus::offline("Engine stopped.");
        Ok(())
    }
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve KUBERA app data directory: {error}"))
}

fn prepare_restricted_project_config(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("Could not prepare OpenCode work directory: {error}"))?;
    // Defense in depth: request-level tools are also disabled in opencode.rs.
    // This project-local configuration prevents a sidecar from inheriting an
    // accidental build-agent profile when it initializes its KUBERA workspace.
    //
    // KUBERA deliberately pins the manager to an official OpenCode free model.
    // `small_model` is a second official free model reserved for lightweight
    // OpenCode internals. It is not an automatic retry fallback: silently
    // changing a model during an audited manager run would undermine
    // reproducibility. A future user-approved model change is a new run.
    let config = include_str!("../opencode.kubera.json");
    fs::write(root.join("opencode.json"), config).map_err(|error| {
        format!("Could not write restricted KUBERA OpenCode configuration: {error}")
    })
}

fn resolve_binary(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("KUBERA_OPENCODE_BIN") {
        return Ok(PathBuf::from(value));
    }
    let name = bundled_binary_name();
    // In a packaged Tauri app, externalBin files live in the resource directory.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join(&name);
        if bundled.exists() {
            return Ok(bundled);
        }
    }
    // During `cargo tauri dev`, CI prepares the target-suffixed binary under
    // src-tauri/binaries. Developers may also use KUBERA_OPENCODE_BIN instead.
    let dev = PathBuf::from("binaries").join(&name);
    if dev.exists() {
        return Ok(dev);
    }
    Err(format!(
        "Bundled OpenCode binary '{name}' was not found. Set KUBERA_OPENCODE_BIN for development."
    ))
}

fn bundled_binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "opencode-cli-x86_64-pc-windows-msvc.exe"
    }
    #[cfg(target_os = "macos")]
    {
        "opencode-cli-aarch64-apple-darwin"
    }
    #[cfg(target_os = "linux")]
    {
        "opencode-cli-x86_64-unknown-linux-gnu"
    }
}

fn wait_for_health() -> EngineStatus {
    let client = match Client::builder().timeout(Duration::from_secs(2)).build() {
        Ok(client) => client,
        Err(error) => {
            return EngineStatus::offline(format!("Could not create local health client: {error}"))
        }
    };
    for _ in 0..15 {
        if let Ok(response) = client.get("http://127.0.0.1:4096/global/health").send() {
            if let Ok(payload) = response.json::<HealthPayload>() {
                if payload.healthy {
                    return EngineStatus {
                        state: "ready".into(),
                        model: PRIMARY_FREE_MODEL.into(),
                        mode: "desktop".into(),
                        message: format!(
                            "OpenCode sidecar is healthy on loopback. Free-model policy: primary {PRIMARY_FREE_MODEL}; lightweight fallback {FALLBACK_FREE_MODEL}; no silent mid-run switch."
                        ),
                        version: payload.version,
                    };
                }
            }
        }
        thread::sleep(Duration::from_millis(350));
    }
    EngineStatus::offline("OpenCode did not pass the loopback health check within 5 seconds.")
}
