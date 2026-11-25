// The mark before a column's name: the icon its field says, declared or
// chosen, or else a mark for the kind of thing the field holds.
//
// Wherever a column's mark is drawn it is drawn by `FieldMark`, so an
// icon somebody chose is the same in every place. An icon is drawn in the
// stroke and at the size the marks of the kinds are, in the colour of
// what it is drawn in.

import { h } from 'preact';
import type { ComponentChildren } from 'preact';
import { useEffect } from 'preact/hooks';
import { drawingOf, iconOf } from '../icons.ts';
import type { IconNode } from '../icons.ts';
import { iconSet, wantIcons } from '../iconset.ts';
import type { Field } from '../table.ts';

export const stroke = { fill: 'none', stroke: 'currentColor', 'stroke-width': 1.3, 'stroke-linecap': 'round', 'stroke-linejoin': 'round' } as const;
const Mark = ({ children }: { children: ComponentChildren }) => (
  <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true" {...stroke}>
    {children}
  </svg>
);

/** What kind of thing a column holds, as a mark before its name. */
export function KindMark({ kind }: { kind: string }) {
  if (kind === 'choice') {
    return (
      <Mark>
        <circle cx="8" cy="8" r="5.5" />
        <path d="m5.8 7.2 2.2 2.2 2.2-2.2" />
      </Mark>
    );
  }
  if (kind === 'actor') {
    return (
      <Mark>
        <circle cx="8" cy="5.6" r="2.6" />
        <path d="M3 13.5c.6-2.6 2.5-3.8 5-3.8s4.4 1.2 5 3.8" />
      </Mark>
    );
  }
  if (kind === 'list') {
    return (
      <Mark>
        <path d="M6 4h7.5M6 8h7.5M6 12h7.5M2.8 4h.01M2.8 8h.01M2.8 12h.01" />
      </Mark>
    );
  }
  if (kind === 'number') {
    return (
      <Mark>
        <path d="M6.2 2.5 5 13.5M11 2.5 9.8 13.5M3 5.8h10.5M2.5 10.2H13" />
      </Mark>
    );
  }
  if (kind === 'date') {
    return (
      <Mark>
        <rect x="2.5" y="3.5" width="11" height="10" rx="2" />
        <path d="M2.5 6.8h11M5.5 2v3M10.5 2v3" />
      </Mark>
    );
  }
  if (kind === 'flag') {
    return (
      <Mark>
        <rect x="2.5" y="2.5" width="11" height="11" rx="2.5" />
        <path d="m5.5 8.2 1.8 1.8 3.4-3.8" />
      </Mark>
    );
  }
  if (kind === 'record' || kind === 'records') {
    return (
      <Mark>
        <path d="M6.5 3.5h-3v9h9v-3M9 3.5h3.5V7M12.5 3.5 7.5 8.5" />
      </Mark>
    );
  }
  return (
    <Mark>
      <path d="M3 12.5 6.3 3.5h.4L10 12.5M4.2 9.5h4.6M11.5 12.5v-4c0-2.6 3-2.6 3 0v4M11.5 10.7c0-1.6 3-1.6 3 0s-3 2-3 0" />
    </Mark>
  );
}

/**
 * One of the set's icons. They are drawn on a square of 24 with a stroke
 * of 2, which at the size of a mark is the weight the marks have.
 */
export function IconDrawing({ name, nodes, size = 14 }: { name: string; nodes: IconNode[]; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" data-icon={name} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      {nodes.map(([tag, drawn], at) => h(tag, { key: at, ...drawn }))}
    </svg>
  );
}

/**
 * A field's mark: its icon, where it names one the set has a drawing of,
 * and the mark of its kind otherwise. What a row is known by is marked as
 * text, whatever kind it is. The set is asked for only by a field that
 * names an icon: a table with none loads nothing for this.
 */
export function FieldMark({ field, title }: { field: Field; title?: boolean }) {
  const name = iconOf(field);
  useEffect(() => {
    if (name) wantIcons();
  }, [name]);
  const drawing = name ? drawingOf(field, iconSet.value) : null;
  return name && drawing ? <IconDrawing name={name} nodes={drawing} /> : <KindMark kind={title ? 'text' : field.kind} />;
}
