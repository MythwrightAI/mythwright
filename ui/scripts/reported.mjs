// Whoever says something in a channel is among its members:
// the fleet's channel with every agent the fleet brought up listed among
// who is in it, the architect, the worker and the reviewer, each said to
// be an agent; and an agent that is not in a channel refused saying
// something there, from a terminal with its own credential, in the
// daemon's sentence that says how it is brought in, with nothing of it on
// the page; brought in, what it says arriving on a page that is never
// read again, and listed with the rest. The environment variables are
// those of walkthrough.mjs.
//
//   DAEMON=../target/debug/factory node scripts/reported.mjs [out-dir]
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'reported-data');
const home = join(out, 'reported-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
// The agents run on the assistant that is built in, which is no program: nothing is started for them.
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const FLEET = ['shop-architect-1', 'shop-worker-1', 'shop-reviewer-1'];

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  // One who reviews too: the fleet's row says none until it is given a count.
  await cli('record', 'set', 'shop-agents-3', 'count=1');
  await cli('agent', 'add', 'helper');
  // The fleet follows its row by itself: what is in the channel is read until the reviewer is there.
  const inShop = async () => {
    const listed = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels;
    return listed.find((one) => one.name === 'shop').members.map((one) => one.name);
  };
  for (let tries = 0; !(await inShop()).includes('shop-reviewer-1'); tries += 1) {
    if (tries > 200) throw new Error(`expected the reviewer brought into #shop, got ${await inShop()}`);
  }

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
  let loads = -1;
  page.on('load', () => (loads += 1));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  // Every branch of the channels starts folded (`folded.mjs`): the project's, which holds `shop`, is unfolded as a person would.
  await unfoldChannels(page);
  await page.locator('.nav').getByText('shop', { exact: true }).first().click();
  await page.getByRole('heading', { name: '# shop' }).waitFor();

  const everyone = page.getByRole('button', { name: /participants?$/ });
  const list = page.getByRole('menu', { name: 'Participants' });
  const listed = async () => {
    await everyone.click();
    await list.locator('.person').first().waitFor();
    const lines = await list.locator('.person').evaluateAll((all) =>
      all.map((line) => ({ name: line.querySelector('.person-name').textContent.trim(), kind: line.querySelector('.person-kind').textContent.trim() })),
    );
    return lines;
  };
  const shut = async () => {
    // A menu starts listening for Escape a moment after it is drawn.
    await page.waitForTimeout(250);
    await page.keyboard.press('Escape');
    await list.waitFor({ state: 'detached' });
  };

  // Every agent the fleet brought up is in the channel it reports in.
  const first = await listed();
  for (const agent of FLEET) {
    const line = first.find((one) => one.name === agent);
    expect(`${agent} to be listed among who is in #shop, got ${JSON.stringify(first)}`, line !== undefined);
    expect(`${agent} to be said to be an agent, got "${line.kind}"`, line.kind === 'agent');
  }
  expect(`helper not to be listed, got ${JSON.stringify(first)}`, !first.some((one) => one.name === 'helper'));
  await page.screenshot({ path: `${out}/reported-1-members.png` });
  await shut();

  // An agent not in the channel says nothing there, and is told how it is brought in.
  const token = (await readFile(join(data, 'slots', 'helper', 'token'), 'utf8')).trim();
  const as = (...args) => run(binary, [...args, '--url', url, '--token', token], { cwd: repo, env });
  const said = await page.locator('.messages .message').count();
  const refused = await as('post', 'shop', 'I have done it.').then(
    () => null,
    (err) => err.stderr,
  );
  expect('the post to be refused', refused !== null);
  expect(`the refusal to say who is not in the channel, got "${refused}"`, refused.includes('`helper` is not in #shop'));
  expect(`the refusal to say how it is brought in, got "${refused}"`, refused.includes('channel add shop helper'));
  expect('nothing of it on the page', (await page.locator('.messages .message').count()) === said && !(await page.getByText('I have done it.').count()));

  // Brought in, what it says arrives, and it is listed with the rest.
  await cli('channel', 'add', 'shop', 'helper');
  await as('post', 'shop', 'I have done it.');
  await page.locator('.messages .message', { hasText: 'I have done it.' }).waitFor();
  // Who is in it is read again as somebody joins: the head counts one more.
  await page.getByRole('button', { name: new RegExp(`${first.length + 1} participants$`) }).waitFor();
  const after = await listed();
  expect(`helper to be listed once it is in, got ${JSON.stringify(after)}`, after.some((one) => one.name === 'helper' && one.kind === 'agent'));
  await page.screenshot({ path: `${out}/reported-2-brought-in.png` });
  await shut();
  expect(`the page never to have been read again, was ${loads} times`, loads === 0);

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
