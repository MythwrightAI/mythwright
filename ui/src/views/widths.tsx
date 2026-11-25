// The edge at the right of a column's head, which is dragged to make the
// column wider or narrower (`widths.ts`).
//
// It is there only for whoever may save a view, and only where something
// points: a press on it is its own, never the head's (which opens the
// column's menu, or carries the column), since it is no part of the
// head's button. While it is held nobody is told and nothing is saved:
// the width goes onto the table once a frame (`held.ts`), by one rule
// over the column's own cells, and is saved when it is let go, from the
// view as it then is. The rule stays until the save is answered, so the
// column does not spring back while the view is on its way, and goes
// then whatever the answer: where the save was refused the column is
// what the view still says. Pressed twice it is as wide as what is in it
// again. The arrows make it a step wider or narrower, saved as each
// step lands; Escape while it is held puts it back.

import { useRef } from 'preact/hooks';
import { hold, type Holding, holding } from '../held.ts';
import type { Change } from '../table.ts';
import { dragged, drawnWhileHeld, heldRule, stepWidth, widens } from '../widths.ts';

let made = 0;

/** The size of text the table is drawn at, against the usual. */
function sizeOf(table: Element): number {
  const said = parseFloat(getComputedStyle(table).getPropertyValue('--size'));
  return Number.isFinite(said) && said > 0 ? said : 1;
}

interface Props {
  /** The column, by name, and what it is called at its head. */
  name: string;
  called: string;
  /** Which column it is among those drawn, from 0. */
  nth: number;
  /** How wide the view says it is, at the usual size, where it says. */
  width: number | null;
  /** Saves a change to the view: answered when it has been. */
  change: (change: Change) => unknown;
}

export function WidthEdge({ name, called, nth, width, change }: Props) {
  const held = useRef<{ holding: Holding; from: number; size: number; table: HTMLElement; rule: HTMLStyleElement | null; mark: string; off: () => void } | null>(null);
  /** The table the edge is in, and its head. */
  const placed = (edge: HTMLElement) => ({ head: edge.closest('th'), table: edge.closest('table') });
  /** Takes the rule away, and the mark it goes by. */
  const unruled = (table: HTMLElement, rule: HTMLStyleElement | null, mark: string) => {
    rule?.remove();
    if (table.dataset.widthHeld === mark) delete table.dataset.widthHeld;
  };
  const done = (edge: HTMLElement, at: { off: () => void }) => {
    at.off();
    edge.classList.remove('held');
    holding(false);
  };
  const save = (next: Change, then?: () => void) => {
    const answered = change(next);
    void Promise.resolve(answered).finally(() => then?.());
  };
  return (
    <div
      class="width-edge"
      role="separator"
      aria-orientation="vertical"
      aria-label={`Resize ${called}`}
      aria-valuenow={width ?? undefined}
      title="Drag to resize, double-click to reset"
      tabIndex={0}
      onPointerDown={(event) => {
        if (event.button !== 0 || event.pointerType === 'touch') return;
        // A press on the edge is the edge's alone: the head under it neither opens its menu nor carries the column.
        event.preventDefault();
        const edge = event.currentTarget;
        const { head, table } = placed(edge);
        if (!head || !table) return;
        const size = sizeOf(table);
        const from = head.getBoundingClientRect().width;
        const mark = String((made += 1));
        const at = { holding: null as unknown as Holding, from, size, table, rule: null as HTMLStyleElement | null, mark, off: () => {} };
        at.holding = hold({
          width: from,
          x: event.clientX,
          towards: 1,
          fit: (asked) => drawnWhileHeld(asked, size),
          draw: (drawn) => {
            // The rule over the column's own cells is made as it is first moved: a press that never moves draws nothing.
            if (!at.rule) {
              at.rule = document.createElement('style');
              document.head.append(at.rule);
              table.dataset.widthHeld = mark;
            }
            at.rule.textContent = heldRule(mark, nth + 1, drawn);
          },
        });
        // Escape puts it back, and is nobody else's while it is held.
        const key = (pressed: KeyboardEvent) => {
          if (pressed.key !== 'Escape' || held.current !== at) return;
          pressed.stopPropagation();
          pressed.preventDefault();
          at.holding.callOff();
          unruled(table, at.rule, mark);
          done(edge, at);
          held.current = null;
        };
        window.addEventListener('keydown', key, true);
        at.off = () => window.removeEventListener('keydown', key, true);
        held.current = at;
        edge.setPointerCapture(event.pointerId);
        edge.classList.add('held');
        holding(true);
      }}
      onPointerMove={(event) => held.current?.holding.move(event.clientX)}
      onPointerUp={(event) => {
        const at = held.current;
        if (!at) return;
        held.current = null;
        const drawn = at.holding.letGo();
        done(event.currentTarget, at);
        // Never moved, or back where it began: nobody is told.
        if (!at.rule || Math.abs(drawn - at.from) < 0.5) return unruled(at.table, at.rule, at.mark);
        save(widens(name, dragged(at.from, drawn - at.from, at.size)), () => unruled(at.table, at.rule, at.mark));
      }}
      onPointerCancel={(event) => {
        const at = held.current;
        if (!at) return;
        held.current = null;
        at.holding.callOff();
        unruled(at.table, at.rule, at.mark);
        done(event.currentTarget, at);
      }}
      onClick={(event) => event.stopPropagation()}
      onDblClick={(event) => {
        event.stopPropagation();
        if (width !== null) save(widens(name, null));
      }}
      onKeyDown={(event) => {
        const by = event.key === 'ArrowLeft' ? -1 : event.key === 'ArrowRight' ? 1 : 0;
        if (!by) return;
        event.preventDefault();
        event.stopPropagation();
        const { head, table } = placed(event.currentTarget);
        if (!head || !table) return;
        save(stepWidth(name, by, head.getBoundingClientRect().width / sizeOf(table)));
      }}
    />
  );
}
