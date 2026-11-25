// What a table's page hands the right of its head (`pagehead.tsx`): a
// table's own, a board's, a library's and a gallery of mockups alike,
// since each is a table underneath.
//
// When it was last changed is worked out from what the page read. What it
// cannot see is said to whoever points at it: a record that was taken
// away, which nothing that is still there says. A table kept private says
// so in a word, since nothing here shares or unshares one: its blueprint
// says who reads it. And the gear opens the instance whose blueprint
// declared the table, which is where its columns are said.
//
// A table and a board are written out as a CSV file, every record of
// them, and one is read back in (`csv.ts`, `csvdialog.tsx`): both are the
// browser's own doing, over the records the page read and the routes
// that add and change one. Reading one in is offered to whoever may
// change the table's records. A library's records are files, which are
// written where they are, and are neither.
//
// The tables that sit beside the table (`sits.ts`) are lines of the gear
// too, under a heading that says so: each by what it is called, with how
// many rows it has, opened in the drawer beside the page. Which lines
// there are is `besideLines` (`gear.ts`); nothing, where none sits beside
// it. After them is what the page hands of its own that it once had a
// button for (`tools`: what can be done to a board), and what one of
// those opens that is more than a line hangs under the gear (`hung`).

import type { ComponentChildren } from 'preact';
import { useRef, useState } from 'preact/hooks';
import type { Collection } from '../api.ts';
import { offered } from '../byhand.ts';
import { toCsv } from '../csv.ts';
import { besideLines } from '../gear.ts';
import { mayChoose } from '../icons.ts';
import { me, openTab, toast } from '../store.ts';
import type { Tab } from '../tab.ts';
import type { Row } from '../table.ts';
import { CsvImport } from './csvdialog.tsx';
import { glyph, PageActs, useEdited } from './pagehead.tsx';
import type { Lines } from './pagehead.tsx';
import { openAside } from './records.tsx';
import { whatIs } from './running.tsx';
import { signs } from './signs.tsx';
import { useBeside } from './sits.tsx';

/** What is not seen of a table's last change, for whoever asks. */
const BLIND = 'The newest change among its records and its views; a record that was taken away is not counted.';

interface Props {
  /** The page it is on. */
  tab: Tab;
  table: Collection;
  /** Its records as the page read them, every one of them; `null` until they are read. */
  rows: Row[] | null;
  /** What one record is called. */
  what?: string;
  share?: () => Lines | Promise<Lines>;
  /** The page's own lines of the gear's menu, before what every table has. */
  menu?: () => Lines;
  /** What can be done to the page's own kind of table, after the tables beside it: a board's tools. */
  tools?: () => Lines;
  /** What hangs under the gear while one of those has opened it there. */
  hung?: (gear: HTMLButtonElement | null) => ComponentChildren;
  /** The page has no button of its own after this: no line is begun for any. */
  last?: boolean;
  /**
   * Where the page reads its table a window at a time (`rowsread.ts`): every record read as it is written out or a
   * file is read in, as it is then, and how many there are to say beside the line.
   */
  every?: { read: () => Promise<Row[]>; count: number; again?: () => void };
}

/** A line of a menu that is a heading over those under it. */
const heading = (words: string) => ({ said: <span class="menu-heading">{words}</span>, apart: true });

/** Hands the browser a text to save as a file. */
function saveAs(name: string, text: string): void {
  const address = URL.createObjectURL(new Blob([text], { type: 'text/csv;charset=utf-8' }));
  const link = document.createElement('a');
  link.href = address;
  link.download = name;
  link.click();
  // Let go of once the browser has begun to save it.
  setTimeout(() => URL.revokeObjectURL(address), 10_000);
}

export function TableActs({ tab, table, rows, what = 'record', share, menu, tools, hung, last, every }: Props) {
  const edited = useEdited(table.project, table.name, table.updated, rows);
  // The tables that sit beside this one, each with its rows as they are when the gear is pressed.
  const sat = useBeside(table.project, table.name);
  // A table's rows and a board's cards are written out and read in; a library's documents and a gallery's mockups are files.
  const tabular = tab.kind === 'table' || tab.kind === 'board';
  const chosen = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<{ name: string; text: string; rows: Row[] } | null>(null);
  const read = async (event: Event) => {
    const input = event.currentTarget as HTMLInputElement;
    const picked = input.files?.[0];
    // The same file can be chosen again: what was chosen is let go of.
    input.value = '';
    if (!picked) return;
    try {
      const text = await picked.text();
      // What is in the table as the file is read against it.
      const now = every ? await every.read() : rows;
      if (now) setFile({ name: picked.name, text, rows: now });
    } catch {
      toast(`${picked.name} could not be read.`, true);
    }
  };
  // What declared it: asked about here, so that the menu says what it runs when it is opened.
  const declared = table.owner ? whatIs(table.owner) : 'gone';
  const lines = (): Lines => {
    const all = menu?.() ?? [];
    if (tabular && (rows || every)) {
      const many = every?.count ?? rows!.length;
      const count = `${many.toLocaleString()} ${many === 1 ? what : `${what}s`}`;
      const write = (all: Row[]) => saveAs(`${table.project}-${table.name}.csv`, toCsv(table.fields, all));
      all.push({ label: 'Export as CSV', mark: signs.down(), hint: count, apart: all.length > 0, act: () => (every ? void every.read().then(write, () => toast(`${table.name} could not be read to be written out.`, true)) : write(rows!)) });
      // Reading one in adds and changes records: offered to whoever may change them, as the daemon asks of each.
      // Not of a table whose rows are written only by what keeps it (`byhand.ts`): a file read in would make up rows.
      if (offered(table).readsIn && mayChoose(me.value?.principal.scopes ?? [], table)) all.push({ label: 'Import or merge a CSV…', mark: signs.up(), act: () => chosen.current?.click() });
    }
    if (table.owner && declared !== 'gone') {
      const owner = table.owner;
      all.push({ label: 'Its blueprint', mark: glyph('◈'), hint: declared?.blueprint, apart: all.length > 0, act: () => openTab({ kind: 'instance', id: owner }) });
    }
    return all;
  };
  // Each table by the mark a table has in the side panel, opened in the drawer beside this page.
  const beside = (): Lines => besideLines(sat).map((line) => ('says' in line ? heading(line.says) : { label: line.label, hint: line.hint, mark: glyph('▥'), act: () => openAside(table.project, line.does) }));
  const more = (): Lines => [...beside(), ...(tools?.() ?? [])];
  const quiet = table.reads ? { words: 'private', more: `Kept private: read only by whoever holds ${table.reads}. Its blueprint says so, and nothing here changes it.` } : null;
  return (
    <>
      <PageActs tab={tab} edited={edited} blind={BLIND} quiet={quiet} share={share} menu={lines} more={more} hung={hung} last={last} />
      {tabular && <input ref={chosen} type="file" accept=".csv,text/csv" hidden aria-label="A CSV file to import" onChange={(event) => void read(event)} />}
      {file && (
        <CsvImport
          table={table}
          rows={file.rows}
          board={tab.kind === 'board'}
          what={what}
          file={file}
          onClose={() => {
            setFile(null);
            // What the reader read in is theirs: the rows are read again, and not counted as having changed under them.
            every?.again?.();
          }}
        />
      )}
    </>
  );
}
