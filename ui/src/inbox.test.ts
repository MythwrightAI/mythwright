import assert from 'node:assert/strict';
import { test } from 'node:test';
import { worn } from './clock.ts';
import { SENTENCE_LETTERS, TIP_LETTERS, cutAtWord, firstSentence, sentenceOf, tipOf, afterAway, afterRead, afterUnread, counted, recordSaid, isOf, offeredOf, opensItsPage, opensRecord, titleOf, toldWhen, fromOf, headSays, heard, heldStill, lineCalled, ofConversation, opens, readBySeeing, thingOf, inboxCalled, waitingIn, whereOf, wordsOf } from './inbox.ts';
import type { Line } from './inbox.ts';

const ME = 'user:acct_nell';

function named(id: string, over: Partial<Line> = {}, said: Partial<NonNullable<Line['said']>> = {}): Line {
  return {
    id,
    owner: ME,
    kind: 'named',
    about: `msg_${id}`,
    place: 'ch_general',
    from: 'user:acct_mira',
    why: 'named',
    count: 1,
    created: 10,
    raised: 10,
    read: null,
    away: null,
    said: { author: 'user:acct_mira', author_name: 'Mira', body: `@nell look at ${id}`, thread: null, created: 10, channel_name: 'general', ...said },
    ...over,
  };
}

const told: Line = { ...named('t'), kind: 'told', about: 'build', from: 'bot:conductor', why: 'told', says: 'The build\n is   green', said: undefined };

test('the inbox on the rail says how many wait, in no more than two digits, and nothing where none does', () => {
  assert.deepEqual([0, -1, 1, 42, 99, 100, 4000].map(counted), ['', '', '1', '42', '99', '99+', '99+']);
  assert.equal(inboxCalled(0), 'Inbox');
  assert.equal(inboxCalled(3), 'Inbox, 3 unread');
});

test('a line says who it is from, where it was said and its words, a private channel by what it is called', () => {
  const open = named('a');
  assert.deepEqual([fromOf(open), whereOf(open), wordsOf(open)], ['Mira', '#general', '@nell look at a']);
  const kept = named('b', {}, { channel_name: 'ch_1', channel_title: 'Pay review', channel_kept: true, thread: 'msg_root' });
  assert.equal(whereOf(kept), 'Pay review, in a thread', 'its name is its own id, which is shown nowhere');
  assert.equal(lineCalled(kept), 'Unread: Mira in Pay review, in a thread: @nell look at b');
  assert.equal(lineCalled({ ...open, read: 12 }), 'Mira in #general: @nell look at a');
  // What a blueprint told has words of its own, and is from whoever it says.
  assert.deepEqual([fromOf(told), whereOf(told), wordsOf(told)], ['conductor', '', 'The build is green']);
  assert.equal(lineCalled(told), 'Unread: conductor: The build is green');
});

test('a line opens its channel at the message, and one said under another at that one with its thread', () => {
  assert.deepEqual(opens(named('a')), { channel: 'ch_general', message: 'msg_a', thread: null });
  assert.deepEqual(opens(named('b', {}, { thread: 'msg_root' })), { channel: 'ch_general', message: 'msg_root', thread: 'msg_root' });
  assert.equal(opens(told), null, 'what opens nothing is nothing to press');
  assert.equal(opens({ ...named('c'), said: undefined }), null);
});

test('a channel that comes into view has read the lines that open it, and only those that wait', () => {
  const lines = [named('a'), named('b', { place: 'ch_ops' }), named('c', { place: 'ch_ops', read: 5 }), named('d', { place: 'ch_done', read: 5 }), named('e', { place: 'ch_away', away: 6 })];
  assert.deepEqual(waitingIn(lines), ['ch_general', 'ch_ops']);
  assert.deepEqual(readBySeeing(lines, ['ch_ops', 'ch_done', 'ch_away', 'ch_other', 'ch_ops']), ['ch_ops']);
  assert.deepEqual(readBySeeing(lines, []), []);
  assert.deepEqual(readBySeeing([], ['ch_ops']), []);
});

