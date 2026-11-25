// Which of a table's columns stay while the rest are scrolled sideways.
//
// A view frozen up to a column keeps that one and those before it where
// they are. A column that stays is told where it begins across the table
// (the widths of those before it, added up), and a column is as wide as
// what is in it, so the heads are measured: once as they are drawn, and
// again when one of them or the room the table scrolls in changes size,
// which the browser says. And only as many stay as leave room for the
// rest to be scrolled in (`staying` in `columns.ts`): at a phone's width
// that is none.

import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { lefts, staying } from '../columns.ts';

/** The first `count` columns of the table `table` is put on: how many of them stay, and where each begins. `drawn` says which columns there are, so that others are measured anew. */
export function useStaying(count: number, drawn: string): { table: { current: HTMLTableElement | null }; stay: number; left: number[] } {
  const table = useRef<HTMLTableElement>(null);
  const [at, setAt] = useState<{ stay: number; left: number[] }>({ stay: 0, left: [] });
  useLayoutEffect(() => {
    const grid = table.current;
    const room = grid?.parentElement;
    const put = (stay: number, left: number[]) => setAt((was) => (was.stay === stay && was.left.join() === left.join() ? was : { stay, left }));
    if (!grid || !room || count === 0 || typeof ResizeObserver === 'undefined') return put(0, []);
    const heads = [...grid.querySelectorAll<HTMLElement>('thead tr:last-child th')].slice(0, count);
    const measure = () => {
      const widths = heads.map((head) => head.getBoundingClientRect().width);
      const stay = staying(widths, count, room.clientWidth);
      put(stay, lefts(widths, stay));
    };
    measure();
    const seen = new ResizeObserver(measure);
    seen.observe(room);
    for (const head of heads) seen.observe(head);
    return () => seen.disconnect();
  }, [count, drawn]);
  return { table, ...at };
}
