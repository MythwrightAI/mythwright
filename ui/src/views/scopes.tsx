// A cell that is a list of scopes (`scopes.ts`), drawn by area.
//
// In a table the cell says it short: the usual, or the areas something
// is held of. Pressed, it is what the Tools panel shows of anybody: a
// line an area, with what is held there and what is not, changed a line
// at a time (`views/holds.tsx`), in one panel over the cell. A row's form
// has the same under the field's name. What the row leaves unsaid is
// shown as what it comes to, and said to be that. Nobody is offered what
// they do not hold, and a row that already gives what they do not hold
// says so here, where a save would only have been refused.

import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { afterLine, briefly, comesTo, lacking, saidIn } from '../scopes.ts';
import { me, meta } from '../store.ts';
import { heading } from '../table.ts';
import type { Field } from '../table.ts';
import { classes } from '../util.ts';
import { HoldsTable } from './holds.tsx';

/** What the cell says, in a table: a few words, and quieter where the row itself says nothing. */
export function ScopesSaid({ field, value }: { field: Field; value: unknown }) {
  const unsaid = saidIn(value) === null;
  return (
    <span class={classes('scopes-said', unsaid && 'faint')} title={comesTo(field, value).join(', ') || undefined}>
      {briefly(field, value, meta.value?.scopes ?? [])}
    </span>
  );
}

/**
 * The list by area, a line an area. Where `onChange` is given a line is
 * changed where it is, and what is handed on is the whole list as it then
 * is: what the row left unsaid is written out by the first change.
 */
export function Scopes({ field, value, onChange }: { field: Field; value: unknown; onChange?: (next: string[]) => void }) {
  const [kept, setKept] = useState('');
  const said = saidIn(value);
  const usual = field.unsaid ?? [];
  const scopes = comesTo(field, value);
  const mine = me.value?.principal.scopes ?? [];
  const areas = meta.value?.scopes ?? [];
  const beyond = lacking(mine, scopes);
  const change = (area: string, verbs: string[]) => {
    const next = afterLine(field, value, areas, area, verbs);
    // An empty list is the row saying nothing, which is not holding nothing: it would be given the usual.
    if (next.length === 0 && usual.length > 0) return setKept(`A row that says nothing here is given the usual, so the last thing it holds is not taken away: nothing was changed.`);
    setKept('');
    onChange?.(next);
  };
  return (
    <div class="scopes">
      <HoldsTable scopes={scopes} onChange={onChange && beyond.length === 0 ? change : undefined} giver={mine} />
      <p class="faint holds-where">
        {beyond.length > 0
          ? `It gives ${beyond.join(', ')}, which you do not hold, so it is not yours to change: nobody gives what they do not hold.`
          : kept ||
            (said === null
              ? usual.length > 0
                ? `The row says nothing here, which comes to this: the usual.${onChange ? ' Change a line and the row says the whole of it.' : ''}`
                : 'The row says nothing here.'
              : 'As the row says.')}
        {onChange && beyond.length === 0 && said !== null && usual.length > 0 && (
          <>
            {' '}
            <button type="button" class="ghost link scopes-usual" onClick={() => onChange([])}>
              Say nothing again: the usual
            </button>
          </>
        )}
      </p>
    </div>
  );
}

/**
 * The cell being filled in a table: the list by area in one panel over
 * the cell, from its corner where there is room for that, which stays while
 * lines are changed and is put away by Escape, by a press outside it, or
 * by its own button.
 */
export function ScopesEdit({ field, value, called, onChange, onClose }: { field: Field; value: unknown; called: string; onChange: (next: string[]) => void; onClose: () => void }) {
  const box = useRef<HTMLDivElement>(null);
  // A line an area is a tall panel, and wider than most cells: begun at the cell's corner it would run past the
  // foot or the side of what the table scrolls in, from a row low down or a column far across. So it is moved
  // up and across over the cell as far as it has to be to be seen whole, and no further than the other edge.
  // Where what the table scrolls in is less tall than the panel, the panel is as tall as it and scrolls in itself.
  const [moved, setMoved] = useState<{ x: number; y: number; tall?: number }>({ x: 0, y: 0 });
  useLayoutEffect(() => {
    const at = box.current?.querySelector('.scopes-panel')?.getBoundingClientRect();
    const frame = box.current?.closest('.grid-scroll')?.getBoundingClientRect();
    if (!at) return;
    const [left, top] = [Math.max(8, frame?.left ?? 0), Math.max(8, frame?.top ?? 0)];
    const [right, bottom] = [Math.min(window.innerWidth - 8, frame?.right ?? Infinity), Math.min(window.innerHeight - 8, frame?.bottom ?? Infinity)];
    const tall = at.height > bottom - top ? Math.floor(bottom - top) : undefined;
    const below = at.top + (tall ?? at.height);
    setMoved({ x: Math.max(0, Math.min(at.right - right, at.left - left)), y: Math.max(0, Math.min(below - bottom, at.top - top)), tall });
  }, []);
  useEffect(() => {
    box.current?.querySelector<HTMLElement>('select, button')?.focus();
    const away = (event: Event) => {
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !box.current?.contains(event.target as Node)) onClose();
    };
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', away);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', away);
    };
  }, []);
  return (
    <div class="cell-edit scopes-edit" ref={box}>
      <div class="scopes-panel" role="dialog" aria-label={`${heading(field)} of ${called}`} style={{ left: `${-moved.x}px`, top: `${-moved.y}px`, ...(moved.tall === undefined ? {} : { maxHeight: `${moved.tall}px` }) }}>
        <Scopes field={field} value={value} onChange={onChange} />
        <div class="row">
          <span class="grow" />
          <button type="button" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
