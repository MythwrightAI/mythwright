// How a project's things are arranged in the side panel: its outline, as
// plain values. A thing is one line the panel draws for a project, named
// by a key (`board`, `table:<name>`, `library:<name>`, `mockups:<name>`,
// `environments`, `pipelines`, `blueprints`, `git`, `repo`, and later
// `entry:<path>`);
// a folder is the outline's own, an id the factory gives it, a name and
// what is in it. A thing may be shown as something else for everybody
// (`name` of a thing, set by the `name` change), while what everything
// else knows it by stays; a table is called something else by its title,
// and is not named here. The daemon keeps it for each project and acts on none of
// it (`kernel::outline`, the `outline` crate).
//
// What an outline is held to (`checked`) and what each change comes to
// (`apply`) are the daemon's rules, written here again and held case for
// case to the one table both read (`outlinecases.json`). What is drawn is
// `laidOut`: with no outline, the lines the panel always drew in the order
// it always drew them; with one, its nodes in order, each thing only where
// the page lists it today, then whatever the page lists that it does not
// name, at the end in the usual order.
//
// Holds nothing of the page, so that its tests can read it.

import { titled } from './table.ts';
import type { At } from './treedrag.ts';

/** One node of an outline: a thing by its key, or a folder with what is in it. A key that is not known here is kept as it was. */
export type Node = ({ thing: string; name?: string } | { folder: string; name: string; items: Node[] }) & Record<string, unknown>;

/** A project's outline, as it is kept. */
export interface Outline {
  nth: number;
  next: number;
  items: Node[];
  [more: string]: unknown;
}

/** The most letters a folder's name has (held to the daemon's by `outlinecases.json`). */
export const LONGEST_NAME = 60;
export const LONGEST_KEY = 2048;
/** How many folders deep an outline goes at most. */
export const DEEPEST = 12;
export const MOST_NODES = 2000;
export const MOST_FOLDERS = 500;
const KEYS = ['board', 'environments', 'pipelines', 'blueprints', 'git', 'repo'];
const NAMED = ['table:', 'library:', 'mockups:', 'entry:'];

const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;
const letters = (text: string) => [...text].length;
const isObject = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);
const shown = (value: unknown) => JSON.stringify(value);

/** Whether `named` is a folder's id rather than a thing's key. */
export function isFolder(named: string): boolean {
  return /^f[0-9]+$/.test(named);
}

const numberOf = (id: string) => Number(id.slice(1)) || 0;

/** Why `key` is no thing's key, if it is not. */
export function unfitKey(key: unknown): string | null {
  if (typeof key !== 'string') return `A thing is named by its key, which is text, and ${shown(key)} is not.`;
  if (CONTROL.test(key)) return "A thing's key is one line of text.";
  if (letters(key) > LONGEST_KEY) return `A thing's key is ${LONGEST_KEY} letters at most, and this one is ${letters(key)}.`;
  if (KEYS.includes(key) || NAMED.some((begins) => key.length > begins.length && key.startsWith(begins))) return null;
  return `\`${key}\` is no thing of a project: a thing is \`board\`, \`table:<name>\`, \`library:<name>\`, \`mockups:<name>\`, \`environments\`, \`pipelines\`, \`blueprints\`, \`git\`, \`repo\` or \`entry:<path>\`.`;
}

/** What a folder is called, trimmed, or why `name` is not that. */
export function namedFolder(name: unknown): { name: string } | { wrong: string } {
  const said = (typeof name === 'string' ? name : '').trim();
  if (said === '') return { wrong: 'A folder is called something: say what this one is called.' };
  if (letters(said) > LONGEST_NAME) return { wrong: `A folder's name is ${LONGEST_NAME} letters at most, and this one is ${letters(said)}.` };
  if (CONTROL.test(said)) return { wrong: "A folder's name is one line of words." };
  return { name: said };
}

/** What a thing is shown as, trimmed: nothing where it is shown as it always was (nothing said, or only spaces); or why `name` is not that. */
export function shownName(name: unknown): { name: string | null } | { wrong: string } {
  if (name === undefined || name === null) return { name: null };
  if (typeof name !== 'string') return { wrong: `What a thing is shown as is words, and ${shown(name)} is not.` };
  const said = name.trim();
  if (said === '') return { name: null };
  if (letters(said) > LONGEST_NAME) return { wrong: `What a thing is shown as is ${LONGEST_NAME} letters at most, and this is ${letters(said)}.` };
  if (CONTROL.test(said)) return { wrong: 'What a thing is shown as is one line of words.' };
  return { name: said };
}

