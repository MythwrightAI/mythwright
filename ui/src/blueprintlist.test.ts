import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { blueprintFolds, counted, familyOf, foldOf, foldsGone, linesOf, matches, partOf, parts, sought, tops, tree, unreached, wordsOf } from './blueprintlist.ts';
import type { Listed, Tree } from './blueprintlist.ts';

interface Case {
  says: string;
  runs: Record<string, string[]>;
  applied: string[];
  tops: string[];
  parts: Record<string, string[]>;
  part_of: Record<string, string[]>;
  unreached: string[];
}

/** The table the daemon's own functions are held to as well (`blueprint::names`, the suite of `blueprint`). */
const CASES: Case[] = JSON.parse(readFileSync(new URL('./blueprintparts.json', import.meta.url), 'utf8'));

test('what is applied and what is a part of what come to what the daemon\'s functions come to', () => {
  assert.ok(CASES.length >= 10, 'the table was found');
  for (const one of CASES) {
    const all: Listed[] = Object.entries(one.runs).map(([name, runs]) => ({ name, runs, applied: one.applied.includes(name) || undefined }));
    assert.deepEqual(tops(all), one.tops, `${one.says}: what is applied`);
    for (const [name, expected] of Object.entries(one.parts)) assert.deepEqual(parts(name, all), expected, `${one.says}: the parts of ${name}`);
    for (const [name, expected] of Object.entries(one.part_of)) assert.deepEqual(partOf(name, all), expected, `${one.says}: what ${name} is a part of`);
    assert.deepEqual(unreached(all), one.unreached, `${one.says}: what is in no list`);
  }
});

/** A factory's list: a fleet with parts two levels down, a board, a slot that is applied and that two run, and a package's two. */
const ALL: Listed[] = [
  { name: 'std/fleet', description: 'A team of agents working the cards of a board.', runs: ['std/arrangement', 'std/pool'] },
  { name: 'std/arrangement', description: 'Brings a fleet to what its tables say.', runs: ['std/role', 'std/working'] },
  { name: 'std/role', description: 'Brings a role up: so many agents.', runs: [] },
  { name: 'std/working', description: 'Works the cards of a column.', runs: ['std/lane-round'] },
  { name: 'std/lane-round', description: 'One round of work on a card.', runs: [] },
  { name: 'std/pool', description: 'Says how many go at once.', runs: [], applied: true },
  { name: 'std/board', description: 'Gives a project its board.', runs: ['std/cards'] },
  { name: 'std/cards', description: 'What a card does beyond being moved.', runs: [] },
  { name: 'std/slot', description: 'Host a service behind a stable address.', runs: ['std/canary-stage'], applied: true },
  { name: 'std/canary-stage', description: 'Watch a new version for a while.', runs: [] },
  { name: 'std/store', description: 'Host a database as a store service.', runs: ['std/slot'] },
  { name: 'std/watch', description: null, runs: [] },
  { name: 'pack/serve', description: 'Serve a thing.', runs: ['pack/inner', 'std/slot'] },
  { name: 'pack/inner', description: 'Works out how.', runs: [] },
  { name: 'alone', description: 'Of no family.', runs: [] },
];

/** A tree as its names: each family with what is applied, and under each its parts. */
function drawn(listed: Tree): Record<string, Record<string, string[]>> {
  const of = (lines: Tree['loose']) => Object.fromEntries(lines.map((line) => [line.one.name, line.parts.map((part) => part.name)]));
  const families = Object.fromEntries(
    listed.families.flatMap((family) => [
      [family.family, of(family.tops)] as const,
      ...(family.misc.length > 0 ? [[`${family.family} misc`, of(family.misc)] as const] : []),
    ]),
  );
  return listed.loose.length > 0 ? { ...families, '': of(listed.loose) } : families;
}

test('the list is what somebody applies, by family with the standard ones first, and under each its parts all the way down', () => {
  const listed = tree(ALL);
  assert.deepEqual(listed.families.map((family) => family.family), ['std', 'pack']);
  assert.deepEqual(drawn(listed), {
    std: {
      'std/board': ['std/cards'],
      'std/fleet': ['std/arrangement', 'std/lane-round', 'std/pool', 'std/role', 'std/working'],
      'std/store': ['std/canary-stage', 'std/slot'],
      'std/watch': [],
    },
    // What is applied though something runs it too is in a group of its own at the end of its family, and under what runs it as well.
    'std misc': { 'std/pool': [], 'std/slot': ['std/canary-stage'] },
    // What a package's blueprint runs of the standard ones is under it, in its family.
    pack: { 'pack/serve': ['pack/inner', 'std/canary-stage', 'std/slot'] },
    '': { alone: [] },
  });
  // No part is at the top, and every blueprint is somewhere.
  const lines = new Set(Object.values(drawn(listed)).flatMap((family) => [...Object.keys(family), ...Object.values(family).flat()]));
  assert.deepEqual([...lines].sort(), ALL.map((one) => one.name).sort());
  const said = Object.fromEntries(listed.families.flatMap((family) => [...family.tops, ...family.misc]).map((line) => [line.one.name, counted(line)]));
  assert.deepEqual(said, { 'std/board': '1 part', 'std/fleet': '5 parts', 'std/pool': '', 'std/slot': '1 part', 'std/store': '2 parts', 'std/watch': '', 'pack/serve': '3 parts' });
  assert.equal(linesOf(listed), 20);
});

