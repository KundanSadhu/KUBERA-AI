use rusqlite::{params, Connection, Result, Transaction};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn connect(path: &Path) -> Result<Connection> {
    let connection = Connection::open(path)?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
    )?;
    Ok(connection)
}

pub fn bootstrap(path: &Path) -> Result<()> {
    let mut connection = connect(path)?;
    migrate(&connection)?;
    seed(&mut connection)?;
    reconcile_interrupted_runs(&mut connection)?;
    Ok(())
}

fn migrate(connection: &Connection) -> Result<()> {
    connection.execute_batch(r#"
      CREATE TABLE IF NOT EXISTS clients (
        id TEXT PRIMARY KEY, name TEXT NOT NULL, industry TEXT,
        status TEXT NOT NULL DEFAULT 'active', owner TEXT, created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY, client_id TEXT NOT NULL REFERENCES clients(id),
        name TEXT NOT NULL, stage TEXT, value REAL NOT NULL DEFAULT 0,
        risk_flag INTEGER NOT NULL DEFAULT 0 CHECK (risk_flag IN (0, 1)), created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS requirements (
        id TEXT PRIMARY KEY, text TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'planning' CHECK (status IN ('planning', 'running', 'done', 'failed')),
        deliverable_type TEXT CHECK (deliverable_type IN ('docx', 'xlsx', 'pptx', 'pdf')),
        created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS agents (
        id TEXT PRIMARY KEY, name TEXT NOT NULL, role TEXT NOT NULL,
        agent_kind TEXT NOT NULL CHECK (agent_kind IN ('manager', 'analyst', 'worker', 'reviewer')),
        capabilities TEXT NOT NULL DEFAULT '[]', max_concurrency INTEGER NOT NULL DEFAULT 1,
        system_prompt TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'busy')),
        trust_level TEXT NOT NULL DEFAULT 'standard', created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY, requirement_id TEXT NOT NULL REFERENCES requirements(id),
        parent_task_id TEXT REFERENCES tasks(id), title TEXT NOT NULL, brief TEXT NOT NULL,
        role TEXT NOT NULL, required_capability TEXT, assignee_id TEXT REFERENCES agents(id),
        status TEXT NOT NULL DEFAULT 'blocked' CHECK (status IN ('blocked', 'ready', 'in_progress', 'review', 'approved', 'failed')),
        deps TEXT NOT NULL DEFAULT '[]', output_path TEXT, review_notes TEXT,
        attempts INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS agent_runs (
        id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), agent_id TEXT NOT NULL REFERENCES agents(id),
        parent_run_id TEXT REFERENCES agent_runs(id), opencode_session_id TEXT UNIQUE,
        status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
        input_summary TEXT, output_summary TEXT, error_code TEXT,
        started_at INTEGER, completed_at INTEGER, created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS agent_handoffs (
        id TEXT PRIMARY KEY, requirement_id TEXT NOT NULL REFERENCES requirements(id), task_id TEXT NOT NULL REFERENCES tasks(id),
        from_run_id TEXT NOT NULL REFERENCES agent_runs(id), to_task_id TEXT REFERENCES tasks(id),
        to_agent_id TEXT REFERENCES agents(id), handoff_type TEXT NOT NULL, payload_json TEXT NOT NULL,
        source_refs_json TEXT NOT NULL DEFAULT '[]', validation_status TEXT NOT NULL DEFAULT 'accepted'
          CHECK (validation_status IN ('accepted', 'rejected')), created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS deliverables (
        id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), file_path TEXT NOT NULL UNIQUE,
        file_name TEXT NOT NULL, file_type TEXT NOT NULL CHECK (file_type IN ('docx', 'xlsx', 'pptx', 'pdf')),
        created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS deliverable_approvals (
        id TEXT PRIMARY KEY, deliverable_id TEXT NOT NULL REFERENCES deliverables(id),
        status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
        requested_by TEXT NOT NULL, decision_note TEXT, requested_at INTEGER NOT NULL, decided_at INTEGER
      );
      CREATE UNIQUE INDEX IF NOT EXISTS idx_deliverable_approvals_pending
        ON deliverable_approvals(deliverable_id) WHERE status = 'pending';
      CREATE TABLE IF NOT EXISTS schedules (
        id TEXT PRIMARY KEY, requirement_text TEXT NOT NULL, cron_expression TEXT NOT NULL,
        timezone TEXT NOT NULL DEFAULT 'Asia/Calcutta', deliverable_type TEXT CHECK (deliverable_type IN ('docx', 'xlsx', 'pptx', 'pdf')),
        last_run INTEGER, next_run INTEGER, enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
        created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS templates (
        id TEXT PRIMARY KEY, name TEXT NOT NULL, file_path TEXT NOT NULL UNIQUE,
        file_type TEXT NOT NULL CHECK (file_type IN ('docx', 'xlsx', 'pptx')),
        placeholders_json TEXT NOT NULL DEFAULT '[]', created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS research_citations (
        id TEXT PRIMARY KEY, provider TEXT NOT NULL CHECK (provider IN ('crossref')),
        query TEXT NOT NULL, title TEXT NOT NULL, citation_url TEXT NOT NULL,
        publisher TEXT, published_year INTEGER, fetched_at INTEGER NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_research_citations_fetched ON research_citations(fetched_at DESC);
      CREATE TABLE IF NOT EXISTS deliveries (
        id TEXT PRIMARY KEY, deliverable_id TEXT NOT NULL REFERENCES deliverables(id),
        channel TEXT NOT NULL CHECK (channel IN ('email_smtp', 'slack_upload')),
        recipient TEXT NOT NULL, status TEXT NOT NULL CHECK (status IN ('sending', 'sent', 'failed')),
        requested_at INTEGER NOT NULL, completed_at INTEGER
      );
      CREATE INDEX IF NOT EXISTS idx_deliveries_deliverable ON deliveries(deliverable_id, requested_at DESC);
      CREATE TABLE IF NOT EXISTS audit_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT, actor TEXT NOT NULL, action TEXT NOT NULL,
        target TEXT, detail TEXT, created_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
      CREATE INDEX IF NOT EXISTS idx_projects_client ON projects(client_id);
      CREATE INDEX IF NOT EXISTS idx_tasks_requirement_status ON tasks(requirement_id, status);
      CREATE INDEX IF NOT EXISTS idx_agent_runs_task ON agent_runs(task_id);
      CREATE INDEX IF NOT EXISTS idx_handoffs_requirement ON agent_handoffs(requirement_id);
      CREATE INDEX IF NOT EXISTS idx_deliverables_task ON deliverables(task_id);
      CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at DESC);
    "#)
}

/// Conservatively release work that was active when the desktop process exited.
/// Completed files and reviewed tasks are never changed; an interrupted running
/// task becomes ready for its bounded manager to retry on the next desktop boot.
fn reconcile_interrupted_runs(connection: &mut Connection) -> Result<()> {
    let now = now_ms();
    let transaction = connection.transaction()?;
    let interrupted = transaction.execute(
        "UPDATE agent_runs SET status = 'cancelled', error_code = 'desktop_restart', completed_at = ?1 WHERE status = 'running'",
        params![now],
    )?;
    if interrupted > 0 {
        transaction.execute(
            "UPDATE agents SET status = 'idle' WHERE status = 'busy'",
            [],
        )?;
        transaction.execute(
            "UPDATE tasks SET status = 'ready', review_notes = COALESCE(review_notes, 'Interrupted by desktop restart; ready for governed retry.'), updated_at = ?1 WHERE status = 'in_progress' AND output_path IS NULL",
            params![now],
        )?;
        append_audit_tx(
            &transaction,
            "system",
            "reconcile_restart",
            None,
            Some("Released interrupted agent runs and returned unfinished tasks to ready."),
            now,
        )?;
    }
    transaction.commit()
}

fn seed(connection: &mut Connection) -> Result<()> {
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM clients", [], |row| row.get(0))?;
    let timestamp = now_ms();
    if count > 0 {
        let transaction = connection.transaction()?;
        // Later bounded roster additions must also reach existing local
        // installations without modifying user business records.
        seed_agents(&transaction, timestamp)?;
        transaction.commit()?;
        return Ok(());
    }
    let transaction = connection.transaction()?;
    seed_clients_and_projects(&transaction, timestamp)?;
    seed_agents(&transaction, timestamp)?;
    append_audit_tx(
        &transaction,
        "system",
        "seed_database",
        None,
        Some("Created demo business records and managed agent roster."),
        timestamp,
    )?;
    transaction.commit()
}

fn seed_clients_and_projects(transaction: &Transaction<'_>, timestamp: i64) -> Result<()> {
    for (id, name, industry, status, owner) in [
        (
            "seed-client-acme",
            "Acme Corp",
            "Enterprise SaaS",
            "active",
            "Priya Shah",
        ),
        (
            "seed-client-northstar",
            "Northstar Health",
            "Healthcare",
            "active",
            "Daniel Reed",
        ),
        (
            "seed-client-meridian",
            "Meridian Retail",
            "Retail",
            "prospect",
            "Priya Shah",
        ),
    ] {
        transaction.execute("INSERT INTO clients (id, name, industry, status, owner, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![id, name, industry, status, owner, timestamp])?;
    }
    for (id, client_id, name, stage, value, risk_flag) in [
        (
            "seed-project-acme-expansion",
            "seed-client-acme",
            "Q3 Platform Expansion",
            "Proposal",
            240000.0,
            0,
        ),
        (
            "seed-project-acme-analytics",
            "seed-client-acme",
            "Analytics Modernization",
            "Negotiation",
            125000.0,
            1,
        ),
        (
            "seed-project-northstar-portal",
            "seed-client-northstar",
            "Patient Portal Discovery",
            "Discovery",
            80000.0,
            0,
        ),
    ] {
        transaction.execute("INSERT INTO projects (id, client_id, name, stage, value, risk_flag, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![id, client_id, name, stage, value, risk_flag, timestamp])?;
    }
    Ok(())
}

fn seed_agents(transaction: &Transaction<'_>, timestamp: i64) -> Result<()> {
    let roster = [
        (
            "agent-manager",
            "Asha",
            "Manager",
            "manager",
            r#"["plan_subagents"]"#,
            "Plan only; never creates sessions, files, or tool calls.",
        ),
        (
            "agent-market-analyst",
            "Maya",
            "Market Analyst",
            "analyst",
            r#"["analyze_market_context","create_handoff"]"#,
            "Analyze supplied local business data and cite source IDs.",
        ),
        (
            "agent-market-analyst-2",
            "Maya-2",
            "Market Analyst",
            "analyst",
            r#"["analyze_market_context","create_handoff"]"#,
            "Analyze supplied local business data and cite source IDs.",
        ),
        (
            "agent-worker-1",
            "W-1",
            "Operations Analyst",
            "worker",
            r#"["analyze_operations","create_handoff"]"#,
            "Prepare exact local pipeline metrics only.",
        ),
        (
            "agent-worker-1b",
            "W-1B",
            "Operations Analyst",
            "worker",
            r#"["analyze_operations","create_handoff"]"#,
            "Prepare exact local pipeline metrics only.",
        ),
        (
            "agent-worker-2",
            "W-2",
            "Document Worker",
            "worker",
            r#"["create_presentation","create_document","create_spreadsheet","merge_template"]"#,
            "Create the single approved Office artifact through the restricted tool.",
        ),
        (
            "agent-worker-2b",
            "W-2B",
            "Document Worker",
            "worker",
            r#"["create_presentation","create_document","create_spreadsheet","merge_template"]"#,
            "Create the single approved Office artifact through the restricted tool.",
        ),
        (
            "agent-reviewer",
            "Sentinel",
            "File Reviewer",
            "reviewer",
            r#"["validate_output"]"#,
            "Check only file policy and parseability.",
        ),
    ];
    for (id, name, role, kind, capabilities, prompt) in roster {
        transaction.execute("INSERT OR IGNORE INTO agents (id, name, role, agent_kind, capabilities, max_concurrency, system_prompt, status, trust_level, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 'idle', 'standard', ?7)", params![id, name, role, kind, capabilities, prompt, timestamp])?;
    }
    Ok(())
}

pub fn append_audit(
    connection: &Connection,
    actor: &str,
    action: &str,
    target: Option<&str>,
    detail: Option<&str>,
) -> Result<()> {
    connection.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![actor, action, target, detail, now_ms()])?;
    Ok(())
}

fn append_audit_tx(
    transaction: &Transaction<'_>,
    actor: &str,
    action: &str,
    target: Option<&str>,
    detail: Option<&str>,
    timestamp: i64,
) -> Result<()> {
    transaction.execute("INSERT INTO audit_log (actor, action, target, detail, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![actor, action, target, detail, timestamp])?;
    Ok(())
}
