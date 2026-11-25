import assert from 'node:assert/strict';
import { test } from 'node:test';
import { CATEGORICAL, GANTT_WIDTH, ON_COLOUR, READABLE, TRIM_ROOM, diagramColours, diagramName, diagramSettings, diagramWidth, moreAt, notDrawnSaid, trimmedBox, typicalText } from './diagrams.ts';
import type { PageColours } from './diagrams.ts';

const DARK: PageColours = { background: 'rgb(12, 12, 12)', node: 'rgb(27, 27, 27)', text: 'rgb(233, 233, 233)', dim: 'rgb(165, 165, 165)', line: 'rgb(58, 58, 58)', faint: 'rgb(111, 111, 111)' };
const LIGHT: PageColours = { background: 'rgb(241, 241, 241)', node: 'rgb(233, 233, 233)', text: 'rgb(22, 22, 22)', dim: 'rgb(82, 82, 82)', line: 'rgb(194, 194, 194)', faint: 'rgb(135, 135, 135)' };

test('a diagram is drawn strictly, by nothing but the client, laid out by what is built in, in the light or dark the page is in, its letters and its colours', () => {
  const settings = diagramSettings(true, 'Inter, sans-serif', LIGHT);
  const { themeVariables, ...rest } = settings;
  assert.deepEqual(rest, { startOnLoad: false, securityLevel: 'strict', theme: 'default', fontFamily: 'Inter, sans-serif', suppressErrorRendering: true, layout: 'dagre', gantt: { useWidth: GANTT_WIDTH } });
  assert.equal(GANTT_WIDTH, 1000, 'a gantt as wide wherever it is drawn');
  assert.equal(themeVariables.fontFamily, 'Inter, sans-serif');
  assert.deepEqual(themeVariables, { fontFamily: 'Inter, sans-serif', ...diagramColours(true, LIGHT) });
  assert.equal(diagramSettings(false, 'serif', DARK).theme, 'dark');
});

/** The relative luminance of `#rrggbb` or `rgb(r, g, b)`, as contrast is reckoned. */
function luminance(colour: string): number {
  const hex = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(colour);
  const parts = hex ? hex.slice(1).map((part) => parseInt(part, 16)) : (/rgb\((\d+), (\d+), (\d+)\)/.exec(colour)?.slice(1).map(Number) ?? []);
  const [r, g, b] = parts.map((part) => {
    const c = part / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}
const contrast = (a: string, b: string) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi! + 0.05) / (lo! + 0.05);
};

test('a pie and a mind map are drawn in the page’s colours: each slice and branch its own hue, told from the page and the next', () => {
  const cases: [boolean, PageColours][] = [
    [true, LIGHT],
    [false, DARK],
  ];
  for (const [light, page] of cases) {
    const said = diagramColours(light, page);
    const hues = light ? CATEGORICAL.light : CATEGORICAL.dark;
    // The eight hues in their order, the same eight in light and dark, then the page's greys.
    assert.deepEqual([1, 2, 3, 4, 5, 6, 7, 8].map((nth) => said[`pie${nth}`]), [...hues]);
    assert.deepEqual([0, 1, 2, 3, 4, 5, 6, 7].map((nth) => said[`cScale${nth}`]), [...hues]);
    assert.deepEqual([9, 10, 11, 12].map((nth) => said[`pie${nth}`]), [page.dim, page.faint, page.line, page.node]);
    assert.equal(new Set(Object.entries(said).filter(([key]) => /^pie\d+$/.test(key)).map(([, colour]) => colour)).size, 12, 'no two slices alike');
    // Each hue is told from the page it is on, and the words on it read.
    for (const hue of hues) {
      assert.ok(contrast(hue, page.background) >= (light ? 1.6 : 2.2), `${hue} on ${page.background}`);
      assert.ok(contrast(hue, ON_COLOUR) >= 3, `${ON_COLOUR} on ${hue}`);
    }
    // Slices are whole, kept apart by the page's own colour; their words, the title and the legend are the page's.
    assert.equal(said.pieOpacity, '1');
    assert.equal(said.pieStrokeColor, page.background);
    assert.equal(said.pieTitleTextColor, page.text);
    assert.equal(said.pieLegendTextColor, page.text);
    assert.equal(said.pieSectionTextColor, ON_COLOUR);
    assert.equal(said.cScaleLabel0, ON_COLOUR);
    // Boxes, lines and words of every other diagram are the page's.
    assert.deepEqual([said.background, said.primaryColor, said.primaryTextColor, said.lineColor], [page.background, page.node, page.text, page.dim]);
    // A mind map's root is the page's words, inverted.
    assert.deepEqual([said.git0, said.gitBranchLabel0], [page.text, page.background]);
  }
  // The same hues in both, each stepped for its page.
  assert.equal(CATEGORICAL.light.length, CATEGORICAL.dark.length);
});

