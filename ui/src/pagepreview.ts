// What the card over a link to a page says (`views/pagepreview.tsx` draws
// it), as plain values: the page's title, where it is (its wiki and the
// page above it), the first few lines of its summary or, where it has
// none, of its article, as plain text, and how many replies it has; for a
// link to a reply, that reply's first lines and who said it.
//
// What it is worked out from is what the client holds of the topics (each
// page's line, `topicsread.ts`), and the page whole where that was read.
// Only pages among those lines are named: a page the reader may not read
// is never in them, so neither its title nor anything of it is said, even
// where a page they may read links to it.

import { CANNOT_OPEN } from './pagelinks.ts';
import { read } from './prose.ts';
import type { Span } from './prose.ts';
import type { Reply } from './replies.ts';
import type { TopicLine, TopicWhole } from './topicpage.ts';
import { pathOf, readTree } from './wikis.ts';

/** About how many characters of what a page says the card shows: a few lines at its width. */
export const MOST_SAID = 240;

/** The words of spans, as plain text: links as their words, a link to a page the reader may not read as one they cannot open, no pictures or markers. */
function wordsOf(all: readonly Span[], readable: (id: string) => boolean): string {
  return all
    .map((span): string => {
      if (span.is === 'text' || span.is === 'code') return span.text;
      if (span.is === 'page') return readable(span.topic) ? wordsOf(span.inside, readable) : CANNOT_OPEN;
      if (span.is === 'cite' || span.is === 'image') return '';
      return wordsOf(span.inside, readable);
    })
    .join('');
}

/** `text` cut to about `most` characters, at the end of a word, with an ellipsis where anything was cut. */
export function cut(text: string, most = MOST_SAID): string {
  if (text.length <= most) return text;
  const short = text.slice(0, most);
  const at = short.lastIndexOf(' ');
  return `${(at > most / 2 ? short.slice(0, at) : short).replace(/[\s,;:.–—-]+$/, '')}…`;
}

/**
 * The first lines of what is written, as plain text on one line: its
 * paragraphs, quotes and lists, without headings, pictures, code, diagrams
 * or sources; cut to `most` characters. Nothing where it says nothing.
 */
export function plainOf(markdown: string, readable: (id: string) => boolean, most = MOST_SAID): string | null {
  const said: string[] = [];
  let length = 0;
  for (const block of read(markdown)) {
    if (length > most) break;
    const parts = block.is === 'text' || block.is === 'quote' ? [block.spans] : block.is === 'list' ? block.items.map((item) => item.spans) : [];
    for (const spans of parts) {
      const words = wordsOf(spans, readable).replace(/\s+/g, ' ').trim();
      // A rule between parts is read as a line of marks, and says nothing.
      if (words === '' || /^[-*_ ]{3,}$/.test(words)) continue;
      said.push(words);
      length += words.length + 1;
    }
  }
  return said.length === 0 ? null : cut(said.join(' '), most);
}

/** Where a page is: the wiki it is in and the page right above it, by their titles; nothing of either where it is a wiki, or above one the reader may not read. */
export function whereOf(lines: readonly TopicLine[], id: string): { wiki: string | null; under: string | null } {
  const path = pathOf(readTree(lines.map((line) => ({ id: line.id, parent: line.parent, position: line.position, space: line.workspace }))), id);
  const title = (at: string | undefined) => (at === undefined ? null : (lines.find((line) => line.id === at)?.title ?? null));
  if (path.length < 2) return { wiki: null, under: null };
  return { wiki: title(path[0]), under: path.length > 2 ? title(path.at(-2)) : null };
}

/** How many replies a page has, in words. */
export const repliesSaid = (n: number): string => (n === 0 ? 'No replies' : n === 1 ? '1 reply' : `${n} replies`);

/** What the card says of a page or one of its replies. */
export interface Preview {
  title: string;
  wiki: string | null;
  under: string | null;
  /** The first lines of its summary or article; nothing where it says nothing, or that is not read yet (`reading`). */
  says: string | null;
  replies: string;
  /** Whether what it says is still to be read, the page whole being read for it. */
  reading: boolean;
  /** For a link to a reply: who said it and its first lines; `gone` where the page was read and it is not there. */
  reply: { who: string; says: string | null } | 'gone' | null;
}

/**
 * Whether the page whole is wanted for the card: where its line has no
 * summary (what it says is then the start of its article), or the link is
 * to one of its replies.
 */
export const wantsWhole = (line: TopicLine, reply: number | null): boolean => reply !== null || line.summary === null;

/**
 * What the card says of the page `line`, or of its reply said at `reply`,
 * from every line the reader holds and, where it was read, the page whole
 * with its replies. A page whole that was read says more than its line.
 */
export function previewOf(lines: readonly TopicLine[], line: TopicLine, reply: number | null, whole: { topic: TopicWhole; replies: readonly Reply[] } | null): Preview {
  const readable = (id: string) => lines.some((one) => one.id === id);
  const says = whole ? (plainOf(whole.topic.summaryWhole, readable) ?? plainOf(whole.topic.body, readable)) : line.summary === null ? null : plainOf(line.summary, readable);
  const replies = whole ? Math.max(whole.topic.replies, whole.replies.length) : line.replies;
  const said = reply === null ? null : whole ? (whole.replies.find((one) => one.seq === reply) ?? 'gone') : null;
  return {
    title: line.title,
    ...whereOf(lines, line.id),
    says,
    replies: repliesSaid(replies),
    reading: whole === null && wantsWhole(line, reply),
    reply: said === null || said === 'gone' ? said : { who: said.who || 'Somebody', says: plainOf(said.body, readable) },
  };
}
