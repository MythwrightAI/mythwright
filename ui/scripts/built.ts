// What a check's daemon serves is the client built from the files beside
// it, or the check says so before it reads a page of it.
//
// A program run from where cargo built it serves this checkout's client as
// it was last built, `dist/`, and nothing builds that for a check. Run
// after the client's files were changed and before they were built again,
// a check is handed the client from before the change and fails at the
// first thing the change drew: the Reference's check waited thirty seconds
// for an example the client it was served never drew, twice, on a page of
// 4,033 elements, which is what the client from before examples draws
// (4,180 with them), and passed once the client was built. Nothing of it
// was the page's or the check's order; it read as a check that fails now
// and then.
//
// So the build writes down what it was built from (`dist/built-from.json`:
// each file of this folder it read, and what that held, by a hash), after
// the page that loads it, so that a record that holds says the client
// served is that one; and where a check's daemon is one that serves this
// checkout's client, the files are read again and held to it
// (`builtFromHere`, which `saysWhere` asks). A client built here before
// this was written has no record, and is said to need building again.
//
// And a client may be built again while a check runs, by whoever else
// works in this checkout (a gate's `npm run check`, somebody's `npm run
// build`). A page fetches some parts of the client when they are first
// used (the code editor, the Reference), and the build keeps the
// files of one build before it (`generations.ts`): a page two builds
// behind asks for a part that is no longer there, and says that a part
// of the client could not be loaded. `within.mjs` waited thirty seconds
// for the editor of `std/board` under that notice, in a checkout whose
// client another hand built twice in that while, and was taken for a
// check that fails under load: crowded, on one processor and on four, it
// never did. So the build a check began on is noted (`heldTo`), and a
// check that fails says, as it exits, where the client was built again
// since (`rebuiltSaid`).

import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Where the record is kept, in what is built. */
export const RECORD = 'dist/built-from.json';

/** Besides what the bundler read, what makes the client what it is: the build, its page, and the packages as they are locked. */
export const BESIDES = ['build.mjs', 'index.html', 'package-lock.json'];

/** What a file held, short and enough to tell two apart. */
export const hashOf = (bytes: Uint8Array | string): string => createHash('sha256').update(bytes).digest('hex').slice(0, 16);

/**
 * The files of this folder a build was made from, of what the bundler
 * says it read (`metafile.inputs`, by paths from the folder): those that
 * are files here, which a package's are not (they are the lock's), nor
 * what a plugin made (`icons:set`). Sorted, and each once.
 */
export function sourcesOf(inputs: readonly string[]): string[] {
  const here = inputs.filter((path) => !path.includes(':') && !path.startsWith('node_modules/') && !path.startsWith('../'));
  return [...new Set([...here, ...BESIDES])].sort();
}

/** The files whose contents are not what the build read: changed, or gone (`null`). */
export function unlike(recorded: Readonly<Record<string, string>>, now: Readonly<Record<string, string | null>>): string[] {
  return Object.keys(recorded)
    .filter((path) => now[path] !== recorded[path])
    .sort();
}

/** What is said of a client that was not built from the files here, with what to do. */
export function staleSaid(changed: readonly string[]): string {
  const named = changed.length <= 3 ? changed.join(', ') : `${changed.slice(0, 3).join(', ')} and ${changed.length - 3} more`;
  return `the client the daemon serves (ui/dist) was not built from the files here: ${named} changed since. Run \`npm run build\` in ui/ and the check again`;
}

/** This folder, the client's. */
const ui = resolve(dirname(fileURLToPath(import.meta.url)), '..');

/**
 * Whether `program` serves this checkout's client: it is in a folder cargo
 * builds into (which holds `.fingerprint`, as `crates/folders/src/place.rs`
 * goes by) inside this repository. An installed copy serves the client
 * built into it, and nothing here is asked of that.
 */
export function servesThisClient(program: string): boolean {
  const folder = dirname(resolve(program));
  const within = relative(resolve(ui, '..'), folder);
  return within !== '' && !within.startsWith('..') && within.split(sep)[0] === 'target' && existsSync(join(folder, '.fingerprint'));
}

/** Refuses, in a sentence, a client in `dist/` that was not built from the files here; else hands back which build it is. */
export async function builtFromHere(): Promise<string> {
  const record = await readFile(join(ui, RECORD), 'utf8').then(
    (text) => JSON.parse(text) as { id?: string; files?: Record<string, string> },
    () => null,
  );
  if (!record?.files) throw new Error('the client the daemon serves (ui/dist) does not say what it was built from. Run `npm run build` in ui/ and the check again');
  const paths = Object.keys(record.files);
  const read = await Promise.all(paths.map((path) => readFile(join(ui, path)).then(hashOf, () => null)));
  const changed = unlike(record.files, Object.fromEntries(paths.map((path, at) => [path, read[at] ?? null])));
  if (changed.length > 0) throw new Error(staleSaid(changed));
  return record.id ?? '';
}

/** Which build `dist/` holds now, as its record says: `null` where it says none. */
function builtNow(): string | null {
  try {
    return (JSON.parse(readFileSync(join(ui, RECORD), 'utf8')) as { id?: string }).id ?? null;
  } catch {
    return null;
  }
}

/** What is said of a check that failed where the client was built again under it: `null` where it was not. */
export function rebuiltSaid(began: string, now: string | null): string | null {
  if (now === began) return null;
  return `The client the daemon serves (ui/dist) was built again while this check ran: it began on build ${began} and ui/dist holds ${now ?? 'no build it can name'} now. A page two builds behind is told that a part of the client could not be loaded (the code editor, the Reference), so what failed may be that and not the check or the page. Run the check again where nothing builds the client meanwhile.`;
}

let begun: string | null = null;

/** Notes the build a check began on, once: a check that fails says, as it exits, where `dist/` was built again since. */
export function heldTo(build: string): void {
  if (begun !== null) return;
  begun = build;
  process.on('exit', (code) => {
    if (code === 0 || begun === null) return;
    const said = rebuiltSaid(begun, builtNow());
    if (said) console.error(`\n${said}`);
  });
}
