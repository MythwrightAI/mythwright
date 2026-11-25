// The Topics panel as one tree of wikis: each wiki a
// fold at the top, its pages under it to any depth, each page that holds
// pages a fold of its own, a closed page faint where it is and a closed
// wiki under Closed (`views/wikitree.tsx`, over `listedWikis` and
// `movesFor` in `src/wikis.ts`).
//
//   DAEMON=../target/debug/factory node scripts/wikis.mjs [out-dir]
//
// Ada, who owns the factory, on two pages of her own, the second never
// read again, and on a third told she may only read records. Two wikis made
// from a terminal, one a project's, with two pages under the first: drawn
// in the order they were made, the first folded as it starts, its pages
// under it unfolded by its arrow. The `+` of a page's line makes a page
// under it at once, called Untitled, unfolds the line and opens the page,
// with the title in a field in the line, selected: written and saved by
// Enter, and left Untitled by Escape; a page three deep, one made from a
// terminal among them, each drawn on the second page. Each line of a
// page's menu: New page as its `+`; Rename… in a small dialog, a title
// that is nothing refused before anything is asked; Move to… in a dialog
// with a box (the top and every page of the workspace's wikis at its
// depth, where it is marked, the page and what is under it not offered,
// narrowed by what is typed and Enter taking the first), moving a page
// under another of its wiki, under another wiki and to the top as a wiki
// of its own; a wiki moved to another workspace behind the question that
// says how many pages go with it, listed there once that workspace is
// chosen; and Move to Trash, said with Undo, gone on both pages. A page
// closed from a terminal faint where it is, and a wiki closed under
// Closed; a page made standing saying so, and nothing once its status is
// taken away (every page made here has none, and says none). The panel's box finding a page by its title and by the title of
// a page above it, held open, and Escape leaving the folds as they were;
// Collapse all and Expand all. Somebody who may only read records offered
// no New topic, no `+` and no menu. Every change seen on the second page
// with nothing read again. Nothing of the panel or of a dialog over
// anything else, scrolling sideways or out of one tree at 1280, 1000 and
// 800 wide, dragged as narrow as it goes and at a phone's, touched,
// plainly and with `LOOK="size=largest density=compact"`; on a phone the
// title of a page made by the `+` written in its line before the page is
// opened, which puts the panel away. A page made by the `+` is an article,
// opened on Article with its post to write in; a reply to it from a
// terminal is counted by Talk and drawn there. (Open in a new
// tab is a browser's line alone, and is held by `inplace.mjs`.)
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';
import { joined, namesCut } from './joined.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'wikis-data');
const home = join(out, 'wikis-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const made = async (...args) => (await cli('topic', 'make', ...args)).stdout.match(/@-topics-\d+/)[0];

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', 'Shop', '--bare');
  const handbook = await made('Handbook');
  await made('Getting started', '--under', handbook);
  const deploying = await made('Deploying', '--under', handbook);
  await made('Payment provider options', '--project', 'shop');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const scheme = process.env.SCHEME ?? 'dark';
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  // `holds`, where it is said, is what the page is told Ada holds: somebody who may read records and change nothing.
  const signedIn = async (how = { viewport: { width: 1280, height: 860 } }, holds = null) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    if (holds)
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = holds;
        await route.fulfill({ response: answer, json: me });
      });
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const toPanel = async (on, name = 'Topics', tap = false) => {
    const rail = on.locator('.rail').getByRole('button', { name, exact: true });
    await rail.waitFor();
    if ((await rail.getAttribute('aria-pressed')) !== 'true') await (tap ? rail.tap() : rail.click());
    await on.getByRole('navigation', { name, exact: true }).waitFor();
  };
  const page = await signedIn();
  const other = await signedIn();
  await toPanel(page);
  await toPanel(other);
  const panel = page.getByRole('navigation', { name: 'Topics', exact: true });
  const there = other.getByRole('navigation', { name: 'Topics', exact: true });

  // What the panel draws, a line a thing: each page by how deep it is (a `-` a level), ▾ or ▸ where it holds pages
  // and · where it holds none, its title (in brackets while it is written), what is said beside it, and `~` where
  // it is drawn faint; Closed with its fold and how many; and what is said where there is nothing.
  const DRAWN = `(() => {
    const nav = document.querySelector('nav[aria-label="Topics"] .nav');
    if (!nav) return 'no panel';
    const said = [];
    for (const one of nav.querySelectorAll('.topic-line, .nav-item[aria-expanded]:not(.topic-line *), .topics-first, .nav-who, .panel-none')) {
      if (one.matches('.topic-line')) {
        let depth = 0;
        for (let at = one.parentElement.closest('.tree'); at && nav.contains(at); at = at.parentElement.closest('.tree')) depth += 1;
        const twist = one.querySelector(':scope > .nav-row > .nav-twist');
        const mark = twist ? (twist.getAttribute('aria-expanded') === 'true' ? '▾' : '▸') : '·';
        const item = one.querySelector(':scope > :is(.nav-row, .nav-line) > .nav-item');
        const input = item.querySelector('input');
        const hint = item.querySelector('.nav-hint');
        said.push('-'.repeat(depth) + mark + (input ? '[' + input.value + ']' : item.querySelector('.truncate').textContent) + (hint ? '(' + hint.textContent + ')' : '') + (one.matches('.closed') ? '~' : ''));
      } else if (one.matches('.nav-item[aria-expanded]')) {
        said.push((one.getAttribute('aria-expanded') === 'true' ? '▾' : '▸') + one.querySelector('.truncate').textContent + '[' + (one.querySelector('.nav-hint')?.textContent ?? '') + ']');
      } else said.push(one.textContent.trim());
    }
    return said.join(' | ');
  })()`;
  const drawn = (on) => on.evaluate(DRAWN);
  const until = async (on, wanted, what) => {
    await on.waitForFunction(`(${DRAWN}) === ${JSON.stringify(wanted)}`, null, { timeout: 10000 }).catch(async () => {
      throw new Error(`expected ${what}: "${wanted}", got "${await drawn(on)}"`);
    });
  };
  const laid = async (on, what) => {
    const wrong = await pageCrowded(on, 'nav[aria-label="Topics"]');
    expect(`nothing of the Topics panel over anything else ${what}: ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await on.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`nothing to scroll sideways ${what}, got ${sideways}px`, sideways <= 0);
    const loose = await joined(on, 'nav[aria-label="Topics"]');
    expect(`the Topics panel to be one tree ${what}: ${loose.join('; ')}`, loose.length === 0);
    const cut = await namesCut(on, 'nav[aria-label="Topics"]');
    expect(`no title cut to nothing ${what}: ${cut.join('; ')}`, cut.length === 0);
  };
  // A page's own line, by its title alone, wherever it is in the tree.
  const lineOf = (nav, title) => nav.locator('.topic-line').filter({ has: nav.page().locator(':scope > :is(.nav-row, .nav-line) > .nav-item .truncate', { hasText: new RegExp(`^${title}$`) }) });
  const menuOf = async (nav, title, line) => {
    const row = lineOf(nav, title);
    await row.locator(':scope > :is(.nav-row, .nav-line) .nav-item').first().hover();
    await row.locator(':scope > :is(.nav-row, .nav-line) > .row-more').first().click();
    const menu = nav.page().getByRole('menu');
    await menu.waitFor();
    if (line) await menu.getByRole('menuitem', { name: line, exact: true }).click();
    return menu;
  };
  const plusOf = async (nav, title) => {
    const row = lineOf(nav, title);
    await row.locator(':scope > :is(.nav-row, .nav-line) .nav-item').first().hover();
    await row.locator(':scope > :is(.nav-row, .nav-line) > .wiki-add').click();
  };
  const field = panel.getByRole('textbox', { name: 'Page title', exact: true });
  const written = async (title) => {
    await field.waitFor();
    await page.waitForFunction(() => document.activeElement?.matches('.wiki-title-field') && document.activeElement.selectionStart === 0 && document.activeElement.selectionEnd === document.activeElement.value.length);
    if (title === null) await page.keyboard.press('Escape');
    else {
      await page.keyboard.type(title);
      await page.keyboard.press('Enter');
    }
    await field.waitFor({ state: 'detached' });
  };

  // ── The tree, in the order the wikis were made, folded as it starts ──────────
  await until(page, '▸Handbook | ·Payment provider options(Shop)', 'each wiki in the order it was made, the one with pages folded as it starts');
  await until(other, '▸Handbook | ·Payment provider options(Shop)', 'the same on the second page');
  await panel.getByRole('button', { name: 'Unfold Handbook', exact: true }).click();
  await until(page, '▾Handbook | -·Getting started | -·Deploying | ·Payment provider options(Shop)', 'its pages under it, unfolded by its arrow');
  await laid(page, 'with a wiki unfolded');
  expect('a fold kept for the page as the panel names it', (await page.evaluate(() => localStorage.getItem('client.turned'))).includes(`channels:topics:page:${handbook}`));

  // ── The `+`: a page made at once, its title written in its line ─────────────
  await plusOf(panel, 'Deploying');
  await page.waitForFunction((said) => [...document.querySelectorAll('.topic-page-title')].some((one) => (one.value ?? one.innerText) === said), 'Untitled');
  expect('no dialog asked for it', (await page.getByRole('dialog').count()) === 0);
  await until(page, '▾Handbook | -·Getting started | -▾Deploying | --·[Untitled] | ·Payment provider options(Shop)', 'the page made under it, the line unfolded, its title in a field');
  await page.screenshot({ path: `${out}/wikis-01-written.png` });
  await written('Rolling back');
  await page.waitForFunction((said) => [...document.querySelectorAll('.topic-page-title')].some((one) => (one.value ?? one.innerText) === said), 'Rolling back');
  await page.getByRole('tab', { name: /Rolling back/ }).first().waitFor();
  const deep = '▾Handbook | -·Getting started | -▾Deploying | --·Rolling back | ·Payment provider options(Shop)';
  await until(page, deep, 'its title saved by Enter');
  // A wiki's page is an article: it opens on Article, ready to write, and its replies are on Talk, which counts them.
  const opened = page.locator('.topic-page', { has: page.locator('.topic-path-page', { hasText: 'Deploying' }) });
  const tabsOf = opened.getByRole('tablist', { name: 'Article, talk and history' });
  await tabsOf.getByRole('tab', { name: 'Article', exact: true, selected: true }).waitFor();
  expect('its article to write in, empty', (await opened.locator('.topic-post .ProseMirror').count()) === 1);
  await cli('topic', 'reply', 'Rolling back', 'Which tag is the last good one?');
  const talkTab = tabsOf.getByRole('tab', { name: /^Talk/ });
  await page.waitForFunction((one) => one.innerText.replace(/\s+/g, ' ') === 'Talk 1', await talkTab.elementHandle());
  await talkTab.click();
  await opened.locator('.topic-talk .topic-reply', { hasText: 'Which tag is the last good one?' }).waitFor();
  await page.screenshot({ path: `${out}/wikis-07-talk.png` });
  await tabsOf.getByRole('tab', { name: 'Article', exact: true }).click();
  // Three deep, from the `+` and left Untitled by Escape, and one more from a terminal.
  await plusOf(panel, 'Rolling back');
  await written(null);
  await made('To staging', '--under', deploying);
  const three = '▾Handbook | -·Getting started | -▾Deploying | --▾Rolling back | ---·Untitled | --·To staging | ·Payment provider options(Shop)';
  await until(page, three, 'a page three deep left Untitled by Escape, and one made from a terminal after the pages there are');
  await there.getByRole('button', { name: 'Unfold Handbook', exact: true }).click();
  await there.getByRole('button', { name: 'Unfold Deploying', exact: true }).click();
  await there.getByRole('button', { name: 'Unfold Rolling back', exact: true }).click();
  await until(other, three, 'each drawn on the second page, never read again');
  await laid(page, 'three deep');
  await page.mouse.move(900, 700);
  await page.screenshot({ path: `${out}/wikis-02-three-deep.png` });

  // ── The menu: New page, Rename…, Move to…, Move to Trash ───────────────────
  const menu = await menuOf(panel, 'Getting started');
  const lines = await menu.getByRole('menuitem').evaluateAll((all) => all.map((one) => one.textContent.trim()));
  expect(`a page's menu to be New page, Rename…, Move to… and Move to Trash, got ${lines.join(', ')}`, lines.join() === 'New page,Rename…,Move to…,Move to Trash');
  const wrongMenu = await pageCrowded(page, '[role="menu"]');
  expect(`nothing of the menu over anything else: ${wrongMenu.join('; ')}`, wrongMenu.length === 0);
  await page.screenshot({ path: `${out}/wikis-03-menu.png` });
  await menu.getByRole('menuitem', { name: 'New page', exact: true }).click();
  await written('First steps');
  await until(page, three.replace('-·Getting started', '-▾Getting started | --·First steps'), 'New page making a page under it, as its `+` does');

  await menuOf(panel, 'Untitled', 'Rename…');
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('heading', { name: 'Rename Untitled' }).waitFor();
  await dialog.getByLabel('Title').fill('   ');
  expect('Save closed for a title that is nothing', await dialog.getByRole('button', { name: 'Save', exact: true }).isDisabled());
  await dialog.getByLabel('Title').fill('Checking it took');
  await dialog.getByRole('button', { name: 'Save', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  const renamed = '▾Handbook | -▾Getting started | --·First steps | -▾Deploying | --▾Rolling back | ---·Checking it took | --·To staging | ·Payment provider options(Shop)';
  await until(page, renamed, 'Rename… saving its title');
  await other.getByRole('button', { name: 'Unfold Getting started', exact: true }).click();
  await until(other, renamed, 'and the second page following');

  // Move to…: the top and every page of the workspace's wikis at its depth, where it is marked, not the page or what is under it.
  await menuOf(panel, 'Deploying', 'Move to…');
  await dialog.getByRole('heading', { name: 'Move Deploying to…' }).waitFor();
  const offered = () =>
    dialog.locator('.wiki-move').evaluateAll((all) => all.map((one) => `${'-'.repeat(Number(getComputedStyle(one).getPropertyValue('--depth') || 0))}${one.querySelector('.truncate').textContent}${one.getAttribute('aria-current') ? '*' : ''}`).join(' | '));
  expect(`Move to… to offer the top and the workspace's pages, Deploying and what is under it not, Handbook marked, got ${await offered()}`, (await offered()) === 'A wiki of its own | Handbook* | -Getting started | --First steps | Payment provider options');
  const wrongMoved = await pageCrowded(page, '[role="dialog"]');
  expect(`nothing of Move to… over anything else: ${wrongMoved.join('; ')}`, wrongMoved.length === 0);
  await page.screenshot({ path: `${out}/wikis-04-move-to.png` });
  await dialog.getByRole('searchbox', { name: 'Find a page' }).fill('payment');
  expect(`the box narrowing what is offered, got ${await offered()}`, (await offered()) === 'Payment provider options');
  await page.keyboard.press('Enter');
  await dialog.waitFor({ state: 'detached' });
  const underOther = '▾Handbook | -▾Getting started | --·First steps | ▾Payment provider options(Shop) | -▾Deploying | --▾Rolling back | ---·Checking it took | --·To staging';
  await until(page, underOther, 'Deploying, with what is under it, moved under another wiki by Enter, and that wiki unfolded for it');
  await menuOf(panel, 'First steps', 'Move to…');
  await dialog.locator('.wiki-move', { hasText: 'Getting started' }).waitFor();
  await dialog.locator('.wiki-move', { hasText: /^Handbook$/ }).click();
  await dialog.waitFor({ state: 'detached' });
  const within = '▾Handbook | -·Getting started | -·First steps | ▾Payment provider options(Shop) | -▾Deploying | --▾Rolling back | ---·Checking it took | --·To staging';
  await until(page, within, 'a page moved within its wiki, at the end of the page it went under');
  await menuOf(panel, 'Checking it took', 'Move to…');
  await dialog.locator('.wiki-move', { hasText: 'A wiki of its own' }).click();
  await dialog.waitFor({ state: 'detached' });
  const toTop = '▾Handbook | -·Getting started | -·First steps | ▾Payment provider options(Shop) | -▾Deploying | --·Rolling back | --·To staging | ·Checking it took(Shop)';
  await until(page, toTop, 'a page moved to the top, a wiki of its own after the others, of the place of the wiki it was in');
  await until(other, toTop.replace('▾Payment provider options(Shop) | -▾Deploying', '▸Payment provider options(Shop)').replace(' | --·Rolling back | --·To staging', ''), 'the second page following every move');

  // ── The panel's box, Collapse all and Expand all ───────────────────────────
  const box = panel.getByRole('searchbox', { name: 'Search these topics', exact: true });
  await box.fill('deploying staging');
  await until(page, '▾Payment provider options(Shop) | -▾Deploying | --·To staging', 'a page found by its title and the page above it, what it is under held open');
  await laid(page, 'with a page looked for');
  await box.fill('rolling');
  await until(page, '▾Payment provider options(Shop) | -▾Deploying | --·Rolling back', 'a page found by its own title');
  await box.press('Escape');
  await until(page, toTop, 'the folds as they were once the box is emptied');
  await panel.getByRole('button', { name: 'Collapse all', exact: true }).click();
  await until(page, '▸Handbook | ▸Payment provider options(Shop) | ·Checking it took(Shop)', 'every page folded by Collapse all');
  await panel.getByRole('button', { name: 'Expand all', exact: true }).click();
  await until(page, toTop, 'every page that holds pages unfolded by Expand all');

  // ── Closed: a page faint where it is, a wiki under Closed ──────────────────
  const closedPage = await lineOf(panel, 'First steps').getAttribute('data-topic');
  const closedWiki = await lineOf(panel, 'Checking it took').getAttribute('data-topic');
  await cli('record', 'set', closedPage, 'status=closed');
  await cli('record', 'set', closedWiki, 'status=closed');
  await lineOf(panel, 'First steps').locator('.nav-hint', { hasText: 'closed' }).waitFor();
  await panel.getByRole('button', { name: 'Expand all', exact: true }).click();
  const closed = '▾Handbook | -·Getting started | -·First steps(closed)~ | ▾Payment provider options(Shop) | -▾Deploying | --·Rolling back | --·To staging | ▾Closed[1] | -·Checking it took(Shop)~';
  await until(page, closed, 'a closed page faint where it is, and a closed wiki under Closed, unfolded by Expand all');
  await laid(page, 'with what is closed');
  await page.mouse.move(900, 700);
  await page.screenshot({ path: `${out}/wikis-05-closed.png` });

  // ── A status given and taken away: said where it is, and nothing once it has none ──
  const getting = await lineOf(panel, 'Getting started').getAttribute('data-topic');
  await cli('record', 'set', getting, 'status=standing');
  await until(page, closed.replace('-·Getting started', '-·Getting started(standing)'), 'a standing page saying so where it is');
  await cli('record', 'set', getting, 'status=');
  await until(page, closed, 'a page whose status was taken away saying nothing of it');

  // ── A wiki moved to another workspace, behind its question ─────────────────
  await cli('workspace', 'create', 'Back office');
  await menuOf(panel, 'Payment provider options', 'Move to…');
  await dialog.locator('.wiki-moves-head', { hasText: 'Another workspace, with the 3 pages under it' }).waitFor();
  await dialog.locator('.wiki-move', { hasText: 'Back office' }).click();
  await dialog.getByRole('heading', { name: 'Move Payment provider options to Back office?' }).waitFor();
  expect('the question saying what goes with it', (await dialog.innerText()).includes("Payment provider options and its 3 pages are listed in Back office from now on, for everybody. They are Shop's no longer."));
  await page.screenshot({ path: `${out}/wikis-06-question.png` });
  await dialog.getByRole('button', { name: 'Move it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  const left = '▾Handbook | -·Getting started | -·First steps(closed)~ | ▾Closed[1] | -·Checking it took(Shop)~';
  await until(page, left, 'the wiki gone from this workspace with its pages');
  await panel.locator('.space-foot .space-pick').click();
  await page.getByRole('menuitemradio', { name: /^Back office(:|$)/ }).click();
  await panel.locator('.space-foot .space-said strong', { hasText: 'Back office' }).waitFor();
  await until(page, '▾Payment provider options | -▾Deploying | --·Rolling back | --·To staging', 'and listed in Back office with its pages, its own now and no project said');
  await panel.locator('.space-foot .space-pick').click();
  await page.getByRole('menuitemradio', { name: /^Ada's workspace(:|$)/ }).click();
  await until(page, left, 'the first workspace again');

  // ── Move to Trash, with Undo ──────────────────────────────────────────────
  await menuOf(panel, 'Getting started', 'Move to Trash');
  await page.locator('.toast', { hasText: 'Getting started: Moved to Trash' }).waitFor();
  const trashed = '▾Handbook | -·First steps(closed)~ | ▾Closed[1] | -·Checking it took(Shop)~';
  await until(page, trashed, 'the page moved to the Trash');
  await until(other, trashed.replace('▾Closed[1] | -·Checking it took(Shop)~', '▸Closed[1]'), 'and gone on the second page');
  // What is given back from the Trash, and with what under it a page goes there, is the Trash's, and not held here.
  expect('Undo offered with it', (await page.locator('.toast', { hasText: 'Getting started' }).getByRole('button', { name: 'Undo' }).count()) === 1);

  // ── Somebody who may only read records ────────────────────────────────────
  const reader = await signedIn(undefined, ['records:read', 'channels:read', 'board:read', 'projects:read', 'blueprints:read']);
  await toPanel(reader);
  const read = reader.getByRole('navigation', { name: 'Topics', exact: true });
  await read.getByRole('button', { name: 'Unfold Handbook', exact: true }).click();
  await until(reader, '▾Handbook | -·First steps(closed)~ | ▸Closed[1]', 'the tree as it is for somebody who may only read');
  expect('no New topic, no `+` and no menu for somebody who may only read', (await read.getByRole('button', { name: 'New topic', exact: true }).count()) === 0 && (await read.locator('.wiki-add').count()) === 0 && (await read.locator('.topic-line .row-more').count()) === 0);
  await reader.context().close();

  // ── By the boxes ─────────────────────────────────────────────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await laid(page, `at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  const edge = await page.locator('.side-edge').boundingBox();
  await page.mouse.move(edge.x + edge.width / 2, edge.y + 300);
  await page.mouse.down();
  await page.mouse.move(60, edge.y + 300, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 180);
  await laid(page, 'dragged as narrow as it goes');
  await page.screenshot({ path: `${out}/wikis-07-narrow.png` });

  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  if ((await phone.locator('nav.sidebar').count()) === 0) await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await toPanel(phone, 'Topics', true);
  const small = phone.getByRole('navigation', { name: 'Topics', exact: true });
  await small.getByRole('button', { name: 'Unfold Handbook', exact: true }).tap();
  await until(phone, '▾Handbook | -·First steps(closed)~ | ▸Closed[1]', "the tree at a phone's width");
  expect('the `+` and the menu drawn under a finger', await small.locator('.topic-line .wiki-add').first().evaluate((one) => getComputedStyle(one).opacity === '1'));
  await laid(phone, "at a phone's width, touched");
  await phone.screenshot({ path: `${out}/wikis-08-phone.png` });
  await lineOf(small, 'Handbook').locator(':scope > .nav-row > .wiki-add').tap();
  const smallField = small.getByRole('textbox', { name: 'Page title', exact: true });
  await smallField.waitFor();
  await phone.waitForFunction(() => document.activeElement?.matches('.wiki-title-field'));
  await laid(phone, "with a title written at a phone's width");

  await phone.keyboard.type('On a phone');
  await phone.keyboard.press('Enter');
  // The panel is put away as the page is opened, once its title is written.
  await phone.waitForFunction((said) => [...document.querySelectorAll('.topic-page-title')].some((one) => (one.value ?? one.innerText) === said), 'On a phone');
  await phone.screenshot({ path: `${out}/wikis-09-phone-made.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
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
