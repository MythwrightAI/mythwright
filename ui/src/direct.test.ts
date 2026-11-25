import assert from 'node:assert/strict';
import { test } from 'node:test';
import { agentSaid, directWith, isDirect, lineTitle, memberActs, membersListed, pressOf, waitingAll, waitsIn } from './direct.ts';
import type { Agented, MemberLine, Talked } from './direct.ts';
import type { Line } from './inbox.ts';
import type { Workspace } from './workspaces.ts';

const spaces: Workspace[] = [
  { id: 'ada-s-workspace', name: "Ada's workspace", created: 1, projects: ['shop'], channels: [], usual: true },
  { id: 'front', name: 'Front', created: 2, projects: ['depot'], channels: [] },
];

const people = [
  { actor: 'user:u2', name: 'Nell' },
  { actor: 'user:u1', name: 'Ada' },
  { actor: 'user:u3', name: 'bob' },
];
const agent = (name: string, over: Partial<Agented> = {}): Agented => ({ name, adapter: 'builtin', project: null, active: true, ...over });
const agents = [
  agent('worker-10', { project: 'shop' }),
  agent('worker-2', { project: 'shop' }),
  agent('scout', { adapter: 'external' }),
  agent('idle', { active: false }),
  agent('depot-worker', { project: 'depot' }),
];
const me = { actor: 'user:u1', name: 'Ada' };
const shown = (lines: MemberLine[]) => lines.map((one) => (one.said ? `${one.name} (${one.said})` : one.name));

test('whoever reads this is first and says so, every other person by name, then the agents not stopped first and each by name', () => {
  const lines = membersListed(people, agents, me, { kind: 'space', id: 'ada-s-workspace' }, spaces);
  assert.deepEqual(shown(lines), ['Ada (you)', 'bob', 'Nell', 'scout (agent (external))', 'worker-2 (agent)', 'worker-10 (agent)', 'idle (stopped)']);
  assert.equal(lines[0]!.kind, 'you');
  assert.ok(lines.find((one) => one.name === 'idle')!.stopped);
  assert.equal(lines.find((one) => one.name === 'scout')!.slot, 'scout');
});

test('the agents of another workspace\'s project are not listed, and the factory\'s own are listed in every workspace', () => {
  const front = shown(membersListed(people, agents, me, { kind: 'space', id: 'front' }, spaces));
  assert.ok(front.includes('depot-worker (agent)'));
  assert.ok(!front.some((one) => one.startsWith('worker-')));
  assert.ok(front.includes('scout (agent (external))') && front.includes('idle (stopped)'));
  // Held to nothing (there is no workspace), every agent is listed; and every person is listed in every workspace.
  const all = shown(membersListed(people, agents, me, { kind: 'all' }, []));
  assert.equal(all.filter((one) => one.includes('agent') || one.includes('stopped')).length, agents.length);
  assert.ok(['bob', 'Nell'].every((one) => front.includes(one)));
});

test('a bot is never listed, nor anything that is not a person among the people', () => {
  const lines = membersListed([...people, { actor: 'bot:factory', name: 'factory' }, { actor: 'agent:x', name: 'x' }], [], me, { kind: 'all' }, []);
  assert.deepEqual(shown(lines), ['Ada (you)', 'bob', 'Nell']);
});

test('whoever reads this is listed though the people could not be read, and nobody is listed as them for an agent signed in', () => {
  assert.deepEqual(shown(membersListed([], [], me, { kind: 'all' }, [])), ['Ada (you)']);
  assert.deepEqual(shown(membersListed(people, [], { actor: 'agent:scout', name: 'scout' }, { kind: 'all' }, [])), ['Ada', 'bob', 'Nell']);
  assert.deepEqual(shown(membersListed(people, [], null, { kind: 'all' }, [])), ['Ada', 'bob', 'Nell']);
});

test('what an agent is, as the list of who is in a channel says it', () => {
  assert.equal(agentSaid(agent('a')), 'agent');
  assert.equal(agentSaid(agent('a', { adapter: 'external' })), 'agent (external)');
  assert.equal(agentSaid(agent('a', { active: false })), 'stopped');
  assert.equal(agentSaid(agent('a', { adapter: 'external', active: false })), 'stopped');
});

const talk = (id: string, members: string[], over: Partial<Talked> = {}): Talked => ({ id, created: 1, conversation: true, members: members.map((actor) => ({ actor })), ...over });
const line = (place: string, over: Partial<Line> = {}): Line => ({
  id: `inb_${place}`,
  owner: 'user:u1',
  kind: 'message',
  about: place,
  place,
  from: 'user:u2',
  why: 'conversation',
  count: 1,
  created: 1,
  raised: 1,
  read: null,
  away: null,
  ...over,
});

