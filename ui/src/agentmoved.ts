// An agent moved to another project, or to none, from its line on the
// Agents tab (`views/agentmoved.tsx`): who is offered it, and what is
// said of it before and after. It holds nothing of the page, so that its
// tests can read it.
//
// The daemon holds the rule (`Agents::move_to`): what is offered here is
// only what it would let through as things stand. An agent that
// something running holds (a fleet, by its row) says so (`held_by`), and
// its project is its fleet's, changed in that row and never here.

import { covers } from './holds.ts';

/** Whether whoever holds `mine` is offered moving the agent: one that nothing running holds, to somebody who may change agents and holds all it holds. */
export function mayMove(slot: { held_by?: string | null; scopes: string[] }, mine: string[]): boolean {
  return !slot.held_by && covers(mine, 'agents:write') && slot.scopes.every((scope) => covers(mine, scope));
}

/** What is said in the question of what a move to `to` (a project's name, or nothing for none) does, and of what it leaves as it was. */
export function movingSaid(slot: { external: boolean; active: boolean }, to: string | null): string[] {
  const where = slot.external
    ? to
      ? `It is listed under ${to}, and offered that project’s cards.`
      : 'It is listed under no project.'
    : to
      ? `From its next piece of work its wall shows its program ${to}’s folder, and it is listed under ${to}.`
      : 'From its next piece of work its wall shows its program a home of its own and no project’s folder, and it is listed under no project.';
  const stays = slot.active ? 'Nothing else of it changes: it holds what it held.' : 'Nothing else of it changes: it holds what it held, and it stays stopped.';
  return [where, stays, 'One with work in hand is not moved until that is over.'];
}

/** What is said once it is moved. */
export function movedSaid(name: string, to: string | null): string {
  return to ? `${name} works in ${to} from its next piece of work.` : `${name} works in no project from its next piece of work.`;
}
