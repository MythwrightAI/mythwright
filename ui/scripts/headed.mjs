// The head of a table's column: one button, its menu, and the column
// carried to another place.
//
//   DAEMON=../target/debug/factory node scripts/headed.mjs [out-dir]
//
// A project has a board with a fleet (whose agents' table says its columns
// in groups and sits beside the board), a table of stock wide enough to be
// scrolled sideways, and a ledger only the factory's owner may change.
// Held here:
//
//   - a head is one button: pressed, or Enter or Space on it, a menu is
//     under it, wholly in the window (at the table's right edge too), over
//     nothing of itself, moving nothing; it takes the arrows, stays as the
//     pointer crosses to it, and is put away by Escape, a press outside and
//     the head's own press; it has no line for what cannot be done;
//   - each line does what it says, is kept when the page is read again, and
//     is seen on a second open page that is not read again: the view held
//     to things of the column, its rows gathered by it, a figure at its
//     foot, the view frozen up to it, the column left out, moved a place;
//     and sorting, which is the reader's own and no other page's;
//   - a column carried by its head stands where it was let go: the drop is
//     marked by a line and nothing moves before it, a press that never moved
//     is the menu, one that came back changes nothing, Escape puts it back,
//     what a record is known by is not carried, and carried across the
//     frozen edge a column joins the frozen ones;
//   - the layout's list of columns in the same order, and moved there by keys;
//   - somebody who may not change a table's views offered sorting alone;
//   - the board's table; the agents' table in the drawer and as a tab, its
//     columns in groups: frozen with the band of headings over each column,
//     a figure at a foot, a column carried within its group and not out;
//   - a phone's width, by the menu's lines, with nothing frozen; and the
//     other of dark and light;
//   - and, by a check of its own that this runs once it is done
//     (`widths.mjs`), a column made wider or narrower by its head's edge.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn, spawnSync } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { gearLine } from './gear.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'headed-data');
const home = join(out, 'headed-home');

