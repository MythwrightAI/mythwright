// What names a record: under a row, the rows of the tables whose records
// name it.
//
// A table says nothing of what is inside its rows. Another table does, by
// a field that names one of them: a quarter names its year. So a year is
// opened into its quarters, a quarter into its months, and so on down, for
// as long as something names what is shown. Nothing has to be declared for
// it but the fields that name, which are there already.
//
// The rows of a table that names are read the first time something is
// opened into them, and again when one is added, changed or taken away.

import { signal } from '@preact/signals';
import type { Signal } from '@preact/signals';
import { useEffect, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Collection } from '../api.ts';
import { coalesce } from '../events.ts';
import { fileOf, placeFor } from '../library.ts';
import { attempt, collections, openTab } from '../store.ts';
import { RENAMING, knownBy, namedBy, naming, renames, said, titleField } from '../table.ts';
import type { Row } from '../table.ts';
import { useEvents } from '../ui.tsx';

/** The rows of a table, kept for every place that is opened into them. `null` until they are read. */
interface Kept {
  rows: Signal<Row[] | null>;
  read: () => void;
}
const kept = new Map<string, Kept>();

export function rowsOf(project: string, name: string): Kept {
  const key = `${project}/${name}`;
  let one = kept.get(key);
  if (!one) {
    const rows = signal<Row[] | null>(null);
    one = { rows, read: coalesce(() => void api.records(project, name).then((read) => (rows.value = read), () => (rows.value = [])), 60) };
    kept.set(key, one);
    one.read();
  }
  return one;
}

/** Opens a record of any table: a document as its file, anything else as the table it is a row of. */
function open(project: string, table: Collection, row: Row): void {
  if (table.store === 'documents') openTab({ kind: 'file', project, path: fileOf(table, row) });
  else if (table.store === 'mockups') openTab({ kind: 'mockups', project, name: table.name });
  else openTab({ kind: 'table', project, name: table.name });
}

/** What one record of a table is called: `quarter`, of `quarters`. */
function one(name: string): string {
  return name.endsWith('s') && name.length > 1 ? name.slice(0, -1) : `one of ${name}`;
}

/**
 * Makes a record of `table` that names the record `id` by `field`, called
 * `title`: a quarter of this year, written from under the year. A document
 * is a file where its title says, as one made from its library is.
 */
async function add(project: string, table: Collection, field: string, id: string, title: string): Promise<Row | undefined> {
  const called = titleField(table.fields)?.name ?? 'title';
  const several = table.fields.find((each) => each.name === field)?.kind === 'records';
  const fields: Record<string, unknown> = { [called]: title, [field]: several ? [id] : id };
  const place = table.store === 'documents' && table.fields.some((each) => each.name === 'path') ? placeFor(title, '') : null;
  if (place) fields.path = place;
  return attempt(() => api.addRecord(project, table.name, fields));
}

/** Whether a row of the table `name` can be opened into anything. */
export function opensInto(project: string, name: string): boolean {
  return namedBy(collections.value[project] ?? [], name).length > 0;
}

/** The rows that name the record `id` of the table `name`, a table at a time, each opened the same way. */
export function Within({ project, name, id }: { project: string; name: string; id: string }) {
  const all = collections.value[project] ?? [];
  const by = namedBy(all, name);
  return (
    <div class="within">
      {by.map((one) => {
        const table = all.find((each) => each.name === one.table);
        return table ? <Naming key={`${one.table}.${one.field}`} project={project} table={table} field={one.field} id={id} alone={by.length === 1} /> : null;
      })}
    </div>
  );
}

function Naming({ project, table, field, id, alone }: { project: string; table: Collection; field: string; id: string; alone: boolean }) {
  const held = rowsOf(project, table.name);
  useEffect(() => held.read(), [held]);
  useEvents(RENAMING, (event) => renames(event, project, [table.name]) && held.read(), [project, table.name, held]);
  const [opened, setOpened] = useState<string[]>([]);
  // What a new one is to be called, while one is being written; `null` otherwise.
  const [writing, setWriting] = useState<string | null>(null);
  const rows = held.rows.value;
  if (rows === null) return <div class="within-none faint">Reading {table.name}</div>;
  const mine = naming(rows, field, id);
  const deeper = opensInto(project, table.name);
  // What is said of each beside what it is called: the day it is of, where it has one.
  const day = table.fields.find((each) => each.kind === 'date');
  return (
    <section class="within-table" aria-label={table.name}>
      {/* Named when it is one of several, or when there is nothing in it to say what it is. */}
      {(!alone || mine.length === 0) && (
        <div class="within-head faint">
          {mine.length === 0 ? `no ${table.name}` : table.name}
        </div>
      )}
      {mine.map((row) => {
        const words = knownBy(row, table.fields);
        const isOpen = opened.includes(row.id);
        return (
          <div key={row.id} class="within-row" data-row={row.id}>
            <div class="within-line">
              {deeper ? (
                <button
                  class="within-turn"
                  aria-expanded={isOpen}
                  aria-label={`${isOpen ? 'Close' : 'Open'} what is in ${words}`}
                  onClick={() => setOpened(isOpen ? opened.filter((one) => one !== row.id) : [...opened, row.id])}
                >
                  {isOpen ? '▾' : '▸'}
                </button>
              ) : (
                <span class="within-turn" aria-hidden="true" />
              )}
              <button class="within-name truncate" title={`Open this one of ${table.name}`} onClick={() => open(project, table, row)}>
                {words}
              </button>
              {day && said(row.data[day.name]) && <span class="faint">{said(row.data[day.name])}</span>}
              <span class="within-of faint">{table.name}</span>
            </div>
            {isOpen && <Within project={project} name={table.name} id={row.id} />}
          </div>
        );
      })}
      {writing === null ? (
        <button class="within-new" onClick={() => setWriting('')}>
          + New {one(table.name)}
        </button>
      ) : (
        <form
          class="within-line within-writing"
          onSubmit={async (event) => {
            event.preventDefault();
            const title = writing.trim();
            if (title === '') return setWriting(null);
            setWriting(null);
            if (await add(project, table, field, id, title)) held.read();
          }}
        >
          <span class="within-turn" aria-hidden="true" />
          <input
            ref={(input) => input?.focus()}
            value={writing}
            aria-label={`What the new ${one(table.name)} is called`}
            placeholder={`What the new ${one(table.name)} is called`}
            onInput={(event) => setWriting(event.currentTarget.value)}
            onKeyDown={(event) => event.key === 'Escape' && setWriting(null)}
            onBlur={() => writing.trim() === '' && setWriting(null)}
          />
          <button class="primary">Add</button>
        </form>
      )}
    </section>
  );
}
