// A change an agent asks for and may not make, decided by a person on the
// page of the line that tells her (`views/askedpart.tsx`, `docs/approvals.md`).
//
//   DAEMON=../target/debug/factory node scripts/asked.mjs [out-dir]
//
// Ada owns the factory; Grace is a member, who holds `shell` and not
// `admin`. A project's table of settings has a `note` that takes `admin`,
// and an agent of the project asks, from a terminal with its own
// credential, for a note to be set, saying the channel it asks from. Ada
// is told by a line of her inbox, on a page opened once and never read
// again; the line's page shows whose change of which record, why, the
// cell from what it was to what it is to be with the scope it takes and
// that she holds it, how it stands, and Approve and Decline. Grace is not
// told, and shown the ask from a terminal she may not decide it. Approved
// there, the change is the agent's, its history says Ada approved it, and
// the agent is told in the channel. A second ask whose record a terminal
// changes meanwhile reads as stale on the page with nothing read again,
// and nothing to press. A third is declined with why, and the agent is
// told why in the channel. A card, whose title an agent that may only
// read the board asks for, is changed by the board once approved, and its
// history says who asked and who approved it. And the part by its boxes at three widths and
// a phone's, nothing of it over anything else or past the page. With
// `LOOK="size=largest density=compact"` the same at that look. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'asked-data');
const home = join(out, 'asked-home');
const hers = join(out, 'asked-home-grace');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const grace = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: { ...env, HOME: hers, XDG_CONFIG_HOME: join(hers, 'config') } });

