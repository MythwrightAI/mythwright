import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
import { EMOJI, LETTERED, LOOKED, LOOKS, STACKS, withEmoji, USUAL, keptLook, lookOf, lookSaid, saysAny, toldLook, unfit, wordOf } from './looks.ts';

const here = (file: string) => readFileSync(new URL(file, import.meta.url), 'utf8');
const table = JSON.parse(here('./themecases.json')) as { looks: [string, string[]][]; look: [unknown, string | null][] };

test('a look comes to what the daemon says of it, case for case', () => {
  assert.ok(table.look.length >= 10, 'the table is there');
  for (const [look, said] of table.look) assert.equal(unfit(look), said, JSON.stringify(look));
});

test('each setting and its words are what the daemon holds a look to', () => {
  assert.deepEqual(
    LOOKS.map(({ setting, words }) => [setting, words.map((one) => one.word)]),
    table.looks,
  );
  for (const { words } of LOOKS) {
    assert.equal(words.filter((one) => one.word === USUAL).length, 1, 'each has the usual one');
    assert.equal(words.find((one) => one.word === USUAL)?.value, null, 'which says nothing of its own');
  }
});

test('a look puts on the page only the measures of what it says that is not as usual', () => {
  assert.deepEqual(lookOf(null), {});
  assert.deepEqual(lookOf({ size: USUAL, corners: USUAL }), {});
  assert.deepEqual(lookOf({ letters: 'serif', size: 'largest', density: 'compact', corners: 'square' }), {
    '--sans': STACKS.serif,
    '--size': '1.2',
    '--density': '0.8',
    '--round': '0',
  });
  assert.deepEqual(Object.keys(lookOf({ letters: 'mono', size: 'smaller', density: 'roomy', corners: 'round' })).sort(), [...LOOKED].sort(), 'every measure there is can be put on');
});

test('the measures a look goes by are said where the page is first drawn, as usual', () => {
  const style = here('./themes.css');
  for (const measure of ['--size: 1;', '--density: 1;', '--round: 1;', '--tall: calc(var(--size) * var(--density));']) {
    assert.ok(style.includes(measure), `themes.css says ${measure}`);
  }
  const page = here('../index.html');
  assert.ok(page.includes("localStorage.getItem('client.theme.look')"), 'index.html puts the look on before anything is drawn');
});

test('every size of text and every corner of the style sheets goes by the look', () => {
  // A size written out that the look does not reach is a line of text that stays as it was at every size.
  const sheets = readdirSync(new URL('.', import.meta.url)).filter((file) => file.endsWith('.css'));
  for (const sheet of sheets) {
    const text = here(`./${sheet}`).replace(/\/\*[\s\S]*?\*\//g, '');
    for (const rule of text.split('}')) {
      // A graph's tiles are measured in their own pixels, and zoom with what they are of.
      if (/gnode|canvas/.test(rule.split('{')[0] ?? '')) continue;
      const written = /(?:font-size|border(?:-[a-z]+)*-radius)\s*:\s*[^;]*(?<![\w(.*-])[1-9][0-9.]*px(?![^;]*var\(--)/.exec(rule);
      assert.equal(written, null, `${sheet}: ${written?.[0]}`);
    }
  }
});

test('a look is kept as what it says that is not usual, and said in a few words', () => {
  assert.deepEqual(keptLook({ size: 'larger', density: USUAL }), { size: 'larger' });
  assert.equal(saysAny({ corners: USUAL }), false);
  assert.equal(saysAny({ corners: 'round' }), true);
  assert.equal(wordOf({}, 'size'), USUAL);
  assert.equal(lookSaid({ size: 'largest', density: 'compact' }), 'Size: largest, Density: compact');
  assert.equal(lookSaid(null), '');
});

test('a check is told a look in words, and refused one that is none', () => {
  assert.equal(toldLook(undefined), null);
  assert.equal(toldLook(' '), null);
  assert.deepEqual(toldLook('size=largest density=compact'), { size: 'largest', density: 'compact' });
  assert.deepEqual(toldLook('size=largest,corners=square'), { size: 'largest', corners: 'square' });
  assert.throws(() => toldLook('largest'), /not a setting of a look/);
  assert.throws(() => toldLook('size=huge'), /is none of them/);
});

test('every stack of letters ends with the systems\' emoji, the usual ones too, so that a ✅ is never an empty box', () => {
  for (const [name, stack] of Object.entries(STACKS)) assert.ok(stack.endsWith(`, ${EMOJI}`), name);
  const style = here('./themes.css');
  for (const measure of ['--sans', '--mono']) {
    const said = style.match(new RegExp(`\\n\\s*${measure}: ([^;]+);`))?.[1];
    assert.ok(said?.endsWith(`, ${EMOJI}`), `${measure} is ${said}`);
  }
});

test('a stack of letters kept from before the emoji were added is worn with them, and one that has them as it is', () => {
  const old = '"Iowan Old Style", "Palatino Linotype", Palatino, "URW Palladio L", P052, Georgia, Cambria, serif';
  const cases: [string, string][] = [
    [old, `${old}, ${EMOJI}`],
    [STACKS.serif, STACKS.serif],
    [`${old}, `, `${old}, ${EMOJI}`],
    [' monospace ', `monospace, ${EMOJI}`],
    ['', ''],
  ];
  for (const [kept, worn] of cases) assert.equal(withEmoji(kept), worn, kept);
  assert.deepEqual(LETTERED, ['--sans', '--mono'], 'the measures that are stacks of letters');
  // The page says the same before anything is drawn, from what a browser kept: the same letters, put on the same two measures.
  const page = here('../index.html');
  assert.ok(page.includes(`var emoji = '${EMOJI}';`), 'index.html ends a kept stack with the same emoji');
  assert.ok(page.includes("measure === '--sans' || measure === '--mono'"), 'index.html ends both stacks of letters with them');
  // And run as the page runs it, over what a browser kept from before: each stack worn as `withEmoji` wears it.
  const early = page.slice(page.indexOf('<script>', page.indexOf('before anything is drawn in another')) + '<script>'.length);
  const script = early.slice(0, early.indexOf('</script>'));
  const kept: Record<string, string> = { 'client.theme.look': JSON.stringify({ '--sans': old, '--mono': STACKS.mono, '--size': '1.2' }) };
  const put: Record<string, string> = {};
  const documentElement = { dataset: {}, style: { setProperty: (name: string, value: string) => (put[name] = value) } };
  runInNewContext(script, { localStorage: { getItem: (key: string) => kept[key] ?? null }, document: { documentElement }, matchMedia: () => ({ matches: false }), JSON, String });
  assert.deepEqual(put, { '--sans': withEmoji(old), '--mono': STACKS.mono, '--size': '1.2' });
});
