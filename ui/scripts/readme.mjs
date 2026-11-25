// The pictures of the README's "Blueprints, by example", made again from
// the README itself: each blueprint is read out of the README's blocks as
// it is written there, put among a project's own and applied, what the
// README says is done is done from a terminal, and what came of it is
// pictured, at 1280 by 800 in the dark:
//
//   - a board of one's own, its columns those the blueprint gave it, with
//     cards in them, beside the projects (`board.png`);
//   - an event of one's own said from outside by a key that may say that
//     topic and is refused another, a card put on the board for each order
//     paid (`event.png`, the side panel put away for the board's columns);
//   - the project's channel: what the orders said there, and the factory's
//     bot asked to add a card, move one and show the board, each answered
//     before the next is asked (`conductor.png`);
//   - the rule that keeps time on its blueprint's page, its Summary in
//     plain words with what it does as it hears unfolded (`summary.png`).
//
//   DAEMON=../target/debug/factory node scripts/readme.mjs [out-dir]
//   WRITE_README=1 DAEMON=../target/debug/factory node scripts/readme.mjs [out-dir]
//
// The pictures go to `out-dir`, which also keeps the daemon's data while
// it runs, so that a walk leaves the checkout as it was. Told
// `WRITE_README=1` they go to `docs/readme/` instead, which is how the
// README's own pictures are made again. Each is checked to be under
// 200 KB.

import { execFile, spawn } from 'node:child_process';
import { mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const pictures = process.env.WRITE_README === '1' ? join(repo, 'docs', 'readme') : out;
const data = join(out, 'readme-data');
const home = join(out, 'readme-home');

const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};

/** The blueprints the README writes out, by the name each says it is. */
async function written() {
  const text = await readFile(join(repo, 'README.md'), 'utf8');
  const found = new Map();
  for (const [, block] of text.matchAll(/```yaml\n([\s\S]*?)```/g)) {
    const name = /^blueprint: (\S+)/.exec(block)?.[1];
    if (name) found.set(name, block);
  }
  return found;
}

