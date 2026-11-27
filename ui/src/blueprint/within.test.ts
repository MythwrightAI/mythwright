import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { explain, suggest } from './complete.ts';
import { caret, told as language, real, world } from './fixture.ts';
import { pathTo, reference } from '../reference.ts';
import { heldAt, insideAt } from './within.ts';
import type { Inside } from './within.ts';

/** What is found where the `|` is. */
function found(marked: string): Inside | null {
  const { text, offset } = caret(marked);
  return insideAt(text, offset, language);
}

/** The same, said short: the argument and the keys down to the place, then the key there. */
function where(marked: string): string | null {
  const one = found(marked);
  return one && `${one.step}/${one.action}: ${[one.argument, ...one.path].join('.')} ${one.at} ${one.key}`;
}

const HEAD = 'blueprint: shop\nsteps:\n  board:\n    do: collection.ensure\n    with:\n      name: board\n';

// Each a text with `|` where the place is, what is found there said short,
// and what the action says of it: the key that is described, what the
// mapping is, the keys it takes and the words a value there may be.
const CASES: { name: string; text: string; where: string | null; described?: string | null; of?: string; keys?: string[]; words?: string[] }[] = [
  {
    name: 'a key of a field written in braces',
    text: `${HEAD}      fields:\n        moved: { type: list, quiet|: true, hidden: true }\n`,
    where: 'board/collection.ensure: fields.moved key quiet',
    described: 'quiet',
    of: 'a field of the table',
    keys: ['type', 'quiet', 'hidden', 'takes', 'unsaid'],
  },
  {
    name: 'one of a key\'s few words, written in braces',
    text: `${HEAD}      fields:\n        moved: { type: list|, quiet: true }\n`,
    where: 'board/collection.ensure: fields.moved value type',
    described: 'type',
    words: ['text', 'list', 'flag'],
  },
  {
    name: 'a key of a field written as a block',
    text: `${HEAD}      fields:\n        holds:\n          type: list\n          unsaid|\n`,
    where: 'board/collection.ensure: fields.holds key unsaid',
    described: 'unsaid',
    of: 'a field of the table',
  },
  {
    name: 'a value of a key written as a block',
    text: `${HEAD}      fields:\n        holds:\n          type: li|\n`,
    where: 'board/collection.ensure: fields.holds value type',
    words: ['text', 'list', 'flag'],
  },
  {
    name: 'a name of the writer\'s choosing, which is no key',
    text: `${HEAD}      fields:\n        moved|\n`,
    where: 'board/collection.ensure: fields key moved',
    described: null,
    of: 'a field of the table',
    keys: [],
  },
  {
    name: 'the whole of an argument written in braces, with the arguments in braces too',
    text: 'blueprint: shop\nsteps:\n  board: { do: collection.ensure, with: { name: board, fields: { title: { type: text, hid| } } } }\n',
    where: 'board/collection.ensure: fields.title key hid',
    described: null,
    keys: ['type', 'quiet', 'hidden', 'takes', 'unsaid'],
  },
  {
    name: 'an empty place in the braces of a field',
    text: `${HEAD}      fields:\n        title: { type: text, | }\n`,
    where: 'board/collection.ensure: fields.title key ',
    keys: ['type', 'quiet', 'hidden', 'takes', 'unsaid'],
  },
  {
    name: 'a key the action does not name',
    text: `${HEAD}      fields:\n        title: { type: text, colour|: red }\n`,
    where: 'board/collection.ensure: fields.title key colour',
    described: null,
    of: 'a field of the table',
  },
  {
    name: 'the value of a key the action does not name',
    text: `${HEAD}      fields:\n        title: { colour: re| }\n`,
    where: 'board/collection.ensure: fields.title value colour',
    described: null,
    words: [],
  },
  {
    name: 'inside what a key holds in turn, whose keys are names',
    text: `${HEAD}      fields:\n        outlives: { type: flag, takes: { on|: shell } }\n`,
    where: 'board/collection.ensure: fields.outlives.takes key on',
    described: null,
    of: 'a value of the field, with the scope giving it that value takes',
    keys: [],
  },
  {
    name: 'a thing by a name that is one of a few words',
    text: `${HEAD}      views:\n        rows: { kind: table, figures: { points: su| } }\n`,
    where: 'board/collection.ensure: views.rows.figures value points',
    of: 'a column, with the figure at its foot',
    words: ['sum', 'filled'],
  },
  {
    name: 'an argument the action says nothing within',
    text: `${HEAD}      history:\n        board.item.created|: "{actor} added it"\n`,
    where: 'board/collection.ensure: history key board.item.created',
    described: null,
    of: '',
    keys: [],
  },
  {
    name: 'keys an argument takes directly',
    text: 'blueprint: shop\nsteps:\n  serve:\n    do: service.start\n    with:\n      name: web\n      health: { http: /, inter| }\n',
    where: 'serve/service.start: health key inter',
    of: '`health`',
    keys: ['tcp', 'http', 'interval'],
  },
  {
    name: 'an item of a list on lines of its own',
    text: 'blueprint: shop\nsteps:\n  first:\n    do: rows.start\n    with:\n      collection: agents\n      rows:\n        - title: worker\n          count|: 2\n',
    where: 'first/rows.start: rows key count',
    described: 'count',
  },
  {
    name: 'an item of a list in brackets',
    text: 'blueprint: shop\nsteps:\n  first:\n    do: rows.start\n    with:\n      collection: agents\n      rows: [{ title: worker }, { title: reviewer, count|: 1 }]\n',
    where: 'first/rows.start: rows key count',
    described: 'count',
  },
  {
    name: 'an item in braces of a list on lines of its own',
    text: 'blueprint: shop\nsteps:\n  first:\n    do: rows.start\n    with:\n      collection: agents\n      rows:\n        - { title: worker, count|: 2 }\n',
    where: 'first/rows.start: rows key count',
    described: 'count',
  },
  {
    name: 'the arguments of something done around a step',
    text: 'blueprint: shop\nsteps:\n  a:\n    do: noop\n    on_fail:\n      do: service.start\n      with:\n        health:\n          tcp|: true\n',
    where: 'on_fail/service.start: health key tcp',
    described: 'tcp',
  },
  { name: 'an argument itself, which the language knows', text: `${HEAD}      fiel|\n`, where: null },
  { name: 'the value of an argument', text: `${HEAD}      store: lo|\n`, where: null },
  { name: 'a step, outside any arguments', text: 'blueprint: shop\nsteps:\n  board:\n    do: collection.ensure\n    whe|\n', where: null },
  { name: 'the arguments of a blueprint that is called', text: 'blueprint: shop\nsteps:\n  slot:\n    call: std/slot\n    with:\n      env:\n        PORT|: 1\n', where: null },
];

