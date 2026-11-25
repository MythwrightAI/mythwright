// The factory's topics as the page has them: read once the
// Topics panel is first shown (or something asks for one by its title),
// and followed. What a topic is, and what the panel works out from them, is
// `topicpage.ts`.
//
// What is read is every topic's line (its title, status, the first line of
// its summary, where it is and how many replies it has) and never a body:
// a topic's page reads its own whole. It is read again, once for several
// things said together, whenever the daemon says something of the topics'
// table at the factory's place (`record.*` of `@`) or of a topic
// (`topic.*`), and when the stream comes back; of two readings on their
// way, only the one asked for last is kept, so that a reading the daemon
// took before something was said is never kept over one it took after.

import { signal } from '@preact/signals';
import { call } from './api.ts';
import { coalesce, stream } from './events.ts';
import type { Reply } from './replies.ts';
import { lineOf, named, topicOfEvent } from './topicpage.ts';
import type { Handed, TopicLine } from './topicpage.ts';

const at = (id: string) => encodeURIComponent(id);

/** What the daemon does with topics, as whoever is signed in (`/api/topics`). */
export const topicsApi = {
  list: () => call<{ topics: Handed[] }>('GET', '/topics').then((read) => read.topics),
  /** A topic whole, by its id or its title, with its replies, oldest first. */
  read: (named: string) => call<{ topic: Handed; replies: Record<string, unknown>[]; path?: unknown[]; pages?: unknown[] }>('GET', `/topics/${at(named)}`),
  /** Each revision of what it says, oldest first: its title, post and summary whole each time, who and when. */
  revisions: (named: string) => call<{ revisions: Record<string, unknown>[] }>('GET', `/topics/${at(named)}/revisions`).then((read) => read.revisions),
  /** Makes one, where it is said to be or in the usual workspace; whoever manages the factory makes the first, which sets topics up. */
  make: (title: string, place: { workspace?: string; project?: string }, body = '') => call<Handed>('POST', '/topics', { title, body, ...place }),
  /** Makes a page under a page, of its wiki's place and after every page there is. */
  makeUnder: (title: string, under: string) => call<Handed>('POST', '/topics', { title, body: '', under }),
  /** Moves a page under another (or to the top, where it is a wiki of its own: `under` nothing), before a page beside it there or at the end. */
  move: (page: string, under: string | null, before: string | null) => call<Handed>('POST', `/topics/${at(page)}/move`, { under, before }),
  /** Puts a wiki, with every page under it, in a workspace. */
  place: (wiki: string, workspace: string) => call<Handed>('POST', `/topics/${at(wiki)}/place`, { workspace }),
  /** Says something in it, under the reply it answers (`parent`, the number that one was said at) or the topic itself. */
  reply: (topic: string, body: string, parent: number | null) => call<{ seq: number }>('POST', `/topics/${at(topic)}/replies`, { body, parent }),
  /** Marks the reply said at `reply` as what answered it, or, with nothing, opens it again. */
  answer: (topic: string, reply: number | null) => call<Handed>('POST', `/topics/${at(topic)}/answer`, { reply }),
  /** Whoever asks follows it, or follows it no longer. */
  follow: (topic: string, follows: boolean) => call<Handed>('POST', `/topics/${at(topic)}/follow`, { follows }),
};

/** Every topic whoever is signed in may read, as the panel lists them. Empty until they are read. */
export const topics = signal<TopicLine[]>([]);

/** Whether they were read: until they are, an empty list says nothing of what there is. */
export const topicsRead = signal(false);

let asked = 0;
let reading: Promise<void> | null = null;

/**
 * Reads them, keeping only the reading asked for last; done once that one is
 * kept, so that whoever waits for a reading (a page moved, then its line
 * given the keyboard) finds what it read drawn, and not one asked for
 * meanwhile (by what the daemon said of the move) still on its way.
 */
export function readTopics(): Promise<void> {
  const mine = ++asked;
  reading = topicsApi.list().then(
    (read) => {
      if (mine !== asked) return reading ?? undefined;
      topics.value = read.map(lineOf);
      topicsRead.value = true;
    },
    () => {
      if (mine !== asked) return reading ?? undefined;
      topics.value = [];
      topicsRead.value = false;
    },
  );
  return reading;
}

/** Reads them where they were not read yet: what shows them asks this as it is first drawn, and asks nothing again for being drawn again. */
export function wantTopics(): void {
  if (!topicsRead.peek() && reading === null) void readTopics();
}

/** The topic a name means, among those read, reading them first where they were not: by its id, or its title whatever its capitals. */
export async function topicNamed(name: string): Promise<TopicLine | null> {
  if (!topicsRead.peek()) await readTopics();
  return named(topics.peek(), name);
}

/** Every topic, reading them first where they were not: what finds which page an address names. */
export async function topicsKnown(): Promise<TopicLine[]> {
  if (!topicsRead.peek()) await readTopics();
  return topics.peek();
}

/** What a topic is called, as the page read it last; nothing where it was not read. */
export function titleOf(id: string): string | null {
  return topics.value.find((topic) => topic.id === id)?.title ?? null;
}

/** Lets go of them, when nobody is signed in. */
export function forgetTopics(): void {
  asked++;
  reading = null;
  topicsRead.value = false;
  topics.value = [];
}

/** A reply, as its page's reading hands it. */
export type { Reply };

// Several are said at once (a topic made says it was created and revised): read once for them.
const changed = coalesce(() => void readTopics());
stream.on(['record.*', 'topic.*'], (event) => {
  if (reading === null) return;
  if (topicOfEvent(event)) changed();
});
// What was said while the page could not hear was not heard.
stream.onStatus((up) => {
  if (up && reading !== null) changed();
});