test('a ring that nothing names is listed with what is applied, so nothing is hidden', () => {
  const ring: Listed[] = [
    { name: 'std/a', runs: [] },
    { name: 'std/x', runs: ['std/y'] },
    { name: 'std/y', runs: ['std/x'] },
  ];
  assert.deepEqual(drawn(tree(ring)), { std: { 'std/a': [], 'std/x': ['std/y'], 'std/y': ['std/x'] } });
});

test('a daemon from before says nothing of what runs what, and the list is every blueprint as it was', () => {
  const before = ALL.map(({ name, description }) => ({ name, description }));
  const listed = tree(before);
  assert.deepEqual(Object.keys(drawn(listed).std ?? {}), before.map((one) => one.name).filter((name) => name.startsWith('std/')).sort());
  assert.ok(listed.families.flatMap((family) => family.tops).every((line) => line.of === 0 && counted(line) === ''));
});

test('what is typed is looked for in a name and in what a blueprint says it is, every word of it', () => {
  assert.deepEqual(wordsOf('  Lane   ROUND '), ['lane', 'round']);
  assert.deepEqual(wordsOf('   '), []);
  const round = ALL.find((one) => one.name === 'std/lane-round')!;
  for (const typed of ['lane', 'LANE-round', 'round card', 'std/', 'one round of work']) assert.ok(matches(round, wordsOf(typed)), typed);
  for (const typed of ['board', 'round board', 'lanes']) assert.ok(!matches(round, wordsOf(typed)), typed);
  // One that says nothing of itself is found by its name alone.
  assert.ok(matches(ALL.find((one) => one.name === 'std/watch')!, ['watch']));
});

test('a part that matches is listed under what it is a part of, and nothing else of that one is', () => {
  // Only a part matches: its parent is listed for it, with that part alone under it.
  assert.deepEqual(drawn(sought(ALL, 'lane-round')), { std: { 'std/fleet': ['std/lane-round'] } });
  const fleet = sought(ALL, 'lane-round').families[0]!.tops[0]!;
  assert.deepEqual([fleet.of, counted(fleet)], [5, '1 of 5 parts']);
});

test('a parent that matches with no part that does is listed alone', () => {
  assert.deepEqual(drawn(sought(ALL, 'team of agents')), { std: { 'std/fleet': [] } });
  const fleet = sought(ALL, 'team of agents').families[0]!.tops[0]!;
  // It still has its parts: none of them is what was looked for.
  assert.deepEqual([fleet.of, fleet.parts.length], [5, 0]);
  // A parent and some of its parts: the parent, with those.
  assert.deepEqual(drawn(sought(ALL, 'fleet')), { std: { 'std/fleet': ['std/arrangement'] } });
});

test('a part of two parents is listed under each, and under itself where it is applied too', () => {
  assert.deepEqual(drawn(sought(ALL, 'canary')), {
    std: { 'std/store': ['std/canary-stage'] },
    'std misc': { 'std/slot': ['std/canary-stage'] },
    pack: { 'pack/serve': ['std/canary-stage'] },
  });
  assert.deepEqual(drawn(sought(ALL, 'stable address')), {
    std: { 'std/store': ['std/slot'] },
    'std misc': { 'std/slot': [] },
    pack: { 'pack/serve': ['std/slot'] },
  });
  assert.equal(linesOf(sought(ALL, 'canary')), 6);
});

test('what matches nothing lists nothing, and nothing typed is the whole list', () => {
  assert.deepEqual(sought(ALL, 'no such thing'), { families: [], loose: [] });
  assert.deepEqual(sought(ALL, ''), tree(ALL));
  assert.deepEqual(sought(ALL, '   '), tree(ALL));
  assert.deepEqual(drawn(sought(ALL, 'family')), { '': { alone: [] } });
});

test('a family is what a name says before its first stroke', () => {
  assert.deepEqual(['std/fleet', 'pack/a/b', 'alone', ''].map(familyOf), ['std', 'pack', '', '']);
});

test('the fold of parts that fold nothing any more is taken out of what a browser kept', () => {
  const kept = [foldOf('std/fleet'), foldOf('std/watch'), foldOf('gone/one'), foldOf('std/cards'), 'blueprints:std', 'project:shop', 'parts'];
  // The fleet still has parts; one with none, one that is gone and one that is a part itself fold nothing.
  assert.deepEqual(foldsGone(kept, ALL), [foldOf('std/watch'), foldOf('gone/one'), foldOf('std/cards')]);
  assert.deepEqual(foldsGone(['blueprints:std', 'project:shop:blueprints'], ALL), []);
  assert.equal(foldOf('std/fleet'), 'parts:std/fleet');
});

test('every fold of the factory\'s blueprints: each family, its misc, and the parts of each blueprint that has any', () => {
  const folds = blueprintFolds(ALL);
  for (const fold of ['blueprints:std', foldOf('std/fleet'), foldOf('std/board')]) assert.ok(folds.includes(fold), fold);
  // A blueprint with no part folds nothing, and no fold is said twice.
  assert.ok(!folds.includes(foldOf('std/role')) && !folds.includes(foldOf('std/cards')));
  assert.equal(new Set(folds).size, folds.length);
  // What a browser kept of these, all of them, is nothing that folds nothing.
  assert.deepEqual(foldsGone(folds, ALL), []);
  assert.deepEqual(blueprintFolds([]), []);
});
