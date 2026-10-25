//! Persistent schedule integration boundary.
//!
//! The SQLite commands already persist schedule rows. Wire this module after
//! the target host validates the selected cron crate's timezone semantics.
//! A scheduler event must contain only a schedule ID; JS then creates a normal
//! requirement and uses the same managed-workforce path as an ad-hoc mission.

use crate::models::Schedule;

pub fn validate_schedule(schedule: &Schedule) -> Result<(), String> {
    if schedule.requirement_text.trim().is_empty() { return Err("Schedule requirement text is required.".into()); }
    if schedule.cron_expression.trim().split_whitespace().count() < 5 { return Err("Cron expression must contain at least five fields.".into()); }
    if schedule.timezone.trim().is_empty() { return Err("Schedule timezone is required.".into()); }
    Ok(())
}

/// The future implementation owns a `tokio_cron_scheduler::JobScheduler`,
/// rehydrates enabled rows on startup, and emits `schedule-fired` to Tauri.
/// Do not execute an LLM or OfficeCLI process directly inside a cron callback;
/// dispatch through the persisted requirement/task/run lifecycle instead.
pub fn schedule_event_name() -> &'static str { "schedule-fired" }
