// Settings' Groups (`views/peoplegroups.tsx`, over `ui/src/peoplegroups.ts`):
// people and agents given what they may ask together. From the owner's
// Settings a group made, what its members may ask changed by area, Grace
// put in it and holding it at once on her page that is never read again,
// an agent put in it from a terminal and drawn with the page not read
// again, the group called something else and said so by the terminal;
// nothing of the part over anything else or scrolling sideways at three
// widths and a phone's; Grace taken out and holding it no longer; the
// group taken away behind a question; and Grace, who manages nothing,
// shown no Groups and refused by the route.
//
//   DAEMON=../target/debug/factory node scripts/peoplegroups.mjs [out-dir]
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
const data = join(out, 'peoplegroups-data');
const home = join(out, 'peoplegroups-home');

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
  await cli('agent', 'add', 'helper');
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
  const grace = `user:${(await joined.json()).account.id}`;

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
      const set = await fetch('/api/projects/shop/environments/prod/secrets/stripe', { method: 'PUT', headers: mine, body: JSON.stringify({ value: 'sk_test_grouped_check' }) });
      return set.status;
    });
  expect("a member not to set an environment's secret before she is in a group that holds it", (await sets()) === 403);

  // The owner's Settings: no group yet, and one made.
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const part = page.locator('.people-groups');
  await part.getByText('No group yet').waitFor();
  await part.getByRole('button', { name: 'New group', exact: true }).click();
  const named = part.getByLabel('What the new group is called');
  await named.fill('  ');
  expect('Make it closed while the group is called nothing', await part.getByRole('button', { name: 'Make it' }).isDisabled());
  await named.fill('Deploys');
  await part.getByRole('button', { name: 'Make it' }).click();
  const line = part.locator('.group-line[data-group="Deploys"]');
  await line.waitFor();
  await line.getByText('nobody in it yet').waitFor();
  // A second called the same whatever its capitals is refused before anything is asked.
  await part.getByRole('button', { name: 'New group', exact: true }).click();
  await part.getByLabel('What the new group is called').fill('deploys');
  await part.getByText('There is a group called Deploys already.').waitFor();
  expect('Make it closed for a name another group has', await part.getByRole('button', { name: 'Make it' }).isDisabled());
  await part.getByRole('button', { name: 'Cancel', exact: true }).click();

  // What its members may ask, by area: the environments' secrets given by the owner, who holds them.
  await line.getByRole('button', { name: /What its members may ask/ }).click();
  await line.getByText('nothing beyond what each holds').waitFor();
  await line.getByLabel('What it may do with environments').selectOption({ index: 1 });
  // Every part of an area chosen is the area, as a table of areas writes it.
  await line.locator('.group-fold').getByText('environments', { exact: true }).waitFor();
  await page.screenshot({ path: `${out}/peoplegroups-0-made.png` });

  // Grace put in it from the page, holding it from her next request.
  const add = line.getByLabel('Somebody to put in Deploys');
  const offered = await add.locator('option').allInnerTexts();
  expect(`the people and the agents offered, got ${offered}`, offered.includes('Grace (person)') && offered.includes('helper (agent)') && offered.includes('Ada (person)'));
  expect('Add closed until somebody is chosen', await line.getByRole('button', { name: 'Add', exact: true }).isDisabled());
  await add.selectOption(grace);
  await line.getByRole('button', { name: 'Add', exact: true }).click();
  await line.locator(`.group-member[data-actor="${grace}"]`).waitFor();
  await line.getByText('1 member').waitFor();
  expect('Grace to hold what the group holds at once', (await sets()) === 200);

  // The agent put in it from a terminal, drawn with the page not read again.
  await cli('group', 'add', 'Deploys', 'helper');
  await line.locator('.group-member[data-actor="agent:helper"]').waitFor();
  await line.getByText('2 members').waitFor();
  expect('whoever is in it not offered again', !(await add.locator('option').allInnerTexts()).includes('Grace (person)'));

  // Called something else in place, and said so by the terminal.
  const name = line.getByLabel('What Deploys is called');
  await name.fill('Deployers');
  await name.press('Enter');
  const renamed = part.locator('.group-line[data-group="Deployers"]');
  await renamed.waitFor();
  const listed = (await cli('group', 'list')).stdout;
  expect(`the terminal to say what it is called now, got ${listed}`, listed.includes('Deployers') && listed.includes('holding environments'));
  await part.screenshot({ path: `${out}/peoplegroups-1-members.png` });

  // Nothing of the part over anything else or scrolling sideways, at three widths and a phone's.
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await renamed.scrollIntoViewIfNeeded();
    const wrong = await pageCrowded(page, '.people-groups');
    expect(`nothing of Groups over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
    expect(`the page not to scroll sideways at ${width}`, !sideways);
    const box = await part.boundingBox();
    expect(`Groups inside the window at ${width}`, box !== null && box.x >= 0 && box.x + box.width <= width + 1);
    await part.screenshot({ path: `${out}/peoplegroups-2-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // Grace taken out: she holds it no longer.
  await renamed.getByRole('button', { name: 'Take Grace out of Deployers' }).click();
  await renamed.getByText('1 member').waitFor();
  expect('Grace not to hold it once she is out of the group', (await sets()) === 403);

  // Taken away behind a question, which says what goes.
  await renamed.getByRole('button', { name: 'Remove…' }).click();
  const question = page.getByRole('dialog');
  await question.getByText('Remove Deployers?').waitFor();
  await question.getByText(/holds what it holds no longer/).waitFor();
  await page.screenshot({ path: `${out}/peoplegroups-3-question.png` });
  await question.getByRole('button', { name: 'Remove it' }).click();
  await part.getByText('No group yet').waitFor();
  expect('the group gone from the page', (await part.locator('.group-line').count()) === 0);

  // A member manages nothing: no Groups, and refused by the route.
  await theirs.getByRole('button', { name: 'Settings', exact: true }).click();
  await theirs.getByRole('heading', { name: 'Settings', level: 1 }).waitFor();
  expect('a member not shown Groups', (await theirs.locator('.people-groups').count()) === 0);
  const refused = await theirs.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const made = await fetch('/api/groups', { method: 'POST', headers: mine, body: JSON.stringify({ name: 'mine', holds: [] }) });
    return made.status;
  });
  expect(`a member refused making a group, got ${refused}`, refused === 403);

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
