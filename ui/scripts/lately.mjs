// What somebody keeps to hand at the top of the Resources panel: what
// they pinned, and what they lately opened.
//
//   DAEMON=../target/debug/factory node scripts/lately.mjs [out-dir]
//
// With nothing opened neither section is drawn. Then things are opened as
// a person opens them (a board, a file of each of two projects, a
// project's page and its files, a blueprint, something running), and are
// listed the latest first, eight at most, each with its mark and what its
// tab is called, and whose it is where two read alike; the factory's own
// three pages, which are always listed under these, are not. A line pressed
// shows its tab, and the list holds still while the pointer is on it or
// the keyboard is in it: the line is where it was until the pointer has
// left, and then it is first. One is pinned from its line and one from a
// project's row, and each is gone from what was lately opened; unpinned,
// each is back where it belongs. One is removed from Recents by its
// line's menu, which a pinned line's does not offer, and is gone at once
// while the pointer is on the panel; opened again, it is the latest. All
// of it is kept when the page is read
// again. More workspaces are then made from a terminal beside the one the
// factory has from the first, and both lists are of the workspace the
// panel is of, with what is the factory's own in each.
// A project is forgotten from a terminal, and its lines are gone and are
// taken out of what the browser kept; what is pinned is the account's, and
// its pin is kept there and passed over. The headings are drawn as the
// panel's others are, nothing lies over anything else, and nothing
// scrolls sideways, with the panel dragged narrow and at a phone's width
// too. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'lately-data');
const home = join(out, 'lately-home');
// Each project's folder is one that was said, so that its Repo is listed under it: folders of their own, beside the data.
const folders = { Shop: join(out, 'lately-shop'), Depot: join(out, 'lately-depot'), Shed: join(out, 'lately-shed') };

