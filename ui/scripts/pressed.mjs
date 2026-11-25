// A button on a card: what a card's column offers to be pressed, a row of
// the fleet's `actions`, and a card held in its column until it is.
//
//   DAEMON=../target/debug/factory node scripts/pressed.mjs [out-dir]
//
// Ada owns the factory and applied the shop's fleet, whose one worker is a
// stand-in that does what it is asked. Nell registered by an invitation.
// Two pages of Ada's are opened once and never read again. The fleet's
// tables, listed in the gear of the board's head, have `actions` among
// them with no row; a row is added there for ready, Signed, holding each
// time a card comes. A card made ready from a terminal is not taken up:
// on the board it says it waits for Signed, and opened, that it waits
// until Signed is pressed, with the button. Pressed on the second page,
// the first follows: the card is taken up and worked, and its drawer
// says who pressed it. Sent back to ready from a terminal, it waits for
// Signed again, and pressed from the terminal it is taken up again.
// Nell is offered no button for what takes `admin`, and the route
// refuses her. Nothing of what a card is offered lies over anything else
// at three widths and a phone's. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { tablesRead } from './gear.mjs';
import { saysWhere } from './where.ts';
import { unfoldAll } from './folded.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'pressed-data');
const home = join(out, 'pressed-home');
const hers = join(out, 'pressed-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
// Does what it is asked, and says so.
const works = join(home, 'works.sh');
await writeFile(works, "#!/bin/sh\ncat >/dev/null\necho floor >> FLOOR.md\necho 'I laid the floor.'\n");
await chmod(works, 0o755);
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off', '--command', `works=${works}`], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('record', 'set', 'shop-agents-2', 'adapter=works');
  await cli('board', 'add', 'shop', 'Lay the floor');
  await cli('board', 'add', 'shop', 'Put the shelves up');

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
  const toBoard = async (page, tap = false) => {
    const press = (one) => (tap ? one.tap() : one.click());
    if (tap) await page.getByRole('button', { name: 'Expand the side panel' }).tap();
    await sideSettled(page);
    // The side panel comes back showing what it last showed: pressing that on the rail would put it away again.
    const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await press(resources);
    // Everything the Resources panel lists starts folded (`folded.mjs`).
    await unfoldAll(page);
    // Unfolded by its fold, which opens nothing: pressed by its name on a phone, a project's page has the screen.
    const unfold = page.getByRole('button', { name: 'Unfold Shop', exact: true });
    if ((await unfold.count()) > 0) await press(unfold);
    await press(page.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }));
    await page.locator('.column', { hasText: 'ready' }).first().waitFor();
  };
  const tile = (page, id) => page.locator(`.card[data-card="${id}"]`);
  const columnOf = (page, id) => tile(page, id).evaluate((card) => card.closest('.column')?.querySelector('.column-name span')?.textContent ?? '');
  const inColumn = async (page, id, column, why) => {
    await page
      .waitForFunction(([card, wanted]) => document.querySelector(`.card[data-card="${card}"]`)?.closest('.column')?.querySelector('.column-name span')?.textContent === wanted, [id, column], { timeout: 30000 })
      .catch(async () => {
        throw new Error(`expected ${id} to be in ${column} ${why}, it is in ${await columnOf(page, id).catch(() => 'nowhere')}`);
      });
  };
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
    expect(`${what}: something to be there`, found.count > 0);
    expect(`${what}: nothing over anything else or outside its place, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px of the page and ${found.within}px of itself`, found.sideways <= 0 && found.within <= 0);
  };
  const ASKED = '.card-actions > .faint, .card-action strong, .card-action .faint, .card-action button';

  // Two pages of Ada's, each opened once and never read again from here to the end.
  const page = await signedIn('ada@example.com');
  const other = await signedIn('ada@example.com');
  let loads = 0;
  for (const one of [page, other]) one.on('load', () => (loads += 1));
  await toBoard(page);
  await toBoard(other);

  // ── What a column offers is a table beside the board, with no row ──
  const empty = await tablesRead(page);
  expect(`the gear of the board's head to list what is pressed among its tables with no row, got ${empty.join(' | ')}`, empty.includes('Actions no actions yet'));
  await page.keyboard.press('Escape');
  expect('no table of what is pressed under the project in the side panel', (await page.locator('.nav [data-project="shop"]').getByText('Actions', { exact: true }).count()) === 0);
  // A row added for ready, Signed, that holds a card each time it comes; and one that takes `admin` and holds nothing.
  await cli('record', 'add', 'shop', 'actions', 'title=Signed', 'column=ready', 'who=person', 'holds=true', 'again=true');
  await cli('record', 'add', 'shop', 'actions', 'title=Looked over', 'column=ready', 'scope=admin', 'says=looked it over');
  for (let tries = 0; ; tries += 1) {
    const tables = await tablesRead(page);
    await page.keyboard.press('Escape');
    if (tables.includes('Actions 2 actions')) break;
    if (tries > 200) throw new Error(`expected the gear to say the two rows of what is pressed, got ${tables.join(' | ')}`);
  }

  // ── A card made ready is not taken up, and says what it waits for ──
  await cli('board', 'move', 'shop-1', 'ready');
  await tile(page, 'shop-1').locator('.card-waits', { hasText: 'Waits for Signed' }).waitFor({ timeout: 30000 });
  await tile(other, 'shop-1').locator('.card-waits', { hasText: 'Waits for Signed' }).waitFor({ timeout: 30000 });
  expect(`the card to stay in ready, it is in ${await columnOf(page, 'shop-1')}`, (await columnOf(page, 'shop-1')) === 'ready');
  await page.screenshot({ path: `${out}/pressed-1-waits.png` });
  await tile(page, 'shop-1').click();
  const drawer = drawerAtRest(page);
  const asked = drawer.getByRole('group', { name: 'What this column asks of it' });
  await asked.waitFor();
  expect(`the drawer to say what it waits for, got "${await asked.innerText()}"`, (await asked.innerText()).includes('It waits here until Signed is pressed'));
  await asked.getByRole('button', { name: 'Signed', exact: true }).waitFor();
  await asked.getByRole('button', { name: 'Looked over', exact: true }).waitFor();
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await apart(asked, ASKED, `what the card is offered at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/pressed-2-drawer.png` });

  // ── Pressed on the second page: the first follows, and the card is taken up ──
  await tile(other, 'shop-1').click();
  const theirs = drawerAtRest(other).getByRole('group', { name: 'What this column asks of it' });
  await theirs.getByRole('button', { name: 'Signed', exact: true }).click();
  await asked.locator('.card-action-done', { hasText: 'Signed pressed by you' }).waitFor({ timeout: 30000 });
  await page.screenshot({ path: `${out}/pressed-3-pressed.png` });
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });
  await inColumn(page, 'shop-1', 'review', 'once Signed was pressed on the other page');
  expect('a card taken up to say it waits for nothing', (await tile(page, 'shop-1').locator('.card-waits').count()) === 0);

  // ── Sent back to ready, it waits again; pressed from a terminal, it is taken up again ──
  await cli('board', 'move', 'shop-1', 'ready', '--note', 'Not level.');
  await tile(page, 'shop-1').locator('.card-waits', { hasText: 'Waits for Signed' }).waitFor({ timeout: 30000 });
  await page.screenshot({ path: `${out}/pressed-4-again.png` });
  const pressed = (await cli('board', 'press', 'shop-1', 'signed')).stdout;
  expect(`the terminal to say the card is held no more, got "${pressed.trim()}"`, pressed.includes('nothing holds it where it is now'));
  await inColumn(page, 'shop-1', 'review', 'once Signed was pressed from the terminal');

  // ── Somebody who may not press is offered no button, and refused ──
  await cli('board', 'move', 'shop-2', 'ready');
  const herPage = await signedIn('nell@example.com');
  await toBoard(herPage);
  await tile(herPage, 'shop-2').locator('.card-waits', { hasText: 'Waits for Signed' }).waitFor({ timeout: 30000 });
  await tile(herPage, 'shop-2').click();
  const offered = drawerAtRest(herPage).getByRole('group', { name: 'What this column asks of it' });
  await offered.getByRole('button', { name: 'Signed', exact: true }).waitFor();
  expect('no button for what takes `admin`', (await offered.getByRole('button', { name: 'Looked over', exact: true }).count()) === 0);
  expect(`who may press it to be said, got "${await offered.innerText()}"`, (await offered.innerText()).includes('Looked over is pressed by anyone who may change it who holds admin'));
  const refused = await nell('board', 'press', 'shop-2', 'looked-over').then(
    () => '',
    (err) => String(err.stderr),
  );
  expect(`the route to refuse her, got "${refused.trim()}"`, refused.includes('which you do not hold'));
  await herPage.screenshot({ path: `${out}/pressed-5-member.png` });

  // ── At a phone's width ──
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await toBoard(phone, true);
  await tile(phone, 'shop-2').locator('.card-waits').waitFor({ timeout: 30000 });
  await tile(phone, 'shop-2').tap();
  const small = drawerAtRest(phone).getByRole('group', { name: 'What this column asks of it' });
  await small.getByRole('button', { name: 'Signed', exact: true }).waitFor();
  await apart(small, ASKED, "what the card is offered at a phone's width");
  await phone.screenshot({ path: `${out}/pressed-6-phone.png` });

  expect(`Ada's two pages never to have been read again, were ${loads} times`, loads === 0);
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
