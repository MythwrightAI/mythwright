// A field that names a record of another collection: a card's sprint,
// and the card it comes after.
//
//   DAEMON=../target/debug/factory node scripts/records.mjs [out-dir]
//
// A blueprint gives a board two such fields. In the board's table each is
// a column: a cell says what the record it names is known by, is changed
// by choosing among the records there are, and goes on saying what it
// named, as gone, when that record is taken away.
//
// The sprints are kept as files, so a sprint is made here by writing a
// file and taken away by deleting it, as a person would. The releases are
// kept in the installation's database and are made, renamed and taken
// away from the terminal while the page is open, which is told of each
// and is not read again. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

/** Turns the side panel to a view on the rail, unless it is showing already: Resources with what it has read unfolded, as everything there starts folded (`folded.mjs`). */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  if (name === 'Resources') await unfoldAll(page);
}

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'records-data');
const home = join(out, 'records-home');

const PLAN = `blueprint: plan
description: Sprints, releases, and a board whose cards may each be in one of each and come after another.
permissions: [projects, board]
steps:
  sprints:
    do: collection.ensure
    with:
      name: sprints
      store: files
      fields:
        title: { type: text, required: true }
  releases:
    do: collection.ensure
    with:
      name: releases
      fields:
        title: { type: text, required: true }
  board:
    do: collection.ensure
    with:
      name: board
      fields:
        title: { type: text, required: true }
        body: { type: text }
        status: { type: choice, of: [todo, doing, done], required: true }
        assignee: { type: actor }
        labels: { type: list }
        sprint: { type: record, of: sprints }
        release: { type: record, of: releases }
        after: { type: record, of: board }
        points: { type: number }
        blocked: { type: flag }
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

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  const shop = join(data, 'projects/shop');
  const plan = join(shop, 'blueprints/plan.bp');
  await mkdir(join(shop, 'blueprints'), { recursive: true });
  await writeFile(plan, PLAN);
  await cli('apply', plan, '--project', 'shop');
  await mkdir(join(shop, 'sprints'), { recursive: true });
  await writeFile(join(shop, 'sprints/1.md'), '# November\n');
  await writeFile(join(shop, 'sprints/2.md'), '# December\n');
  // Files written by hand are found when the project's files are next looked at.
  await cli('project', 'scan', 'shop');
  await cli('board', 'add', 'shop', 'Basket');
  await cli('board', 'add', 'shop', 'Checkout');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();

  await side(page, 'Resources');
  await page.locator('.nav').getByText('Shop', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await page.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }).click();
  await page.locator('.card', { hasText: 'Checkout' }).waitFor();

  // The board's table has a column for each: what a card names comes with who has it.
  // The blueprint above declares no `due`; a board's cards have one all the same, because
  // a card is a model and what the blueprint leaves out of it is given.
  const views = page.getByRole('tablist', { name: 'Views' });
  const addView = async (name) => {
    await page.getByRole('button', { name: 'Add a view' }).click();
    await page.getByRole('menuitem', { name: new RegExp(`^${name}`) }).click();
    await views.getByRole('tab', { name, exact: true, selected: true }).waitFor();
  };
  await addView('Table');
  const heads = (await page.locator('.grid thead th[data-column]').allInnerTexts()).map((one) => one.trim());
  expect(
    `the columns to be Title,Status,Assignee,After,Release,Sprint,Labels,Due,Points,Blocked,Body, got ${heads.join()}`,
    heads.join() === 'Title,Status,Assignee,After,Release,Sprint,Labels,Due,Points,Blocked,Body',
  );
  const line = page.locator('.grid tbody tr', { hasText: 'Checkout' });
  const cell = (name) => line.getByRole('button', { name: new RegExp(`^${name} of`) });
  const choice = (name) => line.getByRole('combobox', { name, exact: true });
  const offered = async (name) => (await choice(name).locator('option').allInnerTexts()).map((one) => one.trim());

  // A sprint is chosen from the sprints there are, each by its title.
  await cell('Sprint').click();
  await choice('Sprint').waitFor();
  expect(`the sprints to be offered by their titles, got ${await offered('Sprint')}`, (await offered('Sprint')).join() === 'nothing,November,December');
  await page.screenshot({ path: `${out}/records-0-choosing.png` });
  await choice('Sprint').selectOption({ label: 'November' });
  await line.locator('.cell-names', { hasText: 'November' }).waitFor();

  // Another card is chosen from the other cards: a card does not come after itself.
  await cell('After').click();
  await choice('After').waitFor();
  expect(`the other card to be offered and not itself, got ${await offered('After')}`, (await offered('After')).join() === 'nothing,Basket');
  await choice('After').selectOption({ label: 'Basket' });
  await line.locator('.cell-names', { hasText: 'Basket' }).waitFor();
  await page.screenshot({ path: `${out}/records-1-named.png` });

  // It was kept, and not only drawn: the page read again says the same.
  await page.reload();
  await line.locator('.cell-names', { hasText: 'November' }).waitFor();
  await line.locator('.cell-names', { hasText: 'Basket' }).waitFor();
  // The same cards as a list say it too.
  await addView('List');
  const listed = page.locator('.rows-line', { hasText: 'Checkout' });
  await listed.locator('.cell-names', { hasText: 'November' }).waitFor();
  await views.getByRole('tab', { name: 'Table', exact: true }).click();
  await line.waitFor();

  // The sprint's file is deleted. The card goes on saying what it named,
  // as gone; the page is told when the project's files are next looked at.
  await rm(join(shop, 'sprints/1.md'));
  await cli('project', 'scan', 'shop');
  const gone = line.locator('.cell-names.gone');
  await gone.waitFor();
  expect(`the cell to say what it named and that it is gone, got "${await gone.innerText()}"`, (await gone.innerText()).replace(/\s+/g, ' ').trim() === 'shop-sprints-1 no longer there');
  await line.locator('.cell-names', { hasText: 'Basket' }).waitFor();
  await page.screenshot({ path: `${out}/records-2-gone.png` });
  // Left alone it stays as it is; it can be given another, or nothing.
  await cell('Sprint').click();
  await choice('Sprint').waitFor();
  expect(`what is gone to stay among the choices, got ${await offered('Sprint')}`, (await offered('Sprint')).join() === 'nothing,shop-sprints-1, which is no longer there,December');
  await choice('Sprint').selectOption({ label: 'December' });
  await line.locator('.cell-names', { hasText: 'December' }).waitFor();
  expect('nothing to be said to be gone any more', (await line.locator('.cell-names.gone').count()) === 0);

  // A release is made from the terminal while the page is open. The page
  // is told, and offers it without being read again.
  await cli('record', 'add', 'shop', 'releases', 'title=Spring');
  await cell('Release').click();
  await choice('Release').locator('option', { hasText: 'Spring' }).waitFor({ state: 'attached' });
  expect(`the release made a moment ago to be offered, got ${await offered('Release')}`, (await offered('Release')).join() === 'nothing,Spring');
  await choice('Release').selectOption({ label: 'Spring' });
  await line.locator('.cell-names', { hasText: 'Spring' }).waitFor();
  // Renamed, the card says its new name; taken away, that it is gone.
  await cli('record', 'set', 'shop-releases-1', 'title=Summer');
  await line.locator('.cell-names', { hasText: 'Summer' }).waitFor();
  await page.screenshot({ path: `${out}/records-3-told.png` });
  await cli('record', 'remove', 'shop-releases-1');
  await line.locator('.cell-names.gone', { hasText: 'shop-releases-1' }).waitFor();

  // ---- what else a board keeps of a card, in the card's own form ------------
  // Opened, the card's form has a place for each field its board's
  // blueprint declared beyond a card's own, each changed by what its kind
  // is changed with in the table: a choice of records, a number, a tick.
  await line.hover();
  await line.getByRole('button', { name: 'Open Checkout', exact: true }).click();
  const drawer = drawerAtRest(page);
  // By the start of its name: what a choice offers is part of what its label says.
  const field = (name) => drawer.getByLabel(new RegExp(`^${name}`));
  await field('Sprint').waitFor();
  const shown = [];
  for (const name of ['After', 'Release', 'Sprint', 'Points', 'Blocked']) shown.push((await field(name).count()) === 1);
  expect(`a place in the form for each of After, Release, Sprint, Points and Blocked, got ${shown.join()}`, shown.every(Boolean));
  expect(`the sprint the card is in to be the one chosen, got "${await field('Sprint').inputValue()}"`, (await field('Sprint').inputValue()) === 'shop-sprints-2');
  expect(`the card it comes after to be the one chosen, got "${await field('After').inputValue()}"`, (await field('After').inputValue()) === 'shop-1');
  await field('Points').fill('5');
  await field('Blocked').check();
  await field('Sprint').selectOption({ label: 'nothing' });
  // The switch moves over a moment; the picture is of where it ends.
  await page.screenshot({ path: `${out}/records-4-in-the-form.png`, animations: 'disabled' });
  await drawer.getByRole('button', { name: 'Save', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  // Saved, the table says the same, and what was not touched is as it was.
  await cell('Points').filter({ hasText: '5' }).waitFor();
  expect('the tick to be kept', await line.getByRole('checkbox', { name: 'Blocked', exact: true }).isChecked());
  // A tick stands where a cell's words begin, and is no wider than itself:
  // it was as wide as its column, and drawn in the middle of it.
  const inCell = (one) => one.evaluate((el) => Math.round(el.getBoundingClientRect().left - el.closest('td').getBoundingClientRect().left));
  const tick = line.getByRole('checkbox', { name: 'Blocked', exact: true });
  const [ticked, worded] = [await inCell(tick), await inCell(cell('Points').locator('.cell-words'))];
  expect(`the tick to begin where a cell's words do, got ${ticked} and ${worded} from the cell's edge`, Math.abs(ticked - worded) <= 1);
  const ticked16 = await tick.boundingBox();
  // Sixteen pixels as the usual height of things goes, and as tall as that is where a theme says otherwise.
  const sixteen = await page.evaluate(() => {
    const ruler = document.body.appendChild(document.createElement('div'));
    ruler.style.height = 'calc(16px * var(--tall))';
    const measured = ruler.getBoundingClientRect().height;
    ruler.remove();
    return measured;
  });
  expect(`the tick to be as wide as itself and a little over the browser's own, ${sixteen} each way, got ${ticked16.width} by ${ticked16.height}`, ticked16.width === sixteen && ticked16.height === sixteen);
  await tick.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/records-5-ticked.png`, animations: 'disabled' });
  expect('the sprint to be nothing', (await cell('Sprint').innerText()).trim() === '');
  expect(`the card it comes after to be as it was, got "${await cell('After').innerText()}"`, (await cell('After').innerText()).trim() === 'Basket');

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
