// The views of a table that is not a board: the same tabs a board has, the
// same ways of laying its records out, kept with the table the same way.
//
// A page of a table (a library's documents, a fleet's agents, anything a
// blueprint declared) has a look of its own, which is the first of its
// views. Whoever looks at it can save others beside it: the same records
// as a table, a list, a gallery, on the days of a calendar, or in columns
// by a choice. A view is the table's (`collection.ensure` declares some,
// people save the rest), so it is there for whoever opens the page next.
// Which one is being looked at is the reader's, and is kept in the browser.

import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Collection } from '../api.ts';
import { InTurn } from '../inturn.ts';
import { attempt, tablesChangedHere } from '../store.ts';
import { groupable, holdable, shown as shownOf } from '../ordered.ts';
import type { Only } from '../ordered.ts';
import { listed, steps } from '../columns.ts';
import { banded } from '../groups.ts';
import { changed, columns as columnsOf, goes, layouts as layoutsOf, offered, leavingOut, leftOut, leftOutAtFirst, nameFor, said, titleField } from '../table.ts';
import type { Change, Field, Layout, Row, View } from '../table.ts';
import { classes } from '../util.ts';
import { arrangementOf } from '../slid.ts';
import { useCardSlide } from './cardslide.ts';
import { Held } from './held.tsx';
import { Bands, OrderLayout } from './ordered.tsx';
import { Beside, useBeside } from './sits.tsx';
import { ColumnSetUp } from './columnset.tsx';
import type { Heads } from './headmenu.tsx';
import { LayoutPanel, ViewTabs } from './layouts.tsx';
import type { LayoutMenuProps } from './layouts.tsx';
import { CalendarLayout, GalleryLayout, ListLayout } from './table.tsx';
import { TableLayout } from './windowed.tsx';
import type { Reads } from './windowed.tsx';
import type { Rows } from './table.tsx';
import { showView, viewShown } from '../viewshown.ts';

/** A choice of the reader's that is kept in this browser. */
function kept(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function keep(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* not remembered */
  }
}

/** The views of one table, and which is being looked at. */
export interface Views {
  ways: Layout[];
  all: View[];
  view: View;
  layout: Layout['kind'];
  /** What the view goes by: for columns a choice, for a calendar a day, for an order the field that says what comes before. */
  by: string | null;
  /** What its records are gathered by, for columns and an order; and for an order the choice that says one is done. */
  group: string | null;
  done: string | null;
  /** What it is held to, where it is held to anything; the table's fields, to say so by; and what holds it to more or less. */
  only: Only | null;
  fields: Field[];
  hold: (only: Only | null) => void;
  /** Where what a reader folds in it is kept: the project, the table and the view. */
  at: string;
  /** Which table it is a view of. */
  of: { project: string; name: string };
  /** The fields the view leaves out, by name. */
  hides: string[];
  lines: boolean;
  wrap: boolean;
  /** What the heads of its columns go by, when it is laid out as a table: all but the records, which whoever draws it has. */
  heads: Omit<Heads, 'rows'>;
  look: (name: string) => void;
  add: (kind: Layout['kind']) => void;
  menu: LayoutMenuProps;
}

/**
 * The views of `table`, where `first` is how its page looks when its
 * blueprint declared none. Saved with the table; chosen in this browser.
 */
