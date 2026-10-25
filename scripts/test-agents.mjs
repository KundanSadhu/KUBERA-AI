import { createSeedState } from '../src/seed.js';
import { startMission } from '../src/orchestration.js';

const timers = [];
globalThis.window = {
  setTimeout(fn, ms) {
    timers.push({ fn, ms });
    return timers.length;
  }
};

let state = createSeedState();
const api = {
  getState: () => state,
  update(mutator) {
    const next = structuredClone(state);
    mutator(next);
    state = next;
  }
};

const result = startMission(api, 'Build a Q3 pipeline deck for Acme Corp');
const executed = new Set();
const runTimersThrough = (limit) => {
  for (const timer of timers
    .map((timer, index) => ({ ...timer, index }))
    .filter((timer) => !executed.has(timer.index) && timer.ms <= limit)
    .sort((left, right) => left.ms - right.ms)) {
    executed.add(timer.index);
    timer.fn();
  }
};

// The manager has finished its bounded plan at this point, while the first
// mission's specialists still occupy primary pool slots. The next plan must
// use the deterministic secondary analyst/document-worker slots.
runTimersThrough(800);
const pooledResult = startMission(api, 'Create a campaign budget tracker for active projects');
runTimersThrough(800);
const pooledMission = state.missions.find((item) => item.id === pooledResult.missionId);
for (const timer of timers.map((timer, index) => ({ ...timer, index })).filter((timer) => !executed.has(timer.index)).sort((left, right) => left.ms - right.ms)) {
  executed.add(timer.index);
  timer.fn();
}

const mission = state.missions.find((item) => item.id === result.missionId);
const report = state.reports.find((item) => item.id === mission?.reportId);
const poolAssigned = pooledMission?.tasks.some((task) => task.agentId === 'agent-market-2')
  && pooledMission?.tasks.some((task) => task.agentId === 'agent-worker-1b')
  && pooledMission?.tasks.some((task) => task.agentId === 'agent-worker-2b');
const passed = result.ok
  && pooledResult.ok
  && poolAssigned
  && mission?.status === 'done'
  && mission.handoffs.length === 2
  && report?.dataSummary?.pipelineValue === 365000
  && report.sourceRecordIds.length === 3
  && mission.tasks.at(-1)?.status === 'approved';

if (!passed) {
  console.error('Managed-agent workflow test failed.', { result, mission, report });
  process.exit(1);
}

console.log('Managed-agent workflow passed: manager → analysts → document worker → reviewer.');
