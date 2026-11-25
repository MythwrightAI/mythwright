import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { COLOURS, DEFAULT_THEME, LIGHT, LIGHTS, LONGEST, SAVED_COLOURS, SAVED_LIGHT, SAVED_THEME, STARTS, chosenOnce, contrast, heardTheme, keptName, lightFrom, lightness, listOf, ownThemes, takenUp, themeOf, THEMES, unreadable, unreadableEither, wornOf } from './themes.ts';
import type { Colours } from './themes.ts';
import type { KeptTheme, Themed } from './themes.ts';

const here = (file: string) => readFileSync(new URL(file, import.meta.url), 'utf8');
const cases = JSON.parse(here('./themecases.json')) as { ships: string[]; cases: [Record<string, unknown>, string | null][]; light: [unknown, unknown, string | null][] };

test('the theme kept is the one used, and anything else is the default', () => {
  assert.equal(themeOf('rose-pine'), 'rose-pine');
  assert.equal(themeOf(null), DEFAULT_THEME);
  assert.equal(themeOf('one-there-was-once'), DEFAULT_THEME);
});

test('every theme has its colours, dark and light, and a sample to be drawn in them', () => {
  const style = here('./themes.css');
  for (const theme of THEMES) {
    const lists = style.split(`.theme-sample[data-theme="${theme.id}"] {`).length - 1;
    assert.equal(lists, 2, `${theme.name} has ${lists} lists of colours; it needs a dark one and a light one`);
  }
});

test('the page is put in the theme that was kept before anything is drawn', () => {
  const page = here('../index.html');
  assert.ok(page.includes(`localStorage.getItem('${SAVED_THEME}') || '${DEFAULT_THEME}'`), 'index.html reads the same place, and falls back to the same theme');
});


test('a theme somebody made is one that can be chosen, and one that was taken away is not', () => {
  const made = [{ id: 'own-dusk', name: 'Dusk', says: '', colours: STARTS['rose-pine']! }];
  assert.equal(themeOf('own-dusk', made), 'own-dusk');
  assert.equal(themeOf('own-dusk', []), DEFAULT_THEME);
});

test('eight colours come to a whole list, as the themes that ship have', () => {
  const style = here('./themes.css');
  const shipped = [...style.split('.theme-sample[data-theme="grid-iron"] {')[1]!.split('}')[0]!.matchAll(/^\s*(--[a-z0-9-]+|color-scheme):/gm)].map((found) => found[1]!);
  const list = listOf(STARTS['grid-iron']!);
  assert.deepEqual(Object.keys(list).sort(), [...shipped].sort(), 'a theme made here says everything a theme that ships does');
  // Worked out from the ground and the text, the rest is near what was written by hand.
  assert.equal(list['--bg0'], '#0c0c0c');
  assert.equal(list['color-scheme'], 'dark');
  assert.equal(listOf({ ...STARTS['grid-iron']!, ground: '#f4f3ef', text: '#1d1f23' })['color-scheme'], 'light');
  // What is written on the accent can be read on it, whichever it is.
  assert.ok(listOf({ ...STARTS['grid-iron']!, accent: '#ffe08a' })['--accent-ink']!.startsWith('#1'), 'dark ink on a light accent');
  assert.ok(listOf({ ...STARTS['grid-iron']!, accent: '#1d2a6b' })['--accent-ink']!.startsWith('#e') || listOf({ ...STARTS['grid-iron']!, accent: '#1d2a6b' })['--accent-ink']!.startsWith('#f'), 'light ink on a dark accent');
  assert.equal(COLOURS.length, Object.keys(STARTS['grid-iron']!).length);
});

test('a theme whose text cannot be read on its ground is not one, and is not kept as one', () => {
  assert.equal(unreadable(STARTS['tokyo-midnight']!), null);
  assert.ok(unreadable({ ...STARTS['grid-iron']!, text: '#222222' }));
  assert.ok(unreadable({ ...STARTS['grid-iron']!, accent: 'orange' }));
  const kept = JSON.stringify([{ id: 'own-a', name: 'A', says: '', colours: STARTS['grid-iron'] }, { id: 'own-b', name: 'B', says: '', colours: { ...STARTS['grid-iron'], text: '#111111' } }, { id: 'grid-iron', name: 'Not one', colours: STARTS['grid-iron'] }]);
  assert.deepEqual(ownThemes(kept).map((one) => one.id), ['own-a']);
  assert.deepEqual(ownThemes('not json'), []);
});

test('the page puts on the colours of a theme somebody made before anything is drawn', () => {
  assert.ok(here('../index.html').includes(`localStorage.getItem('${SAVED_COLOURS}')`), 'index.html reads where the colours in use are kept');
  assert.ok(here('../index.html').includes(`localStorage.getItem('${SAVED_LIGHT}')`), 'and where those of its light version are, for where the system asks for light');
});