await mkdir(out, { recursive: true });
for (const dir of [data, home, ...Object.values(folders)]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  for (const name of ['Shop', 'Depot', 'Shed']) await cli('project', 'create', name, '--bare', '--path', folders[name]);
  // One board, so that one thing is running: whose it is, is read when it is listed.
  await cli('apply', 'std/board', '--project', 'shop');
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1350, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
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
  const side = async (name) => {
    const button = page.locator('.rail').getByRole('button', { name, exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
    await page.getByRole('navigation', { name }).waitFor();
    // Everything of the Resources panel starts folded: what it has read is unfolded as a person would (`folded.mjs`).
    if (name === 'Resources') await unfoldAll(page);
  };
  const resources = page.getByRole('navigation', { name: 'Resources' });
  // A section as it is listed: each line's name, with whose it is where that is said. Nothing where the section is not drawn.
  const section = (called) =>
    resources.evaluate((nav, name) => {
      const head = [...nav.querySelectorAll('.nav > h3')].find((one) => one.textContent.trim() === name);
      if (!head) return '(not drawn)';
      return [...head.nextElementSibling.querySelectorAll('.lately-line')]
        .map((row) => `${row.querySelector('.truncate').textContent}${row.querySelector('.lately-whose') ? ` (${row.querySelector('.lately-whose').textContent})` : ''}`)
        .join(' | ');
    }, called);
  const pinned = () => section('Pinned');
  const recent = () => section('Recents');
  const listed = async (read, wanted, what) => {
    const until = Date.now() + 8000;
    let got = await read();
    while (got !== wanted && Date.now() < until) {
      await page.waitForTimeout(60);
      got = await read();
    }
    expect(`${what}: "${wanted}", got "${got}"`, got === wanted);
  };
  const choose = async (label) => {
    const menu = page.getByRole('menu');
    await menu.waitFor();
    await menu.getByText(label, { exact: true }).click();
    await menu.waitFor({ state: 'detached' });
  };
  const more = async (row, label) => {
    await row.hover();
    await row.getByRole('button', { name: label, exact: true }).click();
  };
  // The pointer taken off the panel: on the page beside it, where nothing answers to being pointed at.
  const away = () => page.mouse.move(980, 700);
  const line = (within, name) => resources.locator(`.lately-${within} .lately-line`, { has: page.locator('.truncate', { hasText: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`) }) });
  const tree = (id) => resources.locator(`[data-project="${id}"] > .nav-row`);
  // Where something is in the panel's list, however far the list is scrolled: what moves it is something coming in above it.
  const placed = (locator) =>
    locator.evaluate((one) => {
      const list = one.closest('.nav');
      const [at, within] = [one.getBoundingClientRect(), list.getBoundingClientRect()];
      return Math.round((at.top - within.top + list.scrollTop) * 10) / 10;
    });
  const showingTab = () => page.locator('.tabs .tab[aria-selected="true"] .truncate').first().innerText();
  // What this browser keeps of both lists, under who is signed in: what was lately opened, and its copy of what the
  // account keeps of what is pinned, which is there only so that the panel is drawn at once.
  const keptText = () =>
    page.evaluate(() =>
      Object.entries(localStorage)
        .filter(([key]) => key.startsWith('client.side.lately:user:') || key.startsWith('client.side.arranged:user:'))
        .map(([, value]) => value)
        .join(' '),
    );
  // What the account keeps of what is pinned, which follows its person to any browser (`arranged.mjs`).
  const keptThere = () =>
    page.evaluate(async () => {
      const asked = await fetch('/api/arranged', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      return JSON.stringify((await asked.json()).arrangement?.things ?? []);
    });
  const narrow = async () => {
    const wide = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`the page not to scroll sideways, and it is ${wide}px too wide`, wide <= 0);
  };
  // The top of the panel: the sections in their order, their headings drawn as the panel's others are, every line
  // inside the list with nothing of it lying over anything else of it, and nothing scrolling sideways.
  const frame = async (where, sections) => {
    const got = await resources.evaluate((nav) => {
      const round = (figure) => Math.round(figure * 10) / 10;
      const box = (one) => {
        const at = one.getBoundingClientRect();
        return { left: round(at.left), right: round(at.right), top: round(at.top), bottom: round(at.bottom) };
      };
      const list = nav.querySelector('.nav');
      const drawn = (one) => {
        const style = getComputedStyle(one);
        return [style.fontSize, style.fontWeight, style.letterSpacing, style.textTransform, style.color, style.paddingLeft, style.paddingTop].join(' ');
      };
      const heads = [...list.querySelectorAll(':scope > h3')].map((one) => ({ says: one.childNodes[0].textContent.trim(), drawn: drawn(one), at: box(one) }));
      const lines = [...list.querySelectorAll('.lately-line')].map((row) => ({
        row: box(row),
        parts: [...row.querySelector('.nav-item').children, row.querySelector('.row-more')].filter(Boolean).map(box),
        name: box(row.querySelector('.truncate')),
      }));
      const foot = nav.querySelector('.space-foot');
      return { list: box(list), heads, lines, foot: foot ? box(foot) : null, sideways: Math.max(nav.scrollWidth - nav.clientWidth, list.scrollWidth - list.clientWidth) };
    });
    const told = JSON.stringify(got);
    const says = got.heads.map((one) => one.says).join(' | ');
    expect(`the sections of the panel to be ${sections} ${where}, got ${says}`, says === sections);
    expect(`the sections to be one under another ${where}: ${told}`, got.heads.every((one, at) => at === 0 || got.heads[at - 1].at.bottom <= one.at.top + 0.5));
    expect(`every heading to be drawn as the others are ${where}: ${told}`, new Set(got.heads.map((one) => one.drawn)).size === 1);
    expect(`some lines to be listed ${where}`, got.lines.length > 0);
    for (const one of got.lines) {
      const apart = one.parts.every((part, at) => at === 0 || one.parts[at - 1].right <= part.left + 0.5);
      const inside = one.parts.every((part) => part.left >= one.row.left - 0.5 && part.right <= one.row.right + 0.5);
      expect(`nothing of a line to lie over anything else of it, and all of it to be in its row and the list ${where}: ${JSON.stringify(one)} in ${JSON.stringify(got.list)}`, apart && inside && one.row.left >= got.list.left - 0.5 && one.row.right <= got.list.right + 0.5);
      expect(`a line's name to have room to be read in ${where}: ${JSON.stringify(one)}`, one.name.right - one.name.left >= 24);
    }
    expect(`the lines to be one under another ${where}: ${told}`, got.lines.every((one, at) => at === 0 || got.lines[at - 1].row.bottom <= one.row.top + 0.5));
    expect(`the list to end above the strip at the panel's foot ${where}: ${told}`, !got.foot || got.list.bottom <= got.foot.top + 0.5);
    expect(`nothing of the panel to scroll sideways ${where}: ${told}`, got.sideways <= 0);
    await narrow();
  };

  // ---- with nothing opened: neither section is drawn
  await side('Resources');
  await tree('shop').waitFor();
  expect(`nothing pinned and nothing lately opened to begin with, got "${await pinned()}" and "${await recent()}"`, (await pinned()) === '(not drawn)' && (await recent()) === '(not drawn)');
  expect(`nothing to be kept yet, got "${await keptText()}"`, (await keptText()) === '');
  await page.screenshot({ path: `${out}/lately-00-nothing-opened.png` });

  // ---- things opened, as a person opens them: listed the latest first
  // What is pressed in the panel, by what it says: a line under Factory, a family of blueprints and one of them, and what a project has.
  const says = (words) => page.getByText(words, { exact: true });
  const factory = (name) => resources.locator('.nav > .tree > button.nav-item', { has: says(name) });
  const has = (id, name) => resources.locator(`[data-project="${id}"] .tree button.nav-item`, { has: says(name) });
  const readme = async (id, name) => {
    const fold = resources.getByRole('button', { name: `Unfold ${name} repo`, exact: true });
    if ((await fold.count()) > 0) await fold.click();
    await has(id, 'README.md').click();
  };
  // The list holds still while the panel is pointed at, so what it comes to is read with the pointer off the panel.
  await has('shop', 'Board').click();
  expect(`nothing to come into the panel under the pointer, got "${await recent()}"`, (await recent()) === '(not drawn)');
  await away();
  await listed(recent, 'Shop board', 'the first thing opened');
  expect(`nothing to be pinned by opening, got "${await pinned()}"`, (await pinned()) === '(not drawn)');
  await readme('shop', 'Shop');
  await has('shop', 'Repo').click();
  await away();
  await listed(recent, 'Shop repo | README.md | Shop board', 'three things opened, the latest first');
  await tree('depot').locator('.nav-item').click();
  await readme('depot', 'Depot');
  await away();
  // Two that read alike each say whose they are; one that reads like no other says nothing more.
  await listed(recent, 'README.md (Depot) | Depot | Shop repo | README.md (Shop) | Shop board', 'two files called the same, each saying whose it is');
  await tree('shed').locator('.nav-item').click();
  await unfold(page, resources.locator('.nav > .tree > div > button.nav-item', { has: says('std') }));
  // Under the family `std`: a project's own list of the blueprints it uses says some of the same names.
  const shipped = (name) => resources.locator('.nav > .tree > div', { has: page.locator(':scope > button.nav-item', { has: says('std') }) }).locator('.tree button.nav-item', { has: says(name) });
  await shipped('std/board').click();
  // What looks after a board's cards is a part of the board: listed under it, once its parts are unfolded.
  await unfold(page, 'the parts of std/board');
  await shipped('std/cards').click();
  await away();
  await listed(recent, 'std/cards | std/board | Shed | README.md (Depot) | Depot | Shop repo | README.md (Shop) | Shop board', 'eight things opened');
  // The factory's own pages are lines that are always there, under these: opened, they are not listed again above themselves.
  await factory('Measurements').click();
  await factory('Activity').click();
  await away();
  await listed(recent, 'std/cards | std/board | Shed | README.md (Depot) | Depot | Shop repo | README.md (Shop) | Shop board', 'the factory\'s own pages not listed among what was lately opened');
  // Something running, opened from where what runs is listed: it is called what it runs. With ten things opened, the latest eight are listed.
  await page.locator('tr.clickable', { hasText: 'std/board' }).first().click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'std/board' }).waitFor();
  const runs = await showingTab();
  expect(`something running to be called what it runs, got "${runs}"`, runs.startsWith('std/board · '));
  await tree('shop').locator('.nav-item').click();
  await away();
  const eight = `Shop | ${runs} | std/cards | std/board | Shed | README.md | Depot | Shop repo`;
  await listed(recent, eight, 'ten things opened, the latest eight listed');
  await frame('at the usual width', 'Recents | Projects | Factory | Blueprints');
  await page.screenshot({ path: `${out}/lately-01-lately-opened.png` });

  // ---- a line pressed shows its tab, and the list holds still while the panel is pointed at
  const shed = line('recent', 'Shed');
  const before = await placed(shed);
  await shed.locator('.nav-item').click();
  await listed(showingTab, 'Shed', 'a line pressed to show its tab');
  expect('the line to say it is what is showing', (await shed.locator('.nav-item').getAttribute('aria-current')) === 'true');
  // The pointer is still on it: it is where it was, though it is the latest now.
  expect(`the list to hold still while it is pointed at, got "${await recent()}"`, (await recent()) === eight);
  expect(`the line that was pressed to be where it was, under the pointer: at ${before} and then at ${await placed(shed)}`, (await placed(shed)) === before);
  // Another pressed while it is held: still nothing moves.
  await line('recent', 'Depot').locator('.nav-item').click();
  await listed(showingTab, 'Depot', 'a second line pressed to show its tab');
  expect(`the list to hold still through a second press, got "${await recent()}"`, (await recent()) === eight);
  // And something opened from the lists below, which is not among the eight: it does not come in at the top and push what was pressed from under the pointer.
  const under = await placed(has('shop', 'Board'));
  await has('shop', 'Board').click();
  await listed(showingTab, 'Shop board', 'something of a project pressed to show its tab');
  expect(`nothing to come into the list while the panel is pointed at, got "${await recent()}"`, (await recent()) === eight);
  expect(`what was pressed to be where it was, under the pointer: at ${under} and then at ${await placed(has('shop', 'Board'))}`, (await placed(has('shop', 'Board'))) === under);
  // The pointer leaves the panel: the list is put right, each thing once, the latest first.
  await away();
  const moved = `Shop board | Depot | Shed | Shop | ${runs} | std/cards | std/board | README.md`;
  await listed(recent, moved, 'the list put right when the pointer has left the panel');
  // And while the keyboard is in it: a line reached by the keyboard and pressed by it stays where it is until the keyboard has left.
  await line('recent', 'std/cards').locator('.nav-item').focus();
  await page.keyboard.press('Tab');
  await page.keyboard.press('Shift+Tab');
  expect('the keyboard to be on the line', await line('recent', 'std/cards').locator('.nav-item').evaluate((one) => document.activeElement === one && one.matches(':focus-visible')));
  await page.keyboard.press('Enter');
  await listed(showingTab, 'std/cards', 'a line pressed by the keyboard to show its tab');
  expect(`the list to hold still while the keyboard is in it, got "${await recent()}"`, (await recent()) === moved);
  await page.locator('.view:not([hidden]) .view-head h1').first().click();
  await listed(recent, `std/cards | Shop board | Depot | Shed | Shop | ${runs} | std/board | README.md`, 'the list put right when the keyboard has left it');

  // ---- pinned: from a line of what was lately opened, and from a project's row
  await more(line('recent', 'std/board'), 'More for std/board');
  await choose('Pin to the top');
  await away();
  await listed(pinned, 'std/board', 'a thing pinned from its line');
  // It is not listed twice, and the ninth thing has room again.
  await listed(recent, `std/cards | Shop board | Depot | Shed | Shop | ${runs} | README.md | Shop repo`, 'what is pinned gone from what was lately opened');
  await more(tree('depot'), 'More for Depot');
  await choose('Pin to the top');
  await away();
  await listed(pinned, 'std/board | Depot', 'a project pinned from its row, after what was pinned before it');
  await listed(recent, `std/cards | Shop board | Shed | Shop | ${runs} | README.md (Depot) | Shop repo | README.md (Shop)`, 'the project gone from what was lately opened');
  // Its menu then offers to take the pin away, which it does.
  await more(tree('shop'), 'More for Shop');
  await choose('Pin to the top');
  await away();
  await listed(pinned, 'std/board | Depot | Shop', 'a third thing pinned');
  await tree('shop').click({ button: 'right' });
  await page.getByRole('menu').waitFor();
  const first = await page.getByRole('menu').getByRole('menuitem').first().innerText();
  expect(`a pinned project's menu to offer to unpin it, got "${first}"`, first === 'Unpin');
  await choose('Unpin');
  await away();
  await listed(pinned, 'std/board | Depot', 'a project unpinned from its row');
  await listed(recent, `std/cards | Shop board | Shed | Shop | ${runs} | README.md (Depot) | Shop repo | README.md (Shop)`, 'the project back among what was lately opened, where it was');
  // A pinned line shows its tab as any line does.
  await line('pinned', 'std/board').locator('.nav-item').click();
  await listed(showingTab, 'std/board', 'a pinned line pressed to show its tab');
  await away();
  await frame('with things pinned', 'Pinned | Recents | Projects | Factory | Blueprints');
  await page.screenshot({ path: `${out}/lately-02-pinned-and-lately-opened.png` });
  // Unpinned from its own line, it is back among what was lately opened, where it belongs: it was shown a moment ago.
  await line('pinned', 'std/board').click({ button: 'right' });
  await choose('Unpin');
  await away();
  await listed(pinned, 'Depot', 'a thing unpinned from its line');
  await listed(recent, `std/board | std/cards | Shop board | Shed | Shop | ${runs} | README.md | Shop repo`, 'what was unpinned back among what was lately opened');
  await more(line('recent', 'Shop board'), 'More for Shop board');
  await choose('Pin to the top');
  await away();
  const kept = { pinned: 'Depot | Shop board', recent: `std/board | std/cards | Shed | Shop | ${runs} | README.md (Depot) | Shop repo | README.md (Shop)` };
  await listed(pinned, kept.pinned, 'two things pinned');
  await listed(recent, kept.recent, 'what was lately opened beside them');
  // One removed from Recents by its line's menu, which a pinned line's does not offer: gone at once, and the list still holds
  // still while the panel is pointed at. Opened again, it is the latest, as anything opened is.
  const offeredBy = async (row, label) => {
    await more(row, label);
    const offered = (await page.getByRole('menu').getByRole('menuitem').allInnerTexts()).map((one) => one.trim()).join(' | ');
    // The menu hears Escape from a moment after it is drawn: one pressed sooner is heard by nothing, and is pressed again.
    for (let tries = 0; (await page.getByRole('menu').count()) > 0 && tries < 40; tries++) {
      await page.keyboard.press('Escape');
      await page.waitForTimeout(50);
    }
    await page.getByRole('menu').waitFor({ state: 'detached' });
    return offered;
  };
  const offeredRecent = await offeredBy(line('recent', 'std/board'), 'More for std/board');
  expect(`a line of Recents to offer to pin it and to remove it from them, got "${offeredRecent}"`, offeredRecent === 'Pin to the top | Remove from Recents');
  const offeredPinned = await offeredBy(line('pinned', 'Depot'), 'More for Depot');
  expect(`a pinned line to offer only to unpin it, got "${offeredPinned}"`, offeredPinned === 'Unpin');
  await more(line('recent', 'std/board'), 'More for std/board');
  await choose('Remove from Recents');
  const without = kept.recent.replace('std/board | ', '');
  await line('recent', 'Shed').hover();
  await listed(recent, without, 'one removed from Recents gone at once, with the pointer on the panel');
  expect(`nothing to be pinned by removing, got "${await pinned()}"`, (await pinned()) === kept.pinned);
  expect('what this browser keeps to have it no longer', !(await keptText()).includes('"name":"std/board"'));
  await shipped('std/board').click();
  await listed(showingTab, 'std/board', 'the one removed opened again');
  expect(`nothing to come into the list while the panel is pointed at, got "${await recent()}"`, (await recent()) === without);
  await away();
  await listed(recent, kept.recent, 'the one removed back as the latest once it was opened again');

  // ---- kept when the page is read again
  await listed(keptThere, '["/project/depot","/board/shop"]', 'what is pinned to be kept with the account');
  expect(`what was lately opened to be kept in this browser under who is signed in, got "${(await keptText()).slice(0, 80)}"`, (await keptText()).includes('"recent":[{"kind":"blueprint","name":"std/board"}'));
  await page.reload();
  await page.locator('.rail').waitFor();
  await side('Resources');
  await listed(pinned, kept.pinned, 'what was pinned, when the page is read again');
  await listed(recent, kept.recent, 'what was lately opened, when the page is read again');

  // ---- the panel of one workspace: both lists are of it, with what is the factory's own in each
  const foot = resources.locator('.space-foot .space-pick');
  const spaceMenu = page.getByRole('menu', { name: 'Workspace', exact: true });
  const toSpace = async (name) => {
    await foot.click();
    await spaceMenu.waitFor();
    await spaceMenu.getByRole('menuitemradio', { name: new RegExp(`^${name}(:|$)`) }).click();
    await spaceMenu.waitFor({ state: 'detached' });
    await away();
    await listed(() => resources.locator('.space-foot .space-said strong').innerText(), name, `the panel to be of ${name}`);
  };
  // The factory has its owner's workspace from the first, with every project in it: so far both lists were of that.
  await foot.waitFor();
  const USUAL = 'Ada\'s workspace';
  await cli('workspace', 'create', 'Front');
  await cli('workspace', 'put', 'front', 'shop');
  await cli('workspace', 'create', 'Back');
  await cli('workspace', 'put', 'back', 'depot');
  expect(`the panel to be of the workspace it was of, got "${await resources.locator('.space-foot .space-said strong').innerText()}"`, (await resources.locator('.space-foot .space-said strong').innerText()) === USUAL);
  // What a menu offers is what there was when it was opened: the workspace is waited for before it is chosen there.
  await listed(
    async () => {
      await foot.click();
      await spaceMenu.waitFor();
      // Opened, it has the keyboard, which is what says it is listening: a key pressed sooner is heard by nothing.
      await page.waitForFunction(() => document.activeElement?.closest('.row-menu') !== null && document.activeElement?.closest('.row-menu') !== undefined);
      const offered = await spaceMenu.getByRole('menuitemradio').count();
      await page.keyboard.press('Escape');
      await spaceMenu.waitFor({ state: 'detached' });
      return String(offered);
    },
    '3',
    'the page to have read the workspaces made from a terminal',
  );
  await toSpace('Front');
  // Of Front: Shop's and the factory's own; nothing of Depot or of Shed, pinned or not. One file reads like no other now.
  await listed(pinned, 'Shop board', 'what is pinned, of the workspace showing');
  await listed(recent, `std/board | std/cards | Shop | ${runs} | Shop repo | README.md`, 'what was lately opened, of the workspace showing');
  await frame('with a workspace showing', 'Pinned | Recents | Projects | Factory | Blueprints');
  await page.screenshot({ path: `${out}/lately-03-of-one-workspace.png` });
  // With the panel dragged as narrow as it goes, by its edge.
  const edge = page.locator('.side-edge');
  const grip = await edge.boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + 300);
  await page.mouse.down();
  await page.mouse.move(grip.x - 60, grip.y + 300, { steps: 6 });
  await page.mouse.move(60, grip.y + 300, { steps: 6 });
  await page.mouse.up();
  await listed(async () => String(Math.round((await resources.boundingBox()).width)), '180', 'the side panel dragged as narrow as it goes');
  await away();
  await frame('with the panel dragged narrow', 'Pinned | Recents | Projects | Factory | Blueprints');
  await page.screenshot({ path: `${out}/lately-04-the-panel-narrow.png` });
  await edge.dblclick();
  await listed(async () => String(Math.round((await resources.boundingBox()).width)), '252', 'the side panel as wide as it usually is again');
  await away();
  // Another workspace chosen: the lists follow.
  await toSpace('Back');
  await listed(pinned, 'Depot', 'what is pinned, of the other workspace');
  await listed(recent, 'std/board | std/cards | README.md', 'what was lately opened, of the other workspace');
  // And the one the factory had from the first, where the project nobody put anywhere else still is.
  await toSpace(USUAL);
  await listed(pinned, '(not drawn)', 'nothing pinned of the usual workspace now, and no heading for it');
  await listed(recent, 'std/board | std/cards | Shed', 'what was lately opened, of the usual workspace');

  // ---- a project forgotten from a terminal: its lines are gone, and are taken out of what was kept
  await toSpace('Back');
  expect(`the project to be in what is kept before it is forgotten`, (await keptText()).includes('"id":"depot"') && (await keptText()).includes('"project":"depot"'));
  await cli('project', 'forget', 'depot');
  await listed(pinned, '(not drawn)', 'a forgotten project\'s pin gone');
  await listed(recent, 'std/board | std/cards', 'a forgotten project\'s lines gone');
  await listed(async () => String((await keptText()).includes('depot')), 'false', 'nothing of a forgotten project to be kept in this browser');
  expect(`its pin to be kept with the account, passed over where it is shown, for a project given back`, (await keptThere()).includes('/project/depot'));
  expect(`what is not gone to be kept still, got "${(await keptText()).slice(0, 60)}"`, (await keptText()).includes('"project":"shop"') && (await keptText()).includes('"id":"shed"'));
  await toSpace('Front');
  await listed(pinned, 'Shop board', 'the first workspace\'s pin as it was');
  await listed(recent, `std/board | std/cards | Shop | ${runs} | Shop repo | README.md`, 'the first workspace\'s as they were');

  // ---- at a phone's width
  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload();
  await page.getByRole('button', { name: 'Expand the side panel' }).click();
  await sideSettled(page);
  await page.locator('.sidebar').waitFor();
  await side('Resources');
  await listed(pinned, 'Shop board', 'what is pinned at a phone\'s width');
  await listed(recent, `std/board | std/cards | Shop | ${runs} | Shop repo | README.md`, 'what was lately opened at a phone\'s width');
  await frame('at a phone\'s width', 'Pinned | Recents | Projects | Factory | Blueprints');
  await page.screenshot({ path: `${out}/lately-05-at-390.png` });
  // A line pressed there shows its tab, with the panel out of its way.
  await line('pinned', 'Shop board').locator('.nav-item').click();
  await page.locator('.sidebar').waitFor({ state: 'detached' });
  await page.locator('.board').waitFor();
  await narrow();

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, ...Object.values(folders)]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
