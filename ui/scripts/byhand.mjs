// A table whose rows are not written by hand: one that the commands and
// blueprints that keep it fill (a row for each heavy run of a script),
// read in the client and written there by nobody.
//
//   DAEMON=../target/debug/factory node scripts/byhand.mjs [out-dir]
//
// A project has a table of runs whose blueprint says `by_hand: false`,
// filled from a terminal, and beside it a table of notes that says
// nothing. Held here, by what the page shows and by the boxes:
//
//   - the runs' page says at its head, in a few faint words, that its rows
//     are written by what keeps it, and why where they are pointed at; it
//     offers no button that adds a row, no `+ New` line at the table's
//     foot, no `+` after the last head, and no cell to press: its cells
//     are words, a pressed one opens nothing to write in, and a tick is
//     shown and cannot be changed;
//   - a column's menu offers none of what changes the table's columns (no
//     name to write in, Change type, Options, Insert left, Insert right,
//     Duplicate column, Delete column), and everything that is how
//     somebody looks at it: sorting, filtering, gathering, a figure,
//     freezing, hiding, moving; gathering by a column there does what it
//     says, and the edge that makes a column wider is there;
//   - the gear writes it out as a CSV file and reads none in;
//   - a row opened is read in the drawer, every field closed and saying
//     why, with no Save, and taken away to the Trash by its Remove;
//   - a row added from a terminal while the page is open is shown;
//   - the notes' page, which says nothing, offers all of it as it did;
//   - nothing of the head, the table or the drawer lies over anything else
//     or past the window at 1280, 900 and 640 wide and at a phone's,
//     touched.
//
// The environment variables are those of walkthrough.mjs, with `LOOK`
// (`looked.ts`) to hold it at a look of a theme's.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { linesOf, menuOf, openGear } from './gear.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
// A check run several at once is given a folder of its own, and keeps its factory there.
const data = join(out, 'byhand-data');
const home = join(out, 'byhand-home');
const looked = process.env.LOOK ? '-looked' : '';

// A row's form has a little room between one field and the next:
// from the foot of a field's box to the head of the next
// field's label, twelve pixels times the density the form is drawn at, which
// is some room at the most compact density too. Measured by the boxes.
async function spaced(page, where) {
  const { rooms, density } = await page.evaluate(() => {
    const form = document.querySelector('.drawer .form');
    const fields = form ? [...form.querySelectorAll(':scope > :is(label, .field-many, .field-closed)')] : [];
    const rooms = [];
    for (let i = 1; i < fields.length; i++) {
      if (fields[i].previousElementSibling !== fields[i - 1]) continue;
      rooms.push(fields[i].getBoundingClientRect().top - fields[i - 1].getBoundingClientRect().bottom);
    }
    return { rooms, density: form ? parseFloat(getComputedStyle(form).getPropertyValue('--density')) || 1 : 1 };
  });
  const least = 12 * density - 0.5;
  if (rooms.length === 0 || !rooms.every((room) => room >= least && room > 0)) {
    throw new Error(`a row's form ${where} to have at least ${least + 0.5} px between one field and the next, got ${JSON.stringify(rooms)}`);
  }
}

const TABLES = `blueprint: kept
description: A table of runs that what keeps it fills, and a table of notes people write in.
permissions: [projects]
steps:
  runs:
    do: collection.ensure
    with:
      name: runs
      by_hand: false
      fields:
        title: { type: text, required: true }
        result: { type: choice, of: [passed, failed] }
        seconds: { type: number }
        held: { type: flag }
        commit: { type: text }
  notes:
    do: collection.ensure
    with:
      name: notes
      fields:
        title: { type: text, required: true }
        status: { type: choice, of: [todo, done] }
`;

