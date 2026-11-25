// Testing, in a blueprint's head: one button and a menu of what tries a
// text without changing what runs. What the menu
// offers, what each line says and why one is closed is `testing.ts`; this
// draws it as the gear's menu is drawn (`RowMenu`: over the page, moving
// nothing, the keyboard in it as it opens and given back by Escape), and
// does what a line says.
//
// It is in the head from the first drawing, closed lines and all, and is
// never drawn a moment later: a button that came once the page knew more
// pushed Apply along from under a pointer that was about to press it.

import { useRef } from 'preact/hooks';
import { testingLines } from '../testing.ts';
import type { Standing, Tried } from '../testing.ts';
import { glyph, note, pressOutside } from './pagehead.tsx';
import type { Lines } from './pagehead.tsx';
import { offer, offeredBy, putAway } from './rowmenu.tsx';

/** How wide the menu is at the usual size of text, as the gear's is. */
const WIDE = 268;

/** Each line's drawing, so that the names begin together. */
const MARKS: Record<Tried, string> = { preview: '▷', sandbox: '◫', beside: '⇆', compare: '⚖' };

export interface TestingProps {
  standing: Standing;
  /** What pressing each line does. */
  does: Record<Tried, () => void>;
  /** How many things asked of this tab's sandbox, and of its texts side by side, were answered: said on the button, where it is whichever view is showing. */
  answered: number;
  besideAnswered: number;
}

export function Testing({ standing, does, answered, besideAnswered }: TestingProps) {
  const from = useRef<HTMLButtonElement>(null);
  const lines = testingLines(standing);
  if (lines.length === 0) return null;
  const open = () => {
    const button = from.current;
    if (!button) return;
    if (offeredBy(button)) return putAway();
    // Whatever else is open over the page is put away first.
    pressOutside();
    // Asked again as it is opened, so that what it says is how things are now.
    const items: Lines = testingLines(standing).flatMap((line, at) => [
      { label: line.label, mark: glyph(MARKS[line.does]), apart: at > 0, closed: line.closed, act: does[line.does] },
      note(line.note),
    ]);
    const at = button.getBoundingClientRect();
    // As much wider as text is larger, and never wider than the window.
    const size = Number.parseFloat(getComputedStyle(button).getPropertyValue('--size'));
    const wide = Math.min(Math.round(WIDE * Math.max(1, Number.isFinite(size) ? size : 1)), window.innerWidth - 8);
    offer({ x: at.right - wide, y: at.bottom + 4 }, items, { from: button, label: 'Testing', wide, fit: true });
  };
  return (
    <button
      ref={from}
      class="head-testing"
      aria-haspopup="menu"
      aria-expanded={offeredBy(from.current)}
      data-answered={answered}
      data-beside-answered={besideAnswered}
      onClick={open}
      title="Try this text without changing what runs: preview it, test it in a sandbox, play it side by side, or compare it on real work"
    >
      Testing <span aria-hidden="true">▾</span>
    </button>
  );
}
