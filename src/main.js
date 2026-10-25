import { createStore } from './store.js';
import { createSeedState } from './seed.js';
import { createPrototypeApi } from './api.js';
import { startMission, saveSchedule, toggleSchedule } from './orchestration.js';
import { escapeHtml, fileIcon, formatCurrency, formatDate, formatDateTime, id, relativeTime, statusTone } from './formatters.js';

const root = document.querySelector('#app');
const store = createStore(createSeedState);
const api = createPrototypeApi(store);

const navItems = [
  ['missions', 'Missions', '◈'],
  ['workforce', 'Workforce', '◎'],
  ['reports', 'Reports', '▱'],
  ['records', 'Records', '▦'],
  ['settings', 'Settings', '⚙']
];

function getAgent(state, id) {
  return state.agents.find((agent) => agent.id === id);
}

function statusPill(status) {
  const label = status.replaceAll('_', ' ');
  return `<span class="pill ${statusTone(status)}">${escapeHtml(label)}</span>`;
}

function initials(name) {
  return name.split(/\s+/).map((part) => part[0]).join('').slice(0, 2);
}

function metric(label, value, accent = '') {
  return `<article class="metric ${accent}"><span>${escapeHtml(label)}</span><strong>${value}</strong></article>`;
}

function currentWeekReports(state) {
  const monday = new Date();
  monday.setHours(0, 0, 0, 0);
  monday.setDate(monday.getDate() - ((monday.getDay() + 6) % 7));
  return state.reports.filter((report) => report.createdAt >= monday.getTime()).length;
}

function activeMission(state) {
  return state.missions.find((mission) => ['planning', 'running'].includes(mission.status)) || state.missions[0];
}

function renderHeader(state) {
  const engine = state.engine;
  return `<header class="topbar">
    <div class="brand"><span class="brand-mark">K</span><span>KUBERA <b>AI</b></span><em>local operations</em></div>
    <div class="engine-strip">
      <span class="engine-state ${statusTone(engine.status)}"><i></i>Engine: ${escapeHtml(engine.status)}</span>
      <span class="model">Model: <b>${escapeHtml(engine.model)}</b></span>
      <span class="patrol"><i class="pulse"></i>Patrol: ${engine.patrol ? 'active' : 'paused'}</span>
      <button class="icon-button" data-action="nav" data-route="settings" aria-label="Open settings">⚙</button>
    </div>
  </header>`;
}

function renderSidebar(state) {
  const active = state.missions.filter((mission) => ['planning', 'running'].includes(mission.status)).length;
  const upcoming = state.schedules.filter((schedule) => schedule.enabled && schedule.nextRun > Date.now()).length;
  const reviews = state.missions.flatMap((mission) => mission.tasks).filter((task) => task.status === 'review').length;
  return `<aside class="sidebar">
    <nav aria-label="Primary navigation" class="nav-list">
      ${navItems.map(([route, label, icon]) => `<button class="nav-item ${state.route === route ? 'active' : ''}" data-action="nav" data-route="${route}"><span>${icon}</span>${label}</button>`).join('')}
    </nav>
    <section class="metrics-panel">
      <p class="eyebrow">Mission pulse</p>
      ${metric('Active missions', active, 'blue')}
      ${metric('Reports this week', currentWeekReports(state), 'green')}
      ${metric('Upcoming schedules', upcoming, 'amber')}
      ${metric('Peer reviews pending', reviews, 'rose')}
    </section>
    <section class="sidebar-note">
      <span class="note-dot"></span>
      <div><b>Controlled workforce</b><small>Manager-led, depth-one deployment.</small></div>
    </section>
  </aside>`;
}

