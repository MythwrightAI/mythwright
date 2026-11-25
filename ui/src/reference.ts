// The Reference: everything a blueprint can say, as the daemon describes
// it, laid out for a page.
//
// Nothing here is written about the language. The words, the actions with
// what each takes down to what is written inside an argument, and the
// events with what each carries are the daemon's descriptions (what
// `/api/blueprints/language` hands an editor), and this only puts them in
// the shape a page draws: three parts, each a list of entries, an entry
// holding the entries under it. The page draws from this and from nothing
// else, and a test holds that everything the daemon hands is in it: so
// something new the daemon comes to describe is on the page with nothing
// done here, or that test fails.
//
// It holds nothing of the page, so that it can be tested without one.

import type { BlueprintLanguage, Choice } from './api.ts';
import type { Scenario, ToldAction, ToldEvent, ToldLanguage, Used, Uses } from './blueprint/told.ts';
import type { Within, WithinKey, WithinWord } from './blueprint/within.ts';

/** The three parts, in the order they are drawn. */
export const PARTS = ['words', 'actions', 'events'] as const;
export type PartName = (typeof PARTS)[number];

/**
 * What an entry is. A `shape` is one of the mappings the language has (a
 * step, a handler), with its `word`s under it; a `name` is one an
 * expression may start with and a `function` one it may call; an `action`
 * has its `argument`s under it, and an argument or a `key` the keys that
 * are written inside it; an `event` stands alone.
 */
export type EntryKind = 'shape' | 'word' | 'name' | 'function' | 'action' | 'argument' | 'key' | 'event';

/** Something said inside an entry that is no entry of its own: a word it may be, something it gives out or carries. */
export interface Row {
  name: string;
  /** What kind of thing it is, where that is said. */
  is?: string;
  says: string;
  /** Carried only some of the time. */
  sometimes?: true;
}

/**
 * Rows of one kind under an entry: the few words it is `one-of`, what
 * `follows` a name in an expression, what an action `gives` out, what an
 * event `carries`.
 */
export interface List {
  what: 'one-of' | 'follows' | 'gives' | 'carries';
  rows: Row[];
}

/**
 * One more thing an entry says of itself, by what it is: that it is
 * `needed`; words of a mapping that stand in for each other, of which it
 * has one and never two (`either`); the scope an action `needs` of
 * whoever applies; what it
 * `makes`, and the argument that names it (`named`), and the one that,
 * said, has what it makes set what something does (`setting`); an argument that
 * `runs` as a command; a blueprint an action `reaches` by itself, which
 * no text that asks it writes; that it may be done `again` after an interruption;
 * that it takes or carries more than is listed (`open`); what `each`
 * thing written inside it by a name of its writer's choosing is; who
 * `hears` an event (the scope, or nothing for anybody signed in); that an
 * event is only `passing`, kept nowhere, so that no handler hears it.
 */
export interface Mark {
  what: 'needed' | 'either' | 'needs' | 'makes' | 'named' | 'setting' | 'runs' | 'reaches' | 'again' | 'open' | 'each' | 'hears' | 'passing';
  /** What it says, where it says more than that it is so. */
  says?: string;
}

export interface Entry {
  /** What it is known by in an address, and on the page: `action/collection.ensure/fields/quiet`. */
  id: string;
  kind: EntryKind;
  /** What it is written as. */
  name: string;
  /** What kind of thing it is, or what goes after it, in the daemon's words: `string`, `a length of time, such as 30s or 5m`. */
  is: string;
  /** Its sentence. */
  says: string;
  marks: Mark[];
  lists: List[];
  /** The entries inside it: a shape's words, an action's arguments, the keys written inside an argument or a key. */
  under: Entry[];
  /** A whole blueprint it is shown by, as the daemon says it. */
  example?: string;
  /** Where the blueprints there are use it, the shortest first: worked out by the daemon, never here. */
  used?: Uses;
  /** Its example shown working, where the daemon has a scenario of that example. */
  scenario?: Scenario;
}

export interface Part {
  name: PartName;
  entries: Entry[];
}

