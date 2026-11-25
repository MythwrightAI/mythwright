// A daemon asked to stop once what it has in hand is done (`mw down
// --drain`, `ui/src/drained.ts`), as pages that are open see it, each
// never read again: one at a desk and one on a phone. An agent whose
// program is a stand-in has a turn in hand, held where this check listens;
// the drain is asked from a terminal; each page says that the daemon is
// draining, by when it stops at the latest and what it waits for, with
// what becomes of it if it is cut off, by the boxes at three widths and a
// phone's: in the window, nothing of it over anything else of it, nothing
// scrolling sideways. The stand-in is let go, its turn ends, the daemon
// stops, and each page says it drained and stopped. Plainly and with
// `LOOK="size=largest density=compact"`; the other environment variables
// are those of walkthrough.mjs, and `STANDIN` is the stand-in program
// (`crates/hosting/standin`, built with the command line's tests:
// `cargo test -p cli --no-run`).
//
//   DAEMON=../target/debug/factory node scripts/drained.mjs [out-dir]
import { execFile, spawn } from 'node:child_process';
import { access, mkdir, readdir, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const standin = resolve(process.env.STANDIN ?? '../target/debug/cli-standin');
const repo = resolve('..');
const data = join(out, 'drained-data');
const home = join(out, 'drained-home');
const look = process.env.LOOK ? '-looked' : '';

await access(standin).catch(() => {
  throw new Error(`there is no stand-in at ${standin}: build it with \`cargo test -p cli --no-run\`, or say where it is in STANDIN`);
});
await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };

// Where the stand-in waits to be let go: it comes, says who it is, and
// goes on when it is told a line (`held` of the stand-in).
const came = [];
let heard = null;
const hold = createServer((socket) => {
  socket.once('data', () => {
    came.push(socket);
    heard?.();
  });
});
await new Promise((listening) => hold.listen(0, '127.0.0.1', listening));
const held = () =>
  came.length
    ? Promise.resolve(came.shift())
    : new Promise((there) => {
        heard = () => {
          heard = null;
          there(came.shift());
        };
      });
const words = [standin, 'swallow', 'held', `127.0.0.1:${hold.address().port}`, 'builder', 'say', 'done'].join(' ');

const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off', '--command', `writer=${words}`], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const stopped = new Promise((gone) => daemon.once('exit', gone));

let failed = null;
let draining = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('channel', 'create', 'shop', '--project', 'shop');
  await cli('agent', 'add', 'builder', '--adapter', 'writer', '--project', 'shop', '--channel', 'shop', '--no-wall');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const scheme = process.env.SCHEME ?? 'dark';
  const desk = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const small = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const phone = await small.newPage();
  const problems = [];
  const loads = new Map();
  for (const page of [desk, phone]) {
    page.on('pageerror', (err) => problems.push(err.message));
    loads.set(page, -1);
    page.on('load', () => loads.set(page, loads.get(page) + 1));
  }
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  for (const page of [desk, phone]) {
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').first().waitFor({ state: 'attached' });
  }
  expect('no notice before anything drains', !(await desk.locator('.toast.draining').count()));

  // The agent has the turn in hand, its program held here.
  await cli('post', 'shop', '@builder add a cart');
  const program = await held();

  // Asked to drain from a terminal: it waits for the turn.
  draining = spawn(binary, ['down', '--drain', '--within', '10m', '--url', url], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
  let said = '';
  draining.stdout.on('data', (chunk) => (said += chunk));
  const drained = new Promise((over) => draining.once('exit', over));
  const WAITS = "builder's turn: if it is cut off, it is begun again by the next start.";

  /** What is wrong with the notice on `page` by the boxes, at the width it is at now. */
  const boxes = async (page, width) => {
    const notice = page.locator('.toast.draining');
    const box = await notice.boundingBox();
    expect(`the notice in the window at ${width} wide, at ${JSON.stringify(box)}`, box && box.x >= 0 && box.x + box.width <= width + 0.5 && box.y >= 0);
    const wrong = await pageCrowded(page, '.toast.draining');
    expect(`nothing of the notice over anything else of it at ${width} wide, got:\n${wrong.join('\n')}`, wrong.length === 0);
    const sideways = await page.evaluate(() => document.scrollingElement.scrollWidth - document.scrollingElement.clientWidth);
    expect(`nothing scrolling sideways at ${width} wide, got ${sideways}`, sideways <= 0);
  };

  for (const page of [desk, phone]) {
    const notice = page.locator('.toast.draining');
    await notice.getByText(/The daemon is draining to stop: it begins nothing new/).waitFor();
    await notice.getByText(/at the latest/).waitFor();
    await notice.getByText(WAITS, { exact: true }).waitFor();
  }
  for (const width of [1280, 900, 640]) {
    await desk.setViewportSize({ width, height: 860 });
    await boxes(desk, width);
    await desk.screenshot({ path: `${out}/drained${look}-1-draining-${width}.png` });
  }
  await boxes(phone, 390);
  await phone.screenshot({ path: `${out}/drained${look}-1-draining-phone.png` });

  // The daemon goes on while the turn is in hand, and the drain waits.
  expect('the daemon to go on while the turn is in hand', daemon.exitCode === null);
  expect('the drain to wait while the turn is in hand', draining.exitCode === null);

  // Let go, the turn ends, and the daemon stops: each page is told so.
  program.write('go\n');
  for (const page of [desk, phone]) {
    await page.locator('.toast.draining').getByText(/The daemon drained and stopped: nothing was left in hand/).waitFor();
  }
  await drained;
  await stopped;
  expect(`the terminal to say it drained, got:\n${said}`, said.includes('drained after'));
  for (const [page, width] of [[desk, 640], [phone, 390]]) {
    await boxes(page, width);
    await page.screenshot({ path: `${out}/drained${look}-2-stopped-${width === 390 ? 'phone' : width}.png` });
  }
  for (const page of [desk, phone]) expect(`the page never to have been read again, was ${loads.get(page)} times`, loads.get(page) === 0);

  await browser.close();
  // What a page says while the daemon it was opened from has gone is
  // expected: the stream is lost, and the requests made then refused.
  const own = problems.filter((one) => !/not answering|Failed to fetch|NetworkError/.test(one));
  if (own.length) throw new Error(`errors in the page:\n${own.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  draining?.kill('SIGTERM');
  for (const socket of came) socket.destroy();
  hold.close();
  daemon.kill('SIGTERM');
  await stopped;
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
