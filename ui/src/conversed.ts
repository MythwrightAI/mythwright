// A conversation, as the page says it: a message written to one or
// several people and agents, which is in the inbox of each of them.
//
// It is a channel underneath, kept to its members, that says it is a
// conversation (`conversation`), so its page is a channel's page and
// everything said, attached and threaded there is a channel's. What is
// its own is where it is listed: in the inbox, as one line that each
// thing somebody else says there raises, and not among the channels
// (`workspaces.ts` leaves it out of what the side panel gathers and
// counts). What is here is what its dialogs and its head are worked out
// from, with nothing of the page in it.

import { covers } from './holds.ts';
import { actorKind } from './util.ts';

/** As much of a channel as these go by. */
export interface Conversed {
  id: string;
  conversation?: boolean;
  members: { actor: string; name: string }[];
}

/** Whether a channel is a conversation: listed in its members' inboxes, and not among the channels. */
export function isConversation(channel: { conversation?: boolean } | null | undefined): boolean {
  return channel?.conversation === true;
}

/** The channels that are listed among the channels: every one but the conversations. */
export function amongChannels<C extends { conversation?: boolean }>(channels: C[]): C[] {
  return channels.filter((channel) => !isConversation(channel));
}

/**
 * Whether anything waits unread in a channel that is listed among the
 * channels. What waits in a conversation is said by the inbox's count on the rail,
 * and one the page has not read yet is counted when it is known not to
 * be a conversation.
 */
export function channelsWait(channels: { id: string; conversation?: boolean; archived?: unknown }[], unread: Record<string, number>): boolean {
  // An archived channel waits for nobody (`archived.ts`).
  return amongChannels(channels).some((channel) => !channel.archived && (unread[channel.id] ?? 0) > 0);
}

/** Somebody a conversation can be written to, as the dialog lists them. */
export interface Candidate {
  actor: string;
  name: string;
  /** Who this is, in a word or two. */
  kind: 'person' | 'agent' | 'external agent';
}

/**
 * Who a conversation can be written to: the people with an account here
 * and the agents of this factory, each saying which it is, the people
 * first and each kind by name. Never whoever writes it, never a bot,
 * and no agent that is stopped.
 */
export function candidates(people: { actor: string; name: string }[], agents: { name: string; adapter?: string; active?: boolean }[], me: string): Candidate[] {
  const byName = (one: Candidate, other: Candidate) => one.name.localeCompare(other.name);
  const persons: Candidate[] = people.filter((one) => actorKind(one.actor) === 'user' && one.actor !== me).map((one) => ({ actor: one.actor, name: one.name, kind: 'person' }));
  const others: Candidate[] = agents
    .filter((slot) => slot.active !== false && `agent:${slot.name}` !== me)
    .map((slot) => ({ actor: `agent:${slot.name}`, name: slot.name, kind: slot.adapter === 'external' ? 'external agent' : 'agent' }));
  return [...persons.sort(byName), ...others.sort(byName)];
}

/** Those of `all` that what was typed finds: by name, wherever in it, whatever the case. Everybody where nothing was typed. */
export function found(all: Candidate[], typed: string): Candidate[] {
  const sought = typed.trim().replace(/^@/, '').toLowerCase();
  return sought ? all.filter((one) => one.name.toLowerCase().includes(sought)) : all;
}

/** Whether whoever holds `scopes` is offered writing one: it takes changing one's own box and changing channels. */
export function mayWrite(scopes: string[]): boolean {
  return covers(scopes, 'inbox:write') && covers(scopes, 'channels:write');
}

/** What is asked of the daemon to write one, or nothing while it is to nobody or says nothing. */
export function written(to: string[], subject: string, text: string): { to: string[]; subject?: string; text: string } | null {
  const [about, said] = [subject.trim(), text.trim()];
  if (to.length === 0 || !said) return null;
  return about ? { to, subject: about, text: said } : { to, text: said };
}

/** Whether `by` is offered opening a conversation as a channel: somebody who is in it and may change channels. */
export function mayOpenAsChannel(channel: Conversed, by: { actor: string; scopes: string[] }): boolean {
  return isConversation(channel) && channel.members.some((member) => member.actor === by.actor) && covers(by.scopes, 'channels:write');
}

/** Who is in it beside whoever reads this, by name, as a line says it: `Mira`, `Mira and Odo`, `Mira, Odo and scout`. Only you, where nobody else is. */
export function withOf(members: { actor: string; name: string }[], me: string): string {
  const names = members.filter((member) => member.actor !== me && actorKind(member.actor) !== 'bot').map((member) => member.name);
  if (names.length === 0) return 'only you';
  if (names.length === 1) return names[0]!;
  return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]!}`;
}

/** Where a channel that a conversation becomes can belong, as the dialog offers it. */
export interface Place {
  to: { workspace: string } | { project: string };
  label: string;
  kind: 'workspace' | 'project';
}

/**
 * Where a conversation that is opened as a channel can belong: the
 * workspace showing first, then the others, then the projects, each as
 * it was given. Never nothing: a channel is a workspace's or a
 * project's.
 */
export function places(workspaces: { id: string; name: string }[], projects: { id: string; name: string }[], showing: string | null): Place[] {
  const first = workspaces.filter((one) => one.id === showing);
  const rest = workspaces.filter((one) => one.id !== showing);
  return [
    ...[...first, ...rest].map((one): Place => ({ to: { workspace: one.id }, label: one.name, kind: 'workspace' })),
    ...projects.map((one): Place => ({ to: { project: one.id }, label: one.name, kind: 'project' })),
  ];
}