test('a direct message is a conversation of exactly the two, whatever bot is in it, and never one with oneself', () => {
  assert.ok(isDirect(talk('c1', ['user:u1', 'user:u2']), 'user:u1', 'user:u2'));
  assert.ok(isDirect(talk('c1', ['user:u2', 'user:u1', 'bot:factory']), 'user:u1', 'user:u2'));
  assert.ok(!isDirect(talk('c1', ['user:u1', 'user:u2', 'agent:scout']), 'user:u1', 'user:u2'));
  assert.ok(!isDirect(talk('c1', ['user:u1']), 'user:u1', 'user:u1'));
  assert.ok(!isDirect(talk('c1', ['user:u1', 'user:u2'], { conversation: false }), 'user:u1', 'user:u2'));
  assert.ok(!isDirect(talk('c1', ['user:u1', 'user:u3']), 'user:u1', 'user:u2'));
});

test('the direct message with somebody is the one not archived, then the one last said something in, then the one made last', () => {
  const channels = [
    talk('old', ['user:u1', 'user:u2'], { created: 5 }),
    talk('new', ['user:u1', 'user:u2'], { created: 9 }),
    talk('gone', ['user:u1', 'user:u2'], { created: 20, archived: { at: 1 } }),
    talk('three', ['user:u1', 'user:u2', 'user:u3'], { created: 30 }),
    { id: 'general', created: 40, members: [{ actor: 'user:u1' }, { actor: 'user:u2' }] },
  ];
  assert.equal(directWith(channels, [], 'user:u1', 'user:u2'), 'new');
  assert.equal(directWith(channels, [line('old', { raised: 50 })], 'user:u1', 'user:u2'), 'old');
  assert.equal(directWith([channels[2]!], [], 'user:u1', 'user:u2'), 'gone');
  assert.equal(directWith(channels, [], 'user:u1', 'user:u3'), null);
  assert.equal(directWith(channels, [], 'user:u1', 'user:u1'), null);
});

test('what waits in a direct message is what was heard there, and at least one where its line in the inbox waits', () => {
  const dm = { id: 'c1' };
  assert.equal(waitsIn(dm, {}, []), 0);
  assert.equal(waitsIn(dm, {}, [line('c1')]), 1);
  assert.equal(waitsIn(dm, { c1: 3 }, [line('c1')]), 3);
  assert.equal(waitsIn(dm, { c1: 2 }, []), 2);
  assert.equal(waitsIn(dm, {}, [line('c1', { read: 5 })]), 0);
  assert.equal(waitsIn(dm, {}, [line('c1', { away: 5 })]), 0);
  // A line that names the reader in it, of another kind, is not the conversation's.
  assert.equal(waitsIn(dm, {}, [line('c1', { kind: 'named' })]), 0);
  assert.equal(waitsIn({ id: 'c1', archived: { at: 1 } }, { c1: 4 }, [line('c1')]), 0);
  assert.equal(waitsIn(null, { c1: 4 }, [line('c1')]), 0);
  assert.equal(waitingAll([1, 0, 3]), 4);
  assert.equal(waitingAll([]), 0);
});

test("a member's menu says who they are, opens or writes the direct message, and offers what a person holds to whoever manages", () => {
  const [you, nell, idle] = [
    { actor: 'user:u1', name: 'Ada', kind: 'you', said: 'you', slot: null, stopped: false },
    { actor: 'user:u2', name: 'Nell', kind: 'person', said: null, slot: null, stopped: false },
    { actor: 'agent:idle', name: 'idle', kind: 'agent', said: 'stopped', slot: 'idle', stopped: true },
  ] as MemberLine[];
  assert.equal(pressOf(you!, null, true), null);
  assert.equal(pressOf(you!, 'c1', true), null);
  assert.equal(pressOf(nell!, 'c1', false), 'open');
  assert.equal(pressOf(nell!, null, true), 'write');
  assert.equal(pressOf(nell!, null, false), null);
  assert.equal(pressOf(idle!, null, true), null);
  assert.equal(pressOf(idle!, 'c1', true), 'open');
  assert.deepEqual(memberActs(you!, null, true, true), ['details', 'permissions']);
  assert.deepEqual(memberActs(you!, null, true, false), ['details']);
  assert.deepEqual(memberActs(nell!, null, true, false), ['details', 'write']);
  assert.deepEqual(memberActs(nell!, 'c1', false, true), ['details', 'open', 'permissions']);
  assert.deepEqual(memberActs(nell!, null, false, false), ['details']);
  // An agent's own lines are its line's on the Agents tab, and what it holds is changed there: no line of a person's.
  assert.deepEqual(memberActs(idle!, null, true, true), ['details']);
  assert.deepEqual(memberActs(idle!, 'c1', true, true), ['details', 'open']);
  assert.equal(lineTitle(you!), 'You: signed in here. Press for what can be done');
  assert.equal(lineTitle(nell!), 'Nell: who they are, a message, and what else can be done');
  assert.equal(lineTitle(idle!), 'idle (stopped): who they are, a message, and what else can be done');
});
