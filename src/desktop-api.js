import { createTauriApi, isDesktopRuntime } from './tauri-api.js';

const asTaskStatus = (status) => ({ in_progress: 'running', review: 'review', ready: 'queued' }[status] || status);
const asDbTaskStatus = (status) => ({ running: 'in_progress', queued: 'ready', succeeded: 'review', approved: 'approved', review: 'review', blocked: 'blocked', failed: 'failed' }[status] || 'blocked');
const dbAgentId = (id) => id === 'agent-market' ? 'agent-market-analyst' : id;
const uiAgentId = (id) => id === 'agent-market-analyst' ? 'agent-market' : id;

function sessionIdFrom(session) {
  const id = session?.id || session?.session?.id || session?.data?.id;
  if (typeof id !== 'string' || !id.trim()) throw new Error('OpenCode did not return a session identifier.');
  return id;
}

function responseSummary(response) {
  const text = JSON.stringify(response);
  return text.length > 1200 ? `${text.slice(0, 1197)}...` : text;
}

function agentColor(kind) {
  return ({ manager: 'violet', analyst: 'blue', worker: 'amber', reviewer: 'rose' }[kind] || 'slate');
}

function reportOutline(type) {
  return type === 'pptx' ? ['Native Office artifact registered by desktop runtime'] : ['Native Office artifact registered by desktop runtime'];
}

/**
 * Desktop-state adapter. The UI keeps its small render store, while this adapter
 * makes SQLite the durable source at boot and mirrors governed lifecycle writes
 * to typed Tauri commands. Browser development keeps using localStorage.
 */
