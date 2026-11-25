import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ENDING, VERSION, fileName, readThemeFile, themeFile } from './themefile.ts';
import { LIGHTS, STARTS } from './themes.ts';

const dusk = STARTS['rose-pine']!;
const dawn = LIGHTS['rose-pine']!;

test('a theme saved as a file is read back as the theme it was', () => {
  const { file, text } = themeFile('Old harbour', { ...dusk, text: dusk.text.toUpperCase() });
  assert.equal(file, `old-harbour${ENDING}`);
  const written = JSON.parse(text);
  assert.deepEqual(Object.keys(written), ['version', 'name', 'colours'], 'what it says, in order, with no light version where it has none');
  assert.equal(written.version, 1, 'a theme that says nothing of its look is the version before, which a client from before reads');
  assert.equal(written.colours.text, dusk.text.toLowerCase(), 'written small');
  assert.deepEqual(readThemeFile(text), { theme: { name: 'Old harbour', colours: dusk, light: null, look: {} } });
  assert.ok(text.endsWith('}\n'), 'one file, ending in a line');
});

test('a theme with a light version is saved with it, and read back with it', () => {
  const { text } = themeFile('Dusk', dusk, { ...dawn, ground: dawn.ground.toUpperCase() });
  const written = JSON.parse(text);
  assert.deepEqual(Object.keys(written), ['version', 'name', 'colours', 'light']);
  assert.equal(written.version, 1, 'the same version: a file with no light version is read as one');
  assert.deepEqual(readThemeFile(text), { theme: { name: 'Dusk', colours: dusk, light: dawn, look: {} } });
  assert.deepEqual(readThemeFile(JSON.stringify({ version: VERSION, name: 'Dusk', colours: dusk, light: null })), { theme: { name: 'Dusk', colours: dusk, light: null, look: {} } });
});

test('a theme with a look is saved as the version that says one, and read back with it; one from before still reads', () => {
  const { text } = themeFile('Dusk', dusk, dawn, { letters: 'serif', size: 'largest', density: 'usual' });
  const written = JSON.parse(text);
  assert.deepEqual(Object.keys(written), ['version', 'name', 'colours', 'light', 'look']);
  assert.equal(written.version, VERSION);
  assert.deepEqual(written.look, { letters: 'serif', size: 'largest' }, 'what is usual is left out');
  assert.deepEqual(readThemeFile(text), { theme: { name: 'Dusk', colours: dusk, light: dawn, look: { letters: 'serif', size: 'largest' } } });
  const before = JSON.stringify({ version: 1, name: 'Dusk', colours: dusk, light: dawn });
  assert.deepEqual(readThemeFile(before), { theme: { name: 'Dusk', colours: dusk, light: dawn, look: {} } }, 'a file of version 1 is a theme that says nothing of its look');
});

test('a theme’s file is called after the theme, with nothing a system refuses in a name', () => {
  const cases: [string, string][] = [
    ['Dusk', 'dusk'],
    ['  Late dusk / dawn: 2 ', 'late-dusk-dawn-2'],
    ['Rosé Pine', 'rosé-pine'],
    ['***', 'theme'],
    ['a'.repeat(70), 'a'.repeat(60)],
    [`${'a'.repeat(59)} b`, 'a'.repeat(59)],
  ];
  for (const [name, called] of cases) assert.equal(fileName(name), `${called}${ENDING}`, name);
});

test('what is not a theme’s file is said in a sentence', () => {
  const file = (said: Record<string, unknown>) => JSON.stringify({ version: VERSION, name: 'Dusk', colours: dusk, ...said });
  const cases: [string, string][] = [
    ['{ "version": 1, ', 'This is not a theme’s file: it is not JSON.'],
    ['', 'This is not a theme’s file: it is not JSON.'],
    ['[1, 2]', 'This is not a theme’s file: a theme’s file is one mapping, of its version, its name and its colours.'],
    ['"dusk"', 'This is not a theme’s file: a theme’s file is one mapping, of its version, its name and its colours.'],
    ['null', 'This is not a theme’s file: a theme’s file is one mapping, of its version, its name and its colours.'],
    [JSON.stringify({ name: 'Dusk', colours: dusk }), 'This is not a theme’s file: it says no version.'],
    [file({ version: 3 }), 'This file says it is version 3 of a theme’s file, and this page reads versions 1 and 2.'],
    [file({ version: '1' }), 'This file says it is version "1" of a theme’s file, and this page reads versions 1 and 2.'],
    [file({ glow: true }), '`glow` is nothing a theme’s file of version 2 says: it says version, name, colours, light, look.'],
    [file({ version: 1, look: { size: 'larger' } }), '`look` is nothing a theme’s file of version 1 says: it says version, name, colours, light.'],
    [file({ look: { size: 'huge' } }), 'A theme\'s size is one of smaller, usual, larger, largest, and "huge" is none of them.'],
    [file({ look: 'large' }), "A theme's look is a mapping of what it says of letters, size, density, corners."],
    [file({ name: '  ' }), 'A theme’s file says what the theme is called, and this one does not.'],
    [file({ name: 5 }), 'A theme’s file says what the theme is called, and this one does not.'],
    [file({ colours: 'pink' }), "A theme's colours are a mapping of eight: ground, text, accent, ok, bad, info, agent, card."],
    [file({ colours: { ...dusk, accent: 'pink' } }), 'accent is not a colour'],
    [file({ colours: { ...dusk, ground: '#fff' } }), 'ground is not a colour'],
    [file({ colours: { ...dusk, text: dusk.ground } }), 'The text is too near the ground to be read on it: make one lighter or the other darker.'],
    [file({ colours: { ...dusk, glow: '#ffffff' } }), '`glow` is no colour of a theme: a theme’s colours are ground, text, accent, ok, bad, info, agent, card.'],
    [file({ light: { ...dawn, text: dawn.ground } }), 'In the light version: The text is too near the ground to be read on it: make one lighter or the other darker.'],
    [file({ light: { ...dawn, card: '#ea9d3' } }), 'In the light version: card is not a colour'],
    [file({ light: 'dawn' }), "In the light version: A theme's colours are a mapping of eight: ground, text, accent, ok, bad, info, agent, card."],
    [file({ light: { ...dawn, glow: '#ffffff' } }), 'In the light version: `glow` is no colour of a theme: a theme’s colours are ground, text, accent, ok, bad, info, agent, card.'],
    [file({ colours: { ...dusk, text: dusk.ground }, light: { ...dawn, text: dawn.ground } }), 'The text is too near the ground to be read on it: make one lighter or the other darker.'],
  ];
  for (const [text, says] of cases) assert.deepEqual(readThemeFile(text), { wrong: says }, text);
});

test('a name the daemon would refuse is the daemon’s to say', () => {
  // What a name may be is held where a theme is kept, and said in its own sentence.
  const read = readThemeFile(JSON.stringify({ version: VERSION, name: 'x'.repeat(61), colours: dusk }));
  assert.ok('theme' in read && read.theme.name.length === 61);
});
