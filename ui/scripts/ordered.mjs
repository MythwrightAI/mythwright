// A board's cards by what comes first, and in bands by what they are part of.
//
//   DAEMON=../target/debug/factory node scripts/ordered.mjs [out-dir]
//
// A new project's board has a second view beside Board: Order. Here a
// board holds a chain of four cards, a diamond of four, two that wait on
// nothing, two that wait on each other and one that waits on a card that
// is not there; some are done, one is being worked, the chain is part of
// one feature and the diamond of another. The order shows them in columns
// by what can be begun now and what waits on what, with what is done
// folded away and what waits in a ring set apart; in bands by the feature
// each is part of, called what the feature is called; and ungrouped. A
// card pointed at marks what it waits on and what waits on it, and pressed
// it opens as it does on the board. The board's own view is gathered in
// the same bands. At a phone's width the columns scroll inside the view
// and the page does not. Nowhere do two cards lie over each other.
//
// A second board is one nobody has put in order: eighty-eight cards with
// long titles and labels, twenty-two of them done, none waiting on
// another and none part of anything. There the order has one step, drawn
// with no band, its cards in rows across the room there is; and at every
// width of the page from a phone's to a wide screen's, with the side
// panel open and folded, no card is wider than the column it is in. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { unfoldAll } from './folded.mjs';
import { gearLine } from './gear.mjs';
import { saysWhere } from './where.ts';

/** Turns the side panel to a view on the rail, unless it is showing already: Resources with what it has read unfolded, as everything there starts folded (`folded.mjs`). */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  if (name === 'Resources') await unfoldAll(page);
}

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'ordered-data');
const home = join(out, 'ordered-home');

