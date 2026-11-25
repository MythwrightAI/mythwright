// Tables that sit beside another: a fleet's three, the one that says who it tells and its chain of who is next, reached from the board
// they are about.
//
//   DAEMON=../target/debug/factory node scripts/beside.mjs [out-dir]
//
// A project with a board and its fleet. The fleet's tables say they sit
// beside the board, so none is a line of its own under the project: the
// gear of the board's head lists them under Its tables with how many rows
// each has, lines that lie over nothing and move nothing, and the head
// has no button for them; each is opened in the drawer as the table it is (a cell changed
// there, and the fleet following; a row opened and the way back; a tab of
// its own from there), and each column's head says the rows that are
// about it, pressed to open that row's form. A row about a column is
// added from a terminal and the head says it with the page not read
// again; a table of the project's own that says the same is said beside
// the fleet's, and is a line of the board's gear with them. A table of
// the project's own with one beside it says so in its gear the same way,
// with no button in its head; opened in the drawer, where there is no
// gear, it has the button it always had. A board with no fleet, and tables
// declared as a program from before declared them, are as they always
// were: no heading in the gear and no line. Nothing lies over
// anything else and nothing scrolls sideways, with the drawer open and at
// a phone's width, where the drawer has the screen. The environment
// variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, watchLeaving, toolsAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { pageCrowded } from './crowded.mjs';
import { unfoldAll, unfoldChannels } from './folded.mjs';
import { gearLine, gearOf, headButtons, linesOf as gearLines, menuOf, openGear, tablesOf, tablesRead } from './gear.mjs';
import { saysWhere } from './where.ts';

/** Turns the side panel to a view on the rail, unless it is showing already: Resources with what it has read unfolded, as everything there starts folded (`folded.mjs`). */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  if (name === 'Resources') await unfoldAll(page);
  if (name === 'Channels') await unfoldChannels(page);
}

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'beside-data');
const home = join(out, 'beside-home');

/** Tables as a program from before declared a fleet's: neither says it sits beside anything. */
const BEFORE = `blueprint: before
permissions: [projects, records, agents]
steps:
  agents:
    do: collection.ensure
    with:
      name: agents
      fields:
        title: { type: text, required: true }
        role: { type: text }
        count: { type: number }
  stages:
    do: collection.ensure
    with:
      name: stages
      fields:
        title: { type: text, required: true }
        column: { type: text }
        fail: { type: text }
  work:
    needs: [stages]
    do: rows.start
    with: { collection: stages, rows: [{ title: review, column: review, fail: ready }] }
`;

/** A table of the project's own that sits beside its board, each row about a column. */
const LIMITS = `blueprint: limits
permissions: [projects, records]
steps:
  limits:
    do: collection.ensure
    with:
      name: limits
      beside: board
      about: column
      fields:
        title: { type: text, required: true }
        column: { type: text }
        most: { type: number }
`;