for (const one of CASES) {
  test(`inside a step's arguments: ${one.name}`, () => {
    assert.equal(where(one.text), one.where);
    const at = found(one.text);
    if (one.described !== undefined) assert.equal(at?.described?.name ?? null, one.described);
    if (one.of !== undefined) assert.equal(at?.of, one.of);
    if (one.keys !== undefined) assert.deepEqual(at?.keys.map((key) => key.name), one.keys);
    if (one.words !== undefined) assert.deepEqual(at?.words.map((word) => word.name), one.words);
  });
}

/** A blueprint that ships, as it stands. */
function shipped(name: string): string {
  return readFileSync(new URL(`../../../blueprints/std/${name}.bp`, import.meta.url), 'utf8');
}

/** Where the first `word` after `after` ends in a text. */
function endOf(text: string, after: string, word: string): number {
  const from = text.indexOf(after);
  assert.ok(from >= 0, `the text has \`${after}\``);
  const at = text.indexOf(word, from);
  assert.ok(at >= 0, `the text has \`${word}\` after \`${after}\``);
  return at + word.length;
}

/** What is found at the end of the first `word` after `after` in a text. */
function atWord(text: string, after: string, word: string): Inside | null {
  return insideAt(text, endOf(text, after, word), language);
}

test('in the board that ships, each word of a field in braces is found by its step, its argument and its keys', () => {
  const text = shipped('board');
  const line = 'moved: { type: list, quiet: true, hidden: true }';
  const quiet = atWord(text, line, 'quiet')!;
  assert.deepEqual([quiet.step, quiet.action, quiet.argument, quiet.path, quiet.at, quiet.key], ['board', 'collection.ensure', 'fields', ['moved'], 'key', 'quiet']);
  assert.equal(quiet.described?.help, 'a change to it alone is not news');
  const list = atWord(text, line, 'list')!;
  assert.deepEqual([list.argument, list.path, list.at, list.key], ['fields', ['moved'], 'value', 'type']);
  assert.ok(list.words.some((word) => word.name === 'list'));
  const view = atWord(text, 'order: { kind: order', 'kind')!;
  assert.deepEqual([view.argument, view.path, view.at, view.key], ['views', ['order'], 'key', 'kind']);
  // What it keeps as history is a map of the writer's own, of which nothing is said.
  const kept = atWord(text, 'history:\n', 'board.item.created')!;
  assert.deepEqual([kept.argument, kept.path, kept.key, kept.keys], ['history', [], 'board.item.created', []]);
});

