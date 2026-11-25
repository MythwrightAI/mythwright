// The head of a table's column: one button, and the menu of what can be
// done with the column.
//
// The whole head is pressed, or Enter or Space on it, for a menu under
// it; and held and moved sideways it is the column being carried to
// another place. The menu's first line is the column itself: its mark,
// which whoever may choose the column's icon presses for the icons
// (`iconpicker.tsx`), and its name. Under that, what is true of a column
// today. Sorting is whoever looks at the table's own, kept in their
// browser for the view, and is there for everybody. The rest change the
// view, which is the table's and everybody's, so they are there for
// whoever may save one: what the view is held to by the column, its rows
// gathered by it, a figure at its foot, the view frozen up to it, the
// column left out, and the column moved a place. Last, quietly, whose the
// column is. A table's columns are also the people's who look at it
// (`added.ts`, `shaping.tsx`): whoever may save a view calls a column
// something else in place at the menu's head and puts what a choice
// offers in order; whoever may change the table's columns inserts one on
// either side, copies one, and of a column somebody added changes what
// it holds and takes it away, behind a question. A declared column says
// what reads it, and offers none of what would change it.
//
// The menu is one panel with no frame, put on the page's body under the
// head and kept wholly in the window (`placed`): inside the table it
// would be cut off where the table scrolls. It moves nothing, stays as
// the pointer crosses to it, takes the arrows, and is put away by Escape,
// by a press outside it and by the head's own press. A change is made as
// every change of a view is: from the view as it is when the change is
// made, in its turn (`Change`, `inturn.ts`).

import { createPortal } from 'preact/compat';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { kindCalled, placing, shapes, whose } from '../added.ts';
import { carries, landing, lines, sortedBy, sortKept, stepped, steps } from '../columns.ts';
import type { Edge, Line } from '../columns.ts';
import { called, figuring, meaning, offered } from '../figures.ts';
import { placed } from '../icons.ts';
import { groupable, holdableBy, holding } from '../ordered.ts';
import { openTab, used } from '../store.ts';
import { heading, leavingOut, named as namedBy, shownIn } from '../table.ts';
import type { Change, Field, Group, Named, Row, Sort, View } from '../table.ts';
import { classes } from '../util.ts';
import { IconPick } from './iconpicker.tsx';
import type { Choosing } from './iconpicker.tsx';
import { FieldMark, stroke } from './mark.tsx';
import { whatIs } from './running.tsx';
import { AddColumn, duplicate, Gone, Kinds, NameBox, Options, ShapeMark, Takes, useColumnsChanged, useShaping } from './shaping.tsx';
import { Tag, worded } from './tag.tsx';

/** What the heads of a table's columns go by: the view being looked at, and what changes it. */
export interface Heads {
  view: View;
  /** Every field of the table as columns stand when nothing says otherwise, whatever the view leaves out. */
  fields: Field[];
  groups?: Group[] | null;
  /** Which table it is, and the instance whose blueprint declared it. */
  name: string;
  owner?: string | null;
  /** Where what a reader does with the view for themselves is kept: the project, the table and the view. */
  at: string;
  /** Every record of the table: what a view can be held to is what they hold. And what its fields that name records can name, to call each by. */
  rows: Row[];
  named?: Named;
  /** Saves a change to the view being looked at, answered once it has been. Asked only for whoever may save one. */
  change: (change: Change) => unknown;
}

/** What the reader sorted a view by, kept in their browser: theirs, and no part of the view. */
export function useSort(at: string | undefined): [Sort | null, (next: Sort | null) => void] {
  const read = (where: string | undefined): Sort | null => {
    try {
      return where ? sortKept(localStorage.getItem(`client.sort.${where}`)) : null;
    } catch {
      return null;
    }
  };
  const [kept, setKept] = useState(() => ({ at, sort: read(at) }));
  // Another view is being looked at: it has a sort of its own.
  const now = kept.at === at ? kept.sort : read(at);
  return [
    now,
    (next) => {
      setKept({ at, sort: next });
      try {
        if (at && next) localStorage.setItem(`client.sort.${at}`, JSON.stringify(next));
        else if (at) localStorage.removeItem(`client.sort.${at}`);
      } catch {
        /* not remembered */
      }
    },
  ];
}

