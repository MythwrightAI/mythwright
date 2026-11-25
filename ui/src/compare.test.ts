import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { Series, Stats } from './api.ts';
import { amount, asks, changed, compare, difference, failedChange, narrowedTo, narrowings, NOTHING, split, standing, unitOf } from './compare.ts';

const stats = (count: number, avg: number, errors = 0): Stats => ({ count, errors, error_rate: count ? errors / count : 0, avg, sum: avg * count, min: avg, max: avg, p50: avg, p95: avg });
// The daemon says what a series is of where its name does.
const series = (name: string, labels: Record<string, string>, total: Stats): Series => ({ name, labels, total, points: [], ...(name.endsWith('_usd') ? { unit: 'usd' } : {}) });

// One card given to two agents, as the daemon answers what `asks` asks:
// every measure split by `variant`, and steps and runs by their parts too.
const TRIAL: Series[] = [
  series('step', { variant: 'a' }, stats(6, 60_020)),
  series('step', { variant: 'b' }, stats(6, 90_020, 1)),
  series('step', {}, stats(40, 3_000)),
  series('run', { variant: 'a' }, stats(4, 90_875)),
  series('run', { variant: 'b' }, stats(3, 181_000, 1)),
  series('agent.turn', { variant: 'b' }, stats(3, 170_000)),
  series('agent.turn', { variant: 'a' }, stats(3, 110_000)),
  series('agent.turn', {}, stats(9, 5_000)),
  series('agent.cost_usd', { variant: 'a' }, stats(3, 0.042)),
  series('agent.cost_usd', { variant: 'b' }, stats(3, 0.021)),
  series('build', { variant: 'a' }, stats(3, 90_000)),
  series('build', { variant: 'b' }, stats(3, 60_000)),
  series('step', { blueprint: 'std/working', step: 'work', variant: 'a' }, stats(3, 120_000)),
  series('step', { blueprint: 'std/working', step: 'work', variant: 'b' }, stats(3, 180_000, 1)),
  series('step', { blueprint: 'std/working', step: 'claim', variant: 'a' }, stats(3, 40)),
  series('step', { blueprint: 'std/working', step: 'claim', variant: 'b' }, stats(3, 40)),
  series('step', { blueprint: 'trial', step: 'a' }, stats(1, 121_000)),
  series('run', { blueprint: 'std/working', flow: 'main', variant: 'a' }, stats(3, 121_000)),
  series('run', { blueprint: 'std/working', flow: 'main', variant: 'b' }, stats(3, 181_000, 1)),
  series('run', { blueprint: 'std/working', flow: 'on.done', variant: 'a' }, stats(1, 500)),
];

test('each measure is a row, and each value of the label a group of columns of it', () => {
  const { values, rows } = compare(TRIAL, 'variant');
  assert.deepEqual(values, ['a', 'b']);
  assert.deepEqual(
    rows.map((row) => [row.title, row.of, row.cells.map((cell) => cell?.count ?? null)]),
    [
      ['Agent turns', '', [3, 3]],
      ['Agent cost, per turn', '', [3, 3]],
      // Runs and steps as a whole, and then each flow and each step.
      ['Runs', '', [4, 3]],
      ['Runs', 'std/working · main', [3, 3]],
      // What only one of them did has nothing beside it.
      ['Runs', 'std/working · on.done', [1, null]],
      ['Steps', '', [6, 6]],
      ['Steps', 'std/working · work', [3, 3]],
      ['Steps', 'std/working · claim', [3, 3]],
      // What a blueprint named itself comes after what everything has.
      ['build', '', [3, 3]],
    ],
  );
});

test('a measure with a great many parts shows those that took the most time', () => {
  const steps = Array.from({ length: 30 }, (_, index) => series('step', { blueprint: 'big', step: `s${index}`, variant: 'a' }, stats(1, index)));
  const { rows } = compare([series('step', { variant: 'a' }, stats(30, 14.5)), ...steps], 'variant');
  assert.equal(rows.length, 21, 'the whole, and twenty of its parts');
  assert.deepEqual([rows[0]!.of, rows[1]!.of, rows[20]!.of], ['', 'big · s29', 'big · s10']);
});

test('what does not carry the label is not in the comparison', () => {
  const { rows } = compare(TRIAL, 'variant');
  assert.ok(!rows.some((row) => row.of.startsWith('trial')), 'the step that called the lanes is under neither');
  assert.equal(rows[0]!.cells[0]!.count + rows[0]!.cells[1]!.count, 6, 'nor are the turns taken for nobody');
  assert.deepEqual(compare(TRIAL, 'experiment'), { values: [], rows: [] });
});

