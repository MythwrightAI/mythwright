// A theme as a small file, saved from the browser and read back in.
//
// The file is JSON: the version of what a theme's file says, what the
// theme is called, its eight colours and, where it has one, its light
// version's eight (`light`, which a file without one leaves out), and
// what it says of how the client looks (`look`, which a file of a theme
// that says nothing of it leaves out). A file of the version before
// (1) says no look, and is read as one that says nothing of it. It is
// made by the browser alone, from what the page has, and nothing is asked
// of the daemon to make it; a theme that ships is saved from the colours
// it is made of here (`STARTS`, `LIGHTS`). What is read is checked here
// first, by the rule the page draws by, and is then sent as a new theme
// of the reader's own through the route that makes one, which checks it
// again and says its own sentence where it refuses. These are functions
// of plain values, with their cases (`themefile.test.ts`).

import { COLOURS, LIGHT, unreadableEither } from './themes.ts';
import type { Colours } from './themes.ts';
import { keptLook, saysAny, unfit } from './looks.ts';
import type { Look } from './looks.ts';

/** The latest version of what a theme's file says: a theme that says nothing of its look is written as the version before, which says the same, so that a client from before reads it. */
export const VERSION = 2;
/** The versions it reads: each before it says less. */
export const READS = [1, 2];
/** What a theme's file is called after the theme. */
export const ENDING = '.theme.json';

/** A theme as its file has it. */
export interface Read {
  name: string;
  colours: Colours;
  /** Its light version, or nothing for one that is the same in both. */
  light: Colours | null;
  /** What it says of how the client looks: nothing said is as usual. */
  look: Look;
}

/** What a theme's file says at its top, in the order it is written, by version. */
const SAYS: Record<number, string[]> = { 1: ['version', 'name', 'colours', 'light'], 2: ['version', 'name', 'colours', 'light', 'look'] };

const small = (colours: Colours) => Object.fromEntries(COLOURS.map(([key]) => [key, colours[key].toLowerCase()])) as unknown as Colours;

/** The file of a theme: its text, and what it is called. */
export function themeFile(name: string, colours: Colours, light: Colours | null = null, look: Look = {}): { file: string; text: string } {
  const said = { version: saysAny(look) ? VERSION : 1, name, colours: small(colours), ...(light ? { light: small(light) } : {}), ...(saysAny(look) ? { look: keptLook(look) } : {}) };
  return { file: fileName(name), text: `${JSON.stringify(said, null, 2)}\n` };
}

/** What a theme's file is called: its name in small letters, words joined by a dash, and nothing a system would refuse in a file's name. */
export function fileName(name: string): string {
  const said = name
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '');
  return `${[...said].slice(0, 60).join('').replace(/-+$/, '') || 'theme'}${ENDING}`;
}

/** What a theme's file comes to: the theme, or why it is not one, in a sentence. */
export function readThemeFile(text: string): { theme: Read } | { wrong: string } {
  let read: unknown;
  try {
    read = JSON.parse(text);
  } catch {
    return { wrong: 'This is not a theme’s file: it is not JSON.' };
  }
  if (typeof read !== 'object' || read === null || Array.isArray(read)) {
    return { wrong: 'This is not a theme’s file: a theme’s file is one mapping, of its version, its name and its colours.' };
  }
  const file = read as Record<string, unknown>;
  if (!('version' in file)) return { wrong: 'This is not a theme’s file: it says no version.' };
  const version = file['version'];
  if (typeof version !== 'number' || !READS.includes(version)) return { wrong: `This file says it is version ${JSON.stringify(version)} of a theme’s file, and this page reads versions ${READS.join(' and ')}.` };
  const says = SAYS[version]!;
  const other = Object.keys(file).find((key) => !says.includes(key));
  if (other !== undefined) return { wrong: `\`${other}\` is nothing a theme’s file of version ${version} says: it says ${says.join(', ')}.` };
  const name = typeof file['name'] === 'string' ? file['name'].trim() : '';
  if (!name) return { wrong: 'A theme’s file says what the theme is called, and this one does not.' };
  const colours = file['colours'] as Colours;
  const light = (file['light'] ?? null) as Colours | null;
  const wrong = unreadableEither(colours, light);
  if (wrong) return { wrong };
  const [inDark, inLight] = [beside(colours), light === null ? null : beside(light)];
  const more = inDark ?? inLight;
  if (more !== null) return { wrong: `${inDark === null ? LIGHT : ''}\`${more}\` is no colour of a theme: a theme’s colours are ${COLOURS.map(([one]) => one).join(', ')}.` };
  const look = file['look'] ?? null;
  const unlike = unfit(look);
  if (unlike) return { wrong: unlike };
  return { theme: { name, colours: small(colours), light: light === null ? null : small(light), look: keptLook(look as Look | null) } };
}

/** A key among a version's colours that is no colour of a theme. */
function beside(colours: Colours): string | null {
  return Object.keys(colours).find((key) => !COLOURS.some(([one]) => one === key)) ?? null;
}
