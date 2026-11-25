// What a fleet tells whoever applied it: a line in their inbox, from the
// factory's bot, when something waits for a person.
//
//   DAEMON=../target/debug/factory node scripts/told.mjs [out-dir]
//
// Ada owns the factory and applied the shop's fleet, whose one worker is
// a stand-in that asks in place of working. Her page is opened once and
// never read again. A card made ready from a terminal has its agent ask:
// the question is said on the card by the agent, naming her, which is how
// she is told: a line of what was said on the card, which opens
// it at the question. A card left for a person is told her by the fleet:
// the inbox's place on the rail counts it; the box lists the line,
// from the bot by its name, in the fleet's own words cut to two lines,
// with nothing over anything else at three widths; pointed at, what it
// says whole is beside the box, wholly in the window and over neither the
// rail nor the panel; a telling of some three thousand letters (what a
// preview's line says of every check) says there only the start of its
// words, cut where a word ends, and last, faint, that the whole is on its
// page, no taller than half the window and wholly in it at three widths,
// and its page says the whole. A line's page is headed by its first
// sentence whole (to its first full stop or line break), on as many lines
// as it takes and over nothing at three widths and a phone's, where its
// tab is called by its first words and says the sentence where it is
// pointed at. Pressed, the line is read and opens a page of its
// own, a tab called by its first words: who told it (the factory's bot,
// by its name), when (on the clock and from now), its words whole, and
// what it is about, which opens the card on its board with its drawer
// where the question is; Mark unread has it count again on the rail, Mark
// read reads it, Put away takes it out of the box with the page saying
// so; and the page by its boxes at three widths. What the fleet tells
// whom is a table that sits beside the board, listed in the gear of the
// board's head with its one row. A telling about something running says
// what runs and opens it; told again while its page is showing, the page
// (never read again) says the words told last and that it was told twice;
// one about nothing says nothing of what it is about. A telling whose
// words hold a web address has it as a link in the box, among words still
// cut to two lines, which opens it in a page of its own with nothing of
// this one (an address the check serves on this machine, asked nothing
// before it is pressed) and leaves the line as it was; pressed anywhere
// else the line opens its page, where the address is a link that opens
// the same way. A line of a message that names her (Nell's, from a
// terminal of her own) opens its channel at the message and no page. And
// at a phone's width the box has the screen, the address is said again
// under the words a finger's size and opens the same, and a line tapped
// opens its page with the panel put away, whole on the screen, which opens
// the card. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { tablesRead } from './gear.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'told-data');
const home = join(out, 'told-home');
const hers = join(out, 'told-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// Reads what it is asked and asks, in place of working.
const asks = join(home, 'asks.sh');
await writeFile(asks, "#!/bin/sh\ncat >/dev/null\necho 'Which total: with tax, or without? The card does not say, and the two come to different sums on every line of the receipt.'\necho 'DECISION: ask'\n");
await chmod(asks, 0o755);
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--command', `asks=${asks}`], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
// Where a told line's link leads: this machine, on a port of its own, counting who asks.
let fetched = 0;
const preview = createServer((request, response) => {
  // What a browser asks of a page's place by itself (an icon) is not the address being opened.
  if (request.url === '/preview') fetched += 1;
  response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  response.end('<!doctype html><title>Preview</title><p>The preview.');
});
await new Promise((ready) => preview.listen(0, '127.0.0.1', ready));
const PREVIEW = `http://127.0.0.1:${preview.address().port}/preview`;
const SAYS = `Preview ready: ${PREVIEW}. Built from origin/dev's last commit, with a home and data of its own, so nothing tried there touches this factory.`;
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
// A telling of some three thousand letters, as a preview's line lists every check it ran.
const CHECKS = Array.from({ length: 120 }, (_, at) => `check ${at + 1} of the client passed`).join(', ');
const LONG = `The preview ran every check: ${CHECKS}.`;
// A telling whose first sentence is longer than the few words its tab is called by.
const SENTENCE = 'shop-370: its agent could not do what it was asked, and the card was left for a person to take up again from ready.';
// Nell, who names Ada in a channel, from a terminal of her own.
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: { ...env, HOME: hers, XDG_CONFIG_HOME: join(hers, 'config') } });

