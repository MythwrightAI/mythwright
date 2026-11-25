// A cell that is a list of scopes.
//
// A table may say of a list that it is scopes a record gives somebody
// (`takes: each`, where its blueprint declares the field: what a fleet's
// agents hold), and what such a list stands for where a record leaves it
// unsaid (`unsaid`). The client tells such a cell by that, never by what
// its field is called, and shows and fills it as it shows what anybody
// may ask of the factory: by area (`holds.ts`). In a table the cell says
// it short; pressed, it is a line an area. This holds nothing of the
// page, so that its tests can read it.

import { covers, scopesAfter, tableOf } from './holds.ts';
import type { Area } from './holds.ts';
import type { Field } from './table.ts';

/** Whether a field is a list of scopes that a record gives somebody. */
export function isScopes(field: Field): boolean {
  return field.kind === 'list' && field.takes_each === true;
}

/** What a cell says: the scopes written there, or nothing where the record leaves it unsaid (never filled, or emptied). */
export function saidIn(value: unknown): string[] | null {
  return Array.isArray(value) && value.length > 0 ? value.map(String) : null;
}

/** What a cell comes to: what it says, and where it says nothing what its field says that stands for. */
export function comesTo(field: Field, value: unknown): string[] {
  return saidIn(value) ?? field.unsaid ?? [];
}

/**
 * A cell in a few words, for a table: `the usual` where the record says
 * nothing and its field says what that stands for, `everything`, or the
 * areas something is held of with how many they are.
 */
export function briefly(field: Field, value: unknown, areas: Area[]): string {
  const said = saidIn(value);
  if (said === null) return (field.unsaid ?? []).length > 0 ? 'the usual' : 'nothing';
  if (said.includes('*')) return 'everything';
  // A daemon from before lists no areas: the scopes are said as they are written.
  if (areas.length === 0) return said.join(', ');
  const held = tableOf(said, areas).filter((row) => row.whole || row.verbs.length > 0 || row.only.length > 0);
  if (held.length === 0) return 'nothing';
  return `${held.length} ${held.length === 1 ? 'area' : 'areas'}: ${held.map((row) => row.area).join(', ')}`;
}

/**
 * The same of a list that is whole, with no cell to leave it unsaid: what
 * an agent holds, as the daemon says it. It is `the usual` where it is
 * what such a one holds unless told otherwise (`usual`, each thing once
 * and in any order), and otherwise said as a cell that says it would be.
 */
export function brieflyHeld(scopes: string[], usual: string[], areas: Area[]): string {
  const held = new Set(scopes);
  if (usual.length > 0 && held.size === new Set(usual).size && usual.every((scope) => held.has(scope))) return 'the usual';
  if (scopes.length === 0) return 'nothing';
  return briefly({ name: '', kind: 'list', of: [], required: false }, scopes, areas);
}

/**
 * What of the scopes a record gives somebody who holds `mine` does not
 * hold themselves. Nobody writes a record that gives what they do not
 * hold, whichever of its cells they change, so a cell that comes to any
 * of these is not theirs to change: said where it is shown, and not by a
 * save that is refused.
 */
export function lacking(mine: string[], scopes: string[]): string[] {
  return scopes.filter((scope) => !covers(mine, scope));
}

/** What a cell holds once one line of it is changed: the whole list, with what was left unsaid written out. */
export function afterLine(field: Field, value: unknown, areas: Area[], area: string, verbs: string[]): string[] {
  return scopesAfter(comesTo(field, value), areas, area, verbs);
}
