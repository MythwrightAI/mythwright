import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mayMove, movedSaid, movingSaid } from './agentmoved.ts';

test('moving an agent is offered for one nothing running holds, to whoever may change agents and holds all it holds', () => {
  const holds = ['channels', 'board:read'];
  const cases: [string | null | undefined, string[], boolean][] = [
    [undefined, ['*'], true],
    [null, ['agents', 'channels', 'board'], true],
    [undefined, ['agents:write', 'channels', 'board:read'], true],
    [undefined, ['agents:read', 'channels', 'board'], false],
    // It holds what they do not: the daemon would refuse it.
    [undefined, ['agents', 'channels'], false],
    // A fleet's agent: its project is its fleet's, changed in its row.
    ['fleet', ['*'], false],
    ['workers', ['agents', 'channels', 'board'], false],
  ];
  for (const [held_by, mine, offered] of cases) assert.equal(mayMove({ held_by, scopes: holds }, mine), offered, `${held_by} ${mine.join(' ')}`);
});

test('what a move is said to do is of its wall where its program is started here, and of where it is listed where it is not', () => {
  assert.deepEqual(movingSaid({ external: false, active: true }, 'Cafe'), [
    'From its next piece of work its wall shows its program Cafe’s folder, and it is listed under Cafe.',
    'Nothing else of it changes: it holds what it held.',
    'One with work in hand is not moved until that is over.',
  ]);
  assert.match(movingSaid({ external: false, active: false }, null)[0] ?? '', /a home of its own and no project’s folder/);
  assert.equal(movingSaid({ external: false, active: false }, null)[1], 'Nothing else of it changes: it holds what it held, and it stays stopped.');
  assert.equal(movingSaid({ external: true, active: true }, 'Cafe')[0], 'It is listed under Cafe, and offered that project’s cards.');
  assert.equal(movingSaid({ external: true, active: true }, null)[0], 'It is listed under no project.');
  assert.equal(movedSaid('helper', 'Cafe'), 'helper works in Cafe from its next piece of work.');
  assert.equal(movedSaid('helper', null), 'helper works in no project from its next piece of work.');
});
