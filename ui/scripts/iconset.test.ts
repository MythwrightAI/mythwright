import assert from 'node:assert/strict';
import { test } from 'node:test';
import { built, moduleOf, reserved, spells } from './iconset.ts';
import type { IconNode } from './iconset.ts';

// Made-up names throughout: what the application may not spell is read
// from the packages when the client is built, and is written nowhere here.
const line: IconNode[] = [['path', { d: 'M1 1h2' }]];

test('the names that may not be spelled are the packages, their programs and what is kept for later', () => {
  const manifest = 'extension: Acme\nagent:\n  program: "acme-cli"\n';
  const other = "# what it is\nextension: 'zed'\nprogram: zd\n";
  const kept = '# Names kept for packages not written yet.\n\nforge\n  Depot  \n';
  assert.deepEqual(reserved(['acme', 'zed'], [manifest, other], kept), ['acme', 'acme-cli', 'depot', 'forge', 'zd', 'zed']);
  assert.deepEqual(reserved([], [], ''), []);
});

test('a long name is spelled wherever it is, and a short one only as a word of its own', () => {
  assert.equal(spells('acme', 'acme'), true);
  assert.equal(spells('square-acme-logo', 'acme'), true);
  assert.equal(spells('acmeish', 'acme'), true);
  assert.equal(spells('lighthouse', 'zd'), false);
  assert.equal(spells('ozdoba', 'zd'), false);
  assert.equal(spells('run zd now', 'zd'), true);
  assert.equal(spells('zd', 'zd'), true);
  assert.equal(spells('ozd, and zd', 'zd'), true);
});

test('an icon that is such a name, or is found by one, is left out of what is built', () => {
  const nodes = { star: line, acme: line, 'acme-square': line, cat: line, box: line };
  const tags = { star: ['Favourite', 'rating'], acme: ['logo'], cat: ['animal', 'Acme mascot'], box: ['zd'] };
  const { icons, left } = built(nodes, tags, ['acme', 'zd']);
  assert.deepEqual(Object.keys(icons), ['star']);
  assert.deepEqual(left, ['acme', 'acme-square', 'box', 'cat']);
  // What is built is the drawing and the words it is found by, in lowercase.
  assert.deepEqual(icons.star, [line, 'favourite,rating']);
  // With nothing to leave out, all of it is built; an icon nothing is said of is found by its name alone.
  const all = built(nodes, {}, []);
  assert.deepEqual(Object.keys(all.icons), ['acme', 'acme-square', 'box', 'cat', 'star']);
  assert.deepEqual(all.icons.box, [line, '']);
});

test('the module carries the notice where a build keeps it', () => {
  const text = moduleOf({ star: [line, 'rating'] }, 'A licence.\nCopyright somebody */ else');
  assert.ok(text.startsWith('/*!\nA licence.\nCopyright somebody * / else\n*/\n'));
  assert.ok(text.includes('export const icons = {"star":[[["path",{"d":"M1 1h2"}]],"rating"]};'));
});
