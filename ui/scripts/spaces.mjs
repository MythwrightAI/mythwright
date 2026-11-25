// Workspaces: the side panel is of one at a time, chosen at its foot.
//
//   DAEMON=../target/debug/factory node scripts/spaces.mjs [out-dir]
//
// A factory always has a workspace, every project is in one, and every
// channel is a project's or a workspace's own. The
// first is made as its owner registers, called after them: the panel is
// of it from the first, its foot says so, and with no other there is
// nothing to take away. Then it is called something else and more are
// made from a terminal, with a project in each, and channels of a
// project, of a workspace and of nothing: the page follows, and both
// panels are of one workspace, said in a strip at the foot that stays
// there while the lists scroll. Its projects are listed as projects
// always were, with no row for the workspace; its channels are its own,
// under no heading, and its projects', under each project, and nothing
// else: no channel is the factory's own, listed whatever is chosen, and
// what is of another workspace is left out and counted. The dialog that
// makes a channel offers the workspace showing and never none. Each
// workspace is chosen from the foot in
// each panel, by the pointer and by the keyboard, and the other panel
// follows; the tabs that are open are each workspace's own (`tabbed.mjs`
// holds that), and what was folded, grouped and pinned is as it was; the
// choice is kept when the page is read again, and where nothing is kept
// (or what a browser kept when a project could be in no workspace) the
// panel begins at the workspace of what was showing in the tabs a
// browser kept before each workspace had its own, else the usual one,
// which is the one made first and not the first by name. Nothing offers what is in no workspace: a
// project nothing is said of is in the usual one, one is put in another
// and never taken out of all, and a project's menu has no line for it. A
// project put in another workspace leaves the list. From the foot's
// menu a workspace is made, and Settings is opened at the part for the
// one showing, where it is called something else in place (what the
// daemon refuses said there in its sentence, asked once for Enter and
// leaving the field, and following what a terminal calls it) and taken
// away by its name, asked about first: one that holds something goes
// with where that goes, the usual one offered first, what the daemon
// refuses is said in the question, and from a terminal it is refused
// until that is said; one taken away while it is showing leaves the
// panel of another. With one workspace there is nothing there that
// takes it away, and somebody who may not change workspaces is offered
// neither, and nothing new, by the menu or by Settings, and is refused by
// the daemon. The part is held by the boxes at three widths and a
// phone's. How many wait unread in a channel the panel leaves out is said
// at the foot, on a button that shows the workspace where they wait in
// one press, and beside the workspace in its menu. What a channel
// belongs to is said from its row's menu and changed, to a workspace or
// a project and never to nothing, which a terminal is refused too; and a
// channel put in a group of somebody's own is listed there, while the
// panel is of its workspace. Nothing of the foot or
// of the panel's head lies over anything else, with the panel dragged
// narrow too; the menu opens upward, on the screen, and moves nothing. A
// project that is not trusted, called something long, says so whole at
// the end of its row. Looked at 1350 wide and at a phone's width. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { mayNot, workspacePart } from './spacepart.mjs';
import { movedTo } from './movedto.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'spaces-data');
const home = join(out, 'spaces-home');
// A folder that is there already: a project made of it is not trusted, and says so in its row.
const folder = join(out, 'spaces-folder');
const LONG = 'Workshop services and stores';
const LONG_ID = 'workshop-services-and-stores';

