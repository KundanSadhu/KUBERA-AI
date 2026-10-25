use lettre::{
    message::{Attachment, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};
use rusqlite::{params, OptionalExtension};
use std::{
    collections::BTreeSet,
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tauri::State;
use uuid::Uuid;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::{
    db, files,
    models::{
        Agent, ApprovalDecision, AuditEvent, Client, CreateDeckRequest, CreateDocumentRequest,
        CreateTemplatedDocxRequest, CreateWorkbookRequest, Deliverable, DeliverableApproval,
        DeliverablePreview, DeliverableVisualPreview, DeliveryRecord, ImportProjectsCsvRequest,
        ImportTemplateBundleRequest, ImportTemplateRequest, MissionContext, MissionSelector,
        NewClient, NewProject, NewSchedule, OfficeExecution, Project, Requirement,
        ResearchCitation, ResearchSearchRequest, Schedule, SendApprovedDeliveryRequest, Template,
        TemplateBundle,
    },
    office,
    p0::{AgentHandoffRecord, AgentRunRecord, NewTask, TaskRecord},
    paths::AppPaths,
    scheduler,
};

fn map_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn supported_type(file_type: &str) -> bool {
    matches!(file_type, "pptx" | "docx" | "xlsx" | "pdf")
}

#[tauri::command]
pub fn list_clients(paths: State<'_, AppPaths>) -> Result<Vec<Client>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, name, industry, status, owner, created_at FROM clients ORDER BY name COLLATE NOCASE").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Client {
                id: row.get(0)?,
                name: row.get(1)?,
                industry: row.get(2)?,
                status: row.get(3)?,
                owner: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn create_client(paths: State<'_, AppPaths>, input: NewClient) -> Result<Client, String> {
    let name = bounded_text(&input.name, "Client name", 200)?;
    let status = input
        .status
        .unwrap_or_else(|| "active".into())
        .trim()
        .to_lowercase();
    if !matches!(status.as_str(), "active" | "prospect" | "inactive") {
        return Err("Client status must be active, prospect, or inactive.".into());
    }
    let client = Client {
        id: Uuid::new_v4().to_string(),
        name,
        industry: optional_bounded_text(input.industry, "Industry", 120)?,
        status,
        owner: optional_bounded_text(input.owner, "Owner", 160)?,
        created_at: db::now_ms(),
    };
    let connection = db::connect(&paths.database).map_err(map_error)?;
    connection.execute(
        "INSERT INTO clients (id, name, industry, status, owner, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![client.id, client.name, client.industry, client.status, client.owner, client.created_at],
    ).map_err(map_error)?;
    db::append_audit(
        &connection,
        "user",
        "create_client",
        Some(&client.id),
        Some("Created a local business client record."),
    )
    .map_err(map_error)?;
    Ok(client)
}

/// Edits an existing local client using the same bounded field validation as
/// creation. Record IDs remain stable so historical provenance keeps pointing
/// at the same source record.
#[tauri::command]
pub fn update_client(
    paths: State<'_, AppPaths>,
    client_id: String,
    input: NewClient,
) -> Result<Client, String> {
    if client_id.trim().is_empty() || client_id.len() > 128 {
        return Err("A valid local client selection is required.".into());
    }
    let name = bounded_text(&input.name, "Client name", 200)?;
    let status = input
        .status
        .unwrap_or_else(|| "active".into())
        .trim()
        .to_lowercase();
    if !matches!(status.as_str(), "active" | "prospect" | "inactive") {
        return Err("Client status must be active, prospect, or inactive.".into());
    }
    let client = Client {
        id: client_id.trim().to_string(),
        name,
        industry: optional_bounded_text(input.industry, "Industry", 120)?,
        status,
        owner: optional_bounded_text(input.owner, "Owner", 160)?,
        // Creation time is immutable, and is fetched below rather than
        // renderer supplied.
        created_at: 0,
    };
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let created_at: i64 = connection
        .query_row(
            "SELECT created_at FROM clients WHERE id = ?1",
            params![&client.id],
            |row| row.get(0),
        )
        .map_err(|_| "Client was not found.")?;
    connection
        .execute(
            "UPDATE clients SET name = ?1, industry = ?2, status = ?3, owner = ?4 WHERE id = ?5",
            params![
                &client.name,
                &client.industry,
                &client.status,
                &client.owner,
                &client.id
            ],
        )
        .map_err(map_error)?;
    db::append_audit(
        &connection,
        "user",
        "update_client",
        Some(&client.id),
        Some("Updated a local business client record."),
    )
    .map_err(map_error)?;
    Ok(Client {
        created_at,
        ..client
    })
}

#[tauri::command]
pub fn list_projects(paths: State<'_, AppPaths>) -> Result<Vec<Project>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT p.id, p.client_id, c.name, p.name, p.stage, p.value, p.risk_flag, p.created_at FROM projects p JOIN clients c ON c.id = p.client_id ORDER BY p.value DESC, p.name").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                client_id: row.get(1)?,
                client_name: row.get(2)?,
                name: row.get(3)?,
                stage: row.get(4)?,
                value: row.get(5)?,
                risk_flag: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn create_project(paths: State<'_, AppPaths>, input: NewProject) -> Result<Project, String> {
    let client_id = input.client_id.trim().to_string();
    if client_id.is_empty() || client_id.len() > 128 {
        return Err("A valid local client selection is required.".into());
    }
    let name = bounded_text(&input.name, "Project name", 240)?;
    if !input.value.is_finite() || input.value < 0.0 || input.value > 1_000_000_000_000.0 {
        return Err("Project value must be a finite non-negative business amount.".into());
    }
    let stage = optional_bounded_text(input.stage, "Project stage", 120)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let client_name: String = connection
        .query_row(
            "SELECT name FROM clients WHERE id = ?1",
            params![&client_id],
            |row| row.get(0),
        )
        .map_err(|_| "Selected client was not found.")?;
    let project = Project {
        id: Uuid::new_v4().to_string(),
        client_id,
        client_name,
        name,
        stage,
        value: input.value,
        risk_flag: input.risk_flag,
        created_at: db::now_ms(),
    };
    connection.execute(
        "INSERT INTO projects (id, client_id, name, stage, value, risk_flag, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![project.id, project.client_id, project.name, project.stage, project.value, if project.risk_flag { 1 } else { 0 }, project.created_at],
    ).map_err(map_error)?;
    db::append_audit(
        &connection,
        "user",
        "create_project",
        Some(&project.id),
        Some("Created a local project record linked to the selected client."),
    )
    .map_err(map_error)?;
    Ok(project)
}

/// Imports a bounded, UTF-8 project CSV into one already-selected local
/// client. The whole upload is validated before a transaction writes any row,
/// so an invalid row never produces an ambiguous partial import.
#[tauri::command]
pub fn import_projects_csv(
    paths: State<'_, AppPaths>,
    input: ImportProjectsCsvRequest,
) -> Result<Vec<Project>, String> {
    const MAX_CSV_BYTES: usize = 1024 * 1024;
    const MAX_ROWS: usize = 500;
    if input.csv_text.is_empty() || input.csv_text.len() > MAX_CSV_BYTES {
        return Err("Project CSV must contain 1 byte to 1 MiB of UTF-8 text.".into());
    }
    let client_id = input.client_id.trim();
    if client_id.is_empty() || client_id.len() > 128 {
        return Err("Select a valid local client before importing projects.".into());
    }
    let records = parse_csv_records(&input.csv_text)?;
    if records.len() < 2 {
        return Err("Project CSV needs one header row and at least one project row.".into());
    }
    if records.len() - 1 > MAX_ROWS {
        return Err("Project CSV may import at most 500 project rows at once.".into());
    }
    let headers = &records[0];
    let expected_headers = ["name", "stage", "value", "risk_flag"];
    if headers.len() != expected_headers.len()
        || headers
            .iter()
            .zip(expected_headers)
            .any(|(actual, expected)| actual.trim().to_ascii_lowercase() != expected)
    {
        return Err("Project CSV header must be exactly: name,stage,value,risk_flag".into());
    }
    let mut parsed = Vec::with_capacity(records.len() - 1);
    for (index, row) in records.iter().skip(1).enumerate() {
        if row.len() != expected_headers.len() {
            return Err(format!(
                "CSV row {} must contain exactly four values.",
                index + 2
            ));
        }
        if row
            .iter()
            .any(|cell| cell.chars().any(|character| character.is_control()))
        {
            return Err(format!(
                "CSV row {} contains unsupported control characters.",
                index + 2
            ));
        }
        let name = bounded_text(&row[0], "Project name", 240)
            .map_err(|_| format!("CSV row {} has an invalid project name.", index + 2))?;
        let stage = optional_bounded_text(Some(row[1].clone()), "Project stage", 120)
            .map_err(|_| format!("CSV row {} has an invalid project stage.", index + 2))?;
        let value = row[2]
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("CSV row {} has an invalid project value.", index + 2))?;
        if !value.is_finite() || !(0.0..=1_000_000_000_000.0).contains(&value) {
            return Err(format!(
                "CSV row {} has an out-of-range project value.",
                index + 2
            ));
        }
        let risk_flag = match row[3].trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => true,
            "0" | "false" | "no" => false,
            _ => {
                return Err(format!(
                    "CSV row {} risk_flag must be true/false, yes/no, or 1/0.",
                    index + 2
                ))
            }
        };
        parsed.push((name, stage, value, risk_flag));
    }
    let created_at = db::now_ms();
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let client_name: String = transaction
        .query_row(
            "SELECT name FROM clients WHERE id = ?1",
            params![client_id],
            |row| row.get(0),
        )
        .map_err(|_| "Selected client was not found.")?;
    let mut projects = Vec::with_capacity(parsed.len());
    for (name, stage, value, risk_flag) in parsed {
        let project = Project {
            id: Uuid::new_v4().to_string(),
            client_id: client_id.to_string(),
            client_name: client_name.clone(),
            name,
            stage,
            value,
            risk_flag,
            created_at,
        };
        transaction.execute(
            "INSERT INTO projects (id, client_id, name, stage, value, risk_flag, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![project.id, project.client_id, project.name, project.stage, project.value, if project.risk_flag { 1 } else { 0 }, project.created_at],
        ).map_err(map_error)?;
        projects.push(project);
    }
    transaction.execute(
        "INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('user', 'import_projects_csv', ?1, ?2, ?3)",
        params![client_id, format!("Imported {} validated project record(s) from a local CSV.", projects.len()), created_at],
    ).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(projects)
}

/// Updates a project while preserving its stable record ID and source history.
#[tauri::command]
pub fn update_project(
    paths: State<'_, AppPaths>,
    project_id: String,
    input: NewProject,
) -> Result<Project, String> {
    if project_id.trim().is_empty() || project_id.len() > 128 {
        return Err("A valid local project selection is required.".into());
    }
    let client_id = input.client_id.trim().to_string();
    if client_id.is_empty() || client_id.len() > 128 {
        return Err("A valid local client selection is required.".into());
    }
    let name = bounded_text(&input.name, "Project name", 240)?;
    if !input.value.is_finite() || input.value < 0.0 || input.value > 1_000_000_000_000.0 {
        return Err("Project value must be a finite non-negative business amount.".into());
    }
    let stage = optional_bounded_text(input.stage, "Project stage", 120)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let client_name: String = connection
        .query_row(
            "SELECT name FROM clients WHERE id = ?1",
            params![&client_id],
            |row| row.get(0),
        )
        .map_err(|_| "Selected client was not found.")?;
    let created_at: i64 = connection
        .query_row(
            "SELECT created_at FROM projects WHERE id = ?1",
            params![project_id.trim()],
            |row| row.get(0),
        )
        .map_err(|_| "Project was not found.")?;
    connection.execute(
        "UPDATE projects SET client_id = ?1, name = ?2, stage = ?3, value = ?4, risk_flag = ?5 WHERE id = ?6",
        params![&client_id, &name, &stage, input.value, if input.risk_flag { 1 } else { 0 }, project_id.trim()],
    ).map_err(map_error)?;
    db::append_audit(
        &connection,
        "user",
        "update_project",
        Some(project_id.trim()),
        Some("Updated a local project record."),
    )
    .map_err(map_error)?;
    Ok(Project {
        id: project_id.trim().to_string(),
        client_id,
        client_name,
        name,
        stage,
        value: input.value,
        risk_flag: input.risk_flag,
        created_at,
    })
}

#[tauri::command]
pub fn list_agents(paths: State<'_, AppPaths>) -> Result<Vec<Agent>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, name, role, agent_kind, capabilities, max_concurrency, status FROM agents ORDER BY CASE agent_kind WHEN 'manager' THEN 0 WHEN 'analyst' THEN 1 WHEN 'worker' THEN 2 ELSE 3 END, name").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            let capabilities: String = row.get(4)?;
            Ok(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                agent_kind: row.get(3)?,
                capabilities: serde_json::from_str(&capabilities).unwrap_or_default(),
                max_concurrency: row.get(5)?,
                status: row.get(6)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn list_requirements(paths: State<'_, AppPaths>) -> Result<Vec<Requirement>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, text, status, deliverable_type, created_at FROM requirements ORDER BY created_at DESC LIMIT 100").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Requirement {
                id: row.get(0)?,
                text: row.get(1)?,
                status: row.get(2)?,
                deliverable_type: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn create_requirement(
    paths: State<'_, AppPaths>,
    text: String,
    deliverable_type: Option<String>,
    requirement_id: Option<String>,
) -> Result<Requirement, String> {
    let text = text.trim().to_string();
    if text.is_empty() || text.len() > 4_000 {
        return Err("Requirement text must be between 1 and 4,000 characters.".into());
    }
    if let Some(file_type) = deliverable_type.as_deref() {
        if !supported_type(file_type) {
            return Err("Unsupported deliverable type.".into());
        }
    }
    let requirement = Requirement {
        id: requirement_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        text,
        status: "planning".into(),
        deliverable_type,
        created_at: db::now_ms(),
    };
    let connection = db::connect(&paths.database).map_err(map_error)?;
    connection.execute("INSERT INTO requirements (id, text, status, deliverable_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![requirement.id, requirement.text, requirement.status, requirement.deliverable_type, requirement.created_at]).map_err(map_error)?;
    db::append_audit(
        &connection,
        "user",
        "create_requirement",
        Some(&requirement.id),
        Some("Created a business requirement from desktop UI."),
    )
    .map_err(map_error)?;
    Ok(requirement)
}

#[tauri::command]
pub fn update_requirement_status(
    paths: State<'_, AppPaths>,
    requirement_id: String,
    status: String,
) -> Result<(), String> {
    if !matches!(status.as_str(), "planning" | "running" | "done" | "failed") {
        return Err("Unsupported requirement status.".into());
    }
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let changed = connection
        .execute(
            "UPDATE requirements SET status = ?1 WHERE id = ?2",
            params![status, requirement_id],
        )
        .map_err(map_error)?;
    if changed != 1 {
        return Err("Requirement was not found.".into());
    }
    db::append_audit(
        &connection,
        "system",
        "update_requirement_status",
        Some(&requirement_id),
        Some("Updated governed mission lifecycle state."),
    )
    .map_err(map_error)
}

#[tauri::command]
pub fn get_context_for_mission(
    paths: State<'_, AppPaths>,
    selector: MissionSelector,
    actor_id: String,
    purpose: String,
) -> Result<MissionContext, String> {
    if purpose.trim().is_empty() || purpose.len() > 500 {
        return Err("A concise context purpose is required.".into());
    }
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let client = match selector.client_name.as_deref() {
        Some(name) => connection.query_row("SELECT id, name, industry, status, owner, created_at FROM clients WHERE name = ?1 COLLATE NOCASE LIMIT 1", params![name.trim()], |row| Ok(Client { id: row.get(0)?, name: row.get(1)?, industry: row.get(2)?, status: row.get(3)?, owner: row.get(4)?, created_at: row.get(5)? })).optional().map_err(map_error)?,
        None => None,
    };
    if let Some(client) = client.as_ref() {
        db::append_audit(
            &connection,
            &actor_id,
            "query_clients",
            Some(&client.id),
            Some(&purpose),
        )
        .map_err(map_error)?;
    }
    let projects = if selector.include_projects {
        if let Some(client) = client.as_ref() {
            let mut statement = connection.prepare("SELECT p.id, p.client_id, c.name, p.name, p.stage, p.value, p.risk_flag, p.created_at FROM projects p JOIN clients c ON c.id = p.client_id WHERE p.client_id = ?1 AND (?2 IS NULL OR p.stage = ?2) ORDER BY p.value DESC").map_err(map_error)?;
            let rows = statement
                .query_map(params![client.id, selector.project_status], |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        client_id: row.get(1)?,
                        client_name: row.get(2)?,
                        name: row.get(3)?,
                        stage: row.get(4)?,
                        value: row.get(5)?,
                        risk_flag: row.get::<_, i64>(6)? != 0,
                        created_at: row.get(7)?,
                    })
                })
                .map_err(map_error)?;
            let projects = rows.collect::<Result<Vec<_>, _>>().map_err(map_error)?;
            let detail = format!(
                "{} scoped project record(s) for {}",
                projects.len(),
                purpose
            );
            db::append_audit(
                &connection,
                &actor_id,
                "query_projects",
                Some(&client.id),
                Some(&detail),
            )
            .map_err(map_error)?;
            projects
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    Ok(MissionContext { client, projects })
}

#[tauri::command]
pub fn create_agent_run(
    paths: State<'_, AppPaths>,
    task_id: String,
    agent_id: String,
    parent_run_id: Option<String>,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = db::now_ms();
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let agent_status: String = transaction
        .query_row(
            "SELECT status FROM agents WHERE id = ?1",
            params![agent_id],
            |row| row.get(0),
        )
        .map_err(map_error)?;
    if agent_status != "idle" {
        return Err("Selected agent is not available.".into());
    }
    let task_status: String = transaction
        .query_row(
            "SELECT status FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(map_error)?;
    if task_status != "ready" {
        return Err("Task is not ready for an agent run.".into());
    }
    transaction
        .execute(
            "UPDATE agents SET status = 'busy' WHERE id = ?1 AND status = 'idle'",
            params![agent_id],
        )
        .map_err(map_error)?;
    transaction.execute("UPDATE tasks SET assignee_id = ?1, status = 'in_progress', attempts = attempts + 1, updated_at = ?2 WHERE id = ?3", params![agent_id, created_at, task_id]).map_err(map_error)?;
    transaction.execute("INSERT INTO agent_runs (id, task_id, agent_id, parent_run_id, status, started_at, created_at) VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5)", params![id, task_id, agent_id, parent_run_id, created_at]).map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES (?1, 'start_agent_run', ?2, 'Atomically claimed ready task for named agent.', ?3)", params![agent_id, task_id, created_at]).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(id)
}

#[tauri::command]
pub fn complete_agent_run(
    paths: State<'_, AppPaths>,
    run_id: String,
    status: String,
    output_summary: Option<String>,
) -> Result<(), String> {
    if !matches!(status.as_str(), "succeeded" | "failed" | "cancelled") {
        return Err("Agent run status must be succeeded, failed, or cancelled.".into());
    }
    let completed_at = db::now_ms();
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let (task_id, agent_id): (String, String) = transaction
        .query_row(
            "SELECT task_id, agent_id FROM agent_runs WHERE id = ?1",
            params![run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(map_error)?;
    transaction.execute("UPDATE agent_runs SET status = ?1, output_summary = ?2, completed_at = ?3 WHERE id = ?4", params![status, output_summary, completed_at, run_id]).map_err(map_error)?;
    transaction
        .execute(
            "UPDATE agents SET status = 'idle' WHERE id = ?1",
            params![agent_id],
        )
        .map_err(map_error)?;
    let task_status = if status == "succeeded" {
        "review"
    } else {
        "failed"
    };
    transaction
        .execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![task_status, completed_at, task_id],
        )
        .map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES (?1, 'complete_agent_run', ?2, ?3, ?4)", params![agent_id, task_id, status, completed_at]).map_err(map_error)?;
    transaction.commit().map_err(map_error)
}

#[tauri::command]
pub fn set_agent_run_session(
    paths: State<'_, AppPaths>,
    run_id: String,
    session_id: String,
) -> Result<(), String> {
    if session_id.trim().is_empty() || session_id.len() > 300 {
        return Err("A valid OpenCode session ID is required.".into());
    }
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let changed = connection
        .execute(
            "UPDATE agent_runs SET opencode_session_id = ?1 WHERE id = ?2",
            params![session_id.trim(), run_id],
        )
        .map_err(map_error)?;
    if changed != 1 {
        return Err("Agent run was not found.".into());
    }
    db::append_audit(
        &connection,
        "system",
        "attach_opencode_session",
        Some(&run_id),
        Some("Attached loopback OpenCode session to governed agent run."),
    )
    .map_err(map_error)
}

#[tauri::command]
pub fn record_agent_handoff(
    paths: State<'_, AppPaths>,
    requirement_id: String,
    task_id: String,
    from_run_id: String,
    to_task_id: Option<String>,
    to_agent_id: Option<String>,
    handoff_type: String,
    payload_json: String,
    source_refs_json: String,
) -> Result<String, String> {
    if !matches!(
        handoff_type.as_str(),
        "market_analysis" | "pipeline_metrics" | "operations_metrics"
    ) {
        return Err("Unsupported handoff type.".into());
    }
    if payload_json.len() > 32_000 || source_refs_json.len() > 8_000 {
        return Err("Handoff payload exceeds policy limits.".into());
    }
    let payload: serde_json::Value =
        serde_json::from_str(&payload_json).map_err(|_| "Handoff payload must be valid JSON.")?;
    let sources: Vec<String> = serde_json::from_str(&source_refs_json)
        .map_err(|_| "Handoff sources must be a JSON string array.")?;
    if !payload.is_object() || sources.is_empty() {
        return Err(
            "Handoff requires an object payload and at least one source record reference.".into(),
        );
    }

    let id = Uuid::new_v4().to_string();
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let run_task: String = transaction
        .query_row(
            "SELECT task_id FROM agent_runs WHERE id = ?1",
            params![from_run_id],
            |row| row.get(0),
        )
        .map_err(|_| "The handoff run was not found.")?;
    if run_task != task_id {
        return Err("A handoff run must belong to its declared source task.".into());
    }
    let task_requirement: String = transaction
        .query_row(
            "SELECT requirement_id FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|_| "The handoff source task was not found.")?;
    if task_requirement != requirement_id {
        return Err("A handoff must remain within its declared requirement.".into());
    }
    if let Some(target_task) = to_task_id.as_deref() {
        let target_requirement: String = transaction
            .query_row(
                "SELECT requirement_id FROM tasks WHERE id = ?1",
                params![target_task],
                |row| row.get(0),
            )
            .map_err(|_| "The handoff target task was not found.")?;
        if target_requirement != requirement_id {
            return Err("A handoff target must belong to the same requirement.".into());
        }
    }
    if let Some(target_agent) = to_agent_id.as_deref() {
        let agents: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM agents WHERE id = ?1",
                params![target_agent],
                |row| row.get(0),
            )
            .map_err(map_error)?;
        if agents != 1 {
            return Err("The handoff target agent was not found.".into());
        }
    }
    transaction.execute(
        "INSERT INTO agent_handoffs (id, requirement_id, task_id, from_run_id, to_task_id, to_agent_id, handoff_type, payload_json, source_refs_json, validation_status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'accepted', ?10)",
        params![id, requirement_id, task_id, from_run_id, to_task_id, to_agent_id, handoff_type, payload_json, source_refs_json, db::now_ms()],
    ).map_err(map_error)?;
    db::append_audit(
        &transaction,
        "system",
        "create_handoff",
        Some(&id),
        Some("Stored accepted, source-linked structured handoff."),
    )
    .map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(id)
}

#[tauri::command]
pub fn register_deliverable(
    paths: State<'_, AppPaths>,
    task_id: String,
    file_name: String,
    file_type: String,
) -> Result<Deliverable, String> {
    let safe_name = files::safe_file_name(&file_name, &file_type).map_err(map_error)?;
    let report_path = paths.report_path(&safe_name).map_err(map_error)?;
    files::validate_nonempty_report(&report_path, &file_type).map_err(map_error)?;
    let canonical = files::canonical_inside(&paths.reports, &report_path).map_err(map_error)?;
    let relative = files::relative_display_path(&paths.reports, &canonical).map_err(map_error)?;
    let deliverable = Deliverable {
        id: Uuid::new_v4().to_string(),
        task_id,
        file_path: relative,
        file_name: safe_name,
        file_type,
        created_at: db::now_ms(),
    };
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    transaction.execute("INSERT INTO deliverables (id, task_id, file_path, file_name, file_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![deliverable.id, deliverable.task_id, deliverable.file_path, deliverable.file_name, deliverable.file_type, deliverable.created_at]).map_err(map_error)?;
    transaction
        .execute(
            "UPDATE tasks SET output_path = ?1, status = 'review', updated_at = ?2 WHERE id = ?3",
            params![
                deliverable.file_path,
                deliverable.created_at,
                deliverable.task_id
            ],
        )
        .map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('agent-worker-2', 'create_file', ?1, ?2, ?3)", params![deliverable.id, deliverable.file_path, deliverable.created_at]).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(deliverable)
}

#[tauri::command]
pub fn list_deliverables(paths: State<'_, AppPaths>) -> Result<Vec<Deliverable>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, task_id, file_path, file_name, file_type, created_at FROM deliverables ORDER BY created_at DESC LIMIT 200").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Deliverable {
                id: row.get(0)?,
                task_id: row.get(1)?,
                file_path: row.get(2)?,
                file_name: row.get(3)?,
                file_type: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

/// Adds a registered artifact to the local human-approval queue. This is a
/// decision record only: it never sends a file or accepts a remote recipient.
#[tauri::command]
pub fn request_deliverable_approval(
    paths: State<'_, AppPaths>,
    deliverable_id: String,
) -> Result<DeliverableApproval, String> {
    Uuid::parse_str(deliverable_id.trim())
        .map_err(|_| "A valid registered deliverable ID is required.")?;
    let requested_at = db::now_ms();
    let approval = DeliverableApproval {
        id: Uuid::new_v4().to_string(),
        deliverable_id: deliverable_id.trim().to_string(),
        file_name: String::new(),
        file_type: String::new(),
        status: "pending".into(),
        requested_by: "local_user".into(),
        decision_note: None,
        requested_at,
        decided_at: None,
    };
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let (file_name, file_type): (String, String) = transaction
        .query_row(
            "SELECT file_name, file_type FROM deliverables WHERE id = ?1",
            params![&approval.deliverable_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Registered deliverable was not found.")?;
    let pending: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM deliverable_approvals WHERE deliverable_id = ?1 AND status = 'pending'",
            params![&approval.deliverable_id],
            |row| row.get(0),
        )
        .map_err(map_error)?;
    if pending > 0 {
        return Err("This registered deliverable is already awaiting a human decision.".into());
    }
    transaction.execute(
        "INSERT INTO deliverable_approvals (id, deliverable_id, status, requested_by, requested_at) VALUES (?1, ?2, 'pending', 'local_user', ?3)",
        params![&approval.id, &approval.deliverable_id, requested_at],
    ).map_err(map_error)?;
    transaction.execute(
        "INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('local_user', 'request_deliverable_approval', ?1, 'Queued a registered artifact for local human approval.', ?2)",
        params![&approval.deliverable_id, requested_at],
    ).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(DeliverableApproval {
        file_name,
        file_type,
        ..approval
    })
}

#[tauri::command]
pub fn list_deliverable_approvals(
    paths: State<'_, AppPaths>,
) -> Result<Vec<DeliverableApproval>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT a.id, a.deliverable_id, d.file_name, d.file_type, a.status, a.requested_by, a.decision_note, a.requested_at, a.decided_at FROM deliverable_approvals a JOIN deliverables d ON d.id = a.deliverable_id ORDER BY CASE a.status WHEN 'pending' THEN 0 ELSE 1 END, a.requested_at DESC LIMIT 200").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(DeliverableApproval {
                id: row.get(0)?,
                deliverable_id: row.get(1)?,
                file_name: row.get(2)?,
                file_type: row.get(3)?,
                status: row.get(4)?,
                requested_by: row.get(5)?,
                decision_note: row.get(6)?,
                requested_at: row.get(7)?,
                decided_at: row.get(8)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn decide_deliverable_approval(
    paths: State<'_, AppPaths>,
    approval_id: String,
    input: ApprovalDecision,
) -> Result<DeliverableApproval, String> {
    if !matches!(input.decision.as_str(), "approved" | "rejected") {
        return Err("Approval decision must be approved or rejected.".into());
    }
    let note = optional_bounded_text(input.note, "Decision note", 1_000)?;
    if input.decision == "rejected" && note.is_none() {
        return Err("A rejection requires a concise reason for the document worker.".into());
    }
    let decided_at = db::now_ms();
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let mut approval = transaction.query_row(
        "SELECT a.id, a.deliverable_id, d.file_name, d.file_type, a.status, a.requested_by, a.decision_note, a.requested_at, a.decided_at FROM deliverable_approvals a JOIN deliverables d ON d.id = a.deliverable_id WHERE a.id = ?1",
        params![approval_id.trim()],
        |row| Ok(DeliverableApproval {
            id: row.get(0)?, deliverable_id: row.get(1)?, file_name: row.get(2)?, file_type: row.get(3)?,
            status: row.get(4)?, requested_by: row.get(5)?, decision_note: row.get(6)?,
            requested_at: row.get(7)?, decided_at: row.get(8)?,
        }),
    ).map_err(|_| "Approval request was not found.")?;
    if approval.status != "pending" {
        return Err("A human decision has already been recorded for this request.".into());
    }
    transaction.execute(
        "UPDATE deliverable_approvals SET status = ?1, decision_note = ?2, decided_at = ?3 WHERE id = ?4 AND status = 'pending'",
        params![input.decision, note, decided_at, approval_id.trim()],
    ).map_err(map_error)?;
    transaction.execute(
        "INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('local_user', 'decide_deliverable_approval', ?1, ?2, ?3)",
        params![approval_id.trim(), format!("Local human {} registered artifact {}.", input.decision, approval.deliverable_id), decided_at],
    ).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    approval.status = input.decision;
    approval.decision_note = note;
    approval.decided_at = Some(decided_at);
    Ok(approval)
}

/// Sends a registered artifact only after a separate human approval was
/// recorded. The renderer can choose a channel and recipient, but credentials
/// are read only from opt-in desktop-host environment variables.
#[tauri::command]
pub fn send_approved_delivery(
    paths: State<'_, AppPaths>,
    input: SendApprovedDeliveryRequest,
) -> Result<DeliveryRecord, String> {
    Uuid::parse_str(input.deliverable_id.trim())
        .map_err(|_| "A valid registered deliverable ID is required.")?;
    if !matches!(input.channel.as_str(), "email_smtp" | "slack_upload") {
        return Err("Choose the approved SMTP email or Slack upload delivery channel.".into());
    }
    let recipient = bounded_text(&input.recipient, "Delivery recipient", 320)?;
    if recipient.chars().any(|character| character.is_control()) {
        return Err("Delivery recipient contains unsupported control characters.".into());
    }
    if input.channel == "slack_upload" && !valid_slack_channel(&recipient) {
        return Err(
            "Slack recipient must be an approved channel ID beginning with C, G, or D.".into(),
        );
    }
    let message = input
        .message
        .map(|value| bounded_text(&value, "Delivery message", 2_000))
        .transpose()?
        .unwrap_or_else(|| "Approved KUBERA artifact attached for your review.".into());
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let latest_approval: Option<String> = connection
        .query_row(
            "SELECT status FROM deliverable_approvals WHERE deliverable_id = ?1 ORDER BY requested_at DESC LIMIT 1",
            params![input.deliverable_id.trim()],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_error)?;
    if latest_approval.as_deref() != Some("approved") {
        return Err("A current local human approval is required before external delivery.".into());
    }
    drop(connection);
    let (file_name, path) = registered_deliverable_path(&paths, input.deliverable_id.trim())?;
    let bytes = fs::read(&path).map_err(|_| "Approved artifact could not be read for delivery.")?;
    if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
        return Err(
            "Approved artifact must be non-empty and no larger than 8 MiB for delivery.".into(),
        );
    }
    let requested_at = db::now_ms();
    let mut delivery = DeliveryRecord {
        id: Uuid::new_v4().to_string(),
        deliverable_id: input.deliverable_id.trim().to_string(),
        file_name: file_name.clone(),
        channel: input.channel.clone(),
        recipient: recipient.clone(),
        status: "sending".into(),
        requested_at,
        completed_at: None,
    };
    let connection = db::connect(&paths.database).map_err(map_error)?;
    connection.execute(
        "INSERT INTO deliveries (id, deliverable_id, channel, recipient, status, requested_at) VALUES (?1, ?2, ?3, ?4, 'sending', ?5)",
        params![delivery.id, delivery.deliverable_id, delivery.channel, delivery.recipient, requested_at],
    ).map_err(map_error)?;
    db::append_audit(
        &connection,
        "local_user",
        "request_approved_delivery",
        Some(&delivery.id),
        Some(
            "User explicitly requested external delivery of a human-approved registered artifact.",
        ),
    )
    .map_err(map_error)?;
    drop(connection);
    let send_result = match input.channel.as_str() {
        "email_smtp" => send_smtp_attachment(&recipient, &file_name, &bytes, &message),
        "slack_upload" => send_slack_attachment(&recipient, &file_name, &bytes, &message),
        _ => unreachable!(),
    };
    let completed_at = db::now_ms();
    let connection = db::connect(&paths.database).map_err(map_error)?;
    match send_result {
        Ok(()) => {
            connection
                .execute(
                    "UPDATE deliveries SET status = 'sent', completed_at = ?1 WHERE id = ?2",
                    params![completed_at, &delivery.id],
                )
                .map_err(map_error)?;
            db::append_audit(
                &connection,
                "system",
                "complete_approved_delivery",
                Some(&delivery.id),
                Some(
                    "Approved artifact delivery completed through the selected configured channel.",
                ),
            )
            .map_err(map_error)?;
            delivery.status = "sent".into();
            delivery.completed_at = Some(completed_at);
            Ok(delivery)
        }
        Err(error) => {
            connection
                .execute(
                    "UPDATE deliveries SET status = 'failed', completed_at = ?1 WHERE id = ?2",
                    params![completed_at, &delivery.id],
                )
                .map_err(map_error)?;
            db::append_audit(&connection, "system", "fail_approved_delivery", Some(&delivery.id), Some("Approved artifact delivery failed without recording provider credentials or diagnostics.")).map_err(map_error)?;
            Err(error)
        }
    }
}

#[tauri::command]
pub fn list_deliveries(paths: State<'_, AppPaths>) -> Result<Vec<DeliveryRecord>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT d.id, d.deliverable_id, r.file_name, d.channel, d.recipient, d.status, d.requested_at, d.completed_at FROM deliveries d JOIN deliverables r ON r.id = d.deliverable_id ORDER BY d.requested_at DESC LIMIT 200").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(DeliveryRecord {
                id: row.get(0)?,
                deliverable_id: row.get(1)?,
                file_name: row.get(2)?,
                channel: row.get(3)?,
                recipient: row.get(4)?,
                status: row.get(5)?,
                requested_at: row.get(6)?,
                completed_at: row.get(7)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

fn required_delivery_env(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} is not configured on this desktop host."))
}

fn valid_smtp_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
        && !value.starts_with('.')
        && !value.ends_with('.')
}

fn send_smtp_attachment(
    recipient: &str,
    file_name: &str,
    bytes: &[u8],
    message: &str,
) -> Result<(), String> {
    let host = required_delivery_env("KUBERA_SMTP_HOST")?;
    if !valid_smtp_host(&host) {
        return Err("KUBERA_SMTP_HOST is not a valid SMTP host name.".into());
    }
    let port = std::env::var("KUBERA_SMTP_PORT")
        .ok()
        .map(|value| value.parse::<u16>().ok())
        .flatten()
        .filter(|value| *value > 0)
        .unwrap_or(587);
    let username = required_delivery_env("KUBERA_SMTP_USERNAME")?;
    let password = required_delivery_env("KUBERA_SMTP_PASSWORD")?;
    let from: Mailbox = required_delivery_env("KUBERA_SMTP_FROM").and_then(|value| {
        value
            .parse()
            .map_err(|_| "KUBERA_SMTP_FROM is not a valid email mailbox.".to_string())
    })?;
    let to: Mailbox = recipient
        .parse()
        .map_err(|_| "Email recipient is not a valid email mailbox.".to_string())?;
    let content_type = "application/octet-stream"
        .parse()
        .map_err(|_| "Could not prepare the approved attachment type.".to_string())?;
    let attachment = Attachment::new(file_name.to_string()).body(bytes.to_vec(), content_type);
    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(format!("KUBERA approved artifact: {file_name}"))
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(message.to_string()))
                .singlepart(attachment),
        )
        .map_err(|_| "Could not prepare the approved email delivery.")?;
    let transport = SmtpTransport::starttls_relay(&host)
        .map_err(|_| "Could not configure the SMTP delivery transport.")?
        .port(port)
        .credentials(Credentials::new(username, password))
        .build();
    transport
        .send(&email)
        .map_err(|_| "SMTP delivery did not complete. Check the host credentials and recipient.")?;
    Ok(())
}

fn valid_slack_channel(value: &str) -> bool {
    (9..=32).contains(&value.len())
        && matches!(value.as_bytes().first().copied(), Some(b'C' | b'G' | b'D'))
        && value
            .bytes()
            .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
}

fn valid_slack_upload_url(value: &str) -> bool {
    reqwest::Url::parse(value).ok().is_some_and(|url| {
        url.scheme() == "https"
            && url
                .host_str()
                .is_some_and(|host| host == "slack.com" || host.ends_with(".slack.com"))
    })
}

fn slack_api_response(response: reqwest::blocking::Response) -> Result<serde_json::Value, String> {
    const MAX_SLACK_RESPONSE_BYTES: u64 = 128 * 1024;
    if !response.status().is_success() {
        return Err("Slack API did not accept the approved delivery request.".into());
    }
    let mut payload = Vec::new();
    response
        .take(MAX_SLACK_RESPONSE_BYTES + 1)
        .read_to_end(&mut payload)
        .map_err(|_| "Could not read the Slack API response.")?;
    if payload.len() as u64 > MAX_SLACK_RESPONSE_BYTES {
        return Err("Slack API response exceeded KUBERA's safety limit.".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&payload).map_err(|_| "Slack API returned an invalid response.")?;
    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err("Slack API rejected the approved delivery request.");
    }
    Ok(value)
}

fn send_slack_attachment(
    channel: &str,
    file_name: &str,
    bytes: &[u8],
    message: &str,
) -> Result<(), String> {
    let token = required_delivery_env("KUBERA_SLACK_BOT_TOKEN")?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("KUBERA-AI/0.1 approved-delivery")
        .build()
        .map_err(|_| "Could not configure the Slack delivery client.")?;
    let length = bytes.len().to_string();
    let request = client
        .post("https://slack.com/api/files.getUploadURLExternal")
        .bearer_auth(&token)
        .form(&[("filename", file_name), ("length", length.as_str())]);
    let upload_info = slack_api_response(
        request
            .send()
            .map_err(|_| "Could not reach Slack for approved delivery.")?,
    )?;
    let upload_url = upload_info
        .get("upload_url")
        .and_then(serde_json::Value::as_str)
        .filter(|value| valid_slack_upload_url(value))
        .ok_or_else(|| "Slack returned an unsafe upload destination.".to_string())?;
    let file_id = upload_info
        .get("file_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 200)
        .ok_or_else(|| "Slack did not return a usable upload identifier.".to_string())?;
    let upload = client
        .post(upload_url)
        .body(bytes.to_vec())
        .send()
        .map_err(|_| "Could not upload the approved artifact to Slack.")?;
    if !upload.status().is_success() {
        return Err("Slack did not accept the approved artifact bytes.".into());
    }
    let files = serde_json::json!([{ "id": file_id, "title": file_name }]).to_string();
    let complete = client
        .post("https://slack.com/api/files.completeUploadExternal")
        .bearer_auth(&token)
        .form(&[
            ("files", files.as_str()),
            ("channel_id", channel),
            ("initial_comment", message),
        ]);
    slack_api_response(
        complete
            .send()
            .map_err(|_| "Could not finalize the approved Slack delivery.")?,
    )?;
    Ok(())
}

#[tauri::command]
pub fn import_template(
    paths: State<'_, AppPaths>,
    input: ImportTemplateRequest,
) -> Result<Template, String> {
    import_approved_template(
        &paths,
        &input.file_name,
        &input.file_type,
        &input.bytes,
        "import_template",
        "Copied a user-selected Office template into the KUBERA template root and inspected its placeholders.",
    )
}

/// Imports a KUBERA-owned template bundle. A bundle contains one Office file
/// and declarative placeholder metadata only; it deliberately cannot carry an
/// executable agent skill, prompt, shell command, or connector configuration.
#[tauri::command]
pub fn import_template_bundle(
    paths: State<'_, AppPaths>,
    input: ImportTemplateBundleRequest,
) -> Result<Template, String> {
    const MAX_BUNDLE_BYTES: usize = 4 * 1024 * 1024 + 64 * 1024;
    if input.bytes.is_empty() || input.bytes.len() > MAX_BUNDLE_BYTES {
        return Err("Template bundle must contain 1 byte to roughly 4 MiB of data.".into());
    }
    if !input
        .file_name
        .to_ascii_lowercase()
        .ends_with(".kubera-template.zip")
    {
        return Err("Choose a .kubera-template.zip export created by KUBERA.".into());
    }
    let (file_name, file_type, bytes, expected_placeholders) =
        unpack_template_bundle(&input.bytes)?;
    let actual_placeholders = inspect_office_template(&bytes, &file_type)?;
    if actual_placeholders != expected_placeholders {
        return Err("Template bundle placeholder metadata does not match its Office file.".into());
    }
    import_approved_template(
        &paths,
        &file_name,
        &file_type,
        &bytes,
        "import_template_bundle",
        "Imported an approved KUBERA template bundle after package and placeholder verification.",
    )
}

/// Exports an approved template as a bounded, user-downloadable interchange
/// bundle. Internal template root paths are never returned to the renderer.
#[tauri::command]
pub fn export_template_bundle(
    paths: State<'_, AppPaths>,
    template_id: String,
) -> Result<TemplateBundle, String> {
    Uuid::parse_str(template_id.trim()).map_err(|_| "A valid approved template ID is required.")?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let (name, stored_path, file_type, placeholders_json): (String, String, String, String) =
        connection
            .query_row(
                "SELECT name, file_path, file_type, placeholders_json FROM templates WHERE id = ?1",
                params![template_id.trim()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|_| "Approved template was not found.")?;
    let placeholders =
        serde_json::from_str::<Vec<String>>(&placeholders_json).map_err(map_error)?;
    let stored = Path::new(&stored_path);
    if stored.is_absolute()
        || stored
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("Approved template path failed template-root validation.".into());
    }
    let source = files::canonical_inside(&paths.templates, &paths.templates.join(stored))
        .map_err(map_error)?;
    let source_bytes = fs::read(&source).map_err(map_error)?;
    if source_bytes.is_empty() || source_bytes.len() > 4 * 1024 * 1024 {
        return Err("Approved template bytes no longer meet bundle policy.".into());
    }
    if inspect_office_template(&source_bytes, &file_type)? != placeholders {
        return Err("Approved template metadata no longer matches its stored Office file.".into());
    }
    let visible_name = stored_path
        .split_once('_')
        .map(|(_, value)| value)
        .unwrap_or(&stored_path);
    let manifest = serde_json::json!({
        "format": "kubera-template-bundle",
        "version": 1,
        "name": name,
        "file_name": visible_name,
        "file_type": file_type,
        "placeholders": placeholders,
        "contract": "office-template-v1"
    });
    let manifest = serde_json::to_vec(&manifest).map_err(map_error)?;
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut bytes);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    writer
        .start_file("kubera-template.json", options)
        .map_err(map_error)?;
    writer.write_all(&manifest).map_err(map_error)?;
    writer
        .start_file("template.bin", options)
        .map_err(map_error)?;
    writer.write_all(&source_bytes).map_err(map_error)?;
    writer.finish().map_err(map_error)?;
    // zip 0.6 retains its mutable writer borrow after finish; drop it before
    // taking the bundle bytes back out of the cursor.
    drop(writer);
    let output = bytes.into_inner();
    if output.len() > 4 * 1024 * 1024 + 64 * 1024 {
        return Err("Template bundle exceeds KUBERA's download safety limit.".into());
    }
    db::append_audit(
        &connection,
        "user",
        "export_template_bundle",
        Some(template_id.trim()),
        Some("Prepared an approved template bundle for user-initiated download."),
    )
    .map_err(map_error)?;
    Ok(TemplateBundle {
        file_name: format!("{}.kubera-template.zip", bundle_stem(visible_name)),
        bytes: output,
    })
}

fn import_approved_template(
    paths: &AppPaths,
    file_name: &str,
    file_type: &str,
    bytes: &[u8],
    audit_action: &str,
    audit_detail: &str,
) -> Result<Template, String> {
    const MAX_TEMPLATE_BYTES: usize = 4 * 1024 * 1024;
    if !matches!(file_type, "docx" | "xlsx" | "pptx") {
        return Err("Only DOCX, XLSX, and PPTX templates are supported.".into());
    }
    if bytes.is_empty() || bytes.len() > MAX_TEMPLATE_BYTES {
        return Err("Template file must contain 1 byte to 4 MiB of Office data.".into());
    }
    let safe_name = files::safe_file_name(file_name, file_type).map_err(map_error)?;
    let placeholders = inspect_office_template(bytes, file_type)?;
    let id = Uuid::new_v4().to_string();
    let stored_name = format!("{id}_{safe_name}");
    let output = paths.templates.join(&stored_name);
    fs::write(&output, bytes).map_err(map_error)?;
    let template = Template {
        id,
        name: Path::new(&safe_name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(&safe_name)
            .replace(['_', '-'], " "),
        file_name: safe_name,
        file_type: file_type.to_string(),
        placeholders,
        created_at: db::now_ms(),
    };
    let placeholders_json = serde_json::to_string(&template.placeholders).map_err(map_error)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    if let Err(error) = connection.execute(
        "INSERT INTO templates (id, name, file_path, file_type, placeholders_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![template.id, template.name, stored_name, template.file_type, placeholders_json, template.created_at],
    ) {
        let _ = fs::remove_file(&output);
        return Err(map_error(error));
    }
    db::append_audit(
        &connection,
        "user",
        audit_action,
        Some(&template.id),
        Some(audit_detail),
    )
    .map_err(map_error)?;
    Ok(template)
}

fn bundle_stem(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("approved-template")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(80)
        .collect::<String>()
}

#[tauri::command]
pub fn list_templates(paths: State<'_, AppPaths>) -> Result<Vec<Template>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, name, file_path, file_type, placeholders_json, created_at FROM templates ORDER BY created_at DESC LIMIT 100").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            let file_name: String = row.get(2)?;
            let placeholders_json: String = row.get(4)?;
            Ok(Template {
                id: row.get(0)?,
                name: row.get(1)?,
                // The on-disk UUID prefix is intentionally not sent to UI.
                file_name: file_name
                    .split_once('_')
                    .map(|(_, value)| value)
                    .unwrap_or(&file_name)
                    .to_string(),
                file_type: row.get(3)?,
                placeholders: serde_json::from_str(&placeholders_json).unwrap_or_default(),
                created_at: row.get(5)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

/// Opens only a deliverable already registered in SQLite. The caller supplies
/// an ID, never a shell command or filesystem path.
#[tauri::command]
pub fn open_deliverable(paths: State<'_, AppPaths>, deliverable_id: String) -> Result<(), String> {
    let (file_name, path) = registered_deliverable_path(&paths, &deliverable_id)?;
    open_path(&path, false)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    db::append_audit(
        &connection,
        "user",
        "open_deliverable",
        Some(&deliverable_id),
        Some(&format!(
            "Opened registered report {file_name} with the OS default application."
        )),
    )
    .map_err(map_error)
}

/// Reveals only a registered deliverable (or its report-root folder where the
/// platform has no native select-in-folder operation).
#[tauri::command]
pub fn reveal_deliverable(
    paths: State<'_, AppPaths>,
    deliverable_id: String,
) -> Result<(), String> {
    let (file_name, path) = registered_deliverable_path(&paths, &deliverable_id)?;
    open_path(&path, true)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    db::append_audit(
        &connection,
        "user",
        "reveal_deliverable",
        Some(&deliverable_id),
        Some(&format!(
            "Revealed registered report {file_name} in the operating system."
        )),
    )
    .map_err(map_error)
}

/// Produces a bounded in-app text outline from a registered local Office file.
/// The renderer receives no filesystem path, HTML, or OfficeCLI command.
#[tauri::command]
pub fn preview_deliverable(
    paths: State<'_, AppPaths>,
    deliverable_id: String,
) -> Result<DeliverablePreview, String> {
    let (file_name, path) = registered_deliverable_path(&paths, &deliverable_id)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let file_type: String = connection
        .query_row(
            "SELECT file_type FROM deliverables WHERE id = ?1",
            params![&deliverable_id],
            |row| row.get(0),
        )
        .map_err(|_| "Registered deliverable was not found.")?;
    let (outline, truncated) = office::preview_outline(&paths, &path)?;
    db::append_audit(
        &connection,
        "user",
        "preview_deliverable",
        Some(&deliverable_id),
        Some("Generated a bounded text outline for a registered Office artifact."),
    )
    .map_err(map_error)?;
    Ok(DeliverablePreview {
        deliverable_id,
        file_name,
        file_type,
        outline,
        truncated,
    })
}

/// Requests a static visual representation of a registered artifact. No source
/// path, OfficeCLI argument, or persistent HTML file reaches the renderer; the
/// UI places the result in an iframe without script or same-origin privileges.
#[tauri::command]
pub fn preview_deliverable_visual(
    paths: State<'_, AppPaths>,
    deliverable_id: String,
) -> Result<DeliverableVisualPreview, String> {
    let (file_name, path) = registered_deliverable_path(&paths, &deliverable_id)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let file_type: String = connection
        .query_row(
            "SELECT file_type FROM deliverables WHERE id = ?1",
            params![&deliverable_id],
            |row| row.get(0),
        )
        .map_err(|_| "Registered deliverable was not found.")?;
    let html = office::preview_rendered_html(&paths, &path, &Uuid::new_v4().to_string())?;
    db::append_audit(
        &connection,
        "user",
        "preview_deliverable_visual",
        Some(&deliverable_id),
        Some("Generated and removed a bounded static OfficeCLI visual preview for a registered artifact."),
    )
    .map_err(map_error)?;
    Ok(DeliverableVisualPreview {
        deliverable_id,
        file_name,
        file_type,
        html,
    })
}

fn unpack_template_bundle(bytes: &[u8]) -> Result<(String, String, Vec<u8>, Vec<String>), String> {
    const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
    const MAX_TEMPLATE_BYTES: u64 = 4 * 1024 * 1024;
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| "Template bundle must be a valid KUBERA zip export.".to_string())?;
    if archive.len() != 2 {
        return Err(
            "Template bundle must contain exactly a manifest and one Office template.".into(),
        );
    }
    let mut manifest_entry = archive
        .by_name("kubera-template.json")
        .map_err(|_| "Template bundle is missing kubera-template.json.")?;
    if manifest_entry.size() == 0 || manifest_entry.size() > MAX_MANIFEST_BYTES {
        return Err("Template bundle manifest exceeds safety limits.".into());
    }
    let mut manifest_bytes = Vec::with_capacity(manifest_entry.size() as usize);
    manifest_entry
        .read_to_end(&mut manifest_bytes)
        .map_err(map_error)?;
    // ZipArchive permits one mutable entry borrow at a time.
    drop(manifest_entry);
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| "Template bundle manifest is not valid JSON.")?;
    let object = manifest
        .as_object()
        .ok_or_else(|| "Template bundle manifest must be a JSON object.".to_string())?;
    if object.get("format").and_then(serde_json::Value::as_str) != Some("kubera-template-bundle")
        || object.get("version").and_then(serde_json::Value::as_u64) != Some(1)
        || object.get("contract").and_then(serde_json::Value::as_str) != Some("office-template-v1")
    {
        return Err("Template bundle is not a supported KUBERA template export.".into());
    }
    let file_name = object
        .get("file_name")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 180)
        .ok_or_else(|| "Template bundle has an invalid Office filename.".to_string())?
        .to_string();
    let file_type = object
        .get("file_type")
        .and_then(serde_json::Value::as_str)
        .filter(|value| matches!(*value, "docx" | "xlsx" | "pptx"))
        .ok_or_else(|| "Template bundle has an unsupported Office file type.".to_string())?
        .to_string();
    let placeholder_values = object
        .get("placeholders")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Template bundle is missing placeholder metadata.".to_string())?;
    if placeholder_values.len() > 40 {
        return Err("Template bundle has too many placeholders.".into());
    }
    let mut expected_placeholders = BTreeSet::new();
    for value in placeholder_values {
        let value = value
            .as_str()
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 80
                    && value.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
                    })
            })
            .ok_or_else(|| "Template bundle has an invalid placeholder name.".to_string())?;
        if !expected_placeholders.insert(value.to_string()) {
            return Err("Template bundle placeholder names must be unique.".into());
        }
    }
    let mut template_entry = archive
        .by_name("template.bin")
        .map_err(|_| "Template bundle is missing its Office template bytes.")?;
    if template_entry.size() == 0 || template_entry.size() > MAX_TEMPLATE_BYTES {
        return Err("Template bundle Office file exceeds safety limits.".into());
    }
    let mut template_bytes = Vec::with_capacity(template_entry.size() as usize);
    template_entry
        .read_to_end(&mut template_bytes)
        .map_err(map_error)?;
    Ok((
        file_name,
        file_type,
        template_bytes,
        expected_placeholders.into_iter().collect(),
    ))
}

fn inspect_office_template(bytes: &[u8], file_type: &str) -> Result<Vec<String>, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| "Template must be a valid Office Open XML zip package.".to_string())?;
    if archive.by_name("[Content_Types].xml").is_err() {
        return Err("Template is missing the required Office package manifest.".into());
    }
    // Placeholders are meaningful for DOCX merge. XLSX/PPTX still receive a
    // package-integrity check and are safely stored as approved templates.
    if file_type != "docx" {
        return Ok(Vec::new());
    }
    let mut placeholders = BTreeSet::new();
    let names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|entry| entry.name().to_string())
        })
        .filter(|name| name.starts_with("word/") && name.ends_with(".xml"))
        .collect::<Vec<_>>();
    if !names.iter().any(|name| name == "word/document.xml") {
        return Err("DOCX template is missing word/document.xml.".into());
    }
    for name in names {
        let mut entry = archive.by_name(&name).map_err(map_error)?;
        let mut xml = String::new();
        entry.read_to_string(&mut xml).map_err(map_error)?;
        let text = office_xml_text(&xml);
        placeholders.extend(placeholder_names(&text));
    }
    Ok(placeholders.into_iter().collect())
}

