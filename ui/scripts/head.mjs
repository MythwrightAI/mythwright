// The right of a page's head: when the thing was last changed, what it is
// shared by, an address that opens it, its pin, and a gear.
//
//   DAEMON=../target/debug/factory node scripts/head.mjs [out-dir]
//
// Shop has three tables (one that keeps a history of its records, one
// that keeps none, one kept private), a board with a table beside it, a
// library and a file that is no document; a second project has a long
// name. The head is looked at on each kind of page: what is drawn and
// what is not, by its boxes, and none of it in the drawer, in Settings or
// on Activity. When a table was changed and by whom is said from one
// history read and then from what is said, with the page not read again.
// The gear's menu is on the screen under its gear, moves nothing, stays
// as the pointer crosses to it, and takes the keyboard. The board's head
// has no button of its own for its tables or its tools: both are lines of
// its gear, and Layout hangs the layout's panel under the gear, which
// `headgear.mjs` holds for this check, at each width and on a phone. The pin in the
// head and the pin in the side panel are one. An address, a path that
// names the thing with its view after the `#`, is copied, read back from
// the clipboard (which this browser is given leave to read and write),
// held to the bar, opened in a second page, and by somebody who is not
// signed in (`headaddress.mjs`).
// A library is shared by its pages and written out; a document by its
// mark, which is found in its file; it is copied, saved, and deleted
// behind a question. A table is written out as a CSV file, held cell
// for cell, read straight back and read in changed, and the board's
// cards the same. Then all of the head at three widths with the side
// panel open and folded, at a phone's width, and in the other of dark
// and light, on every kind of page with a long title: on each line the
// head makes, what the page is called with the words beside it and each
// thing at its right centred on one level, and the words on the title's
// baseline. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, stat, utimes, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { unfoldAll } from './folded.mjs';
import { addresses } from './headaddress.mjs';
import { boardGear, boardGearAt, boardGearOnPhone } from './headgear.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'head-data');
const home = join(out, 'head-home');