class Refused extends Error {}
const refuse = (sentence: string): never => {
  throw new Refused(sentence);
};

const NOT_ITEMS = '`items` is a list, each a thing or a folder.';

interface Read {
  things: string[];
  folders: string[];
  nodes: number;
  deepest: number;
}

function readItems(items: unknown, depth: number, read: Read): Node[] {
  if (items === undefined || items === null) return [];
  if (!Array.isArray(items)) return refuse(NOT_ITEMS);
  const nodes: Node[] = [];
  const names: string[] = [];
  for (const item of items) {
    if (!isObject(item)) return refuse('Each item of an outline is a mapping: a thing (`thing`) or a folder (`folder`).');
    read.nodes += 1;
    const { thing, folder, ...more } = item;
    const has = (key: string) => key in item;
    if (has('thing') && has('folder')) return refuse('An item of an outline is a thing (`thing`) or a folder (`folder`), and this one says both.');
    if (!has('thing') && !has('folder')) return refuse('An item of an outline is a thing (`thing`) or a folder (`folder`), and this one is neither.');
    if (has('thing')) {
      const wrong = unfitKey(thing);
      if (wrong) return refuse(wrong);
      const key = thing as string;
      if (read.things.includes(key)) return refuse(`\`${key}\` is in the outline twice: each thing is in one place.`);
      read.things.push(key);
      const { name: said, ...rest } = more;
      const named = shownName(said);
      if ('wrong' in named) return refuse(named.wrong);
      nodes.push(named.name === null ? { ...rest, thing: key } : { ...rest, thing: key, name: named.name });
      continue;
    }
    if (typeof folder !== 'string' || !isFolder(folder)) {
      return refuse(`A folder is named by an id the factory gives it (\`f1\`, \`f2\`, …), and ${shown(folder)} is not one.`);
    }
    if (read.folders.includes(folder)) return refuse(`Two folders are \`${folder}\`: each folder has an id of its own.`);
    read.folders.push(folder);
    const { name: said, items: inside, ...rest } = more;
    const named = namedFolder(said);
    if ('wrong' in named) return refuse(named.wrong);
    if (names.some((one) => one.toLowerCase() === named.name.toLowerCase())) {
      return refuse(`Two folders here are called ${named.name}: each is called something of its own there.`);
    }
    names.push(named.name);
    read.deepest = Math.max(read.deepest, depth + 1);
    nodes.push({ ...rest, folder, name: named.name, items: readItems(inside, depth + 1, read) });
  }
  return nodes;
}

function count(value: unknown): number {
  if (value === undefined || value === null) return 0;
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 0) return refuse('`nth` and `next` are counts.');
  return value;
}

function checking(said: unknown): Outline {
  if (!isObject(said)) return refuse('An outline is a mapping of its `items`, each a thing or a folder.');
  const { nth, next, items, ...more } = said;
  const [nthSaid, nextSaid] = [count(nth), count(next)];
  const read: Read = { things: [], folders: [], nodes: 0, deepest: 0 };
  const nodes = readItems(items, 0, read);
  if (read.deepest > DEEPEST) refuse(`An outline goes ${DEEPEST} folders deep at most.`);
  if (read.folders.length > MOST_FOLDERS) refuse(`An outline holds ${MOST_FOLDERS} folders at most, and this one holds ${read.folders.length}.`);
  if (read.nodes > MOST_NODES) refuse(`An outline holds ${MOST_NODES} things and folders at most, and this one holds ${read.nodes}.`);
  const past = Math.max(1, ...read.folders.map((id) => numberOf(id) + 1));
  return { ...more, nth: nthSaid, next: Math.max(nextSaid, past, 1), items: nodes };
}

/** The outline `said` is, as it is kept, or why it is not one. */
export function checked(said: unknown): { outline: Outline } | { wrong: string } {
  try {
    return { outline: checking(said) };
  } catch (error) {
    if (error instanceof Refused) return { wrong: error.message };
    throw error;
  }
}

