import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { type Branch, type Read, OneAtATime, branchesBy, cellSaid, changeSaid, gitListed, headSaid, linkOf, noneSaid, short } from './gitpage.ts';

test('a project lists its Git line where its folder is a repository whoever added it said, and not one the factory made', () => {
  assert.equal(gitListed({ folder: 'given', repository: true }), true);
  assert.equal(gitListed({ folder: 'own', repository: true }), false, 'one the factory made a repository so that work can branch it');
  assert.equal(gitListed({ folder: 'given', repository: false }), false);
  assert.equal(gitListed({ folder: 'own', repository: false }), false);
  assert.equal(gitListed({ repository: true }), true, 'a daemon from before says nothing of whose its folder is');
  assert.equal(gitListed({}), false);
});

test('the branch the folder is on is said with how it stands against the one it follows', () => {
  assert.equal(headSaid({ branch: 'main', commit: 'abc', upstream: 'origin/main', ahead: 2, behind: 1 }), 'On main, following origin/main: 2 ahead and 1 behind');
  assert.equal(headSaid({ branch: 'main', commit: 'abc', upstream: 'origin/main', ahead: 0, behind: 3 }), 'On main, following origin/main: 3 behind');
  assert.equal(headSaid({ branch: 'main', commit: 'abc', upstream: 'origin/main', ahead: 0, behind: 0 }), 'On main, following origin/main: level with it');
  assert.equal(headSaid({ branch: 'main', commit: 'abc' }), 'On main', 'one that follows nothing says nothing of it');
  assert.equal(headSaid({ branch: null, commit: '0123456789abcdef' }), 'On no branch, at 0123456789');
  assert.equal(headSaid({ branch: 'main', commit: null }), 'On main, with nothing committed yet');
  assert.equal(headSaid(undefined), '');
});

test('what is changed of a path is said by what is staged and then what is not', () => {
  assert.equal(changeSaid({ path: 'a', staged: 'modified', unstaged: '' }), 'modified, staged');
  assert.equal(changeSaid({ path: 'a', staged: '', unstaged: 'modified' }), 'modified, not staged');
  assert.equal(changeSaid({ path: 'a', staged: 'added', unstaged: 'deleted' }), 'added, staged; deleted, not staged');
  assert.equal(changeSaid({ path: 'a', staged: 'modified', unstaged: 'modified' }), 'modified, staged; and changed again');
  assert.equal(changeSaid({ path: 'a', staged: '', unstaged: 'untracked' }), 'new, not tracked');
  assert.equal(changeSaid({ path: 'a', staged: 'unmerged', unstaged: 'unmerged' }), 'both sides changed it, in the middle of a merge');
});

test('branches are the repository own first and then each remote by name', () => {
  const branch = (name: string, remote?: string): Branch => ({ name, refname: name, commit: 'c', at: 0, subject: '', ...(remote ? { remote } : {}) });
  const by = branchesBy([branch('upstream/x', 'upstream'), branch('main'), branch('origin/main', 'origin'), branch('dev')]);
  assert.deepEqual(by.map(([where, all]) => [where, all.map((one) => one.name)]), [
    ['', ['main', 'dev']],
    ['origin', ['origin/main']],
    ['upstream', ['upstream/x']],
  ]);
  assert.deepEqual(branchesBy([]), []);
});

test('a row of a part is drawn as the daemon handed it, with its address only where it opens a page', () => {
  const read: Read = { part: { name: 'p', title: 'Pull requests', columns: [{ field: 'n', title: '#' }], link: 'url' }, rows: [], more: 0 };
  assert.equal(linkOf(read, { n: 1, url: 'https://example.org/1' }), 'https://example.org/1');
  assert.equal(linkOf(read, { n: 1, url: 'javascript:alert(1)' }), null);
  assert.equal(linkOf(read, { n: 1 }), null);
  assert.equal(linkOf({ ...read, part: { ...read.part, link: undefined } }, { url: 'https://example.org/1' }), null);
  assert.deepEqual([cellSaid(7), cellSaid('x'), cellSaid(undefined)], ['7', 'x', '']);
  assert.equal(noneSaid('Pull requests'), 'No pull requests.');
  assert.equal(short('0123456789abcdef'), '0123456789');
});

test('what is asked many times while it is read is read once more after, and never twice at once', async () => {
  let reading = 0;
  let most = 0;
  let read = 0;
  const waits: (() => void)[] = [];
  const one = new OneAtATime(
    () =>
      new Promise<void>((done) => {
        reading += 1;
        read += 1;
        most = Math.max(most, reading);
        waits.push(() => {
          reading -= 1;
          done();
        });
      }),
  );
  one.ask();
  one.ask();
  one.ask();
  assert.equal(read, 1);
  waits.shift()!();
  await new Promise((done) => setImmediate(done));
  assert.equal(read, 2, 'asked again while it read: once more');
  waits.shift()!();
  await new Promise((done) => setImmediate(done));
  assert.equal(read, 2, 'and no more');
  assert.equal(most, 1);
});
