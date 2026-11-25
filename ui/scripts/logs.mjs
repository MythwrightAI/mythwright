// How long the detail of runs' steps is kept and where it is written, in
// Settings, as the factory's owner sees it: kept for ever with nothing
// said, and written in the data folder; kept for a day, chosen, with what
// that lets go of said at once; a folder of the owner's named, written in
// from then on, and the data folder gone back to with that folder still
// read; a folder that is not said whole refused in the daemon's sentence;
// the terminal saying the same of each; nothing of the panel lying over
// anything else of it or scrolling sideways at three widths and a
// phone's; and somebody who may not manage the factory, who is not
// offered it and is refused by the route.
//
//   DAEMON=../target/debug/factory node scripts/logs.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { fillsPage } from './filled.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'logs-data');
const home = join(out, 'logs-home');
const elsewhere = join(out, 'logs-elsewhere');

await mkdir(out, { recursive: true });
for (const dir of [data, home, elsewhere]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// Started to say what its steps were handed, so that every step writes its file.
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--handed'], {
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
  // A run whose step writes its file.
  const file = join(home, 'greet.bp');
  await writeFile(file, ['blueprint: greet', 'steps:', '  hello: { do: emit, with: { topic: greet.said } }', ''].join('\n'));
  const greet = async (key) => JSON.parse((await cli('apply', file, '--key', key, '--json')).stdout);
  await greet('first');

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
  const signIn = async (on, email) => {
    await on.goto(url);
    await on.getByLabel('Email').fill(email);
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  await signIn(page, 'ada@example.com');

  // With nothing said: kept for ever, in the data folder.
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const panel = page.locator('.logs-setting');
  await panel.getByRole('heading', { name: 'Logs of runs', level: 2 }).waitFor();
  await panel.locator('.logs-how-long').getByText("What runs' steps printed is kept for ever.").waitFor();
  await panel.locator('.logs-where').getByText(`It is written in the data folder, ${join(data, 'logs')}.`).waitFor();
  await panel.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/logs-0-usual.png` });

  // Kept for a day, chosen: said at once, and by the terminal the same.
  await panel.getByLabel('Logs are kept').selectOption('1');
  await panel.locator('.logs-how-long').getByText('is let go of after a day').waitFor();
  await page.locator('.toast').getByText('Said.').waitFor();
  const said = (await cli('logs')).stdout;
  expect(`the terminal to say a day, got ${said}`, said.includes('is kept for a day'));
  await page.screenshot({ path: `${out}/logs-1-a-day.png` });

  // A folder of the owner's named: written in from then on.
  await panel.getByLabel('Folder for logs').fill(elsewhere);
  await panel.getByRole('button', { name: 'Write there', exact: true }).click();
  await panel.locator('.logs-where').getByText(`It is written in ${elsewhere}.`).waitFor();
  const second = await greet('second');
  const runs = JSON.parse((await cli('runs', '--instance', second.instance, '--json')).stdout).runs;
  expect('the run to be there', runs.length === 1);
  const written = await readdir(join(elsewhere, 'runs'));
  expect(`its step's file to be written there, got ${written}`, written.includes(`${runs[0].id}.hello.log`));
  const log = (await cli('logs', runs[0].id)).stdout;
  expect(`the terminal to read it from there, got ${log}`, log.includes('handed'));
  await page.screenshot({ path: `${out}/logs-2-elsewhere.png` });

  // Back to the data folder, with that folder still read.
  await panel.getByRole('button', { name: 'Back to the data folder', exact: true }).click();
  await panel.locator('.logs-where').getByText(`What was written before is read where it is: ${elsewhere}.`).waitFor();
  expect('no way back where it is the data folder already', (await panel.getByRole('button', { name: 'Back to the data folder' }).count()) === 0);

  // One that is not said whole is refused in the daemon's sentence, and nothing changes.
  await panel.getByLabel('Folder for logs').fill('relative/logs');
  await panel.getByRole('button', { name: 'Write there', exact: true }).click();
  await page.locator('.toast.bad').getByText('is said whole, from the top of the disk').waitFor();
  await panel.locator('.logs-where').getByText(`It is written in the data folder, ${join(data, 'logs')}.`).waitFor();
  await page.screenshot({ path: `${out}/logs-3-refused.png` });

  // Nothing of the panel lies over anything else of it or leaves it, at three widths and a phone's.
  for (const [width, height] of [
    [1280, 860],
    [900, 800],
    [640, 800],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    if (width < 700 && (await panel.count()) === 0) await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await panel.scrollIntoViewIfNeeded();
    const found = await panel.evaluate((box) => {
      const outer = box.getBoundingClientRect();
      const parts = [...box.querySelectorAll('h2, label, select, input, button, .logs-how-long, .logs-where, .faint')]
        .map((one) => ({ said: (one.textContent ?? '').trim().slice(0, 30) || one.tagName, box: one.getBoundingClientRect() }))
        .filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of parts.entries()) {
        if (one.box.left < outer.left - 1 || one.box.right > outer.right + 1) wrong.push(`${one.said} leaves the panel`);
        for (const two of parts.slice(index + 1)) {
          const inside = (a, b) => a.left >= b.left - 1 && a.right <= b.right + 1 && a.top >= b.top - 1 && a.bottom <= b.bottom + 1;
          if (inside(one.box, two.box) || inside(two.box, one.box)) continue;
          const over = one.box.left < two.box.right - 1 && two.box.left < one.box.right - 1 && one.box.top < two.box.bottom - 1 && two.box.top < one.box.bottom - 1;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      return { wrong, sideways: document.documentElement.scrollWidth - window.innerWidth };
    });
    expect(`nothing of the panel over anything else of it at ${width}: ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`nothing scrolling sideways at ${width}, got ${found.sideways}`, found.sideways <= 0);
    await page.screenshot({ path: `${out}/logs-4-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // Settings is a page of tables (who is here, the keys, the devices, the agents' sign-ins), and fills its tab (`filled.mjs`).
  const settings = () => page.getByRole('button', { name: 'Settings', exact: true }).click();
  if ((await panel.count()) === 0) await settings();
  await fillsPage(page, 'Settings', page.locator('.settings-panels'), { ready: panel, again: settings, shot: `${out}/logs-settings-filled` });

  // Somebody who may not manage the factory is not offered it, and is refused by the route.
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  const theirs = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const other = await theirs.newPage();
  other.on('pageerror', (err) => problems.push(err.message));
  await signIn(other, 'grace@example.com');
  await other.getByRole('button', { name: 'Settings', exact: true }).click();
  await other.getByRole('heading', { name: 'Theme', level: 2 }).waitFor();
  expect('a member not to be offered the panel', (await other.locator('.logs-setting').count()) === 0);
  const refused = await other.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const read = await fetch('/api/system/logs', { headers: mine });
    const set = await fetch('/api/system/logs', { method: 'POST', headers: mine, body: JSON.stringify({ days: 1 }) });
    return [read.status, set.status];
  });
  expect(`a member to be refused reading and saying it, got ${refused}`, refused.join() === '403,403');

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, elsewhere]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
