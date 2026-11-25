import assert from 'node:assert/strict';
import { test } from 'node:test';
import { addClosed, addedBody, addedSaid, asksOf, beginsOn, broughtBy, comingOf, doneBy, invitedInto, invitesOn, takenBy, thereSaid, waysOffered } from './agentadded.ts';
import type { Coming } from './agentadded.ts';
import type { Shown } from './narrowing.ts';

test('the dialog begins on the project the page is narrowed to where it is offered, and otherwise on none', () => {
  const offered = [{ id: 'shop' }, { id: 'cafe' }];
  const cases: [Shown, string][] = [
    [{ kind: 'projects', projects: ['cafe'], space: 'ada', project: 'cafe' }, 'cafe'],
    // All of a workspace, every workspace, and not known yet: none.
    [{ kind: 'projects', projects: ['shop', 'cafe'], space: 'ada', project: null }, ''],
    [{ kind: 'everything' }, ''],
    [{ kind: 'waiting' }, ''],
    // A project the dialog does not offer is not begun on.
    [{ kind: 'projects', projects: ['quarry'], space: 'annex', project: 'quarry' }, ''],
  ];
  for (const [shown, begins] of cases) assert.equal(beginsOn(shown, offered), begins, JSON.stringify(shown));
});

test('a name is an agent’s already by what the daemon would call it, stopped or not', () => {
  const agents = [
    { name: 'helper', active: true },
    { name: 'night-shift', active: false },
  ];
  assert.equal(takenBy('Helper', agents)?.name, 'helper');
  assert.equal(takenBy('Night Shift', agents)?.name, 'night-shift');
  assert.equal(takenBy('builder', agents), null);
  assert.equal(takenBy('  ', agents), null);
});

test('it is closed for a name that is taken and, in a project with environments, until one is chosen', () => {
  assert.equal(addClosed(null, [], ''), null);
  assert.equal(addClosed(null, ['dev', 'prod'], 'prod'), null);
  assert.match(addClosed('helper', [], '') ?? '', /^There is already an agent called helper\./);
  assert.match(addClosed(null, ['dev', 'prod'], '') ?? '', /^Choose the environment it works under/);
  assert.match(addClosed(null, ['dev'], 'staging') ?? '', /^Choose the environment/);
  // The name is said first: choosing an environment would not open it.
  assert.match(addClosed('helper', ['dev'], '') ?? '', /^There is already/);
});

test('what is sent says the project and an environment only where there are any, and nothing left empty', () => {
  const asked = { name: ' builder ', adapter: 'builtin', model: '', instructions: '', mode: '', project: '', environments: [] as string[], under: '' };
  assert.deepEqual(addedBody(asked), { name: 'builder', adapter: 'builtin' });
  assert.deepEqual(addedBody({ ...asked, project: 'shop' }), { name: 'builder', adapter: 'builtin', project: 'shop' });
  assert.deepEqual(addedBody({ ...asked, project: 'cafe', environments: ['dev', 'prod'], under: 'prod' }), {
    name: 'builder',
    adapter: 'builtin',
    project: 'cafe',
    environment: 'prod',
  });
  // An environment left over from a project chosen before is not sent for one with none.
  assert.deepEqual(addedBody({ ...asked, project: 'shop', under: 'prod' }), { name: 'builder', adapter: 'builtin', project: 'shop' });
  // Nor for no project.
  assert.deepEqual(addedBody({ ...asked, environments: ['dev'], under: 'dev' }), { name: 'builder', adapter: 'builtin' });
  assert.deepEqual(addedBody({ ...asked, model: 'm-1', instructions: 'Be brief.', mode: 'read' }), {
    name: 'builder',
    adapter: 'builtin',
    model: 'm-1',
    instructions: 'Be brief.',
    permission_mode: 'read',
  });
});

test('what is said once it is added names where it works', () => {
  assert.equal(addedSaid('builder', null, null), 'builder is added, and works in no project.');
  assert.equal(addedSaid('builder', 'Shop', null), 'builder is added, and works in Shop.');
  assert.equal(addedSaid('builder', 'Cafe', 'prod'), 'builder is added, and works in Cafe under prod.');
});

test('beside a channel, a name an agent has is that agent whichever way was chosen, and a new one is made or invited by the way', () => {
  const agents = [
    { name: 'helper', adapter: 'builtin', active: true },
    { name: 'walls', adapter: 'external', active: true, external: { joined: true, invited_until: null } },
    { name: 'gone', adapter: 'external', active: false, external: { joined: true, invited_until: null } },
    { name: 'idle', adapter: 'builtin', active: false },
  ];
  const kind = (way: 'runs' | 'outside', typed: string) => {
    const coming = comingOf(way, typed, agents);
    return coming.kind === 'there' ? `there ${coming.slot.name}` : coming.kind;
  };
  assert.equal(kind('runs', ''), 'nothing');
  assert.equal(kind('outside', ' - '), 'nothing');
  assert.equal(kind('runs', 'Builder'), 'new');
  assert.equal(kind('outside', 'Gatekeeper'), 'invited');
  // One that is there, by what the daemon would call it, of either kind and whichever way was chosen.
  assert.equal(kind('runs', 'Walls'), 'there walls');
  assert.equal(kind('outside', 'walls'), 'there walls');
  assert.equal(kind('outside', 'helper'), 'there helper');
  // One that was stopped is made again, or invited again, as the way says.
  assert.equal(kind('outside', 'gone'), 'invited');
  assert.equal(kind('runs', 'idle'), 'new');
});