test('with exactly two values, a row says how the second differs from the first', () => {
  const { rows } = compare(TRIAL, 'variant');
  const turns = rows[0]!;
  assert.deepEqual(turns.difference, { by: 60_000, ratio: 60_000 / 110_000, failed: 0 });
  assert.equal(changed(turns.difference!, turns.unit), '+1m 00s (+55%)');
  assert.equal(failedChange(turns.difference!), null);

  const work = rows.find((row) => row.of.endsWith('work'))!;
  assert.equal(changed(work.difference!, work.unit), '+1m 00s (+50%)');
  assert.equal(failedChange(work.difference!), '+33 points failed');

  const named = rows.find((row) => row.name === 'build')!;
  assert.equal(changed(named.difference!, named.unit), '−30s (−33%)');
  const claim = rows.find((row) => row.of.endsWith('claim'))!;
  assert.equal(changed(claim.difference!, claim.unit), 'the same');
  assert.equal(rows.find((row) => row.of.endsWith('on.done'))!.difference, null, 'one side alone has nothing to differ from');
});

test('with one value or more than two there is no difference to say', () => {
  const three = [...TRIAL, series('agent.turn', { variant: 'c' }, stats(1, 1_000))];
  const compared = compare(three, 'variant');
  assert.deepEqual(compared.values, ['a', 'b', 'c']);
  assert.ok(compared.rows.every((row) => row.difference === null));
  assert.deepEqual(compared.rows[0]!.cells.map((cell) => cell?.count), [3, 3, 1]);

  const one = compare(TRIAL.filter((each) => each.labels.variant !== 'b'), 'variant');
  assert.deepEqual(one.values, ['a']);
  assert.ok(one.rows.every((row) => row.difference === null));
});

test('values are in the order a person would put them', () => {
  const rounds = ['10', '9', '1'].map((round) => series('run', { round }, stats(1, 1)));
  assert.deepEqual(compare(rounds, 'round').values, ['1', '9', '10']);
});

test('a cost is shown as money and everything else as time', () => {
  assert.equal(unitOf({ unit: 'usd' }), 'usd');
  assert.equal(unitOf({}), 'time', 'a series that says nothing is a length of time');
  assert.equal(amount(0.042, 'usd'), '$0.042');
  assert.equal(amount(1.5, 'usd'), '$1.50');
  assert.equal(amount(86_000, 'tokens'), '86,000');
  assert.equal(amount(4_200, 'time'), '4.2s');
  const cost = compare(TRIAL, 'variant').rows[1]!;
  assert.equal(changed(cost.difference!, cost.unit), '−$0.021 (−50%)');
});

test('a difference from nothing has no share to be said as', () => {
  const from = difference(stats(2, 0), stats(2, 300));
  assert.equal(from.ratio, null);
  assert.equal(changed(from, 'time'), '+300ms');
  assert.equal(failedChange(difference(stats(4, 10, 2), stats(4, 10, 1))), '−25 points failed');
});

test('a comparison can be narrowed to one value of another label', () => {
  const labels = { variant: ['b', 'a'], experiment: ['cart-12', 'cart-7'], round: ['2'] };
  assert.deepEqual(narrowings(labels, 'variant'), [
    { label: 'experiment', values: ['cart-7', 'cart-12'] },
    { label: 'round', values: ['2'] },
  ]);
  assert.deepEqual(split('experiment:cart-12'), ['experiment', 'cart-12']);
  assert.deepEqual(split('experiment:a:b'), ['experiment', 'a:b'], 'a value may have a colon in it');
  assert.deepEqual(split(''), ['', '']);
});

test('the daemon is asked for every measure by the label, for steps and runs part by part, and only for what is wanted', () => {
  assert.deepEqual(asks('variant'), [
    { name: '*', by: 'variant' },
    { name: 'step', by: 'variant,blueprint,step' },
    { name: 'run', by: 'variant,blueprint,flow' },
  ]);
  assert.equal(narrowedTo(null, ''), undefined);
  assert.equal(narrowedTo(['shop'], ''), 'project:shop');
  assert.equal(narrowedTo(null, 'experiment:cart-12'), 'experiment:cart-12');
  assert.equal(narrowedTo(['shop'], 'experiment:cart-12'), 'project:shop,experiment:cart-12');
  assert.equal(narrowedTo(['shop', 'blog'], 'experiment:cart-12'), 'project:shop,project:blog,experiment:cart-12', 'several projects are each said');
});

test('a choice stands only while what it names is there', () => {
  const labels = { variant: ['a', 'b'], experiment: ['cart-12'] };
  const choice = { by: 'variant', only: 'experiment:cart-12' };
  assert.deepEqual(standing(labels, choice), choice);
  assert.deepEqual(standing(labels, { by: 'variant', only: 'experiment:cart-13' }), { by: 'variant', only: '' }, 'a value that is gone narrows nothing');
  assert.deepEqual(standing(labels, { by: 'variant', only: 'variant:a' }), { by: 'variant', only: '' }, 'it is not narrowed to what it is compared by');
  assert.deepEqual(standing({}, choice), NOTHING, 'with no labels nothing is compared');
  assert.deepEqual(standing(labels, { by: 'model', only: '' }), NOTHING);
});
