// The icon of a table's column: declared, chosen, followed and taken away.
//
//   DAEMON=../target/debug/factory node scripts/icons.mjs [out-dir]
//
// A blueprint declares four tables: one that says nothing of icons, one
// with a column that says its icon, one that only the factory's owner may
// change, and a board. Held here:
//
//   - a table with no icon looks as it did, its marks where they stood, and
//     the page asks for nothing of the icons; a column's head is one button
//     that opens its menu, whose first line has the column's mark as a
//     button for whoever may save a view, and the menu sorts;
//   - the panel the icons are chosen in: opening it moves nothing on the
//     page, all of it is in the window and nothing of it lies over anything
//     else of it, what is typed has the caret and narrows the icons by name
//     and by what they are found by, more are drawn as the end is scrolled
//     to, the arrows and Enter choose, Escape and a press outside close it,
//     and the pointer crossing to it does not;
//   - an icon chosen on one page is drawn on another that is open, which is
//     not read again; it is there after the blueprint is applied again; what
//     is chosen stands over what is declared, and taken away leaves what is
//     declared, or the mark of the kind; a name the client has no drawing
//     for draws the mark of the kind;
//   - somebody who may not change a table is shown its icons, and the mark
//     in a column's menu is no button for them;
//   - the board's table too; a phone's width; a light theme.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'icons-data');
const home = join(out, 'icons-home');

/** The tables, as they are first declared and as they are declared again: `more` is what the second time adds. */
const tables = (more) => `blueprint: stock
description: Tables whose columns have icons, and one that says nothing of them.
permissions: [projects, board]
steps:
  plain:
    do: collection.ensure
    with:
      name: plain
      fields:
        title: { type: text, required: true }
        points: { type: number }
        due: { type: date }${more ? '\n        note: { type: text }' : ''}
  stock:
    do: collection.ensure
    with:
      name: stock
      fields:
        title: { type: text, required: true }
        points: { type: number }
        shelf: { type: text, icon: ${more ? 'package' : 'warehouse'} }${more ? '\n        bin: { type: text }' : ''}
  ledger:
    do: collection.ensure
    with:
      name: ledger
      takes: admin
      fields:
        title: { type: text, required: true }
        sum: { type: number, icon: coins }
  board:
    do: collection.ensure
    with:
      name: board
      fields:
        title: { type: text, required: true }
        status: { type: choice, of: [todo, doing, done], required: true }
      views:
        board: { kind: board, by: status }
`;

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

// The parts of the client that hold the icons, by the name each is served
// under: the build before this one may still be on the disk beside it.
const assets = join(repo, 'ui/dist/assets');
const parts = [];
for (const name of await readdir(assets)) {
  if (name.endsWith('.js') && (await readFile(join(assets, name), 'utf8')).includes('lucide-static')) parts.push(name);
}

