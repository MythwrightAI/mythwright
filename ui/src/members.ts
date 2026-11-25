// Everybody in a channel, as the one list of them is drawn.
//
// People, agents and bots take part in a channel alike, and are read
// about in one place. The list is in an order that does not move as
// somebody joins: people, then agents, then bots, each in the order the
// channel has them; and each says what kind it is.

import { actorKind } from './util.ts';

/** What kind somebody is, as it is said beside them. */
export type Kind = 'person' | 'agent' | 'bot' | 'nobody';

export interface Listed {
  actor: string;
  name: string;
  kind: Kind;
  /** An agent's slot, which is what its state and its work are kept by; nothing for anybody else. */
  slot: string | null;
}

/** What kind an actor is: somebody with an account is a person, and what was said before there was a bot is nobody's. */
export function kindOf(actor: string): Kind {
  const kind = actorKind(actor);
  return kind === 'user' ? 'person' : kind === 'system' ? 'nobody' : kind;
}

const ORDER: Kind[] = ['person', 'agent', 'bot', 'nobody'];

/** The members of a channel as they are listed: by kind, and within a kind as the channel has them. */
export function listed(members: { actor: string; name: string }[]): Listed[] {
  const all = members.map(({ actor, name }) => {
    const kind = kindOf(actor);
    return { actor, name, kind, slot: kind === 'agent' ? actor.slice('agent:'.length) : null };
  });
  return ORDER.flatMap((kind) => all.filter((one) => one.kind === kind));
}

/** What is said under the list where it has nobody of a kind that there is a way to add: only agents, today. */
export function lacking(list: Listed[]): string | null {
  return list.some((one) => one.kind === 'agent') ? null : 'No agent is in this channel yet. Add one from who is in it, at the head of the channel.';
}
