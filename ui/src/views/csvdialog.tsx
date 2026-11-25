// A CSV file read into a table: what it would do, said before anything is
// done, and then done a record at a time.
//
// What the file comes to is worked out here in the browser (`csv.ts`):
// which of its lines are records to add, which change a record there is,
// and which cannot be read, each with why. Nothing is written until
// somebody says so. Then each record is added or changed by itself,
// through the route that adds or changes one, as whoever is signed in:
// the daemon holds each to what a record of the table takes, and a line
// it refuses is said with its reason while the rest go on. Nothing is
// taken back where it stops part of the way: what was added is added,
// and how far it came is said.

import { useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Collection } from '../api.ts';
import { planOf, summed } from '../csv.ts';
import type { Plan } from '../csv.ts';
import { CARD } from '../table.ts';
import type { Row } from '../table.ts';
import { Modal } from '../ui.tsx';

/** How many lines that could not be read are said, before the rest are counted. */
const SAID = 6;

/** A card's own fields as its route takes them, and whatever else its board keeps of it under `fields`. */
function cardOf(fields: Record<string, unknown>): Record<string, unknown> {
  const own = Object.fromEntries(Object.entries(fields).filter(([name]) => CARD.includes(name)));
  const more = Object.fromEntries(Object.entries(fields).filter(([name]) => !CARD.includes(name)));
  return Object.keys(more).length > 0 ? { ...own, fields: more } : own;
}

interface Props {
  table: Collection;
  rows: Row[];
  /** The table is a board: its records are cards, which are added and changed as cards are. */
  board: boolean;
  /** What one record is called. */
  what: string;
  /** The file: what it is called, and what it says. */
  file: { name: string; text: string };
  onClose: () => void;
}

export function CsvImport({ table, rows, board, what, file, onClose }: Props) {
  // Worked out once, from the table as it was when the file was chosen.
  // A card that says no column is put in its board's first, by the board.
  const [plan] = useState<Plan>(() => planOf(table.fields, rows, file.text, board ? ['status'] : []));
  const [done, setDone] = useState<{ added: number; changed: number; refused: { line: number; why: string }[] } | null>(null);
  const [at, setAt] = useState<number | null>(null);
  const todo = plan.add.length + plan.change.length;
  const run = async () => {
    const came = { added: 0, changed: 0, refused: [] as { line: number; why: string }[] };
    const each = async (line: number, work: () => Promise<unknown>, count: 'added' | 'changed') => {
      setAt(came.added + came.changed + came.refused.length + 1);
      try {
        await work();
        came[count] += 1;
      } catch (err) {
        came.refused.push({ line, why: err instanceof Error ? err.message : String(err) });
      }
    };
    // One at a time, in the order of the file: each is a write of its own, and something may be following the table.
    for (const one of plan.change) {
      await each(one.line, () => (board ? api.changeItem(one.id, cardOf(one.fields)) : api.setRecord(one.id, one.fields)), 'changed');
    }
    for (const one of plan.add) {
      await each(
        one.line,
        async () => {
          if (!board) return api.addRecord(table.project, table.name, one.fields);
          // A card is added by what it is called and where it is, and is then given the rest.
          const { title, body, status, ...rest } = one.fields;
          const card = await api.addItem(table.project, { title: String(title ?? ''), ...(typeof body === 'string' ? { body } : {}), ...(typeof status === 'string' ? { status } : {}) });
          return Object.keys(rest).length > 0 ? api.changeItem(card.id, cardOf(rest)) : card;
        },
        'added',
      );
    }
    setAt(null);
    setDone(came);
  };
  const busy = at !== null;
  const lines = (all: { line: number; why: string }[]) => (
    <ul class="csv-lines">
      {all.slice(0, SAID).map((one) => (
        <li key={one.line}>
          Line {one.line}: {one.why}
        </li>
      ))}
      {all.length > SAID && <li>and {all.length - SAID} more</li>}
    </ul>
  );
  return (
    <Modal onClose={busy ? undefined : onClose}>
      <h2>
        Import {file.name} into {table.name}
      </h2>
      {done ? (
        <div class="stack" role="status">
          <p style="margin:0">
            {done.added} added, {done.changed} changed{done.refused.length > 0 ? `, ${done.refused.length} refused:` : '.'}
          </p>
          {done.refused.length > 0 && lines(done.refused)}
          <div class="row" style="justify-content:flex-end">
            <button class="primary" type="button" onClick={onClose}>
              Done
            </button>
          </div>
        </div>
      ) : (
        <div class="stack">
          <p style="margin:0" class="csv-plan">
            {summed(plan, what)}.
          </p>
          {plan.wrong.length > 0 && (
            <>
              <p style="margin:0">
                {plan.wrong.length === 1 ? 'One line cannot be read in, and is left out:' : `${plan.wrong.length} lines cannot be read in, and are left out:`}
              </p>
              {lines(plan.wrong)}
            </>
          )}
          {plan.left.length > 0 && (
            <p style="margin:0" class="faint">
              {plan.left.length === 1 ? 'A column that is' : 'Columns that are'} no field of this table {plan.left.length === 1 ? 'is' : 'are'} left out: {plan.left.join(', ')}.
            </p>
          )}
          <p style="margin:0" class="faint">
            A line with the id of a record here changes that record where it says something else; a line with no id is added. Each is written by itself, and what was written stays written if a later one is refused.
          </p>
          <div class="row" style="justify-content:flex-end">
            {busy && (
              <span class="faint" role="status">
                {at} of {todo}
              </span>
            )}
            <button type="button" disabled={busy} onClick={onClose}>
              Cancel
            </button>
            <button class="primary" type="button" disabled={busy || todo === 0} onClick={() => void run()}>
              {todo === 0 ? 'Nothing to import' : `Add ${plan.add.length} and change ${plan.change.length}`}
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
