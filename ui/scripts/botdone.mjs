// What the factory's bot did lately, where it is read about.
//
//   DAEMON=../target/debug/factory node scripts/botdone.mjs [out-dir]
//
// Two people, each on a page of their own: Ada, who owns the factory, and
// Nell, who registered by an invitation. A blueprint of Ada's adds a card
// and moves it, adds a row of a table kept private, and has the bot say
// something in `general` and in a channel kept to Ada. Pressed by its
// picture, the bot is read about under Channel members in the Tools
// panel, and says what it did lately, newest first under a heading for
// the day: each line in the words the card's history uses with the record
// in the place of `it` and the blueprint after it, faint (`moved shop-1
// from backlog to ready · via does`); a line pressed opens the card, and one
// of what it said opens the channel at the message. Nell is shown nothing
// of the private table or of the channel kept from her. Called something
// else, the bot is said by its new name where it is read about. Nothing of
// it lies over anything else or leaves the panel at three widths and a
// phone's.
//
// And what it may ask, by area, over that: everything its commands reach
// until somebody narrows it. Ada holds it to reading the board, by the
// table's line for the board; asked to move a card, the bot then says that
// its ceiling is what stands in the way, and moves nothing; given
// everything again, it moves the card for her. Nell, who does not manage
// the factory, is shown what it may ask and offered nothing to change.
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, toolsAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'botdone-data');
const home = join(out, 'botdone-home');
const hers = join(out, 'botdone-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
// The daemon and each command line are given a home of their own, so that nothing here reads or writes what a
// factory already on this machine keeps.
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

const DOES = `blueprint: does
permissions: [board, records, projects, channels]
params: { kept: { type: string } }
steps:
  card: { do: board.add, with: { project: shop, title: Ledger } }
  moved: { needs: [card], do: board.move, with: { id: "\${steps.card.out.id}", to: ready } }
  notes:
    do: collection.ensure
    with: { name: notes, reads: "private:notes", fields: { title: { type: text, required: true } }, history: [record.*] }
  note: { needs: [notes], do: record.add, with: { collection: notes, fields: { title: Quietly } } }
  open: { needs: [moved], do: channel.post, with: { channel: general, body: The ledger is ready. } }
  among: { needs: [open], do: channel.post, with: { channel: "\${params.kept}", body: Between us. } }
`;

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('channel', 'create', 'general');
  await cli('channel', 'create', 'Pay review', '--kept');
  const kept = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.find((one) => one.title === 'Pay review').id;
  await writeFile(join(out, 'does.bp'), DOES);
  await cli('apply', join(out, 'does.bp'), '--project', 'shop', '--param', `kept=${kept}`);

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
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  // The bot read about, from its picture beside what it said in `general`: the panel and what it did lately.
  const readAbout = async (page, tap = false, words = 'The ledger is ready.') => {
    const nav = page.getByRole('navigation', { name: 'Channels' });
    if (tap) await page.getByRole('button', { name: 'Expand the side panel' }).tap();
    await sideSettled(page);
    await nav.locator('.nav-item .truncate', { hasText: /^general$/ }).first()[tap ? 'tap' : 'click']();
    const said = page.locator('.message', { hasText: words });
    await said.waitFor();
    await said.locator('.avatar').first()[tap ? 'tap' : 'click']();
    const panel = toolsAtRest(page);
    await panel.locator('.bot-done .bot-done-line').first().waitFor();
    await panel.locator('.bot-done').scrollIntoViewIfNeeded();
    return panel;
  };
  const linesOf = async (panel) => (await panel.locator('.bot-done .bot-done-line .bot-done-words').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
  // Each line and its parts inside the panel, none over another, and nothing scrolling sideways.
  const apart = async (panel, what) => {
    const found = await panel.evaluate((box) => {
      const frame = box.getBoundingClientRect();
      const boxes = [...box.querySelectorAll('.bot-done h3, .bot-done-line, .holds-table tr, .holds-where')].map((one) => ({ said: one.textContent.trim().slice(0, 40), box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < frame.left - 0.5 || one.box.right > frame.right + 0.5) wrong.push(`${one.said} leaves the panel`);
        for (const two of boxes.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth };
    });
    expect(`${what}: lines to be there`, found.count > 2);
    expect(`${what}: nothing over anything else or outside the panel, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px`, found.sideways <= 0);
  };

  const page = await signedIn('ada@example.com');
  const panel = await readAbout(page);
  await panel.getByRole('heading', { name: 'What it did lately' }).waitFor();
  await panel.locator('.bot-done .turns-day h3', { hasText: 'Today' }).waitFor();
  const adas = (await linesOf(panel)).join(' | ');
  for (const line of ['moved shop-1 from backlog to ready · via does', 'added shop-1 to backlog · via does', 'said in #general: The ledger is ready. · via does', 'said in Pay review: Between us. · via does']) {
    expect(`what the bot did to say "${line}", got ${adas}`, adas.includes(line));
  }
  expect(`the private table's row to be among what Ada reads, got ${adas}`, /shop-notes-1: created/.test(adas));
  expect(`the newest first, got ${adas}`, adas.indexOf('Between us.') < adas.indexOf('moved shop-1'));
  expect('the blueprint to be faint, after the words', (await panel.locator('.bot-done-line .bot-done-words .faint').first().innerText()).trim().startsWith('· via'));
  await page.screenshot({ path: `${out}/botdone-1-ada.png` });
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await apart(panel, `at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // What it may ask: everything its commands reach, held to reading the board by the table's line for it.
  const holds = panel.locator('.holds-table');
  await holds.waitFor();
  const where = () => panel.locator('.holds-where').innerText();
  expect(`what it may ask to be all its commands reach, and a ceiling, got "${await where()}"`, (await where()).startsWith('Everything its commands reach') && (await where()).includes('is not narrowed by this'));
  await holds.getByLabel('What it may do with board').selectOption({ label: 'read' });
  await panel.locator('.holds-where', { hasText: 'Narrowed from' }).waitFor();
  await panel.locator('.holds-table').scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/botdone-2-narrowed.png` });
  await cli('post', 'general', '@conductor board move shop-1 done');
  const refused = page.locator('.message', { hasText: 'I may not do that' });
  await refused.waitFor();
  expect(`the bot to say its limit, got "${await refused.innerText()}"`, (await refused.innerText()).includes('that needs board:write, which you hold'));
  expect('the card not to be moved', !(await cli('board', 'history', 'shop-1')).stdout.includes('to done'));
  await page.screenshot({ path: `${out}/botdone-3-refused.png` });
  await panel.getByRole('button', { name: 'Give it everything its commands reach again' }).click();
  await panel.locator('.holds-where', { hasText: 'which nobody has narrowed' }).waitFor();
  await cli('post', 'general', '@conductor board move shop-1 done');
  await page.locator('.message', { hasText: '`shop-1` is in done.' }).or(page.locator('.message', { hasText: 'shop-1 is in done.' })).first().waitFor();

  // A line about a card opens the card; one of what it said opens the channel at what it said.
  await panel.locator('.bot-done-line', { hasText: 'moved shop-1 from backlog to ready' }).click();
  const card = drawerAtRest(page);
  await card.getByRole('tab', { name: /^History/ }).click();
  await card.locator('.journey li', { hasText: 'conductor moved it from backlog to ready' }).waitFor();
  await page.screenshot({ path: `${out}/botdone-4-opened.png` });

  // Nell reads neither the private table nor the channel kept to Ada: nothing of either.
  const theirs = await signedIn('nell@example.com');
  const theirPanel = await readAbout(theirs);
  const nells = (await linesOf(theirPanel)).join(' | ');
  expect(`Nell to be shown the card and general, got ${nells}`, nells.includes('moved shop-1 from backlog to ready') && nells.includes('said in #general'));
  expect(`Nell to be shown nothing of what is kept from her, got ${nells}`, !nells.includes('Between us') && !nells.includes('Pay review') && !nells.includes('shop-notes'));
  expect('Nell to be offered nothing to change of what it may ask', (await theirPanel.locator('.holds-table select').count()) === 0 && (await theirPanel.locator('.holds-where').innerText()).includes('Whoever manages the factory changes it.'));
  await theirs.screenshot({ path: `${out}/botdone-5-nell.png` });

  // Called something else, it is read about under its new name, its lines the same.
  await cli('channel', 'speaker', 'foreman');
  await writeFile(join(out, 'says.bp'), 'blueprint: says\npermissions: [channels]\nsteps:\n  say: { do: channel.post, with: { channel: general, body: Under another name. } }\n');
  await cli('apply', join(out, 'says.bp'));
  const renamed = await signedIn('ada@example.com');
  const again = await readAbout(renamed, false, 'Under another name.');
  await again.getByRole('navigation', { name: 'Where this is' }).getByText('foreman', { exact: true }).waitFor();
  const now = (await linesOf(again)).join(' | ');
  expect(`its lines to be what it did, under any name, and what it said since, got ${now}`, now.includes('moved shop-1 from backlog to ready') && now.includes('said in #general: Under another name.'));
  await renamed.screenshot({ path: `${out}/botdone-6-renamed.png` });

  // A phone's width, by touch.
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  const small = await readAbout(phone, true, 'Under another name.');
  await apart(small, "at a phone's width");
  await phone.screenshot({ path: `${out}/botdone-7-phone.png` });

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
