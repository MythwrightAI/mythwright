import assert from 'node:assert/strict';
import { test } from 'node:test';
import { real } from './blueprint/fixture.ts';
import type { Scenario, ToldLanguage, Used } from './blueprint/told.ts';
import { actionId, asked, count, eventId, every, found, functionId, matches, nameId, partOf, pathTo, reference, wordId } from './reference.ts';
import type { Entry, Part } from './reference.ts';
import { addressOf, tabAt } from './tab.ts';

test('an entry is named by what it is and the names down to it, and its name says which part it is of', () => {
  const cases: [string, string, string | null][] = [
    [wordId('step'), 'word/step', 'words'],
    [wordId('step', 'on_fail'), 'word/step/on_fail', 'words'],
    [nameId('event'), 'name/event', 'words'],
    [functionId('slug'), 'function/slug', 'words'],
    [actionId('collection.ensure'), 'action/collection.ensure', 'actions'],
    [actionId('collection.ensure', 'fields'), 'action/collection.ensure/fields', 'actions'],
    [actionId('collection.ensure', 'fields', 'quiet'), 'action/collection.ensure/fields/quiet', 'actions'],
    [actionId('collection.ensure', 'views', 'figures'), 'action/collection.ensure/views/figures', 'actions'],
    [eventId('board.item.moved'), 'event/board.item.moved', 'events'],
    ['nothing/at/all', 'nothing/at/all', null],
    ['', '', null],
  ];
  for (const [made, written, part] of cases) {
    assert.equal(made, written);
    assert.equal(partOf(made), part, made);
  }
});

test('an entry\'s name is what its address ends in, and the address opens the page at it', () => {
  for (const entry of [actionId('collection.ensure', 'fields', 'quiet'), wordId('step', 'on_fail'), eventId('board.item.moved')]) {
    const address = addressOf({ kind: 'reference', at: entry })!;
    assert.ok(address.startsWith('/reference/'), address);
    assert.deepEqual(tabAt(address), { tab: { kind: 'reference', at: entry }, shown: {} });
  }
});

// ---- the model, made from what the daemon describes -------------------------------


/**
 * The language as a node hands it: its words from the copy the client's
 * tests are held to, the actions that say what is inside their arguments
 * as a node hands them out, a few made up in the shape of the rest, and
 * events with who hears each. One action says everything an action can
 * say of itself, and one event everything an event can.
 */
const language = {
  ...real,
  actions: real.actions.map((action) =>
    action.name === 'collection.ensure' ? { ...action, makes: { what: 'a table', named: 'name', setting: 'beside' } } : action.name === 'shell' ? { ...action, runs: ['run'], reaches: ['std/rule'] } : action,
  ),
  events: (real.events ?? []).map((event) => ({ ...event, hears: event.topic.startsWith('board.') ? 'board:read' : '', open: event.topic === 'log.line' })),
} as ToldLanguage;

const parts = reference(language);
const part = (name: string): Part => parts.find((one) => one.name === name)!;
const at = (id: string): Entry => {
  const path = pathTo(parts, id);
  assert.ok(path && path.at(-1)!.id === id, `there is an entry \`${id}\``);
  return path.at(-1)!;
};
const marks = (one: Entry) => one.marks.map((mark) => (mark.says === undefined ? mark.what : `${mark.what}: ${mark.says}`));

/** Every word the daemon handed, with where it was: what is said by a mark of another kind is left out. */
function handed(value: unknown, path: string, into: [string, string][]): void {
  // A kind is said in the words beside it (`after`), a type an argument can have as itself; which words
  // stand in for each other are said joined; and a mapping a field names is one of the mappings there are.
  if (/\.(kind|short|alternatives)$/.test(path) || /\.value\.of$/.test(path)) {
    if (!Array.isArray(value)) return;
  }
  if (typeof value === 'string') {
    if (value !== '') into.push([value, path]);
  } else if (Array.isArray(value)) {
    if (/\.alternatives$/.test(path)) return;
    value.forEach((one, index) => handed(one, `${path}[${index}]`, into));
  } else if (value && typeof value === 'object') {
    for (const [key, one] of Object.entries(value)) handed(one, `${path}.${key}`, into);
  }
}

/** Every word the model holds. */
function held(value: unknown, into: Set<string>): void {
  if (typeof value === 'string') into.add(value);
  else if (Array.isArray(value)) value.forEach((one) => held(one, into));
  else if (value && typeof value === 'object') Object.values(value).forEach((one) => held(one, into));
}

