// A channel kept to its members, as the page says it.
//
// An open channel is called its name, which is one of the names channels
// share and is written after a `#`. One kept to its members is called
// what it was made as (`title`), which another may be called too, and
// its name is its own id: nothing here shows that. Who is offered what
// of its door is worked out here, from the channel as it was read and
// who is signed in, so that a line is drawn only where pressing it does
// something; the daemon is what holds each.

import { covers } from './holds.ts';
import { actorKind } from './util.ts';

/** As much of a channel as these go by. */
export interface Called {
  id: string;
  name: string;
  kept?: boolean;
  title?: string;
  members: { actor: string; name: string }[];
}

/** As much of who is signed in as these go by. */
export interface Asks {
  actor: string;
  scopes: string[];
  /** Whether theirs is the account that owns the factory. */
  owner: boolean;
}

/** What a channel is called: its title where it is kept, else its name. */
export function calledOf(channel: Pick<Called, 'name' | 'kept' | 'title'>): string {
  return channel.kept && channel.title ? channel.title : channel.name;
}

/** The same in a sentence: an open one after its `#`, a kept one as it is called. */
export function hashed(channel: Pick<Called, 'name' | 'kept' | 'title'>): string {
  return channel.kept ? calledOf(channel) : `#${channel.name}`;
}

const within = (channel: Called, actor: string) => channel.members.some((member) => member.actor === actor);

/** Whether `by` is offered making an open channel private: somebody who is in it and may change channels. */
export function mayKeep(channel: Called, by: Asks): boolean {
  return !channel.kept && within(channel, by.actor) && covers(by.scopes, 'channels:write');
}

/** Whether `by` is offered opening a private channel again: the factory's owner, and nobody else. */
export function mayOpen(channel: Called, by: Asks): boolean {
  return channel.kept === true && by.owner && covers(by.scopes, 'admin') && covers(by.scopes, 'channels:write');
}

/** Whether `by` is the last person in a private channel, who does not leave it. */
export function lastPerson(channel: Called, by: Asks): boolean {
  return channel.kept === true && !channel.members.some((member) => member.actor !== by.actor && actorKind(member.actor) === 'user');
}

/** Whether `by` is offered leaving: somebody who is in it and may change channels, but for the last person in a private one. */
export function mayLeave(channel: Called, by: Asks): boolean {
  return within(channel, by.actor) && covers(by.scopes, 'channels:write') && !lastPerson(channel, by);
}

/** Whether `by` is offered adding to it and taking an agent out: of a private one, somebody who is in it. */
export function mayAdd(channel: Called, by: Asks): boolean {
  return covers(by.scopes, 'channels:write') && (!channel.kept || within(channel, by.actor) || covers(by.scopes, 'private'));
}

/** The channels that were listed and are listed no longer, by id: taken away, kept from whoever reads this, or left. */
export function gone(before: { id: string }[], after: { id: string }[]): string[] {
  const still = new Set(after.map((one) => one.id));
  return before.map((one) => one.id).filter((id) => !still.has(id));
}
