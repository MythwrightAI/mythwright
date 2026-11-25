// A card that moves to another column is seen to slide there
// (`src/views/cardslide.ts`), on a page that is never read again while the
// card is moved from a terminal:
//
//   DAEMON=../target/debug/factory node scripts/slid.mjs [out-dir]
//
// What moves it is held half way, the moment it is put on the page (as
// `drawer.mjs` holds the drawer's), so that it is looked at on its way
// whatever the machine is doing: the card, flown over the page as a still
// copy from where it was to where it is, part of the way between its two
// places, by `transform` alone in the drawer's time and easing, with the
// card itself waiting out of sight in its place; the card under it in the
// column it left closing up the same way; and nothing else of the page
// moved. Played on, it is at rest in its column with nothing on it and no
// copy left. At three widths and a phone's, where a column scrolled out of
// sight has the card go at once; and nothing slides under the pointer,
// for a card carried by hand and let go, nor where less movement is asked
// for. A row of a table laid out in columns slides the same way. Plainly,
// and with `LOOK="size=largest density=compact"`.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { sized } from './looked.ts';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'slid-data');
const home = join(out, 'slid-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  // A board and nothing that works its cards: what moves them here is the terminal and the pointer.
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  for (let n = 1; n <= 12; n += 1) await cli('board', 'add', 'shop', `Card number ${n}`, '--status', 'backlog');
  for (let n = 13; n <= 15; n += 1) await cli('board', 'add', 'shop', `Card number ${n}`, '--status', 'ready');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: sized(1600), height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(page);
  await page.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }).click();
  const column = (name) => page.locator('.board > .column', { has: page.locator('.column-head', { hasText: new RegExp(`^${name}`) }) });
  const card = (id) => page.locator(`.board [data-card="${id}"]`);
  await card('shop-15').waitFor();

  // What slides a card is held half way the moment it is on the page, and what is said of it kept as `window.held`.
  await page.evaluate(() => {
    window.held = [];
    const seen = new WeakSet();
    const hold = () => {
      for (const one of document.getAnimations()) {
        if (one.animationName !== 'card-slid' || seen.has(one)) continue;
        seen.add(one);
        const timing = one.effect.getComputedTiming();
        const frames = one.effect.getKeyframes();
        const target = one.effect.target;
        window.held.push({
          id: target.dataset.slides ?? target.dataset.of,
          flying: target.classList.contains('card-flying'),
          took: timing.duration,
          eased: frames[0]?.easing ?? '',
          moved: [...new Set(frames.flatMap((frame) => Object.keys(frame)))].filter((key) => !['offset', 'computedOffset', 'easing', 'composite'].includes(key)).sort(),
        });
        one.pause();
        one.currentTime = timing.duration / 2;
      }
    };
    new MutationObserver(hold).observe(document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ['class'] });
  });
  const held = () => page.evaluate(() => window.held);
  const forget = () => page.evaluate(() => (window.held = []));
  const played = async () => {
    await page.evaluate(() => {
      for (const one of document.getAnimations()) if (one.animationName === 'card-slid') one.finish();
    });
    await page.waitForFunction(() => !document.querySelector('.card-flying, .card-sliding, .card-landing'));
  };
  const boxOf = async (locator) => {
    const box = await locator.boundingBox();
    return box && { x: Math.round(box.x), y: Math.round(box.y), width: Math.round(box.width), height: Math.round(box.height) };
  };
  /** Every card's box and every column head's, by what it is. */
  const whereAll = () =>
    page.evaluate(() => {
      const all = {};
      for (const one of document.querySelectorAll('.board [data-card]')) {
        const box = one.getBoundingClientRect();
        all[one.dataset.card] = [Math.round(box.x), Math.round(box.y)];
      }
      for (const one of document.querySelectorAll('.board .column-head')) {
        const box = one.getBoundingClientRect();
        all[`head:${one.querySelector('.column-name')?.textContent}`] = [Math.round(box.x), Math.round(box.y)];
      }
      return all;
    });
  const between = (from, to, at) => {
    const whole = Math.hypot(to.x - from.x, to.y - from.y);
    return Math.hypot(at.x - from.x, at.y - from.y) > whole * 0.2 && Math.hypot(at.x - to.x, at.y - to.y) > whole * 0.05;
  };

  // ---- moved from a terminal, at three widths -------------------------------------------
  let next = 1;
  for (const width of [1600, 1280, 900]) {
    await page.setViewportSize({ width: sized(width), height: 900 });
    await page.mouse.move(2, 2);
    const moving = `shop-${next}`;
    const under = `shop-${next + 1}`;
    next += 1;
    await card(moving).waitFor();
    const from = await boxOf(card(moving));
    const below = await boxOf(card(under));
    const before = await whereAll();
    await forget();
    await cli('board', 'move', moving, 'ready');
    await column('ready').locator(`[data-card="${moving}"]`).waitFor({ state: 'attached' });
    const said = await held();
    const flown = said.find((one) => one.id === moving);
    expect(
      `at ${width} wide the card to be flown from where it was, by transform alone in the drawer's time and easing, got ${JSON.stringify(said)}`,
      flown?.flying && flown.took >= 240 && flown.took <= 280 && flown.moved.join() === 'transform' && flown.eased.startsWith('cubic-bezier'),
    );
    const copy = page.locator(`.card-flying[data-of="${moving}"]`);
    const onItsWay = await boxOf(copy);
    const landing = await card(moving).evaluate((one) => [getComputedStyle(one).visibility, one.getBoundingClientRect().x, one.getBoundingClientRect().y].map((v) => (typeof v === 'number' ? Math.round(v) : v)));
    const to = { x: landing[1], y: landing[2] };
    expect(`at ${width} wide the copy to be part of the way between ${JSON.stringify(from)} and ${JSON.stringify(to)}, at ${JSON.stringify(onItsWay)}`, onItsWay && between(from, to, onItsWay) && onItsWay.width === from.width);
    expect(`the card itself to wait out of sight in its place, got ${landing[0]}`, landing[0] === 'hidden');
    // The card under it in the column it left closes up the same way, by transform within its column.
    const closing = said.find((one) => one.id === under);
    const halfway = await boxOf(card(under));
    expect(`the card under it to close up, sliding, got ${JSON.stringify(closing)} at ${JSON.stringify(halfway)} from ${JSON.stringify(below)}`, closing && !closing.flying && halfway.y < below.y && halfway.y > from.y);
    // Nothing else of the page moved: every card of the other columns, the ones over it and the heads.
    const now = await whereAll();
    const leftAlone = Object.keys(before).filter((key) => key !== moving && !(key.startsWith('shop-') && Number(key.slice(5)) > next - 1 && Number(key.slice(5)) <= 12));
    const shifted = leftAlone.filter((key) => JSON.stringify(before[key]) !== JSON.stringify(now[key]));
    expect(`nothing else of the page to have moved, and ${shifted.join(', ')} did`, shifted.length === 0);
    await page.screenshot({ path: `${out}/slid-1-on-its-way-${width}.png` });
    await played();
    const rested = await card(moving).evaluate((one) => [getComputedStyle(one).transform, getComputedStyle(one).visibility, one.getAnimations().length]);
    expect(`the card at rest in its column with nothing on it, got ${rested}`, rested.join() === 'none,visible,0' && (await page.locator('.card-flying').count()) === 0);
    expect('it to be where a card that comes is put, at the foot of its column', (await column('ready').locator('[data-card]').last().getAttribute('data-card')) === moving);
    await page.screenshot({ path: `${out}/slid-2-at-rest-${width}.png` });
  }

  // ---- nothing slides under the pointer -------------------------------------------------
  // Pointing at a card of the column a card leaves: it, and the one coming to where it is, are where they are at once.
  await page.setViewportSize({ width: sized(1280), height: 900 });
  const pointed = `shop-${next + 1}`;
  const after = `shop-${next + 2}`;
  await card(pointed).hover();
  await forget();
  await cli('board', 'move', `shop-${next}`, 'ready');
  await column('ready').locator(`[data-card="shop-${next}"]`).waitFor({ state: 'attached' });
  const underPointer = (await held()).filter((one) => one.id === pointed || one.id === after);
  expect(`nothing to slide under the pointer, got ${JSON.stringify(underPointer)}`, underPointer.length === 0 && (await card(pointed).evaluate((one) => one.getAnimations().length)) === 0);
  expect('the card moved still to be flown, away from the pointer', (await held()).some((one) => one.id === `shop-${next}` && one.flying));
  await played();
  next += 1;

  // ---- a card carried by hand lands where it was let go, with no slide ------------------
  await page.mouse.move(2, 2);
  await forget();
  const carried = `shop-${next}`;
  await card(carried).dragTo(column('doing'));
  await column('doing').locator(`[data-card="${carried}"]`).waitFor();
  // And the terminal's word that it moved, which follows, moves nothing either.
  await page.waitForFunction((id) => !document.querySelector(`.board [data-card="${id}"].card-landing`), carried);
  expect(`a card let go by hand to land with no slide, got ${JSON.stringify(await held())}`, !(await held()).some((one) => one.id === carried));
  await played();
  next += 1;

  // ---- at a phone's width, to a column out of sight, it goes at once --------------------
  await page.setViewportSize({ width: 390, height: 844 });
  await card(`shop-${next}`).waitFor();
  await forget();
  await cli('board', 'move', `shop-${next}`, 'done');
  await column('done').locator(`[data-card="shop-${next}"]`).waitFor({ state: 'attached' });
  expect(`a card moved to a column out of sight to go at once, got ${JSON.stringify(await held())}`, !(await held()).some((one) => one.id === `shop-${next}`));
  await played();
  await page.screenshot({ path: `${out}/slid-3-phone.png` });
  next += 1;
  // And between the two columns in sight there, it slides.
  await column('backlog').scrollIntoViewIfNeeded();
  await page.evaluate(() => (document.querySelector('.board').scrollLeft = 0));
  await forget();
  const onPhone = `shop-${next}`;
  await cli('board', 'move', onPhone, 'ready');
  await column('ready').locator(`[data-card="${onPhone}"]`).waitFor({ state: 'attached' });
  const phoneSaid = (await held()).find((one) => one.id === onPhone);
  await page.screenshot({ path: `${out}/slid-4-phone-on-its-way.png` });
  const sideways = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
  expect(`on a phone a card going to the column beside it in sight to be flown, with nothing scrolled sideways, got ${JSON.stringify(phoneSaid)} and ${sideways}`, phoneSaid?.flying && sideways <= 0);
  await played();
  next += 1;

  // ---- a table of the project's laid out in columns: a row moved slides the same way --------
  await writeFile(
    join(out, 'tasks.bp'),
    [
      'blueprint: tasks',
      'description: A table of tasks laid out by where each stands.',
      'permissions: [projects, records]',
      'steps:',
      '  table:',
      '    do: collection.ensure',
      '    with:',
      '      name: tasks',
      '      fields:',
      '        title: { type: text, required: true }',
      '        state: { type: choice, of: [todo, doing, done] }',
      '      views:',
      '        columns: { kind: board, by: state }',
      '',
    ].join('\n'),
  );
  await cli('apply', join(out, 'tasks.bp'), '--project', 'shop');
  for (const n of [1, 2, 3]) await cli('record', 'add', 'shop', 'tasks', `title=Task number ${n}`, 'state=todo');
  await page.setViewportSize({ width: sized(1280), height: 900 });
  await page.mouse.move(2, 2);
  await page.evaluate(() => (location.hash = '#/table/shop/tasks'));
  const row = (id) => page.locator(`.board [data-slides="${id}"]`);
  await row('shop-tasks-3').waitFor();
  const rowFrom = await boxOf(row('shop-tasks-1'));
  await forget();
  await cli('record', 'set', 'shop-tasks-1', 'state=doing');
  await page.locator('.board > .column[aria-label="doing"]').locator('[data-slides="shop-tasks-1"]').waitFor({ state: 'attached' });
  const rowSaid = await held();
  const rowCopy = await boxOf(page.locator('.card-flying[data-of="shop-tasks-1"]'));
  const rowTo = await row('shop-tasks-1').evaluate((one) => ({ x: Math.round(one.getBoundingClientRect().x), y: Math.round(one.getBoundingClientRect().y) }));
  expect(
    `a row of a table laid out in columns flown to its column part of the way, and the one under it closing up, got ${JSON.stringify(rowSaid)} at ${JSON.stringify(rowCopy)} from ${JSON.stringify(rowFrom)} to ${JSON.stringify(rowTo)}`,
    rowSaid.some((one) => one.id === 'shop-tasks-1' && one.flying) && rowSaid.some((one) => one.id === 'shop-tasks-2' && !one.flying) && rowCopy && between(rowFrom, rowTo, rowCopy),
  );
  await page.screenshot({ path: `${out}/slid-6-table-on-its-way.png` });
  await played();

  // ---- where less movement is asked for, nothing slides -----------------------------------
  await page.setViewportSize({ width: sized(1280), height: 900 });
  await page.evaluate(() => (location.hash = '#/board/shop'));
  await card(`shop-${next}`).waitFor();
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await forget();
  await cli('board', 'move', `shop-${next}`, 'ready');
  await column('ready').locator(`[data-card="shop-${next}"]`).waitFor({ state: 'attached' });
  expect(`nothing to slide where less movement is asked for, got ${JSON.stringify(await held())}`, (await held()).length === 0 && (await page.locator('.card-flying, .card-sliding').count()) === 0);
  await page.screenshot({ path: `${out}/slid-5-still.png` });

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
