import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { type Branch, type Branches, type Ran, branchSaid, chosen, gave, lastRunSaid, mayRun, runClosed, short } from './envrun.ts';

const branch = (name: string, subject = ''): Branch => ({ name, commit: `${name.length}abcdef0123456789`, at: 0, subject, ...(name.includes('/') ? { remote: name.split('/')[0] } : {}) });
const listed: Branches = { repository: true, branches: [branch('main'), branch('origin/dev', 'dev, as it is there')] };
const ran: Ran = { branch: 'origin/dev', commit: 'abcdef0123456789', blueprint: 'deploy', at: 5, by_name: 'Ada', instance: 'in_1', run: 'run_1', status: 'ok' };

test('who may run is who may apply, and who may fetch writes the project and holds shell', () => {
  assert.deepEqual(mayRun(['blueprints', 'projects:read']), { run: true, fetch: false });
  assert.deepEqual(mayRun(['projects', 'shell']), { run: false, fetch: true });
  assert.deepEqual(mayRun(['*']), { run: true, fetch: true });
  assert.deepEqual(mayRun(['projects:read']), { run: false, fetch: false });
});

test('the branch chosen is what was picked, else what it last ran, else the first, and only one that is there', () => {
  assert.equal(chosen(listed.branches, 'main', ran), 'main');
  assert.equal(chosen(listed.branches, '', ran), 'origin/dev');
  assert.equal(chosen(listed.branches, 'gone', { ...ran, branch: 'gone too' }), 'main');
  assert.equal(chosen([], 'main', ran), '');
});

test('run is closed with why until there is a blueprint, a repository with a branch and a branch chosen, for whoever may', () => {
  const open = { deploy: 'deploy', branches: listed, branch: 'main', may: true };
  assert.equal(runClosed(open), null);
  assert.match(runClosed({ ...open, may: false })!, /applying a blueprint/);
  assert.match(runClosed({ ...open, deploy: null })!, /Say the blueprint/);
  assert.match(runClosed({ ...open, branches: null })!, /Reading/);
  assert.equal(runClosed({ ...open, branches: { repository: false, branches: [], said: '`shop` is no repository' } }), '`shop` is no repository');
  assert.match(runClosed({ ...open, branches: { repository: true, branches: [] } })!, /nothing is committed/);
  assert.match(runClosed({ ...open, branch: 'origin/gone' })!, /Choose the branch/);
});

test('what it last ran is the daemon\'s words, with when beside them where it is a run, and nothing made up', () => {
  const ago = (at: number) => `at ${at}`;
  assert.equal(lastRunSaid({ kind: 'loaded', said: 'origin/dev at abcdef0, went well, by Ada', at: 5, run: 'run_1' }, ago), 'origin/dev at abcdef0, went well, by Ada, at 5');
  assert.equal(lastRunSaid({ kind: 'applied', said: 'preview as dev-preview, running, by Ada', at: 0 }, ago), 'preview as dev-preview, running, by Ada, at 0', 'a when of nought is still one');
  assert.equal(lastRunSaid({ kind: 'running', said: '1 running under it, with no run yet' }, ago), '1 running under it, with no run yet');
  assert.equal(lastRunSaid({ kind: 'never', said: 'never run' }, ago), 'never run');
  assert.equal(lastRunSaid(undefined, ago), '', 'never "never run" where the daemon said nothing');
  assert.equal(short('0123456789'), '0123456');
});

test('a branch is offered by its name, its commit and its first line', () => {
  assert.equal(branchSaid(branch('origin/dev', 'dev, as it is there')), 'origin/dev · 10abcde · dev, as it is there');
  assert.equal(branchSaid(branch('main')), 'main · 4abcdef');
});

test('what a run gave out is each thing by name, an address of the web a link and nothing else one', () => {
  assert.deepEqual(gave({ address: 'http://127.0.0.1:7801/', built: 'ok', count: 3, none: null, empty: '', script: 'javascript:alert(1)' }), [
    { name: 'address', text: 'http://127.0.0.1:7801/', href: 'http://127.0.0.1:7801/' },
    { name: 'built', text: 'ok', href: null },
    { name: 'count', text: '3', href: null },
    { name: 'script', text: 'javascript:alert(1)', href: null },
  ]);
  assert.deepEqual(gave(null), []);
  assert.deepEqual(gave(['a']), []);
});