test('in the fleet that ships, a field written as a block is found the same way, down to a list under one of its keys', () => {
  const text = shipped('fleet');
  const block = '        holds:\n          type: list\n';
  const type = atWord(text, block, 'type')!;
  assert.deepEqual([type.step, type.action, type.argument, type.path, type.at, type.key], ['agents', 'collection.ensure', 'fields', ['holds'], 'key', 'type']);
  const takes = atWord(text, block, 'each')!;
  assert.deepEqual([takes.path, takes.at, takes.key], [['holds'], 'value', 'takes']);
  const scope = atWord(text, block, '- projects')!;
  assert.deepEqual([scope.argument, scope.path, scope.at], ['fields', ['holds', 'unsaid'], 'value']);
  // And a field in braces of the same table, further down.
  const role = atWord(text, 'role: { type: text, suggests', 'suggests')!;
  assert.deepEqual([role.step, role.argument, role.path, role.key], ['agents', 'fields', ['role'], 'suggests']);
  // Its columns in groups are a list, each item a heading of the writer's
  // own: read up to its end the heading is still a word of the list.
  const group = atWord(text, 'groups:\n', 'Who they are')!;
  assert.deepEqual([group.argument, group.path, group.at], ['groups', [], 'value']);
});

test('what an action says is held at the end of a path of keys is found through names and keys, and nothing where it says nothing', () => {
  const action = language.actions.find((one) => one.name === 'collection.ensure');
  assert.deepEqual([heldAt(action, ['fields'])?.named, heldAt(action, ['fields'])?.of], [true, '`fields`']);
  assert.deepEqual([heldAt(action, ['fields', 'title'])?.named, heldAt(action, ['fields', 'title'])?.of], [false, 'a field of the table']);
  assert.equal(heldAt(action, ['fields', 'title', 'takes'])?.named, true);
  assert.equal(heldAt(action, ['fields', 'title', 'type']), null, 'a key that holds no mapping');
  assert.equal(heldAt(action, ['fields', 'title', 'colour']), null, 'a key that is not named');
  assert.equal(heldAt(action, ['history']), null, 'an argument that says nothing');
  assert.equal(heldAt(action, ['nothing']), null, 'no argument');
  assert.equal(heldAt(undefined, ['fields']), null, 'no action');
});

// ---- what the editor says and offers by it ------------------------------------

/** What pointing at the first `word` after `after` says, with the actions as a node hands them out. */
function pointed(text: string, after: string, word: string) {
  return explain(text, endOf(text, after, word) - 1, real, world);
}

test('pointing at a key of a field in the board that ships says what it means, and what it is a key of', () => {
  const text = shipped('board');
  const quiet = pointed(text, 'moved: { type: list, quiet: true, hidden: true }', 'quiet')!;
  assert.equal(quiet.title, 'quiet: bool');
  assert.equal(quiet.note.says, "Its changes are not news: no line of a record's history is written for a change to it alone.");
  assert.equal(quiet.note.where, 'In `fields` of `collection.ensure`: a field of the collection, by its name.');
  assert.equal(text.slice(quiet.from, quiet.to), 'quiet', 'the card is about the word and no more');
  const type = pointed(text, 'moved: { type: list', 'type')!;
  assert.equal(type.title, 'type: string');
  assert.deepEqual(type.note.lists?.[0]?.rows.map((row) => row.name), ['text', 'choice', 'actor', 'list', 'number', 'flag', 'date', 'record', 'records', 'document']);
});

test('pointing at one of a key\'s few words says that word\'s sentence', () => {
  const text = shipped('board');
  const list = pointed(text, 'moved: { type: list', 'list')!;
  assert.deepEqual([list.title, list.note.says], ['list', 'Several words.']);
  assert.equal(list.note.where, 'What `type` may be. In `fields` of `collection.ensure`: a field of the collection, by its name.');
  const order = pointed(text, 'order: { kind: order', 'kind: order')!;
  assert.equal(order.note.says, 'Its records by what comes first: what can be begun now, what waits only on those, and so on.');
  // A word that is none of them, and the value of a key that is one of no few words, are said nothing of.
  assert.equal(pointed(text, 'board: { kind: board, by: status', 'status'), null);
  assert.equal(pointed('blueprint: b\nsteps:\n  t:\n    do: collection.ensure\n    with:\n      fields:\n        a: { type: colour }\n', 'type: ', 'colour'), null);
});

