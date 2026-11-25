// The tab of a project's Git page somebody asked for: an address that
// names one (`/<project>/git#pulls`), said apart from the tab as an entry
// of the Reference is (`referenced.ts`), so that the page brings each
// asking to the tab once and a tab shown again asks for nothing. What a
// tab is known by is `gittabs.ts`.

import { signal } from '@preact/signals';
import { openTab } from './store.ts';

/** The tab asked for, of which project, and which asking it was. */
export const gitAsked = signal<{ project: string; at: string; asked: number } | null>(null);

/** Opens a project's Git page at a tab, by what the tab is known by; with none, as it stands. */
export function showGit(project: string, at = ''): void {
  if (at) gitAsked.value = { project, at, asked: (gitAsked.peek()?.asked ?? 0) + 1 };
  openTab({ kind: 'git', project });
}

/**
 * The tab each project's Git page shows now, by what it is known by, and
 * nothing for the first: what the address bar says after the `#` while the
 * page is showing (`address.ts`, `shownNow`).
 */
export const gitShowing = signal<Record<string, string>>({});

/** Says which tab a project's Git page shows now, as the page chooses it. */
export function sayGitShowing(project: string, at: string): void {
  if ((gitShowing.peek()[project] ?? '') !== at) gitShowing.value = { ...gitShowing.peek(), [project]: at };
}
