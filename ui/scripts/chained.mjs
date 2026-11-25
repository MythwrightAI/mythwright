// What a person sees of a card that waits on somebody: the line in their
// inbox, and the card it opens (`std/chain`).
//
//   DAEMON=../target/debug/factory node scripts/chained.mjs [out-dir]
//
// Ada owns the factory and applied the shop's fleet, whose one worker is
// a stand-in that does what it is asked, and in which nobody reviews.
// Grace registered by an invitation. The shop's chain, which sits beside
// its board, has two rungs for a card that waits in review: Grace first,
// and two seconds on, whoever applied the fleet. A card made ready from
// a terminal is worked and comes to review, where it waits for a person.
// Ada, told by the fleet that it waits (`tells`), has one line of it in
// her box, from the factory's bot, in the fleet's words; when the second
// rung comes to her the line is raised and is still one line, its page
// (never read again) saying that it was told twice. Grace, told by the
// first rung, has her own line of it in hers, told once. Each line's page
// says what it is about, and that opens the card on its board, in
// review, with its drawer. Each page and line by its boxes at three
// widths, and Grace's at a phone's, where a line tapped opens its page
// and that the card. The environment variables are those of
// walkthrough.mjs; with `LOOK` the pages wear that look.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { tablesRead } from './gear.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'chained-data');
const home = join(out, 'chained-home');
const hers = join(out, 'chained-grace');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
// Does what it is asked, and says so.
const works = join(home, 'works.sh');
await writeFile(works, "#!/bin/sh\ncat >/dev/null\necho lamp >> LAMP.md\necho 'I put the lamp up.'\n");
await chmod(works, 0o755);
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off', '--command', `works=${works}`], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const grace = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

