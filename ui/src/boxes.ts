// The boxes an encyclopedia's article has besides its text, read from what
// was written: a function of plain text with nothing of the page in it, so
// that its tests can read it; `views/boxes.tsx` draws them.
//
// Each is a block of code in a language of its own, so that it stays text
// an agent can write, goes through the block editor and every revision as
// it was, and needs nothing stored beside the post:
//
//   ```infobox                               a box of key facts, at the side
//   title: Frog
//   picture: attachment:ratt_1a2b            (and caption: …), where it has one
//   Order: Anura
//   Range: every continent but [Antarctica](topic:@-topics-7)
//   ```
//
//   ```series                                pages that go together, at the side
//   title: Amphibians                        (place: end puts it at the article's end)
//   ## Frogs and toads                       a group, where they are grouped
//   [Frogs](topic:@-topics-5)                one page a line
//   ```
//
//   ```note                                  a line or two at the article's very top
//   This page is about frogs as animals. For frogs in stories, see [Frogs in culture](topic:@-topics-9).
//   ```
//
// The words after a key, a series' lines and a note are read as any line
// is (`prose.ts`), links to pages and plain links among them, and none is
// ever fetched. A block that cannot be read says what is wrong with it, in
// one line, and is drawn as its code where it was written. The daemon
// counts the links in these blocks as a page's links (`kernel::topics::BOXES`).

import { pageLinkOf } from './pagelinks.ts';
import type { Block } from './prose.ts';

/** The languages of the blocks drawn as boxes, as `codeblocks.ts` keeps a language: in small letters. */
export const BOX_LANGUAGES = ['infobox', 'series', 'note'] as const;
export type BoxLanguage = (typeof BOX_LANGUAGES)[number];

/** A fact of an info box: what it is (its key) and what it says, as written. */
export interface Row {
  key: string;
  value: string;
}

export interface InfoBox {
  is: 'infobox';
  title: string;
  /** Where its picture leads, as a picture's address is written, and what is said under it. */
  picture: { to: string; caption: string } | null;
  rows: Row[];
}

/** Pages of a series under a group's name, or under none where they are not grouped. */
export interface Group {
  name: string | null;
  /** Each page's line as written, and the page its link leads to. */
  pages: { line: string; topic: string }[];
}

export interface Series {
  is: 'series';
  title: string;
  /** At the article's side, or at its end. */
  place: 'side' | 'end';
  groups: Group[];
}

/** What a note at an article's top says: its lines as written. */
export interface Note {
  is: 'note';
  lines: string[];
}

export type Box = InfoBox | Series | Note;

/** A block read as a box, or what is wrong with it, in one line. */
export type BoxRead = { box: Box } | { wrong: string };

/** The most lines a note at the top has: it points elsewhere, and says no more. */
export const MOST_NOTE_LINES = 3;

/** Whether a block of code in a language is a box. */
export const isBoxLanguage = (lang: string | undefined): lang is BoxLanguage => (BOX_LANGUAGES as readonly string[]).includes(lang ?? '');

/** A line as `key: value`, where it is one: the key before the first `: ` (or a `:` that ends the line), both trimmed. */
function keyed(line: string): { key: string; value: string } | null {
  const found = /^([^:]*?\S)[ \t]*:(?:[ \t]+(.*)|[ \t]*)$/.exec(line.trim());
  return found ? { key: found[1]!.trim(), value: (found[2] ?? '').trim() } : null;
}

/** The lines of a block that say something, each with its number in the block (from 1). */
const said = (text: string) =>
  text
    .split('\n')
    .map((line, at) => ({ line: line.trim(), n: at + 1 }))
    .filter(({ line }) => line !== '');

/** An info box: `title:` first or anywhere, `picture:` and `caption:` where it has one, and a fact a line. */
export function infoBoxOf(text: string): BoxRead {
  let title: string | null = null;
  let picture: string | null = null;
  let caption: string | null = null;
  const rows: Row[] = [];
  for (const { line, n } of said(text)) {
    const pair = keyed(line);
    if (!pair) return { wrong: `Line ${n} is not a fact written as Key: value.` };
    if (pair.value === '') return { wrong: `Line ${n} says nothing after “${pair.key}:”.` };
    if (pair.key === 'title' || pair.key === 'picture' || pair.key === 'caption') {
      const was = pair.key === 'title' ? title : pair.key === 'picture' ? picture : caption;
      if (was !== null) return { wrong: `Line ${n} gives a second ${pair.key}: an info box has one.` };
      if (pair.key === 'title') title = pair.value;
      else if (pair.key === 'caption') caption = pair.value;
      else if (/\s/.test(pair.value)) return { wrong: `Line ${n}: a picture is one address, attachment:<id>.` };
      else picture = pair.value;
      continue;
    }
    rows.push(pair);
  }
  if (title === null) return { wrong: 'An info box needs a line title: <what it is about>.' };
  if (caption !== null && picture === null) return { wrong: 'A caption is of a picture: add a line picture: attachment:<id>.' };
  return { box: { is: 'infobox', title, picture: picture === null ? null : { to: picture, caption: caption ?? '' }, rows } };
}

