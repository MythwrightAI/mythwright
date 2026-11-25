import assert from 'node:assert/strict';
import { test } from 'node:test';
import { anyShown, channelItems, drawnFirst, everywhere, filesBounded, MOST_SAID, holds, keptItems, linesFound, memberItems, moreSaid, narrowed, nothingFound, ofChannels, ofKept, ofMembers, ofProject, ofTopics, projectItem, seekHint, shows, topicItems } from './panelsought.ts';
import type { Filed, Item, ProjectHas } from './panelsought.ts';
import type { Line } from './inbox.ts';

const shop: ProjectHas = {
  id: 'shop',
  name: 'Shop',
  board: true,
  tables: [{ name: 'open-questions', label: 'Open questions' }],
  libraries: [{ name: 'docs', label: 'Docs' }],
  mockups: [],
  blueprints: [{ name: 'fleet', description: 'Who works the cards' }],
  files: [
    { path: 'src', name: 'src', dir: true },
    { path: 'src/main.rs', name: 'main.rs', dir: false },
    { path: 'src/deep', name: 'deep', dir: true },
    { path: 'src/deep/pricing.md', name: 'pricing.md', dir: false },
    { path: 'README.md', name: 'README.md', dir: false },
  ],
};

const shown = (items: Item[], typed: string) => [...(narrowed(items, typed)?.shown ?? [])].sort();
const held = (items: Item[], typed: string) => [...(narrowed(items, typed)?.held ?? [])].sort();

test('nothing typed narrows nothing and holds nothing open', () => {
  for (const typed of ['', '   ', '\n']) {
    const found = narrowed([projectItem(shop)], typed);
    assert.equal(found, null);
    assert.ok(shows(found, ofProject.board('shop')));
    assert.ok(!holds(found, ofProject.project('shop')));
    assert.ok(anyShown(found));
  }
});

test('a thing found under a project is shown with what it is under held open, and nothing else of the project', () => {
  const items = [projectItem(shop)];
  assert.deepEqual(shown(items, 'pricing'), [ofProject.project('shop'), ofProject.files('shop'), ofProject.entry('shop', 'src'), ofProject.entry('shop', 'src/deep'), ofProject.entry('shop', 'src/deep/pricing.md')].sort());
  assert.deepEqual(held(items, 'pricing'), [ofProject.project('shop'), ofProject.files('shop'), ofProject.entry('shop', 'src'), ofProject.entry('shop', 'src/deep')].sort());
});

test('a thing that says the words itself is shown whole and holds nothing open', () => {
  const items = [projectItem(shop), projectItem({ ...shop, id: 'blog', name: 'Blog' })];
  const found = narrowed(items, 'SHOP')!;
  // Every line under it is shown, as it was left: what is folded stays folded.
  for (const id of [ofProject.board('shop'), ofProject.entry('shop', 'src/main.rs'), ofProject.blueprint('shop', 'fleet')]) assert.ok(shows(found, id), id);
  assert.equal(found.held.size, 0);
  assert.ok(!shows(found, ofProject.project('blog')));
  // A folder found by its name is shown with what is in it as it was left.
  const deep = narrowed(items, 'deep')!;
  assert.ok(shows(deep, ofProject.entry('shop', 'src/deep/pricing.md')));
  assert.ok(!holds(deep, ofProject.entry('shop', 'src/deep')));
  assert.ok(holds(deep, ofProject.entry('shop', 'src')));
});

