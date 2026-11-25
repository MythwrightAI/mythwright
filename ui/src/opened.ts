// Beginning with no registration: a page opened at a link that signs
// somebody in once (`mw open`), and the command that makes one.
//
// The link carries its secret after `#once=`, which a browser never sends
// anywhere: the page takes it off the address as it reads it, before
// anything else reads the address, so that it is in no history the page
// keeps, and trades it for its session once. What the daemon refuses
// (spent already, or more than a minute old) is said on the page that
// says how to be signed in.

import { signal } from '@preact/signals';
import { api, call } from './api.ts';
import type { Account } from './api.ts';

/** The secret of a link in what is after an address's `#`, where it is one. */
export function onceIn(hash: string): string | null {
  const found = /^#once=([A-Za-z0-9_]+)$/.exec(hash);
  return found?.[1] ?? null;
}

/** What opens the client signed in, for whoever reads the page with no session: the factory's own command. */
export function opensWith(cli: string, profile?: string | null): string {
  return profile ? `${cli} --profile ${profile} open` : `${cli} open`;
}

/** Why the link this page was opened at signed nobody in, in the daemon's sentence; nothing where it was not opened at one. */
export const onceRefused = signal('');

/** Trades the link the page was opened at for its session, where it was opened at one: whether it was. */
export async function spendOnce(): Promise<boolean> {
  const once = onceIn(location.hash);
  if (!once) return false;
  history.replaceState(history.state, '', `${location.pathname}${location.search}`);
  onceRefused.value = '';
  try {
    await api.link(once);
    return true;
  } catch (err) {
    onceRefused.value = err instanceof Error ? err.message : String(err);
    return false;
  }
}

/** Gives the account a password to sign in with another way, with an email where it has none. */
export function setPassword(password: string, email?: string): Promise<{ account: Account }> {
  return call('PUT', '/me/password', { password, email: email || undefined });
}

/** Calls whoever is signed in something else. */
export function rename(name: string): Promise<{ account: Account }> {
  return call('PUT', '/me/name', { name });
}
