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
for (const timer of timers.sort((a, b) => a.ms - b.ms)) timer.fn();

const mission = state.missions.find((item) => item.id === result.missionId);
const report = state.reports.find((item) => item.id === mission?.reportId);
const passed = result.ok
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
