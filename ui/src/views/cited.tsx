// Sources, drawn (`cited.ts` reads and numbers them): a footnote's marker
// as a small raised number, which shows its source while it is pointed at
// or reached by the keyboard and, pressed, goes to it in the list of
// sources; and that list, each source with a way back to each place it is
// used. The list is drawn at the end of the text it is of, or, for a
// topic's post, at the end of its article as References
// (`articleend.tsx`). The editor draws its markers by the same card
// (`write/noted.ts`).
//
// Where a marker's source and a source's uses are is found in the page as
// it is drawn, not by an address: the client's addresses are its own, and
// the same label may be used in the post and in a reply beside it.

import type { ComponentChildren, RefObject } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { backMarks, NOT_WRITTEN } from '../cited.ts';
import type { Note, Notes } from '../cited.ts';
import { Floating } from './offered.tsx';

/** How wide a source's card is, where the window has the room. */
const CARD_WIDTH = 360;

/** The quoted form of a label, as a selector of the page holds it. */
const quoted = (label: string) => JSON.stringify(label);

/** Brings `found` into view and gives it the keyboard. */
function goTo(found: Element | null | undefined): void {
  if (!(found instanceof HTMLElement)) return;
  found.scrollIntoView({ block: 'center' });
  found.focus({ preventScroll: true });
}

/** Goes from a marker to its source: in the nearest list of sources around it that has one of its label. */
export function toNote(from: Element, label: string): void {
  for (let at = from.parentElement; at; at = at.parentElement) {
    const found = at.querySelector(`[data-note=${quoted(label)}]`);
    if (found) return goTo(found);
  }
}

/** The markers of a label around a list of sources: in the nearest part of the page around it that has any. */
export function usesNear(from: Element, label: string): Element[] {
  for (let at = from.parentElement; at; at = at.parentElement) {
    const found = [...at.querySelectorAll(`[data-cite=${quoted(label)}]`)];
    if (found.length > 0) return found;
  }
  return [];
}

/** A source, in a card over or under the marker it is of. */
export function SourceCard({ anchor, n, children, onEnter, onLeave }: { anchor: RefObject<HTMLElement | null>; n: number; children: ComponentChildren; onEnter?: () => void; onLeave?: () => void }) {
  return (
    <Floating
      beside={() => {
        const box = anchor.current?.getBoundingClientRect();
        return box ? { left: box.left - 12, above: box.top - 4, below: box.bottom + 4, first: 'over', across: CARD_WIDTH } : null;
      }}
      class="source-card"
      role="tooltip"
      label={`Source ${n}`}
    >
      <div class="source-card-in" onMouseEnter={onEnter} onMouseLeave={onLeave}>
        <span class="source-card-n">{n}</span>
        <span class="source-card-says">{children}</span>
      </div>
    </Floating>
  );
}

/**
 * Whether a card is shown for something pointed at or focused: shown at
 * once, and put away a moment after the pointer leaves, so that it can be
 * moved onto the card to press an address in it.
 */
export function useShownWhilePointed(): { open: boolean; show: () => void; hide: () => void; hideSoon: () => void } {
  const [open, setOpen] = useState(false);
  const leaving = useRef<number | null>(null);
  const stay = () => {
    if (leaving.current !== null) window.clearTimeout(leaving.current);
    leaving.current = null;
  };
  useEffect(() => stay, []);
  return {
    open,
    show: () => {
      stay();
      setOpen(true);
    },
    hide: () => {
      stay();
      setOpen(false);
    },
    hideSoon: () => {
      stay();
      leaving.current = window.setTimeout(() => setOpen(false), 200);
    },
  };
}

/** A footnote's marker: its number, raised; its source shown while it is pointed at or focused; pressed, its source in the list. */
export function Cite({ note, source }: { note: Note; source: ComponentChildren | null }) {
  const mark = useRef<HTMLButtonElement>(null);
  const card = useShownWhilePointed();
  return (
    <sup class="cite" data-cite={note.label}>
      <button
        type="button"
        ref={mark}
        class="cite-mark"
        aria-label={`Source ${note.n}`}
        aria-expanded={card.open}
        onMouseEnter={card.show}
        onMouseLeave={card.hideSoon}
        onFocus={card.show}
        onBlur={card.hide}
        onKeyDown={(event) => {
          if (event.key !== 'Escape' || !card.open) return;
          event.stopPropagation();
          card.hide();
        }}
        onClick={(event) => toNote(event.currentTarget, note.label)}
      >
        {note.n}
      </button>
      {card.open && (
        <SourceCard anchor={mark} n={note.n} onEnter={card.show} onLeave={card.hideSoon}>
          {source ?? <span class="faint">{NOT_WRITTEN}</span>}
        </SourceCard>
      )}
    </sup>
  );
}

/** The ways back from a source to each place it is used: an arrow, or a letter for each where it is used more than once. */
function Back({ note, uses }: { note: Note; uses: (from: Element, label: string) => Element[] }) {
  if (note.uses === 0) return null;
  const marks = backMarks(note.uses);
  return (
    <span class="note-back">
      {marks.map((mark, nth) => (
        <button
          type="button"
          class="ghost note-back-mark"
          key={nth}
          aria-label={marks.length === 1 ? `Back to where source ${note.n} is used` : `Back to use ${nth + 1} of source ${note.n}`}
          onClick={(event) => goTo(uses(event.currentTarget, note.label)[nth])}
        >
          {mark}
        </button>
      ))}
    </span>
  );
}

/**
 * The sources of a text, numbered as its markers are, each with what it
 * says drawn and a way back to where it is used. `uses` finds the markers
 * of a label; by default those of the nearest part of the page that has
 * any.
 */
export function NotesList({ notes, drawn, uses = usesNear, label = 'Sources' }: { notes: Notes; drawn: (note: Note & { text: string }) => ComponentChildren; uses?: (from: Element, label: string) => Element[]; label?: string }) {
  return (
    <ol class="prose-notes" aria-label={label}>
      {notes.list.map((note) => (
        <li key={note.label} value={note.n} data-note={note.label} tabIndex={-1}>
          <Back note={note} uses={uses} />
          <span class="note-says">{note.text !== null ? drawn({ ...note, text: note.text }) : <span class="faint">{NOT_WRITTEN}</span>}</span>
        </li>
      ))}
    </ol>
  );
}