test('a field written as a block is explained the same way, in the fleet that ships', () => {
  const text = shipped('fleet');
  const block = '        holds:\n          type: list\n';
  assert.equal(pointed(text, block, 'takes')?.title, 'takes: any');
  assert.equal(pointed(text, block, 'unsaid')?.note.says, 'With `takes: each`, the scopes such a list stands for where a record leaves it empty; of a choice, the one word of `of` nothing chosen stands for, which is where nothing ranks and what a number written before the field was a choice is counted from.');
  assert.equal(pointed(text, block, 'type')?.note.where, 'In `fields` of `collection.ensure`: a field of the collection, by its name.');
});

test('a key no action names, a name of the writer\'s own and what is inside an argument that says nothing are explained as nothing', () => {
  const head = 'blueprint: b\nsteps:\n  t:\n    do: collection.ensure\n    with:\n      name: t\n';
  const text = `${head}      fields:\n        title: { type: text, colour: red }\n      history:\n        record.commented: "{actor} said"\n`;
  assert.equal(pointed(text, 'fields:', 'colour'), null, 'a key nobody describes');
  assert.equal(pointed(text, 'fields:', 'red'), null, 'and its value');
  assert.equal(pointed(text, 'fields:', 'title'), null, 'the name of a field');
  assert.equal(pointed(text, 'history:\n', 'record.commented'), null, 'a map of the writer\'s own');
  // On a device whose actions say nothing of it, as one older than this client, nothing is said either.
  assert.equal(explain(text, endOf(text, 'fields:', 'type') - 1, { ...real, actions: language.actions.map(({ args, ...rest }) => ({ ...rest, args: args.map(({ within: _, ...arg }) => arg) })) }, world), null);
});

test('pointing at an argument that says what is inside it lists that under what the argument is', () => {
  const text = shipped('board');
  const fields = pointed(text, '      model: card\n', 'fields')!;
  assert.equal(fields.title, 'fields: object, needed');
  assert.equal(fields.note.lists?.[0]?.title, 'Each takes');
  assert.ok(fields.note.lists?.[0]?.rows.some((row) => row.name === 'quiet' && row.type === 'bool'));
});

/** What is offered where the `|` is, by label. */
function offered(marked: string): string[] {
  const { text, offset } = caret(marked);
  return (suggest(text, offset, real, world)?.options ?? []).map((one) => one.label);
}

const TABLE = 'blueprint: b\nsteps:\n  t:\n    do: collection.ensure\n    with:\n      name: t\n';

test('writing inside a field offers the keys it does not have yet, in braces and as a block', () => {
  const all = ['type', 'required', 'of', 'quiet', 'hidden', 'said', 'indexed', 'suggests', 'zoned', 'calls', 'writer_of', 'icon', 'takes', 'unsaid', 'stands'];
  assert.deepEqual(offered(`${TABLE}      fields:\n        title: { | }\n`), all);
  assert.deepEqual(offered(`${TABLE}      fields:\n        title: { type: text, quiet: true, | }\n`), all.filter((key) => key !== 'type' && key !== 'quiet'));
  assert.deepEqual(offered(`${TABLE}      fields:\n        title:\n          type: text\n          |\n`), all.filter((key) => key !== 'type'));
  assert.deepEqual(offered(`${TABLE}      fields:\n        title:\n          type: text\n          hi|\n`), all.filter((key) => key !== 'type'), 'what is typed narrows nothing here: the editor filters');
  // Where the names are the writer's own, nothing is offered: no field is called for it.
  assert.deepEqual(offered(`${TABLE}      fields:\n        |\n`), []);
  assert.deepEqual(offered(`${TABLE}      history:\n        |\n`), [], 'nor inside an argument that says nothing');
});

test('one that is offered says what it is, is put in with its colon, and is followed by its words where it is one of a few', () => {
  const { text, offset } = caret(`${TABLE}      fields:\n        title: { | }\n`);
  const found = suggest(text, offset, real, world)!;
  const type = found.options.find((one) => one.label === 'type')!;
  assert.deepEqual([type.insert, type.again, type.detail, type.kind], ['type: ', true, 'string', 'argument']);
  assert.equal(type.note?.lists?.[0]?.title, 'One of');
  const quiet = found.options.find((one) => one.label === 'quiet')!;
  assert.deepEqual([quiet.insert, quiet.again, quiet.detail], ['quiet: ', true, 'bool']);
  assert.equal(found.options.find((one) => one.label === 'icon')!.again, false);
  assert.equal(found.ready, true, 'inside braces it is offered unasked');
});

