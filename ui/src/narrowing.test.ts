import assert from 'node:assert/strict';
import { test } from 'node:test';
import { chosenBy, chosenIn, cleaned, EVERYTHING, fromBefore, keptOf, lines, only, settled, shownBy, SPACE, USUAL, valueOf } from './narrowing.ts';
import type { Choice, Kept, Offered } from './narrowing.ts';
import { ALL } from './workspaces.ts';
import type { Held, Workspace } from './workspaces.ts';

const space = (id: string, name: string, projects: string[] = [], more: Partial<Workspace> = {}): Workspace => ({ id, name, created: 0, projects, channels: [], ...more });
const project = (id: string, name = id) => ({ id, name });

// Two workspaces. `shed` is a project the daemon has not been read to have placed, which is of the usual workspace.
const all = [space('back', 'Back office', ['depot'], { usual: true }), space('front', 'Shop front', ['shop', 'stall'])];
const projects = [project('shop', 'Shop'), project('depot', 'Depot'), project('stall', 'Market stall'), project('shed', 'Shed')];
const front: Held = { kind: 'space', id: 'front' };
const back: Held = { kind: 'space', id: 'back' };
const one = (id: string): Choice => ({ kind: 'project', id });
const everything: Choice = { kind: 'everything' };
const said = (offered: Offered | null) => (offered ? [offered.space, ...offered.projects, ...(offered.everything ? [offered.everything] : [])].map((line) => line.label) : null);

// ---- what a choice comes to

test('with nothing chosen the pages show the projects of the workspace the panel is of', () => {
  assert.deepEqual(shownBy({}, front, all, projects, true), { kind: 'projects', projects: ['shop', 'stall'], space: 'front', project: null });
  assert.deepEqual(shownBy({}, back, all, projects, true), { kind: 'projects', projects: ['depot', 'shed'], space: 'back', project: null }, 'a project whose place was not read is the usual workspace\'s');
});

test('what was chosen in a workspace is shown there, and nowhere else', () => {
  const kept: Kept = { front: one('stall'), back: everything };
  assert.deepEqual(shownBy(kept, front, all, projects, true), { kind: 'projects', projects: ['stall'], space: 'front', project: 'stall' });
  assert.deepEqual(shownBy(kept, back, all, projects, true), { kind: 'everything' });
  assert.deepEqual(shownBy({ back: one('depot') }, front, all, projects, true).kind, 'projects');
  assert.deepEqual(only(shownBy({ back: one('depot') }, front, all, projects, true)), ['shop', 'stall'], 'another workspace\'s choice is not this one\'s');
});

test('a workspace with no project shows no project, which is not everything', () => {
  const empty = [...all, space('new', 'New')];
  assert.deepEqual(shownBy({}, { kind: 'space', id: 'new' }, empty, projects, true), { kind: 'projects', projects: [], space: 'new', project: null });
  assert.deepEqual(only({ kind: 'projects', projects: [], space: 'new', project: null }), []);
  assert.equal(only({ kind: 'everything' }), null);
});

test('until the workspaces are read nothing is shown yet, and never everything on the way', () => {
  assert.deepEqual(shownBy({ front: everything }, ALL, [], projects, false), { kind: 'waiting' });
  assert.deepEqual(shownBy({}, ALL, [], [], false), { kind: 'waiting' });
});

test('where the workspaces could not be read there is none to be of, and everything is shown', () => {
  assert.deepEqual(shownBy({ front: one('shop') }, ALL, [], projects, true), { kind: 'everything' });
});

// ---- what a kept choice comes to as things stand

test('a project chosen stands while it is there and is of that workspace', () => {
  assert.deepEqual(settled(one('shop'), 'front', all, projects), one('shop'));
  assert.deepEqual(settled(one('gone'), 'front', all, projects), USUAL, 'one that was forgotten');
  assert.deepEqual(settled(one('depot'), 'front', all, projects), USUAL, 'one that is another workspace\'s');
  const moved = [space('back', 'Back office', ['depot', 'shop'], { usual: true }), space('front', 'Shop front', ['stall'])];
  assert.deepEqual(settled(one('shop'), 'front', moved, projects), USUAL, 'one put in another workspace since');
  assert.deepEqual(settled(one('shed'), 'back', all, projects), one('shed'), 'one whose place was not read is the usual workspace\'s');
  assert.deepEqual(settled(one('shed'), 'front', all, projects), USUAL);
  assert.deepEqual(settled(everything, 'front', all, projects), everything);
  assert.deepEqual(settled(undefined, 'front', all, projects), USUAL);
  assert.deepEqual(settled(USUAL, 'front', all, projects), USUAL);
});

test('a choice that can no longer be made comes to the workspace showing', () => {
  const moved = [space('back', 'Back office', ['depot', 'shop'], { usual: true }), space('front', 'Shop front', ['stall'])];
  assert.deepEqual(shownBy({ front: one('shop') }, front, moved, projects, true), { kind: 'projects', projects: ['stall'], space: 'front', project: null });
  assert.deepEqual(shownBy({ front: one('gone') }, front, all, projects, true), { kind: 'projects', projects: ['shop', 'stall'], space: 'front', project: null });
});

