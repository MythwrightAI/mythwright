// Notes by the period they are of, and a year opened into what is in it:
// its quarters, their months, their weeks, their days. Each is a table of
// its own (a library, a folder of files), and what puts one inside another
// is only that it names it.
//
//   DAEMON=../target/debug/factory node scripts/nested.mjs [out-dir]
//
// The project is made by `examples/blueprints/period-notes.bp`, which keeps
// the days private; a key that holds what an agent holds is then shown
// that there are no days. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
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
const data = join(out, 'nested-data');
const home = join(out, 'nested-home');

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
  await cli('project', 'create', '--bare', 'Life');
  await cli('apply', join(repo, 'examples/blueprints/period-notes.bp'), '--project', 'life');
  const add = (table, ...fields) => cli('record', 'add', 'life', table, ...fields);
  await add('years', 'title=2025', 'path=2025');
  await add('years', 'title=2026', 'path=2026');
  await add('quarters', 'title=2026 Q3', 'path=2026-q3', 'year=life-years:2026');
  await add('quarters', 'title=2026 Q4', 'path=2026-q4', 'year=life-years:2026');
  await add('months', 'title=October 2026', 'path=2026-10', 'quarter=life-quarters:2026-q4');
  await add('months', 'title=November 2026', 'path=2026-11', 'quarter=life-quarters:2026-q4');
  await add('weeks', 'title=Week 46', 'path=2026-w46', 'month=life-months:2026-11');
  await add('days', 'title=Friday', 'path=2026-11-13', 'day=2026-11-13', 'week=life-weeks:2026-w46');
  await add('days', 'title=Saturday', 'path=2026-11-14', 'day=2026-11-14', 'week=life-weeks:2026-w46');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
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
  const nav = page.locator('.nav');
  const project = nav.locator(':scope > .tree > [data-project]', { has: page.locator(':scope > .nav-row', { hasText: 'Life' }) });

  // The years, as a table: a row a year, and each can be opened.
  await project.getByRole('button', { name: / Years$/ }).click();
  await page.getByRole('heading', { name: 'Life years', level: 1 }).waitFor();
  const grid = page.locator('table.grid');
  await grid.locator('tbody tr[data-row]').nth(1).waitFor();
  const turn = (called) => page.getByRole('button', { name: `Open what is in ${called}`, exact: true });
  await turn('2026').waitFor();
  await page.screenshot({ path: `${out}/nested-0-years.png` });

  // Opened, a year is its quarters; one of them its months; and so on down to the days.
  await turn('2026').click();
  const within = grid.locator('tr.grid-within');
  const names = async () => (await within.locator('.within-name').allTextContents()).map((name) => name.trim());
  await within.locator('.within-name', { hasText: '2026 Q4' }).waitFor();
  expect(`the year's quarters under it, got ${await names()}`, (await names()).join() === '2026 Q3,2026 Q4');
  await turn('2026 Q4').click();
  await within.locator('.within-name', { hasText: 'November 2026' }).waitFor();
  await turn('November 2026').click();
  await within.locator('.within-name', { hasText: 'Week 46' }).waitFor();
  await turn('Week 46').click();
  await within.locator('.within-name', { hasText: 'Saturday' }).waitFor();
  expect(`every period down to the days, each under what it names, got ${await names()}`, (await names()).join() === '2026 Q3,2026 Q4,October 2026,November 2026,Week 46,Friday,Saturday');
  // A day says the day it is of, and nothing is opened into it: nothing names a day.
  const saturday = within.locator('.within-row', { hasText: 'Saturday' }).last();
  expect('a day to say which day it is', (await saturday.innerText()).includes('2026-11-14'));
  expect('nothing to be opened into a day', (await page.getByRole('button', { name: 'Open what is in Saturday', exact: true }).count()) === 0);
  await page.screenshot({ path: `${out}/nested-1-opened.png` });

  // A quarter with nothing in it says so, and closing a year puts everything under it away.
  await turn('2026 Q3').click();
  await within.getByText('no months', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Close what is in 2026', exact: true }).click();
  await within.waitFor({ state: 'detached' });
  await turn('2025').click();
  await within.getByText('no quarters', { exact: true }).waitFor();
  await page.screenshot({ path: `${out}/nested-2-empty.png` });

  // A quarter written while the year is open is under it without anybody reading again.
  await add('quarters', 'title=2025 Q1', 'path=2025-q1', 'year=life-years:2025');
  await within.locator('.within-name', { hasText: '2025 Q1' }).waitFor();

  // One is written from under what it is in, and names it without being told to.
  await within.getByRole('button', { name: '+ New quarter', exact: true }).click();
  await within.getByLabel('What the new quarter is called').fill('2025 Q2');
  await page.waitForTimeout(150);
  await page.keyboard.press('Enter');
  await within.locator('.within-name', { hasText: '2025 Q2' }).waitFor();
  const written = (await cli('record', 'show', 'life-quarters:2025-q2', '--json')).stdout;
  expect(`the new quarter to name its year, got ${written}`, JSON.parse(written).data.year === 'life-years:2025');

  // A table whose records name several: a goal that spans two years is under each of them.
  const goals = join(home, 'goals.bp');
  await writeFile(goals, 'blueprint: goals\npermissions: [projects]\nsteps:\n  goals:\n    do: collection.ensure\n    with:\n      name: goals\n      fields:\n        title: { type: text, required: true }\n        spans: { type: records, of: years }\n');
  await cli('apply', goals, '--project', 'life');
  await add('goals', 'title=Learn to sail', 'spans=life-years:2025,life-years:2026');
  await within.locator('.within-name', { hasText: 'Learn to sail' }).waitFor();
  expect(`what names the year to be said a table at a time, got ${await within.locator('.within-head').allInnerTexts()}`, (await within.locator('.within-head').allInnerTexts()).map((one) => one.trim().toLowerCase()).join() === 'goals,quarters');
  await page.screenshot({ path: `${out}/nested-3-several.png` });

  // In its own table it says each it names, by what that is called, and is changed by choosing among them.
  await project.getByRole('button', { name: / Goals$/ }).click();
  await page.getByRole('heading', { name: 'Life goals', level: 1 }).waitFor();
  const spans = page.locator('tr[data-row="life-goals-1"] td').nth(1);
  await spans.locator('.cell-names').nth(1).waitFor();
  // What the records it names are called is read beside its rows, and comes a moment after them: waited for, and said where it never comes.
  await page.waitForFunction(() => [...document.querySelectorAll('tr[data-row="life-goals-1"] td:nth-child(2) .cell-names')].map((one) => one.textContent).join() === '2025,2026', null, { timeout: 10000 }).catch(() => undefined);
  expect(`the goal to say both years, got ${await spans.locator('.cell-names').allInnerTexts()}`, (await spans.locator('.cell-names').allInnerTexts()).join() === '2025,2026');
  await page.getByRole('button', { name: 'Spans of Learn to sail' }).click();
  await page.getByRole('option', { name: '2025' }).waitFor();
  await page.waitForTimeout(150);
  await page.getByRole('option', { name: '2025' }).click();
  await page.getByRole('heading', { name: 'Life goals', level: 1 }).click();
  await page.waitForFunction(() => document.querySelectorAll('tr[data-row="life-goals-1"] .cell-names').length === 1);
  const now = JSON.parse((await cli('record', 'show', 'life-goals-1', '--json')).stdout).data.spans;
  expect(`the goal to name the one year left, got ${JSON.stringify(now)}`, JSON.stringify(now) === '["life-years:2026"]');
  await page.screenshot({ path: `${out}/nested-4-goals.png` });

  // The same in the library's own list: a line is opened into what names it.
  await page.locator('.tab', { hasText: 'Life years' }).first().click();
  await page.getByRole('tab', { name: 'List' }).click();
  await page.locator('.library-line').nth(1).waitFor();
  await page.locator('.library-line').getByRole('button', { name: 'Open what is in 2026', exact: true }).click();
  const under = page.locator('.rows-within');
  await under.locator('.within-name', { hasText: '2026 Q4' }).waitFor();
  expect('the goal that still names 2026 to be under it, and the quarters', (await under.locator('.within-name').allInnerTexts()).map((one) => one.trim()).join() === 'Learn to sail,2026 Q3,2026 Q4');
  await page.screenshot({ path: `${out}/nested-5-list.png` });
  await page.getByRole('tab', { name: 'All' }).click();
  await turn('2025').click();

  // What is shown there is opened from there: a document as its file.
  await within.locator('.within-name', { hasText: '2025 Q1' }).click();
  await page.locator('.tab', { hasText: '2025-q1.md' }).first().waitFor();
  await page.screenshot({ path: `${out}/nested-6-document.png` });

  // The days are kept private. To a key that holds what an agent holds
  // there is no such table, no such folder, and nothing under a week.
  const helper = (await cli('key', 'helper', '--scope', 'projects', '--scope', 'records', '--scope', 'search')).stdout.match(/key_[A-Za-z0-9]+/)?.[0];
  expect('a key to have been made', Boolean(helper));
  const asked = async (path) => {
    const answer = await fetch(`${url}${path}`, { headers: { authorization: `Bearer ${helper}` } });
    return [answer.status, await answer.json()];
  };
  const [, tables] = await asked('/api/collections?project=life');
  const shown = tables.collections.map((table) => table.name).sort().join();
  expect(`the days not to be among the tables a helper is shown, got ${shown}`, shown === 'goals,months,quarters,weeks,years');
  const [status] = await asked('/api/collections/life/days/records');
  expect(`the days to be refused to it, got ${status}`, status === 403);
  const [, tree] = await asked('/api/projects/life/tree');
  expect('none of the days\' files to be listed for it', !tree.entries.some((entry) => entry.path.startsWith('notes/days')) && tree.entries.some((entry) => entry.path === 'notes/weeks/2026-w46.md'));

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
