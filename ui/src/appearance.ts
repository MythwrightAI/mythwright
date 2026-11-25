// How what somebody reads looks for them: how big its text is, how wide it
// is laid out, whether the client is light, dark or as the system asks,
// whether a long article's contents are shown beside it or hidden
// until asked for (`views/topiccontents.tsx`), and whether an article and
// a library's document are read in sans or serif letters.
// Reading sites offer these from the page itself; here they are offered
// from the head of a page that is read (`views/appearance.tsx`) and in
// Settings, and both change the one setting.
//
// It is kept with the account (`people::appearance`), so it follows its
// owner to any browser, and read and followed by `appearancekept.ts`. It
// is beside a theme's look and changes none of it: the text and the width
// are those of a page's reading column alone (the element with
// `READING`), and the colour picks a theme's light or dark list, whichever
// theme is worn. What is here is functions of plain values, with their
// cases (`appearance.test.ts`, and `appearancecases.json`, which the
// daemon is held to as well).

import { LOOKS, STACKS, USUAL } from './looks.ts';

/** How what somebody reads looks for them, each a word. */
export interface Appearance {
  text: string;
  width: string;
  colour: string;
  contents: string;
  typeface: string;
}

export type AppearanceSetting = keyof Appearance;

/** A word a setting may be, and what it is called where it is offered. */
export interface AppearanceWord {
  word: string;
  label: string;
}

/** Each thing a setting says, what it is called and its words in the order they are offered: the first said is the usual. */
export const APPEARANCE: { setting: AppearanceSetting; label: string; usual: string; words: AppearanceWord[] }[] = [
  {
    setting: 'text',
    label: 'Text',
    usual: USUAL,
    // The sizes a theme's look has, and what they are called there.
    words: LOOKS.find((one) => one.setting === 'size')!.words.map(({ word, label }) => ({ word, label })),
  },
  {
    setting: 'width',
    label: 'Width',
    usual: 'standard',
    words: [
      { word: 'standard', label: 'Standard' },
      { word: 'wide', label: 'Wide' },
    ],
  },
  {
    setting: 'colour',
    label: 'Colour',
    usual: 'automatic',
    words: [
      { word: 'automatic', label: 'Automatic' },
      { word: 'light', label: 'Light' },
      { word: 'dark', label: 'Dark' },
    ],
  },
  {
    setting: 'contents',
    label: 'Contents',
    usual: 'shown',
    words: [
      { word: 'shown', label: 'Shown' },
      { word: 'hidden', label: 'Hidden' },
    ],
  },
  {
    setting: 'typeface',
    label: 'Typeface',
    usual: 'sans',
    words: [
      { word: 'sans', label: 'Sans' },
      { word: 'serif', label: 'Serif' },
    ],
  },
];

/** As it is where nothing is said. */
export const USUAL_APPEARANCE: Appearance = { text: USUAL, width: 'standard', colour: 'automatic', contents: 'shown', typeface: 'sans' };

/** The class of the element a page is read in: its text and width are what the setting says. */
export const READING = 'reading-column';

/** Where the browser keeps it, so that the page is first drawn in it (`index.html` reads it too). */
export const SAVED_APPEARANCE = 'client.appearance';

/** A setting as it is worn: what `kept` says of each that is one of its words, and the usual for the rest. */
export function appearanceOf(kept: unknown): Appearance {
  const said = kept !== null && typeof kept === 'object' && !Array.isArray(kept) ? (kept as Record<string, unknown>) : {};
  const worn = { ...USUAL_APPEARANCE };
  for (const { setting, words } of APPEARANCE) {
    const word = said[setting];
    if (typeof word === 'string' && words.some((one) => one.word === word)) worn[setting] = word;
  }
  return worn;
}

/** Whether the page is light: as chosen, or as the system asks where it is automatic. */
export function lightWorn(colour: string, systemLight: boolean): boolean {
  return colour === 'light' || (colour !== 'dark' && systemLight);
}

/** The size the text of what is read is drawn at, as `--size` is: nothing for the usual one, which is the theme's own. */
export function readSize(text: string): string | null {
  return LOOKS.find((one) => one.setting === 'size')!.words.find((one) => one.word === text && one.word !== USUAL)?.value ?? null;
}

/** The letters an article and a library's document are read in, as `--read-face` is: nothing for sans, which is the page's own. */
export function readFace(typeface: string): string | null {
  return typeface === 'serif' ? STACKS.serif : null;
}

/**
 * What the browser keeps of a setting: the setting, which the client reads
 * back as it starts, and for the page's first drawing, which `index.html`
 * reads before any of the client is, the text's size as `--size` is, where
 * it is not the usual, and its letters as `--read-face` is, where they are
 * serif.
 */
export function savedOf(appearance: Appearance): Appearance & { size: string | null; face: string | null } {
  return { ...appearance, size: readSize(appearance.text), face: readFace(appearance.typeface) };
}

/** The setting with one thing said otherwise. */
export function changed(appearance: Appearance, setting: AppearanceSetting, word: string): Appearance {
  return appearanceOf({ ...appearance, [setting]: word });
}