// ---- how an entry is named -----------------------------------------------------
//
// An entry's id is what goes after `/reference/` in the address that opens
// the page at it, its parts joined by `/`:
//
//   word/<shape>              one of the language's mappings: `word/step`
//   word/<shape>/<field>      a word of it: `word/step/on_fail`
//   name/<name>               a name an expression may start with: `name/event`
//   function/<name>           a function an expression may call
//   action/<action>           an action: `action/collection.ensure`
//   action/<action>/<argument>[/<key>...]
//                             an argument of it, and a key written inside
//                             that, by the keys from the argument down:
//                             `action/collection.ensure/fields/quiet`. A
//                             name of the writer's own (the field that is
//                             quiet) is no part of it: where a mapping
//                             holds things by name, its keys are theirs.
//   event/<topic>             an event: `event/board.item.moved`

const id = (...parts: string[]) => parts.join('/');

/** The id of a mapping of the language, or of one of its words. */
export const wordId = (shape: string, field?: string) => (field === undefined ? id('word', shape) : id('word', shape, field));
/** The id of a name an expression may start with. */
export const nameId = (name: string) => id('name', name);
/** The id of a function an expression may call. */
export const functionId = (name: string) => id('function', name);
/** The id of an action, of one of its arguments, or of a key written inside one, by the keys from the argument down. */
export const actionId = (action: string, ...keys: string[]) => id('action', action, ...keys);
/** The id of an event. */
export const eventId = (topic: string) => id('event', topic);

/** Which part an id is of, by how it begins; nothing for one that names nothing there could be. */
export function partOf(entry: string): PartName | null {
  const kind = entry.split('/')[0];
  if (kind === 'word' || kind === 'name' || kind === 'function') return 'words';
  if (kind === 'action') return 'actions';
  return kind === 'event' ? 'events' : null;
}

// ---- the model --------------------------------------------------------------------


const entry = (id: string, kind: EntryKind, name: string, is: string, says: string): Entry => ({ id, kind, name, is, says, marks: [], lists: [], under: [] });
/** An entry with its example and where it is used, where there is either. */
const shown = (one: Entry, example: string | null | undefined, used: Uses | undefined): Entry =>
  Object.assign(one, example ? { example } : {}, used && used.places.length > 0 ? { used } : {});
const chosen = (words: (Choice | WithinWord)[]): Row[] => words.map((word) => ({ name: word.name, says: word.help }));

/** The words of the language: each mapping with its words, then what an expression may start with and call. */
function words(language: ToldLanguage, used: Used | null): Entry[] {
  const shapes = language.shapes.map((shape) => {
    // What it is for, where the daemon says that; what it is called, from one that does not.
    const one = shown(entry(wordId(shape.name), 'shape', shape.name, shape.help ? shape.what : '', shape.help ?? shape.what), shape.example, used?.words[shape.name]);
    // Words that stand in for each other: a mapping has one of each group and never two.
    for (const group of shape.alternatives) one.marks.push({ what: 'either', says: group.join(', ') });
    one.under = shape.fields.map((field) => {
      const word = shown(entry(wordId(shape.name, field.name), 'word', field.name, field.after ?? '', field.help), field.example, used?.words[`${shape.name}/${field.name}`]);
      if (field.value.kind === 'one_of') word.lists.push({ what: 'one-of', rows: chosen(field.value.of) });
      // What a step that failed is answered with is one of a few words the daemon lists, or an action.
      if (field.value.kind === 'decision' && language.decisions.length > 0) {
        word.lists.push({ what: 'one-of', rows: language.decisions.map((name) => ({ name, says: '' })) });
      }
      return word;
    });
    return one;
  });
  const names = language.names.map((name) => {
    const one = shown(entry(nameId(name.name), 'name', name.name, '', name.meaning), name.example, used?.names[name.name]);
    // What follows a step's name is said once, apart: `steps` itself is followed by the names of steps.
    const follows = name.members.length > 0 ? name.members : name.name === 'steps' ? language.step : [];
    if (follows.length > 0) one.lists.push({ what: 'follows', rows: chosen(follows) });
    return one;
  });
  const functions = language.functions.map((one) => shown(entry(functionId(one.name), 'function', one.name, '', one.usage), one.example, undefined));
  return [...shapes, ...names, ...functions];
}

