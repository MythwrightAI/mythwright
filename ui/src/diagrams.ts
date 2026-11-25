// What a `mermaid` block of code is drawn with, said as functions of plain
// values so that its tests can read them: what the drawing library is set
// up with, what the diagram is called for whoever hears it read out, how
// wide it is drawn in its column, what of its box is trimmed away, and the
// one line said under one that could not be drawn. What draws it is
// `views/diagram.tsx`.

/** The colours of the page a diagram is drawn on, each as the browser computed the page's token. */
export interface PageColours {
  /** What it is drawn on (`--bg0`). */
  background: string;
  /** A box's fill (`--bg2`). */
  node: string;
  /** Words (`--text`). */
  text: string;
  /** Lines between boxes, and what is said less (`--dim`). */
  dim: string;
  /** A box's edge (`--line-strong`). */
  line: string;
  /** What is said least (`--faint`). */
  faint: string;
}

/**
 * The colours told apart in a diagram (a pie's slices, a mind map's
 * branches), in a fixed order: eight hues, each with clear contrast
 * against the page, the same hues in light and in dark, each a step
 * lighter or darker for the page it is on. Checked as a set for readers
 * who see colours differently, each against its neighbour. A ninth and
 * after are the page's own greys, not more hues.
 */
export const CATEGORICAL = {
  light: ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#6250d6', '#e34948'],
  dark: ['#3987e5', '#d95926', '#199e70', '#c98500', '#d55181', '#0ca30c', '#9085e9', '#e66767'],
} as const;

/** The words written on a coloured slice or branch: dark on every one of the hues, in light and in dark alike. */
export const ON_COLOUR = '#111111';

/** How many colours a pie and a mind map are given. */
const SCALE = 12;

/** What the drawing library is set up with before each diagram: the page's light or dark, its letters and its colours. */
export interface DiagramSettings {
  startOnLoad: false;
  securityLevel: 'strict';
  theme: 'default' | 'dark';
  fontFamily: string;
  suppressErrorRendering: true;
  layout: 'dagre';
  themeVariables: Record<string, string>;
  gantt: { useWidth: number };
}

/**
 * How wide a gantt is drawn: the library draws one as wide as the page it
 * is drawn in, so that on a phone eight weeks' bars came out too narrow for
 * their words and on a wide window too wide to read fitted. One width
 * everywhere, at which a week's bar holds its name, is fitted to the column
 * or scrolled as any diagram is (`diagramWidth`).
 */
export const GANTT_WIDTH = 1000;

/** The colours of a diagram drawn on the page: its boxes, lines and words the page's, and what is told apart by colour the categorical set's. */
export function diagramColours(light: boolean, page: PageColours): Record<string, string> {
  const hues = light ? CATEGORICAL.light : CATEGORICAL.dark;
  const greys = [page.dim, page.faint, page.line, page.node];
  const scale = Array.from({ length: SCALE }, (_, at) => hues[at] ?? greys[(at - hues.length) % greys.length]!);
  const each = (name: string, value: (at: number) => string) => Object.fromEntries(scale.map((_, at) => [`${name}${at}`, value(at)]));
  return {
    background: page.background,
    primaryColor: page.node,
    primaryTextColor: page.text,
    primaryBorderColor: page.line,
    lineColor: page.dim,
    textColor: page.text,
    // A pie: whole slices, kept apart by a line of the page's own colour.
    ...Object.fromEntries(scale.map((colour, at) => [`pie${at + 1}`, colour])),
    pieOpacity: '1',
    pieStrokeColor: page.background,
    pieStrokeWidth: '2px',
    pieOuterStrokeColor: page.line,
    pieOuterStrokeWidth: '1px',
    pieTitleTextColor: page.text,
    pieSectionTextColor: ON_COLOUR,
    pieLegendTextColor: page.text,
    // A mind map (and a timeline): a branch a colour, its words dark on it; its root the page's words, inverted.
    ...each('cScale', (at) => scale[at]!),
    ...each('cScaleInv', (at) => scale[at]!),
    ...each('cScaleLabel', () => ON_COLOUR),
    git0: page.text,
    gitBranchLabel0: page.background,
  };
}

/**
 * The settings a diagram is drawn with. Nothing in it runs and nothing it
 * says is taken as markup (`strict`), and nothing is drawn by the library
 * of itself, neither diagrams found on the page nor the picture of a bomb
 * it draws for one that fails: a block that fails is drawn here, as its
 * code with a line saying why. It is laid out by dagre: the library's own
 * first choice is ELK, which is not built in (`withoutElk` in `build.mjs`).
 * Its colours are the page's (`diagramColours`): the library's own dark
 * draws a pie's slices and a mind map's branches near black. A gantt is
 * drawn as wide as `GANTT_WIDTH` says.
 */
export function diagramSettings(light: boolean, fontFamily: string, page: PageColours): DiagramSettings {
  return { startOnLoad: false, securityLevel: 'strict', theme: light ? 'default' : 'dark', fontFamily, suppressErrorRendering: true, layout: 'dagre', themeVariables: { fontFamily, ...diagramColours(light, page) }, gantt: { useWidth: GANTT_WIDTH } };
}

/** The longest name a diagram is given from its text. */
const LONGEST_NAME = 80;

/** A line cut to at most so many letters, with an ellipsis where it was cut. */
function cut(line: string, most: number): string {
  return line.length <= most ? line : `${line.slice(0, most - 1).trimEnd()}…`;
}

/**
 * What a diagram is called for whoever hears it read out: the title it
 * gives itself (`accTitle:`, `title:` in what heads it between `---`
 * lines, or a gantt's or a pie's `title` line), or else its first line, or
 * else Diagram.
 */
