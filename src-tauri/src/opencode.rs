//! Narrow OpenCode HTTP and event bridge for the desktop runtime.
//!
//! The renderer never calls loopback directly. Tauri commands and events own
//! the local OpenCode connection, allowing a browser-facing UI to keep using
//! relative/Tauri APIs in every environment.

use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter};

const BASE_URL: &str = "http://127.0.0.1:4096";

pub struct EventForwarder {
    started: AtomicBool,
}
impl Default for EventForwarder {
    fn default() -> Self {
        Self {
            started: AtomicBool::new(false),
        }
    }
}

fn client(timeout: Duration) -> Result<Client, String> {
    Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| error.to_string())
}

fn event_client() -> Result<Client, String> {
    // SSE is intentionally long lived. A zero duration is interpreted as an
    // immediate timeout by several HTTP stacks, so it must not be used here.
    Client::builder().build().map_err(|error| error.to_string())
}

pub fn health() -> Result<Value, String> {
    let response = client(Duration::from_secs(5))?
        .get(format!("{BASE_URL}/global/health"))
        .send()
        .map_err(|error| error.to_string())?;
    response
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())
}

pub fn create_session() -> Result<Value, String> {
    let response = client(Duration::from_secs(30))?
        .post(format!("{BASE_URL}/session"))
        .json(&json!({}))
        .send()
        .map_err(|error| error.to_string())?;
    response
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())
}

pub fn send_message(session_id: &str, text: &str) -> Result<Value, String> {
    if session_id.trim().is_empty() || text.trim().is_empty() {
        return Err("OpenCode session ID and message text are required.".into());
    }
    // KUBERA uses OpenCode as a text-only reasoning sidecar. Tool calls are
    // disabled at the server request boundary rather than merely prohibited in
    // the prompt, including shell, edits, reads, browsing, skills, and nested
    // agent creation. The desktop runtime retains its own typed SQLite/Office
    // commands for every governed operation.
    let body = json!({
        "agent": "plan",
        "tools": {
            "bash": false, "read": false, "edit": false, "write": false,
            "patch": false, "glob": false, "grep": false, "list": false,
            "webfetch": false, "websearch": false, "task": false,
            "skill": false, "lsp": false, "todowrite": false, "question": false
        },
        "parts": [{ "type": "text", "text": text }]
    });
    let response = client(Duration::from_secs(120))?
        .post(format!("{BASE_URL}/session/{session_id}/message"))
        .json(&body)
        .send()
        .map_err(|error| error.to_string())?;
    response
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())
}

/// Connects once per app process and relays JSON SSE payloads through Tauri.
/// A broken stream retries with a bounded delay; sidecar shutdown simply makes
/// the forwarder dormant until the next successful connection.
pub fn start_event_forwarder(state: &EventForwarder, app: AppHandle) {
    if state.started.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || loop {
        let response = match event_client().and_then(|http| {
            http.get(format!("{BASE_URL}/global/event"))
                .send()
                .map_err(|error| error.to_string())
        }) {
            Ok(response) => response,
            Err(_) => {
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };
        let reader = BufReader::new(response);
        for line in reader.lines().map_while(Result::ok) {
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            let event = serde_json::from_str::<Value>(payload)
                .unwrap_or_else(|_| json!({ "type": "unparsed", "data": payload }));
            let _ = app.emit("opencode-event", event);
        }
        thread::sleep(Duration::from_secs(1));
    });
}