export function createDesktopApi(store) {
  const tauri = createTauriApi();
  const activeRuns = new Map();
  const completedRuns = new Map();
  const safe = async (promise, fallback) => {
    try { return await promise; } catch (error) { console.warn('Desktop bridge operation failed', error); return fallback; }
  };


  async function startTaskRun(task, parentRunId = null) {
    const existing = activeRuns.get(task.id);
    if (existing) return existing;
    // Tasks enter the DB in ready state before an atomic claim. A downstream
    // task is deliberately promoted only after its dependency gate has passed.
    if (task.status === 'blocked') await tauri.updateTaskStatus(task.id, 'ready', task.summary || null);
    const runId = await tauri.createAgentRun(task.id, dbAgentId(task.agentId), parentRunId);
    activeRuns.set(task.id, runId);
    return runId;
  }

  async function finishTaskRun(task, status, outputSummary) {
    const runId = activeRuns.get(task.id);
    if (!runId) return null;
    await tauri.completeAgentRun(runId, status, outputSummary || task.summary || null);
    activeRuns.delete(task.id);
    completedRuns.set(task.id, runId);
    return runId;
  }

  async function persistHandoff(mission, task, handoff) {
    const runId = activeRuns.get(task.id) || completedRuns.get(task.id);
    if (!runId) throw new Error(`No governed agent run exists for handoff task ${task.id}.`);
    const sourceRefs = handoff.payload?.evidenceRecordIds || handoff.payload?.sourceRecordIds || [];
    const documentTask = mission.tasks.find((candidate) => candidate.agentId === 'agent-worker-2');
    return tauri.recordAgentHandoff({
      requirementId: mission.id,
      taskId: task.id,
      fromRunId: runId,
      toTaskId: documentTask?.id || null,
      toAgentId: documentTask ? dbAgentId(documentTask.agentId) : null,
      handoffType: handoff.type,
      payloadJson: JSON.stringify(handoff.payload || {}),
      sourceRefsJson: JSON.stringify(sourceRefs)
    });
  }

  return {
    mode: 'desktop',
    getState: () => store.getState(),
    update: (mutator) => store.update(mutator),
    reset: () => store.reset(),
    async bootstrap() {
      const engine = await safe(tauri.startEngine(), { state: 'offline', model: 'not configured', message: 'Desktop engine could not start.' });
      const [clients, projects, agents, requirements, tasks, handoffs, deliverables, schedules, activity] = await Promise.all([
        safe(tauri.listClients(), []), safe(tauri.listProjects(), []), safe(tauri.listAgents(), []),
        safe(tauri.listRequirements(), []), safe(tauri.listTasks(), []), safe(tauri.listAgentHandoffs(), []),
        safe(tauri.listDeliverables(), []), safe(tauri.listSchedules(), []), safe(tauri.listAuditEvents(), [])
      ]);
      const taskById = new Map(tasks.map((task) => [task.id, task]));
      const missions = requirements.map((requirement) => {
        const missionTasks = tasks.filter((task) => task.requirementId === requirement.id).map((task) => ({
          ...task, agentId: uiAgentId(task.assigneeId || 'agent-manager'), status: asTaskStatus(task.status), dependsOn: task.deps || [], summary: task.reviewNotes || task.brief
        }));
        const missionHandoffs = handoffs.filter((handoff) => handoff.requirementId === requirement.id).map((handoff) => ({
          id: handoff.id, fromAgentId: uiAgentId(taskById.get(handoff.taskId)?.assigneeId || 'agent-manager'),
          toTaskId: handoff.toTaskId || null, toAgentId: handoff.toAgentId ? uiAgentId(handoff.toAgentId) : null,
          type: handoff.handoffType, sourceCount: JSON.parse(handoff.sourceRefsJson || '[]').length,
          status: handoff.validationStatus === 'accepted' ? 'accepted' : 'pending_review', payload: safeJson(handoff.payloadJson)
        }));
        const report = deliverables.find((item) => taskById.get(item.taskId)?.requirementId === requirement.id);
        return {
          id: requirement.id, text: requirement.text, title: requirement.text, type: requirement.deliverableType || 'pptx',
          status: requirement.status, createdAt: requirement.createdAt, updatedAt: requirement.createdAt, reportId: report?.id || null,
          tasks: missionTasks, handoffs: missionHandoffs, runtime: 'desktop'
        };
      });
      store.update((next) => {
        next.engine = { status: engine.state || 'offline', model: engine.model || 'not configured', patrol: true, message: engine.message || '' };
        next.clients = clients;
        next.projects = projects.map((project) => ({ ...project, risk: project.riskFlag }));
        next.agents = agents.map((agent) => ({ ...agent, id: uiAgentId(agent.id), label: agent.role, capability: (agent.capabilities || []).join(', '), color: agentColor(agent.agentKind) }));
        next.missions = missions;
        next.reports = deliverables.map((report) => ({ ...report, name: report.fileName, type: report.fileType, missionId: taskById.get(report.taskId)?.requirementId || '', size: 'desktop artifact', producer: 'KUBERA desktop runtime', outline: reportOutline(report.fileType), prototype: false }));
        next.schedules = schedules.map((schedule) => ({ ...schedule, prompt: schedule.requirementText, cron: schedule.cronExpression }));
        next.activity = activity.map((event) => ({ id: String(event.id), at: event.createdAt, kind: event.action, tone: 'neutral', text: event.detail || `${event.actor}: ${event.action}` }));
      });
      return store.getState();
    },
    persistRequirement(mission) {
      return tauri.createRequirement(mission.text, mission.type, mission.id);
    },
    persistMissionStatus(mission) {
      return tauri.updateRequirementStatus(mission.id, mission.status);
    },
    persistTasks(mission, tasks) {
      return Promise.all(tasks.map((task) => tauri.createTask({
        id: task.id, requirementId: mission.id, title: task.title, brief: task.summary || task.title,
        role: task.agentId || 'worker', requiredCapability: null, assigneeId: dbAgentId(task.agentId || '') || null,
        // A newly written running task starts as ready so it can be atomically
        // claimed by create_agent_run rather than bypassing agent capacity.
        status: task.status === 'running' ? 'ready' : asDbTaskStatus(task.status), deps: task.dependsOn || []
      }))).then(() => Promise.all(tasks.filter((task) => task.status === 'running').map((task) => startTaskRun(task))));
    },
    persistTaskStatus(task) {
      return tauri.updateTaskStatus(task.id, asDbTaskStatus(task.status), task.summary || null);
    },
    startTaskRun(task, parentRunId = null) { return startTaskRun(task, parentRunId); },
    finishTaskRun(task, status, outputSummary) { return finishTaskRun(task, status, outputSummary); },
    persistHandoff(mission, task, handoff) { return persistHandoff(mission, task, handoff); },
    async startManagerSession({ task, mission, context }) {
      const runId = await startTaskRun(task);
      try {
        const session = await tauri.openCodeCreateSession();
        const sessionId = sessionIdFrom(session);
        await tauri.setAgentRunSession(runId, sessionId);
        const prompt = [
          "You are Asha, KUBERA's bounded business-operations manager.",
          'Return a concise plan only. Do not call tools, access files, create child sessions, or make decisions outside this request.',
          `Requirement: ${mission.text}`,
          `Requested deliverable: ${mission.type.toUpperCase()}.`,
          `Approved local scope: client ${context.client?.name || 'active portfolio'}; ${context.projects.length} selected project record(s).`,
          'Use at most these depth-one roles: market analyst, operations analyst, document worker, and reviewer.',
          'State dependencies and evidence expectations in plain text.'
        ].join('\n');
        const response = await tauri.openCodeSendMessage(sessionId, prompt);
        await finishTaskRun(task, 'succeeded', responseSummary(response));
        return { runId, sessionId, response };
      } catch (error) {
        await safe(finishTaskRun(task, 'failed', String(error?.message || error)), null);
        throw error;
      }
    },
    createPptxDeck(request) { return tauri.createPptxDeck(request); },
    subscribeOpenCodeEvents(listener) { return tauri.subscribeOpenCodeEvents(listener); }
  };
}

function safeJson(value) {
  try { return JSON.parse(value || '{}'); } catch { return {}; }
}

export function createApplicationApi(store, prototypeFactory) {
  return isDesktopRuntime() ? createDesktopApi(store) : prototypeFactory(store);
}
