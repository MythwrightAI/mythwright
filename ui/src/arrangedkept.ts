// How whoever is signed in arranged the side panel, as the page has it:
// read once the stream of events is there, and followed. What an
// arrangement is, and what it comes to on the page, is `arranged.ts`.
//
// It is kept with their account (`people::arranged`), so that their groups
// of channels, the channels they pinned and what they pinned in Resources
// are there in every window and browser they sign in to; what is said of
// it (`arranged.kept`) carries it whole, so another open page follows it
// with nothing read again. The daemon keeps it and acts on none of it.
//
// The browser keeps a copy only so that the panel is drawn at once, before
// the account's is read: never as what is so, and without what the page
// knows to be gone (`copyOf`). A change is worked out from what the account
// keeps as the change is made, one at a time (`InTurn`), and is drawn at
// once; what the daemon answers is what is drawn from then on.
//
// What a browser kept before arrangements were the account's (groups and
// pinned channels under `client.channel.groups`, what was pinned under
// `client.side.lately`) is taken up once, the first time the account's is
// read: where the account keeps none it is handed over to be kept only
// where nothing is kept yet (`/arranged/first`, so that two windows that
// do it at once have one kept), and either way it is then taken out of the
// browser; where the account keeps one already, the browser's is dropped
// and nothing is merged.

import { effect, signal } from '@preact/signals';
import { ApiError, call } from './api.ts';
import { arrangementOf, copyOf, copyRead, groupingFrom, isEmpty, isLater, NOTHING_ARRANGED, takenUp, thingsFrom } from './arranged.ts';
import type { Arrangement, KeptArrangement } from './arranged.ts';
import { groupingOf } from './channelgroups.ts';
import type { Grouping } from './channelgroups.ts';
import { stream } from './events.ts';
import { covers } from './holds.ts';
import { InTurn } from './inturn.ts';
import { keptOf } from './lately.ts';
import { channels, entered, me, toast } from './store.ts';
import type { Tab } from './tab.ts';

const COPY = 'client.side.arranged';
/** Where a browser kept groups and pinned channels before they were the account's. */
export const OLD_GROUPS = 'client.channel.groups';
/** Where it kept what was pinned in Resources beside what was lately opened, which it still keeps there. */
export const OLD_LATELY = 'client.side.lately';
const under = (key: string, actor: string) => `${key}:${actor}`;

export const arranging = {
  read: () => call<KeptArrangement>('GET', '/arranged'),
  keep: (arrangement: Arrangement) => call<KeptArrangement>('PUT', '/arranged', arrangement),
  first: (arrangement: Arrangement) => call<KeptArrangement>('POST', '/arranged/first', arrangement),
};

const held = () => me.value?.principal.scopes ?? [];
/** Whether whoever is signed in has an arrangement kept with their account to read. */
export const readsArranged = () => covers(held(), 'arranged:read');
/** Whether they may change it there: otherwise a change is this page's alone, until it is read again. */
export const writesArranged = () => covers(held(), 'arranged:write');

/** The arrangement as the page draws it. */
export const arrangement = signal<Arrangement>(NOTHING_ARRANGED);
/** Whether the account's was read since whoever is signed in signed in. */
export const arrangedRead = signal(false);
/** What is to be left out of the browser's copy of things pinned, as gone: said by whatever knows what there is (`latelykept.ts`). */
export const goneThing = signal<(thing: Tab) => boolean>(() => false);

/** What the account keeps, as far as the page was told, a change at a time. */
let kept = new InTurn<KeptArrangement>({ arrangement: null, nth: 0 });

