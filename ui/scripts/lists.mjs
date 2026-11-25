// The blueprints in the side panel: every one there is, in a part of the
// panel of their own, by family and in two levels (what somebody applies,
// and under each its parts: `listed.mjs`, which this runs); and under a
// project only what that project uses.
//
//   DAEMON=../target/debug/factory node scripts/lists.mjs [out-dir]
//
// Three projects. Shop has a board and a blueprint of its own that is
// running; Depot has a blueprint of its own that nobody applied, with a
// file it links listed under it and opened from there, and a line under
// them that makes a new one; Yard has nothing. Each lists what it uses and no more, Yard lists nothing at all,
// and a list follows what comes and goes without the page being read
// again. A fourth, Theirs, is made of a folder that was already there: it
// is marked as not trusted wherever it is shown, and its file called
// `std/board` is not listed as its own until its owner trusts it.
//
// And a project's Repo: listed under one whose folder somebody said
// (Theirs; Kept, added from the dialog with a folder), and not under one
// the factory made a folder for, which has nothing under it and no arrow
// until it has something, and whose files are still reached from its own
// page. The dialog asks the folder under the name, of whoever may say
// one, says the daemon's sentence where it refuses one and stays open.
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold as unfoldOne, unfoldAll } from './folded.mjs';
import { twoLevels, twoLevelsOnPhone } from './listed.mjs';
import { saysWhere } from './where.ts';

/** Turns the side panel to a view on the rail, unless it is showing already: Resources with what it has read unfolded, as everything there starts folded (`folded.mjs`). */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  if (name === 'Resources') await unfoldAll(page);
}

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'lists-data');
const home = join(out, 'lists-home');
// A folder that is there before the factory is: somebody else's repository.
const theirs = join(out, 'lists-theirs');
// And one that is not there yet, which a project is added with from the dialog.
const kept = join(out, 'lists-kept');
// Where a made-up package comes from: one that brings two blueprints, one of which runs the other, and no program.
const came = join(out, 'lists-came');
const PACK = 'sample';
const BROUGHT = `extension: ${PACK}
title: Sample
description: Not a real package; it brings two blueprints and runs nothing.
blueprints:
  serve: |
    blueprint: ${PACK}/serve
    description: Serves a sample, by a part of its own.
    steps:
      inner:
        call: ${PACK}/inner
  inner: |
    blueprint: ${PACK}/inner
    description: Works out how a sample is served.
    steps:
      one: { do: noop }
`;

