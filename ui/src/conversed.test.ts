import assert from 'node:assert/strict';
import { test } from 'node:test';
import { amongChannels, candidates, channelsWait, found, isConversation, mayOpenAsChannel, mayWrite, places, withOf, written } from './conversed.ts';
import { ALL, gathered, holdsChannel, leftOut, mostWaiting, unseen, waitingIn } from './workspaces.ts';
import type { Held, Workspace } from './workspaces.ts';

const ME = 'user:acct_nell';
const channel = (id: string, over: { project?: string | null; conversation?: boolean } = {}) => ({ id, project: null, ...over });

test('a conversation is a channel that says it is one, and is not listed among the channels', () => {
  assert.equal(isConversation({ conversation: true }), true);
  for (const none of [{}, { conversation: false }, null, undefined]) assert.equal(isConversation(none), false);
  const all = [channel('general'), channel('quay', { conversation: true }), channel('yard', { project: 'shed' })];
  assert.deepEqual(amongChannels(all).map((one) => one.id), ['general', 'yard']);
});

test('the side panel gathers and counts no conversation, whatever workspace it is of, and a channel as it always did', () => {
  const spaces: Workspace[] = [
    { id: 'usual', name: 'Usual', created: 1, usual: true, projects: ['shed'], channels: ['general', 'quay'] },
    { id: 'front', name: 'Front', created: 2, projects: [], channels: ['plans', 'gate'] },
  ];
  const projects = [{ id: 'shed' }];
  const talk = [channel('general'), channel('quay', { conversation: true }), channel('yard', { project: 'shed' }), channel('plans'), channel('gate', { conversation: true })];
  const unread = { general: 1, quay: 5, yard: 2, plans: 4, gate: 7 };
  const usual: Held = { kind: 'space', id: 'usual' };
  const front: Held = { kind: 'space', id: 'front' };
  // Held to nothing, to its own workspace or to another: it is in no list.
  for (const held of [ALL, usual, front]) {
    const here = gathered(talk, spaces, projects, held);
    const ids = [...here.loose, ...here.projects.flatMap((one) => one.channels)].map((one) => one.id);
    assert.ok(!ids.includes('quay') && !ids.includes('gate'), `${JSON.stringify(held)}: ${ids}`);
    for (const one of talk.filter((one) => one.conversation)) assert.equal(holdsChannel(held, one, spaces, projects), false);
  }
  assert.deepEqual(gathered(talk, spaces, projects, usual).loose.map((one) => one.id), ['general']);
  assert.deepEqual(gathered(talk, spaces, projects, ALL).loose.map((one) => one.id), ['general', 'plans']);
  // Nor is it among what a choice leaves out, or what waits there: its count is the inbox's.
  assert.deepEqual(leftOut(talk, usual, spaces, projects).map((one) => one.id), ['plans']);
  assert.equal(unseen(talk, unread, usual, spaces, projects), 4);
  assert.equal(unseen(talk, unread, front, spaces, projects), 3);
  assert.equal(waitingIn(talk, unread, usual, spaces, projects), 3);
  assert.equal(waitingIn(talk, unread, front, spaces, projects), 4);
  assert.equal(mostWaiting(talk, unread, usual, spaces, projects), 'front');
  assert.equal(mostWaiting([channel('gate', { conversation: true })], unread, usual, spaces, projects), null);
  // Whether anything waits in a channel goes by the channels, too.
  assert.equal(channelsWait(talk, { quay: 5, gate: 7 }), false);
  assert.equal(channelsWait(talk, { quay: 5, yard: 1 }), true);
  assert.equal(channelsWait(talk, { unread_of_one_not_read_yet: 2 }), false);
});

