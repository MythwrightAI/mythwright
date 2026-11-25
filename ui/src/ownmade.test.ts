import assert from 'node:assert/strict';
import { test } from 'node:test';
import { blueprintNamed, firstBlueprint, linkCalled } from './ownmade.ts';

const has = ['deploy', 'tools/ship'];
const ships = ['std/board', 'std/fleet'];

test('a name makes a blueprint where the project writes its own, with its folders, as anything calls it', () => {
  assert.deepEqual(blueprintNamed('release', '@blueprints', has, ships), { name: 'release', path: '@blueprints/release.bp' });
  assert.deepEqual(blueprintNamed(' tools/build ', '@blueprints', has, ships), { name: 'tools/build', path: '@blueprints/tools/build.bp' });
  assert.deepEqual(blueprintNamed('release.bp', '@blueprints', has, ships), { name: 'release', path: '@blueprints/release.bp' });
});

test('a name that is no name, or one the project or the factory has, makes nothing and says why', () => {
  const wrong = (name: string) => {
    const made = blueprintNamed(name, '@blueprints', has, ships);
    return 'wrong' in made ? made.wrong : null;
  };
  assert.equal(wrong('  '), 'Give it a name.');
  assert.match(wrong('ship it') ?? '', /no spaces/);
  assert.match(wrong('tools\\ship') ?? '', /written with \//);
  assert.match(wrong('tools//ship') ?? '', /Each part/);
  assert.match(wrong('.hidden') ?? '', /Each part/);
  assert.match(wrong('tools/../ship') ?? '', /Each part/);
  assert.match(wrong('@blueprints') ?? '', /begins with a letter/);
  assert.equal(wrong('tools/ship'), 'The project has a blueprint called tools/ship already.');
  assert.match(wrong('std/board') ?? '', /ships with the factory: it is forked from its own page/);
});

test('a new blueprint checks as it is made', () => {
  assert.equal(firstBlueprint('tools/build'), 'blueprint: tools/build\nsteps:\n  first: { do: noop }\n');
});

test('a file a blueprint links is called by its path from the blueprint folder', () => {
  assert.equal(linkCalled('reach/round', '@blueprints/reach/prompts/ask.md'), 'prompts/ask.md');
  assert.equal(linkCalled('tell', '@blueprints/prompts/ask.md'), 'prompts/ask.md');
  assert.equal(linkCalled('reach/round', '@blueprints/other/prompts/ask.md'), 'other/prompts/ask.md');
});
