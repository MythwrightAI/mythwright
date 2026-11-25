// How what whoever is signed in reads looks for them, as the page has it:
// read once the stream of events is there, and followed. What a setting is
// is `appearance.ts`.
//
// It is kept with their account (`people::appearance`), so that it is
// there in every window and browser they sign in to; what is said of it
// (`appearance.kept`) carries it whole, so another open page follows it
// with nothing read again. The page wears it at once: the reading column's
// width and text by marks on the page that `appearance.css` reads, and the
// colour through `wearing.ts`, which puts the page in its light or dark
// list. The browser keeps it too, only so that the page is first drawn in
// it (`index.html`).

import { effect, signal } from '@preact/signals';
import { call } from './api.ts';
import { appearanceOf, readFace, readSize, SAVED_APPEARANCE, savedOf, USUAL_APPEARANCE } from './appearance.ts';
import type { Appearance } from './appearance.ts';
import { stream } from './events.ts';
import { covers } from './holds.ts';
import { me, toast } from './store.ts';
import { colourChosen } from './wearing.ts';

/** What the account keeps, as the daemon hands it. */
export type KeptAppearance = { appearance: Appearance | null; nth: number; changed?: number | null };

export const appearing = {
  read: () => call<KeptAppearance>('GET', '/appearance'),
  keep: (setting: Appearance) => call<KeptAppearance>('PUT', '/appearance', setting),
};

const held = () => me.value?.principal.scopes ?? [];
/** Whether whoever is signed in has a setting kept with their account to read: it is how the client looks for them, as their themes are. */
export const readsAppearance = () => covers(held(), 'themes:read');
/** Whether they may change it there: otherwise a change is this page's alone, until it is read again. */
export const writesAppearance = () => covers(held(), 'themes:write');

function savedAppearance(): Appearance {
  try {
    return appearanceOf(JSON.parse(localStorage.getItem(SAVED_APPEARANCE) ?? 'null'));
  } catch {
    return USUAL_APPEARANCE;
  }
}

/** What the page wears: what this browser kept until the account's is read. */
export const appearance = signal<Appearance>(typeof localStorage === 'undefined' ? USUAL_APPEARANCE : savedAppearance());

/** Puts a setting on the page, and keeps it in the browser for the page's first drawing. */
function wear(worn: Appearance): void {
  const root = document.documentElement;
  if (worn.width === 'wide') root.dataset.readWidth = 'wide';
  else delete root.dataset.readWidth;
  const size = readSize(worn.text);
  if (size) {
    root.dataset.readText = '';
    root.style.setProperty('--read-size', size);
  } else {
    delete root.dataset.readText;
    root.style.removeProperty('--read-size');
  }
  const face = readFace(worn.typeface);
  if (face) {
    root.dataset.readFace = 'serif';
    root.style.setProperty('--read-face', face);
  } else {
    delete root.dataset.readFace;
    root.style.removeProperty('--read-face');
  }
  colourChosen.value = worn.colour;
  try {
    localStorage.setItem(SAVED_APPEARANCE, JSON.stringify(savedOf(worn)));
  } catch {
    /* not remembered */
  }
}
if (typeof document !== 'undefined') effect(() => wear(appearance.value));

/** How many times what the page wears was written, as it was told: the later of two is what is worn. */
let nth = 0;

function heard(told: KeptAppearance): void {
  if (told.nth < nth) return;
  nth = told.nth;
  // Nothing kept is what this browser has until something is.
  if (told.appearance) appearance.value = appearanceOf(told.appearance);
}

let reading: Promise<void> | null = null;
/** Reads the account's setting. */
export function readAppearance(): Promise<void> {
  if (reading) return reading;
  reading = (async () => {
    const who = me.value?.actor;
    if (!who || !readsAppearance()) return;
    const read = await appearing.read().catch(() => null);
    if (!read || me.value?.actor !== who) return;
    heard(read);
  })().finally(() => {
    reading = null;
  });
  return reading;
}

/** Keeps `next` as the account's, worn at once; what the daemon refuses is said in its sentence, and what was worn is worn again. */
export async function keepAppearance(next: Appearance): Promise<void> {
  const was = appearance.peek();
  appearance.value = next;
  if (!writesAppearance()) return;
  try {
    heard(await appearing.keep(next));
  } catch (error) {
    appearance.value = was;
    toast(error instanceof Error ? error.message : String(error), true);
  }
}

// What another page of the same person changed: the setting whole.
stream.on('appearance.kept', (event) => {
  const data = event.data as { owner?: string; appearance?: Appearance | null; nth?: number };
  if (!me.value || data.owner !== me.value.actor || typeof data.nth !== 'number') return;
  heard({ appearance: data.appearance ?? null, nth: data.nth });
});
// Read once the stream is there, and again whenever it is back, since what was said meanwhile was not heard.
stream.onStatus((up) => {
  if (up && me.value) void readAppearance();
});

// Somebody else signs in: what they keep is read afresh.
let signedIn: string | undefined;
effect(() => {
  const actor = me.value?.actor;
  if (actor === signedIn) return;
  signedIn = actor;
  nth = 0;
});