const TABLES = `blueprint: stock
description: A table wide enough to be scrolled sideways, and one only the owner may change.
permissions: [projects, board]
steps:
  stock:
    do: collection.ensure
    with:
      name: stock
      fields:
        title: { type: text, required: true }
        status: { type: choice, of: [todo, doing, done] }
        labels: { type: list }
        due: { type: date }
        points: { type: number }
        packed: { type: flag }
        aisle: { type: text }
        bin: { type: text }
        maker: { type: text }
        note: { type: text }
        shelf: { type: text }
        zone: { type: text }
  ledger:
    do: collection.ensure
    with:
      name: ledger
      takes: admin
      fields:
        title: { type: text, required: true }
        sum: { type: number }
`;
/** The columns of the stock as they stand when no view says otherwise. */
const STOOD = ['Title', 'Status', 'Labels', 'Due', 'Points', 'Packed', 'Aisle', 'Bin', 'Maker', 'Note', 'Shelf', 'Zone'];

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
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
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('board', 'add', 'shop', 'Basket');
  const shop = join(data, 'projects/shop');
  const plan = join(shop, 'blueprints/stock.bp');
  await mkdir(join(shop, 'blueprints'), { recursive: true });
  await writeFile(plan, TABLES);
  await cli('apply', plan, '--project', 'shop');
  await cli('record', 'add', 'shop', 'ledger', 'title=Shelving', 'sum=4');
  await cli('record', 'add', 'shop', 'ledger', 'title=Crates', 'sum=9');

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

  /** A page that is signed in, with how many times it saved a view counted beside it. */
  const opened = async (context, email = 'ada@example.com', touch = false) => {
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    const counted = { saves: 0, answered: 0, asked: [] };
    page.on('request', (request) => {
      if (request.method() === 'POST' && request.url().endsWith('/views')) counted.saves += 1;
      if (request.url().includes('/api/')) counted.asked.push({ at: Date.now(), what: `${request.method()} ${new URL(request.url()).pathname}${new URL(request.url()).search}`, request });
    });
    page.on('requestfinished', async (request) => {
      if (request.method() === 'POST' && request.url().endsWith('/views')) counted.answered += 1;
      const one = counted.asked.find((asked) => asked.request === request);
      if (one) one.status = (await request.response())?.status() ?? 'none';
    });
    page.on('requestfailed', (request) => {
      const one = counted.asked.find((asked) => asked.request === request);
      if (one) one.status = `failed: ${request.failure()?.errorText}`;
    });
    await page.goto(url);
    // Whichever the page comes to draw, where to sign in or what somebody signed in is shown: it is waited
    // for before it is asked which. Asked as soon as the page had loaded, a page that had not drawn yet was
    // taken for one signed in, and was waited on for ever (a phone's page, five in eighteen, six at once).
    await page.locator('.panes').or(page.getByLabel('Email')).first().waitFor();
    if (await page.getByLabel('Email').count()) {
      await page.getByLabel('Email').fill(email);
      await page.getByLabel('Password').fill('correct horse');
      const signIn = page.getByRole('button', { name: 'Sign in', exact: true }).last();
      await (touch ? signIn.tap() : signIn.click());
    }
    await page.locator('.panes').waitFor();
    return { page, counted };
  };
  /** Shows one of the project's tables, from the side panel. */
  const show = async (page, table, touch = false) => {
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
    const called = table.charAt(0).toUpperCase() + table.slice(1);
    await press(nav.locator('[data-project="shop"]').getByRole('button', { name: new RegExp(`${called}$`) }).first());
    if (table !== 'board') await page.getByRole('heading', { name: `Shop ${table}`, level: 1 }).waitFor();
  };
  /** The table a page is showing (the one in the drawer, where `within` says so), its heads by what each is called, and the menu of a head. */
  const grid = (page, within = '') => page.locator(`${within} table.grid`.trim());
  const head = (page, column, within = '') => grid(page, within).locator('thead th[data-column]', { has: page.locator('.grid-head .truncate', { hasText: new RegExp(`^${column}$`) }) });
  const pressed = (page, column, within = '') => head(page, column, within).locator('.grid-head');
  const menu = (page) => page.locator('.headmenu');
  /** A line of the menu that is pressed, by what it says: one that does something, one that is on or off, or one of several of which one is. */
  const line = (page, name) => menu(page).getByRole('menuitem', { name }).or(menu(page).getByRole('menuitemcheckbox', { name })).or(menu(page).getByRole('menuitemradio', { name }));
  /** The columns as they stand, by what each is called; and the rows, by their ids. */
  const columns = (page, within = '') => grid(page, within).locator('thead th[data-column] .grid-head .truncate').allInnerTexts();
  const standsAs = (page, wanted, within = '') =>
    page.waitForFunction(([wanted, within]) => [...document.querySelectorAll(`${within} table.grid thead th[data-column] .grid-head .truncate`)].map((one) => one.textContent).join(' ') === wanted, [wanted.join(' '), within]);
  const rows = (page) => grid(page).locator('tbody tr[data-row]').evaluateAll((all) => all.map((one) => one.dataset.row.replace(/^shop-[a-z]+-/, '')));
  /** Waits for the rows to be sorted as said: the daemon sorts a table read a window at a time, a moment after the press. */
  const sortedAs = (page, what, column, sort, first) =>
    page
      .waitForFunction(
        ([column, sort, first]) => {
          const th = [...document.querySelectorAll('table.grid thead th[data-column]')].find((one) => one.querySelector('.grid-head .truncate')?.textContent === column);
          const ids = [...document.querySelectorAll('table.grid tbody tr[data-row]')].map((one) => one.dataset.row.replace(/^shop-[a-z]+-/, ''));
          return th?.getAttribute('aria-sort') === sort && ids.slice(0, first.length).join(' ') === first.join(' ');
        },
        [column, sort, first],
        { timeout: 10_000 },
      )
      .catch(() => expect(what, false));
  const boxOf = async (locator) => {
    const box = await locator.boundingBox();
    if (!box) throw new Error('expected something that is drawn to have a place');
    return box;
  };
  /** Opens the menu of a column and waits until the caret is in it. */
  const open = async (page, column, { touch = false, within = '' } = {}) => {
    await (touch ? pressed(page, column, within).tap() : pressed(page, column, within).click());
    await menu(page).waitFor();
    await page.waitForFunction(() => document.activeElement?.closest('.headmenu') !== null && document.querySelector('.headmenu').style.visibility !== 'hidden');
  };
  /** Presses a line of the menu that is open, and waits for the menu to be put away where the line does that. */
  const choose = async (page, name, { touch = false, stays = false } = {}) => {
    await (touch ? line(page, name).tap() : line(page, name).click());
    if (!stays) await menu(page).waitFor({ state: 'detached' });
  };
  /** Where the things of a page are that a menu opening, or a column taken hold of, must not move. */
  const standing = (page) =>
    page.evaluate(() => {
      const all = [...document.querySelectorAll('.view-head h1, .view-head button, table.grid th, table.grid td, .grid-new, .tab, .rail-button')];
      const scrolls = [...document.querySelectorAll('.grid-scroll')].flatMap((one) => [one.scrollLeft, one.scrollTop]);
      return JSON.stringify([window.scrollX, window.scrollY, ...scrolls, ...all.map((one) => Object.values(one.getBoundingClientRect().toJSON()).map((n) => Math.round(n * 10)))]);
    });
  /** The menu by its boxes: all of it in the window, each line inside it and none over another, and the page no wider for it. */
  const laidOut = async (page, where) => {
    const at = await page.evaluate(() => {
      const box = (one) => {
        const { left, top, right, bottom } = one.getBoundingClientRect();
        return { left, top, right, bottom, words: one.textContent.trim().slice(0, 30) };
      };
      const panel = document.querySelector('.headmenu');
      return { window: { width: window.innerWidth, height: window.innerHeight }, wide: document.documentElement.scrollWidth, panel: box(panel), lines: [...panel.querySelectorAll('.headmenu-line, .headmenu-column, .headmenu-whose')].map(box), sideways: panel.scrollWidth - panel.clientWidth };
    });
    const within = (inner, outer) => inner.left >= outer.left - 0.5 && inner.top >= outer.top - 0.5 && inner.right <= outer.right + 0.5 && inner.bottom <= outer.bottom + 0.5;
    const apart = (one, other) => one.right <= other.left + 0.5 || other.right <= one.left + 0.5 || one.bottom <= other.top + 0.5 || other.bottom <= one.top + 0.5;
    expect(`${where}: the menu to be wholly in the window, and it is at ${JSON.stringify(at.panel)} in ${JSON.stringify(at.window)}`, within(at.panel, { left: 0, top: 0, right: at.window.width, bottom: at.window.height }));
    expect(`${where}: the page not to be made wider by the menu, and it is ${at.wide} in ${at.window.width}`, at.wide <= at.window.width);
    expect(`${where}: the menu not to scroll sideways, and it does by ${at.sideways}`, at.sideways <= 0);
    expect(`${where}: the menu to have lines`, at.lines.length > 1);
    for (const [index, one] of at.lines.entries()) {
      expect(`${where}: each line to be inside the menu, and ${JSON.stringify(one)} is not in ${JSON.stringify(at.panel)}`, within(one, at.panel));
      for (const other of at.lines.slice(index + 1)) expect(`${where}: no line over another, and ${JSON.stringify(one)} is over ${JSON.stringify(other)}`, apart(one, other));
    }
    return at;
  };
  /** What the lines of the open menu say, in order, each without what is at its end. */
  const said = (page) => menu(page).locator('.headmenu-line .grow').allInnerTexts();
  /** Carries a column by its head to a place across the page and holds it there: the mouse is down until `drop` is asked. */
  const carry = async (page, column, toX, within = '') => {
    const from = await boxOf(pressed(page, column, within));
    const y = from.y + from.height / 2;
    await page.mouse.move(from.x + from.width / 2, y);
    await page.mouse.down();
    await page.mouse.move(toX, y + 2, { steps: 10 });
  };
  const dropLine = async (page) => {
    await page.waitForFunction(() => (document.querySelector('.grid-drop')?.style.left ?? '') !== '');
    return boxOf(page.locator('.grid-drop'));
  };

  const scheme = process.env.SCHEME ?? 'dark';
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const { page, counted } = await opened(context);

  // Thirty rows of stock, added as the page's own requests are made: enough for the table to scroll both ways.
  const added = await page.evaluate(async () => {
    const statuses = ['todo', 'doing', 'done'];
    for (let nth = 1; nth <= 30; nth++) {
      const fields = { title: `Item ${String(nth).padStart(2, '0')}`, status: nth === 30 ? null : statuses[nth % 3], labels: nth % 2 ? ['urgent'] : ['soon', 'later'], points: nth, packed: nth % 4 === 0, due: `2026-12-${String((nth % 28) + 1).padStart(2, '0')}`, shelf: `S${nth % 5}`, note: `Note ${nth}`, aisle: `A${nth}`, bin: `B${nth}`, maker: `Maker ${nth}`, zone: `Z${nth}` };
      const answer = await fetch('/api/collections/shop/stock/records', { method: 'POST', headers: { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' }, body: JSON.stringify({ fields }) });
      if (!answer.ok) return `${answer.status} ${await answer.text()}`;
    }
    return 'ok';
  });
  expect(`thirty rows of stock to be added, got ${added}`, added === 'ok');

  await show(page, 'stock');
  await page.locator('tr[data-row="shop-stock-30"]').waitFor();
  const beside = await opened(context);
  await show(beside.page, 'stock');
  await beside.page.locator('tr[data-row="shop-stock-30"]').waitFor();
  await beside.page.evaluate(() => {
    window.readOnce = true;
  });
  /** What a page shows and holds, for a wait that gave up to say what it saw: its table, its view, what it kept and what it asked since `since`. */
  const seen = async (page, counted, since) => {
    const shown = await page
      .evaluate(async () => {
        const all = (selector) => [...document.querySelectorAll(selector)];
        const kept = Object.fromEntries(Object.keys(localStorage).filter((key) => /^client\.(view|bands|sort|tabs)/.test(key)).map((key) => [key, localStorage.getItem(key).slice(0, 300)]));
        let daemon;
        try {
          const answer = await fetch('/api/collections?project=shop', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
          const stock = (await answer.json()).collections?.find((one) => one.name === 'stock');
          daemon = stock?.views ?? `no stock: ${answer.status}`;
        } catch (err) {
          daemon = String(err);
        }
        return {
          title: document.title,
          headings: all('h1').map((one) => one.textContent.trim()),
          tabs: all('.tab').map((one) => `${one.textContent.trim()}${one.getAttribute('aria-selected') === 'true' ? ' (showing)' : ''}`),
          views: all('[role="tablist"][aria-label="Views"] [role="tab"]').map((one) => `${one.textContent.trim()}${one.getAttribute('aria-selected') === 'true' ? ' (selected)' : ''}`),
          grids: all('table.grid').length,
          heads: all('table.grid thead th[data-column]').map((one) => one.dataset.column).join(' '),
          rows: all('table.grid tbody tr[data-row]').length,
          gathered: all('table.grid tr.grid-gathered').map((one) => one.textContent.replace(/\s+/g, ' ').trim()),
          kept,
          daemon,
        };
      })
      .catch((err) => `the page could not be asked: ${err.message}`);
    const asked = counted.asked.filter((one) => one.at >= since).map((one) => `${one.at - since}ms ${one.what} ${one.status ?? 'unanswered'}`);
    return JSON.stringify({ shown, asked }, null, 1);
  };
  const notReadAgain = async (what) => expect(`the other page not to have been read again ${what}`, await beside.page.evaluate(() => window.readOnce === true));

  // ---- a head is one button, and its menu
  expect(`the columns to stand as they always did, got ${(await columns(page)).join(' ')}`, (await columns(page)).join(' ') === STOOD.join(' '));
  expect('each head to be one button and nothing else to press, with the place a column is added from after the last', (await grid(page).locator('thead th[data-column] button').count()) === STOOD.length && (await grid(page).locator('thead th .grid-head').count()) === STOOD.length && (await grid(page).locator('thead th button').count()) === STOOD.length + 1);
  expect('no menu to be on the page until a head is pressed', (await menu(page).count()) === 0);
  const before = await standing(page);
  await open(page, 'Points');
  expect('nothing on the page to move as the menu opens', (await standing(page)) === before);
  const whole = await laidOut(page, 'opened');
  const opener = await boxOf(pressed(page, 'Points'));
  expect(`the menu to be under the head that opened it, and it is at ${JSON.stringify(whole.panel)} under ${JSON.stringify(opener)}`, Math.abs(whole.panel.left - opener.x) <= 0.5 && whole.panel.top >= opener.y + opener.height && whole.panel.top <= opener.y + opener.height + 8);
  const style = await menu(page).evaluate((panel) => {
    const drawn = getComputedStyle(panel);
    return { border: drawn.borderTopWidth, radius: parseFloat(drawn.borderTopLeftRadius), shadow: drawn.boxShadow };
  });
  expect(`the menu to be one panel with no frame, rounded and with a shadow, got ${JSON.stringify(style)}`, style.border === '0px' && style.radius >= 8 && style.shadow !== 'none');
  // What is true of a number today: a view is not held to one, and its rows are not gathered by one.
  expect(`the lines of a number's menu, got ${(await said(page)).join(' | ')}`, (await said(page)).join(' | ') === 'Sort ascending | Sort descending | Calculate | Freeze up to here | Hide | Move left | Move right | Insert left | Insert right | Duplicate column | What filling it takes | This column is read by stock, which declares it');
  // A column its table's blueprint declares is not retyped or taken away here, and there is no line for what is not built, greyed or not (`columned.mjs` holds what changes a table's columns).
  const all = (await menu(page).innerText()).toLowerCase();
  for (const never of ['change type', 'delete', 'property', 'autofill', 'access']) expect(`the menu to say nothing of "${never}"`, !all.includes(never));
  expect('the first line to be the column: its mark as a button for the icons, and what it is called in a box of its own', (await menu(page).locator('.headmenu-column').getByRole('button', { name: 'Icon of Points', exact: true }).count()) === 1 && (await menu(page).locator('.headmenu-column').getByLabel('Name of Points').inputValue()) === 'Points');
  await page.screenshot({ path: `${out}/headed-01-the-menu.png` });

  // The arrows move among its lines, End and Home go to the ends, and Escape puts it away with the caret back on the head.
  const at = () => page.evaluate(() => document.activeElement?.textContent?.trim() ?? '');
  expect(`the caret to be on the first line that does something, got ${await at()}`, (await at()) === 'Sort ascending');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow down to go to the next line, got ${await at()}`, (await at()) === 'Sort descending');
  await page.keyboard.press('End');
  expect(`End to go to the last line, got ${await at()}`, (await at()).startsWith('This column is read by stock'));
  await page.keyboard.press('ArrowDown');
  expect(`the arrow down from the last to come round to the first thing pressed, got ${await at()}`, await page.evaluate(() => document.activeElement?.getAttribute('aria-label') === 'Icon of Points'));
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  expect(`the arrow up to go back a line, got ${await at()}`, (await at()).startsWith('What filling it takes'));
  expect('nothing on the page to move as the caret goes among the lines', (await standing(page)) === before);
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  expect('the caret to be back on the head after Escape', await page.evaluate(() => document.activeElement?.classList.contains('grid-head') && document.activeElement.textContent.trim() === 'Points'));
  // Enter on the head opens it, and so does Space; a line is pressed by Enter.
  await page.keyboard.press('Enter');
  await menu(page).waitFor();
  await page.waitForFunction(() => document.activeElement?.closest('.headmenu') !== null);
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  await page.keyboard.press('Space');
  await menu(page).waitFor();
  await page.waitForFunction(() => document.activeElement?.textContent?.trim() === 'Sort ascending');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  await menu(page).waitFor({ state: 'detached' });
  await sortedAs(page, 'the menu to sort by its column, greatest first', 'Points', 'descending', ['30', '29', '28']);
  // The pointer crossing to it leaves it there; a press outside puts it away, and so does the head's own press.
  await open(page, 'Points');
  const from = await boxOf(pressed(page, 'Points'));
  const to = await boxOf(line(page, 'Hide'));
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 12 });
  expect('the menu to stay as the pointer crosses to it', (await menu(page).count()) === 1);
  expect('the line in force to be marked as on', (await line(page, 'Sort descending').getAttribute('aria-checked')) === 'true' && (await line(page, 'Sort ascending').getAttribute('aria-checked')) === 'false');
  await page.getByRole('heading', { name: 'Shop stock', level: 1 }).click();
  await menu(page).waitFor({ state: 'detached' });
  await open(page, 'Points');
  await pressed(page, 'Points').click();
  await menu(page).waitFor({ state: 'detached' });
  expect('none of that to have saved a view: sorting is the reader\'s own', counted.saves === 0);

  // Sorting is kept in the reader's browser for the view, and is no other page's.
  await page.reload();
  await page.locator('tr[data-row="shop-stock-30"]').waitFor();
  expect('the sort to be there when the page is read again', (await head(page, 'Points').getAttribute('aria-sort')) === 'descending' && (await rows(page))[0] === '30');
  expect('the other page to be sorted by nothing', (await head(beside.page, 'Points').getAttribute('aria-sort')) === 'none' && (await rows(beside.page))[0] === '1');
  // The line in force pressed again leaves the rows as they were added.
  await open(page, 'Points');
  await choose(page, 'Sort descending');
  await sortedAs(page, 'the rows to be as they were added', 'Points', 'none', ['1']);
  await open(page, 'Title');
  await choose(page, 'Sort descending');
  await sortedAs(page, 'a text to sort too', 'Title', 'descending', ['30']);
  await open(page, 'Title');
  await choose(page, 'Sort descending');

  // At the table's right edge the menu is moved to stay in the window.
  await page.locator('.grid-scroll').evaluate((box) => (box.scrollLeft = box.scrollWidth));
  await open(page, 'Zone');
  const edge = await laidOut(page, 'at the right edge');
  const last = await boxOf(pressed(page, 'Zone'));
  expect(`the menu at the right edge to be under its head still, got ${JSON.stringify(edge.panel)} under ${JSON.stringify(last)}`, edge.panel.top >= last.y + last.height && edge.panel.left < last.x + last.width);
  await page.screenshot({ path: `${out}/headed-02-at-the-right-edge.png` });
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  await page.locator('.grid-scroll').evaluate((box) => (box.scrollLeft = 0));

  // ---- hide: the column left out of the view, by the door the layout's switches use
  await open(page, 'Note');
  await choose(page, 'Hide');
  const less = STOOD.filter((one) => one !== 'Note');
  await standsAs(page, less);
  await standsAs(beside.page, less);
  await notReadAgain('for a column left out');
  await page.reload();
  await standsAs(page, less);
  // What a record is known by is never left out.
  await open(page, 'Title');
  expect(`the menu of what a record is known by to offer no hiding and no moving, got ${(await said(page)).join(' | ')}`, (await said(page)).join(' | ') === 'Filter | Sort ascending | Sort descending | Calculate | Freeze up to here | Insert right | Duplicate column | What filling it takes | This column is read by stock, which declares it');
  // Every row is called something of its own: the line says a view is not held to that, and is nothing to press.
  expect('the line that says a column is not filtered by to be nothing to press', (await menu(page).locator('button', { hasText: 'Filter' }).count()) === 0 && (await menu(page).locator('.headmenu-said').innerText()).includes('no two rows share a thing here'));
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  // Layout shows it again.
  const layout = page.locator('.view-head').getByRole('button', { name: /^Layout/ });
  await layout.click();
  const shownColumns = page.getByRole('group', { name: 'Columns shown' });
  await shownColumns.getByRole('switch', { name: 'Note', exact: true }).check();
  await standsAs(page, STOOD);
  await standsAs(beside.page, STOOD);
  await page.keyboard.press('Escape');

  // ---- filter: the view held to things of this column
  await open(page, 'Status');
  expect(`the lines of a choice's menu, got ${(await said(page)).join(' | ')}`, (await said(page)).join(' | ') === 'Options | Filter | Sort ascending | Sort descending | Group | Calculate | Freeze up to here | Hide | Move right | Insert left | Insert right | Duplicate column | What filling it takes | This column is read by stock, which declares it');
  await choose(page, /^Filter/, { stays: true });
  await line(page, 'Only Status doing').waitFor();
  await laidOut(page, 'what the view is held to');
  expect('the things there are to hold it to, in the order they can be chosen', (await menu(page).getByRole('menuitemcheckbox').evaluateAll((every) => every.map((one) => one.getAttribute('aria-label')).join(' | '))) === 'Only Status todo | Only Status doing | Only Status done');
  await page.screenshot({ path: `${out}/headed-03-filter.png` });
  await choose(page, 'Only Status doing', { stays: true });
  await page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 10);
  await page.locator('.held').getByText('Showing only').waitFor();
  await beside.page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 10);
  await notReadAgain('for a view held to something');
  expect('the thing it is held to to be ticked, the menu staying for another', (await line(page, 'Only Status doing').getAttribute('aria-checked')) === 'true' && (await menu(page).count()) === 1);
  await choose(page, 'Only Status done', { stays: true });
  await page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 20);
  // The way back is the list's first line, and the arrow to the left.
  await page.keyboard.press('ArrowLeft');
  await line(page, 'Sort ascending').waitFor();
  expect('the line to say how many things the view is held to here', (await line(page, /^Filter/).innerText()).includes('2 on'));
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 20);
  await open(page, 'Status');
  await choose(page, /^Filter/, { stays: true });
  await choose(page, 'Show all', { stays: true });
  await page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 30);
  await beside.page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 30);
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  // A text two rows share a word of is held to as well: its words in the order of words and numbers.
  await page.locator('.grid-scroll').evaluate((box) => (box.scrollLeft = box.scrollWidth));
  await open(page, 'Shelf');
  await choose(page, /^Filter/, { stays: true });
  expect('a text\'s shared words to be what a view is held to by it', (await menu(page).getByRole('menuitemcheckbox').evaluateAll((every) => every.map((one) => one.textContent.trim()).join(' '))) === 'S0 S1 S2 S3 S4');
  await choose(page, 'Only Shelf S3', { stays: true });
  await page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 6);
  await choose(page, 'Show all', { stays: true });
  await page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 30);
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  await page.locator('.grid-scroll').evaluate((box) => (box.scrollLeft = 0));

  // ---- calculate: a figure at the foot of a column, of the rows the view shows
  expect('no foot of figures where no column says one', (await page.locator('tr.grid-figures').count()) === 0);
  await open(page, 'Points');
  await choose(page, /^Calculate/, { stays: true });
  await laidOut(page, 'the figures');
  expect(`the figures a number can say, got ${(await menu(page).getByRole('menuitemradio').allInnerTexts()).map((one) => one.trim()).join(' | ')}`, (await menu(page).getByRole('menuitemradio').allInnerTexts()).map((one) => one.trim()).join(' | ') === 'Nothing | Filled | Empty | Sum | Mean | Least | Greatest');
  await page.screenshot({ path: `${out}/headed-04-calculate.png` });
  await choose(page, 'Sum');
  const foot = (which, column) => which.locator(`tr.grid-figures td[data-figure="${column}"]`);
  await foot(page, 'points').getByText('465').waitFor();
  await foot(beside.page, 'points').getByText('465').waitFor();
  await notReadAgain('for a figure');
  for (const [column, figure, wanted] of [['Packed', 'Ticked', 'Ticked 7'], ['Status', 'Different', 'Different 3'], ['Title', 'Filled', 'Filled 30'], ['Status', 'Empty', 'Empty 1']]) {
    await open(page, column);
    await choose(page, /^Calculate/, { stays: true });
    await choose(page, figure);
    await page.waitForFunction(([column, wanted]) => document.querySelector(`tr.grid-figures td[data-figure="${column}"]`)?.textContent.replace(/\s+/g, ' ').trim() === wanted, [column.toLowerCase(), wanted]);
  }
  // The foot stays at the table's foot as the heads stay at its top, however far the rows are scrolled.
  const ends = () =>
    page.evaluate(() => {
      const room = document.querySelector('.grid-scroll').getBoundingClientRect();
      const top = document.querySelector('table.grid thead th').getBoundingClientRect();
      const foot = document.querySelector('tr.grid-figures td').getBoundingClientRect();
      return { head: top.top - room.top, foot: room.bottom - foot.bottom, scrolls: document.querySelector('.grid-scroll').scrollHeight - document.querySelector('.grid-scroll').clientHeight };
    });
  for (const down of [0, 180]) {
    await page.locator('.grid-scroll').evaluate((box, down) => (box.scrollTop = down), down);
    const stays = await ends();
    expect(`the heads at the top and the figures at the foot with the rows scrolled ${down} of ${stays.scrolls}, got ${JSON.stringify(stays)}`, stays.scrolls > 200 && Math.abs(stays.head) <= 0.5 && stays.foot >= -0.5 && stays.foot <= 18);
  }
  await page.screenshot({ path: `${out}/headed-05-figures-at-the-foot.png` });
  // Held to some rows, a figure is of those; the mean of the rows shown, to two places.
  await open(page, 'Points');
  await choose(page, /^Calculate/, { stays: true });
  await choose(page, 'Mean');
  await foot(page, 'points').getByText('15.5').waitFor();
  await page.reload();
  await foot(page, 'points').getByText('15.5').waitFor();
  expect('the figures to be there when the page is read again', (await page.locator('tr.grid-figures td[data-figure]').count()) === 4);
  await open(page, 'Status');
  await choose(page, /^Filter/, { stays: true });
  await choose(page, 'Only Status doing', { stays: true });
  await foot(page, 'points').getByText('14.5').waitFor();
  await choose(page, 'Show all', { stays: true });
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  // Told nothing, a column says none; with none said anywhere there is no foot.
  for (const column of ['Packed', 'Status', 'Title']) {
    await open(page, column);
    await choose(page, /^Calculate/, { stays: true });
    await choose(page, 'Nothing');
  }
  await page.waitForFunction(() => document.querySelectorAll('tr.grid-figures td[data-figure]').length === 1);

  // ---- freeze: the columns up to this one stay while the rest are scrolled sideways
  await page.locator('.grid-scroll').evaluate((box) => (box.scrollTop = 0));
  await open(page, 'Status');
  await choose(page, 'Freeze up to here');
  await page.waitForFunction(() => document.querySelectorAll('table.grid thead th.frozen').length === 2);
  await beside.page.waitForFunction(() => document.querySelectorAll('table.grid thead th.frozen').length === 2);
  await notReadAgain('for a view frozen');
  const across = (which, within = '') =>
    which.evaluate((within) => {
      const x = (one) => Math.round(one.getBoundingClientRect().left * 10) / 10;
      const table = document.querySelector(`${within} table.grid`);
      const heads = Object.fromEntries([...table.querySelectorAll('thead th[data-column]')].map((one) => [one.dataset.column, { left: x(one), right: Math.round(one.getBoundingClientRect().right * 10) / 10 }]));
      const cells = [...table.querySelector('tbody tr[data-row]').children].map(x);
      const feet = [...(table.querySelector('tr.grid-figures')?.children ?? [])].map(x);
      const bands = [...table.querySelectorAll('tr.grid-bands th')].map((one) => ({ name: one.textContent.trim(), left: x(one), right: Math.round(one.getBoundingClientRect().right * 10) / 10 }));
      const room = table.parentElement.getBoundingClientRect();
      return { heads, cells, feet, bands, room: { left: room.left, right: room.right } };
    }, within);
  const frozenAt = [];
  for (const sideways of [240, 520]) {
    await page.locator('.grid-scroll').evaluate((box, sideways) => (box.scrollLeft = sideways), sideways);
    frozenAt.push(await across(page));
  }
  const [near, far] = frozenAt;
  expect(`the frozen columns to be where they were however far the rest are scrolled, got ${JSON.stringify(near.heads.title)} ${JSON.stringify(near.heads.status)} then ${JSON.stringify(far.heads.title)} ${JSON.stringify(far.heads.status)}`, near.heads.title.left === far.heads.title.left && near.heads.status.left === far.heads.status.left && Math.abs(near.heads.title.left - near.room.left) <= 0.5 && Math.abs(near.heads.status.left - near.heads.title.right) <= 0.5);
  expect(`the rest to have scrolled under them, got ${JSON.stringify(near.heads.points)} then ${JSON.stringify(far.heads.points)}`, Math.abs(near.heads.points.left - far.heads.points.left - 280) <= 0.5);
  expect('a row\'s cells and the figures at the foot to stay with their heads', Math.abs(far.cells[0] - far.heads.title.left) <= 0.5 && Math.abs(far.cells[1] - far.heads.status.left) <= 0.5 && Math.abs(far.feet[0] - far.heads.title.left) <= 0.5 && Math.abs(far.feet[1] - far.heads.status.left) <= 0.5);
  // What scrolls goes under what stays: at the middle of a frozen cell it is the frozen cell that is met.
  const met = await page.evaluate(() => {
    const cell = document.querySelector('table.grid tbody tr[data-row] td.frozen-last');
    const box = cell.getBoundingClientRect();
    return cell.contains(document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2));
  });
  expect('a frozen cell to lie over what is scrolled under it', met);
  await page.screenshot({ path: `${out}/headed-06-frozen-and-scrolled.png` });
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll('table.grid thead th.frozen').length === 2);

  // ---- a column carried by its head
  await page.locator('.grid-scroll').evaluate((box) => (box.scrollLeft = 0));
  const saves = counted.saves;
  const still = await standing(page);
  const due = await boxOf(pressed(page, 'Due'));
  const labels = await boxOf(pressed(page, 'Labels'));
  // Taken hold of and moved, it is carried: a line says where it would land, and nothing else has moved.
  await carry(page, 'Due', labels.x + 20);
  const mark = await dropLine(page);
  expect(`the line to be between the two heads it would land between, at ${labels.x}, and it is at ${JSON.stringify(mark)}`, Math.abs(mark.x + mark.width / 2 - labels.x) <= 1.5 && mark.width <= 3 && mark.height > 100);
  expect('nothing on the page to move while a column is carried', (await standing(page)) === still);
  expect('no menu to open for a head that is carried', (await menu(page).count()) === 0);
  await page.screenshot({ path: `${out}/headed-07-carried.png` });
  await page.mouse.up();
  const carried = ['Title', 'Status', 'Due', 'Labels', 'Points', 'Packed', 'Aisle', 'Bin', 'Maker', 'Note', 'Shelf', 'Zone'];
  await standsAs(page, carried);
  await standsAs(beside.page, carried);
  await notReadAgain('for a column carried');
  expect('the line to be gone once it is let go, and no menu opened by letting go', (await page.locator('.grid-drop').count()) === 0 && (await menu(page).count()) === 0);
  expect(`one view to have been saved by carrying it, got ${counted.saves - saves}`, counted.saves - saves === 1);
  // Carried and brought back to where it began, nothing is changed and nothing opened.
  await carry(page, 'Due', due.x + due.width + 60);
  await dropLine(page);
  await page.mouse.move((await boxOf(pressed(page, 'Due'))).x + 30, due.y + 10, { steps: 6 });
  await page.mouse.up();
  expect('a column brought back to where it began to change nothing and open nothing', (await columns(page)).join(' ') === carried.join(' ') && counted.saves - saves === 1 && (await menu(page).count()) === 0);
  // Escape while it is carried puts it back.
  await carry(page, 'Due', (await boxOf(pressed(page, 'Maker'))).x + 10);
  await dropLine(page);
  await page.keyboard.press('Escape');
  expect('Escape to take the line away', (await page.locator('.grid-drop').count()) === 0);
  await page.mouse.up();
  expect('a column put back by Escape to change nothing and open nothing', (await columns(page)).join(' ') === carried.join(' ') && counted.saves - saves === 1 && (await menu(page).count()) === 0);
  // What a record is known by is not carried, and nothing is let go before it.
  await carry(page, 'Title', (await boxOf(pressed(page, 'Points'))).x + 10);
  expect('what a record is known by not to be carried', (await page.locator('.grid-drop').count()) === 0);
  await page.mouse.up();
  await menu(page).waitFor({ state: 'detached' });
  await carry(page, 'Points', (await boxOf(pressed(page, 'Title'))).x + 10);
  const first = await dropLine(page);
  expect(`a column carried over what a record is known by to land after it, at ${JSON.stringify(first)}`, Math.abs(first.x + first.width / 2 - (await boxOf(pressed(page, 'Status'))).x) <= 1.5);
  await page.mouse.up();
  // Carried across the frozen edge it joins the frozen ones: the view is frozen up to Status still.
  const joined = ['Title', 'Points', 'Status', 'Due', 'Labels', 'Packed', 'Aisle', 'Bin', 'Maker', 'Note', 'Shelf', 'Zone'];
  await standsAs(page, joined);
  await page.waitForFunction(() => [...document.querySelectorAll('table.grid thead th.frozen')].map((one) => one.dataset.column).join(' ') === 'title points status');
  await standsAs(beside.page, joined);
  // A press that never moved is the menu still.
  await open(page, 'Points');
  expect('a press that carried nothing to open the menu', (await menu(page).count()) === 1);
  // ---- and moved a place by the menu's lines
  await choose(page, 'Move right', { stays: true });
  const moved = ['Title', 'Status', 'Points', 'Due', 'Labels', 'Packed', 'Aisle', 'Bin', 'Maker', 'Note', 'Shelf', 'Zone'];
  await standsAs(page, moved);
  await page.waitForFunction(() => [...document.querySelectorAll('table.grid thead th.frozen')].map((one) => one.dataset.column).join(' ') === 'title status');
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  await standsAs(beside.page, moved);
  await page.reload();
  await standsAs(page, moved);
  await page.screenshot({ path: `${out}/headed-08-the-order-kept.png` });

  // ---- the layout's list of columns is in that order, and a column is moved there by keys
  await layout.click();
  const listed = () => shownColumns.locator('.layout-column .truncate').allInnerTexts();
  expect(`the layout to list the columns as they stand, got ${(await listed()).join(' ')}`, (await listed()).join(' ') === moved.slice(1).join(' '));
  expect('the first of them to have nowhere earlier to go, and the last nowhere later', (await shownColumns.getByRole('button', { name: 'Move Status up' }).isDisabled()) && (await shownColumns.getByRole('button', { name: 'Move Zone down' }).isDisabled()));
  await shownColumns.getByRole('button', { name: 'Move Points down' }).focus();
  await page.keyboard.press('Enter');
  const byKeys = ['Title', 'Status', 'Due', 'Points', 'Labels', 'Packed', 'Aisle', 'Bin', 'Maker', 'Note', 'Shelf', 'Zone'];
  await standsAs(page, byKeys);
  await standsAs(beside.page, byKeys);
  await page.waitForFunction((wanted) => [...document.querySelectorAll('.layout-column .truncate')].map((one) => one.textContent).join(' ') === wanted, byKeys.slice(1).join(' '));
  await page.screenshot({ path: `${out}/headed-09-the-layouts-list.png` });
  await page.keyboard.press('Escape');

  // ---- group: the rows under a line for each thing the column says
  await open(page, 'Status');
  await choose(page, 'Group');
  await page.waitForFunction(() => document.querySelectorAll('table.grid tr.grid-gathered').length === 4);
  expect(`a line for each thing, in the order they can be chosen, and one for what says nothing, got ${(await page.locator('tr.grid-gathered').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim()).join(' | ')}`, (await page.locator('tr.grid-gathered').allInnerTexts()).map((one) => one.replace(/[▾▸\s]+/g, ' ').trim()).join(' | ') === 'todo 9 | doing 10 | done 10 | Ungrouped 1');
  await beside.page.waitForFunction(() => document.querySelectorAll('table.grid tr.grid-gathered').length === 4);
  await notReadAgain('for rows gathered');
  await page.locator('.grid-scroll').evaluate((box) => (box.scrollTop = 0));
  await page.screenshot({ path: `${out}/headed-10-gathered.png` });
  // A line folds what is under it, which is the reader's own.
  await page.locator('tr.grid-gathered').first().getByRole('button').click();
  await page.waitForFunction(() => document.querySelectorAll('table.grid tbody tr[data-row]').length === 21);
  expect('the other page to have folded nothing', (await beside.page.locator('table.grid tbody tr[data-row]').count()) === 30);
  await page.locator('tr.grid-gathered').first().getByRole('button').click();
  const reloaded = Date.now();
  const savesAsked = counted.saves;
  const savesAnswered = counted.answered;
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll('table.grid tr.grid-gathered').length === 4).catch(async (err) => {
    await page.screenshot({ path: `${out}/headed-gave-up.png` }).catch(() => {});
    throw new Error(`${err.message}\nwhen it gave up: ${await seen(page, counted, reloaded)}\nbefore the reload ${savesAsked} views were asked to be saved and ${savesAnswered} answered; errors in the page: ${problems.join(' | ') || 'none'}`);
  });
  await open(page, 'Status');
  await choose(page, 'Ungroup');
  await page.waitForFunction(() => document.querySelectorAll('table.grid tr.grid-gathered').length === 0);
  await beside.page.waitForFunction(() => document.querySelectorAll('table.grid tr.grid-gathered').length === 0);
  // Frozen no more, by the line of the column it is frozen up to.
  await open(page, 'Status');
  await choose(page, 'Unfreeze');
  await page.waitForFunction(() => document.querySelectorAll('table.grid .frozen').length === 0);
  await beside.page.waitForFunction(() => document.querySelectorAll('table.grid .frozen').length === 0);
  await notReadAgain('at the end of it');

  // ---- the board's table: the same head, saved by the board's own door
  await show(page, 'board');
  await page.locator('.card', { hasText: 'Basket' }).waitFor();
  await page.getByRole('button', { name: 'Add a view' }).click();
  await page.getByRole('menuitem', { name: /^Table/ }).click();
  await page.getByRole('tablist', { name: 'Views' }).getByRole('tab', { name: 'Table', exact: true, selected: true }).waitFor();
  await open(page, 'Status');
  await laidOut(page, 'on the board\'s table');
  await choose(page, 'Freeze up to here');
  // What stays is every column up to Status in the board's own order, as
  // the page reads it: which columns stand before Status is the board's
  // (a priority came to stand there), and is not written out here.
  const upTo = await page.waitForFunction(() => {
    const heads = [...document.querySelectorAll('table.grid thead th[data-column]')].map((one) => one.dataset.column);
    const frozen = [...document.querySelectorAll('table.grid thead th.frozen')].map((one) => one.dataset.column);
    const want = heads.slice(0, heads.indexOf('status') + 1);
    return heads.includes('status') && want.length > 1 && frozen.join(' ') === want.join(' ') && frozen;
  });
  const frozenOnBoard = await upTo.jsonValue();
  expect(`the board's table to be frozen from what a card is known by up to Status, got ${frozenOnBoard.join(' ')}`, frozenOnBoard[0] === 'title' && frozenOnBoard.at(-1) === 'status');
  await open(page, 'Title');
  await choose(page, /^Calculate/, { stays: true });
  await choose(page, 'Filled');
  await foot(page, 'title').getByText('1').waitFor();
  await page.screenshot({ path: `${out}/headed-11-the-board.png` });

  // ---- the agents' table, which says its columns in groups: in the drawer beside the board, and as a tab
  await gearLine(page, /^Agents/);
  const DRAWER = '.drawer[data-rest] .sits-table';
  await page.locator(`${DRAWER} table.grid tr[data-row="shop-agents-3"]`).waitFor();
  await open(page, 'Count', { within: DRAWER });
  await laidOut(page, 'from a table in the drawer');
  await page.screenshot({ path: `${out}/headed-12-in-the-drawer.png` });
  // Escape is the menu's first: the drawer stays.
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });
  expect('Escape to put the menu away and leave the drawer', (await page.locator(DRAWER).count()) === 1);
  await open(page, 'Count', { within: DRAWER });
  await choose(page, /^Calculate/, { stays: true });
  await choose(page, 'Sum');
  await page.locator(`${DRAWER} tr.grid-figures td[data-figure="count"]`).getByText('Sum').waitFor();
  await page.locator(DRAWER).getByRole('button', { name: 'Open as a tab' }).click();
  await page.getByRole('heading', { name: 'Shop agents', level: 1 }).waitFor();
  await page.keyboard.press('Escape');
  await drawerAtRest(page).waitFor({ state: 'detached' });
  // The still copy of the drawer that was put away goes out the way it came, a moment longer on a busy machine, and holds
  // a copy of the table: what is read of the table is read once it has gone.
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });
  await page.locator('.panes table.grid tr[data-row="shop-agents-3"]').waitFor();
  const bandsOf = () => page.locator('.panes table.grid tr.grid-bands th').evaluateAll((every) => every.map((one) => `${one.textContent.trim()}:${one.colSpan}`).join(' | '));
  const GROUPS = ':1 | Who they are:3 | What they are told:4 | What they may ask:1 | How their program works:5 | What their program can reach:5';
  expect(`the agents' columns under their five headings, got ${await bandsOf()}`, (await bandsOf()) === GROUPS);
  const stood = await columns(page);
  // Frozen up to a column inside a group: the band of headings is cut there, and each part is over its own columns.
  await open(page, 'Count');
  await choose(page, 'Freeze up to here');
  await page.waitForFunction(() => [...document.querySelectorAll('table.grid thead th[data-column].frozen')].map((one) => one.dataset.column).join(' ') === 'title role count');
  expect(`the heading of the group the edge falls in to be over both parts, got ${await bandsOf()}`, (await bandsOf()) === ':1 | Who they are:2 | Who they are:1 | What they are told:4 | What they may ask:1 | How their program works:5 | What their program can reach:5');
  for (const sideways of [0, 300, 700]) {
    await page.locator('.grid-scroll').evaluate((box, sideways) => (box.scrollLeft = sideways), sideways);
    const now = await across(page);
    const names = Object.keys(now.heads);
    let nth = 0;
    for (const band of now.bands) {
      const span = band.right - band.left;
      const under = names[nth];
      expect(`the heading "${band.name}" to begin where its first column ${under} does with the table scrolled ${sideways}, got ${band.left} and ${now.heads[under].left}`, Math.abs(band.left - now.heads[under].left) <= 0.5);
      let wide = 0;
      while (nth < names.length && wide < span - 0.5) wide += now.heads[names[nth]].right - now.heads[names[nth++]].left;
      expect(`the heading "${band.name}" to be as wide as its columns with the table scrolled ${sideways}, got ${span} and ${wide}`, Math.abs(wide - span) <= 1);
    }
    expect(`the frozen columns of a table in groups to stay with the table scrolled ${sideways}, got ${JSON.stringify(now.heads.count)}`, sideways === 0 || (Math.abs(now.heads.title.left - now.room.left) <= 0.5 && Math.abs(now.heads.count.left - now.heads.role.right) <= 0.5));
  }
  await page.locator('tr.grid-figures td[data-figure="count"]').getByText('Sum').waitFor();
  await page.screenshot({ path: `${out}/headed-13-agents-in-groups-frozen-with-a-figure.png` });
  await page.locator('.grid-scroll').evaluate((box) => (box.scrollLeft = 0));
  // A column is carried within its group: let go far outside it, it lands at the group's edge and no further.
  expect(`the agents' first columns to be what a row is called and who they are, got ${stood.slice(0, 5).join(' ')}`, stood.slice(0, 5).join(' ') === 'Title Role Count Rank Prompt');
  await carry(page, 'Rank', (await boxOf(pressed(page, 'Title'))).x + 30);
  const kept = await dropLine(page);
  expect(`a column carried out of its group to land at the group's edge, at ${JSON.stringify(kept)}`, Math.abs(kept.x + kept.width / 2 - (await boxOf(pressed(page, 'Role'))).x) <= 1.5);
  await page.screenshot({ path: `${out}/headed-13b-carried-within-its-group.png` });
  await page.mouse.up();
  await standsAs(page, ['Title', 'Rank', 'Role', 'Count', ...stood.slice(4)]);
  // The view is frozen up to Count still, so what was carried before it stays with it.
  await page.waitForFunction(() => [...document.querySelectorAll('table.grid thead th[data-column].frozen')].map((one) => one.dataset.column).join(' ') === 'title rank role count');
  expect(`the groups to be as the table says them, got ${await bandsOf()}`, (await bandsOf()) === GROUPS);
  // Its menu moves it within its group, and offers no way out of it.
  await open(page, 'Rank');
  expect(`the first of a group to be offered a move to the right alone, got ${(await said(page)).join(' | ')}`, (await said(page)).includes('Move right') && !(await said(page)).includes('Move left'));
  await choose(page, 'Move right', { stays: true });
  await standsAs(page, ['Title', 'Role', 'Rank', 'Count', ...stood.slice(4)]);
  await page.keyboard.press('Escape');
  await menu(page).waitFor({ state: 'detached' });

  // ---- somebody who may not change a table's views: sorting, which is theirs, and nothing else
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joinedIn = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joinedIn.status}`, joinedIn.ok);
  const theirs = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const reader = await opened(theirs, 'grace@example.com');
  const other = reader.page;
  await show(other, 'ledger');
  await other.locator('tr[data-row="shop-ledger-2"]').waitFor();
  await open(other, 'Sum');
  await laidOut(other, 'for somebody who may not change the view');
  expect(`somebody who may not save a view to be offered sorting alone, got ${(await said(other)).join(' | ')}`, (await said(other)).join(' | ') === 'Sort ascending | Sort descending | What filling it takes | This column is read by stock, which declares it');
  expect('the column\'s mark to be no button for them', (await menu(other).getByRole('button', { name: /^Icon of / }).count()) === 0 && (await menu(other).locator('.headmenu-icon svg').count()) === 1);
  await other.screenshot({ path: `${out}/headed-14-not-theirs-to-change.png` });
  await choose(other, 'Sort descending');
  await sortedAs(other, 'their sort to be theirs', 'Sum', 'descending', ['2', '1']);
  expect('their sort to be theirs, of the two rows there are', (await rows(other)).join(' ') === '2 1');
  // Their head is not carried.
  await carry(other, 'Sum', (await boxOf(pressed(other, 'Title'))).x + 30);
  expect('a head not to be carried by somebody who may not save a view', (await other.locator('.grid-drop').count()) === 0);
  await other.mouse.up();
  expect('nothing of theirs to have saved a view', reader.counted.saves === 0);
  // A table they may change offers them the rest.
  await show(other, 'stock');
  await other.locator('tr[data-row="shop-stock-30"]').waitFor();
  await open(other, 'Points');
  expect(`a member to be offered what changes a view of a table they may change, got ${(await said(other)).join(' | ')}`, (await said(other)).includes('Calculate') && (await said(other)).includes('Hide'));
  await theirs.close();

  // ---- a phone's width, by touch: the menu's lines, and nothing frozen
  await show(page, 'stock');
  await page.locator('tr[data-row="shop-stock-30"]').waitFor();
  await open(page, 'Status');
  await choose(page, 'Freeze up to here');
  await page.waitForFunction(() => document.querySelectorAll('table.grid thead th.frozen').length === 2);
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const held = (await opened(phone, 'ada@example.com', true)).page;
  await show(held, 'stock', true);
  await held.locator('.sidebar').waitFor({ state: 'detached' });
  await held.locator('tr[data-row="shop-stock-30"]').waitFor();
  await foot(held, 'points').getByText('15.5').waitFor();
  // What a record is known by is wider than half a phone's screen: frozen, it would leave no room to scroll, so nothing stays.
  expect('nothing to be frozen at a phone\'s width', (await held.locator('table.grid .frozen').count()) === 0);
  await pressed(held, 'Due').scrollIntoViewIfNeeded();
  const small = await standing(held);
  await open(held, 'Due', { touch: true });
  expect('nothing on a phone\'s page to move as the menu opens', (await standing(held)) === small);
  const fits = await laidOut(held, 'on a phone');
  expect(`the menu to fit a phone's screen, and it is ${fits.panel.right - fits.panel.left} wide`, fits.panel.right - fits.panel.left <= 374);
  const finger = await boxOf(line(held, 'Hide'));
  expect(`a line to be large enough for a finger, and it is ${finger.height} high`, finger.height >= 39.5);
  await held.screenshot({ path: `${out}/headed-15-phone.png` });
  await choose(held, 'Move right', { touch: true, stays: true });
  const byTouch = byKeys.map((one) => (one === 'Due' ? 'Points' : one === 'Points' ? 'Due' : one));
  await standsAs(held, byTouch);
  await held.keyboard.press('Escape');
  await menu(held).waitFor({ state: 'detached' });
  await standsAs(page, byTouch);
  await open(held, 'Status', { touch: true });
  await choose(held, /^Filter/, { touch: true, stays: true });
  await laidOut(held, 'on a phone, what the view is held to');
  await held.screenshot({ path: `${out}/headed-16-phone-filter.png` });
  await phone.close();

  // ---- the other of dark and light
  const otherScheme = scheme === 'dark' ? 'light' : 'dark';
  const turned = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: otherScheme });
  const lit = (await opened(turned)).page;
  await show(lit, 'stock');
  await lit.locator('tr[data-row="shop-stock-30"]').waitFor();
  await lit.waitForFunction(() => document.querySelectorAll('table.grid thead th.frozen').length === 2);
  await lit.locator('.grid-scroll').evaluate((box) => (box.scrollLeft = 260));
  await open(lit, 'Points');
  await laidOut(lit, 'in the other theme');
  await lit.screenshot({ path: `${out}/headed-17-${otherScheme}.png` });
  await choose(lit, /^Calculate/, { stays: true });
  await lit.screenshot({ path: `${out}/headed-18-${otherScheme}-figures.png` });
  await turned.close();

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
// How wide a column is, dragged by its head's edge, is held by a check of its own with a daemon of its own.
if (spawnSync(process.execPath, [new URL('./widths.mjs', import.meta.url).pathname, out], { stdio: 'inherit', env: process.env }).status !== 0) process.exit(1);
console.log(`pictures are in ${out}`);
