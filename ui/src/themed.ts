// The themes of whoever is signed in, as the page has them: read once the
// stream of events is there, and followed. What a theme is, and what
// anything said of one comes to, is `themes.ts`.
//
// A theme somebody makes, and which one they chose, are kept with their
// account (`people::themes`), so that both are there in any browser they
// sign in to; what is said of them (`theme.*`) carries the theme whole, so
// a page follows it with nothing read again. The daemon is asked for no
// themes but those of whoever asks.
//
// What a browser kept before themes were the account's is taken up once,
// the first time the account's are read: each theme made here that the
// account has not got is sent to it, and then taken out of the browser;
// and where the account has chosen nothing, what this browser chose is
// sent as its choice. Until the account's are read the page is in what
// the browser kept.

import { batch, effect } from '@preact/signals';
import { ApiError, call } from './api.ts';
import { stream } from './events.ts';
import { covers } from './holds.ts';
import { madeThemes, me, offeredThemes, setTheme, theme, themesRead, wearable } from './store.ts';
import { DEFAULT_THEME, SAVED_THEME, SAVED_THEMES, chosenOnce, heardTheme, keptName, ownThemes, takenUp } from './themes.ts';
import type { Colours, KeptTheme, OwnTheme, Themed } from './themes.ts';
import type { Look } from './looks.ts';

const at = (id: string) => encodeURIComponent(id);

export const themes = {
  list: () => call<Themed>('GET', '/themes'),
  make: (name: string, colours: Colours, light: Colours | null = null, look: Look = {}) => call<KeptTheme>('POST', '/themes', { name, colours, light, look }),
  change: (id: string, name: string, colours: Colours, light: Colours | null, look: Look = {}) => call<KeptTheme>('PUT', `/themes/${at(id)}`, { name, colours, light, look }),
  takeAway: (id: string) => call<unknown>('DELETE', `/themes/${at(id)}`),
  choose: (id: string | null) => call<{ chosen: string | null }>('PUT', '/themes/chosen', { theme: id }),
  offer: (id: string) => call<KeptTheme>('POST', `/themes/${at(id)}/offer`),
  withdraw: (id: string) => call<KeptTheme>('DELETE', `/themes/${at(id)}/offer`),
};

const held = () => me.value?.principal.scopes ?? [];
/** Whether whoever is signed in has themes kept with their account to read. */
export const readsThemes = () => covers(held(), 'themes:read');
/** Whether they may make, change and choose their own. */
export const writesThemes = () => covers(held(), 'themes:write');
/** Whether they may offer one of their own to everybody, and withdraw one. */
export const offersThemes = () => writesThemes() && covers(held(), 'admin');

/** What the account chose, as the page last heard it. */
let chosen: string | null = null;

function shown(had: Themed): void {
  batch(() => {
    madeThemes.value = had.own;
    offeredThemes.value = had.offered;
    chosen = had.chosen;
    themesRead.value = true;
  });
}

function keptHere(): OwnTheme[] {
  try {
    return ownThemes(localStorage.getItem(SAVED_THEMES));
  } catch {
    return [];
  }
}
function keepHere(left: OwnTheme[]): void {
  try {
    if (left.length) localStorage.setItem(SAVED_THEMES, JSON.stringify(left));
    else localStorage.removeItem(SAVED_THEMES);
  } catch {
    /* not remembered */
  }
}
function chosenHere(): string | null {
  try {
    return localStorage.getItem(SAVED_THEME);
  } catch {
    return null;
  }
}

let under = false;
let again = false;
/** Reads the account's themes, taking up what this browser kept. Asked while a reading is on its way, it is read once more when that is in. */
export async function readThemes(): Promise<void> {
  if (under) {
    again = true;
    return;
  }
  under = true;
  try {
    do {
      again = false;
      const who = me.value?.actor;
      if (!who || !readsThemes()) break;
      let read = await themes.list().catch(() => null);
      if (!read || me.value?.actor !== who) break;
      // What this browser kept of themes made before they were the account's.
      const kept = keptHere();
      const { send, already } = takenUp(kept, read.own);
      const moved: Record<string, string> = { ...already };
      const left: OwnTheme[] = [];
      if (send.length && writesThemes()) {
        for (const one of send) {
          try {
            const made = await themes.make(keptName(one), one.colours);
            moved[one.id] = made.id;
            read = { ...read, own: read.own.some((it) => it.id === made.id) ? read.own : [...read.own, made] };
          } catch (error) {
            // Refused, it is not a theme the account takes and is not kept here either; not sent, it is tried again.
            if (!(error instanceof ApiError) || error.status >= 500) left.push(one);
          }
        }
      } else left.push(...send);
      shown(read);
      const once = chosenOnce(read.chosen, chosenHere(), moved, wearable.value);
      if (once.send && writesThemes()) {
        chosen = once.theme;
        await themes.choose(once.theme).catch(() => undefined);
      }
      // What this browser kept is taken out of it once the account has what it made and what it chose of it: taken
      // out before the choice was kept, a page that read the account in that moment found the theme and no choice.
      if (kept.length) keepHere(left);
      setTheme(once.theme);
    } while (again);
  } finally {
    under = false;
  }
}

/** Chooses a theme: worn here at once, and kept as the account's where whoever is signed in may. */
export async function chooseTheme(id: string): Promise<void> {
  setTheme(id);
  if (!writesThemes() || chosen === id) return;
  chosen = id;
  await themes.choose(id).catch(() => undefined);
}

stream.on('theme.*', (event) => {
  const who = me.value?.actor;
  // A reading on its way may have been taken before this was said: it is made once more.
  if (under) again = true;
  if (!who || !themesRead.value) return;
  const had: Themed = { own: madeThemes.value, offered: offeredThemes.value, chosen };
  const next = heardTheme(had, who, event as { topic: string; data: Record<string, unknown> });
  shown(next);
  // What is worn is what the account chose, or what is worn here goes on being worn while there is such a theme.
  if (event.topic === 'theme.chosen' && next.chosen !== had.chosen) setTheme(next.chosen ?? DEFAULT_THEME);
  else setTheme(theme.value);
});
// Read once the stream is there, so that what is said while it is read is heard; and again whenever the stream is
// back, since what was said while the page could not hear was not heard.
stream.onStatus((up) => {
  if (up && me.value) void readThemes();
});
// Nobody's once nobody is signed in: the page stays in the theme it is in.
effect(() => {
  if (me.value) return;
  batch(() => {
    madeThemes.value = [];
    offeredThemes.value = [];
    themesRead.value = false;
  });
  chosen = null;
});
