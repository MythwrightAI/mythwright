// What a card's Details says of the rows that are about it: of
// each table that sits beside its board and whose rows are each about a
// card (`aboutCards`, the tables a card has a tab for), the row about this
// card, what it says under what each thing is called, an address a link,
// with a way to open it whole. A package's table of each piece of work's
// branch and pull request is one; nothing here knows it. The same rows
// are the table's own tab beside Details, with what names them.
//
// The rows are read once for every place that draws them and again when
// one is added, changed or taken away (`rowsOf`), so a row written from a
// terminal or by a blueprint is said with nothing read again.

import { useEffect } from 'preact/hooks';
import { aboutCards, cellsOf, rowsAbout, tabOf } from '../cardtabs.ts';
import { collections, drawer } from '../store.ts';
import { RENAMING, knownBy, renames } from '../table.ts';
import type { Row } from '../table.ts';
import { useEvents } from '../ui.tsx';
import { Cells, useBranches } from './cardtabs.tsx';
import { openRow } from './records.tsx';
import { rowsOf } from './within.tsx';

export function CardRows({ project, card }: { project: string; card: string }) {
  const tables = aboutCards('board', collections.value[project] ?? []);
  const names = tables.map((table) => table.name);
  const held = names.map((name) => rowsOf(project, name));
  const read = () => held.forEach((one) => one.read());
  useEffect(read, [project, names.join(' ')]);
  useEvents(RENAMING, (event) => names.length > 0 && renames(event, project, names) && read(), [project, names.join(' ')]);
  const branches = useBranches(project);
  const kept = tables.flatMap((table, at) => rowsAbout(table, held[at]!.rows.value ?? [], card).map((row) => ({ table, row })));
  if (kept.length === 0) return null;
  // Opened whole in the drawer, with the way back to the card.
  const openWhole = (name: string, row: Row) => {
    const was = drawer.value;
    openRow(project, name, row, was ? () => (drawer.value = was) : undefined);
  };
  return (
    <div class="card-rows">
      {kept.map(({ table, row }) => (
        <section key={row.id} class="card-row about-card-row" data-table={table.name} data-row={row.id} aria-label={tabOf(table)}>
          <div class="about-card-head">
            <h3 class="truncate">{tabOf(table)}</h3>
            <span class="faint truncate">{knownBy(row, table.fields)}</span>
            <span class="grow" />
            <button type="button" onClick={() => openWhole(table.name, row)} title={`Open this row of ${tabOf(table)} whole`}>
              Open
            </button>
          </div>
          <Cells cells={cellsOf(row, table.fields, table.about ? [table.about] : [])} branches={branches} />
        </section>
      ))}
    </div>
  );
}
