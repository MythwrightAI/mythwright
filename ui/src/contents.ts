// What an article holds, as a list of its headings: each heading's words,
// how far in it stands (levels 1 to 3, as `prose.ts` reads them), and the
// name it is known by in an address (its slug), which is made from its
// words and is unique within the page, so that `#s/<slug>` after a
// topic's address opens it there (`tab.ts`). What `views/topiccontents.tsx`
// draws as Contents, and what a heading's link and its section's Edit are
// found by, in the read article as in the editor: here, as plain values,
// so that their cases can read them (`contents.test.ts`).

import type { Block, Span } from './prose.ts';

/** How many headings an article has before it is given Contents: fewer, and it looks as it always did. */
export const CONTENTS_FROM = 3;

/** A heading of an article: how far in, its words, and the name its address knows it by. */
export interface Heading {
  level: 1 | 2 | 3;
  title: string;
  slug: string;
}

/** An entry of the contents: its heading, its number (`2.1`), and the entries under it. */
export interface Entry {
  heading: Heading;
  number: string;
  under: Entry[];
}

/** The words of a run of marks, as they read with the marks taken off: what a heading's slug is made from. */
export function plainOf(spans: Span[]): string {
  return spans
    .map((span) => {
      if (span.is === 'text' || span.is === 'code') return span.text;
      // A picture, and a footnote's marker, are no words of the heading.
      if (span.is === 'image' || span.is === 'cite') return '';
      return plainOf(span.inside);
    })
    .join('');
}

/**
 * The name a heading's words are known by in an address: in small letters,
 * each run of what is no letter, mark or digit (in any script) a hyphen,
 * none at either end. Words with no letter or digit at all are `section`.
 */
export function slugOf(words: string): string {
  const slug = words
    .normalize('NFKC')
    .toLowerCase()
    .replace(/[^\p{L}\p{M}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '');
  return slug || 'section';
}

/** The slugs of headings in the order they stand, each unique: a second called the same is `-2`, a third `-3`. */
export function slugsOf(titles: string[]): string[] {
  const taken = new Set<string>();
  return titles.map((title) => {
    const first = slugOf(title);
    let slug = first;
    for (let nth = 2; taken.has(slug); nth += 1) slug = `${first}-${nth}`;
    taken.add(slug);
    return slug;
  });
}

/** Whether what follows `#s/` in an address could be a heading's slug: what a slug is made into is itself. */
export const isSlug = (text: string): boolean => text !== '' && slugOf(text) === text;

/** The headings of the blocks an article is drawn as, in the order they stand, each with its slug. */
export function headingsOf(blocks: Block[]): Heading[] {
  const found = blocks.flatMap((block) => (block.is === 'heading' ? [{ level: block.level, title: plainOf(block.spans).trim() }] : []));
  const slugs = slugsOf(found.map((one) => one.title));
  return found.map((one, nth) => ({ ...one, slug: slugs[nth]! }));
}

/**
 * The headings as contents: each under the last heading before it that
 * stands less far in, numbered in its place (`1`, `1.2`); one with none
 * before it standing less far in is at the top, however far in it is.
 */
export function contentsOf(headings: Heading[]): Entry[] {
  const top: Entry[] = [];
  const open: Entry[] = [];
  for (const heading of headings) {
    while (open.length > 0 && open.at(-1)!.heading.level >= heading.level) open.pop();
    const above = open.at(-1);
    const list = above ? above.under : top;
    const entry: Entry = { heading, number: `${above ? `${above.number}.` : ''}${list.length + 1}`, under: [] };
    list.push(entry);
    open.push(entry);
  }
  return top;
}

/**
 * Which section is being read: the last heading whose top (`tops`, from
 * the top of what scrolls) is at or above the line read at; none where
 * the first is still below it. Scrolled to the end (`end`, the bottom of
 * what shows), the last heading that shows is, as a short last section
 * never reaches the line.
 */
export function sectionAt(tops: number[], line: number, end?: number): number {
  let at = -1;
  tops.forEach((top, nth) => {
    if (top <= line || (end !== undefined && top < end)) at = nth;
  });
  return at;
}