function renderRequirementComposer(state) {
  const disabled = state.engine.status !== 'ready' ? 'disabled' : '';
  return `<section class="composer-card">
    <div class="composer-heading"><div><p class="eyebrow">New business mission</p><h1>What should KUBERA deliver?</h1></div><span class="prototype-tag">Local agent runtime</span></div>
    <form data-form="mission" class="composer-form">
      <textarea name="missionText" rows="2" ${disabled} placeholder="Build a Q3 pipeline deck for Acme Corp"></textarea>
      <div class="composer-footer">
        <div class="examples"><span>Try</span><button type="button" data-action="example" data-prompt="Build a Q3 pipeline deck for Acme Corp">Board deck</button><button type="button" data-action="example" data-prompt="Write a Q3 client report for Acme using the client report template">Client report</button><button type="button" data-action="example" data-prompt="Create a campaign budget tracker for active projects">Budget tracker</button></div>
        <button class="primary-button" type="submit" ${disabled}><span>Deploy mission</span><b>→</b></button>
      </div>
    </form>
    <p class="composer-note">Asha, Maya, W-1, W-2, and Sentinel execute a deterministic, source-linked workflow over the local records. Desktop mode uses the same contracts with OpenCode and OfficeCLI.</p>
  </section>`;
}

function renderTask(task, state) {
  const agent = getAgent(state, task.agentId) || { name: 'Unknown', label: 'Unassigned', color: 'slate' };
  const reviewer = task.reviewerId ? getAgent(state, task.reviewerId) : null;
  const dependencies = task.dependsOn?.length ? `${task.dependsOn.length} prerequisite${task.dependsOn.length > 1 ? 's' : ''}` : 'Ready root task';
  return `<article class="task-card ${task.status}">
    <div class="agent-avatar ${agent.color}">${escapeHtml(initials(agent.name))}</div>
    <div class="task-copy"><div class="task-title-row"><strong>${escapeHtml(agent.name)} <span>— ${escapeHtml(agent.label)}</span></strong>${statusPill(task.status)}</div><p>${escapeHtml(task.title)}</p><small>${escapeHtml(task.summary || dependencies)}</small>${reviewer ? `<span class="task-reviewer">Peer reviewer: ${escapeHtml(reviewer.name)} — ${escapeHtml(reviewer.label)}</span>` : ''}${task.workspace ? `<span class="task-workspace">⌁ ${escapeHtml(task.workspace)}</span>` : ''}</div>
  </article>`;
}

function renderTeamBoard(mission, state) {
  const columns = [
    { key: 'queued', label: 'Queued', statuses: ['queued', 'blocked'] },
    { key: 'working', label: 'Working', statuses: ['running'] },
    { key: 'review', label: 'Peer review', statuses: ['review'] },
    { key: 'delivered', label: 'Delivered', statuses: ['succeeded'] },
    { key: 'approved', label: 'Approved', statuses: ['approved'] }
  ];
  return `<section class="team-board" aria-label="Mission task board"><div class="board-topline"><div><p class="eyebrow">Team board</p><h3>${mission.tasks.length} tasks · ${mission.handoffs.filter((handoff) => handoff.status === 'accepted').length} accepted handoffs</h3></div><span>Independent work runs in parallel; document work starts only after peer review.</span></div><div class="board-columns">${columns.map((column) => { const tasks = mission.tasks.filter((task) => column.statuses.includes(task.status)); return `<div class="board-column ${column.key}"><div class="board-column-title"><span>${escapeHtml(column.label)}</span><b>${tasks.length}</b></div><div class="board-task-list">${tasks.length ? tasks.map((task) => renderTask(task, state)).join('') : '<span class="empty-board-slot">—</span>'}</div></div>`; }).join('')}</div></section>`;
}

function renderMissionCard(mission, state, compact = false) {
  const completed = mission.tasks.filter((task) => ['succeeded', 'approved'].includes(task.status)).length;
  const running = mission.tasks.find((task) => task.status === 'running');
  return `<article class="mission-card ${mission.status}" data-mission-id="${mission.id}">
    <div class="mission-card-top"><div><span class="file-type ${mission.type}">${mission.type.toUpperCase()}</span><h3>${escapeHtml(mission.title)}</h3><p>${escapeHtml(mission.text)}</p></div>${statusPill(mission.status)}</div>
    <div class="mission-meta"><span>${completed}/${mission.tasks.length} workforce steps completed</span><span>${relativeTime(mission.updatedAt)}</span></div>
    ${running ? `<div class="running-line"><i></i><span>${escapeHtml((getAgent(state, running.agentId) || {}).name || 'Agent')} is working: ${escapeHtml(running.title)}</span></div>` : ''}
    ${mission.status === 'done' && mission.reportId ? `<div class="mission-success"><span>✓</span><div><b>Artifact is ready</b><small>Validated by Sentinel and registered in Report Center.</small></div><button data-action="open-report" data-report-id="${mission.reportId}">Preview</button></div>` : ''}
    ${compact ? '' : `<button class="text-button" data-action="select-mission" data-mission-id="${mission.id}">View deployment →</button>`}
  </article>`;
}

