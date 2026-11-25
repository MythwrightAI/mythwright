// The files of a project named in what somebody wrote, as links: a function of the words and of what the page knows of
// the files, with nothing of the page in it, so that its tests can read
// it.
//
// A path is a word written as one (`docs/plans/x.md`, `README.md`,
// `@blueprints/fleet.bp`), in a project: the one the words are of (a
// channel's, a record's), or the one named before it with a colon
// (`shop:docs/plans/x.md`). It is a link only where the page knows it is
// a file there that the reader may read: a path that names no file the
// page knows is words, never a guess. What the page does not know yet is
// asked of the daemon, a few at a time (`asked`), and what it then knows
// is handed back in (`Known`).

/** A file of a project. */
export interface File {
  project: string;
  path: string;
}

/** A piece of the words: what it says, and the file it names where it names one the page knows. */
export interface Piece {
  text: string;
  file: File | null;
}

/** What the page knows of a path of a project: a file there, no such file (or none the reader may read), or nothing yet. */
export type Knowing = 'file' | 'not' | 'unknown';

/** What a page hands in: the project the words are of, where they are of one; the projects there are; and what it knows of a path. */
export interface Known {
  own: string | null;
  projects: readonly string[];
  knows: (file: File) => Knowing;
}

// A word as a path is written: letters, digits and `_ . - + @ /`, with
// what a project is called and a colon before it where it says one. Not
// inside a word, an address (`https://…`) or an email.
const WORD = /(?<![\p{L}\p{N}_.@/:+-])(?:([A-Za-z0-9][\w.-]*):)?((?:\.\/)?[\p{L}\p{N}_.@+-][\p{L}\p{N}_.@/+-]*)/gu;
// What a sentence puts after a word and is not part of it.
const AFTER = /[.,;:!?…)\]}'"]+$/u;
// What a file is called after the last dot of its name: two letters or more, the first a letter.
const ENDING = /\.[A-Za-z][A-Za-z0-9]{1,9}$/;
// A line written after a path (`main.rs:12`) is the line's, and the path stands without it.
const LINE = /:\d+(?::\d+)?$/;

/** What a word comes to as a path: the file it would name, and how much of the word that is; nothing for a word that is no path. */
function pathOf(prefix: string | undefined, word: string, known: Known): { file: File; length: number } | null {
  let written = word.replace(AFTER, '');
  const line = LINE.exec(written);
  if (line) written = written.slice(0, line.index);
  const path = written.replace(/^\.\//, '');
  if (path === '' || path.endsWith('/') || path.includes('//') || path.split('/').some((part) => part === '..' || part === '')) return null;
  // An email, or `@` anywhere but at its head (`@blueprints/…`), is not a path.
  if (path.indexOf('@') > 0) return null;
  const last = path.split('/').at(-1)!;
  // A folder alone is no file; a word with no folder is one only where it is called as a file is (`README.md`, not `e.g`).
  if (!ENDING.test(last) && !path.includes('/')) return null;
  if (!/\p{L}/u.test(path)) return null;
  const project = prefix !== undefined ? (known.projects.includes(prefix) ? prefix : null) : known.own;
  if (project === null) return null;
  const length = (prefix !== undefined ? prefix.length + 1 : 0) + written.length;
  return { file: { project, path }, length };
}

/** Each path written in the words, with where it begins and how long it is as written. */
function found(words: string, known: Known): { at: number; length: number; file: File }[] {
  const all: { at: number; length: number; file: File }[] = [];
  for (const match of words.matchAll(WORD)) {
    const at = match.index ?? 0;
    // A word that goes on into an address (`https://…`) is the address's.
    if (words.startsWith('://', at + match[0].length)) continue;
    const said = pathOf(match[1], match[2]!, known);
    if (said) all.push({ at, ...said });
  }
  return all;
}

/**
 * The words in pieces, in order: each path of a file the page knows a
 * piece that names it, and the words between pieces that name nothing.
 * Put together the pieces are the words, letter for letter.
 */
export function pathPieces(words: string, known: Known): Piece[] {
  const pieces: Piece[] = [];
  let from = 0;
  for (const one of found(words, known)) {
    if (known.knows(one.file) !== 'file') continue;
    if (one.at > from) pieces.push({ text: words.slice(from, one.at), file: null });
    pieces.push({ text: words.slice(one.at, one.at + one.length), file: one.file });
    from = one.at + one.length;
  }
  if (from < words.length) pieces.push({ text: words.slice(from), file: null });
  return pieces;
}

/** How many paths the page asks the daemon about for one thing somebody wrote. */
export const MOST_ASKED = 3;

/** The paths of the words the page knows nothing of yet, each once, the first `MOST_ASKED` of them: what it asks the daemon. */
export function asked(words: string, known: Known): File[] {
  const unknown: File[] = [];
  for (const one of found(words, known)) {
    if (known.knows(one.file) !== 'unknown') continue;
    if (unknown.some((had) => had.project === one.file.project && had.path === one.file.path)) continue;
    unknown.push(one.file);
    if (unknown.length === MOST_ASKED) break;
  }
  return unknown;
}

/**
 * What the files a project's page has read say of a path: a file there,
 * none, or nothing where they were not read. What the factory keeps of
 * a project's own blueprints (`@blueprints/…`) is in no list of its
 * folder, and is asked.
 */
export function inTree(entries: readonly { path: string; dir: boolean }[] | undefined, path: string): Knowing {
  if (!entries || path.startsWith('@')) return 'unknown';
  return entries.some((entry) => entry.path === path && !entry.dir) ? 'file' : 'not';
}

/**
 * The project something is of by its id, where the id says it: a
 * record's id begins with its project's and a dash or a colon
 * (`shop-12`, `shop-docs:guides:install`); the longest project that
 * fits, so that `shop-front-3` is of `shop-front` where there is one.
 */
export function projectOfId(id: string, projects: readonly string[]): string | null {
  const fits = projects.filter((project) => id.startsWith(`${project}-`) || id.startsWith(`${project}:`));
  return fits.sort((a, b) => b.length - a.length)[0] ?? null;
}
