// A table's records by what comes first, and in bands by what they name.
//
// Two things a view of a table can do, whatever the table is of. Laid out
// as an order, its records stand in columns from left to right: what can
// be begun now, what waits only on those, and so on, with those that are
// done folded away at the left and those that wait in a ring set apart at
// the right. What is side by side in a column waits on nothing of each
// other, and its head says how many that is. And a board or an order may
// gather its records in bands by a field that names something, each band
// called what that is known by, folded and unfolded as whoever looks
// likes. What column a record is in, and which band, is worked out in
// `ordered.ts`; this draws it.

import type { ComponentChildren } from 'preact';
import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { coalesce } from '../events.ts';
import { bands as bandsOf, columnName, ordered, standing } from '../ordered.ts';
import type { Band } from '../ordered.ts';
import type { Field, Named, Row } from '../table.ts';
import { idsOf, knownBy, said, titleField } from '../table.ts';
import { useEvents } from '../ui.tsx';
import { classes } from '../util.ts';
import { Tag, worded } from './tag.tsx';

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

/**
 * What the things a text field names are called, where they are records:
 * a text that is the id of a record somebody may read is called what that
 * record is, and anything else is called what it says. Read once, and
 * again when one of them is changed or taken away. A field of another
 * kind asks nothing: it says what it names itself.
 */
export function useCalled(by: Field | undefined, rows: Row[]): Record<string, string> {
  const asked =
    by?.kind === 'text'
      ? [...new Set(rows.map((row) => said(row.data[by.name]).trim()).filter((value) => /^\S+$/.test(value)))].sort().join(' ')
      : '';
  const [called, setCalled] = useState<Record<string, string>>({});
  const read = useMemo(
    () =>
      coalesce(() => {
        if (asked === '') return;
        const each = asked.split(' ').map((id) =>
          api
            .record(id)
            .then((row): [string, string] => [id, said(row.data.title).trim() || id])
            .catch((): [string, string] => [id, id]),
        );
        void Promise.all(each).then((found) => setCalled(Object.fromEntries(found)));
      }, 50),
    [asked],
  );
  useEffect(read, [read]);
  useEvents(['record.updated', 'record.removed'], (event) => asked.split(' ').includes(String(event.data?.id ?? '')) && read(), [read, asked]);
  return called;
}

interface BandsProps {
  rows: Row[];
  /** The field the records are gathered by; none, and they are not gathered. */
  by: Field | undefined;
  named: Named;
  /** Where what is folded is kept: the project, the table and the view. */
  at: string;
  /** Draws some of the records: those of one band, or all of them (`null`) where they are not gathered. */
  each: (only: Row[] | null, first: boolean) => ComponentChildren;
}

/**
 * Records in bands by what one of their fields names, each band under a
 * head that folds it. Which are folded is the reader's, and is kept in
 * the browser with the view it is of.
 */
export function Bands({ rows, by, named, at, each }: BandsProps) {
  const { all, folded, fold } = useBands(rows, by, named, at);
  if (!by || !all) return <>{each(null, true)}</>;
  return (
    <div class="bands">
      {all.map((band, nth) => {
        const shut = folded.includes(band.key);
        return (
          <section key={band.key} class="bands-band" aria-label={band.name}>
            <BandHead band={band} by={by} shut={shut} onFold={() => fold(band.key)} />
            {!shut && each(band.rows, nth === 0)}
          </section>
        );
      })}
    </div>
  );
}

/** What heads a band: what folds it, what it is called, and how many are in it. */
/** A band's head; `count`, where the daemon counted its rows and they are not all in hand. */
export function BandHead({ band, by, shut, onFold, count }: { band: Band; by: Field; shut: boolean; onFold: () => void; count?: number }) {
  return (
    <button class="bands-head" aria-expanded={!shut} title={shut ? 'Show these' : 'Fold these away'} onClick={onFold}>
      <span class="bands-fold" aria-hidden="true">
        {shut ? '▸' : '▾'}
      </span>
      {worded(by) && band.key !== '' ? <Tag of={band.name} /> : <strong class="truncate">{band.name}</strong>}
      {band.gone && <span class="faint">which is no longer there</span>}
      <span class="faint">{count ?? band.rows.length}</span>
    </button>
  );
}

/**
 * The records in their bands by a field, with which bands the reader
 * folded, kept in their browser, and what folds one: for whoever draws
 * the bands, a board in sections and a table under lines. No bands
 * (`all` is `null`) where the records are not gathered, or none of them
 * says anything in the field: a band is for telling groups apart.
 */