function renderScheduleForm(state) {
  if (!state.ui?.scheduleOpen) return '';
  return `<form class="schedule-form" data-form="schedule">
    <div class="form-title"><div><p class="eyebrow">Recurring mission</p><h3>Schedule a deliverable</h3></div><button type="button" class="close-button" data-action="hide-schedule" aria-label="Close schedule form">×</button></div>
    <label>Stored business goal<textarea name="schedulePrompt" rows="2" placeholder="Build the monthly Acme pipeline board deck from current records" required></textarea></label>
    <div class="form-grid"><label>Cron expression<input name="cron" value="0 9 1 * *" required /></label><label>Output<select name="type"><option value="pptx">PPTX</option><option value="docx">DOCX</option><option value="xlsx">XLSX</option></select></label></div>
    <div class="form-grid"><label>Timezone<input name="timezone" value="${escapeHtml(state.settings.timezone)}" required /></label><div class="next-run-preview"><span>Prototype next run</span><b>Tomorrow, 09:00</b></div></div>
    <div class="form-actions"><button type="button" class="ghost-button" data-action="hide-schedule">Cancel</button><button class="primary-button" type="submit">Save schedule</button></div>
  </form>`;
}

function renderMissions(state) {
  const missions = state.missions;
  return `<section class="view missions-view">
    ${renderRequirementComposer(state)}
    <div class="section-heading"><div><p class="eyebrow">Mission queue</p><h2>Business outcomes in motion</h2></div><button class="secondary-button" data-action="show-schedule">＋ Schedule mission</button></div>
    ${renderScheduleForm(state)}
    <div class="mission-grid">${missions.length ? missions.map((mission) => renderMissionCard(mission, state)).join('') : `<div class="empty-state"><span>▱</span><h3>No missions yet</h3><p>Ask KUBERA to build a business deliverable from your local records.</p></div>`}</div>
    <section class="schedule-section"><div class="section-heading compact"><div><p class="eyebrow">Patrol schedule</p><h2>Recurring operations</h2></div><span class="caption">Runs while desktop app is open</span></div>${renderSchedules(state)}</section>
  </section>`;
}

function renderSchedules(state) {
  if (!state.schedules.length) return `<div class="empty-inline">No schedules saved. Create a recurring mission for current business records.</div>`;
  return `<div class="schedule-list">${state.schedules.map((schedule) => `<article class="schedule-row"><div class="calendar-icon">◷</div><div class="schedule-copy"><strong>${escapeHtml(schedule.prompt)}</strong><span><code>${escapeHtml(schedule.cron)}</code> · ${escapeHtml(schedule.timezone)} · Next ${formatDateTime(schedule.nextRun)}</span></div><span class="toggle-label ${schedule.enabled ? 'on' : ''}">${schedule.enabled ? 'Enabled' : 'Paused'}</span><button class="small-button" data-action="toggle-schedule" data-schedule-id="${schedule.id}">${schedule.enabled ? 'Pause' : 'Enable'}</button><button class="small-button" data-action="run-schedule" data-schedule-id="${schedule.id}">Run now</button></article>`).join('')}</div>`;
}

