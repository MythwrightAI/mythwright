import assert from 'node:assert/strict';
import { test } from 'node:test';

import { givable } from './givable.ts';
import type { Workspace } from './workspaces.ts';

const space = (id: string, projects: string[], usual = false): Workspace =>
  ({ id, name: id, projects, channels: [], created: usual ? 1 : 2, usual }) as unknown as Workspace;

const all = [space('home', ['shop', 'site'], true), space('press', ['print'])];
const agents = [
  { name: 'builder', project: null },
  { name: 'print-worker-1', project: 'print' },
  { name: 'site-worker-1', project: 'site' },
  { name: 'shop-architect-1', project: 'shop' },
  { name: 'shop-worker-1', project: 'shop' },
];
const names = (given: { name: string }[]) => given.map((one) => one.name).join(' ');

test('a card is offered the agents of its project, then its workspace, then the factory', () => {
  assert.equal(names(givable(agents, 'shop', all)), 'shop-architect-1 shop-worker-1 site-worker-1 builder');
  assert.equal(names(givable(agents, 'site', all)), 'site-worker-1 shop-architect-1 shop-worker-1 builder');
});

test('an agent of a project of another workspace is not offered', () => {
  assert.equal(names(givable(agents, 'print', all)), 'print-worker-1 builder');
  assert.ok(!givable(agents, 'shop', all).some((agent) => agent.project === 'print'));
});

test('whoever has the card already is offered, whatever it is of', () => {
  assert.equal(names(givable(agents, 'shop', all, 'agent:print-worker-1')), 'shop-architect-1 shop-worker-1 print-worker-1 site-worker-1 builder');
  assert.equal(names(givable(agents, 'shop', all, 'user:acct_1')), 'shop-architect-1 shop-worker-1 site-worker-1 builder');
});

test('a project no workspace names is of the usual one, and so are its agents', () => {
  const more = [...agents, { name: 'new-worker-1', project: 'new' }];
  assert.equal(names(givable(more, 'new', all)), 'new-worker-1 site-worker-1 shop-architect-1 shop-worker-1 builder');
  assert.ok(givable(more, 'shop', all).some((agent) => agent.name === 'new-worker-1'));
  assert.ok(!givable(more, 'print', all).some((agent) => agent.name === 'new-worker-1'));
});

test('until the workspaces are read every agent is offered, as it was', () => {
  assert.equal(names(givable(agents, 'shop', [])), names(agents));
});