export function useViews(project: string, given: Collection | undefined, name: string, first: Layout['kind']): Views {
  // A page asks before it knows there is such a table: then there is nothing to lay out.
  const table: Collection = given ?? { project, name, fields: [], views: [], store: 'local', owner: null };
  // Which view is looked at is said by the reader, and by an address opened at one (`viewshown.ts`).
  const viewing = viewShown(project, table.name);
  // The table as a view was last saved of it here, until it is read again.
  const [saved, setSaved] = useState<Collection | null>(null);
  // The table as it was last handed here or a change to its views last
  // left it, which may be a moment ahead of what is drawn: a change to a
  // view is worked out from this, in its turn (`inturn.ts`).
  const turns = useRef(new InTurn<Collection>(table)).current;
  useEffect(() => {
    setSaved(null);
    turns.read(table);
  }, [given]);
  const [lines, setLines] = useState(() => kept('client.table.lines') !== 'no');
  const [wrap, setWrap] = useState(() => kept('client.table.wrap') === 'yes');
  const now = saved ?? table;
  const ways = layoutsOf(now.fields, now.views);
  // The views that are offered: an order is not, for now (`offered`).
  const here = offered(now.views);
  const all: View[] = here.length > 0 ? here : [{ name: now.name, kind: first, by: null }];
  const view = all.find((one) => one.name === viewing) ?? all[0]!;
  const layout = (ways.find((way) => way.kind === view.kind) ?? ways[0]!).kind;
  const looking = useRef(viewing);
  const seen = useRef(viewing);
  if (seen.current !== viewing) looking.current = seen.current = viewing;
  const look = (name: string) => {
    looking.current = seen.current = name;
    showView(project, table.name, name);
  };
  // What the view being looked at leaves out goes with it when it is laid out another way or called something else.
  const hides = leftOut(view, now.fields);
  // Each change is made in its turn, from the views as the change before
  // it left them and not as they are drawn here: two columns switched in
  // the time one save takes were each sent without the other.
  const viewsOf = (of: Collection): View[] => (of.views.length > 0 ? of.views : [{ name: of.name, kind: first, by: null }]);
  const save = (change: Change) =>
    turns.change(async (of) => {
      const next = changed(of.fields, viewsOf(of), looking.current, change);
      const done = await attempt(() => api.saveView(project, of.name, next));
      if (done) {
        tablesChangedHere(project);
        setSaved(done);
        look(next.name);
      }
      return done;
    });
  const remove = () =>
    void turns.change(async (of) => {
      const gone = viewsOf(of).find((one) => one.name === looking.current) ?? viewsOf(of)[0]!;
      const left = await attempt(() => api.removeView(project, of.name, gone.name));
      if (left) {
        tablesChangedHere(project);
        setSaved(left);
        look(left.views[0]?.name ?? of.name);
      }
      return left;
    });
  return {
    ways,
    all,
    view,
    layout,
    ...goes(layout, now.fields, view, all),
    at: `${project}.${now.name}.${view.name}`,
    of: { project, name: now.name },
    fields: now.fields,
    hold: (only) => save((view) => ({ name: view.name, kind: view.kind, only })),
    hides,
    lines,
    wrap,
    heads: { view, fields: columnsOf(now.fields), groups: now.groups, name: now.name, owner: now.owner, at: `${project}.${now.name}.${view.name}`, change: save },
    look,
    // A new view leaves out what its table says is left out to begin with, whatever the one being looked at shows.
    add: (kind) => save((_, fields, all) => ({ name: nameFor(kind, all.map((one) => one.name)), kind, hides: leftOutAtFirst(fields) })),
    menu: {
      layouts: ways,
      layout,
      name: view.name,
      onLayout: (kind) => save((view) => ({ name: view.name, kind })),
      onRename: (name) => save((view) => ({ name, kind: view.kind, was: view.name })),
      onRemove: here.length > 1 ? remove : undefined,
      options: [
        ['Show vertical lines', lines, (on) => (setLines(on), keep('client.table.lines', on ? 'yes' : 'no'))],
        ['Wrap all content', wrap, (on) => (setWrap(on), keep('client.table.wrap', on ? 'yes' : 'no'))],
      ],
      columns: listed({ fields: columnsOf(now.fields), groups: now.groups, order: view.columns }, hides),
      onMove: (name, by) => save(steps(now.groups, name, by, true)),
      onColumn: (name, shown) => save((view, fields) => ({ name: view.name, kind: view.kind, hides: leavingOut(view, fields, name, shown) })),
      groups: groupable(now.fields, now.name, layout === 'board' ? goes(layout, now.fields, view, all).by : null, goes(layout, now.fields, view, all).group),
      group: goes(layout, now.fields, view, all).group,
      onGroup: (name) => save((view) => ({ name: view.name, kind: view.kind, group: name })),
    },
  };
}

/**
 * What goes in a page's head for its views: how this one is laid out, and a tab for each. In the drawer (`aside`),
 * which has no gear, also the button for the tables that sit beside this one; a page that is a tab says those in its gear.
 */
