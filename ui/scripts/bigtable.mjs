// A table of many rows, read a window at a time.
//
//   DAEMON=../target/debug/factory node scripts/bigtable.mjs [out-dir]
//
// A project has a table of runs (`ROWS` of them, ten thousand where
// nothing is said), made through the routes there are a record at a
// time. Held here, by what the page shows, what it asks and the boxes:
//
//   - the page opens drawing no more than two hundred rows, reads no
//     table whole, says at its head and its foot how many rows the
//     daemon counted, and what scrolls is as tall as every row;
//   - scrolled by the wheel to the middle, by what scrolls put at 73% of
//     its height (the scrollbar dragged), by End and by Home: the rows on
//     the screen are drawn there once they are read, each the row of its
//     place among them all, and nothing on the screen is room still
//     waiting for its rows;
//   - sorted by a column, greatest first, by the daemon: the first row is
//     the greatest, and the count the same;
//   - a figure at a column's foot, the daemon's, of every row;
//   - held to one thing of a column: the count and the figure of those
//     rows, and the rows drawn only those;
//   - gathered by a column, each band's head saying how many the daemon
//     counted;
//   - searched for the middle of a commit among them: the count and the
//     rows only those found, and nothing said of how far it looked once
//     it went through every row;
//   - a row gone to that the view does not show (by Ctrl and G): said
//     what keeps it out, and opened all the same; one gone to by its
//     number, brought into view and marked;
//   - a row on the screen changed from a terminal drawn changed where it
//     is, the page not read again;
//   - sorted, and from a terminal a row on the screen changed so that it
//     would stand elsewhere, one taken away and one added before them
//     all: drawn where each is, marked or struck, the new one counted,
//     nothing on the screen moved, and Show reading it again with the row
//     at the top of the screen where it was to a pixel;
//   - another tab shown and this one again: drawn at once, the same row at
//     the top of the screen within a pixel, and read again behind;
//   - frozen up to what a row is known by, scrolled sideways, and nothing
//     of the page over anything else or past the window at 1280, 900 and
//     640 wide and a phone's.
//
// The environment variables are those of walkthrough.mjs, with `LOOK`
// (`looked.ts`) to hold it at a look of a theme's. Several may be run at
// once, each with a folder of its own under `out-dir`.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const ROWS = Number(process.env.ROWS ?? 10_000);
// A check run several at once is given folders of its own.
const own = `bigtable-${process.pid}`;
const data = join(out, `${own}-data`);
const home = join(out, `${own}-home`);
const looked = process.env.LOOK ? '-looked' : '';

const TABLES = `blueprint: runs
description: A table of runs, a row for each, as a script's command keeps them.
permissions: [projects]
steps:
  runs:
    do: collection.ensure
    with:
      name: runs
      fields:
        title: { type: text, required: true }
        result: { type: choice, of: [passed, failed] }
        seconds: { type: number }
        commit: { type: text }
`;

/** The runs, as each is made: one in seven failed. */
const pad = (n) => String(n).padStart(6, '0');
const made = Array.from({ length: ROWS }, (_, nth) => {
  const n = nth + 1;
  return { title: `Run ${pad(n)}`, result: n % 7 === 0 ? 'failed' : 'passed', seconds: (n * 37) % 1000, commit: ((n * 2654435761) >>> 0).toString(16).padStart(8, '0') };
});
const sum = (rows) => rows.reduce((all, row) => all + row.seconds, 0);
const failed = made.filter((row) => row.result === 'failed');

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

