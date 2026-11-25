// Changing a table's columns from the page: what the menu of a column's
// head opens for it (`headmenu.tsx`), and the form a column is added by.
//
// A table's columns are the people's who look at it as well as its
// blueprint's (`added.ts`): a column is added from the `+` after the
// last head and from Insert left and Insert right of a column's menu;
// any column is called something else in place, at the menu's head; one
// somebody added is made to hold another kind of thing, and is taken
// away behind a question that says what goes; and what a choice offers
// is added to, called something else, put in order and taken away in a
// list of the kind a cell's choice is made from, with no frame of its
// own. A declared column offers its label, its icon and the order of
// what it offers, and says whose it is.
//
// Everything here asks the daemon and draws nothing of its own accord:
// the table comes back as it then is, and every page that shows it
// follows what the daemon says (`collection.fields`). What is refused is
// said in the daemon's own sentence.

import { createPortal } from 'preact/compat';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { filling, goes, KINDS, kindCalled, labelled, mayShape, option, placing, refusedLabel } from '../added.ts';
import type { Place } from '../added.ts';
import { offered } from '../byhand.ts';
import { placed } from '../icons.ts';
import { colour } from '../pills.ts';
import { addColumn, callColumn, offerOptions, removeColumn, renameOption, retypeColumn } from '../shaped.ts';
import { attempt, collections, me, refreshCollections, tablesChangedHere } from '../store.ts';
import { heading } from '../table.ts';
import type { Field, Group, Row } from '../table.ts';
import { classes } from '../util.ts';
import type { Choosing } from './iconpicker.tsx';
import { KindMark, stroke } from './mark.tsx';

/** The table as the page last read it, where it has. */
const tableOf = (of: Choosing) => (collections.value[of.project] ?? []).find((one) => one.name === of.name);

/** Whether whoever is signed in may add a column to the table, retype one and take one away: never of a table whose rows are not written by hand (`byhand.ts`). */
export function useShaping(of: Choosing | undefined): boolean {
  if (!of) return false;
  const table = tableOf(of);
  return useColumnsChanged(of) && mayShape(me.value?.principal.scopes ?? [], table ?? { name: of.name });
}

/** Whether the table's columns are changed here at all, by anybody: called something else, what a choice offers, added and taken away. Not of a table whose rows are not written by hand. */
export function useColumnsChanged(of: Choosing | undefined): boolean {
  return !of || offered(tableOf(of)).columns;
}

/** Asks the daemon something of a table's columns, and has every page that shows the table draw it as it then is. */
async function asked<T>(of: Choosing, work: () => Promise<T>): Promise<T | undefined> {
  const answer = await attempt(work);
  if (answer === undefined) return undefined;
  // Every page hears of it and reads the table again; this one does not wait to be told.
  tablesChangedHere(of.project);
  void refreshCollections(of.project);
  of.then?.();
  return answer;
}

const MARKS: Record<string, string> = {
  type: 'M3 4.5h10M8 4.5v8M5.5 12.5h5',
  options: 'M2.5 4.5h7M2.5 8h11M2.5 11.5h5M12 3l1.5 1.5L12 6',
  takes: 'M5 7V5a3 3 0 0 1 6 0v2M3.5 7h9v6.5h-9z',
  'insert-left': 'M13.5 2.5v11M2.5 8h7M6 5v6',
  'insert-right': 'M2.5 2.5v11M6.5 8h7M10 5v6',
  duplicate: 'M5.5 5.5h8v8h-8zM2.5 10.5v-8h8',
  delete: 'M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5',
  back: 'M9.5 3.5 5 8l4.5 4.5',
  on: 'm3.5 8.5 3 3 6-7',
  off: 'M4 4l8 8M12 4l-8 8',
  rename: 'M3 13l.6-2.6L10.8 3.2l2 2L5.6 12.4zM9.6 4.4l2 2',
};
export const ShapeMark = ({ of }: { of: string }) => (
  <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true" {...stroke}>
    <path d={MARKS[of] ?? ''} />
  </svg>
);

