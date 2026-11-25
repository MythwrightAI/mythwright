// An edge that is held and dragged, paced to what the page can draw.
//
// A pointer says where it is far more often than a page is drawn, and
// each time an edge used to tell whoever kept the width, which drew the
// whole view again: a channel with a few hundred things said in it was
// drawn again at every step of a drag, and the edge trailed the pointer.
// Held this way, the width goes straight onto the layout, at most once
// a frame with the last place winning, and whoever keeps the width is
// told once, when the edge is let go. What the width may be is worked
// out once as the drag begins, not at every step.

export interface Holding {
  /** The pointer is at `x` now. */
  move(x: number): void;
  /** The edge was let go: what was last asked for is drawn, and is what it came to. */
  letGo(): number;
  /** The drag was called off: it is as wide as when it was taken hold of. */
  callOff(): number;
}

export interface Held {
  /** How wide it was when it was taken hold of, and where the pointer was. */
  width: number;
  x: number;
  /** Which way makes it wider: 1 where the edge is its right, -1 where it is its left. */
  towards: 1 | -1;
  /** The width it may have for the one that was asked. */
  fit: (asked: number) => number;
  /** Puts a width straight onto the layout. */
  draw: (width: number) => void;
  /** Asks to be called at the next frame, and takes the asking back: the browser's, but for a test. */
  frame?: (work: () => void) => number;
  unframe?: (asked: number) => void;
}

/** Takes hold of an edge. Nothing is drawn until the pointer moves. */
export function hold(held: Held): Holding {
  const frame = held.frame ?? ((work) => requestAnimationFrame(work));
  const unframe = held.unframe ?? ((asked) => cancelAnimationFrame(asked));
  let wanted = held.width;
  let drawn = held.width;
  let asked: number | null = null;
  const draw = () => {
    asked = null;
    if (wanted === drawn) return;
    drawn = wanted;
    held.draw(drawn);
  };
  const settle = (width: number) => {
    if (asked !== null) unframe(asked);
    wanted = width;
    draw();
    return drawn;
  };
  return {
    move(x) {
      wanted = held.fit(held.width + (x - held.x) * held.towards);
      if (asked === null) asked = frame(draw);
    },
    letGo: () => settle(wanted),
    callOff: () => settle(held.width),
  };
}

/** What was in sight at the foot of a list that scrolls, and how far its foot was from the list's. */
interface Kept {
  list: Element;
  seen: Element;
  from: number;
}
let keeping: Kept[] = [];

/** How far the foot of what was in sight is from the foot of its list now. */
function footOf(one: Kept): number {
  return one.list.getBoundingClientRect().bottom - one.seen.getBoundingClientRect().bottom;
}

/**
 * Puts each list that scrolls back where it was read from: what was in
 * sight at its foot is at its foot again. What is out of sight in such a
 * list is not laid out (`inview.ts`), so its height is what it last was or
 * a guess, and the list would slip as an edge is held.
 */
export function keepPlace(): void {
  for (const one of keeping) {
    if (!one.seen.isConnected) continue;
    const slipped = footOf(one) - one.from;
    if (Math.abs(slipped) >= 1) one.list.scrollTop -= slipped;
  }
}

/**
 * While an edge is held the pointer keeps the shape of one over whatever
 * it crosses, nothing is selected on the way, and every list of things
 * said keeps its place: taken hold of, where each is read from is noted;
 * let go, each is put back there once everything is laid out again.
 */
export function holding(on: boolean): void {
  if (on) {
    keeping = [...document.querySelectorAll('.messages')].flatMap((list) => {
      const foot = list.getBoundingClientRect().bottom;
      // The last thing said that begins above the foot of the list is what is read at its foot.
      const seen = [...list.children].reverse().find((each) => each.getBoundingClientRect().top < foot);
      return seen ? [{ list, seen, from: foot - seen.getBoundingClientRect().bottom }] : [];
    });
  }
  document.documentElement.classList.toggle('edge-held', on);
  keepPlace();
  if (!on) keeping = [];
}