export function ViewsHead({ views, rows, counted, what = 'record', aside }: { views: Views; rows?: Row[] | null; counted?: { count: number; of: number } | null; what?: string; aside?: boolean }) {
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
  return (
    <>
      {aside && <Beside project={views.of.project} name={views.of.name} />}
      <div class="menu-anchor" ref={box}>
        <button aria-haspopup="dialog" aria-expanded={open} title="Another way to lay this view out, or another name for it" onClick={() => setOpen(!open)}>
          Layout <span class="faint">▾</span>
        </button>
        {open && (
          <LayoutPanel
            {...views.menu}
            holdings={holdable(views.fields, views.layout === 'board' ? views.by : null, rows ?? [], views.only)}
            only={views.only}
            onOnly={views.hold}
            onClose={() => setOpen(false)}
          />
        )}
      </div>
      <ViewTabs views={views.all} current={views.view.name} ways={views.ways} onSelect={views.look} onAdd={views.add} />
      <Held
        only={views.only}
        fields={views.fields}
        what={what}
        // Counted by the daemon where it reads the table a window at a time.
        shown={counted && views.layout === 'table' ? counted.count : rows ? shownOf(rows, views.only, views.fields).length : undefined}
        of={counted?.of ?? rows?.length}
        onClear={() => views.hold(null)}
      />
    </>
  );
}

interface LaidProps extends Rows {
  views: Views;
  /** The page's own way of drawing a layout, where it has one: a library its list, by folder. */
  own?: Partial<Record<Layout['kind'], () => ComponentChildren>>;
  /** A new row is being written in the table, asked for from outside it. */
  adding?: boolean;
  /** Changes a field of a record. Not given where the records are not changed here. */
  onChange?: (row: Row, field: Field, value: unknown) => void;
  onAdd?: (title: string | null) => void;
  /** `false` where its rows are not written by hand (`byhand.ts`): read, and nothing added or changed where they are laid out. */
  byHand?: boolean;
  /** Where the page reads the table a window at a time, laid out as a table (`rowsread.ts`). */
  reads?: Reads;
}

/** A table's records, laid out as the view being looked at says. */
export function Laid({ views, own, adding, onChange, onAdd, byHand, reads, ...all }: LaidProps) {
  const { layout, by } = views;
  // What the view is held to is all it shows, however it is laid out; what a record names is still found among all of them.
  const rows = { ...all, rows: shownOf(all.rows, views.only, all.fields) };
  const drawn = own?.[layout];
  if (drawn) return <>{drawn()}</>;
  const unchanged = () => undefined;
  // What the view leaves out is not drawn as a column, nor as a line of a list or a card of a gallery. A row is still opened to the whole of it.
  const shown = { ...rows, fields: rows.fields.filter((field) => !views.hides.includes(field.name)) };
  if (layout === 'table') {
    return <TableLayout {...shown} of={views.of} heads={{ ...views.heads, rows: all.rows, named: all.named }} lines={views.lines} wrap={views.wrap} adding={adding ?? false} onChange={onChange ?? unchanged} onAdd={onAdd ?? unchanged} byHand={byHand} reads={reads} />;
  }
  // A list and a gallery say a record's fields side by side in the order the view's columns stand in.
  shown.fields = banded(shown.fields, all.groups, views.view.columns).flatMap((band) => band.fields);
  // Only a table has a row to write a new record in: elsewhere it is asked for above what is laid out.
  const writing = adding && onAdd ? <NewRow what={rows.what} onDone={onAdd} /> : null;
  if (layout === 'calendar') {
    const day = by ?? 'created';
    const dated = rows.fields.find((field) => field.name === day && field.kind === 'date');
    return (
      <>
        {writing && <div class="rows-page">{writing}</div>}
        <CalendarLayout {...rows} by={day} onMove={dated && onChange ? (row, to) => onChange(row, dated, to) : undefined} />
      </>
    );
  }
  const named = rows.named ?? {};
  if (layout === 'order') {
    return (
      <>
        {writing && <div class="rows-page">{writing}</div>}
        <OrderLayout fields={rows.fields} rows={rows.rows} every={all.rows} named={named} by={by} doneBy={views.done} group={views.group} at={views.at} what={rows.what} onOpen={rows.onOpen} />
      </>
    );
  }
  const choice = layout === 'board' ? rows.fields.find((field) => field.name === by && field.kind === 'choice') : undefined;
  if (choice) {
    return (
      <>
        {writing && <div class="rows-page">{writing}</div>}
        <Bands
          rows={rows.rows}
          by={rows.fields.find((field) => field.name === views.group)}
          named={named}
          at={views.at}
          each={(only) => <ColumnsLayout {...rows} rows={only ?? rows.rows} by={choice} onChange={onChange} of={views.of} />}
        />
      </>
    );
  }
  return (
    <div class="rows-page">
      {writing}
      {layout === 'gallery' ? <GalleryLayout {...shown} /> : <ListLayout {...shown} />}
    </div>
  );
}