fn office_xml_text(xml: &str) -> String {
    let mut text = String::with_capacity(xml.len());
    let mut in_tag = false;
    for character in xml.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn placeholder_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("{{") {
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        let name = after_start[..end].trim();
        if !name.is_empty()
            && name.len() <= 80
            && name.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
            })
        {
            names.push(name.to_string());
        }
        remaining = &after_start[end + 2..];
    }
    names
}

fn registered_deliverable_path(
    paths: &AppPaths,
    deliverable_id: &str,
) -> Result<(String, std::path::PathBuf), String> {
    Uuid::parse_str(deliverable_id)
        .map_err(|_| "A valid registered deliverable ID is required.")?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let (file_name, relative_path): (String, String) = connection
        .query_row(
            "SELECT file_name, file_path FROM deliverables WHERE id = ?1",
            params![deliverable_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Registered deliverable was not found.")?;
    let relative = Path::new(&relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("Registered deliverable path failed report-root validation.".into());
    }
    let candidate = paths.reports.join(relative);
    let canonical = files::canonical_inside(&paths.reports, &candidate).map_err(map_error)?;
    Ok((file_name, canonical))
}

fn open_path(path: &Path, reveal: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer.exe");
        if reveal {
            command.arg(format!("/select,{}", path.display()));
        } else {
            command.arg(path);
        }
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        if reveal {
            command.arg("-R");
        }
        command.arg(path);
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(if reveal {
            path.parent()
                .ok_or("Registered report has no parent directory.")?
        } else {
            path
        });
        command
    };
    command.spawn().map(|_| ()).map_err(|error| {
        format!("Could not open registered report with the operating system: {error}")
    })
}

#[tauri::command]
pub fn create_schedule(paths: State<'_, AppPaths>, input: NewSchedule) -> Result<Schedule, String> {
    if input.requirement_text.trim().is_empty()
        || input.cron_expression.trim().is_empty()
        || input.timezone.trim().is_empty()
    {
        return Err("Schedule prompt, cron expression, and timezone are required.".into());
    }
    if let Some(file_type) = input.deliverable_type.as_deref() {
        if !supported_type(file_type) {
            return Err("Unsupported deliverable type.".into());
        }
    }
    let now = db::now_ms();
    let id = match input.id {
        Some(value) => Uuid::parse_str(value.trim())
            .map_err(|_| "Schedule ID must be a UUID generated by KUBERA.")?
            .to_string(),
        None => Uuid::new_v4().to_string(),
    };
    let requirement_text = input.requirement_text.trim().to_string();
    let cron_expression = input.cron_expression.trim().to_string();
    let timezone = input.timezone.trim().to_string();
    let next_run = scheduler::next_run_ms(&cron_expression, &timezone, now)?;
    let schedule = Schedule {
        id,
        requirement_text,
        cron_expression,
        timezone,
        deliverable_type: input.deliverable_type,
        last_run: None,
        next_run: Some(next_run),
        enabled: true,
        created_at: now,
    };
    scheduler::validate_schedule(&schedule)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    connection.execute("INSERT INTO schedules (id, requirement_text, cron_expression, timezone, deliverable_type, last_run, next_run, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, 1, ?7, ?7)", params![schedule.id, schedule.requirement_text, schedule.cron_expression, schedule.timezone, schedule.deliverable_type, schedule.next_run, schedule.created_at]).map_err(map_error)?;
    db::append_audit(
        &connection,
        "user",
        "schedule_created",
        Some(&schedule.id),
        Some("Created enabled recurring mission."),
    )
    .map_err(map_error)?;
    Ok(schedule)
}

#[tauri::command]
pub fn list_schedules(paths: State<'_, AppPaths>) -> Result<Vec<Schedule>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, requirement_text, cron_expression, timezone, deliverable_type, last_run, next_run, enabled, created_at FROM schedules ORDER BY next_run ASC").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Schedule {
                id: row.get(0)?,
                requirement_text: row.get(1)?,
                cron_expression: row.get(2)?,
                timezone: row.get(3)?,
                deliverable_type: row.get(4)?,
                last_run: row.get(5)?,
                next_run: row.get(6)?,
                enabled: row.get::<_, i64>(7)? != 0,
                created_at: row.get(8)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn set_schedule_enabled(
    paths: State<'_, AppPaths>,
    schedule_id: String,
    enabled: bool,
) -> Result<(), String> {
    let now = db::now_ms();
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut next_run = None;
    if enabled {
        let (cron_expression, timezone): (String, String) = connection
            .query_row(
                "SELECT cron_expression, timezone FROM schedules WHERE id = ?1",
                params![&schedule_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| "Schedule was not found.")?;
        next_run = Some(scheduler::next_run_ms(&cron_expression, &timezone, now)?);
    }
    let changed = connection
        .execute(
            "UPDATE schedules SET enabled = ?1, next_run = COALESCE(?2, next_run), updated_at = ?3 WHERE id = ?4",
            params![if enabled { 1 } else { 0 }, next_run, now, schedule_id],
        )
        .map_err(map_error)?;
    if changed == 0 {
        return Err("Schedule was not found.".into());
    }
    db::append_audit(
        &connection,
        "user",
        "schedule_updated",
        Some(&schedule_id),
        Some(if enabled {
            "Enabled schedule and recalculated its next run."
        } else {
            "Paused schedule."
        }),
    )
    .map_err(map_error)
}

/// Queries exactly one approved, citation-only provider. This command has no
/// user-provided endpoint, arbitrary headers, browser navigation, or agent
/// invocation path. Search output is stored as inert citation data.
#[tauri::command]
pub fn search_approved_research(
    paths: State<'_, AppPaths>,
    input: ResearchSearchRequest,
) -> Result<Vec<ResearchCitation>, String> {
    const MAX_RESPONSE_BYTES: u64 = 512 * 1024;
    let query = bounded_text(&input.query, "Research query", 180)?;
    if query.chars().any(|character| character.is_control()) {
        return Err("Research query contains unsupported control characters.".into());
    }
    let mut url = reqwest::Url::parse("https://api.crossref.org/works").map_err(map_error)?;
    url.query_pairs_mut()
        .append_pair("query.bibliographic", &query)
        .append_pair("rows", "5")
        .append_pair(
            "select",
            "title,URL,DOI,publisher,published-print,published-online,issued",
        );
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("KUBERA-AI/0.1 approved-citation-connector")
        .build()
        .map_err(|_| "Could not configure the approved research connector.")?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|_| "Approved research provider could not be reached.")?;
    if !response.status().is_success() {
        return Err("Approved research provider returned an unavailable response.".into());
    }
    let mut payload = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut payload)
        .map_err(|_| "Could not read the approved research response.")?;
    if payload.len() as u64 > MAX_RESPONSE_BYTES {
        return Err("Approved research response exceeded KUBERA's safety limit.".into());
    }
    let items = serde_json::from_slice::<serde_json::Value>(&payload)
        .ok()
        .and_then(|value| {
            value
                .pointer("/message/items")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .ok_or_else(|| {
            "Approved research provider returned an invalid citation response.".to_string()
        })?;
    let fetched_at = db::now_ms();
    let mut citations = Vec::new();
    for item in items.into_iter().take(5) {
        let Some(title) = item
            .get("title")
            .and_then(serde_json::Value::as_array)
            .and_then(|titles| titles.first())
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && value.len() <= 400)
        else {
            continue;
        };
        let citation_url = item
            .get("URL")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                item.get("DOI")
                    .and_then(serde_json::Value::as_str)
                    .map(|doi| format!("https://doi.org/{doi}"))
            })
            .filter(|value| valid_citation_url(value));
        let Some(citation_url) = citation_url else {
            continue;
        };
        let publisher = item
            .get("publisher")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && value.len() <= 240)
            .map(str::to_string);
        let citation = ResearchCitation {
            id: Uuid::new_v4().to_string(),
            provider: "crossref".into(),
            query: query.clone(),
            title: title.to_string(),
            citation_url,
            publisher,
            published_year: citation_year(&item),
            fetched_at,
        };
        citations.push(citation);
    }
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    for citation in &citations {
        transaction.execute(
            "INSERT INTO research_citations (id, provider, query, title, citation_url, publisher, published_year, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![citation.id, citation.provider, citation.query, citation.title, citation.citation_url, citation.publisher, citation.published_year, citation.fetched_at],
        ).map_err(map_error)?;
    }
    transaction.execute(
        "INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('local_user', 'search_approved_research', 'crossref', ?1, ?2)",
        params![format!("Fetched {} bounded citation result(s) for a user-initiated research query.", citations.len()), fetched_at],
    ).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(citations)
}