test('the light colours of each theme that ships are those its light list in themes.css is made of', () => {
  const style = here('./themes.css');
  const light = style.slice(style.indexOf(':root[data-light]'));
  const names: [keyof Colours, string][] = [['ground', '--bg0'], ['text', '--text'], ['accent', '--accent'], ['ok', '--ok'], ['bad', '--bad'], ['info', '--info'], ['agent', '--agent'], ['card', '--card']];
  for (const theme of THEMES) {
    const list = light.split(`.theme-sample[data-theme="${theme.id}"] {`)[1]!.split('}')[0]!;
    for (const [name, property] of names) {
      const written = new RegExp(`${property}:\\s*(#[0-9a-f]{6})`).exec(list)?.[1];
      assert.equal(LIGHTS[theme.id]![name], written, `${theme.name}'s light ${name}`);
    }
    assert.equal(unreadable(LIGHTS[theme.id]!), null);
  }
  assert.equal(names.length, COLOURS.length);
});

test('a theme and its light version are each held to being read, the dark one first, case for case with the daemon', () => {
  for (const [colours, light, says] of cases.light) assert.equal(unreadableEither(colours as never, light as never), says, `${JSON.stringify(colours)} ${JSON.stringify(light)}`);
  assert.ok(cases.light.some(([, , says]) => says?.startsWith(LIGHT)), 'a light version said to be what is wrong');
});

test('a light version begins as worked out from the dark one: ground and text exchanged, everything else kept to be seen', () => {
  const made = [...THEMES.map((one) => STARTS[one.id]!), { ...STARTS['grid-iron']!, text: '#8a8a8a' }, { ...STARTS['rose-pine']!, ground: '#3a3a3a', text: '#ffffff' }, { ...STARTS['tokyo-midnight']!, accent: '#101030', card: '#1a1a1a' }];
  for (const dark of made) {
    const light = lightFrom(dark);
    assert.equal(unreadable(light), null, `${JSON.stringify(dark)} comes to one that is read`);
    assert.ok(lightness(light.ground) > 0.5 && lightness(light.text) < lightness(light.ground), `a dark ground comes to a light one: ${light.ground}`);
    assert.ok(contrast(light.text, light.ground) >= Math.min(7, contrast(dark.text, dark.ground)) - 0.01, 'the text as far from its ground as it was, up to the most that is asked');
    for (const [name] of COLOURS.slice(2)) assert.ok(contrast(light[name], light.ground) >= 3 - 0.01, `${name} stands out from the light ground: ${light[name]} on ${light.ground}`);
  }
  // Exchanged in lightness, each of its own hue: Grid Iron's comes to near what its light list was written as.
  const iron = lightFrom(STARTS['grid-iron']!);
  assert.deepEqual([iron.ground, iron.text], ['#f3f3f3', '#161616']);
  // An accent already far enough from the light ground is kept as it was.
  assert.equal(lightFrom({ ...STARTS['grid-iron']!, accent: '#1d2a6b' }).accent, '#1d2a6b');
  // A theme whose colours are light comes to a dark one by the same rule.
  const dark = lightFrom(LIGHTS['rose-pine']!);
  assert.ok(lightness(dark.ground) < 0.1 && unreadable(dark) === null, `light comes to dark: ${dark.ground}`);
});

test('a theme with no light version is worn the same in both, and one with one wears it where light is asked for', () => {
  const plain = { colours: STARTS['rose-pine']!, light: null };
  assert.equal(wornOf(plain, true), plain.colours);
  assert.equal(wornOf(plain, false), plain.colours);
  const both = { colours: STARTS['rose-pine']!, light: LIGHTS['rose-pine']! };
  assert.equal(wornOf(both, true), both.light);
  assert.equal(wornOf(both, false), both.colours);
  assert.equal(listOf(wornOf(both, true))['color-scheme'], 'light');
});


test('a theme’s colours come to what the daemon says of them, case for case', () => {
  for (const [colours, says] of cases.cases) assert.equal(unreadable(colours as never), says, JSON.stringify(colours));
  assert.deepEqual(THEMES.map((one) => one.id), cases.ships, 'the themes that ship are those the daemon lets be chosen');
});

const kept = (id: string, owner: string, name: string, offered: KeptTheme['offered'] = null): KeptTheme => ({ id, owner, owner_name: owner, name, colours: STARTS['rose-pine']!, light: null, made: 1, changed: 1, offered });