export function diagramName(text: string): string {
  const lines = text.split('\n').map((line) => line.trim());
  const said = (pattern: RegExp, among: string[]) => among.map((line) => pattern.exec(line)?.[1]?.trim()).find((one) => !!one);
  let body = lines;
  let head: string[] = [];
  if (lines[0] === '---') {
    const end = lines.indexOf('---', 1);
    if (end > 0) {
      head = lines.slice(1, end);
      body = lines.slice(end + 1);
    }
  }
  const named = said(/^accTitle\s*:\s*(.+)$/, body) ?? said(/^title\s*:\s*["']?(.+?)["']?$/, head) ?? said(/^(?:pie\s+)?title\s+(?!:)(.+)$/, body);
  const first = body.find((line) => line !== '' && !line.startsWith('%%'));
  return cut(named ?? first ?? 'Diagram', LONGEST_NAME);
}

/** The smallest a diagram's words are drawn at, in pixels, where it is made smaller to fit its column. */
export const READABLE = 11;

/** How a diagram is drawn in its column: how wide, in pixels, and whether its box scrolls sideways for the rest. */
export interface DiagramWidth {
  width: number;
  scrolls: boolean;
}

/**
 * How wide a diagram is drawn, of the width it was drawn at (`natural`),
 * the column's (`column`) and the size its words were drawn at (`text`;
 * nothing where none were measured). One narrower than the column is drawn
 * as it is; a wider one is made smaller to fit while its words still read
 * (`READABLE`), and past that is drawn at the size they read at, never
 * larger than itself, and scrolls sideways in its box.
 */
export function diagramWidth(natural: number, column: number, text: number): DiagramWidth {
  if (!(Number.isFinite(natural) && natural > 0)) return { width: 0, scrolls: false };
  if (!(Number.isFinite(column) && column > 0) || natural <= column) return { width: Math.round(natural), scrolls: false };
  const fit = column / natural;
  // Words never measured are taken to be the library's usual size.
  const said = Number.isFinite(text) && text > 0 ? text : 16;
  const least = Math.min(1, READABLE / said);
  if (fit >= least) return { width: Math.floor(column), scrolls: false };
  const width = Math.round(natural * least);
  return { width, scrolls: width > column };
}

/** The size most of a diagram's words are drawn at: the middle of the sizes measured, nothing where there are none. */
export function typicalText(sizes: readonly number[]): number {
  const known = sizes.filter((size) => Number.isFinite(size) && size > 0).sort((a, b) => a - b);
  return known.length === 0 ? 0 : known[Math.floor((known.length - 1) / 2)]!;
}

/** A box: where it begins, and how wide and high it is. */
export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** What is left round what is drawn when its box is trimmed. */
export const TRIM_ROOM = 8;

/**
 * The box a diagram is shown in, made to fit what it draws (`drawn`) with a
 * little room round it, where the library's box (`given`) leaves an empty
 * band (a mind map's, above it) or cuts something off (a mind map's
 * branches, at its sides); nothing where the two agree within the room, or
 * either was not measured.
 */
export function trimmedBox(given: Box, drawn: Box): Box | null {
  const sane = (box: Box) => [box.x, box.y, box.width, box.height].every(Number.isFinite) && box.width > 0 && box.height > 0;
  if (!sane(given) || !sane(drawn)) return null;
  const fitted = { x: drawn.x - TRIM_ROOM, y: drawn.y - TRIM_ROOM, width: drawn.width + 2 * TRIM_ROOM, height: drawn.height + 2 * TRIM_ROOM };
  const edges = [given.x - fitted.x, given.y - fitted.y, given.x + given.width - (fitted.x + fitted.width), given.y + given.height - (fitted.y + fitted.height)];
  // A few pixels either way is the library's own margin, and left as it is.
  return edges.some((edge) => Math.abs(edge) > 2 * TRIM_ROOM) ? fitted : null;
}

/** Where a box that scrolls sideways has more than shows: at its left, its right, both or neither. */
export function moreAt(scrolled: number, whole: number, shown: number): { left: boolean; right: boolean } {
  return { left: scrolled > 1, right: scrolled + shown < whole - 1 };
}

/** The longest line said under a diagram that could not be drawn. */
const LONGEST_SAID = 200;
/** How many of what a parser expected are said by name. */
const EXPECTED_NAMED = 3;

/**
 * The one line said under a diagram that could not be drawn: what the
 * library said was wrong, on one line. A parser's error says the line it
 * stopped at, then the line itself with a mark under where, then what it
 * expected there: the first and the last are said, and of a long list of
 * what was expected only the first few.
 */
export function notDrawnSaid(err: unknown): string {
  const message = err instanceof Error ? err.message : typeof err === 'string' ? err : typeof (err as { message?: unknown })?.message === 'string' ? (err as { message: string }).message : '';
  const lines = message
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  let said = (lines[0] ?? '').replace(/^Error:\s*/, '');
  const expected = lines.find((line) => line.startsWith('Expecting '));
  if (/^Parse error on line \d+:?$/.test(said) && expected) {
    const parts = /^Expecting (.+), got (.+)$/.exec(expected);
    const offered = parts ? parts[1]!.split(/,\s*/) : [];
    const what = parts && offered.length > EXPECTED_NAMED ? `Expecting ${offered.slice(0, EXPECTED_NAMED).join(', ')} or ${offered.length - EXPECTED_NAMED} more, got ${parts[2]}` : expected;
    said = `${said.replace(/:$/, '')}: ${what}`;
  }
  return `Not drawn: ${cut(said || 'the diagram could not be read.', LONGEST_SAID)}`;
}
