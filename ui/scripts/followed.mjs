// How somebody arranged the side panel, kept with their account and so
// following them to every window and browser.
//
//   DAEMON=../target/debug/factory node scripts/followed.mjs [out-dir]
//
// One person in three browsers that keep nothing of one another's (as the
// desktop window and a browser are), and a fourth from before. In the
// fourth, what a browser kept before arrangements were the account's (a
// group, a channel pinned and a board pinned in Resources) is taken up
// once and taken out of the browser, what was lately opened left where it
// was. In the first a group is made, a channel put in it and another
// pinned, and a project pinned from its row; the second, open all the
// while and never read again, shows each as it is done; and the third,
// signed in fresh, has all of it. Nothing of what is folded follows: every
// branch starts folded in each browser, and is unfolded there by Expand all. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
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
const data = join(out, 'followed-data');
const home = join(out, 'followed-home');

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
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  for (const name of ['general', 'design', 'ops', 'random']) await cli('channel', 'create', name);
  await cli('project', 'create', 'Shop', '--bare');
  await cli('apply', 'std/board', '--project', 'shop');
  const ids = Object.fromEntries(JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.map((one) => [one.name, one.id]));
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
  const listed = async (read, wanted, what) => {
    const until = Date.now() + 8000;
    let got = await read();
    while (got !== wanted && Date.now() < until) {
      await new Promise((later) => setTimeout(later, 60));
      got = await read();
    }
    expect(`${what}: "${wanted}", got "${got}"`, got === wanted);
  };

  /** A browser of its own, which keeps nothing another keeps, signed in as Ada. */
  const window = async (before) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    if (before) await page.evaluate(before);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor();
    const side = async (name) => {
      const button = page.locator('.rail').getByRole('button', { name, exact: true });
      if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
      await page.getByRole('navigation', { name }).waitFor();
    };
    const channels = page.getByRole('navigation', { name: 'Channels' });
    const resources = page.getByRole('navigation', { name: 'Resources' });
    // What the channels' list says, from the top: those pinned, the channels in no group, then each group.
    const grouped = async () => {
      await side('Channels');
      return channels.evaluate((nav) => {
        const names = (within) => [...within.querySelectorAll('.grouped-channel > div > .nav-row .nav-item .truncate')].map((one) => one.textContent);
        return [...nav.querySelectorAll('.grouped, .grouped-loose')]
          .map((part) => (part.classList.contains('grouped-loose') ? names(part).join(' ') : `${part.querySelector('.group-name .truncate').textContent}[${names(part).join(' ')}]`))
          .join(' | ');
      });
    };
    // What Resources lists as pinned.
    const pinned = async () => {
      await side('Resources');
      // Everything the Resources panel lists starts folded (`folded.mjs`): Pinned is unfolded where it is drawn.
      const shut = resources.getByRole('button', { name: 'Unfold Pinned', exact: true });
      if (await shut.count()) await shut.click();
      return resources.evaluate((nav) => {
        const head = [...nav.querySelectorAll('.nav > h3')].find((one) => one.textContent.trim() === 'Pinned');
        return head ? [...head.nextElementSibling.querySelectorAll('.lately-line .truncate')].map((one) => one.textContent).join(' | ') : '(not drawn)';
      });
    };
    // Every branch of the channels starts folded, and what is folded is each browser's (`folding.mjs`): a person unfolds them.
    const expand = async () => {
      await side('Channels');
      await channels.getByRole('button', { name: 'Expand all', exact: true }).click();
    };
    const kept = () => page.evaluate(() => ({ ...localStorage }));
    const there = () =>
      page.evaluate(async () => {
        const asked = await fetch('/api/arranged', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
        return (await asked.json()).arrangement;
      });
    return { context, page, side, channels, resources, grouped, pinned, kept, there, expand };
  };
  const choose = async (page, label) => {
    const menu = page.getByRole('menu');
    await menu.waitFor();
    await menu.getByText(label, { exact: true }).click();
    await menu.waitFor({ state: 'detached' });
  };
  const said = (page) => page.waitForResponse((answer) => answer.url().endsWith('/api/arranged') && answer.request().method() === 'PUT');

  // ---- what a browser kept before is taken up once, and taken out of it
  const older = await window(null);
  const actor = await older.page.evaluate(async () => {
    const asked = await fetch('/api/me', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return (await asked.json()).actor;
  });
  const old = {
    [`client.channel.groups:${actor}`]: JSON.stringify({ groups: [{ id: 'g1', name: 'Before' }], of: { [ids.general]: 'g1' }, pinned: [ids.design] }),
    [`client.side.lately:${actor}`]: JSON.stringify({ recent: [{ kind: 'project', id: 'shop' }], pinned: [{ kind: 'board', project: 'shop' }] }),
  };
  expect('nothing to be kept with the account yet', (await older.there()) === null);
  await older.page.evaluate((seeded) => {
    for (const [key, value] of Object.entries(seeded)) localStorage.setItem(key, value);
  }, old);
  await older.page.reload();
  await older.page.locator('.rail').waitFor();
  await listed(older.grouped, 'Pinned[] | ops random | Before[]', 'what a browser kept before to be drawn, its branches folded');
  await older.expand();
  await listed(older.grouped, 'Pinned[design] | ops random | Before[general]', 'what a browser kept before to be drawn');
  await listed(older.pinned, 'Shop board', 'and what it pinned in Resources');
  await listed(async () => JSON.stringify(await older.there()), JSON.stringify({ groups: [{ channels: [ids.general], id: 'g1', name: 'Before' }], pinned: [ids.design], things: ['/board/shop'] }), 'it to be kept with the account');
  await listed(async () => String(`client.channel.groups:${actor}` in (await older.kept())), 'false', 'the groups a browser kept before to be taken out of it');
  const lately = JSON.parse((await older.kept())[`client.side.lately:${actor}`] ?? '{}');
  expect(`what was lately opened to stay in the browser and what was pinned to go from it, got ${JSON.stringify(lately)}`, !lately.pinned && JSON.stringify(lately.recent).includes('"id":"shop"'));
  // Read again, nothing is taken up twice: what the account keeps is what is drawn.
  await older.page.reload();
  await older.page.locator('.rail').waitFor();
  await listed(older.grouped, 'Pinned[design] | ops random | Before[general]', 'the same once the page is read again');
  await older.context.close();

  // ---- two windows: what is done in one is shown in the other, which is never read again
  const first = await window(null);
  const second = await window(null);
  await listed(second.grouped, 'Pinned[] | ops random | Before[]', 'a fresh browser to have what the account keeps, folded');
  await second.expand();
  await listed(second.grouped, 'Pinned[design] | ops random | Before[general]', 'a fresh browser to have what the account keeps');
  await listed(first.grouped, 'Pinned[] | ops random | Before[]', 'and the other');
  await first.expand();
  await first.page.getByRole('button', { name: 'New group' }).click();
  await first.page.getByLabel('Name').fill('Shop work');
  await Promise.all([said(first.page), first.page.getByRole('button', { name: 'Create', exact: true }).click()]);
  await first.page.getByRole('dialog').waitFor({ state: 'detached' });
  const row = (at, name) => at.channels.locator('.grouped-channel', { has: at.page.getByRole('button', { name: `# ${name}`, exact: true }) });
  await row(first, 'design').click({ button: 'right' });
  await Promise.all([said(first.page), choose(first.page, 'Shop work')]);
  await row(first, 'ops').click({ button: 'right' });
  await Promise.all([said(first.page), choose(first.page, 'Pin to the top')]);
  const now = 'Pinned[ops] | random | Before[general] | Shop work[design]';
  await listed(first.grouped, now, 'the group made and the channels put where they go');
  // What is folded is each browser's: the group made unfolded in the window that made it is folded in the other, as
  // every branch starts, and a group folded in one window is folded there alone.
  await listed(second.grouped, 'Pinned[ops] | random | Before[general] | Shop work[]', 'the second window to show them with nothing read again, the new group folded');
  await second.expand();
  await listed(second.grouped, now, 'the second window to show what was put in the group');
  await first.page.getByRole('button', { name: 'Fold Shop work' }).click();
  await listed(second.grouped, now, 'a fold not to follow');
  await first.page.getByRole('button', { name: 'Unfold Shop work' }).click();
  await second.side('Channels');
  await second.page.screenshot({ path: `${out}/followed-1-second-window.png` });
  // A project pinned from its row in Resources.
  await first.side('Resources');
  await unfoldAll(first.page);
  const tree = first.resources.locator('[data-project="shop"] > .nav-row');
  await tree.hover();
  await tree.getByRole('button', { name: 'More for Shop', exact: true }).click();
  await Promise.all([said(first.page), choose(first.page, 'Pin to the top')]);
  await first.page.mouse.move(980, 700);
  await listed(first.pinned, 'Shop board | Shop', 'a project pinned from its row');
  await listed(second.pinned, 'Shop board | Shop', 'and pinned in the second window with nothing read again');
  await first.side('Channels');
  await first.page.screenshot({ path: `${out}/followed-0-first-window.png` });
  await second.page.screenshot({ path: `${out}/followed-2-second-pinned.png` });
  expect('the second page never to have been read again', (await second.page.evaluate(() => performance.getEntriesByType('navigation').length)) === 1);

  // ---- a browser signed in fresh has all of it
  const third = await window(null);
  await listed(third.grouped, 'Pinned[] | random | Before[] | Shop work[]', 'a fresh browser to have the groups and pins, folded');
  await third.expand();
  await listed(third.grouped, now, 'a fresh browser to have the groups and pins');
  await listed(third.pinned, 'Shop board | Shop', 'and what is pinned in Resources');
  await third.page.screenshot({ path: `${out}/followed-3-fresh.png` });

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