const Back = ({ words, onBack }: { words: string; onBack: () => void }) => (
  <button type="button" class="headmenu-line headmenu-back" title="Back to what can be done with this column" onClick={onBack}>
    <ShapeMark of="back" />
    <strong class="grow truncate">{words}</strong>
  </button>
);

/** Keys that are the box's own while the caret is in it: the menu's arrows are not taken from somebody who is writing. */
const own = (event: KeyboardEvent) => {
  if (['Home', 'End', 'ArrowLeft', 'ArrowRight'].includes(event.key)) event.stopPropagation();
};

/**
 * What a column is called, written in place at the head of its menu:
 * Enter, or the caret leaving, calls it that for everybody; Escape
 * leaves it as it was. What cannot be a column's name is said under it
 * as it is written, and is not asked.
 */
export function NameBox({ of, field, fields, onDone }: { of: Choosing; field: Field; fields: Field[]; onDone: () => void }) {
  const was = heading(field);
  const [typed, setTyped] = useState(was);
  // Given up with Escape: what is written is not the answer.
  const left = useRef(false);
  const changed = typed.trim() !== was;
  const refused = changed ? refusedLabel(fields, typed, field.name) : null;
  const say = () => {
    const label = labelled(typed);
    if (left.current || !changed || refused || !label) return;
    left.current = true;
    void asked(of, () => callColumn(of.project, of.name, field.name, label));
  };
  return (
    <>
      <input
        class="headmenu-name grow"
        aria-label={`Name of ${was}`}
        aria-invalid={refused !== null}
        title={`What this column is called, for everybody. Blueprints and files go on calling it ${field.name}`}
        value={typed}
        maxLength={64}
        spellcheck={false}
        onInput={(event) => setTyped(event.currentTarget.value)}
        onKeyDown={(event) => {
          own(event);
          if (event.key === 'Escape') left.current = true;
          if (event.key !== 'Enter') return;
          event.preventDefault();
          if (refused) return;
          say();
          onDone();
        }}
        onBlur={say}
      />
      {refused && (
        <div class="headmenu-refused" role="alert">
          {refused}
        </div>
      )}
    </>
  );
}

/**
 * What a choice offers, as a list that is carried: each thing is a tag
 * on a row, taken by its grip to another place or moved with Alt and an
 * arrow. Where `edits`, one is called something else in place, taken
 * away, and added at the foot. It keeps nothing: each change is handed
 * over whole, and it draws what it is handed.
 */
