// A factory started from a copy of another says so across the top of every
// page, and the factory copied says nothing there (a copy is easily
// taken for the real one).
//
//   DAEMON=../target/debug/factory node scripts/across.mjs [out-dir]
//
// A factory called prod is copied (`std/snapshot`), and a factory called
// preview started from the copy. Signed in to each: preview says across
// its top its profile, what it is a copy of and when it was taken, in the
// colour a warning is drawn in, and prod says nothing there. The line is
// over the rail, the side panel and the page, lies over none of them,
// leaves the window nowhere and has nothing scroll sideways, at three
// widths and a phone's. Whoever manages the copy says it may stop, and the
// line goes on that page and on a second open page that is never read
// again. The environment variables are those of walkthrough.mjs; `LOOK` is
// worn as every check wears it.
import { execFile, spawn } from 'node:child_process';
import { access, mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const home = join(out, 'across-home');
const copy = join(out, 'across-copy');
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const PASSWORD = 'correct horse';

await mkdir(out, { recursive: true });
for (const dir of [home, copy]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });

/** Starts a daemon and says where it answers. */
async function start(args) {
  const daemon = spawn(binary, ['serve', '--adapter', 'builtin', '--listen', '127.0.0.1:0', ...args], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
  endsWithThis(daemon);
  return { daemon, url: await saysWhere(daemon) };
}
const cli = (url, ...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const problems = [];
const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};

const daemons = [];
let failed = null;
try {
  const prod = await start(['--profile', 'prod', '--data', join(home, 'prod')]);
  daemons.push(prod.daemon);
  await cli(prod.url, 'register', '--email', 'ada@example.com', '--name', 'Ada', '--password', PASSWORD);
  await cli(prod.url, 'apply', 'std/snapshot', '--param', `to=${copy}`);
  for (let tries = 0; ; tries += 1) {
    if (await access(join(copy, 'snapshot.json')).then(() => true, () => false)) break;
    if (tries > 300) throw new Error('the copy of prod was never made');
    await new Promise((later) => setTimeout(later, 100));
  }
  const preview = await start(['--profile', 'preview', '--data', join(home, 'preview'), '--from', copy]);
  daemons.push(preview.daemon);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const signedIn = async (url, size = { viewport: { width: 1280, height: 860 } }) => {
    const page = await (await browser.newContext({ ...size, colorScheme: process.env.SCHEME ?? 'dark' })).newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill(PASSWORD);
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.shell').waitFor();
    return page;
  };

  // The factory copied says nothing there.
  const there = await signedIn(prod.url);
  await there.locator('.rail').waitFor();
  expect('nothing across the top of the factory copied', (await there.locator('.across-line').count()) === 0);
  await there.screenshot({ path: `${out}/across-1-prod.png` });

  // The copy says what it is, before anything is pressed.
  const page = await signedIn(preview.url);
  const line = page.locator('.across-line');
  await line.waitFor();
  const said = (await line.locator('.across-said').innerText()).trim();
  expect(`the copy to say its profile, what it is a copy of and that what is done there stays there, got "${said}"`, /^Profile: preview\. A copy of prod taken .+: what is done here stays here\.$/.test(said));
  const colour = await line.evaluate((one) => [getComputedStyle(one).borderBottomColor, getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()]);
  expect(`drawn in the colour a warning is drawn in, got ${colour.join(' against ')}`, colour[0] !== 'rgba(0, 0, 0, 0)');

  // By the boxes: over the rail, the side panel and the page, under nothing and over nothing, at three widths and a phone's.
  const laidOut = async (on, what) => {
    // As a person sees it: once the page has been drawn, after what follows the line's height has run.
    await on.evaluate(() => new Promise((drawn) => requestAnimationFrame(() => requestAnimationFrame(drawn))));
    const wrong = await on.evaluate(() => {
      const said = [];
      const line = document.querySelector('.across-line');
      const box = line.getBoundingClientRect();
      if (box.left < -0.5 || box.right > innerWidth + 0.5 || box.top < -0.5) said.push('it leaves the window');
      for (const part of line.querySelectorAll('.across-said, .across-quiet')) {
        const at = part.getBoundingClientRect();
        if (at.left < box.left - 0.5 || at.right > box.right + 0.5 || at.bottom > box.bottom + 0.5) said.push(`${part.className} leaves the line`);
      }
      const [said1, button] = [line.querySelector('.across-said')?.getBoundingClientRect(), line.querySelector('.across-quiet')?.getBoundingClientRect()];
      if (said1 && button && Math.min(said1.right, button.right) - Math.max(said1.left, button.left) > 0.5 && Math.min(said1.bottom, button.bottom) - Math.max(said1.top, button.top) > 0.5) said.push('the words lie over the button');
      // What is drawn of the rail and the side panel, which lie over the page on a phone from the top of the screen, and the page.
      for (const other of document.querySelectorAll('.shell > .rail > *, .shell > .sidebar > *, .shell > .main')) {
        const at = other.getBoundingClientRect();
        if (at.width === 0 || getComputedStyle(other).display === 'none') continue;
        if (at.top < box.bottom - 0.5 && at.bottom > box.top + 0.5 && at.left < box.right && at.right > box.left) said.push(`${other.className.split(' ')[0]} lies under it`);
      }
      // Whatever is at its middle is the line itself: nothing lies over it.
      const top = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
      if (!top || !line.contains(top)) said.push(`${top?.className ?? 'nothing'} lies over it`);
      if (document.documentElement.scrollWidth > innerWidth) said.push('the page scrolls sideways');
      return said;
    });
    expect(`the line over nothing and under nothing ${what}, got ${wrong.join('; ')}`, wrong.length === 0);
  };
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `at ${width} wide`);
    await page.screenshot({ path: `${out}/across-2-copy-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  const phone = await signedIn(preview.url, { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.locator('.across-line').waitFor();
  await laidOut(phone, "at a phone's width");
  await phone.screenshot({ path: `${out}/across-3-phone.png` });

  // Said by the terminal too.
  await cli(preview.url, 'login', '--email', 'ada@example.com', '--password', PASSWORD);
  const before = (await cli(preview.url, 'status')).stdout;
  expect(`the terminal to say what it is a copy of, got ${before}`, /Profile: preview\. A copy of prod taken .+ UTC: what is done here stays here\./.test(before));

  // Whoever manages it says it may stop: gone on that page, and on the phone's, which is never read again.
  await line.getByRole('button', { name: 'Stop saying this' }).click();
  await line.waitFor({ state: 'detached' });
  await phone.locator('.across-line').waitFor({ state: 'detached' });
  const status = (await cli(preview.url, 'status')).stdout;
  expect(`the terminal to say nothing of it once it may stop, got ${status}`, !status.includes('A copy of prod'));
  await page.screenshot({ path: `${out}/across-4-quiet.png` });
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
  console.log('across: a copy says what it is across the top of every page until its owner says it may stop, and the factory copied says nothing');
} catch (err) {
  failed = err;
} finally {
  for (const daemon of daemons) {
    daemon.kill('SIGTERM');
    await new Promise((gone) => (daemon.exitCode === null ? daemon.on('exit', gone) : gone()));
  }
  for (const dir of [home, copy]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