test('after a key that is one of a few words those are offered, and true and false after one that is either', () => {
  assert.deepEqual(offered(`${TABLE}      fields:\n        title: { type: | }\n`), ['text', 'choice', 'actor', 'list', 'number', 'flag', 'date', 'record', 'records', 'document']);
  assert.deepEqual(offered(`${TABLE}      fields:\n        title:\n          type: |\n`), ['text', 'choice', 'actor', 'list', 'number', 'flag', 'date', 'record', 'records', 'document']);
  assert.deepEqual(offered(`${TABLE}      fields:\n        title: { quiet: | }\n`), ['true', 'false']);
  assert.deepEqual(offered(`${TABLE}      views:\n        rows: { kind: | }\n`), ['table', 'board', 'list', 'gallery', 'calendar', 'order']);
  assert.deepEqual(offered(`${TABLE}      views:\n        rows: { kind: table, figures: { points: | } }\n`), ['filled', 'empty', 'sum', 'mean', 'least', 'greatest', 'ticked', 'different']);
  assert.deepEqual(offered(`${TABLE}      fields:\n        title: { icon: | }\n`), [], 'a key that is any word is offered none');
  assert.deepEqual(offered(`${TABLE}      fields:\n        title: { colour: | }\n`), [], 'nor one nobody describes');
});

test('the keys a health check takes are offered directly under it, and the forms a cell is given by', () => {
  const service = 'blueprint: b\nsteps:\n  web:\n    do: service.start\n    with:\n      name: web\n';
  assert.deepEqual(offered(`${service}      health: { http: /, | }\n`), ['tcp', 'cmd', 'interval', 'timeout', 'unhealthy_after']);
  const rows = 'blueprint: b\nsteps:\n  read:\n    do: rows.read\n    with:\n      collection: agents\n';
  assert.deepEqual(offered(`${rows}      fill:\n        role: { | }\n`), ['like', 'else', 'slug_of', 'by', 'otherwise', 'share', 'each', 'least']);
  assert.deepEqual(offered(`${rows}      saying:\n        does: { said: | }\n`), ['true', 'false']);
});

// ---- where the Reference says more of what is pointed at ---------------------------

const WHOLE = [
  'blueprint: shop',
  'steps:',
  '  build:',
  '    do: shell',
  '    with:',
  '      run: make',
  '    timeout: 5m',
  '    when: ${ len(params.version) > 0 }',
  '  board:',
  '    do: collection.ensure',
  '    with:',
  '      name: board',
  '      fields:',
  '        moved: { type: list, quiet: true }',
  '      views:',
  '        rows: { kind: table, figures: { points: sum } }',
  '  slot:',
  '    call: std/slot',
  'on:',
  '  moved:',
  '    event: board.item.moved',
  '    scope: fleet',
  '    do: noop',
  '',
].join('\n');

// Each: a line of the text and a word in it, and the entry of the Reference the card about it leads to.
const ENTRIES: [string, string, string | undefined][] = [
  ['    timeout: 5m', 'timeout', 'word/step/timeout'],
  ['    do: shell', 'shell', 'action/shell'],
  ['      run: make', 'run', 'action/shell/run'],
  ['      fields:', 'fields', 'action/collection.ensure/fields'],
  ['moved: { type: list, quiet: true }', 'quiet', 'action/collection.ensure/fields/quiet'],
  ['moved: { type: list, quiet: true }', 'type', 'action/collection.ensure/fields/type'],
  // One of a key's few words leads to the key, where the words are said.
  ['moved: { type: list, quiet: true }', 'list', 'action/collection.ensure/fields/type'],
  ['figures: { points: sum }', 'figures', 'action/collection.ensure/views/figures'],
  ['figures: { points: sum }', 'sum', 'action/collection.ensure/views/figures'],
  ['    event: board.item.moved', 'board.item.moved', 'event/board.item.moved'],
  ['    scope: fleet', 'scope', 'word/handler/scope'],
  ['    scope: fleet', 'fleet', 'word/handler/scope'],
  ['    when: ${ len(params.version) > 0 }', 'len', 'function/len'],
  ['    when: ${ len(params.version) > 0 }', 'params', 'name/params'],
  // What has no entry leads nowhere: a blueprint that is called is a page of its own.
  ['    call: std/slot', 'std/slot', undefined],
];

test('what is said of a word that is pointed at names its entry in the Reference, and each such entry is one there is', () => {
  const parts = reference(real);
  for (const [line, word, entry] of ENTRIES) {
    const found = explain(WHOLE, endOf(WHOLE, line, word) - 1, real, world);
    assert.ok(found, `something is said of \`${word}\` in \`${line.trim()}\``);
    assert.equal(found.entry, entry, `\`${word}\` in \`${line.trim()}\``);
    if (entry) assert.equal(pathTo(parts, entry)?.at(-1)?.id, entry, `the Reference has \`${entry}\``);
  }
});
