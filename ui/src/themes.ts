// The themes the client can be in.
//
// A theme that ships is a list of colours in `themes.css`, under its name;
// this is the names, for choosing among them. The one chosen is put on the
// page as `data-theme`, which is all that choosing does.
//
// A theme somebody makes is a few colours of their choosing, and its list
// is worked out from them and put on the page itself. It may say a light
// version too, a few colours more, which is worn where the system asks
// for light, as the light lists of the ones that ship are; one that says
// none is worn the same in both. The light version begins as worked out
// from the dark one (`lightFrom`) until somebody changes it. It is kept with
// their account by the daemon (`people::themes`), with which theme they
// chose, so that both follow them to any browser; and one an admin offers
// is there for everybody. What reads and follows them is `themed.ts`; what
// is here is functions of plain values, with their cases. The browser
// keeps the choice and the colours in use only so that the page is drawn
// in them before anything is read, and what it kept of made themes before
// they were the account's is taken up once (`takenUp`). A theme somebody
// makes may say how the client looks beyond its colours too, its letters,
// their size, how close things stand and how round corners are: that is
// `looks.ts`.

import type { Look } from './looks.ts';

export interface Theme {
  /** What it is called in `themes.css` and where the choice is kept. */
  id: string;
  name: string;
  says: string;
}

export const THEMES: Theme[] = [
  { id: 'grid-iron', name: 'Grid Iron', says: 'Greys and blacks, with orange for what is live.' },
  { id: 'rose-pine', name: 'Rosé Pine', says: 'Muted plum, with rose and gold.' },
  { id: 'tokyo-midnight', name: 'Tokyo Midnight', says: 'Blue-black, with cool blue and violet.' },
];

/** The one a client is in until another is chosen. */
export const DEFAULT_THEME = 'grid-iron';

/** Where the choice is kept. `index.html` reads it too, before anything is drawn. */
export const SAVED_THEME = 'client.theme';

/** The theme that was kept, or the default where none was or it is not one there is any more. */
export function themeOf(saved: string | null | undefined, own: { id: string }[] = []): string {
  return THEMES.some((theme) => theme.id === saved) || own.some((theme) => theme.id === saved) ? saved! : DEFAULT_THEME;
}

/**
 * The colours somebody chooses for a theme of their own. The rest of a
 * theme's list (its darker and lighter grounds, its lines, its quieter
 * text, what is written on its accent) is worked out from these, so that
 * eight choices make a whole theme and none of them can leave it unreadable
 * by being forgotten.
 */
export interface Colours {
  ground: string;
  text: string;
  accent: string;
  ok: string;
  bad: string;
  info: string;
  agent: string;
  card: string;
}

/** What each of them is for, in the order they are asked. */
export const COLOURS: [keyof Colours, string, string][] = [
  ['ground', 'Ground', 'what everything is drawn on'],
  ['text', 'Text', 'what is written'],
  ['accent', 'Accent', 'what is live, chosen or to be pressed'],
  ['ok', 'Good', 'what went well'],
  ['bad', 'Bad', 'what failed'],
  ['info', 'Note', 'what is only said'],
  ['agent', 'Agent', 'an agent, wherever one is named'],
  ['card', 'Work', 'a piece of work, and an agent that is idle'],
];

/** A theme a browser kept before themes were kept with an account, as it kept it. */
export interface OwnTheme extends Theme {
  colours: Colours;
}

/** A theme somebody made, as the daemon hands it: theirs, or one offered to everybody. */
export interface KeptTheme {
  id: string;
  /** Whose it is, as an actor. */
  owner: string;
  owner_name: string;
  name: string;
  colours: Colours;
  /** Its light version, worn where the system asks for light: nothing for one that is the same in both. */
  light: Colours | null;
  /** What it says of how the client looks beyond its colours (`looks.ts`), in both versions: nothing said is as usual. A daemon from before says none. */
  look?: Look;
  made: number;
  changed: number;
  /** Who offered it to everybody and when, while it is offered. */
  offered: { by: string; name: string; at: number } | null;
}

/** What somebody is handed of themes: their own, those offered that are not theirs, and the one they chose. */
export interface Themed {
  own: KeptTheme[];
  offered: KeptTheme[];
  chosen: string | null;
}

