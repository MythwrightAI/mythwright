// The box under the head of each side panel that lists things: Resources,
// Channels, Topics and the inbox.
//
//   DAEMON=../target/debug/factory node scripts/sought.mjs [out-dir]
//
// Three projects of folders that were said, Shop with a board and files a
// few folders down, Depot with one file, and Bulk with more files that say
// one word than a search draws at once; channels open, private and a
// project's; and two lines in the inbox, said by somebody else. In each
// panel the box stands between the head and what is listed, and what is
// typed there narrows everything the panel lists to what says every word
// of it, nothing more being asked of the daemon for it: in Resources a
// file a few folders down shown with its project, its Repo and its
// folders held open though Shop was folded, `shop board` finding Shop's
// board and not Depot's, `measurements` finding the Factory's line, `nell`
// Nell under its Members line held open, `fleet` the
// blueprint that ships, what was lately opened found by what it is
// called, what nothing says said in one line, and a word two hundred and
// fifty files say drawing the first two hundred with a line saying how
// many more, which draws them when it is pressed; in Channels a channel
// by what it is called, a private one by its title, a project's channel
// under its project held open though it was folded, a pinned one under
// Pinned; in Topics a topic by its title, and a page of a wiki by its title and the wiki it is under, held open; in the inbox a line
// by who said it, where and its words. A held
// arrow folds nothing, and Escape empties the box, after which every fold
// is as it was. What was typed is there when the panel comes back after
// another was shown, is each panel's own, and is gone when the page is
// read again. The box is each panel's one box: searching everything is
// reached from it, by the keys it says at its right while it is empty
// (pressed, they open that search, empty), by the line it offers under it
// while something is typed, and by Enter and Ctrl and K in it, each of
// which opens that search with what was typed already in it. The box
// moves nothing as it is typed in or as the keys come and go, and nothing
// of any panel lies over anything else, leaves the panel or scrolls
// sideways, at three widths, with the panel dragged as narrow as it goes,
// and at a phone's width, touched, at 390 and 412 wide, where each
// heading's button (the `+` of Projects) is inside its heading and over no
// row, there are no keys to say and the line is still offered. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'sought-data');
const home = join(out, 'sought-home');
const hers = join(out, 'sought-home-nell');
const folders = { Shop: join(out, 'sought-shop'), Depot: join(out, 'sought-depot'), Bulk: join(out, 'sought-bulk') };
// More files that say one word than a search draws at once under one project.
const BULK = 250;

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers, ...Object.values(folders)]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
await mkdir(join(folders.Shop, 'src', 'deep'), { recursive: true });
await writeFile(join(folders.Shop, 'src', 'deep', 'pricing.md'), '# Pricing\n');
await writeFile(join(folders.Shop, 'README.md'), '# Shop\n');
await mkdir(folders.Depot, { recursive: true });
await writeFile(join(folders.Depot, 'notes.md'), '# Notes\n');
await mkdir(join(folders.Bulk, 'notes'), { recursive: true });
for (let n = 0; n < BULK; n += 1) await writeFile(join(folders.Bulk, 'notes', `item-${String(n).padStart(3, '0')}.md`), `# Item ${n}\n`);
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config') });
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  for (const name of ['Shop', 'Depot', 'Bulk']) await cli('project', 'create', name, '--bare', '--path', folders[name]);
  await cli('apply', 'std/board', '--project', 'shop');
  for (const name of ['general', 'ops']) await cli('channel', 'create', name);
  await cli('channel', 'create', 'shop-talk', '--project', 'shop');
  await cli('channel', 'create', 'Pay rises', '--kept');
  await nell('post', 'general', '@ada', 'the', 'quince', 'is', 'in');
  await nell('post', 'ops', '@ada', 'the', 'sloe', 'is', 'ripe');
  // Two topics, for the Topics panel's box.
  await cli('topic', 'make', 'The quince harvest');
  await cli('topic', 'make', 'Where the sloes grow');
  // A page of a wiki, found by its title and the wiki it is under.
  await cli('topic', 'make', 'Picking', '--under', 'The quince harvest');

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
  const signedIn = async (viewport, more = {}) => {
    const page = await (await browser.newContext({ viewport, colorScheme: scheme, ...more })).newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    // On a phone the rail is there and comes with the side panel.
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const page = await signedIn({ width: 1350, height: 860 });
  // What the daemon is asked once the panels have read what they list: a box asks nothing more.
  let asked = [];
  page.on('request', (request) => request.url().includes('/api/') && !request.url().includes('/api/events') && asked.push(request.url().replace(url, '')));
  const navOf = (name) => page.getByRole('navigation', { name });
  const side = async (name) => {
    const button = name === 'Inbox' ? page.locator('.rail .rail-inbox') : page.locator('.rail').getByRole('button', { name, exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
    await navOf(name).waitFor();
  };
  const boxOf = (name) => navOf(name).locator(':scope > .panel-seek input');
  const until = async (read, wanted, what) => {
    const end = Date.now() + 8000;
    let got = await read();
    while (got !== wanted && Date.now() < end) {
      await page.waitForTimeout(60);
      got = await read();
    }
    expect(`${what}: "${wanted}", got "${got}"`, got === wanted);
  };
  // What a panel lists, line by line, by the words of each.
  const linesOf = (name) =>
    navOf(name).evaluate((nav) =>
      [...nav.querySelectorAll('.nav h3, .nav .nav-row .nav-item .truncate, .nav button.nav-item:not(.nav-row *) .truncate, .inbox-line .inbox-words, .panel-none')]
        .filter((one) => one.getClientRects().length > 0)
        .map((one) => (one.matches('h3') ? one.firstChild.textContent : one.textContent).trim())
        .join(' | '),
    );
  // A panel by its boxes: the box between its head and its list, every line inside the panel with none over another, and nothing scrolling sideways.
  const laid = async (name, where) => {
    const got = await navOf(name).evaluate((nav) => {
      const box = (one) => {
        const { left, right, top, bottom } = one.getBoundingClientRect();
        return { left, right, top, bottom };
      };
      const seen = (one) => one.getClientRects().length > 0;
      const list = nav.querySelector(':scope > .nav, :scope > .inbox-lines');
      const lines = [...nav.querySelectorAll('.nav-row, button.nav-item, .inbox-line, .panel-none, h3')].filter((one) => seen(one) && !(one.matches('button.nav-item') && one.closest('.nav-row')));
      return {
        side: box(nav),
        head: box(nav.querySelector(':scope > .brand')),
        seek: box(nav.querySelector(':scope > .panel-seek input')),
        // Where what is typed may run to: the box less the room it keeps at its right.
        typedTo: nav.querySelector(':scope > .panel-seek input').getBoundingClientRect().right - parseFloat(getComputedStyle(nav.querySelector(':scope > .panel-seek input')).paddingRight),
        keys: nav.querySelector(':scope > .panel-seek .panel-seek-keys') && box(nav.querySelector(':scope > .panel-seek .panel-seek-keys')),
        offered: nav.querySelector(':scope > .panel-seek .panel-everywhere') && box(nav.querySelector(':scope > .panel-seek .panel-everywhere')),
        // The words typed, as the line shows them: every piece of them, however they wrap, and nothing of the line cut.
        words: [...(nav.querySelector(':scope > .panel-seek .panel-everywhere-words')?.getClientRects() ?? [])].map(({ left, right, top, bottom }) => ({ left, right, top, bottom })),
        cut: (() => {
          const says = nav.querySelector(':scope > .panel-seek .panel-everywhere-says');
          return says ? Math.max(says.scrollWidth - says.clientWidth, says.scrollHeight - says.clientHeight) : 0;
        })(),
        boxes: nav.querySelectorAll('input').length,
        list: box(list),
        lines: lines.map((one) => ({ says: one.textContent.trim().slice(0, 30), ...box(one) })),
        sideways: Math.max(nav.scrollWidth - nav.clientWidth, list.scrollWidth - list.clientWidth),
        page: document.documentElement.scrollWidth - window.innerWidth,
      };
    });
    const told = JSON.stringify({ ...got, lines: got.lines.length });
    const inside = (one) => one.left >= got.side.left - 0.5 && one.right <= got.side.right + 0.5;
    expect(`the box of ${name} to be under its head and over its list, inside the panel and wide enough to type in ${where}: ${told}`, inside(got.seek) && got.seek.top >= got.head.bottom - 0.5 && got.seek.bottom <= got.list.top + 0.5 && got.seek.right - got.seek.left > 80);
    expect(`one box in ${name} ${where}, got ${got.boxes}`, got.boxes === 1);
    // The keys lie inside the box, in the room it keeps for them, and over nothing typed.
    const keys = got.keys;
    expect(`the keys that search everything inside the box of ${name}, past where what is typed runs to, ${where}: ${told}`, !keys || (keys.left >= got.typedTo - 0.5 && keys.right <= got.seek.right + 0.5 && keys.top >= got.seek.top - 0.5 && keys.bottom <= got.seek.bottom + 0.5 && got.typedTo - got.seek.left > 60));
    // The line that searches everything for what is typed, between the box and the list, inside the panel.
    const offered = got.offered;
    expect(`the line that searches everything under the box of ${name} and over its list ${where}: ${told}`, !offered || (inside(offered) && offered.top >= got.seek.bottom - 0.5 && offered.bottom <= got.list.top + 0.5));
    const within = (one) => one.left >= offered.left - 0.5 && one.right <= offered.right + 0.5 && one.top >= offered.top - 0.5 && one.bottom <= offered.bottom + 0.5;
    expect(`the words typed wholly inside the line that searches everything in ${name}, and nothing of it cut, ${where}: ${JSON.stringify({ offered, words: got.words, cut: got.cut })}`, !offered || (got.words.length > 0 && got.words.every(within) && got.cut <= 0.5));
    const out = got.lines.find((one) => !inside(one));
    expect(`every line of ${name} inside the panel ${where}, and this is not: ${JSON.stringify(out)}`, !out);
    const byTop = [...got.lines].sort((a, b) => a.top - b.top);
    const over = byTop.find((one, at) => at > 0 && one.top < byTop[at - 1].bottom - 0.5 && one.left < byTop[at - 1].right && byTop[at - 1].left < one.right);
    expect(`no line of ${name} over another ${where}, and this is: ${JSON.stringify(over)}`, !over);
    expect(`nothing of ${name} or the page to scroll sideways ${where}: ${got.sideways}, ${got.page}`, got.sideways <= 0 && got.page <= 0);
  };
  // Types in a panel's box, and holds that the box did not move as it was typed in.
  const type = async (name, words) => {
    const before = await boxOf(name).boundingBox();
    await boxOf(name).fill(words);
    const after = await boxOf(name).boundingBox();
    expect(`the box of ${name} not to move as "${words}" is typed, ${JSON.stringify(before)} and ${JSON.stringify(after)}`, Math.abs(before.y - after.y) < 0.5 && Math.abs(before.x - after.x) < 0.5 && Math.abs(before.width - after.width) < 0.5);
  };

  // ---- Resources ------------------------------------------------------------------
  await side('Resources');
  // Everything of it starts folded: unfolded first as a person would (`folded.mjs`), and Shop then folded away.
  await unfoldAll(page);
  const resources = navOf('Resources');
  await resources.locator('[data-project="depot"]').waitFor();
  await resources.getByText('std', { exact: true }).waitFor();
  expect('no box of its own over the blueprints', (await resources.locator('.nav input').count()) === 0);
  expect('the box to say what it narrows', (await boxOf('Resources').getAttribute('placeholder')) === 'Search this panel');
  expect('the keys that search everything said at the right of the empty box of Resources', (await resources.locator(':scope > .panel-seek .panel-seek-keys').innerText()).trim() === 'Ctrl K');
  // Something lately opened, and Shop folded away, as somebody leaves them.
  // Its page reads what it shows as it opens; that is in before anything is typed.
  const read = Promise.all(['/api/collections?project=depot', '/api/instances?project=depot', '/api/runs?project=depot'].map((part) => page.waitForResponse((response) => response.url().includes(part))));
  await resources.locator('[data-project="depot"] > .nav-row .nav-item').click();
  await read;
  await page.getByRole('button', { name: 'Fold Shop', exact: true }).click();
  // What was lately opened holds still while the panel is pointed at: it is put right once the pointer has left.
  await page.mouse.move(900, 700);
  await resources.getByText('Recents', { exact: true }).waitFor();
  asked = [];
  const whole = await linesOf('Resources');
  await type('Resources', 'pricing');
  await until(() => linesOf('Resources'), 'Projects | Shop | Repo | src | deep | pricing.md', 'a file a few folders down shown with what it is under, and nothing else');
  const arrow = page.getByRole('button', { name: 'Fold Shop', exact: true });
  expect('Shop to be held open, its arrow saying so', (await arrow.getAttribute('aria-disabled')) === 'true');
  await arrow.click({ force: true });
  expect('a held arrow to fold nothing', (await linesOf('Resources')).includes('pricing.md'));
  await laid('Resources', 'with a file looked for');
  await page.screenshot({ path: `${out}/sought-1-resources.png` });
  await type('Resources', 'shop board');
  // Its board, and the blueprints it uses that are the board or say they are of one.
  await until(() => linesOf('Resources'), 'Projects | Shop | Board | Blueprints | std/board | std/cards', "`shop board` to find Shop's board and nothing of Depot");
  await type('Resources', 'measurements');
  await until(async () => (await linesOf('Resources')).split(' | ').slice(0, 2).join(' | '), 'Factory | Measurements', 'the Factory\'s Measurements');
  // Everybody here is under the Members line of the Factory, found by name with the line held open.
  await type('Resources', 'nell');
  await until(() => linesOf('Resources'), 'Factory | Members | Nell', 'a person found under the Members line of the Factory, held open though it was folded');
  await type('Resources', 'fleet');
  await until(async () => (await linesOf('Resources')).split(' | ').slice(0, 2).join(' | '), 'Blueprints | std', 'the blueprints that ship, and no line of the Factory\'s');
  await resources.getByText('std/fleet', { exact: true }).waitFor();
  await type('Resources', 'depot');
  // The project says it itself, so it is shown as it was left: unfolded, with its Repo folded.
  await until(() => linesOf('Resources'), 'Recents | Depot | Projects | Depot | Repo', 'what was lately opened found by what it is called, beside the project as it was left');
  await type('Resources', 'nothing says this');
  await until(() => linesOf('Resources'), 'Nothing listed here says “nothing says this”. A project\'s files are looked in as far as the page has read them.', 'one line where nothing is found');
  expect('Resources to offer what was typed to be searched for everywhere', (await resources.locator(':scope > .panel-seek .panel-everywhere-says').innerText()).trim() === 'Search everything for “nothing says this”');
  await laid('Resources', 'with nothing found');
  await page.screenshot({ path: `${out}/sought-2-nothing.png` });
  // A search that finds a great many files under one project draws the first two hundred, in the order they are
  // drawn, and says how many more there are; pressed, that line draws the next two hundred.
  const items = () => resources.locator('[data-project="bulk"] .tree .nav-item .truncate', { hasText: /^item-\d+\.md$/ });
  await type('Resources', 'item');
  await until(() => items().count(), 200, 'the first two hundred files found drawn');
  const more = resources.locator('[data-project="bulk"] .nav-more');
  expect(`the line after them to say how many more, got "${await more.innerText()}"`, (await more.innerText()).trim().endsWith(`${BULK - 200} more files found`));
  expect('the first drawn to be the first by name, and the last the two hundredth', (await items().first().innerText()) === 'item-000.md' && (await items().last().innerText()) === 'item-199.md');
  await laid('Resources', 'with two hundred files found');
  await page.screenshot({ path: `${out}/sought-2b-many.png` });
  await more.click();
  await until(() => items().count(), BULK, 'every file found drawn once the line was pressed');
  expect('no line saying more once every one is drawn', (await more.count()) === 0);
  await type('Resources', 'item 24');
  await until(() => items().count(), 13, 'a narrower search drawing every one it finds');
  expect(`nothing to have been asked of the daemon for what was typed, got ${asked.join(', ')}`, asked.length === 0);
  // Escape empties it, and every fold is as it was: Shop folded.
  await boxOf('Resources').press('Escape');
  await until(() => linesOf('Resources'), whole, 'the panel as it was once Escape has emptied the box');
  await page.getByRole('button', { name: 'Unfold Shop', exact: true }).waitFor();
  expect('the box to be empty', (await boxOf('Resources').inputValue()) === '');

  // ---- Channels -------------------------------------------------------------------
  await type('Resources', 'pricing');
  await side('Channels');
  const channels = navOf('Channels');
  expect('the box of Channels to be its own, empty', (await boxOf('Channels').inputValue()) === '');
  const row = (name) => channels.locator('.grouped-channel', { has: page.getByRole('button', { name: `# ${name}`, exact: true }) });
  await row('general').waitFor();
  await row('general').click({ button: 'right' });
  const menu = page.getByRole('menu');
  await menu.getByText('Pin to the top', { exact: true }).click();
  await menu.waitFor({ state: 'detached' });
  // Pinned and Shop are folded, as every branch of the panel starts (`folding.mjs`).
  await page.getByRole('button', { name: 'Unfold Pinned', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Unfold Shop', exact: true }).waitFor();
  await page.mouse.move(900, 700);
  const before = await linesOf('Channels');
  // ---- searching everything, from the panel's one box -----------------------------
  expect('no button that searches everything beside the box', (await page.locator('.search-button').count()) === 0 && (await channels.getByRole('button', { name: /Search everything/ }).count()) === 1);
  const keysOf = (name) => navOf(name).locator(':scope > .panel-seek .panel-seek-keys');
  const offeredOf = (name) => navOf(name).locator(':scope > .panel-seek .panel-everywhere');
  const palette = page.locator('.palette-input');
  // Resources' box holds words just now, and said the keys before anything was typed in it.
  for (const name of ['Channels', 'Topics', 'Inbox']) {
    await side(name);
    expect(`the keys that search everything said at the right of the empty box of ${name}`, (await keysOf(name).innerText()).trim() === 'Ctrl K' && (await offeredOf(name).count()) === 0);
  }
  await side('Channels');
  await laid('Channels', 'with nothing typed');
  await page.screenshot({ path: `${out}/sought-3a-channels-empty.png` });
  // Pressed, the keys open the search of everything, with nothing in it.
  await keysOf('Channels').click();
  await palette.waitFor();
  expect('the search of everything opened by the keys to be empty', (await palette.inputValue()) === '');
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await palette.waitFor({ state: 'detached' });
  // Typed, the keys go and the line that searches everything for it is offered, the box not moving for either.
  asked = [];
  await type('Channels', 'quince');
  expect('no keys said while something is typed', (await keysOf('Channels').count()) === 0);
  await until(async () => (await offeredOf('Channels').locator('.panel-everywhere-says').innerText()).trim(), 'Search everything for “quince”', 'the line that searches everything for what was typed');
  await until(() => linesOf('Channels'), 'Nothing listed here says “quince”.', 'no channel says what was said in one');
  await laid('Channels', 'with something typed');
  await page.screenshot({ path: `${out}/sought-3b-channels-typed.png` });
  expect(`nothing to have been asked of the daemon by the box, got ${asked.join(', ')}`, asked.length === 0);
  await offeredOf('Channels').click();
  await palette.waitFor();
  expect(`the search of everything to open with what was typed in it, got "${await palette.inputValue()}"`, (await palette.inputValue()) === 'quince');
  // What it finds is what was said in a channel, which the panel's box could not find.
  await page.locator('.hit', { hasText: 'quince' }).first().waitFor();
  await page.screenshot({ path: `${out}/sought-3c-everything.png` });
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await palette.waitFor({ state: 'detached' });
  expect('what was typed in the box to be there still', (await boxOf('Channels').inputValue()) === 'quince');
  // Enter in the box, and Ctrl and K in it, hand what is typed on the same way; Ctrl and K elsewhere opens it empty.
  for (const keys of ['Enter', 'Control+K']) {
    await boxOf('Channels').press(keys);
    await palette.waitFor();
    expect(`${keys} in the box to open the search of everything with what was typed, got "${await palette.inputValue()}"`, (await palette.inputValue()) === 'quince');
    await page.waitForTimeout(200);
    await page.keyboard.press('Escape');
    await palette.waitFor({ state: 'detached' });
  }
  await boxOf('Channels').evaluate((box) => box.blur());
  await page.keyboard.press('Control+K');
  await palette.waitFor();
  expect('Ctrl and K anywhere else to open the search of everything empty', (await palette.inputValue()) === '');
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await palette.waitFor({ state: 'detached' });
  await boxOf('Channels').press('Escape');
  await page.mouse.move(900, 700);
  asked = [];
  await type('Channels', 'ops');
  await until(() => linesOf('Channels'), 'ops', 'a channel found by what it is called');
  await type('Channels', 'pay');
  await until(() => linesOf('Channels'), 'Pay rises', 'a private channel found by its title');
  await type('Channels', 'talk');
  await until(() => linesOf('Channels'), 'Shop | shop-talk', "a project's channel under its project, held open though it was folded");
  await type('Channels', 'general');
  await until(() => linesOf('Channels'), 'Pinned | general', 'a pinned channel under Pinned, held open though it was folded');
  await laid('Channels', 'with a pinned channel looked for');
  await page.screenshot({ path: `${out}/sought-3-channels.png` });
  await type('Channels', 'nowhere at all');
  await until(() => linesOf('Channels'), 'Nothing listed here says “nowhere at all”.', 'one line where no channel is found');
  await boxOf('Channels').press('Escape');
  await until(() => linesOf('Channels'), before, 'the channels as they were, Pinned and Shop folded');
  await type('Channels', 'sloe');

  // ---- Inbox ----------------------------------------------------------------------
  await side('Inbox');
  const inbox = navOf('Inbox');
  await until(() => inbox.locator('.inbox-line').count(), 2, 'both lines in the inbox');
  await type('Inbox', 'sloe');
  await until(() => linesOf('Inbox'), '@ada the sloe is ripe', 'a line found by its words');
  await type('Inbox', 'nell #general');
  await until(() => linesOf('Inbox'), '@ada the quince is in', 'a line found by who said it and where');
  await laid('Inbox', 'with a line looked for');
  await page.screenshot({ path: `${out}/sought-4-inbox.png` });
  await type('Inbox', 'damson');
  await until(() => linesOf('Inbox'), 'Nothing listed here says “damson”.', 'one line where no line is found');
  await until(async () => (await offeredOf('Inbox').locator('.panel-everywhere-says').innerText()).trim(), 'Search everything for “damson”', 'the inbox offering the same line');
  await offeredOf('Inbox').click();
  await palette.waitFor();
  expect('the search of everything to open with what was typed in the inbox', (await palette.inputValue()) === 'damson');
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await palette.waitFor({ state: 'detached' });
  // ---- the Topics panel: its box narrows its topics as the others' do
  await side('Topics');
  const topicsBefore = 'The quince harvest | Where the sloes grow';
  await until(() => linesOf('Topics'), topicsBefore, 'the wikis, in the order they were made, the page under one folded');
  asked = [];
  await type('Topics', 'sloes');
  await until(() => linesOf('Topics'), 'Where the sloes grow', 'a topic found by its title');
  await laid('Topics', 'with a topic looked for');
  await type('Topics', 'harvest picking');
  await until(() => linesOf('Topics'), 'The quince harvest | Picking', 'a page found by its title and the wiki it is under, which is held open for it');
  await laid('Topics', 'with a page under a wiki looked for');
  await type('Topics', 'damson');
  await until(() => linesOf('Topics'), 'Nothing listed here says “damson”.', 'one line where no topic is found');
  await until(async () => (await offeredOf('Topics').locator('.panel-everywhere-says').innerText()).trim(), 'Search everything for “damson”', 'the Topics panel offering the same line');
  await boxOf('Topics').press('Escape');
  await until(() => linesOf('Topics'), topicsBefore, 'the topics as they were once the box is emptied');
  expect(`nothing to have been asked of the daemon by the Topics panel's box, got ${asked.join(', ')}`, asked.length === 0);
  await type('Topics', 'quince');
  // Each panel's own is there when it comes back.
  await side('Resources');
  expect('what was typed in Resources to be there when it comes back', (await boxOf('Resources').inputValue()) === 'pricing');
  await side('Channels');
  expect('what was typed in Channels to be there when it comes back', (await boxOf('Channels').inputValue()) === 'sloe');
  await side('Topics');
  expect('what was typed in Topics to be there when it comes back', (await boxOf('Topics').inputValue()) === 'quince');
  await until(() => linesOf('Topics'), 'The quince harvest', 'and the topic it found');
  await side('Channels');
  await until(() => linesOf('Channels'), 'Nothing listed here says “sloe”.', 'no channel says what was said in one');
  // And nothing of it is kept when the page is read again.
  await page.reload();
  await side('Resources');
  await resources.locator('[data-project="depot"]').waitFor();
  expect('nothing typed to be kept over a reading of the page', (await boxOf('Resources').inputValue()) === '');
  await page.getByRole('button', { name: 'Unfold Shop', exact: true }).waitFor();

  // ---- by the boxes, at three widths and with the panel dragged narrow ------------
  for (const width of [1350, 1024, 800]) {
    await page.setViewportSize({ width, height: 800 });
    for (const [name, words] of [['Resources', 'pricing'], ['Channels', 'general'], ['Topics', 'quince'], ['Inbox', 'quince']]) {
      await side(name);
      await type(name, '');
      await page.mouse.move(width - 40, 700);
      await laid(name, `at ${width} with nothing typed`);
      await type(name, words);
      await page.mouse.move(width - 40, 700);
      await laid(name, `at ${width}`);
    }
    await page.screenshot({ path: `${out}/sought-5-at-${width}.png` });
  }
  await side('Resources');
  const edge = page.locator('.side-edge');
  const grip = await edge.boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + 300);
  await page.mouse.down();
  await page.mouse.move(60, grip.y + 300, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 180);
  for (const name of ['Resources', 'Channels', 'Topics', 'Inbox']) {
    await side(name);
    await page.mouse.move(700, 700);
    await laid(name, 'with the panel as narrow as it goes');
    const words = await boxOf(name).inputValue();
    await type(name, '');
    await page.mouse.move(700, 700);
    await laid(name, 'with the panel as narrow as it goes and nothing typed');
    // Words longer than the line has room for wrap onto another, and are still read whole.
    await type(name, 'quince damson marmalade');
    await page.mouse.move(700, 700);
    await laid(name, 'with the panel as narrow as it goes and more typed than one line holds');
    if (name === 'Channels') await page.screenshot({ path: `${out}/sought-6b-narrow-wrapped.png` });
    await type(name, words);
  }
  await page.screenshot({ path: `${out}/sought-6-narrow.png` });

  // ---- at a phone's width, touched -------------------------------------------------
  // Held in the hand: a finger, and no keys.
  // A heading's button is a finger's height there, and drawn back into the heading's room: drawn back by more than
  // the room under the heading, the `+` of Projects lay over the first project's row. So each panel is held by the
  // boxes as a whole, and each heading's buttons to their heading, with the box empty and with something found.
  const headed = (phone, name) =>
    phone.getByRole('navigation', { name }).evaluate((nav) =>
      [...nav.querySelectorAll('.nav h3')]
        .filter((head) => head.getClientRects().length > 0)
        .flatMap((head) => {
          const box = head.getBoundingClientRect();
          return [...head.querySelectorAll('button')].map((button) => {
            const at = button.getBoundingClientRect();
            return { head: head.firstChild?.textContent?.trim(), button: button.getAttribute('aria-label'), inside: at.top >= box.top - 0.5 && at.bottom <= box.bottom + 0.5, tall: at.height, at: [at.top, at.bottom], of: [box.top, box.bottom] };
          });
        }),
    );
  for (const width of [390, 412]) {
    const phone = await signedIn({ width, height: 844 }, { hasTouch: true, isMobile: true });
    await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
    await sideSettled(phone);
    await phone.locator('.sidebar').waitFor();
    for (const [name, words, wanted, before] of [
      ['Resources', 'pricing', 'pricing.md', '[data-project="bulk"]'],
      ['Channels', 'pay rises', 'Pay rises', '.grouped-channel'],
      ['Topics', 'sloes', 'Where the sloes grow', '.topic-line'],
      ['Inbox', 'sloe', '@ada the sloe is ripe', '.inbox-line'],
    ]) {
      const button = name === 'Inbox' ? phone.locator('.rail .rail-inbox') : phone.locator('.rail').getByRole('button', { name, exact: true });
      if ((await button.getAttribute('aria-pressed')) !== 'true') await button.tap();
      const nav = phone.getByRole('navigation', { name });
      // Everything the Resources panel lists starts folded (`folded.mjs`).
      if (name === 'Resources') await unfoldAll(phone);
      await nav.locator(before).first().waitFor();
      for (const [typed, where] of [['', 'with nothing typed'], [words, `with "${words}" typed`]]) {
        await nav.locator(':scope > .panel-seek input').fill(typed);
        if (typed) await nav.getByText(wanted, { exact: true }).waitFor();
        const crowded = await pageCrowded(phone, `nav[aria-label="${name}"]`);
        expect(`nothing of ${name} over anything else at ${width} wide, touched, ${where}, and: ${crowded.join('; ')}`, crowded.length === 0);
        const heads = await headed(phone, name);
        const out = heads.find((one) => !one.inside || one.tall < 36);
        expect(`each heading's button of ${name} inside its heading and a finger's height at ${width} wide, ${where}, and this is not: ${JSON.stringify(out)}`, !out);
        if (name === 'Resources') expect(`the Projects heading's + to be there at ${width} wide, ${where}: ${JSON.stringify(heads)}`, heads.some((one) => one.button === 'New project'));
      }
      // Held in the hand there are no keys to say, and the line that searches everything is offered under the box.
      const got = await nav.evaluate((one) => {
        const seek = one.querySelector(':scope > .panel-seek input').getBoundingClientRect();
        const line = one.querySelector(':scope > .panel-seek .panel-everywhere')?.getBoundingClientRect();
        const words = [...(one.querySelector(':scope > .panel-seek .panel-everywhere-words')?.getClientRects() ?? [])];
        const says = one.querySelector(':scope > .panel-seek .panel-everywhere-says');
        const side = one.getBoundingClientRect();
        return {
          seek: [seek.left, seek.right, seek.height, seek.bottom],
          line: line && [line.left, line.right, line.height, line.top],
          keys: one.querySelectorAll('.panel-seek-keys').length,
          whole: words.length > 0 && words.every((piece) => piece.left >= line.left - 0.5 && piece.right <= line.right + 0.5 && piece.top >= line.top - 0.5 && piece.bottom <= line.bottom + 0.5) && says.scrollWidth <= says.clientWidth && says.scrollHeight <= says.clientHeight,
          side: [side.left, side.right],
          page: document.documentElement.scrollWidth - window.innerWidth,
        };
      });
      expect(`the box of ${name} inside the panel at a phone's width and as tall as a finger needs: ${JSON.stringify(got)}`, got.seek[0] >= got.side[0] && got.seek[1] <= got.side[1] && got.seek[2] >= 36 && got.page <= 0);
      expect(`no keys said in the box of ${name} under a finger, and the line that searches everything offered under it, inside the panel and as tall as a finger needs: ${JSON.stringify(got)}`, got.keys === 0 && got.line && got.line[0] >= got.side[0] && got.line[1] <= got.side[1] && got.line[2] >= 36 && got.line[3] >= got.seek[3] - 0.5);
      expect(`the words typed in ${name} wholly inside that line at ${width} wide: ${JSON.stringify(got)}`, got.whole);
      await phone.screenshot({ path: `${out}/sought-7-${name.toLowerCase()}-at-${width}.png` });
    }
    await phone.context().close();
  }
  expect(`no error on the page, got ${problems.join('; ')}`, problems.length === 0);
  await browser.close();
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers, ...Object.values(folders)]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
