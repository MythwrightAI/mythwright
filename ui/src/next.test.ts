import assert from 'node:assert/strict';
import { test } from 'node:test';
import { agentsTable, examplePath, hasRealAgent, whatNext } from './next.ts';
import type { Next } from './next.ts';

/** The line known by `key`. */
function line(lines: Next[], key: Next['key']): Next {
  const found = lines.find((one) => one.key === key);
  if (!found) throw new Error(`no line for ${key}`);
  return found;
}

const fields = (...names: string[]) => names.map((name) => ({ name }));

test('the table that says what agents run on is the one beside another that has the cell, whatever it is called', () => {
  const board = { name: 'board', fields: fields('title', 'status') };
  const agents = { name: 'agents', beside: 'board', fields: fields('title', 'role', 'adapter') };
  const stages = { name: 'stages', beside: 'board', fields: fields('title', 'column') };
  assert.equal(agentsTable([board, stages, agents]), 'agents');
  // Called something else, it is still the one: nothing goes by the name.
  assert.equal(agentsTable([board, { ...agents, name: 'crew' }]), 'crew');
  // A project with no fleet has none, and a table of its own with such a column that sits beside nothing is not it.
  assert.equal(agentsTable([board, stages]), null);
  assert.equal(agentsTable([board, { name: 'tools', fields: fields('adapter') }]), null);
  assert.equal(agentsTable([]), null);
});

test('a project has a real agent once one of its own that is up runs on anything but the built-in assistant', () => {
  const one = (project: string | null, adapter: string, active = true) => ({ project, adapter, active });
  assert.equal(hasRealAgent([one('shop', 'builtin'), one('shop', 'builtin')], 'shop', ['builtin']), false);
  assert.equal(hasRealAgent([one('shop', 'builtin'), one('shop', 'writer')], 'shop', ['builtin']), true);
  // Another project's, one that is nobody's and one that was stopped do not count.
  assert.equal(hasRealAgent([one('bakery', 'writer'), one(null, 'writer'), one('shop', 'writer', false)], 'shop', ['builtin']), false);
  assert.equal(hasRealAgent([], 'shop', ['builtin']), false);
});

test('the example is looked for where the daemon writes it', () => {
  assert.equal(examplePath('research', 'ask', 'bp', '@blueprints'), '@blueprints/research/ask.bp');
  assert.equal(examplePath('research', 'ask', 'bp', 'tools/bp'), 'tools/bp/research/ask.bp');
});

test('what is next is four lines, each with something to press, and says what was done', () => {
  const stands = { cli: 'fx', project: { id: 'shop', name: 'Shop' }, agents: 'agents', real: false, example: false };
  const lines = whatNext(stands);
  assert.deepEqual(lines.map((one) => one.key), ['card', 'agent', 'experiment', 'device']);
  assert.ok(lines.every((one) => one.action !== '' && one.title !== '' && one.detail !== ''));
  assert.deepEqual(lines.map((one) => one.done), [false, false, false, false]);
  const [agent, experiment, device] = [line(lines, 'agent'), line(lines, 'experiment'), line(lines, 'device')];
  // The fleet's agents are given a program in their row: the cell, and the commands by the project's own id.
  assert.match(agent.detail, /`fx record list shop agents`/);
  assert.match(agent.detail, /`fx record set <row> adapter=<package>`/);
  assert.equal(agent.action, 'Open its agents');
  assert.match(experiment.detail, /`fx example add research --project shop`/);
  assert.equal(experiment.action, 'Add it to Shop');
  assert.match(device.detail, /`fx device invite <name>`/);

  // A project whose fleet made no such table yet is sent to where packages are installed.
  assert.equal(line(whatNext({ ...stands, agents: null }), 'agent').action, 'Open Extensions');
  // Done, each says so and offers what is still worth pressing.
  const done = whatNext({ ...stands, real: true, example: true });
  assert.deepEqual(done.map((one) => one.done), [false, true, true, false]);
  assert.equal(line(done, 'experiment').action, 'Open it');
  assert.doesNotMatch(line(done, 'agent').detail, /cannot work it/);
});
