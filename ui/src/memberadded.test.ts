import assert from 'node:assert/strict';
import { test } from 'node:test';
import { agentLine, agentsFound, beginsAs, invitedSaid, inviting, kindsOffered, lastLine, lineCalled, listPlaced, marking, nobodySaid } from './memberadded.ts';

const open = { id: 'general', name: 'general', members: [{ actor: 'user:ada', name: 'Ada' }] };
const kept = { id: 'ch_porch', name: 'ch_porch', kept: true, title: 'porch', members: [{ actor: 'user:ada', name: 'Ada' }] };
const as = (scopes: string[], actor = 'user:ada') => ({ actor, scopes, owner: false });
const MEMBER = ['projects', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];

test('who is offered what beside a channel goes by what the daemon would refuse', () => {
  const cases: [string, Parameters<typeof kindsOffered>, string][] = [
    ['the owner, on a plan for one', [open, as(['*']), false], 'agent,person,bot'],
    ['the owner, inviting', [open, as(['*']), true], 'agent,person,bot'],
    // A member holds no shell, which std/bot lists under its permissions.
    ['a member', [open, as(MEMBER), false], 'agent,person'],
    ['one who may not change agents', [open, as(['channels', 'blueprints', 'agents:read']), false], 'person'],
    ['one who may only read channels', [open, as(['channels:read', 'agents', 'blueprints', 'shell']), false], ''],
    // A bot takes applying, the channels and shell, each of them.
    ['one who may apply and has shell', [open, as(['channels', 'blueprints:run', 'shell']), false], 'person,bot'],
    ['one who may apply and has no shell', [open, as(['channels', 'blueprints:run']), false], 'person'],
    // Of a private channel, only somebody in it adds to it.
    ['somebody outside a private channel', [kept, as(MEMBER, 'user:grace'), false], ''],
    ['somebody in a private channel', [kept, as(MEMBER), false], 'agent,person'],
    // Whoever may invite is offered a person, to invite, though they may not add here.
    ['whoever may invite, outside a private channel', [kept, as(['admin', 'channels'], 'user:grace'), true], 'person'],
  ];
  for (const [who, args, offered] of cases) assert.equal(kindsOffered(...args).join(), offered, who);
});

test('the dialog begins on the kind asked for where it is offered, else the first', () => {
  assert.equal(beginsAs(['agent', 'person', 'bot']), 'agent');
  assert.equal(beginsAs(['agent', 'person', 'bot'], 'bot'), 'bot');
  assert.equal(beginsAs(['person'], 'agent'), 'person');
  assert.equal(beginsAs([]), null);
});

test('inviting a person is offered to whoever may, and the plan is said only to whoever manages the installation', () => {
  assert.equal(inviting(true, ['*']), 'invite');
  assert.equal(inviting(false, ['*']), 'plan');
  assert.equal(inviting(false, ['admin']), 'plan');
  assert.equal(inviting(false, MEMBER), 'nothing');
});