test('a diagram fits its column while its words still read, and past that is drawn where they read and scrolls in its box', () => {
  assert.equal(READABLE, 11);
  const cases: [string, [number, number, number], { width: number; scrolls: boolean }][] = [
    ['narrower than the column: as drawn', [400, 700, 16], { width: 400, scrolls: false }],
    ['as wide as the column: as drawn', [700, 700, 16], { width: 700, scrolls: false }],
    ['a little wider, words of 16: fitted, its words at 14', [800, 700, 16], { width: 700, scrolls: false }],
    ['words down to 11 exactly: fitted', [1600, 1100, 16], { width: 1100, scrolls: false }],
    ['a wide flowchart: words would be 7, drawn at 11 and scrolled', [1600, 700, 16], { width: 1100, scrolls: true }],
    ['a gantt with words of 11: never smaller, scrolled', [1200, 700, 11], { width: 1200, scrolls: true }],
    ['words already smaller than readable: never made larger', [1200, 700, 9], { width: 1200, scrolls: true }],
    ['words not measured: taken as the usual 16', [1600, 700, 0], { width: 1100, scrolls: true }],
    ['a phone: a mind map of words at 16', [900, 358, 16], { width: 619, scrolls: true }],
    ['no column yet (a tab not showing): as drawn', [1600, 0, 16], { width: 1600, scrolls: false }],
    ['nothing drawn', [0, 700, 16], { width: 0, scrolls: false }],
    ['nonsense', [Number.NaN, 700, 16], { width: 0, scrolls: false }],
  ];
  for (const [said, [natural, column, text], wanted] of cases) assert.deepEqual(diagramWidth(natural, column, text), wanted, said);
});

test('the size of a diagram’s words is what most of them are drawn at', () => {
  const cases: [number[], number][] = [
    [[], 0],
    [[16], 16],
    [[11, 11, 11, 10, 14], 11],
    [[16, 16, 12, 0, Number.NaN], 16],
    [[10, 20], 10],
  ];
  for (const [sizes, wanted] of cases) assert.equal(typicalText(sizes), wanted, String(sizes));
});

test('a diagram’s box is fitted to what it draws where the library left an empty band or cut something off', () => {
  const room = TRIM_ROOM;
  const cases: [string, [number, number, number, number], [number, number, number, number], [number, number, number, number] | null][] = [
    ['fills its box: left as it is', [0, 0, 400, 300], [8, 8, 384, 284], null],
    ['within the library’s margin: left', [0, 0, 400, 300], [4, 12, 390, 280], null],
    ['an empty band above (a mind map)', [0, 0, 800, 900], [10, 400, 780, 480], [10 - room, 400 - room, 780 + 2 * room, 480 + 2 * room]],
    ['cut off at both sides', [0, 0, 600, 400], [-120, 4, 840, 390], [-120 - room, 4 - room, 840 + 2 * room, 390 + 2 * room]],
    ['nothing drawn yet', [0, 0, 600, 400], [0, 0, 0, 0], null],
    ['no box given', [0, 0, 0, 0], [0, 0, 100, 100], null],
  ];
  const box = ([x, y, width, height]: [number, number, number, number]) => ({ x, y, width, height });
  for (const [said, given, drawn, wanted] of cases) assert.deepEqual(trimmedBox(box(given), box(drawn)), wanted && box(wanted), said);
});