const SETTINGS = `blueprint: settings
steps:
  table:
    do: collection.ensure
    with:
      project: shop
      name: settings
      fields:
        title: { type: text }
        note: { type: text, takes: admin }
      history:
        record.updated: "{actor} changed its {changed}"
        record.asked: "{actor} asked to change it: {why}"
        record.ask.declined: "{actor} declined a change of it[: {why}]"
`;

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'grace@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await grace('register', '--email', 'grace@example.com', '--name', 'Grace', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  const file = join(home, 'settings.bp');
  await writeFile(file, SETTINGS);
  await cli('apply', file, '--project', 'shop');
  await cli('record', 'add', 'shop', 'settings', 'title=Front door');
  await cli('channel', 'create', 'talk', '--project', 'shop');
  await cli('agent', 'add', 'helper', '--project', 'shop', '--channel', 'talk');
  const token = (await readFile(join(data, 'slots', 'helper', 'token'), 'utf8')).trim();
  // The agent asks, with its own credential, saying where it asks from.
  let asks = 0;
  const ask = async (value, why) => {
    asks += 1;
    const said = await cli('ask', 'shop-settings-1', `note=${value}`, '--why', why, '--channel', 'talk', '--token', token, '--json');
    return JSON.parse(said.stdout).ask.ask;
  };
  const first = await ask('loud', 'the door is hard to hear');

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
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.setDefaultTimeout(30_000);
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.panes').waitFor();
    return page;
  };
  // The page of the newest line that tells of the agent's ask, opened from the box: each ask is a line of its own.
  const opened = async (page, how = 'click') => {
    if (how === 'tap') {
      await page.getByRole('button', { name: 'Expand the side panel' }).tap();
      // The rail and the panel slide in together on a phone, and are pressed once they are at rest (`drawer.mjs`).
      await sideSettled(page);
      await page.locator('.rail .rail-inbox').first().tap();
    } else if (!(await page.getByRole('navigation', { name: 'Inbox' }).isVisible())) await page.locator('.rail .rail-inbox').first().click();
    const box = page.getByRole('navigation', { name: 'Inbox' });
    // The list holds still while it is pointed at, so it is read with the pointer off it, and waited
    // for until it has a line for each ask there is.
    if (how === 'click') await page.mouse.move(900, 700);
    const lines = box.locator('.inbox-line', { hasText: 'helper asks to change shop-settings-1' });
    for (let tries = 0; (await lines.count()) < asks; tries += 1) {
      if (tries > 100) throw new Error(`a line for each of ${asks} asks, got ${await lines.count()}`);
      await new Promise((later) => setTimeout(later, 100));
    }
    const line = lines.first();
    await line.locator('.inbox-press')[how]({ position: { x: 24, y: 10 } });
    const shown = page.locator('.inbox-page');
    await shown.locator('.told-change').waitFor();
    return shown;
  };
  // Nothing of the part over anything else or past the page, and nothing to scroll sideways.
  const apart = async (page, what) => {
    const found = await page.evaluate(() => {
      const edge = (document.querySelector('.inbox-page-message') ?? document.querySelector('.inbox-page')).getBoundingClientRect();
      const picked = '.told-change > .inbox-page-label, .told-change-why, .told-change-cells th, .told-change-cells td, .told-change-standing, .told-change-whynot, .told-change-acts button';
      const boxes = [...document.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < edge.left - 0.5 || one.box.right > edge.right + 0.5) wrong.push(`${one.said} leaves the page`);
        for (const two of boxes.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      const page = document.querySelector('.inbox-page');
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth, within: page.scrollWidth - page.clientWidth };
    });
    expect(`the change ${what}: something to be there`, found.count > 4);
    expect(`the change ${what}: nothing over anything else or outside the page, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`the change ${what}: nothing to scroll sideways, got ${found.sideways}px and ${found.within}px`, found.sideways <= 0 && found.within <= 0);
  };
  const part = (shown, id) => shown.locator(`.told-change[data-ask="${id}"]`);
  const read = async (...args) => (await cli(...args)).stdout;
  // What a command says, once it says each of `words`: for what the daemon writes a moment after.
  const saysAtLast = async (asked, words, what) => {
    for (let tries = 0; ; tries += 1) {
      const said = await read(...asked);
      if (words.every((one) => said.includes(one))) return;
      if (tries > 100) throw new Error(`expected ${what}, got ${said}`);
      await new Promise((later) => setTimeout(later, 100));
    }
  };

  // ── Ada's page: the change, each cell with what it takes, and what she may do ──
  const page = await signedIn('ada@example.com');
  let loads = 0;
  page.on('load', () => (loads += 1));
  const shown = await opened(page);
  const one = part(shown, first);
  await one.locator('.inbox-page-label', { hasText: 'helper asks to change shop-settings-1 (Front door) of settings' }).waitFor();
  expect('why it asked, in its words', (await one.locator('.told-change-why').textContent()).includes('the door is hard to hear'));
  const cell = one.locator('.told-change-cell[data-cell="note"]');
  expect('the cell from nothing to loud', (await cell.locator('.told-change-was').textContent()).trim() === '—' && (await cell.locator('.told-change-to').textContent()).trim() === 'loud');
  expect('what it takes, which Ada holds', (await cell.locator('.told-change-scope').textContent()).trim() === 'admin');
  expect('it to wait', (await one.locator('.told-change-standing').textContent()).includes('waits for somebody'));
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await apart(page, `at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/asked-1-told.png` });

  // ── Grace may not make it: she is not told, and shown it she may not decide it ──
  const hersLines = JSON.parse((await grace('inbox', '--json')).stdout).lines;
  expect('Grace not told of a change she could not make', !hersLines.some((line) => (line.says ?? '').includes('helper asks')));
  const hersSeen = JSON.parse((await grace('asked', first, '--json')).stdout);
  expect(`Grace shown it with why she may not decide it, got ${JSON.stringify(hersSeen)}`, hersSeen.may === false && /admin/.test(hersSeen.why_not ?? ''));
  const refused = await grace('approve', first).then(() => null, (why) => String(why.stderr));
  expect(`the daemon to refuse Grace's approval, got ${refused}`, refused !== null);

  // ── Approved: made as the agent, said in its history, and the agent told where it asked ──
  await one.getByRole('button', { name: 'Approve', exact: true }).click();
  await one.locator('.told-done', { hasText: "Approved: shop-settings-1 is changed, as helper's." }).waitFor();
  await one.locator('.told-change-standing', { hasText: 'Ada approved it, and it is made.' }).waitFor();
  expect('nothing more to press once it is decided', (await one.getByRole('button', { name: /^(Approve|Decline)$/ }).count()) === 0);
  const record = JSON.parse(await read('record', 'show', 'shop-settings-1', '--json'));
  expect(`the note set, got ${JSON.stringify(record.data)}`, record.data.note === 'loud');
  // Written by what keeps histories, a moment after the change.
  await saysAtLast(['board', 'history', 'shop-settings-1'], ['helper asked to change it: the door is hard to hear', 'helper changed its note · approved by Ada'], 'its history to say the agent asked and changed it, and that Ada approved it');
  for (let tries = 0; !(await read('read', 'talk')).includes('@helper Ada approved your change of shop-settings-1'); tries += 1) {
    if (tries > 100) throw new Error(`the agent was never told in the channel: ${await read('read', 'talk')}`);
    await new Promise((later) => setTimeout(later, 100));
  }
  await page.screenshot({ path: `${out}/asked-2-approved.png` });

  // ── A second ask whose record is changed meanwhile: stale, on the page with nothing read again ──
  const second = await ask('quiet', 'it is too loud now');
  const again = await opened(page);
  const two = part(again, second);
  await two.getByRole('button', { name: 'Approve', exact: true }).waitFor();
  await cli('record', 'set', 'shop-settings-1', 'title=Back door');
  await two.locator('.told-change-standing', { hasText: 'the record was changed since it was asked' }).waitFor();
  expect('nothing to press on a stale one', (await two.getByRole('button', { name: /^(Approve|Decline)$/ }).count()) === 0);
  await page.screenshot({ path: `${out}/asked-3-stale.png` });

  // ── A third, declined with why: nothing made, and the agent told why where it asked ──
  const third = await ask('soft', 'softer, please');
  const third_page = await opened(page);
  const three = part(third_page, third);
  await three.getByRole('button', { name: 'Decline', exact: true }).click();
  await three.locator('textarea').fill('not before the opening');
  await apart(page, 'with Decline open');
  await three.getByRole('button', { name: 'Decline it', exact: true }).click();
  await three.locator('.told-change-standing', { hasText: 'Ada declined it: not before the opening.' }).waitFor();
  expect('the note as approved', JSON.parse(await read('record', 'show', 'shop-settings-1', '--json')).data.note === 'loud');
  for (let tries = 0; !(await read('read', 'talk')).includes('declined your change of shop-settings-1: not before the opening.'); tries += 1) {
    if (tries > 100) throw new Error(`the agent was never told why in the channel: ${await read('read', 'talk')}`);
    await new Promise((later) => setTimeout(later, 100));
  }
  await saysAtLast(['board', 'history', 'shop-settings-1'], ['Ada declined a change of it: not before the opening'], 'the decline a line of the history');
  await page.screenshot({ path: `${out}/asked-4-declined.png` });
  expect(`Ada's page never to have been read again, was ${loads} times`, loads === 0);

  // ── A card: asked by an agent that may only read the board, made by the board, and a line of its history ──
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('board', 'add', 'shop', 'Fix the cart');
  await cli('agent', 'add', 'reader', '--project', 'shop', '--scope', 'board:read', '--scope', 'records:read', '--scope', 'projects:read', '--scope', 'channels');
  const reading = (await readFile(join(data, 'slots', 'reader', 'token'), 'utf8')).trim();
  const card = JSON.parse((await cli('ask', 'shop-1', 'title=Fix the cart, all of it', '--why', 'it is more than the cart', '--token', reading, '--json')).stdout).ask.ask;
  if (!(await page.getByRole('navigation', { name: 'Inbox' }).isVisible())) await page.locator('.rail .rail-inbox').first().click();
  await page.mouse.move(900, 700);
  const cardLine = page.getByRole('navigation', { name: 'Inbox' }).locator('.inbox-line', { hasText: 'reader asks to change shop-1' });
  await cardLine.locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  const five = part(page.locator('.inbox-page'), card);
  await five.locator('.inbox-page-label', { hasText: 'reader asks to change shop-1 (Fix the cart) of board' }).waitFor();
  await five.getByRole('button', { name: 'Approve', exact: true }).click();
  await five.locator('.told-change-standing', { hasText: 'Ada approved it, and it is made.' }).waitFor();
  await saysAtLast(['board', 'history', 'shop-1'], ['reader asked to change its title: it is more than the cart', 'reader changed its title · approved by Ada'], "the card's history to say it was asked for and who approved it");
  expect('the card changed', JSON.parse(await read('board', 'shop', '--json')).items.find((one) => one.id === 'shop-1').title === 'Fix the cart, all of it');
  await page.screenshot({ path: `${out}/asked-5-card.png` });

  // ── A phone: a fourth ask, its cell under its name and the buttons a finger's size ──
  const fourth = await ask('bright', 'for the evening');
  const lines = JSON.parse((await cli('inbox', '--all', '--json')).stdout).lines.filter((line) => (line.says ?? '').includes('helper asks'));
  expect(`each ask a line of its own, got ${lines.length}`, lines.length === 4 && lines.every((line) => line.count === 1));
  await page.close();
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  const small = await opened(phone, 'tap');
  const four = part(small, fourth);
  await four.getByRole('button', { name: 'Approve', exact: true }).waitFor();
  await apart(phone, 'on a phone');
  const fingers = await four.locator('.told-change-acts button').evaluateAll((all) => all.map((button) => button.getBoundingClientRect().height));
  expect(`the buttons a finger's size on a phone, got ${fingers}`, fingers.every((tall) => tall >= 40));
  await phone.screenshot({ path: `${out}/asked-6-phone.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  for (const asked of [['asked'], ['read', 'talk'], ['board', 'history', 'shop-settings-1']]) {
    console.error(await cli(...asked).then((said) => said.stdout, (why) => String(why)));
  }
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