/** What a node is named by in a change: a thing's key, or a folder's id. */
export const namedOf = (node: Node): string => ('folder' in node ? (node.folder as string) : (node.thing as string));

/** Every thing's key an outline names, wherever it is. */
export function thingsOf(items: Node[]): string[] {
  return items.flatMap((node) => ('folder' in node ? thingsOf(node.items as Node[]) : [node.thing as string]));
}

/** Every folder of an outline, by id, wherever it is. */
export function foldersOf(items: Node[]): string[] {
  return items.flatMap((node) => ('folder' in node ? [node.folder as string, ...foldersOf(node.items as Node[])] : []));
}

// Where something is put, what a carry measures and where it lands are a
// tree's and no outline's (`treedrag.ts`), and are offered here under the
// names they always had.
export type { At, Landing, Row, Step } from './treedrag.ts';
export { landing, stepped } from './treedrag.ts';

/** A change of an outline, as the daemon takes it. */
export type Op =
  | ({ op: 'make'; name: string } & At)
  | ({ op: 'put'; nodes?: string[]; node?: string } & At)
  | { op: 'rename'; folder: string; name: string }
  | { op: 'remove'; folder: string }
  | { op: 'restore'; removed: { folder: string; name: string; into?: string | null; before?: string | null; items: string[] } }
  | { op: 'name'; thing: string; name: string | null }
  | { op: 'forget' };

/** What a change came to. */
export interface Applied {
  /** The outline as it is to be kept: nothing where it is forgotten. */
  outline: Outline | null;
  changed: boolean;
  done: Record<string, unknown>;
}

const NOT_AN_OP = 'An outline is changed by one of `make`, `put`, `rename`, `remove`, `restore`, `name` and `forget`, said as `op`.';
const noFolder = (id: string) => `There is no folder \`${id}\` in the outline.`;

function levelOf(items: Node[], into: string | null | undefined): Node[] | null {
  if (!into) return items;
  for (const node of items) {
    if ('folder' in node) {
      if (node.folder === into) return node.items as Node[];
      const found = levelOf(node.items as Node[], into);
      if (found) return found;
    }
  }
  return null;
}

function take(items: Node[], named: string, into: string | null): { node: Node; into: string | null; after: string | null } | null {
  const at = items.findIndex((node) => namedOf(node) === named);
  if (at >= 0) {
    const node = items.splice(at, 1)[0] as Node;
    const next = items[at];
    return { node, into, after: next ? namedOf(next) : null };
  }
  for (const node of items) {
    if ('folder' in node) {
      const found = take(node.items as Node[], named, node.folder as string);
      if (found) return found;
    }
  }
  return null;
}

function find(items: Node[], named: string): Node | null {
  for (const node of items) {
    if (namedOf(node) === named) return node;
    if ('folder' in node) {
      const found = find(node.items as Node[], named);
      if (found) return found;
    }
  }
  return null;
}

function within(items: Node[], outer: string, inner: string): boolean {
  const found = find(items, outer);
  return !!found && 'folder' in found && (outer === inner || find(found.items as Node[], inner) !== null);
}

function findFolder(items: Node[], folder: string): Node | null {
  const found = find(items, folder);
  return found && 'folder' in found ? found : null;
}

function putAt(items: Node[], nodes: Node[], at: At): void {
  const level = levelOf(items, at.into) ?? refuse(noFolder(at.into ?? ''));
  let place = level.length;
  if (at.before) {
    place = level.findIndex((node) => namedOf(node) === at.before);
    if (place < 0) refuse(`\`${at.before}\` is not where it is to be put, so nothing can go before it there.`);
  }
  level.splice(place, 0, ...nodes);
}

/** The same value with every mapping's keys in one order, for telling two apart. */
function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (isObject(value)) return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonical(value[key])]));
  return value;
}
const same = (a: unknown, b: unknown) => JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));
const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const text = (value: unknown): string | null => (typeof value === 'string' ? value : null);

