// The documents a channel keeps to hand: the Docs
// tab of the Tools panel beside a channel.
//
// The list is the channel's and the daemon's (`talk::docs`), the same for
// everybody in it, and is handed to each reader as they may read what an
// entry names (`agents::docs`): there, gone, or kept from them. Nothing of
// it is kept in the browser. What is worked out here is what a line says,
// who is offered changing it, and what may be added: the libraries'
// documents and the files of the projects of the channel's workspace, as
// the reader was handed them, less what the channel keeps already.

import { call } from './api.ts';
import type { Entry } from './api.ts';
import { covers } from './holds.ts';
import { documentAt } from './library.ts';

/** An entry of a channel's documents, as the daemon hands it to whoever asks. */
export interface ChannelDoc {
  /** What it is said as: a library's record by its id, or a project's file as `<project>/<path>`. */
  doc: string;
  kind: 'record' | 'file';
  project?: string;
  /** Its file from the top of its project, where it may be read; of a file, always. */
  path?: string;
  /** What it is called, where it may be read. */
  title?: string;
  state: 'there' | 'gone' | 'kept';
  by: string;
  added: number;
}

const of = (channel: string) => `/channels/${encodeURIComponent(channel)}/docs`;

/** The channel's documents, as whoever asks may see them. */
export const readDocs = (channel: string) => call<{ docs: ChannelDoc[] }>('GET', of(channel)).then((answer) => answer.docs);
/** Adds one, as it is said; answered with the list as it is then. */
export const addDoc = (channel: string, doc: string) => call<{ docs: ChannelDoc[] }>('POST', of(channel), { doc }).then((answer) => answer.docs);
/** Takes one away, as it is said; answered with the list as it is then. */
export const removeDoc = (channel: string, doc: string) =>
  call<{ docs: ChannelDoc[] }>('DELETE', `${of(channel)}?doc=${encodeURIComponent(doc)}`).then((answer) => answer.docs);

/** What a line says of an entry: what it is called, or what has become of it, and under that where it is. */
export function lineOf(one: ChannelDoc): { said: string; where: string } {
  if (one.state === 'there') {
    const where = one.kind === 'record' ? `${one.doc}` : `${one.project}/${one.path}`;
    return { said: one.title ?? one.doc, where };
  }
  if (one.state === 'kept') return { said: 'Not yours to read', where: one.doc };
  return { said: 'Gone', where: one.doc };
}

/** Whoever is asking, as far as changing the list goes. */
export interface Writer {
  actor: string;
  scopes: string[];
  /** Whether theirs is the account that owns the factory. */
  owner: boolean;
}

/**
 * Whether `by` is offered adding to the list and taking from it: whoever
 * may write in the channel, which of a private one is somebody in it or
 * the owner, and of an archived one nobody, as the daemon holds it.
 */
export function mayChange(channel: { kept?: boolean; archived?: unknown; members: { actor: string }[] }, by: Writer): boolean {
  if (channel.archived) return false;
  if (!covers(by.scopes, 'channels:write')) return false;
  return !channel.kept || channel.members.some((member) => member.actor === by.actor) || (by.owner && covers(by.scopes, 'admin'));
}

/** Something that may be added: what it is said as, what it is called and where it is. */
export interface Offer {
  doc: string;
  title: string;
  where: string;
}

/** What may be added from one project: its libraries' documents, and the rest of its files. */
export interface Offered {
  project: string;
  name: string;
  documents: Offer[];
  files: Offer[];
}

/** A library, as the page has it of a project's tables. */
interface Shelved {
  name: string;
  store: string;
  at?: string | null;
}

/**
 * What may be added to a channel's documents: of each of `projects` (the
 * channel's workspace's, in their order), the files the reader was handed
 * of it (which leave out what is kept from them), each a document of a
 * library where it is one (by its record) and a file of the project where
 * it is not, less what the channel keeps already, held to every word of
 * `typed`, and a project with nothing left out.
 */
export function offered(
  projects: { id: string; name: string }[],
  trees: Record<string, Entry[] | undefined>,
  libraries: Record<string, Shelved[] | undefined>,
  kept: string[],
  typed = '',
): Offered[] {
  const has = new Set(kept);
  const words = typed.toLowerCase().split(/\s+/).filter(Boolean);
  const says = (offer: Offer, project: string) => words.every((word) => `${offer.title} ${offer.where} ${project}`.toLowerCase().includes(word));
  const shown: Offered[] = [];
  for (const project of projects) {
    const documents: Offer[] = [];
    const files: Offer[] = [];
    for (const entry of trees[project.id] ?? []) {
      if (entry.dir) continue;
      const document = documentAt(libraries[project.id] ?? [], project.id, entry.path);
      const offer = document
        ? { doc: document.record, title: entry.name.replace(/\.md$/, ''), where: `${project.id}/${entry.path}` }
        : { doc: `${project.id}/${entry.path}`, title: entry.name, where: `${project.id}/${entry.path}` };
      if (has.has(offer.doc) || !says(offer, project.name)) continue;
      (document ? documents : files).push(offer);
    }
    const order = (a: Offer, b: Offer) => a.where.localeCompare(b.where);
    if (documents.length + files.length > 0) shown.push({ project: project.id, name: project.name, documents: documents.sort(order), files: files.sort(order) });
  }
  return shown;
}