test('what was kept is put right: a workspace that is gone, a project that is gone or is another\'s', () => {
  const kept: Kept = { front: one('shop'), back: everything, gone: one('shop') };
  assert.deepEqual(cleaned(kept, all, projects), { front: one('shop'), back: everything });
  assert.deepEqual(cleaned({ front: one('depot'), back: one('gone') }, all, projects), {});
  const whole: Kept = { front: one('stall'), back: one('shed') };
  assert.equal(cleaned(whole, all, projects), whole, 'with nothing to take out it is the same thing, and nothing is written');
});

test('choosing keeps one choice for a workspace, and all of the workspace is nothing kept', () => {
  assert.deepEqual(chosenIn({}, 'front', one('shop')), { front: one('shop') });
  assert.deepEqual(chosenIn({ front: one('shop'), back: everything }, 'front', one('stall')), { back: everything, front: one('stall') });
  assert.deepEqual(chosenIn({ front: one('shop'), back: everything }, 'front', USUAL), { back: everything });
  assert.deepEqual(chosenIn({}, 'front', USUAL), {});
});

test('what a browser kept is read back, and anything that is no choice is nothing chosen', () => {
  const kept: Kept = { front: one('shop'), back: everything };
  assert.deepEqual(keptOf(JSON.stringify(kept)), kept);
  assert.deepEqual(keptOf(null), {});
  assert.deepEqual(keptOf(''), {});
  assert.deepEqual(keptOf('not json'), {});
  assert.deepEqual(keptOf('"shop"'), {});
  assert.deepEqual(keptOf('["shop"]'), {});
  assert.deepEqual(keptOf(JSON.stringify({ front: { kind: 'space' }, back: { kind: 'project' }, side: { kind: 'project', id: 7 }, yard: 'shop', lane: null })), {});
});

// ---- what a browser kept before the pages were of a workspace

test('one project kept for the whole factory is the choice of its own workspace, and of no other', () => {
  const kept = fromBefore({}, 'depot', all, projects);
  assert.deepEqual(kept, { back: one('depot') });
  // The panel is of the other workspace: its pages show that workspace, not the project that was remembered.
  assert.deepEqual(shownBy(kept, front, all, projects, true), { kind: 'projects', projects: ['shop', 'stall'], space: 'front', project: null });
  assert.deepEqual(shownBy(kept, back, all, projects, true), { kind: 'projects', projects: ['depot'], space: 'back', project: 'depot' });
});

test('what was kept before changes nothing where its project is gone, nothing was kept, or its workspace has a choice since', () => {
  const kept: Kept = { back: everything };
  assert.equal(fromBefore(kept, 'depot', all, projects), kept);
  assert.equal(fromBefore(kept, 'gone', all, projects), kept);
  assert.equal(fromBefore(kept, '', all, projects), kept);
  assert.equal(fromBefore(kept, null, all, projects), kept);
  assert.equal(fromBefore(kept, 'shop', [], projects), kept, 'with no workspace there is none to keep it for');
  assert.deepEqual(fromBefore(kept, 'shop', all, projects), { back: everything, front: one('shop') });
});

// ---- the lines of the choice

test('the choice offers the workspace by its name, then its projects, then every workspace', () => {
  assert.deepEqual(said(lines(front, all, projects, USUAL, false)), ['Shop front', 'Shop', 'Market stall', 'All workspaces']);
  assert.deepEqual(said(lines(back, all, projects, USUAL, false)), ['Back office', 'Depot', 'Shed', 'All workspaces']);
  const offered = lines(front, all, projects, USUAL, false)!;
  assert.deepEqual([offered.space.value, ...offered.projects.map((line) => line.value), offered.everything?.value], [SPACE, 'shop', 'stall', EVERYTHING]);
});

test('with one workspace, every workspace is a line only where it would show something more, or is what is chosen', () => {
  const alone = [space('front', 'Shop front', ['shop', 'stall'], { usual: true })];
  const its = [project('shop', 'Shop'), project('stall', 'Market stall')];
  assert.deepEqual(said(lines(front, alone, its, USUAL, false)), ['Shop front', 'Shop', 'Market stall']);
  assert.deepEqual(said(lines(front, alone, its, USUAL, true)), ['Shop front', 'Shop', 'Market stall', 'All workspaces'], 'the page has something that is no project\'s');
  assert.deepEqual(said(lines(front, alone, its, everything, false)), ['Shop front', 'Shop', 'Market stall', 'All workspaces'], 'what is chosen is a line');
  assert.deepEqual(said(lines(front, alone, [], USUAL, false)), ['Shop front'], 'and with no project there is one line, which is nothing to choose');
});

test('nothing is offered while the panel is of no workspace', () => {
  assert.equal(lines(ALL, [], projects, USUAL, true), null);
  assert.equal(lines({ kind: 'space', id: 'gone' }, all, projects, USUAL, true), null);
});

test('a line is chosen by what it is written as, and what is chosen is written as its line', () => {
  const offered = lines(front, all, projects, USUAL, false)!;
  assert.deepEqual(chosenBy(SPACE, offered), USUAL);
  assert.deepEqual(chosenBy('stall', offered), one('stall'));
  assert.deepEqual(chosenBy(EVERYTHING, offered), everything);
  assert.deepEqual(chosenBy('depot', offered), USUAL, 'a project of another workspace is no line');
  assert.equal(valueOf(USUAL), SPACE);
  assert.equal(valueOf(one('stall')), 'stall');
  assert.equal(valueOf(everything), EVERYTHING);
});