let failed = null;
try {
  if (parts.length === 0) throw new Error('expected the client that is built to have a part that holds the icons');
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  const shop = join(data, 'projects/shop');
  const plan = join(shop, 'blueprints/stock.bp');
  await mkdir(join(shop, 'blueprints'), { recursive: true });
  await writeFile(plan, tables(false));
  await cli('apply', plan, '--project', 'shop');
  for (const table of ['plain', 'stock', 'ledger']) {
    await cli('record', 'add', 'shop', table, 'title=Shelving');
    await cli('record', 'add', 'shop', table, 'title=Crates');
  }
  await cli('board', 'add', 'shop', 'Basket');

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

  /** A page that is signed in, with what it asked the daemon for kept beside it. */
  const opened = async (context, email = 'ada@example.com', touch = false) => {
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    const asked = [];
    page.on('request', (request) => asked.push(request.url()));
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
    /** Whether the page has asked for a part of the client that holds the icons. */
    const askedForIcons = () => asked.some((one) => parts.some((part) => one.endsWith(`/${part}`)));
    return { page, askedForIcons };
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
  /** The head of a column, by what the column is called. */
  const head = (page, column) => page.locator('table.grid thead th', { has: page.locator('.grid-head .truncate', { hasText: new RegExp(`^${column}$`) }) });
  /** The mark of a column as a button, in the first line of the column's menu. */
  const mark = (page, column) => page.locator('.headmenu').getByRole('button', { name: `Icon of ${column}`, exact: true });
  /** What a column's head is pressed by, and its menu. */
  const pressed = (page, column) => head(page, column).locator('.grid-head');
  const menu = (page) => page.locator('.headmenu');
  const panel = (page) => page.getByRole('dialog', { name: /^Icon of / });
  /** Waits for a column's mark to be the icon of a name, or with no name the mark of its kind. */
  const drawnAs = (page, column, icon) => head(page, column).locator(icon ? `svg[data-icon="${icon}"]` : 'svg[viewBox="0 0 16 16"]').waitFor();
  const icons = (page, column) => head(page, column).locator('svg[data-icon]').count();
  const boxOf = async (locator) => {
    const box = await locator.boundingBox();
    if (!box) throw new Error('expected something that is drawn to have a place');
    return box;
  };
  /** Holds a column's mark to where the marks stand in a head: nine from the edge of what sorts, the name thirty, fourteen each way and in the middle of its height. */
  const stands = async (page, column) => {
    const [sorts, drawn, name] = [await boxOf(head(page, column).locator('.grid-head')), await boxOf(head(page, column).locator('svg').first()), await boxOf(head(page, column).locator('.truncate'))];
    expect(`the mark of ${column} to stand nine from the edge of its head and its name thirty, and they stand ${drawn.x - sorts.x} and ${name.x - sorts.x}`, Math.abs(drawn.x - sorts.x - 9) <= 0.5 && Math.abs(name.x - sorts.x - 30) <= 0.5);
    expect(`the mark of ${column} to be fourteen each way, and it is ${drawn.width} by ${drawn.height}`, drawn.width === 14 && drawn.height === 14);
    const middle = drawn.y + drawn.height / 2 - (sorts.y + sorts.height / 2);
    expect(`the mark of ${column} to be in the middle of its head's height, and it is ${middle} from it`, Math.abs(middle) <= 0.5);
  };
  /** Opens the panel of a column, from the first line of the column's menu, and waits for its icons. */
  const open = async (page, column, touch = false) => {
    await (touch ? pressed(page, column).tap() : pressed(page, column).click());
    await (touch ? mark(page, column).tap() : mark(page, column).click());
    await menu(page).waitFor({ state: 'detached' });
    await panel(page).getByRole('option').first().waitFor();
  };
  /**
   * The panel by its boxes: all of it in the window, what is typed, the
   * icons and its foot each inside it and none over another, and no icon
   * over another or past the side of where they scroll.
   */
  const laidOut = async (page, where, whole = false) => {
    const at = await page.evaluate(() => {
      const box = (one) => {
        const { left, top, right, bottom } = one.getBoundingClientRect();
        return { left, top, right, bottom };
      };
      const panel = document.querySelector('.iconpick');
      const grid = panel.querySelector('.iconpick-grid');
      const shown = [...grid.querySelectorAll('[role="option"]')].map(box).filter((one) => one.bottom > box(grid).top && one.top < box(grid).bottom);
      return { window: { width: window.innerWidth, height: window.innerHeight }, wide: document.documentElement.scrollWidth, panel: box(panel), filter: box(panel.querySelector('.iconpick-filter')), grid: box(grid), foot: box(panel.querySelector('.iconpick-foot')), shown, sideways: grid.scrollWidth - grid.clientWidth };
    });
    const within = (inner, outer) => inner.left >= outer.left - 0.5 && inner.top >= outer.top - 0.5 && inner.right <= outer.right + 0.5 && inner.bottom <= outer.bottom + 0.5;
    const apart = (one, other) => one.right <= other.left + 0.5 || other.right <= one.left + 0.5 || one.bottom <= other.top + 0.5 || other.bottom <= one.top + 0.5;
    expect(`${where}: the panel to be wholly in the window, and it is at ${JSON.stringify(at.panel)} in ${JSON.stringify(at.window)}`, within(at.panel, { left: 0, top: 0, right: at.window.width, bottom: at.window.height }));
    expect(`${where}: the page not to be made wider by the panel, and it is ${at.wide} in ${at.window.width}`, at.wide <= at.window.width);
    for (const part of ['filter', 'grid', 'foot']) expect(`${where}: its ${part} to be inside it, and it is at ${JSON.stringify(at[part])} in ${JSON.stringify(at.panel)}`, within(at[part], at.panel));
    expect(`${where}: what is typed, the icons and the foot not to lie over each other`, apart(at.filter, at.grid) && apart(at.grid, at.foot) && apart(at.filter, at.foot));
    expect(`${where}: the icons not to scroll sideways, and they do by ${at.sideways}`, at.sideways <= 0);
    expect(`${where}: some icons to be shown`, at.shown.length > 0);
    // Where the panel has the height it asks for, the icons shown are whole rows: the last is not cut by the foot.
    if (whole) {
      const cut = at.shown.filter((one) => one.top < at.grid.top - 0.5 || one.bottom > at.grid.bottom + 0.5);
      expect(`${where}: no row of icons to be cut where they are first shown, and ${cut.length} icons are, in ${JSON.stringify(at.grid)}`, cut.length === 0);
    }
    for (const [index, one] of at.shown.entries()) {
      expect(`${where}: no icon past the side of where they scroll, and one is at ${JSON.stringify(one)}`, one.left >= at.grid.left - 0.5 && one.right <= at.grid.right + 0.5);
      for (const other of at.shown.slice(index + 1)) expect(`${where}: no icon over another, and ${JSON.stringify(one)} is over ${JSON.stringify(other)}`, apart(one, other));
    }
    return at;
  };
  /** Where the things of a page are that a panel opening must not move. */
  const standing = (page) =>
    page.evaluate(() => {
      const all = [...document.querySelectorAll('.view-head h1, .view-head button, table.grid th, table.grid td, .grid-new, .tab, .rail-button')];
      return JSON.stringify([window.scrollX, window.scrollY, document.querySelector('.grid-scroll').scrollLeft, document.querySelector('.grid-scroll').scrollTop, ...all.map((one) => Object.values(one.getBoundingClientRect().toJSON()).map((n) => Math.round(n * 10)))]);
    });

  const scheme = process.env.SCHEME ?? 'dark';
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const { page, askedForIcons } = await opened(context);

  // ---- a table that says nothing of icons: as it was, and nothing of them asked for
  await show(page, 'plain');
  await page.locator('tr[data-row="shop-plain-2"]').waitFor();
  expect('each column\'s head to be one button, with no button of its own for its mark', (await page.locator('table.grid thead th[data-column] button').count()) === 3 && (await page.locator('table.grid thead th .grid-head').count()) === 3);
  // Each mark is of its column's kind, where the marks always stood: nine from the edge of what sorts, its name thirty, in the middle of the head's height.
  for (const column of ['Title', 'Points', 'Due']) {
    await drawnAs(page, column, null);
    await stands(page, column);
  }
  // The head's menu sorts, as a press on the name did.
  await pressed(page, 'Title').click();
  await menu(page).getByRole('menuitemcheckbox', { name: 'Sort ascending' }).click();
  await menu(page).waitFor({ state: 'detached' });
  expect('the menu of a column\'s head to sort by it', (await head(page, 'Title').getAttribute('aria-sort')) === 'ascending');
  // The daemon sorts a table's rows and the window it answers comes a moment after the head says so.
  await page
    .waitForFunction(() => document.querySelector('table.grid tbody tr[data-row]')?.dataset.row === 'shop-plain-2', null, { timeout: 10000 })
    .catch(async () => {
      throw new Error(`expected the first row to be the first by name, got ${await page.locator('table.grid tbody tr[data-row]').first().getAttribute('data-row')}`);
    });
  await page.waitForLoadState('networkidle');
  expect('a page that draws no icon to have asked for no part of the client that holds them', !askedForIcons());
  expect('no panel to be on the page until a mark is pressed', (await page.locator('.iconpick').count()) === 0);
  expect('the menu of a head to have asked for no part that holds the icons either', !askedForIcons());
  await page.screenshot({ path: `${out}/icons-01-a-table-with-none.png` });

  // Another page is open on the same table, and has asked for nothing of the icons either.
  const beside = await opened(context);
  await show(beside.page, 'plain');
  await beside.page.locator('tr[data-row="shop-plain-2"]').waitFor();
  await drawnAs(beside.page, 'Points', null);
  await beside.page.waitForLoadState('networkidle');
  expect('the other page to have asked for no part that holds the icons', !beside.askedForIcons());
  await beside.page.evaluate(() => {
    window.readOnce = true;
  });

  // ---- the panel: opened by the mark, over the page and moving nothing
  const before = await standing(page);
  await open(page, 'Points');
  expect('the icons to have been asked for once the panel is opened', askedForIcons());
  expect('nothing on the page to move as the panel opens', (await standing(page)) === before);
  expect('where one types to have the caret', await page.evaluate(() => document.activeElement?.getAttribute('role') === 'combobox' && document.activeElement.closest('.iconpick') !== null));
  expect('nothing to take away where nothing was chosen', (await panel(page).getByRole('button', { name: 'Remove', exact: true }).count()) === 0);
  const whole = await laidOut(page, 'opened', true);
  const opener = await boxOf(pressed(page, 'Points'));
  expect(`the panel to be under the head it was opened from, and it is at ${JSON.stringify(whole.panel)} under ${JSON.stringify(opener)}`, whole.panel.left <= opener.x + 0.5 && whole.panel.right >= opener.x + opener.width && whole.panel.top >= opener.y + opener.height && whole.panel.top <= opener.y + opener.height + 8);
  // Every icon is said by its name to whoever cannot see it.
  const said = await panel(page).getByRole('option').evaluateAll((all) => all.map((one) => one.getAttribute('aria-label') ?? ''));
  expect(`each icon to be a button that says its name, and ${said.filter((one) => !one).length} of ${said.length} say none`, said.length === 240 && said.every((one) => /^[A-Z0-9]/.test(one)));
  await page.screenshot({ path: `${out}/icons-02-the-panel.png` });
  // More are drawn as the end of them is scrolled to, inside the panel.
  await page.locator('.iconpick-grid').evaluate((grid) => (grid.scrollTop = grid.scrollHeight));
  await page.waitForFunction(() => document.querySelectorAll('.iconpick [role="option"]').length > 240);
  expect('nothing on the page to move as the icons are scrolled', (await standing(page)) === before);
  await laidOut(page, 'scrolled');

  // What is typed narrows them, by name and by what each is found by.
  const filter = panel(page).getByRole('combobox');
  await filter.fill('time');
  await page.waitForFunction(() => document.querySelector('.iconpick [role="option"] svg')?.getAttribute('data-icon')?.startsWith('time'));
  const names = () => panel(page).getByRole('option').evaluateAll((all) => all.map((one) => one.querySelector('svg').getAttribute('data-icon')));
  const found = await names();
  const byName = found.filter((one) => one.includes('time'));
  expect(`those called by what was typed to come first, and the rest to be found by it, got ${found.slice(0, 6).join(', ')} of ${found.length}`, byName.length > 3 && found.length > byName.length && found.length < 240 && found.slice(0, byName.length).join() === byName.join());
  await laidOut(page, 'typed');
  await page.screenshot({ path: `${out}/icons-03-typed.png` });
  await filter.fill('qqqq');
  await panel(page).getByText('No icon is called that, or found by it.').waitFor();
  expect('no icon to be shown where none is found', (await panel(page).getByRole('option').count()) === 0);
  expect('nothing on the page to move as what is found changes', (await standing(page)) === before);

  // The arrows move a mark among them, and Enter chooses the one it is at.
  await filter.fill('time');
  await panel(page).getByRole('option').first().waitFor();
  const row = await panel(page).getByRole('option').evaluateAll((all) => all.filter((one) => one.offsetTop === all[0].offsetTop).length);
  const at = async () => {
    const id = await filter.getAttribute('aria-activedescendant');
    return page.evaluate((id) => document.getElementById(id)?.querySelector('svg')?.getAttribute('data-icon') ?? null, id);
  };
  expect(`the first found to be marked to begin with, got ${await at()}`, (await at()) === found[0]);
  await page.keyboard.press('ArrowDown');
  expect(`the arrow down to mark the one a row under, got ${await at()}`, (await at()) === found[row]);
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowLeft');
  const aimed = found[row + 1];
  expect(`the arrows to either side to move the mark by one, got ${await at()}`, (await at()) === aimed);
  expect('the one that is marked to be said at the foot', (await panel(page).locator('.iconpick-foot .truncate').innerText()).trim().toLowerCase().replaceAll(' ', '-') === aimed);
  await page.keyboard.press('Enter');
  await panel(page).waitFor({ state: 'detached' });
  await drawnAs(page, 'Points', aimed);
  expect('the caret to be back on the head the panel was opened from', await page.evaluate(() => document.activeElement?.classList.contains('grid-head') && document.activeElement.textContent.trim() === 'Points'));
  // Drawn as the marks are: the same size in the same place.
  await stands(page, 'Points');
  expect('the table to stand as it did but for the icon', (await standing(page)) === before);

  // The other page draws it, having been told and not read again.
  await drawnAs(beside.page, 'Points', aimed);
  expect('the other page not to have been read again', await beside.page.evaluate(() => window.readOnce === true));
  expect('the other page to have asked for the icons once there was one to draw', beside.askedForIcons());

  // Escape closes the panel, and so does a press outside it; the pointer crossing to it does not.
  await open(page, 'Title');
  await page.keyboard.press('Escape');
  await panel(page).waitFor({ state: 'detached' });
  expect('the caret to be back on the head after Escape', await page.evaluate(() => document.activeElement?.classList.contains('grid-head') && document.activeElement.textContent.trim().startsWith('Title')));
  await open(page, 'Title');
  const from = await boxOf(pressed(page, 'Title'));
  const to = await boxOf(panel(page).getByRole('option').nth(20));
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 12 });
  expect('the panel to stay as the pointer crosses to it', (await panel(page).count()) === 1);
  expect('nothing on the page to move as the pointer crosses to the panel', (await standing(page)) === before);
  await page.getByRole('heading', { name: 'Shop plain', level: 1 }).click();
  await panel(page).waitFor({ state: 'detached' });
  expect('a press outside to choose nothing', (await icons(page, 'Title')) === 0);
  // The head it was opened from closes it too, and opens nothing by that press.
  await open(page, 'Title');
  await pressed(page, 'Title').click();
  await panel(page).waitFor({ state: 'detached' });
  expect('the press that closes the icons to open no menu', (await menu(page).count()) === 0);
  // One is chosen by pressing it.
  await open(page, 'Title');
  await panel(page).getByRole('combobox').fill('heart');
  await panel(page).getByRole('option', { name: 'Heart', exact: true }).click();
  await panel(page).waitFor({ state: 'detached' });
  await drawnAs(page, 'Title', 'heart');
  await drawnAs(beside.page, 'Title', 'heart');
  await page.screenshot({ path: `${out}/icons-04-chosen.png` });

  // ---- declared, chosen over what is declared, kept through a declaration, taken away
  await show(page, 'stock');
  await drawnAs(page, 'Shelf', 'warehouse');
  await open(page, 'Shelf');
  expect('nothing to take away of an icon the table declared', (await panel(page).getByRole('button', { name: 'Remove', exact: true }).count()) === 0);
  await panel(page).getByRole('combobox').fill('star');
  await panel(page).getByRole('option', { name: 'Star', exact: true }).click();
  await drawnAs(page, 'Shelf', 'star');
  // The blueprint is applied again, saying another icon and more columns.
  await writeFile(plan, tables(true));
  await cli('apply', plan, '--project', 'shop');
  await head(page, 'Bin').waitFor();
  await drawnAs(page, 'Shelf', 'star');
  await head(beside.page, 'Note').waitFor();
  expect('what was chosen to be drawn after the table is declared again', (await head(beside.page, 'Points').locator(`svg[data-icon="${aimed}"]`).count()) === 1 && (await head(beside.page, 'Title').locator('svg[data-icon="heart"]').count()) === 1);
  await page.screenshot({ path: `${out}/icons-05-declared-again.png` });
  // Taken away, what the table now declares is drawn.
  await open(page, 'Shelf');
  await laidOut(page, 'with something to take away');
  await panel(page).getByRole('button', { name: 'Remove', exact: true }).click();
  await panel(page).waitFor({ state: 'detached' });
  await drawnAs(page, 'Shelf', 'package');
  // And of a column whose table declares none, the mark of its kind, on both pages.
  await show(page, 'plain');
  await drawnAs(page, 'Points', aimed);
  await open(page, 'Points');
  expect('the icon that is chosen to be marked among the others', (await panel(page).locator('.iconpick-one.chosen').count()) <= 1);
  await panel(page).getByRole('button', { name: 'Remove', exact: true }).click();
  await drawnAs(page, 'Points', null);
  await head(beside.page, 'Points').locator('svg[data-icon]').waitFor({ state: 'detached' });
  await drawnAs(beside.page, 'Points', null);

  // A name the client has no drawing for is drawn as the mark of the kind, and can be taken away.
  const kept = await page.evaluate(async () => {
    const answer = await fetch('/api/collections/shop/plain/fields/title/icon', { method: 'PUT', headers: { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' }, body: JSON.stringify({ icon: 'no-such-drawing' }) });
    return answer.status;
  });
  expect(`a name the client has never heard of to be kept, got ${kept}`, kept === 200);
  await head(page, 'Title').locator('svg[data-icon]').waitFor({ state: 'detached' });
  await drawnAs(page, 'Title', null);
  await open(page, 'Title');
  await panel(page).getByRole('button', { name: 'Remove', exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await panel(page).waitFor({ state: 'detached' });

  // ---- the board's table
  await show(page, 'board');
  await page.locator('.card', { hasText: 'Basket' }).waitFor();
  await page.getByRole('button', { name: 'Add a view' }).click();
  await page.getByRole('menuitem', { name: /^Table/ }).click();
  await page.getByRole('tablist', { name: 'Views' }).getByRole('tab', { name: 'Table', exact: true, selected: true }).waitFor();
  await open(page, 'Status');
  await panel(page).getByRole('combobox').fill('columns');
  await panel(page).getByRole('option').first().waitFor();
  const columns = (await panel(page).getByRole('option').first().locator('svg').getAttribute('data-icon')) ?? '';
  await page.keyboard.press('Enter');
  await drawnAs(page, 'Status', columns);
  await page.screenshot({ path: `${out}/icons-06-the-board.png` });

  // ---- somebody who may not change a table: its icons, and no button
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  const theirs = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const other = (await opened(theirs, 'grace@example.com')).page;
  await show(other, 'ledger');
  await drawnAs(other, 'Sum', 'coins');
  expect('the icon to be in the head of a table its reader may not change', (await other.locator('table.grid .grid-head svg[data-icon="coins"]').count()) === 1);
  // The head's menu shows them the mark, and it is no button.
  await pressed(other, 'Sum').click();
  await menu(other).locator('.headmenu-icon svg[data-icon="coins"]').waitFor();
  expect('no mark to be a button in the menu of a table its reader may not change', (await menu(other).getByRole('button', { name: /^Icon of / }).count()) === 0);
  await other.keyboard.press('Escape');
  await menu(other).waitFor({ state: 'detached' });
  // Where a mark is no button it stands where one that is a button does.
  await stands(other, 'Sum');
  await stands(other, 'Title');
  const refused = await other.evaluate(async () => {
    const answer = await fetch('/api/collections/shop/ledger/fields/sum/icon', { method: 'PUT', headers: { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' }, body: JSON.stringify({ icon: 'star' }) });
    return answer.status;
  });
  expect(`the daemon to refuse them the choice, got ${refused}`, refused === 403);
  await other.screenshot({ path: `${out}/icons-07-not-theirs-to-choose.png` });
  // A table they may change offers them the choice.
  await show(other, 'stock');
  await pressed(other, 'Shelf').click();
  await mark(other, 'Shelf').waitFor();
  await theirs.close();

  // ---- a phone's width, by touch
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const held = (await opened(phone, 'ada@example.com', true)).page;
  await show(held, 'stock', true);
  await held.locator('.sidebar').waitFor({ state: 'detached' });
  await drawnAs(held, 'Shelf', 'package');
  const still = await standing(held);
  await open(held, 'Points', true);
  expect('nothing on a phone\'s page to move as the panel opens', (await standing(held)) === still);
  const small = await laidOut(held, 'on a phone', true);
  expect(`the panel to fit a phone's screen, and it is ${small.panel.right - small.panel.left} wide in ${small.window.width}`, small.panel.right - small.panel.left <= 374);
  const touched = await boxOf(panel(held).getByRole('option').first());
  expect(`an icon to be large enough for a finger, and it is ${touched.width} by ${touched.height}`, touched.width >= 39.5 && touched.height >= 39.5);
  await held.screenshot({ path: `${out}/icons-08-phone.png` });
  await panel(held).getByRole('combobox').fill('number');
  await panel(held).getByRole('option').first().waitFor();
  await laidOut(held, 'on a phone, typed');
  await held.screenshot({ path: `${out}/icons-09-phone-typed.png` });
  const tapped = (await panel(held).getByRole('option').first().locator('svg').getAttribute('data-icon')) ?? '';
  await panel(held).getByRole('option').first().tap();
  await panel(held).waitFor({ state: 'detached' });
  await drawnAs(held, 'Points', tapped);
  await held.screenshot({ path: `${out}/icons-10-phone-chosen.png` });
  await phone.close();

  // ---- the other of dark and light
  const turned = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme === 'dark' ? 'light' : 'dark' });
  const lit = (await opened(turned)).page;
  await show(lit, 'stock');
  await drawnAs(lit, 'Points', tapped);
  await drawnAs(lit, 'Shelf', 'package');
  await lit.screenshot({ path: `${out}/icons-11-${scheme === 'dark' ? 'light' : 'dark'}.png` });
  await open(lit, 'Title');
  await panel(lit).getByRole('combobox').fill('arrow');
  await panel(lit).getByRole('option').first().waitFor();
  await laidOut(lit, 'in the other theme');
  await lit.screenshot({ path: `${out}/icons-12-${scheme === 'dark' ? 'light' : 'dark'}-typed.png` });
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
console.log(`pictures are in ${out}`);
