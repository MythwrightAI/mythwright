// A fleet's two tables in the client: who is in it, and what they do with
// a card, each opened from the board they sit beside, in a tab of its own,
// and changed where it is.
//
//   DAEMON=../target/debug/factory node scripts/arranged.mjs [out-dir]
//
// A project with a board and its fleet, as it is first made: an architect,
// one worker and nobody to review. The row of those who review is given a
// count in its cell, and the fleet brings them up; how many of them look at
// a card, and which, is said in the other table, whose cell for what a
// stage does offers the three things that ship and takes any other word;
// an agent of its own is added as a row, given a role nobody offered and
// then one that is, and taken away again. Nothing here is told to the fleet
// but by changing a row. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { gearLine } from './gear.mjs';
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
const data = join(out, 'arranged-data');
const home = join(out, 'arranged-home');

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

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1500, height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  // The agents that take work, by name, as the daemon has them: what a
  // changed row is to have brought about. Asked until it is so.
  const active = async () =>
    (await cli('agent', 'list')).stdout
      .split('\n')
      .filter((line) => / active/.test(line) && !/leaving/.test(line))
      .map((line) => line.trim().split(/\s+/)[0])
      .sort();
  const until = async (what, wanted) => {
    let last = [];
    for (let tries = 0; tries < 100; tries += 1) {
      last = await active();
      if (wanted(last)) return last;
      await page.waitForTimeout(100);
    }
    throw new Error(`expected ${what}, and the agents are ${last.join(', ')}`);
  };

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await side(page, 'Resources');
  const nav = page.locator('.nav');
  await nav.getByText('Shop', { exact: true }).waitFor();

  // Both tables sit beside the project's board: each is a line of the
  // gear of the board's head, opened beside it, and from there as a tab of its own,
  // which is where this goes on. (`beside.mjs` holds the rest of that.)
  const twist = nav.getByRole('button', { name: 'Unfold Shop', exact: true });
  if (await twist.count()) await twist.click();
  const shown = (name) => ({
    click: async () => {
      await nav.locator('[data-project="shop"]').getByRole('button', { name: /Board$/ }).first().click();
      await page.locator('.view-head h1', { hasText: 'Shop board' }).waitFor();
      await gearLine(page, new RegExp(`^${name}`));
      await drawerAtRest(page).getByRole('button', { name: 'Open as a tab', exact: true }).click();
      await page.getByRole('heading', { name: `Shop ${name.toLowerCase()}`, level: 1 }).waitFor();
    },
  });
  const agents = shown('Agents');
  const stages = shown('Stages');
  await agents.click();
  const table = page.locator('table.grid');
  await table.locator('tr[data-row="shop-agents-3"]').waitFor();
  const head = (await page.locator('.view-head h1').innerText()).trim();
  expect(`the page to be the project's agents, got ${head}`, head === 'Shop agents');
  const cell = (label) => table.getByRole('button', { name: label, exact: true });
  const words = async (label) => (await cell(label).innerText()).trim();
  expect(`the fleet to start with one who works and nobody who reviews, got ${await words('Count of worker')} and ${await words('Count of reviewer')}`, (await words('Count of worker')) === '1' && (await words('Count of reviewer')) === '0');
  expect(`only the architect and the worker to be up, got ${(await active()).join(', ')}`, (await active()).join() === 'shop-architect-1,shop-worker-1');
  await page.screenshot({ path: `${out}/arranged-0-agents.png` });

  // A cell is changed where it is, and the fleet follows: two who review.
  const holds = (label, value) =>
    page.waitForFunction(([name, wanted]) => [...document.querySelectorAll('table.grid button.cell')].some((one) => one.getAttribute('aria-label') === name && one.innerText.trim() === wanted), [label, value]);
  const write = async (label, field, value) => {
    await cell(label).click();
    const input = table.locator('.cell-edit input').first();
    await input.waitFor();
    await input.fill(value);
    await input.press('Enter');
    await table.locator('.cell-edit').waitFor({ state: 'detached' });
    await page.waitForFunction(([name, wanted]) => [...document.querySelectorAll('table.grid button.cell')].some((one) => one.getAttribute('aria-label') === name && one.innerText.trim() === wanted), [label, value]);
    void field;
  };
  await write('Count of reviewer', 'count', '2');
  await until('two who review to be brought up', (names) => names.includes('shop-reviewer-1') && names.includes('shop-reviewer-2'));
  await page.screenshot({ path: `${out}/arranged-1-reviewers.png` });

  // What they do with a card is the other table: how many of them look at each.
  await stages.click();
  await table.locator('tr[data-row="shop-stages-2"]').waitFor();
  const columns = (await table.locator('thead th[data-column]').allInnerTexts()).map((one) => one.trim());
  for (const column of ['Title', 'Does', 'Column', 'Role', 'Picks', 'Pass', 'Fail', 'Seats', 'Needed', 'Rounds', 'Merge']) {
    expect(`the stages to say their ${column}, got ${columns.join(', ')}`, columns.includes(column));
  }
  expect(`work to go on to review and review to done, got ${await words('Pass of work')} and ${await words('Pass of review')}`, (await words('Pass of work')) === 'review' && (await words('Pass of review')) === 'done');
  await write('Seats of review', 'seats', '1');
  // And which of them: how those of a role are chosen between is a choice
  // of the stage's row, which the stage is told as it is told the rest:
  // a stage is handed its row whole, and is called for what it does.
  const stage = table.locator('tr[data-row="shop-stages-2"]');
  await cell('Picks of review').click();
  await page.getByRole('option', { name: 'turn', exact: true }).waitFor();
  const ways = (await page.getByRole('option').allInnerTexts()).map((one) => one.replace(/\s*✓\s*$/, '').trim());
  expect(`the ways of choosing between agents to be offered, got ${ways.join(', ')}`, ways.join() === 'first,turn,least,rank');
  await page.getByRole('option', { name: 'turn', exact: true }).click();
  await stage.locator('.tag', { hasText: 'turn' }).waitFor();
  const told = async () => JSON.parse((await cli('instances', '--json')).stdout).instances.find((one) => one.key === 'reviewing-shop-stages-2')?.params?.row?.picks;
  let picks;
  for (let tries = 0; tries < 100 && picks !== 'turn'; tries += 1) {
    picks = await told();
    if (picks !== 'turn') await page.waitForTimeout(100);
  }
  expect(`the stage to have been told how its agents are chosen between, got ${picks}`, picks === 'turn');
  await page.screenshot({ path: `${out}/arranged-2-stages.png` });

  // What a stage does takes any word, and the three that ship are offered
  // where the cell is filled, with the one it holds marked. Another word
  // is offered as what would be made of it; left there, the cell is as
  // it was.
  const offered = async () => (await page.getByRole('option').allInnerTexts()).map((one) => one.replace(/\s*✓\s*$/, '').trim());
  // A cell says it is pressed, whatever it holds: words too, which are typed once it is.
  const pointed = (one) => one.evaluate((el) => getComputedStyle(el).cursor);
  const cursors = await table.locator('tbody button.cell').evaluateAll((all) => [...new Set(all.map((el) => getComputedStyle(el).cursor))]);
  expect(`every cell, a choice's and words' alike, to be pointed at as something pressed, got ${cursors.join(', ')} and ${await pointed(cell('Does of review'))}`, cursors.join() === 'pointer');
  await cell('Does of review').click();
  await page.getByRole('option', { name: 'plan', exact: true }).waitFor();
  // The cell being filled has no frame, and what is chosen and what is
  // offered are one panel over it: from its corner, as wide as it at
  // least, with no border of its own or about its options.
  const framed = await table.locator('.cell-edit .select').evaluate((el) => {
    const td = el.closest('td');
    const [cell, panel, menu] = [td.getBoundingClientRect(), el.getBoundingClientRect(), el.querySelector('.select-menu').getBoundingClientRect()];
    const edge = (one) => ['Top', 'Right', 'Bottom', 'Left'].reduce((sum, side) => sum + parseFloat(getComputedStyle(one)[`border${side}Width`]), 0);
    return { frame: getComputedStyle(td).boxShadow, edges: edge(el) + edge(el.querySelector('.select-menu')), left: Math.round(panel.left - cell.left), top: Math.round(panel.top - cell.top), wider: Math.round(panel.width - cell.width), within: menu.left >= panel.left - 1 && menu.right <= panel.right + 1 && menu.bottom <= panel.bottom + 1 };
  });
  expect(`the cell being filled to have no frame, and its options to be one panel over it from its corner with no border, got ${JSON.stringify(framed)}`, framed.frame === 'none' && framed.edges === 0 && Math.abs(framed.left) <= 1 && Math.abs(framed.top) <= 1 && framed.wider >= 0 && framed.within);
  const usual = await offered();
  expect(`what a stage does to offer the three that ship, got ${usual.join(', ')}`, [...usual].sort().join() === 'plan,review,work');
  const marked = await page.getByRole('option', { selected: true }).allInnerTexts();
  expect(`the one the stage holds to be marked, got ${marked.join(', ')}`, marked.length === 1 && /^review/.test(marked[0].trim()));
  await page.screenshot({ path: `${out}/arranged-2-offered.png` });
  await table.locator('.cell-edit input').first().fill('team/audit');
  await page.getByRole('option', { name: /^Create\s*team\/audit$/ }).waitFor();
  await page.screenshot({ path: `${out}/arranged-2-does.png` });
  await page.keyboard.press('Escape');
  await table.locator('.cell-edit').waitFor({ state: 'detached' });
  expect(`the stage to do what it did, got ${await words('Does of review')}`, (await words('Does of review')) === 'review');

  // An agent of its own is a row too: added by what it is called, then
  // given the role. It could be run by something else, on another model.
  await agents.click();
  await table.locator('tr[data-row="shop-agents-3"]').waitFor();
  await page.getByRole('button', { name: 'Add an agent', exact: true }).click();
  const naming = table.locator('.cell-edit input').first();
  await naming.waitFor();
  await naming.fill('second');
  await naming.press('Enter');
  await table.locator('tr[data-row="shop-agents-4"]').waitFor();
  await until('the new row to be an agent, of the role it is called after', (names) => names.includes('shop-second-1'));
  // Its role takes any word too, and the ones the fleet starts with are
  // offered. One nobody offered is written, and is the agent's role.
  const role = async () => JSON.parse((await cli('agent', 'list', '--json')).stdout).agents.find((agent) => agent.name === 'shop-second-1')?.role;
  const becomes = async (wanted) => {
    let last;
    for (let tries = 0; tries < 100 && last !== wanted; tries += 1) {
      last = await role();
      if (last !== wanted) await page.waitForTimeout(100);
    }
    return last;
  };
  await cell('Role of second').click();
  await page.getByRole('option', { name: 'reviewer', exact: true }).waitFor();
  const roles = await offered();
  expect(`the roles the fleet starts with to be offered, got ${roles.join(', ')}`, roles.join() === 'architect,worker,reviewer');
  await table.locator('.cell-edit input').first().fill('auditor');
  await page.getByRole('option', { name: /^Create\s*auditor$/ }).waitFor();
  await table.locator('.cell-edit input').first().press('Enter');
  await table.locator('.cell-edit').waitFor({ state: 'detached' });
  await holds('Role of second', 'auditor');
  expect(`the new agent to be of the role that was written, got ${await becomes('auditor')}`, (await becomes('auditor')) === 'auditor');
  // A word that is written and left is kept as well, as any text is: the
  // caret going elsewhere is not giving it up.
  await cell('Role of second').click();
  await table.locator('.cell-edit input').first().fill('inspector');
  await page.getByRole('option', { name: /^Create\s*inspector$/ }).waitFor();
  await table.locator('.cell-edit input').first().press('Tab');
  await table.locator('.cell-edit').waitFor({ state: 'detached' });
  await holds('Role of second', 'inspector');
  // And one that is offered is one press.
  await cell('Role of second').click();
  await page.getByRole('option', { name: 'reviewer', exact: true }).click();
  await table.locator('.cell-edit').waitFor({ state: 'detached' });
  await holds('Role of second', 'reviewer');
  expect(`the new agent to be one of those who review, got ${await becomes('reviewer')}`, (await becomes('reviewer')) === 'reviewer');
  await page.screenshot({ path: `${out}/arranged-3-added.png` });

  // A row is read whole in the drawer and changed there, written when it
  // is saved and not a letter at a time: the fleet follows what was saved.
  await table.locator('tr[data-row="shop-agents-4"]').hover();
  await table.getByRole('button', { name: 'Open second', exact: true }).click();
  const whole = drawerAtRest(page);
  await whole.getByLabel('Count', { exact: true }).fill('2');
  await page.waitForTimeout(300);
  expect(`nothing to be written before it is saved, got ${(await active()).join(', ')}`, !(await active()).includes('shop-second-2'));
  await whole.getByRole('button', { name: 'Save', exact: true }).click();
  await until('the row to be two agents once it is saved', (names) => names.includes('shop-second-2'));

  // And taken away there: its agents leave. What opens a row is shown
  // when the pointer is on the row.
  await table.locator('tr[data-row="shop-agents-4"]').hover();
  await table.getByRole('button', { name: 'Open second', exact: true }).click();
  const away = page.getByRole('button', { name: 'Remove this agent', exact: true });
  await away.waitFor();
  await page.screenshot({ path: `${out}/arranged-4-row.png` });
  await away.click();
  await table.locator('tr[data-row="shop-agents-4"]').waitFor({ state: 'detached' });
  await until('the agents of the row that was taken away to be gone', (names) => !names.includes('shop-second-1') && !names.includes('shop-second-2'));
  expect(`the others to be as they were, got ${(await active()).join(', ')}`, (await active()).join() === 'shop-architect-1,shop-reviewer-1,shop-reviewer-2,shop-worker-1');

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
