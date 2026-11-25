// The id of something running, wherever one is written.
//
// An instance is known to the factory by an id (`in_…`), and a step that
// starts one gives the id out: `instance: in_1m7qfzbgtp98vz92`. Written
// bare it says nothing: not what runs, and not that it can be opened. So
// wherever the client writes one it says beside it what it is (the
// blueprint it runs and its key, as the list of what is running does)
// and opens it on a press; and one that names nothing any more is said
// to be gone, and is not a way to nowhere.
//
// This is what of that has no page in it: what is known of each id, and
// where the ids are in a text. What a blueprint's name and key are never
// changes for an instance, so each is read once (`views/running.tsx`) and
// kept here.

import { signal } from '@preact/signals';

/** What an id is of. */
export interface Known {
  blueprint: string;
  key: string;
  /** The project it was applied for, or none; not said by what told of it before that was asked. */
  project?: string | null;
}

/** What has been read of each id: what it is, or that it names nothing any more. One not read yet is not here. */
export const known = signal<Record<string, Known | 'gone'>>({});

/** Something read says what an id is. */
export function tell(id: string, what: Known | 'gone'): void {
  const now = known.value[id];
  if (now === what || (now && now !== 'gone' && what !== 'gone' && now.blueprint === what.blueprint && now.key === what.key && now.project === what.project)) return;
  known.value = { ...known.value, [id]: what };
}

/** The ids that have been asked about, so that each is asked about once however often it is drawn. */
export const asked = new Set<string>();

/** Lets go of everything read: whoever was signed in is no longer. */
export function forgetRunning(): void {
  known.value = {};
  asked.clear();
}

/** What an id is of, in words: the blueprint and the key, as the list of what is running says them. */
export function named(what: Known): string {
  return `${what.blueprint} · ${what.key}`;
}

/** What stands for an id where there is room for one line of words and nothing to press: a tab's name. */
export function titled(id: string, what: Known | 'gone' | undefined): string {
  return what === undefined ? id : what === 'gone' ? `${id} (gone)` : named(what);
}

// An id is `in_` and then letters and figures the journal gave it. A word
// such as `in_place` is not one: an id is long, and has a figure in it.
const ID = /(?<![A-Za-z0-9_])in_(?=[a-z]*\d)[0-9a-z]{10,}(?![A-Za-z0-9_])/g;

/** A text as what is around the ids in it and the ids, in order. A text with none is itself. */
export function parts(text: string): (string | { id: string })[] {
  const out: (string | { id: string })[] = [];
  let from = 0;
  for (const found of text.matchAll(ID)) {
    if (found.index > from) out.push(text.slice(from, found.index));
    out.push({ id: found[0] });
    from = found.index + found[0].length;
  }
  if (from < text.length) out.push(text.slice(from));
  return out;
}

/** The ids written in a text, each once. */
export function idsIn(text: string): string[] {
  return [...new Set(parts(text).flatMap((part) => (typeof part === 'string' ? [] : [part.id])))];
}
