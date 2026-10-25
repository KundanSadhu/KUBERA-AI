mod commands;
mod db;
mod files;
mod models;
mod office;
mod opencode;
mod p0;
mod paths;
mod runtime;
mod scheduler;
mod sidecar;

use models::EngineStatus;
use paths::AppPaths;
use sidecar::SidecarState;
use tauri::{Manager, State};

#[tauri::command]
fn get_engine_status(state: State<'_, SidecarState>) -> EngineStatus {
    state.current_status()
}

#[tauri::command]
fn sidecar_start(
    app: tauri::AppHandle,
    state: State<'_, SidecarState>,
    forwarder: State<'_, opencode::EventForwarder>,
) -> Result<EngineStatus, String> {
    let status = state.start(&app)?;
    if status.state == "ready" {
        opencode::start_event_forwarder(&forwarder, app);
    }
    Ok(status)
}

#[tauri::command]
fn sidecar_stop(state: State<'_, SidecarState>) -> Result<(), String> {
    state.stop()
}

/// The renderer subscribes before this command is called, preventing a
/// schedule-fired event from being lost during desktop bootstrap.
#[tauri::command]
fn scheduler_start(
    app: tauri::AppHandle,
    paths: State<'_, AppPaths>,
    state: State<'_, scheduler::SchedulerState>,
) -> Result<(), String> {
    state.start(app, paths.inner().clone())
}

fn require_ready_sidecar(state: &SidecarState) -> Result<(), String> {
    if state.current_status().state != "ready" {
        return Err("OpenCode sidecar is not ready. Start the engine and wait for a healthy loopback status.".into());
    }
    Ok(())
}

#[tauri::command]
fn opencode_health(state: State<'_, SidecarState>) -> Result<serde_json::Value, String> {
    require_ready_sidecar(&state)?;
    opencode::health()
}

#[tauri::command]
fn opencode_create_session(state: State<'_, SidecarState>) -> Result<serde_json::Value, String> {
    require_ready_sidecar(&state)?;
    opencode::create_session()
}

#[tauri::command]
fn opencode_send_message(
    state: State<'_, SidecarState>,
    session_id: String,
    text: String,
) -> Result<serde_json::Value, String> {
    require_ready_sidecar(&state)?;
    opencode::send_message(&session_id, &text)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let paths = AppPaths::from_app(app.handle())?;
            paths.prepare()?;
            db::bootstrap(&paths.database)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            app.manage(paths);
            app.manage(SidecarState::default());
            app.manage(opencode::EventForwarder::default());
            app.manage(scheduler::SchedulerState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_engine_status,
            sidecar_start,
            sidecar_stop,
            scheduler_start,
            opencode_health,
            opencode_create_session,
            opencode_send_message,
            commands::list_clients,
            commands::create_client,
            commands::update_client,
            commands::list_projects,
            commands::create_project,
            commands::import_projects_csv,
            commands::update_project,
            commands::list_agents,
            commands::list_requirements,
            commands::create_requirement,
            commands::update_requirement_status,
            commands::get_context_for_mission,
            commands::create_task,
            commands::list_tasks,
            commands::create_agent_run,
            commands::complete_agent_run,
            commands::set_agent_run_session,
            commands::list_agent_runs,
            commands::record_agent_handoff,
            commands::list_agent_handoffs,
            commands::update_task_status,
            commands::create_pptx_deck,
            commands::create_docx_document,
            commands::create_templated_docx,
            commands::create_xlsx_workbook,
            commands::register_deliverable,
            commands::list_deliverables,
            commands::send_approved_delivery,
            commands::list_deliveries,
            commands::request_deliverable_approval,
            commands::list_deliverable_approvals,
            commands::decide_deliverable_approval,
            commands::import_template,
            commands::import_template_bundle,
            commands::export_template_bundle,
            commands::list_templates,
            commands::open_deliverable,
            commands::reveal_deliverable,
            commands::preview_deliverable,
            commands::preview_deliverable_visual,
            commands::create_schedule,
            commands::list_schedules,
            commands::set_schedule_enabled,
            commands::search_approved_research,
            commands::list_research_citations,
            commands::list_audit_events
        ])
        .run(tauri::generate_context!())
        .expect("error while running KUBERA AI desktop application");
}
