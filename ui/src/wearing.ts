// The theme the client is in, and the look that goes with it: what is put on
// the page and kept in the browser for it. It came out of `store.ts`, which
// offers it under the names it always had.

import { computed, effect, signal, untracked } from '@preact/signals';
import { appearanceOf, lightWorn, SAVED_APPEARANCE } from './appearance.ts';
import { DEFAULT_THEME, SAVED_COLOURS, SAVED_LIGHT, SAVED_LOOK, SAVED_THEME, STARTS, THEMES, listOf, themeOf, wornOf } from './themes.ts';
import { LETTERED, LOOKED, lookOf, withEmoji } from './looks.ts';
import type { Look } from './looks.ts';
import type { KeptTheme } from './themes.ts';

// The theme the client is in: the account's choice once it is read
// (`themed.ts`), put on the page, where the colours are chosen by it. The
// browser keeps the choice and the colours in use so that the page is first
// drawn in them (`index.html`), which is all it is kept for. What draws with
// the colours without the page's help (a view in its box) is told
// by `themechange`.
/** The themes whoever is signed in made, as the daemon hands them: none until they are read. */
export const madeThemes = signal<KeptTheme[]>([]);
/** Those offered to everybody that are not theirs. */
export const offeredThemes = signal<KeptTheme[]>([]);
/** Whether the account's themes have been read: until they are, the page is in what this browser kept. */
export const themesRead = signal(false);
/** Every theme somebody made that may be worn here. */
export const wearable = computed<KeptTheme[]>(() => [...madeThemes.value, ...offeredThemes.value]);
function savedTheme(): string {
  try {
    return localStorage.getItem(SAVED_THEME) ?? DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}
export const theme = signal<string>(typeof localStorage === 'undefined' ? DEFAULT_THEME : savedTheme());
// Whether the page is light: where the system asks for light, or always or never as whoever reads chose
// (`appearance.ts`). The themes that ship follow it in `themes.css` by `data-light` on the page; a theme somebody
// made with a light version wears that while it is, and is put on again when it changes.
const askedLight = typeof matchMedia === 'undefined' ? null : matchMedia('(prefers-color-scheme: light)');
/** Whether the system asks for light, followed as it changes. */
const systemLight = signal(askedLight?.matches ?? false);
function savedColour(): string {
  try {
    return appearanceOf(JSON.parse(localStorage.getItem(SAVED_APPEARANCE) ?? 'null')).colour;
  } catch {
    return 'automatic';
  }
}
/** Light, dark or as the system asks, as whoever reads chose: what the browser kept until the account's is read (`appearancekept.ts`). */
export const colourChosen = signal<string>(typeof localStorage === 'undefined' ? 'automatic' : savedColour());
/** Whether the page is light, followed as the system or the choice changes. */
export const prefersLight = computed(() => lightWorn(colourChosen.value, systemLight.value));
/** Puts the page in a theme: by its name, and for one somebody made by its list of colours as well. */
function wear(id: string): void {
  const root = document.documentElement;
  const own = wearable.value.find((one) => one.id === id);
  const list = own ? listOf(wornOf(own, prefersLight.value)) : null;
  // What a theme of somebody's own put on the page is taken off before the next is put on.
  for (const name of Object.keys(listOf(STARTS[DEFAULT_THEME]!))) root.style.removeProperty(name);
  for (const [name, value] of Object.entries(list ?? {})) root.style.setProperty(name, value);
  wearLook(own?.look);
  root.dataset.theme = id;
  try {
    localStorage.setItem(SAVED_THEME, id);
    // Read by the page before anything is drawn, so that it is never first drawn in another theme.
    if (own) localStorage.setItem(SAVED_COLOURS, JSON.stringify(listOf(own.colours)));
    else localStorage.removeItem(SAVED_COLOURS);
    if (own?.light) localStorage.setItem(SAVED_LIGHT, JSON.stringify(listOf(own.light)));
    else localStorage.removeItem(SAVED_LIGHT);
    const look = lookOf(own?.look);
    if (Object.keys(look).length > 0) localStorage.setItem(SAVED_LOOK, JSON.stringify(look));
    else localStorage.removeItem(SAVED_LOOK);
  } catch {
    /* not remembered */
  }
}
/** Puts a look on the page, in place of what one put there before: the theme's as it is worn, or one being chosen in its form. */
export function wearLook(look: Look | null | undefined): void {
  const root = document.documentElement;
  for (const name of LOOKED) root.style.removeProperty(name);
  for (const [name, value] of Object.entries(lookOf(look))) root.style.setProperty(name, LETTERED.includes(name) ? withEmoji(value) : value);
}
// A theme that ships is put on as the page is read. One somebody made is as the page put it on before anything
// was drawn, from the colours this browser kept, until the account's themes are read and say what it is.
if (typeof document !== 'undefined' && THEMES.some((one) => one.id === theme.value)) wear(theme.value);
/** Puts the page in a theme here, and nowhere else: what the account chose is said by `themed.ts`. */
export function setTheme(id: string): void {
  theme.value = themeOf(id, wearable.value);
  wear(theme.value);
  window.dispatchEvent(new Event('themechange'));
}
askedLight?.addEventListener('change', (asked) => {
  systemLight.value = asked.matches;
});
// A theme that ships follows by its stylesheet alone, from `data-light`; one somebody made is its colours, put on
// again. What draws in the colours without the page's help is told as it is when a theme is chosen.
let wasLight: boolean | null = null;
if (typeof document !== 'undefined') {
  effect(() => {
    const light = prefersLight.value;
    if (light === wasLight) return;
    const first = wasLight === null;
    wasLight = light;
    if (light) document.documentElement.dataset.light = '';
    else delete document.documentElement.dataset.light;
    if (first) return;
    untracked(() => (wearable.value.some((one) => one.id === theme.value) ? setTheme(theme.value) : window.dispatchEvent(new Event('themechange'))));
  });
}
