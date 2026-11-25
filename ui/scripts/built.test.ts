import assert from 'node:assert/strict';
import { test } from 'node:test';
import { BESIDES, hashOf, rebuiltSaid, sourcesOf, staleSaid, unlike } from './built.ts';

test('a build is held to the files of the client it read, with the build, its page and the lock, and to no package or made thing', () => {
  const inputs = ['src/main.tsx', 'node_modules/preact/dist/preact.module.js', 'icons:set', 'src/views/reference.tsx', 'src/main.tsx', '../extensions/reserved'];
  assert.deepEqual(sourcesOf(inputs), [...new Set(['src/main.tsx', 'src/views/reference.tsx', ...BESIDES])].sort());
});

test('a file that changed or is gone is one the client was not built from, and one that is as it was is not', () => {
  const recorded = { 'src/a.ts': hashOf('a'), 'src/b.ts': hashOf('b'), 'src/c.ts': hashOf('c') };
  assert.deepEqual(unlike(recorded, { 'src/a.ts': hashOf('a'), 'src/b.ts': hashOf('b'), 'src/c.ts': hashOf('c') }), []);
  assert.deepEqual(unlike(recorded, { 'src/a.ts': hashOf('a'), 'src/b.ts': hashOf('b, changed'), 'src/c.ts': null }), ['src/b.ts', 'src/c.ts']);
});

test('what is said of a client built from other files names the first few and says what to do', () => {
  assert.match(staleSaid(['src/a.ts']), /src\/a\.ts changed since\. Run `npm run build` in ui\/ and the check again$/);
  assert.match(staleSaid(['a', 'b', 'c', 'd', 'e']), /: a, b, c and 2 more changed since/);
});

test('a check that fails says the client was built again under it only where it was, with both builds and what to do', () => {
  assert.equal(rebuiltSaid('45f690fba5fb', '45f690fba5fb'), null);
  const said = rebuiltSaid('45f690fba5fb', 'a04c062e53d5') ?? '';
  assert.match(said, /began on build 45f690fba5fb and ui\/dist holds a04c062e53d5 now/);
  assert.match(said, /Run the check again where nothing builds the client meanwhile\.$/);
  assert.match(rebuiltSaid('45f690fba5fb', null) ?? '', /holds no build it can name now/);
});