await mkdir(out, { recursive: true });
await mkdir(pictures, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--walls', 'off'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = async (...args) => (await run(binary, [...args, '--url', url], { cwd: repo, env })).stdout;
const json = async (...args) => JSON.parse(await cli(...args, '--json'));

/** Waits until `holds` says yes of what `ask` reads, a few seconds at most. */
async function until(what, ask, holds) {
  const by = Date.now() + 15_000;
  for (;;) {
    const read = await ask();
    if (holds(read)) return read;
    if (Date.now() > by) throw new Error(`expected ${what}, read ${JSON.stringify(read).slice(0, 400)}`);
    await new Promise((later) => setTimeout(later, 200));
  }
}

let failed = null;
let page = null;
try {
  const blueprints = await written();
  for (const name of ['writing', 'orders', 'nudge']) expect(`the README to write out \`${name}\``, blueprints.has(name));
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');

  // A board of one's own: a project with nothing, given the README's board.
  await cli('project', 'create', 'blog', '--bare');
  const put = async (project, name) => {
    const file = join(out, `readme-${name}.bp`);
    await writeFile(file, blueprints.get(name));
    await cli('file', 'write', project, `@blueprints/${name}.bp`, '--from', file);
    await cli('apply', name, '--project', project);
  };
  await put('blog', 'writing');
  const cards = [
    ['Why we write in plain words', 'ideas'],
    ['A tour of the new site', 'ideas'],
    ['A post about the launch', 'drafting'],
    ['Notes from the meetup', 'editing'],
    ['Hello, world', 'published'],
  ];
  for (const [title, column] of cards) await cli('board', 'add', 'blog', title, '--status', column);

  // An event of one's own, and the factory's bot, in a project with a fleet.
  await cli('project', 'create', 'shop');
  await put('shop', 'orders');
  await put('shop', 'nudge');
  const key = (await cli('key', 'payments', '--scope', 'emit:shop.paid')).split('\n')[0].trim();
  for (const order of ['o-17', 'o-18']) {
    const said = await fetch(`${url}/api/emit/shop.paid`, {
      method: 'POST',
      headers: { authorization: `Bearer ${key}`, 'content-type': 'application/json' },
      body: JSON.stringify({ order }),
    });
    expect(`the key to say shop.paid, got ${said.status}`, said.ok);
  }
  const refused = await fetch(`${url}/api/emit/shop.refunded`, {
    method: 'POST',
    headers: { authorization: `Bearer ${key}`, 'content-type': 'application/json' },
    body: JSON.stringify({ order: 'o-17' }),
  });
  expect(`the key to say no other topic, got ${refused.status}`, refused.status === 403);
  await until('a card for each order paid', () => cli('board', 'shop'), (board) => board.includes('Pack order o-18'));
  const shop = (await json('channel', 'list')).channels.find((channel) => channel.name === 'shop');
  expect('the project to have its channel', shop);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, colorScheme: 'dark' });
  page = await context.newPage();
  page.setDefaultTimeout(20_000);
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(`${url}/blog/board`);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();

  /** Pictures the page as it is, into `docs/readme/<name>.png`, and holds it to its size. */
  const picture = async (name, height = 800) => {
    // Nothing is to be on its way (the side panel, a drawer or a card sliding), and no tip under the pointer.
    await page.mouse.move(1270, 790);
    await sideSettled(page);
    await page.waitForTimeout(400);
    const path = join(pictures, `${name}.png`);
    // Cut at `height`, where what is under it is only the board's empty columns.
    await page.screenshot({ path, clip: { x: 0, y: 0, width: 1280, height } });
    const { size } = await stat(path);
    expect(`${name}.png to be under 200 KB, it is ${Math.round(size / 1024)} KB`, size < 200 * 1024);
    console.log(`${name}.png: ${Math.round(size / 1024)} KB`);
  };
  /** Turns the side panel to a place of the rail, unless it shows it already. */
  const side = async (name) => {
    const button = page.getByRole('button', { name, exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  };

  // The board, in the columns its blueprint gave it, beside the projects.
  await page.getByText('Notes from the meetup', { exact: true }).first().waitFor();
  for (const column of ['ideas', 'drafting', 'editing', 'published']) await page.getByText(column, { exact: true }).first().waitFor();
  await side('Resources');
  await picture('board', 480);

  // The shop's board, a card for each order paid; the side panel put away for its five columns.
  await page.goto(`${url}/shop/board`);
  await page.getByText('Pack order o-18', { exact: true }).first().waitFor();
  await page.getByRole('button', { name: 'Collapse the side panel' }).first().click();
  await picture('event', 400);
  await page.getByRole('button', { name: 'Expand the side panel' }).first().click();

  // The bot asked three things from the channel, each answered before the next is asked.
  const asked = ['@conductor board add Order more tape --label supplies', '@conductor board move shop-1 doing', '@conductor board'];
  for (const words of asked) {
    const before = (await json('read', 'shop', '--limit', '200')).messages.length;
    await cli('post', 'shop', words);
    await until(`the bot to answer \`${words}\``, () => json('read', 'shop', '--limit', '200'), (said) => said.messages.length >= before + 2);
  }
  await page.goto(`${url}/channels/${shop.id}`);
  await page.locator('.messages').getByText('Order o-18 is paid', { exact: false }).first().waitFor();
  await page.locator('.messages').getByText('review: nothing', { exact: false }).last().waitFor();
  await side('Channels');
  await picture('conductor');

  // A blueprint's page, said in plain words, opened by its name under the project.
  await page.goto(`${url}/shop/blueprints/nudge`);
  const summary = page.getByRole('tab', { name: 'Summary' }).last();
  await summary.waitFor();
  await summary.click();
  await page.getByText('What it hears, and what it then does').first().waitFor();
  // The project's own, as its file, whose address is its name.
  await page.locator('.crumbs').locator('visible=true').getByRole('heading', { name: 'nudge.bp' }).waitFor();
  await page.waitForFunction((to) => location.href === to, `${url}/shop/blueprints/nudge`);
  // What it does as it hears, unfolded.
  await page.locator('.summary-card .summary-unfolds').first().click();
  await page.locator('.summary-card .summary-step').first().waitFor();
  await side('Resources');
  await picture('summary');

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  // What the page showed as it failed, beside the daemon's data.
  if (page) await page.screenshot({ path: join(out, 'readme-failed.png') }).catch(() => undefined);
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${pictures}`);
