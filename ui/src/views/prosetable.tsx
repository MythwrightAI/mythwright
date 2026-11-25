// A table written in text (`tables.ts`, read by `prose.ts`), drawn as a
// paper draws one: a rule over it and under its headings, a thin rule
// between rows, no lines between columns, and a column the writer set at
// the right (`---:`, as numbers are) at the right with its figures lined up.
// Its cells are drawn as a line's words are, links and footnotes' markers
// with them. Wider than its column, it scrolls sideways in its own box,
// marked at the edge that has more (`wider.tsx`).

import type { ComponentChildren } from 'preact';
import { useRef } from 'preact/hooks';
import type { Block, Span } from '../prose.ts';
import type { Align } from '../tables.ts';
import { useMoreShown } from './wider.tsx';

/** The class a cell of a column is drawn with, by where its words stand. */
const alignClass = (align: Align | undefined): string | undefined => (align ? `align-${align}` : undefined);

/** A table, its cells drawn by `drawn`. */
export function ProseTable({ block, drawn }: { block: Extract<Block, { is: 'table' }>; drawn: (spans: Span[]) => ComponentChildren[] }) {
  const frame = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLDivElement>(null);
  useMoreShown(frame, box);
  return (
    <div class="prose-table-frame wider" ref={frame}>
      {/* Reached by the keyboard, so that one wider than its column can be scrolled with the arrows. */}
      <div class="prose-table-box" ref={box} tabIndex={0} role="group" aria-label="Table">
        <table class="prose-table">
          <thead>
            <tr>
              {block.head.map((cell, column) => (
                <th key={column} scope="col" class={alignClass(block.align[column])}>
                  {drawn(cell)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {block.rows.map((row, nth) => (
              <tr key={nth}>
                {row.map((cell, column) => (
                  <td key={column} class={alignClass(block.align[column])}>
                    {drawn(cell)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
