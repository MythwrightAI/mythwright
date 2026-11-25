// The people with an account here, by name, for what names a person by
// their actor (whom a card is assigned to). Read once, when something first
// asks, and again when an account is made or somebody else signs in; a
// reader who may not list people (`channels:read`) is told nobody, and what
// names a person then says so without a name, never by an account's id.

import { effect, signal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import { api } from './api.ts';
import type { Member } from './api.ts';
import { stream } from './events.ts';
import { me } from './store.ts';

/** Each person's actor (`user:<id>`) and name, as the daemon last said them. */
export const peopleKnown = signal<Member[]>([]);

let begun = false;
let asked = 0;

function read(): void {
  const mine = (asked += 1);
  api
    .people()
    .then((people) => mine === asked && (peopleKnown.value = people))
    .catch(() => mine === asked && (peopleKnown.value = []));
}

function begin(): void {
  if (begun) return;
  begun = true;
  // Whoever is signed in is who the list is read as: somebody else signing in has it read again, as them.
  effect(() => {
    const actor = me.value?.actor;
    asked += 1;
    peopleKnown.value = [];
    if (actor) read();
  });
  stream.on('identity.registered', () => me.value && read());
}

/** The people with an account here, read the first time something asks. */
export function usePeople(): Member[] {
  useEffect(begin, []);
  return peopleKnown.value;
}
