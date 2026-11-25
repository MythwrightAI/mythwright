// The Trash, as the factory's owner sees it on two pages, one of which is
// never read again: a card, a row of a table, a file, a project and a
// channel each deleted (each in the page saying it moved to the Trash,
// kept thirty days, with Undo, a project and a channel each from the last
// line of its row's menu behind a question, and a card taken back by it in
// its column); every one
// in the Trash, by kind, with who deleted it and when it goes, on the page
// that was not read again too; a card given back from there, in its column
// on the other page; a file given back where something is at its place
// now, refused in the daemon's sentence and given back beside it; a
// project given back; a channel given back with what was said in it, on
// the other page too; a page of a wiki deleted from a terminal with the
// pages under it, said in the Trash as a page with how many pages it
// holds, given back with them under its page, and given back as a wiki of
// its own once its page is gone, which is said, from the page never read
// again; one deleted for good behind a question; how long
// what is deleted is kept, said in Settings and by the terminal the same;
// nothing of the page lying over anything else of it or scrolling sideways
// at three widths and a phone's; and a member, who sees what they could
// read and is offered nothing to give back, and is shown nothing of a
// private channel they were not in.
//
//   DAEMON=../target/debug/factory node scripts/trashed.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { fillsPage } from './filled.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'trashed-data');
const home = join(out, 'trashed-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const NOTES = ['blueprint: notes', 'steps:', '  notes:', '    do: collection.ensure', '    with: { name: notes, fields: { title: { type: text, required: true } } }', ''].join('\n');

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', 'Shop', '--without', 'fleet');
  await cli('project', 'create', 'Yard', '--bare');
  for (const title of ['Add a cart', 'Add a till', 'Add a shelf']) await cli('board', 'add', 'shop', title, '--status', 'ready');
  const notes = join(home, 'notes.bp');
  await writeFile(notes, NOTES);
  await cli('apply', notes, '--project', 'shop');
  await cli('record', 'add', 'shop', 'notes', 'title=Opening hours');
  await cli('channel', 'create', 'pantry');
  await cli('post', 'pantry', 'the quince jelly has set');
  const shop = join(data, 'projects', 'shop');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const opened = async (email) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor();
    (globalThis.pagesOpen ??= []).push(page);
    return page;
  };
  const go = async (page, hash) => page.evaluate((to) => (location.hash = to), hash);
  const page = await opened('ada@example.com');
  const other = await opened('ada@example.com');
  // The other page is never read again: what it shows comes by what is said.
  let reread = 0;
  // A page loaded again, and not the address bar following the tab showing, which the browser calls a navigation too.
  other.on('domcontentloaded', () => reread++);
  const side = async (on, name) => {
    const button = on.locator('.rail').getByRole('button', { name, exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
    if (name === 'Resources') await unfoldAll(on);
  };
  const trashLine = (on) => on.getByRole('navigation', { name: 'Resources' }).getByRole('button', { name: /Trash$/ });
  await side(other, 'Resources');
  await trashLine(other).click();
  await other.getByRole('heading', { name: 'Trash', level: 1 }).waitFor();
  await other.getByText('The Trash is empty').waitFor();

  // A card, deleted from its drawer, says where it went, with Undo, which puts it back in its column.
  await go(page, '/board/shop');
  const card = (title) => page.locator('.card', { hasText: title });
  await card('Add a till').click();
  await drawerAtRest(page).getByRole('button', { name: 'Delete', exact: true }).click();
  const moved = page.locator('.toast', { hasText: 'Add a till: Moved to Trash, kept 30 days.' });
  await moved.waitFor();
  await card('Add a till').waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/trashed-0-card-moved.png` });
  await moved.getByRole('button', { name: 'Undo', exact: true }).click();
  await card('Add a till').waitFor();
  const order = await page.locator('.column', { hasText: 'ready' }).locator('.card').allInnerTexts();
  expect(`the card back between the other two, got ${order.join(' | ')}`, order.length === 3 && order[1].includes('Add a till'));
  // Deleted and given back again while what was said the first time is still showing: each sentence is said once,
  // by the newest saying of it, whose Undo is the one that takes back what was done last.
  const back = page.locator('.toast', { hasText: 'Add a till is back.' });
  await back.waitFor();
  await card('Add a till').click();
  await drawerAtRest(page).getByRole('button', { name: 'Delete', exact: true }).click();
  await card('Add a till').waitFor({ state: 'detached' });
  expect(`one toast saying where it went, got ${await moved.count()}`, (await moved.count()) === 1);
  // What is showing now is marked, waiting for nothing: the earlier one may have gone by itself already.
  await page.evaluate(() => document.querySelectorAll('.toast').forEach((said) => (said.dataset.said = 'before')));
  await moved.getByRole('button', { name: 'Undo', exact: true }).click();
  await card('Add a till').waitFor();
  // The second saying is there once it is drawn, as one more or in the first one's place.
  await page.locator('.toast:not([data-said])', { hasText: 'Add a till is back.' }).waitFor();
  expect(`one toast saying it is back, got ${await back.count()}`, (await back.count()) === 1);
  await card('Add a till').click();
  await drawerAtRest(page).getByRole('button', { name: 'Delete', exact: true }).click();
  await card('Add a till').waitFor({ state: 'detached' });

  // A row of a table, from its form.
  await go(page, '/table/shop/notes');
  const row = page.locator('tr[data-row="shop-notes-1"]');
  await row.hover();
  await page.getByRole('button', { name: 'Open Opening hours', exact: true }).click();
  await page.getByRole('button', { name: 'Remove this note', exact: true }).click();
  await page.locator('.toast', { hasText: 'Opening hours: Moved to Trash, kept 30 days.' }).waitFor();
  await row.waitFor({ state: 'detached' });

  // A file, from its gear, behind a question that says it goes to the Trash.
  await go(page, '/file/shop/docs/overview.md');
  await page.getByRole('heading', { name: 'overview.md', level: 1 }).waitFor();
  await page.locator('.view-head .head-acts').getByRole('button', { name: 'More for this page', exact: true }).click();
  await page.locator('.row-menu').getByRole('menuitem', { name: 'Delete this file…', exact: true }).click();
  const asks = page.getByRole('dialog');
  await asks.getByRole('heading', { name: 'Move overview.md to the Trash?' }).waitFor();
  await page.screenshot({ path: `${out}/trashed-1-file-asked.png` });
  await asks.getByRole('button', { name: 'Move to Trash', exact: true }).click();
  await page.locator('.toast', { hasText: 'overview.md: Moved to Trash, kept 30 days.' }).waitFor();
  expect('the file out of the project folder', await stat(join(shop, 'docs/overview.md')).then(() => false, () => true));

  // A project, from its row's menu, set apart at its foot and behind a question that says what happens.
  await side(page, 'Resources');
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const projectRow = (id) => resources.locator(`[data-project="${id}"] > .nav-row`);
  await projectRow('yard').hover();
  await projectRow('yard').getByRole('button', { name: 'More for Yard', exact: true }).click();
  const lines = (await page.getByRole('menu').getByRole('menuitem').allInnerTexts()).map((line) => line.trim());
  expect(`the Trash the last line of a project's menu, got ${lines.join(' | ')}`, lines.at(-1) === 'Move to Trash…');
  await page.getByRole('menu').getByRole('menuitem', { name: 'Move to Trash…' }).click();
  await asks.getByRole('heading', { name: 'Move Yard to the Trash?' }).waitFor();
  await asks.getByText('kept in the Trash for 30 days', { exact: false }).waitFor();
  const question = (await asks.innerText()).replace(/\s+/g, ' ');
  expect(`the question to say nothing stops, its files stay and how long it is kept, got ${question}`, question.includes('Nothing it runs is stopped') && question.includes('its files stay where they are on the disk') && question.includes('kept in the Trash for 30 days'));
  await page.screenshot({ path: `${out}/trashed-1-project-asked.png` });
  await asks.getByRole('button', { name: 'Move to Trash', exact: true }).click();
  await page.locator('.toast', { hasText: 'Yard: Moved to Trash, kept 30 days.' }).waitFor();
  await projectRow('yard').waitFor({ state: 'detached' });
  expect('its files where they were', (await stat(join(data, 'projects', 'yard'))).isDirectory());

  // A channel, from its row's menu, set apart at its foot and behind a question that says what goes and for how long.
  await side(page, 'Channels');
  const channelsPanel = (on) => on.getByRole('navigation', { name: 'Channels' });
  const pantry = (on) => channelsPanel(on).locator('.grouped-channel', { has: on.getByRole('button', { name: '# pantry', exact: true }) });
  await pantry(page).click({ button: 'right' });
  const channelLines = (await page.getByRole('menu').getByRole('menuitem').allInnerTexts()).map((line) => line.trim());
  expect(`the Trash the last line of a channel's menu, got ${channelLines.join(' | ')}`, channelLines.at(-1) === 'Move to Trash…');
  await page.getByRole('menu').getByRole('menuitem', { name: 'Move to Trash…' }).click();
  await asks.getByRole('heading', { name: 'Move #pantry to the Trash?' }).waitFor();
  await asks.getByText('kept in the Trash for 30 days', { exact: false }).waitFor();
  const goes = (await asks.innerText()).replace(/\s+/g, ' ');
  expect(`the question to say what goes and how long it is kept, got ${goes}`, goes.includes('Its messages go with it') && goes.includes('kept in the Trash for 30 days') && goes.includes('given back from there whole'));
  await page.screenshot({ path: `${out}/trashed-1-channel-asked.png` });
  await asks.getByRole('button', { name: 'Move to Trash', exact: true }).click();
  await page.locator('.toast', { hasText: '#pantry: Moved to Trash, kept 30 days.' }).waitFor();
  await pantry(page).waitFor({ state: 'detached' });

  // Every one in the Trash by kind, on the page that was never read again too.
  for (const on of [other, page]) {
    if (on === page) {
      await side(page, 'Resources');
      await trashLine(page).click();
    }
    const kinds = on.locator('.trash-kind');
    await on.locator('.trash-kind[data-kind="project"] .trash-thing').waitFor();
    await on.locator('.trash-kind[data-kind="file"] .trash-thing').waitFor();
    await on.locator('.trash-kind[data-kind="channel"] .trash-thing').waitFor();
    expect('two rows and cards', (await on.locator('.trash-kind[data-kind="record"] .trash-thing').count()) === 2);
    const headings = await kinds.locator('h2').allInnerTexts();
    expect(`the kinds in order, got ${headings.join(' | ')}`, headings.join(' | ') === 'Rows and cards | Files | Projects | Channels');
    const channelSaid = (await on.locator('.trash-kind[data-kind="channel"] .trash-thing').innerText()).replace(/\s+/g, ' ');
    expect(`the channel by what it is called, got ${channelSaid}`, channelSaid.includes('#pantry') && channelSaid.includes('a channel, with everything said and attached in it'));
    const said = (await on.locator('.trash-thing[data-thing="shop-2"]').innerText()).replace(/\s+/g, ' ');
    expect(`who deleted it and when it goes, got ${said}`, said.includes('deleted by Ada') && said.includes('goes for good in 30 days') && said.includes("a card of Shop's board"));
  }
  expect('the other page never read again', reread === 0);
  await other.screenshot({ path: `${out}/trashed-2-listed.png` });

  // A card given back from there is in its column on the other page.
  await page.locator('.trash-thing[data-thing="shop-2"]').getByRole('button', { name: 'Restore', exact: true }).click();
  await page.locator('.toast', { hasText: 'Add a till is back.' }).waitFor();
  await other.locator('.trash-thing[data-thing="shop-2"]').waitFor({ state: 'detached' });
  await go(other, '/board/shop');
  await other.locator('.card', { hasText: 'Add a till' }).waitFor();
  await side(other, 'Resources');
  await trashLine(other).click();

  // A file whose place has something in it again: refused in the daemon's sentence, and given back beside.
  const wrote = await page.evaluate(async () => {
    const headers = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const body = JSON.stringify({ path: 'docs/overview.md', content: '# Another overview\n' });
    return (await fetch('/api/projects/shop/file', { method: 'PUT', headers, body })).status;
  });
  expect(`a file written at its place, got ${wrote}`, wrote === 200);
  const file = page.locator('.trash-thing[data-thing="docs/overview.md"]');
  await file.getByRole('button', { name: 'Restore', exact: true }).click();
  await file.getByText('something is at `docs/overview.md` again').waitFor();
  await page.screenshot({ path: `${out}/trashed-3-taken.png` });
  await file.getByRole('button', { name: 'Restore beside', exact: true }).click();
  await page.locator('.toast', { hasText: 'is back, at docs/overview (restored).md.' }).waitFor();
  expect('the other one as it was', (await readFile(join(shop, 'docs/overview.md'), 'utf8')) === '# Another overview\n');
  expect('the one given back beside it', (await readFile(join(shop, 'docs/overview (restored).md'), 'utf8')).length > 0);

  // A project given back, and a row deleted for good behind a question.
  await page.locator('.trash-thing[data-thing="yard"]').getByRole('button', { name: 'Restore', exact: true }).click();
  await page.locator('.trash-thing[data-thing="yard"]').waitFor({ state: 'detached' });
  expect('the project back', (await cli('project', 'list')).stdout.includes('yard'));
  // A channel given back whole: listed again on the other page, which is not read again, with what was said in it.
  await page.locator('.trash-kind[data-kind="channel"] .trash-thing').getByRole('button', { name: 'Restore', exact: true }).click();
  await page.locator('.toast', { hasText: '#pantry is back.' }).waitFor();
  await side(other, 'Channels');
  await pantry(other).waitFor();
  await pantry(other).getByRole('button', { name: '# pantry', exact: true }).click();
  await other.getByText('the quince jelly has set').waitFor();
  await side(other, 'Resources');
  await trashLine(other).click();

  // A page of a wiki deleted with the pages under it: one line, said as a page with how many pages it holds, on both
  // pages; given back with them under its page; and, its page deleted after it, given back as a wiki of its own,
  // which is said, from the page that was never read again.
  await cli('topic', 'make', 'Handbook');
  await cli('topic', 'make', 'Deploying', '--under', 'Handbook');
  await cli('topic', 'make', 'To staging', '--under', 'Deploying');
  await cli('topic', 'make', 'Rolling back', '--under', 'Deploying');
  const topicId = async (title) => JSON.parse((await cli('topic', 'show', title, '--json')).stdout).topic.id;
  const [handbookId, deployingId] = [await topicId('Handbook'), await topicId('Deploying')];
  const removed = (await cli('record', 'remove', deployingId)).stdout;
  expect(`the terminal to say the pages under it went with it, got ${removed}`, removed.includes('with the 2 records under it'));
  const deployingLine = (on) => on.locator(`.trash-thing[data-thing="${deployingId}"]`);
  for (const on of [other, page]) {
    await deployingLine(on).waitFor();
    const said = (await deployingLine(on).innerText()).replace(/\s+/g, ' ');
    expect(`the page said with how many pages went with it, got ${said}`, said.includes('Deploying') && said.includes('a page of a wiki, with the 2 pages under it'));
  }
  await other.screenshot({ path: `${out}/trashed-3-page-listed.png` });
  await deployingLine(page).getByRole('button', { name: 'Restore', exact: true }).click();
  await page.locator('.toast', { hasText: 'Deploying is back.' }).waitFor();
  await deployingLine(other).waitFor({ state: 'detached' });
  const under = (await cli('topic', 'show', 'Rolling back')).stdout;
  expect(`its pages back under it, got ${under}`, under.startsWith('in Handbook › Deploying\n'));
  await cli('record', 'remove', deployingId);
  await cli('record', 'remove', handbookId);
  await deployingLine(other).waitFor();
  await other.locator(`.trash-thing[data-thing="${handbookId}"]`).waitFor();
  await deployingLine(other).getByRole('button', { name: 'Restore', exact: true }).click();
  await other.locator('.toast', { hasText: 'The page `Deploying` was under is gone, so `Deploying` is a wiki of its own now.' }).waitFor();
  await other.screenshot({ path: `${out}/trashed-3-page-back.png` });
  const wiki = (await cli('topic', 'show', 'Rolling back')).stdout;
  expect(`a wiki of its own with its pages under it, got ${wiki}`, wiki.startsWith('in Deploying\n'));
  await other.locator(`.trash-thing[data-thing="${handbookId}"]`).getByRole('button', { name: 'Restore', exact: true }).click();
  await page.locator(`.trash-thing[data-thing="${handbookId}"]`).waitFor({ state: 'detached' });
  expect('the other page never read again', reread === 0);

  await page.locator('.trash-thing[data-thing="shop-notes-1"]').getByRole('button', { name: 'Delete forever', exact: true }).click();
  await asks.getByRole('heading', { name: 'Delete Opening hours forever?' }).waitFor();
  await page.screenshot({ path: `${out}/trashed-4-forever.png` });
  await asks.getByRole('button', { name: 'Delete forever', exact: true }).click();
  await page.getByText('The Trash is empty').waitFor();
  await other.getByText('The Trash is empty').waitFor();

  // Something more to draw, and nothing of the page over anything else at three widths and a phone's.
  await cli('board', 'add', 'shop', 'A card with a title long enough to have to give way somewhere in a narrow page', '--status', 'ready');
  const id = JSON.parse((await cli('board', 'shop', '--json')).stdout).items.find((item) => item.title.startsWith('A card with')).id;
  await page.evaluate(async (card) => {
    const headers = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
    await fetch(`/api/board/items/${card}`, { method: 'DELETE', headers });
  }, id);
  // And a page with pages under it, whose line says so, among what is drawn.
  await cli('topic', 'make', 'Rolling forward', '--under', 'Deploying');
  await cli('record', 'remove', deployingId);
  await page.locator(`.trash-thing[data-thing="${deployingId}"]`).waitFor();
  await page.locator('.trash-thing').first().waitFor();
  for (const [width, height] of [
    [1280, 860],
    [900, 800],
    [640, 800],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    const view = page.locator('.trash-page');
    await view.waitFor();
    const found = await view.evaluate((box) => {
      const outer = box.getBoundingClientRect();
      const parts = [...box.querySelectorAll('h1, h2, .trash-kept, .trash-called, .trash-where, .trash-when, button')]
        .map((one) => ({ said: (one.textContent ?? '').trim().slice(0, 30) || one.tagName, box: one.getBoundingClientRect() }))
        .filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of parts.entries()) {
        if (one.box.left < outer.left - 1 || one.box.right > outer.right + 1) wrong.push(`${one.said} leaves the page`);
        for (const two of parts.slice(index + 1)) {
          const over = one.box.left < two.box.right - 1 && two.box.left < one.box.right - 1 && one.box.top < two.box.bottom - 1 && two.box.top < one.box.bottom - 1;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      return { wrong, sideways: document.documentElement.scrollWidth - window.innerWidth };
    });
    expect(`nothing of the page over anything else at ${width}: ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`nothing scrolling sideways at ${width}, got ${found.sideways}`, found.sideways <= 0);
    await page.screenshot({ path: `${out}/trashed-5-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // The Trash is a page of lists, and fills its tab (`filled.mjs`).
  await fillsPage(page, 'the Trash', page.locator('.trash-page > .view-body'), { ready: page.locator('.trash-thing'), shot: `${out}/trashed-filled` });
  // Given back, so that a member below is shown the card alone.
  await page.locator(`.trash-thing[data-thing="${deployingId}"]`).getByRole('button', { name: 'Restore', exact: true }).click();
  await page.locator(`.trash-thing[data-thing="${deployingId}"]`).waitFor({ state: 'detached' });

  // How long, in Settings, and the terminal saying the same.
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const panel = page.locator('.trash-setting');
  await panel.getByRole('heading', { name: 'Trash', level: 2 }).waitFor();
  await panel.locator('.trash-how-long').getByText('kept in the Trash for 30 days').waitFor();
  await panel.getByLabel('What is deleted is kept').selectOption('7');
  await panel.locator('.trash-how-long').getByText('kept in the Trash for 7 days').waitFor();
  const said = (await cli('trash')).stdout;
  expect(`the terminal to say seven days, got ${said}`, said.includes('kept in the Trash for 7 days'));
  await panel.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/trashed-6-setting.png` });

  // A member sees what they could read, and is offered nothing to give back: nothing of a private channel they were not in.
  await cli('tier', 'premium');
  await cli('channel', 'create', 'Winter plans', '--kept');
  const winter = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.find((one) => one.title === 'Winter plans').id;
  await cli('post', winter, 'the medlar count');
  const winterGone = (await cli('channel', 'remove', winter)).stdout;
  expect(`the terminal to say where it went, got ${winterGone}`, winterGone.includes('moved `Winter plans` to the Trash') && winterGone.includes('trash restore'));
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  const theirs = await opened('grace@example.com');
  await side(theirs, 'Resources');
  await trashLine(theirs).click();
  await theirs.locator(`.trash-thing[data-thing="${id}"]`).waitFor();
  expect('the owner to have it in the Trash', (await cli('trash')).stdout.includes('Winter plans'));
  expect('a private channel they were not in shown nowhere in their Trash', (await theirs.locator('.trash-page').innerText()).includes('Winter plans') === false);
  const offered = await theirs.locator('.trash-thing').getByRole('button').count();
  const may = await theirs.evaluate(async () => (await (await fetch('/api/trash', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).trash[0].may);
  expect(`a member offered only what the daemon says they may, got ${offered} buttons and may ${may}`, (offered === 2) === may);
  // The line that moves a project to the Trash is drawn only for whoever the route lets.
  const scopes = await theirs.evaluate(async () => (await (await fetch('/api/me', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).principal.scopes);
  const writes = scopes.includes('*') || scopes.includes('projects') || scopes.includes('projects:write');
  const theirRow = theirs.getByRole('navigation', { name: 'Resources' }).locator('[data-project="shop"] > .nav-row');
  await theirRow.hover();
  await theirRow.getByRole('button', { name: 'More for Shop', exact: true }).click();
  const theirLines = await theirs.getByRole('menu').getByRole('menuitem', { name: 'Move to Trash…' }).count();
  expect(`a member offered the Trash for a project as the route lets them, got ${theirLines} with ${scopes.join(',')}`, (theirLines === 1) === writes);
  await theirs.keyboard.press('Escape');
  await theirs.getByRole('button', { name: 'Settings', exact: true }).click();
  await theirs.getByRole('heading', { name: 'Theme', level: 2 }).waitFor();
  expect('a member not offered how long', (await theirs.locator('.trash-setting').count()) === 0);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  for (const [index, open] of (globalThis.pagesOpen ?? []).entries()) await open.screenshot({ path: `${out}/trashed-failed-${index}.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
