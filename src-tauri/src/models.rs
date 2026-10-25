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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewClient {
    pub name: String,
    pub industry: Option<String>,
    pub status: Option<String>,
    pub owner: Option<String>,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProject {
    pub client_id: String,
    pub name: String,
    pub stage: Option<String>,
    pub value: f64,
    pub risk_flag: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProjectsCsvRequest {
    pub client_id: String,
    pub csv_text: String,
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
pub struct DeliverablePreview {
    pub deliverable_id: String,
    pub file_name: String,
    pub file_type: String,
    pub outline: String,
    pub truncated: bool,
}

/// Sandboxed, static OfficeCLI render output for an existing registered
/// deliverable. The renderer never receives a file path or a render command.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliverableVisualPreview {
    pub deliverable_id: String,
    pub file_name: String,
    pub file_type: String,
    pub html: String,
}

/// An explicit, approval-gated outbound delivery record. Credentials are read
/// only from the desktop host environment and never serialized to the UI.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryRecord {
    pub id: String,
    pub deliverable_id: String,
    pub file_name: String,
    pub channel: String,
    pub recipient: String,
    pub status: String,
    pub requested_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendApprovedDeliveryRequest {
    pub deliverable_id: String,
    pub channel: String,
    pub recipient: String,
    pub message: Option<String>,
}

/// A local human decision gate for a registered artifact. This deliberately
/// carries no external-recipient address or sending capability.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliverableApproval {
    pub id: String,
    pub deliverable_id: String,
    pub file_name: String,
    pub file_type: String,
    pub status: String,
    pub requested_by: String,
    pub decision_note: Option<String>,
    pub requested_at: i64,
    pub decided_at: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDecision {
    pub decision: String,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub file_type: String,
    pub placeholders: Vec<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTemplateRequest {
    pub file_name: String,
    pub file_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTemplateBundleRequest {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// Downloadable user-initiated export of one already-approved template. The
/// renderer receives bytes, never KUBERA's internal template path.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBundle {
    pub file_name: String,
    pub bytes: Vec<u8>,
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

/// Citation-only result produced by KUBERA's fixed approved research provider.
/// It is data for human selection, never a browser/tool capability for agents.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchCitation {
    pub id: String,
    pub provider: String,
    pub query: String,
    pub title: String,
    pub citation_url: String,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub fetched_at: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSearchRequest {
    pub query: String,
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
    /// Renderer-provided UUID keeps optimistic UI state and SQLite rows aligned.
    /// It is validated as a UUID rather than accepted as an arbitrary key.
    pub id: Option<String>,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDocumentRequest {
    pub task_id: String,
    pub file_name: String,
    pub title: String,
    #[serde(default)]
    pub paragraphs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkbookRequest {
    pub task_id: String,
    pub file_name: String,
    pub sheet_name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateValue {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTemplatedDocxRequest {
    pub task_id: String,
    pub template_id: String,
    pub file_name: String,
    pub values: Vec<TemplateValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeExecution {
    /// Set only after the generated artifact is durably registered in SQLite.
    pub deliverable_id: Option<String>,
    pub file_name: String,
    pub relative_path: String,
    pub file_type: String,
    pub bytes: u64,
    pub summary: String,
}
