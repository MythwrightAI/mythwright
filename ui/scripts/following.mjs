// Following a wiki with its pages from the page's head, being told when a
// page under it changes, how often in Settings → Following, and Unfollow.
//
//   DAEMON=../target/debug/factory node scripts/following.mjs [out-dir]
//
// Two people: Ada, who owns the factory and works from a terminal; and
// Nell, on a page of her own. Nell opens a wiki, presses Follow in its
// head and chooses This page and the pages under it: the button says
// Following, lit. Ada changes a page under it from the terminal, and
// Nell's inbox has one line of it, opening the page that changed. In
// Settings, Following lists the wiki as a page with its pages, at once;
// set to Daily it offers a time, and her terminal says daily at 09:00.
// The project's board is followed from its head too, and listed. Unfollow
// takes each away, and nothing is left. On a phone's width the Follow
// button is a bell at the end of the head, and the list of what she
// follows is in the window; nothing it looks at lies over anything else
// on either page, plainly and with `LOOK="size=largest density=compact"`,
// dark or with `SCHEME=light`. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'following-data');
const home = join(out, 'following-home');
const hers = join(out, 'following-home-nell');
const scheme = process.env.SCHEME ?? 'dark';
const looked = `${process.env.LOOK ? '-looked' : ''}${scheme === 'light' ? '-light' : ''}`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config') });
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  const wiki = JSON.parse((await cli('topic', 'make', 'Frogs: a field guide', '--json')).stdout).id;
  const tree = JSON.parse((await cli('topic', 'make', 'Tree frogs', '--under', wiki, '--json')).stdout).id;

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
  const signedIn = async (email, viewport) => {
    const context = await browser.newContext({ viewport, colorScheme: scheme });
    const page = await context.newPage();
    lastPage = page;
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const at = (page, hash) => page.evaluate((to) => (location.hash = to), hash);
  const crowded = async (page, within, what) => {
    const wrong = await pageCrowded(page, within);
    expect(`nothing of ${within} over anything else ${what}${looked}, got ${wrong.join('; ')}`, wrong.length === 0);
  };
  const inWindow = async (page, locator, what) => {
    const box = await locator.boundingBox();
    const width = page.viewportSize().width;
    expect(`${what} in the window, got ${JSON.stringify(box)} of ${width}`, box && box.x >= -0.5 && box.x + box.width <= width + 0.5);
  };

  // ── Nell follows the wiki with its pages, from its head ────────────────
  const page = await signedIn('nell@example.com', { width: 1280, height: 860 });
  await at(page, `#/topic/${encodeURIComponent(wiki)}`);
  const head = page.locator('.topic-page-head');
  const bell = head.locator('.head-follow');
  await bell.waitFor();
  expect('Follow said, not pressed', (await bell.getAttribute('aria-pressed')) === 'false' && (await bell.innerText()).includes('Follow'));
  await crowded(page, '.topic-page-head', 'in the wiki page head at 1280');
  await bell.click();
  const menu = page.getByRole('menu', { name: 'Follow' });
  await menu.waitFor();
  const offered = (await menu.getByRole('menuitem').allInnerTexts()).map((one) => one.split('\n')[0].trim());
  expect(`This page and This page and the pages under it offered, got ${JSON.stringify(offered)}`, offered.includes('This page') && offered.includes('This page and the pages under it'));
  await page.screenshot({ path: `${out}/following-0-offered${looked}.png` });
  await menu.getByRole('menuitem', { name: /^This page and the pages under it/ }).click();
  await page.locator('.topic-page-head .head-follow[aria-pressed="true"]').waitFor();
  expect('Following said once followed', (await bell.innerText()).includes('Following'));
  await page.screenshot({ path: `${out}/following-1-following${looked}.png` });

  // ── A page under it changes: one line in her box ───────────────────────
  await cli('record', 'set', tree, 'body=They climb.');
  await page.locator('.rail .rail-inbox[aria-label="Inbox, 1 unread"]').waitFor();
  await page.locator('.rail .rail-inbox').click();
  const line = page.locator('.inbox-line[data-kind="followed"]');
  await line.waitFor();
  expect('one line of the wiki', (await line.count()) === 1);
  const says = (await line.innerText()).replace(/\s+/g, ' ');
  expect(`the line to say the page under it, got "${says}"`, says.includes('Tree frogs') && says.includes('Revised the page'));
  await page.screenshot({ path: `${out}/following-2-told${looked}.png` });
  await line.locator('.inbox-open').click();
  await page.locator(`.topic-page[data-topic="${tree}"]`).waitFor();

  // ── The board from its head ─────────────────────────────────────────────
  await at(page, '#/board/shop');
  const boardBell = page.locator('.head-acts .head-follow');
  await boardBell.waitFor();
  await boardBell.click();
  await page.locator('.head-acts .head-follow[aria-pressed="true"]').waitFor();

  // ── Settings → Following: daily, then unfollow ──────────────────────────
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
  const panel = page.locator('.following-panel');
  await panel.waitFor();
  const wikiLine = panel.locator(`.following-line[data-thing="page:${wiki}"]`);
  await wikiLine.waitFor();
  const listed = (await wikiLine.innerText()).replace(/\s+/g, ' ');
  expect(`the wiki listed with its pages, got "${listed}"`, listed.includes('Wiki page and its pages') && listed.includes('Frogs: a field guide'));
  await panel.locator('.following-line[data-thing="board:shop/board"]').waitFor();
  await panel.scrollIntoViewIfNeeded();
  await crowded(page, '.following-panel', 'in Settings at 1280');
  await wikiLine.getByRole('combobox').selectOption('daily');
  await wikiLine.locator('input[type="time"]').waitFor();
  const said = (await nell('following')).stdout;
  expect(`her terminal to say daily at 09:00, got ${said}`, said.includes('daily at 09:00') && said.includes('with every page under it'));
  await page.screenshot({ path: `${out}/following-3-settings${looked}.png` });

  // ── At a phone's width ──────────────────────────────────────────────────
  const phone = await signedIn('nell@example.com', { width: 390, height: 844 });
  await at(phone, `#/topic/${encodeURIComponent(wiki)}`);
  const phoneBell = phone.locator(`.topic-page[data-topic="${wiki}"] .topic-page-head .head-follow[aria-pressed="true"]`);
  await phoneBell.waitFor();
  await inWindow(phone, phoneBell, 'the Follow bell at a phone');
  await crowded(phone, `.topic-page[data-topic="${wiki}"] .topic-page-head`, 'in the wiki page head at a phone');
  await phone.screenshot({ path: `${out}/following-4-phone-head${looked}.png` });
  await phone.goto(`${url}/settings`);
  const phonePanel = phone.locator('.following-panel');
  await phonePanel.waitFor();
  await phonePanel.locator('.following-line').first().waitFor();
  await phonePanel.scrollIntoViewIfNeeded();
  for (const one of await phonePanel.locator('.following-line').all()) await inWindow(phone, one, 'a line of what she follows at a phone');
  await crowded(phone, '.following-panel', 'in Settings at a phone');
  await phone.screenshot({ path: `${out}/following-5-phone-settings${looked}.png` });

  // ── Unfollow both ───────────────────────────────────────────────────────
  for (const thing of [`page:${wiki}`, 'board:shop/board']) {
    await panel.locator(`.following-line[data-thing="${thing}"]`).getByRole('button', { name: 'Unfollow' }).click();
    await panel.locator(`.following-line[data-thing="${thing}"]`).waitFor({ state: 'detached' });
  }
  await panel.locator('.following-none').waitFor();
  expect('nothing followed in her terminal either', (await nell('following')).stdout.includes('you follow nothing'));

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/following-failed${looked}.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
