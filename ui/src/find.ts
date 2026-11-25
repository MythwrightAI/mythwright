// Finding a file by its name: what is typed is looked for in the names and
// paths of a project's files, which the client already has.
//
// This is beside the daemon's search, not in place of it. That one reads
// what is written inside things and matches whole words; this one finds
// `vocab` in `vocabulary.rs` while it is still being typed.

import type { Entry } from './api.ts';

/** How well a name matches what was typed; smaller is better, and `null` is not at all. */
function rank(entry: Entry, typed: string): number | null {
  const name = entry.name.toLowerCase();
  const path = entry.path.toLowerCase();
  if (name === typed) return 0;
  if (name.startsWith(typed)) return 1;
  if (name.includes(typed)) return 2;
  // Typed with a slash, it is a path that is meant.
  if (path.includes(typed)) return 3;
  // Several words: each of them somewhere in the path, in any order.
  const words = typed.split(/\s+/).filter(Boolean);
  if (words.length > 1 && words.every((word) => path.includes(word))) return 4;
  return null;
}

/**
 * The files whose names what was typed is found in: the nearest matches
 * first, then the shortest paths, which are the ones nearest the top.
 */
export function byName(entries: Entry[], typed: string, limit = 8): Entry[] {
  const wanted = typed.trim().toLowerCase();
  if (wanted === '') return [];
  const found: { entry: Entry; rank: number }[] = [];
  for (const entry of entries) {
    if (entry.dir) continue;
    const how = rank(entry, wanted);
    if (how !== null) found.push({ entry, rank: how });
  }
  found.sort((a, b) => a.rank - b.rank || a.entry.path.length - b.entry.path.length || a.entry.path.localeCompare(b.entry.path));
  return found.slice(0, limit).map((one) => one.entry);
}