test('every word typed is looked for, each in the thing or in what it is under that carries a name', () => {
  const items = [projectItem(shop), projectItem({ ...shop, id: 'blog', name: 'Blog' })];
  // `shop board` is Shop's board, and not Blog's.
  assert.deepEqual(shown(items, 'shop board'), [ofProject.project('shop'), ofProject.board('shop')].sort());
  // A folder carries its name to what is in it.
  assert.ok(shown(items, 'src main').includes(ofProject.entry('shop', 'src/main.rs')));
  // What is only a heading of a kind carries nothing: `repo` finds the Repo lines, and no file by it.
  assert.ok(!shown(items, 'repo readme').includes(ofProject.entry('shop', 'README.md')));
  const files = narrowed(items, 'repo')!;
  assert.ok(shows(files, ofProject.files('shop')) && shows(files, ofProject.files('blog')));
  assert.ok(!holds(files, ofProject.files('shop')));
  // A word nothing says finds nothing.
  assert.deepEqual(shown(items, 'shop nowhere'), []);
  assert.ok(!anyShown(narrowed(items, 'shop nowhere')));
});

test('a project\'s table is found by what it is called and by its name, and a blueprint it uses by what it says it is', () => {
  const items = [projectItem(shop)];
  assert.ok(shown(items, 'questions').includes(ofProject.table('shop', 'open-questions')));
  assert.ok(shown(items, 'open-questions').includes(ofProject.table('shop', 'open-questions')));
  assert.deepEqual(held(items, 'cards'), [ofProject.project('shop'), ofProject.blueprints('shop')].sort());
});

test('a project whose Repo is not listed has no files to be found, and one with none read has none', () => {
  assert.deepEqual(shown([projectItem({ ...shop, files: null })], 'pricing'), []);
  assert.deepEqual(shown([projectItem({ ...shop, files: [] })], 'pricing'), []);
  assert.ok(shown([projectItem({ ...shop, files: [] })], 'repo').includes(ofProject.files('shop')));
});

test('a project’s Pipelines line is found by its word where it has any, and is no line of one that has none', () => {
  assert.deepEqual(shown([projectItem({ ...shop, pipelines: true })], 'pipelines'), [ofProject.project('shop'), ofProject.pipelines('shop')].sort());
  assert.deepEqual(shown([projectItem(shop)], 'pipelines'), []);
});

test('what is kept to hand is found by what its tab is called and by whose it is', () => {
  const items = keptItems([{ key: 'board:shop', title: 'Board', whose: 'Shop' }], [{ key: 'file:blog:a.md', title: 'a.md', whose: 'Blog' }, { key: 'file:shop:a.md', title: 'a.md', whose: null }]);
  assert.deepEqual(shown(items, 'shop'), [ofKept.pinned('board:shop')]);
  assert.deepEqual(shown(items, 'a.md'), [ofKept.recent('file:blog:a.md'), ofKept.recent('file:shop:a.md')].sort());
  assert.deepEqual(shown(items, 'blog a.md'), [ofKept.recent('file:blog:a.md')]);
});

test('a channel is found by what it is called, a private one by its title, and a group or a project\'s branch carries its name', () => {
  const general = { id: 'ch_general', name: 'general' };
  const secret = { id: 'ch_9', name: 'ch_9', kept: true, title: 'Pay rises' };
  const ops = { id: 'ch_ops', name: 'ops' };
  const items = channelItems({
    pinned: [ops],
    loose: [general, secret],
    projects: [{ project: { id: 'shop', name: 'Shop' }, channels: [{ id: 'ch_shop', name: 'shop-work' }] }],
    groups: [{ group: { id: 'g1', name: 'Clients' }, channels: [ops] }],
  });
  assert.deepEqual(shown(items, 'general'), [ofChannels.channel(ofChannels.loose, 'ch_general')]);
  assert.deepEqual(shown(items, '#gen'), [ofChannels.channel(ofChannels.loose, 'ch_general')]);
  // A private channel is called its title, and its id is no name of it.
  assert.deepEqual(shown(items, 'pay'), [ofChannels.channel(ofChannels.loose, 'ch_9')]);
  assert.deepEqual(shown(items, 'ch_9'), []);
  // A channel pinned and in a group is found in both, each with what it is under held open.
  assert.deepEqual(shown(items, 'ops'), [ofChannels.pinned, ofChannels.channel(ofChannels.pinned, 'ch_ops'), ofChannels.group('g1'), ofChannels.channel(ofChannels.group('g1'), 'ch_ops')].sort());
  assert.deepEqual(held(items, 'ops'), [ofChannels.pinned, ofChannels.group('g1')].sort());
  // A group found by its name is shown whole and held open by nothing.
  assert.deepEqual(shown(items, 'clients'), [ofChannels.group('g1'), ofChannels.channel(ofChannels.group('g1'), 'ch_ops')].sort());
  assert.deepEqual(held(items, 'clients'), []);
  assert.deepEqual(shown(items, 'shop work'), [ofChannels.project('shop'), ofChannels.channel(ofChannels.project('shop'), 'ch_shop')].sort());
  // Nothing pinned is no line.
  assert.ok(!channelItems({ pinned: [], loose: [], projects: [], groups: [] }).length);
});

