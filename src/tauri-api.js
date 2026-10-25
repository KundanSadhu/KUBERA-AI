/* Tauri-only repository adapter. Browser prototype keeps using api.js/localStorage. */
const invoke = (command, args) => {
  const tauri = window.__TAURI__?.core;
  if (!tauri?.invoke) throw new Error('Tauri IPC is unavailable in this runtime.');
  return tauri.invoke(command, args);
};

export const isDesktopRuntime = () => Boolean(window.__TAURI__?.core?.invoke);

export function createTauriApi() {
  return {
    getEngineStatus: () => invoke('get_engine_status'),
    startEngine: () => invoke('sidecar_start'),
    stopEngine: () => invoke('sidecar_stop'),
    openCodeHealth: () => invoke('opencode_health'),
    openCodeCreateSession: () => invoke('opencode_create_session'),
    openCodeSendMessage: (sessionId, text) => invoke('opencode_send_message', { sessionId, text }),
    listClients: () => invoke('list_clients'),
    listProjects: () => invoke('list_projects'),
    listAgents: () => invoke('list_agents'),
    listRequirements: () => invoke('list_requirements'),
    createRequirement: (text, deliverableType) => invoke('create_requirement', { text, deliverableType }),
    getContextForMission: (selector, actorId, purpose) => invoke('get_context_for_mission', { selector, actorId, purpose }),
    createAgentRun: (taskId, agentId, parentRunId) => invoke('create_agent_run', { taskId, agentId, parentRunId }),
    completeAgentRun: (runId, status, outputSummary) => invoke('complete_agent_run', { runId, status, outputSummary }),
    recordAgentHandoff: (args) => invoke('record_agent_handoff', args),
    listDeliverables: () => invoke('list_deliverables'),
    createSchedule: (input) => invoke('create_schedule', { input }),
    listSchedules: () => invoke('list_schedules'),
    setScheduleEnabled: (scheduleId, enabled) => invoke('set_schedule_enabled', { scheduleId, enabled }),
    listAuditEvents: (limit = 80) => invoke('list_audit_events', { limit })
  };
}