/** The colours the built-in themes are made of, for one to start from. */
export const STARTS: Record<string, Colours> = {
  'grid-iron': { ground: '#0c0c0c', text: '#e9e9e9', accent: '#dc7b32', ok: '#58c08a', bad: '#f0585c', info: '#6aaaf0', agent: '#8fa8c4', card: '#f2c230' },
  'rose-pine': { ground: '#191724', text: '#e0def4', accent: '#ebbcba', ok: '#9ccfd8', bad: '#eb6f92', info: '#c4a7e7', agent: '#31748f', card: '#f6c177' },
  'tokyo-midnight': { ground: '#16161e', text: '#c0caf5', accent: '#7aa2f7', ok: '#9ece6a', bad: '#f7768e', info: '#7dcfff', agent: '#bb9af7', card: '#e0af68' },
};

/** The colours the light lists of the built-in themes are made of, as `themes.css` writes them: a test holds the two together. */
export const LIGHTS: Record<string, Colours> = {
  'grid-iron': { ground: '#f1f1f1', text: '#161616', accent: '#c4540a', ok: '#1f8f5f', bad: '#d0343a', info: '#1f6fd0', agent: '#4a637d', card: '#e0a800' },
  'rose-pine': { ground: '#faf4ed', text: '#575279', accent: '#b4637a', ok: '#286983', bad: '#a8324a', info: '#56949f', agent: '#907aa9', card: '#ea9d34' },
  'tokyo-midnight': { ground: '#e1e2e7', text: '#343b58', accent: '#2e7de9', ok: '#587539', bad: '#d6365f', info: '#007197', agent: '#7847bd', card: '#c98f00' },
};

/** Where a browser kept the themes somebody made before they were kept with the account, and the colours of the one in use, dark and light: `index.html` reads the last two, before anything is drawn. */
export const SAVED_THEMES = 'client.themes';
export const SAVED_COLOURS = 'client.theme.colours';
export const SAVED_LIGHT = 'client.theme.light';
/** Where the measures of the look of the theme in use are kept, which `index.html` puts on with its colours. */
export const SAVED_LOOK = 'client.theme.look';

