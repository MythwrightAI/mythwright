// A table's records drawn from what they are: a cell by the kind its
// field is, read and changed, a field of a record's form, and the same
// rows as a list, a gallery and on a calendar. Laid out as a table they
// are drawn by `windowed.tsx`; a board, which is also the rows, is in
// `board.tsx` because cards on it are dragged between columns.
//
// Nothing here knows what the records are of, and what changing one does
// is whoever uses this to say.

import { Fragment } from 'preact';
import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { touched } from '../phone.ts';
import { isScopes } from '../scopes.ts';
import { Select } from '../select.tsx';
import { choosable, dayOf, dayString, heading, idsOf, nameable, named, namedAll, nothingSaid, offeredBy, said, sorted, titleField, weeksOf } from '../table.ts';
import type { Field, Group, Named, Row } from '../table.ts';
import { classes } from '../util.ts';
import { worn } from '../clock.ts';
import { zoned } from '../zoned.ts';
import { Scopes } from './scopes.tsx';
import { Tag } from './tag.tsx';

/** Someone an `actor` field can hold: what is kept, and what they are called. */
export interface Person {
  value: string;
  name: string;
}

export interface Rows {
  /** The fields, in the order of the columns. */
  fields: Field[];
  rows: Row[];
  /** What one record is called: `card`. */
  what: string;
  people: Person[];
  /** What the fields that name a record of another collection can name, by that collection. */
  named?: Named;
  onOpen: (row: Row) => void;
  /** What a row is opened into, under it, where the rows of other tables name it. */
  within?: (row: Row) => ComponentChildren;
  /** The table's columns in groups, where it says any: a table is drawn with their headings over its heads (`groups.ts`). */
  groups?: Group[] | null;
}

/** What is in a cell, as it is read. */
export function Shown({ field, value, people, others }: { field: Field; value: unknown; people: Person[]; others?: Named }) {
  if (value === null || value === undefined || value === '' || (Array.isArray(value) && value.length === 0)) return null;
  if (field.kind === 'record') {
    const one = named(field, value, others ?? {});
    if (!one) return null;
    // One that is gone is still said, as what it was called here: its id.
    return one.gone ? (
      <span class="cell-names gone" title={`There is no ${one.id} in ${field.of[0]} any more`}>
        <s>{one.words}</s> no longer there
      </span>
    ) : (
      <span class="cell-names" title={one.id}>
        {one.words}
      </span>
    );
  }
  if (field.kind === 'records') {
    return (
      <span class="cell-tags">
        {namedAll(field, value, others ?? {}).map((one) =>
          one.gone ? (
            <span key={one.id} class="cell-names gone" title={`There is no ${one.id} in ${field.of[0]} any more`}>
              <s>{one.words}</s>
            </span>
          ) : (
            <span key={one.id} class="tag cell-names" title={one.id}>
              {one.words}
            </span>
          ),
        )}
      </span>
    );
  }
  if (field.kind === 'choice') return <Tag of={value} />;
  if (field.kind === 'list' && Array.isArray(value)) {
    return (
      <span class="cell-tags">
        {value.map((one) => (
          <Tag key={String(one)} of={one} />
        ))}
      </span>
    );
  }
  if (field.kind === 'actor') {
    const words = String(value);
    const known = people.find((person) => person.value === words);
    return <span class={classes('cell-who', words.startsWith('agent:') && 'agent')}>{known?.name ?? words.slice(words.indexOf(':') + 1)}</span>;
  }
  if (field.kind === 'date') return <>{dayWritten(String(value))}</>;
  return <>{said(value)}</>;
}

/** What is written in a field, with the reader's zone after a time where its table says it is one with a zone. */
function ownZone(field: Field, written: string): string {
  return field.zoned ? zoned(written, worn.value.zone) : written;
}

/** A day as it is read: `30 Nov 2026`. */
function dayWritten(day: string): string {
  const [year, month, date] = day.split('-').map(Number);
  if (!year || !month || !date) return day;
  return new Date(year, month - 1, date).toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
}

