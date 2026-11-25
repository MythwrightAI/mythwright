// A library: a collection whose records are the Markdown files of a
// folder of its project. What its page needs worked out, with nothing of
// the page in it: which collections are libraries, where a document's file
// is, how the documents are set out by folder, and which events may have
// changed them.

import type { Row } from './table.ts';

/** What of a collection says it is a library and where its documents are. */
export interface Kept {
  name: string;
  store: string;
  at?: string | null;
}

/** Whether a collection's records are documents. */
export function isLibrary(collection: Kept): boolean {
  return collection.store === 'documents';
}

/** The folder of the project a library's documents are in, from the top of the project; nothing for the project itself. */
export function folderOf(library: Kept): string {
  if (library.at === '.') return '';
  return library.at ?? library.name;
}

/** Where a document is in its library: `guides/install.md`. Its record says; failing that, what it is known by does. */
export function pathOf(row: Row): string {
  const said = row.data.path;
  if (typeof said === 'string' && said) return said;
  const named = row.id.slice(row.id.indexOf(':') + 1);
  return `${named.split(':').join('/')}.md`;
}

/** A document's file, from the top of its project: what is opened to write in it. */
export function fileOf(library: Kept, row: Row): string {
  const folder = folderOf(library);
  return folder ? `${folder}/${pathOf(row)}` : pathOf(row);
}

/**
 * Which document a file of a project is: the library it is of, and what
 * the document is known by. Nothing for a file that is no library's
 * document: one outside every library's folder, one that is not Markdown,
 * or one whose name could not be written in an id. `libraries` are the
 * project's own, in the order they are listed, and a file that two of
 * them hold (a library of a folder inside another's) is the first one's,
 * which is how the daemon says whose a file is when it is written.
 */
export function documentAt<L extends Kept>(libraries: L[], project: string, file: string): { library: L; record: string } | null {
  for (const library of libraries.filter(isLibrary)) {
    const folder = folderOf(library);
    if (folder && !file.startsWith(`${folder}/`)) continue;
    const path = folder ? file.slice(folder.length + 1) : file;
    const parts = path.slice(0, -'.md'.length).split('/');
    const sound = path.endsWith('.md') && !path.includes(':') && parts.every((part) => part !== '' && part !== '.' && part !== '..');
    if (sound) return { library, record: `${project}-${library.name}:${parts.join(':')}` };
  }
  return null;
}

/** What a document is called: its title, or its file's name where it has none. */
export function titleOf(row: Row): string {
  const title = row.data.title;
  if (typeof title === 'string' && title.trim()) return title;
  const name = pathOf(row).split('/').pop() ?? row.id;
  return name.replace(/\.md$/, '');
}

export interface Shelf {
  /** The folder within the library, with `/` between its parts; nothing for the library's own. */
  folder: string;
  rows: Row[];
}

/**
 * The documents by the folder each is in: the library's own first, then
 * each folder under it by name, and within a folder by what they are
 * called.
 */
export function shelves(rows: Row[]): Shelf[] {
  const by = new Map<string, Row[]>();
  for (const row of rows) {
    const path = pathOf(row);
    const folder = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
    by.set(folder, [...(by.get(folder) ?? []), row]);
  }
  return [...by.entries()]
    .sort(([a], [b]) => (a === '' ? -1 : b === '' ? 1 : a.localeCompare(b)))
    .map(([folder, found]) => ({ folder, rows: [...found].sort((a, b) => titleOf(a).localeCompare(titleOf(b))) }));
}

/** Where a library is read as pages. */
export function pagesOf(project: string, name: string): string {
  return `/library/${encodeURIComponent(project)}/${encodeURIComponent(name)}/`;
}

/** The events after which a library may hold other documents than it did. */
export const CHANGES = ['record.created', 'record.updated', 'record.removed', 'project.file.changed', 'project.rescanned', 'collection.ensured'];

/**
 * Whether an event may have changed what is in a library: something done
 * to one of its documents, a file under its folder written by anything,
 * the project's files looked over, or the library declared again.
 */
export function changes(event: { topic: string; data?: Record<string, unknown> }, project: string, library: Kept): boolean {
  const data = event.data ?? {};
  if (data.project !== project) return false;
  if (event.topic.startsWith('record.')) return data.collection === library.name;
  if (event.topic === 'collection.ensured') return data.name === library.name;
  if (event.topic === 'project.rescanned') return true;
  const path = typeof data.path === 'string' ? data.path : '';
  const folder = folderOf(library);
  return path.endsWith('.md') && (folder === '' || path.startsWith(`${folder}/`));
}

/**
 * Where a new document goes, from what was typed: its folder within the
 * library, if one was said, and its title as a file's name. Nothing where
 * there is no title to name it by.
 */
export function placeFor(title: string, folder: string): string | null {
  const word = (text: string) =>
    text
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  const name = word(title);
  if (!name) return null;
  const parts = folder.split('/').map(word).filter(Boolean);
  return [...parts, name].join('/');
}
