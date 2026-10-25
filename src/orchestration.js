import { id, typeFromPrompt } from './formatters.js';
import {
  buildArtifact,
  planWorkforce,
  reviewArtifact,
  reviewHandoff,
  runMarketAnalyst,
  runOperationsAnalyst,
  selectBusinessContext
} from './agent-runtime.js';

const MAX_ACTIVITY = 60;
const delay = (ms, fn) => window.setTimeout(fn, ms);

function addActivity(next, text, tone = 'neutral', kind = 'event') {
  next.activity.unshift({ id: id('audit'), at: Date.now(), text, tone, kind });
  next.activity = next.activity.slice(0, MAX_ACTIVITY);
}

function recalculateAgentAvailability(next) {
  const busy = new Set(
    next.missions.flatMap((mission) => mission.tasks)
      .filter((task) => task.status === 'running')
      .map((task) => task.agentId)
  );
  next.agents.forEach((agent) => { agent.status = busy.has(agent.id) ? 'busy' : 'idle'; });
}

function getMission(next, missionId) {
  return next.missions.find((mission) => mission.id === missionId);
}

function updateTask(next, missionId, taskId, status, summary) {
  const mission = getMission(next, missionId);
  const task = mission?.tasks.find((item) => item.id === taskId);
  if (!task) return null;
  task.status = status;
  if (summary) task.summary = summary;
  task.updatedAt = Date.now();
  mission.updatedAt = Date.now();
  recalculateAgentAvailability(next);
  return task;
}

function missionTitle(text, context) {
  const prefix = context.client?.name || (context.clients.length ? 'Active portfolio' : 'Business');
  const lower = text.toLowerCase();
  if (lower.includes('report')) return `${prefix} client report`;
  if (lower.includes('budget') || lower.includes('tracker')) return `${prefix} budget tracker`;
  if (lower.includes('deck') || lower.includes('presentation')) return `${prefix} board deck`;
  return text.length > 52 ? `${text.slice(0, 52)}…` : text;
}

function artifactName(text, type, context) {
  const scope = (context.client?.name || 'active-portfolio').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  const purpose = text.toLowerCase().includes('budget') ? 'budget-tracker' : type === 'pptx' ? 'board-deck' : type === 'docx' ? 'client-report' : 'business-tracker';
  const stamp = new Date().toISOString().replace(/[:T]/g, '-').slice(0, 16);
  return `${scope}_${purpose}_${stamp}.${type}`;
}

function hasActiveMission(state) {
  return state.missions.some((mission) => ['planning', 'running'].includes(mission.status));
}

function taskFor(mission, agentId) {
  return mission?.tasks.find((task) => task.agentId === agentId);
}

/**
 * Runs the managed workforce over actual in-browser business records.
 *
 * The deterministic runtime makes the full task/handoff/provenance model usable
 * today. The same mission contract is designed to be fulfilled by OpenCode once
 * the Tauri sidecar and OfficeCLI bridge are running on the desktop host.
 */
