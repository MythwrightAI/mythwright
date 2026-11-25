// How a time is written for somebody, in their own zone, kept with their
// account.
//
//   DAEMON=../target/debug/factory node scripts/clocked.mjs [out-dir]
//
// Ada, in two browsers that each say they are in Denver, both with
// `general` open, one never read again. Her account keeps nothing, so the
// first page to read it takes up the zone its browser says; a message's
// time is written as she writes one, `10:07 PM MST`, on a clock of twelve
// hours with the zone's short name as it is that day. In Settings she
// chooses New York from the zones, found by what she types, and the other
// page, never read again, writes the same message's time in New York's; she
// chooses a clock of twenty-four hours and it follows again. Nothing of
// the part lies over anything else of it, leaves it or scrolls sideways
// at three widths and a phone's. The environment variables are those of
// walkthrough.mjs, `LOOK` among them.
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
const data = join(out, 'clocked-data');
const home = join(out, 'clocked-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and each command line are given a home of their own, so that nothing here reads or writes what a
// factory already on this machine keeps.
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('channel', 'create', 'general');
  await cli('post', 'general', 'the quince ledger');

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
  // Ada's page in a browser of its own that says it is in `zone`.
  const signedIn = async (zone) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, timezoneId: zone, colorScheme: 'dark' });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor();
    return page;
  };
  // What the account keeps, asked of the daemon from inside the page as the page asks it.
  const kept = (on) =>
    on.evaluate(async () => {
      const answer = await fetch('/api/clock', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      return answer.json();
    });
  // The time the message is written at, waited for until it is as `wanted` says.
  const TIME = `[...document.querySelectorAll('.channel .message')].find((one) => one.textContent.includes('the quince ledger'))?.querySelectorAll('.when')`;
  const writes = async (on, wanted, what) => {
    const said = () => on.evaluate(`(() => { const all = ${TIME}; return all ? all[all.length - 1].textContent.trim() : null })()`);
    const there = on.waitForFunction(`(() => { const all = ${TIME}; return !!all && new RegExp(${JSON.stringify(wanted.source)}).test(all[all.length - 1].textContent.trim()) })()`, null, { timeout: 8000 });
    await there.catch(async () => {
      throw new Error(`expected ${what} to be written as ${wanted}, got "${await said()}"`);
    });
    return said();
  };
  const general = async (on) => {
    const nav = on.getByRole('navigation', { name: 'Channels' });
    if ((await nav.count()) === 0) await on.getByRole('button', { name: 'Channels', exact: true }).first().click();
    await nav.locator('.nav-item', { hasText: /^#?\s*general/ }).first().click();
    await on.locator('.channel .message', { hasText: 'the quince ledger' }).waitFor();
  };

  const page = await signedIn('America/Denver');
  const other = await signedIn('America/Denver');
  for (const on of [page, other]) await general(on);

  // ── Nothing kept: the zone the browser says is taken up, on a clock of twelve hours ──
  const first = await writes(page, /^\d{1,2}:\d\d (AM|PM) M[SD]T$/, "the message's time in Denver");
  await page.waitForFunction(async () => {
    const answer = await fetch('/api/clock', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return (await answer.json()).clock?.zone === 'America/Denver';
  });
  const taken = await kept(page);
  expect(`the account to keep Denver on a clock of twelve hours, got ${JSON.stringify(taken)}`, taken.clock.zone === 'America/Denver' && taken.clock.hours === '12h' && taken.nth === 1);
  await page.screenshot({ path: `${out}/clocked-0-denver.png` });

  // ── Settings: New York, found by what is typed ──
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const panel = page.locator('.clock-setting');
  await panel.getByRole('heading', { name: 'Time', level: 2 }).waitFor();
  await panel.scrollIntoViewIfNeeded();
  expect('Denver to be the zone chosen', (await panel.locator('[aria-selected="true"] [data-zone]').getAttribute('data-zone')) === 'America/Denver');
  await panel.getByLabel('Find a zone').fill('new york');
  const york = panel.locator('[data-zone="America/New_York"]');
  await york.waitFor();
  expect('one zone to be found for new york', (await panel.locator('[data-zone]').count()) === 1);
  await page.screenshot({ path: `${out}/clocked-1-finding.png` });
  await york.click();
  await panel.locator('[data-clock-now]', { hasText: / E[SD]T$/ }).waitFor();
  await panel.getByRole('button', { name: /^Use this browser's zone/ }).waitFor();
  await page.screenshot({ path: `${out}/clocked-2-new-york.png` });

  // The other page, never read again, follows.
  const there = await writes(other, /^\d{1,2}:\d\d (AM|PM) E[SD]T$/, "the message's time on the other page");
  expect(`New York to be two hours on from Denver, got ${first} and ${there}`, first !== there);
  await other.screenshot({ path: `${out}/clocked-3-other-page.png` });

  // ── A clock of twenty-four hours ──
  await panel.getByRole('radio', { name: '24 hours' }).click();
  await writes(other, /^\d\d:\d\d E[SD]T$/, "the message's time on twenty-four hours");
  const after = await kept(page);
  expect(`the account to keep New York on twenty-four hours, got ${JSON.stringify(after)}`, after.clock.zone === 'America/New_York' && after.clock.hours === '24h');
  await general(page);
  await writes(page, /^\d\d:\d\d E[SD]T$/, "the message's time on this page");
  await page.screenshot({ path: `${out}/clocked-4-24h.png` });
  await other.screenshot({ path: `${out}/clocked-5-other-24h.png` });

  // ── Nothing of the part over anything else of it or leaving it, at three widths and a phone's ──
  for (const [width, height] of [
    [1280, 860],
    [900, 800],
    [640, 800],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    if ((await panel.count()) === 0) await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
    if ((await panel.count()) === 0) {
      await page.getByRole('button', { name: 'Expand the side panel' }).click();
      await sideSettled(page);
      await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
    }
    await panel.getByLabel('Find a zone').fill('america');
    await panel.scrollIntoViewIfNeeded();
    const found = await panel.evaluate((box) => {
      const outer = box.getBoundingClientRect();
      const parts = [...box.querySelectorAll(':scope > .panel-head > *, :scope > .stack > :not(.clock-zones):not(.pills), .clock-zones > *, .pills button')]
        .map((one) => ({ said: (one.textContent ?? '').trim().slice(0, 30) || one.tagName, box: one.getBoundingClientRect() }))
        .filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of parts.entries()) {
        if (one.box.left < outer.left - 1 || one.box.right > outer.right + 1) wrong.push(`${one.said} leaves the part`);
        for (const two of parts.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 1 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 1;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      return { wrong, sideways: document.documentElement.scrollWidth - window.innerWidth, count: parts.length };
    });
    expect(`the part to be there at ${width}`, found.count > 4);
    expect(`nothing of the part over anything else of it at ${width}, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`nothing to scroll sideways at ${width}, got ${found.sideways}px`, found.sideways <= 0);
    await page.screenshot({ path: `${out}/clocked-6-at-${width}.png` });
  }

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
