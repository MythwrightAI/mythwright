import { useState } from 'preact/hooks';
import { NAMES, beginsAt } from '../turning.ts';

/** Seconds a name is shown before the next comes up under it. */
const EACH = 2.8;

/**
 * Names turning over where a field holds nothing: one at a time, each coming up from below and going out above.
 * It is drawn over the field and is no part of it: nothing here is the field's value, nothing takes the pointer,
 * and what reads the page aloud is told nothing of it. The turning is the style sheet's alone, so nothing here
 * keeps time.
 */
export function Turning() {
  const [from] = useState(() => beginsAt(Math.random()));
  return (
    <span class="turning" aria-hidden="true" style={`--names:${NAMES.length};--each:${EACH}s;--from:${from}`}>
      <span class="turning-name">
        <span class="turning-reel">
          {NAMES.map((name) => (
            <span key={name}>{name}</span>
          ))}
        </span>
      </span>
    </span>
  );
}
