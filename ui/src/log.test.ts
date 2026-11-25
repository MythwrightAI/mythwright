import assert from 'node:assert/strict';
import { test } from 'node:test';
import { line } from './log.ts';

const event = (topic: string, data: unknown) => ({ seq: 1, ts: 0, topic, data });

test('a start says which version and where, and why when whoever started it said', () => {
  assert.deepEqual(line(event('system.started', { address: '127.0.0.1:7700', version: '0.1.0', resumed: 0, note: null })), {
    what: 'Started',
    more: 'Version 0.1.0, at 127.0.0.1:7700.',
  });
  const noted = line(event('system.started', { address: '127.0.0.1:7700', version: '0.1.0', note: 'The daemon was not running, so the window started it.' }));
  assert.equal(noted.more, 'Version 0.1.0, at 127.0.0.1:7700. The daemon was not running, so the window started it.');
});

test('a start says whether it was a copy on its own or a program run from its checkout', () => {
  const start = (runs_from: string) => line(event('system.started', { address: '127.0.0.1:7700', version: '0.1.0', runs_from })).more;
  assert.equal(start('installed'), 'Version 0.1.0, a copy on its own, at 127.0.0.1:7700.');
  assert.equal(start('checkout'), 'Version 0.1.0, run from its checkout, at 127.0.0.1:7700.');
  assert.equal(start('somewhere'), 'Version 0.1.0, at 127.0.0.1:7700.', 'one this does not know says nothing of it');
});

test('a stop says what became of what was running', () => {
  assert.deepEqual(line(event('system.stopped', {})), { what: 'Stopped', more: 'What was running was left running for the next start.' });
});

test('doing without something says what, why and what it costs', () => {
  const said = line(event('system.degraded', { what: 'file notifications', reason: 'too many watches', effect: 'output is looked for a few times a second' }));
  assert.deepEqual(said, { what: 'Running without file notifications', more: 'Too many watches; output is looked for a few times a second.' });
});

test('an event this does not know is still a line', () => {
  assert.deepEqual(line(event('system.moved', { note: 'To another disk.' })), { what: 'Moved', more: 'To another disk.' });
  assert.deepEqual(line(event('system.checked', null)), { what: 'Checked', more: '' });
});