export function OptionsList({ label, options, edits, onOffer, onRename }: { label: string; options: string[]; edits: boolean; onOffer: (next: string[]) => void; onRename: (from: string, to: string) => void }) {
  const list = useRef<HTMLDivElement>(null);
  const [typed, setTyped] = useState('');
  const [renaming, setRenaming] = useState<{ from: string; to: string } | null>(null);
  // While one is carried: the order as it would be, drawn here and told to nobody until it is let go.
  const [carried, setCarried] = useState<{ order: string[]; at: number } | null>(null);
  const shown = carried?.order ?? options;
  const moved = (order: string[], from: number, to: number) => {
    const next = [...order];
    const [one] = next.splice(from, 1);
    next.splice(to, 0, one!);
    return next;
  };
  const drag = (event: PointerEvent, from: number) => {
    const grip = event.currentTarget as HTMLElement;
    const rows = list.current;
    if (!rows) return;
    event.preventDefault();
    grip.setPointerCapture(event.pointerId);
    let at = from;
    let order = options;
    setCarried({ order, at });
    const onMove = (now: PointerEvent) => {
      const row = rows.querySelector<HTMLElement>('.select-option');
      const height = row?.getBoundingClientRect().height || 1;
      const to = Math.max(0, Math.min(order.length - 1, Math.floor((now.clientY - rows.getBoundingClientRect().top) / height)));
      if (to === at) return;
      order = moved(order, at, to);
      at = to;
      setCarried({ order, at });
    };
    const onUp = () => {
      grip.removeEventListener('pointermove', onMove);
      grip.removeEventListener('pointerup', onUp);
      grip.removeEventListener('pointercancel', onUp);
      setCarried(null);
      if (order.join('\n') !== options.join('\n')) onOffer(order);
    };
    grip.addEventListener('pointermove', onMove);
    grip.addEventListener('pointerup', onUp);
    grip.addEventListener('pointercancel', onUp);
  };
  const add = () => {
    const made = option(typed);
    setTyped('');
    if (made !== '' && !options.includes(made)) onOffer([...options, made]);
  };
  // The box is answered once: Enter takes it away, and the caret leaving it as it goes is no second answer.
  const answered = useRef(false);
  const rename = () => {
    if (!renaming || answered.current) return;
    answered.current = true;
    const to = option(renaming.to);
    setRenaming(null);
    if (to !== '' && to !== renaming.from && !options.includes(to)) onRename(renaming.from, to);
  };
  return (
    <div class="options-edit">
      <div ref={list} role="list" aria-label={`What ${label} offers`}>
        {shown.map((one, at) => (
          <div key={one} role="listitem" data-option={one} class={classes('select-option', carried?.at === at && 'dragged')}>
            <button
              type="button"
              class="select-grip options-grip"
              aria-label={`Move ${one}`}
              title="Drag to put it somewhere else in the order; or Alt and an arrow"
              disabled={options.length < 2}
              onPointerDown={(event) => drag(event, at)}
              onKeyDown={(event) => {
                const by = event.altKey ? { ArrowUp: -1, ArrowDown: 1 }[event.key] : undefined;
                if (by === undefined || at + by < 0 || at + by >= options.length) return;
                event.preventDefault();
                event.stopPropagation();
                onOffer(moved(options, at, at + by));
              }}
            >
              ⠿
            </button>
            {renaming?.from === one ? (
              <input
                class="options-word grow"
                aria-label={`What ${one} is called`}
                value={renaming.to}
                ref={(box) => box?.focus()}
                onInput={(event) => setRenaming({ from: one, to: event.currentTarget.value })}
                onKeyDown={(event) => {
                  own(event);
                  if (event.key === 'Enter') (event.preventDefault(), rename());
                  if (event.key === 'Escape') (event.stopPropagation(), (answered.current = true), setRenaming(null));
                }}
                onBlur={rename}
              />
            ) : (
              <span class="grow options-tag">
                <span class={`tag tag-${colour(one)}`}>
                  <span class="truncate">{one}</span>
                </span>
              </span>
            )}
            {edits && renaming?.from !== one && (
              <>
                <button type="button" class="options-act" aria-label={`Rename ${one}`} title="Call it something else; rows that chose it say the new word" onClick={() => ((answered.current = false), setRenaming({ from: one, to: one }))}>
                  <ShapeMark of="rename" />
                </button>
                <button type="button" class="options-act" aria-label={`Remove ${one}`} title="Remove it; one a row still chooses stays until the row says something else" disabled={options.length < 2} onClick={() => onOffer(options.filter((other) => other !== one))}>
                  <ShapeMark of="off" />
                </button>
              </>
            )}
          </div>
        ))}
      </div>
      {edits && (
        <input
          class="options-add"
          aria-label={`Add something ${label} offers`}
          placeholder="Add an option"
          value={typed}
          onInput={(event) => setTyped(event.currentTarget.value)}
          onKeyDown={(event) => {
            own(event);
            if (event.key !== 'Enter') return;
            event.preventDefault();
            event.stopPropagation();
            add();
          }}
          onBlur={add}
        />
      )}
    </div>
  );
}

