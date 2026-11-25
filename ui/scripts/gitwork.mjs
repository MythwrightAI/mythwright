// A project's Git table, as a card shows it: a made-up
// package's blueprint keeps a table beside the board, a row about each
// card that has a branch, written only from there (`by_hand: false`), as
// the package for a project's hosting service does; the card a row is
// about saying it in its Details under what the table is called
// (`views/cardrows.tsx`, over `cardtabs.ts`), each thing under what its
// field is called, an address a link that opens a tab of its own and words
// that are none not, and the same row in the table's own tab beside
// Details, which offers no new row of a table nobody writes by hand;
// nothing of it for a card no row is about; a cell changed from a terminal
// said there with the page never read again; the table listed in the
// board's gear with its one row and not in the side panel; the row opened
// whole from there; and nothing of it over anything else or past the
// drawer's sides at 1280, 900 and 640 wide and a phone's, touched.
//
//   DAEMON=../target/debug/factory node scripts/gitwork.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs. Nothing here
// reaches the network: the package runs no program, and its addresses are
// ones nothing asks.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { menuOf, tablesRead } from './gear.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'gitwork-data');
const home = join(out, 'gitwork-home');

const PULL = 'http://127.0.0.1:9/acme/shop/pull/12';
const ISSUE = 'http://127.0.0.1:9/acme/shop/issues/3';

