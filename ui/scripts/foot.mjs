// The foot of the side panel: Help, and the menu of the workspace beside it.
//
//   DAEMON=../target/debug/factory node scripts/foot.mjs [out-dir]
//
// A factory always has a workspace, the one made for its owner as they
// registered, so the foot says it from the first; and Help is at its
// right: a round button, with a dot on it while the newest release is
// one this browser has not been shown. Its
// menu opens upward, on the screen, moves nothing, stays as the pointer
// crosses to it, is gone through by the arrows, Home and End, left by
// Escape and put away by pressing the button again. With no address for
// support written into the product it holds no line that opens one: what
// somebody would say in a report is copied (the program and its version,
// the daemon's system, the browser) and said to be, and under What's new
// each release the notes tell of is a line, joined down their left, with
// all of them after. A release pressed opens the notes as a page in a
// tab, boxed off with nothing in it run, at that release; shown the list,
// the dot is gone and stays gone when the page is read again. With an
// address (said here in place of the daemon's own answer, since what the
// product says is built into it) there is a line that opens it in a tab
// of its own, and the page itself asks nothing of anywhere but its
// daemon, here or at any other step; nor is the part of the client that
// holds the icons ever asked for.
//
// Then another workspace is made from a terminal and chosen, and its menu
// is read from its head down: the workspace with how many projects it
// has, who is signed in, the workspaces, a new one, and set apart at its
// foot Settings and Log out. Settings opens Settings with the workspace's
// part in view (calling it something else and taking it away are there:
// spaces.mjs does both). Inviting is offered only to somebody who may
// invite as things stand, beside Settings, and brings who is here into
// view. Nothing of the foot lies over
// anything else or leaves the panel with the panel dragged as narrow as
// it goes and at a phone's width, where both menus are on the screen.
// Log out signs out. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'foot-data');
const home = join(out, 'foot-home');
// Where support is asked for, in this check: a name that is nobody's and leads nowhere.
const SUPPORT = 'https://example.invalid/help';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

// The releases as the notes tell of them: the headings of the one file they are written in.
const RELEASES = (await readFile(join(repo, 'docs/releases.md'), 'utf8'))
  .split('\n')
  .filter((line) => line.startsWith('## '))
  .map((line) => line.slice(3).trim());
