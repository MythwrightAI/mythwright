import assert from 'node:assert/strict';
import { test } from 'node:test';
import { doneSays, scenarioKey } from './scenario.ts';

test('each thing a scenario does is said in a line, with its code in backticks', () => {
  const cases: [Record<string, unknown>, string][] = [
    [{ apply: {} }, 'Applied it'],
    [{ apply: { params: {} } }, 'Applied it'],
    [{ apply: { params: { ship: true } } }, 'Applied it, told ship: true'],
    [{ say: { topic: 'lock.give', data: {} } }, 'Said `lock.give`'],
    [{ say: { topic: 'lock.take', data: { who: 'ada' } } }, 'Said `lock.take` who: ada'],
    [{ do: { action: 'channel.ensure', with: { name: 'general' } } }, 'Did `channel.ensure` name: general'],
    [{ advance: { by: '10s' } }, 'Moved the clock on 10s'],
    [{ answer: { asked: 1, fails: 'it could not connect' } }, 'Answered the 1st stand-in: it fails, it could not connect'],
    [{ answer: { asked: 2, gives: {} } }, 'Answered the 2nd stand-in: it went well'],
    [{ answers: { answers: 'default' } }, 'Had every stand-in answer as usual from here on'],
    [{ answers: { answers: 'asked' } }, 'Had every stand-in wait to be answered from here on'],
  ];
  for (const [act, said] of cases) assert.equal(doneSays(act), said, JSON.stringify(act));
  // Something this page does not know is said as it was sent, never left out.
  assert.equal(doneSays({ dance: { how: 'slowly' } }), 'Did {"dance":{"how":"slowly"}}');
});

test('a scenario opened again is kept by a key of its own', () => {
  const first = scenarioKey();
  assert.notEqual(scenarioKey(), first);
  assert.ok(first.startsWith('scenario:'));
});