const MARKS: Record<string, string> = {
  filter: 'M2.5 3.5h11L9.5 8.6v4.2l-3-1.4V8.6z',
  up: 'M4.5 13V3M2 5.5 4.5 3 7 5.5M9.5 4.5h4.5M9.5 8h3.2M9.5 11.5h2',
  down: 'M4.5 3v10M2 10.5 4.5 13 7 10.5M9.5 4.5h2M9.5 8h3.2M9.5 11.5H14',
  group: 'M2.5 3h11v3.5h-11zM4.5 9.5h9M4.5 12.5h9',
  figure: 'M11.5 3.5h-7L8.3 8l-3.8 4.5h7',
  freeze: 'M2.5 2.5h11v11h-11zM6.5 2.5v11M2.5 6.5l4-4M2.5 10.5l4-4M2.8 13.2l3.7-3.7',
  hide: 'M2 8s2.2-4 6-4c1 0 1.9.3 2.7.7M14 8s-2.2 4-6 4c-1 0-1.9-.3-2.7-.7M2.5 13.5l11-11',
  left: 'M13 8H3M6.5 4.5 3 8l3.5 3.5',
  right: 'M3 8h10M9.5 4.5 13 8l-3.5 3.5',
  back: 'M9.5 3.5 5 8l4.5 4.5',
  on: 'm3.5 8.5 3 3 6-7',
};
const LineMark = ({ of }: { of: string }) => (
  <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true" {...stroke}>
    <path d={MARKS[of] ?? ''} />
  </svg>
);

interface HeadProps {
  field: Field;
  /** It is what a record is known by: never left out, and never carried. */
  known: boolean;
  sort: Sort | null;
  onSort: (next: Sort | null) => void;
  /** The table, where whoever looks may save a view of it and choose its icons; nothing, where they may not. */
  choosing: Choosing | null;
  /** The table, whoever looks: what filling a column takes is said to everybody. */
  where?: Choosing;
  heads?: Heads;
  /** The columns drawn that this one may stand among, from left to right: those of its band but for what a record is known by. */
  among: string[];
}

