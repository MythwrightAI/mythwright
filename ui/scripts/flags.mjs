// A project's feature flags beside its environments
// (`views/projectflags.tsx`, over `ui/src/flags.ts`): a project set up with
// its environments and its flags from a terminal, three flags added there
// as rows of its table; on its Environments page, never read again, a
// table under the environments' with a row for each flag and a cell for
// each environment saying what it says there, off where nothing is said,
// and the one past its day said to be; a switch pressed in dev and on at
// once there and nowhere else, and what dev's variable then holds the
// flag's; a percentage written in its cell, what is not one refused
// before anything is asked, and one that is set; the switch pressed in
// prod, whose rule names Grace as approving, saying it waits for a
// go-ahead and staying off, and on once Grace lets it go on from a
// terminal; turned off, asked on again and said off from a terminal while
// that waits, Grace's approval writing nothing and the cell staying off;
// dev's drawer saying each flag as it is there, its variable
// not among the variables to change by hand; Settings saying no
// experiment is listed in this version; and nothing of the table, the
// drawer or Settings' part over anything else or past the window at three
// widths and a phone's.
//
//   DAEMON=../target/debug/factory node scripts/flags.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'flags-data');
const home = join(out, 'flags-home');
const graceHome = join(out, 'flags-grace');

await mkdir(out, { recursive: true });
for (const dir of [data, home, graceHome]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// A home of its own, so that nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const graceEnv = { ...process.env, HOME: graceHome, XDG_CONFIG_HOME: join(graceHome, 'config'), XDG_DATA_HOME: join(graceHome, 'share') };
const grace = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: graceEnv });

const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};

