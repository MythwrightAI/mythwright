// Whom a card is assigned to, said the same wherever a card's assignee is
// drawn (`assignedTo` in `src/assigned.ts`, drawn by `views/assignee.tsx`):
// an agent the factory does not start, one it starts, the reader, another
// person by her name and a person nothing names, never by an account's id;
// on the board's cards, in the order by what comes first, on the project's
// page, and in the tools panel beside a channel (its rows and the card
// read there). Each by what the page shows and by the
// boxes at three widths and a phone's: nothing of it outside what holds it,
// a row's on one line, and nothing scrolling sideways.
//
//   DAEMON=../target/debug/factory node scripts/assigned.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'assigned-data');
const home = join(out, 'assigned-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};
const WIDTHS = [
  [1280, 860, 'wide'],
  [900, 700, 'middling'],
  [640, 720, 'narrow'],
  [390, 780, 'phone'],
];

let failed = null;
let browser = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  // Its board as a project is given one, with the order beside it.
  await cli('project', 'create', 'Shop', '--without', 'fleet', '--without', 'docs');
  await cli('channel', 'create', 'yard', '--project', 'shop');
  await cli('agent', 'add', 'helper', '--project', 'shop', '--channel', 'yard');
  // An agent the factory does not start: invited, which is enough for it to be one.
  await cli('agent', 'invite', 'walls', '--project', 'shop');
  const add = async (title) => JSON.parse((await cli('board', 'add', 'shop', title, '--json')).stdout).id;
  const ada = (await cli('whoami')).stdout.match(/\((user:[^)]+)\)/)?.[1];
  expect('whoever signed in to say who they are', Boolean(ada));

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  page.setDefaultTimeout(20_000);
  // The order is not offered for now (`orderOffered` in `src/table.ts`): this browser is told that it is, so that
  // a card's face there is held too.
  await page.addInitScript(() => {
    try {
      localStorage.setItem('client.order', 'on');
    } catch {
      /* a browser that keeps nothing is not offered it, and this check then fails where it looks for the tab */
    }
  });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  // Grace's actor, as the page would read it.
  const people = await page.evaluate(async () => (await (await fetch('/api/people', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).people);
  const grace = people.find((one) => one.name === 'Grace')?.actor;
  expect(`Grace to be among the people, got ${JSON.stringify(people)}`, Boolean(grace));

  // Each card given to somebody of each kind, the first two in hand.
  const cards = [
    ['Hang the gate', 'walls', 'walls (external)'],
    ['Oil the latch', 'helper', 'helper (agent)'],
    ['Paint the door', ada, 'Ada (you)'],
    ['Sweep the yard', grace, 'Grace (person)'],
    ['Fix the roof', 'user:acct_nobody_here', 'someone (person)'],
  ];
  for (const [title, who] of cards) await cli('board', 'give', await add(title), who);
  const ids = JSON.parse((await cli('board', 'shop', '--json')).stdout).items;
  const idOf = (title) => ids.find((one) => one.title === title).id;
  await cli('board', 'move', idOf('Hang the gate'), 'doing');
  await cli('board', 'move', idOf('Oil the latch'), 'doing');

  const said = (text) => text.replace(/\s+/g, ' ').trim();
  const until = async (what, ask) => {
    let last;
    for (let tries = 0; tries < 300; tries += 1) {
      last = await ask();
      if (last === true) return;
      await new Promise((later) => setTimeout(later, 100));
    }
    throw new Error(`expected ${what}, and it is ${JSON.stringify(last)}`);
  };
  /** What each card in `where` says of whom it is assigned to, by its title: read as it stands, nothing waited for. */
  const sayings = (where, row) =>
    where.evaluate(
      (at, row) =>
        Object.fromEntries(
          [...at.querySelectorAll(row)].map((one) => [one.textContent, one.querySelector('.assigned')?.textContent.replace(/\s+/g, ' ').trim() ?? '']),
        ),
      row,
    );
  /** That each card, found in `where` by its title, says whom it is assigned to as `prefix` and the words of its kind. */
  const saysEach = async (where, row, prefix) => {
    await until(`each card in ${row} to say whom it is assigned to`, async () => {
      const read = await sayings(where, row);
      const wrong = cards.filter(([title, , words]) => {
        const said = Object.entries(read).find(([text]) => text.includes(title))?.[1];
        return said !== `${prefix}${words}`.replace(/\s+/g, ' ');
      });
      return wrong.length === 0 || read;
    });
    const all = said(await where.innerText());
    expect(`no account's id where a person is said, got "${all}"`, !/acct_|user:/.test(all));
  };
  /** Nothing of whom a card is assigned to outside what holds it, a row's on one line, and nothing scrolling sideways. */
  const sound = async (what) => {
    const wrong = await page.evaluate(() => {
      const said = [];
      for (const one of document.querySelectorAll('.assigned')) {
        const holder = one.closest('.card, .glance-card, .context-row, .field, .drawer');
        if (!holder || one.getClientRects().length === 0) continue;
        const box = holder.getBoundingClientRect();
        for (const part of one.children)
          for (const piece of part.getClientRects())
            if (piece.left < box.left - 0.5 || piece.right > box.right + 0.5 || piece.top < box.top - 0.5 || piece.bottom > box.bottom + 0.5)
              said.push(`"${part.textContent}" leaves its ${holder.className}`);
        const lines = one.getBoundingClientRect().height / parseFloat(getComputedStyle(one).lineHeight);
        if (one.classList.contains('assigned-brief') ? lines > 1.5 : lines > 2.5) said.push(`"${one.textContent}" is ${lines.toFixed(1)} lines`);
        const column = holder.closest('.column');
        if (column && (box.left < column.getBoundingClientRect().left - 0.5 || box.right > column.getBoundingClientRect().right + 0.5)) said.push(`a card is wider than its column`);
      }
      const beside = document.documentElement.scrollWidth - window.innerWidth;
      if (beside > 0) said.push(`the page scrolls ${beside} sideways`);
      return said;
    });
    expect(`whom each card is assigned to whole and inside what holds it ${what}, and ${wrong.join('; ')}`, wrong.length === 0);
  };
  const atWidths = async (what, shot) => {
    for (const [width, height, called] of WIDTHS) {
      await page.setViewportSize({ width, height });
      await sound(`${what} at ${width} wide`);
      await page.screenshot({ path: `${out}/assigned-${shot}-${called}.png` });
    }
    await page.setViewportSize({ width: 1280, height: 860 });
  };

  // ---- the board's cards, and the order by what comes first ----------------------------------
  await page.evaluate(() => (location.hash = '/board/shop'));
  await page.getByRole('heading', { name: 'Shop board', level: 1 }).waitFor();
  await saysEach(page.locator('body'), '.card', 'Assigned to ');
  await atWidths('on the board', '1-board');
  const views = page.getByRole('tablist', { name: 'Views' });
  await views.getByRole('tab', { name: 'Order', exact: true }).click();
  await page.locator('.ordered-card').first().waitFor();
  await saysEach(page.locator('body'), '.ordered-card', 'Assigned to ');
  await atWidths('in the order', '2-order');
  await views.getByRole('tab', { name: 'Board', exact: true }).click();

  // ---- the project's page, its board at a glance -------------------------------------------
  await page.evaluate(() => (location.hash = '/project/shop'));
  await page.getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await page.locator('.glance-card').first().waitFor();
  await saysEach(page.locator('body'), '.glance-card', '');
  await atWidths("on the project's page", '3-project');

  // ---- beside a channel: the board's rows in the tools panel, and a card read there -------------
  await page.getByRole('button', { name: 'Channels', exact: true }).click();
  await unfoldChannels(page);
  await page.locator('.nav').getByText('yard', { exact: true }).click();
  await page.getByRole('heading', { name: '# yard' }).waitFor();
  const panel = page.getByRole('complementary', { name: 'Tools' });
  if ((await panel.count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  await panel.waitFor();
  await panel.getByRole('tab', { name: 'Board' }).click();
  await panel.locator('.context-row', { hasText: 'Hang the gate' }).waitFor();
  await saysEach(panel, '.context-row', '');
  await atWidths('in the tools panel', '4-tools');
  for (const [title, , words] of cards) {
    await panel.locator('.context-row', { hasText: title }).click();
    // The card's own form, as its drawer has it (`views/cardform.tsx`): whom it is given to is what its choice says.
    const card = panel.locator('form.card-form');
    await card.getByLabel('Title', { exact: true }).waitFor();
    expect(`the card opened there to be ${title}`, (await card.getByLabel('Title', { exact: true }).inputValue()) === title);
    const chosen = await card.getByLabel('Assigned to').evaluate((choice) => choice.options[choice.selectedIndex].textContent);
    expect(`the card read there to say it is assigned to ${words}, got "${said(chosen)}"`, said(chosen) === words);
    if (title === 'Hang the gate') await atWidths('on a card read in the tools panel', '5-tools-card');
    await panel.getByRole('button', { name: 'Back to the board' }).click();
  }
  await page.getByRole('button', { name: 'Collapse the tools panel' }).click();

  expect(`the page to have thrown nothing, and it threw ${problems.join('; ')}`, problems.length === 0);
  console.log(`whom a card is assigned to, wherever it is said: pictures in ${out}`);
} catch (error) {
  failed = error;
} finally {
  await browser?.close().catch(() => {});
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
