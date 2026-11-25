// A table laid out as a table: its heads, its rows and its foot.
//
// Where the page reads its table a window at a time (`reads`, a table's
// own page and the drawer: `rowsread.ts`), the daemon sorts it, holds it
// to what the view is held to, gathers it into bands and counts it, and
// what is drawn is the rows on the screen and so many either side of
// them, with room above and below as tall as the rest would be
// (`window.ts`): the scrollbar is as long as every row the view shows,
// past a few million pixels a page of them at a time. What stands at the
// top of the screen stays there as rows above it are measured and read
// in. Where the page has its rows in hand (the board's table), they are
// all drawn, sorted and gathered here, as they always were.
//
// Nothing here knows what the records are of. A column is a field, a
// cell is drawn and changed by the kind its field is (`table.tsx`), and
// what changing one does is whoever uses this to say.

import { Fragment } from 'preact';
import type { ComponentChildren } from 'preact';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { frozen } from '../columns.ts';
import { called as figureCalled, figure, fits, meaning, written } from '../figures.ts';
import { banded, headed, headings } from '../groups.ts';
import { bandCalled } from '../ordered.ts';
import type { Reading } from '../rowsread.ts';
import { isScopes } from '../scopes.ts';
import { heading, knownBy, said, sorted, titleField } from '../table.ts';
import type { Field, Row, Sort } from '../table.ts';
import { classes } from '../util.ts';
import { anchorAt, drawn, keyOf, measured, paged, roomFor, rowAt, shifted, totalOf, yOf } from '../window.ts';
import { markSaid, said as liveSaid } from '../rowslive.ts';
import type { Mark } from '../rowslive.ts';
import type { Odd, Segment } from '../window.ts';
import { sizedStyle, widthOf } from '../widths.ts';
import { useStaying } from './frozen.ts';
import { ColumnHead, useSort } from './headmenu.tsx';
import type { Heads } from './headmenu.tsx';
import { useChoosing } from './iconpicker.tsx';
import type { Choosing } from './iconpicker.tsx';
import { BandHead, useBands, useCalled } from './ordered.tsx';
import { useKept } from './returned.tsx';
import { ScopesEdit, ScopesSaid } from './scopes.tsx';
import { AddHead, useShaping } from './shaping.tsx';
import { Editing, Shown } from './table.tsx';
import type { Rows } from './table.tsx';
import { WidthEdge } from './widths.tsx';

/** How the page reads the table a window at a time: what it has read, and the reader's sort, which it reads by. */
export interface Reads {
  reading: Reading;
  sort: Sort | null;
  setSort: (next: Sort | null) => void;
  /** Bands start folded, and each is read once it is opened: a table past `SMALL` rows. */
  large: boolean;
}

interface TableProps extends Rows {
  lines: boolean;
  wrap: boolean;
  /** A new row is being written, asked for from outside the table. */
  adding: boolean;
  onChange: (row: Row, field: Field, value: unknown) => void;
  /** A new record with this title, or none when the writing was given up. */
  onAdd: (title: string | null) => void;
  /** Which table this is, so that whoever may can choose a column's icon here and change the view from a column's head. Not said, the icons are drawn and none is chosen. */
  of?: Choosing;
  /** What the heads of its columns go by: the view being looked at and what changes it. Not said, a head sorts and does nothing else. */
  heads?: Heads;
  /** `false` where its rows are not written by hand (`byhand.ts`): no cell is changed where it is and no row is added here. */
  byHand?: boolean;
  /** Where the rows are read a window at a time; not said, `rows` are every row and are all drawn. */
  reads?: Reads;
}

/** A choice of the reader's that is kept in this browser. */
function keptHere(key: string): string[] {
  try {
    const read: unknown = JSON.parse(localStorage.getItem(key) ?? '[]');
    return Array.isArray(read) ? read.map(String) : [];
  } catch {
    return [];
  }
}
function keepHere(key: string, value: string[]): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* not remembered */
  }
}

/**
 * Which bands the reader folded, kept in their browser: as `useBands`
 * keeps them where bands start open, and which they opened where every
 * band starts folded (a table past `SMALL` rows, whose bands are read as
 * each is opened).
 */