/** What a choice there is offers, opened in its column's menu: each change asks the daemon, and the list is drawn from what the table then says. */
export function Options({ of, field, shapes, onBack }: { of: Choosing; field: Field; shapes: boolean; onBack: () => void }) {
  // The table as it is now, which is a moment ahead of the menu's own once something was asked.
  const now = tableOf(of)?.fields.find((one) => one.name === field.name) ?? field;
  const edits = shapes && now.added === true;
  return (
    <>
      <Back words={`What ${heading(field)} offers`} onBack={onBack} />
      <OptionsList
        label={heading(field)}
        options={now.of}
        edits={edits}
        onOffer={(next) => void asked(of, () => offerOptions(of.project, of.name, field.name, next))}
        onRename={(from, to) => void asked(of, () => renameOption(of.project, of.name, field.name, from, to))}
      />
      {!edits && <div class="headmenu-whose faint">{now.added ? 'Their order is yours to change. Adding to them takes changing the table’s columns.' : 'Their order is yours to change, for everybody. What it offers is said where the table is declared.'}</div>}
    </>
  );
}

/** What a kind needs said beside itself: what a choice offers, or the table a relation is of. */
function KindNeeds({ label, kind, said, tables, onSay }: { label: string; kind: string; said: string[]; tables: string[]; onSay: (next: string[]) => void }) {
  if (kind === 'choice') {
    return <OptionsList label={label} options={said} edits onOffer={onSay} onRename={(from, to) => onSay(said.map((one) => (one === from ? to : one)))} />;
  }
  if (kind === 'record' || kind === 'records') {
    return (
      <label class="colform-of">
        <span class="faint">Rows of</span>
        <select aria-label="The table whose rows it names" value={said[0] ?? ''} onChange={(event) => onSay(event.currentTarget.value ? [event.currentTarget.value] : [])}>
          <option value="">a table…</option>
          {tables.map((one) => (
            <option key={one} value={one}>
              {one}
            </option>
          ))}
        </select>
      </label>
    );
  }
  return null;
}

/** Whether a kind has been told what it needs. */
const told = (kind: string, said: string[]) => (kind === 'choice' || kind === 'record' || kind === 'records' ? said.length > 0 : true);

/** The kinds to choose from, one line each. */
function KindLines({ chosen, onChoose }: { chosen: string; onChoose: (kind: string) => void }) {
  return (
    <div class="headmenu-list" role="radiogroup" aria-label="What it holds">
      {KINDS.map((one) => (
        <button key={one.kind} type="button" class="headmenu-line" role="menuitemradio" aria-checked={chosen === one.kind} title={one.says} onClick={() => onChoose(one.kind)}>
          <KindMark kind={one.kind} />
          <span class="grow">{one.name}</span>
          {chosen === one.kind && <ShapeMark of="on" />}
        </button>
      ))}
    </div>
  );
}

/** The other tables of a project that whoever looks may name rows of. */
const tablesOf = (of: Choosing) => (collections.value[of.project] ?? []).map((one) => one.name);

/**
 * Changing what kind of thing a column somebody added holds, opened in
 * its menu: a kind is pressed and asked for, or, where it needs
 * something said (what a choice offers, the table a relation is of),
 * that is said first. The daemon refuses a kind some row does not fit,
 * in a sentence that says how many.
 */
export function Kinds({ of, field, onBack, onDone }: { of: Choosing; field: Field; onBack: () => void; onDone: () => void }) {
  const [kind, setKind] = useState<string | null>(null);
  const [said, setSaid] = useState<string[]>([]);
  const ask = async (to: string, with_: string[]) => {
    if ((await asked(of, () => retypeColumn(of.project, of.name, field.name, to, with_))) !== undefined) onDone();
  };
  const choose = (to: string) => {
    if (to === field.kind) return onBack();
    if (told(to, [])) return void ask(to, []);
    setKind(to);
    setSaid([]);
  };
  if (kind) {
    return (
      <>
        <Back words={`${heading(field)} as ${kindCalled(kind)}`} onBack={() => setKind(null)} />
        <KindNeeds label={heading(field)} kind={kind} said={said} tables={tablesOf(of)} onSay={setSaid} />
        <div class="colform-foot">
          <button type="button" class="primary" disabled={!told(kind, said)} onClick={() => void ask(kind, said)}>
            Change type
          </button>
        </div>
      </>
    );
  }
  return (
    <>
      <Back words={`${heading(field)} holds`} onBack={onBack} />
      <KindLines chosen={field.kind} onChoose={choose} />
      <div class="headmenu-whose faint">A kind is taken where every row fits it; what does not fit is said, and nothing is changed.</div>
    </>
  );
}

