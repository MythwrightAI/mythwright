// How often something was drawn again, counted.
//
// Nobody can feel a drag from a check, so what a drag costs is held by a
// number: a view that is not to be drawn again while an edge is held
// counts itself here, and the check that drags the edge reads the count
// before and after (`window.__drawn`). It is an addition and nothing
// more: nothing in the client reads it.

export const drawn: Record<string, number> = {};

/** Counts one more drawing of `what`, or `by` more. */
export function countDrawn(what: string, by = 1): void {
  drawn[what] = (drawn[what] ?? 0) + by;
}

if (typeof window !== 'undefined') (window as unknown as { __drawn: Record<string, number> }).__drawn = drawn;
