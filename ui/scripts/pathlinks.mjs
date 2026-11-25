// A file of a project named in what somebody wrote, as a link that opens
// it (`src/pathlinked.ts`, `src/pathsknown.ts`,
// `views/pathlink.tsx`).
//
//   DAEMON=../target/debug/factory node scripts/pathlinks.mjs [out-dir]
//
// Ada has a project, Shop, whose folder holds a plan and a README, with a
// channel of its own and a board, and a channel of no project. From a
// terminal she says, in Shop's channel, a path in backticks and one bare,
// one that names nothing, and the path again in a block of code; on a
// card, a comment that names the plan in backticks and bare, and one
// that names nothing; and in the channel of no project, the plan named
// by its project before it, and the README bare, which names nothing
// there. Each file named is a link on its own words, and each that
// names nothing is words; pressed, a link opens the file as a tab, with
// what it holds shown. Nothing is read of the project's files to find
// out but what the page asks of each path, once, and a message that
// names many paths has at most three asked. Each link is inside its
// message and nothing scrolls sideways, at three widths and a phone's.
// Run plainly and with `LOOK="size=largest density=compact"`. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { channelsOf, unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'pathlinks-data');
const home = join(out, 'pathlinks-home');
const shop = join(out, 'pathlinks-shop');
const looked = process.env.LOOK ? '-looked' : '';

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
await mkdir(join(shop, 'docs', 'plans'), { recursive: true });
await mkdir(home, { recursive: true });
await writeFile(join(shop, 'docs', 'plans', 'x.md'), '# The plan\n\nThe plan itself, as it is written.\n');
await writeFile(join(shop, 'README.md'), '# Shop\n\nWhat the shop is.\n');
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const SAID = 'The plan is `docs/plans/x.md`, beside README.md and @blueprints/watch.bp; docs/plans/missing.md is not there yet.\n```\ncat docs/plans/x.md\n```';
const MANY = 'Five at once: @blueprints/a.bp @blueprints/b.bp @blueprints/c.bp @blueprints/d.bp @blueprints/e.bp';
const COMMENTED = 'Read `docs/plans/x.md` first, then docs/plans/x.md again; nothing/here.md is nothing.';
const ELSEWHERE = 'Over there: shop:docs/plans/x.md, and README.md, which is of no project here.';