/** What filling a column takes, and whether whoever looks may: read from what the table and the field say, and nothing to press. */
export function Takes({ of, field, onBack }: { of: Choosing; field: Field; onBack: () => void }) {
  const table = tableOf(of) ?? { name: of.name };
  const { needs, may, notes } = filling(field, table, me.value?.principal.scopes ?? []);
  return (
    <>
      <Back words={`Filling ${heading(field)} takes`} onBack={onBack} />
      <div class="takes" role="list" aria-label={`What filling ${heading(field)} takes`}>
        {needs.map((one) => (
          <div key={one.words} role="listitem" class={classes('takes-line', one.held ? 'takes-held' : 'takes-lacking')}>
            <ShapeMark of={one.held ? 'on' : 'off'} />
            <span class="grow">
              {one.words}
              <span class="faint"> · {one.held ? 'you hold it' : 'you do not hold it'}</span>
            </span>
          </div>
        ))}
        {notes.map((note) => (
          <div key={note} role="listitem" class="takes-line">
            <span class="headmenu-tick" />
            <span class="grow faint">{note}</span>
          </div>
        ))}
      </div>
      <div class="headmenu-whose">{may ? 'You may fill it.' : 'You may not fill it.'}</div>
    </>
  );
}

/** The question a column is taken away behind: what goes with it, said before anything is asked of the daemon. */
export function Gone({ of, field, rows, onBack, onDone }: { of: Choosing; field: Field; rows: Row[]; onBack: () => void; onDone: () => void }) {
  return (
    <>
      <Back words={`Delete ${heading(field)}?`} onBack={onBack} />
      <div class="headmenu-ask" role="alert">
        {goes(field, rows)}
      </div>
      <div class="colform-foot">
        <button type="button" class="ghost" onClick={onBack}>
          Keep it
        </button>
        <button
          type="button"
          class="danger"
          onClick={async () => {
            if ((await asked(of, () => removeColumn(of.project, of.name, field.name))) !== undefined) onDone();
          }}
        >
          Delete column
        </button>
      </div>
    </>
  );
}

/** Adds a copy of a column beside it: the same kind and what it offers, and none of what its rows hold. */
export function duplicate(of: Choosing, field: Field, place: Place): void {
  void asked(of, () => addColumn(of.project, of.name, { like: field.name, place }));
}

interface AddProps {
  of: Choosing;
  /** The table's fields and groups, and the view the column is put in. */
  fields: Field[];
  groups?: Group[] | null;
  view: { name: string; kind: string; columns?: string[] | null };
  /** The column it stands beside, and on which side; after the last, where none is said. */
  beside: string | null;
  side: -1 | 1;
  /** What opened it, which it is put under and gives the caret back to. */
  opener: { current: HTMLElement | null };
  onClose: () => void;
}

/**
 * The form a column is added by: what it is called, what it holds, and
 * what that kind needs said. It is one panel with no frame on the
 * page's body, put under what opened it and kept wholly in the window,
 * as a column's menu is; it moves nothing, and is put away by Escape and
 * by a press outside it.
 */
