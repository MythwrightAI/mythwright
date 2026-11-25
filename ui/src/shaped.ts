// Asking the daemon to change a table's columns, for everybody who
// opens the table (`added.ts` says what each is; `studio::added` holds
// it). Each answers with the table as it then is.

import type { Place } from './added.ts';
import { call } from './api.ts';
import type { Collection } from './api.ts';

const part = encodeURIComponent;
const at = (project: string, table: string, field?: string) => `/collections/${part(project)}/${part(table)}/fields${field === undefined ? '' : `/${part(field)}`}`;

/** A column to add: what it is called and holds, or the column it is a copy of; and where it stands in a view. */
export interface Asked {
  label?: string;
  kind?: string;
  of?: string[];
  like?: string;
  place?: Place;
}

/** Adds a column. What it is called by everything that reads it comes back with the table. */
export function addColumn(project: string, table: string, asked: Asked): Promise<{ table: Collection; field: string }> {
  return call('POST', at(project, table), asked);
}

/** Calls a column something else, or with nothing what its name reads as. */
export function callColumn(project: string, table: string, field: string, label: string | null): Promise<Collection> {
  return call('PATCH', at(project, table, field), { label });
}

/** Changes what kind of thing a column somebody added holds. */
export function retypeColumn(project: string, table: string, field: string, kind: string, of: string[]): Promise<Collection> {
  return call('PATCH', at(project, table, field), { kind, of });
}

/** Says what a choice offers, in order: the same things in another order, or other things. */
export function offerOptions(project: string, table: string, field: string, of: string[]): Promise<Collection> {
  return call('PATCH', at(project, table, field), { of });
}

/** Calls one thing a choice offers something else; rows that chose it say the new word. */
export function renameOption(project: string, table: string, field: string, from: string, to: string): Promise<Collection> {
  return call('PATCH', at(project, table, field), { rename: { from, to } });
}

/** Takes a column somebody added away, with what the rows hold there. */
export function removeColumn(project: string, table: string, field: string): Promise<Collection> {
  return call('DELETE', at(project, table, field));
}