/** A cell being changed: by what its field is. It says what it was changed to, or that it was left. */
export function Editing({ field, value, people, others, self, onDone }: { field: Field; value: unknown; people: Person[]; others?: Named; self?: string; onDone: (next?: unknown) => void }) {
  const box = useRef<HTMLDivElement>(null);
  // Whichever comes first, Enter or the caret leaving, is the answer.
  const answered = useRef(false);
  const done = (next?: unknown) => {
    if (answered.current) return;
    answered.current = true;
    onDone(next);
  };
  useEffect(() => box.current?.querySelector<HTMLElement>('input, select')?.focus(), []);
  // What was last chosen, for a field that is chosen from.
  const kept = useRef<unknown>(undefined);
  if (field.kind === 'choice' || field.kind === 'list') {
    const many = field.kind === 'list';
    const chosen = many ? (Array.isArray(value) ? value.map(String) : []) : value ? [String(value)] : [];
    // A choice is one press; labels are as many as are wanted, then away.
    return (
      <div
        class="cell-edit"
        ref={box}
        onFocusOut={(event) => {
          if (!box.current?.contains(event.relatedTarget as Node | null)) done(kept.current);
        }}
      >
        <Select
          label={heading(field)}
          chosen={chosen}
          options={field.of}
          many={many}
          makes={many}
          focused
          onChange={(next) => {
            kept.current = many ? next.map(String) : (next[0] ?? null);
          }}
        />
      </div>
    );
  }
  // A text whose table offers the usual words is chosen from them, or written as any other: one press, or a word and Enter.
  if (offeredBy(field).length > 0) {
    const now = said(value).trim();
    return (
      <div
        class="cell-edit"
        ref={box}
        onFocusOut={(event) => {
          if (!box.current?.contains(event.relatedTarget as Node | null)) done(kept.current);
        }}
      >
        <Select
          label={heading(field)}
          chosen={now ? [now] : []}
          options={offeredBy(field)}
          makes
          clears
          focused
          onChange={(next) => {
            kept.current = next[0] === undefined ? '' : String(next[0]);
          }}
        />
      </div>
    );
  }
  if (field.kind === 'actor') {
    const now = value ? String(value) : '';
    return (
      <div class="cell-edit" ref={box}>
        <select value={now} aria-label={heading(field)} onChange={(event) => done(event.currentTarget.value || null)} onBlur={() => done()} onKeyDown={(event) => event.key === 'Escape' && done()}>
          <option value="">nobody</option>
          {now && !people.some((person) => person.value === now) && <option value={now}>{now}</option>}
          {people.map((person) => (
            <option key={person.value} value={person.value}>
              {person.name}
            </option>
          ))}
        </select>
      </div>
    );
  }
  if (field.kind === 'records') {
    // As many as are wanted, then away: chosen by what each is known by.
    const from = choosable(field, value, others ?? {}, self);
    const held = idsOf(value);
    return (
      <div
        class="cell-edit"
        ref={box}
        onFocusOut={(event) => {
          if (!box.current?.contains(event.relatedTarget as Node | null)) done(kept.current);
        }}
      >
        <Select
          label={heading(field)}
          chosen={from.filter((one) => held.includes(one.id)).map((one) => one.name)}
          options={from.map((one) => one.name)}
          many
          focused
          onChange={(next) => {
            kept.current = from.filter((one) => next.map(String).includes(one.name)).map((one) => one.id);
          }}
        />
      </div>
    );
  }
  if (field.kind === 'record') {
    return (
      <div class="cell-edit" ref={box}>
        <select value={said(value)} aria-label={heading(field)} onChange={(event) => done(event.currentTarget.value || null)} onBlur={() => done()} onKeyDown={(event) => event.key === 'Escape' && done()}>
          <option value="">nothing</option>
          {nameable(field, value, others ?? {}, self).map((one) => (
            <option key={one.value} value={one.value}>
              {one.name}
            </option>
          ))}
        </select>
      </div>
    );
  }
  if (field.kind === 'date') {
    return (
      <div class="cell-edit" ref={box}>
        <input
          type="date"
          aria-label={heading(field)}
          defaultValue={said(value)}
          onBlur={(event) => done(event.currentTarget.value || null)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') done(event.currentTarget.value || null);
            else if (event.key === 'Escape') done();
          }}
        />
      </div>
    );
  }
  const number = field.kind === 'number';
  // A number that is not one changes nothing.
  const read = (text: string): unknown => (!number ? ownZone(field, text) : text.trim() === '' ? null : Number.isFinite(Number(text)) ? Number(text) : undefined);
  return (
    <div class="cell-edit" ref={box}>
      <input
        type="text"
        inputMode={number ? 'decimal' : undefined}
        aria-label={heading(field)}
        defaultValue={said(value)}
        onBlur={(event) => done(read(event.currentTarget.value))}
        onKeyDown={(event) => {
          if (event.key === 'Enter') done(read(event.currentTarget.value));
          else if (event.key === 'Escape') done();
        }}
      />
    </div>
  );
}