function applying(kept: Outline | null, said: unknown, seen: string[]): Applied {
  if (!isObject(said)) return refuse(NOT_AN_OP);
  const op = said.op;
  if (op === 'forget') return { outline: null, changed: kept !== null, done: {} };
  if (!['make', 'put', 'rename', 'remove', 'restore', 'name'].includes(op as string)) return refuse(NOT_AN_OP);
  if (op === 'name') {
    const wrong = unfitKey(said.thing ?? null);
    if (wrong) refuse(wrong);
    if ((said.thing as string).startsWith('table:')) {
      refuse(`\`${said.thing as string}\` is a table, which is called something else by its own title, so that every page calls it by one name: rename the table itself.`);
    }
    const named = shownName(said.name);
    if ('wrong' in named) refuse(named.wrong);
  }
  // What the change says is read before anything is done, as the daemon reads it.
  const nameSaid = op === 'make' || op === 'rename' ? said.name : op === 'restore' && isObject(said.removed) ? said.removed.name : 'said';
  const asked = namedFolder(nameSaid);
  if ('wrong' in asked) refuse(asked.wrong);
  const outline: Outline = kept ? copy(kept) : { nth: 0, next: 1, items: [] };
  const named = thingsOf(outline.items);
  for (const key of seen) {
    const wrong = unfitKey(key);
    if (wrong) refuse(wrong);
    if (!named.includes(key)) {
      named.push(key);
      outline.items.push({ thing: key });
    }
  }
  const before = copy(outline);
  let done: Record<string, unknown> = {};
  const at: At = { into: text(said.into), before: text(said.before) };
  if (op === 'make') {
    const name = namedFolder(said.name);
    if ('wrong' in name) refuse(name.wrong);
    const id = `f${outline.next}`;
    outline.next += 1;
    putAt(outline.items, [{ folder: id, name: (name as { name: string }).name, items: [] }], at);
    done = { made: id };
  } else if (op === 'put') {
    const list = (value: unknown): string[] => {
      if (value === undefined || value === null) return [];
      if (typeof value === 'string') return [value];
      if (Array.isArray(value) && value.every((one) => typeof one === 'string')) return value as string[];
      return refuse(`\`${value === said.nodes ? 'nodes' : 'node'}\` is a list of names.`);
    };
    const nodes = [...list(said.nodes), ...list(said.node)];
    if (nodes.length === 0) refuse("Say what is to be put: a thing's key, or a folder's id.");
    nodes.forEach((one, n) => {
      if (nodes.slice(0, n).includes(one)) refuse(`\`${one}\` is said twice in what is to be put.`);
      if (at.before === one) refuse(`\`${one}\` cannot be put before itself.`);
      if (isFolder(one)) {
        if (at.into && within(outline.items, one, at.into)) refuse('A folder cannot be put inside itself.');
      } else {
        const wrong = unfitKey(one);
        if (wrong) refuse(wrong);
      }
    });
    const moving: Node[] = [];
    for (const one of nodes) {
      const taken = take(outline.items, one, null);
      if (taken) moving.push(taken.node);
      else if (isFolder(one)) refuse(noFolder(one));
      else moving.push({ thing: one });
    }
    putAt(outline.items, moving, at);
  } else if (op === 'rename') {
    const name = namedFolder(said.name);
    if ('wrong' in name) refuse(name.wrong);
    const folder = text(said.folder) ?? '';
    const found = findFolder(outline.items, folder) ?? refuse(noFolder(folder));
    found.name = (name as { name: string }).name;
  } else if (op === 'name') {
    const thing = said.thing as string;
    const name = (shownName(said.name) as { name: string | null }).name;
    const found = find(outline.items, thing);
    if (found && !('folder' in found)) {
      if (name === null) delete found.name;
      else found.name = name;
    } else if (name !== null) {
      // A thing the outline does not name yet is named at the end of the top, as it is put there.
      outline.items.push({ thing, name });
    }
  } else if (op === 'remove') {
    const folder = text(said.folder) ?? '';
    const found = findFolder(outline.items, folder) ? take(outline.items, folder, null) : null;
    if (!found) return refuse(noFolder(folder));
    const items = found.node.items as Node[];
    const removed = { folder, name: found.node.name, into: found.into, before: found.after, items: items.map(namedOf) };
    putAt(outline.items, items, { into: found.into, before: found.after });
    done = { removed };
  } else {
    const removed = isObject(said.removed) ? said.removed : refuse('What is put back is what removing a folder handed back, said as `removed`.');
    const name = namedFolder(removed.name);
    if ('wrong' in name) refuse(name.wrong);
    const folder = text(removed.folder) ?? '';
    const items = removed.items === undefined || removed.items === null ? [] : typeof removed.items === 'string' ? [removed.items] : (removed.items as string[]);
    if (!isFolder(folder)) refuse(`A folder is named by an id the factory gives it (\`f1\`, \`f2\`, …), and "${folder}" is not one.`);
    if (findFolder(outline.items, folder)) refuse(`There is a folder \`${folder}\` already: what was removed is put back only where its id is free.`);
    const into = text(removed.into) && levelOf(outline.items, text(removed.into)) ? text(removed.into) : null;
    const level = levelOf(outline.items, into) ?? [];
    const names = level.map(namedOf);
    let last = -1;
    names.forEach((one, n) => {
      if (items.includes(one)) last = n;
    });
    const after = last >= 0 && last + 1 < names.length ? names[last + 1] : null;
    const stood = text(removed.before);
    const there = stood && names.includes(stood) ? stood : after;
    const inside: Node[] = [];
    for (const one of items) {
      const taken = take(outline.items, one, null);
      if (taken) inside.push(taken.node);
      else if (!isFolder(one)) inside.push({ thing: one });
    }
    putAt(outline.items, [{ folder, name: (name as { name: string }).name, items: inside }], { into, before: there && !items.includes(there) ? there : null });
    outline.next = Math.max(outline.next, numberOf(folder) + 1);
  }
  const result = checking(outline);
  const { nth: _n, next: _x, items: _i, ...beforeMore } = before;
  const { nth: _m, next: _y, items: _j, ...resultMore } = result;
  if (same(result.items, before.items) && same(resultMore, beforeMore)) return { outline: kept, changed: false, done };
  return { outline: result, changed: true, done };
}

