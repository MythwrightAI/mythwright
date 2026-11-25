// The client in a browser, where the browser's tabs are its tabs
// (`src/inplace.ts`): one thing shows at a time, in place.
//
//   DAEMON=../target/debug/factory node scripts/inplace.mjs [out-dir]
//
// This check's pages are told nothing of the desktop window (`windowed.ts`
// leaves it out): they are a browser's. There is no row of tabs. A project,
// its board, a channel and Activity are each opened from the side panel in
// place of what showed, the address bar saying each by its address, one
// view on the page; Back and Forward go between them, each shown again as
// it was left (the board's drawer given back to it); the page read again at
// an address shows that thing. Something half written in a document is
// kept when it is left by Back and is there by Forward, with nothing asked;
// left by a press in place, the reader is asked first, Stay keeping it
// there and Discard them going on and leaving it read anew by Back. A line
// of the side panel pressed with Ctrl, by the middle button, and "Open in a
// new tab" of a line's menu each open the thing in a tab of the browser's
// own, at its address, signed in, the page it was pressed on unchanged. The
// browser's tab says what shows. With the side panel folded away the bar
// over what shows holds what brings it back, and at a phone's width it is
// there always; nothing lies over anything else or scrolls sideways at
// three widths and a phone's. The environment variables are those of
// walkthrough.mjs; `LOOK` as every check.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll, unfoldChannels } from './folded.mjs';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
// Each run of this check has folders of its own, so that several can be run at once.
const own = `inplace-${process.pid}`;
const data = join(out, `${own}-data`);
const home = join(out, `${own}-home`);
const shop = join(out, `${own}-shop`);

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
// The page as it stood when something was not as meant, for whoever reads why.
let shown = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', 'Shop', '--bare', '--path', shop);
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('channel', 'create', 'lobby');
  await cli('board', 'add', 'shop', 'Count the till');
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1360, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  shown = page;
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const listed = async (read, wanted, what) => {
    const until = Date.now() + 8000;
    let got = await read();
    while (got !== wanted && Date.now() < until) {
      await page.waitForTimeout(60);
      got = await read();
    }
    expect(`${what}: "${wanted}", got "${got}"`, got === wanted);
  };
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  expect('the page to be a browser\'s, told nothing of the window', await page.evaluate(() => window.desktop === undefined));
  // A channel's address names it by its id.
  const lobby = await page.evaluate(async () => {
    const asked = await fetch('/api/channels', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return (await asked.json()).channels.find((one) => one.name === 'lobby').id;
  });

  const side = async (name) => {
    const button = page.locator('.rail').getByRole('button', { name, exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
    await page.getByRole('navigation', { name }).waitFor();
    if (name === 'Resources') await unfoldAll(page);
    if (name === 'Channels') await unfoldChannels(page);
  };
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const shopTree = resources.locator('[data-project="shop"]');
  // What the page shows: the path of the address bar and each view on it, by the key of what it is of.
  const where = () => page.evaluate(() => location.pathname);
  const views = () => page.evaluate(() => [...document.querySelectorAll('.view')].map((view) => view.dataset.tab).join(' '));
  const showing = async (path, key, what) => {
    await listed(where, path, `${what}: the address bar`);
    await listed(views, key, `${what}: the one thing on the page`);
  };
  // The browser's history as long as it is: each thing opened in place is an entry of its own.
  const history = () => page.evaluate(() => history.length);

  // ---- no row of tabs, and the start page at the top of the page
  await showing('/', 'home', 'the start page at first');
  expect('no row of tabs in a browser', (await page.locator('[role="tablist"], .tab').count()) === 0);
  await page.screenshot({ path: `${out}/inplace-1-start.png` });

  // ---- opened in place, each by its address
  await side('Resources');
  const before = await history();
  await shopTree.locator(':scope > .nav-row .nav-item').click();
  await showing('/shop', 'project:shop', 'a project\'s page opened in place');
  await unfold(page, 'Shop');
  await shopTree.getByRole('button', { name: /^\W*Board$/ }).click();
  await showing('/shop/board', 'board:shop', 'the board opened in place of the project\'s page');
  expect(`the browser's tab to say what shows, got "${await page.title()}"`, (await page.title()).startsWith('Shop board · '));
  // A card's drawer opened beside the board, to be given back to it.
  await page.locator('.view .card', { hasText: 'Count the till' }).first().click();
  await drawerAtRest(page).waitFor();
  await side('Channels');
  await page.getByRole('navigation', { name: 'Channels' }).getByRole('button', { name: /^\W*lobby$/ }).click();
  await showing(`/channels/${lobby}`, `channel:${lobby}`, 'a channel opened in place, the drawer put away');
  expect('the board\'s drawer put away with the board', (await page.locator('.drawer').count()) === 0);
  await side('Resources');
  await resources.getByRole('button', { name: /^\W*Activity$/ }).click();
  await showing('/activity', 'activity', 'Activity opened in place');
  expect(`an entry of the browser's history for each, got ${(await history()) - before}`, (await history()) - before === 4);
  await page.screenshot({ path: `${out}/inplace-2-in-place.png` });

  // ---- Back and Forward, each shown again as it was left
  await page.goBack();
  await showing(`/channels/${lobby}`, `channel:${lobby}`, 'Back to the channel');
  await page.goBack();
  await showing('/shop/board', 'board:shop', 'Back to the board');
  await drawerAtRest(page).waitFor();
  expect('the board\'s drawer given back to it, simply there', (await page.locator('.drawer').evaluate((drawer) => drawer.getAnimations().length)) === 0);
  await page.goBack();
  await showing('/shop', 'project:shop', 'Back to the project\'s page');
  await page.goBack();
  await showing('/', 'home', 'Back to the start page, at the top of the page');
  await page.goForward();
  await showing('/shop', 'project:shop', 'Forward to the project\'s page');
  await page.goForward();
  await showing('/shop/board', 'board:shop', 'Forward to the board');
  await page.goForward();
  await showing(`/channels/${lobby}`, `channel:${lobby}`, 'Forward to the channel');
  await page.goForward();
  await showing('/activity', 'activity', 'Forward to Activity');
  // The page read again at an address shows that thing.
  await page.goto(`${url}/shop/board`);
  await showing('/shop/board', 'board:shop', 'the page read again at the board\'s address');

  // ---- what is half written, kept by Back and asked about by a press in place
  await page.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const put = await fetch('/api/projects/shop/file', { method: 'PUT', headers: mine, body: JSON.stringify({ path: 'notes.md', content: '# Notes\n\nNothing yet.\n' }) });
    if (!put.ok) throw new Error('could not write notes.md');
  });
  // Opened in place from the panel, so that Back and Forward stay within the one page and what is half written with it.
  await side('Resources');
  await unfold(page, 'Shop repo');
  await shopTree.getByRole('button', { name: /notes\.md$/ }).click();
  await showing('/shop/repo/notes.md', 'file:shop:notes.md', 'a document opened in place');
  await page.locator('.view .writer h1', { hasText: 'Notes' }).waitFor();
  await page.locator('.view .writer h1', { hasText: 'Notes' }).click();
  await page.keyboard.press('End');
  await page.waitForTimeout(150);
  await page.keyboard.type(' of the shop');
  await page.locator('.placebar, .view').first().waitFor();
  await listed(() => page.evaluate(() => document.querySelectorAll('.view .writer h1')[0]?.textContent ?? ''), 'Notes of the shop', 'what was typed');
  // Left by Back nobody is asked, and Forward finds it as it was.
  await page.goBack();
  await showing('/shop/board', 'board:shop', 'Back from the document with something half written in it');
  expect('nobody asked anything by Back', (await page.getByRole('dialog').count()) === 0);
  await page.goForward();
  await page.locator('.view .writer h1', { hasText: 'Notes of the shop' }).waitFor();
  // Left by a press in place, the reader is asked first.
  await side('Resources');
  await resources.getByRole('button', { name: /^\W*Activity$/ }).click();
  const asked = page.getByRole('dialog');
  await asked.getByText('has changes that are not saved').waitFor();
  await page.screenshot({ path: `${out}/inplace-3-asked-before-leaving.png` });
  const inDialog = await pageCrowded(page, '.modal');
  expect(`nothing of the question over anything else of it or outside the window: ${inDialog.join('; ')}`, inDialog.length === 0);
  await asked.getByRole('button', { name: 'Stay', exact: true }).click();
  await showing('/shop/repo/notes.md', 'file:shop:notes.md', 'the document still showing once Stay is pressed');
  await page.locator('.view .writer h1', { hasText: 'Notes of the shop' }).waitFor();
  await resources.getByRole('button', { name: /^\W*Activity$/ }).click();
  await asked.getByRole('button', { name: 'Discard them', exact: true }).click();
  await showing('/activity', 'activity', 'Activity, once what was written is discarded');
  await page.goBack();
  await page.locator('.view .writer h1', { hasText: /^Notes$/ }).waitFor();

  // ---- a new tab of the browser's own: Ctrl, the middle button, and the line's menu
  const opened = async (press, path, what) => {
    const pages = context.pages().length;
    const there = await where();
    const [other] = await Promise.all([context.waitForEvent('page'), press()]);
    await other.waitForLoadState();
    await other.locator('.rail').waitFor();
    const until = Date.now() + 8000;
    while ((new URL(other.url()).pathname !== path || (await other.locator('.view').count()) !== 1) && Date.now() < until) await other.waitForTimeout(60);
    expect(`${what} at ${path}, got ${new URL(other.url()).pathname}`, new URL(other.url()).pathname === path);
    expect(`${what} signed in and showing it`, (await other.locator('.view').count()) === 1);
    expect(`the page it was pressed on unchanged by ${what}, at ${await where()}`, (await where()) === there);
    await other.close();
    expect('the new tab closed', context.pages().length === pages);
  };
  await opened(() => shopTree.getByRole('button', { name: /^\W*Board$/ }).click({ modifiers: ['Control'] }), '/shop/board', 'the board opened by Ctrl');
  await opened(() => resources.getByRole('button', { name: /^\W*Activity$/ }).click({ button: 'middle' }), '/activity', 'Activity opened by the middle button');
  // From the menu at the end of a line of Recents.
  const recentBoard = resources.locator('.lately-recent .lately-line', { hasText: 'Board' }).first();
  await recentBoard.hover();
  await recentBoard.getByRole('button', { name: /^More for / }).click();
  await opened(() => page.getByRole('menuitem', { name: 'Open in a new tab' }).click(), '/shop/board', 'the board opened from its line\'s menu');

  // ---- the bar over what shows, folded away and on a phone, by the boxes
  for (const width of [1360, 1000, 760]) {
    await page.setViewportSize({ width, height: 860 });
    await page.waitForTimeout(150);
    const crowded = await pageCrowded(page);
    expect(`nothing over anything else at ${width} wide: ${crowded.join('; ')}`, crowded.length === 0);
    await page.screenshot({ path: `${out}/inplace-4-${width}.png` });
  }
  await page.setViewportSize({ width: 1360, height: 860 });
  await page.waitForTimeout(150);
  // A narrow window may have folded the side panel away: it is brought back as a person would.
  await side('Resources');
  await sideSettled(page);
  expect('no bar over what shows while the side panel is there', (await page.locator('.placebar').count()) === 0);
  await page.getByRole('button', { name: 'Collapse the side panel' }).first().click();
  await sideSettled(page);
  await page.locator('.placebar').getByRole('button', { name: 'Expand the side panel' }).waitFor();
  const crowded = await pageCrowded(page);
  expect(`nothing over anything else with the side panel folded away: ${crowded.join('; ')}`, crowded.length === 0);
  await page.screenshot({ path: `${out}/inplace-5-folded.png` });
  await page.locator('.placebar').getByRole('button', { name: 'Expand the side panel' }).click();
  await sideSettled(page);
  await page.setViewportSize({ width: 390, height: 780 });
  await page.waitForTimeout(200);
  await page.locator('.placebar').getByRole('button', { name: 'Expand the side panel' }).waitFor();
  const phoned = await pageCrowded(page);
  expect(`nothing over anything else at a phone's width: ${phoned.join('; ')}`, phoned.length === 0);
  await page.screenshot({ path: `${out}/inplace-6-phone.png` });
  expect(`no errors on the page: ${problems.join('; ')}`, problems.length === 0);
  await browser.close();
} catch (err) {
  failed = err;
  await shown?.screenshot({ path: `${out}/inplace-failed.png` }).catch(() => undefined);
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