/** A column's head: pressed for its menu, carried to another place. */
export function ColumnHead({ field, known, sort, onSort, choosing, where, heads, among }: HeadProps) {
  const [open, setOpen] = useState(false);
  const [picking, setPicking] = useState(false);
  // A column is being added beside this one: on which side.
  const [inserting, setInserting] = useState<-1 | 1 | null>(null);
  const shaping = useShaping(where);
  const button = useRef<HTMLButtonElement>(null);
  // A press that carried the column is no press for the menu.
  const carriedNow = useRef(false);
  const may = choosing !== null && heads !== undefined;
  const mine = sort?.field === field.name ? sort : null;

  /** Taken hold of: moved sideways past a few pixels it is carried, and let go it stands where the line was. */
  const hold = (event: PointerEvent) => {
    carriedNow.current = false;
    const head = button.current?.closest('th');
    const row = head?.parentElement;
    if (!may || known || event.button !== 0 || event.pointerType === 'touch' || !head || !row) return;
    const from = event.clientX;
    let carrying: { edges: Edge[]; line: HTMLElement; x: number; asked: number | null } | null = null;
    const draw = () => {
      if (!carrying) return;
      carrying.asked = null;
      const to = landing(carrying.edges, carrying.x);
      if (to) carrying.line.style.left = `${to.at - 1}px`;
    };
    const begin = () => {
      // Where each head it may stand among is, worked out once as it is taken up.
      const edges = among.flatMap((name): Edge[] => {
        const box = [...row.children].find((one) => (one as HTMLElement).dataset.column === name)?.getBoundingClientRect();
        return box ? [{ name, left: box.left, right: box.right }] : [];
      });
      // The line that says where it would land lies over the page: nothing under the pointer moves for it.
      const line = document.createElement('div');
      line.className = 'grid-drop';
      const top = head.getBoundingClientRect().top;
      // Down the table as far as it is in sight.
      const foot = Math.min(head.closest('.grid-scroll')?.getBoundingClientRect().bottom ?? top + 34, head.closest('table')?.getBoundingClientRect().bottom ?? top + 34);
      line.style.top = `${top}px`;
      line.style.height = `${Math.max(34, foot - top)}px`;
      document.body.append(line);
      head.classList.add('head-carried');
      return { edges, line, x: from, asked: null };
    };
    const over = (drop: boolean) => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('pointercancel', off);
      window.removeEventListener('keydown', key, true);
      if (!carrying) return;
      if (carrying.asked !== null) cancelAnimationFrame(carrying.asked);
      carrying.line.remove();
      head.classList.remove('head-carried');
      const to = drop ? landing(carrying.edges, carrying.x) : null;
      carrying = null;
      carriedNow.current = true;
      // Nobody is told until it is let go; back where it began, nothing is told at all.
      if (to && to.before !== field.name) heads.change(carries(heads.groups, field.name, to.before));
    };
    const move = (moved: PointerEvent) => {
      if (!carrying) {
        if (Math.abs(moved.clientX - from) < 6) return;
        carrying = begin();
      }
      carrying.x = moved.clientX;
      if (carrying.asked === null) carrying.asked = requestAnimationFrame(draw);
    };
    const up = () => over(true);
    const off = () => over(false);
    const key = (pressed: KeyboardEvent) => {
      if (pressed.key !== 'Escape' || !carrying) return;
      // Escape puts it back, and is nobody else's while it is carried.
      pressed.stopPropagation();
      pressed.preventDefault();
      over(false);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    window.addEventListener('pointercancel', off);
    window.addEventListener('keydown', key, true);
  };

  return (
    <>
      <button
        ref={button}
        type="button"
        class="grid-head"
        aria-haspopup="menu"
        aria-expanded={open}
        title={may && !known ? 'What can be done with this column; held and moved, it is carried to another place' : 'What can be done with this column'}
        onPointerDown={hold}
        onClick={(event) => {
          // A press that carried the column opens nothing; one from the keyboard never carried anything.
          if (carriedNow.current && event.detail !== 0) return void (carriedNow.current = false);
          if (picking) return setPicking(false);
          if (inserting !== null) return setInserting(null);
          setOpen(!open);
        }}
      >
        <FieldMark field={field} title={known} />
        <span class="truncate">{heading(field)}</span>
        {mine && <span class="faint">{mine.down ? '↓' : '↑'}</span>}
      </button>
      {open && (
        <HeadMenu
          field={field}
          known={known}
          sort={sort}
          onSort={onSort}
          choosing={choosing}
          where={where}
          shaping={shaping}
          heads={heads}
          opener={button}
          onPick={() => {
            setOpen(false);
            setPicking(true);
          }}
          onInsert={(side) => {
            setOpen(false);
            setInserting(side);
          }}
          onClose={(back) => {
            setOpen(false);
            if (back) button.current?.focus({ preventScroll: true });
          }}
        />
      )}
      {picking && choosing && <IconPick of={choosing} field={field} opener={button} onClose={() => setPicking(false)} />}
      {inserting !== null && choosing && heads && <AddColumn of={choosing} fields={heads.fields} groups={heads.groups} view={heads.view} beside={field.name} side={inserting} opener={button} onClose={() => setInserting(null)} />}
    </>
  );
}

interface MenuProps extends Omit<HeadProps, 'among'> {
  opener: { current: HTMLElement | null };
  /** The icons are asked for: the menu is put away and they are opened under the head. */
  onPick: () => void;
  /** Whether whoever looks may change what columns the table has. */
  shaping: boolean;
  /** A column is to be added beside this one: the menu is put away and the form opened under the head. */
  onInsert: (side: -1 | 1) => void;
  /** Put away; `back` where the caret goes back to the head. */
  onClose: (back: boolean) => void;
}

const WIDE = 252;