test('a box that scrolls sideways says at which edge it has more', () => {
  const cases: [[number, number, number], { left: boolean; right: boolean }][] = [
    [[0, 700, 700], { left: false, right: false }],
    [[0, 1200, 700], { left: false, right: true }],
    [[250, 1200, 700], { left: true, right: true }],
    [[500, 1200, 700], { left: true, right: false }],
    [[499.5, 1200, 700], { left: true, right: false }],
  ];
  for (const [[scrolled, whole, shown], wanted] of cases) assert.deepEqual(moreAt(scrolled, whole, shown), wanted, String([scrolled, whole, shown]));
});

test('a diagram is called by the title it gives itself, or else by its first line, or else Diagram', () => {
  const cases: [string, string][] = [
    ['graph TD; Card-->Bank', 'graph TD; Card-->Bank'],
    ['\n  flowchart LR\n  A --> B\n', 'flowchart LR'],
    ['%%{init: {"theme": "forest"}}%%\nsequenceDiagram\n  A->>B: hi', 'sequenceDiagram'],
    ['flowchart LR\n  accTitle: How a sale goes\n  A --> B', 'How a sale goes'],
    ['---\ntitle: "The till"\n---\nflowchart LR\n  A --> B', 'The till'],
    ['---\nconfig:\n  look: handDrawn\n---\nflowchart LR', 'flowchart LR'],
    ['gantt\n  title Eight weeks of practice\n  dateFormat YYYY-MM-DD', 'Eight weeks of practice'],
    ['pie title Time by practice\n  "Breath" : 32', 'Time by practice'],
    ['pie showData\n  title Kept\n  "a" : 1', 'Kept'],
    ['', 'Diagram'],
    ['   \n%% only a comment\n', 'Diagram'],
  ];
  for (const [text, name] of cases) assert.equal(diagramName(text), name, text);
  const long = diagramName(`flowchart LR; ${'A-->B; '.repeat(30)}`);
  assert.ok(long.length <= 80 && long.endsWith('…'), long);
});

test('what is said under a diagram that could not be drawn is the library’s words, on one line', () => {
  const parse = new Error("Parse error on line 2:\ngraph TD; A-->\n--------------^\nExpecting 'AMP', 'COLON', 'PIPE', 'TESTSTR', 'DOWN', 'DEFAULT', got 'EOF'");
  assert.equal(notDrawnSaid(parse), "Not drawn: Parse error on line 2: Expecting 'AMP', 'COLON', 'PIPE' or 3 more, got 'EOF'");
  const few = new Error("Parse error on line 1:\nA\n^\nExpecting 'NEWLINE', got 'EOF'");
  assert.equal(notDrawnSaid(few), "Not drawn: Parse error on line 1: Expecting 'NEWLINE', got 'EOF'");
  assert.equal(notDrawnSaid(new Error('No diagram type detected matching given configuration for text: nonsense')), 'Not drawn: No diagram type detected matching given configuration for text: nonsense');
  assert.equal(notDrawnSaid({ message: 'Parsing failed: Expecting token of type \':\' but found `x`.\nmore' }), "Not drawn: Parsing failed: Expecting token of type ':' but found `x`.");
  assert.equal(notDrawnSaid('Error: said as text'), 'Not drawn: said as text');
  assert.equal(notDrawnSaid(undefined), 'Not drawn: the diagram could not be read.');
  const said = notDrawnSaid(new Error('x'.repeat(500)));
  assert.ok(!said.includes('\n') && said.length <= 'Not drawn: '.length + 200 && said.endsWith('…'));
});