/** A package for a service there is not, whose blueprint keeps a table of each card's branch beside the board. */
const PACKAGE = `extension: sample
title: Sample Host
description: Not a real service.
blueprints:
  work: |
    blueprint: sample/work
    description: Keep a table beside the board of each card's branch and pull request.
    permissions: [records, board:read, projects]
    params:
      card: { type: string }
      branch: { type: string }
      pull: { type: string }
      issue: { type: string }
    steps:
      table:
        do: collection.ensure
        with:
          name: git
          title: Git
          beside: board
          about: card
          by_hand: false
          fields:
            card: { type: record, of: board }
            branch: { type: text }
            commits: { type: number }
            pull_request: { type: text }
            state: { type: text }
            checks: { type: text }
            issues: { type: list }
          groups:
            - Branch: [branch, commits]
            - Pull request: [pull_request, state, checks]
            - Issues: [issues]
      row:
        do: record.add
        with:
          collection: git
          fields: { card: "\${params.card}", branch: "\${params.branch}", commits: 2, pull_request: "\${params.pull}", state: open, checks: "1 success, 1 pending, a long name of a check that goes on and on", issues: ["\${params.issue}", "later"] }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};

let failed = null;
let shot = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('board', 'add', 'shop', 'Add a cart');
  await cli('board', 'add', 'shop', 'Write the returns policy');
  await mkdir(join(home, 'sample'), { recursive: true });
  await writeFile(join(home, 'sample', 'extension.yaml'), PACKAGE);
  await cli('extension', 'add', join(home, 'sample'));
  await cli('apply', 'sample/work', '--project', 'shop', '--param', 'card=shop-1', '--param', 'branch=lane/shop-1', '--param', `pull=${PULL}`, '--param', `issue=${ISSUE}`);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const signedIn = async (how) => {
    const context = await browser.newContext({ ...how, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const page = await signedIn({ viewport: { width: 1280, height: 860 } });
  shot = () => page.screenshot({ path: `${out}/gitwork-failed.png` }).catch(() => {});
  let reads = 0;
  page.on('framenavigated', (frame) => frame === page.mainFrame() && (reads += 1));
  const board = async (on, touch = false) => {
    const press = (locator) => (touch ? locator.tap() : locator.click());
    if (touch) await press(on.getByRole('button', { name: 'Expand the side panel' }));
    await sideSettled(on);
    const resources = on.getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await press(resources);
    await unfoldAll(on);
    await press(on.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }));
  };
  await board(page);
  const readsAfterSignIn = reads;

  // ---- the table sits beside the board: in its gear, and not in the side panel ------------------
  const shop = page.locator('.nav [data-project="shop"]');
  expect('the Git table not listed under the project', (await shop.getByText('Git', { exact: true }).count()) === 0);
  const beside = await tablesRead(page);
  expect(`the Git table under Its tables in the board's gear with its one row, got ${JSON.stringify(beside)}`, beside.includes('Git 1 row'));
  await page.keyboard.press('Escape');
  await menuOf(page).waitFor({ state: 'detached' });

  // ---- the card's Details --------------------------------------------------------------------
  await page.locator('.card', { hasText: 'Add a cart' }).click();
  const drawer = drawerAtRest(page);
  const kept = drawer.locator('.card-row[data-table="git"]');
  await kept.getByRole('heading', { name: 'Git', level: 3 }).waitFor();
  // A cell by what its field is called where it is shown (`calledOf`: its name in words).
  const cell = (called) => kept.locator('.about-card-cell').filter({ has: page.locator('dt', { hasText: new RegExp(`^${called}$`) }) });
  await cell('branch').getByText('lane/shop-1', { exact: true }).waitFor();
  expect('the card it is about not said again', (await cell('card').count()) === 0);
  const pull = cell('pull request').getByRole('link');
  expect('the pull request a link where it is read', (await pull.getAttribute('href')) === PULL);
  expect('opening a tab of its own with no hold on this one', (await pull.getAttribute('target')) === '_blank' && (await pull.getAttribute('rel')).includes('noopener'));
  expect('each issue a link, and words that are no address none', (await cell('issues').getByRole('link').count()) === 1 && (await cell('issues').getByText('later', { exact: true }).count()) === 1);
  // The same row is the table's own tab beside Details, which offers no new row of a table nobody writes by hand.
  await drawer.getByRole('tab', { name: 'Git', exact: true }).click();
  await drawer.locator('.about-card .about-card-row[data-row="shop-git-1"]').waitFor();
  expect('no new row offered of a table written only by what keeps it', (await drawer.locator('.about-card-new').count()) === 0);
  await drawer.getByRole('tab', { name: 'Details', exact: true }).click();
  await kept.waitFor();
  await page.screenshot({ path: `${out}/gitwork-1-details.png` });

  // ---- written from a terminal, said with the page never read again ---------------------------
  await cli('record', 'set', 'shop-git-1', 'state=merged');
  await cell('state').getByText('merged', { exact: true }).waitFor();

  // ---- a card no row is about ----------------------------------------------------------------
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });
  await page.locator('.card', { hasText: 'Write the returns policy' }).click();
  await drawerAtRest(page).getByLabel('Title').waitFor();
  expect('nothing of the table in the Details of a card no row is about', (await drawerAtRest(page).locator('.card-row').count()) === 0);
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });

  // ---- by the boxes ----------------------------------------------------------------------------
  const laidOut = async (on, what) => {
    const wrong = await on.evaluate(() => {
      const said = [];
      const rows = document.querySelector('.drawer .card-rows');
      if (!rows) return ['there are no rows in the Details'];
      const edge = rows.getBoundingClientRect();
      const cells = [...rows.querySelectorAll('dt, dd, h3, .about-card-head > *')].map((one) => ({ one, box: one.getBoundingClientRect() }));
      for (const [index, { one, box }] of cells.entries()) {
        if (box.left < edge.left - 0.5 || box.right > edge.right + 0.5) said.push(`${one.textContent.trim().slice(0, 30)} leaves the rows`);
        for (const other of cells.slice(index + 1)) {
          if (one.contains(other.one) || other.one.contains(one)) continue;
          const over = Math.min(box.right, other.box.right) - Math.max(box.left, other.box.left) > 0.5 && Math.min(box.bottom, other.box.bottom) - Math.max(box.top, other.box.top) > 0.5;
          if (over) said.push(`${one.textContent.trim().slice(0, 30)} lies over ${other.one.textContent.trim().slice(0, 30)}`);
        }
      }
      const scroller = rows.closest('.drawer-body') ?? rows.closest('.drawer');
      if (scroller && scroller.scrollWidth > scroller.clientWidth) said.push(`the drawer scrolls ${scroller.scrollWidth - scroller.clientWidth}px sideways`);
      if (document.documentElement.scrollWidth > window.innerWidth) said.push('the page scrolls sideways');
      return said;
    });
    expect(`nothing of the rows over anything else ${what}, got ${wrong.join('; ')}`, wrong.length === 0);
    const crowded = await pageCrowded(on, '.drawer .card-rows');
    expect(`nothing of them crowded ${what}, got ${crowded.join('; ')}`, crowded.length === 0);
  };
  await page.locator('.card', { hasText: 'Add a cart' }).click();
  await kept.waitFor();
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await kept.scrollIntoViewIfNeeded();
    await laidOut(page, `at ${width} wide`);
    await page.screenshot({ path: `${out}/gitwork-2-${width}.png` });
  }

  // ---- the row opened in its form -------------------------------------------------------------
  await page.setViewportSize({ width: 1280, height: 860 });
  await kept.getByRole('button', { name: 'Open', exact: true }).click();
  await drawerAtRest(page).getByText('lane/shop-1').first().waitFor();
  expect(`the page never read again, read ${reads - readsAfterSignIn} more times`, reads === readsAfterSignIn);

  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await board(phone, true);
  await phone.locator('.card', { hasText: 'Add a cart' }).tap();
  const there = drawerAtRest(phone).locator('.card-row[data-table="git"]');
  await there.waitFor();
  await there.scrollIntoViewIfNeeded();
  await laidOut(phone, "at a phone's width");
  await phone.screenshot({ path: `${out}/gitwork-3-phone.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await shot?.();
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