// Bigger pieces of work for cards to be part of: a table and nothing more,
// so that nobody is asked to plan anything here.
const FEATURES = `blueprint: pieces
description: The bigger pieces of work a card may be part of.
permissions: [projects, records]
steps:
  features:
    do: collection.ensure
    with:
      name: features
      fields:
        title: { type: text, required: true }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  // A board as a new project has it, with no fleet to take the cards up and no library.
  await cli('project', 'create', 'Shop', '--without', 'fleet', '--without', 'docs');
  const shop = join(data, 'projects/shop');
  await mkdir(join(shop, 'blueprints'), { recursive: true });
  await writeFile(join(shop, 'blueprints/pieces.bp'), FEATURES);
  await cli('apply', join(shop, 'blueprints/pieces.bp'), '--project', 'shop');
  await cli('record', 'add', 'shop', 'features', 'title=Fit the shop out');
  await cli('record', 'add', 'shop', 'features', 'title=A basket');

  // The cards, each with what it waits on, where it is and what it is part of.
  const cards = [
    ['Lay the floor', [], 'done', 1],
    ['Put the shelves up', [1], 'backlog', 1],
    ['Stock the shelves', [2], 'backlog', 1],
    ['Open the doors', [3], 'backlog', 1],
    ['Draw the basket', [], 'doing', 2],
    ['A page for the basket', [5], 'backlog', 2],
    ['What the basket comes to', [5], 'backlog', 2],
    ['A button to pay', [6, 7], 'backlog', 2],
    ['Write the returns policy', [], 'backlog', 0],
    ['Print the signs', [], 'ready', 0],
    ['Which came first', [12], 'backlog', 0],
    ['The other one', [11], 'backlog', 0],
    ['Hang the lights', [99], 'backlog', 0],
  ];
  for (const [title, , status] of cards) await cli('board', 'add', 'shop', title, '--status', status);
  for (const [nth, [, after, , part]] of cards.entries()) {
    const set = [...(after.length ? [`after=${after.map((one) => `shop-${one}`).join(',')}`] : []), ...(part ? [`part_of=shop-features-${part}`] : [])];
    if (set.length) await cli('board', 'set', `shop-${nth + 1}`, ...set);
  }

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1350, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  // The order is not offered for now (`orderOffered` in `ui/src/table.ts`): this browser is told
  // that it is, before any page of it loads, so that what the order does goes on being held.
  await page.addInitScript(() => {
    try {
      localStorage.setItem('client.order', 'on');
    } catch {
      /* a browser that keeps nothing is not offered it, and this check then fails where it looks for the tab */
    }
  });
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

  await side(page, 'Resources');
  await page.locator('.nav').getByText('Shop', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await page.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }).click();
  await page.locator('.card', { hasText: 'Print the signs' }).waitFor();

  /** No two cards lie over each other, by their own boxes; touching is not lying over. */
  const apart = async (what) => {
    const boxes = await page.locator('.card').evaluateAll((all) => all.map((one) => ({ at: one.getBoundingClientRect().toJSON(), says: one.textContent.slice(0, 30) })));
    for (const [nth, a] of boxes.entries()) {
      for (const b of boxes.slice(nth + 1)) {
        const over = Math.min(a.at.right, b.at.right) - Math.max(a.at.left, b.at.left) > 0.5 && Math.min(a.at.bottom, b.at.bottom) - Math.max(a.at.top, b.at.top) > 0.5;
        expect(`no two cards to lie over each other ${what}: "${a.says}" and "${b.says}" do`, !over);
      }
    }
    return boxes.length;
  };
  /** The page itself does not scroll sideways. */
  const still = async (what) => {
    const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(`the page not to scroll sideways ${what}, and it is ${wide} wider than the window`, wide <= 0);
  };
  const lane = (within, name) => within.locator('.ordered-lane').filter({ has: page.locator('.column-head', { hasText: name }) });
  const inLane = async (within, name) => (await lane(within, name).locator('.card > .card-name').allInnerTexts()).map((one) => one.trim());
  const band = (name) => page.getByRole('region', { name, exact: true });
  // How the board's view is laid out is a line of the gear of its head, and hangs under the gear.
  const layoutPanel = async () => {
    await gearLine(page, /^Layout/);
    return page.getByRole('dialog', { name: 'Layout' });
  };
  const groupBy = async (label) => {
    const panel = await layoutPanel();
    await panel.getByRole('combobox', { name: 'Group by' }).selectOption({ label });
    await page.keyboard.press('Escape');
    await panel.waitFor({ state: 'detached' });
  };

  // A new project's board has the order beside the board, with nothing done to get it.
  const views = page.getByRole('tablist', { name: 'Views' });
  expect(`the board's views to be Board and Order, got ${await views.getByRole('tab').allInnerTexts()}`, (await views.getByRole('tab').allInnerTexts()).map((one) => one.trim()).join() === 'Board,Order');
  await views.getByRole('tab', { name: 'Order', exact: true }).click();
  await page.locator('.ordered').waitFor();

  // In bands by the feature each card is part of, called what the feature is called, with those of none last.
  await band('Fit the shop out').waitFor();
  const bandNames = await page.locator('.bands-band').evaluateAll((all) => all.map((one) => one.getAttribute('aria-label')));
  expect(`the bands to be the two features and those of none, got ${bandNames.join(' | ')}`, bandNames.join('|') === 'Fit the shop out|A basket|Ungrouped');

  // A chain: one a column, and what is done folded away and counted.
  const fit = band('Fit the shop out');
  expect(`the chain's next card to be the one that can be begun, got ${await inLane(fit, 'Can begin now')}`, (await inLane(fit, 'Can begin now')).join() === 'Put the shelves up');
  expect('the one after it to be in Then', (await inLane(fit, 'Then')).join() === 'Stock the shelves');
  expect('the last to be in After that', (await inLane(fit, 'After that')).join() === 'Open the doors');
  const doneHead = fit.getByRole('button', { name: /^Done/ });
  expect(`what is done to be folded away and counted, got "${(await doneHead.innerText()).replace(/\s+/g, ' ')}"`, (await doneHead.getAttribute('aria-expanded')) === 'false' && /Done\s*1/.test(await doneHead.innerText()));
  expect('the card that is done not to be drawn while that is folded', (await page.locator('.card', { hasText: 'Lay the floor' }).count()) === 0);

  // A diamond: the one being worked says where it is, and the two that wait on it stand side by side.
  const basket = band('A basket');
  const drawn = lane(basket, 'Can begin now').locator('.card', { hasText: 'Draw the basket' });
  expect('the card somebody is working to say where it is', (await drawn.locator('.ordered-where').innerText()).trim() === 'doing');
  expect(
    `the head to count only what nobody has begun, and to say which is which, got "${(await lane(basket, 'Can begin now').locator('.ordered-note').innerText()).trim()}"`,
    (await lane(basket, 'Can begin now').locator('.ordered-note').innerText()).trim() === '0 to begin, side by side; 1 more under way (1 doing)',
  );
  expect(`the two in the middle to be side by side, got ${await inLane(basket, 'Then')}`, (await inLane(basket, 'Then')).join() === 'A page for the basket,What the basket comes to');
  expect('the one that waits on both to come after them', (await inLane(basket, 'After that')).join() === 'A button to pay');
  expect('a card to say what it waits on, by what those are called', (await basket.locator('.card', { hasText: 'A button to pay' }).locator('.ordered-waits').innerText()).trim() === 'waits on A page for the basket; What the basket comes to');

  // Of none: two that wait on nothing, one of them given to be worked; one that waits on a card that is not there; and a ring.
  const loose = band('Ungrouped');
  expect(`those that wait on nothing, and the one that waits only on what is not there, to be first, got ${await inLane(loose, 'Can begin now')}`, (await inLane(loose, 'Can begin now')).join() === 'Write the returns policy,Print the signs,Hang the lights');
  expect('one that waits on a card that is not there to say so', (await loose.locator('.card', { hasText: 'Hang the lights' }).locator('.ordered-waits').innerText()).trim() === 'waits on shop-99, which is not here');
  expect(`the two that wait on each other to be set apart, got ${await inLane(loose, 'Waits in a ring')}`, (await inLane(loose, 'Waits in a ring')).join() === 'Which came first,The other one');
  expect('each of them to be marked as in a ring', (await lane(loose, 'Waits in a ring').locator('.badge', { hasText: 'in a ring' }).count()) === 2);
  const sum = (await page.locator('.ordered-sum').innerText()).trim();
  expect(`what is said above them to count what can be begun, got "${sum}"`, sum === '3 cards can be begun now, side by side · 2 more under way · 2 steps after them · 1 done · 2 waiting in a ring');
  expect(`twelve cards to be drawn, got ${await apart('in the order, in bands')}`, (await apart('in the order, in bands')) === 12);
  await still('with the order in bands');
  await page.screenshot({ path: `${out}/ordered-0-in-bands.png` });

  // Pointed at, a card marks what it waits on and what waits on it, and nothing moves.
  const middle = basket.locator('[data-ordered="shop-6"]');
  const before = await middle.boundingBox();
  await middle.hover();
  expect('what it waits on to be marked', (await basket.locator('.card.ordered-first').innerText()).includes('Draw the basket'));
  expect('what waits on it to be marked', (await basket.locator('.card.ordered-next').innerText()).includes('A button to pay'));
  const after = await middle.boundingBox();
  expect('the card not to move for being pointed at', before.x === after.x && before.y === after.y && before.width === after.width && before.height === after.height);
  await page.screenshot({ path: `${out}/ordered-1-pointed-at.png` });

  // Pressed, it opens as it does on the board; and it is not something to carry.
  expect('a card of the order not to be something that is dragged', (await middle.getAttribute('draggable')) !== 'true');
  await middle.click();
  const drawer = drawerAtRest(page);
  await drawer.waitFor();
  expect('the card pressed to be the one opened', (await drawer.getByLabel('Title', { exact: true }).inputValue()) === 'A page for the basket');
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });

  // What is done is unfolded, for every band at once, and stays so when the page is read again.
  await doneHead.click();
  await fit.locator('.card', { hasText: 'Lay the floor' }).waitFor();
  expect('what is done to be drawn as done', (await fit.locator('.card.ordered-over').count()) === 1);
  await apart('with what is done shown');
  await page.screenshot({ path: `${out}/ordered-2-done-shown.png` });

  // A band is folded, and that is remembered with the view.
  await page.getByRole('button', { name: /A basket/ }).click();
  expect('a folded band to draw none of its cards', (await page.locator('.card', { hasText: 'A button to pay' }).count()) === 0);
  await page.reload();
  await page.locator('.ordered').waitFor();
  await band('Fit the shop out').locator('.card', { hasText: 'Lay the floor' }).waitFor();
  expect('the band to be folded still when the page is read again', (await page.getByRole('button', { name: /A basket/ }).getAttribute('aria-expanded')) === 'false');
  await page.screenshot({ path: `${out}/ordered-3-a-band-folded.png` });
  await page.getByRole('button', { name: /A basket/ }).click();
  await fit.getByRole('button', { name: /^Done/ }).click();

  // Gathered by nothing, the same cards are one set of columns: the first holds what can be worked side by side.
  await groupBy('nothing');
  await page.locator('.bands-band').first().waitFor({ state: 'detached' });
  const all = page.locator('.ordered');
  expect(`everything that waits on nothing to be side by side, got ${await inLane(all, 'Can begin now')}`, (await inLane(all, 'Can begin now')).join() === 'Put the shelves up,Draw the basket,Write the returns policy,Print the signs,Hang the lights');
  expect(
    `the head to say how many of them nobody has begun, got "${(await lane(all, 'Can begin now').locator('.ordered-note').innerText()).trim()}"`,
    (await lane(all, 'Can begin now').locator('.ordered-note').innerText()).trim() === '3 to begin, side by side; 2 more under way (1 doing, 1 ready)',
  );
  expect('the count on the head to be of those nobody has begun', (await lane(all, 'Can begin now').locator('[data-ordered-count]').innerText()).trim() === '3');
  await apart('in the order, ungrouped');
  await still('with the order ungrouped');
  await page.screenshot({ path: `${out}/ordered-4-ungrouped.png` });

  // At a phone's width the columns scroll inside the view, and the page does not.
  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator('.ordered').waitFor();
  await still('at 390 wide, ungrouped');
  await apart('at 390 wide, ungrouped');
  const inside = await page.locator('.ordered').evaluate((view) => view.scrollWidth - view.clientWidth);
  expect(`the columns to scroll sideways inside the view at 390 wide, and they are ${inside} wider than it`, inside > 0);
  await page.screenshot({ path: `${out}/ordered-5-ungrouped-at-390.png` });
  await page.setViewportSize({ width: 1350, height: 860 });
  await groupBy('Part of');
  await band('Fit the shop out').waitFor();
  await page.setViewportSize({ width: 390, height: 844 });
  await band('A basket').waitFor();
  await still('at 390 wide, in bands');
  await apart('at 390 wide, in bands');
  await page.screenshot({ path: `${out}/ordered-6-in-bands-at-390.png` });
  await page.setViewportSize({ width: 1350, height: 860 });

  // The board's own view is gathered the same way: its columns, a band a feature.
  await views.getByRole('tab', { name: 'Board', exact: true }).click();
  await page.locator('.board').first().waitFor();
  expect('the board to be in no bands until it is asked', (await page.locator('.bands-band').count()) === 0);
  await groupBy('Part of');
  await band('A basket').waitFor();
  expect(`the board's bands to be the same, got ${await page.locator('.bands-band').evaluateAll((every) => every.map((one) => one.getAttribute('aria-label')).join('|'))}`, (await page.locator('.bands-band').count()) === 3);
  expect('a band of the board to hold its own cards in their columns', (await band('A basket').locator('.column').filter({ has: page.locator('.column-head', { hasText: 'doing' }) }).locator('.card').innerText()).includes('Draw the basket'));
  expect(`every card to be on the board once, got ${await apart('on the board in bands')}`, (await apart('on the board in bands')) === 13);
  await still('with the board in bands');
  await page.screenshot({ path: `${out}/ordered-7-the-board-in-bands.png` });
  // And a card is still carried from one column to another there, within its band.
  expect('a card of the board to be something that is dragged', (await band('A basket').locator('.card').first().getAttribute('draggable')) === 'true');
  await groupBy('nothing');
  await page.locator('.bands-band').first().waitFor({ state: 'detached' });

  // Cards are marked with labels: one holds two, most hold none.
  for (const [card, labels] of [[1, 'landed'], [2, 'before-preview,in-progress'], [3, 'before-preview'], [5, 'in-progress'], [9, 'before-preview']]) {
    await cli('board', 'set', `shop-${card}`, `labels=${labels}`);
  }
  await page.locator('[data-card="shop-9"]', { hasText: 'before-preview' }).waitFor();

  // Gathered by label: a band for each, by name, a card that holds two in both, and those that hold none last.
  await groupBy('Labels');
  await band('before-preview').waitFor();
  const byLabel = await page.locator('.bands-band').evaluateAll((every) => every.map((one) => one.getAttribute('aria-label')).join('|'));
  expect(`the bands to be the labels by name and those with none, got ${byLabel}`, byLabel === 'before-preview|in-progress|landed|Ungrouped');
  expect('the card that holds two labels to be in both bands', (await band('before-preview').locator('[data-card="shop-2"]').count()) === 1 && (await band('in-progress').locator('[data-card="shop-2"]').count()) === 1);
  expect(`every card to be drawn once for each label it holds, got ${await apart('on the board by label')}`, (await apart('on the board by label')) === 14);
  await still('with the board by label');
  await page.screenshot({ path: `${out}/ordered-11-the-board-by-label.png` });
  // It is one card: carried to another column in one band, it is there in the other too, and holds what it held.
  const column = (within, name) => within.locator('.column').filter({ has: page.locator('.column-head', { hasText: name }) });
  await band('before-preview').locator('[data-card="shop-2"]').dragTo(column(band('before-preview'), 'ready'));
  await column(band('in-progress'), 'ready').locator('[data-card="shop-2"]').waitFor();
  await column(band('before-preview'), 'ready').locator('[data-card="shop-2"]').waitFor();
  expect('a card moved in one band to be moved, not copied', (await page.locator('[data-card="shop-2"]').count()) === 2);
  await groupBy('nothing');
  await page.locator('.bands-band').first().waitFor({ state: 'detached' });

  // Held to a label: only the cards that hold it, said in the view's head, counted, and cleared with one press.
  const only = async (label, on) => {
    const panel = await layoutPanel();
    const turn = panel.getByRole('switch', { name: `Only Labels ${label}`, exact: true });
    if ((await turn.isChecked()) !== on) await turn.click();
    await page.keyboard.press('Escape');
    await panel.waitFor({ state: 'detached' });
  };
  const held = page.getByRole('status', { name: 'What this view is held to' });
  const said = async () => (await held.locator('span').first().innerText()).replace(/\s+/g, ' ').trim();
  const counted = async () => (await page.locator('.column-head').evaluateAll((heads) => heads.map((head) => Number(head.querySelector('.faint')?.textContent ?? 0)))).reduce((sum, one) => sum + one, 0);
  await only('before-preview', true);
  await held.waitFor();
  expect(`the head to say what the view is held to, got "${await said()}"`, (await said()) === 'Showing only cards with labels before-preview: 3 of 13.');
  expect('only the cards that hold the label to be drawn', (await page.locator('.board .card').count()) === 3 && (await page.locator('[data-card="shop-1"]').count()) === 0);
  expect(`the columns to count what is shown, got ${await counted()}`, (await counted()) === 3);
  expect('the head of the page to count them too', (await page.locator('.view-head .muted').first().innerText()).trim() === '3 of 13 cards');
  await page.screenshot({ path: `${out}/ordered-12-held-to-a-label.png` });
  await only('in-progress', true);
  await page.locator('[data-card="shop-5"]').waitFor();
  expect(`two labels to show the cards that hold either, got "${await said()}"`, (await said()) === 'Showing only cards with labels before-preview or in-progress: 4 of 13.' && (await page.locator('.board .card').count()) === 4);
  await held.getByRole('button', { name: 'Show all' }).click();
  await held.waitFor({ state: 'detached' });
  await page.locator('[data-card="shop-1"]').waitFor();
  expect('every card to be back', (await page.locator('.board .card').count()) === 13);

  // A view held to a label is a tab of its own, and is held still when the page is read again.
  await page.getByRole('button', { name: 'Add a view' }).click();
  await page.getByRole('menuitem', { name: /^Board/ }).click();
  await views.getByRole('tab', { name: 'Board 2', exact: true, selected: true }).waitFor();
  await only('before-preview', true);
  await held.waitFor();
  const renaming = await layoutPanel();
  await renaming.getByLabel('What this view is called').fill('Before preview');
  await page.keyboard.press('Enter');
  await views.getByRole('tab', { name: 'Before preview', exact: true, selected: true }).waitFor();
  await page.keyboard.press('Escape');
  await renaming.waitFor({ state: 'detached' });
  await page.reload();
  await views.getByRole('tab', { name: 'Before preview', exact: true, selected: true }).waitFor();
  await held.waitFor();
  expect(`the saved view to be held still, got "${await said()}"`, (await said()) === 'Showing only cards with labels before-preview: 3 of 13.' && (await page.locator('.board .card').count()) === 3);
  const tabsNow = (await views.getByRole('tab').allInnerTexts()).map((one) => one.trim()).join();
  expect(`the view held to a label to be a tab beside the board and the order, got ${tabsNow}`, tabsNow === 'Board,Order,Before preview');
  await apart('held to a label');
  await page.screenshot({ path: `${out}/ordered-13-a-tab-held-to-a-label.png` });
  await page.setViewportSize({ width: 390, height: 844 });
  await held.waitFor();
  await still('at 390 wide, held to a label');
  await apart('at 390 wide, held to a label');
  await page.screenshot({ path: `${out}/ordered-14-held-at-390.png` });
  await page.setViewportSize({ width: 1350, height: 860 });

  // The order is held the same way: a card keeps its place by all of them, and what is said is of what is shown.
  await views.getByRole('tab', { name: 'Order', exact: true }).click();
  await page.locator('.ordered').waitFor();
  await groupBy('nothing');
  await only('before-preview', true);
  await held.waitFor();
  const heldSum = (await page.locator('.ordered-sum').innerText()).trim();
  expect(`the order to say what is shown, got "${heldSum}"`, heldSum === '1 card can be begun now, side by side · 1 more under way · 1 step after them · 0 done');
  expect('a card shown to say what it waits on still', (await page.locator('[data-ordered="shop-3"] .ordered-waits').innerText()).trim() === 'waits on Put the shelves up');
  await apart('in the order held to a label');
  await page.screenshot({ path: `${out}/ordered-15-the-order-held-to-a-label.png` });
  await held.getByRole('button', { name: 'Show all' }).click();
  await held.waitFor({ state: 'detached' });

  // A label has a colour, worked out from what it is called and kept nowhere: the same on every card, in the
  // order and on the board, at a band's head, where a view is held to it and where it is offered to be.
  const SIX = ['before-preview', 'bug', 'in-progress', 'landed', 'soon', 'urgent'];
  for (const [card, labels] of [[4, 'soon'], [6, 'bug,urgent'], [7, 'urgent'], [8, 'soon,bug'], [10, 'landed']]) {
    await cli('board', 'set', `shop-${card}`, `labels=${labels}`);
  }
  await page.locator('[data-ordered="shop-10"] .tag', { hasText: 'landed' }).waitFor();
  // Each label drawn in a place, with the colours it is drawn in there: one, if it is always the same.
  const tints = (where) =>
    page.locator(`${where} .tag`).evaluateAll((every) => {
      const by = {};
      for (const one of every) (by[one.textContent.trim()] ??= new Set()).add(`${[...one.classList].find((name) => /^tag-\d$/.test(name))} ${getComputedStyle(one).backgroundColor}`);
      return Object.fromEntries(Object.entries(by).map(([label, drawn]) => [label, [...drawn].join(' and ')]));
    });
  const same = async (where, as, labels = SIX) => {
    const drawn = await tints(where);
    for (const label of labels) expect(`${label} to be the colour ${where} that it is in the order (${as[label]}), got ${drawn[label]}`, drawn[label] === as[label]);
  };
  const inOrder = await tints('.ordered');
  expect(`the six labels to be drawn in the order, got ${Object.keys(inOrder).sort().join()}`, Object.keys(inOrder).sort().join() === SIX.join());
  expect(`six labels to be six colours, and none the plain one, got ${JSON.stringify(inOrder)}`, new Set(Object.values(inOrder)).size === 6 && Object.values(inOrder).every((one) => /^tag-[1-8] /.test(one)));
  await views.getByRole('tab', { name: 'Board', exact: true }).click();
  await page.locator('.board').first().waitFor();
  await same('.board', inOrder);
  await still('with labels in colour');
  await page.screenshot({ path: `${out}/ordered-16-labels-in-colour.png` });
  await groupBy('Labels');
  await band('urgent').waitFor();
  await same('.bands-head', inOrder);
  expect('a band of cards that hold no label to be headed in plain words', (await band('Ungrouped').locator('.bands-head .tag').count()) === 0);
  await page.screenshot({ path: `${out}/ordered-17-labels-in-colour-by-label.png` });
  const offered = await layoutPanel();
  await offered.getByRole('switch', { name: 'Only Labels urgent', exact: true }).waitFor();
  await same('[role="dialog"] [aria-label="Only by Labels"]', inOrder);
  expect('the name beside what records are gathered by to have its room', await offered.locator('.layout-option', { hasText: 'Group by' }).locator('span').first().evaluate((name) => name.scrollWidth <= name.clientWidth && name.getBoundingClientRect().height < 24));
  await offered.getByRole('switch', { name: 'Only Labels urgent', exact: true }).click();
  await page.screenshot({ path: `${out}/ordered-18-labels-where-a-view-is-held.png` });
  await page.keyboard.press('Escape');
  await offered.waitFor({ state: 'detached' });
  await held.waitFor();
  await same('.held', inOrder, ['urgent']);
  expect(`what the view is held to to be said in words still, got "${await said()}"`, (await said()) === 'Showing only cards with labels urgent: 2 of 13.');
  await held.getByRole('button', { name: 'Show all' }).click();
  await held.waitFor({ state: 'detached' });
  // A card's labels are picked from those its board's cards already hold, not typed again for each card.
  await page.locator('.board [data-card="shop-13"]').click();
  const opened = drawerAtRest(page);
  await opened.waitFor();
  await opened.getByRole('combobox', { name: 'Labels' }).click();
  const labelled = opened.getByRole('listbox', { name: 'Labels options' });
  await labelled.waitFor();
  const offeredLabels = (await labelled.getByRole('option').allInnerTexts()).map((one) => one.trim());
  expect(`the labels on the board's cards to be offered on a card that holds none, got ${offeredLabels.join()}`, offeredLabels.join() === SIX.join());
  await labelled.getByRole('option', { name: 'urgent', exact: true }).click();
  await opened.locator('.select-field .tag', { hasText: 'urgent' }).waitFor();
  await page.screenshot({ path: `${out}/ordered-18b-a-label-picked-on-a-card.png` });
  await opened.getByRole('button', { name: 'Save', exact: true }).click();
  await opened.waitFor({ state: 'detached' });
  await page.locator('.board [data-card="shop-13"] .tag', { hasText: 'urgent' }).waitFor();
  // In the theme's other light the colours are that light's own, and a label is still one colour wherever it is.
  // What is drawn is read once the page has followed (`lit.mjs`).
  const started = process.env.SCHEME ?? 'dark';
  await litAs(page, started === 'light' ? 'dark' : 'light');
  const inLight = await tints('.board');
  expect(`the colours in the theme's other light to be its own, got ${JSON.stringify(inLight)}`, SIX.every((label) => inLight[label] !== inOrder[label] && inLight[label]?.split(' ')[0] === inOrder[label].split(' ')[0]));
  expect('six labels to be six colours there too', new Set(SIX.map((label) => inLight[label])).size === 6);
  await same('.bands-head', inLight);
  await page.screenshot({ path: `${out}/ordered-19-labels-in-a-light-theme-by-label.png` });
  await groupBy('nothing');
  await page.locator('.bands-band').first().waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/ordered-20-labels-in-a-light-theme.png` });
  await views.getByRole('tab', { name: 'Order', exact: true }).click();
  await page.locator('.ordered').waitFor();
  await same('.ordered', inLight);
  await page.screenshot({ path: `${out}/ordered-21-the-order-in-a-light-theme.png` });
  await litAs(page, started);
  await groupBy('Part of');
  await views.getByRole('tab', { name: 'Board', exact: true }).click();
  await page.locator('.board').first().waitFor();

  // A card is added from the order as from the board, and waiting on nothing it can be begun.
  await views.getByRole('tab', { name: 'Order', exact: true }).click();
  await page.locator('.ordered').waitFor();
  await gearLine(page, /^Add a card/);
  await page.locator('.ordered-above input').fill('Sweep up');
  await page.keyboard.press('Enter');
  await lane(page.locator('.ordered'), 'Can begin now').locator('.card', { hasText: 'Sweep up' }).waitFor();
  expect('where it was written to be put away once it is added', (await page.locator('.ordered-above').count()) === 0);

  // A board nobody has put in order: many cards, long titles, labels, and nothing waiting on anything.
  await cli('project', 'create', 'Works', '--without', 'fleet', '--without', 'docs');
  const long = [
    "A card's history says who really added it: the blueprint and the agent, not whoever applied the fleet",
    'Client: wherever the id of something running is shown, say what it is and open it',
    'Sweep up',
    'An easy way to give an agent what it says it lacks (leave to apply blueprints, first)',
  ];
  for (let nth = 0; nth < 88; nth++) await cli('board', 'add', 'works', `${long[nth % long.length]} (${nth + 1})`, '--status', nth < 22 ? 'done' : nth % 9 === 0 ? 'ready' : 'backlog');
  for (let nth = 0; nth < 88; nth += 3) await cli('board', 'set', `works-${nth + 1}`, `labels=${nth < 22 ? 'landed' : nth % 2 ? 'before-preview,in-progress,soon' : 'soon'}`);
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Works', { exact: true }).click();
  await page.getByRole('heading', { name: 'Works', level: 1 }).waitFor();
  await page.locator('.nav [data-project="works"]').getByText('Board', { exact: true }).click();
  await page.getByRole('heading', { name: 'Works board', level: 1 }).waitFor();
  await views.getByRole('tab', { name: 'Order', exact: true }).click();
  await page.locator('.ordered').waitFor();
  await page.locator('.ordered .card').first().waitFor();

  // Nothing is grouped there, so no band is drawn; and there is one step, which has the room.
  expect('no band to be drawn where no card is part of anything', (await page.locator('.bands-band').count()) === 0 && (await page.locator('.bands-head').count()) === 0);
  const begin = lane(page.locator('.ordered'), 'Can begin now');
  expect(`the head to count what can be begun, got "${(await begin.locator('.ordered-note').innerText()).trim()}"`, (await begin.locator('.ordered-note').innerText()).trim() === '59 to begin, side by side; 7 more under way (7 ready)');
  expect('a card to show its labels here as it does on the board', (await begin.locator('.card', { hasText: '(25)' }).locator('.tag').allInnerTexts()).map((one) => one.trim()).join() === 'soon');

  /** Every card is inside the column it is in, by their boxes; and how many cards stand side by side in the first row. */
  const within = async (what) => {
    const found = await page.locator('.ordered .column').evaluateAll((columns) =>
      columns.flatMap((column) => {
        const box = column.getBoundingClientRect();
        return [...column.querySelectorAll('.card')].map((card) => {
          const at = card.getBoundingClientRect();
          return { says: card.textContent.slice(0, 40), out: Math.max(box.left - at.left, at.right - box.right), top: at.top, left: at.left };
        });
      }),
    );
    for (const card of found) expect(`no card to be wider than its column ${what}: "${card.says}" is out of it by ${card.out}`, card.out <= 0.5);
    const first = Math.min(...found.map((card) => card.top));
    return new Set(found.filter((card) => Math.abs(card.top - first) < 1).map((card) => Math.round(card.left))).size;
  };
  const folded = async (fold) => {
    const button = page.getByRole('button', { name: 'Resources', exact: true });
    if (((await button.getAttribute('aria-pressed')) === 'true') === fold) await button.click();
  };
  for (const fold of [false, true]) {
    await page.setViewportSize({ width: 1350, height: 860 });
    await folded(fold);
    for (let wide = 390; wide <= 1410; wide += 60) {
      await page.setViewportSize({ width: wide, height: 860 });
      const what = `at ${wide} wide, the side panel ${fold ? 'folded' : 'open'}`;
      await page.locator('.ordered .card').first().waitFor();
      const across = await within(what);
      await apart(what);
      await still(what);
      // With one step there is nothing to scroll sideways to: the view is as wide as the page gives it.
      const past = await page.locator('.ordered').evaluate((view) => view.scrollWidth - view.clientWidth);
      expect(`the one step to fit the view ${what}, and it is ${past} wider`, past <= 0);
      // Where there is room for more than one card across, they stand across.
      const room = await begin.evaluate((column) => column.getBoundingClientRect().width);
      expect(`the cards to stand in as many rows across as there is room for ${what}: ${across} across in ${Math.round(room)}`, across === Math.max(1, Math.floor((room - 16 + 7) / (220 + 7))));
      if (fold && wide === 870) await page.screenshot({ path: `${out}/ordered-8-not-in-order-at-870.png` });
      if (fold && wide === 1350) await page.screenshot({ path: `${out}/ordered-9-not-in-order-at-1350.png` });
    }
  }
  // What is done is shown beside it, a card to a line, and still nothing is out of its column; and it scrolls within the view.
  await page.setViewportSize({ width: 850, height: 860 });
  await page.getByRole('button', { name: /^Done/ }).click();
  await page.locator('.ordered .card.ordered-over').first().waitFor();
  await within('at 850 wide with what is done shown');
  await apart('at 850 wide with what is done shown');
  await still('at 850 wide with what is done shown');
  expect('a column longer than the page to scroll inside the view', await page.locator('.ordered').evaluate((view) => view.scrollHeight > view.clientHeight));
  await page.screenshot({ path: `${out}/ordered-10-not-in-order-done-shown-at-850.png` });
  await page.getByRole('button', { name: /^Done/ }).click();
  await page.setViewportSize({ width: 1350, height: 860 });
  await folded(false);

  // The same cards as a table: a label in a cell is the tag it is on a card, in the colour it was in the other project.
  await page.getByRole('button', { name: 'Add a view' }).click();
  await page.getByRole('menuitem', { name: /^Table/ }).click();
  await page.locator('.cell-tags .tag', { hasText: 'soon' }).first().waitFor();
  await same('.cell-tags', inOrder, ['before-preview', 'in-progress', 'landed', 'soon']);
  await page.screenshot({ path: `${out}/ordered-22-labels-in-a-table.png` });

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
