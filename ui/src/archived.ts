// An archived channel, as the page says it.
//
// Archived, a channel is read and searched by whoever read it, and
// nobody says anything there until it is given back: the daemon holds
// that (`talk::archived`), and refuses what is said there in a sentence
// of its own. What is here is what the page works out from the channel
// as it was read: whether it is archived, who is offered archiving and
// giving it back, what the side panel lists under its Archived part,
// and what is counted as waiting unread. Nothing of the page is in it.
//
// Archiving changes nothing that somebody arranged for themselves: a
// group, a pin and a fold are kept in the browser as they were, and an
// archived channel is listed still (`/api/channels` answers it), so it
// is never taken for one that is gone; it is drawn in the Archived part
// instead, and given back it stands where it stood.

import { call } from './api.ts';
import { covers } from './holds.ts';

/** When a channel was archived and by whom, as the daemon says it. */
export interface Archived {
  at: number;
  by: string;
  name: string;
}

/** As much of a channel as these go by. */
interface Of {
  id: string;
  kept?: boolean;
  archived?: Archived | null;
  members: { actor: string }[];
}

/** Whether a channel is archived. */
export function isArchived(channel: { archived?: Archived | null } | null | undefined): boolean {
  return Boolean(channel?.archived);
}

/** The channels that are not archived, as they were given: those listed where they always were. */
export function live<C extends { archived?: Archived | null }>(channels: C[]): C[] {
  return channels.filter((channel) => !isArchived(channel));
}

/** The archived ones, by when each was archived, the latest first: those listed under the Archived part. */
export function archivedOnly<C extends { archived?: Archived | null }>(channels: C[]): C[] {
  return channels.filter(isArchived).sort((one, other) => (other.archived?.at ?? 0) - (one.archived?.at ?? 0));
}

/**
 * Whether `by` is offered archiving a channel, or giving one back: whoever
 * may write in it, which of one kept to its members is somebody in it or
 * whoever holds what reads every kept one, as its door says.
 */
export function mayArchive(channel: Of, by: { actor: string; scopes: string[] }): boolean {
  if (!covers(by.scopes, 'channels:write')) return false;
  return !channel.kept || channel.members.some((member) => member.actor === by.actor) || covers(by.scopes, 'private');
}

/** How many wait unread in a channel, as a count goes by it: none in an archived one, which waits for nobody. */
export function waiting(channel: { id: string; archived?: Archived | null }, unread: Record<string, number>): number {
  return isArchived(channel) ? 0 : (unread[channel.id] ?? 0);
}

/** What the page says of an archived channel at its head and where something is written: by whom, and that nothing is said there. */
export function archivedSays(channel: { archived?: Archived | null }): string {
  const by = channel.archived?.name;
  return `This channel is archived${by ? ` by ${by}` : ''}: it is read as before, and nothing is said here until it is given back.`;
}

/** Archives a channel, for everybody who reads it: what it is then. */
export function archiveChannel<C>(id: string): Promise<C> {
  return call('POST', `/channels/${encodeURIComponent(id)}/archive`, {});
}

/** Gives an archived channel back, as it was: what it is then. */
export function unarchiveChannel<C>(id: string): Promise<C> {
  return call('POST', `/channels/${encodeURIComponent(id)}/unarchive`, {});
}