export function AddColumn({ of, fields, groups, view, beside, side, opener, onClose }: AddProps) {
  const panel = useRef<HTMLDivElement>(null);
  const name = useRef<HTMLInputElement>(null);
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null);
  const [label, setLabel] = useState('');
  const [kind, setKind] = useState('text');
  const [said, setSaid] = useState<string[]>([]);
  const [asking, setAsking] = useState(false);
  const refused = label.trim() === '' ? null : refusedLabel(fields, label);
  const ready = labelled(label) !== null && refused === null && told(kind, said) && !asking;
  const close = (back: boolean) => {
    onClose();
    if (back) opener.current?.focus({ preventScroll: true });
  };
  const where = () => {
    const box = panel.current;
    const at = opener.current?.getBoundingClientRect() ?? { left: 8, top: 8, bottom: 8 };
    return placed(at, { width: box?.offsetWidth ?? 252, height: box?.offsetHeight ?? 0 }, { width: window.innerWidth, height: window.innerHeight });
  };
  useLayoutEffect(() => setPlace(where()), [kind, said.length, refused]);
  useEffect(() => name.current?.focus({ preventScroll: true }), [place === null]);
  useEffect(() => {
    const away = (event: Event) => {
      const inside = event.target instanceof Node && (panel.current?.contains(event.target) || opener.current?.contains(event.target));
      if (!inside) close(false);
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close(true);
    };
    const put = (event: Event) => {
      if (event.type === 'scroll' && event.target instanceof Node && panel.current?.contains(event.target)) return;
      setPlace(where());
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('keydown', key);
    window.addEventListener('resize', put);
    window.addEventListener('scroll', put, true);
    return () => {
      window.removeEventListener('pointerdown', away, true);
      window.removeEventListener('keydown', key);
      window.removeEventListener('resize', put);
      window.removeEventListener('scroll', put, true);
    };
  }, []);
  const add = async () => {
    const called = labelled(label);
    if (!ready || !called) return;
    setAsking(true);
    const done = await asked(of, () => addColumn(of.project, of.name, { label: called, kind, of: said, place: placing({ fields, groups }, view, beside, side) }));
    setAsking(false);
    if (done !== undefined) close(true);
  };
  return createPortal(
    <div
      ref={panel}
      class="headmenu colform"
      role="dialog"
      aria-label="Add a column"
      style={`left:${place?.left ?? 0}px;top:${place?.top ?? 0}px;${place ? '' : 'visibility:hidden'}`}
      onKeyDown={(event) => {
        if (event.key === 'Enter' && event.target === name.current) (event.preventDefault(), void add());
      }}
    >
      <input ref={name} class="headmenu-name" aria-label="What the column is called" aria-invalid={refused !== null} placeholder="Column name" maxLength={64} spellcheck={false} value={label} onInput={(event) => setLabel(event.currentTarget.value)} />
      {refused && (
        <div class="headmenu-refused" role="alert">
          {refused}
        </div>
      )}
      <div class="headmenu-rule" role="separator" />
      <KindLines
        chosen={kind}
        onChoose={(next) => {
          setKind(next);
          setSaid([]);
        }}
      />
      <KindNeeds label={label.trim() || 'it'} kind={kind} said={said} tables={tablesOf(of)} onSay={setSaid} />
      <div class="colform-foot">
        <span class="grow faint">For everybody who opens this table.</span>
        <button type="button" class="primary" disabled={!ready} onClick={() => void add()}>
          Add column
        </button>
      </div>
    </div>,
    document.body,
  );
}

/** The `+` after a table's last head, for whoever may add a column: pressed, the form is under it. */
export function AddHead({ of, fields, groups, view }: Omit<AddProps, 'beside' | 'side' | 'opener' | 'onClose'>) {
  const button = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  return (
    <>
      <button ref={button} type="button" class="grid-add" aria-label="Add a column" aria-haspopup="dialog" aria-expanded={open} title="Add a column to this table, for everybody who opens it" onClick={() => setOpen(!open)}>
        +
      </button>
      {open && <AddColumn of={of} fields={fields} groups={groups} view={view} beside={null} side={1} opener={button} onClose={() => setOpen(false)} />}
    </>
  );
}
