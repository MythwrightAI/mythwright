// A blueprint's text scrolled quickly: every line that comes into view has
// its number beside it and its colours, from the first frame it is shown.
//
//   DAEMON=../target/debug/factory node scripts/scrolled.mjs [out-dir]
//
// The fleet that ships, opened on its text and scrolled down by the wheel
// the first time, then back up, with what is in view read every frame. The
// colours were the language's as far as it had parsed, which was little
// further than what was in view: lines came in bare and were coloured a
// moment later. What the browser draws apart (the numbers' column, once a
// layer of its own) cannot be seen from here, so of the numbers this holds
// only that each line has its own. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
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
const data = join(out, 'scrolled-data');
const home = join(out, 'scrolled-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
let page = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
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
  const signIn = async (colorScheme, viewport = { width: 1280, height: 860 }) => {
    const opened = await (await browser.newContext({ viewport, colorScheme })).newPage();
    opened.on('pageerror', (err) => problems.push(err.message));
    await opened.goto(url);
    await opened.getByLabel('Email').fill('ada@example.com');
    await opened.getByLabel('Password').fill('correct horse');
    await opened.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await opened.locator('.rail').waitFor();
    return opened;
  };
  page = await signIn(scheme);

  await side(page, 'Resources');
  await unfold(page, page.locator('.nav button.nav-item', { has: page.getByText('std', { exact: true }) }));
  await page.locator('.nav').getByText('std/fleet', { exact: true }).click();
  await page.locator('.view-head').getByText('std/fleet', { exact: false }).first().waitFor();
  await page.getByRole('tab', { name: 'Text' }).last().click();
  await page.locator('.cm-content').waitFor();
  await page.locator('.view-head').getByText('checks').waitFor({ timeout: 8000 });
  await page.evaluate(() => {
    window.frames_ = [];
    const look = () => {
      const scroller = document.querySelector('.cm-scroller');
      if (scroller) {
        const box = scroller.getBoundingClientRect();
        const seen = (r) => r.bottom > box.top + 2 && r.top < box.bottom - 2;
        const lines = [...document.querySelectorAll('.cm-line')].map((l) => l.getBoundingClientRect()).filter(seen);
        const numbers = [...document.querySelectorAll('.cm-lineNumbers .cm-gutterElement')].filter((g) => g.textContent.trim() !== '').map((g) => g.getBoundingClientRect()).filter(seen);
        const bare = lines.filter((l) => !numbers.some((n) => Math.abs(n.top - l.top) < 3)).length;
        const plain = [...document.querySelectorAll('.cm-line')].filter((l) => seen(l.getBoundingClientRect()) && l.textContent.trim() !== '' && !l.querySelector('span')).length;
        const gutter = document.querySelector('.cm-gutters')?.getBoundingClientRect();
        window.frames_.push({ t: Math.round(performance.now()), top: Math.round(scroller.scrollTop), lines: lines.length, numbers: numbers.length, bare, plain, gutterTop: gutter ? Math.round(gutter.top - box.top) : null, gutterH: gutter ? Math.round(gutter.height) : null });
      }
      requestAnimationFrame(look);
    };
    requestAnimationFrame(look);
  });
  const box = await page.locator('.cm-scroller').boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  // A wheel turned about as fast as a hand turns it: what is under test is
  // lines coming into view faster than the language parses on its own.
  // Where the end is grows as the lines that come into view are measured, so it is asked each time.
  const left = () => page.evaluate(() => { const s = document.querySelector('.cm-scroller'); return s.scrollHeight - s.clientHeight - s.scrollTop; });
  const at = () => page.evaluate(() => document.querySelector('.cm-scroller').scrollTop);
  for (let turns = 0; turns < 80 && (await left()) > 2; turns++) {
    await page.mouse.wheel(0, 400);
    await page.waitForTimeout(16);
  }
  await page.waitForFunction(() => { const s = document.querySelector('.cm-scroller'); return s.scrollHeight - s.clientHeight - s.scrollTop < 2; });
  for (let turns = 0; turns < 80 && (await at()) > 0; turns++) {
    await page.mouse.wheel(0, -400);
    await page.waitForTimeout(16);
  }
  await page.waitForFunction(() => document.querySelector('.cm-scroller').scrollTop === 0);
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  await page.screenshot({ path: `${out}/scrolled-1-back-at-the-top.png` });
  const frames = await page.evaluate(() => window.frames_);
  expect(`the text to have been scrolled to its end and back, read in ${frames.length} frames`, frames.length > 60 && Math.max(...frames.map((f) => f.top)) > 8000 && frames.at(-1).top === 0);
  const unnumbered = frames.filter((f) => f.bare > 0);
  expect(`every line in view to have its number, and ${unnumbered.length} frames had one without: ${JSON.stringify(unnumbered.slice(0, 3))}`, unnumbered.length === 0);
  const bare = frames.filter((f) => f.plain > 0);
  expect(`every line in view to be coloured as it came, and ${bare.length} frames had one bare: ${JSON.stringify(bare.slice(0, 3))}`, bare.length === 0);
  console.log(`scrolled: ${frames.length} frames read, every line numbered and coloured as it came`);
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/scrolled-0-failed.png` }).catch(() => {});
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
