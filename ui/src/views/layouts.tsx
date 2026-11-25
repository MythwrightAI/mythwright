// The views of a table, where its page is headed: a tab for each, a way
// to save another, and the panel that says how the one being looked at
// is laid out.
//
// Drawn from what a table is and knowing nothing of what its records are
// of, as `table.tsx` is, which these were part of until a column's head
// came to do more. The board's page and every other table's (`laid.tsx`)
// draw the same.

import type { Ref } from 'preact';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { holding } from '../ordered.ts';
import type { Holding, Only } from '../ordered.ts';
import { heading, LAYOUTS, titled } from '../table.ts';
import type { Field, Layout, View } from '../table.ts';
import { stroke } from './mark.tsx';
import { sideways } from './sideways.ts';
import { Tag, worded } from './tag.tsx';

/** The views saved of a table, a tab each, and a way to save another. */
export function ViewTabs({ views, current, ways, onSelect, onAdd }: { views: View[]; current: string; ways: Layout[]; onSelect: (name: string) => void; onAdd: (kind: Layout['kind']) => void }) {
  const [adding, setAdding] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!adding) return;
    const away = (event: Event) => {
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !box.current?.contains(event.target as Node)) setAdding(false);
    };
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', away);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', away);
    };
  }, [adding]);
  return (
    <div class="view-tabs">
      <div role="tablist" aria-label="Views" class="view-tabs-list" onWheel={sideways}>
        {views.map((view) => (
          <button key={view.name} role="tab" class="view-tab" aria-selected={view.name === current} onClick={() => onSelect(view.name)}>
            <LayoutMark kind={LAYOUTS.find((layout) => layout.kind === view.kind)?.kind ?? 'table'} />
            {titled(view.name)}
          </button>
        ))}
      </div>
      <div class="menu-anchor" ref={box}>
        <button class="ghost icon" aria-label="Add a view" title="Save another view of this: the same records, laid out another way" aria-haspopup="menu" aria-expanded={adding} onClick={() => setAdding(!adding)}>
          +
        </button>
        {adding && (
          <div class="menu view-add" role="menu">
            {ways.map((way) => (
              <button
                key={way.kind}
                role="menuitem"
                onClick={() => {
                  setAdding(false);
                  onAdd(way.kind);
                }}
              >
                <strong>{way.name}</strong>
                <span class="muted">{way.says}</span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export function LayoutMark({ kind }: { kind: Layout['kind'] }) {
  const d = {
    table: 'M2.5 3.5h11v9h-11zM2.5 6.5h11M2.5 9.5h11M6.5 3.5v9',
    board: 'M2.5 3h3v10h-3zM6.5 3h3v6.5h-3zM10.5 3h3v8.5h-3z',
    list: 'M5.5 4h8M5.5 8h8M5.5 12h8M2.5 4h.01M2.5 8h.01M2.5 12h.01',
    gallery: 'M2.5 2.5h4.5v4.5H2.5zM9 2.5h4.5v4.5H9zM2.5 9h4.5v4.5H2.5zM9 9h4.5v4.5H9z',
    calendar: 'M2.5 3.5h11v10h-11zM2.5 6.8h11M5.5 2v3M10.5 2v3M5.5 9.5h.01M8 9.5h.01M10.5 9.5h.01M5.5 11.5h.01M8 11.5h.01',
    order: 'M2 3h3.5v3H2zM2 10h3.5v3H2zM6.8 6.5h3v3h-3zM11 6.5h3v3h-3zM5.5 4.5h1.3v2M5.5 11.5h1.3v-2M9.8 8H11',
  }[kind];
  return (
    <svg width="18" height="18" viewBox="0 0 16 16" aria-hidden="true" {...stroke}>
      <path d={d} />
    </svg>
  );
}

export interface LayoutMenuProps {
  layouts: Layout[];
  layout: Layout['kind'];
  onLayout: (kind: Layout['kind']) => void;
  /** What a table can be told about how it is drawn: each by name, whether it is on, and what turns it. */
  options: [string, boolean, (on: boolean) => void][];
  /**
   * The columns this view can show or leave out, in the order they stand
   * in it, each with whether it is shown, and what shows or leaves one
   * out. What a row is known by is not among them: it is always there
   * and always first.
   */
  columns?: { field: Field; shown: boolean; up?: boolean; down?: boolean }[];
  onColumn?: (name: string, shown: boolean) => void;
  /** Moves a column a place among the others: up the list is to the left in the table. Each says whether it has a place to go (`up`, `down`). */
  onMove?: (name: string, by: -1 | 1) => void;
  /**
   * The fields this view's records can be gathered by, which of them
   * they are, and what gathers them by another or by none. Not given
   * where the table has no such field: then nothing is offered.
   */
  groups?: Field[];
  group?: string | null;
  onGroup?: (name: string | null) => void;
  /**
   * What this view can be held to: each field with the things there are
   * to hold it to, what it is held to now, and what holds it to more or
   * less. Not given where nothing can be: then nothing is offered.
   */
  holdings?: Holding[];
  only?: Only | null;
  onOnly?: (next: Only | null) => void;
  /** What the view being laid out is called, and what calls it something else. */
  name: string;
  onRename: (name: string) => void;
  /** Takes the view away. Not given for the only one there is. */
  onRemove?: () => void;
}

/**
 * How a view is laid out: the ways there are, and what can be said about
 * the one in use. Drawn in the menu of whatever opens it.
 */
export function LayoutPanel({ layouts, layout, onLayout, options, columns, onColumn, onMove, groups, group, onGroup, holdings, only, onOnly, name, onRename, onRemove, onClose, box }: LayoutMenuProps & { onClose: () => void; box?: Ref<HTMLDivElement> }) {
  const now = layouts.find((one) => one.kind === layout) ?? layouts[0]!;
  return (
    <div class="layout-menu" role="dialog" aria-label="Layout" ref={box}>
      <input
        class="layout-name"
        aria-label="What this view is called"
        defaultValue={titled(name)}
        key={name}
        onBlur={(event) => {
          const next = event.currentTarget.value.trim();
          if (next && next !== titled(name)) onRename(next);
        }}
        onKeyDown={(event) => event.key === 'Enter' && event.currentTarget.blur()}
      />
      <div class="layout-tiles" role="radiogroup" aria-label="Laid out as">
        {layouts.map((one) => (
          <button key={one.kind} class="layout-tile" role="radio" aria-checked={one.kind === now.kind} title={one.says} onClick={() => onLayout(one.kind)}>
            <LayoutMark kind={one.kind} />
            {one.name}
          </button>
        ))}
      </div>
      {now.kind === 'table' &&
        options.map(([name, on, turn]) => (
          <label key={name} class="layout-option">
            <span class="grow">{name}</span>
            <input type="checkbox" role="switch" class="switch" checked={on} onChange={(event) => turn(event.currentTarget.checked)} />
          </label>
        ))}
      {/* A board, an order and a table can gather their records, by a field that names something. */}
      {(now.kind === 'board' || now.kind === 'order' || now.kind === 'table') && groups && groups.length > 0 && onGroup && (
        <label class="layout-option">
          <span class="grow">Group by</span>
          <select aria-label="Group by" value={group ?? ''} onChange={(event) => onGroup(event.currentTarget.value || null)}>
            <option value="">nothing</option>
            {groups.map((field) => (
              <option key={field.name} value={field.name}>
                {heading(field)}
              </option>
            ))}
          </select>
        </label>
      )}
      {/* What the view is held to: only the records that hold one of the things turned on, in each field that has any on. */}
      {holdings && holdings.length > 0 && onOnly && (
        <div class="layout-columns" role="group" aria-label="Show only">
          <div class="layout-heading">
            Show only <span class="faint">{Object.keys(only ?? {}).length === 0 ? 'everything, until something here is on' : 'what holds one of those that are on'}</span>
          </div>
          <div class="layout-columns-all">
            {holdings.map(({ field, things }) => (
              <div key={field.name} role="group" aria-label={`Only by ${heading(field)}`}>
                <div class="faint layout-only">{heading(field)}</div>
                {things.map((thing) => (
                  <label key={thing} class="layout-option">
                    <span class="grow truncate">{worded(field) ? <Tag of={thing} small /> : thing}</span>
                    <input
                      type="checkbox"
                      role="switch"
                      class="switch"
                      aria-label={`Only ${heading(field)} ${thing}`}
                      checked={(only?.[field.name] ?? []).includes(thing)}
                      onChange={(event) => onOnly(holding(only, field.name, thing, event.currentTarget.checked))}
                    />
                  </label>
                ))}
              </div>
            ))}
          </div>
        </div>
      )}
      {/* Which columns this view shows. One left out is a switch away, and is no more private for being left out. */}
      {now.kind === 'table' && columns && columns.length > 0 && onColumn && (
        <div class="layout-columns" role="group" aria-label="Columns shown">
          <div class="layout-heading">
            Columns <span class="faint">{columns.filter((one) => one.shown).length} of {columns.length} shown</span>
          </div>
          {/* The switches scroll under what they are headed by, which says how many are on however far down they are. */}
          <div class="layout-columns-all">
            {columns.map(({ field, shown, up, down }) => (
              <div key={field.name} class="layout-option layout-column">
                <span class="grow truncate">{heading(field)}</span>
                {/* A column is carried by its head; here it is moved a place by a key, which a drag is not. */}
                {onMove && (
                  <>
                    <button type="button" class="ghost icon layout-move" disabled={!up} aria-label={`Move ${heading(field)} up`} title="A place earlier: to the left, in the table" onClick={() => onMove(field.name, -1)}>
                      ↑
                    </button>
                    <button type="button" class="ghost icon layout-move" disabled={!down} aria-label={`Move ${heading(field)} down`} title="A place later: to the right, in the table" onClick={() => onMove(field.name, 1)}>
                      ↓
                    </button>
                  </>
                )}
                <input type="checkbox" role="switch" class="switch" aria-label={heading(field)} checked={shown} onChange={(event) => onColumn(field.name, event.currentTarget.checked)} />
              </div>
            ))}
          </div>
        </div>
      )}
      {onRemove && (
        <button
          class="ghost layout-remove"
          onClick={() => {
            onClose();
            onRemove();
          }}
        >
          Remove this view
        </button>
      )}
    </div>
  );
}

/**
 * The layout's panel where a line of a menu opened it and no button of
 * its own stands by it: hung under the gear of a page's head
 * (`pagehead.tsx`). There is nothing beside it to press again, so it puts
 * itself away: on a press outside it, and on Escape, which is then for it
 * alone and goes no further (the frame would put a drawer away on the same
 * key), with the keyboard given back to what opened it.
 *
 * It is no taller than the window has room for under where it hangs, and
 * scrolls in itself past that: laid out as a table a view has its columns
 * to list, and a panel that ran off the foot of the window could not be
 * reached there.
 *
 * It stops listening in the moment it is taken off the page, and not a
 * little later as something that only listens may: a key it keeps from
 * everything else must not be kept once it is gone, or the Escape pressed
 * straight after it was put away, meant for the menu that took its place,
 * went to nobody.
 */
export function HungLayout({ from, onClose, ...layout }: LayoutMenuProps & { from: HTMLElement | null; onClose: () => void }) {
  const box = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const away = (event: Event) => !box.current?.contains(event.target as Node) && onClose();
    const key = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.stopPropagation();
      onClose();
      from?.focus();
    };
    const fit = () => {
      const panel = box.current;
      if (!panel) return;
      panel.style.maxHeight = `${Math.max(120, window.innerHeight - panel.getBoundingClientRect().top - 8)}px`;
      panel.style.overflowY = 'auto';
    };
    fit();
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', key, true);
    window.addEventListener('resize', fit);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', key, true);
      window.removeEventListener('resize', fit);
    };
  }, [from]);
  return <LayoutPanel {...layout} onClose={onClose} box={box} />;
}