function HeadMenu({ field, known, sort, onSort, choosing, where: of, shaping, heads, opener, onPick, onInsert, onClose }: MenuProps) {
  const panel = useRef<HTMLDivElement>(null);
  // What is opened from a line in the menu's place: what the view is held to here, the figures, and what changes the column itself.
  const [within, setWithin] = useState<'filter' | 'figure' | 'type' | 'options' | 'takes' | 'delete' | null>(null);
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null);
  const name = heading(field);
  const may = choosing !== null && heads !== undefined;
  const view = heads?.view;
  const table = heads ? { fields: heads.fields, groups: heads.groups, order: view?.columns } : null;
  const shown = heads && view ? shownIn(view, heads.fields).map((one) => one.name) : [];
  const holds = heads ? holdableBy(field, heads.rows, view?.only) : null;
  const gathers = heads !== undefined && view?.kind === 'table' && groupable(heads.fields, heads.name, null, view.group ?? null, heads.rows).some((one) => one.name === field.name);
  const all = lines({
    may,
    known,
    holds: !holds ? 'never' : holds.things.length > 0 ? 'things' : 'unshared',
    gathers,
    left: table !== null && stepped(table, shown, field.name, -1) !== undefined,
    right: table !== null && stepped(table, shown, field.name, 1) !== undefined,
  });
  const mine = sort?.field === field.name ? sort : null;
  const held = view?.only?.[field.name] ?? [];
  const figure = view?.figures?.[field.name] ?? null;
  const owner = heads?.owner ? whatIs(heads.owner) : undefined;
  // What changes the table's columns, for whoever may: `added.ts` says which lines there are. A table whose rows are
  // not written by hand (`byhand.ts`) has none of them, its name included: the rest is how somebody looks at it.
  const columns = useColumnsChanged(of);
  const shaped = shapes({ may: may && columns, shapes: shaping && may, added: field.added === true, kind: field.kind, known });
  const declares = owner && owner !== 'gone' ? owner.blueprint : null;
  const its = whose(field, declares);
  /** Opens what declares the table: the blueprint's own text where it is a file of the project, else what is running of it. */
  const openIts = () => {
    const file = declares && of ? used.value[of.project]?.own.find((one) => one.name === declares)?.file : undefined;
    if (file && of) openTab({ kind: 'file', project: of.project, path: file });
    else if (heads?.owner) openTab({ kind: 'instance', id: heads.owner });
    onClose(false);
  };

  const where = () => {
    const box = panel.current;
    const size = { width: box?.offsetWidth ?? WIDE, height: box?.offsetHeight ?? 0 };
    const at = opener.current?.getBoundingClientRect() ?? { left: 8, top: 8, bottom: 8 };
    return placed(at, size, { width: window.innerWidth, height: window.innerHeight });
  };
  // Put where it is whole in the window once it is known how tall it is, and again when what is in it changes or its column is moved from under it.
  useLayoutEffect(() => setPlace(where()), [within, all.join(' '), shaped.join(' '), field.kind, field.of.join(' '), view?.columns?.join(' ')]);
  // And again whenever it comes to be another height by itself (a sentence said under a name, words that wrap where the
  // text is large): as soon as what is in it changes, in the moment after, and wherever else the browser says it was
  // laid out another height. Told only that, it was put right a frame later, and stood past the window's foot until
  // then.
  useLayoutEffect(() => {
    const box = panel.current;
    if (!box || typeof ResizeObserver === 'undefined' || typeof MutationObserver === 'undefined') return;
    // Put again only where it is to stand elsewhere: put again as it stands, it is drawn again, and what that changes
    // in it would be told here once more.
    const again = () => {
      const next = where();
      setPlace((was) => (was && was.left === next.left && was.top === next.top ? was : next));
    };
    const changed = new MutationObserver(again);
    changed.observe(box, { childList: true, subtree: true, characterData: true });
    const resized = new ResizeObserver(again);
    resized.observe(box);
    return () => {
      changed.disconnect();
      resized.disconnect();
    };
  }, []);
  const items = () => [...(panel.current?.querySelectorAll<HTMLElement>('button:not([disabled])') ?? [])];
  // The caret is in the menu from when it opens, on its first line that does something, and on the way back when a list is opened.
  useEffect(() => {
    const first = panel.current?.querySelector<HTMLElement>(within ? 'button' : '[role^="menuitem"]');
    first?.focus({ preventScroll: true });
  }, [within, place === null]);

  useEffect(() => {
    const inside = (target: EventTarget | null) => target instanceof Node && (panel.current?.contains(target) || opener.current?.contains(target));
    // What opened it closes it by its own press; anything else outside it does here.
    const away = (event: Event) => {
      // The caret goes back to the head where it was in the menu; a press that lands on something takes it from there by itself.
      if (!inside(event.target)) onClose(panel.current?.contains(document.activeElement) ?? false);
    };
    const put = (event: Event) => {
      if (event.type === 'scroll' && event.target instanceof Node && panel.current?.contains(event.target)) return;
      setPlace(where());
    };
    // Escape puts it away wherever the caret is, and gives the caret back to the head.
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose(true);
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

  const onKey = (event: KeyboardEvent) => {
    // Where something is written in it (what a choice offers, a kind's needs), Tab goes from one box to the next.
    if (event.key === 'Tab' && (within === 'options' || within === 'type' || within === 'delete')) return;
    if (event.key === 'Tab') return onClose(false);
    const lines = items();
    const at = lines.indexOf(document.activeElement as HTMLElement);
    const to = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: lines.length - 1 }[event.key];
    if (to !== undefined) {
      event.preventDefault();
      lines[(to + lines.length) % lines.length]?.focus({ preventScroll: true });
    } else if (event.key === 'ArrowLeft' && within) {
      event.preventDefault();
      setWithin(null);
    } else if (event.key === 'ArrowRight' && document.activeElement?.getAttribute('aria-haspopup') === 'true') {
      event.preventDefault();
      (document.activeElement as HTMLElement).click();
    }
  };

  /** A thing a view is held to, as it is read: a record by what it is known by, somebody by their name. */
  const saidOf = (thing: string) => (field.kind === 'record' || field.kind === 'records' ? (namedBy(field, thing, heads?.named ?? {})?.words ?? thing) : field.kind === 'actor' ? thing.slice(thing.indexOf(':') + 1) : thing);
  const change = (next: Change, stay = false) => {
    heads?.change(next);
    if (!stay) onClose(true);
  };
  const same = (now: View) => ({ name: now.name, kind: now.kind });
  /** One line of the menu: its mark, its words, and at its end what is so now or that it opens a list. */
  const line = (which: Line) => {
    if (which === 'unshared') {
      return (
        <div key={which} class="headmenu-line headmenu-said" title="A view is held to the rows that hold one of some things; here every row says something of its own, or nothing">
          <LineMark of="filter" />
          <span class="grow">Filter</span>
          <span class="faint">no two rows share a thing here</span>
        </div>
      );
    }
    const sorts = which === 'up' || which === 'down';
    const on = which === 'up' ? mine?.down === false : which === 'down' ? mine?.down === true : which === 'group' ? view?.group === field.name : which === 'freeze' ? view?.freezes === field.name : false;
    const words = {
      filter: 'Filter',
      up: 'Sort ascending',
      down: 'Sort descending',
      group: on ? 'Ungroup' : 'Group',
      figure: 'Calculate',
      freeze: on ? 'Unfreeze' : 'Freeze up to here',
      hide: 'Hide',
      left: 'Move left',
      right: 'Move right',
    }[which];
    const says = {
      filter: 'Show only the rows that hold one of some things here; the view is saved so',
      up: on ? 'Sorted so now; pressed again, the rows are as they were added. It is your own look, kept in this browser' : 'Least first. It is your own look, kept in this browser',
      down: on ? 'Sorted so now; pressed again, the rows are as they were added. It is your own look, kept in this browser' : 'Greatest first. It is your own look, kept in this browser',
      group: on ? 'The rows are gathered by this column; pressed, they are gathered by none' : 'Gather the rows under a line for each thing this column says; the view is saved so',
      figure: 'A figure at the foot of this column, of the rows the view shows',
      freeze: on ? 'The view is frozen up to this column; pressed, nothing is' : 'This column and those before it stay while the rest are scrolled sideways; the view is saved so',
      hide: 'Leave this column out of this view; Layout shows it again',
      left: 'Move this column one place to the left; the view is saved so',
      right: 'Move this column one place to the right; the view is saved so',
    }[which];
    const press = {
      filter: () => setWithin('filter'),
      up: () => (onSort(sortedBy(sort, field.name, false)), onClose(true)),
      down: () => (onSort(sortedBy(sort, field.name, true)), onClose(true)),
      group: () => change((now) => ({ ...same(now), group: now.group === field.name ? null : field.name })),
      figure: () => setWithin('figure'),
      freeze: () => change((now) => ({ ...same(now), freezes: now.freezes === field.name ? null : field.name })),
      hide: () => change((now, fields) => ({ ...same(now), hides: leavingOut(now, fields, field.name, false) })),
      left: () => change(steps(heads?.groups, field.name, -1), true),
      right: () => change(steps(heads?.groups, field.name, 1), true),
    }[which];
    const opens = which === 'filter' || which === 'figure';
    const now = which === 'filter' ? (held.length > 0 ? `${held.length} on` : '') : which === 'figure' ? (figure ? called(figure) : '') : '';
    return (
      <button key={which} type="button" class="headmenu-line" role={sorts || which === 'group' || which === 'freeze' ? 'menuitemcheckbox' : 'menuitem'} aria-checked={sorts || which === 'group' || which === 'freeze' ? on : undefined} aria-haspopup={opens ? 'true' : undefined} title={says} onClick={press}>
        <LineMark of={which} />
        <span class="grow">{words}</span>
        {now && <span class="faint">{now}</span>}
        {sorts && on && <LineMark of="on" />}
        {opens && (
          <span class="faint" aria-hidden="true">
            ›
          </span>
        )}
      </button>
    );
  };
  /** A line that changes the column itself, or opens what does. */
  const shape = (mark: string, words: string, says: string, press: () => void, now = '', opens = false) => (
    <button key={mark} type="button" class={classes('headmenu-line', mark === 'delete' && 'headmenu-danger')} role="menuitem" aria-haspopup={opens ? 'true' : undefined} title={says} onClick={press}>
      <ShapeMark of={mark} />
      <span class="grow">{words}</span>
      {now && <span class="faint">{now}</span>}
      {opens && (
        <span class="faint" aria-hidden="true">
          ›
        </span>
      )}
    </button>
  );
  const back = (words: string) => (
    <button type="button" class="headmenu-line headmenu-back" title="Back to what can be done with this column" onClick={() => setWithin(null)}>
      <LineMark of="back" />
      <strong class="grow truncate">{words}</strong>
    </button>
  );

  return createPortal(
    <div
      ref={panel}
      class="headmenu"
      role="menu"
      aria-label={`What can be done with ${name}`}
      style={`left:${place?.left ?? 0}px;top:${place?.top ?? 0}px;${place ? '' : 'visibility:hidden'}`}
      onKeyDown={onKey}
    >
      {within === 'filter' && holds && (
        <>
          {back(`Only rows whose ${name.toLowerCase()} is`)}
          <div class="headmenu-list">
            {holds.things.map((thing) => (
              <button key={thing} type="button" class="headmenu-line" role="menuitemcheckbox" aria-checked={held.includes(thing)} aria-label={`Only ${name} ${thing}`} onClick={() => change((now) => ({ ...same(now), only: holding(now.only, field.name, thing, !(now.only?.[field.name] ?? []).includes(thing)) }), true)}>
                <span class="headmenu-tick">{held.includes(thing) && <LineMark of="on" />}</span>
                <span class="grow truncate">{worded(field) ? <Tag of={thing} small /> : saidOf(thing)}</span>
              </button>
            ))}
          </div>
          {held.length > 0 && (
            <button type="button" class="headmenu-line" role="menuitem" title="Show every row again, whatever it holds here" onClick={() => change((now) => ({ ...same(now), only: Object.fromEntries(Object.entries(now.only ?? {}).filter(([one]) => one !== field.name)) }), true)}>
              <span class="headmenu-tick" />
              <span class="grow faint">Show all</span>
            </button>
          )}
        </>
      )}
      {within === 'figure' && (
        <>
          {back(`At the foot of ${name}`)}
          <button type="button" class="headmenu-line" role="menuitemradio" aria-checked={figure === null} onClick={() => change((now) => ({ ...same(now), figures: figuring(now.figures, field.name, null) }))}>
            <span class="headmenu-tick">{figure === null && <LineMark of="on" />}</span>
            <span class="grow">Nothing</span>
          </button>
          {offered(field).map((one) => (
            <button key={one} type="button" class="headmenu-line" role="menuitemradio" aria-checked={figure === one} title={meaning(one)} onClick={() => change((now) => ({ ...same(now), figures: figuring(now.figures, field.name, one) }))}>
              <span class="headmenu-tick">{figure === one && <LineMark of="on" />}</span>
              <span class="grow">{called(one)}</span>
            </button>
          ))}
        </>
      )}
      {within === null && (
        <>
          <div class="headmenu-column">
            {choosing ? (
              <button type="button" class="headmenu-icon" aria-label={`Icon of ${name}`} title="Choose the icon this column is drawn with" aria-haspopup="dialog" onClick={onPick}>
                <FieldMark field={field} title={known} />
              </button>
            ) : (
              <span class="headmenu-icon">
                <FieldMark field={field} title={known} />
              </span>
            )}
            {choosing && heads && columns ? <NameBox key={name} of={choosing} field={field} fields={heads.fields} onDone={() => onClose(true)} /> : <strong class="grow truncate">{name}</strong>}
          </div>
          {shaped.includes('type') && shape('type', 'Change type', 'What kind of thing this column holds; a kind is taken where every row fits it', () => setWithin('type'), kindCalled(field.kind), true)}
          {shaped.includes('options') && shape('options', 'Options', 'What this column offers to choose from, and in what order', () => setWithin('options'), String(field.of.length), true)}
          <div class="headmenu-rule" role="separator" />
          {all.filter((one) => one !== 'left' && one !== 'right').map(line)}
          {(all.some((one) => one === 'left' || one === 'right') || shaped.some((one) => one !== 'type' && one !== 'options')) && <div class="headmenu-rule" role="separator" />}
          {all.filter((one) => one === 'left' || one === 'right').map(line)}
          {shaped.includes('insert-left') && shape('insert-left', 'Insert left', 'Add a column before this one, for everybody who opens this table', () => onInsert(-1))}
          {shaped.includes('insert-right') && shape('insert-right', 'Insert right', 'Add a column after this one, for everybody who opens this table', () => onInsert(1))}
          {shaped.includes('duplicate') && choosing && heads && shape('duplicate', 'Duplicate column', 'Add a column beside this one that holds the same kind of thing and offers the same; none of what the rows say here is copied', () => (duplicate(choosing, field, placing({ fields: heads.fields, groups: heads.groups }, heads.view, field.name, 1)), onClose(true)))}
          {shaped.includes('delete') && (
            <>
              <div class="headmenu-rule" role="separator" />
              {shape('delete', 'Delete column', 'Remove this column for everybody, with what the rows say in it; asked about first', () => setWithin('delete'))}
            </>
          )}
          {heads && (
            <>
              <div class="headmenu-rule" role="separator" />
              {of && shape('takes', 'What filling it takes', 'What it takes to give this column a value, and whether you may', () => setWithin('takes'), '', true)}
              {!field.added && heads.owner && declares ? (
                <button type="button" class={classes('headmenu-line', 'headmenu-whose')} role="menuitem" title="Open what declares this table: its columns are said there" onClick={openIts}>
                  <span class="grow faint">{its.words}</span>
                  <span class="faint" aria-hidden="true">
                    ›
                  </span>
                </button>
              ) : (
                <div class={classes('headmenu-whose', 'faint')}>{its.words}</div>
              )}
              <div class={classes('headmenu-whose', 'headmenu-called', 'faint')}>{its.more}</div>
            </>
          )}
        </>
      )}
      {within === 'type' && choosing && <Kinds of={choosing} field={field} onBack={() => setWithin(null)} onDone={() => onClose(true)} />}
      {within === 'options' && choosing && <Options of={choosing} field={field} shapes={shaping} onBack={() => setWithin(null)} />}
      {within === 'takes' && of && <Takes of={of} field={field} onBack={() => setWithin(null)} />}
      {within === 'delete' && choosing && heads && <Gone of={choosing} field={field} rows={heads.rows} onBack={() => setWithin(null)} onDone={() => onClose(false)} />}
    </div>,
    document.body,
  );
}