function renderWorkforce(state) {
  const selectedId = state.ui?.selectedMissionId;
  const mission = state.missions.find((item) => item.id === selectedId) || activeMission(state);
  const runningCount = mission?.tasks.filter((task) => task.status === 'running').length || 0;
  return `<section class="view workforce-view">
    <div class="section-heading"><div><p class="eyebrow">Managed deployment</p><h1>Agent workforce</h1><p class="subhead">Every deployment is durable, bounded, and tied to a business outcome.</p></div><div class="capacity"><span>${runningCount}/2</span><small>specialist sessions active</small></div></div>
    <section class="roster-grid">${state.agents.map((agent) => `<article class="roster-card"><div class="agent-avatar ${agent.color}">${escapeHtml(initials(agent.name))}</div><div><strong>${escapeHtml(agent.name)}</strong><span>${escapeHtml(agent.label)}</span><small>${escapeHtml(agent.capability)}</small></div>${statusPill(agent.status)}</article>`).join('')}</section>
    ${mission ? `<section class="deployment-panel"><div class="deployment-heading"><div><p class="eyebrow">Selected mission</p><h2>${escapeHtml(mission.title)}</h2><p>${escapeHtml(mission.text)}</p></div>${statusPill(mission.status)}</div>${renderTeamBoard(mission, state)}<div class="deployment-flow">${mission.tasks.map((task, index) => `<div class="flow-node ${task.status}"><span class="flow-number">${index + 1}</span>${renderTask(task, state)}</div>${index === 0 || index === 2 ? '<div class="flow-connector">↓</div>' : ''}`).join('')}</div><div class="handoff-row"><span class="eyebrow">Structured handoffs</span>${mission.handoffs.length ? mission.handoffs.map((handoff) => { const agent = getAgent(state, handoff.fromAgentId); const reviewer = getAgent(state, handoff.reviewerId); return `<span class="handoff-chip ${handoff.status}">${escapeHtml(agent?.name || 'Agent')} → ${escapeHtml(handoff.type)} · ${handoff.sourceCount} sources · ${handoff.status === 'accepted' ? 'peer approved' : `awaiting ${reviewer?.name || 'review'}`}</span>`; }).join('') : '<span class="caption">Handoffs will appear after analyst work succeeds.</span>'}</div></section>` : `<div class="empty-state"><span>◎</span><h3>No workforce deployment selected</h3><p>Create a mission to see Asha deploy the manager-led team.</p></div>`}
    <section class="rules-card"><strong>Deployment guardrails</strong><span>Maximum four child tasks</span><span>Depth-one hierarchy</span><span>No shell or raw SQL tools</span><span>Structured source-linked handoffs</span></section>
  </section>`;
}

function renderReports(state) {
  return `<section class="view reports-view"><div class="section-heading"><div><p class="eyebrow">Report center</p><h1>Business artifacts</h1><p class="subhead">Files are the primary outcome. Prototype previews show artifact metadata; the Tauri build will open real Office files.</p></div><span class="report-count">${state.reports.length} files</span></div>
  ${state.reports.length ? `<div class="report-table"><div class="report-head"><span>Artifact</span><span>Mission</span><span>Produced by</span><span>Created</span><span></span></div>${state.reports.map((report) => { const mission = state.missions.find((item) => item.id === report.missionId); return `<article class="report-row"><div class="artifact"><span class="artifact-icon ${report.type}">${fileIcon(report.type)}</span><div><strong>${escapeHtml(report.name)}</strong><small>${report.type.toUpperCase()} · ${escapeHtml(report.size)} ${report.prototype ? '· prototype preview' : ''}</small></div></div><span>${escapeHtml(mission?.title || 'Mission')}</span><span>${escapeHtml(report.producer)}</span><span>${formatDateTime(report.createdAt)}</span><button class="small-button" data-action="open-report" data-report-id="${report.id}">Preview</button></article>`; }).join('')}</div>` : `<div class="empty-state"><span>▱</span><h3>No reports yet</h3><p>Ask KUBERA to build a document from your business records.</p></div>`}</section>`;
}