/** A table of the project's own that sits beside one of the fleet's, which is no board. */
const STEPS = `blueprint: steps
permissions: [projects, records]
steps:
  steps:
    do: collection.ensure
    with:
      name: steps
      beside: stages
      fields:
        title: { type: text, required: true }
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

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('board', 'add', 'shop', 'Add a cart');
  // A board with no fleet, and one whose tables were declared before a table could say what it sits beside.
  await cli('project', 'create', '--bare', 'Plain');
  await cli('apply', 'std/board', '--project', 'plain');
  await cli('project', 'create', '--bare', 'Old');
  await cli('apply', 'std/board', '--project', 'old');
  const before = join(out, 'beside-before.bp');
  await writeFile(before, BEFORE);
  await cli('apply', before, '--project', 'old');
  await cli('board', 'add', 'old', 'A card');
  await cli('board', 'move', 'old-1', 'review');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1500, height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const active = async () =>
    (await cli('agent', 'list')).stdout
      .split('\n')
      .filter((line) => / active/.test(line) && !/leaving/.test(line))
      .map((line) => line.trim().split(/\s+/)[0])
      .sort();

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await side(page, 'Resources');
  const nav = page.locator('.nav');
  const under = (project) => nav.locator(`[data-project="${project}"]`);
  await nav.getByText('Shop', { exact: true }).waitFor();
  // What is listed under a project, by what each line says.
  const lines = async (project) => (await under(project).locator('.tree > .nav-item, .tree > .nav-line > .nav-item, .tree > div > .nav-row .nav-item').allInnerTexts()).map((one) => one.trim().replace(/^\S\s+/, ''));
  const line = (project, name) => under(project).getByRole('button', { name: new RegExp(`${name}$`) }).first();

  // The fleet's three tables are no lines of their own under the project.
  for (const project of ['Shop', 'Plain', 'Old']) {
    const twist = nav.getByRole('button', { name: `Unfold ${project}`, exact: true });
    if (await twist.count()) await twist.click();
  }
  await line('shop', 'Board').waitFor();
  await line('old', 'Stages').waitFor();
  const listed = await lines('shop');
  expect(`the shop to list its board and none of the tables that sit beside it, got ${listed.join(', ')}`, listed.includes('Board') && !listed.some((line) => ['Agents', 'Stages', 'Segments', 'Tells'].includes(line)));
  // One declared as a program from before declared them lists them as it always did, and a board alone lists a board.
  const old = await lines('old');
  expect(`tables that say nothing of what they sit beside to be listed as they were, got ${old.join(', ')}`, old.includes('Board') && old.includes('Agents') && old.includes('Stages'));
  const plain = await lines('plain');
  expect(`a project with a board and no fleet to list its board, got ${plain.join(', ')}`, plain.includes('Board') && !plain.includes('Agents'));
  await page.screenshot({ path: `${out}/beside-0-side.png` });

  // The board's head has no button for them: its gear lists each under a heading, by what it is called and how many
  // rows it has, and nothing of the page moves as the menu opens.
  await line('shop', 'Board').click();
  await page.locator('.column', { hasText: 'backlog' }).first().waitFor();
  await gearOf(page).waitFor();
  expect(`the board's head to have no button for its tables, got ${(await headButtons(page)).join(' | ')}`, !(await headButtons(page)).some((one) => /^(Its tables|Tools)/.test(one)));
  const menu = menuOf(page);
  const standing = () => page.locator('.split').first().evaluate((split) => [...split.querySelectorAll('.view-head *, .column')].map((one) => { const at = one.getBoundingClientRect(); return [at.left, at.top, at.width, at.height].map((figure) => Math.round(figure * 10) / 10).join(','); }).join(' '));
  const stoodBefore = await standing();
  const offered = await tablesRead(page);
  expect(`the board's tables to be listed with their rows, got ${offered.join(' | ')}`, offered.join(' | ') === 'Actions no actions yet | Agents 3 agents | Chain no rows yet | Segments no segments yet | Stages 2 stages | Tells 1 tell');
  expect('nothing of the board or its head to move as its gear opens', (await standing()) === stoodBefore);
  // The heading is said and not pressed, the tables are under it and over what can be done to the board, and each
  // line is inside the menu, in the window, and under the one before it.
  const all = await gearLines(page);
  expect(`the tables to be under their heading and over the board's tools, got ${all.join(' | ')}`, all.slice(all.indexOf('Its tables'), all.indexOf('Its tables') + 8).join(' | ') === 'Its tables | Actions no actions yet | Agents 3 agents | Chain no rows yet | Segments no segments yet | Stages 2 stages | Tells 1 tell | Add a card to backlog');
  expect('the heading to be nothing to press', (await menu.getByRole('menuitem', { name: /^Its tables/ }).count()) === 0);
  const listedAt = await menu.evaluate((box) => {
    const edge = box.getBoundingClientRect();
    // Taller than the room under its gear, it scrolls in itself: a line is inside what it scrolls through.
    const scrolls = ['auto', 'scroll'].includes(getComputedStyle(box).overflowY) && box.scrollHeight > box.clientHeight + 0.5;
    const inside = { left: edge.left, right: edge.right, top: edge.top, bottom: scrolls ? edge.top + box.scrollHeight : edge.bottom };
    return { edge: { left: edge.left, right: edge.right, top: edge.top, bottom: edge.bottom }, inside, room: { width: window.innerWidth, height: window.innerHeight }, lines: [...box.querySelectorAll('button, .menu-said')].map((one) => { const at = one.getBoundingClientRect(); return { left: at.left, right: at.right, top: at.top, bottom: at.bottom }; }) };
  });
  expect(`the gear's menu to be in the window: ${JSON.stringify(listedAt.edge)}`, listedAt.edge.left >= 0 && listedAt.edge.top >= 0 && listedAt.edge.right <= listedAt.room.width && listedAt.edge.bottom <= listedAt.room.height);
  for (const [nth, one] of listedAt.lines.entries()) {
    expect(`line ${nth} of the gear's menu to be inside it: ${JSON.stringify(one)}`, one.left >= listedAt.inside.left - 0.5 && one.right <= listedAt.inside.right + 0.5 && one.top >= listedAt.inside.top - 0.5 && one.bottom <= listedAt.inside.bottom + 0.5);
    if (nth > 0) expect(`line ${nth} of the gear's menu not to lie over the one before it`, one.top >= listedAt.lines[nth - 1].bottom - 0.5);
  }
  await page.screenshot({ path: `${out}/beside-1-listed.png` });

  // What is laid out in something, by the boxes: which of its things lie
  // over each other, which reach outside it, and how far it and the page
  // scroll sideways.
  const boxes = (within, things) =>
    page.evaluate(
      ([root, chosen]) => {
        const frame = document.querySelector(root);
        const seen = [...frame.querySelectorAll(chosen)].filter((el) => el.getClientRects().length > 0);
        const box = (el) => {
          const at = el.getBoundingClientRect();
          return { what: (el.getAttribute('aria-label') || el.innerText || el.className).trim().slice(0, 40), left: at.left, right: at.right, top: at.top, bottom: at.bottom };
        };
        const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
        const all = seen.map(box);
        const edge = frame.getBoundingClientRect();
        return {
          count: all.length,
          over: all.flatMap((a, at) => all.slice(at + 1).filter((b) => over(a, b)).map((b) => `${a.what} over ${b.what}`)),
          outside: all.filter((one) => one.left < edge.left - 0.5 || one.right > edge.right + 0.5).map((one) => one.what),
          sideways: frame.scrollWidth - frame.clientWidth,
          page: document.documentElement.scrollWidth - window.innerWidth,
        };
      },
      [within, things],
    );
  const laid = async (how, within, things) => {
    const found = await boxes(within, things);
    expect(`something to be laid out ${how}`, found.count > 0);
    expect(`nothing to lie over anything else ${how}, got ${found.over.join('; ')}`, found.over.length === 0);
    expect(`everything to be inside ${how}, got ${found.outside.join('; ')}`, found.outside.length === 0);
    expect(`nothing to scroll sideways ${how}, by ${found.sideways} and the page by ${found.page}`, found.sideways <= 0 && found.page <= 0);
  };
  // What a column's head is made of: its name, the mark for what is set up for it, how many cards, and what adds one.
  const HEAD = '.column-name > span:first-child, .column-set, .column-head > .faint, .column-head > button';
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  await laid('in the board\'s head', '.view-head', 'h1, button, .muted');
  for (const column of await page.locator('.board .column').all()) {
    await column.evaluate((el) => el.setAttribute('data-checked', 'yes'));
    await laid('in a column\'s head', '.column[data-checked="yes"] .column-head', HEAD);
    await column.evaluate((el) => el.removeAttribute('data-checked'));
  }

  // Each column's head says only its name and its count, with one mark beside the name where anything is about the
  // column, which opens a menu under it: a heading for each table, and each row about the column by what it
  // is called with a few faint words on what it is. Nothing where no row is about one.
  const columnOf = (on, column) => on.locator('.board .column', { has: on.locator('.column-name > span:first-child', { hasText: new RegExp(`^${column}$`, 'i') }) });
  const setMenu = (on) => on.locator('.row-menu:not(.row-beside)');
  // The menu a column's mark opens, a line a thing: a heading in square brackets, a row by what it is called with its
  // few words after `—`, and a line that adds one after `+`; by its boxes, under its mark, in the window, with nothing
  // of the board moved.
  const setOf = async (on, column, press = 'click') => {
    const mark = columnOf(on, column).locator('.column-set');
    if ((await mark.count()) === 0) return null;
    const cards = () => on.locator('.board .column .cards').evaluateAll((all) => all.map((one) => Math.round(one.getBoundingClientRect().top)).join());
    const before = await cards();
    await mark[press]();
    await setMenu(on).waitFor();
    const read = await on.evaluate(() => {
      const menu = document.querySelector('.row-menu:not(.row-beside)');
      return {
        lines: [...menu.children].map((one) =>
          one.matches('button')
            ? (one.querySelector('.menu-sign svg') ? '+' : '') + one.querySelector('.truncate').textContent.trim() + (one.querySelector('.menu-under') ? ' — ' + one.querySelector('.menu-under').textContent.trim() : '')
            : '[' + one.textContent.trim() + ']',
        ),
        box: menu.getBoundingClientRect().toJSON(),
        width: window.innerWidth,
        height: window.innerHeight,
      };
    });
    const at = await mark.boundingBox();
    expect(`the menu of ${column}'s mark under it and in the window: ${JSON.stringify(read.box)}`, read.box.top >= at.y + at.height - 0.5 && read.box.left >= 0 && read.box.right <= read.width + 0.5 && read.box.bottom <= read.height + 0.5);
    const wrong = await pageCrowded(on, '.row-menu');
    expect(`nothing of the menu of ${column} over anything else of it: ${wrong.join('; ')}`, wrong.length === 0);
    expect(`nothing of the board moved by the menu of ${column}`, (await cards()) === before);
    return read.lines;
  };
  const shut = async (on) => {
    await on.keyboard.press('Escape');
    await setMenu(on).waitFor({ state: 'detached' });
  };
  const said = async (column) => {
    const lines = await setOf(page, column);
    if (lines === null) return [];
    await shut(page);
    return lines.filter((one) => !one.startsWith('[') && !one.startsWith('+')).map((one) => one.split(' — ')[0]);
  };
  const heads = async () => {
    const all = {};
    for (const column of ['backlog', 'ready', 'doing', 'review', 'done']) all[column] = (await said(column)).join('+');
    return all;
  };
  expect('a column\'s head to say nothing but its name and count beside its mark', (await page.locator('.board .column-head .sits-row, .board .column-head .sits-about').count()) === 0);
  const first = await heads();
  expect(`ready to say the stage that works and review the one that reviews, and the rest nothing, got ${JSON.stringify(first)}`, JSON.stringify(first) === JSON.stringify({ backlog: '', ready: 'work', doing: '', review: 'review', done: '' }));
  // Where every head is, before anything more is said in one: nothing moves when a row comes to be about a column.
  const tops = () => page.locator('.board .column .cards').evaluateAll((all) => all.map((one) => Math.round(one.getBoundingClientRect().top)));
  const stood = await tops();
  expect(`the cards of every column to begin at one height, got ${stood.join(', ')}`, new Set(stood).size === 1);
  // What ready's mark opens, whole: the stage under its table's heading, with what the factory's bot does there in a
  // sentence, and a line to add one for ready.
  const readySet = await setOf(page, 'ready');
  expect(`ready's menu to say its stage's rule under its heading and offer to add one, got ${JSON.stringify(readySet)}`, JSON.stringify(readySet) === JSON.stringify(['[Stages]', 'work — Worked by the worker role, as many at a time as its agents can take → review on pass', '+Add a stage for ready…']));
  // And review's, where cards are reviewed: where a card goes on to, and where one sent back goes.
  const reviewSet = await setOf(page, 'review');
  expect(`review's menu to say its stage's rule, got ${JSON.stringify(reviewSet)}`, JSON.stringify(reviewSet) === JSON.stringify(['[Stages]', 'review — Reviewed by the reviewer role, as many at a time as its agents can take → done on pass · back to doing on fail', '+Add a stage for review…']));
  await shut(page);
  expect('a mark with nothing holding the column\'s cards to have no dot', (await columnOf(page, 'ready').locator('.column-set-dot').count()) === 0);
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await shut(page);
    await setOf(page, 'ready');
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/beside-1b-column-set.png` });
  // By the keyboard: Enter on the mark, the menu with the keyboard in it, and Escape giving it back to the mark.
  await shut(page);
  await columnOf(page, 'ready').locator('.column-set').focus();
  await page.keyboard.press('Enter');
  await setMenu(page).waitFor();
  await page.waitForFunction(() => !!document.activeElement?.closest('.row-menu'));
  await shut(page);
  expect('the keyboard given back to the mark', await columnOf(page, 'ready').locator('.column-set').evaluate((one) => document.activeElement === one));

  // One is opened in the drawer, as the table it is, beside the board.
  const drawer = drawerAtRest(page);
  const aside = drawer.locator('.sits-table');
  const open = async (name) => {
    await gearLine(page, new RegExp(`^${name}`));
    await aside.waitFor();
    await drawer.getByRole('heading', { name: `Shop ${name.toLowerCase()}`, exact: true }).waitFor();
  };
  await open('Agents');
  const table = aside.locator('table.grid');
  await table.locator('tr[data-row="shop-agents-3"]').waitFor();
  expect('the board to be showing still, beside it', await page.locator('.board .column').first().isVisible());
  const cell = (label) => table.getByRole('button', { name: label, exact: true });
  const words = async (label) => (await cell(label).innerText()).trim();
  expect(`the fleet to start with nobody who reviews, got ${await words('Count of reviewer')}`, (await words('Count of reviewer')) === '0');
  await laid('in the head of a table in the drawer', '.drawer .view-head', 'button, .muted');
  await laid('in the drawer, which the table does not push sideways', '.drawer-body', '.view-head, .grid-scroll');
  await page.screenshot({ path: `${out}/beside-2-drawer.png` });

  // A cell is changed there, and the fleet follows, as it does from the table's own page.
  const until = async (what, wanted) => {
    let last = [];
    for (let tries = 0; tries < 100; tries += 1) {
      last = await active();
      if (wanted(last)) return last;
      await page.waitForTimeout(100);
    }
    throw new Error(`expected ${what}, and the agents are ${last.join(', ')}`);
  };
  await cell('Count of reviewer').click();
  const input = table.locator('.cell-edit input').first();
  await input.waitFor();
  await input.fill('1');
  await input.press('Enter');
  await table.locator('.cell-edit').waitFor({ state: 'detached' });
  await until('one who reviews to be brought up from a cell changed in the drawer', (names) => names.includes('shop-reviewer-1'));
  // The cell says it once the page has heard of the change and read its
  // rows again, which may be after the fleet has followed: it is waited
  // for, and was read straight away where the fleet was the slower.
  await cell('Count of reviewer').filter({ hasText: /^\s*1\s*$/ }).waitFor();
  expect(`the cell to say what was written, got ${await words('Count of reviewer')}`, (await words('Count of reviewer')) === '1');

  // A row is added there and taken away again: opened, its form takes the table's place and gives it back.
  await aside.getByRole('button', { name: 'Add an agent', exact: true }).click();
  const naming = table.locator('.cell-edit input').first();
  await naming.waitFor();
  await naming.fill('second');
  await naming.press('Enter');
  await table.locator('tr[data-row="shop-agents-4"]').waitFor();
  await until('the row added in the drawer to be an agent', (names) => names.includes('shop-second-1'));
  await table.locator('tr[data-row="shop-agents-4"]').hover();
  await table.getByRole('button', { name: 'Open second', exact: true }).click();
  const back = drawer.getByRole('button', { name: '‹ All agents', exact: true });
  await back.waitFor();
  await drawer.getByRole('heading', { name: 'second', exact: true }).waitFor();
  await laid('in a row\'s form in the drawer', '.drawer-body', 'label, .drawer-body > button, .form > .row > button');
  await page.screenshot({ path: `${out}/beside-3-row.png` });
  await back.click();
  await table.locator('tr[data-row="shop-agents-4"]').waitFor();
  await table.locator('tr[data-row="shop-agents-4"]').hover();
  await table.getByRole('button', { name: 'Open second', exact: true }).click();
  await drawer.getByRole('button', { name: 'Remove this agent', exact: true }).click();
  // Taken away, the table is shown again without it.
  await table.locator('tr[data-row="shop-agents-3"]').waitFor();
  await table.locator('tr[data-row="shop-agents-4"]').waitFor({ state: 'detached' });
  await until('the agent of the row that was taken away to be gone', (names) => !names.includes('shop-second-1'));

  // Its views are there as on its own page, and what opens under a button of its head is inside the drawer.
  await aside.getByRole('button', { name: /^Layout/ }).click();
  const panel = aside.getByRole('dialog', { name: 'Layout' });
  await panel.waitFor();
  const [within, opened] = [await drawer.boundingBox(), await panel.boundingBox()];
  expect(`what Layout opens to be inside the drawer, at ${JSON.stringify(opened)} in ${JSON.stringify(within)}`, opened.x >= within.x - 0.5 && opened.x + opened.width <= within.x + within.width + 0.5 && opened.y + opened.height <= within.y + within.height + 0.5);
  await page.screenshot({ path: `${out}/beside-3-layout.png` });
  await page.waitForTimeout(200);
  // Escape is first for what is open in the table, and the table stays: what Layout opened, and a cell being
  // filled in, which is then as it was. With nothing open it puts the drawer away, as it does a card.
  await page.keyboard.press('Escape');
  await panel.waitFor({ state: 'detached' });
  // Wherever the caret is: with it nowhere in the table, Escape still puts away what is open and leaves the table.
  await aside.getByRole('button', { name: /^Layout/ }).click();
  await panel.waitFor();
  await page.evaluate(() => document.activeElement?.blur());
  expect('the caret to be out of the drawer', await page.evaluate(() => !document.querySelector('.drawer').contains(document.activeElement)));
  // What Layout opens listens for Escape, and for a press outside it, from a moment after it is drawn.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await panel.waitFor({ state: 'detached' });
  expect('the table to be showing still, once what was open in it is put away with the caret elsewhere', await aside.isVisible());
  await cell('Count of worker').click();
  await table.locator('.cell-edit input').first().fill('7');
  await page.keyboard.press('Escape');
  await table.locator('.cell-edit').waitFor({ state: 'detached' });
  expect(`a cell given up with Escape to be as it was, in a table still showing, got ${await words('Count of worker')}`, (await words('Count of worker')) === '1' && (await aside.isVisible()));
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });

  // A stage is opened from the head of its column into the form its row has anywhere.
  await setOf(page, 'ready');
  await setMenu(page).locator(':scope > button', { hasText: /^work/ }).click();
  await drawer.getByRole('heading', { name: 'work', exact: true }).waitFor();
  expect(`the form to be the row's, saying its column, got ${await drawer.getByLabel('Column', { exact: true }).inputValue()}`, (await drawer.getByLabel('Column', { exact: true }).inputValue()) === 'ready');
  expect('a form opened from a column to have no table to go back to', (await drawer.getByRole('button', { name: /^‹ All/ }).count()) === 0);
  await page.screenshot({ path: `${out}/beside-4-stage.png` });
  // Changed there, the row is about another column, and that column's head says it: nothing is read again by hand.
  await drawer.getByLabel('Column', { exact: true }).fill('doing');
  await drawer.getByRole('button', { name: 'Save', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  await page.waitForFunction(() => [...document.querySelectorAll('.board .column')].some((column) => /^doing$/i.test(column.querySelector('.column-name > span')?.textContent ?? '') && column.querySelector('.column-set')));
  expect(`ready to say nothing once its stage is about doing, got ${JSON.stringify(await heads())}`, (await said('ready')).length === 0);
  await cli('record', 'set', 'shop-stages-1', 'column=ready');
  await page.waitForFunction(() => [...document.querySelectorAll('.board .column')].some((column) => /^ready$/i.test(column.querySelector('.column-name > span')?.textContent ?? '') && column.querySelector('.column-set')));
  expect(`ready to say its stage again, got ${JSON.stringify(await heads())}`, (await said('ready')).join('+') === 'work');

  // A table of the project's own says the same, and is listed and said with the fleet's from the moment it is
  // declared: a row about a column is added from a terminal and the head says it, two rows of two tables in one.
  const limits = join(out, 'beside-limits.bp');
  await writeFile(limits, LIMITS);
  await cli('apply', limits, '--project', 'shop');
  await cli('record', 'add', 'shop', 'limits', 'title=three waiting', 'column=ready', 'most=3');
  await cli('record', 'add', 'shop', 'limits', 'title=two in hand', 'column=doing', 'most=2');
  // A row that says no column is about the one it is called after.
  await cli('record', 'add', 'shop', 'limits', 'title=done');
  await page.waitForFunction(() => document.querySelectorAll('.board .column-set').length === 4);
  const second = await heads();
  expect(`each head to say the rows about it, a table at a time, got ${JSON.stringify(second)}`, JSON.stringify(second) === JSON.stringify({ backlog: '', ready: 'three waiting+work', doing: 'two in hand', review: 'review', done: 'done' }));
  expect(`nothing to have moved for what the heads came to say, got ${(await tops()).join(', ')}`, (await tops()).join() === stood.join());
  for (const column of await page.locator('.board .column').all()) {
    await column.evaluate((el) => el.setAttribute('data-checked', 'yes'));
    await laid('in a column\'s head that says two rows', '.column[data-checked="yes"] .column-head', HEAD);
    await column.evaluate((el) => el.removeAttribute('data-checked'));
  }
  const withLimits = await tablesRead(page);
  expect(`the table of the project's own to be a line of the board's gear with the fleet's, got ${withLimits.join(' | ')}`, withLimits.join(' | ') === 'Actions no actions yet | Agents 3 agents | Chain no rows yet | Limits 3 limits | Segments no segments yet | Stages 2 stages | Tells 1 tell');
  await page.screenshot({ path: `${out}/beside-5-heads.png` });
  // Something that holds ready's cards until it is pressed is a dot on its mark, and its row is under its table's heading
  // in the menu, before what else is about ready, saying that it holds them.
  await cli('record', 'add', 'shop', 'actions', 'title=Signed', 'column=ready', 'holds=true', 'who=person');
  await columnOf(page, 'ready').locator('.column-set .column-set-dot').waitFor();
  const held = await setOf(page, 'ready');
  expect(`ready's menu to say what holds its cards first, under its heading, got ${JSON.stringify(held)}`, JSON.stringify(held) === JSON.stringify(['[Actions]', 'Signed — Holds each card here until Signed is pressed · once for good', '[Limits]', 'three waiting — a limit · most 3', '[Stages]', 'work — Worked by the worker role, as many at a time as its agents can take → review on pass', '+Add an action for ready…', '+Add a limit for ready…', '+Add a stage for ready…']));
  await page.screenshot({ path: `${out}/beside-5b-held.png` });
  // Opened, its form is in one row of tabs at its top, as a card's is: Details first, with its fields and
  // what removes it, then Comments, History and Attachments; and its flags each a switch, Holds on and Again off.
  await setMenu(page).locator(':scope > button', { hasText: /^Signed/ }).click();
  await drawer.getByRole('heading', { name: 'Signed', exact: true }).waitFor();
  const tabsOf = drawer.locator('.record-tabs [role="tablist"] [role="tab"]');
  await drawer.getByRole('tab', { name: 'Attachments', exact: true }).waitFor();
  const rowTabs = (await tabsOf.allInnerTexts()).map((one) => one.trim());
  expect(`Details, Comments, History and Attachments in one row at the top of the row's form, got ${JSON.stringify(rowTabs)}`, JSON.stringify(rowTabs) === JSON.stringify(['Details', 'Comments', 'History', 'Attachments']));
  expect('one row of tabs, and none under the form', (await drawer.getByRole('tablist').count()) === 1);
  expect('Details showing, with its fields and what removes it', (await drawer.getByRole('tab', { name: 'Details', exact: true }).getAttribute('aria-selected')) === 'true' && (await drawer.getByRole('button', { name: 'Remove this action', exact: true }).isVisible()));
  const flagOf = (name) => drawer.getByRole('switch', { name, exact: true });
  for (const [name, on] of [['Holds', true], ['Again', false]]) {
    const flag = flagOf(name);
    const box = await flag.boundingBox();
    expect(`${name} to be a switch that is drawn, got ${JSON.stringify(box)}`, box !== null && box.width >= 20 && box.height >= 12);
    expect(`${name} to say ${on}`, (await flag.isChecked()) === on && (await flag.isEnabled()));
  }
  // Each switch a little past its name and all of them at one place down the form, never at its far edge, at three
  // widths and a phone's: by the boxes, the name's own words measured.
  const flagsLaid = async (how) => {
    const read = await drawer.evaluate((on) =>
      [...on.querySelectorAll('.record-tabs .form .flag-field')].map((one) => {
        const words = document.createRange();
        words.selectNodeContents(one.querySelector('.flag-name'));
        const [name, flip, form] = [words.getBoundingClientRect(), one.querySelector('.switch').getBoundingClientRect(), one.closest('.form').getBoundingClientRect()];
        return { name: one.textContent.trim(), gap: flip.left - name.right, left: flip.left, top: flip.top - name.top, into: flip.left - form.left, form: form.width };
      }),
    );
    expect(`the action's two flags laid out ${how}, got ${JSON.stringify(read)}`, read.length === 2);
    for (const one of read) expect(`${one.name}'s switch a little past its name ${how}, not at the form's far edge, got ${JSON.stringify(one)}`, one.gap >= 4 && one.gap <= 40 && one.into <= one.form / 2);
    expect(`every switch's left edge at one place ${how}, got ${JSON.stringify(read)}`, read.every((one) => Math.abs(one.left - read[0].left) < 0.5));
  };
  for (const width of [1280, 1000, 820, 390]) {
    await page.setViewportSize({ width, height: 860 });
    await drawerAtRest(page).waitFor();
    await flagsLaid(`at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // At a phone's width the side panel was put away; it is brought back for what follows.
  await sideSettled(page);
  if ((await page.locator('.shell > .sidebar').count()) === 0) {
    await page.getByRole('button', { name: 'Expand the side panel' }).click();
    await sideSettled(page);
  }
  await page.screenshot({ path: `${out}/beside-5d-flags.png` });
  await flagOf('Again').click();
  expect('Again pressed to be on', await flagOf('Again').isChecked());
  // The tabs are above the form, and over nothing of it: the row of tabs is the drawer's first thing after the way to the row.
  const [tabsAt, formAt] = [await drawer.locator('.record-tabs > .view-tabs').boundingBox(), await drawer.locator('.record-tabs .form').boundingBox()];
  expect(`the tabs over the form, got ${JSON.stringify({ tabsAt, formAt })}`, tabsAt.y + tabsAt.height <= formAt.y + 0.5);
  // The row of tabs is one row, inside the drawer: where it has no room for every tab (the largest size) it scrolls
  // sideways in itself, with the tab showing in sight, and nothing else does.
  await laid("in an action's form, its tabs at its top", '.drawer-body', '.record-tabs > .view-tabs, label, .form > .row > button');
  const tabRow = await drawer.locator('.record-tabs .view-tabs-list').evaluate((row) => {
    const [box, chosen] = [row.getBoundingClientRect(), row.querySelector('[aria-selected="true"]').getBoundingClientRect()];
    return { rows: new Set([...row.children].map((one) => Math.round(one.getBoundingClientRect().top))).size, scrolls: getComputedStyle(row).overflowX, seen: chosen.left >= box.left - 0.5 && chosen.right <= box.right + 0.5 };
  });
  expect(`the tabs on one row that scrolls sideways in itself, the one showing in sight, got ${JSON.stringify(tabRow)}`, tabRow.rows === 1 && tabRow.scrolls === 'auto' && tabRow.seen);
  await page.screenshot({ path: `${out}/beside-5c-row-tabs.png` });
  await drawer.getByRole('button', { name: 'Save', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  const again = JSON.parse((await cli('record', 'list', 'shop', 'actions', '--json')).stdout).records.find((row) => row.data.title === 'Signed');
  expect(`Again saved as on, got ${JSON.stringify(again?.data)}`, again?.data.again === true);
  // The rows beside the board are read again once the change is heard of: the menu says what there was as it opened.
  let heldAgain = await setOf(page, 'ready');
  for (let tries = 0; tries < 50 && !heldAgain[1].endsWith('each time it comes'); tries += 1) {
    await shut(page);
    await page.waitForTimeout(100);
    heldAgain = await setOf(page, 'ready');
  }
  expect(`ready's menu to say Signed holds each time a card comes, got ${JSON.stringify(heldAgain)}`, heldAgain[1] === 'Signed — Holds each card here until Signed is pressed · each time it comes');
  // A line that adds one asks what it is called, adds it about ready, and opens it to be filled in.
  await setMenu(page).locator(':scope > button', { hasText: 'Add a limit for ready…' }).click();
  const asked = page.getByRole('dialog');
  await asked.getByText('New limit for ready', { exact: true }).waitFor();
  await asked.getByLabel('What it is called').fill('one more');
  await asked.getByRole('button', { name: 'Add', exact: true }).click();
  await drawer.getByRole('heading', { name: 'one more', exact: true }).waitFor();
  expect(`the row added to be about ready, got ${await drawer.getByLabel('Column', { exact: true }).inputValue()}`, (await drawer.getByLabel('Column', { exact: true }).inputValue()) === 'ready');
  await drawer.getByRole('button', { name: 'Remove this limit', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  await page.waitForFunction(() => document.querySelectorAll('.board .column-set').length === 4);
  expect(`ready to say its rows once the one added is taken away, got ${JSON.stringify(await heads())}`, (await said('ready')).join('+') === 'Signed+three waiting+work');
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  // A row taken away is no longer said.
  const gone = JSON.parse((await cli('record', 'list', 'shop', 'limits', '--json')).stdout).records.find((row) => row.data.title === 'done').id;
  await cli('record', 'remove', gone);
  await columnOf(page, 'done').locator('.column-set').waitFor({ state: 'detached' });

  // From the drawer a table is opened as a tab of its own, for the whole width; and from there what sits beside
  // that table, where anything does, is reached the same way.
  await open('Stages');
  await aside.locator('tr[data-row="shop-stages-2"]').waitFor();
  await aside.getByRole('button', { name: 'Open as a tab', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  // The still copy of the drawer that was put away goes out the way it came, a moment longer on a busy machine, and holds a copy of the table.
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });
  await page.locator('.view-head h1', { hasText: 'Shop stages' }).waitFor();
  await page.locator('.panes table.grid tr[data-row="shop-stages-2"]').waitFor();
  expect('the tab to be called what the table is', (await page.locator('.tab.active, [role="tab"][aria-selected="true"]').first().innerText()).includes('Shop stages'));
  // A table that says no groups has no band over its heads, as it always was.
  expect('a table that says no groups to have no band of headings', (await page.locator('.panes table.grid tr.grid-bands').count()) === 0);
  expect(`the tables beside the board still not to be listed under the project, got ${(await lines('shop')).join(', ')}`, !(await lines('shop')).some((one) => ['Agents', 'Stages', 'Segments', 'Limits', 'Tells'].includes(one)));
  await page.screenshot({ path: `${out}/beside-6-tab.png` });

  // A table's own page says what sits beside it the same way: nothing in its gear while nothing does, and once a
  // table says it sits beside this one, a heading and a line with how many rows it has, which opens it in the
  // drawer. Its head has no button for it, and keeps the one that lays its view out.
  await openGear(page);
  expect(`the gear of a table with nothing beside it to say nothing of tables, got ${(await gearLines(page)).join(' | ')}`, !(await gearLines(page)).includes('Its tables') && (await tablesOf(page)).length === 0);
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  const steps = join(out, 'beside-steps.bp');
  await writeFile(steps, STEPS);
  await cli('apply', steps, '--project', 'shop');
  await cli('record', 'add', 'shop', 'steps', 'title=Lint');
  const theirSteps = async () => {
    // The table is heard of and its row read a moment after it is declared: the gear is asked until it says so.
    for (let tries = 0; tries < 300; tries += 1) {
      const under = await tablesRead(page);
      if (under.join(' | ') === 'Steps 1 step') return under;
      await page.keyboard.press('Escape');
      await menu.waitFor({ state: 'detached' });
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    }
    return tablesOf(page);
  };
  expect(`the gear of a table to list the table beside it with its rows, got ${(await theirSteps()).join(' | ')}`, (await tablesOf(page)).join(' | ') === 'Steps 1 step');
  const ofStages = await gearLines(page);
  expect(`a table's gear to say its tables under what it had and over when it was changed, and nothing of a board's tools, got ${ofStages.join(' | ')}`, ofStages.indexOf('Its tables') === ofStages.indexOf('Measurements of Shop') + 1 && ofStages.indexOf('Steps 1 step') === ofStages.length - 2 && /^Last edited/.test(ofStages.at(-1)) && !ofStages.some((one) => /^(Add a card|Layout)/.test(one)));
  const stagesHead = await headButtons(page);
  expect(`a table's head to have no button for its tables and still the one that lays its view out, got ${stagesHead.join(' | ')}`, !stagesHead.some((one) => /^Its tables/.test(one)) && stagesHead.some((one) => /^Layout/.test(one)));
  await page.screenshot({ path: `${out}/beside-6-table-gear.png` });
  await menu.getByRole('menuitem', { name: /^Steps/ }).click();
  await aside.locator('tr[data-row="shop-steps-1"]').waitFor();
  await drawer.getByRole('heading', { name: 'Shop steps', exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });
  // In the drawer there is no gear: a table shown there has the button it always had for what sits beside it.
  await line('shop', 'Board').click();
  await open('Stages');
  await aside.locator('tr[data-row="shop-stages-2"]').waitFor();
  expect('no gear in the drawer', (await drawer.locator('.head-acts').count()) === 0);
  await aside.getByRole('button', { name: /^Its tables/ }).click();
  const inDrawer = aside.getByRole('menu', { name: 'The tables beside this table' });
  await inDrawer.getByRole('menuitem', { name: /Steps\s*1 step/ }).waitFor();
  const [drawerAt, listAt] = [await drawer.boundingBox(), await inDrawer.boundingBox()];
  expect(`what Its tables opens in the drawer to be inside the drawer, at ${JSON.stringify(listAt)} in ${JSON.stringify(drawerAt)}`, listAt.x >= drawerAt.x - 0.5 && listAt.x + listAt.width <= drawerAt.x + drawerAt.width + 0.5);
  await page.screenshot({ path: `${out}/beside-6-drawer-button.png` });
  await inDrawer.getByRole('menuitem', { name: /^Steps/ }).click();
  await drawer.getByRole('heading', { name: 'Shop steps', exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });

  // ---- an agent's row, read as three kinds of limit ----------------------------------------------------------
  // The agents' table says its columns in groups: a band of headings over the heads, each as wide as the columns
  // under it, in the order the table says them and the columns of each in the order it says those.
  await line('shop', 'Board').click();
  await open('Agents');
  // The drawer goes out as a still copy of itself, which held this very table: it is held there, half way out,
  // while the table's own page is read, as a busy machine would leave it, and nothing of it is found.
  await watchLeaving(page);
  await aside.getByRole('button', { name: 'Open as a tab', exact: true }).click();
  await page.getByRole('heading', { name: 'Shop agents', level: 1 }).waitFor();
  await page.locator('.drawer-leaving').waitFor();
  const grid = page.locator('table.grid');
  await grid.locator('tr[data-row="shop-agents-3"]').waitFor();
  const bands = await grid.evaluate((table) => {
    const heads = [...table.querySelectorAll('thead tr:not(.grid-bands) th[data-column]')];
    let at = 0;
    return [...table.querySelectorAll('tr.grid-bands th')].map((band) => {
      const under = heads.slice(at, at + band.colSpan);
      at += band.colSpan;
      const [box, first, last] = [band.getBoundingClientRect(), under[0].getBoundingClientRect(), under.at(-1).getBoundingClientRect()];
      return { name: band.textContent.trim(), columns: under.map((head) => head.querySelector('.grid-head .truncate').textContent.trim()).join(' '), fits: Math.abs(box.left - first.left) <= 1 && Math.abs(box.right - last.right) <= 1 && box.bottom <= first.top + 1, whole: band.scrollWidth <= band.clientWidth + 1 };
    });
  });
  const GROUPS = [
    ['', 'Title'],
    ['Who they are', 'Role Count Rank'],
    ['What they are told', 'Prompt Told Listens Plans'],
    ['What they may ask', 'Holds'],
    ['How their program works', 'Permission Adapter Model Outlives Web'],
    ['What their program can reach', 'Wall Reads Changes Git Secrets'],
  ];
  expect(`the agents' columns to be under their five headings, in order, got ${JSON.stringify(bands.map((one) => [one.name, one.columns]))}`, JSON.stringify(bands.map((one) => [one.name, one.columns])) === JSON.stringify(GROUPS));
  expect(`each heading to be over its own columns, as wide as they are, and read whole, got ${JSON.stringify(bands)}`, bands.every((one) => one.fits && one.whole));
  // What is still going out is nothing to find: the copy's table is the page's no more than it is pressed. Then it goes.
  await grid.locator('tr[data-row="shop-agents-2"]').hover();
  await page.evaluate(() => document.querySelector('.drawer-leaving')?.getAnimations()[0]?.play());
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/beside-12-groups.png` });

  // A row's form has a section under each heading, in the same order, with what is in no group before them.
  await grid.locator('tr[data-row="shop-agents-2"]').hover();
  await grid.getByRole('button', { name: 'Open worker', exact: true }).click();
  await drawer.getByRole('heading', { name: 'worker', exact: true }).waitFor();
  const sections = await drawer.locator('.form').evaluate((form) => {
    const said = [['']];
    for (const one of form.children) {
      if (one.matches('h3.form-band')) said.push([one.textContent.trim()]);
      else if (one.matches('label, .field-many')) said.at(-1).push((one.getAttribute('aria-label') ?? one.firstChild.textContent).trim().replace(/, separated by commas$/, ''));
    }
    return said.map(([name, ...fields]) => [name, fields.join(' ')]);
  });
  expect(`the form to have a section for each group, with its fields, got ${JSON.stringify(sections)}`, JSON.stringify(sections) === JSON.stringify(GROUPS));
  // What it may ask is there by area, as in the table's cell, and says that the row says nothing: the usual.
  const inForm = drawer.locator('.field-many[aria-label="Holds"]');
  await inForm.locator('.holds-table').getByLabel('What it may do with board').waitFor();
  expect(`the form to say that the row leaves what it holds unsaid, got "${await inForm.locator('.holds-where').innerText()}"`, /The row says nothing here, which comes to this: the usual\./.test(await inForm.locator('.holds-where').innerText()));
  await laid('in a form in sections', '.drawer-body', 'h3.form-band, .form > label, .form > .field-many, .form > .row > button');
  await drawer.getByRole('heading', { name: 'What they may ask' }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/beside-13-form.png` });
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });

  // What a row's agents may ask of the factory is said short in its cell: the usual, where the row says nothing.
  const holdsOf = (name) => grid.getByRole('button', { name: `Holds of ${name}`, exact: true });
  const cellSays = async (name) => (await holdsOf(name).innerText()).trim();
  expect(`each row that says nothing of what it holds to say the usual, got ${await cellSays('worker')} and ${await cellSays('architect')}`, (await cellSays('worker')) === 'the usual' && (await cellSays('architect')) === 'the usual');
  // Pressed, it is one panel over the cell: a line an area, with what is held there and what is not.
  await holdsOf('architect').click();
  const byArea = page.getByRole('dialog', { name: 'Holds of architect' });
  await byArea.getByLabel('What it may do with board').waitFor();
  const linesOf = (within) =>
    within.locator('.holds-table tr').evaluateAll((rows) =>
      Object.fromEntries(
        rows.map((row) => {
          const choice = row.querySelector('select');
          return [row.querySelector('th').textContent.trim(), choice ? choice.options[choice.selectedIndex].textContent.trim() : row.querySelector('td').textContent.trim()];
        }),
      ),
    );
  const usual = await linesOf(byArea);
  expect(`what the row leaves unsaid to be shown as what it comes to, by area, got ${JSON.stringify(usual)}`, usual.board === 'read and change' && usual.blueprints === 'read' && usual.fleet === 'read' && usual.search === 'yes' && usual.shell === 'nothing' && usual.admin === 'nothing');
  expect(`it to say that the row says nothing, got "${await byArea.locator('.holds-where').innerText()}"`, /The row says nothing here, which comes to this: the usual\. Change a line and the row says the whole of it\./.test(await byArea.locator('.holds-where').innerText()));
  const over = await byArea.evaluate((panel) => {
    const [cell, box] = [panel.closest('td').getBoundingClientRect(), panel.getBoundingClientRect()];
    const edge = ['Top', 'Right', 'Bottom', 'Left'].reduce((sum, side) => sum + parseFloat(getComputedStyle(panel)[`border${side}Width`]), 0);
    const frame = panel.closest('.grid-scroll').getBoundingClientRect();
    return { over: box.left <= cell.left + 1 && box.top <= cell.top + 1 && box.right > cell.left && box.bottom > cell.top, wider: Math.round(box.width - cell.width), edge, inside: box.left >= Math.max(0, frame.left) - 0.5 && box.top >= Math.max(0, frame.top) - 0.5 && box.right <= Math.min(window.innerWidth, frame.right) + 0.5 && box.bottom <= Math.min(window.innerHeight, frame.bottom) + 0.5 };
  });
  expect(`the panel to lie over the cell, as wide as it at least, with no border, all of it in what the table scrolls in and on the screen, got ${JSON.stringify(over)}`, over.over && over.wider >= 0 && over.edge === 0 && over.inside);
  expect('nothing to be closed to the owner, who holds everything', (await byArea.locator('option:disabled').count()) === 0);
  // Its lines are on the panel's own ground, each as tall as what it says: they are not cells of the table it lies over.
  await grid.locator('tr[data-row="shop-agents-1"]').hover();
  const ground = await byArea.locator('.holds-table').evaluate((table) => {
    const cells = [...table.querySelectorAll('th, td')];
    return { grounds: [...new Set(cells.map((cell) => getComputedStyle(cell).backgroundColor))], stays: [...new Set(cells.map((cell) => getComputedStyle(cell).position))], tallest: Math.round(Math.max(...[...table.rows].map((row) => row.getBoundingClientRect().height))) };
  });
  expect(`the panel's lines to have no ground of their own, pointed at or not, and to stay nowhere, got ${JSON.stringify(ground)}`, ground.grounds.join() === 'rgba(0, 0, 0, 0)' && ground.stays.join() === 'static' && ground.tallest <= 33);
  await page.screenshot({ path: `${out}/beside-14-holds.png` });

  // A line is changed where it is: the row says the whole list, the fleet follows, and the panel stays.
  const worker = async () => JSON.parse((await cli('record', 'list', 'shop', 'agents', '--json')).stdout).records.find((one) => one.data.title === 'architect');
  const scopesOf = async (name) => {
    const listed = JSON.parse((await cli('agent', 'list', '--json')).stdout);
    return (listed.agents ?? listed).find((one) => one.name === name)?.scopes ?? [];
  };
  const comes = async (what, ask) => {
    let last;
    for (let tries = 0; tries < 150; tries += 1) {
      last = await ask();
      if (last === true) return;
      await page.waitForTimeout(100);
    }
    throw new Error(`expected ${what}, and it is ${JSON.stringify(last)}`);
  };
  await byArea.getByLabel('What it may do with board').selectOption({ label: 'read' });
  await comes('the row to say the whole of what the architect holds, the board to read', async () => {
    const holds = (await worker()).data.holds ?? [];
    return (holds.includes('board:read') && !holds.includes('board') && holds.includes('projects') && holds.includes('blueprints:read')) || holds;
  });
  await comes('the agent to hold what its row says', async () => {
    const held = await scopesOf('shop-architect-1');
    return (held.includes('board:read') && !held.includes('board')) || held;
  });
  await comes('the panel to say what the row says, still open', async () => ((await linesOf(byArea)).board === 'read' && /As the row says\./.test(await byArea.locator('.holds-where').innerText())) || (await linesOf(byArea)));

  // The Tools panel shows the same of the same agent on another open page, which is not read again; and what is
  // changed there is what the cell here says.
  const tooled = await context.newPage();
  tooled.on('pageerror', (err) => problems.push(err.message));
  await tooled.goto(url);
  await tooled.locator('.rail').waitFor();
  await side(tooled, 'Channels');
  await tooled.locator('.nav').getByText('shop', { exact: true }).click();
  await tooled.getByRole('heading', { name: '# shop' }).waitFor();
  const tools = tooled.getByRole('complementary', { name: 'Tools' });
  if ((await tools.count()) === 0) await tooled.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(tooled).waitFor();
  await tools.waitFor();
  await tools.getByRole('tab', { name: /^Channel\b/ }).click();
  await tools.locator('.context-agent', { hasText: 'shop-architect-1' }).click();
  await tools.locator('.holds-table').getByLabel('What it may do with board').waitFor();
  await comes('the Tools panel to say what the row was given in the table', async () => (await linesOf(tools)).board === 'read' || (await linesOf(tools)));
  await tooled.screenshot({ path: `${out}/beside-15-tools.png` });
  await tools.locator('.holds-table').getByLabel('What it may do with blueprints').selectOption({ label: 'read and run' });
  await comes('the cell\'s panel on the first page to say what was changed in the Tools panel, with nothing read again', async () => (await linesOf(byArea)).blueprints === 'read and run' || (await linesOf(byArea)));
  await byArea.getByRole('button', { name: 'Done', exact: true }).click();
  await byArea.waitFor({ state: 'detached' });
  await comes('the cell to say the areas the row holds something of, and how many', async () => /^8 areas: /.test(await cellSays('architect')) || (await cellSays('architect')));
  expect(`the cell to name the areas, got ${await cellSays('architect')}`, /board/.test(await cellSays('architect')) && /blueprints/.test(await cellSays('architect')));
  await page.screenshot({ path: `${out}/beside-16-said.png` });
  // The row says nothing again, by one press: it is given the usual, and both pages say so.
  await holdsOf('architect').click();
  await byArea.getByRole('button', { name: 'Say nothing again: the usual', exact: true }).click();
  await comes('the row to say nothing of what it holds again', async () => ((await worker()).data.holds ?? []).length === 0 || (await worker()).data.holds);
  await comes('the Tools panel to say the usual again', async () => ((await linesOf(tools)).board === 'read and change' && (await linesOf(tools)).blueprints === 'read') || (await linesOf(tools)));
  await page.keyboard.press('Escape');
  await byArea.waitFor({ state: 'detached' });
  await comes('the cell to say the usual again', async () => (await cellSays('architect')) === 'the usual' || (await cellSays('architect')));
  await tooled.close();

  // Somebody who holds less is not offered what they do not hold, on the line; and a row that gives what they do
  // not hold says that it is not theirs to change, where a save would only have been refused.
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }) });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  const theirs = await browser.newContext({ viewport: { width: 1500, height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const other = await theirs.newPage();
  other.on('pageerror', (err) => problems.push(err.message));
  await other.goto(url);
  await other.getByLabel('Email').fill('grace@example.com');
  await other.getByLabel('Password').fill('correct horse');
  await other.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await other.locator('.rail').waitFor();
  await side(other, 'Resources');
  const unfolds = other.locator('.nav').getByRole('button', { name: 'Unfold Shop', exact: true });
  await other.locator('.nav').getByText('Shop', { exact: true }).waitFor();
  if (await unfolds.count()) await unfolds.click();
  await other.locator('.nav [data-project="shop"]').getByRole('button', { name: /Board$/ }).first().click();
  await gearLine(other, /^Agents/);
  const beside = drawerAtRest(other).locator('.sits-table');
  await beside.locator('tr[data-row="shop-agents-2"]').waitFor();
  await beside.getByRole('button', { name: 'Holds of worker', exact: true }).click();
  const hers = other.getByRole('dialog', { name: 'Holds of worker' });
  await hers.getByLabel('What it may do with board').waitFor();
  const closed = await hers.locator('.holds-table tr').evaluateAll((rows) => Object.fromEntries(rows.map((row) => [row.querySelector('th').textContent.trim(), [...row.querySelectorAll('option:disabled')].map((one) => one.textContent.trim())])));
  expect(`a member not to be offered what they do not hold, and to be offered what they do, got ${JSON.stringify(closed)}`, closed.admin.join() === 'yes' && closed.board.length === 0 && closed.shell.length === 0);
  expect(`what is not offered to say why, where it is pointed at, got ${await hers.getByLabel('What it may do with admin').locator('option:disabled').getAttribute('title')}`, /You do not hold this yourself/.test((await hers.getByLabel('What it may do with admin').locator('option:disabled').getAttribute('title')) ?? ''));
  await other.screenshot({ path: `${out}/beside-17-member.png` });
  // The owner has the row give more than she holds: it is hers to read and not to change, and says so.
  await page.evaluate(async (id) => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const done = await fetch(`/api/records/${id}`, { method: 'PATCH', headers: mine, body: JSON.stringify({ fields: { holds: ['board', 'records', 'admin'] } }) });
    if (!done.ok) throw new Error(await done.text());
  }, 'shop-agents-2');
  await hers.getByText('It gives admin, which you do not hold, so it is not yours to change: nobody gives what they do not hold.').waitFor();
  expect('a row that gives what somebody does not hold to offer them no line to change', (await hers.locator('select').count()) === 0);
  await other.screenshot({ path: `${out}/beside-18-not-hers.png` });
  // Escape puts the panel away and leaves the table in the drawer: what is open in it comes first.
  await other.keyboard.press('Escape');
  await hers.waitFor({ state: 'detached' });
  expect('the table to be showing still in the drawer once the panel over its cell is put away', await beside.isVisible());
  await theirs.close();
  await page.evaluate(async (id) => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    await fetch(`/api/records/${id}`, { method: 'PATCH', headers: mine, body: JSON.stringify({ fields: { holds: [] } }) });
  }, 'shop-agents-2');
  await comes('the cell to say the usual once the row says nothing', async () => (await cellSays('worker')) === 'the usual' || (await cellSays('worker')));

  // A board with no fleet, and tables that say nothing of what they sit beside, are as they always were: no
  // heading in the board's gear and no line under one, and nothing in any column's head.
  for (const project of ['plain', 'old']) {
    await line(project, 'Board').click();
    await page.locator('.view-head h1', { hasText: new RegExp(`^${project} board$`, 'i') }).waitFor();
    await page.locator('.board .column', { hasText: 'backlog' }).first().waitFor();
    await openGear(page);
    const theirs = await gearLines(page);
    expect(`the gear of the board of ${project} to say nothing of tables beside it and still what can be done to a board, got ${theirs.join(' | ')}`, !theirs.includes('Its tables') && (await tablesOf(page)).length === 0 && theirs.includes('Add a card to backlog') && theirs.includes('Layout Board'));
    await page.keyboard.press('Escape');
    await menu.waitFor({ state: 'detached' });
    expect(`the board of ${project} to have no button for tables beside it`, !(await headButtons(page)).some((one) => /^Its tables/.test(one)));
    expect(`no head of the board of ${project} to have a mark`, (await page.locator('.column-set').count()) === 0);
  }
  // And a card of such a board is still sent back where the row of the table called `stages` says, as it was.
  await page.locator('.board .card', { hasText: 'A card' }).click();
  await drawer.getByText('It goes back to ready, where it is taken up again.').waitFor();
  await page.screenshot({ path: `${out}/beside-7-before.png` });
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });

  // At a phone's width the drawer has the screen, as it does for a card: the board's head and each column's lay
  // nothing over anything else, a table opened from there fills the screen, and nothing scrolls sideways.
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: process.env.SCHEME ?? 'dark' });
  const small = await phone.newPage();
  small.setDefaultTimeout(20_000);
  small.on('pageerror', (err) => problems.push(err.message));
  await small.goto(url);
  await small.getByLabel('Email').fill('ada@example.com');
  await small.getByLabel('Password').fill('correct horse');
  await small.getByRole('button', { name: 'Sign in', exact: true }).last().tap();
  await small.locator('.panes').waitFor();
  await small.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(small);
  const rail = small.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
  if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.tap();
  await unfoldAll(small);
  const listing = small.locator('.nav');
  await listing.getByText('Shop', { exact: true }).waitFor();
  const unfold = listing.getByRole('button', { name: 'Unfold Shop', exact: true });
  if (await unfold.count()) await unfold.tap();
  await listing.locator('[data-project="shop"]').getByRole('button', { name: /Board$/ }).first().tap();
  await small.locator('.sidebar').waitFor({ state: 'detached' });
  await small.locator('.board .column-set').first().waitFor();
  // The same question of a phone's page: what lies over what, and what scrolls sideways.
  const narrow = async (how, within, things) => {
    const found = await small.evaluate(
      ([root, chosen]) => {
        const frame = document.querySelector(root);
        const seen = [...frame.querySelectorAll(chosen)].filter((el) => el.getClientRects().length > 0);
        const all = seen.map((el) => ({ what: (el.getAttribute('aria-label') || el.innerText || el.className).trim().slice(0, 40), at: el.getBoundingClientRect() }));
        const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
        return {
          count: all.length,
          over: all.flatMap((a, at) => all.slice(at + 1).filter((b) => over(a.at, b.at)).map((b) => `${a.what} over ${b.what}`)),
          sideways: frame.scrollWidth - frame.clientWidth,
          page: document.documentElement.scrollWidth - window.innerWidth,
        };
      },
      [within, things],
    );
    expect(`something to be laid out ${how}`, found.count > 0);
    expect(`nothing to lie over anything else ${how}, got ${found.over.join('; ')}`, found.over.length === 0);
    expect(`nothing to scroll sideways ${how}, by ${found.sideways} and the page by ${found.page}`, found.sideways <= 0 && found.page <= 0);
  };
  await narrow('in the board\'s head on a phone', '.view-head', 'h1, button, .muted');
  await narrow('in a column\'s head on a phone', '.board .column:nth-child(2) .column-head', HEAD);
  await small.screenshot({ path: `${out}/beside-8-phone.png` });
  expect(`a phone's head to have no button for the board's tables, got ${(await headButtons(small)).join(' | ')}`, !(await headButtons(small)).some((one) => /^(Its tables|Tools)/.test(one)));
  const listedSmall = await openGear(small, 'tap');
  await listedSmall.getByRole('menuitem', { name: /^Agents/ }).waitFor();
  const reach = await listedSmall.boundingBox();
  expect(`the gear's menu, with the board's tables in it, to be on a phone's screen, at ${JSON.stringify(reach)}`, reach && reach.x >= -0.5 && reach.x + reach.width <= 390.5 && reach.y >= -0.5 && reach.y + reach.height <= 844.5);
  await small.screenshot({ path: `${out}/beside-9-phone-listed.png` });
  await listedSmall.getByRole('menuitem', { name: /^Agents/ }).tap();
  const whole = drawerAtRest(small);
  await whole.locator('.sits-table table.grid tr[data-row="shop-agents-3"]').waitFor();
  const screen = await whole.boundingBox();
  expect(`a table opened from the board to have a phone's screen, as a card has, at ${JSON.stringify(screen)}`, screen && screen.x <= 0.5 && screen.y <= 0.5 && screen.width >= 389.5 && screen.height >= 843.5);
  await narrow('in the head of a table that has a phone\'s screen', '.drawer .view-head', 'button, .muted');
  await narrow('in the drawer on a phone', '.drawer-body', '.view-head, .grid-scroll');
  await small.screenshot({ path: `${out}/beside-10-phone-table.png` });
  // Its columns are under their headings there too, and what a row may ask is opened by area over its cell.
  expect(`the headings to be over the columns on a phone, got ${(await whole.locator('tr.grid-bands th').allInnerTexts()).join(' | ')}`, (await whole.locator('tr.grid-bands th').allInnerTexts()).map((one) => one.trim()).join('|') === '|Who they are|What they are told|What they may ask|How their program works|What their program can reach');
  await whole.getByRole('button', { name: 'Holds of worker', exact: true }).tap();
  const touched = small.getByRole('dialog', { name: 'Holds of worker' });
  await touched.getByLabel('What it may do with board').waitFor();
  await narrow('with what a row may ask opened on a phone', '.drawer-body', '.view-head, .grid-scroll');
  await small.screenshot({ path: `${out}/beside-10-phone-holds.png` });
  await touched.getByRole('button', { name: 'Done', exact: true }).tap();
  await touched.waitFor({ state: 'detached' });
  await whole.getByRole('button', { name: 'Close', exact: true }).tap();
  await whole.waitFor({ state: 'detached' });
  // And a stage from the head of its column, the same.
  expect(`ready's menu on a phone to be what it is at a desk, got ${JSON.stringify(await setOf(small, 'ready', 'tap'))}`, (await small.evaluate(() => document.querySelector('.row-menu').textContent)).includes('work'));
  await setMenu(small).locator(':scope > button', { hasText: /^work/ }).tap();
  await whole.getByRole('heading', { name: 'work', exact: true }).waitFor();
  await small.screenshot({ path: `${out}/beside-11-phone-stage.png` });
  await phone.close();

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  // The fleet's agents are slots and no processes of their own are left
  // running by the built-in one; the daemon is all there is to stop.
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