/** The outline `kept` (nothing where none is kept), with `seen` named at the end of its top and then `op` done: what it comes to, or why it is refused. */
export function apply(kept: Outline | null, op: unknown, seen: string[]): Applied | { wrong: string } {
  try {
    return applying(kept, op, seen);
  } catch (error) {
    if (error instanceof Refused) return { wrong: error.message };
    throw error;
  }
}

// ---- What is drawn -------------------------------------------------------------

/** What a project lists under it today, as much as says which lines there are. */
export interface Listing {
  board: boolean;
  tables: string[];
  libraries: string[];
  mockups: string[];
  environments: boolean;
  pipelines: boolean;
  blueprints: boolean;
  git: boolean;
  repo: boolean;
}

/** The keys of the lines a project lists today, in the order they have always been drawn. */
export function keysOf(listing: Listing): string[] {
  return [
    ...(listing.board ? ['board'] : []),
    ...listing.tables.map((name) => `table:${name}`),
    ...listing.libraries.map((name) => `library:${name}`),
    ...listing.mockups.map((name) => `mockups:${name}`),
    ...(listing.environments ? ['environments'] : []),
    ...(listing.pipelines ? ['pipelines'] : []),
    ...(listing.blueprints ? ['blueprints'] : []),
    ...(listing.git ? ['git'] : []),
    ...(listing.repo ? ['repo'] : []),
  ];
}

/** One line drawn under a project: a thing by its key (and what it is shown as, where somebody named it), or a folder with what is drawn in it. */
export type Laid = { thing: string; name?: string } | { folder: string; name: string; items: Laid[] };

/**
 * What is drawn under a project, from the keys of what it lists today
 * (`keysOf`) and its outline. With none, those lines in their order. With
 * one, its nodes in order: a folder whether or not anything in it is
 * drawn, a thing only where it is listed today (one that sits beside
 * another, is kept from the reader or is gone is passed over); and then
 * whatever is listed that it does not name, at the end, in the order
 * those lines always stood.
 */
export function laidOut(listed: string[], outline: Outline | null): Laid[] {
  if (!outline) return listed.map((thing) => ({ thing }));
  const placed = new Set<string>();
  const walk = (items: Node[]): Laid[] =>
    items.flatMap((node): Laid[] => {
      if ('folder' in node) return [{ folder: node.folder as string, name: node.name as string, items: walk(node.items as Node[]) }];
      const key = node.thing as string;
      if (!listed.includes(key) || placed.has(key)) return [];
      placed.add(key);
      return [typeof node.name === 'string' ? { thing: key, name: node.name } : { thing: key }];
    });
  const laid = walk(outline.items);
  return [...laid, ...listed.filter((key) => !placed.has(key)).map((thing) => ({ thing }))];
}