function renderRecordForm(state) {
  const mode = state.ui?.recordForm;
  if (!mode) return '';
  if (mode === 'client') return `<form class="record-form-inline" data-form="record"><input type="hidden" name="recordType" value="client" /><div class="form-title"><div><p class="eyebrow">Add business record</p><h3>New client</h3></div><button type="button" class="close-button" data-action="hide-record-form">×</button></div><div class="form-grid"><label>Client name<input name="clientName" required placeholder="Orchid Labs" /></label><label>Industry<input name="industry" required placeholder="Technology" /></label></div><div class="form-grid"><label>Status<select name="status"><option value="active">Active</option><option value="prospect">Prospect</option></select></label><label>Owner<input name="owner" required placeholder="Account owner" /></label></div><div class="form-actions"><button type="button" class="ghost-button" data-action="hide-record-form">Cancel</button><button class="primary-button" type="submit">Add client</button></div></form>`;
  return `<form class="record-form-inline" data-form="record"><input type="hidden" name="recordType" value="project" /><div class="form-title"><div><p class="eyebrow">Add business record</p><h3>New project</h3></div><button type="button" class="close-button" data-action="hide-record-form">×</button></div><div class="form-grid"><label>Project name<input name="projectName" required placeholder="Expansion initiative" /></label><label>Client<select name="clientId">${state.clients.map((client) => `<option value="${client.id}">${escapeHtml(client.name)}</option>`).join('')}</select></label></div><div class="form-grid"><label>Stage<select name="stage"><option>Discovery</option><option>Proposal</option><option>Negotiation</option><option>Won</option></select></label><label>Value (USD)<input name="value" type="number" min="0" value="50000" required /></label></div><label class="check-label"><input name="risk" type="checkbox" /> Flag as at risk</label><div class="form-actions"><button type="button" class="ghost-button" data-action="hide-record-form">Cancel</button><button class="primary-button" type="submit">Add project</button></div></form>`;
}

function renderRecords(state) {
  const projectSort = state.ui?.projectSort || 'value';
  const projects = [...state.projects].sort((a, b) => projectSort === 'name' ? a.name.localeCompare(b.name) : b.value - a.value);
  return `<section class="view records-view"><div class="section-heading"><div><p class="eyebrow">Local business context</p><h1>Records</h1><p class="subhead">Durable CRM-lite records. Agents receive only the scoped fields needed for a mission.</p></div><div class="heading-actions"><button class="secondary-button" data-action="show-client-form">＋ Client</button><button class="secondary-button" data-action="show-project-form">＋ Project</button></div></div>${renderRecordForm(state)}
  <section class="records-section"><div class="table-title"><h2>Clients</h2><span>${state.clients.length} records</span></div><div class="data-table"><div class="data-head clients"><span>Name</span><span>Industry</span><span>Status</span><span>Owner</span></div>${state.clients.map((client) => `<div class="data-row clients"><strong>${escapeHtml(client.name)}</strong><span>${escapeHtml(client.industry)}</span><span>${statusPill(client.status)}</span><span>${escapeHtml(client.owner)}</span></div>`).join('')}</div></section>
  <section class="records-section"><div class="table-title"><h2>Projects</h2><div><button class="sort-button ${projectSort === 'value' ? 'selected' : ''}" data-action="sort-projects" data-sort="value">Value</button><button class="sort-button ${projectSort === 'name' ? 'selected' : ''}" data-action="sort-projects" data-sort="name">Name</button></div></div><div class="data-table"><div class="data-head projects"><span>Project</span><span>Client</span><span>Stage</span><span>Value</span><span>Risk</span></div>${projects.map((project) => { const client = state.clients.find((item) => item.id === project.clientId); return `<div class="data-row projects"><strong>${escapeHtml(project.name)}</strong><span>${escapeHtml(client?.name || 'Unknown')}</span><span><span class="stage">${escapeHtml(project.stage)}</span></span><span>${formatCurrency(project.value)}</span><span>${project.risk ? '<span class="risk">At risk</span>' : '<span class="low-risk">On track</span>'}</span></div>`; }).join('')}</div></section>
  </section>`;
}