#[tauri::command]
pub fn list_research_citations(
    paths: State<'_, AppPaths>,
) -> Result<Vec<ResearchCitation>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, provider, query, title, citation_url, publisher, published_year, fetched_at FROM research_citations ORDER BY fetched_at DESC LIMIT 100").map_err(map_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(ResearchCitation {
                id: row.get(0)?,
                provider: row.get(1)?,
                query: row.get(2)?,
                title: row.get(3)?,
                citation_url: row.get(4)?,
                publisher: row.get(5)?,
                published_year: row.get(6)?,
                fetched_at: row.get(7)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

fn valid_citation_url(value: &str) -> bool {
    reqwest::Url::parse(value)
        .ok()
        .is_some_and(|url| url.scheme() == "https" && url.host_str().is_some())
}

fn citation_year(item: &serde_json::Value) -> Option<i32> {
    ["published-print", "published-online", "issued"]
        .iter()
        .find_map(|field| {
            item.get(*field)
                .and_then(|value| value.get("date-parts"))
                .and_then(serde_json::Value::as_array)
                .and_then(|groups| groups.first())
                .and_then(serde_json::Value::as_array)
                .and_then(|parts| parts.first())
                .and_then(serde_json::Value::as_i64)
                .and_then(|year| i32::try_from(year).ok())
                .filter(|year| (1000..=3000).contains(year))
        })
}

