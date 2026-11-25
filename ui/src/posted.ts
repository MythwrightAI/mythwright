// What is attached to a message that is a file of a project, posted as
// that file and not as a copy (`api::posted`), and what is offered of a
// copy: to be added to a project, after which the message names that file.
//
// The daemon says which an attachment is (`file`, by its project and its
// path) and what that file is now as the reader may read it (`state`,
// read when the message is read, or for one that came by what was said
// of it when its chip is first drawn). What is worked out here is what a
// chip says and does, who is offered adding a copy to a project, where it
// may go, and what a message comes to once one was added.

import { call } from './api.ts';
import type { Attachment, Entry, Message } from './api.ts';
import { mayChange } from './channeldocs.ts';
import type { Writer } from './channeldocs.ts';
import { covers } from './holds.ts';
import { fileSize } from './util.ts';

/** What a file of a project is now, as the reader may read it. */
export type Standing = 'there' | 'gone' | 'kept';

/** Something attached as the daemon hands it: where it is a file of a project, which, and what it is now. */
export type Posted = Attachment;

/** What a chip of a file of a project says and does. */
export interface Chip {
  /** What it is called. */
  said: string;
  /** Under it, fainter: where it is and how big, or what has become of it. */
  note: string;
  /** Whether pressing it opens it. */
  opens: boolean;
}

/**
 * The chip of a file of a project: its name, and where it is with its
 * size while it is there; a file that went says so and opens nothing,
 * and one kept from the reader says that it names a file and nothing of it.
 */
export function chipOf(one: Posted): Chip | null {
  const file = one.file;
  if (!file) return null;
  const where = `${file.project}/${file.path}`;
  switch (file.state) {
    case 'gone':
      return { said: one.name, note: `${where} · no longer there`, opens: false };
    case 'kept':
      return { said: one.name, note: 'a file of a project that is not yours to read', opens: false };
    case 'there':
      return { said: one.name, note: `${where} · ${fileSize(one.size)}`, opens: true };
    default:
      // Not yet read for this reader: it is named, and opened once it is.
      return { said: one.name, note: where, opens: false };
  }
}

/** The channel and who reads it, as adding a copy to a project goes by. */
export interface Placing {
  channel: { project?: string | null; kept?: boolean; archived?: unknown; members: { actor: string }[] };
  by: Writer | null;
}

/** Whether a copy is offered to be added to a project: to whoever may write in the channel and write a project's files. */
export function mayPlace({ channel, by }: Placing): boolean {
  return !!by && mayChange(channel, by) && covers(by.scopes, 'projects:write');
}

/** A folder offered to put a copy in: where it is from the top of its project (`''` the top), and how it is said. */
export interface Folder {
  path: string;
  said: string;
  /** Where it is a library's folder: the library. */
  library?: string;
}

/**
 * The folders of a project a copy may be put in: its libraries' first,
 * each said as the library, then the top of the project and every folder
 * of it, by where they are. A library's folder is not offered twice.
 */
export function foldersOf(tree: Entry[], libraries: { name: string; at?: string | null }[]): Folder[] {
  const kept = libraries.flatMap((one) => (one.at ? [{ path: one.at.replace(/\/+$/, ''), said: `${one.name} (library)`, library: one.name }] : []));
  const theirs = new Set(kept.map((one) => one.path));
  const folders = tree.filter((entry) => entry.dir && !theirs.has(entry.path)).map((entry) => ({ path: entry.path, said: `${entry.path}/` }));
  folders.sort((a, b) => a.path.localeCompare(b.path));
  return [...kept, ...(theirs.has('') ? [] : [{ path: '', said: 'The top of the project' }]), ...folders];
}

/** Where a copy called `name` is written in `folder`: refused, as `null`, where the name is no file's name. */
export function pathFor(folder: string, name: string): string | null {
  const plain = name.trim();
  if (!plain || plain.includes('/') || plain.includes('\\') || plain === '.' || plain === '..') return null;
  return folder ? `${folder}/${plain}` : plain;
}

/** What `message` comes to once its attachment `attachment` names the file of `project` at `path`. */
export function placedIn(message: Message, attachment: string, project: string, path: string): Message {
  const attachments = message.attachments ?? [];
  if (!attachments.some((one) => one.id === attachment)) return message;
  const name = path.split('/').pop() ?? path;
  const changed = attachments.map((one) => (one.id === attachment ? { ...one, name, size: 0, file: { project, path } } : one));
  return { ...message, attachments: changed };
}

/** Something attached as whoever asks may see it: for one that came by what was said of it. */
export const seenOf = (id: string) => call<Posted>('GET', `/attachments/${encodeURIComponent(id)}/seen`);
/** Writes a copy among the files of `project` at `path`, and has its message name that file: what it is now. */
export const placeAttachment = (id: string, project: string, path: string) => call<Posted>('POST', `/attachments/${encodeURIComponent(id)}/project`, { project, path });