test('a conversation is written to the people and agents here, each saying which it is, and never to oneself or a bot', () => {
  const people = [
    { actor: 'user:acct_odo', name: 'Odo' },
    { actor: ME, name: 'Nell' },
    { actor: 'user:acct_mira', name: 'Mira' },
    { actor: 'bot:conductor', name: 'conductor' },
  ];
  const agents = [
    { name: 'scout', adapter: 'external', active: true },
    { name: 'builder', adapter: 'builtin', active: true },
    { name: 'retired', adapter: 'builtin', active: false },
  ];
  const all = candidates(people, agents, ME);
  assert.deepEqual(
    all.map((one) => [one.actor, one.kind]),
    [
      ['user:acct_mira', 'person'],
      ['user:acct_odo', 'person'],
      ['agent:builder', 'agent'],
      ['agent:scout', 'external agent'],
    ],
  );
  assert.deepEqual(candidates(people, agents, 'agent:scout').map((one) => one.name), ['Mira', 'Nell', 'Odo', 'builder']);
  assert.deepEqual(found(all, ' @MI ').map((one) => one.name), ['Mira']);
  assert.deepEqual(found(all, 'o').map((one) => one.name), ['Odo', 'scout']);
  assert.equal(found(all, '  ').length, 4);
  assert.deepEqual(found(all, 'nobody'), []);
});

test('writing one takes changing channels and one’s own box, and asks nothing while it is to nobody or says nothing', () => {
  assert.equal(mayWrite(['inbox', 'channels']), true);
  assert.equal(mayWrite(['*']), true);
  for (const less of [['inbox', 'channels:read'], ['inbox:read', 'channels'], ['channels'], []]) assert.equal(mayWrite(less), false, less.join());
  assert.deepEqual(written(['user:acct_mira'], ' The quay ', ' who mends it? '), { to: ['user:acct_mira'], subject: 'The quay', text: 'who mends it?' });
  assert.deepEqual(written(['agent:scout'], '  ', 'hello'), { to: ['agent:scout'], text: 'hello' }, 'with no subject the daemon calls it by its first words');
  assert.equal(written([], 'The quay', 'hello'), null);
  assert.equal(written(['agent:scout'], 'The quay', '  \n'), null);
});

test('a line says who else is in it, and one of its members is offered opening it as a channel', () => {
  const members = [
    { actor: ME, name: 'Nell' },
    { actor: 'user:acct_mira', name: 'Mira' },
    { actor: 'user:acct_odo', name: 'Odo' },
    { actor: 'agent:scout', name: 'scout' },
    { actor: 'bot:conductor', name: 'conductor' },
  ];
  assert.equal(withOf(members, ME), 'Mira, Odo and scout');
  assert.equal(withOf(members.slice(0, 3), ME), 'Mira and Odo');
  assert.equal(withOf(members.slice(0, 2), ME), 'Mira');
  assert.equal(withOf(members.slice(0, 1), ME), 'only you');
  const quay = { id: 'quay', conversation: true, members };
  const by = { actor: ME, scopes: ['channels', 'inbox'] };
  assert.equal(mayOpenAsChannel(quay, by), true);
  assert.equal(mayOpenAsChannel(quay, { ...by, scopes: ['channels:read', 'inbox'] }), false);
  assert.equal(mayOpenAsChannel(quay, { actor: 'user:acct_ada', scopes: ['*'] }), false, 'the owner reads it and is not in it');
  assert.equal(mayOpenAsChannel({ ...quay, conversation: false }, by), false, 'it is a channel already');
});

test('a channel a conversation becomes is offered the workspace showing first, and never nothing', () => {
  const spaces = [
    { id: 'back', name: 'Back' },
    { id: 'front', name: 'Front' },
  ];
  const projects = [{ id: 'shed', name: 'Shed' }];
  assert.deepEqual(
    places(spaces, projects, 'front').map((one) => [one.label, one.kind]),
    [
      ['Front', 'workspace'],
      ['Back', 'workspace'],
      ['Shed', 'project'],
    ],
  );
  assert.deepEqual(places(spaces, projects, null)[0]!.to, { workspace: 'back' });
  assert.deepEqual(places(spaces, projects, 'gone')[0]!.to, { workspace: 'back' });
  assert.deepEqual(places([], projects, null).map((one) => one.to), [{ project: 'shed' }]);
});