#[tauri::command]
pub fn list_audit_events(
    paths: State<'_, AppPaths>,
    limit: Option<u32>,
) -> Result<Vec<AuditEvent>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let limit = i64::from(limit.unwrap_or(80).min(250));
    let mut statement = connection.prepare("SELECT id, actor, action, target, detail, created_at FROM audit_log ORDER BY created_at DESC LIMIT ?1").map_err(map_error)?;
    let rows = statement
        .query_map(params![limit], |row| {
            Ok(AuditEvent {
                id: row.get(0)?,
                actor: row.get(1)?,
                action: row.get(2)?,
                target: row.get(3)?,
                detail: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

/// Parses RFC-4180-style comma-delimited records for the bounded local import
/// contract. Newlines and doubled quotes are accepted inside quoted fields;
/// malformed quote placement is rejected instead of guessed.
fn parse_csv_records(input: &str) -> Result<Vec<Vec<String>>, String> {
    let mut records = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut after_quote = false;
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                    after_quote = true;
                }
            }
            '"' if field.is_empty() && !after_quote => in_quotes = true,
            '"' => return Err("CSV contains a quote outside a quoted field.".into()),
            ',' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                after_quote = false;
            }
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                after_quote = false;
                if !row.is_empty() && !(row.len() == 1 && row[0].is_empty()) {
                    records.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    continue;
                }
                row.push(std::mem::take(&mut field));
                after_quote = false;
                if !row.is_empty() && !(row.len() == 1 && row[0].is_empty()) {
                    records.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            character if after_quote => {
                return Err("CSV has text after a closing quote before its delimiter.".into())
            }
            character if character.is_control() && character != '\t' => {
                return Err("CSV contains an unsupported control character.".into())
            }
            character => field.push(character),
        }
    }
    if in_quotes {
        return Err("CSV contains an unclosed quoted field.".into());
    }
    if !row.is_empty() || !field.is_empty() {
        row.push(field);
        records.push(row);
    }
    Ok(records)
}