/** What a new record is called, written where there is no table to write it in. */
function NewRow({ what, onDone }: { what: string; onDone: (title: string | null) => void }) {
  const [title, setTitle] = useState('');
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => input.current?.focus(), []);
  return (
    <form
      class="row"
      onSubmit={(event) => {
        event.preventDefault();
        onDone(title.trim() || null);
      }}
    >
      <input
        ref={input}
        class="grow"
        value={title}
        aria-label={`What the new ${what} is called`}
        placeholder={`What the new ${what} is called`}
        onInput={(event) => setTitle(event.currentTarget.value)}
        onKeyDown={(event) => event.key === 'Escape' && onDone(null)}
      />
      <button class="primary" disabled={!title.trim()}>
        Add
      </button>
      <button type="button" onClick={() => onDone(null)}>
        Cancel
      </button>
    </form>
  );
}

/**
 * The records in columns by a choice: a column for each thing that can be
 * chosen, and one for those with none. Carried to another column, a record
 * is given that choice.
 */
function ColumnsLayout({ fields, rows, by, onOpen, onChange, of }: Rows & { by: Field; onChange?: (row: Row, field: Field, value: unknown) => void; of: { project: string; name: string } }) {
  // What sits beside this table, whose rows a column's head says where they are about the column.
  const sat = useBeside(of.project, of.name);
  const [carried, setCarried] = useState<string | null>(null);
  const [over, setOver] = useState<string | null>(null);
  const called = titleField(fields);
  const chosen = (row: Row) => (typeof row.data[by.name] === 'string' ? (row.data[by.name] as string) : '');
  const columns = [...by.of, ...(rows.some((row) => !by.of.includes(chosen(row))) ? [''] : [])];
  const others = fields.filter((field) => field !== called && field !== by).slice(0, 2);
  const inColumn = (column: string) => rows.filter((row) => (by.of.includes(chosen(row)) ? chosen(row) : '') === column);
  // A record that comes to another column slides there, as a card on the board does (`cardslide.ts`); not while one is carried, nor the one let go by hand.
  const slid = useRef<HTMLDivElement>(null);
  const byHand = useRef<string | null>(null);
  useCardSlide(slid, arrangementOf(columns.map((column) => [column, inColumn(column).map((row) => row.id)])), carried !== null, byHand);
  return (
    <div class="board" ref={slid}>
      {columns.map((column) => {
        const here = inColumn(column);
        return (
          <section
            key={column}
            class={classes('column', over === column && 'over')}
            aria-label={column || `No ${by.name}`}
            onDragOver={(event) => {
              if (!carried || !onChange) return;
              event.preventDefault();
              if (over !== column) setOver(column);
            }}
            onDragLeave={(event) => {
              if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOver(null);
            }}
            onDrop={(event) => {
              event.preventDefault();
              const row = rows.find((one) => one.id === carried);
              setCarried(null);
              setOver(null);
              if (row && onChange && chosen(row) !== column) {
                byHand.current = row.id;
                onChange(row, by, column === '' ? null : column);
              }
            }}
          >
            <div class="column-head">
              <span class="grow column-name">
                <span>{column || <span class="faint">none</span>}</span>
                <ColumnSetUp project={of.project} sat={sat} column={column} columns={columns} />
              </span>
              <span class="faint">{here.length}</span>
            </div>
            <div class="cards">
              {here.map((row) => (
                <article
                  key={row.id}
                  data-slides={row.id}
                  class={classes('card', carried === row.id && 'dragging')}
                  draggable={onChange !== undefined}
                  tabIndex={0}
                  onDragStart={(event) => {
                    event.dataTransfer?.setData('text/plain', row.id);
                    if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
                    setCarried(row.id);
                  }}
                  onDragEnd={() => {
                    setCarried(null);
                    setOver(null);
                  }}
                  onClick={() => onOpen(row)}
                  onKeyDown={(event) => event.key === 'Enter' && onOpen(row)}
                >
                  <strong class="truncate">{String((called && row.data[called.name]) ?? row.id)}</strong>
                  {others.map((field) => {
                    const words = said(row.data[field.name]);
                    return words ? (
                      <div key={field.name} class="faint truncate">
                        {words}
                      </div>
                    ) : null;
                  })}
                </article>
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}
