use rusqlite::{params, OptionalExtension};
use tauri::State;
use uuid::Uuid;

use crate::{db, files, models::{Agent, AuditEvent, Client, Deliverable, MissionContext, MissionSelector, NewSchedule, Project, Requirement, Schedule}, paths::AppPaths};

fn map_error(error: impl std::fmt::Display) -> String { error.to_string() }
fn supported_type(file_type: &str) -> bool { matches!(file_type, "pptx" | "docx" | "xlsx" | "pdf") }

#[tauri::command]
pub fn list_clients(paths: State<'_, AppPaths>) -> Result<Vec<Client>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, name, industry, status, owner, created_at FROM clients ORDER BY name COLLATE NOCASE").map_err(map_error)?;
    let rows = statement.query_map([], |row| Ok(Client {
        id: row.get(0)?, name: row.get(1)?, industry: row.get(2)?, status: row.get(3)?, owner: row.get(4)?, created_at: row.get(5)?,
    })).map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn list_projects(paths: State<'_, AppPaths>) -> Result<Vec<Project>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT p.id, p.client_id, c.name, p.name, p.stage, p.value, p.risk_flag, p.created_at FROM projects p JOIN clients c ON c.id = p.client_id ORDER BY p.value DESC, p.name").map_err(map_error)?;
    let rows = statement.query_map([], |row| Ok(Project {
        id: row.get(0)?, client_id: row.get(1)?, client_name: row.get(2)?, name: row.get(3)?, stage: row.get(4)?, value: row.get(5)?, risk_flag: row.get::<_, i64>(6)? != 0, created_at: row.get(7)?,
    })).map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn list_agents(paths: State<'_, AppPaths>) -> Result<Vec<Agent>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, name, role, agent_kind, capabilities, max_concurrency, status FROM agents ORDER BY CASE agent_kind WHEN 'manager' THEN 0 WHEN 'analyst' THEN 1 WHEN 'worker' THEN 2 ELSE 3 END, name").map_err(map_error)?;
    let rows = statement.query_map([], |row| {
        let capabilities: String = row.get(4)?;
        Ok(Agent {
            id: row.get(0)?, name: row.get(1)?, role: row.get(2)?, agent_kind: row.get(3)?,
            capabilities: serde_json::from_str(&capabilities).unwrap_or_default(), max_concurrency: row.get(5)?, status: row.get(6)?,
        })
    }).map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn list_requirements(paths: State<'_, AppPaths>) -> Result<Vec<Requirement>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, text, status, deliverable_type, created_at FROM requirements ORDER BY created_at DESC LIMIT 100").map_err(map_error)?;
    let rows = statement.query_map([], |row| Ok(Requirement {
        id: row.get(0)?, text: row.get(1)?, status: row.get(2)?, deliverable_type: row.get(3)?, created_at: row.get(4)?,
    })).map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn create_requirement(paths: State<'_, AppPaths>, text: String, deliverable_type: Option<String>) -> Result<Requirement, String> {
    let text = text.trim().to_string();
    if text.is_empty() || text.len() > 4_000 { return Err("Requirement text must be between 1 and 4,000 characters.".into()); }
    if let Some(file_type) = deliverable_type.as_deref() { if !supported_type(file_type) { return Err("Unsupported deliverable type.".into()); } }
    let requirement = Requirement { id: Uuid::new_v4().to_string(), text, status: "planning".into(), deliverable_type, created_at: db::now_ms() };
    let connection = db::connect(&paths.database).map_err(map_error)?;
    connection.execute("INSERT INTO requirements (id, text, status, deliverable_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![requirement.id, requirement.text, requirement.status, requirement.deliverable_type, requirement.created_at]).map_err(map_error)?;
    db::append_audit(&connection, "user", "create_requirement", Some(&requirement.id), Some("Created a business requirement from desktop UI.")).map_err(map_error)?;
    Ok(requirement)
}

#[tauri::command]
pub fn get_context_for_mission(paths: State<'_, AppPaths>, selector: MissionSelector, actor_id: String, purpose: String) -> Result<MissionContext, String> {
    if purpose.trim().is_empty() || purpose.len() > 500 { return Err("A concise context purpose is required.".into()); }
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let client = match selector.client_name.as_deref() {
        Some(name) => connection.query_row("SELECT id, name, industry, status, owner, created_at FROM clients WHERE name = ?1 COLLATE NOCASE LIMIT 1", params![name.trim()], |row| Ok(Client { id: row.get(0)?, name: row.get(1)?, industry: row.get(2)?, status: row.get(3)?, owner: row.get(4)?, created_at: row.get(5)? })).optional().map_err(map_error)?,
        None => None,
    };
    if let Some(client) = client.as_ref() {
        db::append_audit(&connection, &actor_id, "query_clients", Some(&client.id), Some(&purpose)).map_err(map_error)?;
    }
    let projects = if selector.include_projects {
        if let Some(client) = client.as_ref() {
            let mut statement = connection.prepare("SELECT p.id, p.client_id, c.name, p.name, p.stage, p.value, p.risk_flag, p.created_at FROM projects p JOIN clients c ON c.id = p.client_id WHERE p.client_id = ?1 AND (?2 IS NULL OR p.stage = ?2) ORDER BY p.value DESC").map_err(map_error)?;
            let rows = statement.query_map(params![client.id, selector.project_status], |row| Ok(Project { id: row.get(0)?, client_id: row.get(1)?, client_name: row.get(2)?, name: row.get(3)?, stage: row.get(4)?, value: row.get(5)?, risk_flag: row.get::<_, i64>(6)? != 0, created_at: row.get(7)? })).map_err(map_error)?;
            let projects = rows.collect::<Result<Vec<_>, _>>().map_err(map_error)?;
            let detail = format!("{} scoped project record(s) for {}", projects.len(), purpose);
            db::append_audit(&connection, &actor_id, "query_projects", Some(&client.id), Some(&detail)).map_err(map_error)?;
            projects
        } else { Vec::new() }
    } else { Vec::new() };
    Ok(MissionContext { client, projects })
}

#[tauri::command]
pub fn create_agent_run(paths: State<'_, AppPaths>, task_id: String, agent_id: String, parent_run_id: Option<String>) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let created_at = db::now_ms();
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let agent_status: String = transaction.query_row("SELECT status FROM agents WHERE id = ?1", params![agent_id], |row| row.get(0)).map_err(map_error)?;
    if agent_status != "idle" { return Err("Selected agent is not available.".into()); }
    let task_status: String = transaction.query_row("SELECT status FROM tasks WHERE id = ?1", params![task_id], |row| row.get(0)).map_err(map_error)?;
    if task_status != "ready" { return Err("Task is not ready for an agent run.".into()); }
    transaction.execute("UPDATE agents SET status = 'busy' WHERE id = ?1 AND status = 'idle'", params![agent_id]).map_err(map_error)?;
    transaction.execute("UPDATE tasks SET assignee_id = ?1, status = 'in_progress', attempts = attempts + 1, updated_at = ?2 WHERE id = ?3", params![agent_id, created_at, task_id]).map_err(map_error)?;
    transaction.execute("INSERT INTO agent_runs (id, task_id, agent_id, parent_run_id, status, started_at, created_at) VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5)", params![id, task_id, agent_id, parent_run_id, created_at]).map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES (?1, 'start_agent_run', ?2, 'Atomically claimed ready task for named agent.', ?3)", params![agent_id, task_id, created_at]).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(id)
}

#[tauri::command]
pub fn complete_agent_run(paths: State<'_, AppPaths>, run_id: String, status: String, output_summary: Option<String>) -> Result<(), String> {
    if !matches!(status.as_str(), "succeeded" | "failed" | "cancelled") { return Err("Agent run status must be succeeded, failed, or cancelled.".into()); }
    let completed_at = db::now_ms();
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    let (task_id, agent_id): (String, String) = transaction.query_row("SELECT task_id, agent_id FROM agent_runs WHERE id = ?1", params![run_id], |row| Ok((row.get(0)?, row.get(1)?))).map_err(map_error)?;
    transaction.execute("UPDATE agent_runs SET status = ?1, output_summary = ?2, completed_at = ?3 WHERE id = ?4", params![status, output_summary, completed_at, run_id]).map_err(map_error)?;
    transaction.execute("UPDATE agents SET status = 'idle' WHERE id = ?1", params![agent_id]).map_err(map_error)?;
    let task_status = if status == "succeeded" { "review" } else { "failed" };
    transaction.execute("UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3", params![task_status, completed_at, task_id]).map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES (?1, 'complete_agent_run', ?2, ?3, ?4)", params![agent_id, task_id, status, completed_at]).map_err(map_error)?;
    transaction.commit().map_err(map_error)
}

#[tauri::command]
pub fn record_agent_handoff(paths: State<'_, AppPaths>, requirement_id: String, task_id: String, from_run_id: String, handoff_type: String, payload_json: String, source_refs_json: String) -> Result<String, String> {
    if !matches!(handoff_type.as_str(), "market_analysis" | "pipeline_metrics" | "operations_metrics") { return Err("Unsupported handoff type.".into()); }
    if payload_json.len() > 32_000 || source_refs_json.len() > 8_000 { return Err("Handoff payload exceeds policy limits.".into()); }
    let payload: serde_json::Value = serde_json::from_str(&payload_json).map_err(|_| "Handoff payload must be valid JSON.")?;
    let sources: Vec<String> = serde_json::from_str(&source_refs_json).map_err(|_| "Handoff sources must be a JSON string array.")?;
    if !payload.is_object() || sources.is_empty() { return Err("Handoff requires an object payload and at least one source record reference.".into()); }
    let id = Uuid::new_v4().to_string();
    let connection = db::connect(&paths.database).map_err(map_error)?;
    connection.execute("INSERT INTO agent_handoffs (id, requirement_id, task_id, from_run_id, handoff_type, payload_json, source_refs_json, validation_status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'accepted', ?8)", params![id, requirement_id, task_id, from_run_id, handoff_type, payload_json, source_refs_json, db::now_ms()]).map_err(map_error)?;
    db::append_audit(&connection, "system", "create_handoff", Some(&id), Some("Stored accepted, source-linked structured handoff.")).map_err(map_error)?;
    Ok(id)
}

#[tauri::command]
pub fn register_deliverable(paths: State<'_, AppPaths>, task_id: String, file_name: String, file_type: String) -> Result<Deliverable, String> {
    let safe_name = files::safe_file_name(&file_name, &file_type).map_err(map_error)?;
    let report_path = paths.report_path(&safe_name).map_err(map_error)?;
    files::validate_nonempty_report(&report_path, &file_type).map_err(map_error)?;
    let canonical = files::canonical_inside(&paths.reports, &report_path).map_err(map_error)?;
    let relative = files::relative_display_path(&paths.reports, &canonical).map_err(map_error)?;
    let deliverable = Deliverable { id: Uuid::new_v4().to_string(), task_id, file_path: relative, file_name: safe_name, file_type, created_at: db::now_ms() };
    let mut connection = db::connect(&paths.database).map_err(map_error)?;
    let transaction = connection.unchecked_transaction().map_err(map_error)?;
    transaction.execute("INSERT INTO deliverables (id, task_id, file_path, file_name, file_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![deliverable.id, deliverable.task_id, deliverable.file_path, deliverable.file_name, deliverable.file_type, deliverable.created_at]).map_err(map_error)?;
    transaction.execute("UPDATE tasks SET output_path = ?1, status = 'review', updated_at = ?2 WHERE id = ?3", params![deliverable.file_path, deliverable.created_at, deliverable.task_id]).map_err(map_error)?;
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES ('agent-worker-2', 'create_file', ?1, ?2, ?3)", params![deliverable.id, deliverable.file_path, deliverable.created_at]).map_err(map_error)?;
    transaction.commit().map_err(map_error)?;
    Ok(deliverable)
}

#[tauri::command]
pub fn list_deliverables(paths: State<'_, AppPaths>) -> Result<Vec<Deliverable>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, task_id, file_path, file_name, file_type, created_at FROM deliverables ORDER BY created_at DESC LIMIT 200").map_err(map_error)?;
    let rows = statement.query_map([], |row| Ok(Deliverable { id: row.get(0)?, task_id: row.get(1)?, file_path: row.get(2)?, file_name: row.get(3)?, file_type: row.get(4)?, created_at: row.get(5)? })).map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn create_schedule(paths: State<'_, AppPaths>, input: NewSchedule) -> Result<Schedule, String> {
    if input.requirement_text.trim().is_empty() || input.cron_expression.trim().is_empty() || input.timezone.trim().is_empty() { return Err("Schedule prompt, cron expression, and timezone are required.".into()); }
    if let Some(file_type) = input.deliverable_type.as_deref() { if !supported_type(file_type) { return Err("Unsupported deliverable type.".into()); } }
    // Full cron parse/next-time calculation is added with the scheduler module.
    let now = db::now_ms();
    let schedule = Schedule { id: Uuid::new_v4().to_string(), requirement_text: input.requirement_text.trim().into(), cron_expression: input.cron_expression.trim().into(), timezone: input.timezone.trim().into(), deliverable_type: input.deliverable_type, last_run: None, next_run: Some(now + 86_400_000), enabled: true, created_at: now };
    let connection = db::connect(&paths.database).map_err(map_error)?;
    connection.execute("INSERT INTO schedules (id, requirement_text, cron_expression, timezone, deliverable_type, last_run, next_run, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, 1, ?7, ?7)", params![schedule.id, schedule.requirement_text, schedule.cron_expression, schedule.timezone, schedule.deliverable_type, schedule.next_run, schedule.created_at]).map_err(map_error)?;
    db::append_audit(&connection, "user", "schedule_created", Some(&schedule.id), Some("Created enabled recurring mission." )).map_err(map_error)?;
    Ok(schedule)
}

#[tauri::command]
pub fn list_schedules(paths: State<'_, AppPaths>) -> Result<Vec<Schedule>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let mut statement = connection.prepare("SELECT id, requirement_text, cron_expression, timezone, deliverable_type, last_run, next_run, enabled, created_at FROM schedules ORDER BY next_run ASC").map_err(map_error)?;
    let rows = statement.query_map([], |row| Ok(Schedule { id: row.get(0)?, requirement_text: row.get(1)?, cron_expression: row.get(2)?, timezone: row.get(3)?, deliverable_type: row.get(4)?, last_run: row.get(5)?, next_run: row.get(6)?, enabled: row.get::<_, i64>(7)? != 0, created_at: row.get(8)? })).map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}

#[tauri::command]
pub fn set_schedule_enabled(paths: State<'_, AppPaths>, schedule_id: String, enabled: bool) -> Result<(), String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let changed = connection.execute("UPDATE schedules SET enabled = ?1, updated_at = ?2 WHERE id = ?3", params![if enabled { 1 } else { 0 }, db::now_ms(), schedule_id]).map_err(map_error)?;
    if changed == 0 { return Err("Schedule was not found.".into()); }
    db::append_audit(&connection, "user", "schedule_updated", Some(&schedule_id), Some(if enabled { "Enabled schedule." } else { "Paused schedule." })).map_err(map_error)
}

#[tauri::command]
pub fn list_audit_events(paths: State<'_, AppPaths>, limit: Option<u32>) -> Result<Vec<AuditEvent>, String> {
    let connection = db::connect(&paths.database).map_err(map_error)?;
    let limit = i64::from(limit.unwrap_or(80).min(250));
    let mut statement = connection.prepare("SELECT id, actor, action, target, detail, created_at FROM audit_log ORDER BY created_at DESC LIMIT ?1").map_err(map_error)?;
    let rows = statement.query_map(params![limit], |row| Ok(AuditEvent { id: row.get(0)?, actor: row.get(1)?, action: row.get(2)?, target: row.get(3)?, detail: row.get(4)?, created_at: row.get(5)? })).map_err(map_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(map_error)
}