fn bounded_text(value: &str, label: &str, limit: usize) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > limit {
        return Err(format!("{label} must contain 1–{limit} characters."));
    }
    Ok(value.to_string())
}

fn optional_bounded_text(
    value: Option<String>,
    label: &str,
    limit: usize,
) -> Result<Option<String>, String> {
    match value {
        Some(value) if !value.trim().is_empty() => bounded_text(&value, label, limit).map(Some),
        _ => Ok(None),
    }
}

fn allowed_task_status(status: &str) -> bool {
    matches!(
        status,
        "blocked" | "ready" | "in_progress" | "review" | "approved" | "failed"
    )
}

#[tauri::command]
pub fn create_task(paths: State<'_, AppPaths>, input: NewTask) -> Result<TaskRecord, String> {
    if input.id.trim().is_empty()
        || input.requirement_id.trim().is_empty()
        || input.title.trim().is_empty()
        || input.brief.trim().is_empty()
        || input.role.trim().is_empty()
    {
        return Err("Task ID, requirement, title, brief, and role are required.".into());
    }
    if !allowed_task_status(&input.status) {
        return Err("Unsupported task status.".into());
    }
    let now = db::now_ms();
    let deps = serde_json::to_string(&input.deps).map_err(map_error)?;
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let exists: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM requirements WHERE id = ?1",
            params![input.requirement_id],
            |row| row.get(0),
        )
        .map_err(map_error)?;
    if exists != 1 {
        return Err("Requirement was not found for task creation.".into());
    }
    if let Some(agent_id) = input.assignee_id.as_deref() {
        let agents: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM agents WHERE id = ?1",
                params![agent_id],
                |row| row.get(0),
            )
            .map_err(map_error)?;
        if agents != 1 {
            return Err("Assigned agent was not found.".into());
        }
    }
    transaction.execute("INSERT INTO tasks (id, requirement_id, title, brief, role, required_capability, assignee_id, status, deps, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)", params![input.id, input.requirement_id, input.title, input.brief, input.role, input.required_capability, input.assignee_id, input.status, deps, now]).map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('agent-manager', 'create_task', ?1, 'Validated manager task persisted.', ?2)", params![input.id, now]).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(TaskRecord {
        id: input.id,
        requirement_id: input.requirement_id,
        title: input.title,
        brief: input.brief,
        role: input.role,
        required_capability: input.required_capability,
        assignee_id: input.assignee_id,
        status: input.status,
        deps: input.deps,
        output_path: None,
        review_notes: None,
        attempts: 0,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub fn list_tasks(
    paths: State<'_, AppPaths>,
    requirement_id: Option<String>,
) -> Result<Vec<TaskRecord>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let sql = "SELECT id, requirement_id, title, brief, role, required_capability, assignee_id, status, deps, output_path, review_notes, attempts, created_at, updated_at FROM tasks WHERE (?1 IS NULL OR requirement_id = ?1) ORDER BY created_at ASC";
    let mut statement = connection.prepare(sql).map_err(map_error)?;
    let rows = statement
        .query_map(params![requirement_id], |row| {
            let deps_json: String = row.get(8)?;
            Ok(TaskRecord {
                id: row.get(0)?,
                requirement_id: row.get(1)?,
                title: row.get(2)?,
                brief: row.get(3)?,
                role: row.get(4)?,
                required_capability: row.get(5)?,
                assignee_id: row.get(6)?,
                status: row.get(7)?,
                deps: serde_json::from_str(&deps_json).unwrap_or_default(),
                output_path: row.get(9)?,
                review_notes: row.get(10)?,
                attempts: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn list_agent_runs(
    paths: State<'_, AppPaths>,
    requirement_id: Option<String>,
) -> Result<Vec<AgentRunRecord>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let sql = "SELECT r.id, r.task_id, r.agent_id, r.parent_run_id, r.opencode_session_id, r.status, r.input_summary, r.output_summary, r.error_code, r.started_at, r.completed_at, r.created_at FROM agent_runs r JOIN tasks t ON t.id = r.task_id WHERE (?1 IS NULL OR t.requirement_id = ?1) ORDER BY r.created_at ASC";
    let mut statement = connection.prepare(sql).map_err(map_error)?;
    let rows = statement
        .query_map(params![requirement_id], |row| {
            Ok(AgentRunRecord {
                id: row.get(0)?,
                task_id: row.get(1)?,
                agent_id: row.get(2)?,
                parent_run_id: row.get(3)?,
                opencode_session_id: row.get(4)?,
                status: row.get(5)?,
                input_summary: row.get(6)?,
                output_summary: row.get(7)?,
                error_code: row.get(8)?,
                started_at: row.get(9)?,
                completed_at: row.get(10)?,
                created_at: row.get(11)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn list_agent_handoffs(
    paths: State<'_, AppPaths>,
    requirement_id: Option<String>,
) -> Result<Vec<AgentHandoffRecord>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, requirement_id, task_id, from_run_id, to_task_id, to_agent_id, handoff_type, payload_json, source_refs_json, validation_status, created_at FROM agent_handoffs WHERE (?1 IS NULL OR requirement_id = ?1) ORDER BY created_at ASC").map_err(map_error)?;
    let rows = statement
        .query_map(params![requirement_id], |row| {
            Ok(AgentHandoffRecord {
                id: row.get(0)?,
                requirement_id: row.get(1)?,
                task_id: row.get(2)?,
                from_run_id: row.get(3)?,
                to_task_id: row.get(4)?,
                to_agent_id: row.get(5)?,
                handoff_type: row.get(6)?,
                payload_json: row.get(7)?,
                source_refs_json: row.get(8)?,
                validation_status: row.get(9)?,
                created_at: row.get(10)?,
            })
        })
        .map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn update_task_status(
    paths: State<'_, AppPaths>,
    task_id: String,
    status: String,
    review_notes: Option<String>,
) -> Result<(), String> {
    if !allowed_task_status(&status) {
        return Err("Unsupported task status.".into());
    }
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let changed = connection
        .execute(
            "UPDATE tasks SET status = ?1, review_notes = ?2, updated_at = ?3 WHERE id = ?4",
            params![status, review_notes, db::now_ms(), task_id],
        )
        .map_err(map_error)?;
    if changed != 1 {
        return Err("Task was not found.".into());
    }
    db::append_audit(
        &connection,
        "system",
        "update_task_status",
        Some(&task_id),
        Some("Updated task state through governed lifecycle."),
    )
    .map_err(map_error)
}

#[tauri::command]
pub fn create_pptx_deck(
    paths: State<'_, AppPaths>,
    request: CreateDeckRequest,
) -> Result<OfficeExecution, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let (assignee_id, task_status): (Option<String>, String) = connection
        .query_row(
            "SELECT assignee_id, status FROM tasks WHERE id = ?1",
            params![request.task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "A persisted document-worker task is required before creating a deck.")?;
    if !is_document_worker(assignee_id.as_deref())
        || !matches!(task_status.as_str(), "in_progress" | "review")
    {
        return Err("Only an active fixed-pool document worker may create a deck.".into());
    }
    drop(connection);
    let mut execution = office::create_pptx_deck(&paths, &request)?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let deliverable_id = Uuid::new_v4().to_string();
    let now = db::now_ms();
    transaction.execute("INSERT INTO deliverables (id, task_id, file_path, file_name, file_type, created_at) VALUES (?1, ?2, ?3, ?4, 'pptx', ?5)", params![&deliverable_id, request.task_id, execution.relative_path, execution.file_name, now]).map_err(map_error)?;
    transaction
        .execute(
            "UPDATE tasks SET output_path = ?1, status = 'review', updated_at = ?2 WHERE id = ?3",
            params![execution.relative_path, now, request.task_id],
        )
        .map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('agent-worker-2', 'create_file', ?1, ?2, ?3)", params![&deliverable_id, execution.summary, now]).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    execution.deliverable_id = Some(deliverable_id);
    Ok(execution)
}

#[tauri::command]
pub fn create_docx_document(
    paths: State<'_, AppPaths>,
    request: CreateDocumentRequest,
) -> Result<OfficeExecution, String> {
    require_active_document_worker(&paths, &request.task_id)?;
    let task_id = request.task_id.clone();
    let execution = office::create_docx_document(&paths, &request)?;
    register_office_execution(&paths, &task_id, &execution)
}

#[tauri::command]
pub fn create_templated_docx(
    paths: State<'_, AppPaths>,
    request: CreateTemplatedDocxRequest,
) -> Result<OfficeExecution, String> {
    require_active_document_worker(&paths, &request.task_id)?;
    Uuid::parse_str(&request.template_id)
        .map_err(|_| "A valid approved template ID is required.")?;
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let (relative_path, file_type, placeholders_json): (String, String, String) = connection
        .query_row(
            "SELECT file_path, file_type, placeholders_json FROM templates WHERE id = ?1",
            params![&request.template_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "Approved template was not found.")?;
    if file_type != "docx" {
        return Err("Only an approved DOCX template can be merged into a DOCX report.".into());
    }
    let expected = serde_json::from_str::<Vec<String>>(&placeholders_json).unwrap_or_default();
    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    let supplied = request
        .values
        .iter()
        .map(|item| item.key.clone())
        .collect::<BTreeSet<_>>();
    if expected.is_empty() || expected != supplied {
        return Err(
            "Template merge values must match the approved template placeholder set exactly."
                .into(),
        );
    }
    let stored = Path::new(&relative_path);
    if stored.is_absolute()
        || stored
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("Approved template path failed template-root validation.".into());
    }
    let template_path = files::canonical_inside(&paths.templates, &paths.templates.join(stored))
        .map_err(map_error)?;
    drop(connection);
    let task_id = request.task_id.clone();
    let execution = office::merge_docx_template(&paths, &template_path, &request)?;
    register_office_execution(&paths, &task_id, &execution)
}

#[tauri::command]
pub fn create_xlsx_workbook(
    paths: State<'_, AppPaths>,
    request: CreateWorkbookRequest,
) -> Result<OfficeExecution, String> {
    require_active_document_worker(&paths, &request.task_id)?;
    let task_id = request.task_id.clone();
    let execution = office::create_xlsx_workbook(&paths, &request)?;
    register_office_execution(&paths, &task_id, &execution)
}

fn require_active_document_worker(paths: &AppPaths, task_id: &str) -> Result<(), String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let (assignee_id, task_status): (Option<String>, String) = connection
        .query_row(
            "SELECT assignee_id, status FROM tasks WHERE id = ?1",
            params![task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| {
            "A persisted document-worker task is required before creating an Office artifact."
        })?;
    if !is_document_worker(assignee_id.as_deref())
        || !matches!(task_status.as_str(), "in_progress" | "review")
    {
        return Err(
            "Only an active fixed-pool document worker may create an Office artifact.".into(),
        );
    }
    Ok(())
}

fn is_document_worker(agent_id: Option<&str>) -> bool {
    matches!(agent_id, Some("agent-worker-2" | "agent-worker-2b"))
}

fn register_office_execution(
    paths: &AppPaths,
    task_id: &str,
    execution: &OfficeExecution,
) -> Result<OfficeExecution, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let deliverable_id = Uuid::new_v4().to_string();
    let now = db::now_ms();
    transaction.execute("INSERT INTO deliverables (id, task_id, file_path, file_name, file_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![&deliverable_id, task_id, execution.relative_path, execution.file_name, execution.file_type, now]).map_err(map_error)?;
    transaction
        .execute(
            "UPDATE tasks SET output_path = ?1, status = 'review', updated_at = ?2 WHERE id = ?3",
            params![execution.relative_path, now, task_id],
        )
        .map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('agent-worker-2', 'create_file', ?1, ?2, ?3)", params![&deliverable_id, execution.summary, now]).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    let mut registered = execution.clone();
    registered.deliverable_id = Some(deliverable_id);
    Ok(registered)
}

#[cfg(test)]
mod template_tests {
    use super::{office_xml_text, parse_csv_records, placeholder_names};

    #[test]
    fn parses_quoted_csv_without_guessing_malformed_quotes() {
        let records = parse_csv_records(
            "name,stage,value,risk_flag\n\"Acme, expansion\",Proposal,120000,false\n",
        )
        .unwrap();
        assert_eq!(
            records[1],
            vec!["Acme, expansion", "Proposal", "120000", "false"]
        );
        assert!(parse_csv_records("name,stage\n\"unclosed,field").is_err());
        assert!(parse_csv_records("name,stage\n\"closed\"text,field").is_err());
    }

    #[test]
    fn finds_bounded_placeholder_names_across_word_text_runs() {
        let xml = "<w:t>{{client_</w:t><w:t>name}}</w:t><w:t> {{quarter}} {{bad value}}</w:t>";
        let text = office_xml_text(xml);
        assert_eq!(placeholder_names(&text), vec!["client_name", "quarter"]);
    }
}