export function useBands(rows: Row[], by: Field | undefined, named: Named, at: string): { all: Band[] | null; folded: string[]; fold: (band: string) => void } {
  const key = `client.bands.${at}`;
  const [folded, setFolded] = useState<string[]>(() => {
    try {
      const read: unknown = JSON.parse(kept(key) ?? '[]');
      return Array.isArray(read) ? read.map(String) : [];
    } catch {
      return [];
    }
  });
  const called = useCalled(by, rows);
  const fold = (band: string) => {
    const next = folded.includes(band) ? folded.filter((one) => one !== band) : [...folded, band];
    setFolded(next);
    keep(key, JSON.stringify(next));
  };
  const found = by ? bandsOf(rows, by, named, called) : null;
  // A band is for telling groups apart: where no record says anything in the field there are none to tell.
  return { all: found && found.some((band) => band.key !== '') ? found : null, folded, fold };
}

interface OrderProps {
  /** Every field of the table, whatever the view leaves out. */
  fields: Field[];
  /** The records shown; and every record of the table, where the view is held to some: a record's place goes by all of them. */
  rows: Row[];
  every?: Row[];
  named: Named;
  /** The field in which a record says which others come before it. */
  by: string | null;
  /** The choice whose last value says a record is done; the table's first choice, where none is said. */
  doneBy: string | null;
  /** The field the records are gathered by, where they are. */
  group: string | null;
  at: string;
  /** What one record is called: `card`. */
  what: string;
  onOpen: (row: Row) => void;
  /** What the page says of a record on its card, beside what is said here. */
  marks?: (row: Row) => ComponentChildren;
  /** What the page puts above the columns: where a new record is being written. */
  above?: ComponentChildren;
}

/** A sentence's count: `3 cards`, `1 card`. */
const some = (count: number, what: string) => `${count} ${what}${count === 1 ? '' : 's'}`;

/**
 * The records by what comes first. A record is pressed to be opened, and
 * pointed at to see what it waits on and what waits on it; it is not
 * carried anywhere, since where it stands is what it waits on and not
 * something to be let go of in another column.
 */
