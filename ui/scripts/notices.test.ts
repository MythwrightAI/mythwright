import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { CLARIFIED, accepted, by, differs, drawnIn, licenceFiles, named, packageOf, saidUnder, taken, withinPrebuilt } from './notices.ts';

test('a file of the bundle is of the package whose folder it is in, and a file of the client is of none', () => {
  assert.equal(packageOf('node_modules/preact/dist/preact.module.js'), 'node_modules/preact');
  assert.equal(packageOf('node_modules/@codemirror/view/dist/index.js'), 'node_modules/@codemirror/view');
  // A package inside another is the inner one, by where it is.
  assert.equal(packageOf('node_modules/outer/node_modules/@scope/inner/index.js'), 'node_modules/outer/node_modules/@scope/inner');
  assert.equal(packageOf('src/main.tsx'), null);
  assert.equal(packageOf('icons:set'), null);
});

test('the packages drawn in are those with something in what was written, each once', () => {
  const written = {
    outputs: {
      'dist/assets/app-ONE.js': {
        inputs: {
          'src/main.tsx': { bytesInOutput: 900 },
          'node_modules/used/index.js': { bytesInOutput: 120 },
          'node_modules/used/more.js': { bytesInOutput: 40 },
          // Read by the bundler and left out of what it wrote.
          'node_modules/shaken-out/index.js': { bytesInOutput: 0 },
        },
      },
      'dist/assets/chunk-TWO.js': {
        inputs: { 'node_modules/@scope/late/index.js': { bytesInOutput: 7 }, 'node_modules/used/index.js': { bytesInOutput: 3 } },
      },
    },
  };
  assert.deepEqual(drawnIn(written), ['node_modules/@scope/late', 'node_modules/used']);
});

test('a package served as its authors built it holds what it depends on', async () => {
  // The charts are served whole, and their build has the colours in it.
  assert.deepEqual(await withinPrebuilt('node_modules/chart.js'), ['node_modules/@kurkle/color', 'node_modules/chart.js']);
});

test('the licences a package may be under are the ones the tool is told for crates', async () => {
  const config = '# said first\naccepted = [\n    "MIT",\n    # why this one\n    "Apache-2.0 WITH LLVM-exception",\n    "ISC",\n]\n\n[some-crate]\naccepted = ["MPL-2.0"]\n';
  assert.deepEqual(accepted(config), ['MIT', 'Apache-2.0 WITH LLVM-exception', 'ISC']);
  assert.deepEqual(accepted('targets = []\n'), []);
  // The list there is: what a notice is all that is asked by, and nothing
  // that asks for a library's source to be handed on.
  const real = accepted(await readFile('../about.toml', 'utf8'));
  for (const one of ['MIT', 'ISC', 'BSD-2-Clause']) assert.ok(real.includes(one), one);
  assert.deepEqual(real.filter((one) => /^(MPL|GPL|LGPL|AGPL|EPL|CDDL)/.test(one)), []);
});

test('a package is taken under the licence it names, or the first accepted of several it offers', () => {
  const list = ['MIT', 'Apache-2.0', 'ISC'];
  assert.equal(taken('ISC', list), 'ISC');
  assert.equal(taken('(Apache-2.0 OR MIT)', list), 'MIT');
  assert.equal(taken('GPL-3.0-only OR ISC', list), 'ISC');
  // One that is not accepted, one that is under two at once, and one whose
  // manifest says nothing that can be read: none is taken under anything.
  assert.equal(taken('GPL-3.0-only', list), null);
  assert.equal(taken('MIT AND ISC', list), null);
  assert.equal(taken({ type: 'MIT' }, list), null);
  assert.equal(taken(undefined, list), null);
});

test('who a package is by is read from its manifest however it is written there', () => {
  assert.deepEqual(by({ author: 'Somebody <somebody@example.org>' }), ['Somebody <somebody@example.org>']);
  assert.deepEqual(by({ author: { name: 'One', email: 'one@example.org' }, contributors: ['Two', { name: 'Three' }, {}] }), ['One <one@example.org>', 'Two', 'Three']);
  assert.deepEqual(by({}), []);
});

const NOTICES = [
  '# Third-party notices',
  '## The programs: Rust crates',
  '### MIT License (`MIT`)',
  '- `a-crate` 1.0.0, https://example.org/a-crate',
  '````text',
  'words',
  '````',
  '## The client: packages',
  '### MIT License (`MIT`)',
  '- `preact` 11.2.0, https://example.org/preact',
  '- `@scope/package` 3.2.1',
  '  By: Somebody',
  '`````text',
  '## The programs: Rust crates',
  '- `inside-a-text` 9.9.9',
  '````',
  '`````',
  '### ISC License (`ISC`)',
  '- `icons` 1.66.0, https://example.org/icons',
  '````text',
  'words',
  '````',
].join('\n');

test('the packages a notices file names are those of the client, and nothing inside a text', () => {
  assert.deepEqual(named(NOTICES), ['@scope/package 3.2.1', 'icons 1.66.0', 'preact 11.2.0']);
  assert.deepEqual(named(''), []);
});

test('a build whose packages are not the ones the notices name says which, and what to run', () => {
  const built = ['@scope/package 3.2.1', 'icons 1.66.0', 'preact 11.2.0'];
  assert.equal(differs(built, named(NOTICES)), null);
  const more = differs([...built, 'new-one 0.1.0'], named(NOTICES));
  assert.match(more ?? '', /in what was built and not in the notices: new-one 0\.1\.0\)/);
  assert.match(more ?? '', /Run `cargo xtask notices` at the top of the repository and commit what it writes/);
  const updated = differs(['@scope/package 3.2.1', 'icons 1.67.0', 'preact 11.2.0'], named(NOTICES));
  assert.match(updated ?? '', /not in the notices: icons 1\.67\.0; in the notices and no longer in what is built: icons 1\.66\.0/);
  assert.match(differs(built, []) ?? '', /not in the notices: @scope\/package 3\.2\.1, icons 1\.66\.0, preact 11\.2\.0\)/);
});

test('a package whose manifest names no licence is under the one said for it, while its file is the one that was read', () => {
  const sha256 = CLARIFIED.khroma!.sha256;
  assert.deepEqual(saidUnder({ name: 'khroma' }, () => sha256), { said: 'MIT', file: 'license' });
  const changed = saidUnder({ name: 'khroma' }, () => 'f00d');
  assert.ok('changed' in changed && changed.changed.includes('read it again'));
  assert.ok('changed' in saidUnder({ name: 'khroma' }, () => null));
  // One that names a licence is under it, and one that names none and was never read is under nothing.
  assert.deepEqual(saidUnder({ name: 'khroma', license: 'ISC' }, () => null), { said: 'ISC' });
  assert.deepEqual(saidUnder({ name: 'nobody-read-it' }, () => sha256), { said: undefined });
});

test('the files of a licence are those called so and no program, and of several those called after the licence taken', () => {
  assert.deepEqual(licenceFiles(['LICENSE', 'license-update.mjs', 'README.md'], 'MIT'), ['LICENSE']);
  assert.deepEqual(licenceFiles(['LICENSE', 'LICENSE-MPL'], 'Apache-2.0'), ['LICENSE', 'LICENSE-MPL']);
  assert.deepEqual(licenceFiles(['LICENSE-APACHE', 'LICENSE-MIT'], 'MIT'), ['LICENSE-MIT']);
  assert.deepEqual(licenceFiles(['COPYING.txt', 'index.js'], 'ISC'), ['COPYING.txt']);
  assert.deepEqual(licenceFiles(['index.js'], 'MIT'), []);
});
