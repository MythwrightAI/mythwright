// How a time is written for whoever is signed in, as the page has it: read
// once the stream of events is there, and followed. What a setting is, and
// how a time is written by it, is `clock.ts`.
//
// It is kept with their account (`people::clocked`), so that the zone and
// the clock of twelve hours or twenty-four are there in every window and
// browser they sign in to; what is said of it (`clock.kept`) carries it
// whole, so another open page follows it with nothing read again, and
// every time drawn there is drawn again in it (`worn`, which `clock` and
// `when` of `util.ts` read).
//
// The first time it is read with nothing kept, the zone this browser says
// it is in is taken up as the account's (`/clock/first`, so that two
// windows that do it at once have one kept), on a clock of twelve hours.
// A browser on another system may say another zone than the system
// somebody looks at (a window of a program run under another system, as
// the desktop app may be): Settings is where it is put right.

import { effect } from '@preact/signals';
import { call } from './api.ts';
import { browserZone, settingOf, USUAL_HOURS, worn, zoneKnown } from './clock.ts';
import type { Setting } from './clock.ts';
import { stream } from './events.ts';
import { covers } from './holds.ts';
import { me, toast } from './store.ts';

/** What the account keeps, as the daemon hands it. */
export type KeptClock = { clock: Setting | null; nth: number; changed?: number | null };

export const clocking = {
  read: () => call<KeptClock>('GET', '/clock'),
  keep: (setting: Setting) => call<KeptClock>('PUT', '/clock', setting),
  first: (setting: Setting) => call<KeptClock>('POST', '/clock/first', setting),
};

const held = () => me.value?.principal.scopes ?? [];
/** Whether whoever is signed in has a setting kept with their account to read. */
export const readsClock = () => covers(held(), 'clock:read');
/** Whether they may change it there: otherwise a change is this page's alone, until it is read again. */
export const writesClock = () => covers(held(), 'clock:write');

/** How many times what the page wears was written, as it was told: the later of two is what is worn. */
let nth = 0;

function heard(told: KeptClock): void {
  if (told.nth < nth) return;
  nth = told.nth;
  const setting = settingOf(told.clock);
  // One the browser cannot write a time in is not worn: its own zone is, until it is put right.
  worn.value = setting ?? { zone: browserZone(), hours: told.clock?.hours === '24h' ? '24h' : USUAL_HOURS };
}

let reading: Promise<void> | null = null;
/** Reads the account's setting, taking up the browser's zone where nothing is kept. */
export function readClock(): Promise<void> {
  if (reading) return reading;
  reading = (async () => {
    const who = me.value?.actor;
    if (!who || !readsClock()) return;
    let read = await clocking.read().catch(() => null);
    if (!read || me.value?.actor !== who) return;
    const zone = browserZone();
    if (!read.clock && writesClock() && zoneKnown(zone)) {
      read = (await clocking.first({ zone, hours: USUAL_HOURS }).catch(() => null)) ?? read;
    }
    heard(read);
  })().finally(() => {
    reading = null;
  });
  return reading;
}

/** Keeps `setting` as the account's, worn at once; what the daemon refuses is said in its sentence, and what is kept is worn again. */
export async function keepClock(setting: Setting): Promise<void> {
  const was = worn.peek();
  worn.value = setting;
  if (!writesClock()) return;
  try {
    heard(await clocking.keep(setting));
  } catch (error) {
    worn.value = was;
    toast(error instanceof Error ? error.message : String(error), true);
  }
}

// What another page of the same person changed: the setting whole.
stream.on('clock.kept', (event) => {
  const data = event.data as { owner?: string; clock?: Setting | null; nth?: number };
  if (!me.value || data.owner !== me.value.actor || typeof data.nth !== 'number') return;
  heard({ clock: data.clock ?? null, nth: data.nth });
});
// Read once the stream is there, and again whenever it is back, since what was said meanwhile was not heard.
stream.onStatus((up) => {
  if (up && me.value) void readClock();
});

// Somebody signs in, or out: the browser's zone until theirs is read.
let signedIn: string | undefined;
effect(() => {
  const actor = me.value?.actor;
  if (actor === signedIn) return;
  signedIn = actor;
  nth = 0;
  worn.value = { zone: browserZone(), hours: USUAL_HOURS };
});
