import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import { APPEARANCE, appearanceOf, changed, lightWorn, readSize, READING, SAVED_APPEARANCE, savedOf, USUAL_APPEARANCE } from './appearance.ts';
import { LOOKS, STACKS } from './looks.ts';

const here = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8');
const table = JSON.parse(here('./appearancecases.json')) as { words: Record<string, string[]>; cases: [unknown, string | null][] };

test('the words offered for each setting are those the daemon keeps, the usual among them', () => {
  for (const { setting, words, usual } of APPEARANCE) {
    assert.deepEqual(words.map((one) => one.word), table.words[setting], setting);
    assert.ok(words.some((one) => one.word === usual), `${setting}'s usual is one of its words`);
    assert.equal(USUAL_APPEARANCE[setting], usual);
  }
  assert.deepEqual(Object.keys(table.words), APPEARANCE.map((one) => one.setting));
  // The text's sizes are a theme's look's, called as they are there.
  const sizes = LOOKS.find((one) => one.setting === 'size')!.words;
  assert.deepEqual(APPEARANCE[0]!.words, sizes.map(({ word, label }) => ({ word, label })));
});

test('what is kept is worn word for word, and whatever is not one of its words is as usual', () => {
  const cases: [unknown, typeof USUAL_APPEARANCE][] = [
    [null, USUAL_APPEARANCE],
    ['wide', USUAL_APPEARANCE],
    [[], USUAL_APPEARANCE],
    [{ width: 'wide' }, { ...USUAL_APPEARANCE, width: 'wide' }],
    [{ text: 'largest', width: 'standard', colour: 'dark' }, { text: 'largest', width: 'standard', colour: 'dark', contents: 'shown', typeface: 'sans' }],
    [{ contents: 'hidden' }, { ...USUAL_APPEARANCE, contents: 'hidden' }],
    [{ contents: 'folded' }, USUAL_APPEARANCE],
    [{ typeface: 'serif' }, { ...USUAL_APPEARANCE, typeface: 'serif' }],
    [{ typeface: 'mono' }, USUAL_APPEARANCE],
    [{ text: 'huge', width: 7, colour: 'Dark', margins: 'none' }, USUAL_APPEARANCE],
  ];
  for (const [kept, worn] of cases) assert.deepEqual(appearanceOf(kept), worn, JSON.stringify(kept));
  // Every setting the daemon keeps is worn as it is.
  for (const [said, refused] of table.cases) {
    if (refused !== null) continue;
    const worn = appearanceOf(said);
    for (const { setting, usual } of APPEARANCE) assert.equal(worn[setting], (said as Record<string, string | null>)[setting] ?? usual);
  }
  assert.deepEqual(changed(USUAL_APPEARANCE, 'colour', 'light'), { ...USUAL_APPEARANCE, colour: 'light' });
  assert.deepEqual(changed(USUAL_APPEARANCE, 'width', 'widest'), USUAL_APPEARANCE, 'nothing that is not a word of it');
});

test('the page is light where it is chosen, or where the system asks and neither is', () => {
  const cases: [string, boolean, boolean][] = [
    ['automatic', true, true],
    ['automatic', false, false],
    ['light', false, true],
    ['light', true, true],
    ['dark', true, false],
    ['dark', false, false],
    ['anything else', true, true],
  ];
  for (const [colour, system, light] of cases) assert.equal(lightWorn(colour, system), light, `${colour}, the system ${system ? 'light' : 'dark'}`);
});

test('the text is drawn at a theme look\'s sizes, and the usual one at the theme\'s own', () => {
  assert.equal(readSize('usual'), null);
  assert.equal(readSize('larger'), '1.1');
  assert.equal(readSize('smaller'), '0.9');
  assert.equal(readSize('nonsense'), null);
  const saved = savedOf({ text: 'largest', width: 'wide', colour: 'dark', contents: 'hidden', typeface: 'serif' });
  assert.deepEqual(saved, { text: 'largest', width: 'wide', colour: 'dark', contents: 'hidden', typeface: 'serif', size: '1.2', face: STACKS.serif });
  assert.deepEqual(appearanceOf(JSON.parse(JSON.stringify(saved))), { text: 'largest', width: 'wide', colour: 'dark', contents: 'hidden', typeface: 'serif' }, 'read back as it was');
  assert.equal(savedOf(USUAL_APPEARANCE).face, null, 'sans is the page\'s own letters, and nothing is put on the page for it');
});

test('the page is first drawn as it was kept: light, wide and its text, as index.html runs it', () => {
  const page = here('../index.html');
  assert.ok(page.includes(`localStorage.getItem('${SAVED_APPEARANCE}')`), 'index.html reads where the browser keeps it');
  const early = page.slice(page.indexOf('<script>', page.indexOf('before anything is drawn in another')) + '<script>'.length);
  const script = early.slice(0, early.indexOf('</script>'));
  const run = (kept: Record<string, string>, systemLight: boolean) => {
    const put: Record<string, string> = {};
    const documentElement = { dataset: {} as Record<string, string>, style: { setProperty: (name: string, value: string) => (put[name] = value) } };
    runInNewContext(script, { localStorage: { getItem: (key: string) => kept[key] ?? null }, document: { documentElement }, matchMedia: () => ({ matches: systemLight }), JSON, String });
    return { marks: documentElement.dataset, put };
  };
  const saved = (said: Parameters<typeof savedOf>[0]) => ({ [SAVED_APPEARANCE]: JSON.stringify(savedOf(said)) });
  assert.deepEqual(run({}, false).marks, { theme: 'grid-iron' }, 'nothing kept: dark where the system is');
  assert.ok('light' in run({}, true).marks, 'and light where it asks for it');
  assert.ok(!('light' in run(saved({ ...USUAL_APPEARANCE, colour: 'dark' }), true).marks), 'dark where it was chosen');
  const chosen = run(saved({ text: 'larger', width: 'wide', colour: 'light', contents: 'hidden', typeface: 'serif' }), false);
  assert.deepEqual(chosen.marks, { theme: 'grid-iron', light: '', readWidth: 'wide', readText: '', readFace: 'serif' });
  assert.deepEqual(chosen.put, { '--read-size': '1.1', '--read-face': STACKS.serif });
  // A light version of a theme somebody made is worn where the page is light, chosen or asked.
  const own = { 'client.theme.colours': JSON.stringify({ '--bg0': '#000000' }), 'client.theme.light': JSON.stringify({ '--bg0': '#ffffff' }) };
  assert.equal(run({ ...own, ...saved({ ...USUAL_APPEARANCE, colour: 'light' }) }, false).put['--bg0'], '#ffffff');
  assert.equal(run({ ...own, ...saved({ ...USUAL_APPEARANCE, colour: 'dark' }) }, true).put['--bg0'], '#000000');
});

test('the reading column is widened and its text sized by the marks the page wears, and by nothing else', () => {
  const sheet = here('./appearance.css');
  assert.ok(sheet.includes(`:root[data-read-width="wide"] .${READING} { --column: var(--wide-column); }`));
  assert.ok(sheet.includes(`:root[data-read-text] .${READING} { --size: var(--read-size);`));
  assert.ok(sheet.includes(`:root[data-read-face] .${READING}:is(.topic-page[data-shows="article"], .doc-body) :is(.topic-article, .topic-title-sized, .topic-title-field, .writer) { font-family: var(--read-face); }`));
  // Standard is the measure every page has, said once.
  assert.ok(here('./page.css').includes(':root { --column: 720px; }'));
});
