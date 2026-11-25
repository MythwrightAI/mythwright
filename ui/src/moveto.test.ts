import assert from 'node:assert/strict';
import { test } from 'node:test';
import { channelPlaces, channelQuestion, projectPlaces, projectQuestion } from './moveto.ts';
import type { Workspace } from './workspaces.ts';

const space = (id: string, name: string, projects: string[] = [], channels: string[] = [], created = 1): Workspace => ({ id, name, created, projects, channels });
const all = [space('ledger', 'Ledger', ['notes'], ['diary'], 2), space('mw', 'Garden', ['garden'], [], 1), space('move-to', 'Move to', [], [], 3)];

test('a project can be moved to each workspace, the one it is in marked, and then to a new one', () => {
  assert.deepEqual(projectPlaces('garden', all), [
    { kind: 'workspace', id: 'ledger', name: 'Ledger', here: false },
    { kind: 'workspace', id: 'mw', name: 'Garden', here: true },
    { kind: 'workspace', id: 'move-to', name: 'Move to', here: false },
    { kind: 'new' },
  ]);
});

test('a project no workspace names is marked where the daemon puts it, the usual one', () => {
  assert.deepEqual(
    projectPlaces('new-one', all).filter((place) => place.kind === 'workspace' && place.here),
    [{ kind: 'workspace', id: 'mw', name: 'Garden', here: true }],
  );
});

test('with no workspace read a project can only be moved to a new one', () => {
  assert.deepEqual(projectPlaces('garden', []), [{ kind: 'new' }]);
});

test('a channel can be moved to each workspace and then, under a heading, each project, what it belongs to marked', () => {
  const projects = [{ id: 'notes', name: 'Notes' }, { id: 'garden', name: 'Garden' }];
  assert.deepEqual(channelPlaces({ id: 'diary', project: null }, all, projects), [
    { kind: 'workspace', id: 'ledger', name: 'Ledger', here: true },
    { kind: 'workspace', id: 'mw', name: 'Garden', here: false },
    { kind: 'workspace', id: 'move-to', name: 'Move to', here: false },
    { kind: 'heading', said: 'Projects' },
    { kind: 'project', id: 'notes', name: 'Notes', here: false },
    { kind: 'project', id: 'garden', name: 'Garden', here: false },
  ]);
  const ofProject = channelPlaces({ id: 'talk', project: 'notes' }, all, projects);
  assert.deepEqual(ofProject.filter((place) => 'here' in place && place.here), [{ kind: 'project', id: 'notes', name: 'Notes', here: true }]);
});

test('a channel offers nothing before the workspaces are read, and no heading where there is no project', () => {
  assert.deepEqual(channelPlaces({ id: 'diary', project: null }, [], [{ id: 'notes', name: 'Notes' }]), []);
  assert.deepEqual(
    channelPlaces({ id: 'diary', project: null }, all, []).map((place) => place.kind),
    ['workspace', 'workspace', 'workspace'],
  );
});

test('moving a project names it and the workspace, says its channels go with it and that it lets nobody in or out', () => {
  assert.deepEqual(projectQuestion('Garden', 'Ledger'), {
    title: 'Move Garden to Ledger?',
    says: 'Garden is listed in Ledger from now on, for everybody, and the channels that are its go with it. Nothing else of it changes: its board, its tables, its files and what runs for it stay as they are, and nobody is let in or kept out by where it is listed.',
  });
});

test('moving a channel to a workspace makes it that one\'s own, and says when it is a project\'s no longer', () => {
  assert.deepEqual(channelQuestion('#plans', { kind: 'workspace', name: 'Annex' }), {
    title: 'Move #plans to Annex?',
    says: "#plans is Annex's own from now on, and is listed there for everybody. Nothing else of it changes: what was said in it stays, and nobody is let in or kept out by where it is listed.",
  });
  assert.equal(
    channelQuestion('#talk', { kind: 'workspace', name: 'Annex' }, 'Shop').says,
    "#talk is Annex's own from now on, and is listed there for everybody. It is Shop's no longer, and the factory's bot there is no project's. Nothing else of it changes: what was said in it stays, and nobody is let in or kept out by where it is listed.",
  );
});

test('moving a channel to a project says it is listed with it in its workspace and whose the bot there is', () => {
  assert.deepEqual(channelQuestion('Quay works', { kind: 'project', name: 'Shop', space: 'Annex' }), {
    title: 'Move Quay works to Shop?',
    says: "Quay works is Shop's from now on, listed with it in Annex, and the factory's bot there is Shop's. Nothing else of it changes: what was said in it stays, and nobody is let in or kept out by where it is listed.",
  });
  assert.equal(channelQuestion('#a', { kind: 'project', name: 'Shop', space: null }).says.startsWith("#a is Shop's from now on, and the factory's bot"), true);
});