function storage(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

/** What a browser kept before: groups and pinned channels, and what was pinned in Resources. */
function before(actor: string): { grouping: Grouping; pinned: Tab[] } {
  const saved = storage();
  const groups = saved?.getItem(under(OLD_GROUPS, actor));
  const lately = saved?.getItem(under(OLD_LATELY, actor));
  return { grouping: groupingOf(groups), pinned: keptOf(lately).pinned };
}

/** Takes out of the browser what it kept before, leaving what was lately opened where it is. */
function dropBefore(actor: string): void {
  const saved = storage();
  if (!saved) return;
  try {
    saved.removeItem(under(OLD_GROUPS, actor));
    const lately = saved.getItem(under(OLD_LATELY, actor));
    if (lately !== null) {
      const { recent } = keptOf(lately);
      saved.setItem(under(OLD_LATELY, actor), JSON.stringify({ recent }));
    }
  } catch {
    /* not taken out: taken out the next time */
  }
}

function shown(next: Arrangement): void {
  arrangement.value = next;
}

/** What the account keeps, as the page is told it: the later of what it had and this. */
function heard(told: KeptArrangement): void {
  if (!isLater(kept.now.nth, told.nth)) return;
  kept.read(told);
  shown(told.arrangement ?? NOTHING_ARRANGED);
}

let reading: Promise<void> | null = null;
let again = false;
/** Reads the account's arrangement, taking up what this browser kept before. Asked while a reading is on its way, it is read once more when that is in. */
export function readArranged(): Promise<void> {
  if (reading) {
    again = true;
    return reading;
  }
  reading = (async () => {
    do {
      again = false;
      const who = me.value?.actor;
      if (!who || !readsArranged()) break;
      const asked = kept.asked();
      let read = await arranging.read().catch(() => null);
      if (!read || me.value?.actor !== who) break;
      const was = before(who);
      const { send } = takenUp(read.arrangement, was);
      if (send && writesArranged()) {
        try {
          read = await arranging.first(send);
          dropBefore(who);
        } catch (error) {
          // Refused, it is not one the account takes, and is not kept here either; not sent, it is tried again.
          if (error instanceof ApiError && error.status < 500) dropBefore(who);
        }
      } else if (read.arrangement) dropBefore(who);
      if (!kept.read(read, asked)) again = true;
      else shown(read.arrangement ?? arrangement.peek());
      arrangedRead.value = true;
    } while (again);
  })().finally(() => {
    reading = null;
  });
  return reading;
}

/**
 * Changes the arrangement: drawn at once, worked out again from what the
 * account keeps once every change asked before it is answered, and kept
 * there where whoever is signed in may. What the daemon refuses is said in
 * its sentence, and the page draws what is kept.
 */
export function arrange(change: (now: Arrangement) => Arrangement): Promise<void> {
  shown(change(arrangement.peek()));
  if (!me.peek() || !writesArranged()) return Promise.resolve();
  return kept
    .change(async (now) => {
      if (!arrangedRead.peek()) await readArranged();
      // Never sent from what was not read: it would be kept over what the account keeps. It is this page's alone then.
      if (!arrangedRead.peek()) return null;
      const base = kept.now.arrangement ?? now.arrangement ?? NOTHING_ARRANGED;
      const next = change(base);
      if (JSON.stringify(next) === JSON.stringify(base)) return null;
      try {
        const answer = await arranging.keep(next);
        if (isLater(kept.now.nth, answer.nth) || answer.nth === kept.now.nth) shown(answer.arrangement ?? NOTHING_ARRANGED);
        return isLater(kept.now.nth, answer.nth) ? answer : null;
      } catch (error) {
        shown(kept.now.arrangement ?? NOTHING_ARRANGED);
        toast(error instanceof Error ? error.message : String(error), true);
        return null;
      }
    })
    .then(() => undefined);
}

// What another page of the same person changed: the arrangement whole.
stream.on('arranged.kept', (event) => {
  const data = event.data as { owner?: string; arrangement?: Arrangement | null; nth?: number };
  if (!me.value || data.owner !== me.value.actor || typeof data.nth !== 'number') return;
  if (reading) again = true;
  heard({ arrangement: data.arrangement ?? null, nth: data.nth });
});
// Read once the stream is there, so that what is said while it is read is heard; and again whenever the stream is
// back, since what was said while the page could not hear was not heard.
stream.onStatus((up) => {
  if (up && me.value) void readArranged();
});

// Somebody signs in: drawn at once from what this browser kept, a copy or what it kept before; nobody's once nobody is.
let signedIn: string | undefined;
effect(() => {
  const actor = me.value?.actor;
  if (actor === signedIn) return;
  signedIn = actor;
  kept = new InTurn<KeptArrangement>({ arrangement: null, nth: 0 });
  arrangedRead.value = false;
  if (!actor) {
    shown(NOTHING_ARRANGED);
    return;
  }
  const copy = copyRead(storage()?.getItem(under(COPY, actor)));
  const was = before(actor);
  shown(copy ?? arrangementOf(was.grouping, was.pinned));
});

// The copy, written as the arrangement or what the page lists changes, once the account's was read.
effect(() => {
  const [now, read, listed, gone] = [arrangement.value, arrangedRead.value, channels.value, goneThing.value];
  const actor = me.value?.actor;
  if (!actor || !read) return;
  const ids = entered.value ? new Set(listed.map((channel) => channel.id)) : null;
  const copy = copyOf(now, ids, gone);
  try {
    if (isEmpty(copy)) storage()?.removeItem(under(COPY, actor));
    else storage()?.setItem(under(COPY, actor), JSON.stringify(copy));
  } catch {
    /* not remembered */
  }
});

/** The groups and pins of channels as the page draws them. */
export const groupingNow = () => groupingFrom(arrangement.value);
/** What is pinned in Resources as the page draws it. */
export const thingsNow = () => thingsFrom(arrangement.value);
