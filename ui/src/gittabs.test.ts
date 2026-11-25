import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import type { Offered, Part, Seen } from './gitpage.ts';
import { gitTabs, partKey, readKey, showing, toRead } from './gittabs.ts';

const part = (name: string, title: string): Part => ({ name, title, columns: [{ field: 'title', title: 'Title' }] });
const sample: Offered = { package: 'sample', title: 'Sample Host', parts: [part('pulls', 'Pull requests'), part('checks', 'Checks')] };
const seen = (shows: Offered[] = []): Seen => ({
  project: 'shop',
  repository: true,
  changes: [{ path: 'a', staged: '', unstaged: 'modified' }],
  more: 2,
  branches: [{ name: 'main', refname: 'refs/heads/main', commit: 'c', at: 1, subject: 's' }],
  remotes: [],
  shows,
});

test("git's own tabs come first, in order, with how many each shows where that is known", () => {
  const tabs = gitTabs(seen());
  assert.deepEqual(
    tabs.map((tab) => [tab.key, tab.title, tab.count]),
    [
      ['changes', 'Changes', 3],
      ['commits', 'Commits', undefined],
      ['branches', 'Branches', 1],
      ['remotes', 'Remotes', 0],
    ],
  );
});

test('a folder that is no repository, and one not read yet, has no tabs', () => {
  assert.deepEqual(gitTabs(null), []);
  assert.deepEqual(gitTabs({ project: 'plain', repository: false, said: 'no' }), []);
});

test("each part a package offers is a tab after git's own, known by its own name and called what the package calls it", () => {
  const tabs = gitTabs(seen([sample]));
  assert.deepEqual(
    tabs.slice(4).map((tab) => [tab.key, tab.title, tab.package, tab.packageTitle, tab.whose]),
    [
      ['pulls', 'Pull requests', 'sample', 'Sample Host', undefined],
      ['checks', 'Checks', 'sample', 'Sample Host', undefined],
    ],
  );
});

test('a part whose package whoever reads may not use is a tab that says why, never left out', () => {
  const tabs = gitTabs(seen([{ ...sample, refused: 'Ada may not use the package `sample` (extensions:sample)' }]));
  assert.equal(tabs.length, 6);
  assert.ok(tabs.slice(4).every((tab) => tab.refused?.includes('extensions:sample')));
  assert.equal(toRead(tabs[4], {}), false, 'nothing is read for whoever may not');
});

test("a part's tab is known by its package too where another package's part, or one of git's own, has its name", () => {
  const other: Offered = { package: 'other', title: 'Other Host', parts: [part('pulls', 'Merge requests'), part('commits', 'Commits')] };
  assert.equal(partKey([sample, other], 'sample', 'pulls'), 'sample.pulls');
  assert.equal(partKey([sample, other], 'other', 'pulls'), 'other.pulls');
  assert.equal(partKey([sample, other], 'sample', 'checks'), 'checks');
  assert.equal(partKey([sample, other], 'other', 'commits'), 'other.commits', "one of git's own words is git's");
  const tabs = gitTabs(seen([sample, other]));
  assert.deepEqual(
    tabs.map((tab) => tab.key),
    ['changes', 'commits', 'branches', 'remotes', 'sample.pulls', 'checks', 'other.pulls', 'other.commits'],
  );
  assert.equal(tabs.find((tab) => tab.key === 'other.commits')?.whose, true, 'called what a tab of git is called, it says whose it is');
  assert.equal(tabs.find((tab) => tab.key === 'sample.pulls')?.whose, undefined);
});

test('the tab that shows is the one kept where it is there, else the first', () => {
  const tabs = gitTabs(seen([sample]));
  assert.equal(showing(tabs, 'checks')?.key, 'checks');
  assert.equal(showing(tabs, null)?.key, 'changes');
  assert.equal(showing(tabs, 'gone')?.key, 'changes', "a part of a package taken away since: git's first");
  assert.equal(showing([], 'checks'), null);
});

test('a part is read when its tab shows and nothing of it was read, and git\'s own tabs read nothing more', () => {
  const tabs = gitTabs(seen([sample]));
  assert.equal(readKey(tabs[4]), 'sample/pulls');
  assert.equal(toRead(tabs[4], {}), true);
  assert.equal(toRead(tabs[4], { 'sample/pulls': { reading: true } }), false);
  assert.equal(toRead(tabs[0], {}), false);
  assert.equal(toRead(null, {}), false);
});