function renderSettings(state) {
  return `<section class="view settings-view"><div class="section-heading"><div><p class="eyebrow">Environment</p><h1>Settings</h1><p class="subhead">Prototype values are stored in this browser only. A desktop build stores secrets through a secure OS-backed mechanism.</p></div></div>
  <div class="settings-grid"><form class="settings-card" data-form="settings"><div class="card-title"><span class="card-icon">⌁</span><div><h2>Model connection</h2><p>OpenAI-compatible provider details</p></div></div><label>Provider base URL<input name="providerBaseUrl" value="${escapeHtml(state.settings.providerBaseUrl)}" /></label><label>Model<input name="model" value="${escapeHtml(state.settings.model)}" /></label><label>API key<input type="password" value="••••••••••••••••" disabled aria-label="API key is masked in prototype" /></label><button class="primary-button" type="submit">Save local settings</button></form>
  <section class="settings-card"><div class="card-title"><span class="card-icon">◷</span><div><h2>Operations</h2><p>Runtime and persistence controls</p></div></div><div class="key-value"><span>Engine state</span>${statusPill(state.engine.status)}</div><div class="key-value"><span>Active runtime</span><b>Local deterministic agents</b></div><div class="key-value"><span>Default timezone</span><b>${escapeHtml(state.settings.timezone)}</b></div><div class="key-value"><span>Persistence</span><b>Browser localStorage</b></div><div class="key-value"><span>Desktop target</span><b>Tauri + SQLite</b></div><button class="danger-button" data-action="reset-demo">Reset demo data</button></section>
  <section class="settings-card template-card"><div class="card-title"><span class="card-icon">▧</span><div><h2>Template library</h2><p>Approved Office starting points</p></div></div><div class="template-list">${(state.templates || []).map((template) => `<div class="template-row"><span class="file-type ${template.type}">${template.type.toUpperCase()}</span><div><b>${escapeHtml(template.name)}</b><small>${escapeHtml(template.fileName)} · ${template.placeholders} placeholders</small></div></div>`).join('') || '<span class="caption">No templates imported.</span>'}</div><label class="file-picker">Import Office template<input id="template-file" type="file" accept=".docx,.pptx,.xlsx" /></label><small class="template-note">Prototype stores template metadata only. Desktop mode copies approved files into the local template root.</small></section>
  <section class="settings-card docs-card"><div class="card-title"><span class="card-icon">▤</span><div><h2>Implementation hand-off</h2><p>What changes when Rust is available</p></div></div><ul><li>Replace localStorage adapter with typed Tauri IPC commands.</li><li>Start OpenCode loopback sidecar and read SSE sessions.</li><li>Persist tasks, agent runs, handoffs, and audit log in SQLite.</li><li>Route W-2’s typed action through restricted OfficeCLI.</li></ul></section></div></section>`;
}

function renderActivity(state) {
  return `<aside class="activity-panel"><div class="activity-header"><div><p class="eyebrow">Traceability</p><h2>Activity</h2></div><span class="live-dot">Live</span></div><p class="activity-intro">A compact audit feed—not raw agent chatter.</p><div class="activity-list">${state.activity.slice(0, 12).map((event) => `<article class="activity-event"><span class="event-dot ${event.tone}"></span><div><p>${escapeHtml(event.text)}</p><small>${relativeTime(event.at)} · ${escapeHtml(event.kind.replaceAll('_', ' '))}</small></div></article>`).join('')}</div><div class="activity-foot">All subagent plans, handoffs, and files require a durable audit entry.</div></aside>`;
}

function renderModal(state) {
  const reportId = state.ui?.modalReportId;
  if (!reportId) return '';
  const report = state.reports.find((item) => item.id === reportId);
  if (!report) return '';
  return `<div class="modal-backdrop" role="presentation"><section class="modal" role="dialog" aria-modal="true" aria-labelledby="artifact-title"><button class="close-button modal-close" data-action="close-report" aria-label="Close artifact preview">×</button><span class="file-type ${report.type}">${report.type.toUpperCase()} · Prototype preview</span><h2 id="artifact-title">${escapeHtml(report.name)}</h2><p>This browser implementation visualizes the registered artifact outline. The desktop OfficeCLI bridge will create and open a native ${report.type.toUpperCase()} file.</p><div class="outline-preview">${report.outline.map((item, index) => `<div><span>${String(index + 1).padStart(2, '0')}</span><strong>${escapeHtml(item)}</strong></div>`).join('')}</div>${report.dataSummary ? `<div class="provenance-summary"><div><span>Pipeline value</span><b>${formatCurrency(report.dataSummary.pipelineValue)}</b></div><div><span>Projects</span><b>${report.dataSummary.projectCount}</b></div><div><span>At risk</span><b>${report.dataSummary.atRiskCount}</b></div><div><span>Linked sources</span><b>${report.sourceRecordIds?.length || 0}</b></div></div>` : ''}<div class="modal-footer"><span>Produced by ${escapeHtml(report.producer)} · ${formatDateTime(report.createdAt)}</span><button class="primary-button" data-action="close-report">Done</button></div></section></div>`;
}

