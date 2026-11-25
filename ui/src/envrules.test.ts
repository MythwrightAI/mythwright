import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { actedSaid, addable, approveSaid, branchesOf, branchesSaid, branchesWritten, joined, mayRule, nameOf, wentSaid } from './envrules.ts';

const people = [
  { actor: 'user:ada', name: 'Ada' },
  { actor: 'user:grace', name: 'Grace' },
  { actor: 'user:bob', name: 'Bob' },
];

test('who may say what an environment holds to is whoever may change its secrets', () => {
  assert.equal(mayRule(['environments:secrets']), true);
  assert.equal(mayRule(['*']), true);
  assert.equal(mayRule(['projects', 'shell', 'secrets']), false);
});

test('who approves is said by name, and nobody named says that whoever may apply starts anything', () => {
  assert.equal(approveSaid(undefined, people), 'Nobody is named: whoever may apply starts anything under it.');
  assert.equal(approveSaid({ approve: [] }, people), 'Nobody is named: whoever may apply starts anything under it.');
  assert.equal(approveSaid({ approve: ['user:grace'] }, people), "Grace approves what goes under it: anybody else's run waits for a go-ahead from one of them.");
  assert.equal(approveSaid({ approve: ['user:grace', 'user:ada'] }, people).slice(0, 26), 'Grace and Ada approve what');
  assert.equal(nameOf('user:gone', people), 'somebody no longer here');
  assert.equal(joined(['Grace', 'Ada', 'Bob']), 'Grace, Ada and Bob');
  assert.equal(joined([]), '');
});

test('which branches run under it is said, written in its box by spaces, and read back from spaces or commas each once', () => {
  assert.equal(branchesSaid(undefined), 'Any branch may be run under it.');
  assert.equal(branchesSaid({ branches: ['main', 'release/*'] }), 'Only main and release/* may be run under it.');
  assert.equal(branchesWritten({ branches: ['main', 'release/*'] }), 'main release/*');
  assert.equal(branchesWritten({}), '');
  assert.deepEqual(branchesOf(' main, release/*  main\norigin/main'), ['main', 'release/*', 'origin/main']);
  assert.deepEqual(branchesOf('  '), []);
});

test('those who may be added as approving are the people not among them, by name, and never anything that is no person', () => {
  const with_agent = [...people, { actor: 'agent:helper', name: 'Helper' }];
  assert.deepEqual(
    addable(with_agent, { approve: ['user:grace'] }).map((one) => one.name),
    ['Ada', 'Bob'],
  );
  assert.equal(addable(people, undefined).length, 3);
});

test('a run that acted under it is said by what it is of, how it went, by whom and when', () => {
  const ago = (at: number) => `${at} minutes ago`;
  assert.equal(actedSaid({ run: 'run_1', status: 'ok', at: 3, what: 'deploy as deploy-prod', by_name: 'Ada' }, ago), 'deploy as deploy-prod, went well, by Ada, 3 minutes ago');
  assert.equal(wentSaid('failed'), 'failed');
  assert.equal(wentSaid('cancelled'), 'was cancelled');
  assert.equal(wentSaid('running'), 'running');
  assert.equal(wentSaid('odd'), 'not known');
});
