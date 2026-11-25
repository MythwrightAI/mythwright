// Who changes a channel's details, as the page asks it.
//
// The daemon holds the rule (`talk::managed`): whoever may write in
// channels and either manages the factory (`admin`) or is one of the
// channel's managers, the people in it who made it or were named one by a
// manager since; every person in a conversation manages it. The page works
// it out the same way, held case for case to one table with the daemon's
// (`managedcases.json`), so that what is drawn to be changed is what the
// daemon lets change; what the daemon refuses all the same is said in its
// sentence where it was asked. What a channel says it is for is one line.

import { call } from './api.ts';
import { covers } from './holds.ts';
import { actorKind } from './util.ts';

/** As much of a channel as these go by. */
export interface Managed {
  conversation?: boolean;
  members: { actor: string; name?: string; manages?: boolean }[];
}

/** As much of whoever asks as these go by. */
export interface Asking {
  actor: string;
  scopes: string[];
}

/** The most letters what a channel says it is for may be, as the daemon holds it. */
export const MOST_TOPIC = 250;

/** Whether a member manages the channel: a person named one, or any person of a conversation. */
const managing = (channel: Managed, member: { actor: string; manages?: boolean }) =>
  actorKind(member.actor) === 'user' && (member.manages === true || channel.conversation === true);

/** Whether `by` manages `channel`: the one rule, as the daemon has it. */
export function manages(channel: Managed, by: Asking): boolean {
  const named = channel.members.some((member) => member.actor === by.actor && managing(channel, member));
  return covers(by.scopes, 'channels:write') && (covers(by.scopes, 'admin') || named);
}

/** The people who manage `channel`, in the order they came in. */
export function managersOf<M extends { actor: string; name?: string; manages?: boolean }>(channel: { conversation?: boolean; members: M[] }): M[] {
  return channel.members.filter((member) => managing(channel, member));
}

/** Who manages it, in a sentence of the Details part. */
export function managedSaid(channel: Managed & { members: { actor: string; name: string; manages?: boolean }[] }, me: string | null): string {
  if (channel.conversation) return 'Everybody in it manages it.';
  const names = managersOf(channel).map((one) => (one.actor === me ? 'you' : one.name));
  if (names.length === 0) return 'Nobody in it manages it: whoever manages the factory does.';
  const joined = names.length === 1 ? names[0] : `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`;
  return `Managed by ${joined}.`;
}

/** What is offered of a member's managing, to `by`: nothing, or making them one or no longer one, closed with why where it cannot be. */
export function managerLine(channel: Managed, member: { actor: string; manages?: boolean }, by: Asking): { label: string; manages: boolean; closed: string | null } | null {
  if (channel.conversation || actorKind(member.actor) !== 'user' || !manages(channel, by)) return null;
  if (!channel.members.some((one) => one.actor === member.actor)) return null;
  if (member.manages !== true) return { label: 'Make manager', manages: true, closed: null };
  const last = managersOf(channel).length <= 1;
  return { label: 'Not a manager', manages: false, closed: last ? 'The last manager stays: make somebody else one first.' : null };
}

/** What a channel would say it is for, once it is told what was typed: nothing where it says so already, and why where it cannot. */
export function describing(now: string, typed: string): { topic: string | null; changes: boolean; says: string | null } {
  const topic = typed.trim();
  if (topic.includes('\n')) return { topic: null, changes: false, says: 'It is one line.' };
  if ([...topic].length > MOST_TOPIC) return { topic: null, changes: false, says: `It is ${MOST_TOPIC} letters at most.` };
  return { topic, changes: topic !== now.trim(), says: null };
}

/** Has a channel say it is for `topic`, for everybody who reads it: what it is then. */
export function describeChannel<C>(id: string, topic: string): Promise<C> {
  return call('PATCH', `/channels/${encodeURIComponent(id)}`, { topic });
}

/** Makes `member` one of a channel's managers, or no longer one: what it is then. */
export function setManager<C>(id: string, member: string, manages: boolean): Promise<C> {
  return call('POST', `/channels/${encodeURIComponent(id)}/managers`, { member, manages });
}
