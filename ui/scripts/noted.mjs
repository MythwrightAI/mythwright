// What an instance's row says beside its status on Activity, in a column
// of its own (`noteOf` in `ui/src/noted.ts`, drawn by `views/activity.tsx`).
//
//   DAEMON=../target/debug/factory node scripts/noted.mjs [out-dir]
//
// A watcher calls a worker when something is said in a channel, and the
// worker's file is then saved by hand: called again, the watcher goes on
// with the worker as it was applied, and its row says so, a warning on
// an instance that is active. Beside it a blueprint whose one step fails,
// which says why, a failure. With neither, no column is drawn. Each note
// is one line cut where its column ends, red only for the failure and the
// other in the colour a warning is drawn in, and the status stands alone
// in its cell; a row with nothing to say has its cell empty. Pressed, a
// note opens the instance's details in the drawer with the whole sentence
// wrapped, and opens no tab. Nothing lies over anything else or scrolls
// sideways at three widths, and at a phone's width the note is under the
// row's name and there is no column. The watcher, whose file is then
// saved by hand, is active and behind: its two pills have room between
// them, side by side where its column has room for both and one under
// the other where it has not, at three widths and a phone's. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
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
const data = join(out, 'noted-data');
const home = join(out, 'noted-home');

const WATCHER = `blueprint: watcher
description: Calls a worker when something is said.
on:
  said:
    event: channel.message
    steps:
      go: { call: worker, as: the-worker }
`;
const worker = (about) => `blueprint: worker
steps:
  note: { do: emit, with: { topic: shop.worked${about} } }
`;
const BROKEN = `blueprint: broken
description: Fails at its one step.
steps:
  build: { do: shell, with: { run: "exit 3" } }
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
  await cli('channel', 'create', 'general');
  // The project's own blueprints, where the factory keeps them (`@blueprints/` of the project's).
  const blueprints = join(data, 'blueprints', 'projects', 'shop');
  await mkdir(blueprints, { recursive: true });
  await writeFile(join(blueprints, 'watcher.bp'), WATCHER);
  await writeFile(join(blueprints, 'worker.bp'), worker(''));
  await cli('project', 'scan', 'shop');
  await cli('apply', join(blueprints, 'watcher.bp'), '--project', 'shop');

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
  await page.locator('.nav').getByText('Activity', { exact: true }).click();
  await page.getByRole('heading', { name: 'Activity', level: 1 }).waitFor();
  const instances = page.locator('table.instances');
  const row = (name) => instances.locator('tbody tr', { has: page.locator('strong', { hasText: new RegExp(`^${name}$`) }) }).first();

  // ---- with nothing to say, there is no column for it
  await row('watcher').locator('.badge.active').waitFor();
  expect('no column for notes while no row has one', (await instances.locator('th', { hasText: /^Note$/i }).count()) === 0);

  // ---- the worker's file saved by hand: called again, the watcher goes on with it as it was applied, and says so
  await writeFile(join(blueprints, 'worker.bp'), worker(', again: true'));
  await cli('project', 'scan', 'shop');
  await cli('post', 'general', 'go');
  // And a blueprint that fails.
  await writeFile(join(blueprints, 'broken.bp'), BROKEN);
  await cli('apply', join(blueprints, 'broken.bp'), '--project', 'shop').catch(() => {});
  await page.locator('.view-head .pills').getByRole('tab', { name: 'All', exact: true }).click();
  const warning = row('watcher').locator('td.activity-note .activity-said');
  const failure = row('broken').locator('td.activity-note .activity-said');
  await warning.waitFor({ timeout: 20_000 });
  await failure.waitFor({ timeout: 20_000 });
  expect('the column to be headed Note', (await instances.locator('thead th.activity-note').innerText()).trim().toLowerCase() === 'note');
  const said = (await warning.getAttribute('title')) ?? '';
  expect(`the watcher's note to say it went on with the worker as it was applied, got "${said}"`, said.includes('went on with it as it was applied'));
  expect(`the failure to say why, got "${await failure.getAttribute('title')}"`, ((await failure.getAttribute('title')) ?? '').length > 0);
  expect('the watcher to be active', (await row('watcher').locator('.badge.active').count()) === 1);
  // Each in its colour: red for the failure alone, and what was gone on past in the colour a warning is drawn in.
  const colours = await page.evaluate(() => {
    const probe = (look) => {
      const one = document.createElement('span');
      one.style.color = look;
      document.body.append(one);
      const got = getComputedStyle(one).color;
      one.remove();
      return got;
    };
    const of = (selector) => getComputedStyle(document.querySelector(selector)).color;
    return {
      warning: of('table.instances td.activity-note .activity-said.warning'),
      failure: of('table.instances td.activity-note .activity-said.failure'),
      bad: probe('var(--bad)'),
      warned: probe('var(--accent)'),
    };
  });
  expect(`the failure red and the warning in the colour of a warning, got ${JSON.stringify(colours)}`, colours.failure === colours.bad && colours.warning === colours.warned && colours.warning !== colours.bad);
  // The status stands alone in its cell, and a row with nothing to say has its cell empty.
  const statusCell = row('watcher').locator('td').nth(1);
  expect(`the status cell to hold the badge alone, got "${(await statusCell.innerText()).trim()}"`, (await statusCell.innerText()).trim() === 'active' && (await statusCell.locator('.error-text, .activity-said').count()) === 0);
  expect('a row with nothing to say to have its note cell empty', (await row('worker').locator('td.activity-note').innerText()).trim() === '');

  // ---- an instance's pills: room between them, side by side where its column has room for both, else one under the other
  // The watcher's file saved by hand, and nothing applied: the watcher, which is active, is behind the text now there.
  await writeFile(join(blueprints, 'watcher.bp'), WATCHER.replace('Calls a worker when something is said.', 'Calls the worker when something is said.'));
  await cli('project', 'scan', 'shop');
  await row('watcher').locator('.lag-mark').waitFor();
  const arranged = [];
  const pills = async (where) => {
    const got = await row('watcher').locator('td .activity-status').evaluate((status) => {
      const box = (one) => {
        const at = one.getBoundingClientRect();
        return { left: at.left, right: at.right, top: at.top, bottom: at.bottom };
      };
      const cell = status.closest('td');
      const pad = getComputedStyle(cell);
      const room = cell.clientWidth - parseFloat(pad.paddingLeft) - parseFloat(pad.paddingRight);
      return { cell: box(cell), room, gap: parseFloat(getComputedStyle(status).columnGap), pills: [...status.querySelectorAll('.badge')].map(box) };
    });
    const told = JSON.stringify(got);
    expect(`two pills, active and behind, ${where}: ${told}`, got.pills.length === 2);
    const [first, second] = got.pills;
    const inside = got.pills.every((one) => one.left >= got.cell.left - 0.5 && one.right <= got.cell.right + 0.5 && one.top >= got.cell.top - 0.5 && one.bottom <= got.cell.bottom + 0.5);
    expect(`the pills inside their cell ${where}: ${told}`, inside);
    const beside = Math.abs(first.top - second.top) <= 0.5;
    const fits = first.right - first.left + got.gap + (second.right - second.left) <= got.room + 0.5;
    if (beside) expect(`room between the pills side by side ${where}: ${told}`, second.left - first.right >= 4 - 0.5);
    else expect(`room between the pills one under the other ${where}, and only where they have no room side by side: ${told}`, second.top - first.bottom >= 3 - 0.5 && !fits);
    if (fits) expect(`the pills side by side where their column has room for both ${where}: ${told}`, beside);
    arranged.push(`${where}: ${beside ? 'side by side' : 'one under the other'}`);
  };

  // ---- by the boxes: one line, cut where its column ends, nothing over anything else
  const boxes = async (where) => {
    await pills(where);
    const got = await warning.evaluate((one) => {
      const text = one.querySelector('.truncate');
      const cell = one.closest('td').getBoundingClientRect();
      const [at, line] = [one.getBoundingClientRect(), parseFloat(getComputedStyle(text).lineHeight) || 20];
      return {
        cut: text.scrollWidth > text.clientWidth + 0.5,
        lines: Math.round(text.getBoundingClientRect().height / line),
        inside: at.left >= cell.left - 0.5 && at.right <= cell.right + 0.5,
        sideways: document.documentElement.scrollWidth - window.innerWidth,
      };
    });
    expect(`the note on one line, cut short, inside its cell and nothing scrolling sideways ${where}: ${JSON.stringify(got)}`, got.cut && got.lines === 1 && got.inside && got.sideways <= 0);
    const crowded = await pageCrowded(page, '.view:not([hidden])');
    expect(`nothing of Activity over anything else ${where}: ${crowded.join('; ')}`, crowded.length === 0);
  };
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await boxes(`at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/noted-0-column.png` });

  // ---- pressed, the details in the drawer with the whole of it, and no tab opened
  const tabs = await page.locator('.tabs .tab').count();
  await warning.click();
  const drawer = drawerAtRest(page);
  await drawer.waitFor();
  const whole = drawer.locator('pre.said-whole');
  await whole.waitFor();
  const wrapped = await whole.evaluate((one) => ({ text: one.textContent, lines: Math.round(one.getBoundingClientRect().height / (parseFloat(getComputedStyle(one).lineHeight) || 18)), sideways: one.scrollWidth - one.clientWidth }));
  expect(`the drawer to say the whole of it, wrapped, got ${JSON.stringify(wrapped)}`, wrapped.text.includes('went on with it as it was applied') && wrapped.lines > 1 && wrapped.sideways <= 0);
  const wholeColour = await whole.evaluate((one) => getComputedStyle(one).color);
  expect(`the whole of a warning in the drawer in the colour of a warning, got ${wholeColour}`, wholeColour === colours.warned);
  expect('pressing the note to open no tab', (await page.locator('.tabs .tab').count()) === tabs);
  await page.screenshot({ path: `${out}/noted-1-drawer.png` });
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });

  // ---- at a phone's width: under the row's name, and no column
  await page.setViewportSize({ width: 390, height: 800 });
  const under = row('watcher').locator('td').first().locator('.activity-said.under');
  await under.waitFor();
  expect('no column for notes at a phone\'s width', !(await instances.locator('thead th.activity-note').isVisible()));
  const phone = await under.evaluate((one) => {
    const at = one.getBoundingClientRect();
    const text = one.querySelector('.truncate');
    return { left: at.left, right: at.right, screen: window.innerWidth, oneLine: text.getBoundingClientRect().height < 2 * (parseFloat(getComputedStyle(text).lineHeight) || 20) };
  });
  expect(`the note under the name inside the window, on one line, got ${JSON.stringify(phone)}`, phone.left >= 0 && phone.oneLine);
  await pills("at a phone's width");
  const crowdedPhone = await pageCrowded(page, '.view:not([hidden])');
  expect(`nothing of Activity over anything else at a phone's width: ${crowdedPhone.join('; ')}`, crowdedPhone.length === 0);
  console.log(`the pills were ${arranged.join('; ')}`);
  await page.screenshot({ path: `${out}/noted-2-phone.png` });

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