let failed = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  // A blueprint of the project's own, which the factory keeps in its data folder and lists in no list of the project's files.
  await mkdir(join(data, 'blueprints', 'projects', 'shop'), { recursive: true });
  await writeFile(join(data, 'blueprints', 'projects', 'shop', 'watch.bp'), 'blueprint: watch\nsteps:\n  noted: { do: noop }\n');
  await cli('channel', 'create', 'shop-talk', '--project', 'shop');
  await cli('channel', 'create', 'general');
  await cli('post', 'shop-talk', SAID);
  await cli('post', 'shop-talk', MANY);
  await cli('post', 'general', ELSEWHERE);
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('board', 'add', 'shop', 'Follow the plan');
  await cli('record', 'comment', 'shop-1', COMMENTED);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const scheme = process.env.SCHEME ?? 'dark';
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  /** Every path the page asked the daemon about, by what it asked. */
  const asked = [];
  const signedIn = async (how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    lastPage = page;
    page.on('pageerror', (err) => problems.push(err.message));
    page.on('request', (request) => {
      const at = new URL(request.url());
      if (at.pathname.endsWith('/file') && at.searchParams.has('head')) asked.push(`${at.pathname.split('/')[3]}:${at.searchParams.get('path')}`);
    });
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const at = (page, hash) => page.evaluate((to) => (location.hash = to), hash);
  const message = (page, words) => page.locator('.message', { hasText: words }).first();
  /** The links of a part of the page, each by what it says and what it opens. */
  const links = (part) => part.locator('a.path-link').evaluateAll((all) => all.map((one) => `${one.textContent}=${one.dataset.path}`));
  /** Each link of a part inside it and in the window, and nothing of the page scrolling sideways. */
  const laid = (part, when) =>
    part.evaluate((box) => {
      const own = box.getBoundingClientRect();
      const wrong = [...box.querySelectorAll('a.path-link')].flatMap((link) =>
        [...link.getClientRects()].filter((at) => at.left < own.left - 0.5 || at.right > own.right + 0.5 || at.left < -0.5 || at.right > innerWidth + 0.5).map(() => link.textContent),
      );
      return { wrong, sideways: document.documentElement.scrollWidth - innerWidth };
    }).then((now) => {
      expect(`every link to be inside its words and the window ${when}, got ${now.wrong.join(', ')}`, now.wrong.length === 0);
      expect(`nothing to scroll sideways ${when}, got ${now.sideways}`, now.sideways <= 0);
    });
  /** Presses a link and waits for the file it names to be shown with what it holds. */
  const opens = async (page, link, holds, how = 'click') => {
    await link[how]();
    await page.locator('.view', { hasText: holds }).filter({ visible: true }).first().waitFor();
  };

  // ── A project's channel ─────────────────────────────────────────────────
  const page = await signedIn();
  /** Opens Shop's channel from the side panel, every branch of which is unfolded first (a project's channels start folded). */
  const shopTalk = async (on, how = 'click') => {
    const panel = channelsOf(on);
    if (!(await panel.isVisible())) await on.locator('.rail').getByRole('button', { name: 'Channels', exact: true })[how]();
    await panel.waitFor();
    await panel.locator('.grouped-channel', { has: on.locator('.nav-item .truncate', { hasText: /^shop-talk$/ }) }).locator('.nav-item').first()[how]();
  };
  await unfoldChannels(page);
  await shopTalk(page);
  const said = message(page, 'The plan is');
  await said.locator('a.path-link').first().waitFor();
  // What the page asked the daemon of is a link once it has answered.
  await said.locator('a.path-link[data-path="@blueprints/watch.bp"]').waitFor();
  const there = await links(said);
  expect(
    `the plan in backticks, the README bare and the plan in a block of code each a link to its file, and the path that names nothing words, got ${there.join(' | ')}`,
    there.join('|') === 'docs/plans/x.md=docs/plans/x.md|README.md=README.md|@blueprints/watch.bp=@blueprints/watch.bp|docs/plans/x.md=docs/plans/x.md',
  );
  expect('the path in backticks to be drawn as code with the link on it', (await said.locator('code a.path-link').count()) === 1);
  expect('the path in a block of code to be a link inside the block', (await said.locator('.code-block a.path-link').count()) === 1);
  expect('a path that names nothing to be no link', (await said.locator('a.path-link', { hasText: 'missing' }).count()) === 0 && (await said.innerText()).includes('docs/plans/missing.md'));
  expect(`its address to be the file's, got ${await said.locator('a.path-link').first().getAttribute('href')}`, (await said.locator('a.path-link').first().getAttribute('href')) === '/shop/repo/docs/plans/x.md');
  // What the files the page read of a project say is not asked again; what is in no list of them (what the factory
  // keeps of the project's own blueprints) is asked, a few at most for one message, each path once.
  const many = asked.filter((one) => /^shop:@blueprints\/[a-e]\.bp$/.test(one));
  expect(`at most three of five paths asked for one message, got ${asked.join(', ')}`, many.length === 3);
  expect(`each path asked once, and none of the project's listed files asked, got ${asked.join(', ')}`, new Set(asked).size === asked.length && asked.includes('shop:@blueprints/watch.bp') && !asked.some((one) => /docs|README/.test(one)));
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await laid(said, `in a project's channel at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/pathlinks-1-channel${looked}.png` });
  await opens(page, said.locator('a.path-link', { hasText: 'README.md' }), 'What the shop is.');
  await page.screenshot({ path: `${out}/pathlinks-2-opened${looked}.png` });
  await shopTalk(page);
  await opens(page, message(page, 'The plan is').locator('code a.path-link'), 'The plan itself');

  // ── A channel of no project: a path names a file only where its project is said before it ──
  const panel = channelsOf(page);
  await panel.locator('.grouped-channel', { has: page.locator('.nav-item .truncate', { hasText: /^general$/ }) }).locator('.nav-item').first().click();
  const elsewhere = message(page, 'Over there');
  await elsewhere.locator('a.path-link').first().waitFor();
  const named = await links(elsewhere);
  expect(`the plan named with its project before it to be a link to it, and the README, of no project here, words, got ${named.join(' | ')}`, named.join('|') === 'shop:docs/plans/x.md=docs/plans/x.md');
  await laid(elsewhere, 'in a channel of no project');
  await page.screenshot({ path: `${out}/pathlinks-3-elsewhere${looked}.png` });

  // ── A card's comment, in its drawer ─────────────────────────────────────
  await at(page, '#/board/shop');
  await page.locator('.card', { hasText: 'Follow the plan' }).click();
  const drawer = drawerAtRest(page);
  await drawer.getByRole('tab', { name: 'Comments', exact: true }).click();
  const comment = drawer.locator('.journey-words', { hasText: 'Read' });
  await comment.locator('a.path-link').first().waitFor();
  const commented = await links(comment);
  expect(`the plan named in backticks and bare in a comment each a link, and what names nothing words, got ${commented.join(' | ')}`, commented.join('|') === 'docs/plans/x.md=docs/plans/x.md|docs/plans/x.md=docs/plans/x.md');
  await laid(comment, "in a card's comment");
  await page.screenshot({ path: `${out}/pathlinks-4-comment${looked}.png` });
  await opens(page, comment.locator('a.path-link').last(), 'The plan itself');

  // ── A phone ─────────────────────────────────────────────────────────────
  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await unfoldChannels(phone);
  await shopTalk(phone, 'tap');
  const small = message(phone, 'The plan is');
  await small.locator('a.path-link').first().waitFor();
  await laid(small, "at a phone's width");
  await phone.screenshot({ path: `${out}/pathlinks-5-phone${looked}.png` });
  await opens(phone, small.locator('a.path-link', { hasText: 'README.md' }), 'What the shop is.', 'tap');

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/pathlinks-failed${looked}.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