const HEX = /^#[0-9a-f]{6}$/i;
const parts = (hex: string): [number, number, number] => [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16)) as [number, number, number];
const hex = (rgb: number[]) => `#${rgb.map((part) => Math.round(Math.min(255, Math.max(0, part))).toString(16).padStart(2, '0')).join('')}`;
/** One colour moved towards another: by nothing it is itself, by 1 the other. */
export function mix(from: string, to: string, by: number): string {
  const [a, b] = [parts(from), parts(to)];
  return hex(a.map((part, at) => part + (b[at]! - part) * by));
}
/** How light a colour is to the eye, from 0 to 1. */
export function lightness(colour: string): number {
  const [r, g, b] = parts(colour).map((part) => {
    const share = part / 255;
    return share <= 0.03928 ? share / 12.92 : ((share + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}
/** How far apart two colours are to the eye, as the text and its ground are held: 1 for the same, 21 for black and white. */
export function contrast(one: string, other: string): number {
  const [low, high] = [lightness(one), lightness(other)].sort((a, b) => a - b);
  return (high! + 0.05) / (low! + 0.05);
}
/** A colour as its hue, its saturation and its lightness, each from 0 to 1. */
function hslOf(colour: string): [number, number, number] {
  const [r, g, b] = parts(colour).map((part) => part / 255) as [number, number, number];
  const [high, low] = [Math.max(r, g, b), Math.min(r, g, b)];
  const light = (high + low) / 2;
  if (high === low) return [0, 0, light];
  const span = high - low;
  const saturation = light > 0.5 ? span / (2 - high - low) : span / (high + low);
  const hue = high === r ? (g - b) / span + (g < b ? 6 : 0) : high === g ? (b - r) / span + 2 : (r - g) / span + 4;
  return [hue / 6, saturation, light];
}
function fromHsl([hue, saturation, light]: [number, number, number]): string {
  if (saturation === 0) return hex([light * 255, light * 255, light * 255]);
  const q = light < 0.5 ? light * (1 + saturation) : light + saturation - light * saturation;
  const p = 2 * light - q;
  const channel = (t: number) => {
    const at = t < 0 ? t + 1 : t > 1 ? t - 1 : t;
    if (at < 1 / 6) return p + (q - p) * 6 * at;
    if (at < 1 / 2) return q;
    if (at < 2 / 3) return p + (q - p) * (2 / 3 - at) * 6;
    return p;
  };
  return hex([channel(hue + 1 / 3), channel(hue), channel(hue - 1 / 3)].map((part) => part * 255));
}
/** A colour as light as it was dark, of the same hue. */
const turned = (colour: string) => {
  const [hue, saturation, light] = hslOf(colour);
  return fromHsl([hue, saturation, 1 - light]);
};
/** One colour moved towards `to` a tenth at a time until it is at least `least` apart from `from`, or is `to`. */
function apart(colour: string, from: string, to: string, least: number): string {
  for (let by = 0; by <= 1.001; by += 0.1) {
    const moved = mix(colour, to, Math.min(by, 1));
    if (contrast(moved, from) >= least) return moved;
  }
  return to;
}
/** The most the text of a light version worked out is held apart from its ground, which `unreadable` asks 4.5 of. */
const TEXT_APART = 7;
/** How far apart from the ground what is drawn in a colour of its own is held, which is what a thing that is not text asks. */
const MARK_APART = 3;

/**
 * The light version of a theme as it begins, worked out from its dark one:
 * the ground and the text exchanged in lightness, each of its own hue, the
 * text taken further from the ground where that leaves it hard to read,
 * and each other colour of its own hue too, taken towards the text's side
 * only as far as it has to be to stand out from the new ground. A theme
 * whose colours are light comes to a dark one by the same rule.
 */
export function lightFrom(dark: Colours): Colours {
  const ground = turned(dark.ground);
  const away = lightness(ground) > 0.18 ? '#000000' : '#ffffff';
  const text = apart(turned(dark.text), ground, away, Math.min(TEXT_APART, Math.max(4.5, contrast(dark.text, dark.ground))));
  const marked = (colour: string) => apart(colour, ground, away, MARK_APART);
  return {
    ground,
    text,
    accent: marked(dark.accent),
    ok: marked(dark.ok),
    bad: marked(dark.bad),
    info: marked(dark.info),
    agent: marked(dark.agent),
    card: marked(dark.card),
  };
}

/** The colours of a theme somebody made that are worn, where the system asks for light or does not. */
export function wornOf(theme: { colours: Colours; light?: Colours | null }, light: boolean): Colours {
  return light && theme.light ? theme.light : theme.colours;
}

const soft = (colour: string, by: number) => `rgba(${parts(colour).join(', ')}, ${by})`;

/** A theme's whole list, as `themes.css` writes one, from the colours somebody chose. */
export function listOf(colours: Colours): Record<string, string> {
  const { ground, text, accent } = colours;
  return {
    '--bg0': ground,
    '--bg1': mix(ground, text, 0.04),
    '--bg2': mix(ground, text, 0.075),
    '--bg3': mix(ground, text, 0.13),
    '--line': mix(ground, text, 0.11),
    '--line-strong': mix(ground, text, 0.21),
    '--text': text,
    '--dim': mix(text, ground, 0.3),
    '--faint': mix(text, ground, 0.55),
    '--accent': accent,
    // What is written on the accent is read against it: dark on a light one, light on a dark one.
    '--accent-ink': lightness(accent) > 0.3 ? mix(accent, '#000000', 0.88) : mix(accent, '#ffffff', 0.92),
    '--ok': colours.ok,
    '--bad': colours.bad,
    '--info': colours.info,
    '--agent': colours.agent,
    '--card': colours.card,
    '--accent-soft': soft(accent, 0.13),
    '--ok-soft': soft(colours.ok, 0.14),
    '--bad-soft': soft(colours.bad, 0.14),
    '--info-soft': soft(colours.info, 0.14),
    'color-scheme': lightness(ground) < 0.35 ? 'dark' : 'light',
  };
}

/** Why these colours would not make a theme that can be read, if they would not: the daemon says the same, case for case (`themecases.json`). */
export function unreadable(colours: Colours): string | null {
  if (typeof colours !== 'object' || colours === null || Array.isArray(colours)) return `A theme's colours are a mapping of eight: ${COLOURS.map(([name]) => name).join(', ')}.`;
  for (const [name] of COLOURS) if (typeof colours[name] !== 'string' || !HEX.test(colours[name])) return `${name} is not a colour`;
  return contrast(colours.ground, colours.text) < 4.5 ? 'The text is too near the ground to be read on it: make one lighter or the other darker.' : null;
}

/** What is said where a theme's light version is not one: the sentence for a dark one, after the words that say which. */
export const LIGHT = 'In the light version: ';
/** Why a theme and its light version, where it says one, would not be read: the dark one's first, then the light one's on its own. The daemon says the same (`themecases.json`). */
export function unreadableEither(colours: Colours, light: Colours | null | undefined): string | null {
  const wrong = unreadable(colours);
  if (wrong || light === null || light === undefined) return wrong;
  const dim = unreadable(light);
  return dim ? `${LIGHT}${dim}` : null;
}

/** The themes a browser kept before they were the account's, leaving out anything that is not one. */
export function ownThemes(kept: string | null | undefined): OwnTheme[] {
  try {
    const read: unknown = JSON.parse(kept ?? '[]');
    if (!Array.isArray(read)) return [];
    return read.filter((one): one is OwnTheme => {
      const theme = one as Partial<OwnTheme> | null;
      return typeof theme?.id === 'string' && theme.id.startsWith('own-') && typeof theme.name === 'string' && typeof theme.colours === 'object' && theme.colours !== null && unreadable(theme.colours as Colours) === null;
    });
  } catch {
    return [];
  }
}

/** The most letters a theme's name has, as the daemon holds it. */
export const LONGEST = 60;
/** What a theme a browser kept is called as the account keeps it: trimmed, and no longer than a name may be. */
export function keptName(kept: OwnTheme): string {
  return [...kept.name.trim()].slice(0, LONGEST).join('').trim();
}
/** A theme a browser kept is one of the account's where both say the same name and the same colours, and the account's says no light version, as no browser kept one. */
function same(kept: OwnTheme, theirs: KeptTheme): boolean {
  return keptName(kept) === theirs.name && !theirs.light && Object.keys(theirs.look ?? {}).length === 0 && COLOURS.every(([name]) => kept.colours[name].toLowerCase() === theirs.colours[name]?.toLowerCase());
}

/**
 * What a browser kept of themes made before they were kept with the account,
 * against what the account has: those to be sent to the account, and for each
 * of the rest the theme of the account's it is, by the id it was kept by here.
 */
export function takenUp(kept: OwnTheme[], own: KeptTheme[]): { send: OwnTheme[]; already: Record<string, string> } {
  const send: OwnTheme[] = [];
  const already: Record<string, string> = {};
  for (const one of kept) {
    const theirs = own.find((it) => same(one, it));
    if (theirs) already[one.id] = theirs.id;
    else send.push(one);
  }
  return { send, already };
}

/**
 * Which theme to be in once the account's are read: what the account chose; else,
 * once, what this browser chose (by the id it is now kept by), where that is a
 * theme there is and not the usual one; else the usual one. `send` says whether the browser's is to be
 * kept as the account's.
 */
export function chosenOnce(account: string | null, browser: string | null, moved: Record<string, string>, wearable: { id: string }[]): { theme: string; send: boolean } {
  if (account !== null) return { theme: themeOf(account, wearable), send: false };
  const mine = browser === null ? null : (moved[browser] ?? browser);
  // The usual one is what every browser is in until somebody chooses: it says nothing of a choice.
  if (mine !== null && mine !== DEFAULT_THEME && themeOf(mine, wearable) === mine) return { theme: mine, send: true };
  return { theme: DEFAULT_THEME, send: false };
}

/** One theme in place of the one of its id, or added after the rest. */
function put(list: KeptTheme[], theme: KeptTheme): KeptTheme[] {
  return list.some((one) => one.id === theme.id) ? list.map((one) => (one.id === theme.id ? theme : one)) : [...list, theme];
}

/** What the page has of themes, and what an event said of them comes to for `me`, an actor: nothing for what is not of themes. */
export function heardTheme(had: Themed, me: string, event: { topic: string; data: Record<string, unknown> }): Themed {
  const data = event.data;
  const theme = data['theme'] as KeptTheme | string | null;
  const id = typeof theme === 'string' ? theme : theme?.id;
  const offeredOnly = (list: KeptTheme[], one: KeptTheme) => (one.offered && one.owner !== me ? put(list, one) : list.filter((it) => it.id !== one.id));
  switch (event.topic) {
    case 'theme.kept':
    case 'theme.offered': {
      if (typeof theme !== 'object' || theme === null) return had;
      const own = theme.owner === me ? put(had.own, theme) : had.own;
      return { ...had, own, offered: offeredOnly(had.offered, theme) };
    }
    case 'theme.gone':
      return { ...had, own: had.own.filter((one) => one.id !== id), offered: had.offered.filter((one) => one.id !== id), chosen: had.chosen === id ? null : had.chosen };
    case 'theme.withdrawn': {
      const own = had.own.map((one) => (one.id === id ? { ...one, offered: null } : one));
      const mine = own.some((one) => one.id === id);
      return { ...had, own, offered: had.offered.filter((one) => one.id !== id), chosen: had.chosen === id && !mine ? null : had.chosen };
    }
    case 'theme.chosen':
      return data['owner'] === me ? { ...had, chosen: (theme as string | null) ?? null } : had;
    default:
      return had;
  }
}
