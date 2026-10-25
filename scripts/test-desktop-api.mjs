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

console.log('Desktop persistence contract passed: task claim, source-linked handoff, and run completion.');
