//! P0 runtime contracts shared by Tauri commands and the frontend adapter.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub id: String,
    pub requirement_id: String,
    pub title: String,
    pub brief: String,
    pub role: String,
    pub required_capability: Option<String>,
    pub assignee_id: Option<String>,
    pub status: String,
    #[serde(default)]
    pub deps: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub id: String,
    pub requirement_id: String,
    pub title: String,
    pub brief: String,
    pub role: String,
    pub required_capability: Option<String>,
    pub assignee_id: Option<String>,
    pub status: String,
    pub deps: Vec<String>,
    pub output_path: Option<String>,
    pub review_notes: Option<String>,
    pub attempts: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunRecord {
    pub id: String,
    pub task_id: String,
    pub agent_id: String,
    pub parent_run_id: Option<String>,
    pub opencode_session_id: Option<String>,
    pub status: String,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub error_code: Option<String>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHandoffRecord {
    pub id: String,
    pub requirement_id: String,
    pub task_id: String,
    pub from_run_id: String,
    pub to_task_id: Option<String>,
    pub to_agent_id: Option<String>,
    pub handoff_type: String,
    pub payload_json: String,
    pub source_refs_json: String,
    pub validation_status: String,
    pub created_at: i64,
}