// The parts of the client that hold the icons, by the name each is served under.
const assets = join(repo, 'ui/dist/assets');
const icons = [];
for (const name of await readdir(assets)) {
  if (name.endsWith('.js') && (await readFile(join(assets, name), 'utf8')).includes('lucide-static')) icons.push(name);
}

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  for (const name of ['Shop', 'Depot']) await cli('project', 'create', name, '--bare');
  await cli('channel', 'create', 'general');
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1350, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  await context.grantPermissions(['clipboard-read', 'clipboard-write'], { origin: url });
  // What the address for support leads to is answered here: nothing of this check goes to the network.
  await context.route('https://example.invalid/**', (route) => route.fulfill({ contentType: 'text/html', body: '<title>Support</title><p>Where support is asked for.</p>' }));
  const page = await context.newPage();
  page.setDefaultTimeout(20_000);
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  // Everything the page asks for, of anybody.
  const asked = [];
  page.on('request', (request) => asked.push(request.url()));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async () => {
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor();
  };
  await page.goto(url);
  await signIn();
  const side = async (name) => {
    const button = page.locator('.rail').getByRole('button', { name, exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
    await page.getByRole('navigation', { name }).waitFor();
    // Everything of the Resources panel starts folded: what it has read is unfolded as a person would (`folded.mjs`).
    if (name === 'Resources') await unfoldAll(page);
  };
  const listed = async (read, wanted, what) => {
    const until = Date.now() + 8000;
    let got = await read();
    while (got !== wanted && Date.now() < until) {
      await page.waitForTimeout(60);
      got = await read();
    }
    expect(`${what}: "${wanted}", got "${got}"`, got === wanted);
  };
  const tabs = async () => (await page.locator('.tabs .tab').allInnerTexts()).map((tab) => tab.trim()).join(' | ');
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const panel = page.getByRole('navigation', { name: 'Channels' });
  const help = (nav) => nav.locator('.space-foot').getByRole('button', { name: /^Help/ });
  const helpMenu = page.getByRole('menu', { name: 'Help', exact: true });
  const spaceMenu = page.getByRole('menu', { name: 'Workspace', exact: true });
  const focused = () => page.evaluate(() => `${document.activeElement?.getAttribute('role')}:${document.activeElement?.getAttribute('aria-label') ?? document.activeElement?.querySelector('.truncate')?.textContent}`);
  // Opened, a menu has the keyboard, which is what says it is listening: a key pressed sooner is heard by nothing.
  const opened = async (menu) => {
    await menu.waitFor();
    await page.waitForFunction(() => document.activeElement?.closest('.row-menu') !== null && document.activeElement?.closest('.row-menu') !== undefined);
  };
  const openHelp = async (nav) => {
    await help(nav).click();
    await opened(helpMenu);
  };
  const lines = async (menu) => (await menu.getByRole('menuitem').allInnerTexts()).map((line) => line.trim()).join(' | ');
  const says = async (menu) => (await menu.locator('.menu-said').allInnerTexts()).map((line) => line.replace(/\s+/g, ' ').trim()).join(' | ');

  // The foot of a panel, by the boxes themselves: at the panel's foot and on the screen, the list ending above it,
  // what it holds side by side and inside it, Help at its right, and nothing scrolling sideways.
  const strip = async (name, where, holds) => {
    const nav = page.getByRole('navigation', { name });
    await help(nav).waitFor();
    const got = await nav.evaluate((bar) => {
      const round = (figure) => Math.round(figure * 10) / 10;
      const box = (one) => {
        const at = one.getBoundingClientRect();
        return { left: round(at.left), right: round(at.right), top: round(at.top), bottom: round(at.bottom) };
      };
      const foot = bar.querySelector('.space-foot');
      const list = bar.querySelector('.nav');
      return {
        screen: { width: window.innerWidth, height: window.innerHeight },
        bar: box(bar),
        list: box(list),
        strip: box(foot),
        // What of it is drawn: what only pushes two things apart has no size, and is nothing to lie over.
        parts: [...foot.children].map(box).filter((at) => at.right - at.left > 0.5 && at.bottom - at.top > 0.5).sort((one, other) => one.left - other.left),
        help: box(foot.querySelector('.foot-help')),
        sideways: Math.max(bar.scrollWidth - bar.clientWidth, list.scrollWidth - list.clientWidth, foot.scrollWidth - foot.clientWidth, document.documentElement.scrollWidth - window.innerWidth),
      };
    });
    const told = JSON.stringify(got);
    const apart = got.parts.every((one, at) => at === 0 || got.parts[at - 1].right <= one.left + 0.5);
    const inside = got.parts.every((one) => one.left >= got.strip.left - 0.5 && one.right <= got.strip.right + 0.5 && one.top >= got.strip.top - 0.5 && one.bottom <= got.strip.bottom + 0.5);
    expect(`the foot of ${name} to hold ${holds} ${holds === 1 ? 'thing' : 'things'} ${where}: ${told}`, got.parts.length === holds);
    expect(`the foot to be at the foot of ${name}, on the screen, with the list ending above it ${where}: ${told}`, got.list.bottom <= got.strip.top + 0.5 && got.strip.bottom <= got.bar.bottom + 0.5 && got.strip.bottom <= got.screen.height + 0.5 && got.strip.bottom >= got.bar.bottom - 40);
    expect(`nothing of the foot of ${name} to lie over anything else of it, and all of it to be inside the panel ${where}: ${told}`, apart && inside && got.strip.left >= got.bar.left - 0.5 && got.strip.right <= got.bar.right + 0.5);
    expect(`Help to be at the right of the foot of ${name}, and large enough to press ${where}: ${told}`, got.help.right >= got.strip.right - 12 && got.help.right - got.help.left >= 24 && got.help.bottom - got.help.top >= 24);
    expect(`nothing of ${name} to scroll sideways ${where}: ${told}`, got.sideways <= 0);
    return got;
  };
  // A menu of the foot, opened: it lies above its button and on the screen, the list and the foot are where they
  // were, and it is there still when the pointer has crossed from the button to it.
  const overhead = async (name, button, menu, where, picture) => {
    const nav = page.getByRole('navigation', { name });
    const places = () =>
      nav.evaluate((bar) => {
        const at = (one) => {
          const box = one.getBoundingClientRect();
          return [box.left, box.top, box.width, box.height].map((figure) => Math.round(figure * 10) / 10).join(',');
        };
        return `${at(bar.querySelector('.nav'))} ${at(bar.querySelector('.space-foot'))} ${[...bar.querySelector('.space-foot').children].map(at).join(' ')} ${bar.querySelector('.nav').scrollTop}`;
      });
    const before = await places();
    await button.click();
    await opened(menu);
    const [over, from, screen] = [await menu.boundingBox(), await button.boundingBox(), page.viewportSize()];
    const told = JSON.stringify({ over, from, screen });
    expect(`the menu to open upward from its button in ${name} ${where}: ${told}`, over.y + over.height <= from.y + 0.5);
    expect(`the menu to be on the screen ${where}: ${told}`, over.x >= -0.5 && over.y >= -0.5 && over.x + over.width <= screen.width + 0.5 && over.y + over.height <= screen.height + 0.5);
    expect(`the menu to be no narrower than what opened it ${where}: ${told}`, over.width >= from.width - 0.5);
    expect(`the menu to push nothing about ${where}: "${before}" and then "${await places()}"`, (await places()) === before);
    expect('the button to say that its menu is open', (await button.getAttribute('aria-expanded')) === 'true');
    // Nothing a menu says or offers lies over anything else of it, and all of it is inside the menu.
    const within = await menu.evaluate((drawn) => {
      const at = drawn.getBoundingClientRect();
      const rows = [...drawn.children].map((row) => row.getBoundingClientRect());
      return {
        stacked: rows.every((row, place) => place === 0 || rows[place - 1].bottom <= row.top + 0.5),
        inside: rows.every((row) => row.left >= at.left - 0.5 && row.right <= at.right + 0.5),
        cut: [...drawn.querySelectorAll('.menu-sign, .mark, .waits')].some((one) => one.getBoundingClientRect().right > at.right + 0.5),
        sideways: drawn.scrollWidth - drawn.clientWidth,
      };
    });
    expect(`nothing of the menu to lie over anything else of it or to leave it ${where}: ${JSON.stringify(within)}`, within.stacked && within.inside && !within.cut && within.sideways <= 0);
    // The pointer crosses from the button to the menu, over what is between them.
    await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
    await page.mouse.move(over.x + Math.min(over.width / 2, 60), over.y + over.height - 12, { steps: 8 });
    await page.mouse.move(over.x + Math.min(over.width / 2, 60), over.y + 12, { steps: 8 });
    expect(`the menu to be there still when the pointer has crossed to it ${where}`, await menu.isVisible());
    if (picture) await page.screenshot({ path: `${out}/${picture}` });
    await page.keyboard.press('Escape');
    await menu.waitFor({ state: 'detached' });
    expect('the button to say that its menu is put away', (await button.getAttribute('aria-expanded')) === 'false');
  };
  // The page read again, signed in as before, once it has read what there is to say of releases.
  const again = async () => {
    const read = page.waitForResponse((response) => response.url() === `${url}/api/releases`);
    await page.reload();
    await page.locator('.shell').waitFor();
    await (await read).finished();
    // What was read is drawn by the frame after it is in.
    await page.evaluate(() => new Promise((drawn) => requestAnimationFrame(() => requestAnimationFrame(drawn))));
  };

  // ---- the foot: the workspace made for the factory's owner, and Help at its right
  expect(`the notes to tell of a release: ${RELEASES}`, RELEASES.length > 0);
  await side('Resources');
  await resources.locator('.space-foot .foot-new').waitFor();
  await listed(() => resources.locator('.space-foot .space-said strong').innerText(), 'Ada\'s workspace', 'the foot to say the workspace the factory has from the first');
  expect('nothing of the panel to say no workspace', !/no workspace/i.test(await page.locator('.sidebar').innerText()));
  expect(`Help to say that something is new, got "${await help(resources).getAttribute('aria-label')}"`, (await help(resources).getAttribute('aria-label')) === 'Help: there is something new');
  await strip('Resources', 'with the owner\'s workspace', 2);
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/foot-00-the-owners-workspace.png` });

  // ---- Help's menu: nothing that opens an address while none is written, what to say in a report, and what is new
  const NEW = [...RELEASES.slice(0, 5), 'View all releases'].join(' | ');
  await openHelp(resources);
  expect(`Help to offer the guide, what to say in a report and the releases, and no way to an address, got "${await lines(helpMenu)}"`, (await lines(helpMenu)) === `Setup Guide | Reference | Copy what to say in a report | ${NEW}`);
  expect(`the releases to be under their heading, got "${await says(helpMenu)}"`, (await says(helpMenu)) === "What's new");
  expect(`the menu to begin at its first line, got ${await focused()}`, (await focused()) === 'menuitem:Setup Guide');
  // Shown the list, what is newest is new no longer: the dot is gone, and nothing has moved for it.
  await page.locator('.foot-new').waitFor({ state: 'detached' });
  expect(`Help to say nothing of anything new once the list was shown, got "${await help(resources).getAttribute('aria-label')}"`, (await help(resources).getAttribute('aria-label')) === 'Help');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go to the next line, got ${await focused()}`, (await focused()) === 'menuitem:Reference');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go on to the one after, got ${await focused()}`, (await focused()) === 'menuitem:Copy what to say in a report');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go on to the newest release, got ${await focused()}`, (await focused()) === `menuitem:${RELEASES[0]}`);
  await page.keyboard.press('End');
  expect(`End to go to the last line, got ${await focused()}`, (await focused()) === 'menuitem:View all releases');
  await page.keyboard.press('Home');
  expect(`Home to go to the first, got ${await focused()}`, (await focused()) === 'menuitem:Setup Guide');
  await page.keyboard.press('ArrowUp');
  expect(`the arrow to go round, got ${await focused()}`, (await focused()) === 'menuitem:View all releases');
  await page.keyboard.press('Escape');
  await helpMenu.waitFor({ state: 'detached' });
  expect(`Help to have the keyboard again, got ${await focused()}`, (await focused()) === 'null:Help');
  // Pressed again while it is open, the button puts its menu away.
  await openHelp(resources);
  await help(resources).click();
  await helpMenu.waitFor({ state: 'detached' });
  expect(`nothing to have been opened by leaving the menu, got "${await tabs()}"`, !(await tabs()).includes("What's new"));
  await overhead('Resources', help(resources), helpMenu, 'at the usual width', 'foot-01-help-opened.png');

  // The guide of first steps is the menu's first line, and leads back to the start page from whatever is showing.
  await page.locator('.nav').getByText('Activity', { exact: true }).click();
  await listed(async () => String((await tabs()).includes('Activity')), 'true', 'another page to be showing');
  await openHelp(resources);
  await helpMenu.getByRole('menuitem', { name: 'Setup Guide', exact: true }).click();
  await helpMenu.waitFor({ state: 'detached' });
  await page.getByRole('heading', { name: 'Setup Guide' }).waitFor();
  expect(`the guide to be the page showing, got "${await page.locator('.tabs .tab.active, .tabs .tab[aria-selected="true"]').first().innerText().catch(() => '')}"`, (await page.getByRole('heading', { name: 'Setup Guide' }).isVisible()));

  // What to say in a report is copied, and said to be: by the keyboard, the menu's second line.
  const meta = await page.evaluate(() => fetch('/api/meta').then((answer) => answer.json()));
  await help(resources).focus();
  await page.keyboard.press('Enter');
  await opened(helpMenu);
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  await helpMenu.waitFor({ state: 'detached' });
  await page.locator('.toast', { hasText: 'Copied. Paste it into what you report.' }).waitFor();
  const copied = (await page.evaluate(() => navigator.clipboard.readText())).split('\n');
  const agent = await page.evaluate(() => navigator.userAgent);
  expect(`what was copied to say the program and its version first, got ${JSON.stringify(copied)}`, copied.length === 3 && copied[0].startsWith(`${meta.name} ${meta.version} (`) && copied[0].includes(`client build ${meta.client_build}`));
  expect(`what was copied to say the daemon's system and the browser, got ${JSON.stringify(copied)}`, /^System: \S+/.test(copied[1]) && copied[2] === `Browser: ${agent}`);

  // ---- a release pressed: the notes as a page in a tab, at that release, with nothing in it run
  await openHelp(resources);
  await helpMenu.getByRole('menuitem', { name: RELEASES[0], exact: true }).click();
  await helpMenu.waitFor({ state: 'detached' });
  const notes = page.locator('.releases iframe.page-frame.whole');
  await notes.waitFor();
  await listed(async () => String((await tabs()).includes("What's new")), 'true', 'the notes to be a tab called for what they are');
  expect(`the notes to be boxed off with nothing in them run, got sandbox="${await notes.getAttribute('sandbox')}"`, (await notes.getAttribute('sandbox')) === 'allow-same-origin');
  const inside = page.frameLocator('.releases iframe.page-frame.whole');
  await inside.locator('#release-1 h2').waitFor();
  expect(`the newest release to be the first part of the notes, got "${await inside.locator('#release-1 h2').innerText()}"`, (await inside.locator('#release-1 h2').innerText()).trim() === RELEASES[0]);
  expect(`each release to be a part of the notes: ${RELEASES.length}, got ${await inside.locator('section[id^="release-"]').count()}`, (await inside.locator('section[id^="release-"]').count()) === RELEASES.length);
  expect('nothing in the notes to be a script', (await inside.locator('script').count()) === 0);
  // As tall as the page is, so that what scrolls is the tab and not a box within it.
  await page.waitForFunction(() => {
    const frame = document.querySelector('.releases iframe.page-frame.whole');
    const whole = frame?.contentDocument?.documentElement;
    return !!whole && Math.abs(frame.getBoundingClientRect().height - Math.ceil(whole.getBoundingClientRect().height)) <= 1;
  });
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/foot-02-the-notes.png` });
  // All of them is the same page, in the one tab.
  await openHelp(resources);
  await helpMenu.getByRole('menuitem', { name: 'View all releases', exact: true }).click();
  await helpMenu.waitFor({ state: 'detached' });
  expect(`the notes to be one tab however often they are opened, got "${await tabs()}"`, (await tabs()).split("What's new").length === 2);
  // What this browser was shown is kept under who is signed in, and the dot stays gone when the page is read again.
  const seen = () => page.evaluate(() => Object.entries(localStorage).filter(([key]) => key.startsWith('client.releases.seen:user:')).map(([, value]) => value).join(' | '));
  expect(`the newest release this browser was shown to be kept, got "${await seen()}"`, (await seen()) === RELEASES[0]);
  await again();
  await side('Resources');
  expect('the dot to stay gone when the page is read again', (await page.locator('.foot-new').count()) === 0 && (await help(resources).getAttribute('aria-label')) === 'Help');
  // Forgotten, it is new again: the dot is by what was kept and nothing else.
  await page.evaluate(() => Object.keys(localStorage).filter((key) => key.startsWith('client.releases.seen')).forEach((key) => localStorage.removeItem(key)));
  await again();
  await side('Resources');
  await resources.locator('.space-foot .foot-new').waitFor();

  // ---- with an address for support: a line that opens it in a tab of its own, and nothing fetched by this page
  await page.route('**/api/meta', async (route) => {
    const answer = await route.fetch();
    await route.fulfill({ response: answer, json: { ...(await answer.json()), support: SUPPORT } });
  });
  await again();
  await side('Resources');
  await openHelp(resources);
  expect(`Help to offer the way to support first, got "${await lines(helpMenu)}"`, (await lines(helpMenu)) === `Get support or report a problem | Setup Guide | Reference | Copy what to say in a report | ${NEW}`);
  await page.screenshot({ path: `${out}/foot-03-with-support.png` });
  const before = asked.length;
  const [support] = await Promise.all([context.waitForEvent('page'), helpMenu.getByRole('menuitem', { name: 'Get support or report a problem', exact: true }).click()]);
  await support.waitForLoadState();
  expect(`the address for support to be opened in a tab of its own, got ${support.url()}`, support.url() === SUPPORT);
  // Of anywhere but its daemon: the page goes on reading what its daemon
  // says has changed (the blueprints in use, as instances come and go),
  // and one such reading fell here six times in eight once the daemon
  // answered sooner. What is held is that opening the address is the
  // browser's doing, in a tab of its own, and nothing this page fetched.
  const outside = asked.slice(before).filter((one) => !one.startsWith(`${url}/`));
  expect(`this page to have asked nothing of anywhere but its daemon as support was opened, got ${outside}`, outside.length === 0);
  expect(`this page to be where it was, got ${page.url()}`, page.url().startsWith(url));
  await support.close();
  await page.unroute('**/api/meta');
  await again();
  await side('Resources');
  await openHelp(resources);
  expect(`no way to an address again once none is said, got "${await lines(helpMenu)}"`, (await lines(helpMenu)) === `Setup Guide | Reference | Copy what to say in a report | ${NEW}`);
  await page.keyboard.press('Escape');
  await helpMenu.waitFor({ state: 'detached' });

  // ---- the panel dragged as narrow as it goes
  const edge = page.locator('.side-edge');
  const narrowed = async (nav) => {
    const grip = await edge.boundingBox();
    await page.mouse.move(grip.x + grip.width / 2, grip.y + 300);
    await page.mouse.down();
    await page.mouse.move(grip.x - 60, grip.y + 300, { steps: 6 });
    await page.mouse.move(60, grip.y + 300, { steps: 6 });
    await page.mouse.up();
    // As wide as it was let go at once the page has drawn it: 180 is the least it can be.
    await listed(async () => String(Math.round((await nav.boundingBox()).width)), '180', 'the side panel dragged as narrow as it goes');
  };
  await narrowed(resources);
  await strip('Resources', 'with the panel dragged narrow', 2);
  await overhead('Resources', help(resources), helpMenu, 'with the panel dragged narrow', 'foot-04-help-narrow.png');
  await edge.dblclick();
  await listed(async () => String(Math.round((await resources.boundingBox()).width)), '252', 'the side panel as wide as it usually is again');

  // ---- another workspace made from a terminal, and chosen: its menu, from its head down
  await cli('workspace', 'create', 'Shop', 'front');
  await cli('workspace', 'put', 'shop-front', 'shop');
  const pick = (nav) => nav.locator('.space-foot .space-pick');
  await pick(resources).waitFor();
  // What the panel is of is kept: one made later does not take its place, and is chosen from the foot.
  expect(`the panel to be of the workspace it was of, got "${await resources.locator('.space-foot .space-said strong').innerText()}"`, (await resources.locator('.space-foot .space-said strong').innerText()) === 'Ada\'s workspace');
  await listed(
    async () => {
      await pick(resources).click();
      await opened(spaceMenu);
      const offered = await spaceMenu.getByRole('menuitemradio', { name: /^Shop front/ }).count();
      if (offered === 1) await spaceMenu.getByRole('menuitemradio', { name: /^Shop front/ }).click();
      else await page.keyboard.press('Escape');
      await spaceMenu.waitFor({ state: 'detached' });
      return String(offered);
    },
    '1',
    'the workspace made from a terminal to be offered at the foot',
  );
  await listed(() => resources.locator('.space-foot .space-said strong').innerText(), 'Shop front', 'the foot to say the workspace chosen');
  expect(`the workspace to be drawn by the first letter of its name, got "${await pick(resources).locator('.space-face').innerText()}"`, (await pick(resources).locator('.space-face').innerText()) === 'S');
  await strip('Resources', 'with a workspace', 2);
  const openSpace = async (nav) => {
    await pick(nav).click();
    await opened(spaceMenu);
  };
  const DOES = 'New workspace… | Settings | Log out';
  // What a menu offers is what there was when it was opened: the project is waited for where the menu says how many.
  await listed(
    async () => {
      await openSpace(resources);
      const got = `${await says(spaceMenu)} || ${await lines(spaceMenu)} || ${(await spaceMenu.getByRole('menuitemradio').allInnerTexts()).map((line) => line.replace(/\s+/g, ' ').trim()).join(' | ')}`;
      await page.keyboard.press('Escape');
      await spaceMenu.waitFor({ state: 'detached' });
      return got;
    },
    // Each workspace is drawn with the first letter of its name before it, and the one showing with a tick after.
    `S Shop front 1 project | Signed in as Ada || ${DOES} || A Ada's workspace | S Shop front ✓`,
    'the menu to say the workspace with what it holds and who is signed in, to offer what is done, and each workspace',
  );
  // Settings and signing out are set apart at the foot of the menu: a line is drawn over Settings, and none between it and Log out.
  await openSpace(resources);
  const apart = await spaceMenu.getByRole('menuitem').evaluateAll((all) => all.filter((one) => one.classList.contains('apart')).map((one) => one.innerText.trim()).join(' | '));
  expect(`Settings alone to be set apart, with Log out under it, got "${apart}"`, apart === 'Settings');
  const last = await spaceMenu.evaluate((drawn) => [...drawn.children].slice(-2).map((one) => one.innerText.trim()).join(' | '));
  expect(`Settings and Log out to be the menu's last two lines, got "${last}"`, last === 'Settings | Log out');
  await page.keyboard.press('Escape');
  await spaceMenu.waitFor({ state: 'detached' });
  await overhead('Resources', pick(resources), spaceMenu, 'at the usual width', 'foot-05-the-workspace-opened.png');
  await overhead('Resources', help(resources), helpMenu, 'beside a workspace', 'foot-06-help-beside-a-workspace.png');

  // Settings is opened from there, showing the part for the workspace the menu is of. In a short window who is here
  // is further down that page than is showing.
  await page.setViewportSize({ width: 1350, height: 460 });
  await openSpace(resources);
  await spaceMenu.getByRole('menuitem', { name: 'Settings', exact: true }).click();
  await spaceMenu.waitFor({ state: 'detached' });
  const people = page.getByRole('heading', { name: 'People', exact: true });
  await people.waitFor();
  await listed(async () => String((await tabs()).includes('Settings')), 'true', 'Settings to be opened from the menu');
  const inSight = async (what = people) => {
    const at = await what.boundingBox();
    return at.y >= 0 && at.y + at.height <= page.viewportSize().height;
  };
  const part = page.getByRole('region', { name: 'Workspace', exact: true });
  await listed(async () => String(await inSight(part.getByRole('heading', { name: 'Workspace', exact: true }))), 'true', 'the workspace\'s part of Settings to be in view');
  await listed(() => part.getByLabel('Name', { exact: true }).inputValue(), 'Shop front', 'the part to be of the workspace the menu was of');
  expect('who is here to be out of sight until it is asked for', !(await inSight()));
  // Inviting is offered to somebody who may invite as things stand, and to nobody else: not on a plan for one person.
  await page.locator('.tabs .tab', { hasText: 'Start' }).first().click();
  const tier = (plan) => page.evaluate((to) => fetch('/api/tier', { method: 'POST', headers: { 'content-type': 'application/json', 'x-page-key': localStorage['page-key'] }, body: JSON.stringify({ tier: to }) }).then((answer) => answer.status), plan);
  expect('the plan to be changed', (await tier('premium')) === 200);
  await again();
  await side('Resources');
  await pick(resources).waitFor();
  await openSpace(resources);
  expect(`inviting to be offered to somebody who may, after Settings, got "${await lines(spaceMenu)}"`, (await lines(spaceMenu)) === DOES.replace('Settings | ', 'Settings | Invite members | '));
  await spaceMenu.getByRole('menuitem', { name: 'Invite members', exact: true }).click();
  await spaceMenu.waitFor({ state: 'detached' });
  await people.waitFor();
  await listed(async () => String(await inSight()), 'true', 'who is here to be brought into view');
  expect('inviting to be what is there to press', await page.getByRole('button', { name: 'Invite someone', exact: true }).isEnabled());
  await page.screenshot({ path: `${out}/foot-07-inviting.png` });
  await page.setViewportSize({ width: 1350, height: 860 });

  // ---- with the panel dragged as narrow as it goes, and at a phone's width
  await side('Channels');
  await narrowed(panel);
  await strip('Channels', 'with a workspace and the panel dragged narrow', 2);
  await overhead('Channels', pick(panel), spaceMenu, 'with the panel dragged narrow', 'foot-08-the-workspace-narrow.png');
  await overhead('Channels', help(panel), helpMenu, 'with the panel dragged narrow', 'foot-09-help-narrow-beside-a-workspace.png');
  await edge.dblclick();
  await listed(async () => String(Math.round((await panel.boundingBox()).width)), '252', 'the side panel as wide as it usually is again');
  await page.setViewportSize({ width: 390, height: 844 });
  await again();
  await page.getByRole('button', { name: 'Expand the side panel' }).click();
  await sideSettled(page);
  await page.locator('.sidebar').waitFor();
  await side('Resources');
  await pick(resources).waitFor();
  const reach = await strip('Resources', "at a phone's width", 2);
  expect(`Help to be large enough for a finger at a phone's width: ${JSON.stringify(reach.help)}`, reach.help.right - reach.help.left >= 40 && reach.help.bottom - reach.help.top >= 40);
  await page.screenshot({ path: `${out}/foot-10-at-390.png` });
  await overhead('Resources', pick(resources), spaceMenu, "at a phone's width", 'foot-11-the-workspace-opened-at-390.png');
  await overhead('Resources', help(resources), helpMenu, "at a phone's width", 'foot-12-help-opened-at-390.png');

  // ---- nothing was asked of anywhere but this daemon, and the icons' part of the client never
  const elsewhere = asked.filter((one) => !one.startsWith(`${url}/`) && !one.startsWith('data:') && !one.startsWith('blob:') && !one.startsWith('about:'));
  expect(`the page to have asked nothing of anywhere but its daemon, got ${elsewhere}`, elsewhere.length === 0);
  expect(`the check to know which parts of the client hold the icons: ${icons}`, icons.length > 0);
  expect('no part of the client that holds the icons to have been asked for', !asked.some((one) => icons.some((part) => one.endsWith(`/${part}`))));

  // ---- Log out signs out
  await openSpace(resources);
  await spaceMenu.getByRole('menuitem', { name: 'Log out', exact: true }).click();
  await page.getByLabel('Email').waitFor();
  expect('nobody to be signed in once somebody logged out', (await page.locator('.rail').count()) === 0 && (await page.locator('.space-foot').count()) === 0);
  await page.screenshot({ path: `${out}/foot-13-logged-out.png` });

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