test('an invited person is told that registering does not bring them in', () => {
  assert.match(invitedSaid('#general', true), /they choose Register and enter it\. It works once\. Registering does not bring them into #general: once they have registered, add them here\.$/);
  assert.match(invitedSaid('porch', false), /somebody in it adds them once they have registered\.$/);
});

test('what is said where nobody can be added from those with an account', () => {
  assert.equal(nobodySaid(1, 0), 'Nobody else has an account here yet.');
  assert.equal(nobodySaid(3, 0), 'Everyone with an account here is already in this channel.');
  assert.equal(nobodySaid(3, 2), null);
});

test('a line of the list says where an agent works, what it runs on or that it is external, and how it stands', () => {
  const helper = { name: 'helper', adapter: 'builtin', active: true, project: 'shop' };
  assert.deepEqual(agentLine(helper, { project: 'Shop', on: null, here: false }), { name: 'helper', said: ['Shop'] });
  assert.deepEqual(agentLine(helper, { project: 'Shop', on: 'model-one', here: true }), { name: 'helper', said: ['Shop', 'model-one', 'in this channel'] });
  // One the factory does not start says external, and never what it runs on.
  const walls = { name: 'walls', adapter: 'external', active: true, project: null };
  assert.deepEqual(agentLine(walls, { project: null, on: 'model-one', here: false }), { name: 'walls', said: ['no project', 'external'] });
  assert.deepEqual(agentLine({ ...walls, active: false }, { project: null, on: null, here: true }), { name: 'walls', said: ['no project', 'external', 'stopped', 'in this channel'] });
  // A project the page does not know of is said by its id.
  assert.deepEqual(agentLine({ ...helper, project: 'gone' }, { project: null, on: null, here: false }), { name: 'helper', said: ['gone'] });
});

test('typing narrows the list by name and by what is said beside it, those stopped last', () => {
  const lines = [
    { name: 'night', said: ['Shop', 'stopped'] },
    { name: 'helper', said: ['Shop', 'model-one'] },
    { name: 'walls', said: ['no project', 'external'] },
    { name: 'builder', said: ['Cafe', 'model-one', 'in this channel'] },
  ];
  const names = (typed: string) => agentsFound(lines, typed).map((line) => line.name).join();
  assert.equal(names(''), 'helper,walls,builder,night');
  assert.equal(names('ext'), 'walls');
  assert.equal(names('shop'), 'helper,night');
  assert.equal(names('MODEL cafe'), 'builder');
  assert.equal(names('in this channel'), 'builder');
  assert.equal(names('gate-keeper'), '');
});

test('the last line says what a name no agent has comes to', () => {
  const agents = [
    { name: 'helper', adapter: 'builtin', active: true },
    { name: 'night', adapter: 'builtin', active: false },
  ];
  assert.equal(lastLine(false, '', agents), 'A name no agent has makes a new one, run by this factory.');
  assert.equal(lastLine(true, '  ', agents), 'A name no agent has makes a new one, invited from outside.');
  assert.equal(lastLine(false, 'Gate Keeper', agents), 'No agent is called gate-keeper: a new one is made.');
  assert.equal(lastLine(true, 'gate-keeper', agents), 'No agent is called gate-keeper: a new one is invited.');
  assert.equal(lastLine(false, 'night', agents), 'night is stopped: it is made again.');
  assert.equal(lastLine(true, 'night', agents), 'night is stopped: it is invited again.');
  // A working agent's own line says it is that one.
  assert.equal(lastLine(false, 'helper', agents), null);
});

test('the mark goes round the lines by the arrows, from none', () => {
  assert.equal(marking(-1, 3, 'ArrowDown'), 0);
  assert.equal(marking(-1, 3, 'ArrowUp'), 2);
  assert.equal(marking(2, 3, 'ArrowDown'), 0);
  assert.equal(marking(0, 3, 'ArrowUp'), 2);
  assert.equal(marking(1, 3, 'a'), 1);
  // Fewer lines than where the mark was: it is on the last.
  assert.equal(marking(4, 2, 'a'), 1);
  assert.equal(marking(0, 0, 'ArrowDown'), -1);
});

test('the list goes under its field, over it where only there it fits, and else where there is more room, never over the field', () => {
  const field = { left: 20, top: 300, bottom: 336, width: 300 };
  const window = { width: 390, height: 780 };
  assert.deepEqual(listPlaced(field, 200, window), { left: 20, top: 340, width: 300, most: null });
  // No room under: over it.
  assert.deepEqual(listPlaced({ ...field, top: 700, bottom: 736 }, 200, window), { left: 20, top: 496, width: 300, most: null });
  // Room for it on neither side: on the side with more, as tall as that room.
  assert.deepEqual(listPlaced({ ...field, top: 420, bottom: 456 }, 450, window), { left: 20, top: 8, width: 300, most: 408 });
  assert.deepEqual(listPlaced({ ...field, top: 300, bottom: 336 }, 500, window), { left: 20, top: 340, width: 300, most: 432 });
  // Kept inside the window at its sides.
  assert.equal(listPlaced({ ...field, left: 200 }, 100, window).left, 82);
});

test('the line that adds somebody names only what may be added, and is none where nothing may', () => {
  const cases: [Parameters<typeof lineCalled>[0], string | null][] = [
    [['agent', 'person', 'bot'], 'Add a person, agent or bot'],
    [['agent', 'person'], 'Add a person or agent'],
    [['person', 'bot'], 'Add a person or bot'],
    [['agent', 'bot'], 'Add an agent or bot'],
    [['person'], 'Add a person'],
    [['agent'], 'Add an agent'],
    [['bot'], 'Add a bot'],
    [[], null],
  ];
  for (const [offered, said] of cases) assert.equal(lineCalled(offered), said, offered.join() || 'nothing');
});
