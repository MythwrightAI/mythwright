// What a theme may say of how the client looks beyond its colours: its
// letters, how big they are, how close things stand and how round corners
// are. Each is a word of a few, and one a theme does not say is as it
// usually is (`usual`, which is kept as nothing said).
//
// Each comes to one measure the style sheets go by, put on the page as
// the colours of a theme somebody made are (`lookOf`): the letters are
// `--sans`, one of the stacks of letters a system has, written out here,
// so that nothing is fetched for them; every size of text is written as
// so many pixels times `--size`; the heights of rows and controls go by
// `--size` and `--density` together (`--tall`, in `themes.css`), and the
// room within and between the things that are most of a page by
// `--density`; and every corner by `--round`. The daemon holds a look to
// the same words (`people::themes::LOOKS`), and both say the same of one
// that is not a look, held to the table of cases both read
// (`themecases.json`). These are functions of plain values, with their
// cases (`looks.test.ts`).

/** What a look may say. */
export type Setting = 'letters' | 'size' | 'density' | 'corners';

/** What a theme says of how the client looks, by setting: one it does not say is as usual. */
export type Look = Partial<Record<Setting, string>>;

/** The word for a setting that says nothing. */
export const USUAL = 'usual';

/** A word a setting may be: what it is called where it is offered, and what its measure then is. */
export interface Word {
  word: string;
  label: string;
  /** The measure's value; nothing for the usual one, which `themes.css` says. */
  value: string | null;
}

/** A setting, the measure on the page it comes to, and its words in the order they are offered. */
export interface LookSetting {
  setting: Setting;
  label: string;
  says: string;
  property: string;
  words: Word[];
}

/**
 * What every stack of letters ends with: the systems' own letters for emoji.
 * Letters chosen for their look seldom have any, and a status
 * whose ✅ and ❌ were drawn as empty boxes said nothing; with these last,
 * whatever the letters before them lack is drawn in the system's emoji.
 */
export const EMOJI = '"Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji", emoji';

/** The measures that are stacks of letters, each of which ends with `EMOJI`. */
export const LETTERED = ['--sans', '--mono'];

/**
 * A stack of letters with the systems' emoji at its end where it has none
 * there: a look a browser kept from before they were added is worn with
 * them at once (`index.html` says the same, as the page is first drawn,
 * and `wearLook` as a look is put on), so that nobody wears it again for it.
 */
export function withEmoji(stack: string): string {
  const said = stack.trim();
  return said === '' || said.endsWith(EMOJI) ? stack : `${said.replace(/,\s*$/, '')}, ${EMOJI}`;
}

/** The stacks of letters a theme may be written in: each is what a system already has, and nothing is fetched. */
export const STACKS = {
  serif: `"Iowan Old Style", "Palatino Linotype", Palatino, "URW Palladio L", P052, Georgia, Cambria, serif, ${EMOJI}`,
  humanist: `Seravek, "Gill Sans Nova", Ubuntu, Calibri, "DejaVu Sans", "Noto Sans", sans-serif, ${EMOJI}`,
  mono: `ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, "DejaVu Sans Mono", monospace, ${EMOJI}`,
};

export const LOOKS: LookSetting[] = [
  {
    setting: 'letters',
    label: 'Letters',
    says: 'what everything is written in',
    property: '--sans',
    words: [
      { word: USUAL, label: 'The system’s own', value: null },
      { word: 'serif', label: 'Serif', value: STACKS.serif },
      { word: 'humanist', label: 'Humanist', value: STACKS.humanist },
      { word: 'mono', label: 'Monospace', value: STACKS.mono },
    ],
  },
  {
    setting: 'size',
    label: 'Size',
    says: 'how big text is',
    property: '--size',
    words: [
      { word: 'smaller', label: 'Smaller', value: '0.9' },
      { word: USUAL, label: 'Balanced', value: null },
      { word: 'larger', label: 'Larger', value: '1.1' },
      { word: 'largest', label: 'Largest', value: '1.2' },
    ],
  },
  {
    setting: 'density',
    label: 'Density',
    says: 'how close things stand',
    property: '--density',
    words: [
      { word: 'compact', label: 'Compact', value: '0.8' },
      { word: USUAL, label: 'Balanced', value: null },
      { word: 'roomy', label: 'Roomy', value: '1.25' },
    ],
  },
  {
    setting: 'corners',
    label: 'Corners',
    says: 'how round they are',
    property: '--round',
    words: [
      { word: 'square', label: 'Square', value: '0' },
      { word: USUAL, label: 'Balanced', value: null },
      { word: 'round', label: 'Round', value: '1.8' },
    ],
  },
];