/** What is written inside an argument or a key, as entries under it, and what it says of the things it holds. */
function inside(of: Entry, within: Within | undefined, action: string, path: string[]): void {
  if (!within) return;
  if (within.each !== undefined) of.marks.push({ what: 'each', says: within.each });
  if (within.words?.length) of.lists.push({ what: 'one-of', rows: chosen(within.words) });
  of.under = (within.keys ?? []).map((key: WithinKey) => {
    const one = entry(actionId(action, ...path, key.name), 'key', key.name, key.type, key.help);
    if (key.words?.length) one.lists.push({ what: 'one-of', rows: chosen(key.words) });
    inside(one, key.within, action, [...path, key.name]);
    return one;
  });
}

function actions(all: ToldAction[], used: Used | null): Entry[] {
  return all.map((action) => {
    const one = shown(entry(actionId(action.name), 'action', action.name, '', action.summary), action.example, used?.actions[action.name]);
    if (action.capability) one.marks.push({ what: 'needs', says: action.capability });
    if (action.makes) one.marks.push({ what: 'makes', says: action.makes.what }, { what: 'named', says: action.makes.named });
    if (action.makes?.setting) one.marks.push({ what: 'setting', says: action.makes.setting });
    for (const argument of action.runs ?? []) one.marks.push({ what: 'runs', says: argument });
    for (const name of action.reaches ?? []) one.marks.push({ what: 'reaches', says: name });
    if (action.idempotent) one.marks.push({ what: 'again' });
    if (action.open) one.marks.push({ what: 'open' });
    if (action.outputs.length > 0) one.lists.push({ what: 'gives', rows: action.outputs.map(([name, says]) => ({ name, says })) });
    one.under = action.args.map((arg) => {
      const argument = entry(actionId(action.name, arg.name), 'argument', arg.name, arg.type, arg.help);
      if (arg.required) argument.marks.push({ what: 'needed' });
      inside(argument, arg.within, action.name, [arg.name]);
      return argument;
    });
    return one;
  });
}

function events(all: ToldEvent[]): Entry[] {
  return all.map((event) => {
    const one = entry(eventId(event.topic), 'event', event.topic, '', event.summary);
    // Who is told of it: a scope, or anybody signed in where it names none.
    if (typeof event.hears === 'string') one.marks.push(event.hears === '' ? { what: 'hears' } : { what: 'hears', says: event.hears });
    if (!event.kept) one.marks.push({ what: 'passing' });
    if (event.open) one.marks.push({ what: 'open' });
    if (event.fields.length > 0) {
      one.lists.push({ what: 'carries', rows: event.fields.map((field) => ({ name: field.name, is: field.type, says: field.help, ...(field.always ? {} : { sometimes: true as const }) })) });
    }
    return one;
  });
}

/**
 * Everything the daemon describes of what a blueprint can say, as the
 * page's three parts; with where the blueprints there are use each thing,
 * where that was read.
 */
export function reference(language: BlueprintLanguage, used: Used | null = null, scenarios: Scenario[] = []): Part[] {
  const told = language as ToldLanguage;
  // The words as the language lists them, the file first; the actions and the events by name, to be found by eye.
  const byName = (entries: Entry[]) => [...entries].sort((a, b) => a.name.localeCompare(b.name));
  const parts: Part[] = [
    { name: 'words', entries: words(told, used) },
    { name: 'actions', entries: byName(actions(told.actions, used)) },
    { name: 'events', entries: byName(events(told.events ?? [])) },
  ];
  // A scenario is of an example, and is offered wherever that example is shown, at any level.
  const ofExample = new Map(scenarios.map((one) => [one.example, one]));
  const offer = (entries: Entry[]): void => {
    for (const one of entries) {
      const scenario = one.example ? ofExample.get(one.example) : undefined;
      if (scenario) one.scenario = scenario;
      offer(one.under);
    }
  };
  for (const part of parts) offer(part.entries);
  return parts;
}