export function OrderLayout({ fields, rows, every, named, by, doneBy, group, at, what, onOpen, marks, above }: OrderProps) {
  const [near, setNear] = useState<string | null>(null);
  const [doneOpen, setDoneOpen] = useState(() => kept(`client.done.${at}`) === 'open');
  const comes = fields.find((field) => field.name === by && (field.kind === 'list' || field.kind === 'records'));
  const ends = fields.find((field) => field.name === doneBy && field.kind === 'choice') ?? fields.find((field) => field.kind === 'choice');
  // What comes first is worked out over every record, shown or not: one that waits on a record this view leaves out still waits on it.
  const all = every ?? rows;
  const sorted = useMemo(() => [...all].sort((a, b) => a.number - b.number || a.id.localeCompare(b.id)), [all]);
  const seen = useMemo(() => new Set(rows.map((row) => row.id)), [rows]);
  const order = useMemo(
    () => ordered(sorted.map((row) => ({ id: row.id, after: comes ? idsOf(row.data[comes.name]) : [], done: standing(ends, ends && row.data[ends.name]).done }))),
    [sorted, comes?.name, ends?.name],
  );
  if (!comes) {
    return <div class="empty">This view goes by a field in which a record says which others come before it, and the table has none it names.</div>;
  }
  const byId = new Map(sorted.map((row) => [row.id, row]));
  const title = titleField(fields);
  const called = (id: string) => {
    const row = byId.get(id);
    return row ? knownBy(row, title ? [title] : fields) : id;
  };
  const stands = (row: Row) => standing(ends, ends && row.data[ends.name]);
  const done = new Set(order.done);
  const turning = new Set(order.turning);
  const openDone = (open: boolean) => {
    setDoneOpen(open);
    keep(`client.done.${at}`, open ? 'open' : 'shut');
  };

  const card = (id: string) => {
    const row = byId.get(id)!;
    const stand = stands(row);
    const waits = order.after[id]!.filter((one) => !done.has(one));
    const gone = order.missing[id]!;
    return (
      <article
        key={id}
        class={classes(
          'card',
          'ordered-card',
          done.has(id) && 'ordered-over',
          near !== null && near !== id && order.after[near]?.includes(id) && 'ordered-first',
          near !== null && near !== id && order.before[near]?.includes(id) && 'ordered-next',
        )}
        data-ordered={id}
        tabIndex={0}
        onClick={() => onOpen(row)}
        onKeyDown={(event) => event.key === 'Enter' && onOpen(row)}
        onPointerEnter={() => setNear(id)}
        onPointerLeave={() => setNear((now) => (now === id ? null : now))}
        onFocus={() => setNear(id)}
        onBlur={() => setNear((now) => (now === id ? null : now))}
      >
        <span class="id">{id}</span>
        <span class="card-name">{called(id)}</span>
        <span class="row">
          {/* One somebody has begun says where it is: it is here because of what it waits on, not because nobody has it. */}
          {stand.begun && (
            <span class="badge plain ordered-where" title={`It is in ${stand.at}: somebody has begun it.`}>
              {stand.at}
            </span>
          )}
          {turning.has(id) && (
            <span class="badge warning" title="It waits on something that, by one road or another, waits on it: none of them can be begun until one of them waits on less.">
              in a ring
            </span>
          )}
          {marks?.(row)}
        </span>
        {(waits.length > 0 || gone.length > 0) && !done.has(id) && (
          <span class="faint ordered-waits">
            waits on {[...waits.map(called), ...gone.map((one) => `${one}, which is not here`)].join('; ')}
          </span>
        )}
      </article>
    );
  };

  // Where everything that is not done can be begun, there is one step and
  // nothing beside it: its cards are then laid across the room there is,
  // a row at a time, so that sixty of them are not one column down the
  // page. A card added goes after the last, and the rest keep their place.
  const alone = order.columns.length <= 1 && order.ring.length === 0;
  // The columns for some of the records: every column is drawn for every
  // band, so that a band's columns stand under the ones above them.
  const lanes = (only: Row[] | null) => {
    const here = only ? new Set(only.map((row) => row.id)) : null;
    const of = (ids: string[]) => ids.filter((id) => seen.has(id) && (!here || here.has(id)));
    const over = of(order.done);
    const ring = of(order.ring);
    return (
      <div
        class={classes('ordered-lanes', alone && 'ordered-room')}
        style={`grid-template-columns: ${doneOpen ? '240px' : '104px'} ${alone ? 'minmax(0, 1fr)' : `repeat(${order.columns.length + (order.ring.length > 0 ? 1 : 0)}, 240px)`}`}
      >
        <section class={classes('column', 'ordered-lane', 'ordered-donelane', !doneOpen && 'ordered-shut')} aria-label="Done">
          <button
            class="column-head ordered-fold"
            aria-expanded={doneOpen}
            title={doneOpen ? 'Fold away what is done' : 'Show what is done'}
            onClick={() => openDone(!doneOpen)}
          >
            <span class="grow">Done</span>
            <span class="faint">{over.length}</span>
          </button>
          {doneOpen && <div class="cards">{over.map(card)}</div>}
        </section>
        {order.columns.map((ids, nth) => {
          const mine = of(ids);
          const begun = mine.filter((id) => stands(byId.get(id)!).begun);
          const where = [...new Set(begun.map((id) => stands(byId.get(id)!).at))].map((value) => `${begun.filter((id) => stands(byId.get(id)!).at === value).length} ${value}`);
          return (
            <section key={nth} class={classes('column', 'ordered-lane', mine.length === 0 && 'ordered-none', alone && 'ordered-wide')} aria-label={columnName(nth)}>
              <div class="column-head ordered-head">
                <span class="grow">{columnName(nth)}</span>
                {/* How many of these nobody has begun: that many can be worked side by side. */}
                <span class="faint" data-ordered-count={mine.length - begun.length}>
                  {mine.length - begun.length}
                </span>
              </div>
              {mine.length > 0 && (
                <div class="faint ordered-note">
                  {nth === 0 ? `${mine.length - begun.length} to begin, side by side` : `side by side, once what they wait on is done`}
                  {begun.length > 0 && `; ${begun.length} more under way (${where.join(', ')})`}
                </div>
              )}
              <div class="cards">{mine.map(card)}</div>
            </section>
          );
        })}
        {/* Drawn only where some of these are in one: it is the last column, so nothing after it loses its place. */}
        {ring.length > 0 && (
          <section class="column ordered-lane ordered-ringlane" aria-label="Waits in a ring">
            <div class="column-head ordered-head">
              <span class="grow">Waits in a ring</span>
              <span class="faint">{ring.length}</span>
            </div>
            <div class="faint ordered-note">None of these can be begun: each waits on another of them. Change what one of them waits on.</div>
            <div class="cards">{ring.map(card)}</div>
          </section>
        )}
      </div>
    );
  };

  // What is said above the columns is of what is shown.
  const shownIn = (ids: string[]) => ids.filter((id) => seen.has(id));
  const first = shownIn(order.columns[0] ?? []);
  const begun = first.filter((id) => stands(byId.get(id)!).begun).length;
  const later = order.columns.slice(1).filter((ids) => shownIn(ids).length > 0).length;
  const said = [
    `${some(first.length - begun, what)} can be begun now, side by side`,
    begun > 0 ? `${begun} more under way` : '',
    later > 0 ? `${some(later, 'step')} after them` : '',
    `${shownIn(order.done).length} done`,
    shownIn(order.ring).length > 0 ? `${shownIn(order.ring).length} waiting in a ring` : '',
  ].filter((part) => part !== '');
  return (
    <div class="ordered">
      {above && <div class="ordered-above">{above}</div>}
      <p class="faint ordered-sum">{said.join(' · ')}</p>
      <Bands rows={sorted.filter((row) => seen.has(row.id))} by={fields.find((field) => field.name === group)} named={named} at={at} each={lanes} />
    </div>
  );
}