let failure = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  const plan = join(data, 'blueprints', 'projects', 'shop', 'runs.bp');
  await mkdir(join(data, 'blueprints', 'projects', 'shop'), { recursive: true });
  await writeFile(plan, TABLES);
  await cli('project', 'scan', 'shop');
  await cli('apply', plan, '--project', 'shop');
  // Made through the route that adds one record, a few at a time: what a script's command does, faster.
  const key = JSON.parse((await cli('key', 'filler', '--json')).stdout).token;
  const add = (fields) =>
    fetch(`${url}/api/collections/shop/runs/records`, { method: 'POST', headers: { authorization: `Bearer ${key}`, 'content-type': 'application/json' }, body: JSON.stringify({ fields }) }).then((answer) => {
      if (!answer.ok) throw new Error(`a run was not added: ${answer.status}`);
    });
  let next = 0;
  await Promise.all(
    Array.from({ length: 16 }, async () => {
      while (next < made.length) await add(made[next++]);
    }),
  );

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
  /** What the page asked of the daemon, in order. */
  const asked = [];

  const opened = async (context, touch = false) => {
    const page = await context.newPage();
    lastPage = page;
    page.setDefaultTimeout(30_000);
    page.on('pageerror', (err) => problems.push(err.message));
    page.on('request', (request) => asked.push(request.url()));
    await page.goto(url);
    await page.locator('.panes').or(page.getByLabel('Email')).first().waitFor();
    if (await page.getByLabel('Email').count()) {
      await page.getByLabel('Email').fill('ada@example.com');
      await page.getByLabel('Password').fill('correct horse');
      const signIn = page.getByRole('button', { name: 'Sign in', exact: true }).last();
      await (touch ? signIn.tap() : signIn.click());
    }
    await page.locator('.panes').waitFor();
    return page;
  };
  const show = async (page, touch = false) => {
    const press = (locator) => (touch ? locator.tap() : locator.click());
    if (touch) await press(page.getByRole('button', { name: 'Expand the side panel' }));
    await sideSettled(page);
    const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await press(resources);
    await unfoldAll(page);
    const nav = page.locator('.nav');
    await nav.getByText('Shop', { exact: true }).waitFor();
    const twist = nav.getByRole('button', { name: 'Unfold Shop', exact: true });
    if (await twist.count()) await press(twist);
    await press(nav.locator('[data-project="shop"]').getByRole('button', { name: /Runs$/ }).first());
    await page.getByRole('heading', { name: 'Shop runs', level: 1 }).waitFor();
    await page.locator('table.grid tbody tr[data-row]').first().waitFor();
  };
  const grid = (page) => page.locator('.split table.grid').first();
  const head = (page, column) => grid(page).locator('thead th[data-column]', { has: page.locator('.grid-head .truncate', { hasText: new RegExp(`^${column}$`) }) });
  const menu = (page) => page.locator('.headmenu:not(.colform)');
  const line = (page, name) => menu(page).getByRole('menuitem', { name }).or(menu(page).getByRole('menuitemcheckbox', { name })).or(menu(page).getByRole('menuitemradio', { name }));
  const openMenu = async (page, column) => {
    await head(page, column).locator('.grid-head').click();
    await menu(page).waitFor();
    await page.waitForFunction(() => document.querySelector('.headmenu')?.style.visibility !== 'hidden');
  };
  const choose = async (page, column, ...names) => {
    await openMenu(page, column);
    for (const [nth, name] of names.entries()) {
      await line(page, name).click();
      if (nth === names.length - 1) await menu(page).waitFor({ state: 'detached' }).catch(() => page.keyboard.press('Escape'));
    }
    if (await menu(page).count()) {
      await page.keyboard.press('Escape');
      await menu(page).waitFor({ state: 'detached' });
    }
  };
  /** What the foot says of how many. */
  const footCount = async (page) => (await grid(page).locator('tfoot .grid-count').first().innerText()).trim();
  /**
   * What is on the screen once it is all read: every row in view, by its rank and what it holds, and whether any of
   * the screen is still room waiting for rows. Waited for until nothing in view waits.
   */
  const onScreen = async (page) => {
    // Where it stops: a key scrolls a while.
    await page.evaluate(
      () =>
        new Promise((done) => {
          const box = document.querySelector('.split .grid-scroll');
          let was = -1;
          let same = 0;
          const look = () => {
            same = box.scrollTop === was ? same + 1 : 0;
            was = box.scrollTop;
            if (same >= 5) done();
            else requestAnimationFrame(look);
          };
          look();
        }),
    );
    await page.waitForFunction(() => {
      const box = document.querySelector('.split .grid-scroll');
      if (!box) return false;
      const view = box.getBoundingClientRect();
      const head = box.querySelector('thead tr:last-child th')?.getBoundingClientRect().bottom ?? view.top;
      const waiting = [...box.querySelectorAll('tbody tr.grid-room')].some((room) => {
        const at = room.getBoundingClientRect();
        return at.height > 0 && at.bottom > head + 1 && at.top < view.bottom - 1;
      });
      return !waiting && box.querySelector('tbody tr[data-row]') !== null;
    }).catch(async (err) => {
      const rooms = await page.evaluate(() => {
        const box = document.querySelector('.split .grid-scroll');
        const view = box.getBoundingClientRect();
        const head = box.querySelector('thead tr:last-child th').getBoundingClientRect().bottom;
        return { view: [view.top, view.bottom, head, box.scrollTop, box.scrollHeight, box.clientHeight], rooms: [...box.querySelectorAll('tbody tr.grid-room')].map((room) => [room.getBoundingClientRect().top, room.getBoundingClientRect().height]) };
      });
      throw new Error(`the screen never all read: ${JSON.stringify(rooms)}: ${err.message}`);
    });
    return page.evaluate(() => {
      const box = document.querySelector('.split .grid-scroll');
      const view = box.getBoundingClientRect();
      const head = box.querySelector('thead tr:last-child th').getBoundingClientRect().bottom;
      const foot = box.querySelector('tfoot').getBoundingClientRect().top;
      return [...box.querySelectorAll('tbody tr[data-row]')]
        .map((row) => ({ row, at: row.getBoundingClientRect() }))
        .filter(({ at }) => at.bottom > head + 1 && at.top < Math.min(view.bottom, foot) - 1)
        .map(({ row, at }) => ({ id: row.dataset.row, rank: Number(row.dataset.rank), top: at.top - head, title: row.querySelector('td.grid-title .cell-words')?.textContent ?? '', cells: [...row.querySelectorAll('td .cell-words')].map((one) => one.textContent) }));
    });
  };
  const scrollTo = (page, fraction) => page.locator('.split .grid-scroll').evaluate((box, fraction) => (box.scrollTop = fraction * (box.scrollHeight - box.clientHeight)), fraction);
  const drawnRows = (page) => grid(page).locator('tbody tr[data-row]').count();
  const crowdedAt = async (page, where) => {
    const wrong = await pageCrowded(page, 'body');
    expect(`nothing ${where} over anything else or past the window: ${wrong.join('; ')}`, wrong.length === 0);
  };

  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const page = await opened(context);
  await page.evaluate(() => (window.readOnce = true));
  await show(page);
  await grid(page).locator('tfoot .grid-count[data-counted="whole"]').waitFor();

  // ---- opened: a window of it, counted by the daemon, nothing read whole
  const many = `${ROWS.toLocaleString('en-US')} runs`;
  expect(`at most two hundred rows drawn, got ${await drawnRows(page)}`, (await drawnRows(page)) <= 200);
  expect(`the foot to say ${many}, got ${await footCount(page)}`, (await footCount(page)).replace(/\D/g, '') === String(ROWS) && (await footCount(page)).endsWith(' runs'));
  expect('the head to say how many', (await page.locator('.view-head .muted').first().innerText()).replace(/\D/g, '') === String(ROWS));
  const whole = asked.filter((one) => /\/collections\/shop\/runs\/records(\?|$)/.test(one));
  if (ROWS > 5000) expect(`no reading of the table whole, got ${whole.join(' ')}`, whole.length === 0);
  const tall = await page.evaluate(() => {
    const box = document.querySelector('.split .grid-scroll');
    const row = box.querySelector('tbody tr[data-row]').getBoundingClientRect().height;
    const rest = box.querySelector('thead').getBoundingClientRect().height + box.querySelector('tfoot').getBoundingClientRect().height;
    return { height: box.scrollHeight, row, rest };
  });
  // As tall as every row, and past what a browser draws (`CAP` in `window.ts`) that tall and read in pages of it.
  const room = Math.min(ROWS * tall.row, 8_000_000);
  expect(`what scrolls to be as tall as every row, ${tall.height} for ${ROWS} of ${tall.row}`, Math.abs(tall.height - tall.rest - room) < tall.row * 3 + 40);
  let screen = await onScreen(page);
  expect(`the first row first, got ${screen[0]?.id}`, screen[0]?.id.endsWith('-1') && screen[0].rank === 0);
  await page.screenshot({ path: `${out}/bigtable-1-opened${looked}.png` });

  /** Every row on the screen the one of its place among them all, in the order they were made: by the number each was given, which its id ends in. */
  const inPlace = (screen, where) => {
    for (const one of screen) expect(`the row at ${one.rank} ${where} to be the ${one.rank + 1}th made, got ${one.id}`, one.id.endsWith(`-${one.rank + 1}`));
  };

  // ---- the wheel to the middle
  await page.mouse.move(700, 500);
  for (let turn = 0; turn < 10; turn++) await page.mouse.wheel(0, (tall.height / 2 - 400) / 10);
  screen = await onScreen(page);
  inPlace(screen, 'after the wheel');
  expect(`the middle after the wheel, got ${screen[0]?.rank}`, Math.abs(screen[0].rank - ROWS / 2) < ROWS / 20);
  expect(`still no more than two hundred rows drawn, got ${await drawnRows(page)}`, (await drawnRows(page)) <= 260);
  await page.screenshot({ path: `${out}/bigtable-2-middle${looked}.png` });

  // ---- the scrollbar dragged to 73%
  await scrollTo(page, 0.73);
  screen = await onScreen(page);
  inPlace(screen, 'at 73%');
  expect(`73% of the way, got ${screen[0]?.rank}`, Math.abs(screen[0].rank / ROWS - 0.73) < 0.02);

  // ---- End and Home
  await page.locator('.split .grid-scroll').focus();
  await page.keyboard.press('End');
  screen = await onScreen(page);
  inPlace(screen, 'at the end');
  expect(`the last row at the end, got ${screen.at(-1)?.id}`, screen.at(-1)?.id.endsWith(`-${ROWS}`));
  await page.keyboard.press('Home');
  screen = await onScreen(page);
  expect(`the first row at the start, got ${screen[0]?.id}`, screen[0]?.id.endsWith('-1'));

  // ---- sorted by the daemon, greatest first
  await choose(page, 'Seconds', 'Sort descending');
  await page.waitForFunction(() => document.querySelector('.split table.grid thead th[data-column="seconds"]')?.getAttribute('aria-sort') === 'descending');
  await page.waitForFunction((most) => document.querySelector('.split table.grid tbody tr[data-row] td:nth-child(3) .cell-words')?.textContent === String(most), made.reduce((most, row) => Math.max(most, row.seconds), 0));
  expect('the count the same, sorted', (await footCount(page)).replace(/\D/g, '') === String(ROWS));
  await choose(page, 'Seconds', 'Sort descending');
  await page.waitForFunction(() => document.querySelector('.split table.grid thead th[data-column="seconds"]')?.getAttribute('aria-sort') === 'none');
  await page.waitForFunction(() => document.querySelector('.split table.grid tbody tr[data-row]')?.dataset.row.endsWith('-1'));

  // ---- a figure at the foot, of every row
  await choose(page, 'Seconds', /^Calculate/, 'Sum');
  await page.waitForFunction((want) => (document.querySelector('.split tr.grid-figures td[data-figure="seconds"]')?.textContent ?? '').includes(want), String(sum(made)));

  // ---- held to the failed runs
  await openMenu(page, 'Result');
  await line(page, /^Filter/).click();
  await line(page, 'Only Result failed').click();
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  await page.waitForFunction((want) => (document.querySelector('.split tfoot .grid-count')?.textContent ?? '').replace(/\D/g, '') === want, String(failed.length));
  await page.waitForFunction((want) => (document.querySelector('.split tr.grid-figures td[data-figure="seconds"]')?.textContent ?? '').includes(want), String(sum(failed)));
  screen = await onScreen(page);
  expect('only failed runs drawn', screen.every((one) => one.cells.includes('failed')));
  await page.locator('.held').getByText('Showing only').waitFor();
  await page.screenshot({ path: `${out}/bigtable-3-held${looked}.png` });

  // ---- searched for the middle of a commit, among the failed runs: what the daemon found, counted
  const word = failed[3].commit.slice(2, 6);
  const finds = (row) => [row.title, row.result, String(row.seconds), row.commit].some((text) => text.toLowerCase().includes(word));
  const searchedFor = failed.filter(finds);
  await page.getByRole('searchbox', { name: 'Search this table' }).fill(word.toUpperCase());
  await page.waitForFunction((want) => (document.querySelector('.split tfoot .grid-count')?.textContent ?? '').replace(/\D/g, '') === want, String(searchedFor.length));
  screen = await onScreen(page);
  expect(`only what was searched for drawn, ${searchedFor.length} of them`, screen.length === Math.min(searchedFor.length, screen.length) && screen.every((one) => one.cells.some((cell) => cell.toLowerCase().includes(word))));
  // Made sixteen at a time, a run's number is not always its place among those made: what was found is among them.
  expect('every row drawn one of those searched for', screen.every((one) => searchedFor.some((row) => row.title === one.title)));
  expect('nothing said of how far it looked once it looked through every row', (await page.locator('.rows-find [data-looked]').count()) === 0);
  await page.screenshot({ path: `${out}/bigtable-3-searched${looked}.png` });
  await page.getByRole('searchbox', { name: 'Search this table' }).press('Escape');
  await page.waitForFunction((want) => (document.querySelector('.split tfoot .grid-count')?.textContent ?? '').replace(/\D/g, '') === want, String(failed.length));

  // ---- a row gone to that the view does not show: said why, and opened all the same
  await page.locator('.split .grid-scroll').focus();
  await page.keyboard.press('Control+G');
  expect('Ctrl and G in the box that goes to a row', await page.evaluate(() => document.activeElement?.getAttribute('aria-label') === 'Go to a row, by its id or its number'));
  await page.keyboard.type('1');
  await page.keyboard.press('Enter');
  await page.locator('.rows-find [role="status"]').getByText('It is not in this view: Result keeps it out.').waitFor();
  await page.locator('.rows-find').getByRole('button', { name: 'Open it' }).click();
  await page.locator('.drawer').getByText('shop-runs-1', { exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await openMenu(page, 'Result');
  await line(page, /^Filter/).click();
  await line(page, 'Show all').click();
  await page.keyboard.press('Escape');
  await page.waitForFunction((want) => (document.querySelector('.split tfoot .grid-count')?.textContent ?? '').replace(/\D/g, '') === want, String(ROWS));

  // ---- a row gone to by its number: read about it, brought into view and marked
  const goneTo = Math.floor(ROWS * 0.6);
  await page.getByRole('textbox', { name: 'Go to a row, by its id or its number' }).fill(String(goneTo));
  await page.getByRole('textbox', { name: 'Go to a row, by its id or its number' }).press('Enter');
  await page.locator(`.split tr.grid-gone-to[data-row$="-${goneTo}"]`).waitFor();
  screen = await onScreen(page);
  inPlace(screen, 'gone to');
  expect(`the row gone to on the screen, got ${screen.map((one) => one.rank).join(',')}`, screen.some((one) => one.id.endsWith(`-${goneTo}`)));
  await page.screenshot({ path: `${out}/bigtable-3-gone${looked}.png` });
  await scrollTo(page, 0);
  await onScreen(page);

  // ---- gathered by the result, each band counted by the daemon
  await choose(page, 'Result', 'Group');
  await grid(page).locator('tr.grid-gathered').first().waitFor();
  // Only what is on the screen is drawn: the band of the failed runs is under every passed one, until that is
  // folded. Past ten thousand rows every band starts folded, and is read as it is opened.
  const large = ROWS > 10_000;
  expect(`the band of the passed runs first, with its count`, (await grid(page).locator('tr.grid-gathered .bands-head').first().innerText()).includes(String(ROWS - failed.length)));
  if (large) expect('every band folded as it starts past ten thousand rows', (await drawnRows(page)) === 0);
  else await grid(page).locator('tr.grid-gathered .bands-head').first().click();
  await grid(page).locator('tr.grid-gathered').nth(1).waitFor();
  const heads = await grid(page).locator('tr.grid-gathered .bands-head').allInnerTexts();
  expect(`a band for passed and one for failed with their counts, got ${heads.join(' | ')}`, heads.length === 2 && heads[0].includes(String(ROWS - failed.length)) && heads[1].includes(String(failed.length)));
  if (large) await grid(page).locator('tr.grid-gathered .bands-head').nth(1).click();
  screen = await onScreen(page);
  expect('the failed runs under their head', screen.length > 0 && screen.every((one) => one.cells.includes('failed')));
  await grid(page).locator('tr.grid-gathered .bands-head').nth(large ? 1 : 0).click();
  await page.screenshot({ path: `${out}/bigtable-4-gathered${looked}.png` });
  await choose(page, 'Result', 'Ungroup');
  await grid(page).locator('tr.grid-gathered').first().waitFor({ state: 'detached' });

  // ---- a row on the screen changed from a terminal, drawn where it is
  await scrollTo(page, 0.4);
  screen = await onScreen(page);
  const changed = screen[3];
  await cli('record', 'set', changed.id, 'commit=changed-here');
  await page.waitForFunction(({ id }) => [...document.querySelectorAll(`.split tr[data-row="${id}"] .cell-words`)].some((one) => one.textContent === 'changed-here'), { id: changed.id });
  expect('the page not read again for it', await page.evaluate(() => window.readOnce === true));

  // ---- sorted, and from a terminal a row on the screen changed where it would stand elsewhere, one taken away and one
  // added before them all: each drawn where it is or counted, nothing on the screen moved, until Show reads it again
  // with the row at the top where it was
  await page.locator('.split .grid-scroll').evaluate((box) => (box.scrollTop = 0));
  await choose(page, 'Seconds', 'Sort ascending');
  await page.waitForFunction(() => document.querySelector('.split table.grid thead th[data-column="seconds"]')?.getAttribute('aria-sort') === 'ascending');
  await page.waitForFunction(() => document.querySelector('.split table.grid tbody tr[data-row] td:nth-child(3) .cell-words')?.textContent === '0');
  let before = await onScreen(page);
  await cli('record', 'set', before[2].id, 'seconds=999');
  await cli('record', 'remove', before[4].id);
  await cli('record', 'add', 'shop', 'runs', 'title=Run newest', 'result=passed', 'seconds=-1', 'commit=new');
  await page.locator('.split .grid-live-line').getByText('1 new row · 1 moved · 1 no longer here').waitFor();
  await page.locator(`.split tr[data-row="${before[2].id}"][data-live="moved"]`).waitFor();
  await page.locator(`.split tr[data-row="${before[4].id}"][data-live="removed"]`).waitFor();
  screen = await onScreen(page);
  expect(`nothing on the screen moved, got ${screen.map((one) => one.id).join(' ')}`, screen.slice(0, 8).every((one, nth) => one.id === before[nth].id && Math.abs(one.top - before[nth].top) < 1));
  await page.screenshot({ path: `${out}/bigtable-4-changed${looked}.png` });
  await page.mouse.move(1, 859);
  await crowdedAt(page, 'with what changed counted');
  await page.locator('.split .grid-live-line').getByRole('button', { name: 'Show' }).click();
  await page.locator('.split .grid-live-line').waitFor({ state: 'detached' });
  await page.waitForFunction(({ id, at }) => {
    const box = document.querySelector('.split .grid-scroll');
    const row = box?.querySelector(`tr[data-row="${id}"]`);
    return row && !row.dataset.live && Math.abs(row.getBoundingClientRect().top - box.querySelector('thead tr:last-child th').getBoundingClientRect().bottom - at) < 1;
  }, { id: before[0].id, at: before[0].top });
  screen = await onScreen(page);
  expect('the moved and the removed no longer where they were', !screen.some((one) => one.id === before[2].id || one.id === before[4].id));
  await page.locator('.split .grid-scroll').evaluate((box) => (box.scrollTop = 0));
  screen = await onScreen(page);
  expect(`the row added first once scrolled to, got ${screen[0]?.title}`, screen[0]?.title === 'Run newest');
  const readWhole = asked.filter((one) => /\/collections\/shop\/runs\/records(\?|$)/.test(one));
  expect('the page not read again for any of it, nor the table whole', (await page.evaluate(() => window.readOnce === true)) && (ROWS <= 5000 || readWhole.length === 0));
  await choose(page, 'Seconds', 'Sort ascending');
  await page.waitForFunction(() => document.querySelector('.split table.grid thead th[data-column="seconds"]')?.getAttribute('aria-sort') === 'none');
  await scrollTo(page, 0.4);
  await onScreen(page);

  // ---- another tab and this one again
  screen = await onScreen(page);
  const top = screen[0];
  await page.locator('.tab', { hasText: 'Start' }).first().click();
  await grid(page).waitFor({ state: 'detached' });
  const againBehind = page.waitForRequest(/\/collections\/shop\/runs\/rows/);
  await page.locator('.tab', { hasText: 'Shop runs' }).first().click();
  // Drawn at once from what it had: the row is there before anything it asked was answered.
  await page.locator(`.split tr[data-row="${top.id}"]`).waitFor();
  await page.waitForFunction(({ id, at }) => {
    const box = document.querySelector('.split .grid-scroll');
    const row = box?.querySelector(`tr[data-row="${id}"]`);
    return row && Math.abs(row.getBoundingClientRect().top - box.querySelector('thead tr:last-child th').getBoundingClientRect().bottom - at) < 1;
  }, { id: top.id, at: top.top });
  await againBehind;

  // ---- frozen up to what a row is known by, at three widths and a phone's
  await choose(page, 'Title', 'Freeze up to here');
  await grid(page).locator('th.frozen').first().waitFor();
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await page.locator('.split .grid-scroll').evaluate((box) => (box.scrollLeft = box.scrollWidth));
    await onScreen(page);
    // Nothing pointed at: a row pointed at shows what opens it over its title.
    await page.mouse.move(1, 859);
    await crowdedAt(page, `of the table at ${width} wide`);
    await page.screenshot({ path: `${out}/bigtable-5-${width}${looked}.png` });
  }
  await context.close();

  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const touched = await opened(phone, true);
  await show(touched, true);
  screen = await onScreen(touched);
  await scrollTo(touched, 0.5);
  screen = await onScreen(touched);
  // One was taken away from a terminal and one added at the end: every row after the one before it, all the same.
  const numbers = screen.map((one) => Number(one.id.split('-').at(-1)));
  expect(`the rows on a phone in the order they were made, got ${numbers.join(',')}`, numbers.length > 0 && numbers.every((number, nth) => nth === 0 || number > numbers[nth - 1]) && Math.abs(screen[0].rank / ROWS - 0.5) < 0.02);
  await crowdedAt(touched, 'of the table on a phone');
  await touched.screenshot({ path: `${out}/bigtable-6-phone${looked}.png` });
  await phone.close();

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failure = err;
  await lastPage?.screenshot({ path: `${out}/bigtable-failed${looked}.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failure) {
  console.error(failure);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