const WAITS = 'shop-1 is in review, where nobody in the fleet has the role reviewer: it waits for a person to review it.';

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'grace@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await grace('register', '--email', 'grace@example.com', '--name', 'Grace', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('record', 'set', 'shop-agents-2', 'adapter=works');
  await cli('board', 'add', 'shop', 'Add a lamp');
  await cli('record', 'add', 'shop', 'chain', 'title=Grace first', 'waits=review', 'step=1', 'who=@grace', 'after=2s');
  await cli('record', 'add', 'shop', 'chain', 'title=then the owner', 'waits=review', 'step=2', 'who=applier');

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
  const signedIn = async (who, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill(who);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    // On a phone the rail comes with the side panel, and is on the page meanwhile.
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const tray = (on, unread) => on.locator('.rail .rail-inbox').and(on.getByRole('button', { name: unread ? `Inbox, ${unread} unread` : 'Inbox', exact: true }));
  const waits = async (on, unread, why) => {
    await tray(on, unread)
      .first()
      .waitFor({ timeout: 60000 })
      .catch(async () => {
        throw new Error(`expected the rail's inbox to say ${unread} unread ${why}, got "${await on.locator('.rail .rail-inbox').first().getAttribute('aria-label')}"`);
      });
  };
  const boxOf = (on) => on.getByRole('navigation', { name: 'Inbox' });
  const line = (within, words) => within.locator('.inbox-line', { hasText: words });
  // The boxes of what `selector` picks inside `within`: none lies over another, and each is inside `within`.
  const apart = async (within, selector, what) => {
    const found = await within.evaluate((frame, picked) => {
      const edge = frame.getBoundingClientRect();
      const boxes = [...frame.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < edge.left - 0.5 || one.box.right > edge.right + 0.5) wrong.push(`${one.said} leaves it`);
        for (const two of boxes.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth, within: frame.scrollWidth - frame.clientWidth };
    }, selector);
    expect(`${what}: something to be there`, found.count > 1);
    expect(`${what}: nothing over anything else or outside its place, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px of the page and ${found.within}px of itself`, found.sideways <= 0 && found.within <= 0);
  };
  const LINE = '.inbox-from, .inbox-when, .inbox-words, .inbox-away';
  const PAGE = '.inbox-page-title, .inbox-page-from, .inbox-page-when, .inbox-page-times, .inbox-page-words, .inbox-page-label, .inbox-page-opens, .inbox-page-acts button';
  const laidOut = async (on, what) => {
    for (const one of await boxOf(on).locator('.inbox-line').all()) await apart(one, LINE, `a line of the box ${what}`);
    if ((await on.locator('.inbox-page').count()) > 0) await apart(on.locator('.inbox-page'), PAGE, `the page of a line ${what}`);
  };
  const atWidths = async (on, what) => {
    for (const width of [1280, 1000, 820]) {
      await on.setViewportSize({ width, height: 860 });
      await laidOut(on, `${what} at ${width} wide`);
    }
    await on.setViewportSize({ width: 1280, height: 860 });
  };
  const pageOf = async (on, words) => {
    const shown = on.locator('.inbox-page');
    await shown.locator('.inbox-page-words', { hasText: words }).waitFor();
    return shown;
  };
  // What a line's page is about opens the card on its board, in review, with its drawer.
  const opensTheCard = async (on, shown, press, what) => {
    const about = shown.locator('.inbox-page-about .inbox-page-opens');
    const says = await about.innerText();
    expect(`what ${what} is about to say the card, got "${says}"`, says.includes('Add a lamp') && says.includes('card shop-1 on the Shop board'));
    await press(about);
    const drawer = drawerAtRest(on);
    await drawer.getByLabel('Title').waitFor();
    const title = await drawer.getByLabel('Title').inputValue();
    expect(`the drawer ${what} opened to be the card's, got "${title}"`, title === 'Add a lamp');
    const column = await on.locator('.card[data-card="shop-1"]').first().evaluate((card) => card.closest('.column')?.querySelector('.column-name span')?.textContent ?? '');
    expect(`the card ${what} opened to be in review on its board, got "${column}"`, column === 'review');
    return drawer;
  };

  // Ada's page is opened once, and is never read again from here to the end.
  const ada = await signedIn('ada@example.com');
  let loads = 0;
  ada.on('load', () => (loads += 1));
  await waits(ada, 0, 'with nothing told');
  // The chain sits beside the board, with its two rows.
  await ada.getByRole('button', { name: 'Resources', exact: true }).click();
  await unfoldAll(ada);
  await ada.locator('.nav').getByText('Shop', { exact: true }).click();
  await ada.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }).click();
  await ada.locator('.column', { hasText: 'ready' }).first().waitFor();
  const tables = await tablesRead(ada);
  expect(`the gear of the board's head to list the chain among its tables with its two rows, got ${tables.join(' | ')}`, tables.some((one) => one.startsWith('Chain 2')));
  await ada.keyboard.press('Escape');

  // ── A card comes to wait in review: Ada is told by the fleet, Grace by the first rung ──
  await cli('board', 'move', 'shop-1', 'ready');
  await waits(ada, 1, 'once the card waited in review, on a page that was not read again');
  await tray(ada, 1).first().click();
  const box = boxOf(ada);
  const told = line(box, WAITS);
  await told.waitFor();
  expect(`Ada's line to be what a blueprint told, got ${await told.getAttribute('data-kind')}`, (await told.getAttribute('data-kind')) === 'told');
  expect(`Ada's line to be from the factory's bot, got "${await told.locator('.inbox-from').innerText()}"`, (await told.locator('.inbox-from').innerText()) === 'conductor');
  expect(`Ada's line to say the card waits, got "${await told.locator('.inbox-words').innerText()}"`, (await told.locator('.inbox-words').innerText()) === WAITS);
  await ada.screenshot({ path: `${out}/chained-1-ada-box.png` });
  await told.locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  await ada.mouse.move(760, 520);
  const hers1 = await pageOf(ada, WAITS);
  // The second rung comes to her: the line she has is raised, and is one line still.
  await hers1.locator('.inbox-page-times', { hasText: 'Told 2 times, first' }).waitFor({ timeout: 60000 });
  await waits(ada, 1, 'once the second rung raised the line she had read');
  expect(`one line of it in Ada's box, got ${await line(box, WAITS).count()}`, (await line(box, WAITS).count()) === 1);
  await atWidths(ada, "on Ada's page");
  await ada.screenshot({ path: `${out}/chained-2-ada-page.png` });
  const drawer = await opensTheCard(ada, hers1, (about) => about.click(), "Ada's line");
  await ada.screenshot({ path: `${out}/chained-3-ada-card.png` });
  await ada.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });
  expect(`Ada's page never to have been read again, was ${loads} times`, loads === 0);

  // ── Grace's line, from the first rung, told once ──
  const theirs = await signedIn('grace@example.com');
  await waits(theirs, 1, "with the chain's first rung told to her");
  await tray(theirs, 1).first().click();
  const graceBox = boxOf(theirs);
  const rung = line(graceBox, WAITS);
  await rung.waitFor();
  expect(`one line of it in Grace's box, got ${await rung.count()}`, (await rung.count()) === 1);
  expect(`Grace's line to be from the factory's bot, got "${await rung.locator('.inbox-from').innerText()}"`, (await rung.locator('.inbox-from').innerText()) === 'conductor');
  await rung.locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  await theirs.mouse.move(760, 520);
  const graces = await pageOf(theirs, WAITS);
  expect("Grace's line to say it was told once", (await graces.locator('.inbox-page-times').count()) === 0);
  await atWidths(theirs, "on Grace's page");
  await theirs.screenshot({ path: `${out}/chained-4-grace-page.png` });
  await opensTheCard(theirs, graces, (about) => about.click(), "Grace's line");
  await theirs.screenshot({ path: `${out}/chained-5-grace-card.png` });

  // ── On a phone: the box has the screen, a line tapped opens its page, and that the card ──
  const phone = await signedIn('grace@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await phone.locator('.rail').getByRole('button', { name: 'Inbox', exact: true }).tap();
  const small = line(boxOf(phone), WAITS);
  await small.waitFor();
  await apart(small, LINE, 'a line of the box on a phone');
  await small.locator('.inbox-press').tap({ position: { x: 24, y: 10 } });
  const onPhone = await pageOf(phone, WAITS);
  await apart(onPhone, PAGE, 'the page of a line on a phone');
  await phone.screenshot({ path: `${out}/chained-6-phone-page.png` });
  await opensTheCard(phone, onPhone, (about) => about.tap(), "Grace's line on a phone");
  await phone.screenshot({ path: `${out}/chained-7-phone-card.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
