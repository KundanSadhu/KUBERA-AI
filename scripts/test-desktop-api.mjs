import assert from 'node:assert/strict';

const calls = [];
globalThis.window = {
  __TAURI__: {
    core: {
      invoke: async (command, args = {}) => {
        calls.push({ command, args });
        if (command === 'create_agent_run') return `run-${args.taskId}`;
        if (command === 'record_agent_handoff') return 'handoff-durable-1';
        return null;
      }
    }
  }
};

const { createDesktopApi } = await import('../src/desktop-api.js');
const store = {
  state: { missions: [], reports: [], agents: [], activity: [] },
  getState() { return this.state; },
  update(mutator) { mutator(this.state); },
  reset() {}
};
const api = createDesktopApi(store);

const mission = { id: 'mission-1', text: 'Create a governed pipeline deck', type: 'pptx' };
const marketTask = {
  id: 'task-market', agentId: 'agent-market', title: 'Analyze market context',
  summary: 'Use the approved local project records.', status: 'running', dependsOn: []
};
const documentTask = {
  id: 'task-document', agentId: 'agent-worker-2', title: 'Create presentation',
  summary: 'Await accepted source-linked handoffs.', status: 'blocked', dependsOn: ['task-market']
};
mission.tasks = [marketTask, documentTask];

await api.persistTasks(mission, [marketTask]);
const createTask = calls.find((call) => call.command === 'create_task');
assert.equal(createTask.args.input.assigneeId, 'agent-market-analyst');
assert.equal(createTask.args.input.status, 'ready', 'running UI work must be claimed atomically from ready');
assert.ok(calls.some((call) => call.command === 'create_agent_run' && call.args.taskId === marketTask.id));

await api.persistHandoff(mission, marketTask, {
  type: 'market_analysis',
  payload: { evidenceRecordIds: ['project-1'], executiveSummary: 'Source-linked finding.' }
});
const handoff = calls.find((call) => call.command === 'record_agent_handoff');
assert.equal(handoff.args.requirementId, mission.id);
assert.equal(handoff.args.taskId, marketTask.id);
assert.equal(handoff.args.toTaskId, documentTask.id);
assert.equal(handoff.args.toAgentId, 'agent-worker-2');
assert.deepEqual(JSON.parse(handoff.args.sourceRefsJson), ['project-1']);

await api.finishTaskRun(marketTask, 'succeeded', 'Market analysis complete.');
assert.ok(calls.some((call) => call.command === 'complete_agent_run' && call.args.runId === 'run-task-market'));

const schedule = {
  id: '2c856854-2d15-4e5a-9f9b-79a2b291ed1f',
  prompt: 'Build the monthly pipeline deck from current records', type: 'pptx',
  cron: '0 9 1 * *', timezone: 'Asia/Calcutta', enabled: true
};
await api.persistSchedule(schedule);
const createSchedule = calls.find((call) => call.command === 'create_schedule');
assert.deepEqual(createSchedule.args.input, {
  id: schedule.id,
  requirementText: schedule.prompt,
  cronExpression: schedule.cron,
  timezone: schedule.timezone,
  deliverableType: schedule.type
});
await api.persistScheduleEnabled({ ...schedule, enabled: false });
assert.ok(calls.some((call) => call.command === 'set_schedule_enabled' && call.args.scheduleId === schedule.id && call.args.enabled === false));

await api.createDocxDocument({ taskId: documentTask.id, fileName: 'report.docx', title: 'Report', paragraphs: ['Summary'] });
await api.createTemplatedDocx({ taskId: documentTask.id, templateId: '1a539ea1-b540-4fd3-81b6-4dbeb41a3fee', fileName: 'merged.docx', values: [{ key: 'client_name', value: 'Acme' }] });
await api.createXlsxWorkbook({ taskId: documentTask.id, fileName: 'tracker.xlsx', sheetName: 'Sheet1', columns: ['Project'], rows: [['Acme']] });
assert.ok(calls.some((call) => call.command === 'create_docx_document'));
assert.ok(calls.some((call) => call.command === 'create_templated_docx'));
assert.ok(calls.some((call) => call.command === 'create_xlsx_workbook'));