/**
 * One field of a record in a form: its name, and what it is changed with,
 * which is what its kind is changed with in a table. Where a table's cell
 * is pressed and then left, this stays open among the others until the
 * form is saved, and says each change as it is made.
 */
export function FieldInput({ field, value, people, others, self, onChange }: { field: Field; value: unknown; people: Person[]; others?: Named; self?: string; onChange: (next: unknown) => void }) {
  const name = heading(field);
  if (isScopes(field)) {
    // A list of scopes is read and filled by area, as it is in a table's cell.
    return (
      <div class="field-many" role="group" aria-label={name}>
        <span>{name}</span>
        <Scopes field={field} value={value} onChange={onChange} />
      </div>
    );
  }
  if (field.kind === 'flag') {
    return (
      // Its name, and its switch a little past it: in a record's form every flag's switch stands in one column, as far
      // in as the longest flag's name and no further (`flagColumn`, the form's `--flag-column`), never at its far edge.
      <label class="flag-field">
        <span class="flag-name">{name}</span>
        <input type="checkbox" role="switch" class="switch" checked={value === true} onChange={(event) => onChange(event.currentTarget.checked)} />
      </label>
    );
  }
  if (field.kind === 'choice' || field.kind === 'actor' || field.kind === 'record') {
    const now = value ? String(value) : '';
    const options: { value: string; name: string }[] =
      field.kind === 'choice' ? field.of.map((one) => ({ value: one, name: one })) : field.kind === 'actor' ? people : nameable(field, value, others ?? {}, self);
    return (
      <label>
        {name}
        <select value={now} onChange={(event) => onChange(event.currentTarget.value || null)}>
          <option value="">{field.kind === 'actor' ? 'nobody' : nothingSaid(field)}</option>
          {now && !options.some((one) => one.value === now) && <option value={now}>{now}</option>}
          {options.map((one) => (
            <option key={one.value} value={one.value}>
              {one.name}
            </option>
          ))}
        </select>
      </label>
    );
  }
  if (offeredBy(field).length > 0) {
    const now = said(value).trim();
    return (
      <div class="field-many">
        <span>{name}</span>
        <Select label={name} chosen={now ? [now] : []} options={offeredBy(field)} makes clears onChange={(next) => onChange(next[0] === undefined ? '' : String(next[0]))} />
      </div>
    );
  }
  if (field.kind === 'records') {
    const from = choosable(field, value, others ?? {}, self);
    const held = idsOf(value);
    return (
      <div class="field-many">
        <span>{name}</span>
        <Select
          label={name}
          chosen={from.filter((one) => held.includes(one.id)).map((one) => one.name)}
          options={from.map((one) => one.name)}
          many
          onChange={(next) => onChange(from.filter((one) => next.map(String).includes(one.name)).map((one) => one.id))}
        />
      </div>
    );
  }
  if (field.kind === 'date') {
    return (
      <label>
        {name}
        <input type="date" value={said(value)} onInput={(event) => onChange(event.currentTarget.value || null)} />
      </label>
    );
  }
  if (field.kind === 'list') {
    // A thing at a time, as its cell takes them: one may hold a comma.
    const items = Array.isArray(value) ? value.map(String) : [];
    return (
      <div class="field-many">
        <span>{name}</span>
        <Select label={name} chosen={items} options={field.of} many makes onChange={(next) => onChange(next.map(String))} />
      </div>
    );
  }
  const number = field.kind === 'number';
  // Words that run over a line, or to several, are read and written as such.
  const words = said(value);
  if (!number && (words.includes('\n') || words.length > 80)) {
    return (
      <label>
        {name}
        <textarea rows={Math.min(14, Math.max(3, words.split('\n').length + Math.floor(words.length / 90)))} value={words} onInput={(event) => onChange(event.currentTarget.value)} />
      </label>
    );
  }
  return (
    <label>
      {name}
      <input
        type="text"
        inputMode={number ? 'decimal' : undefined}
        value={said(value)}
        onBlur={(event) => field.zoned && onChange(ownZone(field, event.currentTarget.value))}
        onInput={(event) => {
          const written = event.currentTarget.value;
          // A number that is not one is left as it was written, for the daemon to say what is wrong with it.
          onChange(!number ? written : written.trim() === '' ? null : Number.isFinite(Number(written)) ? Number(written) : written);
        }}
      />
    </label>
  );
}

