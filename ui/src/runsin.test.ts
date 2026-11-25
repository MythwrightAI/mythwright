import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { GraphParam } from './api.ts';
import { againOf, askedOf, heardOf, namesChannel, offeredOf, saidOf, stopSaid } from './runsin.ts';

const updates = { id: 'ch_7', name: 'project-updates' };
const param = (name: string): GraphParam => ({ name, type: 'string', default: null, required: false, description: null, choices: [] });

test('a channel is named by its id or its name, a # before it or not, and by nothing else', () => {
  assert.equal(namesChannel('project-updates', updates), true);
  assert.equal(namesChannel('#project-updates', updates), true);
  assert.equal(namesChannel('ch_7', updates), true);
  assert.equal(namesChannel('general', updates), false);
  assert.equal(namesChannel(7, updates), false);
  assert.equal(namesChannel(null, updates), false);
});

test('what was applied by somebody is changed and stopped by whoever may apply, and what something started is neither', () => {
  assert.deepEqual(offeredOf({ parent: null }, ['blueprints']), { change: true, stop: true });
  assert.deepEqual(offeredOf({ parent: null }, ['*']), { change: true, stop: true });
  assert.deepEqual(offeredOf({ parent: null }, ['blueprints:read', 'channels']), { change: false, stop: false });
  assert.deepEqual(offeredOf({ parent: 'in_1' }, ['*']), { change: false, stop: false });
});

test('a line says the key where it is not the blueprint, the environment and what started it, and nothing where there is nothing', () => {
  assert.equal(saidOf({ blueprint: 'status-updates', key: 'status-updates', environment: '', parent: null }), '');
  assert.equal(saidOf({ blueprint: 'std/discuss', key: 'discuss-ch_7', parent: null }), 'as discuss-ch_7');
  assert.equal(saidOf({ blueprint: 'std/watch', key: 'watch', environment: 'staging', parent: 'in_2' }), 'as watch, under staging, started by what runs above it');
});

test('the form asks every param but the channel it is listed by, which is kept as it was', () => {
  const params = [param('channel'), param('every'), param('say')];
  const named = askedOf(params, { params: { channel: '#project-updates', every: '10m' } }, updates);
  assert.deepEqual(
    named.asked.map((one) => one.name),
    ['every', 'say'],
  );
  assert.deepEqual(named.kept, { channel: '#project-updates' });
  // Listed here by its state, its channel param says something else and is asked.
  const other = askedOf(params, { params: { channel: 'general' } }, updates);
  assert.deepEqual(
    other.asked.map((one) => one.name),
    ['channel', 'every', 'say'],
  );
  assert.deepEqual(other.kept, {});
});

test('applied again it is the same blueprint, key, project, environment and labels, with what the form says and the channel kept', () => {
  const instance = { blueprint: 'status-updates', key: 'status-updates', project: 'shop', environment: 'dev', labels: { experiment: 'weekly' } };
  assert.deepEqual(againOf(instance, { every: '5m' }, { channel: 'project-updates' }), {
    blueprint: 'status-updates',
    key: 'status-updates',
    project: 'shop',
    environment: 'dev',
    labels: { experiment: 'weekly' },
    params: { every: '5m', channel: 'project-updates' },
    wait: true,
  });
  // No environment and no labels are said where it has none, and a kept value is not written over by the form.
  assert.deepEqual(againOf({ ...instance, project: null, environment: '', labels: {} }, { channel: 'elsewhere' }, { channel: 'ch_7' }), {
    blueprint: 'status-updates',
    key: 'status-updates',
    project: null,
    params: { channel: 'ch_7' },
    wait: true,
  });
});

test('the question before stopping names what stops, its key where that is other, and says what stays', () => {
  assert.equal(stopSaid({ blueprint: 'status-updates', key: 'status-updates' }, '#project-updates').title, 'Stop status-updates?');
  const discuss = stopSaid({ blueprint: 'std/discuss', key: 'discuss-ch_7' }, '#design');
  assert.equal(discuss.title, 'Stop std/discuss (discuss-ch_7)?');
  assert.match(discuss.says, /says nothing more in #design/);
  assert.match(discuss.stays, /stays/);
});

test('what runs is read again for something applied by somebody, anything changed and something listed that ended, and for nothing else', () => {
  assert.equal(heardOf('instance.started', { parent: null }, 'in_9', []), true);
  assert.equal(heardOf('instance.started', {}, 'in_9', []), true);
  assert.equal(heardOf('instance.started', { parent: 'in_1' }, 'in_9', []), false);
  assert.equal(heardOf('instance.updated', {}, 'in_9', []), true);
  assert.equal(heardOf('instance.finished', {}, 'in_9', ['in_9']), true);
  assert.equal(heardOf('instance.finished', {}, 'in_8', ['in_9']), false);
  assert.equal(heardOf('instance.finished', {}, undefined, ['in_9']), false);
  assert.equal(heardOf('instance.state', {}, 'in_9', ['in_9']), false);
});