/** The page a line of a series leads to: the first link to a page in it. */
function pageIn(line: string): string | null {
  const links = line.matchAll(/(?<!!)\[[^\]\n]+\]\(([^)\s]+)\)/g);
  for (const link of links) {
    const page = pageLinkOf(link[1]!);
    if (page) return page.topic;
  }
  return null;
}

/** A series: `title:` and `place:` where it says one, `## Group` lines, and one page a line, each a link to it. */
export function seriesOf(text: string): BoxRead {
  let title: string | null = null;
  let place: Series['place'] | null = null;
  const groups: Group[] = [];
  let count = 0;
  for (const { line, n } of said(text)) {
    const group = /^##[ \t]+(\S.*)$/.exec(line);
    if (group) {
      groups.push({ name: group[1]!.trim(), pages: [] });
      continue;
    }
    const pair = keyed(line);
    if (pair?.key === 'title') {
      if (pair.value === '') return { wrong: `Line ${n} says nothing after “title:”.` };
      if (title !== null) return { wrong: `Line ${n} gives a second title: a series has one.` };
      title = pair.value;
      continue;
    }
    if (pair?.key === 'place') {
      if (pair.value !== 'side' && pair.value !== 'end') return { wrong: `Line ${n}: a series is placed at the side or at the end (place: side, place: end).` };
      place = pair.value;
      continue;
    }
    const topic = pageIn(line);
    if (topic === null) return { wrong: `Line ${n} is not a link to a page: write it [Its title](topic:<id>), typed as [[.` };
    if (groups.length === 0) groups.push({ name: null, pages: [] });
    groups.at(-1)!.pages.push({ line: line.replace(/^[-*•][ \t]+/, ''), topic });
    count += 1;
  }
  if (title === null) return { wrong: 'A series needs a line title: <what its pages have in common>.' };
  if (count === 0) return { wrong: 'A series lists its pages, a link a line: none is written yet.' };
  return { box: { is: 'series', title, place: place ?? 'side', groups: groups.filter((one) => one.pages.length > 0) } };
}

/** A note at the top: a line or two that say something. */
export function noteOf(text: string): BoxRead {
  const lines = said(text).map(({ line }) => line);
  if (lines.length === 0) return { wrong: 'A note says where else to look: it says nothing yet.' };
  if (lines.length > MOST_NOTE_LINES) return { wrong: `A note at the top is a line or two; this one has ${lines.length}.` };
  return { box: { is: 'note', lines } };
}

/** A block of code in a box's language, read as its box. */
export function boxOf(lang: BoxLanguage, text: string): BoxRead {
  return lang === 'infobox' ? infoBoxOf(text) : lang === 'series' ? seriesOf(text) : noteOf(text);
}

/** Whether a series has a page among its lines (the page it is drawn on, marked there). */
export const lists = (series: Series, page: string | null): boolean => page !== null && series.groups.some((group) => group.pages.some((one) => one.topic === page));

/**
 * What an article draws apart from its text, of the blocks `prose.ts`
 * reads in its post: the first note, at its top; the info boxes and the
 * series at its side, in the order written, but for a series placed at
 * its end. `apart` is where each of those is among the blocks, which the
 * text then does not draw again. A box that cannot be read is drawn where
 * it was written, as its code, and so is every note after the first.
 */
export interface Boxed {
  head: Note | null;
  side: (InfoBox | Series)[];
  end: Series[];
  apart: Set<number>;
}

export function boxedOf(blocks: Block[]): Boxed {
  const boxed: Boxed = { head: null, side: [], end: [], apart: new Set() };
  blocks.forEach((block, at) => {
    if (block.is !== 'code' || !isBoxLanguage(block.lang)) return;
    const read = boxOf(block.lang, block.text);
    if (!('box' in read)) return;
    const { box } = read;
    if (box.is === 'note') {
      if (boxed.head) return;
      boxed.head = box;
    } else if (box.is === 'series' && box.place === 'end') boxed.end.push(box);
    else boxed.side.push(box);
    boxed.apart.add(at);
  });
  return boxed;
}

/** What the block editor's `/` menu puts in for each box: a block to be filled in, which reads as one. */
export const STARTERS: Record<BoxLanguage, string> = {
  infobox: 'title: What it is about\nKey: value',
  series: 'title: What its pages have in common\n',
  note: 'This page is about … For …, see ',
};