function render(state) {
  let content = '';
  if (state.route === 'missions') content = renderMissions(state);
  if (state.route === 'workforce') content = renderWorkforce(state);
  if (state.route === 'reports') content = renderReports(state);
  if (state.route === 'records') content = renderRecords(state);
  if (state.route === 'settings') content = renderSettings(state);
  root.innerHTML = `<div class="app-shell">${renderHeader(state)}<div class="workspace">${renderSidebar(state)}<main class="main-panel">${content}</main>${renderActivity(state)}</div></div>${renderModal(state)}`;
}

function toast(message, tone = 'info') {
  const region = document.querySelector('#toast-region');
  const item = document.createElement('div');
  item.className = `toast ${tone}`;
  item.textContent = message;
  region.append(item);
  window.setTimeout(() => item.remove(), 3600);
}

root.addEventListener('click', (event) => {
  const actionTarget = event.target.closest('[data-action]');
  if (!actionTarget) return;
  const { action } = actionTarget.dataset;

  if (action === 'nav') {
    api.update((next) => { next.route = actionTarget.dataset.route; });
  }
  if (action === 'example') {
    const input = root.querySelector('[name="missionText"]');
    if (input) { input.value = actionTarget.dataset.prompt; input.focus(); }
  }
  if (action === 'select-mission') {
    api.update((next) => { next.ui = { ...(next.ui || {}), selectedMissionId: actionTarget.dataset.missionId }; next.route = 'workforce'; });
  }
  if (action === 'open-report') {
    api.update((next) => { next.ui = { ...(next.ui || {}), modalReportId: actionTarget.dataset.reportId }; });
  }
  if (action === 'close-report') {
    api.update((next) => { next.ui = { ...(next.ui || {}), modalReportId: null }; });
  }
  if (action === 'show-schedule') {
    api.update((next) => { next.ui = { ...(next.ui || {}), scheduleOpen: true }; });
  }
  if (action === 'hide-schedule') {
    api.update((next) => { next.ui = { ...(next.ui || {}), scheduleOpen: false }; });
  }
  if (action === 'toggle-schedule') {
    toggleSchedule(api, actionTarget.dataset.scheduleId);
  }
  if (action === 'run-schedule') {
    const schedule = api.getState().schedules.find((item) => item.id === actionTarget.dataset.scheduleId);
    const result = schedule && startMission(api, schedule.prompt, 'schedule');
    if (!result?.ok) toast(result?.reason || 'Unable to run schedule.', 'danger');
    else toast('Scheduled mission deployed in prototype mode.', 'success');
  }
  if (action === 'sort-projects') {
    api.update((next) => { next.ui = { ...(next.ui || {}), projectSort: actionTarget.dataset.sort }; });
  }
  if (action === 'show-client-form') {
    api.update((next) => { next.ui = { ...(next.ui || {}), recordForm: 'client' }; });
  }
  if (action === 'show-project-form') {
    api.update((next) => { next.ui = { ...(next.ui || {}), recordForm: 'project' }; });
  }
  if (action === 'hide-record-form') {
    api.update((next) => { next.ui = { ...(next.ui || {}), recordForm: null }; });
  }
  if (action === 'reset-demo') {
    if (window.confirm('Reset all local prototype missions, reports, schedules, and settings?')) {
      api.reset();
      toast('Prototype data reset to the Acme demo state.', 'success');
    }
  }
});

