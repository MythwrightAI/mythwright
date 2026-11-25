// The mark beside a column's name, on a board and on any table laid out in
// columns: one small drawing that stands for what is set up for
// the column (the rows of the tables beside it that are about it), with a
// dot on it where something there holds the column's cards until it is
// pressed. Pressed, or Enter or Space on it, it opens a menu under it, as
// a row's menu is (`RowMenu`): a heading for each table, each row under it
// by its name with a few faint words on what it is (where its table's rows
// say what the factory's bot does there, that rule as a sentence), pressed to open its
// form, and at its foot a line to add one of each such table for the
// column, for whoever may. What it offers is `columnset.ts`; a column with
// nothing about it has no mark. The column's head says only its name and
// its count beside it.

import { signal } from '@preact/signals';
import { useState } from 'preact/hooks';
import { api } from '../api.ts';
import { a, setUpOf } from '../columnset.ts';
import type { SetTable } from '../columnset.ts';
import type { Sat } from '../sits.ts';
import { attempt, collections, me } from '../store.ts';
import { titleField } from '../table.ts';
import { Modal, useFocus } from '../ui.tsx';
import { openRow } from './records.tsx';
import { offer, offeredBy, putAway } from './rowmenu.tsx';
import type { Offered, Said } from './rowmenu.tsx';
import { signs } from './signs.tsx';

/** A row being added for a column, until it is added or left. */
const adding = signal<{ project: string; table: string; one: string; about: string; column: string } | null>(null);

/** The mark, and the menu it opens: nothing where nothing is about the column. */
export function ColumnSetUp({ project, sat, column, columns }: { project: string; sat: Sat[]; column: string; columns?: string[] }) {
  const set = setUpOf(sat as Sat<SetTable>[], column, me.value?.principal.scopes ?? [], columns ?? null);
  if (set.groups.length === 0) return null;
  const count = set.groups.reduce((sum, group) => sum + group.rows.length, 0);
  const items = (): (Offered | Said)[] => [
    ...set.groups.flatMap((group, at): (Offered | Said)[] => [
      { said: <span class="menu-heading">{group.heading}</span>, apart: at > 0 },
      ...group.rows.map(
        (one): Offered => ({
          label: one.called,
          under: one.says || undefined,
          act: () => openRow(project, one.table, one.row),
        }),
      ),
    ]),
    ...set.adds.map(
      (add, at): Offered => ({
        label: `Add ${a(add.one)} for ${column}…`,
        mark: signs.plus(),
        apart: at === 0,
        act: () => {
          const table = sat.find((one) => one.table.name === add.table)?.table;
          if (table?.about) adding.value = { project, table: add.table, one: add.one, about: table.about, column };
        },
      }),
    ),
  ];
  const said = `What is set up for ${column}: ${count === 1 ? '1 row' : `${count} rows`}${set.holds ? ', and something holds its cards until it is pressed' : ''}`;
  return (
    <button
      type="button"
      class="ghost icon column-set"
      data-column={column}
      aria-haspopup="menu"
      aria-label={said}
      title={said}
      onClick={(event) => {
        event.stopPropagation();
        const from = event.currentTarget;
        if (offeredBy(from)) return putAway();
        const at = from.getBoundingClientRect();
        offer({ x: at.left, y: at.bottom + 2 }, items(), { from, label: `What is set up for ${column}`, wide: 300, fit: true });
      }}
    >
      {signs.setUp()}
      {set.holds && <span class="column-set-dot" aria-hidden="true" />}
    </button>
  );
}

/** What is asked before a row is added for a column, drawn once in the frame: what it is called; it is then opened to be filled in. */
export function ColumnAsked() {
  const asked = adding.value;
  return asked ? <AddRow key={`${asked.table}/${asked.column}`} asked={asked} onClose={() => (adding.value = null)} /> : null;
}

function AddRow({ asked, onClose }: { asked: NonNullable<typeof adding.value>; onClose: () => void }) {
  const [title, setTitle] = useState('');
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const what = asked.one === 'row' ? 'row' : asked.one;
  const submit = async (event: Event) => {
    event.preventDefault();
    if (!title.trim() || busy) return;
    const fields = (collections.value[asked.project] ?? []).find((one) => one.name === asked.table)?.fields ?? [];
    setBusy(true);
    const row = await attempt(() => api.addRecord(asked.project, asked.table, { [titleField(fields)?.name ?? 'title']: title.trim(), [asked.about]: asked.column }));
    setBusy(false);
    // Refused, it is said why, and what was written stays to be put right.
    if (!row) return;
    onClose();
    openRow(asked.project, asked.table, row);
  };
  return (
    <Modal onClose={onClose}>
      <h2>
        New {what} for {asked.column}
      </h2>
      <form class="stack" onSubmit={submit}>
        <label>
          What it is called
          <input ref={input} required value={title} onInput={(event) => setTitle(event.currentTarget.value)} />
        </label>
        <p class="faint">It is added to {asked.table} about {asked.column}, and opened to be filled in.</p>
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || !title.trim()}>
            Add
          </button>
        </div>
      </form>
    </Modal>
  );
}
