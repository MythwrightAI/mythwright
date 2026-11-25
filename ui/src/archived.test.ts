import assert from 'node:assert/strict';
import { test } from 'node:test';
import { archivedOnly, archivedSays, isArchived, live, mayArchive, waiting } from './archived.ts';
import { channelsWait } from './conversed.ts';
import { asListed } from './inbox.ts';
import type { Line } from './inbox.ts';
import { channelItems, holds, narrowed, ofChannels } from './panelsought.ts';
import { gathered, mostWaiting, unseen, waitingIn } from './workspaces.ts';
import type { Workspace } from './workspaces.ts';

const on = (at: number) => ({ at, by: 'user:nell', name: 'Nell' });
const channel = (id: string, archived?: number, extra: { kept?: boolean; members?: string[]; project?: string | null } = {}) => ({
  id,
  name: id,
  project: extra.project ?? null,
  kept: extra.kept,
  members: (extra.members ?? []).map((actor) => ({ actor, name: actor })),
  archived: archived === undefined ? undefined : on(archived),
});
const space = (id: string, channels: string[], projects: string[] = []): Workspace => ({ id, name: id, created: 1, projects, channels });

test('a channel is archived where the daemon says when and by whom, and not otherwise', () => {
  assert.equal(isArchived(channel('general', 5)), true);
  assert.equal(isArchived(channel('general')), false);
  assert.equal(isArchived(null), false);
  assert.equal(isArchived({ archived: null }), false);
});

test('the archived ones are listed apart, the latest archived first, and the rest as they were given', () => {
  const all = [channel('a', 1), channel('b'), channel('c', 9), channel('d')];
  assert.deepEqual(
    live(all).map((one) => one.id),
    ['b', 'd'],
  );
  assert.deepEqual(
    archivedOnly(all).map((one) => one.id),
    ['c', 'a'],
  );
});

test('whoever may write in a channel is offered archiving it and giving it back, and of a private one whoever is in it or reads every private one', () => {
  const writes = { actor: 'user:mira', scopes: ['channels'] };
  const reads = { actor: 'user:mira', scopes: ['channels:read'] };
  const owner = { actor: 'user:ada', scopes: ['channels', 'private'] };
  assert.equal(mayArchive(channel('general'), writes), true);
  assert.equal(mayArchive(channel('general'), reads), false);
  const kept = channel('ch_1', undefined, { kept: true, members: ['user:nell'] });
  assert.equal(mayArchive(kept, writes), false);
  assert.equal(mayArchive(channel('ch_1', undefined, { kept: true, members: ['user:mira'] }), writes), true);
  assert.equal(mayArchive(kept, owner), true);
});

test('nothing of an archived channel is counted as waiting unread, anywhere a count is said', () => {
  const unread = { general: 3, quiet: 4, other: 2 };
  const all = [channel('general'), channel('quiet', 1), channel('other', undefined)];
  assert.equal(waiting(all[0]!, unread), 3);
  assert.equal(waiting(all[1]!, unread), 0);
  const spaces = [space('front', ['general', 'quiet']), space('back', ['other'])];
  const front = { kind: 'space' as const, id: 'front' };
  const back = { kind: 'space' as const, id: 'back' };
  assert.equal(waitingIn(all, unread, front, spaces, []), 3);
  assert.equal(unseen(all, unread, back, spaces, []), 3);
  assert.equal(mostWaiting([channel('quiet', 1), channel('other')], unread, front, spaces, []), 'back');
  assert.equal(mostWaiting([channel('quiet', 1)], unread, back, spaces, []), null);
  assert.equal(channelsWait([channel('quiet', 1)], unread), false);
  assert.equal(channelsWait([channel('quiet', 1), channel('general')], unread), true);
});

test('the side panel gathers an archived channel under the Archived part alone, of its own workspace, the latest archived first', () => {
  const spaces = [space('front', ['general', 'old', 'older'], ['shop']), space('back', ['far'])];
  const projects = [{ id: 'shop' }];
  const all = [channel('general'), channel('old', 3), channel('older', 1), channel('shop-talk', 7, { project: 'shop' }), channel('far', 9), channel('shop-live', undefined, { project: 'shop' })];
  const found = gathered(all, spaces, projects, { kind: 'space', id: 'front' });
  assert.deepEqual(
    found.loose.map((one) => one.id),
    ['general'],
  );
  assert.deepEqual(
    found.projects.map(({ channels }) => channels.map((one) => one.id)),
    [['shop-live']],
  );
  assert.deepEqual(
    found.archived.map((one) => one.id),
    ['shop-talk', 'old', 'older'],
  );
  // A project with only archived channels is no heading over nothing.
  const only = gathered([channel('shop-talk', 7, { project: 'shop' })], spaces, projects, { kind: 'space', id: 'front' });
  assert.deepEqual(only.projects, []);
});

test('the inbox lists a line of an archived conversation under its Archived part, and every other line where it was', () => {
  const line = (id: string, said: Line['said']): Line => ({ id, said }) as Line;
  const lines = [
    line('a', { conversation: true, channel_archived: true } as Line['said']),
    line('b', { conversation: true } as Line['said']),
    line('c', { channel_archived: true } as Line['said']),
    line('d', undefined),
  ];
  const { live: shown, archived } = asListed(lines);
  assert.deepEqual(
    shown.map((one) => one.id),
    ['b', 'c', 'd'],
  );
  assert.deepEqual(
    archived.map((one) => one.id),
    ['a'],
  );
});

test('an archived channel says by whom, and that nothing is said there until it is given back', () => {
  assert.equal(archivedSays(channel('general', 1)), 'This channel is archived by Nell: it is read as before, and nothing is said here until it is given back.');
  assert.equal(archivedSays({ archived: { at: 1, by: '', name: '' } }), 'This channel is archived: it is read as before, and nothing is said here until it is given back.');
});

test('the panel box finds an archived channel under the Archived part, which it holds open', () => {
  const listed = { pinned: [], loose: [channel('general')], projects: [], groups: [], archived: [channel('ledger', 1)] };
  const found = narrowed(channelItems(listed), 'ledger')!;
  assert.ok(found.shown.has(ofChannels.channel(ofChannels.archived, 'ledger')));
  assert.ok(holds(found, ofChannels.archived));
  assert.ok(!found.shown.has(ofChannels.channel(ofChannels.loose, 'general')));
  // Its name is carried down: `archived` finds what is under it.
  assert.ok(narrowed(channelItems(listed), 'archived')!.shown.has(ofChannels.channel(ofChannels.archived, 'ledger')));
});