/** What turns a line open into the rows of the tables that name it, and closed again. */
export function Turn({ open, called, onTurn }: { open: boolean; called: string; onTurn: () => void }) {
  return (
    <button class="within-turn" aria-expanded={open} aria-label={`${open ? 'Close' : 'Open'} what is in ${called}`} title="What names this, in the other tables" onClick={onTurn}>
      {open ? '▾' : '▸'}
    </button>
  );
}

/** What is said of a record beside what it is known by: its choices, its labels, who, and what it names. */
function Aside({ fields, row, people, others }: { fields: Field[]; row: Row; people: Person[]; others?: Named }) {
  return (
    <>
      {fields
        .filter((field) => ['choice', 'list', 'actor', 'record', 'records'].includes(field.kind))
        .map((field) => (
          <Shown key={field.name} field={field} value={row.data[field.name]} people={people} others={others} />
        ))}
    </>
  );
}

/** A line a record. */
export function ListLayout({ fields, rows, people, named: others, onOpen, within }: Rows) {
  const title = titleField(fields);
  const [opened, setOpened] = useState<string[]>([]);
  return (
    <div class="rows-list">
      {sorted(rows, fields, null).map((row) => {
        const called = said(title && row.data[title.name]) || row.id;
        const line = (
          <button key={row.id} class="rows-line" onClick={() => onOpen(row)}>
            <span class="truncate grow">{called}</span>
            <Aside fields={fields} row={row} people={people} others={others} />
            <span class="id faint">{row.id}</span>
          </button>
        );
        if (!within) return line;
        // A line that can be opened into what names it has what turns it before it.
        const isOpen = opened.includes(row.id);
        return (
          <Fragment key={row.id}>
            <div class="rows-turned">
              <Turn open={isOpen} called={called} onTurn={() => setOpened(isOpen ? opened.filter((one) => one !== row.id) : [...opened, row.id])} />
              {line}
            </div>
            {isOpen && <div class="rows-within">{within(row)}</div>}
          </Fragment>
        );
      })}
      {rows.length === 0 && <div class="faint rows-none">Nothing here yet.</div>}
    </div>
  );
}

/** A card a record. */
export function GalleryLayout({ fields, rows, people, named: others, onOpen }: Rows) {
  const title = titleField(fields);
  // What a record says at length, if it says anything: its other text.
  const words = fields.find((field) => field.kind === 'text' && field !== title);
  return (
    <div class="rows-gallery">
      {sorted(rows, fields, null).map((row) => (
        <button key={row.id} class="rows-card" onClick={() => onOpen(row)}>
          <strong>{said(title && row.data[title.name]) || row.id}</strong>
          {words && said(row.data[words.name]) && <span class="rows-card-words">{said(row.data[words.name])}</span>}
          <span class="rows-card-foot">
            <Aside fields={fields} row={row} people={people} others={others} />
          </span>
        </button>
      ))}
      {rows.length === 0 && <div class="faint rows-none">Nothing here yet.</div>}
    </div>
  );
}

