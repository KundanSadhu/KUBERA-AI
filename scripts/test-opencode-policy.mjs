import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const config = JSON.parse(await readFile(new URL('../src-tauri/opencode.kubera.json', import.meta.url), 'utf8'));
const disabledTools = [
  'bash', 'read', 'edit', 'write', 'patch', 'glob', 'grep', 'list',
  'webfetch', 'websearch', 'task', 'skill', 'lsp', 'todowrite', 'question'
];

assert.equal(config.model, 'opencode/deepseek-v4.1-flash-free');
assert.equal(config.small_model, 'opencode/glm-5.3-flash-free');
assert.equal(config.agent?.plan?.mode, 'primary');
assert.equal(config.agent?.plan?.model, config.model);
assert.equal(config.agent?.plan?.maxSteps, 1);
assert.equal(config.share, 'disabled');
assert.equal(config.autoupdate, false);
for (const tool of disabledTools) {
  assert.equal(config.tools?.[tool], false, `global ${tool} must stay disabled`);
  assert.equal(config.agent?.plan?.tools?.[tool], false, `plan ${tool} must stay disabled`);
}
for (const permission of ['edit', 'bash', 'webfetch', 'external_directory', 'doom_loop']) {
  assert.equal(config.agent?.plan?.permission?.[permission], 'deny', `${permission} permission must stay denied`);
}

const bridge = await readFile(new URL('../src-tauri/src/opencode.rs', import.meta.url), 'utf8');
assert.match(bridge, /"model": \{ "providerID": FREE_MODEL_PROVIDER, "modelID": PRIMARY_FREE_MODEL \}/);
assert.match(bridge, /"bash": false, "read": false, "edit": false, "write": false/);
assert.match(bridge, /"webfetch": false, "websearch": false, "task": false/);

console.log('OpenCode free-model policy contract passed.');
