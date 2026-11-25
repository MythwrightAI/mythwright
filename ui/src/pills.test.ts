import assert from 'node:assert/strict';
import { test } from 'node:test';
import { add, asItems, begin, colour, COLOURS, making, matching, move, only, toggle, value } from './pills.ts';

const COLUMNS = ['backlog', 'ready', 'doing', 'review', 'done'];

test('a list of words or numbers can be tags; anything else cannot', () => {
  assert.deepEqual(asItems(COLUMNS), COLUMNS);
  assert.deepEqual(asItems([1, 2, true]), [1, 2, true]);
  assert.deepEqual(asItems([]), []);
  assert.equal(asItems([{ name: 'a' }]), null);
  assert.equal(asItems('backlog'), null);
  assert.equal(asItems(undefined), null);
});

test('what it starts with is chosen, and what else there is comes after it', () => {
  assert.deepEqual(begin(COLUMNS), { options: COLUMNS, chosen: COLUMNS });
  const store = begin(['files'], ['local', 'files']);
  assert.deepEqual(store.options, ['files', 'local']);
  assert.deepEqual(value(store), ['files']);
});

test('an option that is not chosen is kept, to be chosen again where it was', () => {
  const without = toggle(begin(COLUMNS), 'review');
  assert.deepEqual(value(without), ['backlog', 'ready', 'doing', 'done']);
  assert.deepEqual(without.options, COLUMNS, 'it is still there to choose');
  assert.deepEqual(value(toggle(without, 'review')), COLUMNS, 'and comes back where it was');
});

test('the order of what is chosen is the order of the options, which can be changed', () => {
  let picked = begin(COLUMNS);
  picked = move(picked, 1, 3);
  assert.deepEqual(value(picked), ['backlog', 'doing', 'review', 'ready', 'done']);
  picked = move(picked, 4, 0);
  assert.deepEqual(value(picked), ['done', 'backlog', 'doing', 'review', 'ready']);
  assert.deepEqual(move(picked, 2, 99).options.at(-1), 'doing', 'past the end is the end');
  assert.equal(move(picked, 7, 0), picked, 'nothing there to move');
  assert.equal(move(picked, 2, 2), picked);
});

test('what is typed makes an option after the others, once, and chooses it', () => {
  let picked = begin(COLUMNS);
  assert.equal(making(picked, '  blocked '), 'blocked');
  picked = add(picked, '  blocked ');
  assert.deepEqual(value(picked).at(-1), 'blocked');
  assert.equal(making(picked, 'blocked'), null, 'there is one already');
  assert.deepEqual(add(picked, 'blocked'), picked);
  assert.equal(making(picked, '   '), null);
  assert.deepEqual(add(picked, '   '), picked, 'nothing typed makes nothing');
});

test('typing an option that is not chosen chooses it', () => {
  const picked = add(toggle(begin(COLUMNS), 'review'), 'review');
  assert.deepEqual(value(picked), COLUMNS);
  assert.equal(picked.options.length, 5, 'and makes no second one');
});

test('a choice of one is one', () => {
  const store = only(begin(['local'], ['local', 'files']), 'files');
  assert.deepEqual(value(store), ['files']);
});

test('in a list of numbers a number typed is a number; in a list of words it is a word', () => {
  assert.deepEqual(value(add(begin([80, 443]), '8080')), [80, 443, 8080]);
  assert.deepEqual(value(add(begin([80, 443]), 'http')), [80, 443, 'http']);
  assert.deepEqual(value(add(begin(['a']), '8080')), ['a', '8080']);
  assert.deepEqual(value(add(begin([]), '8080')), ['8080'], 'with nothing to go by, it is what was typed');
});

test('typing narrows the options to those it is found in', () => {
  assert.deepEqual(matching(begin(COLUMNS), 'DO'), ['doing', 'done']);
  assert.deepEqual(matching(begin(COLUMNS), ''), COLUMNS);
  assert.deepEqual(matching(begin(COLUMNS), 'zzz'), []);
});