/** What the runs' head says, and what it says where it is pointed at. */
const WORDS = 'written by what keeps it';

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
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  // A blueprint of the project's own, where the factory keeps it (`@blueprints/` of the project's).
  const plan = join(data, 'blueprints', 'projects', 'shop', 'kept.bp');
  await mkdir(join(data, 'blueprints', 'projects', 'shop'), { recursive: true });
  await writeFile(plan, TABLES);
  await cli('project', 'scan', 'shop');
  await cli('apply', plan, '--project', 'shop');
  // What a script's command adds, a row a run.
  for (const [title, result, seconds, held, commit] of [
    ['Nov 22, 9:14 AM', 'passed', 152, 'true', 'a0a76d9c'],
    ['Nov 22, 9:40 AM', 'failed', 61, 'false', 'b1c2d3e4'],
    ['Nov 22, 10:02 AM', 'passed', 148, 'true', 'c5d6e7f8'],
  ]) {
    await cli('record', 'add', 'shop', 'runs', `title=${title}`, `result=${result}`, `seconds=${seconds}`, `held=${held}`, `commit=${commit}`);
  }
  await cli('record', 'add', 'shop', 'notes', 'title=Shelving', 'status=todo');

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

  /** A page that is signed in. */
  const opened = async (context, touch = false) => {
    const page = await context.newPage();
    lastPage = page;
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
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
    await page.getByRole('heading', { name: `Shop ${table}`, level: 1 }).waitFor();
    await page.locator('table.grid tbody tr[data-row]').first().waitFor();
  };
  const grid = (page) => page.locator('.view-body table.grid, .split table.grid').first();
  const head = (page, column) => grid(page).locator('thead th[data-column]', { has: page.locator('.grid-head .truncate', { hasText: new RegExp(`^${column}$`) }) });
  const menu = (page) => page.locator('.headmenu:not(.colform)');
  /** What the lines of the open column's menu say, in order. */
  const said = (page) => menu(page).locator('.headmenu-line .grow').allInnerTexts();
  const openMenu = async (page, column) => {
    await head(page, column).locator('.grid-head').click();
    await menu(page).waitFor();
    await page.waitForFunction(() => document.querySelector('.headmenu')?.style.visibility !== 'hidden');
  };
  const shut = async (page) => {
    await page.keyboard.press('Escape');
    await menu(page).waitFor({ state: 'detached' });
  };
  const kept = (page) => page.locator('.view-head .table-kept');
  const crowdedAt = async (page, where, within = 'body') => {
    const wrong = await pageCrowded(page, within);
    expect(`nothing ${where} over anything else or past the window: ${wrong.join('; ')}`, wrong.length === 0);
  };
  const SHAPES = ['Change type', 'Options', 'Insert left', 'Insert right', 'Duplicate column', 'Delete column'];

  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const page = await opened(context);

  // The notes, which say nothing, are offered everything they always were.
  await show(page, 'notes');
  expect('no words at the head of a table that says nothing', (await kept(page).count()) === 0);
  expect('the notes offering a new row at their head', (await page.locator('.view-head').getByRole('button', { name: 'Add a note', exact: true }).count()) === 1);
  expect('the notes offering + New at their foot', (await page.getByRole('button', { name: '+ New note' }).count()) === 1);
  expect('the notes offering a column after their last head', (await page.getByRole('button', { name: 'Add a column', exact: true }).count()) === 1);
  expect('the notes\' cells pressed to be changed', (await grid(page).locator('tbody button.cell').count()) > 0 && (await grid(page).locator('tbody .cell.read').count()) === 0);
  await openMenu(page, 'Status');
  const notesMenu = await said(page);
  for (const line of ['Options', 'Insert left', 'Insert right', 'Duplicate column']) expect(`the notes' Status offering ${line}: ${notesMenu.join(' | ')}`, notesMenu.includes(line));
  expect('a name to write in at the head of the notes\' column menu', (await menu(page).locator('.headmenu-column input').count()) === 1);
  await shut(page);
  await openGear(page);
  expect('the notes read in from a CSV file', (await linesOf(page)).some((line) => line.startsWith('Import or merge a CSV')));
  await page.keyboard.press('Escape');
  await menuOf(page).waitFor({ state: 'detached' });

  // The runs: said at the head, and nothing that writes.
  await show(page, 'runs');
  await kept(page).waitFor();
  expect(`the runs' head saying ${WORDS}`, (await kept(page).innerText()).trim() === WORDS);
  expect('why, where those words are pointed at', /commands and blueprints that keep it/.test((await kept(page).getAttribute('title')) ?? ''));
  expect('no button that adds a run', (await page.locator('.view-head').getByRole('button', { name: 'Add a run', exact: true }).count()) === 0);
  expect('no + New at the foot', (await page.locator('.grid-new').count()) === 0);
  expect('the count still at the foot', /^3 runs$/.test((await grid(page).locator('.grid-count').innerText()).trim()));
  expect('no column added after the last head', (await page.getByRole('button', { name: 'Add a column', exact: true }).count()) === 0 && (await grid(page).locator('.grid-adds').count()) === 0);
  expect('no cell to press', (await grid(page).locator('tbody button.cell').count()) === 0);
  expect('every cell read', (await grid(page).locator('tbody .cell.read').count()) === 3 * 4);
  expect('a tick shown and not to be changed', (await grid(page).locator('tbody input[type="checkbox"]:checked:disabled').count()) === 2);
  await grid(page).locator('tbody .cell.read', { hasText: 'b1c2d3e4' }).click();
  expect('nothing to write in where a cell is pressed', (await page.locator('.cell-edit').count()) === 0);
  expect('an edge that makes a column wider', (await head(page, 'Commit').locator('.width-edge').count()) === 1);
  await page.mouse.move(1, 859);
  await page.screenshot({ path: `${out}/byhand-1-runs${looked}.png` });

  // A column's menu: how it is looked at, and nothing of what the columns are.
  await openMenu(page, 'Result');
  const lines = await said(page);
  for (const line of SHAPES) expect(`no ${line} in a column's menu: ${lines.join(' | ')}`, !lines.includes(line));
  for (const line of ['Filter', 'Sort ascending', 'Sort descending', 'Group', 'Calculate', 'Freeze up to here', 'Hide']) expect(`${line} in a column's menu: ${lines.join(' | ')}`, lines.includes(line));
  expect('no name to write in at the menu\'s head', (await menu(page).locator('.headmenu-column input').count()) === 0);
  expect('the column\'s name said at the menu\'s head', (await menu(page).locator('.headmenu-column strong').innerText()).trim() === 'Result');
  await crowdedAt(page, 'of the column\'s menu', '.headmenu');
  await page.screenshot({ path: `${out}/byhand-2-menu${looked}.png` });
  // Gathered by it, as any table is: the view is saved so.
  await menu(page).getByRole('menuitemcheckbox', { name: 'Group' }).click();
  await menu(page).waitFor({ state: 'detached' });
  await grid(page).locator('tr.grid-gathered').first().waitFor();
  expect('the runs gathered under passed and failed', (await grid(page).locator('tr.grid-gathered').count()) === 2);
  await openMenu(page, 'Result');
  await menu(page).getByRole('menuitemcheckbox', { name: 'Ungroup' }).click();
  await menu(page).waitFor({ state: 'detached' });
  await grid(page).locator('tr.grid-gathered').first().waitFor({ state: 'detached' });

  // The gear: written out, and none read in.
  await openGear(page);
  const gear = await linesOf(page);
  expect(`the runs written out as a CSV file: ${gear.join(' | ')}`, gear.some((line) => line.startsWith('Export as CSV')));
  expect(`no CSV read in: ${gear.join(' | ')}`, !gear.some((line) => line.startsWith('Import or merge a CSV')));
  await page.keyboard.press('Escape');
  await menuOf(page).waitFor({ state: 'detached' });

  // A row added from a terminal is shown, with the page not read again.
  await cli('record', 'add', 'shop', 'runs', 'title=Nov 22, 10:30 AM', 'result=passed', 'seconds=150', 'commit=d9e0f1a2');
  await grid(page).locator('tbody .cell.read', { hasText: 'd9e0f1a2' }).waitFor();

  // A row opened is read whole, closed, and still taken away.
  // Its Open is drawn where its row is pointed at.
  await grid(page).locator('tbody tr[data-row]', { hasText: 'b1c2d3e4' }).hover();
  await grid(page).getByRole('button', { name: 'Open Nov 22, 9:40 AM' }).click();
  const drawer = drawerAtRest(page);
  await drawer.waitFor();
  expect('every field of the form closed', (await drawer.locator('fieldset.field-closed[disabled]').count()) === 5);
  expect('why, under the form', /commands and blueprints that keep it/.test(await drawer.locator('.row-closed').innerText()));
  expect('no Save', (await drawer.getByRole('button', { name: 'Save', exact: true }).count()) === 0);
  await crowdedAt(page, 'of the drawer', '.drawer');
  await spaced(page, 'in the drawer');
  await page.screenshot({ path: `${out}/byhand-3-row${looked}.png` });
  await drawer.getByRole('button', { name: 'Remove this run', exact: true }).click();
  await grid(page).locator('tbody .cell.read', { hasText: 'b1c2d3e4' }).waitFor({ state: 'detached' });
  const trash = (await cli('trash', 'list')).stdout;
  expect(`the run in the Trash: ${trash}`, /9:40/.test(trash));

  // By the boxes, at three widths.
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await page.mouse.move(1, 859);
    await crowdedAt(page, `of the runs' page at ${width}`);
    await page.screenshot({ path: `${out}/byhand-4-runs-${width}${looked}.png` });
  }
  await context.close();

  // And a phone's, touched.
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const touched = await opened(phone, true);
  await show(touched, 'runs', true);
  await kept(touched).waitFor();
  expect('no cell to touch on a phone', (await grid(touched).locator('tbody button.cell').count()) === 0);
  expect('no button that adds a run on a phone', (await touched.locator('.view-head').getByRole('button', { name: 'Add a run', exact: true }).count()) === 0);
  await crowdedAt(touched, 'of the runs\' page on a phone');
  await touched.screenshot({ path: `${out}/byhand-5-phone${looked}.png` });
  await grid(touched).getByRole('button', { name: 'Open Nov 22, 9:14 AM' }).tap();
  await drawerAtRest(touched).waitFor();
  await crowdedAt(touched, 'of the drawer on a phone', '.drawer');
  await spaced(touched, 'in the drawer on a phone');
  await touched.screenshot({ path: `${out}/byhand-6-phone-row${looked}.png` });
  await phone.close();

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/byhand-failed${looked}.png` }).catch(() => {});
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
