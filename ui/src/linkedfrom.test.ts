import assert from 'node:assert/strict';
import { test } from 'node:test';
import { gatheredLinks } from './linkedfrom.ts';
import type { Linking } from './linkedfrom.ts';

const page = (title: string, series?: string | null): Linking => ({ id: `@-topics-${title}`, title, wiki: null, ...(series === undefined ? {} : { series }) });

test('pages that link only through a series box are gathered by their series, and the rest listed as said', () => {
  const cases: [Linking[], string[], [string, string[]][]][] = [
    [[], [], []],
    [[page('A'), page('B', null)], ['A', 'B'], []],
    [[page('A', 'Meditation'), page('B'), page('C', 'Meditation'), page('D', 'Moving'), page('E', '')], ['B'], [['Meditation', ['A', 'C']], ['Moving', ['D']], ['', ['E']]]],
  ];
  for (const [linked, listed, series] of cases) {
    const said = gatheredLinks(linked);
    assert.deepEqual(said.listed.map((one) => one.title), listed);
    assert.deepEqual(said.series.map((one) => [one.title, one.pages.map((each) => each.title)]), series);
  }
});
