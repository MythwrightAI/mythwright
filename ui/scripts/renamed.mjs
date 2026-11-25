// A channel called something else, from its row's menu.
//
//   DAEMON=../target/debug/factory node scripts/renamed.mjs [out-dir]
//
// Two people, each on a page of their own that is never read again: Ada,
// who owns the factory, and Nell, who registered by an invitation. Both
// have `general` open. Ada opens its row's menu, whose first line is
// Rename…, and is asked in a small dialog with its name in a field: what
// she types is said as the name it comes to, a name another channel has
// is refused in the daemon's sentence with the dialog staying as it was
// typed, and Enter saves another. On both pages the side panel, the tab
// and the channel's head say what it is called now, with what was said
// there; Nell is told nothing of it but what it is called. A private
// channel of Ada's is called something else by its title, and Nell, who
// is not in it, sees nothing of it. Nothing of the dialog lies over
// anything else of it at three widths and a phone's, where the row's
// menu is pressed by touch. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'renamed-data');
const home = join(out, 'renamed-home');
const hers = join(out, 'renamed-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
// The daemon and each command line are given a home of their own, so that nothing here reads or writes what a
// factory already on this machine keeps.
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  for (const name of ['general', 'lobby']) await cli('channel', 'create', name);
  await cli('channel', 'create', 'Pay review', '--kept');
  await cli('post', 'general', 'the quince ledger');

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
  // Somebody's page, signed in: each in a browser of their own, so that each has their own session.
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  const page = await signedIn('ada@example.com');
  const theirs = await signedIn('nell@example.com');
  const panel = page.getByRole('navigation', { name: 'Channels' });
  const other = theirs.getByRole('navigation', { name: 'Channels' });
  for (const one of [panel, other]) await one.waitFor();

  // What a panel lists, as it is drawn.
  const LISTED = `[...document.querySelectorAll('nav[aria-label="Channels"] .grouped-channel > div > .nav-row .nav-item')]
    .map((item) => item.querySelector('.truncate').textContent)
    .join(' ')`;
  const lists = async (on, wanted, whose) => {
    const there = on.waitForFunction(`(${LISTED}) === ${JSON.stringify(wanted)}`, null, { timeout: 8000 });
    await there.catch(async () => {
      throw new Error(`expected ${whose} to list "${wanted}", got "${await on.evaluate(LISTED)}"`);
    });
  };
  const row = (nav, called) => nav.locator('.grouped-channel', { has: nav.page().locator('.nav-item .truncate', { hasText: new RegExp(`^${called}$`) }) });
  // The boxes of what `selector` picks inside `within`: none lies over another, and each is inside `within`.
  const apart = async (within, selector, what) => {
    const found = await within.evaluate((box, picked) => {
      const frame = box.getBoundingClientRect();
      const boxes = [...box.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < frame.left - 0.5 || one.box.right > frame.right + 0.5) wrong.push(`${one.said} leaves it`);
        if (one.box.top < 0 || one.box.bottom > window.innerHeight + 0.5) wrong.push(`${one.said} is off the screen`);
        for (const two of boxes.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth };
    }, selector);
    expect(`${what}: something to be there`, found.count > 1);
    expect(`${what}: nothing over anything else or outside its place, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px`, found.sideways <= 0);
  };
  const WIDTHS = [1280, 1000, 800];
  const DIALOG = 'h2, input, #rename-says, [role="alert"], .row button';
  // A channel's page as it says what it is called: its head and its tab.
  const shows = async (on, called, whose) => {
    const head = on.locator('.channel .view-head h1');
    const said = on.waitForFunction((wanted) => document.querySelector('.channel .view-head h1')?.textContent.replace(/\s+/g, ' ').trim() === wanted, `# ${called}`, { timeout: 8000 });
    await said.catch(async () => {
      throw new Error(`expected ${whose} head to say # ${called}, got "${await head.innerText()}"`);
    });
    await on.getByRole('tab', { name: new RegExp(`^#?\\s*${called}`) }).first().waitFor({ timeout: 8000 });
    await on.locator('.channel .message', { hasText: 'the quince ledger' }).waitFor();
  };

  await lists(page, 'general lobby Pay review', "Ada's panel");
  await lists(theirs, 'general lobby', "Nell's panel");
  for (const [on, nav] of [
    [page, panel],
    [theirs, other],
  ]) {
    await row(nav, 'general').locator('.nav-item').click();
    await shows(on, 'general', 'the page');
  }

  // ── Renamed from its row's menu, the first line of it ─────────────────────
  await row(panel, 'general').click({ button: 'right' });
  const menu = page.getByRole('menu');
  await menu.waitFor();
  const first = (await menu.locator('[role="menuitem"]').first().innerText()).trim();
  expect(`Rename… to be the first line of the channel's menu, got "${first}"`, first === 'Rename…');
  await menu.getByText('Rename…', { exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByText('Rename #general', { exact: true }).waitFor();
  const field = dialog.getByLabel('Channel name');
  expect('the field to hold its name, with the keyboard in it', (await field.inputValue()) === 'general' && (await field.evaluate((input) => document.activeElement === input)));
  await field.fill('The Front Desk');
  await dialog.getByText('It will be called #the-front-desk.', { exact: true }).waitFor();
  // A name another channel has: refused in the daemon's sentence, the dialog staying as it was typed.
  await field.fill('Lobby');
  await page.keyboard.press('Enter');
  const refused = dialog.getByRole('alert');
  await refused.waitFor();
  expect(`the daemon's sentence, got "${await refused.innerText()}"`, (await refused.innerText()) === 'there is already a channel called lobby');
  expect('what was typed to stay', (await field.inputValue()) === 'Lobby');
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(dialog, DIALOG, `the dialog at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/renamed-0-refused.png` });
  await field.fill('The Front Desk');
  await page.screenshot({ path: `${out}/renamed-1-asked.png` });
  await page.keyboard.press('Enter');
  await dialog.waitFor({ state: 'detached' });

  // ── On both pages, neither read again ─────────────────────────────────────
  await lists(page, 'lobby Pay review the-front-desk', "Ada's panel");
  await lists(theirs, 'lobby the-front-desk', "Nell's panel");
  await shows(page, 'the-front-desk', "Ada's");
  await shows(theirs, 'the-front-desk', "Nell's");
  await page.screenshot({ path: `${out}/renamed-2-renamed.png` });
  await theirs.screenshot({ path: `${out}/renamed-3-other-page.png` });
  // Said there by its new name, and heard on the other page.
  await cli('post', 'the-front-desk', 'the medlar ledger');
  await theirs.locator('.channel .message', { hasText: 'the medlar ledger' }).waitFor();

  // ── A private channel, by what it is called ──────────────────────────────
  await row(panel, 'Pay review').locator('.nav-item').click();
  await row(panel, 'Pay review').getByRole('button', { name: /^More for/ }).click();
  await menu.getByText('Rename…', { exact: true }).click();
  await dialog.getByText('Rename Pay review', { exact: true }).waitFor();
  const name = dialog.getByLabel('Name', { exact: true });
  expect('the field to hold what it is called', (await name.inputValue()) === 'Pay review');
  await name.fill('Salary review');
  await page.keyboard.press('Enter');
  await dialog.waitFor({ state: 'detached' });
  await row(panel, 'Salary review').waitFor();
  await page.locator('.channel .view-head h1', { hasText: 'Salary review' }).waitFor();
  // Something in the open for Nell's page to have heard: all that came before it has come by then.
  await cli('post', 'lobby', 'a marker');
  await row(other, 'lobby').locator('.nav-item').click();
  await theirs.locator('.channel .message', { hasText: 'a marker' }).waitFor();
  expect("nothing of the private channel on Nell's page", !(await theirs.locator('body').innerText()).match(/Salary|Pay review/));
  await page.screenshot({ path: `${out}/renamed-4-private.png` });

  // ── A phone's width ─────────────────────────────────────────────────────
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  const small = phone.getByRole('navigation', { name: 'Channels' });
  await row(small, 'lobby').waitFor();
  await row(small, 'lobby').getByRole('button', { name: /^More for/ }).tap();
  await phone.getByRole('menu').getByText('Rename…', { exact: true }).tap();
  const onPhone = phone.getByRole('dialog');
  await onPhone.getByText('Rename #lobby', { exact: true }).waitFor();
  await onPhone.getByLabel('Channel name').fill('the-front-desk');
  await onPhone.getByRole('button', { name: 'Save', exact: true }).tap();
  await onPhone.getByRole('alert').waitFor();
  await apart(onPhone, DIALOG, 'the dialog on a phone');
  await phone.screenshot({ path: `${out}/renamed-5-phone.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
