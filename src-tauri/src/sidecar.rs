use std::{process::{Child, Command}, sync::Mutex, thread, time::Duration};
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::models::EngineStatus;

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
        self.status.lock().map(|status| status.clone()).unwrap_or_else(|_| EngineStatus::offline("Engine state lock unavailable."))
    }

    pub fn start(&self) -> Result<EngineStatus, String> {
        let mut child_guard = self.child.lock().map_err(|_| "Sidecar process lock unavailable.")?;
        if child_guard.is_some() {
            return Ok(self.current_status());
        }
        *self.status.lock().map_err(|_| "Sidecar status lock unavailable.")? = EngineStatus::starting();

        // Development can override this with KUBERA_OPENCODE_BIN=/absolute/path/opencode.
        // Packaging replaces this lookup with the target-specific Tauri external binary.
        let binary = std::env::var("KUBERA_OPENCODE_BIN").unwrap_or_else(|_| "opencode-cli".to_owned());
        let child = Command::new(&binary)
            .args(["serve", "--port", "4096", "--hostname", "127.0.0.1"])
            .spawn()
            .map_err(|error| {
                let message = format!("Could not start OpenCode binary '{binary}': {error}");
                if let Ok(mut status) = self.status.lock() { *status = EngineStatus::offline(message.clone()); }
                message
            })?;
        *child_guard = Some(child);
        drop(child_guard);

        let status = wait_for_health();
        *self.status.lock().map_err(|_| "Sidecar status lock unavailable.")? = status.clone();
        Ok(status)
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut child_guard = self.child.lock().map_err(|_| "Sidecar process lock unavailable.")?;
        if let Some(mut child) = child_guard.take() {
            child.kill().map_err(|error| format!("Could not stop OpenCode: {error}"))?;
            let _ = child.wait();
        }
        *self.status.lock().map_err(|_| "Sidecar status lock unavailable.")? = EngineStatus::offline("Engine stopped.");
        Ok(())
    }
}

fn wait_for_health() -> EngineStatus {
    let client = match Client::builder().timeout(Duration::from_secs(2)).build() {
        Ok(client) => client,
        Err(error) => return EngineStatus::offline(format!("Could not create local health client: {error}")),
    };
    for _ in 0..15 {
        if let Ok(response) = client.get("http://127.0.0.1:4096/global/health").send() {
            if let Ok(payload) = response.json::<HealthPayload>() {
                if payload.healthy {
                    return EngineStatus {
                        state: "ready".into(),
                        model: "configured by OpenCode".into(),
                        mode: "desktop".into(),
                        message: "OpenCode sidecar is healthy on loopback.".into(),
                        version: payload.version,
                    };
                }
            }
        }
        thread::sleep(Duration::from_millis(350));
    }
    EngineStatus::offline("OpenCode did not pass the loopback health check within 5 seconds.")
}
