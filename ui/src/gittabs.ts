// The tabs of a project's Git page, as functions of what the page read:
// what git says of the repository itself first (Changes, Commits,
// Branches, Remotes), then a tab for each part an installed package says
// it shows of a repository it serves, as the package names it and calls
// it. Nothing here knows a service or a package: a part is its package's
// name, its own name and its title.
//
// A tab is known by a key, which is what the page keeps as the tab that
// shows and what an address says after its `#` (`/<project>/git#pulls`):
// git's own by a word, a part by its own name where no other tab is known
// by that, and by its package's name and its own (`sample.pulls`) where
// another is.

import type { Offered, Part, Seen } from './gitpage.ts';

/** What git says of the repository itself, a tab each, in the order they are drawn. */
export const OWN = [
  { key: 'changes', title: 'Changes' },
  { key: 'commits', title: 'Commits' },
  { key: 'branches', title: 'Branches' },
  { key: 'remotes', title: 'Remotes' },
] as const;

export type Own = (typeof OWN)[number]['key'];

/** One tab of the page. */
export interface GitTab {
  key: string;
  title: string;
  /** How many there are of what it shows, where the page knows without reading anything more. */
  count?: number;
  /** Of git's own, which it is. */
  own?: Own;
  /** Of a package's, the package, what it is called, and the part. */
  package?: string;
  packageTitle?: string;
  part?: Part;
  /** Why whoever reads may not read it, where they may not: the tab is there and says so. */
  refused?: string;
  /** Where another part is called the same: the package's title is said beside it. */
  whose?: boolean;
}

/** What a part's tab is known by, among every part offered. */
export function partKey(offered: Offered[], pkg: string, name: string): string {
  const taken =
    OWN.some((one) => one.key === name) ||
    offered.some((other) => other.package !== pkg && other.parts.some((part) => part.name === name));
  return taken ? `${pkg}.${name}` : name;
}

/** The tabs of the page: git's own, then each part of each package offered, in the order the daemon handed them. */
export function gitTabs(seen: Seen | null): GitTab[] {
  if (!seen?.repository) return [];
  const counts: Record<Own, number | undefined> = {
    changes: (seen.changes?.length ?? 0) + (seen.more ?? 0),
    commits: undefined,
    branches: seen.branches?.length ?? 0,
    remotes: seen.remotes?.length ?? 0,
  };
  const own: GitTab[] = OWN.map((one) => ({ key: one.key, title: one.title, own: one.key, count: counts[one.key] }));
  const offered = seen.shows ?? [];
  const titles = offered.flatMap((one) => one.parts.map((part) => part.title.toLowerCase()));
  const parts: GitTab[] = offered.flatMap((one) =>
    one.parts.map((part) => ({
      key: partKey(offered, one.package, part.name),
      title: part.title,
      package: one.package,
      packageTitle: one.title,
      part,
      ...(one.refused ? { refused: one.refused } : {}),
      ...(titles.filter((title) => title === part.title.toLowerCase()).length > 1 || OWN.some((own) => own.title.toLowerCase() === part.title.toLowerCase())
        ? { whose: true }
        : {}),
    })),
  );
  return [...own, ...parts];
}

/** The tab that shows: the one kept, where it is one of the page's, else the first. Nothing while there are none. */
export function showing(tabs: GitTab[], kept: string | null): GitTab | null {
  return tabs.find((tab) => tab.key === kept) ?? tabs[0] ?? null;
}

/** What a part's reading is kept under: its package and its name, whatever its tab is called. */
export const readKey = (tab: GitTab | null | undefined): string | null => (tab?.package && tab.part ? `${tab.package}/${tab.part.name}` : null);

/** Whether the tab that shows is a part still to be read: one whose package whoever reads may use, read nothing of yet. */
export const toRead = (tab: GitTab | null | undefined, read: Record<string, unknown>): boolean => {
  const key = tab && readKey(tab);
  return !!key && !tab?.refused && read[key] === undefined;
};