test('everything the daemon describes is on the model: every name and every sentence it hands, wherever it is', () => {
  const all: [string, string][] = [];
  handed(language, 'language', all);
  const model = new Set<string>();
  held(parts, model);
  const missing = all.filter(([word]) => !model.has(word));
  assert.deepEqual(missing, [], 'each a word the daemon handed that no entry, mark or row says');
  // And there was something of each kind to hold it to.
  for (const path of ['.shapes[', '.names[', '.functions[', '.step[', '.decisions[', '.actions[', '.within.', '.words[', '.outputs[', '.makes.', '.runs[', '.reaches[', '.capability', '.events[', '.fields[', '.hears']) {
    assert.ok(all.some(([, where]) => where.includes(path)), `the language that was handed has something at \`${path}\``);
  }
});

test('what is so or not of a thing is said by a mark, and only where it is so', () => {
  const table = at('action/collection.ensure');
  assert.deepEqual(marks(table), ['needs: projects:write', 'makes: a table', 'named: name', 'setting: beside', 'again']);
  assert.deepEqual(marks(at('action/shell')), ['needs: shell', 'runs: run', 'reaches: std/rule']);
  assert.deepEqual(marks(at('action/emit')), ['again', 'open']);
  assert.deepEqual(marks(at('action/collection.ensure/name')), ['needed']);
  assert.deepEqual(marks(at('action/collection.ensure/views')), ['each: a way of looking at the collection, by its name']);
  assert.deepEqual(marks(at('action/collection.ensure/fields')), ['needed', 'each: a field of the collection, by its name']);
  assert.deepEqual(marks(at('event/board.item.moved')), ['hears: board:read']);
  assert.deepEqual(marks(at('event/log.line')), ['hears', 'passing', 'open'], 'told to anybody signed in, kept nowhere, carrying more than is listed');
  assert.deepEqual(marks(at('word/step')), ['either: do, call, spawn, task']);
  const moved = at('event/board.item.moved').lists[0]!;
  assert.equal(moved.what, 'carries');
  assert.deepEqual(moved.rows.map((row) => [row.name, row.is, row.sometimes ?? false]), [['id', 'string', false], ['to', 'string', false], ['note', 'string', true]]);
});

test('an action is said with its arguments under it, and what is written inside one to every level', () => {
  const table = at('action/collection.ensure');
  assert.equal(table.says, language.actions.find((one) => one.name === 'collection.ensure')!.summary);
  assert.ok(table.under.every((one) => one.kind === 'argument'));
  const quiet = at('action/collection.ensure/fields/quiet');
  assert.deepEqual([quiet.kind, quiet.name, quiet.is, quiet.says], ['key', 'quiet', 'bool', "its changes are not news: no line of a record's history is written for a change to it alone"]);
  const type = at('action/collection.ensure/fields/type');
  assert.deepEqual(type.lists.map((list) => [list.what, list.rows.map((row) => row.name)]), [['one-of', ['text', 'choice', 'actor', 'list', 'number', 'flag', 'date', 'record', 'records', 'document']]]);
  // A key that holds things by a name in turn: what each is, and the few words each may be.
  const figures = at('action/collection.ensure/views/figures');
  assert.deepEqual(marks(figures), ['each: a column, with the figure at its foot']);
  assert.deepEqual(figures.lists[0]!.rows.map((row) => row.name), ['filled', 'empty', 'sum', 'mean', 'least', 'greatest', 'ticked', 'different']);
  assert.deepEqual(at('action/service.start/health').under.map((one) => one.name), ['tcp', 'http', 'cmd', 'interval', 'timeout', 'unhealthy_after']);
  assert.deepEqual(at('action/shell').lists.map((list) => [list.what, list.rows.map((row) => row.name)]), [['gives', ['code', 'stdout']]]);
});

test('a word of the language is said with what goes after it, the few words it may be, and what follows a name', () => {
  const timeout = at('word/step/timeout');
  const said = language.shapes.find((shape) => shape.name === 'step')!.fields.find((field) => field.name === 'timeout')!.help;
  assert.deepEqual([timeout.kind, timeout.is, timeout.says], ['word', 'a length of time, such as 30s or 5m', said]);
  assert.equal(at('word/blueprint/steps').is, 'names, each a step');
  const scope = at('word/handler/scope');
  assert.deepEqual(scope.lists.map((list) => list.what), ['one-of']);
  assert.ok(scope.lists[0]!.rows.some((row) => row.name === 'fleet' && row.says.length > 10));
  // What a step that failed is answered with: the words the daemon lists for it.
  const decides = every(part('words').entries).filter((one) => one.lists.some((list) => list.rows.some((row) => row.name === 'retry')));
  assert.ok(decides.length > 0 && decides.every((one) => one.is === 'an action, or a decision'), decides.map((one) => one.id).join());
  assert.ok(at('name/event').lists[0]!.rows.some((row) => row.name === 'data'));
  assert.deepEqual(at('name/steps').lists[0]!.rows.map((row) => row.name).slice(0, 3), ['ok', 'failed', 'skipped']);
  assert.equal(at('function/slug').says, language.functions.find((one) => one.name === 'slug')!.usage);
});

