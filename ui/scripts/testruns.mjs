// A table of a command's runs, made by saying where they are kept: a
// row of the fleet's `segments` that runs a command after a card's work
// is kept says `record_to=test-runs`, and each run of it is a row there.
//
//   DAEMON=../target/debug/factory node scripts/testruns.mjs [out-dir]
//
// The shop is a repository, and its fleet's worker a stand-in that adds a
// line to a file each time it is given a card. The row's command passes
// once the file has two lines: the first run fails and the card goes back
// to its worker, the second passes. Before anything is run there is no
// such table; once the first run ends, the table is listed under the
// project in the side panel, on a page that is never read again, and
// opened it holds the failed run, the passed one arriving under it as it
// happens: when it began, the card, the result, its code, how long it
// took, the command and the commit it ran on. Nothing of the table's page
// lies over anything else at three widths and a phone's. A row's address
// is copied from the head of its form and opens it, over its table, on a
// page signed in elsewhere; one that names no row says so. The environment
// variables are those of walkthrough.mjs; `LOOK` is worn as every check
// wears it (`looked.ts`).
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { fleetTook } from './fleettook.mjs';
import { unfoldAll, unfold } from './folded.mjs';
import { saysWhere } from './where.ts';
import { drawerAtRest, sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'testruns-data');
const home = join(out, 'testruns-home');
const shop = join(out, 'testruns-shop');

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
await mkdir(shop, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const git = (...args) => run('git', ['-C', shop, ...args], { env });
await git('init', '--quiet', '.');
await git('config', 'user.name', 'Ada');
await git('config', 'user.email', 'ada@example.com');
await writeFile(join(shop, 'README.md'), '# Shop\n');
await git('add', '-A');
await git('commit', '--quiet', '-m', 'first');

// The worker adds a line to a file in the card's place, each time it is given the card.
const maker = join(home, 'maker.sh');
await writeFile(maker, `#!/bin/sh\ncat >/dev/null\necho changed >> CHANGES.md\nprintf '%s\\n' '{"text":"I changed a file."}'\n`);
await chmod(maker, 0o755);
const PACKAGE = `extension: maker
title: maker
description: A stand-in that adds a line to a file.
agent:
  program: ${maker}
  input: \${ turn.prompt }
  output:
    - when: \${ event.text != null }
      reply: \${ event.text }
`;
// Passes where the file has two lines or more, saying how many it has either way.
const COMMAND = 'n=$(wc -l < CHANGES.md); echo "CHANGES.md has $n lines"; [ "$n" -ge 2 ]';

const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await mkdir(join(home, 'maker'), { recursive: true });
  await writeFile(join(home, 'maker', 'extension.yaml'), PACKAGE);
  await cli('extension', 'add', join(home, 'maker'));
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  await cli('project', 'trust', 'shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('record', 'set', 'shop-agents-2', 'adapter=maker');

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
  const signedIn = async (how) => {
    const context = await browser.newContext({ ...how, colorScheme: process.env.SCHEME ?? 'dark', permissions: ['clipboard-read', 'clipboard-write'] });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const resources = async (on, touch = false) => {
    const press = (locator) => (touch ? locator.tap() : locator.click());
    if (touch) await press(on.getByRole('button', { name: 'Expand the side panel' }));
    await sideSettled(on);
    const button = on.getByRole('button', { name: 'Resources', exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await press(button);
    await unfoldAll(on);
  };
  const page = await signedIn({ viewport: { width: 1280, height: 860 } });
  await resources(page);
  const project = page.locator('.nav [data-project="shop"]');
  await project.getByText('Board', { exact: true }).waitFor();
  const runsLine = project.getByText(/^test.runs$/i);
  expect('no table of runs before anything is run', (await runsLine.count()) === 0);
  await page.screenshot({ path: `${out}/testruns-0-before.png` });

  // ── The row that says where runs are kept, and a card worked ──
  await cli('record', 'add', 'shop', 'segments', 'title=Tests', 'stage=work', 'order=25', 'does=std/run', `command=${COMMAND}`, 'record_to=test-runs');
  // The fleet takes the change of its rows: a card made ready before it has took the round as it was.
  await fleetTook(cli, 'shop');
  await cli('board', 'add', 'shop', 'Add a cart', '--body', 'Customers need one.');
  await cli('board', 'move', 'shop-1', 'ready');

  // Listed under the project once its first run is kept, with the page never read again.
  await runsLine.waitFor({ timeout: 90000 });
  await runsLine.click();
  const grid = page.locator('table.grid');
  const rows = grid.locator('tbody tr[data-row]');
  await rows.first().waitFor();
  await page.screenshot({ path: `${out}/testruns-1-first.png` });
  // The second run arrives under it as it happens.
  await rows.nth(1).waitFor({ timeout: 90000 });
  const kept = JSON.parse((await cli('record', 'list', 'shop', 'test-runs', '--json')).stdout);
  const listed = (kept.records ?? kept).map((one) => one.data);
  expect(`two runs kept, failed and then passed, got ${JSON.stringify(listed.map((one) => one.result))}`, listed.map((one) => one.result).join() === 'failed,passed');
  for (const one of listed) {
    expect(`each run of shop-1 with its command, got ${JSON.stringify(one)}`, one.card === 'shop-1' && one.command === COMMAND);
    expect(`each run on a commit of the card's place, got ${one.commit}`, /^[0-9a-f]{40}$/.test(one.commit ?? ''));
    expect(`each run known by a run id, when it began and its card, got ${one.title}`, /^\d{8}-\d{6}-shop-1$/.test(one.title ?? '') && one.title.startsWith((one.day ?? '').replaceAll('-', '')));
    expect(`each run's day as a date and its time, got ${one.day} ${one.time}`, /^\d{4}-\d{2}-\d{2}$/.test(one.day ?? '') && / UTC$/.test(one.time ?? '') && typeof one.seconds === 'number');
  }
  expect(`the failed run's code and what it printed, got ${JSON.stringify(listed[0])}`, listed[0].code === 1 && listed[0].printed.includes('CHANGES.md has 1 lines'));
  expect(`the passed run's code, got ${listed[1].code}`, listed[1].code === 0);
  // What the page draws of each row: the result as a tag, the card by what it is called, the command.
  const drawn = await rows.evaluateAll((all) => all.map((row) => row.innerText));
  expect(`each row's result drawn, got ${JSON.stringify(drawn)}`, drawn[0].includes('failed') && drawn[1].includes('passed'));
  expect(`each row's card and command drawn, got ${JSON.stringify(drawn)}`, drawn.every((one) => one.includes('Add a cart') && one.includes('CHANGES.md')));
  await page.screenshot({ path: `${out}/testruns-2-both.png` });

  // ── A row's address, copied from its form's head and opened on another page ──
  const first = (kept.records ?? kept)[0];
  await rows.first().hover();
  await rows.first().locator('.grid-open').click();
  const form = drawerAtRest(page);
  await form.locator('.drawer-head h2', { hasText: first.data.title }).waitFor();
  await form.getByRole('button', { name: 'Copy link', exact: true }).click();
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(`the row's address its table's with its id after the #, got ${copied}`, copied === `${url}/shop/test-runs#${first.id}`);
  await page.screenshot({ path: `${out}/testruns-2-row.png` });
  const other = await signedIn({ viewport: { width: 1280, height: 860 } });
  await other.goto(copied);
  const form2 = drawerAtRest(other);
  await form2.locator('.drawer-head h2', { hasText: first.data.title }).waitFor();
  // Over its table, which is the tab showing behind the drawer.
  await other.locator('table.grid tbody tr[data-row]').nth(1).waitFor();
  const fields = await form2.locator('input, textarea').evaluateAll((all) => all.map((one) => one.value));
  expect(`the row's own form, got ${JSON.stringify(fields)}`, fields.includes(first.data.command));
  await other.screenshot({ path: `${out}/testruns-2-opened.png` });
  // One that names no row says so.
  await other.goto(`${url}/shop/test-runs#shop-test-runs-99`);
  await drawerAtRest(other).getByText('There is no such row here').waitFor();
  await other.close();

  // ── Nothing over anything else, at three widths and a phone's ──
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    const crowded = await pageCrowded(page);
    expect(`nothing of the table over anything else at ${width} wide, got ${crowded.join('; ')}`, crowded.length === 0);
  }
  await page.screenshot({ path: `${out}/testruns-3-narrow.png` });
  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await resources(phone, true);
  await unfold(phone, 'Shop');
  await phone.locator('.nav [data-project="shop"]').getByText(/^test.runs$/i).tap();
  await phone.locator('tbody tr[data-row], .records-list li, .row-card').nth(1).waitFor();
  await phone.screenshot({ path: `${out}/testruns-4-phone.png` });
  // On a phone a row's Open is always drawn, over the right of the cell
  // the row is known by, whose words give way before it: that is the one
  // thing laid over another, and its words are held clear of it.
  const opened = /^"[^"]+ of (.+)" lies over "Open \1"$/;
  const crowded = (await pageCrowded(phone)).filter((said) => !opened.test(said));
  expect(`nothing of the table over anything else at a phone's width, got ${crowded.join('; ')}`, crowded.length === 0);
  const clear = await phone.locator('tbody tr[data-row]').evaluateAll((all) =>
    all.every((row) => {
      // The words are cut where the cell's room for them ends. Its rows are
      // the command's (`by_hand: false`), so a cell is words and no button.
      const cell = row.querySelector('.grid-open')?.parentElement?.querySelector('.cell');
      const words = cell?.querySelector('.cell-words')?.getBoundingClientRect();
      const open = row.querySelector('.grid-open')?.getBoundingClientRect();
      if (!cell || !words || !open) return false;
      const room = cell.getBoundingClientRect().right - parseFloat(getComputedStyle(cell).paddingRight);
      return Math.min(words.right, room) <= open.left + 0.5;
    }),
  );
  expect("each row's words clear of its Open at a phone's width", clear);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
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