const WHY = 'Which total: with tax, or without? The card does not say, and the two come to different sums on every line of the receipt.';
const QUESTION = `shop-1 is left for a person: ${WHY}`;
/** Says that a card is left for a person, as what works it says (`card.left`), which the fleet tells whoever its rows say. */
const leaves = (card) => `blueprint: leaves-${card}
steps:
  left:
    do: emit
    with: { topic: card.left, card: ${card}, project: shop, why: "${WHY}" }
`;
// Tells whoever applied it, about what it is handed: something running, or nothing.
const NIGHTLY = `blueprint: nightly
permissions: [inbox]
params:
  about: { type: string, default: "" }
  says: { type: string }
steps:
  tell:
    do: inbox.tell
    with: { to: applier, says: "\${params.says}", why: built, about: "\${params.about}" }
`;

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('channel', 'create', 'general');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('record', 'set', 'shop-agents-2', 'adapter=asks');
  await cli('board', 'add', 'shop', 'Add a cart');
  const nightly = join(home, 'nightly.bp');
  await writeFile(nightly, NIGHTLY);

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
  const signedIn = async (how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  // Ada's page is opened once, and is never read again from here to the end.
  const page = await signedIn();
  await page.locator('.rail').waitFor();
  let loads = 0;
  page.on('load', () => (loads += 1));

  // The inbox is a place of the rail, which says how many wait.
  const tray = (on, unread) => on.locator('.rail .rail-inbox').and(on.getByRole('button', { name: unread ? `Inbox, ${unread} unread` : 'Inbox', exact: true }));
  const waits = async (on, unread, why) => {
    await tray(on, unread)
      .first()
      .waitFor({ timeout: 20000 })
      .catch(async () => {
        throw new Error(`expected the rail's inbox to say ${unread} unread ${why}, got "${await on.locator('.rail .rail-inbox').first().getAttribute('aria-label')}"`);
      });
  };
  const box = page.getByRole('navigation', { name: 'Inbox' });
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
  const laidOut = async (on, what) => {
    const side = on.getByRole('navigation', { name: 'Inbox' });
    for (const one of await side.locator('.inbox-line').all()) await apart(one, LINE, `a line of the box ${what}`);
  };

  // ── Nothing waits, and what the fleet tells whom is a table beside its board ──
  await waits(page, 0, 'with nothing told');
  await page.getByRole('button', { name: 'Resources', exact: true }).click();
  await unfoldAll(page);
  await page.locator('.nav').getByText('Shop', { exact: true }).click();
  await page.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }).click();
  await page.locator('.column', { hasText: 'ready' }).first().waitFor();
  // It is listed with the fleet's own three, by its name and its one row, and not under the project.
  const tables = await tablesRead(page);
  expect(`the gear of the board's head to list who is told what among its tables, got ${tables.join(' | ')}`, tables.includes('Tells 1 tell'));
  await page.screenshot({ path: `${out}/told-1-beside.png` });
  await page.keyboard.press('Escape');
  expect('no table of who is told under the project in the side panel', (await page.locator('.nav [data-project="shop"]').getByText('Tells', { exact: true }).count()) === 0);

  // ── A card asks: said on the card by its agent, naming whoever applied the fleet, who is told by being named ──
  await cli('board', 'move', 'shop-1', 'ready');
  await waits(page, 1, 'once the card asked, on a page that was not read again');
  await tray(page, 1).first().click();
  await box.waitFor();
  const question = line(box, 'Which total: with tax, or without?');
  await question.waitFor();
  expect(`the question's line to be of what was said on the card, got ${await question.getAttribute('data-kind')}`, (await question.getAttribute('data-kind')) === 'commented');
  expect(`the question to be from its agent, got "${await question.locator('.inbox-from').innerText()}"`, (await question.locator('.inbox-from').innerText()) === 'shop-worker-1');
  expect(`the question to name her, got "${await question.locator('.inbox-words').innerText()}"`, (await question.locator('.inbox-words').innerText()).startsWith('@ada Which total'));
  await question.locator('.inbox-open').click();
  const atQuestion = drawerAtRest(page);
  await atQuestion.locator('.journey-spoken.journey-at', { hasText: 'Which total' }).waitFor();
  await waits(page, 0, 'once the question was opened');
  await atQuestion.getByRole('tab', { name: /^Details/ }).click();
  await atQuestion.getByRole('button', { name: 'Waiting on an answer from @ada: answer in Comments', exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await atQuestion.waitFor({ state: 'detached' });
  await page.mouse.move(760, 520);

  // ── A card left for a person, and whoever applied the fleet is told ──
  await writeFile(join(home, 'leaves-1.bp'), leaves('shop-1'));
  await cli('apply', join(home, 'leaves-1.bp'), '--project', 'shop');
  await waits(page, 1, 'once the card was left, on a page that was not read again');
  await page.screenshot({ path: `${out}/told-2-rail.png` });
  if ((await box.count()) === 0) await tray(page, 1).first().click();
  await box.waitFor();
  const asked = line(box, 'shop-1 is left for a person');
  await asked.waitFor();
  expect(`the line to be what a blueprint told, got ${await asked.getAttribute('data-kind')}`, (await asked.getAttribute('data-kind')) === 'told');
  // From the bot, by its name; in the fleet's words; and of no channel.
  expect(`the line to be from the factory's bot, got "${await asked.locator('.inbox-from').innerText()}"`, (await asked.locator('.inbox-from').innerText()) === 'conductor');
  expect(`the line to say why the card was left, got "${await asked.locator('.inbox-words').innerText()}"`, (await asked.locator('.inbox-words').innerText()) === QUESTION);
  expect('the line to say no channel', (await asked.locator('.inbox-where').count()) === 0);
  expect('the line to wait unread', await asked.evaluate((one) => one.classList.contains('unread')));
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // Its words are cut to two lines in the box: the line's own page says them whole.
  const cut = await asked.locator('.inbox-words').evaluate((words) => words.scrollHeight - words.clientHeight);
  expect(`the words to be cut in the box, got ${cut}px more of them`, cut > 0);

  // Pointed at, what the line says whole is beside the box, wholly in the window, and over neither the rail nor the panel.
  const tipped = async (on, one, what) => {
    await on.mouse.move(5, 500);
    await on.locator('.tip-said').waitFor({ state: 'detached' });
    const at = await one.boundingBox();
    await on.mouse.move(at.x + 30, at.y + 10);
    const tip = on.locator('.tip-said');
    await tip.waitFor();
    const found = await on.evaluate(() => {
      const box = (selector) => document.querySelector(selector).getBoundingClientRect();
      const [tip, rail, side] = [box('.tip-said'), box('.rail'), box('nav.sidebar')];
      return { tip: tip.toJSON(), rail: rail.toJSON(), side: side.toJSON(), width: window.innerWidth, height: window.innerHeight, says: document.querySelector('.tip-said').textContent };
    });
    const { tip: box } = found;
    expect(`the tip ${what} to be wholly in the window: ${JSON.stringify(found)}`, box.left >= 0 && box.top >= 0 && box.right <= found.width && box.bottom <= found.height);
    expect(`the tip ${what} to lie over neither the rail nor the panel, beside the panel: ${JSON.stringify(found)}`, box.left >= found.side.right - 0.5 && box.left >= found.rail.right);
    expect(`the tip ${what} to say the line whole, got "${found.says}"`, found.says.includes(QUESTION));
    await on.mouse.move(5, 500);
  };
  for (const width of [1280, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await tipped(page, asked, `at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  const at3 = await asked.boundingBox();
  await page.mouse.move(at3.x + 30, at3.y + 10);
  await page.locator('.tip-said').waitFor();
  await page.screenshot({ path: `${out}/told-3-line.png` });
  await page.mouse.move(760, 520);

  // ── Pressed: read, and a page of the line itself ──
  const message = page.locator('.inbox-page');
  const pageOf = async (on, words) => {
    const shown = on.locator('.inbox-page');
    await shown.locator('.inbox-page-words', { hasText: words }).waitFor();
    return shown;
  };
  // The page by its boxes: nothing over anything else, nothing outside it or scrolling sideways.
  const PAGE = '.inbox-page-title, .inbox-page-from, .inbox-page-when, .inbox-page-times, .inbox-page-words, .inbox-page-label, .inbox-page-opens, .inbox-page-about .running-id, .inbox-page-acts button';
  const pageLaidOut = async (on, what) => apart(on.locator('.inbox-page'), PAGE, `the page of a line ${what}`);
  await asked.locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  await waits(page, 0, 'once the line was pressed');
  const shown = await pageOf(page, 'Which total: with tax, or without?');
  // Its tab is called by its first words; the page says who told it, when, and the words whole.
  await page.locator('.tab[aria-selected="true"]', { hasText: 'shop-1 is left for a person:…' }).waitFor();
  expect(`the page to be from the factory's bot, got "${await shown.locator('.inbox-page-from').innerText()}"`, (await shown.locator('.inbox-page-from').innerText()) === "conductor · the factory's bot");
  const when = await shown.locator('.inbox-page-when').innerText();
  expect(`the page to say when on the clock and from now, got "${when}"`, /^\d{1,2}:\d\d (?:AM|PM) \S+ · (just now|\dm ago)$/.test(when));
  expect(`the page to say the words whole, got "${await shown.locator('.inbox-page-words').innerText()}"`, (await shown.locator('.inbox-page-words').innerText()) === QUESTION);
  expect('the page to say it was told once', (await shown.locator('.inbox-page-times').count()) === 0);
  expect('the line read and nothing more open', (await drawerAtRest(page).count()) === 0);
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await pageLaidOut(page, `at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/told-4-page.png` });
  // Told with no parts: nothing under what it is about, and no row of tabs (`toldtabs.mjs` has the tabs).
  expect('no row of tabs on a line told with no parts', (await shown.locator('.told-tabs, .told-parts').count()) === 0);
  // What it is about opens it: the card on its board with its drawer.
  const about = shown.locator('.inbox-page-about .inbox-page-opens');
  expect(`what it is about to say the card, got "${await about.innerText()}"`, (await about.innerText()).includes('Add a cart') && (await about.innerText()).includes('card shop-1 on the Shop board'));
  await about.click();
  const drawer = drawerAtRest(page);
  await drawer.getByRole('button', { name: /^Waiting on an answer/ }).waitFor();
  expect('the card to be the one that was left', (await drawer.getByLabel('Title').inputValue()) === 'Add a cart');
  await page.screenshot({ path: `${out}/told-5-about.png` });
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });

  // Made to wait unread again, it counts on the rail; read again by its own button; then put away.
  const tabOf = page.locator('.tabs .tab', { hasText: 'shop-1 is left for a person' });
  await tabOf.click();
  await shown.getByRole('button', { name: 'Mark unread' }).click();
  await waits(page, 1, 'once the line was made to wait unread again');
  await shown.getByRole('button', { name: 'Mark read' }).waitFor();
  await page.waitForFunction(() => document.querySelector('nav[aria-label="Inbox"] .inbox-line')?.classList.contains('unread'));
  await shown.getByRole('button', { name: 'Mark read' }).click();
  await waits(page, 0, 'once the line was read from its page');
  await shown.getByRole('button', { name: 'Put away' }).click();
  await shown.locator('.inbox-page-away').waitFor();
  await asked.waitFor({ state: 'detached' });
  expect('nothing more offered of a line put away', (await shown.locator('.inbox-page-acts button').count()) === 0);
  await page.screenshot({ path: `${out}/told-6-away.png` });

  // ── Told about something running, and about nothing ──
  await page.mouse.move(760, 520);
  const running = JSON.parse((await cli('instances', '--json')).stdout).instances.find((one) => one.blueprint === 'std/fleet').id;
  await cli('apply', nightly, '--key', 'one', '--param', `about=${running}`, '--param', 'says=The fleet is as its tables say.');
  await cli('apply', nightly, '--key', 'two', '--param', 'says=The night went well.');
  await waits(page, 2, 'once two more things were told');
  const fleet = line(box, 'The fleet is as its tables say.');
  await fleet.locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  await page.mouse.move(760, 520);
  await waits(page, 1, 'once the line about something running was pressed');
  const ofFleet = await pageOf(page, 'The fleet is as its tables say.');
  // Something running is said for what it is, and opened as a tab of its own, called what it runs.
  const runs = ofFleet.locator('.inbox-page-about .running-id');
  await runs.locator('.running-is').waitFor();
  expect(`what it is about to say what runs, got "${await runs.innerText()}"`, (await runs.innerText()).includes('std/fleet'));
  // A page never read again follows the same thing told again: told twice, its words as told last.
  await cli('apply', nightly, '--key', 'one-again', '--param', `about=${running}`, '--param', 'says=The fleet is as its tables say, still.');
  await ofFleet.locator('.inbox-page-words', { hasText: 'The fleet is as its tables say, still.' }).waitFor();
  await ofFleet.locator('.inbox-page-times', { hasText: 'Told 2 times, first' }).waitFor();
  await waits(page, 2, 'once the line on the page was told again');
  await page.screenshot({ path: `${out}/told-7-told-again.png` });
  await runs.click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'std/fleet' }).waitFor();
  const night = line(box, 'The night went well.');
  await night.locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  const ofNight = await pageOf(page, 'The night went well.');
  expect('a line about nothing to say nothing of what it is about', (await ofNight.locator('.inbox-page-about').count()) === 0);
  await page.mouse.move(760, 520);

  // ── A web address in what was told is a link, in the box and on the line's page ──
  await cli('apply', nightly, '--key', 'three', '--param', `says=${SAYS}`);
  await waits(page, 2, 'once a telling with an address in it came');
  const ready = line(box, 'Preview ready:');
  const link = ready.locator('.inbox-words a.inbox-link');
  await link.waitFor();
  // What a sentence puts after it is not part of it, and it is opened in a page of its own with nothing of this one.
  expect(`the link to lead to ${PREVIEW}, got ${await link.getAttribute('href')}`, (await link.getAttribute('href')) === PREVIEW);
  expect(`the link's words to be the address, got "${await link.innerText()}"`, (await link.innerText()) === PREVIEW);
  expect('the link to open in a tab of its own', (await link.getAttribute('target')) === '_blank');
  const rel = ` ${(await link.getAttribute('rel')) ?? ''} `;
  expect(`the link to hand nothing of this page on, got rel "${rel.trim()}"`, rel.includes(' noopener ') && rel.includes(' noreferrer '));
  expect('the words of the line to be the telling, letter for letter', (await ready.locator('.inbox-words').innerText()) === SAYS);
  expect('no link inside a button', (await page.locator('nav[aria-label="Inbox"] button a').count()) === 0);
  expect('the words to be cut to two lines', await ready.locator('.inbox-words').evaluate((words) => getComputedStyle(words).webkitLineClamp === '2' && words.scrollHeight > words.clientHeight));
  // The link is inside the words, on lines of its own boxes, and over nothing else of the line.
  const linkLaidOut = async (what) => {
    const wrong = await ready.evaluate((one) => {
      const box = (selector) => one.querySelector(selector).getBoundingClientRect();
      const words = box('.inbox-words');
      const others = ['.inbox-from', '.inbox-when'].map(box).concat(one.querySelector('.inbox-away').getBoundingClientRect());
      const found = [];
      for (const rect of one.querySelector('.inbox-words a.inbox-link').getClientRects()) {
        if (rect.width <= 0) continue;
        if (rect.left < words.left - 0.5 || rect.right > words.right + 0.5 || rect.top < words.top - 0.5) found.push('the link leaves the words');
        for (const other of others) if (Math.min(rect.right, other.right) - Math.max(rect.left, other.left) > 0.5 && Math.min(rect.bottom, other.bottom) - Math.max(rect.top, other.top) > 0.5) found.push('the link lies over the head of the line');
      }
      return found;
    });
    expect(`${what}: the link inside the words and over nothing, got ${wrong.join('; ')}`, wrong.length === 0);
  };
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `with a link in a line at ${width} wide`);
    await linkLaidOut(`at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  expect('no row of links under the words at a pointer', (await ready.locator('.inbox-links').evaluate((links) => getComputedStyle(links).display)) === 'none');
  expect(`nothing to have asked the address before the link was pressed, got ${fetched}`, fetched === 0);
  // Pressed, the link is its own: the address in a page of its own, which has no hold on this one, and the line as it was.
  const tabBefore = await page.locator('.tab[aria-selected="true"]').innerText();
  const [opened] = await Promise.all([page.context().waitForEvent('page'), link.click()]);
  await opened.waitForLoadState();
  expect(`the page opened to be the address, got ${opened.url()}`, opened.url() === PREVIEW);
  expect('the page opened to have no hold on the one it was opened from', await opened.evaluate(() => window.opener === null));
  expect(`the address to have been asked once, by the page opened, got ${fetched}`, fetched === 1);
  await opened.close();
  await waits(page, 2, 'once the link was pressed and not the line');
  expect('the line still unread once its link was pressed', await ready.evaluate((one) => one.classList.contains('unread')));
  expect('the link to have opened nothing on the page', (await page.locator('.tab[aria-selected="true"]').innerText()) === tabBefore && (await drawerAtRest(page).count()) === 0);
  // Pressed anywhere else, the line is read and its page opened, the address a link there too, opened the same way.
  await ready.locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  await waits(page, 1, 'once the line with a link was pressed');
  const ofReady = await pageOf(page, 'Preview ready:');
  const onPage = ofReady.locator('.inbox-page-words a.inbox-link');
  expect(`the page's link to lead to ${PREVIEW}, in a tab of its own handing nothing on`, (await onPage.getAttribute('href')) === PREVIEW && (await onPage.getAttribute('target')) === '_blank' && /noopener/.test(await onPage.getAttribute('rel')) && /noreferrer/.test(await onPage.getAttribute('rel')));
  expect('the page to say the words letter for letter', (await ofReady.locator('.inbox-page-words').innerText()) === SAYS);
  const [fromPage] = await Promise.all([page.context().waitForEvent('page'), onPage.click()]);
  await fromPage.waitForLoadState();
  expect(`the page's link to open the address, got ${fromPage.url()}`, fromPage.url() === PREVIEW && fetched === 2 && (await fromPage.evaluate(() => window.opener === null)));
  await fromPage.close();
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await pageLaidOut(page, `with a link at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.mouse.move(760, 520);
  await page.screenshot({ path: `${out}/told-8-link.png` });

  // ── A long telling: pointed at, the start of its words, and that the whole is on its page ──
  await cli('apply', nightly, '--key', 'long', '--param', `says=${LONG}`);
  await waits(page, 2, 'once a long telling came');
  const long = line(box, 'The preview ran every check:');
  await long.waitFor();
  expect(`the long telling to be some three thousand letters, got ${LONG.length}`, LONG.length > 2800);
  const tippedLong = async (what) => {
    await page.mouse.move(5, 500);
    await page.locator('.tip-said').waitFor({ state: 'detached' });
    const at = await long.boundingBox();
    await page.mouse.move(at.x + 30, at.y + 10);
    await page.locator('.tip-said').waitFor();
    const found = await page.evaluate(() => {
      const box = (selector) => document.querySelector(selector).getBoundingClientRect().toJSON();
      const tip = document.querySelector('.tip-said');
      const more = tip.querySelector('.tip-more');
      const [words, line] = [tip.firstChild?.textContent ?? '', more ? box('.tip-said .tip-more') : null];
      return { tip: box('.tip-said'), side: box('nav.sidebar'), rail: box('.rail'), width: window.innerWidth, height: window.innerHeight, words, more: more?.textContent ?? null, line, faint: more ? getComputedStyle(more).color !== getComputedStyle(tip).color : false };
    });
    const { tip } = found;
    expect(`the long tip ${what} to be wholly in the window: ${JSON.stringify(found)}`, tip.left >= 0 && tip.top >= 0 && tip.right <= found.width && tip.bottom <= found.height);
    expect(`the long tip ${what} to be no taller than half the window, got ${tip.height} of ${found.height}`, tip.height <= found.height / 2);
    expect(`the long tip ${what} to lie beside the panel, over neither it nor the rail: ${JSON.stringify(found)}`, tip.left >= found.side.right - 0.5 && tip.left >= found.rail.right);
    const start = `Unread: conductor: ${LONG}`;
    const kept = found.words.slice(0, -1);
    expect(`the long tip ${what} to say the start of the words, cut where a word ends with an ellipsis, got "${found.words}"`, found.words.endsWith('…') && found.words.length <= 400 && start.startsWith(kept) && /[\s,.]/.test(start[kept.length] ?? ''));
    expect(`the long tip ${what} to say last, faint and on a line of its own, that the whole is on its page, got ${JSON.stringify(found)}`, found.more === 'The whole of it is on its page, which pressing the line opens.' && found.faint && found.line.top >= tip.top && found.line.bottom <= tip.bottom + 0.5);
    await page.screenshot({ path: `${out}/told-8b-long-tip-${what.replace(/\W+/g, '-')}.png` });
    await page.mouse.move(5, 500);
  };
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await tippedLong(`at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // A short one says itself whole, with no line of where the rest is.
  await page.mouse.move(5, 500);
  const night2 = line(box, 'The night went well.');
  const n2 = await night2.boundingBox();
  await page.mouse.move(n2.x + 30, n2.y + 10);
  await page.locator('.tip-said').waitFor();
  expect('a short telling to say itself whole, with no line of where the rest is', (await page.locator('.tip-said .tip-more').count()) === 0 && (await page.locator('.tip-said').innerText()).includes('The night went well.'));
  await page.mouse.move(5, 500);
  // Pressed, its page says the whole of it.
  await long.locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  await waits(page, 1, 'once the long line was pressed');
  const ofLong = await pageOf(page, 'The preview ran every check:');
  expect('its page to say the whole of it', (await ofLong.locator('.inbox-page-words').innerText()) === LONG);
  // Headed by its first sentence, which is all of it here and longer than a head says: cut where a word ends.
  const longHead = (await ofLong.locator('.inbox-page-title').innerText()).trim();
  expect(`the long line's page to be headed by the start of its one sentence, cut at a word, got "${longHead}"`, longHead.endsWith('…') && longHead.length <= 201 && LONG.startsWith(longHead.slice(0, -1)) && /[\s,]/.test(LONG[longHead.length - 1] ?? ''));
  await page.mouse.move(760, 520);

  // ── A line's page headed by its first sentence whole, its tab by its first words ──
  await cli('apply', nightly, '--key', 'sentence', '--param', `says=${SENTENCE} What it said: the tests would not build. Nothing was merged.`);
  await waits(page, 2, 'once a telling of a long first sentence came');
  await line(box, 'its agent could not do').locator('.inbox-press').click({ position: { x: 24, y: 10 } });
  await waits(page, 1, 'once that line was pressed');
  const ofSentence = await pageOf(page, 'its agent could not do');
  const headed = ofSentence.locator('.inbox-page-title');
  const tabOfIt = page.locator('.tab[aria-selected="true"]');
  const tabSays = (await tabOfIt.locator('.truncate').innerText()).trim();
  expect(`the page to be headed by its first sentence whole, got "${await headed.innerText()}"`, (await headed.innerText()).trim() === SENTENCE);
  expect(`the tab to be called by its first words still, got "${tabSays}"`, tabSays.endsWith('…') && SENTENCE.startsWith(tabSays.slice(0, -1)) && tabSays.length < SENTENCE.length);
  for (const width of [1280, 1000, 820, 390]) {
    await page.setViewportSize({ width, height: 860 });
    await pageLaidOut(page, `headed by a long first sentence at ${width} wide`);
    // On as many lines as it takes: nothing of it cut off, inside the page and over nothing else of it.
    const cut = await headed.evaluate((head) => ({ across: head.scrollWidth - head.clientWidth, down: head.scrollHeight - head.clientHeight }));
    expect(`the head at ${width} wide to be whole on its lines, got ${JSON.stringify(cut)}`, cut.across <= 0 && cut.down <= 0);
    await page.screenshot({ path: `${out}/told-8c-sentence-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // At a phone's width the side panel was put away: the box is brought back, as a person would.
  if (!(await box.isVisible())) await page.locator('.rail .rail-inbox').first().click();
  await box.waitFor();
  // The tab pointed at says the first sentence whole.
  await page.mouse.move(5, 500);
  const tabAt = await tabOfIt.boundingBox();
  await page.mouse.move(tabAt.x + 20, tabAt.y + tabAt.height / 2);
  const tabTip = page.locator('.tip-said');
  await tabTip.waitFor();
  expect(`the tab's tip to say the first sentence whole, got "${await tabTip.innerText()}"`, (await tabTip.innerText()).trim() === SENTENCE);
  await page.mouse.move(760, 520);

  // ── A line of something said opens its channel at the message, as it always did ──
  await nell('post', 'general', '@ada', 'the', 'quince', 'is', 'in');
  await waits(page, 2, 'once Nell named her');
  const named = line(box, 'the quince is in');
  expect(`a named line to be no told one, got ${await named.getAttribute('data-kind')}`, (await named.getAttribute('data-kind')) === 'named');
  await named.locator('.inbox-open').click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'general' }).waitFor();
  await page.locator('.message', { hasText: 'the quince is in' }).first().waitFor();
  expect('a named line to open no page of its own', (await page.locator('.tab[aria-selected="true"]').innerText()).includes('general') && (await message.count()) === 0);
  await waits(page, 1, 'once the named line was pressed');
  await page.mouse.move(760, 520);

  // ── A phone's width ──
  await cli('board', 'add', 'shop', 'Add a till');
  await writeFile(join(home, 'leaves-2.bp'), leaves('shop-2'));
  await cli('apply', join(home, 'leaves-2.bp'), '--project', 'shop');
  await waits(page, 2, 'once a second card was left');
  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await waits(phone, 2, 'on a phone');
  await tray(phone, 2).first().tap();
  const small = phone.getByRole('navigation', { name: 'Inbox' });
  await small.waitFor();
  await line(small, 'shop-2 is left for a person').waitFor();
  await laidOut(phone, 'on a phone');
  // Under a finger the address is said again under the words, a finger's size and over nothing of the line.
  const onPhone = line(small, 'Preview ready:');
  const row = onPhone.locator('a.inbox-link-row');
  expect(`one row of links under the words on a phone, got ${await row.count()}`, (await row.count()) === 1);
  expect(`the row to lead to ${PREVIEW}`, (await row.getAttribute('href')) === PREVIEW && (await row.getAttribute('target')) === '_blank');
  await apart(onPhone, `${LINE}, .inbox-link-row`, 'a line with a link on a phone');
  const finger = await row.evaluate((one) => {
    const ruler = document.createElement('div');
    ruler.style.cssText = 'position: absolute; height: calc(40px * var(--tall))';
    one.append(ruler);
    const tall = ruler.getBoundingClientRect().height;
    ruler.remove();
    return { tall, height: one.getBoundingClientRect().height };
  });
  expect(`the row to be a finger's size, ${finger.tall}px, got ${finger.height}px`, finger.height >= finger.tall - 0.5 && finger.height >= 40);
  await phone.screenshot({ path: `${out}/told-9-phone.png` });
  const fetchedBefore = fetched;
  const [fromPhone] = await Promise.all([phone.context().waitForEvent('page'), row.tap()]);
  await fromPhone.waitForLoadState();
  expect(`the row tapped to open the address, got ${fromPhone.url()}`, fromPhone.url() === PREVIEW && fetched === fetchedBefore + 1);
  await fromPhone.close();
  expect('the box to stay as it was once the row was tapped', (await small.count()) === 1);
  // A line tapped opens its page with the panel put away, the page whole on the phone; what it is about opens the card.
  await line(small, 'shop-2 is left for a person').locator('.inbox-press').tap({ position: { x: 24, y: 10 } });
  const onItsPage = await pageOf(phone, 'Which total: with tax, or without?');
  expect('the panel to be put away from over the page it opened', (await small.count()) === 0);
  await pageLaidOut(phone, 'on a phone');
  const touched = await onItsPage.locator('.inbox-page-acts button').evaluateAll((all) => all.map((one) => one.getBoundingClientRect().height));
  expect(`what is done with the line a finger's size on a phone, got ${touched}`, touched.length > 0 && touched.every((tall) => tall >= 40));
  await phone.screenshot({ path: `${out}/told-10-phone-page.png` });
  await onItsPage.locator('.inbox-page-about .inbox-page-opens').tap();
  await drawerAtRest(phone).getByLabel('Title').waitFor();
  await phone.screenshot({ path: `${out}/told-11-phone-card.png` });
  await waits(page, 1, 'once it was read on the phone');
  expect(`Ada's page never to have been read again, was ${loads} times`, loads === 0);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  preview.closeAllConnections();
  await new Promise((gone) => preview.close(gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
