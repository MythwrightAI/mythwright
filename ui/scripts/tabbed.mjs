// The open tabs of each workspace: choosing another workspace shows its
// tabs, and going back shows what was open.
//
//   DAEMON=../target/debug/factory node scripts/tabbed.mjs [out-dir]
//
// A factory with three workspaces, each with a project and a channel,
// made from a terminal. What a browser kept from before each workspace
// had its own tabs is there as the page is first read, with nothing
// chosen: the panel is of the workspace of the tab that was showing in
// it, that set is that workspace's, and it is taken out of where it was
// kept. Tabs are opened there and split into two groups, something is
// half written in a channel and in a file, and a drawer is opened. Then
// another workspace is chosen at the foot: the start page alone, nothing
// of the first on the page, the drawer put away, nobody asked about what
// is unsaved, what was lately opened kept as it was, nothing asked of the
// daemon that says a workspace or a tab, and no moment at which the page
// shows anything but the one set whole and then the other. Tabs are
// opened there; and back, by the keyboard and under the other panel: the
// first one's tabs, its two groups, the tab that was showing in each, the
// group last used, the drawer given back to the tab it was opened beside,
// and what was half written. The page is read again in
// each workspace and shows that workspace's set. Something of the other
// workspace opened from a search, and by an address, is a tab where it
// was opened and moves nothing. The count of what waits unread, pressed,
// shows that workspace's tabs. A project forgotten from a terminal while
// its tabs are in the set that is not showing is gone when that set is
// shown; a channel removed is in no set this browser keeps; a workspace
// taken away has its set forgotten, and one made under its name finds
// none; the one showing taken away leaves the panel of another, with
// that one's tabs. What a browser kept from before is not shown to a
// workspace that has a set of its own. The same at a phone's width; and
// signing out forgets every set. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll, unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'tabbed-data');
const home = join(out, 'tabbed-home');

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

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  // The owner's workspace is the usual one: Shed and its channel are in it. Then two more, each with a project and a channel.
  for (const name of ['Shop', 'Depot', 'Shed']) await cli('project', 'create', name, '--bare');
  await cli('workspace', 'create', 'Front');
  await cli('workspace', 'put', 'front', 'shop');
  await cli('workspace', 'create', 'Back');
  await cli('workspace', 'put', 'back', 'depot');
  await cli('channel', 'create', 'shop-talk', '--project', 'shop');
  await cli('channel', 'create', 'depot-talk', '--project', 'depot');
  await cli('channel', 'create', 'yard', '--workspace', 'back');
  await cli('channel', 'create', 'chat');
  const USUAL = 'Ada\'s workspace';
  // Each channel's id, by its name: a tab is kept by the id.
  const ids = Object.fromEntries(JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.map((one) => [one.name, one.id]));
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
  // Everything the page asks of its daemon, as it asks: what a choice of workspace must add nothing to.
  const asked = [];
  page.on('request', (request) => asked.push(`${request.method()} ${new URL(request.url()).pathname}${new URL(request.url()).search} ${request.postData() ?? ''}`));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
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

  // ---- what a browser kept from before each workspace had its own tabs, with nothing chosen
  // One set for the whole client, as it was kept: the start page, the usual workspace's project and Shop's page, which was showing.
  await page.goto(url);
  await page.getByLabel('Email').waitFor();
  const BEFORE = { tabs: [{ kind: 'home' }, { kind: 'project', id: 'shed' }, { kind: 'project', id: 'shop' }], layout: { kind: 'group', id: 'g1', tabs: ['home', 'project:shed', 'project:shop'], active: 'project:shop' }, focused: 'g1' };
  await page.evaluate((kept) => localStorage.setItem('client.tabs', JSON.stringify(kept)), BEFORE);
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
    if (name === 'Channels') await unfoldChannels(page);
  };
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const panel = page.getByRole('navigation', { name: 'Channels' });
  // The pointer taken off the panel: the list of what was lately opened holds still while it is pointed at.
  const away = () => page.mouse.move(980, 700);
  /** The groups as they are on the page: each one's tabs, the one showing marked with a star, and the group last used in brackets. */
  const groups = () =>
    page.locator('.tabs').evaluateAll((strips) =>
      strips
        .map((strip) => {
          const tabs = [...strip.querySelectorAll('.tab')].map((tab) => `${tab.querySelector('.truncate').textContent}${tab.getAttribute('aria-selected') === 'true' ? '*' : ''}`).join(',');
          return strip.classList.contains('focused') ? `[${tabs}]` : tabs;
        })
        .join(' | '),
    );
  /** The views that are on the page, by the tab each is of. */
  const views = () => page.locator('.view').evaluateAll((all) => all.map((view) => view.dataset.tab).sort().join(' '));
  const tab = (name) => page.locator('.tab', { has: page.locator('.truncate', { hasText: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`) }) });
  const showing = (nav) => nav.locator('.space-foot .space-said strong').innerText();
  const foot = (nav) => nav.locator('.space-foot .space-pick');
  const spaceMenu = page.getByRole('menu', { name: 'Workspace', exact: true });
  // Opened, the menu has the keyboard, which is what says it is listening.
  const opened = async () => {
    await spaceMenu.waitFor();
    await page.waitForFunction(() => document.activeElement?.closest('.row-menu') !== null && document.activeElement?.closest('.row-menu') !== undefined);
  };
  const toSpace = async (nav, name) => {
    await foot(nav).click();
    await opened();
    await spaceMenu.getByRole('menuitemradio', { name: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(:|$)`) }).click();
    await spaceMenu.waitFor({ state: 'detached' });
    await away();
    await listed(() => showing(nav), name, `the foot to say ${name}`);
  };
  // How many workspaces the foot's menu offers: what a menu offers is what there was when it was opened, so one made or taken away is waited for.
  const offered = async (nav) => {
    await foot(nav).click();
    await opened();
    const count = await spaceMenu.getByRole('menuitemradio').count();
    await page.keyboard.press('Escape');
    await spaceMenu.waitFor({ state: 'detached' });
    return String(count);
  };
  // What this browser keeps, under who is signed in: the choice, the sets of tabs by workspace, what was lately opened; and what one from before kept.
  const kept = (under) =>
    page.evaluate(
      (begins) =>
        Object.entries(localStorage)
          .filter(([key]) => key.startsWith(begins))
          .map(([, value]) => value)
          .join(' '),
      under,
    );
  const keptSets = async () => JSON.parse((await kept('client.tabs.kept:user:')) || '{}');
  const keptBefore = () => page.evaluate(() => localStorage.getItem('client.tabs'));
  /** The tabs this browser keeps for a workspace, by key; nothing where it keeps none. */
  const keptFor = async (id) => ((await keptSets())[id]?.tabs ?? []).map((one) => [one.kind, one.id ?? one.project ?? '', one.name ?? one.path ?? ''].filter(Boolean).join(':')).join(' ');
  const recent = () =>
    resources.evaluate((nav) => {
      const head = [...nav.querySelectorAll('.nav > h3')].find((one) => one.textContent.trim() === 'Recents');
      return head ? [...head.nextElementSibling.querySelectorAll('.lately-line')].map((row) => row.querySelector('.truncate').textContent).join(' | ') : '(not drawn)';
    });
  const tree = (id) => resources.locator(`[data-project="${id}"] > .nav-row`);
  const says = (words) => page.getByText(words, { exact: true });
  const channel = (name) => panel.getByRole('button', { name: new RegExp(`^# ${name}( \\d+)?$`) });
  const composer = (name) => page.locator('.view', { has: page.locator('.view-head', { hasText: name }) }).locator('.channel > .split .composer').getByLabel('Message');
  const narrow = async () => {
    const wide = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`the page not to scroll sideways, and it is ${wide}px too wide`, wide <= 0);
  };

  await side('Resources');
  await foot(resources).waitFor();
  // Nothing was chosen: the panel is of the workspace of the tab that was showing, and not the usual one.
  await listed(() => showing(resources), 'Front', 'the panel to begin at the workspace of the tab that was showing in what a browser kept from before');
  await listed(groups, '[Start,Shed,Shop*]', 'what a browser kept from before to be what is open');
  await listed(() => kept('client.workspace.held:user:'), '{"kind":"space","id":"front"}', 'the workspace the panel began at to be kept as the choice');
  await listed(() => keptFor('front'), 'home project:shed project:shop', 'what a browser kept from before to be kept as that workspace\'s set');
  expect(`what a browser kept from before to be taken out of where it was kept, got ${await keptBefore()}`, (await keptBefore()) === null);
  expect(`no other workspace to have a set by it, got ${Object.keys(await keptSets())}`, Object.keys(await keptSets()).join() === 'front');
  // Putting tabs back is nobody opening them.
  await away();
  expect(`nothing to be lately opened by the page putting its tabs back, got "${await recent()}"`, (await recent()) === '(not drawn)');
  await page.screenshot({ path: `${out}/tabbed-00-what-a-browser-kept-from-before.png` });

  // ---- tabs opened in the first workspace, split into two groups, with something half written
  // A file of the project, written through the daemon so that it is listed.
  await page.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const put = await fetch('/api/projects/shop/file', { method: 'PUT', headers: mine, body: JSON.stringify({ path: 'notes.md', content: '# Notes\n\nNothing yet.\n' }) });
    if (!put.ok) throw new Error('could not write notes.md');
  });
  // Its folder is of the factory's making, so its files are not listed in the panel: the file is opened by its address.
  await page.evaluate(() => (location.hash = '#/file/shop/notes.md'));
  await page.locator('.view .writer h1', { hasText: 'Notes' }).waitFor();
  await side('Channels');
  await channel('shop-talk').click();
  await composer('shop-talk').waitFor();
  await listed(groups, '[Start,Shed,Shop,notes.md,shop-talk*]', 'a file and a channel opened beside what was open');
  // The channel carried to the right edge of the group: two groups side by side.
  const whole = await page.locator('.view').first().boundingBox();
  const from = await tab('shop-talk').boundingBox();
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(whole.x + whole.width * 0.95, whole.y + whole.height * 0.5, { steps: 6 });
  await page.locator('.landing').waitFor();
  await page.mouse.up();
  await listed(groups, 'Start,Shed,Shop,notes.md* | [shop-talk*]', 'two groups side by side');
  // Something half written in the file, which its tab then says is not saved.
  await page.locator('.view .writer h1', { hasText: 'Notes' }).click();
  await page.keyboard.press('End');
  await page.waitForTimeout(150);
  await page.keyboard.type(' of the shop');
  await tab('notes.md').getByLabel('not saved').waitFor();
  // Another tab showing in the first group than its last, and something half written in the channel: the caret is in the second group.
  await tab('Shop').click();
  await composer('shop-talk').click();
  await composer('shop-talk').fill('half a thought about the till');
  const FIRST = 'Start,Shed,Shop*,notes.md | [shop-talk*]';
  await listed(groups, FIRST, 'the first workspace\'s tabs: two groups, a tab showing in each, the caret in the second');
  const firstViews = await views();
  expect(`a view for the tab showing in each group, got "${firstViews}"`, firstViews.split(' ').length === 2 && firstViews.includes('project:shop'));
  await side('Resources');
  // A drawer: what a blueprint is applied with is asked in one, beside its text.
  await unfold(page, resources.locator('.nav > .tree > div > button.nav-item', { has: says('std') }));
  await resources.locator('.nav > .tree > div', { has: page.locator(':scope > button.nav-item', { has: says('std') }) }).locator('.tree button.nav-item', { has: says('std/board') }).click();
  // A blueprint that ships opens on what it says in plain words; what it is applied with is asked from there as from its text.
  await page.locator('.summary[data-of="std/board"]').waitFor();
  await page.locator('.view-head button.primary').last().click();
  await drawerAtRest(page).getByRole('heading', { name: /^Apply / }).waitFor();
  const FRONT = 'Start,Shed,Shop*,notes.md | [shop-talk,std/board*]';
  await listed(groups, FRONT, 'the first workspace\'s tabs with a blueprint opened in the group last used');
  await away();
  await listed(recent, 'std/board | Shop | notes.md', 'what was opened to be what was lately opened');
  const keptLately = await kept('client.side.lately:user:');
  await page.screenshot({ path: `${out}/tabbed-01-a-drawer-open.png` });

  // ---- the second workspace chosen at the foot: the start page alone, and nothing of the first on the page
  // Every set of tabs the page draws from before a press until it is at rest: looked at as often as the page changes, and at every frame.
  const watch = () =>
    page.evaluate(() => {
      const drawn = () =>
        [...document.querySelectorAll('.tabs')]
          .map((strip) => [...strip.querySelectorAll('.tab .truncate')].map((one) => one.textContent).join(','))
          .join(' | ') +
        ' // ' +
        [...document.querySelectorAll('.view')]
          .map((view) => view.dataset.tab)
          .sort()
          .join(' ');
      const seen = [drawn()];
      const look = () => {
        const now = drawn();
        if (seen.at(-1) !== now) seen.push(now);
      };
      const watcher = new MutationObserver(look);
      watcher.observe(document.body, { childList: true, subtree: true, characterData: true, attributes: true });
      let going = true;
      const frame = () => {
        look();
        if (going) requestAnimationFrame(frame);
      };
      requestAnimationFrame(frame);
      window.watched = () => {
        going = false;
        watcher.disconnect();
        look();
        return seen;
      };
    });
  const drawn = (tabs, shown) => `${tabs.replace(/[*[\]]/g, '')} // ${shown}`;
  // What was drawn while a workspace was chosen: the set that was showing, whole, and then the other, whole, and nothing between.
  const wholly = async (was, wasViews, now, nowViews, where) => {
    await page.waitForTimeout(300);
    const saw = await page.evaluate(() => window.watched());
    expect(`the page to draw the set that was showing and then the other's, each whole and nothing between, ${where}: got ${JSON.stringify(saw)}`, saw.length === 2 && saw[0] === drawn(was, wasViews) && saw[1] === drawn(now, nowViews));
  };
  const frontViews = await views();
  expect(`a view for the tab showing in each group, got "${frontViews}"`, frontViews === 'blueprint::std/board project:shop');
  await watch();
  const mark = asked.length;
  await toSpace(resources, 'Back');
  await listed(groups, '[Start*]', 'a workspace with nothing kept to show the start page alone');
  await listed(views, 'home', 'the start page to be the one view on the page');
  await wholly(FRONT, frontViews, 'Start', 'home', 'as the second workspace is chosen');
  expect('the drawer to be put away by another workspace being shown', (await page.locator('.drawer').count()) === 0);
  expect('nobody to be asked about what is not saved: nothing was closed', (await page.getByRole('dialog').count()) === 0);
  const text = await page.locator('.panes').innerText();
  expect(`nothing of the first workspace's tabs to be on the page, got "${text.slice(0, 200)}"`, !/shop-talk|notes\.md|half a thought|std\/board/.test(text));
  // What was lately opened is what somebody opened: nothing is opened by a workspace being shown.
  expect(`what was lately opened to be kept as it was, got ${await kept('client.side.lately:user:')}`, (await kept('client.side.lately:user:')) === keptLately);
  // Nothing is asked of the daemon that says which workspace is looked at, or which tabs are open.
  const since = asked.slice(mark);
  expect(`nothing but readings to be asked of the daemon as a workspace is chosen, got ${JSON.stringify(since)}`, since.every((one) => one.startsWith('GET ')));
  expect(`nothing asked to say a workspace or a tab, got ${JSON.stringify(since)}`, since.every((one) => !/back|front|workspace|tabs/i.test(one)));
  await listed(() => kept('client.workspace.held:user:'), '{"kind":"space","id":"back"}', 'the choice to be kept');
  await listed(() => keptFor('front'), 'home project:shed project:shop file:shop:notes.md channel blueprint:std/board'.replace('channel', `channel:${ids['shop-talk']}`), 'the first workspace\'s set to be kept under it');
  await listed(() => keptFor('back'), 'home', 'the second workspace\'s set to be kept under it');
  await page.screenshot({ path: `${out}/tabbed-02-the-second-workspace-with-nothing-kept.png` });

  // ---- tabs opened there are that workspace's
  await tree('depot').locator('.nav-item').click();
  await side('Channels');
  await channel('depot-talk').click();
  await channel('yard').click();
  await composer('yard').waitFor();
  const BACK = '[Start,Depot,depot-talk,yard*]';
  await listed(groups, BACK, 'the second workspace\'s tabs');
  expect(`the first workspace's set to be as it was kept, got "${await keptFor('front')}"`, (await keptFor('front')).endsWith('blueprint:std/board') && !(await keptFor('front')).includes('depot'));
  await page.screenshot({ path: `${out}/tabbed-03-the-second-workspace.png` });

  // ---- back to the first, by the keyboard and under the other panel: what was open, as it was
  const focused = () => page.evaluate(() => `${document.activeElement?.getAttribute('role')}:${document.activeElement?.getAttribute('aria-label') ?? document.activeElement?.querySelector('.truncate')?.textContent}`);
  const backViews = await views();
  await panel.getByRole('button', { name: 'Workspace: Back', exact: true }).focus();
  await page.keyboard.press('Enter');
  await opened();
  expect(`the menu to begin at the workspace showing, got ${await focused()}`, (await focused()) === 'menuitemradio:Back');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go to the next, got ${await focused()}`, (await focused()) === 'menuitemradio:Front');
  await watch();
  await page.keyboard.press('Enter');
  await spaceMenu.waitFor({ state: 'detached' });
  await listed(() => showing(panel), 'Front', 'the workspace the keyboard chose');
  await listed(groups, FRONT, 'the first workspace\'s tabs again: its two groups, the tab that was showing in each, and the group last used');
  await listed(views, frontViews, 'the views of the tabs that were showing');
  await wholly(BACK, backViews, FRONT, frontViews, 'as the first is chosen again');
  // The drawer is the tab's it was opened beside: shown again, that tab has it back as it was, simply there.
  await drawerAtRest(page).getByRole('heading', { name: /^Apply / }).waitFor();
  expect('the drawer to come back with its tab and not come in again', (await page.locator('.drawer').evaluate((drawer) => drawer.getAnimations().length)) === 0);
  expect(`the other panel to be of the same workspace, got "${await showing(panel)}"`, (await showing(panel)) === 'Front');
  // What was half written is there: in the channel, and in the file, whose tab still says it is not saved.
  await tab('shop-talk').click();
  await listed(() => composer('shop-talk').inputValue(), 'half a thought about the till', 'what was half written in a channel');
  expect('the file\'s tab to say still that it is not saved', (await tab('notes.md').getByLabel('not saved').count()) === 1);
  await tab('notes.md').click();
  await page.locator('.view .writer h1', { hasText: 'Notes of the shop' }).waitFor();
  expect('nobody to have been asked anything', (await page.getByRole('dialog').count()) === 0);
  await side('Resources');
  await away();
  await listed(recent, 'notes.md | std/board | Shop', 'what was lately opened in the first workspace, with what was shown since');
  await page.screenshot({ path: `${out}/tabbed-04-back-in-the-first.png` });
  // Closing is still closing: a tab that holds something unsaved is asked about, and what was typed goes when it is thrown away.
  await tab('notes.md').locator('.close').click();
  await page.getByRole('dialog').getByText('notes.md has changes that are not saved').waitFor();
  await page.getByRole('dialog').getByRole('button', { name: 'Discard them', exact: true }).click();
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  await tab('Shop').click();
  await tab('shop-talk').click();
  const FRONT_NOW = 'Start,Shed,Shop* | [shop-talk*,std/board]';
  await listed(groups, FRONT_NOW, 'the first workspace\'s tabs with one closed and the channel showing');

  // ---- the page read again, in each workspace: that workspace's set
  const again = async () => {
    await page.reload();
    await page.locator('.rail').waitFor();
    await side('Resources');
    await foot(resources).waitFor();
  };
  await again();
  await listed(() => showing(resources), 'Front', 'the choice when the page is read again');
  await listed(groups, FRONT_NOW, 'the first workspace\'s set when the page is read again in it');
  await listed(() => composer('shop-talk').inputValue(), 'half a thought about the till', 'what was half written in a channel, over the page being read again');
  await toSpace(resources, 'Back');
  await listed(groups, BACK, 'the second workspace\'s set as it was left');
  await listed(recent, 'Depot | std/board', 'what was lately opened in the second workspace, with what is the factory\'s own');
  await again();
  await listed(() => showing(resources), 'Back', 'the choice when the page is read again in the second');
  await listed(groups, BACK, 'the second workspace\'s set when the page is read again in it');
  await away();
  expect(`nothing to be lately opened by the page being read again, got "${await recent()}"`, (await recent()) === 'Depot | std/board');

  // ---- something of the other workspace, opened from a search and by an address: a tab where it was opened
  await page.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const put = await fetch('/api/projects/shop/file', { method: 'PUT', headers: mine, body: JSON.stringify({ path: 'prices.md', content: '# Prices\n\nA florin each.\n' }) });
    if (!put.ok) throw new Error('could not write prices.md');
  });
  // The index follows what is written: asked until the file is found.
  await page.keyboard.press('Control+k');
  const hit = page.locator('.hit', { has: page.locator('strong', { hasText: 'prices.md' }) }).first();
  const until = Date.now() + 20000;
  while ((await hit.count()) === 0 && Date.now() < until) {
    await page.getByLabel('Search', { exact: true }).fill('');
    await page.getByLabel('Search', { exact: true }).fill('florin');
    await hit.waitFor({ timeout: 1500 }).catch(() => {});
  }
  await hit.click();
  await page.locator('.view .writer h1', { hasText: 'Prices' }).waitFor();
  await listed(groups, '[Start,Depot,depot-talk,yard,prices.md*]', 'a file of the other workspace found by a search, opened where the reader is');
  await page.evaluate(() => (location.hash = '#/project/shop'));
  await listed(groups, '[Start,Depot,depot-talk,yard,prices.md,Shop*]', 'a project of the other workspace opened by its address, beside what was open');
  // Written as addresses were before they were paths, and said by the bar the new way: the thing's path, as the tab showing is.
  await page.waitForFunction(() => location.pathname === '/shop' && location.hash === '');
  expect(`the panel to be of the workspace it was of, got "${await showing(resources)}"`, (await showing(resources)) === 'Back');
  expect(`the other workspace's set to be as it was, got "${await keptFor('front')}"`, !(await keptFor('front')).includes('prices.md'));
  const BACK_NOW = '[Start,Depot,depot-talk,yard,prices.md,Shop*]';
  await page.screenshot({ path: `${out}/tabbed-05-something-of-the-other-opened-here.png` });

  // ---- what waits unread in the other workspace, pressed at the foot: that workspace's tabs
  const bot = /`([^`]+)`/.exec((await cli('channel', 'speaker')).stdout)?.[1];
  expect('the factory\'s bot to say what it is called', !!bot);
  // Said in a channel that is open in the first workspace's set and is not on the page: it waits unread.
  await cli('post', 'shop-talk', `@${bot} help`);
  const press = resources.locator('.space-foot .space-out-press');
  await press.waitFor();
  await press.click();
  await listed(() => showing(resources), 'Front', 'the panel to be of the workspace where something waits, in one press');
  // The channel is showing in that set, so it has come into view and is read: nothing waits any more.
  await listed(groups, FRONT_NOW, 'that workspace\'s tabs');
  await listed(async () => String(await page.locator('.space-out').count()), '0', 'a channel that came into view by a workspace being shown to be read');

  // ---- a project forgotten, and a channel removed, from a terminal while tabs of theirs are in the set that is not showing
  // The project's page is open here too, as something of another workspace: what is on the page stays open, as it always did.
  await page.evaluate(() => (location.hash = '#/project/depot'));
  await listed(groups, 'Start,Shed,Shop* | [shop-talk,std/board,Depot*]', 'the other workspace\'s project opened here by its address');
  expect(`the other workspace's set to hold tabs of the project and the channel, got "${await keptFor('back')}"`, (await keptFor('back')).includes('project:depot') && (await keptFor('back')).includes(`channel:${ids.yard}`));
  await cli('project', 'forget', 'depot');
  await cli('channel', 'remove', 'yard');
  // A channel that is removed is known to be gone there and then: it is in nothing this browser keeps.
  await listed(async () => String(Object.values({ ...(await page.evaluate(() => ({ ...localStorage }))) }).join('\n').includes(ids.yard)), 'false', 'a removed channel to be in no set this browser keeps');
  // The page has heard that the project is gone once the tab that is open here is called by its id.
  await listed(groups, 'Start,Shed,Shop* | [shop-talk,std/board,depot*]', 'the forgotten project\'s tab on the page to stay, called what is left of it');
  await toSpace(resources, 'Back');
  await listed(groups, '[Start,depot-talk,prices.md,Shop*]', 'the tabs of what is gone to be gone from the set when it is shown, and the rest as they were');
  await listed(() => keptFor('back'), `home channel:${ids['depot-talk']} file:shop:prices.md project:shop`, 'the set to be kept without them');
  // And from the first one's, which was not showing meanwhile.
  await toSpace(resources, 'Front');
  await listed(groups, 'Start,Shed,Shop* | [shop-talk,std/board*]', 'the forgotten project\'s tab gone from the first workspace\'s set too, once that was kept and is shown again');
  const FRONT_LAST = 'Start,Shed,Shop* | [shop-talk,std/board*]';
  await page.screenshot({ path: `${out}/tabbed-06-what-is-gone-is-gone-from-a-set.png` });

  // ---- a workspace taken away from a terminal: its set is forgotten, and one made under its name finds none
  expect(`the second workspace's set to be kept, got ${Object.keys(await keptSets())}`, Object.keys(await keptSets()).sort().join() === 'back,front');
  await cli('workspace', 'remove', 'back', '--to', 'front');
  await listed(async () => Object.keys(await keptSets()).sort().join(), 'front', 'the set of a workspace that was taken away to be forgotten');
  await listed(groups, FRONT_LAST, 'what is open to stay as it was');
  await cli('workspace', 'create', 'Back');
  await listed(() => offered(resources), '3', 'the page to have read the workspace made again');
  await toSpace(resources, 'Back');
  await listed(groups, '[Start*]', 'a workspace made under the name of one that was taken away to show the start page alone');
  // The one showing taken away: the panel is of another, and that one's tabs are shown.
  await side('Channels');
  await channel('depot-talk').waitFor({ state: 'detached' });
  await side('Resources');
  await page.evaluate(() => (location.hash = '#/project/shed'));
  await listed(groups, '[Start,Shed*]', 'a project of the usual workspace opened in the one made again');
  await cli('workspace', 'remove', 'back');
  await listed(() => showing(resources), USUAL, 'the panel to be of the workspace of what was showing, once the one it was of is taken away');
  await listed(groups, '[Start*]', 'that workspace\'s own tabs to be shown, which are none yet, and not those of the one taken away');
  await listed(async () => Object.keys(await keptSets()).sort().join(), 'front,unnamed-workspace', 'nothing to be kept of the workspace taken away');
  await page.screenshot({ path: `${out}/tabbed-07-the-one-showing-taken-away.png` });

  // ---- a workspace made from the foot is shown, with nothing open; and called something else it keeps its set
  await tree('shed').locator('.nav-item').click();
  await listed(groups, '[Start,Shed*]', 'a tab opened in the usual workspace');
  await foot(resources).click();
  await opened();
  await spaceMenu.getByRole('menuitem', { name: 'New workspace…', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Name').fill('Spare');
  await page.getByRole('dialog').getByRole('button', { name: 'Create', exact: true }).click();
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  await listed(() => showing(resources), 'Spare', 'a workspace made from the foot to be what the panel is of');
  await listed(groups, '[Start*]', 'a workspace just made to show the start page alone');
  await cli('workspace', 'rename', 'unnamed-workspace', 'Works');
  await listed(() => offered(resources), '3', 'the workspaces to be read');
  await toSpace(resources, 'Works');
  await listed(groups, '[Start,Shed*]', 'a workspace called something else to have the tabs it had');

  // ---- what a browser kept from before is not shown to a workspace that has a set of its own
  await toSpace(resources, 'Front');
  await listed(groups, FRONT_LAST, 'the first workspace\'s tabs');
  await page.evaluate((kept) => localStorage.setItem('client.tabs', JSON.stringify(kept)), BEFORE);
  await again();
  await listed(groups, FRONT_LAST, 'the workspace\'s own set, and not what a browser kept from before');
  expect(`what a browser kept from before to be taken out of where it was kept, got ${await keptBefore()}`, (await keptBefore()) === null);
  await toSpace(resources, 'Spare');
  await listed(groups, '[Start*]', 'and no other workspace to be shown it');

  // ---- at a phone's width
  await toSpace(resources, 'Front');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload();
  // One row of tabs: every tab of the set, with the one showing in the group last used.
  const one = () => page.locator('.tabs .tab').evaluateAll((all) => all.map((tab) => `${tab.querySelector('.truncate').textContent}${tab.getAttribute('aria-selected') === 'true' ? '*' : ''}`).join(','));
  await listed(one, 'Start,Shed,Shop,shop-talk,std/board*', 'the first workspace\'s tabs in one row at a phone\'s width');
  await narrow();
  await page.getByRole('button', { name: 'Expand the side panel' }).click();
  await sideSettled(page);
  await page.locator('.sidebar').waitFor();
  await side('Resources');
  await toSpace(resources, 'Works');
  await listed(one, 'Start,Shed*', 'another workspace\'s tabs at a phone\'s width');
  await narrow();
  await page.screenshot({ path: `${out}/tabbed-08-another-workspace-at-390.png` });
  await toSpace(resources, 'Front');
  await listed(one, 'Start,Shed,Shop,shop-talk,std/board*', 'the first workspace\'s tabs again at a phone\'s width');
  // Something chosen in the panel takes the panel from over the page: the tabs are under it.
  await tree('shop').locator('.nav-item').click();
  await page.locator('.sidebar').waitFor({ state: 'detached' });
  await listed(one, 'Start,Shed,Shop*,shop-talk,std/board', 'a tab shown at a phone\'s width');
  await narrow();
  await page.screenshot({ path: `${out}/tabbed-09-the-first-at-390.png` });
  await page.setViewportSize({ width: 1350, height: 860 });
  await listed(groups, '[Start,Shed,Shop*] | shop-talk,std/board*', 'the groups to be as they were on a wider screen');

  // ---- signing out forgets every set, as it forgot the one
  await side('Resources');
  await foot(resources).click();
  await opened();
  await spaceMenu.getByRole('menuitem', { name: 'Log out', exact: true }).click();
  await page.getByLabel('Email').waitFor();
  expect(`no set of tabs to be kept for somebody who signed out, got "${await kept('client.tabs')}"`, (await kept('client.tabs')) === '');
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await side('Resources');
  await listed(() => showing(resources), 'Front', 'the workspace that was chosen, which signing out does not forget');
  await listed(groups, '[Start*]', 'the start page alone for somebody who signed out and in again');

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
