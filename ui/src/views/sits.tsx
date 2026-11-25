// What sits beside a table, drawn where that table is.
//
// A table may say it sits beside another (`sits.ts`). It is then no line
// of its own under its project: the page of the table it sits beside
// lists what sits beside it under Its tables, each by what it is called
// and how many rows it has, and each is opened in the drawer as the table
// it is, beside the page. On a page that is a tab the list is lines of
// the gear in its head (`tablehead.tsx`, `gear.ts`); a table shown in the
// drawer has no gear, and has one button there for it (`Beside`). And
// where the page lays its records out in columns, a column's head says
// the rows that are about that column, each by what it is called, pressed
// to open that row's form.
//
// Nothing here knows what any such table is of. The rows are read once
// for every place that draws them and again when one is added, changed
// or taken away, so a head says a row the moment it is about its column.

import { useEffect, useRef, useState } from 'preact/hooks';
import type { Collection } from '../api.ts';
import { headed } from '../gear.ts';
import { besideOf, counted } from '../sits.ts';
import type { Sat } from '../sits.ts';
import { collections } from '../store.ts';
import { RENAMING, renames } from '../table.ts';
import { useEvents } from '../ui.tsx';
import { openAside } from './records.tsx';
import { rowsOf } from './within.tsx';

/** The tables that sit beside the table `name` of a project, each with its rows as they are now. */
export function useBeside(project: string, name: string): Sat<Collection>[] {
  const tables = besideOf(name, collections.value[project] ?? []);
  const names = tables.map((one) => one.name).join(' ');
  const held = tables.map((table) => rowsOf(project, table.name));
  const read = () => held.forEach((one) => one.read());
  useEffect(read, [project, names]);
  useEvents(RENAMING, (event) => names !== '' && renames(event, project, names.split(' ')) && read(), [project, names]);
  return tables.map((table, index) => ({ table, rows: held[index]!.rows.value }));
}

/**
 * What goes in the head of a table shown in the drawer for the tables
 * that sit beside it: one button, and under it each of them. Nothing,
 * where none does. A page that is a tab says the same in its gear.
 */
export function Beside({ project, name, what = 'table' }: { project: string; name: string; what?: string }) {
  const sat = useBeside(project, name);
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const away = (event: Event) => {
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !box.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', away);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', away);
    };
  }, [open]);
  if (sat.length === 0) return null;
  return (
    <div class="menu-anchor" ref={box}>
      <button aria-haspopup="menu" aria-expanded={open} title={`The tables kept beside this ${what}, each opened here beside it`} onClick={() => setOpen(!open)}>
        Its tables <span class="faint">▾</span>
      </button>
      {open && (
        <div class="menu" role="menu" aria-label={`The tables beside this ${what}`}>
          {sat.map(({ table, rows }) => (
            <button
              key={table.name}
              role="menuitem"
              onClick={() => {
                setOpen(false);
                openAside(project, table.name);
              }}
            >
              <strong>{table.title_chosen || table.title || headed(table.name)}</strong>
              <span class="muted">{counted(table, rows) || 'being read'}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
