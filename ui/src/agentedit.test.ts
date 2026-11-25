import assert from 'node:assert/strict';
import { test } from 'node:test';
import { bodyOf, closedOf, linesOf, outsideChange, sameWritten, setupOf, stoppedSaid, writtenOf } from './agentedit.ts';
import type { Reader, SetupSlot } from './agentedit.ts';

const helper: SetupSlot = {
  name: 'helper',
  adapter: 'builtin',
  model: null,
  project: 'shop',
  instructions: 'Be brief.',
  scopes: ['board:read', 'search'],
  permission_mode: 'edit',
  wall: true,
  reads: [],
  changes: [],
  git: 'read',
  web: false,
  outlives: false,
  signins: [],
  secrets: {},
  environment: '',
  active: true,
};

const owner: Reader = { kind: 'user', id: 'ada', scopes: ['*'] };

test('an agent’s setup is kept by its own text, its fleet’s row, what else brings it up, or nobody here', () => {
  const tables = ['board', 'agents', 'stages'];
  assert.deepEqual(setupOf(helper, tables, false), { kind: 'hand' });
  assert.deepEqual(setupOf({ ...helper, name: 'shop-worker-1', held_by: 'role-shop-agents-2' }, tables, false), { kind: 'row', table: 'agents', record: 'shop-agents-2' });
  // A row of a table called otherwise, found by the key all the same: no table is known by its name.
  assert.deepEqual(setupOf({ ...helper, held_by: 'role-shop-crew-7' }, ['crew'], false), { kind: 'row', table: 'crew', record: 'shop-crew-7' });
  // What ends in no row of its project's tables is said by its key.
  assert.deepEqual(setupOf({ ...helper, held_by: 'night-watch' }, tables, false), { kind: 'brought', by: 'night-watch' });
  assert.deepEqual(setupOf({ ...helper, held_by: 'role-cafe-agents-2' }, tables, false), { kind: 'brought', by: 'role-cafe-agents-2' });
  assert.deepEqual(setupOf({ ...helper, project: null, held_by: 'role-shop-agents-2' }, tables, false), { kind: 'brought', by: 'role-shop-agents-2' });
  assert.deepEqual(setupOf(helper, tables, true), { kind: 'outside' });
});

test('the form begins as the slot stands, and says everything again under the text’s params', () => {
  const slot = { ...helper, model: 'small', reads: ['~/.cargo'], secrets: { REMOTE_TOKEN: 'remote-bot' }, signins: ['key'] };
  const written = writtenOf(slot, { motto: 'steady' });
  assert.equal(written.reads, '~/.cargo');
  assert.equal(written.secrets, 'REMOTE_TOKEN=remote-bot');
  assert.deepEqual(bodyOf('helper', { ...written, reads: '~/.cargo\n\n  /opt/tools ', instructions: '', model: '' }, { motto: 'steady' }), {
    name: 'helper',
    project: 'shop',
    environment: undefined,
    params: {
      motto: 'steady',
      instructions: '',
      adapter: 'builtin',
      model: '',
      permission: 'edit',
      holds: ['board:read', 'search'],
      wall: 'on',
      reads: ['~/.cargo', '/opt/tools'],
      changes: [],
      git: 'read',
      web: 'off',
      outlives: 'off',
      signins: ['key'],
      secrets: ['REMOTE_TOKEN=remote-bot'],
    },
  });
  // A slot from before says less, and is read as what a new one is.
  const before = writtenOf({ ...helper, wall: undefined, git: undefined, web: undefined, outlives: undefined, secrets: undefined, signins: undefined });
  assert.deepEqual([before.wall, before.git, before.web, before.secrets], [true, 'read', false, '']);
});

test('a form said the same, laid out otherwise in its boxes of lines or its scopes in another order, is no change', () => {
  const written = writtenOf(helper);
  assert.ok(sameWritten(written, { ...written, reads: '\n', holds: ['search', 'board:read'] }));
  assert.ok(!sameWritten(written, { ...written, instructions: '' }));
  assert.deepEqual(linesOf(' a \n\n b\n'), ['a', 'b']);
});

test('what is closed of an agent’s setup is what the daemon would refuse whoever reads it', () => {
  const member: Reader = { kind: 'user', id: 'grace', scopes: ['agents', 'blueprints', 'board', 'search'] };
  const cases: [string, SetupSlot, Reader, string | null][] = [
    ['the owner', helper, owner, null],
    ['a member who holds what it holds', helper, member, null],
    ['its own credential', helper, { kind: 'agent', id: 'helper', scopes: ['*'] }, 'An agent never changes its own slot'],
    ['another agent that holds it all', helper, { kind: 'agent', id: 'other', scopes: ['*'] }, null],
    ['somebody who may only see agents', helper, { ...member, scopes: ['agents:read', 'board', 'search'] }, 'agents:write'],
    ['somebody who may not apply', helper, { ...member, scopes: ['agents', 'board', 'search'] }, 'blueprints:run'],
    ['one that holds more', { ...helper, scopes: ['board', 'shell'] }, member, '(shell)'],
    ['one handed a key', { ...helper, secrets: { T: 'bot' } }, member, 'secrets:use'],
    ['one drawing a sign-in', { ...helper, signins: ['key'] }, member, 'secrets:use'],
    ['one that may do anything', { ...helper, permission_mode: 'full' }, member, 'may do anything unasked'],
    ['one with no wall', { ...helper, wall: false }, member, 'no wall'],
    ['one shown more', { ...helper, reads: ['~/.cargo'] }, member, 'more of this device'],
  ];
  for (const [who, slot, reader, said] of cases) {
    const all = closedOf(slot, { kind: 'hand' }, reader).all;
    if (said === null) assert.equal(all, null, who);
    else assert.ok(all?.includes(said), `${who}: ${all}`);
  }
  // Of one a fleet counts, applying is not asked: its row is.
  assert.equal(closedOf(helper, { kind: 'row', table: 'agents', record: 'shop-agents-1' }, { ...member, scopes: ['agents', 'board', 'search'] }).all, null);
  // One from outside is asked nothing of a program.
  assert.equal(closedOf({ ...helper, wall: false }, { kind: 'outside' }, { ...member, scopes: ['agents', 'board', 'search'] }).all, null);
  assert.ok(closedOf(helper, { kind: 'brought', by: 'night-watch' }, owner).all?.includes('night-watch'));
  // Each part a member who holds no `shell` or `secrets:use` may not turn on says so.
  const parts = closedOf(helper, { kind: 'hand' }, member);
  assert.ok(parts.full?.includes('shell') && parts.device?.includes('shell') && parts.secrets?.includes('secrets:use'));
  assert.deepEqual(Object.values(closedOf(helper, { kind: 'hand' }, owner)), [null, null, null, null]);
});

test('a stopped agent’s form says that saving brings it in', () => {
  assert.equal(stoppedSaid(helper), null);
  assert.ok(stoppedSaid({ active: false })?.includes('brought in again'));
});

test('one from outside sends what changed of what it holds and its project, and nothing where nothing did', () => {
  const slot = { scopes: ['board:read', 'inbox'], project: 'shop' };
  assert.equal(outsideChange(slot, ['inbox', 'board:read'], 'shop'), null);
  assert.deepEqual(outsideChange(slot, ['board:read'], 'shop'), { scopes: ['board:read'] });
  assert.deepEqual(outsideChange(slot, ['board:read', 'inbox'], ''), { project: null });
  assert.deepEqual(outsideChange({ ...slot, project: null }, ['board'], 'cafe'), { scopes: ['board'], project: 'cafe' });
});