let failed = null;
let shot = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'grace@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await grace('register', '--email', 'grace@example.com', '--name', 'Grace', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', 'Shop', '--without', 'board', '--without', 'fleet', '--without', 'docs', '--with', 'environments', '--with', 'flags');
  await cli('record', 'add', 'shop', 'flags', 'name=new-checkout', 'about=The one-page checkout');
  await cli('record', 'add', 'shop', 'flags', 'name=fast-path', 'kind=percentage');
  await cli('record', 'add', 'shop', 'flags', 'name=old-banner', 'until=2020-01-31');
  await cli('project', 'environment', 'shop', 'prod', 'approve', 'grace@example.com');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  shot = () => page.screenshot({ path: `${out}/flags-failed.png` }).catch(() => {});
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(page);
  await page.locator('.nav [data-project="shop"]').getByText('Environments', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop environments', level: 1 }).waitFor();

  // ---- the table of flags, beside the environments ---------------------------------------
  const flags = page.locator('.flags-panel');
  const cell = (flag, environment) => flags.locator(`tr[data-flag="${flag}"] td[data-environment="${environment}"]`);
  const value = (flag, environment) => cell(flag, environment).locator('.flags-value');
  await value('new-checkout', 'dev').waitFor();
  for (const environment of ['dev', 'prod']) {
    expect(`new-checkout off in ${environment} where nothing is said`, (await value('new-checkout', environment).innerText()) === 'off');
    expect(`fast-path 0% in ${environment}`, (await value('fast-path', environment).innerText()) === '0%');
  }
  await flags.locator('tr[data-flag="old-banner"] .flags-due', { hasText: 'past its day' }).waitFor();
  expect('a flag with no day not past it', (await flags.locator('tr[data-flag="new-checkout"] .flags-due').count()) === 0);
  await page.screenshot({ path: `${out}/flags-0-table.png` });

  // A switch pressed in dev is on there at once, and nowhere else.
  await value('new-checkout', 'dev').click();
  await flags.locator('tr[data-flag="new-checkout"] td[data-environment="dev"] .flags-value.on', { hasText: 'on' }).waitFor();
  expect('prod still off', (await value('new-checkout', 'prod').innerText()) === 'off');
  const dev = await page.evaluate(async () => {
    const asked = await fetch('/api/projects/shop/environments/dev', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return asked.json();
  });
  expect(`dev's variable to hold the flag, got ${JSON.stringify(dev.vars?.FLAG_NEW_CHECKOUT)}`, dev.vars?.FLAG_NEW_CHECKOUT?.value === 'on');

  // A percentage is written: what is not one is refused before anything is asked.
  await value('fast-path', 'dev').click();
  const typing = cell('fast-path', 'dev').locator('.flags-typing');
  await typing.getByLabel('What fast-path says in dev').fill('250');
  await typing.getByText('A percentage is a whole number from 0 to 100.').waitFor();
  expect('Set closed for what is no percentage', await typing.getByRole('button', { name: 'Set', exact: true }).isDisabled());
  await typing.getByLabel('What fast-path says in dev').fill('30%');
  await typing.getByRole('button', { name: 'Set', exact: true }).click();
  await flags.locator('tr[data-flag="fast-path"] td[data-environment="dev"] .flags-value', { hasText: '30%' }).waitFor();

  // In prod, whose rule names Grace, turning it on waits for her.
  await value('new-checkout', 'prod').click();
  await cell('new-checkout', 'prod').getByText('on waits for a go-ahead').waitFor();
  expect('prod still off while it waits', (await value('new-checkout', 'prod').innerText()) === 'off');
  await page.screenshot({ path: `${out}/flags-1-waits.png` });
  // Grace lets it go on from a terminal, and the page says so, never read again.
  // Told a moment after the page was answered, by what looks after the flags: asked again until it is there.
  let ask = '';
  for (let tries = 0; tries < 300 && !ask; tries += 1) {
    const listed = JSON.parse((await grace('asked', '--json')).stdout);
    ask = (listed.asked ?? []).find((one) => one.status === 'waiting')?.ask ?? '';
  }
  expect('Grace to be asked', ask !== '');
  await grace('approve', ask);
  await flags.locator('tr[data-flag="new-checkout"] td[data-environment="prod"] .flags-value.on', { hasText: 'on' }).waitFor();
  await cell('new-checkout', 'prod').getByText('waits for a go-ahead').waitFor({ state: 'detached' });

  // Turned off, asked on again, and said off from a terminal while that waits: Grace's approval writes nothing.
  await value('new-checkout', 'prod').click();
  await flags.locator('tr[data-flag="new-checkout"] td[data-environment="prod"] .flags-value:not(.on)', { hasText: 'off' }).waitFor();
  await value('new-checkout', 'prod').click();
  await cell('new-checkout', 'prod').getByText('on waits for a go-ahead').waitFor();
  await cli('flag', 'set', 'shop', 'new-checkout', 'prod', 'off');
  let again = '';
  for (let tries = 0; tries < 300 && !again; tries += 1) {
    const listed = JSON.parse((await grace('asked', '--json')).stdout);
    again = (listed.asked ?? []).find((one) => one.status === 'waiting')?.ask ?? '';
  }
  expect('Grace to be asked again', again !== '');
  await grace('approve', again);
  await cell('new-checkout', 'prod').getByText('waits for a go-ahead').waitFor({ state: 'detached' });
  expect('prod still off: the approval undid nothing', (await value('new-checkout', 'prod').innerText()) === 'off');

  // ---- dev's drawer: each flag as it is there, not among the variables -------------------
  await page.locator('.envs-page tr[data-environment="dev"]').click();
  const drawer = drawerAtRest(page);
  const opened = drawer.locator('[data-opened="dev"]');
  await opened.locator('.flags-in [data-flag-var="FLAG_NEW_CHECKOUT"]', { hasText: 'on' }).waitFor();
  await opened.locator('.flags-in [data-flag-var="FLAG_FAST_PATH"]', { hasText: '30%' }).waitFor();
  expect('a flag\'s variable not offered to change by hand', (await opened.locator('[data-var="FLAG_NEW_CHECKOUT"]').count()) === 0);

  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await opened.locator('.flags-in').scrollIntoViewIfNeeded();
    const wrong = await pageCrowded(page, '.drawer');
    expect(`nothing of the drawer over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    await page.screenshot({ path: `${out}/flags-2-drawer-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await flags.scrollIntoViewIfNeeded();
    const wrong = await pageCrowded(page, '.envs-page');
    expect(`nothing of the page over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    expect(`the page not to scroll sideways at ${width}`, await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
    await page.screenshot({ path: `${out}/flags-3-table-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // ---- Settings, Experiments: none listed in this version --------------------------------
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
  const experiments = page.locator('.experiments-panel');
  await experiments.getByText('None is listed in this version.', { exact: false }).waitFor();
  await experiments.scrollIntoViewIfNeeded();
  const wrong = await pageCrowded(page, '.experiments-panel');
  expect(`nothing of Experiments over anything else: ${wrong.join('; ')}`, wrong.length === 0);
  await page.screenshot({ path: `${out}/flags-4-experiments.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await shot?.();
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, graceHome]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