root.addEventListener('submit', (event) => {
  const form = event.target;
  event.preventDefault();
  if (form.dataset.form === 'mission') {
    const text = new FormData(form).get('missionText') || '';
    const result = startMission(api, String(text));
    if (!result.ok) toast(result.reason, 'danger');
    else toast('Mission deployed: Asha is assembling the workforce.', 'success');
  }
  if (form.dataset.form === 'schedule') {
    const data = new FormData(form);
    const result = saveSchedule(api, {
      prompt: String(data.get('schedulePrompt') || ''),
      cron: String(data.get('cron') || ''),
      type: String(data.get('type') || 'pptx'),
      timezone: String(data.get('timezone') || '')
    });
    if (!result.ok) toast(result.reason, 'danger');
    else {
      api.update((next) => { next.ui = { ...(next.ui || {}), scheduleOpen: false }; });
      toast('Recurring mission saved and enabled.', 'success');
    }
  }
  if (form.dataset.form === 'record') {
    const data = new FormData(form);
    const recordType = String(data.get('recordType'));
    if (recordType === 'client') {
      const name = String(data.get('clientName') || '').trim();
      if (!name) { toast('Client name is required.', 'danger'); return; }
      api.update((next) => {
        next.clients.push({ id: id('client'), name, industry: String(data.get('industry') || '').trim(), status: String(data.get('status') || 'active'), owner: String(data.get('owner') || '').trim(), createdAt: Date.now() });
        next.activity.unshift({ id: id('audit'), at: Date.now(), kind: 'create_client', tone: 'success', text: `Added client record for ${name}.` });
        next.ui = { ...(next.ui || {}), recordForm: null };
      });
      toast('Client record added to local prototype data.', 'success');
    }
    if (recordType === 'project') {
      const name = String(data.get('projectName') || '').trim();
      const value = Number(data.get('value') || 0);
      if (!name || !data.get('clientId') || value < 0) { toast('Provide a project name, client, and valid value.', 'danger'); return; }
      api.update((next) => {
        next.projects.push({ id: id('project'), clientId: String(data.get('clientId')), name, stage: String(data.get('stage') || 'Discovery'), value, risk: data.get('risk') === 'on', createdAt: Date.now() });
        next.activity.unshift({ id: id('audit'), at: Date.now(), kind: 'create_project', tone: 'success', text: `Added project record for ${name}.` });
        next.ui = { ...(next.ui || {}), recordForm: null };
      });
      toast('Project record added to local prototype data.', 'success');
    }
  }
  if (form.dataset.form === 'settings') {
    const data = new FormData(form);
    api.update((next) => {
      next.settings.providerBaseUrl = String(data.get('providerBaseUrl') || '');
      next.settings.model = String(data.get('model') || '');
      next.engine.model = next.settings.model || next.engine.model;
      next.activity.unshift({ id: `audit-${Date.now()}`, at: Date.now(), kind: 'settings_updated', tone: 'success', text: 'Updated local prototype provider/model settings.' });
    });
    toast('Local prototype settings saved.', 'success');
  }
});

root.addEventListener('change', (event) => {
  const input = event.target;
  if (input.id !== 'template-file' || !input.files?.[0]) return;
  const file = input.files[0];
  const extension = file.name.split('.').pop()?.toLowerCase();
  if (!['docx', 'pptx', 'xlsx'].includes(extension)) { toast('Choose a DOCX, PPTX, or XLSX template.', 'danger'); return; }
  api.update((next) => {
    next.templates = next.templates || [];
    next.templates.unshift({ id: id('template'), name: file.name.replace(/\.[^.]+$/, '').replaceAll(/[-_]/g, ' '), fileName: file.name, type: extension, placeholders: extension === 'docx' ? 0 : 0, createdAt: Date.now() });
    next.activity.unshift({ id: id('audit'), at: Date.now(), kind: 'import_template', tone: 'success', text: `Imported ${file.name} into the prototype template library.` });
  });
  toast('Template metadata imported. Desktop mode will copy the approved file locally.', 'success');
});

store.subscribe(render);
api.bootstrap().then(() => render(api.getState()));
