// What a card's column offers to be pressed on it, as a button, and what
// was pressed.
//
// That is not the client's to say. Each is a row of a table that sits
// beside the board, is about its columns and says who may press a row
// and whether it holds a card where it is (a fleet's `actions`): no table
// is known by its name for that. What was pressed is on the card's own
// row (`pressed`), which the board alone writes, and each press is said
// (`board.item.pressed`, and `board.item.held` where a card is said to
// wait for one), so that a page that is never read again follows it from
// what is said. The hold itself is the fleet's and the runner's: the
// page only says it, and presses through the daemon's one door. This
// holds nothing of the page, so that its tests can read it.

import { covers } from './holds.ts';
import { besideOf } from './sits.ts';
import type { Sat, Sitting } from './sits.ts';
import type { Row } from './table.ts';

/** One row: something that can be pressed on a card in a column. */
export interface Action {
  id: string;
  /** What it is pressed by: what its row says, or what it is called, written as a name is. */
  name: string;
  title: string;
  column: string;
  who: 'anyone' | 'person';
  scope: string;
  holds: boolean;
  again: boolean;
}

/** One press, as a card's row keeps it. */
export interface Press {
  action: string;
  coming: number;
  at: number;
  by: string;
  name: string;
}

/** What a card keeps of what was pressed on it. */
export interface Pressed {
  presses: Press[];
  /** The time it came to its column that it was last said to wait for a press. */
  held: number | null;
}

/** A name, as one is written: lowercase letters and digits, with a dash where anything else was. */
export function named(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/** The fields a table says who may press a row in, and whether it holds a card. */
const WHO = 'who';
const HOLDS = 'holds';

/**
 * The tables of a project whose rows are something pressed on a card: of
 * all its tables, those that sit beside its board, say which of its
 * columns a row is about, and have somewhere to say who may press a row
 * and whether it holds a card. No table is known by its name.
 */
export function pressing(all: Sitting[]): Sitting[] {
  return besideOf('board', all).filter(pressed);
}

/** Whether a table that sits beside the board is one whose rows are pressed: it says a column, who may press and whether a row holds. */
export function pressed(one: Sitting): boolean {
  const says = (field: string | null | undefined) => one.fields.some((each) => each.name === field);
  return says(one.about) && says(WHO) && says(HOLDS);
}

/** What can be pressed on a board's cards, of the tables that sit beside it as they were read: every row of each that is one, in order. */
export function among(sat: Sat[]): Action[] {
  return sat.filter(({ table }) => pressed(table)).flatMap(({ table, rows }) => actionsOf(table, rows ?? []));
}

const said = (value: unknown): string => (typeof value === 'string' ? value.trim() : '');
const flag = (value: unknown): boolean => value === true || value === 'true';

/** The rows of such a table, each read as the daemon reads one; a row called nothing is nothing to press. */
export function actionsOf(table: Sitting, rows: Row[]): Action[] {
  return rows
    .map((row): Action | null => {
      const title = said(row.data.title);
      const name = named(said(row.data.name) || title);
      if (name === '') return null;
      return {
        id: row.id,
        name,
        title: title || name,
        // The column it says, and no other: one that says none is offered nowhere, as the daemon reads it.
        column: said(row.data[table.about ?? '']),
        who: said(row.data.who) === 'person' ? 'person' : 'anyone',
        scope: said(row.data.scope),
        holds: flag(row.data.holds),
        again: flag(row.data.again),
      };
    })
    .filter((one): one is Action => one !== null);
}

/** What a card's row keeps of what was pressed, as the board writes it; a line that is not one of its own is passed over. */
export function pressedOf(kept: unknown): Pressed {
  const out: Pressed = { presses: [], held: null };
  if (!Array.isArray(kept)) return out;
  for (const line of kept) {
    if (typeof line !== 'string') continue;
    const words = line.split(' ');
    if (words[0] === 'pressed' && words.length >= 5) {
      const [, action, coming, at, by, ...name] = words as [string, string, string, string, string, ...string[]];
      if (!/^\d+$/.test(coming) || !/^-?\d+$/.test(at)) continue;
      out.presses = out.presses.filter((one) => one.action !== action);
      out.presses.push({ action, coming: Number(coming), at: Number(at), by, name: name.join(' ') });
    } else if (words[0] === 'held' && words.length === 2 && /^\d+$/.test(words[1]!)) out.held = Number(words[1]);
  }
  return out;
}

/** The same, as a card's row keeps it. */
export function keptOf(pressed: Pressed): string[] {
  const lines = pressed.presses.map((one) => `pressed ${one.action} ${one.coming} ${one.at} ${one.by}${one.name ? ` ${one.name}` : ''}`);
  return pressed.held === null ? lines : [...lines, `held ${pressed.held}`];
}

/** Which time a card came to where it is: the count of its moves, which its row says; none for one never moved. */
export function comingOf(moved: unknown): number {
  const count = Array.isArray(moved) && moved.length === 4 ? Number(moved[3]) : 0;
  return Number.isInteger(count) && count > 0 ? count : 0;
}

/** The press of `action` that stands at `coming`: its latest, where a press is for good, and only one of this coming where it is for one at a time. */
export function standing(pressed: Pressed, action: Action, coming: number): Press | null {
  return pressed.presses.find((one) => one.action === action.name && (!action.again || one.coming === coming)) ?? null;
}

/** What a card's column offers, each with the press that stands, in the order of the rows. */
export function offered(actions: Action[], status: string, pressed: Pressed, coming: number): { action: Action; press: Press | null }[] {
  return actions.filter((one) => one.column === status).map((action) => ({ action, press: standing(pressed, action, coming) }));
}

/** What a card waits for in its column: what holds there and stands unpressed. */
export function waitsFor(actions: Action[], status: string, pressed: Pressed, coming: number): Action[] {
  return offered(actions, status, pressed, coming)
    .filter(({ action, press }) => action.holds && press === null)
    .map(({ action }) => action);
}

/**
 * Whether somebody who holds `held`, and whose actor is of the kind
 * `kind`, is offered the button: what the daemon asks of whoever presses
 * (changing cards, a person where the row says so, and its scope). The
 * daemon asks the same and is what holds it; this is only what is drawn.
 */
export function mayPress(action: Action, held: string[], kind: string): boolean {
  return covers(held, 'board:write') && !(action.who === 'person' && kind === 'agent') && (action.scope === '' || covers(held, action.scope));
}

/** What a card's row keeps once what was said of it (`board.item.pressed`, `board.item.held`) is taken in, said at `ts`. */
export function after(kept: unknown, topic: string, data: Record<string, unknown>, ts: number): string[] {
  const pressed = pressedOf(kept);
  const coming = typeof data.coming === 'number' ? data.coming : 0;
  if (topic === 'board.item.held') return keptOf({ ...pressed, held: coming });
  if (topic !== 'board.item.pressed') return keptOf(pressed);
  const action = said(data.action);
  const rest = pressed.presses.filter((one) => one.action !== action);
  if (data.back === true) return keptOf({ ...pressed, presses: rest });
  return keptOf({ ...pressed, presses: [...rest, { action, coming, at: ts, by: said(data.by), name: said(data.name) }] });
}

/** Who pressed, in a few words: you, or what they are called. */
export function whoPressed(press: Press, me: string): string {
  if (press.by === me) return 'you';
  return press.name || press.by.split(':').pop() || 'somebody';
}
