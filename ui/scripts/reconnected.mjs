// What a page shows of a run that ended while it was not hearing the
// daemon. What the runner says of its runs and steps is passed on as it is
// said and kept only where a text hears it, so the stream that goes on
// after a reconnect cannot hand that over: it says first that it is behind
// (`behind`, with the topics), and whatever follows one of them reads again.
// A run of a handler is begun and shown under way on Activity, the page
// loses the daemon, the run ends meanwhile, and once the page has the
// daemon again it shows the run as over with nothing read again by hand.
//
//   DAEMON=../target/debug/factory node scripts/reconnected.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { createServer, request } from 'node:http';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'reconnected-data');
const home = join(out, 'reconnected-home');

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

// The page reaches the daemon through something in front of it, which can
// cut the stream of events and keep it cut: a browser's own going offline
// does not reach a connection to this machine. Cut, each stream is ended
// as it is opened, which a browser takes for the daemon not answering and
// tries again; let be, everything goes through as it is.
const daemonAt = new URL(url);
const streams = new Set();
let cut = false;
// How many times a stream was asked for while it was cut, and opened since it was let be.
let refused = 0;
let opened = 0;
const front = createServer((asked, answer) => {
  const isStream = asked.url?.startsWith('/api/events?') || asked.url === '/api/events';
  if (isStream && cut) {
    refused++;
    return void asked.socket.destroy();
  }
  const onward = request(
    { host: daemonAt.hostname, port: daemonAt.port, path: asked.url, method: asked.method, headers: asked.headers },
    (came) => {
      answer.writeHead(came.statusCode ?? 502, came.headers);
      came.pipe(answer);
    },
  );
  onward.on('error', () => answer.destroy());
  asked.pipe(onward);
  if (isStream) {
    opened++;
    streams.add(asked.socket);
    asked.socket.on('close', () => streams.delete(asked.socket));
  }
});
await new Promise((listening) => front.listen(0, '127.0.0.1', listening));
const pageAt = `http://127.0.0.1:${front.address().port}`;
/** Until `holds` says so, asked again as the proxy hears something: what is waited for here is the browser trying again. */
const until = async (holds, what) => {
  for (let tries = 0; !holds(); tries++) {
    if (tries > 600) throw new Error(`expected ${what}`);
    await new Promise((later) => setTimeout(later, 50));
  }
};
const cutStreams = () => {
  cut = true;
  for (const socket of streams) socket.destroy();
};

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  // A handler whose run waits until it is told to go on: nothing hears what
  // the runner says of it, so all of that is passed on and kept nowhere.
  const file = join(home, 'waits.bp');
  await writeFile(
    file,
    ['blueprint: waits', 'on:', '  begun:', '    event: go.on', '    steps:', '      hold: { do: wait, with: { event: go.end } }', ''].join('\n'),
  );
  const applied = JSON.parse((await cli('apply', file, '--key', 'waits', '--project', 'shop', '--json')).stdout);
  const runs = async () => JSON.parse((await cli('runs', '--instance', applied.instance, '--json')).stdout).runs;

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
  await page.goto(pageAt);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const railed = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
  if ((await railed.getAttribute('aria-pressed')) !== 'true') await railed.click();
  await unfoldAll(page);
  await resources.locator('.nav').getByText('Activity', { exact: true }).click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'Activity' }).first().waitFor();

  // The handler's run is begun, and shown under way.
  await cli('emit', 'go.on');
  const row = page.locator('tr', { hasText: 'on begun' });
  await row.locator('.badge.running').waitFor();
  await page.screenshot({ path: `${out}/reconnected-0-under-way.png` });

  // The page loses what the daemon says as it says it.
  cutStreams();
  await until(() => refused > 0, 'the browser to have tried the stream again and been refused');
  // The run ends meanwhile, which the page does not hear.
  await cli('emit', 'go.end');
  for (let tries = 0; ; tries++) {
    const handled = (await runs()).find((one) => one.flow === 'on.begun');
    if (handled?.status === 'ok') break;
    expect(`the run to have ended by itself, got ${JSON.stringify(handled)}`, tries < 200);
    await new Promise((later) => setTimeout(later, 50));
  }
  expect('the page to show the run under way still, having heard nothing', (await row.locator('.badge.running').count()) === 1);
  await page.screenshot({ path: `${out}/reconnected-1-lost.png` });

  // The daemon again: the stream says it is behind, and the page reads the
  // runs again by itself.
  const before = opened;
  cut = false;
  await until(() => opened > before, 'the browser to have opened the stream again');
  await row.locator('.badge.ok').waitFor();
  await page.screenshot({ path: `${out}/reconnected-2-read-again.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  front.close();
  front.closeAllConnections();
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
