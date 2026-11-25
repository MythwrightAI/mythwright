import assert from 'node:assert/strict';
import { test } from 'node:test';
import { OWN, banded, leftOut } from './agentbands.ts';

const agent = (name: string, project: string | null, active = true) => ({ name, project, active });
const PROJECTS = [
  { id: 'shop', name: 'Shop' },
  { id: 'atlas', name: 'Atlas' },
  { id: 'empty', name: 'Empty' },
];
const said = (bands: ReturnType<typeof banded<ReturnType<typeof agent>>>) => bands.map((band) => `${band.name}: ${band.agents.map((one) => one.name).join(' ')}`);

test('with no agent there is no band', () => {
  assert.deepEqual(banded([], PROJECTS), []);
  assert.deepEqual(banded([], []), []);
});

test('one band alone is a band all the same, a project\'s or the factory\'s own', () => {
  const shop = banded([agent('helper', 'shop'), agent('maker', 'shop')], PROJECTS);
  assert.deepEqual(said(shop), ['Shop: helper maker']);
  assert.equal(shop[0]!.project, 'shop');
  const own = banded([agent('loner', null)], PROJECTS);
  assert.deepEqual(said(own), [`${OWN}: loner`]);
  assert.equal(own[0]!.project, null);
});

test('several bands are in the order the projects are listed, with the factory\'s own last and no band for a project with nobody', () => {
  const bands = banded([agent('loner', null), agent('cartographer', 'atlas'), agent('helper', 'shop'), agent('scribe', null), agent('maker', 'shop')], PROJECTS);
  assert.deepEqual(said(bands), ['Shop: helper maker', 'Atlas: cartographer', `${OWN}: loner scribe`]);
  // The order is the projects' and never the names': listed the other way round, the bands are too.
  assert.deepEqual(said(banded([agent('cartographer', 'atlas'), agent('helper', 'shop')], [...PROJECTS].reverse())), ['Atlas: cartographer', 'Shop: helper']);
});

test('an agent whose project the page has not read is listed under the project\'s id, after the projects it has and before the factory\'s own', () => {
  const bands = banded([agent('loner', null), agent('newcomer', 'zeta'), agent('early', 'beta'), agent('helper', 'shop')], PROJECTS);
  assert.deepEqual(said(bands), ['Shop: helper', 'beta: early', 'zeta: newcomer', `${OWN}: loner`]);
  assert.deepEqual(bands.map((band) => band.project), ['shop', 'beta', 'zeta', null]);
  // With no project read at all, every band is called by its id.
  assert.deepEqual(said(banded([agent('helper', 'shop')], [])), ['shop: helper']);
});

test('in a band those that are stopped come last, and each is by its name with its number read as one', () => {
  const bands = banded(
    [agent('shop-worker-10', 'shop'), agent('shop-worker-2', 'shop'), agent('architect', 'shop', false), agent('Helper', 'shop'), agent('zed', 'shop', false), agent('old', null, false), agent('new', null)],
    PROJECTS,
  );
  assert.deepEqual(said(bands), ['Shop: Helper shop-worker-2 shop-worker-10 architect zed', `${OWN}: new old`]);
});

test('an agent that says an empty project belongs to none, and what was handed in is left as it was', () => {
  const agents = [agent('b', ''), agent('a', null)];
  assert.deepEqual(said(banded(agents, PROJECTS)), [`${OWN}: a b`]);
  assert.deepEqual(agents.map((one) => one.name), ['b', 'a']);
});

// ---- held to the projects the Agents tab shows (`narrowed.ts`): a workspace's, one of them, or everything

const EVERY = [agent('loner', null), agent('cartographer', 'atlas'), agent('helper', 'shop'), agent('newcomer', 'zeta'), agent('maker', 'shop')];

test('held to some projects, only theirs are banded, in the order the projects are listed, and the factory\'s own are not among them', () => {
  assert.deepEqual(said(banded(EVERY, PROJECTS, ['shop', 'atlas'])), ['Shop: helper maker', 'Atlas: cartographer']);
  assert.deepEqual(said(banded(EVERY, PROJECTS, ['atlas'])), ['Atlas: cartographer']);
  // A project of the workspace that the page has not read is still its: its agents are under its id.
  assert.deepEqual(said(banded(EVERY, PROJECTS, ['zeta', 'shop'])), ['Shop: helper maker', 'zeta: newcomer']);
});

test('held to everything, every band is there and the factory\'s own last, as with nothing said', () => {
  assert.deepEqual(said(banded(EVERY, PROJECTS, null)), said(banded(EVERY, PROJECTS)));
  assert.deepEqual(said(banded(EVERY, PROJECTS, null)), ['Shop: helper maker', 'Atlas: cartographer', 'zeta: newcomer', `${OWN}: loner`]);
});

test('held to no project (a workspace with none), there is no band at all, the factory\'s own included', () => {
  assert.deepEqual(banded(EVERY, PROJECTS, []), []);
});

test('how many are left out by what is held to: nothing where everything is shown, and the factory\'s own counted with another workspace\'s', () => {
  assert.equal(leftOut(EVERY, null), 0);
  assert.equal(leftOut(EVERY, ['shop', 'atlas']), 2);
  assert.equal(leftOut(EVERY, ['shop']), 3);
  assert.equal(leftOut(EVERY, []), 5);
  assert.equal(leftOut([agent('b', ''), agent('helper', 'shop')], ['shop']), 1);
});
