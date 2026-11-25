// What the page knows of the paths named in what somebody wrote
// (`pathlinked.ts`): from the files a project's page has read, and where
// those were not read, from asking the daemon once for each path as the
// words are drawn (`asked`). What it was told is kept for the page's
// life; a file made or taken away since is followed for the projects
// whose files were read, and is otherwise seen when the page is read
// again.

import { signal } from '@preact/signals';
import { api, ApiError } from './api.ts';
import { inTree } from './pathlinked.ts';
import type { File, Known, Knowing } from './pathlinked.ts';
import { projects, trees } from './store.ts';

/** What the daemon said of each path asked, by project and path; a path being asked is `unknown` until it answers. */
const told = signal<ReadonlyMap<string, Knowing>>(new Map());
const keyOf = (file: File) => `${file.project}\n${file.path}`;

/** What the page knows of a path: the project's files where they were read, else what the daemon said of it. */
export function knowing(file: File): Knowing {
  const read = inTree(trees.value[file.project], file.path);
  return read !== 'unknown' ? read : (told.value.get(keyOf(file)) ?? 'unknown');
}

/** What is handed to `pathlinked.ts` for words of a project, or of none. */
export function knownFor(own: string | null): Known {
  return { own, projects: projects.value.map((project) => project.id), knows: knowing };
}

/** Paths asked and not yet answered, so that a path drawn twice meanwhile is asked once. */
const asking = new Set<string>();

/** Asks the daemon whether each is a file the reader may read, each once for the page's life. */
export function ask(files: File[]): void {
  for (const file of files) {
    const key = keyOf(file);
    if (asking.has(key) || told.value.has(key)) continue;
    asking.add(key);
    api.fileThere(file.project, file.path).then(
      (entry) => settle(key, entry.dir ? 'not' : 'file'),
      // Not there, not a file, or not the reader's to read: words, as a path that names nothing is. A daemon not answering said nothing of it.
      (err) => (err instanceof ApiError && err.status === 0 ? asking.delete(key) : settle(key, 'not')),
    );
  }
}

function settle(key: string, now: Knowing): void {
  asking.delete(key);
  told.value = new Map(told.value).set(key, now);
}
