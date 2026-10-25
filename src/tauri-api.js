/* Tauri-only repository adapter. Browser prototype keeps using api.js/localStorage. */
const invoke = (command, args) => {
  const tauri = window.__TAURI__?.core;
  if (!tauri?.invoke) throw new Error('Tauri IPC is unavailable in this runtime.');
  return tauri.invoke(command, args);
};

const listen = (event, handler) => {
  const tauri = window.__TAURI__?.event;
  if (!tauri?.listen) throw new Error('Tauri event bridge is unavailable in this runtime.');
  return tauri.listen(event, ({ payload }) => handler(payload));
};

export const isDesktopRuntime = () => Boolean(window.__TAURI__?.core?.invoke);

export function createTauriApi() {
  return {
    getEngineStatus: () => invoke('get_engine_status'),
    startEngine: () => invoke('sidecar_start'),
    stopEngine: () => invoke('sidecar_stop'),
    startScheduler: () => invoke('scheduler_start'),
    openCodeHealth: () => invoke('opencode_health'),
    openCodeCreateSession: () => invoke('opencode_create_session'),
    openCodeSendMessage: (sessionId, text) => invoke('opencode_send_message', { sessionId, text }),
    listClients: () => invoke('list_clients'),
    createClient: (input) => invoke('create_client', { input }),
    updateClient: (clientId, input) => invoke('update_client', { clientId, input }),
    listProjects: () => invoke('list_projects'),
    createProject: (input) => invoke('create_project', { input }),
    importProjectsCsv: (input) => invoke('import_projects_csv', { input }),
    updateProject: (projectId, input) => invoke('update_project', { projectId, input }),
    listAgents: () => invoke('list_agents'),
    listRequirements: () => invoke('list_requirements'),
    createRequirement: (text, deliverableType, requirementId = null) => invoke('create_requirement', { text, deliverableType, requirementId }),
    updateRequirementStatus: (requirementId, status) => invoke('update_requirement_status', { requirementId, status }),
    getContextForMission: (selector, actorId, purpose) => invoke('get_context_for_mission', { selector, actorId, purpose }),
    createTask: (input) => invoke('create_task', { input }),
    listTasks: (requirementId = null) => invoke('list_tasks', { requirementId }),
    createAgentRun: (taskId, agentId, parentRunId) => invoke('create_agent_run', { taskId, agentId, parentRunId }),
    completeAgentRun: (runId, status, outputSummary) => invoke('complete_agent_run', { runId, status, outputSummary }),
    setAgentRunSession: (runId, sessionId) => invoke('set_agent_run_session', { runId, sessionId }),
    listAgentRuns: (requirementId = null) => invoke('list_agent_runs', { requirementId }),
    recordAgentHandoff: (args) => invoke('record_agent_handoff', args),
    listAgentHandoffs: (requirementId = null) => invoke('list_agent_handoffs', { requirementId }),
    updateTaskStatus: (taskId, status, reviewNotes = null) => invoke('update_task_status', { taskId, status, reviewNotes }),
    createPptxDeck: (request) => invoke('create_pptx_deck', { request }),
    createDocxDocument: (request) => invoke('create_docx_document', { request }),
    createTemplatedDocx: (request) => invoke('create_templated_docx', { request }),
    createXlsxWorkbook: (request) => invoke('create_xlsx_workbook', { request }),
    listDeliverables: () => invoke('list_deliverables'),
    sendApprovedDelivery: (input) => invoke('send_approved_delivery', { input }),
    listDeliveries: () => invoke('list_deliveries'),
    requestDeliverableApproval: (deliverableId) => invoke('request_deliverable_approval', { deliverableId }),
    listDeliverableApprovals: () => invoke('list_deliverable_approvals'),
    decideDeliverableApproval: (approvalId, input) => invoke('decide_deliverable_approval', { approvalId, input }),
    openDeliverable: (deliverableId) => invoke('open_deliverable', { deliverableId }),
    revealDeliverable: (deliverableId) => invoke('reveal_deliverable', { deliverableId }),
    previewDeliverable: (deliverableId) => invoke('preview_deliverable', { deliverableId }),
    previewDeliverableVisual: (deliverableId) => invoke('preview_deliverable_visual', { deliverableId }),
    importTemplate: (input) => invoke('import_template', { input }),
    importTemplateBundle: (input) => invoke('import_template_bundle', { input }),
    exportTemplateBundle: (templateId) => invoke('export_template_bundle', { templateId }),
    listTemplates: () => invoke('list_templates'),
    createSchedule: (input) => invoke('create_schedule', { input }),
    listSchedules: () => invoke('list_schedules'),
    setScheduleEnabled: (scheduleId, enabled) => invoke('set_schedule_enabled', { scheduleId, enabled }),
    searchApprovedResearch: (input) => invoke('search_approved_research', { input }),
    listResearchCitations: () => invoke('list_research_citations'),
    listAuditEvents: (limit = 80) => invoke('list_audit_events', { limit }),
    subscribeOpenCodeEvents: (listener) => listen('opencode-event', listener),
    subscribeScheduleEvents: (listener) => listen('schedule-fired', listener)
  };
}