test('a tag is always the same colour for the same words', () => {
  assert.equal(colour('Public Domain'), colour('public domain'));
  for (const column of COLUMNS) assert.ok(colour(column) >= 1 && colour(column) < COLOURS, 'one of the eight, never the plain one');
  assert.ok(new Set(COLUMNS.map(colour)).size > 1, 'and not all tags are one colour');
  // It is worked out from the name and nothing else: written down, so that it is the same tomorrow.
  assert.deepEqual(['before-preview', 'in-progress', 'landed', 'soon'].map(colour), [6, 1, 3, 2]);
});

// Labels a board is likely to have.
const LABELS = ['before-preview', 'in-progress', 'landed', 'soon', 'bug', 'urgent', 'later', 'blocked', 'question', 'docs', 'client', 'runner', 'security', 'feature', 'chore', 'design', 'backend', 'frontend', 'p1', 'p2', 'wontfix', 'help wanted', 'good first issue', 'duplicate', 'enhancement', 'invalid', 'documentation'];

test('the colours are spread over the labels a board is likely to have', () => {
  const times = Array.from({ length: COLOURS }, () => 0);
  for (const label of LABELS) times[colour(label)]!++;
  assert.equal(times[0], 0, 'none is the plain one');
  assert.ok(times.slice(1).every((count) => count > 0), `every colour is some label's: ${times.slice(1)}`);
  assert.ok(Math.max(...times) <= Math.ceil(LABELS.length / (COLOURS - 1)) * 2, `and none is most labels': ${times.slice(1)}`);
  assert.equal(new Set(LABELS.slice(0, 4).map(colour)).size, 4, 'four labels of one board are four colours');
  assert.ok(new Set(LABELS.slice(0, 8).map(colour)).size >= 6);
});

