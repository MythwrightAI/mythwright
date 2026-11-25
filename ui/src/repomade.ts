// What is made anew in a project's repo from the client: a file, and a
// folder to keep files in. Functions of plain values, with their cases in
// `repomade.test.ts`; what draws and asks is `views/repomade.tsx`.
//
// What a name comes to is said here before anything is asked, so that a
// name that cannot be one is said under the field as it is typed; what is
// still refused (a name another program took meanwhile, a folder a table
// keeps private) is the daemon's to say, in its own sentence.

import { covers } from './holds.ts';
import type { Entry } from './api.ts';

/** What can be made: a file, or a folder. */
export type Made = 'file' | 'folder';

/** What is offered to be made, in the order it is offered: nothing for whoever may not write a project's files, as the daemon asks. */
export function offered(scopes: string[]): Made[] {
  return covers(scopes, 'projects:write') ? ['file', 'folder'] : [];
}

/** What a line that makes one is called, wherever it is offered. */
export const lineOf = (made: Made): string => (made === 'file' ? 'New file…' : 'New folder…');

/**
 * The folder a press means: the folder of the line pressed, or the one a
 * page is showing; the top of the repo where none is said, and where what
 * was said is not a folder of the project as it was read (taken away, or a
 * file), so that nothing is made where nobody is looking.
 */
export function folderMeant(said: string | null | undefined, entries: Entry[]): string {
  if (!said) return '';
  return entries.some((entry) => entry.dir && entry.path === said) ? said : '';
}

/** What a name comes to: where it is made, or why it cannot be. */
export type Named = { path: string } | { wrong: string };

/**
 * Where a new file or folder called `name` is made in `folder`, as the
 * project was read: one name and not a path, since the folder it goes in
 * is the one pressed; never one that is there already, file or folder,
 * however its case is written, since some systems do not tell the two
 * apart; and never a name a project's path may not have (the daemon says
 * the same of `..` and of a name that ends in a dot or a space).
 */
export function named(name: string, folder: string, entries: Entry[]): Named {
  const called = name.trim();
  if (called === '') return { wrong: 'Give it a name.' };
  if (/[\\/]/.test(called)) return { wrong: 'One name, without a slash: it is made in the folder it was asked for in.' };
  if (called === '.' || called === '..') return { wrong: `${called} is no name a file or a folder can have.` };
  if (called.endsWith('.')) return { wrong: 'A name may not end in a dot.' };
  if (called.toLowerCase() === '.git') return { wrong: "That is the repository's own folder." };
  const path = folder ? `${folder}/${called}` : called;
  const there = entries.find((entry) => entry.path.toLowerCase() === path.toLowerCase());
  if (there) return { wrong: `There is ${there.dir ? 'a folder' : 'a file'} called ${there.name} here already.` };
  return { path };
}

/** Whether a file is written as Markdown, by its name. */
export const isMarkdown = (path: string): boolean => /\.(md|markdown)$/i.test(path);

/** What a new file holds as it is made: a heading of its name for Markdown, and nothing for anything else. */
export function firstText(path: string): string {
  if (!isMarkdown(path)) return '';
  const name = (path.split('/').pop() ?? path).replace(/\.(md|markdown)$/i, '');
  return `# ${name}\n`;
}

/** Where something made goes, said in the dialog: the folder, or the top of the repo. */
export function saidWhere(folder: string): string {
  return folder ? `in ${folder}/` : 'at the top of the repo';
}