function line(id: string, over: Partial<Line> = {}, said: Partial<NonNullable<Line['said']>> = {}): Line {
  return {
    id,
    owner: 'user:acct_nell',
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

test('an inbox line is found by who said it, where and its words, every word', () => {
  const lines = [
    line('a'),
    line('b', {}, { author_name: 'Omar', channel_name: 'ops', body: 'the build is red' }),
    { ...line('t'), kind: 'told' as const, from: 'bot:conductor', why: 'told', says: 'The card waits for you', said: undefined },
  ];
  assert.deepEqual(linesFound(lines, '').map((one) => one.id), ['a', 'b', 't']);
  assert.deepEqual(linesFound(lines, 'mira').map((one) => one.id), ['a']);
  assert.deepEqual(linesFound(lines, '#ops').map((one) => one.id), ['b']);
  assert.deepEqual(linesFound(lines, 'omar red').map((one) => one.id), ['b']);
  assert.deepEqual(linesFound(lines, 'omar green').map((one) => one.id), []);
  assert.deepEqual(linesFound(lines, 'conductor waits').map((one) => one.id), ['t']);
});

test('where nothing is found the panel says so, in one line with what was typed', () => {
  assert.equal(nothingFound('  shop  '), 'Nothing listed here says “shop”.');
});

test('the files found under a project are drawn the first so many in the order they are drawn, with how many more there are', () => {
  const files: Filed[] = [
    { path: 'a', name: 'a', dir: true },
    ...[3, 1, 2].map((n) => ({ path: `a/item-${n}.md`, name: `item-${n}.md`, dir: false })),
    { path: 'b', name: 'b', dir: true },
    ...[1, 2].map((n) => ({ path: `b/item-${n}.md`, name: `item-${n}.md`, dir: false })),
    { path: 'item.bp', name: 'item.bp', dir: false },
    { path: 'item-0.md', name: 'item-0.md', dir: false },
    { path: 'other.md', name: 'other.md', dir: false },
  ];
  const has = { ...shop, files };
  const found = narrowed([projectItem(has)], 'item');
  const order = drawnFirst((path) => path.endsWith('.bp'));
  const kept = (most: number) => [...filesBounded('shop', files, found, most, order)!.kept].map((id) => id.replace('project:shop:entry:', '')).sort();
  // Nothing typed is nothing bounded.
  assert.equal(filesBounded('shop', files, null, 2, order), null);
  // Folders first, then a blueprint, then by name: a's three come before b's.
  assert.deepEqual(kept(2), ['a', 'a/item-1.md', 'a/item-2.md']);
  assert.equal(filesBounded('shop', files, found, 2, order)!.more, 5);
  assert.deepEqual(kept(4), ['a', 'a/item-1.md', 'a/item-2.md', 'a/item-3.md', 'b', 'b/item-1.md']);
  // Room for all of them is every one found, and nothing more.
  assert.deepEqual(kept(200), ['a', 'a/item-1.md', 'a/item-2.md', 'a/item-3.md', 'b', 'b/item-1.md', 'b/item-2.md', 'item-0.md', 'item.bp']);
  assert.equal(filesBounded('shop', files, found, 200, order)!.more, 0);
  assert.deepEqual(kept(6), ['a', 'a/item-1.md', 'a/item-2.md', 'a/item-3.md', 'b', 'b/item-1.md', 'b/item-2.md', 'item.bp']);
  // A folder found by its own name is drawn while there is room, as it is come to; held open only for what is under it, it is not drawn once nothing under it is.
  const byFolder = narrowed([projectItem(has)], 'b');
  assert.deepEqual([...filesBounded('shop', files, byFolder, 0, order)!.kept], []);
  assert.deepEqual([...filesBounded('shop', files, byFolder, 1, order)!.kept].sort(), [ofProject.entry('shop', 'b'), ofProject.entry('shop', 'b/item-1.md')]);
});

test('the line after the files drawn says how many more were found', () => {
  assert.equal(moreSaid(1), '1 more file found');
  assert.equal(moreSaid(412), '412 more files found');
});

test('the box says the keys that search everything while nothing is typed, where there are keys', () => {
  assert.equal(seekHint('', false, false), 'Ctrl K');
  assert.equal(seekHint('', false, true), '⌘ K');
  // Under a finger there are no keys to hold, and nothing is said of them.
  assert.equal(seekHint('', true, false), null);
  // Something typed, even a space, is what the box holds: the line under it is offered instead.
  assert.equal(seekHint('a', false, false), null);
  assert.equal(seekHint(' ', false, false), null);
});

test('what is typed is offered to be searched for everywhere, as it is meant, whole up to a sensible length', () => {
  assert.equal(everywhere(''), null);
  assert.equal(everywhere('   '), null);
  assert.deepEqual(everywhere(' shop board '), { says: 'Search everything for “shop board”', words: '“shop board”' });
  // Up to the most it says, all of it.
  const most = 'q'.repeat(MOST_SAID);
  assert.equal(everywhere(most)!.words, `“${most}”`);
  // Past that it is cut, by letters and not by the halves of one.
  assert.equal(everywhere(`${most}r`)!.words, `“${most}…”`);
  assert.equal(everywhere(`${'é'.repeat(MOST_SAID)}🍐`)!.words, `“${'é'.repeat(MOST_SAID)}…”`);
  // A cut that falls after a space does not keep the space.
  assert.equal(everywhere(`${'q'.repeat(MOST_SAID - 1)} rest`)!.words, `“${'q'.repeat(MOST_SAID - 1)}…”`);
});

test('the Members line under Factory is found by who is under it and by the words beside each, and holds itself open for them', () => {
  const lines = [
    { actor: 'user:u1', name: 'Ada', said: 'you' },
    { actor: 'user:u2', name: 'Nell', said: null },
    { actor: 'agent:scout', name: 'scout', said: 'agent (external)' },
    { actor: 'agent:helper', name: 'helper', said: 'agent' },
  ];
  const items = memberItems(lines);
  assert.deepEqual(shown(items, 'nell'), [ofMembers.part, ofMembers.member('user:u2')].sort());
  assert.deepEqual(held(items, 'nell'), [ofMembers.part]);
  // By the words beside a name: `external` is the one agent that says it, `agent` both.
  assert.deepEqual(shown(items, 'external'), [ofMembers.part, ofMembers.member('agent:scout')].sort());
  assert.deepEqual(shown(items, 'agent'), [ofMembers.part, ofMembers.member('agent:helper'), ofMembers.member('agent:scout')].sort());
  assert.deepEqual(shown(items, 'you'), [ofMembers.part, ofMembers.member('user:u1')].sort());
  // The part found by its name is shown whole, held open by nothing.
  assert.equal(shown(items, 'members').length, lines.length + 1);
  assert.deepEqual(held(items, 'members'), []);
  assert.deepEqual(shown(items, 'members ada'), [ofMembers.part, ofMembers.member('user:u1')].sort());
  // Nobody listed is no line; and the Resources panel's other things and members are found together without one taking the other's.
  assert.deepEqual(memberItems([]), []);
  const both = [projectItem({ id: 'nell', name: 'Nell', board: true, tables: [], libraries: [], mockups: [], blueprints: [], files: null }), ...items];
  assert.deepEqual(shown(both, 'nell'), [ofProject.project('nell'), ofProject.board('nell'), ofMembers.part, ofMembers.member('user:u2')].sort());
});

// ---- the Topics panel

const topics = {
  listed: [
    { id: '@-topics-1', title: 'Why the build is slow on Windows', status: 'open', summary: 'It is the linker', where: null },
    { id: '@-topics-2', title: 'Payment provider options', status: 'answered', summary: null, where: 'Shop' },
    { id: '@-topics-4', title: 'Handbook', status: null, summary: null, where: null },
  ],
  closed: [{ id: '@-topics-3', title: 'Old payment question', status: 'closed', summary: null, where: null }],
};

test('a topic is found by its title, the first line of its summary, its status and the project it is of', () => {
  const items = topicItems(topics);
  const found = (typed: string) => [...narrowed(items, typed)!.shown].sort();
  assert.deepEqual(found('slow windows'), [ofTopics.topic('@-topics-1')]);
  assert.deepEqual(found('linker'), [ofTopics.topic('@-topics-1')]);
  assert.deepEqual(found('answered'), [ofTopics.topic('@-topics-2')]);
  assert.deepEqual(found('open'), [ofTopics.topic('@-topics-1')], 'one with no status is found by none');
  assert.deepEqual(found('shop'), [ofTopics.topic('@-topics-2')]);
});

test('a closed topic is found under the branch of those that are closed, which is held open for it', () => {
  const found = narrowed(topicItems(topics), 'old payment')!;
  assert.deepEqual([...found.shown].sort(), [ofTopics.closed, ofTopics.topic('@-topics-3')].sort());
  assert.ok(holds(found, ofTopics.closed));
  // With none closed there is no branch.
  assert.deepEqual(topicItems({ listed: [], closed: [] }), []);
});

test("a page of a wiki is found by its own words and by the title of a page above it, which is held open for it", () => {
  const wikis = {
    listed: [
      {
        id: 'h', title: 'Handbook', status: 'open', summary: null,
        pages: [
          { id: 'g', title: 'Getting started', status: 'open', summary: null },
          { id: 'd', title: 'Deploying', status: 'open', summary: null, pages: [{ id: 'r', title: 'Rolling back', status: 'closed', summary: null }] },
        ],
      },
    ],
    closed: [],
  };
  const found = narrowed(topicItems(wikis), 'deploying rolling')!;
  assert.deepEqual([...found.shown].sort(), [ofTopics.topic('d'), ofTopics.topic('h'), ofTopics.topic('r')].sort(), 'Rolling back, by its own words and the page it is under');
  assert.ok(holds(found, ofTopics.topic('h')) && holds(found, ofTopics.topic('d')));
  assert.ok(!holds(found, ofTopics.topic('r')));
  // A page found by its own words shows what is under it as it was left.
  const whole = narrowed(topicItems(wikis), 'deploying')!;
  assert.deepEqual([...whole.shown].sort(), [ofTopics.topic('d'), ofTopics.topic('h'), ofTopics.topic('r')].sort());
  assert.ok(!holds(whole, ofTopics.topic('d')));
  // A closed page is found by how it stands where it is, in its wiki.
  assert.deepEqual([...narrowed(topicItems(wikis), 'closed')!.shown].sort(), [ofTopics.topic('d'), ofTopics.topic('h'), ofTopics.topic('r')].sort());
});

test('the Topics panel finds nothing by being called Topics, and with nothing typed is the whole of it', () => {
  const items = topicItems(topics);
  assert.equal(narrowed(items, 'topics')!.shown.size, 0);
  assert.equal(narrowed(items, '  '), null);
});