test('a word on its tag is read in every theme, light and dark, and the tags are told apart', async () => {
  const { readFile } = await import('node:fs/promises');
  const { STARTS, listOf, mix } = await import('./themes.ts');
  const sheet = await readFile(new URL('./style.css', import.meta.url), 'utf8');
  const themes = await readFile(new URL('./themes.css', import.meta.url), 'utf8');
  // What the sheet says a tag's colour is: one of a theme's own, or two of them in equal parts.
  const said = [...sheet.matchAll(/^\.tag-(\d) \{ --tag: ([^;]+); \}/gm)].map((found) => found[2]!);
  assert.equal(said.length, COLOURS, 'a colour for each number a word can come to, and the plain one');
  assert.ok(!/#[0-9a-f]{3,6}/i.test(said.join()), 'none is written out: each is the theme\'s, so that every theme has them');
  const tint = Number(/\.tag \{[^}]*background: color-mix\(in srgb, var\(--tag[^)]*\)\) (\d+)%/.exec(sheet)?.[1]) / 100;
  assert.ok(tint > 0.1 && tint < 1, `how much of its colour a tag is tinted with is said in the sheet, and is enough to see: ${tint}`);
  const colourOf = (words: string, list: Record<string, string>): string => {
    const names = [...words.matchAll(/var\((--[a-z0-9-]+)\)/g)].map((found) => list[found[1]!]!);
    assert.ok(names.length > 0 && names.every(Boolean), `${words} names what a theme says`);
    return names.length === 1 ? names[0]! : mix(names[0]!, names[1]!, 0.5);
  };
  // How bright a colour is, and how far apart two are, as readers of screens measure it.
  const parts = (hex: string) => [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16));
  const bright = (hex: string) => {
    const [red, green, blue] = parts(hex).map((part) => (part / 255 <= 0.03928 ? part / 255 / 12.92 : ((part / 255 + 0.055) / 1.055) ** 2.4));
    return 0.2126 * red! + 0.7152 * green! + 0.0722 * blue!;
  };
  const apart = (a: string, b: string) => (Math.max(bright(a), bright(b)) + 0.05) / (Math.min(bright(a), bright(b)) + 0.05);
  const far = (a: string, b: string) => Math.hypot(...parts(a).map((part, at) => part - parts(b)[at]!));
  // Every theme there is: each built in, dark and light, as its sheet says it, and one made from eight colours.
  const light = themes.indexOf(':root[data-light]');
  assert.ok(light > 0);
  const lists: [string, Record<string, string>][] = [...themes.matchAll(/([^{}]*)\{([^{}]*--bg0:[^{}]*)\}/g)].map((found) => [
    `${/data-theme="([a-z-]+)"/.exec(found[1]!)?.[1]}, ${found.index! > light ? 'light' : 'dark'}`,
    Object.fromEntries([...found[2]!.matchAll(/(--[a-z0-9-]+):\s*(#[0-9a-fA-F]{6})\b/g)].map((one) => [one[1]!, one[2]!])),
  ]);
  assert.equal(lists.length, 2 * Object.keys(STARTS).length, 'each built-in theme, dark and light');
  // What marks a match where a text is searched is tinted the same way, in one colour of the theme's.
  const finder = await readFile(new URL('./finder.css', import.meta.url), 'utf8');
  const match = /\.cm-searchMatch \{ background: color-mix\(in srgb, (var\(--[a-z0-9-]+\)) (\d+)%, transparent\)/.exec(finder);
  assert.ok(match, 'a match is tinted with a colour a theme says, by a part the sheet says');
  assert.ok(Number(match[2]) / 100 <= tint, `a match is tinted no more than a tag is: ${match[2]}%`);
  assert.ok(!/#[0-9a-f]{3,6}\b|rgba?\(/i.test(finder.replace(/\/\*[\s\S]*?\*\//g, '')), 'and nothing of the bar is a colour written out');
  // The match the caret is at is also what is selected, and is tinted over the selection's own tint: the most of its accent any theme selects with.
  const selects = Math.max(...[...themes.matchAll(/--accent-soft: rgba\([^)]*, ([\d.]+)\)/g)].map((found) => Number(found[1])));
  assert.ok(selects > 0 && selects < 0.3, `a theme says how much of its accent what is selected is tinted with: ${selects}`);
  for (const [name, colours] of Object.entries(STARTS)) lists.push([`${name}, as one somebody made`, listOf(colours)]);
  for (const [name, list] of lists) {
    const tags = said.map((words) => colourOf(words, list));
    // A tag is drawn on a table's ground, on a menu and on a card.
    for (const under of ['--bg0', '--bg1', '--bg2']) {
      for (const [nth, tag] of tags.entries()) {
        const behind = mix(list[under]!, tag, tint);
        assert.ok(apart(list['--text']!, behind) >= 4.5, `tag ${nth} in ${name}: ${list['--text']} on ${behind} is ${apart(list['--text']!, behind).toFixed(2)}`);
      }
    }
    // A match is marked in a text, on the editor's ground and on the line the caret is in.
    for (const under of ['--bg0', '--bg1']) {
      const behind = mix(list[under]!, colourOf(match[1]!, list), Number(match[2]) / 100);
      assert.ok(apart(list['--text']!, behind) >= 4.5, `a match in ${name}: ${list['--text']} on ${behind} is ${apart(list['--text']!, behind).toFixed(2)}`);
      const chosen = mix(mix(list[under]!, list['--accent']!, selects), colourOf(match[1]!, list), Number(match[2]) / 100);
      assert.ok(apart(list['--text']!, chosen) >= 4.5, `the match the caret is at in ${name}: ${list['--text']} on ${chosen} is ${apart(list['--text']!, chosen).toFixed(2)}`);
    }
    // The eight a name comes to are eight colours: the two a theme does not say itself are mixed from those it does, and stay clear of them.
    for (const [nth, tag] of tags.entries()) {
      for (const [other, next] of tags.entries()) if (nth > 0 && other > nth) assert.ok(far(tag, next) >= 32, `tags ${nth} and ${other} in ${name} are near one colour: ${tag}, ${next}`);
    }
  }
});
