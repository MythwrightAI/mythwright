// How wide a table's columns are: dragged by the edge at the right of a
// column's head, and saved with the view for everybody.
//
//   DAEMON=../target/debug/factory node scripts/widths.mjs [out-dir]
//
// `headed.mjs` runs this once it is done. A project has a table of stock
// wide enough to be scrolled sideways, with two of its columns under a
// heading. Held here:
//
//   - each head has an edge at its right for whoever may save a view and
//     nothing more to press: the heads are each one button still;
//   - a press on the edge is the edge's alone: held and moved it neither
//     carries the column nor opens its menu, and the click after it opens
//     nothing; a press beside it is the head's, as it always was;
//   - while it is held the column follows the pointer, every cell of it,
//     nobody is told, nothing is saved and nothing else moves but what
//     stands after it; let go it is saved once, by the size of the
//     reader's text, stays as wide as it was let go, and is seen on a
//     second open page that is not read again; read again it is as wide;
//   - never narrower than the narrowest a column may be; Escape while it is
//     held, and a pointer let go where it began, put it back with nothing
//     saved; pressed twice it is as wide as what is in it, saved as no
//     width, on the other page too;
//   - the keyboard reaches the edge, and the arrows make it a step wider or
//     narrower, each step saved as it lands;
//   - frozen columns follow a width while it is held and after, and the
//     heading over two columns stays over them;
//   - somebody who may not save a view sees the widths and no edge;
//   - a phone's width, where nothing is offered and the widths are not
//     drawn; and the other of dark and light.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { unfoldAll } from './folded.mjs';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'widths-data');
const home = join(out, 'widths-home');
const NARROWEST = 60;
const STEP = 16;