/** Every measure a look puts on the page, so that what one put there is taken off before the next is put on. */
export const LOOKED = LOOKS.map((one) => one.property);

const settings = () => LOOKS.map((one) => one.setting).join(', ');

/** Why this would not be a theme's look, if it would not: the daemon says the same, case for case (`themecases.json`). Nothing is a look that says nothing. */
export function unfit(look: unknown): string | null {
  if (look === null || look === undefined) return null;
  if (typeof look !== 'object' || Array.isArray(look)) return `A theme's look is a mapping of what it says of ${settings()}.`;
  const said = look as Record<string, unknown>;
  // In the order the daemon reads them, which is by name.
  for (const key of Object.keys(said).sort()) {
    const setting = LOOKS.find((one) => one.setting === key);
    if (!setting) return `\`${key}\` is nothing a theme's look says: it says ${settings()}.`;
    const word = said[key];
    if (typeof word !== 'string' || !setting.words.some((one) => one.word === word)) {
      return `A theme's ${key} is one of ${setting.words.map((one) => one.word).join(', ')}, and ${JSON.stringify(word)} is none of them.`;
    }
  }
  return null;
}

/** A look as it is kept: what it says that is not as usual, in the order the settings are asked. */
export function keptLook(look: Look | null | undefined): Look {
  const kept: Look = {};
  for (const { setting } of LOOKS) {
    const word = look?.[setting];
    if (word && word !== USUAL) kept[setting] = word;
  }
  return kept;
}

/** The word a look says of a setting, the usual one where it says none. */
export function wordOf(look: Look | null | undefined, setting: Setting): string {
  return look?.[setting] ?? USUAL;
}

/** The measures a look puts on the page: only those of what it says that is not as usual. */
export function lookOf(look: Look | null | undefined): Record<string, string> {
  const measures: Record<string, string> = {};
  for (const { setting, property, words } of LOOKS) {
    const value = words.find((one) => one.word === look?.[setting])?.value;
    if (value) measures[property] = value;
  }
  return measures;
}

/** Whether a look says anything. */
export function saysAny(look: Look | null | undefined): boolean {
  return Object.keys(keptLook(look)).length > 0;
}

/** What a look says, in a few words: nothing for one that says nothing. */
export function lookSaid(look: Look | null | undefined): string {
  return LOOKS.flatMap(({ setting, label, words }) => {
    const word = look?.[setting];
    const chosen = words.find((one) => one.word === word && word !== USUAL);
    return chosen ? [`${label}: ${chosen.label.toLowerCase()}`] : [];
  }).join(', ');
}

/**
 * The look a check of the client is told to wear, in the words it was
 * told (`size=largest density=compact`, by `LOOK` in the environment):
 * nothing where it was told none, and refused in a sentence where it was
 * told something that is not a look.
 */
export function toldLook(told: string | undefined): Look | null {
  if (!told?.trim()) return null;
  const look: Record<string, string> = {};
  for (const part of told.trim().split(/[\s,]+/)) {
    const [setting, word] = part.split('=');
    if (!setting || !word) throw new Error(`\`${part}\` is not a setting of a look and its word, as \`size=largest\` is.`);
    look[setting] = word;
  }
  const wrong = unfit(look);
  if (wrong) throw new Error(wrong);
  return look as Look;
}