test('what the dialog asks under the name, and what its button says, is of what it comes to', () => {
  const walls = { name: 'walls', adapter: 'external', active: true };
  const cases: [Coming, { program: boolean; invitation: boolean }, string][] = [
    // With no name yet, what the way chosen asks.
    [{ kind: 'nothing', way: 'runs' }, { program: true, invitation: false }, 'Add agent'],
    [{ kind: 'nothing', way: 'outside' }, { program: false, invitation: true }, 'Invite'],
    [{ kind: 'new' }, { program: true, invitation: false }, 'Add agent'],
    [{ kind: 'invited' }, { program: false, invitation: true }, 'Invite'],
    // One that is there is asked nothing else.
    [{ kind: 'there', slot: walls }, { program: false, invitation: false }, 'Add agent'],
  ];
  for (const [coming, asks, button] of cases) {
    assert.deepEqual(asksOf(coming), asks, JSON.stringify(coming));
    assert.equal(doneBy(coming), button, JSON.stringify(coming));
  }
});

test('one from outside that is there is brought in as a member is, and one this factory runs by its name where whoever asks may apply', () => {
  assert.equal(broughtBy({ name: 'walls', adapter: 'external', active: true }, true), 'member');
  assert.equal(broughtBy({ name: 'walls', adapter: 'external', active: true }, false), 'member');
  assert.equal(broughtBy({ name: 'helper', adapter: 'builtin', active: true }, true), 'name');
  // The route that brings one in by its name applies its text as whoever asks: somebody who may not is not sent there.
  assert.equal(broughtBy({ name: 'helper', adapter: 'builtin', active: true }, false), 'member');
});

test('one that is there is said as what it is, one from outside with how it stands', () => {
  const NOW = 1_000_000;
  assert.equal(
    thereSaid({ name: 'walls', adapter: 'external', active: true, external: { joined: true, invited_until: null } }, NOW),
    'walls is an agent from outside (joined). Adding it brings it into this channel, and nothing else of it changes.',
  );
  assert.equal(
    thereSaid({ name: 'gate', adapter: 'external', active: true, external: { joined: false, invited_until: NOW + 3_600_000 } }, NOW),
    'gate is an agent from outside (invited · has yet to join). Adding it brings it into this channel, and nothing else of it changes.',
  );
  assert.equal(thereSaid({ name: 'helper', adapter: 'builtin', active: true, project: 'shop' }, NOW), 'helper already has a slot (builtin, working on shop).');
  assert.equal(thereSaid({ name: 'rover', adapter: 'builtin', active: true }, NOW), 'rover already has a slot (builtin).');
});

test('one invited beside a channel begins on the channel’s project where it is offered', () => {
  const offered = [{ id: 'shop' }, { id: 'cafe' }];
  assert.equal(invitesOn('shop', offered), 'shop');
  assert.equal(invitesOn(null, offered), '');
  assert.equal(invitesOn(undefined, offered), '');
  assert.equal(invitesOn('quarry', offered), '');
});

test('what is said under its invitation is where it is, or why it is not there and how it is brought in', () => {
  assert.equal(invitedInto('gate', '#porch', null), 'gate is in #porch, listed as invited until it joins.');
  assert.equal(
    invitedInto('gate', 'Porch', 'you may not write in that channel'),
    "gate is invited, and was not brought into Porch: you may not write in that channel. It is brought in from the channel's members, by its name.",
  );
  assert.equal(invitedInto('gate', 'Porch', 'Refused. '), "gate is invited, and was not brought into Porch: Refused. It is brought in from the channel's members, by its name.");
});

test('the Agents tab offers each way only to whoever the daemon would let take it', () => {
  const cases: [string, string[], string][] = [
    ['the owner', ['*'], 'runs,outside'],
    ['one who changes agents and applies', ['agents', 'blueprints'], 'runs,outside'],
    ['one who changes agents and may only apply', ['agents:write', 'blueprints:run'], 'runs,outside'],
    // Adding one this factory runs applies its text as whoever asks: without that, only inviting is left.
    ['one who changes agents and does not apply', ['agents:write', 'blueprints:read'], 'outside'],
    ['one who applies and only reads agents', ['agents:read', 'blueprints'], ''],
    ['one who holds nothing of either', ['channels', 'board'], ''],
  ];
  for (const [who, scopes, ways] of cases) assert.equal(waysOffered(scopes).join(), ways, who);
});
