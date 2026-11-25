// The Members page: everybody in the factory, the people on one tab and
// the agents on the other (`views/members.tsx`). Which of the two is shown
// is asked for apart from the tab, as the page about the program's are
// (`aboutshown.ts`), so that the page brings each asking to its tab once
// and a tab shown again asks for nothing.
//
// The People tab is the page's address (`/#/members`); the Agents tab is
// the address the Agents page had (`/agents`), so that a link, a bookmark
// or a tab kept from before that says `agents` opens the agents where they
// are now. Nothing of the page is imported here, so that the store may ask
// for the agents (`showAgent`) without the two needing each other.

import { signal } from '@preact/signals';

/** The Members page's two tabs, in the order they are drawn. */
export const MEMBERS_TABS = ['people', 'agents'] as const;
export type MembersTab = (typeof MEMBERS_TABS)[number];

/** What each tab is called. */
export const MEMBERS_CALLED: Record<MembersTab, string> = { people: 'People', agents: 'Agents' };

/** Which tab what an address or a kept tab said is: the People tab for anything that is not the agents. */
export function membersTabOf(said: string | null | undefined): MembersTab {
  return said === 'agents' ? 'agents' : 'people';
}

/** The tab asked for, and which asking it was. */
export const membersAsked = signal<{ at: MembersTab; asked: number } | null>(null);

/** Asks the page for one of its tabs: it is brought there as it is drawn, or at once where it is. */
export function askMembers(at: MembersTab): void {
  membersAsked.value = { at, asked: (membersAsked.peek()?.asked ?? 0) + 1 };
}

/** The tab the page shows now, as the page chooses it: what the address bar says. */
export const membersShowing = signal<MembersTab>('people');

/** Says which tab the page shows now. */
export function sayMembersShowing(at: MembersTab): void {
  if (membersShowing.peek() !== at) membersShowing.value = at;
}
