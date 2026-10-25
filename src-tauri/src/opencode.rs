//! Narrow OpenCode HTTP bridge for the desktop runtime.
//!
//! The renderer never needs to know the loopback host. Tauri commands call this
//! module after `SidecarState` has reported healthy. The payload shape follows
//! the pinned OpenCode session contract and must be smoke-tested when pinning a
//! real binary release.

use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::time::Duration;

const BASE_URL: &str = "http://127.0.0.1:4096";

fn client() -> Result<Client, String> {
    Client::builder().timeout(Duration::from_secs(30)).build().map_err(|error| error.to_string())
}

pub fn health() -> Result<Value, String> {
    let response = client()?.get(format!("{BASE_URL}/global/health")).send().map_err(|error| error.to_string())?;
    response.error_for_status().map_err(|error| error.to_string())?.json().map_err(|error| error.to_string())
}

pub fn create_session() -> Result<Value, String> {
    let response = client()?.post(format!("{BASE_URL}/session")).json(&json!({})).send().map_err(|error| error.to_string())?;
    response.error_for_status().map_err(|error| error.to_string())?.json().map_err(|error| error.to_string())
}

pub fn send_message(session_id: &str, text: &str) -> Result<Value, String> {
    if session_id.trim().is_empty() || text.trim().is_empty() { return Err("OpenCode session ID and message text are required.".into()); }
    let body = json!({
        "parts": [{ "type": "text", "text": text }]
    });
    let response = client()?
        .post(format!("{BASE_URL}/session/{session_id}/message"))
        .json(&body)
        .send()
        .map_err(|error| error.to_string())?;
    response.error_for_status().map_err(|error| error.to_string())?.json().map_err(|error| error.to_string())
}
