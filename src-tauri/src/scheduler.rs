//! Persistent, in-process scheduler for governed recurring missions.
//!
//! This module never invokes an LLM, OfficeCLI, shell, or arbitrary path. It
//! computes the next IANA-timezone cron occurrence, durably advances a schedule
//! in SQLite, appends an audit event, then emits one `schedule-fired` event for
//! the renderer to create a normal managed mission. The desktop must remain
//! open; missed occurrences are not replayed as an unbounded backlog.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule as CronSchedule;
use rusqlite::params;
use serde::Serialize;
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tauri::{AppHandle, Emitter};

use crate::{db, models::Schedule, paths::AppPaths};

const POLL_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleFired {
    pub schedule_id: String,
    pub requirement_text: String,
    pub deliverable_type: Option<String>,
    pub timezone: String,
    pub last_run: i64,
    pub next_run: i64,
}

/// One scheduler worker per desktop process. It is started only after the
/// renderer subscribes to `schedule-fired`, so a just-fired event cannot be
/// lost while the UI is booting.
pub struct SchedulerState {
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        }
    }
}

impl Drop for SchedulerState {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(worker) = self.worker.get_mut() {
            if let Some(handle) = worker.take() {
                let _ = handle.join();
            }
        }
    }
}

/// Parses a five-, six-, or seven-field cron expression and returns the first
/// future occurrence after `after_ms` in the supplied IANA timezone.
pub fn next_run_ms(expression: &str, timezone: &str, after_ms: i64) -> Result<i64, String> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Err("Cron expression is required.".into());
    }
    let timezone = Tz::from_str(timezone.trim()).map_err(|_| {
        "Timezone must be a valid IANA identifier, for example Asia/Calcutta.".to_string()
    })?;
    // The KUBERA UI uses standard five-field cron (minute through weekday),
    // while the Rust cron crate also requires a leading seconds field. Preserve
    // six/seven-field expressions for advanced, explicit schedules.
    let fields = expression.split_whitespace().count();
    let normalized = match fields {
        5 => format!("0 {expression}"),
        6 | 7 => expression.to_string(),
        _ => return Err("Cron expression must have five, six, or seven fields.".into()),
    };
    let schedule = CronSchedule::from_str(&normalized)
        .map_err(|error| format!("Invalid cron expression: {error}"))?;
    let after = DateTime::<Utc>::from_timestamp_millis(after_ms)
        .ok_or_else(|| "Schedule timestamp is outside the supported range.".to_string())?
        .with_timezone(&timezone);
    schedule
        .after(&after)
        .next()
        .map(|time| time.timestamp_millis())
        .ok_or_else(|| "Cron expression has no future occurrence.".to_string())
}

pub fn validate_schedule(schedule: &Schedule) -> Result<(), String> {
    if schedule.requirement_text.trim().is_empty() {
        return Err("Schedule requirement text is required.".into());
    }
    if schedule.timezone.trim().is_empty() {
        return Err("Schedule timezone is required.".into());
    }
    next_run_ms(&schedule.cron_expression, &schedule.timezone, db::now_ms()).map(|_| ())
}

pub fn schedule_event_name() -> &'static str {
    "schedule-fired"
}

impl SchedulerState {
    pub fn start(&self, app: AppHandle, paths: AppPaths) -> Result<(), String> {
        let mut worker = self
            .worker
            .lock()
            .map_err(|_| "Scheduler worker lock unavailable.")?;
        if worker.is_some() {
            return Ok(());
        }
        self.stop.store(false, Ordering::SeqCst);
        let stop = Arc::clone(&self.stop);
        *worker = Some(thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                // Errors are contained to this poll. Invalid schedules are
                // rejected at write time, while transient SQLite contention is
                // retried at the next bounded interval.
                let _ = dispatch_due_schedules(&paths, &app);
                for _ in 0..POLL_INTERVAL.as_secs() {
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }));
        Ok(())
    }
}

fn dispatch_due_schedules(paths: &AppPaths, app: &AppHandle) -> Result<(), String> {
    let now = db::now_ms();
    let connection = db::connect(&paths.database).map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare("SELECT id, requirement_text, cron_expression, timezone, deliverable_type, last_run, next_run, enabled, created_at FROM schedules WHERE enabled = 1 AND next_run IS NOT NULL AND next_run <= ?1 ORDER BY next_run ASC")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![now], schedule_from_row)
        .map_err(|error| error.to_string())?;
    let due = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    drop(connection);

    for schedule in due {
        let next_run = next_run_ms(&schedule.cron_expression, &schedule.timezone, now)?;
        let mut connection = db::connect(&paths.database).map_err(|error| error.to_string())?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        // The guarded update makes each emitted event exactly once for this
        // single desktop instance, even if a future worker is restarted.
        let changed = transaction
            .execute(
                "UPDATE schedules SET last_run = ?1, next_run = ?2, updated_at = ?1 WHERE id = ?3 AND enabled = 1 AND next_run IS NOT NULL AND next_run <= ?1",
                params![now, next_run, schedule.id],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            transaction.commit().map_err(|error| error.to_string())?;
            continue;
        }
        db::append_audit(
            &transaction,
            "scheduler",
            "schedule_fired",
            Some(&schedule.id),
            Some("Advanced recurring schedule and dispatched a governed mission event."),
        )
        .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        app.emit(
            schedule_event_name(),
            ScheduleFired {
                schedule_id: schedule.id,
                requirement_text: schedule.requirement_text,
                deliverable_type: schedule.deliverable_type,
                timezone: schedule.timezone,
                last_run: now,
                next_run,
            },
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn schedule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Schedule> {
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
}

#[cfg(test)]
mod tests {
    use super::next_run_ms;

    #[test]
    fn standard_five_field_cron_respects_iana_timezone() {
        let now = 1_792_886_400_000; // 2026-10-25T00:00:00Z
        let next = next_run_ms("0 9 1 * *", "Asia/Calcutta", now).unwrap();
        assert!(next > now);
    }

    #[test]
    fn invalid_cron_and_timezone_are_rejected() {
        assert!(next_run_ms("not a cron", "Asia/Calcutta", 1_700_000_000_000).is_err());
        assert!(next_run_ms("0 9 1 * *", "Not/A_Timezone", 1_700_000_000_000).is_err());
    }
}
