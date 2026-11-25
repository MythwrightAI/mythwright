// What a view is held to, said where the view is headed: that not every
// record is shown, by what, how many are, and one press to show them all.

import { Fragment } from 'preact';

import { heldBy } from '../ordered.ts';
import type { Only } from '../ordered.ts';
import type { Field } from '../table.ts';
import { Tag, worded } from './tag.tsx';

interface HeldProps {
  only: Only | null | undefined;
  fields: Field[];
  /** What one record is called: `card`. */
  what: string;
  /** How many are shown, of how many there are, where the page has read them. */
  shown?: number;
  of?: number;
  /** Shows every record again. Not given where the view is not somebody's to change here. */
  onClear?: () => void;
}

/** A line in a view's head saying what it is held to; nothing, for a view that shows everything. */
export function Held({ only, fields, what, shown, of, onClear }: HeldProps) {
  const held = heldBy(only, fields);
  if (held.length === 0) return null;
  return (
    <div class="held" role="status" aria-label="What this view is held to">
      <span>
        Showing only {what}s with{' '}
        {held.map(({ field, words, any }, nth) => (
          <Fragment key={field.name}>
            {nth > 0 ? ', and ' : ''}
            {words}{' '}
            {any.map((thing, at) => (
              <Fragment key={thing}>
                {at > 0 ? ' or ' : ''}
                {worded(field) ? <Tag of={thing} small /> : thing}
              </Fragment>
            ))}
          </Fragment>
        ))}
        {shown !== undefined && of !== undefined ? `: ${shown} of ${of}` : ''}.
      </span>
      {onClear && (
        <button class="ghost held-clear" title="Show every one again; the view is saved so" onClick={onClear}>
          Show all
        </button>
      )}
    </div>
  );
}