// Three tables: one that keeps a history of its records, one that keeps none, and one kept private.
const TABLES = `blueprint: tables
description: A table that keeps what happens to its records, one that keeps nothing, and one kept private.
permissions: [projects, records]
steps:
  tickets:
    do: collection.ensure
    with:
      name: tickets
      fields:
        title: { type: text, required: true }
        points: { type: number }
        urgent: { type: flag }
        tags: { type: list }
        state: { type: choice, of: [open, shut] }
        notes: { type: text }
      history:
        record.created: "{actor} added it"
        record.updated: "{actor} changed its {changed}"
        record.removed: "{actor} took it away"
  plain:
    do: collection.ensure
    with:
      name: plain
      fields:
        title: { type: text, required: true }
        points: { type: number }
      # Two views, so that an address can say which by its name.
      views:
        rows: { kind: table }
        newest: { kind: list }
  ledger:
    do: collection.ensure
    with:
      name: ledger
      reads: private
      fields:
        title: { type: text, required: true }
        sum: { type: number }
`;
// One that sits beside the board: opened from the board's page, in the drawer.
const EXTRAS = `blueprint: extras
description: A table that sits beside the board.
permissions: [projects, records]
steps:
  extras:
    do: collection.ensure
    with:
      name: extras
      beside: board
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
  await cli('project', 'create', '--bare', 'Shop');
  await cli('project', 'create', '--bare', 'A workshop with a long name for its sign');
  const shop = join(data, 'projects/shop');
  await mkdir(join(shop, 'blueprints'), { recursive: true });
  await writeFile(join(shop, 'blueprints/tables.bp'), TABLES);
  await cli('apply', join(shop, 'blueprints/tables.bp'), '--project', 'shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await writeFile(join(shop, 'blueprints/extras.bp'), EXTRAS);
  await cli('apply', join(shop, 'blueprints/extras.bp'), '--project', 'shop');
  await cli('record', 'add', 'shop', 'extras', 'title=Spare');
  const long = 'a-workshop-with-a-long-name-for-its-sign';
  const workshop = join(data, 'projects', long);
  await mkdir(join(workshop, 'blueprints'), { recursive: true });
  await writeFile(join(workshop, 'blueprints/tables.bp'), TABLES);
  await cli('apply', join(workshop, 'blueprints/tables.bp'), '--project', long);
  await cli('record', 'add', long, 'tickets', 'title=A ticket');
  await cli('apply', 'std/docs', '--project', 'shop');
  await mkdir(join(shop, 'docs/guides'), { recursive: true });
  await writeFile(join(shop, 'docs/welcome.md'), '# Welcome\n\nStart here.\n');
  await writeFile(join(shop, 'docs/guides/install.md'), '# Installing it\n\nRun the installer.\n');
  await writeFile(join(shop, 'notes.txt'), 'Not a document.\n');
  await cli('project', 'scan', 'shop');
  // The project with a long name has a page of every kind too, each headed by its long name: its board, its library
  // with a document called at length, and its mockups.
  await cli('apply', 'std/board', '--project', long);
  await cli('apply', 'std/docs', '--project', long);
  await cli('apply', 'std/mockups', '--project', long);
  await mkdir(join(workshop, 'docs'), { recursive: true });
  await writeFile(join(workshop, 'docs/what-the-workshop-keeps-on-its-longest-shelves.md'), '# Shelves\n\nWhat is kept.\n');
  await cli('project', 'scan', long);
  for (const title of ['Shelving', 'Crates', 'Labels']) await cli('record', 'add', 'shop', 'tickets', `title=${title}`);
  for (const title of ['One', 'Two']) await cli('record', 'add', 'shop', 'plain', `title=${title}`);
  await cli('record', 'add', 'shop', 'ledger', 'title=Rent', 'sum=900');
  await cli('board', 'add', 'shop', 'A first card');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1350, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark', permissions: ['clipboard-read', 'clipboard-write'], acceptDownloads: true });
  const page = await context.newPage();
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async (on) => {
    await on.getByLabel('Email').fill('ada@example.com');
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  await page.goto(url);
  await signIn(page);
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const railed = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
  if ((await railed.getAttribute('aria-pressed')) !== 'true') await railed.click();
  await resources.waitFor();
  await unfoldAll(page);
  // An address followed under the open page: the thing it names is shown as a tab.
  const open = async (hash, heading) => {
    await page.evaluate((to) => (location.hash = to), hash);
    await page.getByRole('heading', { name: heading, level: 1 }).waitFor();
  };
  const acts = page.locator('.view-head .head-acts');
  const edited = acts.locator('.head-edited');
  const gear = acts.getByRole('button', { name: 'More for this page', exact: true });
  const pin = acts.locator('.head-pin');
  const link = acts.getByRole('button', { name: 'Copy link', exact: true });
  // The menu the gear and Share open: a column's menu is a menu too, and is not this one.
  const menu = page.locator('.row-menu');
  const away = () => page.mouse.move(980, 700);
  // What the right of the head holds, by what each thing is: the words and then what is pressed, in their order.
  const drawn = () =>
    acts.evaluate((box) =>
      [...box.children]
        .map((one) => (one.classList.contains('head-edited') ? 'edited' : one.classList.contains('head-quiet') ? one.textContent.trim() : (one.getAttribute('aria-label') ?? one.textContent.trim())))
        .join(' | '),
    );
  const narrow = async (where) => {
    const wide = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`the page not to scroll sideways ${where}, and it is ${wide}px too wide`, wide <= 0);
  };
  // The head by its boxes: everything of it inside it, nothing of a line lying over anything else of that line,
  // the right of it at the end of the first line, and what the page has besides under that line.
  const frame = async (where) => {
    const got = await page.locator('.view-head').first().evaluate((head) => {
      const round = (figure) => Math.round(figure * 10) / 10;
      const box = (one) => {
        const at = one.getBoundingClientRect();
        return { left: round(at.left), right: round(at.right), top: round(at.top), bottom: round(at.bottom), is: one.className || one.tagName };
      };
      const there = (one) => one.getClientRects().length > 0 && !one.classList.contains('grow') && !one.classList.contains('head-break');
      const acts = head.querySelector('.head-acts');
      return {
        head: box(head),
        parts: [...head.children].filter(there).map(box),
        acts: box(acts),
        inside: [...acts.children].filter(there).map(box),
        after: [...head.children].slice([...head.children].indexOf(acts) + 1).filter(there).map(box),
      };
    });
    const told = JSON.stringify(got);
    const over = (a, b) => a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5;
    for (const all of [got.parts, got.inside]) {
      for (const [at, one] of all.entries()) {
        expect(`everything of the head to be inside it ${where}: ${JSON.stringify(one)} in ${JSON.stringify(got.head)}`, one.left >= got.head.left - 0.5 && one.right <= got.head.right + 0.5 && one.top >= got.head.top - 0.5 && one.bottom <= got.head.bottom + 0.5);
        for (const other of all.slice(at + 1)) expect(`nothing of the head to lie over anything else of it ${where}: ${JSON.stringify(one)} and ${JSON.stringify(other)}`, !over(one, other));
      }
    }
    expect(`something to be pressed at the right of the head ${where}: ${told}`, got.inside.length >= 3);
    expect(`the right of the head to end where the head's room ends ${where}: ${told}`, got.acts.right >= got.head.right - 24);
    expect(`what the page has in its head besides to be under the line that ends with its right ${where}: ${told}`, got.after.every((one) => one.top >= got.acts.bottom - 0.5));
    await narrow(where);
    await centred(page, where);
  };
  // The head's lines by their boxes: what the page is called with the words beside it (`.head-title`), each thing at
  // the right of the head and what the page has besides on the line after, each centred on its line within a pixel
  // of the rest of it, however many lines a narrow page makes; and what is called and the words beside it standing on
  // one baseline, within a pixel, on each line they make. Centred each by its own box, the smaller words beside a title
  // sat higher than it.
  const centred = async (on, where) => {
    const lines = await on.locator('.view-head').first().evaluate((head) => {
      const there = (one) => one.getClientRects().length > 0 && !one.classList.contains('grow') && !one.classList.contains('head-break');
      const called = (one) => `${one.tagName.toLowerCase()}${[...one.classList].map((name) => `.${name}`).join('')} "${one.textContent.trim().slice(0, 24)}"`;
      // Where the first words of a thing stand: the top of their text's box and the ascent of the letters it is drawn in.
      const pen = document.createElement('canvas').getContext('2d');
      const baseline = (one) => {
        const walk = document.createTreeWalker(one, NodeFilter.SHOW_TEXT, { acceptNode: (text) => (text.textContent.trim() ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP) });
        const text = walk.nextNode();
        if (!text) return null;
        const range = document.createRange();
        range.selectNodeContents(text);
        const box = range.getClientRects()[0];
        const style = getComputedStyle(text.parentElement);
        pen.font = `${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
        return box.top + pen.measureText(text.textContent.trim()).fontBoundingBoxAscent;
      };
      // What the page is called and what stands beside it is everything before the head's first gap, and what is at its
      // right the rest: each thing of the right, and what the page has besides on the line after.
      const children = [...head.children];
      const gap = children.findIndex((one) => one.classList.contains('grow'));
      const named = children.slice(0, gap < 0 ? children.length : gap).flatMap((one) => (one.classList.contains('head-title') ? [...one.children] : [one])).filter(there);
      const rest = children.slice(gap < 0 ? children.length : gap).filter(there).flatMap((one) => (one.classList.contains('head-acts') ? [...one.children].filter(there) : [one]));
      const box = (one, titled) => {
        const at = one.getBoundingClientRect();
        return { is: called(one), top: at.top, bottom: at.bottom, titled, baseline: titled ? baseline(one) : null };
      };
      // Things stand on lines: a line begins where something stands wholly under everything of the line before it.
      const lines = [];
      for (const one of [...named.map((it) => box(it, true)), ...rest.map((it) => box(it, false))].sort((a, b) => a.top - b.top)) {
        const line = lines.at(-1);
        if (line && one.top < Math.max(...line.map((it) => it.bottom)) - 0.5) line.push(one);
        else lines.push([one]);
      }
      return lines;
    });
    const level = (all) => Math.max(...all) - Math.min(...all);
    for (const [nth, line] of lines.entries()) {
      // What is called and the words beside it, as one, centred on the line as each thing at the right is.
      const titled = line.filter((one) => one.titled);
      const middles = [
        ...(titled.length ? [{ is: 'what it is called with the words beside it', at: (Math.min(...titled.map((one) => one.top)) + Math.max(...titled.map((one) => one.bottom))) / 2 }] : []),
        ...line.filter((one) => !one.titled).map((one) => ({ is: one.is, at: (one.top + one.bottom) / 2 })),
      ];
      expect(`everything of the head's line ${nth + 1} to be centred on one level ${where}, within a pixel: ${middles.map((one) => `${one.is} at ${one.at.toFixed(1)}`).join(', ')}`, level(middles.map((one) => one.at)) <= 1);
      const lettered = titled.filter((one) => one.baseline !== null);
      expect(
        `what the page is called and the words beside it to stand on one baseline on the head's line ${nth + 1} ${where}, within a pixel: ${lettered.map((one) => `${one.is} at ${one.baseline.toFixed(1)}`).join(', ')}`,
        lettered.length < 2 || level(lettered.map((one) => one.baseline)) <= 1,
      );
    }
  };
  const tree = (id) => resources.locator(`[data-project="${id}"] > .nav-row`);
  await tree('shop').waitFor();

  // ---- the head on each kind of page: what is drawn, and what is not
  await open('/table/shop/tickets', 'Shop tickets');
  await page.locator('table.grid tr[data-row]').first().waitFor();
  await edited.waitFor();
  expect(`a table's head to say when it was changed, and then Follow, its address, its pin and its gear, with nothing to share it by; got ${await drawn()}`, (await drawn()) === 'edited | Follow | Copy link | Pin | More for this page');
  await frame("on a table's page");
  await page.screenshot({ path: `${out}/head-01-table.png` });

  // A table kept private says so in a word, and is still shared by nothing here.
  await open('/table/shop/ledger', 'Shop ledger');
  await edited.waitFor();
  expect(`a table kept private to say so beside when it was changed; got ${await drawn()}`, (await drawn()) === 'edited | private | Follow | Copy link | Pin | More for this page');
  const kept = await acts.locator('.head-quiet').getAttribute('title');
  expect(`the word to say what reading it takes, got "${kept}"`, /holds private/.test(kept ?? ''));
  await frame("on a private table's page");
  await page.screenshot({ path: `${out}/head-02-private-table.png` });

  await open('/board/shop', 'Shop board');
  await edited.waitFor();
  expect(`the board's head to be a table's; got ${await drawn()}`, (await drawn()) === 'edited | Follow | Copy link | Pin | More for this page');
  await frame("on the board's page");
  await page.screenshot({ path: `${out}/head-03-board.png` });

  // The board's tables and its tools are lines of its gear, and a table opened from there, in the drawer, has the
  // drawer's own head and none of this (`headgear.mjs`).
  await boardGear({ page, out, expect });

  await open('/project/shop', 'Shop');
  expect(`a project's page to say nothing of when it changed, and to have Follow, its address, its pin and its gear; got ${await drawn()}`, (await drawn()) === 'Follow | Copy link | Pin | More for this page');
  await frame("on a project's page");
  await page.screenshot({ path: `${out}/head-06-project.png` });
  await open('/files/shop', 'Shop repo');
  expect(`a project's files to have the same but Follow, which is the project's; got ${await drawn()}`, (await drawn()) === 'Copy link | Pin | More for this page');
  await frame("on a project's files");

  // What has no address and no pin has none of it: a channel, the fleet, what is always listed, settings.
  await page.locator('.rail').getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('heading', { name: 'Settings', level: 1 }).waitFor();
  expect('nothing of it in Settings', (await acts.count()) === 0);
  await resources.locator('.nav > .tree > button.nav-item', { hasText: 'Activity' }).click();
  await page.getByRole('heading', { name: 'Activity', level: 1 }).waitFor();
  expect('nothing of it on Activity', (await acts.count()) === 0);

  // ---- when it was changed, and by whom: from what the page read, and then from what is said
  const asked = [];
  page.on('request', (request) => /\/api\/records\/[^/]+\/history/.test(request.url()) && asked.push(request.url()));
  const at = async () => Number(await edited.getAttribute('data-at'));
  await open('/table/shop/tickets', 'Shop tickets');
  await edited.waitFor();
  // Who is said once the history of its newest record is in: the one thing asked for it.
  await page.waitForFunction(() => /^Last edited by Ada/.test(document.querySelector('.view-head .head-edited')?.getAttribute('title') ?? ''));
  const whole = await edited.getAttribute('title');
  expect(`whoever points at it to be told what it does not count, got "${whole}"`, /taken away is not counted/.test(whole ?? ''));
  expect(`its words in the head to be short, got "${await edited.innerText()}"`, /^Edited \d{1,2}:\d\d (?:AM|PM) \S+$/.test((await edited.innerText()).trim()));
  const first = await at();
  const reads = asked.length;
  expect(`one record's history to be read for a table's head, and ${reads} were`, reads === 1);
  // A cell changed from a terminal: when it was changed moves, with the page not read again and no history asked for.
  const began = await page.evaluate(() => performance.timeOrigin);
  await cli('record', 'set', 'shop-tickets-1', 'points=8');
  await page.waitForFunction((was) => Number(document.querySelector('.view-head .head-edited')?.getAttribute('data-at')) > was, first);
  await page.waitForFunction(() => /^Last edited by Ada/.test(document.querySelector('.view-head .head-edited')?.getAttribute('title') ?? ''));
  expect('the page not to have been read again', (await page.evaluate(() => performance.timeOrigin)) === began);
  expect(`no history to be asked for again as a record changes, and ${asked.length - reads} were`, asked.length === reads);
  // A table that keeps no history says when and names nobody.
  await open('/table/shop/plain', 'Shop plain');
  await edited.waitFor();
  const plain = await at();
  await cli('record', 'set', 'shop-plain-1', 'points=3');
  await page.waitForFunction((was) => Number(document.querySelector('.view-head .head-edited')?.getAttribute('data-at')) > was, plain);
  const nobody = await edited.getAttribute('title');
  expect(`a table with no history to say when and name nobody, got "${nobody}"`, /^Last edited \d/.test(nobody ?? ''));
  // The line of the gear's menu says the same, whole.
  await open('/table/shop/tickets', 'Shop tickets');
  await page.waitForFunction(() => /^Last edited by Ada/.test(document.querySelector('.view-head .head-edited')?.getAttribute('title') ?? ''));

  // ---- the gear's menu: on the screen, moving nothing, staying as the pointer crosses to it, and the keyboard
  const boxes = () => page.locator('.view-head').first().evaluate((head) => [...head.querySelectorAll('*')].map((one) => { const at = one.getBoundingClientRect(); return [at.left, at.top, at.width, at.height].map((figure) => Math.round(figure * 10) / 10).join(','); }).join(' '));
  const table = page.locator('table.grid');
  const tableAt = async () => JSON.stringify(await table.boundingBox());
  const [headBefore, tableBefore] = [await boxes(), await tableAt()];
  await gear.click();
  await menu.waitFor();
  // What a line of a menu says: its name and the faint words after it, without the drawing before it; or what it only says.
  const lines = () =>
    menu.locator('button, .menu-said').evaluateAll((all) =>
      all.map((one) => (one.matches('button') ? [...one.querySelectorAll('.truncate, .hint')].map((part) => part.textContent).join(' ') : one.textContent).replace(/\s+/g, ' ').trim()),
    );
  const offered = await lines();
  expect(
    `the gear to offer a table's address, what it is called for people (\`views/outline.tsx\`), the table as a file and one read in, its blueprint, what its project runs and measures, and say when it was changed; got ${offered.join(' | ')}`,
    offered.length === 8 && offered.slice(0, 7).join(' | ') === 'Copy link | Rename… | Export as CSV 3 tickets | Import or merge a CSV… | Its blueprint tables | Activity of Shop | Measurements of Shop' && /^Last edited by Ada · /.test(offered[7]),
  );
  expect('nothing of the head to move as the menu opens', (await boxes()) === headBefore && (await tableAt()) === tableBefore);
  expect('the gear to say its menu is open', (await gear.getAttribute('aria-expanded')) === 'true');
  const onScreen = async (where) => {
    const [at, view] = [await menu.boundingBox(), page.viewportSize()];
    expect(`the menu to be on the screen ${where}: ${JSON.stringify(at)} in ${JSON.stringify(view)}`, at.x >= 0 && at.y >= 0 && at.x + at.width <= view.width && at.y + at.height <= view.height);
    return at;
  };
  const opened = await onScreen('at the usual width');
  const under = await gear.boundingBox();
  expect(`the menu to open under the gear, its right at the gear's: ${JSON.stringify(opened)} under ${JSON.stringify(under)}`, opened.y >= under.y + under.height && Math.abs(opened.x + opened.width - (under.x + under.width)) <= 4);
  // The line that is said is not pressed: it is no button, and the arrows pass it.
  expect('the line that says when it was changed to be something said, and nothing to press', (await menu.locator('.menu-said').count()) === 1 && (await menu.getByRole('menuitem').count()) === 7);
  const focused = () => page.evaluate(() => [...(document.activeElement?.querySelectorAll('.truncate, .hint') ?? [])].map((part) => part.textContent).join(' ').replace(/\s+/g, ' ').trim());
  expect(`the keyboard to be in the menu at its first line, got ${await focused()}`, (await focused()) === 'Copy link');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go to the next line, got ${await focused()}`, (await focused()) === 'Rename…');
  await page.keyboard.press('End');
  expect(`End to go to the last line that is pressed, got ${await focused()}`, (await focused()) === 'Measurements of Shop');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go round to the first, got ${await focused()}`, (await focused()) === 'Copy link');
  // The pointer crosses from the gear to a line of the menu: the menu stays.
  await gear.hover();
  await menu.getByRole('menuitem', { name: 'Activity of Shop' }).hover();
  expect('the menu to stay as the pointer crosses to it', await menu.isVisible());
  await page.screenshot({ path: `${out}/head-08-gear-on-a-table.png` });
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  expect('Escape to give the keyboard back to the gear', await gear.evaluate((one) => one === document.activeElement));
  // Pressed again while its menu is open, the gear puts it away; and so does a press outside.
  await gear.click();
  await menu.waitFor();
  await gear.click();
  await menu.waitFor({ state: 'detached' });
  await gear.click();
  await menu.waitFor();
  await page.mouse.click(700, 600);
  await menu.waitFor({ state: 'detached' });
  // A column's menu and the gear's are never open together, by the pointer or by the keyboard, and neither is left
  // for the other's Escape.
  const column = page.locator('table.grid thead th[data-column] .grid-head', { hasText: 'Points' });
  const ofColumn = page.locator('.headmenu');
  // What is opened over the page begins to listen for a press outside it a moment after it is drawn, as a dialog
  // does for Escape: a person is never that quick, and the check lets the page draw before it presses elsewhere.
  const settled = () => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  await column.click();
  await ofColumn.waitFor();
  await settled();
  await gear.click();
  await menu.waitFor();
  await ofColumn.waitFor({ state: 'detached' });
  await settled();
  await column.click();
  await ofColumn.waitFor();
  await menu.waitFor({ state: 'detached' });
  await page.keyboard.press('Escape');
  await ofColumn.waitFor({ state: 'detached' });
  expect("Escape to give the keyboard back to the column's head, with no menu of the gear's to take it", await page.evaluate(() => document.activeElement?.classList.contains('grid-head') === true));
  // A view's layout opened by the keyboard, and then the gear by the keyboard: the first is put away, and Escape is the gear's.
  const layout = page.locator('.view-head').getByRole('button', { name: /^Layout/ });
  await layout.focus();
  await page.keyboard.press('Enter');
  await page.locator('.layout-menu').waitFor();
  await settled();
  await gear.focus();
  await page.keyboard.press('Enter');
  await menu.waitFor();
  await page.locator('.layout-menu').waitFor({ state: 'detached' });
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  expect('Escape to give the keyboard back to the gear', await gear.evaluate((one) => one === document.activeElement));
  // Its blueprint opens what declared the table, where its columns are said.
  await gear.click();
  await menu.getByRole('menuitem', { name: /^Its blueprint/ }).click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'tables · ' }).waitFor();
  // What a project runs, held to the project as its own page holds it.
  await open('/table/shop/tickets', 'Shop tickets');
  await gear.click();
  await menu.getByRole('menuitem', { name: 'Activity of Shop' }).click();
  await page.getByRole('heading', { name: 'Activity', level: 1 }).waitFor();
  const held = await page.locator('.view-head select').first().evaluate((one) => one.selectedOptions[0]?.textContent);
  expect(`Activity to be held to the project, got "${held}"`, held === 'Shop');

  // ---- the pin in the head is the pin in the panel
  const section = (called) =>
    resources.evaluate((nav, name) => {
      const head = [...nav.querySelectorAll('.nav > h3')].find((one) => one.textContent.trim() === name);
      if (!head) return '(not drawn)';
      return [...head.nextElementSibling.querySelectorAll('.lately-line .truncate')].map((one) => one.textContent).join(' | ');
    }, called);
  const listed = async (read, wanted, what) => {
    await page.waitForFunction(
      ([name, want]) => {
        const nav = document.querySelector('nav[aria-label="Resources"]');
        const head = [...(nav?.querySelectorAll('.nav > h3') ?? [])].find((one) => one.textContent.trim() === name);
        const got = head ? [...head.nextElementSibling.querySelectorAll('.lately-line .truncate')].map((one) => one.textContent).join(' | ') : '(not drawn)';
        return want instanceof Array ? !got.split(' | ').includes(want[0]) : got === want;
      },
      [read, wanted],
    ).catch(async () => expect(`${what}: "${wanted}", got "${await section(read)}"`, false));
  };
  await open('/table/shop/tickets', 'Shop tickets');
  await away();
  expect(`nothing to be pinned yet, got "${await section('Pinned')}"`, (await section('Pinned')) === '(not drawn)');
  expect('the pin in the head to say it is not pinned', (await pin.getAttribute('aria-pressed')) === 'false' && (await pin.getAttribute('aria-label')) === 'Pin');
  await pin.click();
  await listed('Pinned', 'Shop tickets', 'what was pinned from its head to be under Pinned in the panel at once');
  await listed('Recents', ['Shop tickets'], 'what is pinned not to be listed again under it');
  expect('the pin in the head to say it is pinned', (await pin.getAttribute('aria-pressed')) === 'true' && (await pin.getAttribute('aria-label')) === 'Unpin');
  await page.screenshot({ path: `${out}/head-09-pinned.png` });
  // Unpinned from its line in the panel, the head's pin is off: there is one pin.
  const line = resources.locator('.lately-pinned .lately-line', { hasText: 'Shop tickets' });
  await line.hover();
  await line.getByRole('button', { name: 'More for Shop tickets', exact: true }).click();
  await menu.getByText('Unpin', { exact: true }).click();
  await page.waitForFunction(() => document.querySelector('.view-head .head-pin')?.getAttribute('aria-pressed') === 'false');
  await away();
  await listed('Pinned', '(not drawn)', 'nothing to be pinned once it is unpinned from the panel');
  // And the other way: pinned from the panel, the head says so; kept when the page is read again.
  await tree('shop').hover();
  await tree('shop').getByRole('button', { name: 'More for Shop', exact: true }).click();
  await menu.getByText('Pin to the top', { exact: true }).click();
  await open('/project/shop', 'Shop');
  expect("a project pinned from its row to be pinned in its page's head", (await pin.getAttribute('aria-pressed')) === 'true');
  await open('/table/shop/tickets', 'Shop tickets');
  await pin.click();
  await away();
  await listed('Pinned', 'Shop | Shop tickets', 'two things pinned, in the order they were');
  await page.reload();
  await page.getByRole('heading', { name: 'Shop tickets', level: 1 }).waitFor();
  expect('the pin to be kept when the page is read again', (await pin.getAttribute('aria-pressed')) === 'true');
  await listed('Pinned', 'Shop | Shop tickets', 'what is pinned to be kept when the page is read again');

  // ---- an address that opens the page: its path names the thing, and the bar follows the tab showing (`headaddress.mjs`)
  await addresses({ page, context, browser, url, link, gear, menu, open, signIn, expect, problems });

  // ---- a library: shared by its pages, read and written out
  const share = acts.getByRole('button', { name: 'Share', exact: true });
  const press = async (opener, name) => {
    await opener.click();
    await menu.getByRole('menuitem', { name, exact: true }).click();
    await menu.waitFor({ state: 'detached' });
  };
  const pasted = () => page.evaluate(() => navigator.clipboard.readText());
  await open('/library/shop/docs', 'Shop docs');
  await edited.waitFor();
  expect(`a library's head to have Share before its address; got ${await drawn()}`, (await drawn()) === 'edited | Follow | Share | Copy link | Pin | More for this page');
  await frame("on a library's page");
  await page.screenshot({ path: `${out}/head-04-library.png` });
  await share.click();
  await menu.waitFor();
  let shared = await lines();
  expect(`Share on a library to offer its pages and to write them out; got ${shared.join(' | ')}`, shared.length === 4 && shared[0] === 'Read as pages' && shared[1] === "Copy the pages' address" && shared[2] === 'Write it out as pages' && /^Writes the documents marked public to site\//.test(shared[3]));
  await onScreen("for a library's Share");
  await page.screenshot({ path: `${out}/head-13-share-on-a-library.png` });
  const [asPages] = await Promise.all([context.waitForEvent('page'), menu.getByRole('menuitem', { name: 'Read as pages', exact: true }).click()]);
  await asPages.waitForLoadState();
  expect(`Read as pages to open the library as pages, got ${asPages.url()}`, asPages.url() === `${url}/library/shop/docs/`);
  await asPages.close();
  await press(share, "Copy the pages' address");
  expect(`the pages' address to be copied, got "${await pasted()}"`, (await pasted()) === `${url}/library/shop/docs/`);
  // Written out: the blueprint that ships is applied, and stays.
  await press(share, 'Write it out as pages');
  await page.locator('.toast', { hasText: 'Written out to site/' }).waitFor();
  expect('the pages to be in the project', (await stat(join(shop, 'site/index.html'))).isFile());
  await share.click();
  await menu.waitFor();
  shared = await lines();
  expect(`Share to say then what writes it out, and where; got ${shared.join(' | ')}`, shared.length === 4 && shared[2] === 'What writes it out site/' && /^It is written out to site\/ in the project .* the documents marked public/.test(shared[3]));
  await menu.getByRole('menuitem', { name: /^What writes it out/ }).click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'std/site · ' }).waitFor();

  // ---- a document: when and by whom, its mark, its page, a copy, and taken away
  await cli('record', 'set', 'shop-docs:welcome', 'tags=first');
  await open('/file/shop/docs/welcome.md', 'welcome.md');
  await page.waitForFunction(() => /^Last edited by Ada/.test(document.querySelector('.view-head .head-edited')?.getAttribute('title') ?? ''));
  expect(
    `a document's head to have Share, and the speech bubble that opens its comments right after the gear; got ${await drawn()}`,
    /^edited \| Share \| Copy link \| Pin \| More for this page \| Comments and details(, \d+ comments?)?$/.test(await drawn()),
  );
  await frame("on a document's page");
  await page.screenshot({ path: `${out}/head-05-document.png` });
  // What another program does to the file is nobody's to name: when is said, and nobody.
  const written = await at();
  await writeFile(join(shop, 'docs/welcome.md'), `${await readFile(join(shop, 'docs/welcome.md'), 'utf8')}\nMore, from another program.\n`);
  // Written a while after anything the factory did to it: a change in the very moment of one made through the factory is taken for that one.
  const later = new Date(Date.now() + 90_000);
  await utimes(join(shop, 'docs/welcome.md'), later, later);
  await cli('project', 'scan', 'shop');
  await page.waitForFunction((was) => Number(document.querySelector('.view-head .head-edited')?.getAttribute('data-at')) > was, written);
  const unseen = await edited.getAttribute('title');
  expect(`a file another program wrote to name nobody, got "${unseen}"`, /^Last edited \d/.test(unseen ?? ''));
  // The gear: what can be done with a file through the daemon as it is.
  await gear.click();
  await menu.waitFor();
  const forFile = await lines();
  expect(
    `the gear of a document to offer its address, a copy, its history, itself as Markdown, its path, and to delete it; got ${forFile.join(' | ')}`,
    forFile.slice(0, 8).join(' | ') === 'Copy link | Duplicate | History | Export as Markdown | Copy path | Delete this file… | Activity of Shop | Measurements of Shop' && /^Last edited /.test(forFile[8]) && forFile.length === 9,
  );
  expect('the line that deletes to be marked as taking something away', (await menu.locator('button.menu-grave').count()) === 1);
  await onScreen("for a document's gear");
  await page.screenshot({ path: `${out}/head-14-gear-on-a-document.png` });
  // Its history opens the panel beside it on History, and the bubble puts that away.
  await menu.getByRole('menuitem', { name: 'History', exact: true }).click();
  await page.locator('.doc-tools[data-rest] .journey').waitFor();
  await frame('on a document with the panel beside it');
  await acts.locator('.head-tool').click();
  await page.locator('.doc-tools, .doc-tools-leaving').waitFor({ state: 'detached' });
  await press(gear, 'Copy path');
  expect(`its path to be copied, got "${await pasted()}"`, (await pasted()) === 'docs/welcome.md');
  await gear.click();
  const [saved] = await Promise.all([page.waitForEvent('download'), menu.getByRole('menuitem', { name: 'Export as Markdown', exact: true }).click()]);
  const markdown = await readFile(await saved.path(), 'utf8');
  expect(`the document to be saved as the file it is, called ${saved.suggestedFilename()}`, saved.suggestedFilename() === 'welcome.md' && markdown === (await readFile(join(shop, 'docs/welcome.md'), 'utf8')));

  // Share: its mark, which the daemon writes into the file, and its page.
  await share.click();
  await menu.waitFor();
  shared = await lines();
  expect(`Share on a document to offer its mark and its page; got ${shared.join(' | ')}`, shared.length === 4 && shared[0] === 'Public' && /^A public document is written out/.test(shared[1]) && shared[2] === 'Open as a page' && shared[3] === "Copy the page's address");
  const mark = menu.getByRole('menuitemcheckbox', { name: 'Public', exact: true });
  expect('the mark to be off for a document that does not say it', (await mark.getAttribute('aria-checked')) === 'false');
  await mark.click();
  await menu.locator('[role="menuitemcheckbox"][aria-checked="true"]').waitFor();
  expect('the menu to stay while its switch is pressed', await menu.isVisible());
  expect('the mark to be written at the head of the file', /^---\n(.*\n)*?public: true\n(.*\n)*?---\n/.test(await readFile(join(shop, 'docs/welcome.md'), 'utf8')));
  await page.screenshot({ path: `${out}/head-15-share-on-a-document.png` });
  const [onePage] = await Promise.all([context.waitForEvent('page'), menu.getByRole('menuitem', { name: 'Open as a page', exact: true }).click()]);
  await onePage.waitForLoadState();
  expect(`Open as a page to open the document's page, got ${onePage.url()}`, onePage.url() === `${url}/library/shop/docs/welcome.md`);
  await onePage.close();
  await press(share, "Copy the page's address");
  expect(`the page's address to be copied, got "${await pasted()}"`, (await pasted()) === `${url}/library/shop/docs/welcome.md`);
  // Opened again it says how the document is now; pressed, the mark is off and gone from the file.
  await share.click();
  await menu.locator('[role="menuitemcheckbox"][aria-checked="true"]').waitFor();
  await mark.click();
  await menu.locator('[role="menuitemcheckbox"][aria-checked="false"]').waitFor();
  expect('the mark to be gone from the file', !/public: true/.test(await readFile(join(shop, 'docs/welcome.md'), 'utf8')));
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  // With something written and not saved the mark is not changed: the daemon would write it under what is being written.
  await page.locator('.view-head').getByRole('tab', { name: 'Source', exact: true }).click();
  await page.locator('.cm-content').click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('Unsaved words.');
  await page.locator('.view-head', { hasText: 'unsaved' }).waitFor();
  await share.click();
  await menu.waitFor();
  expect('the mark to be closed while there is something unsaved, saying why', (await mark.getAttribute('aria-disabled')) === 'true' && /^Save what is written first/.test((await mark.getAttribute('title')) ?? ''));
  await mark.click({ force: true });
  expect('a closed switch to do nothing, and the menu to stay', (await mark.getAttribute('aria-checked')) === 'false' && (await menu.isVisible()));
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  // A copy is of what is saved, so it is closed too until this is.
  await gear.click();
  await menu.waitFor();
  expect('Duplicate to be closed while there is something unsaved', (await menu.getByRole('menuitem', { name: 'Duplicate', exact: true }).getAttribute('aria-disabled')) === 'true');
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  await page.locator('.view-head').getByRole('button', { name: 'Save', exact: true }).click();
  await page.locator('.view-head .faint', { hasText: 'unsaved' }).waitFor({ state: 'detached' });

  // A copy goes beside it and opens; a second is numbered.
  const original = await readFile(join(shop, 'docs/welcome.md'), 'utf8');
  await press(gear, 'Duplicate');
  await page.locator('.crumbs h1', { hasText: 'welcome-copy.md' }).waitFor();
  expect('the copy to say what the document says', (await readFile(join(shop, 'docs/welcome-copy.md'), 'utf8')) === original);
  await open('/file/shop/docs/welcome.md', 'welcome.md');
  await press(gear, 'Duplicate');
  await page.locator('.crumbs h1', { hasText: 'welcome-copy-2.md' }).waitFor();
  expect('a second copy to be numbered, and the first to be as it was', (await readFile(join(shop, 'docs/welcome-copy-2.md'), 'utf8')) === original && (await readFile(join(shop, 'docs/welcome-copy.md'), 'utf8')) === original);

  // Moved to the Trash behind a question: its tab closes, and what this browser kept of it goes with it.
  await open('/file/shop/docs/welcome-copy.md', 'welcome-copy.md');
  await pin.click();
  await away();
  await listed('Pinned', 'Shop | Shop tickets | welcome-copy.md', 'the copy to be pinned');
  await gear.click();
  await menu.getByRole('menuitem', { name: 'Delete this file…', exact: true }).click();
  const asks = page.getByRole('dialog');
  await asks.getByRole('heading', { name: 'Move welcome-copy.md to the Trash?' }).waitFor();
  const question = (await asks.innerText()).replace(/\s+/g, ' ');
  expect(`the question to name the file and say it goes to the Trash, got "${question}"`, question.includes("docs/welcome-copy.md leaves the project's folder") && question.includes('is given back from there'));
  await page.screenshot({ path: `${out}/head-16-delete-asked.png` });
  await asks.getByRole('button', { name: 'Cancel', exact: true }).click();
  await asks.waitFor({ state: 'detached' });
  expect('a file to be there still when the question is put away', (await stat(join(shop, 'docs/welcome-copy.md'))).isFile());
  await gear.click();
  await menu.getByRole('menuitem', { name: 'Delete this file…', exact: true }).click();
  await asks.getByRole('button', { name: 'Move to Trash', exact: true }).click();
  await asks.waitFor({ state: 'detached' });
  await page.locator('.tabs .tab', { hasText: 'welcome-copy.md' }).waitFor({ state: 'detached' });
  await page.locator('.toast', { hasText: 'welcome-copy.md: Moved to Trash, kept 30 days.' }).waitFor();
  expect('the file to be gone from the project', await stat(join(shop, 'docs/welcome-copy.md')).then(() => false, () => true));
  await away();
  await listed('Pinned', 'Shop | Shop tickets', 'what was pinned of a deleted file to go with it');
  await listed('Recents', ['welcome-copy.md'], 'a deleted file not to be listed as lately opened');
  await open('/library/shop/docs', 'Shop docs');
  await page.locator('.library-line', { hasText: 'welcome-copy-2.md' }).waitFor();
  await page.locator('.library-line .id', { hasText: /^welcome-copy\.md$/ }).waitFor({ state: 'detached' });

  // A file that is no document: nothing to share it by, no history, and saved as a copy of itself.
  await open('/file/shop/notes.txt', 'notes.txt');
  await edited.waitFor();
  expect(`a file that is no document to have no Share; got ${await drawn()}`, (await drawn()) === 'edited | Copy link | Pin | More for this page');
  await gear.click();
  await menu.waitFor();
  const forOther = await lines();
  expect(`its gear to offer a copy, itself, its path and to delete it; got ${forOther.join(' | ')}`, forOther.slice(0, 5).join(' | ') === 'Copy link | Duplicate | Save a copy | Copy path | Delete this file…');
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  await frame("on a file's page");

  // Somebody who may not write the project's files is offered neither a copy nor to delete, and is not theirs to mark.
  const reader = await browser.newContext({ viewport: { width: 1350, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const lesser = await reader.newPage();
  lesser.on('pageerror', (err) => problems.push(err.message));
  // What they hold, as the daemon says it, without what writes a project's files and its records.
  await lesser.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const who = await answer.json();
    who.principal.scopes = ['projects:read', 'records:read', 'board:read', 'channels', 'search', 'blueprints:read', 'fleet:read', 'agents:read'];
    await route.fulfill({ response: answer, json: who });
  });
  await lesser.goto(`${url}/#/file/shop/docs/welcome.md`);
  await signIn(lesser);
  await lesser.locator('.crumbs h1', { hasText: 'welcome.md' }).waitFor();
  await lesser.locator('.view-head .head-acts').getByRole('button', { name: 'More for this page', exact: true }).click();
  const theirGear = await lesser.getByRole('menu').locator('button').evaluateAll((all) => all.map((one) => [...one.querySelectorAll('.truncate, .hint')].map((part) => part.textContent).join(' ').replace(/\s+/g, ' ').trim()));
  expect(`somebody who may not write files to be offered no copy and nothing to delete; got ${theirGear.join(' | ')}`, theirGear.join(' | ') === 'Copy link | History | Export as Markdown | Copy path | Activity of Shop | Measurements of Shop');
  await lesser.keyboard.press('Escape');
  await lesser.locator('.view-head .head-acts').getByRole('button', { name: 'Share', exact: true }).click();
  const theirMark = lesser.getByRole('menu').getByRole('menuitemcheckbox', { name: 'Public', exact: true });
  expect('the mark to be closed to somebody who may not change the document', (await theirMark.getAttribute('aria-disabled')) === 'true' && /records:write/.test((await theirMark.getAttribute('title')) ?? ''));
  await reader.close();

  // ---- a table written out as a CSV file, and one read back in
  // Asked of the daemon from inside the page, as whoever is signed in.
  const ask = (method, path, body) =>
    page.evaluate(
      async ([how, where, what]) => {
        const answer = await fetch(where, { method: how, headers: { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' }, body: what === undefined ? undefined : JSON.stringify(what) });
        return answer.json();
      },
      [method, path, body],
    );
  const B = String.fromCharCode(92);
  await ask('PATCH', '/api/records/shop-tickets-2', { fields: { title: 'Crates, "large"', state: 'open', tags: ['wood', 'a; b'], urgent: true, notes: 'two\nlines' } });
  await open('/table/shop/tickets', 'Shop tickets');
  await page.locator('table.grid tr[data-row="shop-tickets-2"]', { hasText: 'Crates, "large"' }).waitFor();
  const save = async (opener, name) => {
    await opener.click();
    const [file] = await Promise.all([page.waitForEvent('download'), menu.getByRole('menuitem', { name }).click()]);
    return { name: file.suggestedFilename(), text: await readFile(await file.path(), 'utf8') };
  };
  const tickets = await save(gear, /^Export as CSV/);
  // Every record, a line each; a cell in quotation marks where it holds a comma, a quotation mark or a line's end; a list with `; ` between its things.
  const wanted = ['id,title,state,tags,points,urgent,notes', 'shop-tickets-1,Shelving,,,8,,', `shop-tickets-2,"Crates, ""large""",open,wood; a${B}; b,,true,"two\nlines"`, 'shop-tickets-3,Labels,,,,,', ''].join('\r\n');
  expect(`the table to be written out cell for cell as shop-tickets.csv, got ${tickets.name}: ${JSON.stringify(tickets.text)}`, tickets.name === 'shop-tickets.csv' && tickets.text === wanted);
  await gear.click();
  await menu.waitFor();
  expect(`the line that writes it out to say how many records, got ${(await lines()).join(' | ')}`, (await lines()).slice(0, 4).join(' | ') === 'Copy link | Rename… | Export as CSV 3 tickets | Import or merge a CSV…');
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  const read = async (opener, called, text) => {
    await writeFile(join(out, called), text);
    await opener.click();
    const [chooser] = await Promise.all([page.waitForEvent('filechooser'), menu.getByRole('menuitem', { name: 'Import or merge a CSV…', exact: true }).click()]);
    await chooser.setFiles(join(out, called));
    await page.getByRole('dialog').getByRole('heading', { name: new RegExp(`^Import ${called.replace('.', '.')} into`) }).waitFor();
    return page.getByRole('dialog');
  };
  // Read straight back, it changes nothing, and says so before anything is done.
  let reading = await read(gear, 'head-tickets.csv', tickets.text);
  expect(`a table read straight back to change nothing, got "${await reading.locator('.csv-plan').innerText()}"`, (await reading.locator('.csv-plan').innerText()).trim() === '0 tickets to add, 0 to change, 3 that say what is there already.');
  expect('nothing to be offered to do', await reading.getByRole('button', { name: 'Nothing to import', exact: true }).isDisabled());
  await reading.getByRole('button', { name: 'Cancel', exact: true }).click();
  await reading.waitFor({ state: 'detached' });
  // Changed in another program: a cell changed, a record added, a line that cannot be read, one that names no
  // record here, one the daemon refuses, and a column that is no field of the table.
  const merged = [
    'id,title,state,tags,points,urgent,notes,colour',
    'shop-tickets-1,Shelving,,,13,,,red',
    `shop-tickets-2,"Crates, ""large""",open,wood;a${B}; b,,TRUE,"two\nlines",`,
    'shop-tickets-3,"Labels, printed",,,,,,',
    ',Paint,open,tin; brush,2,yes,,blue',
    ',Brushes,,,many,,,',
    'shop-tickets-99,Ghost,,,,,,',
    ',Varnish,broken,,1,,,',
    '',
  ].join('\n');
  reading = await read(gear, 'head-merged.csv', merged);
  const planned = (await reading.innerText()).replace(/\s+/g, ' ');
  expect(`what the file would do to be said first, got "${planned}"`, planned.includes('2 tickets to add, 2 to change, 1 that says what is there already.'));
  // Each by where it begins in the file, as an editor counts its lines: a cell above them holds a line's end.
  expect(`the lines that cannot be read to be said with why, got "${planned}"`, planned.includes('2 lines cannot be read in, and are left out:') && planned.includes('Line 7: `points` is a number, and "many" is not one') && planned.includes('Line 8: no record of this table is called shop-tickets-99'));
  expect(`a column that is no field to be said to be left out, got "${planned}"`, planned.includes('A column that is no field of this table is left out: colour.'));
  await page.screenshot({ path: `${out}/head-17-csv-read-in.png` });
  const before = await ask('GET', '/api/collections/shop/tickets/records');
  expect('nothing to be written until it is asked for', before.records.length === 3 && before.records.find((one) => one.id === 'shop-tickets-1').data.points === 8);
  await reading.getByRole('button', { name: 'Add 2 and change 2', exact: true }).click();
  await reading.getByRole('button', { name: 'Done', exact: true }).waitFor();
  const came = (await reading.innerText()).replace(/\s+/g, ' ');
  // The one the daemon refuses is said with its reason, and the rest are written.
  expect(`what was done to be said, with what was refused and why, got "${came}"`, came.includes('1 added, 2 changed, 1 refused:') && /Line 9: .*broken/.test(came));
  await reading.getByRole('button', { name: 'Done', exact: true }).click();
  await reading.waitFor({ state: 'detached' });
  const after = (await ask('GET', '/api/collections/shop/tickets/records')).records;
  const by = (title) => after.find((one) => one.data.title === title)?.data;
  expect(`the table to read as the file said, got ${JSON.stringify(after.map((one) => one.data))}`, after.length === 4 && by('Shelving').points === 13 && by('Labels, printed') && by('Crates, "large"').urgent === true && JSON.stringify(by('Paint')) === JSON.stringify({ ...by('Paint'), title: 'Paint', state: 'open', tags: ['tin', 'brush'], points: 2, urgent: true }) && !by('Brushes') && !by('Ghost') && !by('Varnish'));
  // And the page shows it, not read again.
  await page.locator('table.grid tr[data-row]', { hasText: 'Paint' }).waitFor();
  await page.locator('table.grid tr[data-row="shop-tickets-3"]', { hasText: 'Labels, printed' }).waitFor();
  expect('the table to show four tickets', (await page.locator('table.grid tr[data-row]').count()) === 4);

  // The board's cards the same way: written out, and read in as cards are added and changed.
  await open('/board/shop', 'Shop board');
  await page.locator('.card', { hasText: 'A first card' }).first().waitFor();
  const cards = await save(gear, /^Export as CSV/);
  expect(`the board to be written out as shop-board.csv with its card, got ${cards.name}: ${JSON.stringify(cards.text)}`, cards.name === 'shop-board.csv' && /^id,title,/.test(cards.text) && cards.text.includes('A first card'));
  reading = await read(gear, 'head-board.csv', `${cards.text.replace('A first card', 'A first card renamed')},A second card\n`);
  expect(`a card to change and a card to add, got "${await reading.locator('.csv-plan').innerText()}"`, (await reading.locator('.csv-plan').innerText()).trim() === '1 card to add, 1 to change.');
  await reading.getByRole('button', { name: 'Add 1 and change 1', exact: true }).click();
  await reading.locator('[role="status"]', { hasText: '1 added, 1 changed.' }).waitFor();
  await reading.getByRole('button', { name: 'Done', exact: true }).click();
  const board = await ask('GET', '/api/board/shop');
  expect(`the board to hold both cards as the file said, got ${board.items.map((one) => one.title).join(' | ')}`, board.items.length === 2 && board.items.some((one) => one.title === 'A first card renamed') && board.items.some((one) => one.title === 'A second card'));
  await page.locator('.card', { hasText: 'A second card' }).first().waitFor();

  // A library is no table to write out: its documents are files. And somebody who may not change a table's records is offered it written out and nothing to read in.
  await open('/library/shop/docs', 'Shop docs');
  await gear.click();
  await menu.waitFor();
  expect(`a library's gear to offer no CSV, got ${(await lines()).join(' | ')}`, !(await lines()).some((one) => /CSV/.test(one)));
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  const looker = await browser.newContext({ viewport: { width: 1350, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const looks = await looker.newPage();
  looks.on('pageerror', (err) => problems.push(err.message));
  await looks.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const who = await answer.json();
    who.principal.scopes = ['projects:read', 'records:read', 'board:read', 'channels', 'search', 'blueprints:read', 'fleet:read', 'agents:read'];
    await route.fulfill({ response: answer, json: who });
  });
  await looks.goto(`${url}/#/table/shop/tickets`);
  await signIn(looks);
  await looks.locator('table.grid tr[data-row]').first().waitFor();
  await looks.locator('.view-head .head-acts').getByRole('button', { name: 'More for this page', exact: true }).click();
  const theirTable = await looks.getByRole('menu').locator('button').evaluateAll((all) => all.map((one) => [...one.querySelectorAll('.truncate, .hint')].map((part) => part.textContent).join(' ').replace(/\s+/g, ' ').trim()));
  expect(`somebody who may not change its records to be offered the table written out and nothing to read in; got ${theirTable.join(' | ')}`, theirTable.includes('Export as CSV 4 tickets') && !theirTable.some((one) => /^Import/.test(one)));
  await looker.close();

  // ---- at every width, the side panel open and folded, and a long name
  const pages = [
    ['/table/shop/tickets', 'Shop tickets', 'a table'],
    ['/board/shop', 'Shop board', 'the board'],
    [`/table/${long}/tickets`, 'A workshop with a long name for its sign tickets', 'a table of a project with a long name'],
    [`/files/${long}`, 'A workshop with a long name for its sign repo', 'the files of a project with a long name'],
    [`/board/${long}`, 'A workshop with a long name for its sign board', 'the board of a project with a long name'],
    [`/library/${long}/docs`, 'A workshop with a long name for its sign docs', 'the library of a project with a long name'],
    [`/file/${long}/docs/what-the-workshop-keeps-on-its-longest-shelves.md`, 'what-the-workshop-keeps-on-its-longest-shelves.md', 'a document called at length'],
    [`/project/${long}`, 'A workshop with a long name for its sign', 'the page of a project with a long name'],
    [`/mockups/${long}/mockups`, 'A workshop with a long name for its sign mockups', 'the mockups of a project with a long name'],
  ];
  const fold = page.getByRole('button', { name: /the side panel$/ }).first();
  for (const width of [1350, 900, 600]) {
    await page.setViewportSize({ width, height: 860 });
    for (const folded of [false, true]) {
      if (((await fold.getAttribute('aria-expanded')) === 'true') === folded) await fold.click();
      for (const [hash, heading, what] of pages) {
        await open(hash, heading);
        await frame(`on ${what} at ${width} wide with the side panel ${folded ? 'folded' : 'open'}`);
      }
      // The menu is on the screen there too, and moves nothing.
      await open('/table/shop/tickets', 'Shop tickets');
      // Its rows read and who changed it known: what the head says by itself has been said.
      await page.waitForFunction(() => /^Last edited by Ada/.test(document.querySelector('.view-head .head-edited')?.getAttribute('title') ?? ''));
      const before = await boxes();
      await gear.click();
      await menu.waitFor();
      await onScreen(`at ${width} wide with the side panel ${folded ? 'folded' : 'open'}`);
      expect(`nothing of the head to move as the menu opens at ${width} wide`, (await boxes()) === before);
      if (width === 600 && folded) await page.screenshot({ path: `${out}/head-10-narrow.png` });
      await page.keyboard.press('Escape');
      await menu.waitFor({ state: 'detached' });
      // And the board's gear, with its tables and its tools, and the layout it hangs.
      await open('/board/shop', 'Shop board');
      await boardGearAt({ page, out, expect }, `at ${width} wide with the side panel ${folded ? 'folded' : 'open'}`, width === 600 && !folded);
    }
  }
  await page.setViewportSize({ width: 1350, height: 860 });
  if ((await fold.getAttribute('aria-expanded')) !== 'true') await fold.click();

  // ---- a phone
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: process.env.SCHEME ?? 'dark' });
  const small = await phone.newPage();
  small.on('pageerror', (err) => problems.push(err.message));
  await small.goto(`${url}/#/table/shop/tickets`);
  await small.getByLabel('Email').fill('ada@example.com');
  await small.getByLabel('Password').fill('correct horse');
  await small.getByRole('button', { name: 'Sign in', exact: true }).last().tap();
  await small.getByRole('heading', { name: 'Shop tickets', level: 1 }).waitFor();
  const onPhone = async (hash, heading, what) => {
    await small.evaluate((to) => (location.hash = to), hash);
    await small.getByRole('heading', { name: heading, level: 1 }).waitFor();
    await small.locator('.view-head .head-acts').waitFor();
    const got = await small.locator('.view-head').first().evaluate((head) => {
      const box = (one) => {
        const at = one.getBoundingClientRect();
        return { left: at.left, right: at.right, top: at.top, bottom: at.bottom, is: one.className || one.tagName };
      };
      const there = (one) => one.getClientRects().length > 0 && !one.classList.contains('grow') && !one.classList.contains('head-break');
      return { parts: [...head.children].filter(there).map(box), inside: [...head.querySelector('.head-acts').children].filter(there).map(box), wide: document.documentElement.scrollWidth - window.innerWidth, room: window.innerWidth };
    });
    const over = (a, b) => a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5;
    for (const all of [got.parts, got.inside]) {
      for (const [nth, one] of all.entries()) {
        expect(`everything of the head to be on the screen on a phone, on ${what}: ${JSON.stringify(one)}`, one.left >= -0.5 && one.right <= got.room + 0.5);
        for (const other of all.slice(nth + 1)) expect(`nothing of the head to lie over anything else of it on a phone, on ${what}: ${JSON.stringify(one)} and ${JSON.stringify(other)}`, !over(one, other));
      }
    }
    expect(`the page not to scroll sideways on a phone, on ${what}; it is ${got.wide}px too wide`, got.wide <= 0);
    await centred(small, `on a phone, on ${what}`);
  };
  for (const [hash, heading, what] of pages) await onPhone(hash, heading, what);
  await onPhone('/table/shop/tickets', 'Shop tickets', 'a table');
  await small.locator('table.grid tr[data-row]').first().waitFor();
  // When it was changed is not in a phone's head, which has no room for it: the gear's menu says it.
  await small.locator('.view-head .head-edited').waitFor({ state: 'attached' });
  expect("when it was changed not to take room in a phone's head", !(await small.locator('.view-head .head-edited').isVisible()));
  await onPhone('/table/shop/tickets', 'Shop tickets', 'a table once it is read');
  await small.locator('.view-head .head-acts').getByRole('button', { name: 'More for this page', exact: true }).tap();
  const theirMenu = small.getByRole('menu');
  await theirMenu.waitFor();
  const [menuAt, room] = [await theirMenu.boundingBox(), small.viewportSize()];
  expect(`the menu to be on a phone's screen: ${JSON.stringify(menuAt)}`, menuAt.x >= 0 && menuAt.y >= 0 && menuAt.x + menuAt.width <= room.width && menuAt.y + menuAt.height <= room.height);
  expect("a phone's menu to say when it was changed", /^Last edited/.test((await theirMenu.locator('.menu-said').innerText()).trim()));
  await small.screenshot({ path: `${out}/head-11-phone.png` });
  await small.keyboard.press('Escape');
  await theirMenu.waitFor({ state: 'detached' });
  await onPhone('/board/shop', 'Shop board', 'the board');
  await boardGearOnPhone({ page: small, out, expect });
  await phone.close();

  // ---- the other of dark and light
  const other = (process.env.SCHEME ?? 'dark') === 'dark' ? 'light' : 'dark';
  await litAs(page, other);
  await open('/table/shop/tickets', 'Shop tickets');
  await gear.click();
  await menu.waitFor();
  await page.screenshot({ path: `${out}/head-12-${other}.png` });
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  await litAs(page, process.env.SCHEME ?? 'dark');

  if (problems.length > 0) throw new Error(`the page reported errors: ${problems.join(' | ')}`);
  await browser.close();
  console.log(`the head: pictures in ${out}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((done) => daemon.once('exit', done));
}
if (failed) {
  console.error(failed.stack ?? failed);
  process.exit(1);
}