export function startMission(api, text, source = 'user') {
  const trimmed = text.trim();
  if (!trimmed) return { ok: false, reason: 'Enter a business goal before creating a mission.' };
  if (hasActiveMission(api.getState())) return { ok: false, reason: 'One mission is already running. Wait for its workforce to finish.' };

  const originalState = api.getState();
  const context = selectBusinessContext(originalState, trimmed);
  const type = typeFromPrompt(trimmed);
  const plan = planWorkforce({ requirementText: trimmed, fileType: type, context });
  const missionId = id('mission');
  const managerTaskId = id('task-manager');

  api.update((next) => {
    next.missions.unshift({
      id: missionId,
      text: trimmed,
      title: missionTitle(trimmed, context),
      type,
      status: 'planning',
      source,
      runtime: 'local_deterministic',
      context: {
        clientId: context.client?.id || null,
        clientName: context.client?.name || null,
        projectIds: context.projects.map((project) => project.id),
        sourceRecordIds: context.sourceRecordIds
      },
      plan: null,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      reportId: null,
      tasks: [{
        id: managerTaskId,
        agentId: 'agent-manager',
        title: 'Plan a bounded delivery team',
        status: 'queued',
        dependsOn: [],
        summary: 'Waiting to validate the selected business scope.'
      }],
      handoffs: []
    });
    addActivity(next, `Requirement created: “${trimmed}”.`, 'info', 'create_requirement');
    addActivity(next, `Selected ${context.clients.length} client and ${context.projects.length} project record(s) for governed local-agent execution.`, 'neutral', 'query_records');
    if (source === 'schedule') addActivity(next, 'A persisted schedule started this mission.', 'info', 'schedule_fired');
  });

  delay(280, () => api.update((next) => {
    const task = updateTask(next, missionId, managerTaskId, 'running', 'Validating role limits, dependencies, and source scope.');
    if (!task) return;
    addActivity(next, 'Asha is creating a bounded, depth-one workforce plan.', 'info', 'plan_mission');
  }));

  delay(760, () => api.update((next) => {
    const mission = getMission(next, missionId);
    if (!mission) return;
    updateTask(next, missionId, managerTaskId, 'succeeded', plan.summary);
    mission.plan = plan;
    const taskByKey = Object.fromEntries(plan.tasks.map((item) => [item.key, id(`task-${item.key}`)]));
    const marketTask = { id: taskByKey.market_analysis, agentId: 'agent-market', reviewerId: plan.tasks[0].reviewerId, workspace: `workspaces/${missionId}/maya-market-analysis`, title: plan.tasks[0].title, status: 'running', dependsOn: [], summary: `Analyzing ${context.projects.length} selected project record(s).` };
    const opsTask = { id: taskByKey.pipeline_metrics, agentId: 'agent-worker-1', reviewerId: plan.tasks[1].reviewerId, workspace: `workspaces/${missionId}/w1-pipeline-metrics`, title: plan.tasks[1].title, status: 'running', dependsOn: [], summary: 'Calculating current value, stage, and risk facts.' };
    const documentTask = { id: taskByKey.document, agentId: 'agent-worker-2', reviewerId: plan.tasks[2].reviewerId, workspace: `workspaces/${missionId}/w2-document`, title: plan.tasks[2].title, status: 'blocked', dependsOn: [marketTask.id, opsTask.id], summary: 'Waiting for accepted analyst handoffs and peer review.' };
    const reviewTask = { id: taskByKey.review, agentId: 'agent-reviewer', reviewerId: null, workspace: `workspaces/${missionId}/sentinel-validation`, title: plan.tasks[3].title, status: 'blocked', dependsOn: [documentTask.id], summary: 'Waiting for document-worker output.' };
    mission.tasks.push(marketTask, opsTask, documentTask, reviewTask);
    mission.status = 'running';
    recalculateAgentAvailability(next);
    addActivity(next, 'Asha deployed Maya, W-1, W-2, and Sentinel with fixed capabilities and dependencies.', 'success', 'deploy_subagent');
  }));

  delay(1350, () => api.update((next) => {
    const mission = getMission(next, missionId);
    const task = taskFor(mission, 'agent-market');
    if (!mission || !task) return;
    const payload = runMarketAnalyst(context);
    updateTask(next, missionId, task.id, 'review', `${payload.executiveSummary} Awaiting peer review from W-1.`);
    mission.handoffs.push({ id: id('handoff-market'), fromAgentId: 'agent-market', reviewerId: task.reviewerId, type: payload.handoffType, sourceCount: payload.evidenceRecordIds.length, status: 'pending_review', payload });
    addActivity(next, `Maya created a source-linked market_analysis handoff with ${payload.evidenceRecordIds.length} reference(s); W-1 is assigned as peer reviewer.`, 'info', 'create_handoff');
  }));

  delay(1620, () => api.update((next) => {
    const mission = getMission(next, missionId);
    const task = taskFor(mission, 'agent-worker-1');
    if (!mission || !task) return;
    const payload = runOperationsAnalyst(context);
    updateTask(next, missionId, task.id, 'review', `Calculated ${payload.projectCount} projects, ${payload.atRiskCount} at risk, and ${payload.totalPipelineValue.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })} in value. Awaiting peer review from Maya.`);
    mission.handoffs.push({ id: id('handoff-ops'), fromAgentId: 'agent-worker-1', reviewerId: task.reviewerId, type: payload.handoffType, sourceCount: payload.evidenceRecordIds.length, status: 'pending_review', payload });
    addActivity(next, `W-1 created a source-linked pipeline_metrics handoff with ${payload.evidenceRecordIds.length} reference(s); Maya is assigned as peer reviewer.`, 'info', 'create_handoff');
  }));

  delay(1910, () => api.update((next) => {
    const mission = getMission(next, missionId);
    const marketTask = taskFor(mission, 'agent-market');
    const opsTask = taskFor(mission, 'agent-worker-1');
    if (!mission || !marketTask || !opsTask) return;
    const marketHandoff = mission.handoffs.find((handoff) => handoff.type === 'market_analysis');
    const opsHandoff = mission.handoffs.find((handoff) => handoff.type === 'pipeline_metrics');
    const marketReview = reviewHandoff(marketHandoff?.payload);
    const opsReview = reviewHandoff(opsHandoff?.payload);
    if (!marketReview.approved || !opsReview.approved) {
      updateTask(next, missionId, marketTask.id, 'failed', marketReview.issues.join(' '));
      updateTask(next, missionId, opsTask.id, 'failed', opsReview.issues.join(' '));
      mission.status = 'failed';
      addActivity(next, 'Peer review rejected an analyst handoff; downstream document work remains blocked.', 'danger', 'peer_review_failed');
      return;
    }
    marketHandoff.status = 'accepted';
    opsHandoff.status = 'accepted';
    updateTask(next, missionId, marketTask.id, 'approved', 'Peer-reviewed by W-1; handoff accepted.');
    updateTask(next, missionId, opsTask.id, 'approved', 'Peer-reviewed by Maya; handoff accepted.');
    const documentTask = taskFor(mission, 'agent-worker-2');
    if (documentTask) updateTask(next, missionId, documentTask.id, 'running', `Building a ${type.toUpperCase()} from two peer-reviewed, source-linked handoffs.`);
    addActivity(next, 'W-1 peer-reviewed Maya’s market analysis and accepted its cited evidence.', 'success', 'peer_review');
    addActivity(next, 'Maya peer-reviewed W-1’s pipeline metrics and accepted its cited evidence.', 'success', 'peer_review');
    addActivity(next, 'W-2 consumed accepted handoffs; raw agent chat was never passed downstream.', 'info', 'consume_handoff');
  }));

  delay(2570, () => api.update((next) => {
    const mission = getMission(next, missionId);
    const documentTask = taskFor(mission, 'agent-worker-2');
    if (!mission || !documentTask) return;
    const marketAnalysis = mission.handoffs.find((handoff) => handoff.type === 'market_analysis')?.payload;
    const pipelineMetrics = mission.handoffs.find((handoff) => handoff.type === 'pipeline_metrics')?.payload;
    if (!marketAnalysis || !pipelineMetrics) {
      updateTask(next, missionId, documentTask.id, 'failed', 'Required source-linked handoffs were unavailable.');
      mission.status = 'failed';
      addActivity(next, 'W-2 could not start because a required handoff was unavailable.', 'danger', 'handoff_failed');
      return;
    }
    const artifact = buildArtifact({ requirementText: mission.text, fileType: type, context, marketAnalysis, pipelineMetrics });
    updateTask(next, missionId, documentTask.id, 'succeeded', `Built an evidence-backed ${type.toUpperCase()} artifact plan with ${artifact.outline.length} sections.`);
    const reportId = id('report');
    mission.reportId = reportId;
    next.reports.unshift({
      id: reportId,
      missionId,
      name: artifactName(mission.text, type, context),
      type,
      createdAt: Date.now(),
      size: 'local agent plan',
      producer: 'W-2 — Document Worker',
      prototype: true,
      outline: artifact.outline,
      sourceRecordIds: artifact.sourceRecordIds,
      dataSummary: artifact.dataSummary,
      provenance: artifact.provenance
    });
    const reviewTask = taskFor(mission, 'agent-reviewer');
    if (reviewTask) updateTask(next, missionId, reviewTask.id, 'running', 'Checking deliverable type, content outline, and source provenance.');
    addActivity(next, `W-2 created a source-linked ${type.toUpperCase()} artifact plan in Report Center.`, 'info', 'create_file');
  }));

  delay(3150, () => api.update((next) => {
    const mission = getMission(next, missionId);
    const reviewTask = taskFor(mission, 'agent-reviewer');
    const report = next.reports.find((item) => item.id === mission?.reportId);
    if (!mission || !reviewTask || !report) return;
    const validation = reviewArtifact({ report, fileType: mission.type });
    if (validation.approved) {
      updateTask(next, missionId, reviewTask.id, 'approved', `Approved artifact with ${report.sourceRecordIds.length} linked source record(s).`);
      mission.status = 'done';
      addActivity(next, `Sentinel approved the ${mission.type.toUpperCase()} artifact and its source provenance.`, 'success', 'validate_output');
    } else {
      updateTask(next, missionId, reviewTask.id, 'failed', validation.issues.join(' '));
      mission.status = 'failed';
      addActivity(next, `Sentinel rejected the artifact: ${validation.issues.join(' ')}`, 'danger', 'validate_output');
    }
    mission.updatedAt = Date.now();
    recalculateAgentAvailability(next);
  }));

  return { ok: true, missionId };
}

export function saveSchedule(api, { prompt, cron, type, timezone }) {
  if (!prompt.trim() || !cron.trim()) return { ok: false, reason: 'Mission prompt and cron expression are required.' };
  api.update((next) => {
    next.schedules.unshift({
      id: id('schedule'),
      prompt: prompt.trim(),
      type,
      cron: cron.trim(),
      timezone: timezone || next.settings.timezone,
      enabled: true,
      lastRun: null,
      nextRun: Date.now() + 24 * 60 * 60_000
    });
    addActivity(next, `Saved enabled ${type.toUpperCase()} schedule with cron “${cron.trim()}”.`, 'success', 'schedule_created');
  });
  return { ok: true };
}

export function toggleSchedule(api, scheduleId) {
  api.update((next) => {
    const schedule = next.schedules.find((item) => item.id === scheduleId);
    if (!schedule) return;
    schedule.enabled = !schedule.enabled;
    addActivity(next, `${schedule.enabled ? 'Enabled' : 'Paused'} schedule “${schedule.prompt}”.`, schedule.enabled ? 'success' : 'warning', 'schedule_updated');
  });
}
