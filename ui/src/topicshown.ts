// A topic's page asked to show one of its tabs, and at one of its replies
// or revisions: an address that names its talk (`/topic/<title>#talk`,
// `#talk/r-42`) or its history (`#history`, `#history/3`), or one that names
// it alone, which is its article, or its article at a heading (`#s/<slug>`);
// an inbox line of somebody
// named in a reply, which opens its talk at that reply, marked and brought
// into view. Said here and taken up by the page as it is drawn, as an entry
// of the Reference is asked for (`referenced.ts`); and which tab the page
// shows now, which the address bar says while it shows (`address.ts`,
// `shownNow`), as a Git page's is (`gitshown.ts`).

import { signal } from '@preact/signals';
import { openTab } from './store.ts';
import type { TopicPart } from './topicpage.ts';

/** The tab, the reply or revision (`seq`) and the heading of its article (`section`, by its slug) a topic's page is asked to show, until the page takes it up. */
export const topicAsked = signal<{ topic: string; part: TopicPart; seq: number | null; section?: string } | null>(null);

/** Asks a topic's page to show a tab, at a reply, revision or heading where one is said, without opening it: whoever asks opens it. */
export function askTopic(topic: string, part: TopicPart, seq: number | null = null, section?: string): void {
  topicAsked.value = section ? { topic, part, seq, section } : { topic, part, seq };
}

/** Opens a topic's page on its talk, at the reply said at `seq`. */
export function showReply(topic: string, seq: number): void {
  askTopic(topic, 'talk', seq);
  openTab({ kind: 'topic', id: topic });
}

/**
 * What each topic's page shows now, by the topic's id: `talk`, or `history`
 * at the revision shown, and nothing for its article or a thread; and the
 * heading of its article it was last taken to (`section`).
 */
export const topicShowing = signal<Record<string, { part: string; at?: number; section?: string }>>({});

/** Says which tab a topic's page shows now, the revision shown on its history, and the heading its article was taken to, as the page chooses them. */
export function sayTopicShowing(topic: string, part: string, at?: number, section?: string): void {
  const was = topicShowing.peek()[topic];
  if ((was?.part ?? '') === part && was?.at === at && was?.section === section) return;
  topicShowing.value = { ...topicShowing.peek(), [topic]: { part, ...(at === undefined ? {} : { at }), ...(section ? { section } : {}) } };
}

/** The pages just made from a page's foot, whose title is to be written: each opens with its title chosen in its field. */
export const freshPages = new Set<string>();
