// The Members line under Factory in the Resources panel (it was
// the last branch of the Channels panel), as the page says it: who there
// is in the workspace the panel is of, what can be done with each from
// their line's menu, and a direct message with each, with
// nothing of the page in it.
//
// A direct message is a conversation (`conversed.ts`) of exactly two:
// whoever reads this and the one it is with. Nothing in the daemon knows
// it as anything else, and nothing here asks it to: which conversation is
// the one with somebody is worked out from the channels the page reads,
// which are those whoever reads may read, and a new one is written as any
// conversation is, to that one alone.
//
// Who is listed is everybody with an account here (a workspace lets
// nobody in and keeps nobody out, so every person is of every workspace),
// and of the agents those the workspace has: those of its projects, by
// the one rule that says which projects a workspace lists
// (`holdsProject`), and the factory's own, which are no project's. Never
// a bot: a bot is something a blueprint says things as, and nobody to
// write to.

import { actorKind } from './util.ts';
import { holdsProject } from './workspaces.ts';
import type { Held, Workspace } from './workspaces.ts';
import type { Line } from './inbox.ts';

/** One line of the part. */
export interface MemberLine {
  actor: string;
  name: string;
  /** Whoever reads this, a person, or an agent. */
  kind: 'you' | 'person' | 'agent';
  /** The few faint words beside the name: `you`, or what an agent is. Nothing for a person. */
  said: string | null;
  /** An agent's slot, which is what how it stands is kept by. */
  slot: string | null;
  /** An agent that is stopped, to which nothing new is written. */
  stopped: boolean;
}

/** As much of an agent as the part goes by. */
export interface Agented {
  name: string;
  adapter: string;
  project: string | null;
  active: boolean;
}

/** What runs an agent the factory does not start (`agents::EXTERNAL`). */
const EXTERNAL = 'external';

/** What an agent is, in the faint words beside it, as the list of who is in a channel says it. */
export function agentSaid(agent: Agented): string {
  if (!agent.active) return 'stopped';
  return agent.adapter === EXTERNAL ? 'agent (external)' : 'agent';
}

const byName = new Intl.Collator('en', { numeric: true, sensitivity: 'base' });

/**
 * The part's lines: whoever reads this first, then every other person by
 * name, then the workspace's agents, those that are not stopped first and
 * each by its name, `worker-2` before `worker-10`. `held` is what the
 * panel is held to; with no workspace it is everything, and so are the
 * agents.
 */
export function membersListed(people: { actor: string; name: string }[], agents: Agented[], me: { actor: string; name: string } | null, held: Held, workspaces: Workspace[]): MemberLine[] {
  const persons = people.filter((one) => actorKind(one.actor) === 'user');
  const you = me && actorKind(me.actor) === 'user' ? (persons.find((one) => one.actor === me.actor) ?? me) : null;
  const others = persons
    .filter((one) => one.actor !== me?.actor)
    .sort((one, other) => byName.compare(one.name, other.name))
    .map((one): MemberLine => ({ actor: one.actor, name: one.name, kind: 'person', said: null, slot: null, stopped: false }));
  const theirs = agents
    .filter((agent) => !agent.project || holdsProject(held, agent.project, workspaces))
    .sort((one, other) => Number(other.active) - Number(one.active) || byName.compare(one.name, other.name))
    .map((agent): MemberLine => ({ actor: `agent:${agent.name}`, name: agent.name, kind: 'agent', said: agentSaid(agent), slot: agent.name, stopped: !agent.active }));
  return [...(you ? [{ actor: you.actor, name: you.name, kind: 'you' as const, said: 'you', slot: null, stopped: false }] : []), ...others, ...theirs];
}

/** As much of a channel as says whether it is a direct message. */
export interface Talked {
  id: string;
  created: number;
  conversation?: boolean;
  archived?: unknown;
  members: { actor: string }[];
}

/** Whether a channel is a conversation of exactly `me` and `them`: no third, and no bot counted. */
export function isDirect(channel: Talked, me: string, them: string): boolean {
  if (channel.conversation !== true || me === them) return false;
  const within = new Set(channel.members.map((member) => member.actor).filter((actor) => actorKind(actor) !== 'bot'));
  return within.size === 2 && within.has(me) && within.has(them);
}

/**
 * The direct message of `me` with `them`, by its id: of the conversations
 * of exactly those two that the page has, one that is not archived before
 * one that is, and of those the one somebody said something in last, by
 * its line in the inbox, else the one made last. Nothing where there is
 * none, and nothing with oneself.
 */
export function directWith(channels: Talked[], lines: Line[], me: string, them: string): string | null {
  const raised = new Map(lines.filter((line) => line.kind === 'message').map((line) => [line.place, line.raised]));
  const when = (channel: Talked) => raised.get(channel.id) ?? channel.created;
  const found = channels
    .filter((channel) => isDirect(channel, me, them))
    .sort((one, other) => Number(!!one.archived) - Number(!!other.archived) || when(other) - when(one));
  return found[0]?.id ?? null;
}

/**
 * How many wait unread in a direct message, said on its line as a
 * channel says it: what the page heard said there since it was last
 * shown, and at least one where its line in the inbox waits unread,
 * which is what the inbox counts of it. Nothing in one that is archived,
 * which waits for nobody, and nothing where there is none.
 */
export function waitsIn(channel: { id: string; archived?: unknown } | null | undefined, unread: Record<string, number>, lines: Line[]): number {
  if (!channel || channel.archived) return 0;
  const line = lines.find((one) => one.kind === 'message' && one.place === channel.id);
  const waiting = line && line.read === null && line.away === null ? 1 : 0;
  return Math.max(unread[channel.id] ?? 0, waiting);
}

/**
 * What the line of a direct message in somebody's menu does: opens the
 * one there is, writes a new one, or is not there. Not there for whoever
 * reads this, who has nobody to write to on their own line; and where
 * there is none yet, not for somebody who may not write a conversation,
 * or for an agent that is stopped, as the dialog that writes one offers
 * none.
 */
export function pressOf(line: MemberLine, direct: string | null, mayWrite: boolean): 'open' | 'write' | null {
  if (line.kind === 'you') return null;
  if (direct) return 'open';
  if (!mayWrite || line.stopped) return null;
  return 'write';
}

/** A line of somebody's menu that is theirs to be offered whatever they are: who they are, a message, and what a person holds. */
export type MemberAct = 'details' | 'open' | 'write' | 'permissions';

/**
 * What a member's menu offers of its own, in its order: who they are,
 * always; the direct message (`pressOf`) where there is one to open or one
 * may be written; and of a person, for whoever manages the factory, what
 * they hold, which Settings' People says. An agent's menu has what its
 * line on the Agents tab offers after these (`agentacts.tsx`).
 */
export function memberActs(line: MemberLine, direct: string | null, mayWrite: boolean, manages: boolean): MemberAct[] {
  const message = pressOf(line, direct, mayWrite);
  return ['details', ...(message ? [message] : []), ...(manages && line.kind !== 'agent' ? (['permissions'] as const) : [])];
}

/** What a line says of itself to whoever points at it: that it opens what can be done with them. */
export function lineTitle(line: MemberLine): string {
  if (line.kind === 'you') return 'You: signed in here. Press for what can be done';
  const what = line.said ? `${line.name} (${line.said})` : line.name;
  return `${what}: who they are, a message, and what else can be done`;
}

/** How many wait unread in all of the part, said on it where it is folded. */
export function waitingAll(counts: number[]): number {
  return counts.reduce((sum, one) => sum + one, 0);
}
