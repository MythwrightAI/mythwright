// A channel called something else, as the page asks it.
//
// The daemon holds what renaming is (`talk::renamed`): an open channel
// is called its name, which is made of the words as any name is and is
// one channel's, and one kept to its members, a conversation among them,
// is called its title, its name being its own id. What it refuses (a
// name another has, an archived channel, something running that finds it
// by its name, an agent asked something there) is said in its sentence
// where it was asked. What is here is what the page works out from the
// channel as it was read and what is typed: who is offered it, and what
// the channel would then be called. Nothing of the page is in it.

import { call } from './api.ts';
import { mayArchive } from './archived.ts';
import { manages } from './managed.ts';
import type { Archived } from './archived.ts';

/** As much of a channel as these go by. */
interface Of {
  id: string;
  name: string;
  kept?: boolean;
  title?: string | null;
  archived?: Archived | null;
  conversation?: boolean;
  members: { actor: string; manages?: boolean }[];
}

/** The most a title is, in letters, as the daemon holds it. */
export const MOST = 200;

/**
 * Whether `by` is offered calling a channel something else: whoever
 * manages it (`managed.ts`), of those who may write in it as archiving it
 * asks, and of one that is archived nobody, since it is given back first.
 */
export function mayRename(channel: Of, by: { actor: string; scopes: string[] }): boolean {
  return !channel.archived && mayArchive(channel, by) && manages(channel, by);
}

/** A name as the daemon makes one of what is typed: its letters and digits, lowercase, the rest one dash between them. */
export function nameOf(typed: string): string {
  let made = '';
  for (const letter of typed) {
    if (/^[A-Za-z0-9]$/.test(letter)) made += letter.toLowerCase();
    else if (made && !made.endsWith('-')) made += '-';
  }
  return made.replace(/-+$/, '');
}

/** What a channel would be called once it is renamed to what was typed, and what is said of that under the field. */
export interface Renaming {
  /** What it is called then: its name, or its title; nothing where what was typed comes to none. */
  called: string | null;
  /** Whether that is other than what it is called now, which is when there is anything to save. */
  changes: boolean;
  /** What is said under the field, where anything is: what the name comes to, or why there is none. */
  says: string | null;
}

/** What renaming `channel` to `typed` comes to, as the daemon would take it. */
export function renaming(channel: Of, typed: string): Renaming {
  if (channel.kept) {
    const title = typed.trim();
    if (!title) return { called: null, changes: false, says: 'It is called something.' };
    if ([...title].length > MOST) return { called: null, changes: false, says: `It is called something in ${MOST} letters at most.` };
    return { called: title, changes: title !== (channel.title ?? ''), says: null };
  }
  const name = nameOf(typed);
  if (!name) return { called: null, changes: false, says: 'A name has at least one letter or digit.' };
  // Said where what was typed is not the name it comes to, so that nobody is surprised by the dashes.
  const says = name === typed.trim().replace(/^#/, '') ? null : `It will be called #${name}.`;
  return { called: name, changes: name !== channel.name, says };
}

/** Calls a channel what `name` says, for everybody who reads it: what it is then. */
export function renameChannel<C>(id: string, name: string): Promise<C> {
  return call('PATCH', `/channels/${encodeURIComponent(id)}`, { name });
}
