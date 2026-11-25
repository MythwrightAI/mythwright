// A page of a wiki as its own page has it: which
// page an address means among the topics the page read, how a page's
// address says it, and where it is drawn as being. Plain values, so that
// its cases can read it; the tree itself is `wikis.ts`.
//
// An address is `/topic/<wiki>/<page>`, by their titles: its wiki's and its
// own, never the pages between, so that a page moved within its wiki keeps
// its address. A wiki is `/topic/<wiki>`. A title that is not the one page
// of its wiki called so is said by its id, and a wiki's title another wiki
// has by the wiki's id. Every address there was still opens: `/topic/<title>`
// is the one page of any wiki called so, and `/topic/<id>` its page.

import { isTopicId, titleKey } from './topicpage.ts';
import type { TopicLine } from './topicpage.ts';
import { besideOf, pathOf, readTree, subtreeOf } from './wikis.ts';
import type { Tree } from './wikis.ts';

/** A page as what it is under and how it stands says it: its id and title. */
export interface Named {
  id: string;
  title: string;
}

/** The tree of the topics as the page read them. */
export function treeOfLines(lines: readonly TopicLine[]): Tree {
  return readTree(
    lines.map((line) => ({
      id: line.id,
      parent: line.parent,
      position: line.position,
      space: null,
    })),
  );
}

const called = (line: TopicLine, key: string) => titleKey(line.title) === key;

/**
 * How a page's address says it (`addressOf` in `tab.ts`): its title, and
 * its wiki's where it is under one, each only where it says which it is;
 * nothing where it is said by its id.
 */
export function addressNames(lines: readonly TopicLine[], id: string): { title?: string; wiki?: string } {
  const line = lines.find((one) => one.id === id);
  if (!line) return {};
  const tree = treeOfLines(lines);
  const path = pathOf(tree, id);
  const key = titleKey(line.title);
  if (path.length <= 1) return lines.filter((one) => called(one, key)).length === 1 ? { title: line.title } : {};
  const wiki = lines.find((one) => one.id === path[0])!;
  const inWiki = subtreeOf(tree, wiki.id).filter((page) =>
    called(
      lines.find((one) => one.id === page)!,
      key,
    ),
  );
  if (inWiki.length !== 1) return {};
  const wikis = besideOf(tree, null).filter((top) =>
    called(
      lines.find((one) => one.id === top.id)!,
      titleKey(wiki.title),
    ),
  );
  return { title: line.title, wiki: wikis.length === 1 ? wiki.title : wiki.id };
}

/** What an address names: the page, the pages it could be, or nothing. */
export type Found = { found: TopicLine } | { several: TopicLine[] } | null;

const ofThese = (these: TopicLine[]): Found => (these.length === 1 ? { found: these[0]! } : these.length > 1 ? { several: these } : null);

/**
 * The page `name` names, in the wiki `wiki` names where it says one: the
 * page of that wiki called so, whatever its capitals; failing that, the one
 * page of any wiki called so; where several are, which they are. An id is
 * that page, wherever it is.
 */
export function namedIn(lines: readonly TopicLine[], wiki: string | null, name: string): Found {
  if (isTopicId(name)) return ofThese(lines.filter((line) => line.id === name));
  const key = titleKey(name);
  if (wiki !== null) {
    const tree = treeOfLines(lines);
    const wikis = besideOf(tree, null)
      .map((top) => lines.find((one) => one.id === top.id)!)
      .filter((top) => top.id === wiki || titleKey(top.title) === titleKey(wiki));
    const pages = wikis.flatMap((top) => subtreeOf(tree, top.id)).map((page) => lines.find((one) => one.id === page)!);
    const there = ofThese(pages.filter((page) => called(page, key)));
    if (there) return there;
  }
  return ofThese(lines.filter((line) => called(line, key)));
}

/** A page's path in words, its wiki first: what the inbox says it by, and what says which of several a page is. */
export function pathSaid(above: readonly string[], title: string): string {
  return [...above, title].join(' › ');
}

/** The pages above a page as they are drawn over its title: its wiki and the last two, the rest folded into `…`. */
export function crumbsOf<T>(path: readonly T[]): {
  shown: (T | 'fold')[];
  folded: T[];
} {
  if (path.length <= 3) return { shown: [...path], folded: [] };
  return {
    shown: [path[0]!, 'fold', ...path.slice(-2)],
    folded: path.slice(1, -2),
  };
}

/** The pages directly under `id`, in the order they stand, as the list the page read says them: what tells it that one was made, moved or taken away. */
export function underOf(lines: readonly TopicLine[], id: string): string[] {
  return besideOf(treeOfLines(lines), id).map((line) => line.id);
}

/** Whether something said of the topics is of this page's place in its wiki: a page moved to it or from it. */
export function movedHereOrAway(event: { topic: string; data?: unknown }, id: string): boolean {
  if (event.topic !== 'topic.moved') return false;
  const data = (event.data ?? {}) as {
    parent?: unknown;
    was?: { parent?: unknown };
  };
  return data.parent === id || data.was?.parent === id;
}
