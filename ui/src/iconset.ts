// The set of icons, read when it is first wanted, and the asking of the
// daemon to keep one for a column.
//
// The drawings are a part of the client that is loaded by itself: a page
// that draws no icon and opens no picker never asks for it. It is asked
// for once and kept; a table that names an icon draws the mark of each
// field's kind until it has come.

import { signal } from '@preact/signals';
import { call } from './api.ts';
import type { Collection } from './api.ts';
import type { IconSet } from './icons.ts';

/** The drawings, once they have been read. */
export const iconSet = signal<IconSet | null>(null);

let asked: Promise<void> | null = null;

/** Reads the set unless it is read or being read. One that could not be read is asked for again by whoever next wants it. */
export function wantIcons(): void {
  asked ??= import('icons:set').then(
    (set) => {
      iconSet.value = set.icons;
    },
    () => {
      asked = null;
    },
  );
}

const part = encodeURIComponent;

/** Chooses the icon a field's column is drawn with, for everybody; with none, takes the choice away. The table comes back as it then is. */
export function chooseIcon(project: string, table: string, field: string, icon: string | null): Promise<Collection> {
  const address = `/collections/${part(project)}/${part(table)}/fields/${part(field)}/icon`;
  return icon === null ? call<Collection>('DELETE', address) : call<Collection>('PUT', address, { icon });
}