await api.createClient({ name: 'Globex', industry: 'Manufacturing', status: 'prospect', owner: 'Asha' });
await api.updateClient('client-1', { name: 'Globex', industry: 'Manufacturing', status: 'active', owner: 'Asha' });
await api.createProject({ clientId: 'client-1', name: 'Expansion', stage: 'Discovery', value: 120000, riskFlag: false });
await api.importProjectsCsv({ clientId: 'client-1', csvText: 'name,stage,value,risk_flag\nExpansion,Proposal,140000,false\n' });
await api.updateProject('project-1', { clientId: 'client-1', name: 'Expansion', stage: 'Proposal', value: 140000, riskFlag: false });
assert.ok(calls.some((call) => call.command === 'create_client'));
assert.ok(calls.some((call) => call.command === 'update_client'));
assert.ok(calls.some((call) => call.command === 'create_project'));
assert.ok(calls.some((call) => call.command === 'import_projects_csv' && call.args.input.clientId === 'client-1'));
assert.ok(calls.some((call) => call.command === 'update_project'));

await api.importTemplate({ fileName: 'client-report.docx', fileType: 'docx', bytes: [80, 75, 3, 4] });
await api.importTemplateBundle({ fileName: 'client-report.kubera-template.zip', bytes: [80, 75, 3, 4] });
await api.exportTemplateBundle('1a539ea1-b540-4fd3-81b6-4dbeb41a3fee');
await api.searchApprovedResearch({ query: 'customer retention strategy' });
assert.ok(calls.some((call) => call.command === 'import_template' && call.args.input.fileName === 'client-report.docx'));
assert.ok(calls.some((call) => call.command === 'import_template_bundle' && call.args.input.fileName === 'client-report.kubera-template.zip'));
assert.ok(calls.some((call) => call.command === 'export_template_bundle' && call.args.templateId === '1a539ea1-b540-4fd3-81b6-4dbeb41a3fee'));
assert.ok(calls.some((call) => call.command === 'search_approved_research' && call.args.input.query === 'customer retention strategy'));

const deliverableId = 'e74488af-d510-405f-8a8c-dfd569641f8f';
await api.openDeliverable(deliverableId);
await api.revealDeliverable(deliverableId);
await api.previewDeliverable(deliverableId);
await api.previewDeliverableVisual(deliverableId);
await api.sendApprovedDelivery({ deliverableId, channel: 'email_smtp', recipient: 'board@example.com', message: 'Approved.' });
await api.requestDeliverableApproval(deliverableId);
await api.decideDeliverableApproval('8248ec47-6105-4ace-80ff-a38377aac241', { decision: 'approved', note: 'Ready for the board.' });
await api.startScheduler();
assert.ok(calls.some((call) => call.command === 'open_deliverable' && call.args.deliverableId === deliverableId));
assert.ok(calls.some((call) => call.command === 'reveal_deliverable' && call.args.deliverableId === deliverableId));
assert.ok(calls.some((call) => call.command === 'preview_deliverable' && call.args.deliverableId === deliverableId));
assert.ok(calls.some((call) => call.command === 'preview_deliverable_visual' && call.args.deliverableId === deliverableId));
assert.ok(calls.some((call) => call.command === 'send_approved_delivery' && call.args.input.channel === 'email_smtp'));
assert.ok(calls.some((call) => call.command === 'request_deliverable_approval' && call.args.deliverableId === deliverableId));
assert.ok(calls.some((call) => call.command === 'decide_deliverable_approval' && call.args.input.decision === 'approved'));
assert.ok(calls.some((call) => call.command === 'scheduler_start'));

console.log('Desktop persistence contract passed: task claim, source-linked handoff, schedule persistence, and governed Office/report actions.');
