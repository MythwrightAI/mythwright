// A key revoked and a key rolled from the owner's Settings
// (`views/keyrevoked.tsx`, over `ui/src/keyrevoked.ts`): a key that holds
// less, which applied something that stays and made a key that applied
// something too; Revoke… on its line asking first, saying the key made
// with it and what each applied, which is stopped; revoked, the two keys
// nobody's and what they applied stopped, another key's left alone;
// another revoked leaving what it applied running, as the question was
// told to; and one rolled, its new secret shown once, the old one refused
// and the new one the same key, what it applied going on. Nothing of the
// panel or of either question over anything else or scrolling sideways at
// three widths and a phone's.
//
//   DAEMON=../target/debug/factory node scripts/keyrevoked.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
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
const repo = resolve('..');
const data = join(out, 'keyrevoked-data');
const home = join(out, 'keyrevoked-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
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
const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};
const SCOPES = ['blueprints', 'channels', 'emit'];

/** A key made from the terminal, holding less than its maker: its id and its token. */
const made = async (name) => {
  const scopes = SCOPES.flatMap((scope) => ['--scope', scope]);
  const token = (await cli('key', name, ...scopes)).stdout.split('\n')[0];
  const listed = JSON.parse((await cli('keys', '--json')).stdout);
  return { id: listed.credentials.find((one) => one.name === name).id, token };
};
/** Applies a blueprint called `name` that stays, as `token`: the instance. */
const applied = async (token, name) => {
  const file = join(home, `${name}.bp`);
  await writeFile(
    file,
    `blueprint: ${name}\non:\n  heard:\n    event: channel.message\n    steps:\n      again: { do: emit, with: { topic: ${name}.heard } }\n`,
  );
  const before = await alive();
  await cli('apply', file, '--token', token);
  return (await alive()).find((one) => !before.includes(one));
};
/** The ids of what is alive. */
const alive = async () => {
  const listed = JSON.parse((await cli('instances', '--json')).stdout);
  return (listed.instances ?? listed).map((one) => one.id);
};
/** Who a token is: the status. */
const whose = async (token) => (await fetch(`${url}/api/me`, { headers: { authorization: `Bearer ${token}` } })).status;

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  const first = await made('deploys');
  const nested = await fetch(`${url}/api/credentials`, {
    method: 'POST',
    headers: { authorization: `Bearer ${first.token}`, 'content-type': 'application/json' },
    body: JSON.stringify({ name: 'nightly', scopes: ['blueprints', 'emit'] }),
  }).then((answer) => answer.json());
  const left = await made('reports');
  const rolled = await made('rotated');
  const byFirst = await applied(first.token, 'deploying');
  const byNested = await applied(nested.token, 'nightly-run');
  const byLeft = await applied(left.token, 'reporting');
  const byRolled = await applied(rolled.token, 'rotating');

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
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const panel = page.locator('.credentials-panel');
  const line = (name) => panel.locator('tr', { has: page.getByRole('cell', { name, exact: true }) });
  await line('deploys').waitFor();

  // What is held by the boxes: nothing over anything else, nothing past the window, nothing scrolling sideways.
  const held = async (within, what, picture) => {
    for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
      await page.setViewportSize({ width, height });
      const wrong = await pageCrowded(page, within);
      expect(`nothing of ${what} over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
      const sideways = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
      expect(`the page not to scroll sideways with ${what} at ${width}`, !sideways);
      await page.screenshot({ path: `${out}/keyrevoked-${picture}-at-${width}.png` });
    }
    await page.setViewportSize({ width: 1280, height: 860 });
  };
  await line('deploys').scrollIntoViewIfNeeded();
  await held('.credentials-panel', 'the credentials', 0);

  // Revoke… asks first: the key made with it and what each applied, stopped.
  await line('deploys').getByRole('button', { name: 'Revoke…' }).click();
  const question = page.locator('.key-revoke');
  await question.locator('.key-revoke-goes').waitFor();
  const goes = await question.locator('.key-revoke-goes li').allInnerTexts();
  expect(
    `the key made with it and what each applied said, got ${goes}`,
    goes.length === 3 &&
      goes[0] === 'the key nightly, which was made with it' &&
      goes.some((one) => one.includes(byFirst) && one.endsWith('stopped')) &&
      goes.some((one) => one.includes(byNested) && one.endsWith('stopped')),
  );
  expect('nothing revoked by being asked', (await whose(first.token)) === 200);
  await held('.key-revoke', 'the question', 1);
  await question.getByRole('button', { name: 'Revoke it' }).click();
  await page.getByText('deploys is revoked with one key made with it; 2 things it applied stopped.').waitFor();
  await line('deploys').waitFor({ state: 'detached' });
  await line('nightly').waitFor({ state: 'detached' });
  expect('both keys nobody\'s', (await whose(first.token)) === 401 && (await whose(nested.token)) === 401);
  let still = await alive();
  expect(`what they applied stopped and nothing else, got ${still}`, !still.includes(byFirst) && !still.includes(byNested) && still.includes(byLeft) && still.includes(byRolled));

  // Told to leave it, what the key applied goes on.
  await line('reports').getByRole('button', { name: 'Revoke…' }).click();
  await question.locator('.key-revoke-goes').waitFor();
  await question.getByLabel('Leave what it applied running, with what the key held').check();
  await question.locator('li', { hasText: 'left running' }).waitFor();
  await question.getByRole('button', { name: 'Revoke it' }).click();
  await page.getByText('reports is revoked; one thing it applied still running.').waitFor();
  expect('the key nobody\'s', (await whose(left.token)) === 401);
  still = await alive();
  expect('what it applied left running', still.includes(byLeft));

  // Rolled: its new secret shown once, the old one refused, the same key.
  await line('rotated').getByRole('button', { name: 'Roll…' }).click();
  const roll = page.locator('.key-roll');
  await roll.getByText('It is given another secret').waitFor();
  await held('.key-roll', 'the question of a roll', 2);
  await roll.getByRole('button', { name: 'Roll it' }).click();
  const shown = (await roll.locator('.code-box').innerText()).trim();
  expect(`a new secret shown, got ${shown}`, shown.startsWith('key_') && shown !== rolled.token);
  await held('.key-roll', 'the new secret', 3);
  expect('the old secret refused and the new one taken', (await whose(rolled.token)) === 401 && (await whose(shown)) === 200);
  expect('what it applied going on', (await alive()).includes(byRolled));
  await roll.getByRole('button', { name: 'Done' }).click();
  await line('rotated').waitFor();

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