test('an entry is shown by the example the daemon hands with it, and by where the blueprints there are use it', () => {
  const used: Used = {
    words: { 'step/retry': { places: [{ blueprint: 'std/rolling-step', line: 25, at: 'roll' }], more: 0 }, step: { places: [{ blueprint: 'std/run', line: 9, at: 'run' }], more: 40 } },
    names: { event: { places: [{ blueprint: 'std/watch', line: 31, at: 'on.said' }], more: 2 } },
    actions: { emit: { places: [{ blueprint: 'std/pool', line: 80, at: 'on.asked/said' }], more: 0 } },
  };
  const shown = reference(language, used);
  const entryOf = (id: string) => pathTo(shown, id)!.at(-1)!;
  const retry = language.shapes.find((shape) => shape.name === 'step')!.fields.find((field) => field.name === 'retry')!;
  assert.ok(retry.example?.startsWith('blueprint: '), 'the daemon hands a word with a whole blueprint');
  assert.equal(entryOf('word/step/retry').example, retry.example);
  assert.deepEqual(entryOf('word/step/retry').used, used.words['step/retry']);
  assert.deepEqual(entryOf('word/step').used, used.words.step);
  assert.ok(entryOf('word/step').example);
  assert.deepEqual(entryOf('name/event').used, used.names.event);
  assert.ok(entryOf('name/event').example?.includes('event.'));
  assert.ok(entryOf('function/slug').example?.includes('slug('));
  assert.deepEqual(entryOf('action/emit').used, used.actions.emit);
  // What nothing uses says nothing of it, and with nothing read nothing is said of any.
  assert.equal(entryOf('word/step/timeout').used, undefined);
  assert.equal(at('word/step/retry').used, undefined);
  assert.equal(at('word/step/retry').example, retry.example);
});

test('an example the daemon has a scenario of is offered to be run wherever it is shown, and no other is', () => {
  const retry = language.shapes.find((shape) => shape.name === 'step')!.fields.find((field) => field.name === 'retry')!;
  const scenario: Scenario = { name: 'retry', example: retry.example!, answers: 'asked', acts: [{ apply: {} }], notice: 'It was tried again.' };
  const shown = reference(language, null, [scenario]);
  const all = shown.flatMap((one) => every(one.entries));
  const offered = all.filter((one) => one.scenario);
  // Every entry shown by that example, at any level, and none shown by another.
  assert.ok(offered.some((one) => one.id === 'word/step/retry'), 'the word itself');
  assert.ok(offered.length > 1, 'and every other entry whose example it is');
  for (const one of all) assert.equal(one.scenario === scenario, one.example === retry.example, one.id);
  assert.equal(at('word/step/retry').scenario, undefined, 'with none handed, none is offered');
});

test('every entry is named once, by a name of its own part, and is found by it', () => {
  const seen = new Set<string>();
  for (const one of parts) {
    for (const each of every(one.entries)) {
      assert.ok(!seen.has(each.id), `\`${each.id}\` names one entry`);
      seen.add(each.id);
      assert.equal(partOf(each.id), one.name, each.id);
      assert.equal(pathTo(parts, each.id)?.at(-1), each);
    }
  }
  assert.ok(seen.size > 200, `${seen.size} entries`);
});

test('a part counts the things it lists, and not what is under them', () => {
  const words = language.shapes.reduce((sum, shape) => sum + shape.fields.length, 0) + language.names.length + language.functions.length;
  assert.deepEqual(parts.map((one) => [one.name, count(one)]), [['words', words], ['actions', language.actions.length], ['events', language.events!.length]]);
  assert.deepEqual(found(parts, '   ').counts, { words, actions: language.actions.length, events: language.events!.length });
  assert.deepEqual(found(parts, '').open, []);
  assert.equal(found(parts, '').parts, parts, 'nothing typed is everything, as it was');
  // The actions and the events are listed by name, whatever order the daemon handed them in.
  for (const name of ['actions', 'events']) {
    const names = part(name).entries.map((one) => one.name);
    assert.deepEqual(names, [...names].sort((a, b) => a.localeCompare(b)), name);
  }
});

