use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub state: String,
    pub model: String,
    pub mode: String,
    pub message: String,
    pub version: Option<String>,
}

impl EngineStatus {
    pub fn starting() -> Self {
        Self {
            state: "starting".into(),
            model: "not configured".into(),
            mode: "desktop".into(),
            message: "Starting OpenCode sidecar".into(),
            version: None,
        }
    }

    pub fn offline(message: impl Into<String>) -> Self {
        Self {
            state: "offline".into(),
            model: "not configured".into(),
            mode: "desktop".into(),
            message: message.into(),
            version: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: String,
    pub name: String,
    pub industry: Option<String>,
    pub status: String,
    pub owner: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub client_id: String,
    pub client_name: String,
    pub name: String,
    pub stage: Option<String>,
    pub value: f64,
    pub risk_flag: bool,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub agent_kind: String,
    pub capabilities: Vec<String>,
    pub max_concurrency: i64,
    pub status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub id: String,
    pub text: String,
    pub status: String,
    pub deliverable_type: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deliverable {
    pub id: String,
    pub task_id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub id: String,
    pub requirement_text: String,
    pub cron_expression: String,
    pub timezone: String,
    pub deliverable_type: Option<String>,
    pub last_run: Option<i64>,
    pub next_run: Option<i64>,
    pub enabled: bool,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: i64,
    pub actor: String,
    pub action: String,
    pub target: Option<String>,
    pub detail: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionSelector {
    pub client_name: Option<String>,
    #[serde(default)]
    pub include_projects: bool,
    pub project_status: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionContext {
    pub client: Option<Client>,
    pub projects: Vec<Project>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSchedule {
    pub requirement_text: String,
    pub cron_expression: String,
    pub timezone: String,
    pub deliverable_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckSlide {
    pub title: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeckRequest {
    pub task_id: String,
    pub file_name: String,
    pub title: String,
    pub slides: Vec<DeckSlide>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeExecution {
    pub file_name: String,
    pub relative_path: String,
    pub file_type: String,
    pub bytes: u64,
    pub summary: String,
}