test('what a browser kept of made themes is sent to the account where it has none the same, and the rest are found by it', () => {
  const browser = [
    { id: 'own-dusk', name: ' Dusk ', says: '', colours: { ...STARTS['rose-pine']!, text: STARTS['rose-pine']!.text.toUpperCase() } },
    { id: 'own-dawn', name: 'Dawn', says: '', colours: STARTS['rose-pine']! },
    { id: 'own-long', name: 'x'.repeat(70), says: '', colours: STARTS['grid-iron']! },
  ];
  const { send, already } = takenUp(browser, [kept('thm_1', 'user:mira', 'Dusk'), kept('thm_2', 'user:mira', 'Dawn colours')]);
  assert.deepEqual(send.map((one) => one.id), ['own-dawn', 'own-long'], 'one that is the same by its name and its colours is not sent again');
  assert.deepEqual(already, { 'own-dusk': 'thm_1' });
  assert.equal(keptName(browser[2]!), 'x'.repeat(LONGEST));
  assert.deepEqual(takenUp([], [kept('thm_1', 'user:mira', 'Dusk')]), { send: [], already: {} });
});

test('the theme worn once the account’s are read is the account’s choice, else once what this browser chose', () => {
  const wearable = [kept('thm_1', 'user:mira', 'Dusk')];
  assert.deepEqual(chosenOnce('rose-pine', 'own-dusk', { 'own-dusk': 'thm_1' }, wearable), { theme: 'rose-pine', send: false }, 'the account’s stands over the browser’s');
  assert.deepEqual(chosenOnce('thm_gone', null, {}, wearable), { theme: DEFAULT_THEME, send: false }, 'one there is no more is the usual one');
  assert.deepEqual(chosenOnce(null, 'own-dusk', { 'own-dusk': 'thm_1' }, wearable), { theme: 'thm_1', send: true }, 'by the id the account keeps it by');
  assert.deepEqual(chosenOnce(null, 'tokyo-midnight', {}, wearable), { theme: 'tokyo-midnight', send: true });
  assert.deepEqual(chosenOnce(null, DEFAULT_THEME, {}, wearable), { theme: DEFAULT_THEME, send: false }, 'the usual one says nothing of a choice');
  assert.deepEqual(chosenOnce(null, 'own-gone', {}, wearable), { theme: DEFAULT_THEME, send: false });
  assert.deepEqual(chosenOnce(null, null, {}, wearable), { theme: DEFAULT_THEME, send: false });
});

test('what is said of themes comes to what the page has of them, for whose they are', () => {
  const none: Themed = { own: [], offered: [], chosen: null };
  const said = (topic: string, data: Record<string, unknown>) => ({ topic, data });
  const dusk = kept('thm_1', 'user:mira', 'Dusk');
  let mira = heardTheme(none, 'user:mira', said('theme.kept', { owner: 'user:mira', theme: dusk }));
  assert.deepEqual(mira.own.map((one) => one.id), ['thm_1']);
  mira = heardTheme(mira, 'user:mira', said('theme.kept', { owner: 'user:mira', theme: { ...dusk, name: 'Late dusk' } }));
  assert.deepEqual(mira.own.map((one) => one.name), ['Late dusk'], 'changed in its place');
  // One of the owner's, offered: in the offered of everybody else, and the owner's own with its mark.
  const harbour = kept('thm_2', 'user:ada', 'Harbour', { by: 'user:ada', name: 'Ada', at: 2 });
  mira = heardTheme(mira, 'user:mira', said('theme.offered', { owner: 'user:ada', theme: harbour, by: 'user:ada' }));
  assert.deepEqual(mira.offered.map((one) => one.id), ['thm_2']);
  let ada = heardTheme({ own: [{ ...harbour, offered: null }], offered: [], chosen: null }, 'user:ada', said('theme.offered', { owner: 'user:ada', theme: harbour }));
  assert.deepEqual([ada.own[0]!.offered?.name, ada.offered.length], ['Ada', 0], 'the owner’s own is not offered to them again');
  mira = heardTheme(mira, 'user:mira', said('theme.chosen', { owner: 'user:mira', theme: 'thm_2' }));
  assert.equal(mira.chosen, 'thm_2');
  assert.equal(heardTheme(mira, 'user:mira', said('theme.chosen', { owner: 'user:nell', theme: 'rose-pine' })).chosen, 'thm_2', 'somebody else’s choice is theirs');
  // Withdrawn: gone from everybody else's, chosen by them no longer; the owner's own still, unmarked.
  const withdrawn = said('theme.withdrawn', { owner: 'user:ada', theme: 'thm_2', by: 'user:ada' });
  mira = heardTheme(mira, 'user:mira', withdrawn);
  assert.deepEqual([mira.offered, mira.chosen], [[], null]);
  ada = heardTheme({ ...ada, chosen: 'thm_2' }, 'user:ada', withdrawn);
  assert.deepEqual([ada.own[0]!.offered, ada.chosen], [null, 'thm_2']);
  // Taken away, it is in nothing.
  mira = heardTheme(mira, 'user:mira', said('theme.gone', { owner: 'user:mira', theme: 'thm_1', offered: false }));
  assert.deepEqual(mira, none);
  assert.deepEqual(heardTheme(none, 'user:mira', said('inbox.told', { owner: 'user:mira' })), none);
});