test('lines read are read once and lines put away are gone, and a list nothing changed in is the list it was', () => {
  const lines = [named('a'), named('b', { read: 5 }), named('c')];
  const read = afterRead(lines, ['a', 'b', 'nonesuch'], 20);
  assert.deepEqual(read.map((line) => line.read), [20, 5, null], 'one read before keeps when it was');
  assert.equal(afterRead(read, ['a', 'b'], 30), read, 'nothing to change, nothing drawn again');
  assert.equal(afterRead(lines, [], 30), lines);
  assert.deepEqual(afterAway(lines, ['b', 'nonesuch']).map((line) => line.id), ['a', 'c']);
  assert.equal(afterAway(lines, ['nonesuch']), lines);
});

test('what is said of the box settles the count, marks and drops lines, and asks for a reading only where a line was told', () => {
  const lines = [named('a'), named('b'), named('c')];
  const event = (topic: string, data: object) => ({ topic, data: { owner: ME, ...data } });
  // Told: the count is what was said, and the list is read again, since the event has no words.
  assert.deepEqual(heard(lines, 3, ME, event('inbox.told', { line: 'd', unread: 4 }), 50), { lines, unread: 4, again: true });
  // Read, one or many: marked where they are, with nothing read again.
  const read = heard(lines, 3, ME, event('inbox.read', { lines: ['a', 'c'], place: 'ch_general', unread: 1 }), 50);
  assert.deepEqual([read.lines.map((line) => line.read), read.unread, read.again], [[50, null, 50], 1, false]);
  // Put away, or gone with the channel they opened.
  const away = heard(lines, 3, ME, event('inbox.away', { lines: ['b'], unread: 2 }), 50);
  assert.deepEqual([away.lines.map((line) => line.id), away.unread, away.again], [['a', 'c'], 2, false]);
  // One that says no count leaves the count; another topic, and somebody else's box, change nothing.
  assert.equal(heard(lines, 3, ME, event('inbox.read', { lines: [] }), 50).unread, 3);
  assert.deepEqual(heard(lines, 3, ME, event('channel.message', { unread: 9 }), 50), { lines, unread: 3, again: false });
  const theirs = { topic: 'inbox.told', data: { owner: 'user:acct_mira', line: 'x', unread: 9 } };
  assert.deepEqual(heard(lines, 3, ME, theirs, 50), { lines, unread: 3, again: false });
});

test('the list holds still while it is pointed at: nothing comes in, a line read stays where it is, one put away leaves', () => {
  const was = ['a', 'b', 'c'];
  // A line told meanwhile is at the top of what was read, and is kept back.
  const now = [named('n'), named('a', { read: 9 }), named('b'), named('c')];
  const held = heldStill(was, now);
  assert.deepEqual([held.lines.map((line) => line.id), held.kept], [['a', 'b', 'c'], 1]);
  assert.equal(held.lines[0]!.read, 9, 'each line is as it is now');
  // One raised again would come to the top: it stays where it was.
  assert.deepEqual(heldStill(was, [named('c'), named('a'), named('b')]).lines.map((line) => line.id), ['a', 'b', 'c']);
  // One put away is listed no longer.
  assert.deepEqual(heldStill(was, [named('a'), named('c')]), { lines: [named('a'), named('c')], kept: 0 });
  assert.equal(headSays(0, 0), 'Nothing unread');
  assert.equal(headSays(2, 0), '2 unread');
  assert.equal(headSays(2, 1), '2 unread · 1 new');
});

