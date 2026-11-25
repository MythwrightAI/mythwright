import assert from 'node:assert/strict';
import { test } from 'node:test';
import { knownBy, modelSaid, readable, reportedNow } from './modelsaid.ts';

const LISTS = { reasoner: [{ id: 'maker-big-5-5', label: 'Big 5.5' }, { id: 'quick', label: '' }] };

test('a model is said by the name its provider gives it wherever that is known, and by its id only where none is', () => {
  const told = { adapter: 'reasoner', model: 'maker-big-5-5', model_name: 'Big 5.5' };
  const left = { adapter: 'reasoner', model: null, model_name: 'maker-big-5-5' };
  const cases: [string, Parameters<typeof modelSaid>, string | null][] = [
    // Two messages met in use: the answer, whose model is the id its program reported, and what the program posted itself, which says none.
    ['the id the slot was told, by the name the slot gives it', ['maker-big-5-5', told, LISTS, 'Big 5.5'], 'Big 5.5'],
    ['no model said, by what the agent runs on now', [null, told, LISTS, 'Big 5.5'], 'Big 5.5'],
    ['an id the slot was not told, by what its provider lists it as', ['maker-big-5-5', left, LISTS, 'maker-big-5-5'], 'Big 5.5'],
    ['an id the slot was told, named by nobody, by the list', ['maker-big-5-5', { adapter: 'reasoner', model: 'maker-big-5-5' }, LISTS, null], 'Big 5.5'],
    // Told an alias (`big`), its program says which model that is by an id no list holds: the slot's name for what it was told.
    ['the id its program said it used, by the name of what it was told', ['maker-big-5-5', { adapter: 'reasoner', model: 'big', model_name: 'Big 5.5', model_reported: 'maker-big-5-5' }, {}, 'Big 5.5'], 'Big 5.5'],
    ['an id its program used before, in a readable form of itself', ['maker-big-5', { adapter: 'reasoner', model: 'big', model_name: 'Big 5.5', model_reported: 'maker-big-5-5' }, {}, 'Big 5.5'], 'Maker Big 5'],
    ['left to its provider, the id it said, by the list where the slot names it only by that id', ['maker-big-5-5', { ...left, model_reported: 'maker-big-5-5' }, LISTS, 'maker-big-5-5'], 'Big 5.5'],
    ['an id listed with no name, as it is', ['quick', left, LISTS, null], 'quick'],
    ['an id nothing names, in a readable form of itself', ['maker-small-5', told, LISTS, 'Big 5.5'], 'Maker Small 5'],
    ['an id somebody chose that nothing names, as they wrote it', ['maker-small-5', { adapter: 'reasoner', model: 'maker-small-5', model_name: 'maker-small-5' }, LISTS, 'maker-small-5'], 'maker-small-5'],
    ['a provider never asked, in a readable form of itself', ['maker-big-5-5', { adapter: 'other' }, LISTS, null], 'Maker Big 5.5'],
    ['an agent that is gone, in a readable form of itself', ['maker-big-5-5', undefined, LISTS, null], 'Maker Big 5.5'],
    // A case met in use: left to its provider, whose list is of what its picker offers by short names, a turn reports the whole id.
    ['left to its provider, the whole id its program reported and no list holds, readable', ['maker-fern-5-2', { adapter: 'reasoner', model: null, model_name: 'maker-fern-5-2', model_reported: 'maker-fern-5-2' }, LISTS, null], 'Maker Fern 5.2'],
    ['nothing said of a built-in assistant, nothing', [undefined, { adapter: 'builtin' }, LISTS, null], null],
  ];
  for (const [what, args, said] of cases) assert.equal(modelSaid(...args), said, what);
});

test('two messages of one model, said once by its id and once not at all, are said alike', () => {
  const slot = { adapter: 'reasoner', model: 'maker-big-5-5', model_name: 'Big 5.5' };
  assert.equal(modelSaid('maker-big-5-5', slot, LISTS, 'Big 5.5'), modelSaid(null, slot, LISTS, 'Big 5.5'));
});

test('a name is known for an id the slot names or a list holds, and its provider is asked for the rest', () => {
  const slot = { adapter: 'reasoner', model: 'big', model_name: 'Big 5.5' };
  assert.equal(knownBy('big', slot, LISTS), true);
  assert.equal(knownBy('maker-big-5-5', slot, {}), false);
  assert.equal(knownBy('maker-big-5-5', slot, LISTS), true);
  assert.equal(knownBy('big', { adapter: 'reasoner', model: 'big' }, {}), false);
  assert.equal(knownBy('maker-big-5-5', { ...slot, model_reported: 'maker-big-5-5' }, {}), true);
});

test('an id is said in a readable form of itself only where it is written as an id', () => {
  const cases: [string, string][] = [
    ['maker-big-5-5', 'Maker Big 5.5'],
    ['maker-fern-5-2', 'Maker Fern 5.2'],
    ['maker-3-5-quick-20241022', 'Maker 3.5 Quick 20241022'],
    ['maker-5.1-long', 'Maker 5.1 Long'],
    ['quick', 'quick'],
    ['Big 5.5', 'Big 5.5'],
    ['default', 'default'],
    ['Maker-Big', 'Maker-Big'],
    ['path/to-model', 'path/to-model'],
  ];
  for (const [id, said] of cases) assert.equal(readable(id), said, id);
});

test('an id its program is heard to have said is known at once, and the answer that names it said by the name of what it was told', () => {
  const told = { name: 'maker', adapter: 'reasoner', model: 'otter', model_name: 'Otter 5.5' };
  const other = { name: 'other', adapter: 'reasoner', model: null };
  // A case met in use: before the agents are read again, the answer was said by its id made readable.
  assert.equal(modelSaid('stand-otter-5-5', told, {}, 'Otter 5.5'), 'Stand Otter 5.5');
  const [heard, untouched] = reportedNow([told, other], 'maker', 'stand-otter-5-5');
  assert.equal(modelSaid('stand-otter-5-5', heard, {}, 'Otter 5.5'), 'Otter 5.5');
  assert.equal(untouched, other);
  // An event that says no model, or of nobody here, changes nothing.
  const agents = [told, other];
  assert.equal(reportedNow(agents, 'maker', undefined), agents);
  assert.deepEqual(reportedNow(agents, 'gone', 'stand-otter-5-5'), agents);
});