const TABLES = `blueprint: stock
description: A table wide enough to be scrolled sideways, two of its columns under a heading.
permissions: [projects, board]
steps:
  stock:
    do: collection.ensure
    with:
      name: stock
      fields:
        title: { type: text, required: true }
        status: { type: choice, of: [todo, doing, done] }
        points: { type: number }
        aisle: { type: text }
        shelf: { type: text }
        maker: { type: text }
        note: { type: text }
        zone: { type: text }
        bin: { type: text }
      groups: [{ Where: [aisle, shelf] }]
  ledger:
    do: collection.ensure
    with:
      name: ledger
      takes: admin
      fields:
        title: { type: text, required: true }
        sum: { type: number }
`;

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
  await cli('project', 'create', '--bare', 'Shop');
  const shop = join(data, 'projects/shop');
  const plan = join(shop, 'blueprints/stock.bp');
  await mkdir(join(shop, 'blueprints'), { recursive: true });
  await writeFile(plan, TABLES);
  await cli('apply', plan, '--project', 'shop');
  await cli('record', 'add', 'shop', 'ledger', 'title=Shelving', 'sum=4');

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
  const near = (a, b, by = 1.5) => Math.abs(a - b) <= by;

  /** A page that is signed in, with how many times it saved a view counted beside it. */
  const opened = async (context, email = 'ada@example.com', touch = false) => {
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    const counted = { saves: 0 };
    page.on('request', (request) => {
      if (request.method() === 'POST' && request.url().endsWith('/views')) counted.saves += 1;
    });
    await page.goto(url);
    // Whichever the page comes to draw, where to sign in or what somebody signed in is shown: it is waited
    // for before it is asked which. Asked as soon as the page had loaded, a page that had not drawn yet was
    // taken for one signed in, and was waited on for ever (a phone's page, five in eighteen, six at once).
    await page.locator('.panes').or(page.getByLabel('Email')).first().waitFor();
    if (await page.getByLabel('Email').count()) {
      await page.getByLabel('Email').fill(email);
      await page.getByLabel('Password').fill('correct horse');
      const signIn = page.getByRole('button', { name: 'Sign in', exact: true }).last();
      await (touch ? signIn.tap() : signIn.click());
    }
    await page.locator('.panes').waitFor();
    return { page, counted };
  };
  const show = async (page, touch = false, table = 'stock') => {
    const press = (locator) => (touch ? locator.tap() : locator.click());
    if (touch) await press(page.getByRole('button', { name: 'Expand the side panel' }));
    await sideSettled(page);
    const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await press(resources);
    // Everything the Resources panel lists starts folded (`folded.mjs`).
    await unfoldAll(page);
    const nav = page.locator('.nav');
    await nav.getByText('Shop', { exact: true }).waitFor();
    const twist = nav.getByRole('button', { name: 'Unfold Shop', exact: true });
    if (await twist.count()) await press(twist);
    const called = table.charAt(0).toUpperCase() + table.slice(1);
    await press(nav.locator('[data-project="shop"]').getByRole('button', { name: new RegExp(`${called}$`) }).first());
    await page.getByRole('heading', { name: `Shop ${table}`, level: 1 }).waitFor();
    await page.locator(`tr[data-row="shop-${table}-${table === 'stock' ? 30 : 1}"]`).waitFor();
  };
  const head = (page, column) => page.locator(`table.grid thead th[data-column="${column}"]`);
  const edge = (page, column) => head(page, column).locator('.width-edge');
  const boxOf = async (locator) => {
    const box = await locator.boundingBox();
    if (!box) throw new Error('expected something that is drawn to have a place');
    return box;
  };
  /** How wide a column's head is, and each of its cells, and its figure. */
  const widths = (page, column) =>
    page.evaluate((column) => {
      const th = document.querySelector(`table.grid thead th[data-column="${column}"]`);
      const nth = [...th.parentElement.children].indexOf(th);
      const cells = [...document.querySelectorAll('table.grid tbody tr[data-row]')].map((row) => row.children[nth].getBoundingClientRect().width);
      return { head: th.getBoundingClientRect().width, cells };
    }, column);
  const widthOf = async (page, column) => (await widths(page, column)).head;
  /** What the daemon has of the view's widths, asked with the page's own key. */
  const kept = (page, table = 'stock') =>
    page.evaluate(async (table) => {
      const answer = await fetch('/api/collections?project=shop', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      const read = (await answer.json()).collections.find((one) => one.name === table);
      return read?.views?.find((one) => one.saved)?.widths ?? null;
    }, table);
  /** Waits for the daemon to keep what a view says of one column's width. */
  const keptSoon = async (page, column, wanted) => {
    for (let tries = 0; tries < 100; tries++) {
      if (((await kept(page)) ?? {})[column] === wanted) return;
      await page.waitForTimeout(200);
    }
    throw new Error(`expected the daemon to keep ${column} ${wanted} wide, and it keeps ${JSON.stringify(await kept(page))}`);
  };
  /** The size of text the page draws at, against the usual. */
  const sizeOf = (page) => page.evaluate(() => parseFloat(getComputedStyle(document.querySelector('table.grid')).getPropertyValue('--size')) || 1);
  /** Takes hold of a column's edge and moves it `by` across, holding it there until `letGo`. */
  const takeHold = async (page, column, by) => {
    const at = await boxOf(edge(page, column));
    const x = at.x + at.width / 2;
    const y = at.y + at.height / 2;
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + by, y + 1, { steps: 12 });
    return { x, y };
  };
  const waitWidth = (page, column, wanted, by = 1.5) =>
    page.waitForFunction(([column, wanted, by]) => Math.abs(document.querySelector(`table.grid thead th[data-column="${column}"]`).getBoundingClientRect().width - wanted) <= by, [column, wanted, by]);
  /** Where the things of the page are that a drag must not move, but for what stands after the column held. */
  const standing = (page) =>
    page.evaluate(() => JSON.stringify([...document.querySelectorAll('.view-head h1, .view-head button, .tab, .rail-button, table.grid thead th[data-column="title"]')].map((one) => Object.values(one.getBoundingClientRect().toJSON()).map((n) => Math.round(n)))));

  const scheme = process.env.SCHEME ?? 'dark';
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const { page, counted } = await opened(context);
  const added = await page.evaluate(async () => {
    for (let nth = 1; nth <= 30; nth++) {
      const fields = { title: `Item ${String(nth).padStart(2, '0')} with a name long enough to be cut short`, status: ['todo', 'doing', 'done'][nth % 3], points: nth, aisle: `A${nth}`, shelf: `S${nth % 5}`, maker: `Maker ${nth}`, note: `Note ${nth}`, zone: `Z${nth}`, bin: `B${nth}` };
      const answer = await fetch('/api/collections/shop/stock/records', { method: 'POST', headers: { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' }, body: JSON.stringify({ fields }) });
      if (!answer.ok) return `${answer.status} ${await answer.text()}`;
    }
    return 'ok';
  });
  expect(`thirty rows of stock to be added, got ${added}`, added === 'ok');
  await show(page);
  const beside = await opened(context);
  await show(beside.page);
  await beside.page.evaluate(() => {
    window.readOnce = true;
  });
  const notReadAgain = async (what) => expect(`the other page not to have been read again ${what}`, await beside.page.evaluate(() => window.readOnce === true));
  const size = await sizeOf(page);

  // ---- an edge at the right of each head, and nothing more to press
  const heads = await page.locator('table.grid thead th[data-column]').count();
  expect(`an edge at each head, got ${await page.locator('table.grid thead th[data-column] .width-edge').count()} of ${heads}`, (await page.locator('table.grid thead th[data-column] .width-edge').count()) === heads);
  expect('each head to be one button still, the edge no button', (await page.locator('table.grid thead th[data-column] button').count()) === heads);
  const points = await boxOf(head(page, 'points'));
  const pointsEdge = await boxOf(edge(page, 'points'));
  expect(`the edge to be at the head's right, inside it, at ${JSON.stringify(pointsEdge)} of ${JSON.stringify(points)}`, near(pointsEdge.x + pointsEdge.width, points.x + points.width, 0.6) && pointsEdge.width <= 8 && near(pointsEdge.height, points.height, 1));
  expect('the edge to say what it is', (await edge(page, 'points').getAttribute('role')) === 'separator' && (await edge(page, 'points').getAttribute('aria-label')) === 'Resize Points');
  await page.locator('.grid-scroll').evaluate((box) => box.scrollTo(0, 0));

  // ---- held and moved: the column follows, nobody is told, nothing is carried and no menu opens
  const before = await widths(page, 'points');
  expect(`the cells of a column to be as wide as its head, got ${JSON.stringify(before)}`, before.cells.every((one) => near(one, before.head, 0.6)));
  const still = await standing(page);
  const aisleBefore = await boxOf(head(page, 'aisle'));
  await takeHold(page, 'points', 120);
  await waitWidth(page, 'points', before.head + 120);
  const during = await widths(page, 'points');
  expect(`every cell of the column to follow its head while it is held, got ${JSON.stringify(during)}`, during.cells.every((one) => near(one, during.head, 0.6)));
  expect('nothing to be carried and no menu to open while the edge is held', (await page.locator('.grid-drop').count()) === 0 && (await page.locator('th.head-carried').count()) === 0 && (await page.locator('.headmenu').count()) === 0);
  expect('nothing to be saved while the edge is held', counted.saves === 0);
  expect('nothing before the column to move while it is held', (await standing(page)) === still);
  expect('what stands after the column to move over by what it grew', near((await boxOf(head(page, 'aisle'))).x, aisleBefore.x + 120, 1.5));
  expect('the other page not to be told while the edge is held', near(await widthOf(beside.page, 'points'), before.head, 0.6));
  await page.screenshot({ path: `${out}/widths-01-dragged.png` });
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelector('table.grid thead th[data-column="points"]').classList.contains('sized'));
  expect(`one save for the drag, got ${counted.saves}`, counted.saves === 1);
  const wanted = Math.round((before.head + 120) / size);
  expect(`the daemon to keep the width at the usual size of text, ${wanted}, got ${JSON.stringify(await kept(page))}`, (await kept(page))?.points === wanted);
  expect('the column to stay as wide as it was let go', near(await widthOf(page, 'points'), wanted * size));
  expect('the click after the drag to open no menu', (await page.locator('.headmenu').count()) === 0);
  await page.screenshot({ path: `${out}/widths-02-saved.png` });
  await waitWidth(beside.page, 'points', wanted * size);
  await notReadAgain('for a width saved');
  await beside.page.screenshot({ path: `${out}/widths-03-second-page.png` });
  await page.reload();
  await page.locator('tr[data-row="shop-stock-30"]').waitFor();
  await waitWidth(page, 'points', wanted * size);

  // A press beside the edge is the head's, as it always was.
  const pointsHead = await boxOf(head(page, 'points'));
  await page.mouse.click(pointsHead.x + 20, pointsHead.y + pointsHead.height / 2);
  await page.locator('.headmenu').waitFor();
  await page.keyboard.press('Escape');
  await page.locator('.headmenu').waitFor({ state: 'detached' });
  // Held on the edge and moved a long way: the edge's still, and never past the narrowest.
  await takeHold(page, 'points', -600);
  await waitWidth(page, 'points', NARROWEST * size);
  expect('nothing to be carried for an edge moved a long way', (await page.locator('.grid-drop').count()) === 0);
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelector('table.grid thead th[data-column="points"]').getBoundingClientRect().width < 100);
  await keptSoon(page, 'points', 60);
  expect(`the narrowest a column may be, got ${await widthOf(page, 'points')}`, near(await widthOf(page, 'points'), NARROWEST * size));
  await waitWidth(beside.page, 'points', NARROWEST * size);

  // ---- Escape while it is held, and a pointer let go where it began: put back, nothing saved
  const saves = counted.saves;
  const narrow = await widthOf(page, 'points');
  const held = await takeHold(page, 'points', 200);
  await waitWidth(page, 'points', narrow + 200);
  await page.keyboard.press('Escape');
  await waitWidth(page, 'points', narrow);
  await page.mouse.up();
  expect('Escape to have put it back with nothing saved, and no menu', counted.saves === saves && (await page.locator('.headmenu').count()) === 0);
  await takeHold(page, 'points', 90);
  await page.mouse.move(held.x, held.y, { steps: 8 });
  await page.mouse.up();
  await waitWidth(page, 'points', narrow);
  expect('a pointer let go where it began to save nothing', counted.saves === saves);

  // ---- pressed twice: as wide as what is in it, saved as no width
  await edge(page, 'points').dblclick();
  await page.waitForFunction(() => !document.querySelector('table.grid thead th[data-column="points"]').classList.contains('sized'));
  expect(`a double press to save no width for the column, got ${JSON.stringify(await kept(page))}`, (await kept(page)) === null);
  expect('the column to be as wide as what is in it again', near(await widthOf(page, 'points'), before.head, 1));
  await waitWidth(beside.page, 'points', before.head, 1);
  await notReadAgain('for a width reset');

  // ---- the keyboard: the edge is reached, and the arrows step it, each step saved
  const keyed = counted.saves;
  await edge(page, 'points').focus();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowRight');
  const stepped = Math.round(before.head / size) + 2 * STEP;
  await keptSoon(page, 'points', stepped);
  await page.keyboard.press('ArrowLeft');
  await keptSoon(page, 'points', stepped - STEP);
  await waitWidth(page, 'points', (stepped - STEP) * size);
  expect(`each step to be saved as it lands, got ${counted.saves - keyed}`, counted.saves - keyed === 3);
  expect('the caret to be on the edge still', await page.evaluate(() => document.activeElement?.classList.contains('width-edge')));
  await waitWidth(beside.page, 'points', (stepped - STEP) * size);

  // ---- frozen columns follow a width while it is held and after
  await head(page, 'status').locator('.grid-head').click();
  await page.locator('.headmenu').waitFor();
  const menu = page.locator('.headmenu');
  await menu.getByRole('menuitem', { name: 'Freeze up to here' }).or(menu.getByRole('menuitemcheckbox', { name: 'Freeze up to here' })).click();
  await page.waitForFunction(() => document.querySelectorAll('table.grid thead th[data-column].frozen').length === 2);
  await page.locator('.grid-scroll').evaluate((box) => box.scrollTo(0, 0));
  const title = await widthOf(page, 'title');
  await takeHold(page, 'title', 70);
  await waitWidth(page, 'title', title + 70);
  // The frozen columns are told where they begin by what the browser says of the heads' widths.
  await page.waitForFunction(() => {
    const [one, two] = ['title', 'status'].map((name) => document.querySelector(`table.grid thead th[data-column="${name}"]`).getBoundingClientRect());
    return Math.abs(one.right - two.left) <= 0.6;
  });
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelector('table.grid thead th[data-column="title"]').classList.contains('sized'));
  await page.locator('.grid-scroll').evaluate((box) => box.scrollTo(400, 0));
  const scrolled = await page.evaluate(() => {
    const box = (name) => document.querySelector(`table.grid thead th[data-column="${name}"]`).getBoundingClientRect();
    return { room: document.querySelector('.grid-scroll').getBoundingClientRect().left, title: box('title'), status: box('status') };
  });
  expect(`the frozen columns to stay side by side at the widths saved, scrolled, got ${JSON.stringify(scrolled)}`, near(scrolled.title.right, scrolled.status.left, 0.6) && near(scrolled.title.width, Math.round((title + 70) / size) * size, 1.5));
  await page.screenshot({ path: `${out}/widths-04-frozen.png` });
  await page.locator('.grid-scroll').evaluate((box) => box.scrollTo(0, 0));

  // ---- the heading over two columns stays over them: they are the last, so the table is scrolled to its end
  await page.locator('.grid-scroll').evaluate((box) => box.scrollTo(box.scrollWidth, 0));
  const aisle = await widthOf(page, 'aisle');
  await takeHold(page, 'aisle', 90);
  await waitWidth(page, 'aisle', aisle + 90);
  const banded = () =>
    page.evaluate(() => {
      const band = [...document.querySelectorAll('table.grid .grid-bands th')].find((one) => one.textContent.trim() === 'Where').getBoundingClientRect();
      const [first, last] = ['aisle', 'shelf'].map((name) => document.querySelector(`table.grid thead th[data-column="${name}"]`).getBoundingClientRect());
      return Math.abs(band.left - first.left) <= 0.6 && Math.abs(band.right - last.right) <= 0.6;
    });
  expect('the heading to be over its two columns while one of them is held', await banded());
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelector('table.grid thead th[data-column="aisle"]').classList.contains('sized'));
  expect('the heading to be over its two columns once it is saved', await banded());
  await page.screenshot({ path: `${out}/widths-05-under-a-heading.png` });
  await notReadAgain('for widths saved');
  const ours = { title: await widthOf(page, 'title'), points: await widthOf(page, 'points'), aisle: await widthOf(page, 'aisle') };

  // ---- somebody who may not save a view sees the widths and no edge
  await show(page, false, 'ledger');
  const sum = await widthOf(page, 'sum');
  await takeHold(page, 'sum', 110);
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelector('table.grid thead th[data-column="sum"]').classList.contains('sized'));
  expect(`the ledger's width to be kept, got ${JSON.stringify(await kept(page, 'ledger'))}`, (await kept(page, 'ledger'))?.sum === Math.round((sum + 110) / size));
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }) });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  const theirs = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const reader = (await opened(theirs, 'grace@example.com')).page;
  await show(reader, false, 'ledger');
  await waitWidth(reader, 'sum', Math.round((sum + 110) / size) * size);
  expect('no edge for somebody who may not save a view', (await reader.locator('table.grid .width-edge').count()) === 0);
  await reader.screenshot({ path: `${out}/widths-06-not-theirs-to-change.png` });
  // A table they may change is theirs to say widths of, and shows everybody's.
  await show(reader);
  for (const [column, width] of Object.entries(ours)) await waitWidth(reader, column, width, 1.5);
  expect('an edge for a member on a table they may change', (await reader.locator('table.grid thead th[data-column] .width-edge').count()) > 0);
  await theirs.close();

  // ---- a phone's width: nothing offered, and the widths not drawn
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const small = (await opened(phone, 'ada@example.com', true)).page;
  await show(small, true);
  expect('no edge to be offered on a phone', (await small.locator('table.grid .width-edge:visible').count()) === 0);
  // The column says its width, and the phone's layout draws it as it draws any: as wide as can be touched and read there.
  const onPhone = await small.evaluate(() => {
    const th = document.querySelector('table.grid thead th[data-column="points"]');
    return { sized: th.classList.contains('sized'), min: getComputedStyle(th).minWidth };
  });
  expect(`a width the view says not to be drawn on a phone, got ${JSON.stringify(onPhone)}`, onPhone.sized && onPhone.min === '112px');
  await small.screenshot({ path: `${out}/widths-07-phone.png` });
  await phone.close();

  // ---- the other of dark and light
  const otherScheme = scheme === 'dark' ? 'light' : 'dark';
  const turned = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: otherScheme });
  const lit = (await opened(turned)).page;
  await show(lit);
  await edge(lit, 'points').hover();
  await lit.screenshot({ path: `${out}/widths-08-${otherScheme}.png` });
  await turned.close();

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
