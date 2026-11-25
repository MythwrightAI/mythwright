import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { ABOUT_TABS, asked, atOf, facts, guideOfLink, guideShown, guidesOf, packageSaid, packagesOf, tabCalled } from './about.ts';

const guides = [
  { name: 'first-run', title: 'The first run' },
  { name: 'blueprints', title: 'Blueprints' },
];

test('the tabs are what the program is, its docs and what is technical of it, in that order', () => {
  assert.deepEqual([...ABOUT_TABS], ['about', 'docs', 'technical']);
  assert.equal(tabCalled('about', 'Factory'), 'What is Factory');
  assert.equal(tabCalled('docs', 'Factory'), 'Docs');
  assert.equal(tabCalled('technical', 'Factory'), 'Technical');
  // Before the name is read, the tab says what it is all the same.
  assert.equal(tabCalled('about', ''), 'What is this');
});

test('an address after its # asks for a tab, and under Docs a guide; anything else is the first tab', () => {
  assert.deepEqual(asked(''), { tab: 'about' });
  assert.deepEqual(asked(undefined), { tab: 'about' });
  assert.deepEqual(asked('docs'), { tab: 'docs' });
  assert.deepEqual(asked('docs/blueprints'), { tab: 'docs', guide: 'blueprints' });
  assert.deepEqual(asked('technical'), { tab: 'technical' });
  assert.deepEqual(asked('nonsense'), { tab: 'about' });
});

test('what the address says is the tab and the guide, and nothing for the first tab', () => {
  assert.equal(atOf('about'), '');
  assert.equal(atOf('docs'), 'docs');
  assert.equal(atOf('docs', 'views'), 'docs/views');
  assert.equal(atOf('technical', 'views'), 'technical');
  for (const at of ['', 'docs', 'docs/views', 'technical']) assert.equal(atOf(asked(at).tab, asked(at).guide), at);
});

test('the guide read is the one chosen where there is one, else the first', () => {
  assert.equal(guideShown(guides, 'blueprints')?.name, 'blueprints');
  assert.equal(guideShown(guides, 'gone')?.name, 'first-run');
  assert.equal(guideShown(guides, null)?.name, 'first-run');
  assert.equal(guideShown([], 'blueprints'), null);
});

test('a link in a guide leads to another guide listed, and nowhere else', () => {
  assert.equal(guideOfLink('blueprints.md', guides), 'blueprints');
  assert.equal(guideOfLink('./blueprints.md#a-heading', guides), 'blueprints');
  assert.equal(guideOfLink('architecture.md', guides), null);
  assert.equal(guideOfLink('../README.md', guides), null);
  assert.equal(guideOfLink('https://example.invalid/blueprints.md', guides), null);
});

test('a reading says only the guides and packages that are whole, and no packages where the reader was told none', () => {
  assert.deepEqual(guidesOf({ guides: [{ name: 'views', title: '' }, { name: '', title: 'x' }, null as never] }), [{ name: 'views', title: 'views' }]);
  assert.deepEqual(guidesOf(null), []);
  assert.equal(packagesOf({}), null);
  assert.deepEqual(packagesOf({ packages: [{ name: 'a', title: 'A', stands: 'came' }, { name: 'b', title: '', stands: 'own' }, { name: 'c', title: 'C', stands: 'what' as never }] }), [
    { name: 'a', title: 'A', stands: 'came' },
    { name: 'b', title: 'b', stands: 'own' },
  ]);
});

test('a package says whether it is the one that came', () => {
  assert.equal(packageSaid({ name: 'a', title: 'A', stands: 'came' }), 'The one that came with this version');
  assert.equal(packageSaid({ name: 'a', title: 'A', stands: 'differs' }), 'Differs from the one that came');
  assert.equal(packageSaid({ name: 'a', title: 'A', stands: 'own' }), 'Installed from its own manifest');
});

test('what is technical is said only where it is known, the data folder among it', () => {
  assert.deepEqual(facts({}), []);
  assert.deepEqual(
    facts({ version: '0.4.0', build: 'abc', runsFrom: 'installed', os: 'linux', arch: 'x86_64', profile: 'dev', data: '/srv/data' }).map((one) => one.label),
    ['Version', 'Client build', 'Runs from', 'System', 'Profile', 'Data folder'],
  );
  assert.deepEqual(facts({ version: '0.4.0', runsFrom: 'checkout' }), [
    { label: 'Version', said: '0.4.0' },
    { label: 'Runs from', said: 'the checkout it was built in' },
  ]);
});