/** Every entry of a list and under it, each before what is under it. */
export function every(entries: Entry[]): Entry[] {
  return entries.flatMap((one) => [one, ...every(one.under)]);
}

/**
 * How many things a part lists: the words, the actions, the events. What
 * is under one (an action's arguments, a mapping of the language, which
 * only holds words) is not counted as another.
 */
export function count(part: Part): number {
  const counted: EntryKind[] = part.name === 'words' ? ['word', 'name', 'function'] : part.name === 'actions' ? ['action'] : ['event'];
  return every(part.entries).filter((one) => counted.includes(one.kind)).length;
}

// ---- what is found ------------------------------------------------------------------

/** Everything an entry says of itself, as one text to look in: its name, its sentence, what is listed in it, and its example. */
function saying(one: Entry): string {
  const rows = one.lists.flatMap((list) => list.rows.flatMap((row) => [row.name, row.is ?? '', row.says]));
  return [one.name, one.is, one.says, ...one.marks.map((mark) => mark.says ?? ''), ...rows, one.example ?? ''].join('\n').toLowerCase();
}

/** The words somebody typed, as they are looked for: each of them, anywhere in what an entry says. */
export function asked(query: string): string[] {
  return query.toLowerCase().split(/\s+/).filter(Boolean);
}

/** Whether an entry says every word that was typed, in its name or anywhere in what it says of itself. */
export function matches(one: Entry, query: string[]): boolean {
  const text = saying(one);
  return query.every((word) => text.includes(word));
}

export interface Found {
  /** The parts with what was found: an entry that matches whole, and one that does not with only what was found under it. */
  parts: Part[];
  /** How many entries of each part match, at any depth. */
  counts: Record<PartName, number>;
  /**
   * The entries that are shown unfolded for what was found: one kept for
   * what is under it, and one that says what was typed only in what is
   * listed inside it, which is seen only where it is unfolded.
   */
  open: string[];
}

/**
 * What a search finds. An entry that matches is kept with everything
 * under it; one that does not is kept for what matches under it, with
 * only that, so that a key is found with what it is a key of. Nothing
 * typed finds everything, and counts what each part lists.
 */
export function found(parts: Part[], typed: string): Found {
  const query = asked(typed);
  const counts = { words: 0, actions: 0, events: 0 };
  const open: string[] = [];
  if (query.length === 0) {
    for (const part of parts) counts[part.name] = count(part);
    return { parts, counts, open };
  }
  // What an entry says at its head, which is drawn whether or not it is unfolded.
  const atHead = (one: Entry) => [one.name, one.is, one.says].join('\n').toLowerCase();
  const kept = (entries: Entry[], part: PartName): Entry[] =>
    entries.flatMap((one) => {
      if (matches(one, query)) {
        const below = every(one.under).filter((under) => matches(under, query));
        counts[part] += 1 + below.length;
        const head = atHead(one);
        if (below.length > 0 || !query.every((word) => head.includes(word))) open.push(one.id);
        return [one];
      }
      const under = kept(one.under, part);
      if (under.length > 0) open.push(one.id);
      return under.length > 0 ? [{ ...one, under }] : [];
    });
  return { parts: parts.map((part) => ({ name: part.name, entries: kept(part.entries, part.name) })), counts, open };
}

/**
 * The entries from the top of a part down to the one an id names, the
 * last being that one. Where nothing is named so, down to the nearest
 * there is above it (an address written for a key the daemon no longer
 * says is still an address of its argument); nothing where even its
 * first part names nothing.
 */
export function pathTo(parts: Part[], id: string): Entry[] | null {
  const down = (entries: Entry[]): Entry[] | null => {
    for (const one of entries) {
      if (one.id === id) return [one];
      if (id.startsWith(`${one.id}/`)) return [one, ...(down(one.under) ?? [])];
    }
    return null;
  };
  for (const part of parts) {
    const path = down(part.entries);
    if (path) return path;
  }
  return null;
}
