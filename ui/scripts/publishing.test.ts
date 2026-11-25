import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { STALE_AFTER, lockText, readLock, standing, whileHolding } from './publishing.ts';

const NOW = Date.UTC(2026, 10, 21);
const held = { pid: 4242, token: 'a1b2c3', at: NOW - 1000 };
const everyone = () => true;
const nobody = () => false;

/** A folder of its own under the repository's `target/`, as everything a test makes is. */
async function folder(): Promise<string> {
  const under = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'target', 'ui-tests');
  await mkdir(under, { recursive: true });
  return mkdtemp(join(under, 'publishing-'));
}

test('a lock says who holds it and since when, and is read back as it was written', () => {
  assert.deepEqual(readLock(lockText(held)), held);
});

test('a lock that says nothing that can be read is nobody’s', () => {
  for (const text of ['', '\n', '4242', '4242 a1b2c3', 'x a1b2c3 1', '4242 NOT-HEX 1', '4242 a1b2c3 1 more', '-1 a1b2c3 1']) {
    assert.equal(readLock(text), null, JSON.stringify(text));
    assert.equal(standing(text, NOW, everyone), 'stale', JSON.stringify(text));
  }
});

test('with no lock a build takes it', () => {
  assert.equal(standing(null, NOW, everyone), 'free');
});

test('a lock held by a build at work is waited for', () => {
  assert.equal(standing(lockText(held), NOW, everyone), 'held');
  assert.equal(standing(lockText(held), held.at + STALE_AFTER - 1, everyone), 'held');
});

test('a lock whose build is gone, or that was held longer than a build takes, is taken away', () => {
  assert.equal(standing(lockText(held), NOW, nobody), 'stale');
  assert.equal(standing(lockText(held), held.at + STALE_AFTER, everyone), 'stale', 'its process number may be another’s by now');
});

test('two that want the lock at once each have it alone, and nothing of it is left after', async () => {
  const dir = await folder();
  try {
    const lock = join(dir, 'dist.lock');
    let inside = 0;
    let most = 0;
    const work = async () => {
      inside += 1;
      most = Math.max(most, inside);
      await new Promise((later) => setTimeout(later, 20));
      inside -= 1;
    };
    await Promise.all(Array.from({ length: 6 }, () => whileHolding(lock, work)));
    assert.equal(most, 1);
    assert.deepEqual(await readdir(dir), []);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test('a lock left by a build that is gone is taken, and one taken from a holder is not taken away by it', async () => {
  const dir = await folder();
  try {
    const lock = join(dir, 'dist.lock');
    // No process is numbered so high.
    await writeFile(lock, lockText({ pid: 2 ** 22 + 7, token: 'dead', at: Date.now() }));
    const seen = await whileHolding(lock, () => readFile(lock, 'utf8'));
    assert.equal(readLock(seen)?.pid, process.pid);
    assert.deepEqual(await readdir(dir), []);

    const someone = lockText({ pid: process.pid, token: 'beef', at: Date.now() });
    await whileHolding(lock, () => writeFile(lock, someone));
    assert.equal(await readFile(lock, 'utf8'), someone);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