/** What a thing's line has always said, by its key: Board, a library or mockups by its name as it reads, Environments, Git, … (a table says its title, `called.ts`). */
export function usualOf(key: string): string {
  return titled(key.includes(':') ? key.slice(key.indexOf(':') + 1) : key);
}

/** What a thing is, said faintly beside what somebody named it so that it can still be told: `Board · table board`, `Library · table docs`. */
export function whatIs(key: string): string {
  const [kind, name] = key.includes(':') ? [key.slice(0, key.indexOf(':')), key.slice(key.indexOf(':') + 1)] : [key, key];
  if (key === 'board' || kind === 'library' || kind === 'mockups') return `${titled(kind)} · table ${name}`;
  if (kind === 'entry') return `File · ${name}`;
  return titled(kind);
}

/** What the thing `key` is shown as where somebody named it in `outline`: nothing where it is shown as it always was. */
export function shownAs(outline: Outline | null, key: string): string | null {
  const found = outline ? find(outline.items, key) : null;
  return found && !('folder' in found) && typeof found.name === 'string' ? found.name : null;
}

// ---- What is asked from the page ------------------------------------------------

/**
 * What a change is sent with (`seen`): the things the page draws at the
 * top that the outline does not name, in the order drawn. Those are named
 * first, at the end of the top, so that a place said by what was seen is
 * the place meant; with no outline, every line drawn.
 */
export function seenOf(listed: string[], outline: Outline | null): string[] {
  const named = new Set(thingsOf(outline?.items ?? []));
  return listed.filter((key) => !named.has(key));
}

/** A line of Move to…: the top of the project, or a folder at its depth (from 0, the top's own); where the thing is now marked. */
export type Place = { kind: 'top'; here: boolean } | { kind: 'folder'; id: string; name: string; depth: number; here: boolean };

/**
 * Where something drawn under a project can be moved (`node`, a thing's
 * key or a folder's id), from what is drawn (`laidOut`): the top, then
 * each folder in the order drawn, each at its depth, where it is now
 * marked. A folder is not offered itself, nor anything inside it.
 */
export function placesFor(laid: Laid[], node: string): Place[] {
  const places: Place[] = [];
  let here: string | null | undefined;
  const walk = (items: Laid[], depth: number, within: string | null) => {
    for (const one of items) {
      const named = 'folder' in one ? one.folder : one.thing;
      if (named === node) here = within;
      if (!('folder' in one)) continue;
      if (one.folder === node) {
        // What is inside it is no place for it; what is in it is still looked through for nothing more.
        continue;
      }
      places.push({ kind: 'folder', id: one.folder, name: one.name, depth, here: false });
      walk(one.items, depth + 1, one.folder);
    }
  };
  walk(laid, 0, null);
  return [{ kind: 'top', here: here === null }, ...places.map((place) => (place.kind === 'folder' && place.id === here ? { ...place, here: true } : place))];
}

/** The folders drawn under a project, by id, with what each is called: for what is offered and said of them. */
export function foldersLaid(laid: Laid[]): { id: string; name: string }[] {
  return laid.flatMap((one) => ('folder' in one ? [{ id: one.folder, name: one.name }, ...foldersLaid(one.items)] : []));
}

/** What a folder is to be called, as a dialog asks it: refused before anything is asked where it is empty, too long, or another folder beside it is called it. */
export function folderNamed(typed: string, beside: string[]): { name: string } | { wrong: string } {
  const named = namedFolder(typed);
  if ('wrong' in named) return named;
  if (beside.some((one) => one.toLowerCase() === named.name.toLowerCase())) {
    return { wrong: `Another folder here is called ${named.name}: call this one something of its own.` };
  }
  return named;
}

/** The names of the folders directly in `into` (the top, where nothing), as drawn: what a new folder there may not be called. */
export function namesIn(laid: Laid[], into: string | null): string[] {
  const level = (items: Laid[]): Laid[] | null => {
    if (into === null) return items;
    for (const one of items) {
      if (!('folder' in one)) continue;
      if (one.folder === into) return one.items;
      const found = level(one.items);
      if (found) return found;
    }
    return null;
  };
  return (level(laid) ?? []).flatMap((one) => ('folder' in one ? [one.name] : []));
}