test('a line of a conversation says what it is about and who else is in it, and opens it at what raised it last', () => {
  const members = [
    { actor: ME, name: 'Nell' },
    { actor: 'user:acct_mira', name: 'Mira' },
    { actor: 'agent:scout', name: 'scout' },
  ];
  const said = { channel_name: 'ch_quay', channel_title: 'The quay', channel_kept: true, conversation: true, members, body: 'who mends it?' };
  const quay = named('q', { kind: 'message', about: 'ch_quay', place: 'ch_quay', why: 'message', said_by: 'msg_latest' }, said);
  assert.equal(ofConversation(quay), true);
  assert.deepEqual([fromOf(quay), whereOf(quay), wordsOf(quay)], ['Mira', 'The quay · with Mira and scout', 'who mends it?']);
  assert.equal(lineCalled(quay), 'Unread: Mira in The quay · with Mira and scout: who mends it?');
  assert.deepEqual(opens(quay), { channel: 'ch_quay', message: 'msg_latest', thread: null });
  assert.deepEqual(opens({ ...quay, said: { ...quay.said!, thread: 'msg_root' } }), { channel: 'ch_quay', message: 'msg_root', thread: 'msg_root' });
  // Opened as a channel since, its line is a private channel's, and opens it the same.
  const channel = named('c', { kind: 'message', about: 'ch_quay', place: 'ch_quay', said_by: 'msg_latest' }, { channel_name: 'ch_quay', channel_title: 'Quay works', channel_kept: true });
  assert.equal(ofConversation(channel), false);
  assert.equal(whereOf(channel), 'Quay works');
  assert.deepEqual(opens(channel), { channel: 'ch_quay', message: 'msg_latest', thread: null });
  assert.equal(ofConversation(named('a')), false);
});

test('what a blueprint told opens what it says it is about: a channel the reader has, something running, or a record', () => {
  const about = (place: string): Line => ({ ...told, about: place, place });
  const channels = ['ch_general', 'ch_1m7x20qfm1mvc3bd'];
  assert.deepEqual(thingOf(about('ch_general'), channels), { kind: 'channel', id: 'ch_general' });
  assert.deepEqual(thingOf(about('in_1m7x20qk9k118w4b'), channels), { kind: 'running', id: 'in_1m7x20qk9k118w4b' });
  assert.deepEqual(thingOf(about('run_1m85442mg8j93j1c'), channels), { kind: 'run', id: 'run_1m85442mg8j93j1c' });
  // A card, a row of another table, and a document, which is known by where it is: each asked for by its id.
  for (const id of ['shop-4', 'shop-features-2', 'shop-docs:guides:install', 'in_place', 'inbox-3']) {
    assert.deepEqual(thingOf(about(id), channels), { kind: 'record', id }, id);
  }
  // A channel the reader has not got is not taken for one by how its id begins: asked for as a record, it is nothing they are shown.
  assert.deepEqual(thingOf(about('ch_elsewhere'), channels), { kind: 'record', id: 'ch_elsewhere' });
  // What opens it is its place, where that is not what it is about; about nothing, it opens nothing.
  assert.deepEqual(thingOf({ ...told, about: 'shop-4', place: ' ch_general ' }, channels), { kind: 'channel', id: 'ch_general' });
  assert.equal(thingOf(about(''), channels), null);
  assert.equal(thingOf(about('  '), channels), null);
  // A line of something said opens its channel at the message, and is none of this.
  assert.equal(thingOf(named('a'), channels), null);
  assert.equal(opens(about('shop-4')), null);
  // It is from the bot, by its name, and says its own words.
  assert.deepEqual([fromOf(told), whereOf(told), wordsOf(told)], ['conductor', '', 'The build is green']);
});

test('a told line opens a page of its own, and a line of something said its channel at the message', () => {
  assert.equal(opensItsPage(told), true);
  assert.equal(opens(told), null);
  for (const line of [named('n'), { ...named('c'), kind: 'message' as const, said_by: 'msg_c2' }]) {
    assert.equal(opensItsPage(line), false, line.kind);
    assert.ok(opens(line), line.kind);
  }
});

