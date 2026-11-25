// Whom a card is given to, as its face says it: "Assigned to" and then
// who, with what kind of somebody they are where the page knows it. It is
// worked out from what the page has read (the agents, who is signed in and
// the people with an account here, `peopleknown.ts`): a person is said by
// their name, and where nothing names them as somebody, never by the id of
// their account. This holds nothing of the page, so that its tests can read it.

import { isExternal } from './external.ts';

/** Who a card is given to, to be said: their name, and what kind of somebody they are, where that is known. */
export interface Assigned {
  name: string;
  /** `external` (an agent the factory does not start), `agent`, `bot`, `you`, `person`, or nothing where it is not known. */
  kind: 'external' | 'agent' | 'bot' | 'you' | 'person' | null;
  /** Whether it is an agent, which is drawn in the agents' colour. */
  agent: boolean;
}

/** What a person is said as where nothing names them: an account's id is no name. */
export const SOMEONE = 'someone';

/**
 * Whom `assignee` (`agent:<name>`, `user:<id>`, `bot:<name>`, or a word a
 * program from before wrote) names, as a card says it. `agents` are the
 * agents the page has read, `me` whoever is signed in, and `people` the
 * people with an account here, each by actor and name.
 */
export function assignedTo(
  assignee: string,
  agents: readonly { name: string; adapter: string }[],
  me: { actor: string; name: string | null } | null,
  people: readonly { actor: string; name: string }[] = [],
): Assigned {
  const colon = assignee.indexOf(':');
  const [kind, name] = colon < 0 ? ['', assignee] : [assignee.slice(0, colon), assignee.slice(colon + 1)];
  if (kind === 'agent') {
    const slot = agents.find((one) => one.name === name);
    return { name, kind: isExternal(slot) ? 'external' : 'agent', agent: true };
  }
  if (kind === 'user') {
    if (me && me.actor === assignee && me.name) return { name: me.name, kind: 'you', agent: false };
    const named = people.find((one) => one.actor === assignee)?.name.trim();
    return { name: named || SOMEONE, kind: 'person', agent: false };
  }
  if (kind === 'bot') return { name, kind: 'bot', agent: false };
  return { name: assignee, kind: null, agent: false };
}
