// What each thing a new project starts with is set up with, in the
// dialog that adds a project.
//
// What may be set of each is its blueprint's own params, less what it is
// told of the project, as the daemon reads them (`/api/starts`): nothing
// here names a param. What somebody writes of one is kept as the text of
// its box, as the drawer that applies a blueprint keeps it, and only what
// they wrote is handed on: a box nobody wrote in says nothing, and the
// blueprint goes by what its text says there.
//
// The rows are a setup's: the factory's default, or one it keeps under a
// name (`/api/setups`). A row of somebody's own is added beside them by
// the blueprint it applies (`ownRow`) and is asked for whole
// (`askedWith`); and the table as it stands is saved as a setup
// (`savedRows`), each row on as its switch is and handed what was written.

import type { Fields, GraphParam } from './api.ts';
import { answered } from './applyasked.ts';
import type { Start } from './starts.ts';

/** What may be set of each thing a project starts with, by its name. */
export type Settable = Record<string, GraphParam[]>;

/** What was written of each, by its name and then its param's. */
export type Written = Record<string, Record<string, string>>;

/** One thing asked for as a project is added: its name, or its name with what its blueprint is handed. */
export type AskedStart = string | { name: string; with: Fields };

/** What was written of `name` once its param `param` says `text`. */
export function writtenAfter(written: Written, name: string, param: string, text: string): Written {
  return { ...written, [name]: { ...(written[name] ?? {}), [param]: text } };
}

/** How many of its settings somebody wrote something other than what it starts as. */
export function changedOf(params: GraphParam[], written: Record<string, string> | undefined, startsAs: (param: GraphParam) => string): number {
  return params.filter((param) => written?.[param.name] !== undefined && written[param.name] !== startsAs(param)).length;
}

/**
 * What the daemon is asked a project is to start with: each of `chosen`
 * by its name, and where somebody wrote any of its settings, its name
 * with what they wrote read by each param's type. Or the first thing that
 * cannot be read, said with whose setting it is.
 */
export function askedOf(chosen: string[], settable: Settable, written: Written, titled: (name: string) => string): { asked: AskedStart[] } | { wrong: string; at: string } {
  const asked: AskedStart[] = [];
  for (const name of chosen) {
    const mine = written[name] ?? {};
    const params = (settable[name] ?? []).filter((param) => mine[param.name] !== undefined);
    if (params.length === 0) {
      asked.push(name);
      continue;
    }
    const read = answered(params, mine);
    if ('wrong' in read) return { wrong: `${titled(name)}: ${read.wrong}`, at: name };
    asked.push(Object.keys(read.values).length > 0 ? { name, with: read.values } : name);
  }
  return { asked };
}

/** One setup the factory keeps, as `/api/setups` hands it. */
export interface Setup {
  name: string;
  rows: Start[];
  by: string;
  at: number;
  /** Whether it is the factory's default for new projects. */
  default: boolean;
}

/** What the factory's own default for new projects is called. */
export const DEFAULT = 'default';

/** What a name a person writes comes to as the name of a row or a setup: small letters, digits and dashes. */
export function nameOf(words: string): string {
  return words
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .replace(/-{2,}/g, '-');
}

/**
 * A row of somebody's own, for `blueprint`, beside the rows that are
 * there (`taken`, by name): called by the last part of the blueprint's
 * name, numbered where a row is called that already, on, needing
 * nothing, offered as the project is added.
 */
export function ownRow(blueprint: string, taken: string[]): Start {
  const last = blueprint.split('/').at(-1) ?? blueprint;
  const base = nameOf(last) || 'row';
  let name = base;
  for (let nth = 2; taken.includes(name); nth += 1) name = `${base}-${nth}`;
  return { name, title: name === base ? last : `${last} ${name.slice(base.length + 1)}`, says: blueprint, blueprint, on: true, needs: [], why: '', offered: 'adding' };
}

/**
 * What the daemon is asked a project is to start with where some rows
 * are somebody's own (`own`, by name): those of the setup by their names
 * as `askedOf` asks them, and each of one's own whole, with what was
 * written of it.
 */
export function askedWith(chosen: string[], rows: Start[], own: string[], settable: Settable, written: Written, titled: (name: string) => string): { asked: AskedOwn[] } | { wrong: string; at: string } {
  const read = askedOf(chosen, settable, written, titled);
  if ('wrong' in read) return read;
  const asked = read.asked.map((one): AskedOwn => {
    const name = typeof one === 'string' ? one : one.name;
    if (!own.includes(name)) return one;
    const row = rows.find((each) => each.name === name)!;
    const handed = typeof one === 'string' ? {} : one.with;
    return { name, title: row.title, says: row.says, blueprint: row.blueprint, with: handed };
  });
  return { asked };
}

/** One thing asked for: by its name, with what its blueprint is handed, or a row of one's own. */
export type AskedOwn = AskedStart | { name: string; title: string; says: string; blueprint: string; with: Fields };

/**
 * The rows a setup is saved as, from the table as it stands: every row
 * of it, those offered as a project is added on where they are `chosen`
 * and those offered elsewhere as they were, each handed what its setup
 * said with what was written over it, read by each param's type. Or the
 * first thing that cannot be read, said with whose setting it is.
 */
export function savedRows(rows: Start[], chosen: string[], settable: Settable, written: Written, titled: (name: string) => string): { rows: Start[] } | { wrong: string; at: string } {
  const saved: Start[] = [];
  for (const row of rows) {
    const mine = written[row.name] ?? {};
    const params = (settable[row.name] ?? []).filter((param) => mine[param.name] !== undefined);
    const read = answered(params, mine);
    if ('wrong' in read) return { wrong: `${titled(row.name)}: ${read.wrong}`, at: row.name };
    const on = (row.offered ?? 'adding') === 'adding' ? chosen.includes(row.name) : row.on;
    saved.push({ ...row, on, with: { ...(row.with ?? {}), ...read.values } });
  }
  return { rows: saved };
}