function useFolds(at: string, startsFolded: boolean): { shut: (band: string) => boolean; fold: (band: string) => void } {
  const key = startsFolded ? `client.bands.opened.${at}` : `client.bands.${at}`;
  const [turned, setTurned] = useState<{ key: string; list: string[] }>(() => ({ key, list: keptHere(key) }));
  const list = turned.key === key ? turned.list : keptHere(key);
  return {
    shut: (band) => list.includes(band) !== startsFolded,
    fold: (band) => {
      const next = list.includes(band) ? list.filter((one) => one !== band) : [...list, band];
      setTurned({ key, list: next });
      keepHere(key, next);
    },
  };
}

/** How tall a row and a band's head are taken to be before one was drawn. */
const FIRST = 34;

/**
 * A row a record, a column a field. A cell is pressed to change it, and a
 * column's head for what can be done with the column (`headmenu.tsx`).
 * The view being looked at says the order the columns stand in, which of
 * them stay while the rest are scrolled sideways, what the rows are
 * gathered by and the figure at the foot of a column; the reader's own
 * sort is kept in their browser.
 */
export function TableLayout(props: TableProps) {
  const { people, what, heads, reads } = props;
  const view = heads?.view;
  // The columns under their headings, where the table says groups: those in none first, and in each the order the view says.
  const bands = banded(props.fields, props.groups, view?.columns);
  const fields = bands.flatMap((band) => band.fields);
  const drawnColumns = fields.map((field) => field.name);
  // Where a group begins, which is marked down the table.
  const begins = new Set(bands.slice(1).map((band) => band.fields[0]!.name));
  const [ownSort, setOwnSort] = useSort(heads?.at);
  const [sort, setSort] = reads ? [reads.sort, reads.setSort] : [ownSort, setOwnSort];
  const [editing, setEditing] = useState<{ row: string; field: string; band: string } | null>(null);
  const [writing, setWriting] = useState(false);
  // The rows that are opened into what names them.
  const [opened, setOpened] = useState<string[]>([]);
  const title = titleField(fields);
  // A table that says what its records are called by (`calls`: an attempt by its card and contender) has them called
  // so in the column of their title too, and never by the title, which may be what something finds one by.
  const calledBy = heads?.fields ?? props.fields;
  const calls = title?.name === 'title' && calledBy.some((field) => (field.calls ?? 0) > 0);
  const shownIn = (row: Row, field: Field, value: unknown) =>
    field === title && calls ? <span class="cell-called">{knownBy(row, calledBy)}</span> : <Shown field={field} value={value} people={people} others={props.named} />;
  // A table whose rows are written only by what keeps it is read here: its cells are words, and nothing adds a row.
  const hand = props.byHand !== false;
  const adding = hand && (props.adding || writing);
  const choosing = useChoosing(props.of);
  // Whoever may add a column has a place for it after the last head, and every row a cell under that.
  const adds = useShaping(props.of) && heads !== undefined && props.of !== undefined;
  const wide = fields.length + (adds ? 1 : 0);
  // The columns that stay while the rest are scrolled sideways, each told where it begins.
  const { table, stay, left } = useStaying(frozen(drawnColumns, view?.freezes), drawnColumns.join(' '));
  // How wide each column is where the view says (`widths.ts`): its cells say so, by the size of the reader's text.
  const widths = fields.map((field) => widthOf(view, field.name));
  // Read a window at a time, a column the view says nothing of is as wide as the widest it has been drawn: the rows drawn change as it is scrolled.
  const [least, setLeast] = useKept<Record<string, number>>('rows.widths', {});
  const stays = (nth: number, more?: string | false) => classes(more, nth < stay && 'frozen', nth === stay - 1 && 'frozen-last');
  const from = (nth: number) => (nth < stay ? `left:calc(${left[nth] ?? 0}px - var(--side, 0px))` : undefined);
  // A column's own cells: where it begins if it stays, and how wide it is; a heading over several is as wide as they are.
  const cell = (nth: number, more?: string | false) => classes(stays(nth, more), widths[nth] !== null && 'sized');
  const put = (nth: number) => [from(nth), sizedStyle(widths[nth] ?? null)].filter(Boolean).join(';') || undefined;
  const headPut = (nth: number) => {
    const most = widths[nth] === null ? least[fields[nth]!.name] : undefined;
    return [put(nth), most ? `min-width:${most}px` : null].filter(Boolean).join(';') || undefined;
  };
  // Whoever may save a view drags a column's edge to make it wider or narrower.
  const resizes = choosing !== null && heads !== undefined;
  // The rows under a line for each thing the field they are gathered by says, where the view says one.
  const by = view?.group ? heads?.fields.find((field) => field.name === view.group) : undefined;
  const gathered = useBands(reads ? [] : props.rows, reads ? undefined : by, props.named ?? {}, heads?.at ?? '');
  // The figures at the feet of the columns, where the view says any of a column that is drawn.
  const figures = fields.map((field) => ({ field, which: view?.figures?.[field.name] })).filter((one) => one.which && fits(one.field.kind, one.which));

  const same = (a: unknown, b: unknown) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
  const change = (row: Row, field: Field, next: unknown) => {
    setEditing(null);
    if (next === undefined || same(next, row.data[field.name])) return;
    // What a record must have is not taken from it by leaving a cell empty.
    if (field.required && said(next).trim() === '') return;
    props.onChange(row, field, next);
  };
  /** One row. A record that holds several of what the rows are gathered by is drawn under each, and is one record all the same. */
  const line = (row: Row, band: string, rank?: number, kept?: string, mark?: Mark, marked?: boolean) => {
    const isOpen = props.within !== undefined && opened.includes(row.id);
    const called = said(title && row.data[title.name]) || row.id;
    return (
      <Fragment key={`${band} ${row.id}`}>
        {/* A row that changed is drawn where it is: marked where it would now stand elsewhere, struck where the view no longer shows it. */}
        <tr data-row={row.id} data-rank={rank} data-band={kept} data-live={mark} class={classes(mark && `grid-${mark === 'moved' ? 'moved' : 'struck'}`, marked && 'grid-gone-to')} title={mark ? markSaid(mark) : undefined}>
          {fields.map((field, nth) => {
            const value = row.data[field.name];
            const here = editing?.row === row.id && editing.field === field.name && editing.band === band;
            return (
              <td key={field.name} style={put(nth)} class={cell(nth, classes(field === title && 'grid-title', field === title && props.within && 'grid-turns', here && 'editing', begins.has(field.name) && 'band-first'))}>
                {field === title && props.within && (
                  <button
                    class="grid-turn"
                    aria-expanded={isOpen}
                    aria-label={`${isOpen ? 'Close' : 'Open'} what is in ${called}`}
                    title="What names this, in the other tables"
                    onClick={() => setOpened(isOpen ? opened.filter((one) => one !== row.id) : [...opened, row.id])}
                  >
                    {isOpen ? '▾' : '▸'}
                  </button>
                )}
                {field.kind === 'flag' ? (
                  <label class="cell">
                    <input type="checkbox" aria-label={heading(field)} checked={value === true} disabled={!hand} onChange={(event) => props.onChange(row, field, event.currentTarget.checked)} />
                  </label>
                ) : here && isScopes(field) ? (
                  // A list of scopes is filled a line at a time, each written as it is changed, in a panel that stays.
                  <ScopesEdit field={field} value={value} called={called} onChange={(next) => props.onChange(row, field, next)} onClose={() => setEditing(null)} />
                ) : here ? (
                  <Editing field={field} value={value} people={people} others={props.named} self={row.id} onDone={(next) => change(row, field, next)} />
                ) : hand ? (
                  <button class="cell" aria-label={`${heading(field)} of ${said(title && row.data[title.name]) || row.id}`} onClick={() => setEditing({ row: row.id, field: field.name, band })}>
                    <span class="cell-words">
                      {isScopes(field) ? <ScopesSaid field={field} value={value} /> : shownIn(row, field, value)}
                    </span>
                  </button>
                ) : (
                  <div class="cell read">
                    <span class="cell-words">
                      {isScopes(field) ? <ScopesSaid field={field} value={value} /> : shownIn(row, field, value)}
                    </span>
                  </div>
                )}
                {field === title && mark && <span class="grid-live-said faint">{markSaid(mark)}</span>}
                {field === title && !here && (
                  <button class="grid-open" aria-label={`Open ${said(value) || row.id}`} title={`Open this ${what}`} onClick={() => props.onOpen(row)}>
                    Open
                  </button>
                )}
              </td>
            );
          })}
          {adds && <td class="grid-adds" aria-hidden="true" />}
        </tr>
        {isOpen && (
          <tr class="grid-within">
            <td colSpan={wide}>{props.within!(row)}</td>
          </tr>
        )}
      </Fragment>
    );
  };
  const windowed = useWindowed(reads, by, props, wide, line, (most) => setLeast((was) => (Object.entries(most).every(([name, width]) => (was[name] ?? 0) >= width) ? was : { ...was, ...Object.fromEntries(Object.entries(most).map(([name, width]) => [name, Math.max(width, was[name] ?? 0)])) })));
  const counted = reads?.reading.counted ?? null;
  const count = counted ? counted.count : props.rows.length;
  return (
    <div
      class={classes('grid-scroll', windowed.paging && 'grid-paged')}
      ref={windowed.scroller}
      onScroll={windowed.onScroll}
      // Read a window at a time, it takes the keys that scroll (Home, End, a page at a time) where it is pressed, and has the keyboard back where a row that had it is no longer drawn.
      tabIndex={reads ? -1 : undefined}
      onFocusIn={windowed.focused}
      data-scrolls-itself={windowed.paging ? '' : undefined}
    >
      {windowed.live}
      <table ref={table} class={classes('grid', props.lines && 'lined', props.wrap && 'wrapped', headed(bands) && 'banded')} data-windowed={reads ? '' : undefined}>
        <thead>
          {headed(bands) && (
            <tr class="grid-bands">
              {headings(bands, stay).map((band) => (
                <th key={`${band.name ?? ''} ${band.from}`} style={from(band.from)} class={stays(band.from + band.span - 1, classes('grid-band', band.begins && 'band-first'))} colSpan={band.span} scope="colgroup">
                  {band.name && <span class="truncate">{band.name}</span>}
                </th>
              ))}
              {/* Over the place a column is added from: no heading, and no head. */}
              {adds && <td class="grid-adds" aria-hidden="true" />}
            </tr>
          )}
          <tr>
            {fields.map((field, nth) => (
              <th key={field.name} data-column={field.name} style={headPut(nth)} class={cell(nth, classes(field === title && 'grid-title', begins.has(field.name) && 'band-first'))} aria-sort={sort?.field === field.name ? (sort.down ? 'descending' : 'ascending') : 'none'}>
                <ColumnHead
                  field={field}
                  known={field === title}
                  sort={sort}
                  onSort={setSort}
                  choosing={choosing}
                  where={props.of}
                  heads={heads}
                  among={(bands.find((band) => band.fields.includes(field))?.fields ?? []).filter((one) => one !== title).map((one) => one.name)}
                />
                {resizes && heads && <WidthEdge name={field.name} called={heading(field)} nth={nth} width={widths[nth] ?? null} change={heads.change} />}
              </th>
            ))}
            {adds && heads && props.of && (
              <th class="grid-adds">
                <AddHead of={props.of} fields={heads.fields} groups={heads.groups} view={heads.view} />
              </th>
            )}
          </tr>
        </thead>
        <tbody ref={windowed.body}>
          {reads
            ? windowed.drawn
            : gathered.all && by
              ? gathered.all.map((band) => {
                  const shut = gathered.folded.includes(band.key);
                  return (
                    <Fragment key={band.key}>
                      <tr class="grid-gathered">
                        <td colSpan={wide}>
                          <BandHead band={band} by={by} shut={shut} onFold={() => gathered.fold(band.key)} />
                        </td>
                      </tr>
                      {!shut && sorted(band.rows, fields, sort).map((row) => line(row, band.key))}
                    </Fragment>
                  );
                })
              : sorted(props.rows, fields, sort).map((row) => line(row, ''))}
          {adding && (
            <tr>
              <td class="grid-title editing" colSpan={wide}>
                <Editing
                  field={title ?? fields[0]!}
                  value=""
                  people={people}
                  onDone={(next) => {
                    setWriting(false);
                    const words = typeof next === 'string' ? next.trim() : '';
                    props.onAdd(words === '' ? null : words);
                  }}
                />
              </td>
            </tr>
          )}
        </tbody>
        <tfoot>
          <tr>
            <td colSpan={wide}>
              {hand && (
                <button class="grid-new" onClick={() => setWriting(true)}>
                  + New {what}
                </button>
              )}
              <span class="grid-count faint" data-counted={reads ? (counted ? (counted.whole ? 'whole' : 'part') : 'not yet') : undefined}>
                {counted && !counted.whole ? 'at least ' : ''}
                {count.toLocaleString()} {count === 1 ? what : `${what}s`}
              </span>
              {reads?.reading.error && <span class="grid-count faint grid-unread">{reads.reading.error}</span>}
            </td>
          </tr>
          {/* A figure at the foot of each column the view says one of, of the rows it shows: there only where some column says one, and it stays at the table's foot as the heads stay at its top. */}
          {figures.length > 0 && (
            <tr class="grid-figures">
              {fields.map((field, nth) => {
                const which = figures.find((one) => one.field === field)?.which;
                // Counted by the daemon where it reads a window at a time: over every row the view shows, so far where it is not through.
                const value = reads ? (counted?.figures[field.name] ?? null) : which ? figure(props.rows, field, which) : null;
                return (
                  <td key={field.name} style={put(nth)} class={cell(nth, begins.has(field.name) && 'band-first')} data-figure={which ? field.name : undefined} title={which ? meaning(which) : undefined}>
                    {which && (
                      <span class="grid-figure">
                        <span class="faint">{figureCalled(which)}</span> {reads && !counted ? '…' : written(value)}
                        {counted && !counted.whole ? '…' : ''}
                      </span>
                    )}
                  </td>
                );
              })}
              {adds && <td class="grid-adds" aria-hidden="true" />}
            </tr>
          )}
        </tfoot>
      </table>
    </div>
  );
}