test('the way to an entry is the entries above it, and for a name nothing has, the nearest above', () => {
  assert.deepEqual(pathTo(parts, 'action/collection.ensure/fields/quiet')!.map((one) => one.name), ['collection.ensure', 'fields', 'quiet']);
  assert.deepEqual(pathTo(parts, 'word/step/on_fail')!.map((one) => one.name), ['step', 'on_fail']);
  assert.deepEqual(pathTo(parts, 'action/collection.ensure/fields/colour')!.map((one) => one.name), ['collection.ensure', 'fields'], 'a key the daemon does not say is found as far as its argument');
  assert.deepEqual(pathTo(parts, 'action/collection.ensure/nothing/at/all')!.map((one) => one.name), ['collection.ensure']);
  for (const nothing of ['action/collection', 'action/no.such', 'event/', 'word', '', 'nothing/at/all']) assert.equal(pathTo(parts, nothing), null, nothing);
});

// Each: what is typed, how many entries of each part say it, and some that must and must not be shown.
const SEARCHES: { typed: string; counts?: [number, number, number]; shown?: string[]; hidden?: string[]; open?: string[]; folded?: string[] }[] = [
  {
    typed: 'quiet',
    shown: ['action/collection.ensure', 'action/collection.ensure/fields', 'action/collection.ensure/fields/quiet'],
    hidden: ['action/collection.ensure/fields/hidden', 'action/collection.ensure/views', 'action/shell'],
    // What is kept for what is under it is shown unfolded, so that what was found is seen.
    open: ['action/collection.ensure', 'action/collection.ensure/fields'],
  },
  { typed: 'QUIET', shown: ['action/collection.ensure/fields/quiet'] },
  // In a sentence, and not only in a name.
  { typed: 'not news', shown: ['action/collection.ensure/fields/quiet'], hidden: ['action/collection.ensure/fields/type'] },
  // Every word typed, wherever each is in what the entry says.
  { typed: 'news quiet', shown: ['action/collection.ensure/fields/quiet'] },
  { typed: 'quiet zebra', counts: [0, 0, 0] },
  // An entry that matches is shown with everything under it.
  { typed: 'collection.ensure', shown: ['action/collection.ensure', 'action/collection.ensure/fields/quiet', 'action/collection.ensure/views/figures'], hidden: ['action/shell'], folded: ['action/collection.ensure'] },
  // One of the few words a key may be finds the key, and a thing an event carries finds the event.
  { typed: 'greatest', shown: ['action/collection.ensure/views/figures'], hidden: ['action/collection.ensure/views/kind'] },
  // Said only in what is listed inside it, an entry is unfolded for it; said at its head, it is as it was.
  { typed: 'said as it was moved', counts: [0, 0, 1], shown: ['event/board.item.moved'], hidden: ['event/board.item.removed'], open: ['event/board.item.moved'] },
  { typed: 'moved to another column', folded: ['event/board.item.moved'] },
  { typed: 'board:read', counts: [0, 0, 2] },
  { typed: 'on_fail', shown: ['word/step', 'word/step/on_fail'], hidden: ['word/step/timeout', 'word/handler'] },
  { typed: 'length of time', shown: ['word/step/timeout'] },
  // A word written only in an example finds what it is the example of.
  { typed: 'max_backoff: 30s', shown: ['word/step/retry', 'word/retry/max'], hidden: ['word/handler/turn'] },
];

for (const one of SEARCHES) {
  test(`what is typed finds what says it: ${one.typed}`, () => {
    const result = found(parts, one.typed);
    const shown = new Set(result.parts.flatMap((each) => every(each.entries).map((entry) => entry.id)));
    for (const id of one.shown ?? []) assert.ok(shown.has(id), `\`${id}\` is shown`);
    for (const id of one.hidden ?? []) assert.ok(!shown.has(id), `\`${id}\` is not shown`);
    if (one.counts) assert.deepEqual([result.counts.words, result.counts.actions, result.counts.events], one.counts);
    for (const id of one.open ?? []) assert.ok(result.open.includes(id), `\`${id}\` is unfolded for what was found`);
    for (const id of one.folded ?? []) assert.ok(shown.has(id) && !result.open.includes(id), `\`${id}\` is found and is as it was`);
    // What is counted is what says it; what is shown beside is above or under something that does.
    const query = asked(one.typed);
    const saying = parts.flatMap((each) => every(each.entries)).filter((entry) => matches(entry, query));
    assert.equal(result.counts.words + result.counts.actions + result.counts.events, saying.length);
    for (const entry of saying) assert.ok(shown.has(entry.id), `\`${entry.id}\` says it and is shown`);
  });
}