test('a line\'s page is called by its first words, cut with an ellipsis where there are more', () => {
  assert.equal(titleOf(told), 'The build is green');
  const long = { ...told, says: 'The preview of origin/dev is ready to try at http://127.0.0.1:7760/' };
  assert.equal(titleOf(long), 'The preview of origin/dev is ready…');
  // A word longer than a title is cut by its letters.
  const one = { ...told, says: `https://example.com/${'a'.repeat(60)}` };
  assert.equal(titleOf(one), `${`https://example.com/${'a'.repeat(60)}`.slice(0, 39)}…`);
  assert.equal(titleOf({ ...told, says: '  ' }), 'Inbox message');
  assert.equal(titleOf(named('n')), '@nell look at n');
});

test('a line\'s page is called by its words, and a block of code is no words', () => {
  const says = (text: string) => titleOf({ ...told, says: text });
  assert.equal(says('The preview is ready:\n```\nmw sandbox run std/board\n```\nTry it.'), 'The preview is ready:');
  assert.equal(says('```sh\ncargo build --release\n```\nThen run the tests.'), 'Then run the tests.');
  assert.equal(says('    cargo build\n\nThen run it.'), 'Then run it.');
  assert.equal(says('Before:\n\n    make check\n\nafter'), 'Before:');
  assert.equal(says('Run `cargo test` now'), 'Run cargo test now');
  assert.equal(says('One\ntwo\nthree'), 'One two three');
  assert.equal(says('```\nonly code\n```'), 'Inbox message');
  assert.equal(says('```sh\nmake\n```\nThe build of `origin/dev` passed and is ready to try now'), 'The build of origin/dev passed and…');
});

test('when a line was told is said on the clock and from now, and how often only where it was told again', () => {
  // On the clock of the reader's zone, as they have a time written: here Denver's, on a clock of twenty-four hours.
  worn.value = { zone: 'America/Denver', hours: '24h' };
  const at = Date.UTC(2026, 10, 22, 21, 3);
  const once = toldWhen({ ...told, created: at, raised: at, count: 1 }, at + 5 * 60_000);
  assert.deepEqual(once, { at: '14:03 MST', ago: '5m ago', times: null });
  const thrice = toldWhen({ ...told, created: at - 3_600_000, raised: at, count: 3 }, at + 10_000);
  assert.deepEqual(thrice, { at: '14:03 MST', ago: 'just now', times: 'Told 3 times, first 13:03 MST' });
});

test('a line\'s page offers to have it wait unread again once read, to read it while it waits, and to put it away; nothing once put away', () => {
  assert.deepEqual(offeredOf({ ...told, read: null }), { unread: false, read: true, away: true });
  assert.deepEqual(offeredOf({ ...told, read: 20 }), { unread: true, read: false, away: true });
  assert.deepEqual(offeredOf({ ...told, read: 20, away: 30 }), { unread: false, read: false, away: false });
});

test('a page of a line hears what is said of that line and of no other', () => {
  assert.equal(isOf('t', { topic: 'inbox.told', data: { owner: ME, line: 't' } }), true);
  assert.equal(isOf('t', { topic: 'inbox.read', data: { owner: ME, lines: ['a', 't'] } }), true);
  assert.equal(isOf('t', { topic: 'inbox.unread', data: { owner: ME, lines: ['t'] } }), true);
  assert.equal(isOf('t', { topic: 'inbox.away', data: { owner: ME, lines: ['a'] } }), false);
  assert.equal(isOf('t', { topic: 'inbox.told', data: { owner: ME, line: 'a' } }), false);
});

test('a line made to wait unread again waits, and the count is what is said', () => {
  const lines = [{ ...named('a'), read: 5 }, { ...named('b'), read: 6 }];
  assert.deepEqual(afterUnread(lines, ['a']).map((line) => line.read), [null, 6]);
  assert.equal(afterUnread(lines, ['c']), lines, 'nothing changed is the same list');
  const after = heard(lines, 0, ME, { topic: 'inbox.unread', data: { owner: ME, lines: ['b'], unread: 1 } }, 99);
  assert.deepEqual([after.lines.map((line) => line.read), after.unread, after.again], [[5, null], 1, false]);
});

test('what a told line is about is said by what it is called and where it is, and one that could not be read by its id', () => {
  const card = { id: 'shop-1', project: 'shop', collection: 'board', data: { title: 'Add a cart' } };
  assert.deepEqual(recordSaid('shop-1', card, 'Shop'), { says: 'Add a cart', where: 'card shop-1 on the Shop board', opens: true });
  const row = { id: 'shop-agents-2', project: 'shop', collection: 'agents', data: { name: 'worker', title: '  ' } };
  assert.deepEqual(recordSaid('shop-agents-2', row, null), { says: 'worker', where: 'shop-agents-2, a row of shop agents', opens: true });
  assert.equal(recordSaid('x-1', { id: 'x-1', project: 'x', collection: 'notes' }, 'X').says, 'x-1');
  assert.deepEqual(recordSaid('shop-9', null, 'Shop'), { says: 'shop-9', where: 'which is gone, or is not yours to read', opens: false });
});

const onCard: Line = {
  ...named('c'),
  kind: 'commented',
  about: '412',
  place: 'shop-4',
  said: undefined,
  on_record: { record: 'shop-4', project: 'shop', table: 'board', title: 'Fix the login', who: 'Mira', body: '@nell the login\n fails', seq: 412, created: 10 },
};

test('a line of something said on a record that names its owner says who said it, on what, and the words its history keeps', () => {
  assert.deepEqual([fromOf(onCard), whereOf(onCard), wordsOf(onCard)], ['Mira', 'on Fix the login', '@nell the login fails']);
  assert.equal(lineCalled(onCard), 'Unread: Mira on Fix the login: @nell the login fails');
  assert.equal(titleOf(onCard), '@nell the login fails');
  // A record that says nothing of what it is called is said by its id.
  const untitled = { ...onCard, on_record: { ...onCard.on_record!, title: undefined } };
  assert.equal(whereOf(untitled), 'on shop-4');
  // A page of a wiki is said by where it is now, its wiki first.
  const onPage = { ...onCard, on_record: { ...onCard.on_record!, record: '@-topics-3', project: '@', table: 'topics', title: 'Rolling back', path: ['Handbook', 'Deploying'] } };
  assert.equal(whereOf(onPage), 'on Handbook › Deploying › Rolling back');
  assert.equal(lineCalled(onPage), 'Unread: Mira on Handbook › Deploying › Rolling back: @nell the login fails');
});

test('a line of something said on a record opens the record at the comment, and opens no channel and no page of its own', () => {
  assert.deepEqual(opensRecord(onCard), { project: 'shop', table: 'board', record: 'shop-4', at: 412 });
  assert.equal(opens(onCard), null);
  assert.equal(opensItsPage(onCard), false);
  assert.equal(thingOf(onCard, ['shop-4']), null);
  // A line of a channel, and one handed with nothing of the record, open no record.
  assert.equal(opensRecord(named('d')), null);
  assert.equal(opensRecord({ ...onCard, on_record: undefined }), null);
});

/** A line of a page its owner follows, raised last by a revision of it. */
const followed: Line = {
  ...onCard,
  kind: 'followed',
  about: '@-topics-3',
  place: '@-topics-3',
  why: 'follows',
  said_by: '520',
  count: 3,
  on_record: { record: '@-topics-3', project: '@', table: 'topics', title: 'Rolling back', path: ['Handbook'], who: 'Mira', body: '', seq: 520, created: 10, topic: 'topic.revised', did: 'Revised the page' },
};

test('a line of a page its owner follows says who changed it, where it is, and what the change did, or a reply by its words', () => {
  assert.deepEqual([fromOf(followed), whereOf(followed), wordsOf(followed)], ['Mira', 'on Handbook › Rolling back', 'Revised the page']);
  assert.equal(lineCalled(followed), 'Unread: Mira on Handbook › Rolling back: Revised the page');
  const replied = { ...followed, on_record: { ...followed.on_record!, body: 'Which tag?', topic: 'record.commented', did: undefined } };
  assert.equal(wordsOf(replied), 'Which tag?');
});

test('a line of a page its owner follows opens the page, at the reply where a reply raised it last', () => {
  assert.deepEqual(opensRecord(followed), { project: '@', table: 'topics', record: '@-topics-3', at: null });
  const replied = { ...followed, on_record: { ...followed.on_record!, topic: 'record.commented', did: undefined } };
  assert.deepEqual(opensRecord(replied), { project: '@', table: 'topics', record: '@-topics-3', at: 520 });
  assert.equal(opens(followed), null);
  assert.equal(opensItsPage(followed), false);
  assert.equal(opensRecord({ ...followed, on_record: undefined }), null);
});

test('a told line says the start of its words where it is pointed at, cut where a word ends, and whether that is the whole', () => {
  const cases: [string, number, string][] = [
    ['short words', 40, 'short words'],
    ['exactly ten', 11, 'exactly ten'],
    ['one two three four', 12, 'one two…'],
    // Cut after a word, what ends a clause is not left hanging before the mark.
    ['one two, three four', 13, 'one two…'],
    ['the build is green: and more', 20, 'the build is green…'],
    // A word longer than half the room is cut where it falls: nothing else would be said.
    ['https://example.test/a/very/long/address and then', 20, 'https://example.test…'],
  ];
  for (const [words, most, says] of cases) assert.equal(cutAtWord(words, most), says, `${words} in ${most}`);
  const long = { ...told, says: Array.from({ length: 500 }, (_, at) => `check${at} passed`).join(', ') };
  const tip = tipOf(long);
  assert.equal(tip.whole, false);
  assert.ok(tip.says.length <= TIP_LETTERS + 1 && tip.says.endsWith('…'), tip.says);
  assert.ok(lineCalled(long).startsWith(tip.says.slice(0, -1)), 'the start of what it is called, and nothing else');
  assert.deepEqual(tipOf(told), { says: lineCalled(told), whole: true });
});

test("a told line's page is headed by its first sentence whole, to its first full stop or line break, cut at a word only past some two hundred letters", () => {
  const cases: [string, string][] = [
    ['shop-370: its agent could not do what it was asked, and it is left for a person. The rest of it.', 'shop-370: its agent could not do what it was asked, and it is left for a person.'],
    ['The build is green\nand more on the next line.', 'The build is green'],
    // The dot of an address or a version ends no sentence.
    ['Preview ready at http://127.0.0.1:7760/preview from v0.1.2 of it. Try it.', 'Preview ready at http://127.0.0.1:7760/preview from v0.1.2 of it.'],
    ['No stop at all', 'No stop at all'],
    ['  Spaces   about it.  ', 'Spaces about it.'],
    ['', ''],
  ];
  for (const [words, says] of cases) assert.equal(firstSentence(words), says, words);
  const long = Array.from({ length: 80 }, (_, at) => `word${at}`).join(' ');
  const cut = firstSentence(`${long}.`);
  assert.ok(cut.endsWith('…') && cut.length <= SENTENCE_LETTERS + 1 && long.startsWith(cut.slice(0, -1)) && long[cut.length - 1] === ' ', cut);
  // Of a line: its words as it was told, a block of code no words, and what it is where it says none.
  assert.equal(sentenceOf({ ...told, says: 'The build\nis green. And more.' }), 'The build');
  assert.equal(sentenceOf({ ...told, says: '```\ncargo test\n```\nIt passed. All of it.' }), 'It passed.');
  assert.equal(sentenceOf({ ...told, says: '' }), 'Inbox message');
});