const listens = (name) => `blueprint: ${name}
description: Waits to be told something.
on:
  told:
    event: ${name}.told
    steps:
      say: { do: emit, with: { topic: ${name}.heard } }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, theirs, kept, came]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
await mkdir(join(came, PACK), { recursive: true });
await writeFile(join(came, PACK, 'extension.yaml'), BROUGHT);
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--packages', came], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('extension', 'add', PACK);
  for (const name of ['Shop', 'Depot', 'Yard']) await cli('project', 'create', '--bare', name);
  // A project's own are kept by the factory, in its data folder (`@blueprints/` of the project's).
  const own = async (project, name, text = listens(name)) => {
    const dir = join(data, 'blueprints', 'projects', project);
    await mkdir(join(dir, 'prompts'), { recursive: true });
    await writeFile(join(dir, `${name}.bp`), text);
    await cli('project', 'scan', project);
    return join(dir, `${name}.bp`);
  };
  const watcher = await own('shop', 'watcher');
  // Depot's links a file of its own, which is listed under it.
  await mkdir(join(data, 'blueprints', 'projects', 'depot', 'prompts'), { recursive: true });
  await writeFile(join(data, 'blueprints', 'projects', 'depot', 'prompts', 'ask.md'), 'Ask it plainly.\n');
  await own('depot', 'dispatch', `${listens('dispatch')}steps:\n  put:\n    do: text.put\n    with:\n      parts:\n        - from: ask.md\n`);
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('apply', watcher, '--project', 'shop');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 1100 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await side(page, 'Resources');
  const nav = page.locator('.nav');
  await nav.getByText('Yard', { exact: true }).waitFor();

  // What is under a project: the lines of its tree, and under its
  // Blueprints, when it has that line, the names listed there.
  const project = (name) => nav.locator(':scope > .tree > [data-project]', { has: page.locator(':scope > .nav-row', { hasText: name }) });
  const list = (name) => project(name).locator(':scope > .tree > div', { has: page.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item', { hasText: 'Blueprints' }) });
  const names = async (within) => (await within.locator(':scope > .tree > .nav-item').allInnerTexts()).map((one) => one.replace(/^◆\s*/, '').trim());
  const unfold = async (within) => {
    const head = within.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item');
    if ((await head.getAttribute('aria-expanded')) !== 'true') await head.click();
  };

  // Every blueprint there is, once: in a part of the panel of their own,
  // the last, after the projects and the factory's (whose last line is its
  // Trash), each family under its name.
  await nav.getByText('std', { exact: true }).waitFor();
  const parts = await nav.locator(':scope > h3').evaluateAll((all) => all.map((one) => one.firstChild.textContent.trim()));
  expect(`the panel's parts to be Projects, Factory and Blueprints, got ${parts.join(', ')}`, parts.join(',') === 'Projects,Factory,Blueprints');
  const order = await nav.locator(':scope > h3, :scope > .tree > *').evaluateAll((all) => all.map((one) => (one.querySelector('.truncate') ?? one.firstChild ?? one).textContent.trim()));
  // The Reference, which says what any of them can say, is the first line under the heading; then the families.
  // The box that narrows them is the panel's, under its head and over everything it lists (`sought.mjs`).
  expect(
    `the families of blueprints to come under Blueprints, after its Reference, and Blueprints after the projects and then what is the factory's, got ${order.join(', ')}`,
    order.indexOf('std') === order.indexOf('Reference') + 1 &&
      order.indexOf('Reference') === order.indexOf('Blueprints') + 1 &&
      order.indexOf('Factory') > order.indexOf('Projects') &&
      order.indexOf('Blueprints') === order.indexOf('Trash') + 1,
  );
  expect('no box of their own to stand over the families', (await nav.locator('input[type="search"]').count()) === 0);
  expect('the panel\'s box to stand between its head and its list', await page.locator('.sidebar > .panel-seek + .nav').count() === 1);
  // In two levels: what somebody applies, and under each its parts (`listed.mjs`), from every family folded: Collapse all,
  // and the panel's three parts unfolded again by their arrows.
  await page.getByRole('button', { name: 'Collapse all', exact: true }).click();
  for (const part of ['Projects', 'Factory', 'Blueprints']) await unfoldOne(page, part);
  await twoLevels({ page, out, expect, pack: PACK });
  for (const name of ['Shop', 'Depot']) await unfoldOne(page, name);
  const factory = nav.locator(':scope > .tree > div', { has: page.locator(':scope > button.nav-item', { hasText: /^\s*std\s*$/ }) });
  await unfold(factory);
  await factory.getByText('std/board', { exact: true }).waitFor();
  const all = await factory.locator(':scope > .tree .nav-item .truncate').allInnerTexts();
  expect(`the factory's list to hold the ones that ship and not a project's own, got ${all.join(', ')}`, all.includes('std/watch') && all.includes('std/store') && !all.includes('watcher') && !all.includes('dispatch'));

  // A project lists what it uses: its own first, then what is applied there.
  await list('Shop').waitFor();
  await unfold(list('Shop'));
  await unfold(list('Depot'));
  expect(`Shop to list its own blueprint, the board it has and what looks after the board's cards, got ${(await names(list('Shop'))).join(', ')}`, (await names(list('Shop'))).join() === 'watcher,std/board,std/cards');
  expect(`Depot to list the one blueprint that is its own, got ${(await names(list('Depot'))).join(', ')}`, (await names(list('Depot'))).join() === 'dispatch');
  // With the file of its own it links under it, opened from there; and a line, apart from them, that makes a new one.
  const linked = list('Depot').locator(':scope > .tree > .tree > .nav-item');
  await linked.first().waitFor();
  expect(`Depot's blueprint to have what it links under it, got ${await linked.allInnerTexts()}`, (await linked.allInnerTexts()).map((one) => one.replace(/^¶\s*/, '').trim()).join() === 'prompts/ask.md');
  await list('Depot').locator('.own-new .nav-item', { hasText: 'New blueprint…' }).waitFor();
  await linked.first().click();
  await page.locator('.crumbs').locator('visible=true').getByRole('heading', { name: 'ask.md' }).waitFor();
  expect('Yard, which uses none, to have no list of blueprints', (await list('Yard').count()) === 0);
  // Nobody said a folder for any of the three: the factory made each one, and no Repo is listed under them.
  // Yard has nothing at all under it, and no arrow that would unfold nothing.
  const arrow = (name) => project(name).locator(':scope > .nav-row > button.nav-twist');
  for (const name of ['Shop', 'Depot', 'Yard']) expect(`${name}, whose folder nobody said, to list no Repo`, (await project(name).getByText('Repo', { exact: true }).count()) === 0);
  expect('Yard to have nothing under it and no arrow', (await project('Yard').locator(':scope > .tree > *').count()) === 0 && (await arrow('Yard').count()) === 0);
  expect('Shop, which has a board, to have its arrow', (await arrow('Shop').count()) === 1);
  await page.screenshot({ path: `${out}/lists-0-side.png` });

  // A card is a row of the board's table: with a card on its board, Shop uses what it did.
  await cli('board', 'add', 'shop', 'A till');
  expect(`Shop to list what it did once it has a card, got ${(await names(list('Shop'))).join(', ')}`, (await names(list('Shop'))).join() === 'watcher,std/board,std/cards');
  // Stopped, a blueprint is no longer used there; one that is the project's own file still is its own.
  const board = /^(in_\S+)\s+std\/board\s/m.exec((await cli('instances', '--project', 'shop')).stdout)?.[1];
  expect('the board to be among the instances of its project', !!board);
  await cli('stop', board);
  await list('Shop').getByText('std/board', { exact: true }).waitFor({ state: 'detached' });
  expect(`Shop to list only its own once its board is stopped, got ${(await names(list('Shop'))).join(', ')}`, (await names(list('Shop'))).join() === 'watcher');
  // And one applied to a project that used none gives it a list.
  await cli('apply', 'std/board', '--project', 'yard');
  await arrow('Yard').waitFor();
  expect('Yard to have its arrow once it has something under it', (await arrow('Yard').count()) === 1);
  await unfoldOne(page, 'Yard');
  await list('Yard').waitFor();
  await unfold(list('Yard'));
  // What looks after a board's cards is started by the board, a moment
  // after it: the list has the board first, and then both.
  await list('Yard').getByText('std/cards', { exact: true }).waitFor();
  expect(`Yard to list the board it now has, with what looks after its cards, got ${(await names(list('Yard'))).join(', ')}`, (await names(list('Yard'))).join() === 'std/board,std/cards');
  await page.screenshot({ path: `${out}/lists-1-followed.png` });

  // A project made of a folder that was already there is not trusted, and
  // says so wherever it is shown. A file of its own with the name of a
  // blueprint that ships is not its own there until its owner trusts it.
  await mkdir(join(theirs, 'blueprints', 'std'), { recursive: true });
  await writeFile(join(theirs, 'blueprints', 'courier.bp'), listens('courier'));
  await writeFile(join(theirs, 'blueprints', 'std', 'board.bp'), 'blueprint: std/board\ndescription: Somebody else\'s board.\nsteps:\n  one: { do: noop }\n');
  await cli('project', 'create', '--bare', 'Theirs', '--path', theirs);
  // Its blueprints are read from that folder of its repository only once it is said, as a project said one before.
  await cli('project', 'blueprints', 'theirs', 'blueprints');
  await project('Theirs').locator(':scope > .nav-row .not-trusted').waitFor();
  await unfoldOne(page, 'Theirs');
  // Its folder is one somebody said: its Repo is listed under it, the last line there.
  await project('Theirs').getByRole('button', { name: 'Repo', exact: true }).waitFor();
  await list('Theirs').waitFor();
  await unfold(list('Theirs'));
  expect(`Theirs to list the blueprint under a name of its own and not its file called std/board, got ${(await names(list('Theirs'))).join(', ')}`, (await names(list('Theirs'))).join() === 'courier');
  expect('no project the factory made to be marked', (await nav.locator('.not-trusted').count()) === 1);
  await project('Theirs').locator(':scope > .nav-row .nav-item').click();
  const said = page.locator('.view-head .not-trusted');
  await said.waitFor();
  expect(`the mark to say how the project is trusted, got ${await said.getAttribute('title')}`, /project trust theirs$/.test((await said.getAttribute('title')) ?? ''));
  await page.screenshot({ path: `${out}/lists-2-not-trusted.png` });
  // Trusted from the terminal, the page follows without being read again.
  await cli('project', 'trust', 'theirs');
  await said.waitFor({ state: 'detached' });
  await list('Theirs').getByText('std/board', { exact: true }).waitFor();
  expect('the mark to be gone from the side panel as well', (await nav.locator('.not-trusted').count()) === 0);
  expect(`Theirs to list both of its own once it is trusted, got ${(await names(list('Theirs'))).join(', ')}`, (await names(list('Theirs'))).join() === 'courier,std/board');
  await page.screenshot({ path: `${out}/lists-3-trusted.png` });

  // A project added by hand starts with what one starts with, each a
  // switch in the dialog that adds it, on unless it is turned off: its
  // list has what it was given as soon as it is there, and its page is
  // what is opened.
  const SAYS = "Its whole path; one that is not there yet is made. Left empty, the factory keeps the project's documents and blueprints itself, and no Repo is listed under it.";
  const add = async (name, without = [], shot = null, folder = null) => {
    await page.getByRole('button', { name: 'New project' }).click();
    const asked = page.getByRole('dialog');
    // It asks no workspace: it is made in the one the panel is of, which its head says in a line, and is moved
    // afterwards by its row's Move to…. The caret begins in the name, and the keyboard goes on from it to a switch.
    await asked.getByLabel('Name').waitFor();
    const where = asked.locator('.ask-where');
    expect(`the dialog's head to say the workspace it is made in, got "${await where.innerText().catch(() => '')}"`, (await where.innerText()) === "In Ada's workspace");
    expect('no workspace to be asked', (await asked.getByLabel(/^Workspace/).count()) === 0);
    const caret = () => page.evaluate(() => document.activeElement?.closest('label')?.firstChild?.textContent?.trim() ?? document.activeElement?.getAttribute('role') ?? document.activeElement?.tagName);
    await page.waitForFunction(() => document.activeElement?.tagName === 'INPUT');
    expect(`the caret to begin in the name, got "${await caret()}"`, (await caret()) === 'Name');
    await page.keyboard.press('Tab');
    // Whoever may say a folder is asked it directly under the name, and what the project starts with is after that.
    if ((await caret()) === 'Folder') await page.keyboard.press('Tab');
    expect(`what the project starts with to be after the name by the keyboard, got "${await page.evaluate(() => document.activeElement?.getAttribute('role'))}"`, (await page.evaluate(() => document.activeElement?.getAttribute('role'))) === 'switch');
    await asked.getByLabel('Name').fill(name);
    // The folder is asked directly under the name, and says what leaving it empty means.
    const folderAt = asked.getByLabel('Folder', { exact: true });
    expect('the folder to be asked of whoever may say one, and to be empty unless said', (await folderAt.count()) === 1 && (await folderAt.inputValue()) === '');
    expect(`the field to say what it is for, got ${await folderAt.getAttribute('placeholder')}`, (await folderAt.getAttribute('placeholder')) === 'A folder on this device, for a repository or files already there');
    expect(`what leaving it empty means to be said under it, got ${await asked.locator('.ask-folder .faint').innerText()}`, (await asked.locator('.ask-folder .faint').innerText()).trim() === SAYS);
    const order = await asked.locator('form > *').evaluateAll((all) => all.map((one) => (one.matches('label') ? one : one.querySelector('label, legend'))?.firstChild?.textContent?.trim() ?? ''));
    expect(`the folder to be asked directly under the name, got ${order.join(', ')}`, order.indexOf('Folder') === order.indexOf('Name') + 1);
    // What is said under the switches is what is so: nothing here takes away what a blueprint made.
    const foot = (await asked.locator('.starts > .faint').innerText()).trim();
    expect(`the dialog to say what each is and what stopping one comes to, got ${foot}`, foot === 'Each is a blueprint, applied now or later; stopping one keeps what it made.');
    if (folder) await folder(asked, folderAt);
    const sw = (title) => asked.getByRole('switch', { name: new RegExp(`^${title}`) });
    // A switch for each thing that is offered as a project is added, in the list's order. What is
    // offered on the page of something a project has (its features, on its board's) is not asked here.
    const offered = await asked.locator('.start-with').evaluateAll((all) => all.map((one) => one.querySelector('.grow')?.firstChild?.textContent?.trim() ?? ''));
    expect(`a switch for each of a board, a fleet, docs, pipelines, environments and flags, and no other, got ${offered.join(', ')}`, offered.join() === 'Board,Fleet,Docs,Pipelines,Environments,Flags' && (await asked.getByRole('switch').count()) === 6);
    expect('nothing of features to be asked as a project is added', (await asked.getByText(/Features/).count()) === 0);
    for (const title of ['Board', 'Fleet', 'Docs']) expect(`${title} to be on unless told otherwise`, await sw(title).isChecked());
    for (const title of without) await sw(title).uncheck();
    if (without.includes('Board')) {
      // The fleet goes with the board, cannot be turned on without it, and says why where it is.
      expect('the fleet to be off once the board is', !(await sw('Fleet').isChecked()) && (await sw('Fleet').isDisabled()));
      await asked.getByText('A fleet works a board: turn on a board to have one.').waitFor();
      // It went off only for the lack of a board: with the board back it is back, and it goes again with it.
      await sw('Board').check();
      expect('the fleet to come back with the board, since nobody turned it off', await sw('Fleet').isChecked());
      await sw('Fleet').uncheck();
      await sw('Board').uncheck();
      await sw('Board').check();
      expect('a fleet somebody turned off themselves to stay off when the board comes back', !(await sw('Fleet').isChecked()) && !(await sw('Fleet').isDisabled()));
      await sw('Fleet').check();
      await sw('Board').uncheck();
    }
    // The dialog is never taller than the window, and what it is left by is in reach.
    const box = await asked.boundingBox();
    expect(`the dialog to fit the window, got ${box.y} and ${box.height} of ${page.viewportSize().height}`, box.y >= 0 && box.y + box.height <= page.viewportSize().height);
    if (shot) await page.screenshot({ path: `${out}/${shot}.png` });
    await asked.getByRole('button', { name: 'Create' }).click();
    await asked.waitFor({ state: 'detached' });
    await project(name).waitFor();
  };
  await add('Works', [], 'lists-4-new-project');
  await unfoldOne(page, 'Works');
  await list('Works').waitFor();
  await unfold(list('Works'));
  // What a project starts with is applied one after another, and the list
  // follows each as it comes: the last of them is waited for as a person
  // sees it arrive, a moment after the project is there.
  for (const name of ['std/board', 'std/fleet', 'std/working', 'std/docs']) {
    await list('Works').locator(':scope > .tree > .nav-item', { hasText: name }).first().waitFor();
  }
  const uses = await names(list('Works'));
  expect(`Works to list its board, its fleet, what the fleet is made of and its library, got ${uses.join(', ')}`, ['fleet', 'std/board', 'std/docs', 'std/fleet', 'std/working'].every((name) => uses.includes(name)));
  await page.locator('.tabs').getByRole('tab', { name: /Works/ }).waitFor();
  // What a project has is listed straight under it, in one order and with
  // nothing between: its board, its library, the blueprints it uses, its
  // files. No line says what kind of project it is, and the tables of its
  // fleet, which sit beside its board, are reached from the board's page
  // and are no lines here.
  const under = async (name) => {
    // A line is a button, or something that folds with its button first.
    return project(name)
      .locator(':scope > .tree > *')
      .evaluateAll((lines) => lines.map((line) => (line.matches('.nav-item') ? line : line.querySelector('.nav-item'))?.textContent?.replace(/\s+/g, ' ').trim() ?? ''));
  };
  await project('Works').getByText('Docs', { exact: true }).waitFor();
  const has = await under('Works');
  const places = ['Board', 'Docs', 'Blueprints'].map((line) => has.findIndex((one) => one.endsWith(line)));
  expect(`Works to list its board, its library and its blueprints, in that order and straight under it, and nothing else, got ${has.join(', ')}`, places.every((at, nth) => at >= 0 && (nth === 0 || at > places[nth - 1])) && places[0] === 0 && places[places.length - 1] === has.length - 1);
  // Nobody said a folder for it, so no Repo is listed: nobody asked for any. They are still reached, from its own page.
  expect(`Works, added with no folder said, to list no Repo, got ${has.join(', ')}`, !has.some((one) => /Repo$/.test(one)));
  await page.screenshot({ path: `${out}/lists-4c-no-files.png` });
  await page.getByRole('button', { name: 'Its repo', exact: true }).click();
  await page.getByRole('heading', { name: 'Works repo', level: 1 }).waitFor();
  await page.locator('.files tbody').getByText('README.md').first().waitFor();
  expect('opening its files to have put no Repo line under it', (await project('Works').getByText('Repo', { exact: true }).count()) === 0);
  expect(`the tables that sit beside the board of Works not to be listed under it, got ${has.join(', ')}`, !has.some((one) => /(Agents|Stages|Segments)$/.test(one)));
  expect(`nothing under Works to be called App or Repo, got ${has.join(', ')}`, !has.some((one) => /\b(App|Repo)\b/.test(one)));
  // Without a board there is no fleet either: the library alone.
  await add('Shelf', ['Board'], 'lists-4b-no-board');
  await unfoldOne(page, 'Shelf');
  await list('Shelf').waitFor();
  await unfold(list('Shelf'));
  await list('Shelf').locator(':scope > .tree > .nav-item', { hasText: 'std/docs' }).first().waitFor();
  expect(`Shelf to list its library and nothing else, got ${(await names(list('Shelf'))).join(', ')}`, (await names(list('Shelf'))).join() === 'std/docs');
  expect('nothing to be said about what was left out on purpose', (await page.locator('.toast').count()) === 0);
  await add('Plain', ['Board', 'Docs']);
  expect('a project added without any of them to use no blueprint', (await list('Plain').count()) === 0);
  expect('Plain, which was given nothing and no folder, to have nothing under it and no arrow', (await project('Plain').locator(':scope > .tree > *').count()) === 0 && (await arrow('Plain').count()) === 0);
  await page.screenshot({ path: `${out}/lists-5-started.png` });

  // A project added with a folder said. What the daemon refuses a folder for is said in its own
  // sentence, and the dialog stays, with what was written still there to put right.
  await add('Kept', ['Fleet'], 'lists-6-folder', async (asked, where) => {
    const refused = async (folder, why) => {
      await where.fill(folder);
      await asked.getByRole('button', { name: 'Create' }).click();
      await page.locator('.toast.bad', { hasText: why }).waitFor();
      expect(`the dialog to stay with what was written once ${folder} is refused`, (await asked.getByLabel('Name').inputValue()) === 'Kept' && (await where.inputValue()) === folder);
      await asked.getByRole('button', { name: 'Create', exact: true }).waitFor();
    };
    // A path that is not whole, and a folder that is another project's.
    await refused('lists-kept', "a project's path must be absolute");
    await refused(theirs, `${theirs} is already a project`);
    expect('nothing to have been added for a folder that was refused', (await project('Kept').count()) === 0);
    await where.fill(kept);
  });
  // Its folder is the one that was said, made since it was not there, and its Repo is listed, last under it.
  await unfoldOne(page, 'Kept');
  await project('Kept').getByRole('button', { name: 'Repo', exact: true }).waitFor();
  await project('Kept').getByText('Docs', { exact: true }).waitFor();
  const theirLines = await under('Kept');
  expect(`Kept to list its board, its library, its blueprints and then its files, got ${theirLines.join(', ')}`, /Board$/.test(theirLines[0]) && /Repo$/.test(theirLines[theirLines.length - 1]) && theirLines.some((one) => /Docs$/.test(one)));
  await unfoldOne(page, 'Kept repo');
  await project('Kept').getByText('README.md', { exact: true }).waitFor();
  expect('the project to be the folder that was said', (await readdir(kept)).includes('README.md') && (await readdir(join(data, 'projects'))).every((one) => one !== 'kept'));
  await page.screenshot({ path: `${out}/lists-7-with-folder.png` });

  // Somebody who may add a project and may not take a folder up is not asked for one.
  const member = await browser.newContext({ viewport: { width: 1280, height: 1100 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const theirPage = await member.newPage();
  theirPage.on('pageerror', (err) => problems.push(err.message));
  await theirPage.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const who = await answer.json();
    who.principal.scopes = ['projects', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];
    await route.fulfill({ response: answer, json: who });
  });
  await theirPage.goto(url);
  await theirPage.getByLabel('Email').fill('ada@example.com');
  await theirPage.getByLabel('Password').fill('correct horse');
  await theirPage.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await theirPage.locator('.rail').waitFor();
  await side(theirPage, 'Resources');
  await theirPage.getByRole('button', { name: 'New project' }).click();
  const theirAsk = theirPage.getByRole('dialog');
  await theirAsk.getByLabel('Name').waitFor();
  await theirAsk.getByRole('switch').first().waitFor();
  expect('somebody who may not take a folder up not to be asked for one', (await theirAsk.getByLabel('Folder', { exact: true }).count()) === 0 && (await theirAsk.locator('.ask-folder').count()) === 0);
  await theirPage.screenshot({ path: `${out}/lists-8-no-folder-asked.png` });
  await member.close();

  // The list of blueprints at a phone's width, where the panel has the screen.
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const small = await phone.newPage();
  small.on('pageerror', (err) => problems.push(err.message));
  await small.goto(url);
  await small.getByLabel('Email').fill('ada@example.com');
  await small.getByLabel('Password').fill('correct horse');
  await small.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await small.getByRole('button', { name: 'Expand the side panel' }).waitFor();
  await twoLevelsOnPhone({ page: small, out, expect });
  await phone.close();

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, theirs, kept, came]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