const WEEK = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

interface CalendarProps extends Rows {
  /** What the records are put on the days of: a date field, or `created` or `updated`. */
  by: string;
  /** Puts a record on another day. Not given where the day is not the reader's to change. */
  onMove?: (row: Row, day: string) => void;
}

/** The records on the days one of their dates says, a month at a time. */
export function CalendarLayout({ fields, rows, by, onOpen, onMove }: CalendarProps) {
  const today = dayString(new Date());
  const [shown, setShown] = useState(() => ({ year: new Date().getFullYear(), month: new Date().getMonth() }));
  const [carried, setCarried] = useState<string | null>(null);
  const [over, setOver] = useState<string | null>(null);
  const title = titleField(fields);
  const named = (row: Row) => said(title && row.data[title.name]) || row.id;
  const on = new Map<string, Row[]>();
  const nowhere: Row[] = [];
  for (const row of sorted(rows, fields, null)) {
    const day = dayOf(row, by);
    if (day) on.set(day, [...(on.get(day) ?? []), row]);
    else nowhere.push(row);
  }
  const turn = (months: number) => {
    const next = new Date(shown.year, shown.month + months, 1);
    setShown({ year: next.getFullYear(), month: next.getMonth() });
  };
  const chip = (row: Row) => (
    <button
      key={row.id}
      class={classes('cal-card', carried === row.id && 'dragging')}
      draggable={!!onMove}
      title={named(row)}
      onDragStart={(event) => {
        event.dataTransfer?.setData('text/plain', row.id);
        setCarried(row.id);
      }}
      onDragEnd={() => {
        setCarried(null);
        setOver(null);
      }}
      onClick={() => onOpen(row)}
    >
      <span class="truncate">{named(row)}</span>
    </button>
  );
  const what = by === 'created' ? 'the day each was made' : by === 'updated' ? 'the day each was last changed' : heading({ name: by, kind: 'date', of: [], required: false }).toLowerCase();
  return (
    <div class="cal">
      <div class="cal-head">
        <h2 class="grow">{new Date(shown.year, shown.month, 1).toLocaleDateString(undefined, { month: 'long', year: 'numeric' })}</h2>
        <span class="faint">by {what}</span>
        <button class="ghost icon" aria-label="The month before" onClick={() => turn(-1)}>
          ‹
        </button>
        <button class="ghost" onClick={() => setShown({ year: new Date().getFullYear(), month: new Date().getMonth() })}>
          Today
        </button>
        <button class="ghost icon" aria-label="The month after" onClick={() => turn(1)}>
          ›
        </button>
      </div>
      <div class="cal-grid" role="grid" aria-label="Calendar">
        {WEEK.map((day) => (
          <div key={day} class="cal-weekday" role="columnheader">
            {day}
          </div>
        ))}
        {weeksOf(shown.year, shown.month)
          .flat()
          .map((day) => (
            <div
              key={day}
              class={classes('cal-day', Number(day.slice(5, 7)) - 1 !== shown.month && 'other', day === today && 'today', over === day && 'over')}
              role="gridcell"
              data-day={day}
              onDragOver={(event) => {
                if (!carried || !onMove) return;
                event.preventDefault();
                if (over !== day) setOver(day);
              }}
              onDrop={(event) => {
                event.preventDefault();
                const row = rows.find((one) => one.id === carried);
                setCarried(null);
                setOver(null);
                if (row && onMove && dayOf(row, by) !== day) onMove(row, day);
              }}
            >
              <span class="cal-date">{Number(day.slice(8))}</span>
              {(on.get(day) ?? []).map(chip)}
            </div>
          ))}
      </div>
      {nowhere.length > 0 && (
        <div class="cal-nowhere">
          {/* A finger cannot carry one: under it, a card is given its day where the card is opened. */}
          <span class="faint">{!onMove ? 'With no day:' : touched ? 'With no day yet; open one to give it one:' : 'With no day yet; carry one on to a day to give it one:'}</span>
          {nowhere.map(chip)}
        </div>
      )}
    </div>
  );
}