/** Where the reader is: what scrolls, where the room's top stands among the rows, and how much is on the screen below the heads. */
interface Place {
  top: number;
  base: number;
  view: number;
  /** Where the rows begin inside what scrolls: under the heads. */
  head: number;
}

/**
 * The rows drawn a window at a time: the room above and below them, the
 * heads of the bands, the rows read and the room of each still to be
 * read, and what scrolling does to them. Nothing where the rows are all
 * in hand.
 */
function useWindowed(
  reads: Reads | undefined,
  by: Field | undefined,
  props: TableProps,
  wide: number,
  line: (row: Row, band: string, rank?: number, kept?: string, mark?: Mark, marked?: boolean) => ComponentChildren,
  widest: (most: Record<string, number>) => void,
) {
  const scroller = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLTableSectionElement>(null);
  const [place, setPlace] = useKept<Place>('rows.place', { top: 0, base: 0, view: 800, head: FIRST });
  // How tall a row is (the first drawn, measured), a band's head, and the rows measured otherwise, by band.
  const [sizes, setSizes] = useKept<{ each: number; head: number; odd: Record<string, Odd> }>('rows.sizes', { each: FIRST, head: FIRST, odd: {} });
  const heads = props.heads;
  const counted = reads?.reading.counted ?? null;
  const bandsSaid = counted?.bands && by && counted.bands.some((band) => band.key !== '') ? counted.bands : null;
  const folds = useFolds(heads?.at ?? '', reads?.large ?? false);
  const called = useCalled(bandsSaid ? by : undefined, (bandsSaid ?? []).map((band) => ({ id: band.key, number: 0, data: { [by?.name ?? '']: band.key }, position: 0, created: 0, updated: 0 })));
  const segments: Segment[] = !reads
    ? []
    : bandsSaid
      ? bandsSaid.map((band) => ({ key: keyOf(band.key), head: sizes.head, count: band.count, open: !folds.shut(band.key), odd: sizes.odd[keyOf(band.key)] ?? [] }))
      : [{ key: keyOf(null), head: 0, count: counted?.count ?? 0, open: true, odd: sizes.odd[keyOf(null)] ?? [] }];
  const total = totalOf(segments, sizes.each);
  const room = roomFor(total);
  const y0 = place.top + place.base;
  const shown = drawn(segments, sizes.each, y0, y0 + place.view);
  // Where the page puts what scrolls itself: what it set is no move of the reader's.
  const putting = useRef<number | null>(null);
  const was = useRef<{ segments: Segment[]; each: number } | null>(null);

  // The rows of what is drawn, asked for as they are wanted.
  useEffect(() => {
    if (!reads) return;
    for (const piece of shown.pieces) if (!piece.head) reads.reading.want(bandOf(piece.key), piece.from, piece.to);
  });

  const read = () => {
    const box = scroller.current;
    const rows = body.current;
    if (!box || !rows) return;
    const top = box.scrollTop;
    const head = rows.getBoundingClientRect().top - box.getBoundingClientRect().top + top;
    // Where the page put what scrolls itself is where the reader was: what it set is no move of theirs.
    const from = putting.current === null ? place : { top: putting.current, base: place.base };
    putting.current = null;
    const next = paged(from, top, total, Math.max(1, box.clientHeight - head));
    if (next.put !== undefined) {
      putting.current = next.put;
      box.scrollTop = next.put;
    }
    const now = { top: next.put ?? top, base: next.base, view: box.clientHeight - head, head };
    if (now.top !== place.top || now.base !== place.base || now.view !== place.view || now.head !== place.head) setPlace(now);
  };

  // Measured as it is drawn: what is on the screen, how tall rows are, and how wide each column has been.
  useLayoutEffect(() => {
    if (!reads) return;
    const box = scroller.current;
    const rows = body.current;
    if (!box || !rows) return;
    let { each, head } = sizes;
    const odd = { ...sizes.odd };
    const lines = [...rows.querySelectorAll<HTMLTableRowElement>(':scope > tr[data-rank]')];
    const tall = (row: HTMLTableRowElement) => {
      const next = row.nextElementSibling;
      return row.getBoundingClientRect().height + (next instanceof HTMLTableRowElement && next.classList.contains('grid-within') ? next.getBoundingClientRect().height : 0);
    };
    const plain = lines.find((row) => !(row.nextElementSibling instanceof HTMLTableRowElement && row.nextElementSibling.classList.contains('grid-within')));
    if (plain && !props.wrap) each = Math.round(plain.getBoundingClientRect().height * 100) / 100;
    const gatheredRow = rows.querySelector<HTMLElement>(':scope > tr.grid-gathered');
    if (gatheredRow) head = Math.round(gatheredRow.getBoundingClientRect().height * 100) / 100;
    for (const row of lines) {
      const key = row.dataset.band ?? keyOf(null);
      const rank = Number(row.dataset.rank);
      odd[key] = measured(each, odd[key] ?? [], rank, tall(row), shown.pieces[0] && !shown.pieces[0].head ? shown.pieces[0].from : 0);
    }
    if (props.wrap && lines.length > 0 && Object.values(odd).every((list) => list.length === 0)) each = tall(lines[0]!);
    if (each !== sizes.each || head !== sizes.head || JSON.stringify(odd) !== JSON.stringify(sizes.odd)) setSizes({ each, head, odd });
    // Not every row is drawn: a column the view says nothing of is kept as wide as it has been, so that it does not narrow as rows pass.
    if (counted && lines.length < counted.count) {
      const most: Record<string, number> = {};
      box.querySelectorAll<HTMLElement>('thead tr:last-child th[data-column]').forEach((th) => {
        const name = th.dataset.column!;
        if (!th.classList.contains('sized')) most[name] = Math.ceil(th.getBoundingClientRect().width);
      });
      widest(most);
    }
  });

  // What stands at the top of the screen stays there as what is above it is measured, read in or counted again.
  useLayoutEffect(() => {
    const box = scroller.current;
    const now = { segments, each: sizes.each };
    const before = was.current;
    was.current = now;
    if (!reads || !box || !before || before.segments.length === 0) return;
    const y = place.top + place.base;
    if (y <= 0) return;
    const there = shifted(before, now, y);
    if (Math.abs(there - y) < 0.5) return;
    const top = Math.max(0, there - place.base);
    putting.current = top;
    box.scrollTop = top;
    setPlace({ ...place, top });
  });

  // A row read about once the view was read again is kept where it was, and one gone to is brought into view.
  useLayoutEffect(() => {
    const placed = reads?.reading.placed;
    const box = scroller.current;
    if (!reads || !placed || !box) return;
    const segment = segments.find((one) => one.key === placed.key);
    // Gone to in a band that is folded: the band is opened first, and the row placed once it is drawn.
    if (segment && !segment.open && placed.from === null) return folds.fold(bandOf(placed.key) ?? '');
    const to = yOf(segments, sizes.each, placed.key, placed.to);
    const from = placed.from === null ? null : yOf(segments, sizes.each, placed.key, placed.from);
    // Not yet counted as far as it stands: placed once it is.
    if (to === null || (placed.from !== null && from === null)) return;
    reads.reading.took();
    if (from !== null) {
      const top = Math.max(0, place.top + (to - from));
      putting.current = top;
      box.scrollTop = top;
      return setPlace({ ...place, top });
    }
    // Brought into view a few rows from the top, as a reader dragging the scrollbar there would: a jump past the cap is a jump among all the rows.
    const y = Math.max(0, to - 2 * sizes.each);
    const most = room - place.view;
    const near = y - place.base;
    box.scrollTop = total <= room || (near >= 0 && near <= most && Math.abs(near - place.top) <= 2 * place.view) ? near : Math.round(Math.min(1, y / Math.max(1, total - place.view)) * most);
  });

  // The room on the screen changes with the window.
  useEffect(() => {
    const box = scroller.current;
    if (!reads || !box || typeof ResizeObserver === 'undefined') return;
    read();
    const seen = new ResizeObserver(() => read());
    seen.observe(box);
    return () => seen.disconnect();
  }, [!!reads]);

  // Past the cap the page puts back where it was itself, since a place in the room is no place among the rows by itself.
  useLayoutEffect(() => {
    const box = scroller.current;
    if (!reads || !box || total <= room || place.top === 0) return;
    putting.current = place.top;
    box.scrollTop = place.top;
  }, []);

  // Where the keyboard was in a row that is no longer drawn, what scrolls has it, so that the keys go on scrolling.
  const had = useRef(false);
  useLayoutEffect(() => {
    if (had.current && scroller.current && (document.activeElement === document.body || document.activeElement === null)) scroller.current.focus({ preventScroll: true });
    had.current = scroller.current?.contains(document.activeElement) ?? false;
  });
  const focused = () => (had.current = true);

  if (!reads) return { scroller, body, onScroll: undefined, focused: undefined, drawn: null, paging: false, live: null };

  const pieces: ComponentChildren[] = [];
  const empty = (key: string, height: number) => (
    <tr key={key} class="grid-room" aria-hidden="true">
      <td colSpan={wide} style={`height:${height}px`} />
    </tr>
  );
  pieces.push(empty('room-before', Math.max(0, shown.top - place.base)));
  for (const piece of shown.pieces) {
    const band = bandOf(piece.key);
    if (piece.head) {
      const said = bandsSaid!.find((one) => keyOf(one.key) === piece.key)!;
      const shut = folds.shut(said.key);
      pieces.push(
        <tr key={`head ${piece.key}`} class="grid-gathered">
          <td colSpan={wide}>
            <BandHead band={{ key: said.key, ...bandCalled(said.key, by!, props.named ?? {}, called), rows: [] }} count={said.count} by={by!} shut={shut} onFold={() => folds.fold(said.key)} />
          </td>
        </tr>,
      );
      continue;
    }
    const runs = reads.reading.bands[piece.key]?.runs ?? [];
    const odd = sizes.odd[piece.key] ?? [];
    // Read again in part, a row may be held at two places for a moment: it is drawn at one.
    const seen = new Set<string>();
    let waiting = 0;
    let waitFrom = piece.from;
    const flush = (rank: number) => {
      if (waiting > 0) pieces.push(empty(`wait ${piece.key} ${waitFrom}`, waiting));
      waiting = 0;
      waitFrom = rank;
    };
    for (let rank = piece.from; rank < piece.to; rank++) {
      const row = rowAt(runs, rank);
      if (!row || seen.has(row.id)) {
        if (waiting === 0) waitFrom = rank;
        waiting += odd.find(([at]) => at === rank)?.[1] ?? sizes.each;
        continue;
      }
      flush(rank);
      seen.add(row.id);
      pieces.push(line(row, band ?? '', rank, piece.key, reads.reading.live.marks[row.id], reads.reading.marked === row.id));
    }
    flush(piece.to);
  }
  pieces.push(empty('room-after', Math.max(0, room - (shown.bottom - place.base))));
  // What changed since the view was read, counted at the top of the table, with the one press that reads it again where the reader is.
  const changed = liveSaid(reads.reading.live);
  const showAgain = () => {
    const at = anchorAt(segments, sizes.each, place.top + place.base);
    const id = at && at.rank >= 0 ? rowAt(reads.reading.bands[at.key]?.runs ?? [], at.rank)?.id : undefined;
    reads.reading.again(at && id ? { key: at.key, id, rank: at.rank } : undefined);
  };
  const live = changed && (
    <div class="grid-live" style={`top:${place.head}px`}>
      <div class="grid-live-line" role="status">
        <span>{changed}</span>
        <button type="button" onClick={showAgain}>
          Show
        </button>
      </div>
    </div>
  );
  return { scroller, body, onScroll: read, focused, drawn: pieces, paging: total > room, live };
}

/** The band a key is of: `null` for the one band of a view gathered by no column. */
function bandOf(key: string): string | null {
  return key.startsWith('=') ? key.slice(1) : null;
}
