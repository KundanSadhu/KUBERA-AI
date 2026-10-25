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

function mirror(api, operation, ...args) {
  if (typeof api[operation] !== 'function') return;
  Promise.resolve(api[operation](...args)).catch((error) => console.warn(`Desktop persistence failed for ${operation}`, error));
}

function finishDurableRun(api, task, outcome = 'succeeded', afterFinish = null) {
  if (typeof api.finishTaskRun !== 'function') {
    if (afterFinish) afterFinish();
    else mirror(api, 'persistTaskStatus', task);
    return;
  }
  Promise.resolve(api.finishTaskRun(task, outcome, task.summary || null))
    .then(() => { if (afterFinish) afterFinish(); })
    .catch((error) => {
      console.warn('Desktop agent-run completion failed', error);
      if (afterFinish) afterFinish();
      else mirror(api, 'persistTaskStatus', task);
    });
}

function startDurableRun(api, task) {
  if (typeof api.startTaskRun !== 'function') {
    mirror(api, 'persistTaskStatus', task);
    return;
  }
  Promise.resolve(api.startTaskRun(task)).catch((error) => {
    console.warn('Desktop agent-run start failed', error);
    mirror(api, 'persistTaskStatus', task);
  });
}

function completeArtifact(next, mission, documentTask, artifact, reportMeta) {
  updateTask(next, mission.id, documentTask.id, 'succeeded', `Built an evidence-backed ${mission.type.toUpperCase()} artifact with ${artifact.outline.length} sections.`);
  mission.reportId = reportMeta.id;
  next.reports.unshift({
    id: reportMeta.id,
    missionId: mission.id,
    name: reportMeta.name,
    type: mission.type,
    createdAt: Date.now(),
    size: reportMeta.size,
    producer: 'W-2 — Document Worker',
    prototype: reportMeta.prototype,
    outline: artifact.outline,
    sourceRecordIds: artifact.sourceRecordIds,
    dataSummary: artifact.dataSummary,
    provenance: artifact.provenance
  });
  const reviewTask = taskFor(mission, 'agent-reviewer');
  if (reviewTask) updateTask(next, mission.id, reviewTask.id, 'running', 'Checking deliverable type, content outline, and source provenance.');
  addActivity(next, `W-2 created a source-linked ${mission.type.toUpperCase()} artifact in Report Center.`, 'info', 'create_file');
  return reviewTask;
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
  let durableSetup = Promise.resolve();

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

  const createdMission = api.getState().missions.find((mission) => mission.id === missionId);
  if (createdMission && typeof api.persistRequirement === 'function') {
    durableSetup = api.persistRequirement(createdMission)
      .then(() => api.persistTasks(createdMission, createdMission.tasks))
      .catch((error) => console.warn('Desktop requirement persistence failed', error));
  }

  delay(280, () => api.update((next) => {
    const task = updateTask(next, missionId, managerTaskId, 'running', 'Validating role limits, dependencies, and source scope.');
    if (!task) return;
    addActivity(next, 'Asha is creating a bounded, depth-one workforce plan.', 'info', 'plan_mission');
    if (typeof api.startManagerSession !== 'function') {
      mirror(api, 'persistTaskStatus', task);
      return;
    }
    const mission = getMission(next, missionId);
    durableSetup
      .then(() => api.startManagerSession({ task, mission, context }))
      .then(({ sessionId }) => api.update((state) => addActivity(state, `Asha completed a bounded OpenCode manager session (${sessionId.slice(0, 8)}…).`, 'success', 'opencode_session')))
      .catch((error) => api.update((state) => addActivity(state, `OpenCode manager session was unavailable; governed local execution continued. ${error.message || ''}`, 'warning', 'opencode_session')));
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
    mirror(api, 'persistMissionStatus', mission);
    mirror(api, 'persistTaskStatus', mission.tasks[0]);
    mirror(api, 'persistTasks', mission, [marketTask, opsTask, documentTask, reviewTask]);
  }));

  delay(1350, () => api.update((next) => {
    const mission = getMission(next, missionId);
    const task = taskFor(mission, 'agent-market');
    if (!mission || !task) return;
    const payload = runMarketAnalyst(context);
    updateTask(next, missionId, task.id, 'review', `${payload.executiveSummary} Awaiting peer review from W-1.`);
    mission.handoffs.push({ id: id('handoff-market'), fromAgentId: 'agent-market', reviewerId: task.reviewerId, type: payload.handoffType, sourceCount: payload.evidenceRecordIds.length, status: 'pending_review', payload });
    addActivity(next, `Maya created a source-linked market_analysis handoff with ${payload.evidenceRecordIds.length} reference(s); W-1 is assigned as peer reviewer.`, 'info', 'create_handoff');
    mirror(api, 'persistTaskStatus', task);
  }));

  delay(1620, () => api.update((next) => {
    const mission = getMission(next, missionId);
    const task = taskFor(mission, 'agent-worker-1');
    if (!mission || !task) return;
    const payload = runOperationsAnalyst(context);
    updateTask(next, missionId, task.id, 'review', `Calculated ${payload.projectCount} projects, ${payload.atRiskCount} at risk, and ${payload.totalPipelineValue.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 })} in value. Awaiting peer review from Maya.`);
    mission.handoffs.push({ id: id('handoff-ops'), fromAgentId: 'agent-worker-1', reviewerId: task.reviewerId, type: payload.handoffType, sourceCount: payload.evidenceRecordIds.length, status: 'pending_review', payload });
    addActivity(next, `W-1 created a source-linked pipeline_metrics handoff with ${payload.evidenceRecordIds.length} reference(s); Maya is assigned as peer reviewer.`, 'info', 'create_handoff');
    mirror(api, 'persistTaskStatus', task);
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
      mirror(api, 'persistMissionStatus', mission);
      finishDurableRun(api, marketTask, 'failed');
      finishDurableRun(api, opsTask, 'failed');
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
    // Source-linked handoffs are written only after the peer gate accepts
    // them; their `from_run_id` refers to the completed deterministic analyst
    // run, not an unstructured chat transcript.
    if (marketHandoff) mirror(api, 'persistHandoff', mission, marketTask, marketHandoff);
    if (opsHandoff) mirror(api, 'persistHandoff', mission, opsTask, opsHandoff);
    // Complete analyst runs only after their peer acceptance. The ordered
    // completion prevents the run command's `review` transition from racing
    // and overwriting the final `approved` task state.
    finishDurableRun(api, marketTask, 'succeeded', () => mirror(api, 'persistTaskStatus', marketTask));
    finishDurableRun(api, opsTask, 'succeeded', () => mirror(api, 'persistTaskStatus', opsTask));
    if (documentTask) startDurableRun(api, documentTask);
  }));

  const nativeDeck = type === 'pptx' && typeof api.createPptxDeck === 'function';

  const finishReview = () => api.update((next) => {
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
    finishDurableRun(api, reviewTask, 'succeeded', () => mirror(api, 'persistTaskStatus', reviewTask));
    mirror(api, 'persistMissionStatus', mission);
  });

  delay(2570, () => {
    let nativeRequest = null;
    let nativeArtifact = null;
    api.update((next) => {
      const mission = getMission(next, missionId);
      const documentTask = taskFor(mission, 'agent-worker-2');
      if (!mission || !documentTask) return;
      const marketAnalysis = mission.handoffs.find((handoff) => handoff.type === 'market_analysis')?.payload;
      const pipelineMetrics = mission.handoffs.find((handoff) => handoff.type === 'pipeline_metrics')?.payload;
      if (!marketAnalysis || !pipelineMetrics) {
        updateTask(next, missionId, documentTask.id, 'failed', 'Required source-linked handoffs were unavailable.');
        mission.status = 'failed';
        addActivity(next, 'W-2 could not start because a required handoff was unavailable.', 'danger', 'handoff_failed');
        mirror(api, 'persistTaskStatus', documentTask);
        mirror(api, 'persistMissionStatus', mission);
        return;
      }
      const artifact = buildArtifact({ requirementText: mission.text, fileType: type, context, marketAnalysis, pipelineMetrics });
      nativeArtifact = artifact;
      if (nativeDeck) {
        updateTask(next, missionId, documentTask.id, 'running', 'Submitting a typed OfficeCLI deck request through the desktop runtime.');
        nativeRequest = {
          taskId: documentTask.id,
          fileName: artifactName(mission.text, type, context),
          title: artifact.title,
          slides: artifact.outline.map((title, index) => ({ title, bullets: index === 1 ? [`Pipeline value: ${artifact.dataSummary.pipelineValue}`, `Projects: ${artifact.dataSummary.projectCount}`, `At risk: ${artifact.dataSummary.atRiskCount}`] : [] }))
        };
        addActivity(next, 'W-2 submitted a restricted, typed OfficeCLI PPTX request.', 'info', 'office_request');
        mirror(api, 'persistTaskStatus', documentTask);
        return;
      }
      const reviewTask = completeArtifact(next, mission, documentTask, artifact, { id: id('report'), name: artifactName(mission.text, type, context), size: 'local agent plan', prototype: true });
      mirror(api, 'persistTaskStatus', documentTask);
      if (reviewTask) mirror(api, 'persistTaskStatus', reviewTask);
    });

    if (!nativeRequest) return;
    api.createPptxDeck(nativeRequest)
      .then((execution) => {
        api.update((next) => {
          const mission = getMission(next, missionId);
          const documentTask = taskFor(mission, 'agent-worker-2');
          if (!mission || !documentTask || !nativeArtifact) return;
          const reviewTask = completeArtifact(next, mission, documentTask, nativeArtifact, { id: id('report'), name: execution.fileName, size: `${execution.bytes.toLocaleString()} bytes`, prototype: false });
          addActivity(next, `OfficeCLI created ${execution.fileName} inside the approved Report Center path.`, 'success', 'create_file');
          finishDurableRun(api, documentTask);
          if (reviewTask) startDurableRun(api, reviewTask);
        });
        delay(260, finishReview);
      })
      .catch((error) => {
        api.update((next) => {
          const mission = getMission(next, missionId);
          const documentTask = taskFor(mission, 'agent-worker-2');
          if (!mission || !documentTask) return;
          updateTask(next, missionId, documentTask.id, 'failed', `OfficeCLI request failed: ${error.message || 'unknown error'}`);
          mission.status = 'failed';
          addActivity(next, 'OfficeCLI failed; no deliverable was registered.', 'danger', 'office_request');
          finishDurableRun(api, documentTask, 'failed');
          mirror(api, 'persistMissionStatus', mission);
        });
      });
  });

  delay(3150, () => {
    if (!nativeDeck) finishReview();
  });

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
