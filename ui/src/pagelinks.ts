// Links between topics, as plain values: how a page names another (or one
// of its replies) in what is written, and how one is typed. Nothing of the
// page is in it, so that its cases can read it; `views/pagelink.tsx` draws
// a link, and the daemon reads the same form (`kernel::topics::linked`),
// case for case with `linkcases.json`.
//
// A link is a Markdown link whose address is the page's id:
//
//   [Chytrid fungus](topic:@-topics-49)          the page
//   [what Ada said](topic:@-topics-49#817039)    one of its replies
//
// so that it reads well as text, an agent can write it, it goes through
// the block editor as any link does, and neither a new title nor a move
// changes where it leads. The words are the writer's, as any link's are.
// Typing `[[` offers the pages by title, as `@` offers people, and the
// page chosen is written in this form with its title as the words.

import type { TopicLine } from './topicpage.ts';
import type { Tree } from './wikis.ts';
import { pathOf } from './wikis.ts';

/** How the address of a link to a topic begins. */
export const PAGE_LINK = 'topic:';

/** Where a link to a topic leads: the page by its id, and one of its replies by the number it was said at. */
export interface PageLink {
  topic: string;
  reply: number | null;
}

/** Where an address leads, where it is a link to a topic: `topic:`, an id of letters, digits and `@_.-`, and `#` and a number for a reply. */
export function pageLinkOf(to: string): PageLink | null {
  if (!to.startsWith(PAGE_LINK)) return null;
  const found = /^([A-Za-z0-9@_.-]+)(?:#(\d+))?$/.exec(to.slice(PAGE_LINK.length));
  if (!found) return null;
  return { topic: found[1]!, reply: found[2] === undefined ? null : Number(found[2]) };
}

/** The address of a link to a page, or to one of its replies. */
export const pageAddress = (topic: string, reply: number | null = null): string => `${PAGE_LINK}${topic}${reply === null ? '' : `#${reply}`}`;

/**
 * A title as the words of a link: on one line, and with no brackets, which
 * would end the words early (a `]` written `\]` is kept by the editor, but
 * not read as one by every reader of the text).
 */
export const linkWords = (title: string): string => title.replace(/\[/g, '(').replace(/\]/g, ')').replace(/\s+/g, ' ').trim() || 'a page';

/** A link to a page as it is written, its title the words. */
export const writtenLink = (title: string, topic: string, reply: number | null = null): string => `[${linkWords(title)}](${pageAddress(topic, reply)})`;

/** What is drawn in place of a link to a page the reader cannot open: it was removed, or it is not theirs to read. Never its title. */
export const CANNOT_OPEN = 'a page you cannot open';
export const CANNOT_OPEN_WHY = 'A link to a page you cannot open: it was removed, or it is not yours to read.';

/** The `[[` being typed before the caret, where one is: where its `[[` begins and what was typed after it. */
export function pageTyping(text: string, caret: number): { from: number; typed: string } | null {
  const found = /\[\[([^[\]\n]{0,60})$/.exec(text.slice(0, caret));
  return found ? { from: caret - found[0].length, typed: found[1]! } : null;
}

/** The text with the `[[` being typed made a link to `page`, and where the caret then is: after it. */
export function completePage(text: string, caret: number, from: number, page: { id: string; title: string }): { text: string; caret: number } {
  // What closed it, typed before it was chosen, goes with it.
  const rest = text.slice(caret).replace(/^\]\]/, '');
  const put = writtenLink(page.title, page.id);
  return { text: text.slice(0, from) + put + rest, caret: from + put.length };
}

/** A page offered as `[[` is typed: it, and the wiki it is in where it is under one. */
export interface Offered {
  page: TopicLine;
  wiki: string | null;
}

/** How many pages are offered at once. */
export const MOST_OFFERED = 8;

/**
 * The pages whose title starts with what was typed, then those whose
 * title holds it, whatever their capitals; the page being written is not
 * offered. Each with the title of the wiki it is in, where it is under one.
 */
export function pagesOffered(lines: readonly TopicLine[], tree: Tree, typed: string, except: string | null = null): Offered[] {
  const wanted = typed.trim().toLowerCase();
  const all = lines.filter((line) => line.id !== except);
  const starts = all.filter((line) => line.title.toLowerCase().startsWith(wanted));
  const holds = all.filter((line) => !starts.includes(line) && line.title.toLowerCase().includes(wanted));
  return [...starts, ...holds].slice(0, MOST_OFFERED).map((page) => {
    const wiki = pathOf(tree, page.id)[0];
    return { page, wiki: wiki && wiki !== page.id ? (lines.find((line) => line.id === wiki)?.title ?? null) : null };
  });
}