await mkdir(out, { recursive: true });
for (const dir of [data, home, folder]) await rm(dir, { recursive: true, force: true });
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
  for (const name of ['Shop', 'Depot', 'Shed']) await cli('project', 'create', name, '--bare');
  for (const name of ['chat', 'general']) await cli('channel', 'create', name);
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
  // Asked from a terminal and refused, in words that say why.
  const refused = async (args, words) => {
    let said = null;
    try {
      await cli(...args);
    } catch (err) {
      said = `${err.stderr ?? err.message}`.trim();
    }
    expect(`\`${args.join(' ')}\` to be refused saying "${words}", got ${said === null ? 'no refusal' : `"${said}"`}`, said !== null && said.includes(words));
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

  // The projects as the side panel lists them, each directly in the list.
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const projects = () => resources.evaluate((nav) => [...nav.querySelectorAll('[data-project]')].map((tree) => tree.querySelector(':scope > .nav-row .nav-item .truncate').textContent).join(' | '));
  // The channels in no group of somebody's own, as they are gathered: those of nothing, then each branch with what is under it.
  const panel = page.getByRole('navigation', { name: 'Channels' });
  const channels = () =>
    panel.evaluate((nav) => {
      const names = (rows) => rows.map((row) => row.querySelector('.nav-item .truncate').textContent);
      const direct = (within) => [...within.querySelectorAll(':scope > .grouped-channel')];
      const branch = (space) => {
        const tree = space.querySelector(':scope > .tree');
        const inside = tree ? [...names(direct(tree)), ...[...tree.querySelectorAll(':scope > .spaced')].map(branch)] : ['…'];
        return `${space.querySelector(':scope > .space-row .space-name .truncate').textContent}[${inside.join(' ')}]`;
      };
      const loose = nav.querySelector('.grouped-loose');
      return [...names(direct(loose)), ...[...loose.querySelectorAll(':scope > .spaced')].map(branch)].join(' | ');
    });
  // Every branch of the Channels panel starts folded (`folding.mjs`): a project's is unfolded as a person would, by Expand all.
  const expandChannels = () => panel.getByRole('button', { name: 'Expand all', exact: true }).click();
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
  const more = async (within, label) => {
    const button = within.getByRole('button', { name: label, exact: true });
    await button.locator('xpath=..').hover();
    await button.click();
  };
  const narrow = async () => {
    const wide = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`the page not to scroll sideways, and it is ${wide}px too wide`, wide <= 0);
  };

  // A project that is not trusted says so at the end of its row. What says so is whole, inside its row and the
  // panel, and it is the name that gives way: by the boxes themselves.
  const whole = async (where) => {
    const row = resources.locator(`[data-project="${LONG_ID}"] > .nav-row`);
    await row.locator('.not-trusted').waitFor();
    const got = await row.evaluate((line) => {
      const box = (one) => {
        const at = one.getBoundingClientRect();
        return { left: Math.round(at.left * 10) / 10, right: Math.round(at.right * 10) / 10, width: Math.round(at.width * 10) / 10 };
      };
      const mark = line.querySelector('.not-trusted');
      const name = line.querySelector('.nav-item .truncate');
      const more = line.querySelector('.row-more');
      return {
        panel: box(line.closest('.nav')),
        row: box(line),
        mark: box(mark),
        says: mark.textContent.trim(),
        cut: mark.scrollWidth > mark.clientWidth,
        name: box(name),
        gaveWay: name.scrollWidth > name.clientWidth,
        more: more ? box(more) : null,
      };
    });
    const told = JSON.stringify(got);
    expect(`what says a project is not trusted to say so whole ${where}: ${told}`, got.says === 'not trusted' && !got.cut && got.mark.width > 20);
    expect(`it to be inside its row and the panel ${where}: ${told}`, got.mark.right <= got.row.right + 0.5 && got.row.right <= got.panel.right + 0.5 && got.mark.left >= got.name.right - 0.5);
    expect(`the row's menu to be inside the row ${where}: ${told}`, !got.more || (got.more.right <= got.row.right + 0.5 && got.more.left >= got.mark.right - 0.5));
    expect(`the long name to be what gives way ${where}: ${told}`, got.gaveWay && got.name.width > 20);
  };

  // ---- the foot of a side panel: which workspace it is of, and its menu
  const foot = (nav) => nav.locator('.space-foot .space-pick');
  const showing = (nav) => nav.locator('.space-foot .space-said strong').innerText();
  const spaceMenu = page.getByRole('menu', { name: 'Workspace', exact: true });
  // Opened, it has the keyboard, which is what says it is listening: a key pressed sooner is heard by nothing.
  const opened = async () => {
    await spaceMenu.waitFor();
    await page.waitForFunction(() => document.activeElement?.closest('.row-menu') !== null && document.activeElement?.closest('.row-menu') !== undefined);
  };
  const openSpaces = async (nav) => {
    await foot(nav).click();
    await opened();
  };
  const toSpace = async (nav, name) => {
    await openSpaces(nav);
    await spaceMenu.getByRole('menuitemradio', { name: new RegExp(`^${name}(:|$)`) }).click();
    await spaceMenu.waitFor({ state: 'detached' });
    await listed(() => showing(nav), name, `the foot to say ${name}`);
  };
  const footDoes = async (nav, label) => {
    await openSpaces(nav);
    await spaceMenu.getByRole('menuitem', { name: label, exact: true }).click();
    await spaceMenu.waitFor({ state: 'detached' });
  };
  // What the menu offers: each choice, the one showing marked, with how many wait unread in it; then what is done with a workspace.
  const offers = async (nav) => {
    await openSpaces(nav);
    const choices = await spaceMenu.getByRole('menuitemradio').evaluateAll((all) =>
      all.map((one) => `${one.querySelector('.truncate').textContent}${one.getAttribute('aria-checked') === 'true' ? '*' : ''}${one.querySelector('.waits') ? `(${one.querySelector('.waits').textContent})` : ''}`),
    );
    const does = (await spaceMenu.getByRole('menuitem').allInnerTexts()).map((line) => line.trim());
    await page.keyboard.press('Escape');
    await spaceMenu.waitFor({ state: 'detached' });
    return `${choices.join(' | ')} || ${does.join(' | ')}`;
  };
  // What the menu does beside choosing: a new one, and at its foot Settings and Log out. Calling the one showing
  // something else and taking it away are in Settings, whatever it is called and however many there are.
  const done = () => ['New workspace…', 'Settings', 'Log out'].join(' | ');
  const { part, removal, named, settingsOf, closeSettings, partBoxes, atTheFoot, typedName, questionBoxes } = workspacePart({ page, expect, listed, footDoes });
  const moves = movedTo({ browser, page, url, out, expect, listed, problems });
  // What is asked of the daemon to call a workspace something else, as it is asked.
  const renames = [];
  page.on('request', (request) => {
    if (request.method() === 'PATCH' && /\/api\/workspaces\/[^/]+$/.test(new URL(request.url()).pathname)) renames.push(JSON.parse(request.postData() ?? '{}').name);
  });
  // What this browser keeps of the choice, under who is signed in.
  const keptChoice = () =>
    page.evaluate(() =>
      Object.entries(localStorage)
        .filter(([key]) => key.startsWith('client.workspace.held:user:'))
        .map(([, value]) => value)
        .join(' '),
    );
  const kept = (id) => `{"kind":"space","id":"${id}"}`;
  // The workspace made with the factory's owner as it first started: the usual one, by the id it was made with (`unnamed-workspace`), whatever it is called since.
  const USUAL = 'unnamed-workspace';
  // A side panel with the strip at its foot. The strip is at the panel's foot and on the screen, the list ends
  // above it, nothing of the strip (the workspace and Help beside it, and what the workspace's button holds) or of
  // the panel's head lies over anything else of it, all of it is inside the panel, and nothing scrolls sideways:
  // by the boxes themselves.
  const strip = async (name, where) => {
    const nav = page.getByRole('navigation', { name });
    await foot(nav).waitFor();
    const got = await nav.evaluate((bar) => {
      const round = (figure) => Math.round(figure * 10) / 10;
      const box = (one) => {
        const at = one.getBoundingClientRect();
        return { left: round(at.left), right: round(at.right), top: round(at.top), bottom: round(at.bottom) };
      };
      // What of something is drawn: a thing with no size (what only pushes two others apart) is nothing to lie over.
      const drawn = (within) =>
        [...within.children]
          .map(box)
          .filter((at) => at.right - at.left > 0.5 && at.bottom - at.top > 0.5)
          .sort((one, other) => one.left - other.left);
      const pick = bar.querySelector('.space-foot .space-pick');
      const list = bar.querySelector('.nav');
      return {
        screen: { width: window.innerWidth, height: window.innerHeight },
        bar: box(bar),
        // The head's title and each thing pressed at its head, which on a phone may stand on a line under the title.
        brand: drawn({ children: [...bar.querySelectorAll('.brand > :not(.brand-acts), .brand > .brand-acts > *')] }),
        list: box(list),
        strip: box(bar.querySelector('.space-foot')),
        foot: drawn(bar.querySelector('.space-foot')),
        // How many wait out of sight is a button of its own between the workspace and Help, while any do.
        waits: bar.querySelectorAll('.space-foot > .space-out-press').length,
        pick: box(pick),
        parts: drawn(pick),
        name: box(pick.querySelector('strong')),
        sideways: Math.max(bar.scrollWidth - bar.clientWidth, list.scrollWidth - list.clientWidth, pick.scrollWidth - pick.clientWidth),
      };
    });
    const told = JSON.stringify(got);
    const apart = (line) => line.every((one, at) => at === 0 || line[at - 1].right <= one.left + 0.5);
    // Of what may stand on two lines: no two lie over each other.
    const clear = (all) => all.every((one, at) => all.slice(at + 1).every((two) => Math.min(one.right, two.right) - Math.max(one.left, two.left) <= 0.5 || Math.min(one.bottom, two.bottom) - Math.max(one.top, two.top) <= 0.5));
    const inside = (line, within) => line.every((one) => one.left >= within.left - 0.5 && one.right <= within.right + 0.5);
    expect(`the strip to be at the foot of ${name}, on the screen, with the list ending above it ${where}: ${told}`, got.list.bottom <= got.strip.top + 0.5 && got.strip.bottom <= got.bar.bottom + 0.5 && got.strip.bottom <= got.screen.height + 0.5 && got.strip.bottom >= got.bar.bottom - 40);
    expect(`nothing of the strip or of the head of ${name} to lie over anything else of it ${where}: ${told}`, clear(got.brand) && apart(got.parts) && apart(got.foot) && got.foot.length === 2 + got.waits && got.waits <= 1 && got.brand.every((one) => one.bottom <= got.list.top + 0.5));
    expect(`all of the strip and of the head of ${name} to be inside the panel ${where}: ${told}`, inside(got.brand, got.bar) && inside(got.parts, got.pick) && inside(got.foot, got.strip) && inside([got.pick, got.strip], got.bar));
    expect(`the workspace's name to have room to be read in ${where}: ${told}`, got.name.right - got.name.left >= 40);
    expect(`nothing of ${name} to scroll sideways ${where}: ${told}`, got.sideways <= 0);
    await narrow();
    return got;
  };
  // The menu, opened from the strip: it lies above the strip and on the screen, the lists and the strip are where
  // they were, and it is there still when the pointer has crossed from the strip to it.
  const overhead = async (name, where, picture) => {
    const nav = page.getByRole('navigation', { name });
    const places = () =>
      nav.evaluate((bar) => {
        const at = (one) => {
          const box = one.getBoundingClientRect();
          return [box.left, box.top, box.width, box.height].map((figure) => Math.round(figure * 10) / 10).join(',');
        };
        return `${at(bar.querySelector('.nav'))} ${at(bar.querySelector('.space-foot'))} ${bar.querySelector('.nav').scrollTop}`;
      });
    const before = await places();
    await openSpaces(nav);
    const [menu, pick, screen] = [await spaceMenu.boundingBox(), await foot(nav).boundingBox(), page.viewportSize()];
    const told = JSON.stringify({ menu, pick, screen });
    expect(`the menu to open upward from the strip of ${name} ${where}: ${told}`, menu.y + menu.height <= pick.y + 0.5);
    expect(`the menu to be on the screen ${where}: ${told}`, menu.x >= -0.5 && menu.y >= -0.5 && menu.x + menu.width <= screen.width + 0.5 && menu.y + menu.height <= screen.height + 0.5);
    expect(`the menu to push nothing about ${where}: "${before}" and then "${await places()}"`, (await places()) === before);
    expect('the strip to say that its menu is open', (await foot(nav).getAttribute('aria-expanded')) === 'true');
    // The pointer crosses from the strip to the menu, over what is between them.
    await page.mouse.move(pick.x + pick.width / 2, pick.y + pick.height / 2);
    await page.mouse.move(menu.x + menu.width / 2, menu.y + menu.height - 12, { steps: 8 });
    await page.mouse.move(menu.x + menu.width / 2, menu.y + 12, { steps: 8 });
    expect(`the menu to be there still when the pointer has crossed to it ${where}`, await spaceMenu.isVisible());
    if (picture) await page.screenshot({ path: `${out}/${picture}` });
    await page.keyboard.press('Escape');
    await spaceMenu.waitFor({ state: 'detached' });
    expect('the strip to say that its menu is put away', (await foot(nav).getAttribute('aria-expanded')) === 'false');
  };
  const tabs = async () => (await page.locator('.tabs .tab .truncate').allInnerTexts()).join(' | ');

  // ---- with its owner's workspace alone: every project is in it, and the foot says it
  await side('Resources');
  await listed(projects, 'Shop | Depot | Shed', 'the projects of the one workspace there is');
  expect('no branch to be drawn over the projects', (await resources.locator('.spaced').count()) === 0);
  await listed(() => showing(resources), 'Ada\'s workspace', 'the foot to say the workspace made for the factory\'s owner');
  await listed(keptChoice, kept(USUAL), 'what the panel is of to be kept as soon as it is known');
  await listed(() => offers(resources), `Ada's workspace* || ${done()}`, 'the foot to offer the one workspace, a new one, Settings and Log out');
  expect('nothing anywhere to say no workspace', !/no workspace/i.test(await page.locator('.sidebar').innerText()));
  // With no other there, nothing takes it away: a factory always has one. Settings says so, and offers nothing that would.
  await settingsOf(resources, 'Ada\'s workspace');
  expect('nothing in Settings to take the last workspace away', (await page.getByRole('button', { name: /^Remove/ }).count()) === 0 && (await removal.count()) === 0);
  const last = (await part.innerText()).replace(/\s+/g, ' ');
  expect(`Settings to say that the last workspace stays, got "${last}"`, last.includes('The last workspace stays'));
  await partBoxes('with the owner\'s workspace alone');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-00b-settings-of-the-last-workspace.png` });
  await closeSettings();
  await strip('Resources', 'with the owner\'s workspace alone');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-00-projects-of-the-owners-workspace.png` });
  await side('Channels');
  await listed(channels, 'chat | general', 'the channels made with nothing said, which are the workspace\'s own');
  expect('nothing to be drawn over the workspace\'s own channels', (await panel.locator('.spaced').count()) === 0);
  await listed(() => showing(panel), 'Ada\'s workspace', 'the foot of the channels to say the same');
  await page.screenshot({ path: `${out}/spaces-01-channels-of-the-owners-workspace.png` });

  // ---- called something else and more made from a terminal while the page is open: the page follows
  // Called what comes last by name, it is the usual one still: where a project goes when nobody says.
  await cli('workspace', 'rename', USUAL, 'Works');
  await listed(() => showing(panel), 'Works', 'the foot to say what the workspace is called now');
  // What the panel is of is kept: one made later that comes before it by name does not take its place.
  await cli('workspace', 'create', 'Shop', 'front');
  await cli('workspace', 'create', 'Annex');
  await cli('workspace', 'put', 'shop-front', 'shop');
  await cli('workspace', 'put', 'annex', 'depot');
  await cli('channel', 'create', 'shop-talk', '--project', 'shop');
  await cli('channel', 'create', 'plans', '--workspace', 'shop-front');
  await cli('channel', 'create', 'yard', '--project', 'shed');
  await cli('channel', 'create', 'depot-talk', '--project', 'depot');
  await listed(channels, 'chat | general | Shed[…]', 'the channels of the workspace the panel is of: its own, and those of the project nobody put anywhere else, folded');
  await expandChannels();
  await listed(channels, 'chat | general | Shed[yard]', 'the channels of the workspace the panel is of: its own, and those of the project nobody put anywhere else');
  await listed(() => showing(panel), 'Works', 'the panel to be of the workspace it was of');
  await listed(keptChoice, kept(USUAL), 'the choice to be kept by the id the workspace always had');
  await toSpace(panel, 'Shop front');
  await listed(channels, 'plans | Shop[…]', 'the channels of the workspace: its own and its projects\', folded, and none of another\'s');
  await expandChannels();
  await listed(channels, 'plans | Shop[shop-talk]', 'the channels of the workspace: its own and its projects\', and none of another\'s');
  // A channel made from the panel is offered the workspace showing, chosen, and never none.
  await panel.getByRole('button', { name: 'New channel', exact: true }).click();
  const making = page.getByRole('dialog');
  await making.getByText('New channel', { exact: true }).waitFor();
  const makingIn = making.getByLabel(/^Workspace/);
  const offeredFor = (await makingIn.locator('option').allInnerTexts()).join(' | ');
  expect(`a new channel to be offered each workspace, the one showing chosen, and never none, got "${offeredFor}" with "${await makingIn.inputValue()}" chosen`, offeredFor === 'Annex | Shop front | Works' && (await makingIn.inputValue()) === 'shop-front');
  await page.screenshot({ path: `${out}/spaces-02b-a-channel-made-in-the-one-showing.png` });
  await making.getByLabel('Name').fill('tills');
  await making.getByRole('button', { name: 'Create', exact: true }).click();
  await making.waitFor({ state: 'detached' });
  await listed(channels, 'plans | tills | Shop[shop-talk]', 'a channel made from the panel, the own of the workspace showing');
  await cli('channel', 'remove', 'tills');
  await listed(channels, 'plans | Shop[shop-talk]', 'the channels of the workspace again');
  await listed(() => showing(panel), 'Shop front', 'the panel to be of the workspace it was of');
  await strip('Channels', 'with a workspace');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-02-channels-of-one-workspace.png` });
  await side('Resources');
  await listed(projects, 'Shop', 'the projects of the workspace, and no others');
  expect('no row for the workspace over its projects, and no branch', (await resources.locator('.spaced').count()) === 0 && (await resources.getByRole('button', { name: 'Fold Shop front' }).count()) === 0);
  expect(`the other panel to be of the same workspace, got "${await showing(resources)}"`, (await showing(resources)) === 'Shop front');
  await listed(() => offers(resources), `Annex | Shop front* | Works || ${done()}`, 'the foot to offer each workspace and what is done with one, and nothing of what is in none');
  await strip('Resources', 'with a workspace');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-03-projects-of-one-workspace.png` });
  await overhead('Resources', 'at the usual width', 'spaces-04-the-foot-opened.png');
  // The strip stays where it is while the list scrolls above it. The window is short enough for the list
  // to scroll: nobody said a folder for these projects, so no Repo is listed under them.
  await page.setViewportSize({ width: 1350, height: 280 });
  const scrolls = () => resources.locator('.nav').evaluate((list) => [list.scrollHeight - list.clientHeight, list.scrollTop].join(','));
  expect(`the list to be longer than the room it has, got ${await scrolls()}`, Number((await scrolls()).split(',')[0]) > 20);
  const low = await strip('Resources', 'in a short window');
  await resources.locator('.nav').evaluate((list) => (list.scrollTop = list.scrollHeight));
  const lower = await strip('Resources', 'with the list scrolled to its end');
  expect(`the list to have scrolled, got ${await scrolls()}`, Number((await scrolls()).split(',')[1]) > 20);
  expect(`the strip to stay where it was while the list scrolled: ${JSON.stringify(low.strip)} and then ${JSON.stringify(lower.strip)}`, JSON.stringify(low.strip) === JSON.stringify(lower.strip));
  await page.setViewportSize({ width: 1350, height: 860 });

  // ---- another chosen from the foot: the other panel follows, the tabs are that workspace's own, and nothing else changes
  const tree = (id) => resources.locator(`[data-project="${id}"] > .nav-row`);
  await tree('shop').locator('.nav-item').click();
  const open = await tabs();
  expect(`a tab of a project to be open, got "${open}"`, open === 'Start | Shop');
  await toSpace(resources, 'Annex');
  await listed(projects, 'Depot', 'the projects of the workspace chosen');
  // Each workspace has its own open tabs: this one has had none opened in it.
  await listed(tabs, 'Start', 'the tabs of the workspace chosen, in which nothing was opened');
  await listed(keptChoice, kept('annex'), 'the choice to be kept by the workspace\'s id');
  await side('Channels');
  expect(`the channels' panel to be of the same workspace, got "${await showing(panel)}"`, (await showing(panel)) === 'Annex');
  await expandChannels();
  await listed(channels, 'Depot[depot-talk]', 'the channels of the workspace chosen, and no others');
  await strip('Channels', 'with another workspace chosen');
  // What somebody folded is as they left it when the workspace is shown again.
  await panel.getByRole('button', { name: 'Fold Depot', exact: true }).click();
  await listed(channels, 'Depot[…]', 'a project\'s channels folded');
  await toSpace(panel, 'Shop front');
  await listed(channels, 'plans | Shop[shop-talk]', 'the channels of the first workspace again');
  await listed(tabs, open, 'the tabs that were open in the first workspace');
  await toSpace(panel, 'Annex');
  await listed(channels, 'Depot[…]', 'what was folded to be folded still');
  await panel.getByRole('button', { name: 'Unfold Depot', exact: true }).click();
  await listed(channels, 'Depot[depot-talk]', 'the project\'s channels unfolded');

  // By the keyboard: the strip is a button that says what it is, and its menu is gone through by the arrows.
  const strap = panel.getByRole('button', { name: 'Workspace: Annex', exact: true });
  await strap.focus();
  await page.keyboard.press('Enter');
  await opened();
  const focused = () => page.evaluate(() => `${document.activeElement?.getAttribute('role')}:${document.activeElement?.getAttribute('aria-label') ?? document.activeElement?.querySelector('.truncate')?.textContent}`);
  expect(`the menu to begin at the workspace showing, got ${await focused()}`, (await focused()) === 'menuitemradio:Annex');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go to the next, got ${await focused()}`, (await focused()) === 'menuitemradio:Shop front');
  await page.keyboard.press('Enter');
  await spaceMenu.waitFor({ state: 'detached' });
  await listed(() => showing(panel), 'Shop front', 'the workspace the keyboard chose');
  await listed(channels, 'plans | Shop[shop-talk]', 'the channels of the workspace the keyboard chose');
  expect(`the strip to have the keyboard again, got ${await focused()}`, (await focused()) === 'null:Workspace: Shop front');
  await page.keyboard.press('Enter');
  await opened();
  await page.keyboard.press('ArrowUp');
  expect(`the arrow to go back, got ${await focused()}`, (await focused()) === 'menuitemradio:Annex');
  await page.keyboard.press('End');
  expect(`End to go to the last thing offered, got ${await focused()}`, (await focused()) === 'menuitem:Log out');
  // What only says something (the workspace and who is signed in, at the head) is passed by: round from the last,
  // the next thing to press is the first workspace.
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go round and pass by what is not pressed, got ${await focused()}`, (await focused()) === 'menuitemradio:Annex');
  await page.keyboard.press('End');
  await page.keyboard.press('Home');
  expect(`Home to go to the first thing offered, got ${await focused()}`, (await focused()) === 'menuitemradio:Annex');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  expect(`the arrows to reach the last workspace, got ${await focused()}`, (await focused()) === 'menuitemradio:Works');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('Enter');
  await spaceMenu.waitFor({ state: 'detached' });
  await listed(() => showing(panel), 'Annex', 'the workspace the keyboard chose next');
  // Escape puts it away and chooses nothing; and so does pressing the strip again.
  await page.keyboard.press('Enter');
  await opened();
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Escape');
  await spaceMenu.waitFor({ state: 'detached' });
  await foot(panel).click();
  await opened();
  await foot(panel).click();
  await spaceMenu.waitFor({ state: 'detached' });
  expect(`nothing to be chosen by leaving the menu, got "${await showing(panel)}"`, (await showing(panel)) === 'Annex');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-05-another-workspace-channels.png` });
  await overhead('Channels', 'at the usual width', 'spaces-06-the-foot-opened-channels.png');

  // ---- kept when the page is read again, whatever is showing beside the panel; and where nothing is kept
  const again = async () => {
    await page.reload();
    await page.locator('.rail').waitFor();
    await side('Channels');
    await foot(panel).waitFor();
  };
  // Something of another workspace opened here, by its address, is a tab here and does not move the panel.
  await page.evaluate(() => (location.hash = '#/project/shop'));
  await listed(tabs, open, 'a project of another workspace opened beside what is open here');
  await again();
  await listed(() => showing(panel), 'Annex', 'the choice when the page is read again, with a tab of another workspace showing');
  await listed(channels, 'Depot[depot-talk]', 'the channels of the workspace chosen, when the page is read again');
  expect(`what was open in it to be open still, got "${await tabs()}"`, (await tabs()) === open);
  const forget = () => page.evaluate(() => Object.keys(localStorage).filter((key) => key.startsWith('client.workspace.held')).forEach((key) => localStorage.removeItem(key)));
  // What a browser kept when a project could be in no workspace, and that was what somebody looked at.
  const keptNone = () => page.evaluate(() => Object.keys(localStorage).filter((key) => key.startsWith('client.workspace.held')).forEach((key) => localStorage.setItem(key, '{"kind":"loose"}')));
  // The one set of tabs a browser kept before each workspace had its own, with a project's page showing in it.
  const keptTabs = (project) =>
    page.evaluate((id) => {
      const tabs = [{ kind: 'home' }, { kind: 'project', id }];
      localStorage.setItem('client.tabs', JSON.stringify({ tabs, layout: { kind: 'group', id: 'g1', tabs: ['home', `project:${id}`], active: `project:${id}` }, focused: 'g1' }));
    }, project);
  // Nothing kept that can be chosen now: the panel begins at the workspace of the project that was showing in the
  // tabs a browser kept from before, and which tabs are then shown is that workspace's to say.
  await keptNone();
  await keptTabs('shop');
  await again();
  await listed(() => showing(panel), 'Shop front', 'the workspace of what was showing, where a browser kept the choice of what is in no workspace');
  await listed(keptChoice, kept('shop-front'), 'what the panel began at to be kept');
  expect(`what a browser kept from before to be taken out of where it was kept`, (await page.evaluate(() => localStorage.getItem('client.tabs'))) === null);
  // And where nothing was showing in such a set, or there is none, at the usual one, which is not the first by name:
  // the tabs each workspace keeps say nothing of which the panel is of.
  await forget();
  await again();
  await listed(() => showing(panel), 'Works', 'the usual workspace, where nothing was chosen and no tabs are kept from before');
  await listed(keptChoice, kept(USUAL), 'the usual one to be kept');
  await keptNone();
  await again();
  await listed(() => showing(panel), 'Works', 'the usual workspace, where a browser kept what is in none and no tabs are kept from before');

  // ---- every project is in a workspace: the one nobody put anywhere is in the usual one, and is never taken out of all
  await listed(channels, 'chat | general | Shed[yard]', 'the channels of the usual workspace: its own, which were made with nothing said, and its project\'s');
  await side('Resources');
  await listed(projects, 'Shed', 'the project nobody put anywhere, in the usual workspace');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-07-the-usual-workspace.png` });
  await cli('workspace', 'put', 'annex', 'shed');
  await listed(projects, '', 'a project put in another workspace from a terminal to leave the list');
  await resources.getByText('no project in this workspace yet').waitFor();
  expect(`the panel to be of the workspace it was of, with nothing in it, got "${await showing(resources)}"`, (await showing(resources)) === 'Works');
  await listed(() => offers(resources), `Annex | Shop front | Works* || ${done()}`, 'each workspace to be offered, one with nothing in it among them, and nothing else to choose');
  // Taken out of its workspace it would be in none: refused, and it is where it was.
  await refused(['workspace', 'take', 'shed'], 'a project is always in a workspace');
  await cli('workspace', 'put', USUAL, 'shed');
  await listed(projects, 'Shed', 'the project put back in the usual workspace');
  // One made with nothing said is in the usual one, and is listed there at once.
  await cli('project', 'create', 'Barn', '--bare');
  await listed(projects, 'Shed | Barn', 'a project made from a terminal with nothing said, in the usual workspace');
  await cli('workspace', 'put', 'annex', 'barn');
  await listed(projects, 'Shed', 'the project where it was put');

  // ---- a project put in another workspace leaves the list at once: from its row's one line, Move to… (`movedto.mjs`)
  await toSpace(resources, 'Shop front');
  await listed(projects, 'Shop', 'the projects of the workspace');
  await moves.project({ tree, more, toSpace, resources, projects, showing, offers, done, cli });
  await toSpace(resources, 'Yard works');
  await listed(projects, 'Shed', 'a workspace made for a project, with the project in it');

  // ---- in Settings, opened from the foot's menu: the workspace showing called something else in place
  await settingsOf(resources, 'Yard works');
  const about = (await part.innerText()).replace(/\s+/g, ' ');
  expect(`the part to say what the workspace holds, got "${about}"`, about.includes('It holds 1 project and 0 channels of its own'));
  expect('the part itself to offer nothing that takes it away', (await part.getByRole('button', { name: /^Remove/ }).count()) === 0);
  expect('the foot of Settings to offer taking it away by its name', (await removal.getByRole('button', { name: 'Remove Yard works…', exact: true }).count()) === 1);
  // By the boxes, at three widths: the part, and taking it away at the very foot.
  for (const [width, picture] of [[1350, 'spaces-08a-settings-of-a-workspace.png'], [1000, 'spaces-08b-settings-at-1000.png'], [820, 'spaces-08c-settings-at-820.png']]) {
    await page.setViewportSize({ width, height: 860 });
    await partBoxes(`at ${width} wide`);
    await page.mouse.move(Math.min(820, width - 20), 560);
    await page.screenshot({ path: `${out}/${picture}` });
    await atTheFoot(`at ${width} wide`);
    await removal.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${out}/${picture.replace('.png', '-foot.png')}` });
  }
  await part.scrollIntoViewIfNeeded();
  await page.setViewportSize({ width: 1350, height: 860 });
  // Called something else from a terminal while it is showing: the field follows.
  await cli('workspace', 'rename', 'yard-works', 'Yard');
  await listed(() => named.inputValue(), 'Yard', 'the field to say what a terminal called the workspace');
  await listed(() => showing(resources), 'Yard', 'the foot to say the same');
  // What the daemon refuses is said under the field, in its sentence, with what was written left as it was.
  await named.fill('—');
  await named.press('Enter');
  const refusal = part.getByRole('alert');
  await refusal.waitFor();
  const refusedWith = await refusal.innerText();
  expect(`the daemon's refusal to be said where it was asked, got "${refusedWith}"`, refusedWith.includes('a workspace needs a name with at least one letter or digit'));
  expect(`what was written to be left as it was, got "${await named.inputValue()}"`, (await named.inputValue()) === '—');
  expect(`nothing to be called anything else by what was refused, got "${await showing(resources)}"`, (await showing(resources)) === 'Yard');
  await partBoxes('with a refusal said under the name');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-08d-a-name-refused.png` });
  // Written again: Enter says it, and leaving the field after that asks nothing more.
  const before = renames.length;
  await named.fill('Outside');
  expect('the refusal to go as the name is written again', (await refusal.count()) === 0);
  await named.press('Enter');
  await listed(() => showing(resources), 'Outside', 'a workspace called something else in Settings to be the one showing still');
  await named.press('Tab');
  await listed(() => named.inputValue(), 'Outside', 'the field to say what it is called now');
  expect(`it to have been asked for once, got ${JSON.stringify(renames.slice(before))}`, JSON.stringify(renames.slice(before)) === '["Outside"]');
  await listed(projects, 'Shed', 'its projects to be listed still');
  expect(`the choice to be kept by the id it always had, got "${await keptChoice()}"`, (await keptChoice()) === kept('yard-works'));
  // Left, by the pointer to the foot, the field says it too.
  await named.fill('Outside works');
  await foot(resources).click();
  await opened();
  await listed(() => showing(resources), 'Outside works', 'a name said by leaving the field');
  await page.keyboard.press('Escape');
  await spaceMenu.waitFor({ state: 'detached' });
  await named.fill('Outside');
  await named.press('Enter');
  await listed(() => showing(resources), 'Outside', 'called back what it was');
  expect(`each name to have been asked for once, got ${JSON.stringify(renames.slice(before))}`, JSON.stringify(renames.slice(before)) === '["Outside","Outside works","Outside"]');
  await closeSettings();
  await footDoes(resources, 'New workspace…');
  await page.getByRole('dialog').getByText('New workspace', { exact: true }).waitFor();
  await page.getByRole('dialog').getByLabel('Name').fill('Spare');
  await page.getByRole('dialog').getByRole('button', { name: 'Create', exact: true }).click();
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  await listed(() => showing(resources), 'Spare', 'a workspace made from the foot to be what the panel is of');
  await listed(projects, '', 'nothing in a workspace just made');
  await resources.getByText('no project in this workspace yet').waitFor();
  await listed(keptChoice, kept('spare'), 'the workspace just made to be kept as the choice');
  // A project added while it is showing is made in it, which the dialog says; one added in it is listed.
  await resources.getByRole('button', { name: 'New project' }).click();
  const adding = page.getByRole('dialog');
  await adding.getByLabel('Name').fill('Till');
  // It asks no workspace: its head says the one showing, which is where it is made.
  const offeredIn = await adding.locator('.ask-where').innerText();
  expect(`a new project to be made in the workspace that is showing, said in its head, got "${offeredIn}"`, offeredIn === 'In Spare' && (await adding.getByLabel(/^Workspace/).count()) === 0);
  await page.screenshot({ path: `${out}/spaces-08-a-project-added-in-the-one-showing.png` });
  for (const start of await adding.getByRole('switch').all()) if (await start.isChecked()) await start.click();
  await adding.getByRole('button', { name: 'Create', exact: true }).click();
  await adding.waitFor({ state: 'detached' });
  await listed(projects, 'Till', 'a project added in the workspace showing');
  await cli('workspace', 'put', 'shop-front', 'till');
  await listed(projects, '', 'a project put in another workspace from a terminal to leave the list');
  // Taken away from Settings: asked first, saying what becomes of what is in it. Escape leaves it as it was.
  await settingsOf(resources, 'Spare');
  await removal.getByRole('button', { name: 'Remove Spare…', exact: true }).click();
  const going = page.getByRole('dialog');
  await going.getByText('Remove Spare?', { exact: true }).waitFor();
  const empty = (await going.innerText()).replace(/\s+/g, ' ');
  expect(`to be told that one with nothing in it has nothing to go anywhere, got "${empty}"`, empty.includes('It has no project and no channel of its own') && (await going.getByLabel('They go to').count()) === 0);
  // Its button is closed until its name is typed as it is written, and says why.
  await typedName(going, 'Spare', { closely: true });
  await page.screenshot({ path: `${out}/spaces-08e-its-name-typed.png` });
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await going.waitFor({ state: 'detached' });
  expect(`nothing taken away by leaving the question, got "${await showing(resources)}"`, (await showing(resources)) === 'Spare');
  // What is showing is Settings, which is no workspace's: the panel is of the usual one once the one it was of is taken away.
  await removal.getByRole('button', { name: 'Remove Spare…', exact: true }).click();
  await going.getByText('Remove Spare?', { exact: true }).waitFor();
  expect('nothing typed to be kept from the question that was left', (await going.getByLabel('Its name, typed to say it is this one').inputValue()) === '');
  // Enter in the field removes, once the name is typed.
  await typedName(going, 'Spare');
  await going.getByLabel('Its name, typed to say it is this one').press('Enter');
  await going.waitFor({ state: 'detached' });
  await listed(() => showing(resources), 'Works', 'the panel to be of the usual workspace, once the one it was of is taken away with Settings showing');
  await listed(keptChoice, kept(USUAL), 'what was kept of a workspace that is gone to be put right');
  await toSpace(resources, 'Shop front');
  await listed(projects, 'Shop | Till', 'the projects of the workspace the project went to');
  // One made later under the same name does not find itself chosen.
  await cli('workspace', 'create', 'Spare');
  await listed(() => offers(resources), `Annex | Outside | Shop front* | Spare | Works || ${done()}`, 'a workspace made again under the name of one that was showing, and not chosen by that');
  // Taken away from a terminal while the question is asked here: what the daemon refuses is said in the question,
  // in its sentence, and the question stays.
  await toSpace(resources, 'Spare');
  await settingsOf(resources, 'Spare');
  await removal.getByRole('button', { name: 'Remove Spare…', exact: true }).click();
  await going.getByText('Remove Spare?', { exact: true }).waitFor();
  await cli('workspace', 'remove', 'spare');
  await listed(() => showing(resources), 'Works', 'the panel to be of another once the one showing is taken away from a terminal');
  await (await typedName(going, 'Spare')).click();
  const gone = going.getByRole('alert');
  await gone.waitFor();
  expect(`the daemon's refusal to be said in the question, got "${await gone.innerText()}"`, (await gone.innerText()).includes('there is no workspace'));
  expect('the question to stay', await going.isVisible());
  await page.screenshot({ path: `${out}/spaces-09a-a-removal-refused.png` });
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await going.waitFor({ state: 'detached' });
  await toSpace(resources, 'Shop front');
  // One taken away from a terminal while it is showing leaves the panel of another. It holds a project, which is
  // always in a workspace: refused until it is said where that goes, and then the project is there.
  await toSpace(resources, 'Outside');
  await refused(['workspace', 'remove', 'yard-works'], 'workspace remove yard-works --to <workspace>');
  expect(`nothing taken away by what was refused, got "${await showing(resources)}"`, (await showing(resources)) === 'Outside');
  await cli('workspace', 'remove', 'yard-works', '--to', USUAL);
  // Nothing of a workspace was showing in its tabs: the panel is of the usual one.
  await listed(() => showing(resources), 'Works', 'the panel to be of another, once the workspace showing is taken away from a terminal');
  await listed(() => offers(resources), `Annex | Shop front | Works* || ${done()}`, 'the workspaces that are left');
  await toSpace(resources, 'Shop front');
  // One with something in it: what becomes of that is said, and all of it stays.
  await settingsOf(resources, 'Shop front');
  await removal.getByRole('button', { name: 'Remove Shop front…', exact: true }).click();
  await going.getByText('Remove Shop front?', { exact: true }).waitFor();
  const said = (await going.innerText()).replace(/\s+/g, ' ');
  expect(`to be told what becomes of what is in it, got "${said}"`, said.includes('Its 2 projects and 1 channel are put in another workspace, and nothing of them is removed') && !/no workspace/i.test(said));
  // Where they go is chosen there: every other workspace, the usual one first and chosen.
  const goes = going.getByLabel('They go to');
  const others = (await goes.locator('option').allInnerTexts()).join(' | ');
  expect(`the other workspaces to be offered for what is in it, the usual one first and chosen, got "${others}" with "${await goes.inputValue()}" chosen`, others === 'Works | Annex' && (await goes.inputValue()) === USUAL);
  const removing = await typedName(going, 'Shop front', { closely: true });
  await page.screenshot({ path: `${out}/spaces-09-taking-one-away.png` });
  for (const width of [1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await questionBoxes(`at ${width} wide`);
  }
  await page.setViewportSize({ width: 1350, height: 860 });
  await removing.click();
  await going.waitFor({ state: 'detached' });
  // What was showing in it is Settings, which is no workspace's: the panel is of the usual one, where its projects went.
  await listed(() => showing(resources), 'Works', 'the panel to be of the usual workspace, where its projects went');
  await listed(projects, 'Shop | Shed | Till', 'the projects of the workspace taken away to be in the one that was chosen');
  await side('Channels');
  await listed(channels, 'chat | general | plans | Shop[shop-talk] | Shed[yard]', 'its own channel to be that workspace\'s own, and its projects\' to be theirs');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-09b-channels-of-a-workspace-and-two-projects.png` });
  const all = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.map((one) => one.name);
  expect(`every channel to be there still, got ${all}`, ['chat', 'depot-talk', 'general', 'plans', 'shop-talk', 'yard'].every((name) => all.includes(name)));
  // Made again, for what follows.
  await cli('workspace', 'create', 'Shop', 'front');
  await cli('workspace', 'put', 'shop-front', 'shop');
  await cli('workspace', 'put', 'shop-front', 'till');
  await cli('channel', 'belongs', 'plans', '--workspace', 'shop-front');
  // What a menu offers is what there was when it was opened: the workspace is waited for before it is looked for there.
  await listed(() => offers(panel), `Annex | Shop front | Works* || ${done()}`, 'the workspace made again to be offered');
  await toSpace(panel, 'Shop front');
  await listed(channels, 'plans | Shop[shop-talk]', 'the channels of the workspace made again');

  // ---- what a channel belongs to, from its row's menu, by the same line (`movedto.mjs`)
  const row = (name) => panel.locator('.grouped-channel', { has: page.getByRole('button', { name: new RegExp(`^# ${name}( \\d+)?$`) }) });
  await moves.channel({ panel, row, channels, toSpace, cli, refused });
  // Two made this workspace's own from a terminal, for what follows.
  await cli('channel', 'belongs', 'chat', '--workspace', 'annex');
  await cli('channel', 'belongs', 'general', '--workspace', 'annex');
  await listed(channels, 'chat | general | Depot[depot-talk]', 'channels made the workspace\'s own from a terminal');

  // ---- somebody's own groups and pins, with the panel of one workspace
  // A group of somebody's own wins: a channel put in one is listed there and not by what it belongs to.
  await row('depot-talk').click({ button: 'right' });
  await choose('New group…');
  await page.getByLabel('Name').fill('Mine');
  await page.getByRole('button', { name: 'Create', exact: true }).click();
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  await listed(channels, 'chat | general', 'a channel in a group of one\'s own to be out of what is gathered');
  const mine = panel.locator('.grouped', { hasText: 'Mine' });
  expect('the channel to be in the group', (await mine.getByRole('button', { name: '# depot-talk', exact: true }).count()) === 1);
  // Nothing moves as a drag begins: where a channel is let go to leave its group is where it was.
  const looseAt = await panel.locator('.grouped-loose').boundingBox();
  await mine.locator('.grouped-channel .nav-row').first().dragTo(panel.locator('.grouped-loose'));
  await listed(channels, 'chat | general | Depot[depot-talk]', 'a channel let go among those in no group to be gathered again');
  expect('where channels in no group are listed not to have moved', (await panel.locator('.grouped-loose').boundingBox()).y === looseAt.y);
  // Carried by its own line, as a person carries it: with who is in it unfolded under it (Expand all unfolds every channel), the
  // middle of what it holds is no part of the line.
  await row('depot-talk').locator('.nav-row').first().dragTo(mine.locator('.group-row'));
  await listed(channels, 'chat | general', 'a channel dragged into a group');
  await row('chat').click({ button: 'right' });
  await choose('Pin to the top');
  await listed(channels, 'general', 'a channel pinned, out of what is gathered');
  expect('a group and what is pinned to be drawn', (await panel.locator('.grouped').count()) === 2);
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-11-channels-arranged.png` });
  // A group all of whose channels the panel leaves out is not drawn, and neither is what is pinned of another
  // workspace: both are as they were when the panel is of theirs again.
  await toSpace(panel, 'Shop front');
  await listed(channels, 'plans | Shop[shop-talk]', 'the channels of the other workspace');
  await listed(async () => String(await panel.locator('.grouped').count()), '0', 'the group and the pin of channels the panel leaves out not to be drawn');

  // ---- something said in a channel the panel leaves out: the foot says so, and its menu says where
  expect('nothing said to wait unread while nothing does', (await page.locator('.space-out').count()) === 0);
  const bot = /`([^`]+)`/.exec((await cli('channel', 'speaker')).stdout)?.[1];
  expect('the factory\'s bot to say what it is called', !!bot);
  await cli('post', 'depot-talk', `@${bot} help`);
  const waits = panel.locator('.space-foot .space-out');
  await waits.waitFor();
  const unread = await waits.innerText();
  const says = await foot(panel).getAttribute('aria-label');
  expect(`the foot to say how many wait unread out of sight, got "${unread}", said as "${says}"`, /^[1-9]\d*$/.test(unread) && says === `Workspace: Shop front, and ${unread} unread in ${unread === '1' ? 'a channel' : 'channels'} of another`);
  await listed(() => offers(panel), `Annex(${unread}) | Shop front* | Works || ${done()}`, 'the menu to say which workspace it is unread in');
  await strip('Channels', 'with something unread out of sight');
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-12-unread-out-of-sight.png` });
  await overhead('Channels', 'with something unread out of sight', 'spaces-13-unread-in-the-menu.png');
  // With the panel dragged as narrow as it goes, by its edge.
  const edge = page.locator('.side-edge');
  const grip = await edge.boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + 300);
  await page.mouse.down();
  await page.mouse.move(grip.x - 60, grip.y + 300, { steps: 6 });
  await page.mouse.move(60, grip.y + 300, { steps: 6 });
  await page.mouse.up();
  // As wide as it was let go at once the page has drawn it: 180 is the least it can be.
  await listed(async () => String(Math.round((await panel.boundingBox()).width)), '180', 'the side panel dragged as narrow as it goes');
  await strip('Channels', 'with the panel dragged narrow');
  await overhead('Channels', 'with the panel dragged narrow', 'spaces-14-the-panel-narrow.png');
  await side('Resources');
  await listed(() => resources.locator('.space-foot .space-out').innerText(), unread, 'the same said under Resources');
  await strip('Resources', 'with the panel dragged narrow');
  await edge.dblclick();
  await listed(async () => String(Math.round((await resources.boundingBox()).width)), '252', 'the side panel as wide as it usually is again');
  // One press on the count shows the workspace where they wait: it is no longer out of sight, and is said where
  // the channel is, in the group somebody put it in.
  const press = resources.locator('.space-foot .space-out-press');
  const pressed = await press.getAttribute('aria-label');
  expect(`the count to say where one press goes, got "${pressed}"`, pressed === `${unread} unread in ${unread === '1' ? 'a channel' : 'channels'} of another workspace: show Annex`);
  await press.click();
  await listed(() => showing(resources), 'Annex', 'the panel to be of the workspace where they wait, in one press');
  await listed(keptChoice, kept('annex'), 'the workspace shown so to be kept as the choice');
  expect('nothing said to be out of sight once it is in sight', (await page.locator('.space-out').count()) === 0);
  await side('Channels');
  const grouped = mine.locator('.grouped-channel', { has: page.getByRole('button', { name: /^# depot-talk/ }) });
  await grouped.waitFor();
  const counted = await grouped.locator('.count').innerText();
  expect(`the channel to be in its group still, saying what waits there: ${counted}, where the foot said ${unread}`, counted === unread);
  // Put back as they were for what follows.
  await grouped.click({ button: 'right' });
  await choose('No group');
  // What is pinned is a branch, folded as every branch starts, and unfolded as a person would.
  await panel.getByRole('button', { name: 'Unfold Pinned', exact: true }).click();
  await panel.locator('.grouped.pinned .grouped-channel').click({ button: 'right' });
  await choose('Unpin');
  await listed(channels, 'chat | general | Depot[depot-talk]', 'the channels in no group again');

  // ---- a project that is not trusted, called something long, in the workspace showing: its name gives way and what says so is whole
  await mkdir(folder, { recursive: true });
  await writeFile(join(folder, 'notes.txt'), 'What was here before the factory was.\n');
  await cli('project', 'create', LONG, '--path', folder, '--bare', '--workspace', 'annex');
  await side('Resources');
  await listed(projects, `Depot | Barn | ${LONG}`, 'a project made of a folder that was there, in the workspace showing');
  await whole('at the panel\'s usual width');
  // The pointer is taken off the rail, whose word for what it shows would lie over the panel's head in the picture.
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/spaces-15-not-trusted-whole.png` });

  // ---- somebody who may not change workspaces: offered nothing new by the menu, and in Settings the name alone (`spacepart.mjs`)
  await mayNot({ browser, page, url, out, expect, problems, USUAL });

  // ---- at a phone's width
  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload();
  const opener = page.getByRole('button', { name: 'Expand the side panel' });
  await opener.click();
  await sideSettled(page);
  await page.locator('.sidebar').waitFor();
  await side('Resources');
  await listed(() => showing(resources), 'Annex', 'the workspace chosen, at a phone\'s width');
  await listed(projects, `Depot | Barn | ${LONG}`, 'the projects of the workspace at a phone\'s width');
  const reach = await strip('Resources', 'at a phone\'s width');
  expect(`the strip to be tall enough for a finger at a phone's width: ${JSON.stringify(reach.pick)}`, reach.pick.bottom - reach.pick.top >= 40);
  await whole('at a phone\'s width');
  // A row's menu is drawn all the time where there is nothing to hover
  // with (`hover: none`, as a channel's always was). This browser hovers,
  // so the picture does not show them.
  await page.screenshot({ path: `${out}/spaces-16-projects-at-390.png` });
  await overhead('Resources', 'at a phone\'s width', 'spaces-17-the-foot-opened-at-390.png');
  await toSpace(resources, 'Shop front');
  await listed(projects, 'Shop | Till', 'the projects of the workspace chosen at a phone\'s width');
  await side('Channels');
  await listed(channels, 'plans | Shop[shop-talk]', 'the channels of the workspace chosen at a phone\'s width');
  await cli('post', 'yard', `@${bot} help`);
  await panel.locator('.space-foot .space-out').waitFor();
  await strip('Channels', 'at a phone\'s width');
  await page.screenshot({ path: `${out}/spaces-18-channels-at-390.png` });
  await overhead('Channels', 'at a phone\'s width', 'spaces-19-the-foot-opened-channels-at-390.png');
  // The workspace's part of Settings, opened from the menu at a phone's width.
  await settingsOf(panel, 'Shop front');
  await partBoxes('at a phone\'s width');
  await page.screenshot({ path: `${out}/spaces-20-settings-at-390.png` });
  await atTheFoot('at a phone\'s width');
  await removal.getByRole('button', { name: 'Remove Shop front…', exact: true }).click();
  await going.getByText('Remove Shop front?', { exact: true }).waitFor();
  await typedName(going, 'Shop front');
  await page.screenshot({ path: `${out}/spaces-21-taking-one-away-at-390.png` });
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await going.waitFor({ state: 'detached' });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, folder]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
