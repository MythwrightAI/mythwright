// A person given a scope beyond their role, from the owner's Settings:
// the line of People that says what each holds, a scope given to a
// member from there and held by her at once, on her page that is never
// read again; what the owner holds offered and nothing more; the owner's
// own line offering nothing; taken away again, from her and from the key
// she made with it; a member, who manages nothing, offered none of it and
// refused by the route; and nothing of the line over anything else or
// scrolling sideways at three widths and a phone's.
//
//   DAEMON=../target/debug/factory node scripts/given.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
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
const data = join(out, 'given-data');
const home = join(out, 'given-home');

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
const SCOPE = 'environments:secrets';

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('project', 'environment', 'shop', 'prod', 'add');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const opened = async () => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    return page;
  };
  const signIn = async (on, email) => {
    await on.goto(url);
    await on.getByLabel('Email').fill(email);
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  const page = await opened();
  await signIn(page, 'ada@example.com');
  const theirs = await opened();
  await signIn(theirs, 'grace@example.com');
  // What Grace's page may do, asked from it as it stands: it is never read again.
  const sets = () =>
    theirs.evaluate(async () => {
      const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
      const set = await fetch('/api/projects/shop/environments/prod/secrets/stripe', { method: 'PUT', headers: mine, body: JSON.stringify({ value: 'sk_test_given_check' }) });
      return set.status;
    });
  expect('a member not to set an environment\'s secret before she is given it', (await sets()) === 403);

  // The owner's People: each line says what that person holds.
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const line = (email) => page.locator(`.beyond[data-account="${email}"]`);
  await line('grace@example.com').getByText('Holds what a member holds').waitFor();
  await line('ada@example.com').getByText('Holds everything, as the owner').waitFor();
  expect('nothing offered for the owner, who holds everything', (await line('ada@example.com').locator('select, button').count()) === 0);
  const offer = line('grace@example.com').getByLabel('A scope to give Grace');
  const offers = await offer.locator('option').allInnerTexts();
  expect(`what Ada holds and Grace does not, and nothing she holds already, got ${offers}`, offers.includes(SCOPE) && offers.includes('admin') && offers.includes('private') && !offers.includes('board:read') && !offers.includes('shell'));
  expect('Give closed until something is chosen', await line('grace@example.com').getByRole('button', { name: 'Give', exact: true }).isDisabled());
  await page.locator('.people-panel').screenshot({ path: `${out}/given-0-offered.png` });

  // Given: said on the line, and held on her page at once.
  await offer.selectOption(SCOPE);
  await line('grace@example.com').getByRole('button', { name: 'Give', exact: true }).click();
  await line('grace@example.com').getByText('Holds what a member holds, and', { exact: true }).waitFor();
  await line('grace@example.com').locator('.beyond-given code', { hasText: SCOPE }).waitFor();
  expect('what she was given not offered again', !(await offer.locator('option').allInnerTexts()).includes(SCOPE));
  expect('the member to hold it from her next request', (await sets()) === 200);
  const key = await theirs.evaluate(async (scope) => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const made = await fetch('/api/credentials', { method: 'POST', headers: mine, body: JSON.stringify({ name: 'deploys', scopes: [scope, 'projects'] }) });
    return (await made.json()).token;
  }, SCOPE);
  await page.locator('.people-panel').screenshot({ path: `${out}/given-1-given.png` });

  // Nothing of the line over anything else or scrolling sideways, at three widths and a phone's.
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await line('grace@example.com').scrollIntoViewIfNeeded();
    const wrong = await pageCrowded(page, '.people-panel');
    expect(`nothing of People over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
    expect(`the page not to scroll sideways at ${width}`, !sideways);
    await page.locator('.people-panel').screenshot({ path: `${out}/given-2-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // Taken away: from her, and from the key she made with it.
  await line('grace@example.com').getByRole('button', { name: `Take ${SCOPE} away from Grace` }).click();
  await line('grace@example.com').getByText('Holds what a member holds', { exact: true }).waitFor();
  await page.getByText('Grace holds environments:secrets no longer, nor do one of their keys').waitFor();
  expect('the member not to hold it any more', (await sets()) === 403);
  const withKey = await fetch(`${url}/api/projects/shop/environments/prod/secrets/stripe`, {
    method: 'PUT',
    headers: { authorization: `Bearer ${key}`, 'content-type': 'application/json' },
    body: JSON.stringify({ value: 'sk_test_given_check' }),
  });
  expect(`her key not to hold it either, got ${withKey.status}`, withKey.status === 403);
  await page.locator('.people-panel').screenshot({ path: `${out}/given-3-taken.png` });

  // A member manages nothing: no People, and refused by the route.
  await theirs.getByRole('button', { name: 'Settings', exact: true }).click();
  await theirs.getByRole('heading', { name: 'Settings', level: 1 }).waitFor();
  expect('a member not shown People', (await theirs.locator('.people-panel').count()) === 0);
  const refused = await theirs.evaluate(async (scope) => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
    const me = await (await fetch('/api/me', { headers: mine })).json();
    const given = await fetch(`/api/accounts/${me.account?.id ?? me.principal?.id}/given/${scope}`, { method: 'PUT', headers: mine });
    return given.status;
  }, SCOPE);
  expect(`a member refused giving herself anything, got ${refused}`, refused === 403);

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
