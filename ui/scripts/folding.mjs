// The side panel's folds: each part of the Resources panel under its
// heading folds as anything in its tree does, the Channels panel's
// branches start folded, and Collapse all and Expand all at each panel's
// head fold and unfold all of it (`src/folds.ts`, `views/folding.tsx`).
//
//   DAEMON=../target/debug/factory node scripts/folding.mjs [out-dir]
//
// Signed in with nothing kept, the Resources panel is as a fresh browser
// shows it: Projects, Factory and
// Blueprints unfolded, and everything under them folded, each project,
// the Members line under Factory and each family of
// blueprints; no Recents while nothing was lately opened,
// and no Pinned while nothing is pinned. Each part folds and unfolds again
// by its arrow and by the keyboard, a project too, and Recents and Pinned
// are headings that fold as the rest do, starting folded. Collapse all
// folds everything; Expand all unfolds everything that was read (the
// parts, the project, the blueprints it uses, the factory's blueprints
// and their parts) and leaves the project's files folded where they were
// never read, asking the daemon for nothing; read, they and their folders
// unfold with the rest. What was turned is kept when the page is read
// again. What is typed in the panel's box holds
// open what it finds whatever is folded, and emptied the folds are as
// they were.
//
// The Channels panel the same way: signed in with nothing kept, a
// project's channels and what is archived are each a branch, folded; a
// group made there and what is pinned are branches too, each folded when
// it is next drawn as everything is; a folded branch says how many wait
// unread under it, and the channel says it once the branch is unfolded.
// Each folds by its arrow, its name and the keyboard, with nothing moving
// under the pointer. Collapse all folds every branch and every channel,
// one unfolded by hand afterwards showing who is in it as one nobody
// touched does, and keeps the Resources panel's folds, as that panel's
// keeps these; Expand all unfolds every branch and every channel with who
// is in it, what was folded of that by hand among it. What was unfolded is kept when the page is read again, and
// what is typed in the panel's box holds open the branch it finds
// something in, and emptied the folds are as they were.
//
// The Topics panel the same way: a wiki whose page holds a page
// is a fold, as that page is, each folded as it starts and unfolded by its
// arrow; Expand all unfolds every page that holds pages and Collapse all
// folds them, kept as the panel's folds are; what is typed in its box
// holds open what a page found is under, and emptied the folds are as
// they were; and the Channels panel's folds are left as they were.
//
// The buttons are where each panel is headed, inside it, and nothing of
// either panel lies over anything else or scrolls sideways at three
// widths, with the panel dragged as narrow as it goes, and at a phone's.
// And each panel is one tree at each of them (`joined.mjs`):
// every fold's line runs down from its mark, a part's
// heading as much as a project or a folder, with a line across to each
// thing under it, every level the same step further in, and that step
// gives way at the narrowest panel before any name is cut to nothing.
// The environment variables are those of walkthrough.mjs, with `LOOK`.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { joined, namesCut } from './joined.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'folding-data');
const home = join(out, 'folding-home');
const shop = join(out, 'folding-shop');

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
await mkdir(join(shop, 'docs', 'old'), { recursive: true });
await writeFile(join(shop, 'docs', 'old', 'notes.md'), '# Notes\n');
await writeFile(join(shop, 'README.md'), '# Shop\n');
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', 'Shop', '--bare', '--path', shop);
  await cli('apply', 'std/board', '--project', 'shop');
  // The workspace's own channels, a project's, and one that is archived: each a part of the Channels panel.
  for (const name of ['general', 'design', 'old']) await cli('channel', 'create', name);
  await cli('channel', 'create', 'shop-talk', '--project', 'shop');
  await cli('channel', 'archive', 'old');
  // A wiki with a page under it and one under that: each page that holds pages a fold of the Topics panel.
  await cli('topic', 'make', 'Handbook');
  await cli('topic', 'make', 'Deploying', '--under', 'Handbook');
  await cli('topic', 'make', 'Rolling back', '--under', 'Deploying');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const until = async (read, wanted, what) => {
    let got;
    for (let tries = 0; tries < 100; tries += 1) {
      got = await read();
      if (got === wanted) return;
      await page.waitForTimeout(80);
    }
    throw new Error(`expected ${what}: "${wanted}", got "${got}"`);
  };
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const rail = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
  const panel = page.locator('nav.sidebar[aria-label="Resources"]');
  await panel.waitFor();
  // What the panel draws, a line a thing in order: a heading with + or − as it is unfolded or folded, a line by its words.
  const drawn = () =>
    panel.locator('.nav').evaluate((nav) =>
      [...nav.querySelectorAll('h3 .section-fold, .nav-item, .nav-twist')]
        .filter((one) => one.getBoundingClientRect().height > 0)
        .map((one) => {
          if (one.matches('.section-fold')) return `${one.getAttribute('aria-expanded') === 'true' ? '−' : '+'}${one.textContent.trim()}`;
          if (one.matches('.nav-twist')) return one.getAttribute('aria-expanded') === 'true' ? '−' : '+';
          const folds = one.getAttribute('aria-expanded');
          return `${folds === 'true' ? '−' : folds === 'false' ? '+' : ''}${one.querySelector('.truncate')?.textContent.trim() ?? one.textContent.trim()}`;
        })
        .join(' '),
    );
  const press = (name) => panel.getByRole('button', { name, exact: true }).click();
  // The panel's parts by their boxes, top to bottom: each heading wholly under the one before it, in the panel's order
  // (Pinned, Recents, Projects, Factory, Blueprints), whatever is unfolded between them.
  const headingsInOrder = async (wanted, where) => {
    const heads = await panel.locator('h3.nav-section').evaluateAll((all) =>
      all.map((one) => ({ name: one.querySelector('.section-fold .truncate')?.textContent.trim(), top: one.getBoundingClientRect().top, bottom: one.getBoundingClientRect().bottom })),
    );
    const names = [...heads].sort((a, b) => a.top - b.top).map((one) => one.name);
    const stacked = heads.every((one, at) => at === 0 || one.top >= heads[at - 1].bottom - 0.5);
    expect(`the panel's parts to stand ${wanted.join(', ')} from the top ${where}, got ${names.join(', ')}`, names.join() === wanted.join() && stacked);
  };
  // A row stays where it was from the moment it is pressed: read as the press is made and every frame after it for a
  // while, as one and the same element, so that a second press lands on what the first did.
  const rowStill = async (row, act, what) => {
    const one = await row.elementHandle();
    const before = await one.evaluate((it) => {
      window.foldingSeen = [];
      const began = performance.now();
      const look = () => {
        window.foldingSeen.push(it.isConnected ? it.getBoundingClientRect().toJSON() : null);
        if (performance.now() - began < 500) requestAnimationFrame(look);
      };
      requestAnimationFrame(look);
      return it.getBoundingClientRect().toJSON();
    });
    await act();
    await page.waitForTimeout(600);
    const seen = await page.evaluate(() => window.foldingSeen);
    const moved = seen.map((at) => (at ? Math.max(Math.abs(at.x - before.x), Math.abs(at.y - before.y), Math.abs(at.width - before.width), Math.abs(at.height - before.height)) : Infinity));
    expect(`the row not to move as ${what}, over ${seen.length} frames it moved by ${Math.max(...moved)}`, seen.length > 5 && moved.every((by) => by <= 0.5));
  };
  // Pointed at, a row's ⋯ says nothing of itself: no tip lies over the panel. It is still called so for whoever hears
  // it read out. First a control that does say something is pointed at (Collapse all), so that tips are seen to be
  // said on this page at all, and the ⋯ is then rested on for longer than that took.
  const untipped = async (within, name, what) => {
    const said = page.locator('.tip-said');
    const begun = Date.now();
    await within.getByRole('button', { name: 'Collapse all', exact: true }).hover();
    await said.waitFor();
    const took = Date.now() - begun;
    const more = within.getByRole('button', { name, exact: true });
    await more.hover();
    await said.waitFor({ state: 'detached' });
    await page.waitForTimeout(Math.max(800, took * 2));
    expect(`pointing at the ⋯ of ${what} to show no tip, got "${(await said.allInnerTexts()).join(' ')}"`, (await said.count()) === 0);
    expect(`the ⋯ of ${what} to be called ${name} for whoever hears it read out and to carry no title`, (await more.count()) === 1 && (await more.getAttribute('title')) === null);
    await page.mouse.move(900, 700);
  };
  const fileReads = [];
  page.on('request', (request) => /\/(tree|scan)$/.test(new URL(request.url()).pathname) && fileReads.push(request.url()));

  // ---- with nothing kept: three parts unfolded, and everything under them folded
  const fresh = '−Projects + Shop −Factory Activity Measurements + Members Trash −Blueprints Reference +std';
  await until(drawn, fresh, 'Projects, Factory and Blueprints unfolded, Shop and std folded under them, and no Recents with nothing lately opened');
  await headingsInOrder(['Projects', 'Factory', 'Blueprints'], 'with nothing kept');
  await page.screenshot({ path: `${out}/folding-0-fresh.png` });
  // Each heading folds and unfolds by its arrow, and by the keyboard.
  await press('Fold Factory');
  await until(drawn, '−Projects + Shop +Factory −Blueprints Reference +std', 'Factory folded by its press');
  await press('Unfold Factory');
  await until(drawn, fresh, 'Factory unfolded again by its press');
  await panel.getByRole('button', { name: 'Fold Blueprints', exact: true }).focus();
  await page.keyboard.press('Enter');
  await until(drawn, '−Projects + Shop −Factory Activity Measurements + Members Trash +Blueprints', 'Blueprints folded by Enter');
  await page.keyboard.press('Enter');
  await until(drawn, fresh, 'Blueprints unfolded again by Enter');
  await panel.getByRole('button', { name: 'Fold Projects', exact: true }).focus();
  await page.keyboard.press('Space');
  await until(drawn, '+Projects −Factory Activity Measurements + Members Trash −Blueprints Reference +std', 'Projects folded by Space');
  await page.keyboard.press('Space');
  await until(drawn, fresh, 'Projects unfolded again by Space, and Shop folded in it');
  // A part kept folded stays folded when the page is read again.
  await press('Fold Factory');
  await page.reload();
  await page.locator('.rail').waitFor();
  await panel.waitFor();
  await until(drawn, '−Projects + Shop +Factory −Blueprints Reference +std', 'Factory folded as it was left, when the page is read again');
  await press('Unfold Factory');
  await until(drawn, fresh, 'Factory unfolded again');
  // A project's name opens its page and unfolds it: its row stays where it was, every frame from the press, and what it
  // has comes under it.
  const shopRow = panel.locator('[data-project="shop"] > .nav-row');
  const shopName = shopRow.locator('.nav-item');
  const unfolded = '−Projects − Shop Board +Blueprints + Repo −Factory Activity Measurements + Members Trash −Blueprints Reference +std';
  await rowStill(shopRow, () => shopName.click(), "Shop's name is pressed");
  await page.locator('.view-head h1', { hasText: /^Shop$/ }).waitFor();
  await until(drawn, unfolded, 'Shop unfolded by its name, its page shown, its files folded');
  await page.screenshot({ path: `${out}/folding-8-project-by-name.png` });
  // Pressed again it stays unfolded: its arrow is the way to fold it.
  await rowStill(shopRow, () => shopName.click(), "Shop's name is pressed again");
  await until(drawn, unfolded, 'Shop still unfolded, pressed again by its name');
  await press('Fold Shop');
  await until(drawn, '−Projects + Shop −Factory Activity Measurements + Members Trash −Blueprints Reference +std', 'Shop folded by its arrow');
  // By Enter on its name too, and kept when the page is read again.
  await shopName.focus();
  await page.keyboard.press('Enter');
  await until(drawn, unfolded, 'Shop unfolded by Enter on its name');
  // Its page has the daemon look at its folder, which this page then knows it may read; closed and read again, the
  // page has looked at nothing, as Expand all below asks.
  await page.getByRole('button', { name: 'Close Shop', exact: true }).click();
  await page.reload();
  await page.locator('.rail').waitFor();
  await panel.waitFor();
  await until(drawn, `+Recents ${unfolded}`, 'Shop unfolded as it was left, when the page is read again');
  await headingsInOrder(['Recents', 'Projects', 'Factory', 'Blueprints'], 'with a project unfolded');
  // Pointed at only now: the pointer leaving the panel puts Recents right, which the presses above hold still.
  await untipped(panel, 'More for Shop', "a project's row");
  // Something opened is under Recents, folded as the rest; pinned, under Pinned, folded.
  await panel.locator('[data-project="shop"] .tree .nav-item', { has: page.locator('.truncate', { hasText: /^Board$/ }) }).click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'Shop board' }).waitFor();
  await page.mouse.move(900, 700);
  await panel.getByRole('button', { name: 'Unfold Recents', exact: true }).waitFor();
  await press('Unfold Recents');
  await untipped(panel, 'More for Shop board', 'a line of Recents');
  await panel.locator('.lately-recent .lately-line', { has: page.locator('.truncate', { hasText: /^Shop board$/ }) }).hover();
  await panel.getByRole('button', { name: 'More for Shop board', exact: true }).click();
  await page.getByRole('menu').getByText('Pin to the top', { exact: true }).click();
  // Something pinned and something lately opened at once: all five parts, by their boxes, in the panel's order.
  await page.mouse.move(900, 700);
  await panel.getByRole('button', { name: 'Unfold Pinned', exact: true }).waitFor();
  await headingsInOrder(['Pinned', 'Recents', 'Projects', 'Factory', 'Blueprints'], 'with something pinned and something lately opened');
  // Shop's page, opened by its name above, is taken out of Recents, so that Recents goes.
  const shopRecent = panel.locator('.lately-recent .lately-line', { has: page.locator('.truncate', { hasText: /^Shop$/ }) });
  await shopRecent.hover();
  await shopRecent.getByRole('button', { name: 'More for Shop', exact: true }).click();
  await page.getByRole('menu').getByText('Remove from Recents', { exact: true }).click();
  await page.mouse.move(900, 700);
  await panel.getByRole('button', { name: 'Unfold Pinned', exact: true }).waitFor();
  expect('Recents to be drawn no longer, its one thing pinned', (await panel.getByRole('button', { name: 'Fold Recents', exact: true }).count()) === 0);
  await press('Unfold Pinned');
  await panel.locator('.lately-pinned .lately-line').waitFor();
  await press('Fold Factory');
  await until(drawn, '−Pinned Shop board −Projects − Shop Board +Blueprints + Repo +Factory −Blueprints Reference +std', 'Factory folded again by its press');

  // ---- Collapse all and Expand all
  await press('Collapse all');
  await until(drawn, '+Pinned +Projects +Factory +Blueprints', 'everything folded by Collapse all');
  await headingsInOrder(['Pinned', 'Projects', 'Factory', 'Blueprints'], 'with everything folded');
  await page.screenshot({ path: `${out}/folding-1-collapsed.png` });
  const readsBefore = fileReads.length;
  await press('Expand all');
  // Everything is unfolded but the project's files, whose folder was never looked at: every family of blueprints, each
  // blueprint's parts and misc among them.
  await panel.locator('.nav-item[aria-expanded="true"]', { hasText: 'misc' }).waitFor();
  const all = await drawn();
  const shut = all.split(' ').filter((one) => one.startsWith('+'));
  expect(`everything that was read unfolded by Expand all, got "${all}"`, all.startsWith('−Pinned Shop board −Projects − Shop Board −Blueprints std/board std/cards New blueprint… + Repo −Factory Activity Measurements − Members Ada Trash −Blueprints Reference −std − std/ab ') && all.includes(' − std/board std/cards ') && all.includes(' − std/fleet ') && all.includes(' −misc '));
  expect(`nothing folded but the files that were not looked at, got ${JSON.stringify(shut)}`, shut.length === 1);
  expect(`Expand all to ask the daemon for no project's files, and it asked ${fileReads.length - readsBefore} times`, fileReads.length === readsBefore);
  await page.screenshot({ path: `${out}/folding-2-expanded.png` });
  // The files read, by their own arrow: Expand all then unfolds them and their folders too.
  await press('Unfold Shop repo');
  await panel.locator('[data-project="shop"] .nav-item', { hasText: 'docs' }).waitFor();
  await press('Collapse all');
  await until(drawn, '+Pinned +Projects +Factory +Blueprints', 'everything folded again');
  await press('Expand all');
  await panel.locator('[data-project="shop"] .nav-item', { hasText: 'notes.md' }).waitFor();
  const expanded = await drawn();
  expect(`the files and their folders unfolded once they were read, got "${expanded}"`, expanded.includes('− Repo −docs −old notes.md README.md'));

  // ---- kept when the page is read again
  await press('Fold Blueprints');
  const kept = await drawn();
  await page.reload();
  await page.locator('.rail').waitFor();
  await panel.waitFor();
  await panel.locator('[data-project="shop"] .nav-item', { hasText: 'notes.md' }).waitFor();
  await until(drawn, kept, 'the folds as they were left, when the page is read again');

  // ---- the panel's box holds open what it finds, and emptied the folds are as they were
  await press('Collapse all');
  await until(drawn, '+Pinned +Projects +Factory +Blueprints', 'everything folded before typing');
  const box = panel.getByRole('searchbox').or(panel.locator('.panel-seek input')).first();
  await box.fill('notes');
  await until(drawn, '−Projects − Shop − Repo −docs −old notes.md', 'what was found held open, folded though everything was');
  await box.fill('activity');
  await until(drawn, '−Factory Activity', 'a line of the factory found, its part held open');
  await box.press('Escape');
  await until(drawn, '+Pinned +Projects +Factory +Blueprints', 'the folds as they were once the box is emptied');

  // ---- the Channels panel: every branch starts folded, and says what waits unread under it
  const channelsRail = page.getByRole('button', { name: 'Channels', exact: true });
  const channelsPanel = page.locator('nav.sidebar[aria-label="Channels"]');
  const toChannels = async () => {
    if ((await channelsRail.getAttribute('aria-pressed')) !== 'true') await channelsRail.click();
    await channelsPanel.waitFor();
  };
  const toResources = async () => {
    if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
    await panel.waitFor();
  };
  // What the Channels panel draws, in order: an arrow as + or − as what it folds is folded or not, then the line's
  // name, with how many wait unread in brackets and the few faint words of a folded part in square ones.
  const listed = () =>
    channelsPanel.locator('.nav').evaluate((nav) =>
      [...nav.querySelectorAll('.nav-twist, .nav-item')]
        .filter((one) => one.getBoundingClientRect().height > 0)
        .map((one) => {
          if (one.matches('.nav-twist')) return one.getAttribute('aria-expanded') === 'true' ? '−' : '+';
          const folds = one.getAttribute('aria-expanded');
          const name = one.querySelector('.truncate')?.textContent.trim() ?? one.textContent.trim();
          const count = one.querySelector('.count')?.textContent.trim();
          const hint = one.querySelector('.hint')?.textContent.trim();
          return `${folds === 'true' ? '−' : folds === 'false' ? '+' : ''}${name}${count ? `(${count})` : ''}${hint ? `[${hint}]` : ''}`;
        })
        .join(' '),
    );
  const inChannels = (name) => channelsPanel.getByRole('button', { name, exact: true });
  // Nothing under the pointer moves as a fold opens or closes: what was pressed, and the line it is on, stay where they were.
  // Read by the element itself, since what it says of itself (Unfold, Fold) changes with the press.
  const stays = async (button, act, what) => {
    const one = await button.elementHandle();
    const at = () => one.evaluate((pressed) => [pressed, pressed.closest('.nav-row, .brand')].map((it) => it.getBoundingClientRect().toJSON()));
    const before = await at();
    await act();
    await page.waitForTimeout(120);
    const after = await at();
    const moved = before.map((one, at) => (one && after[at] ? Math.max(Math.abs(one.x - after[at].x), Math.abs(one.y - after[at].y), Math.abs(one.width - after[at].width)) : Infinity));
    expect(`nothing under the pointer to move as ${what}, moved ${moved.join(' and ')}`, moved.every((one) => one <= 0.5));
  };
  const resourcesWere = await drawn();
  await toChannels();
  await until(listed, '+ design + general + Shop + Archived[1]', "the Channels panel's channels in no group, its project's and what is archived, each branch folded");
  // Something said under a folded branch is said on the branch, with nothing unfolded.
  const bot = /`([^`]+)`/.exec((await cli('channel', 'speaker')).stdout)?.[1];
  expect("the factory's bot to say what it is called", !!bot);
  await cli('post', 'shop-talk', `@${bot} help`);
  await channelsPanel.locator('.space-name .count').waitFor();
  let waiting = await channelsPanel.locator('.space-name .count').innerText();
  expect(`how many wait unread to be a number, got "${waiting}"`, /^[1-9]\d*$/.test(waiting));
  await until(listed, `+ design + general + Shop(${waiting}) + Archived[1]`, 'what waits under a folded branch said on the branch');
  // Unfolded by its arrow, the channel says it and the branch no longer does; folded by its name, the branch again.
  await stays(inChannels('Unfold Shop'), () => inChannels('Unfold Shop').click(), 'a branch unfolds by its arrow');
  await until(listed, `+ design + general − Shop + shop-talk(${waiting}) + Archived[1]`, 'the project unfolded, its channel saying what waits');
  await stays(inChannels('Fold Shop'), () => channelsPanel.locator('.space-name', { hasText: 'Shop' }).click(), 'a branch folds by its name');
  await until(listed, `+ design + general + Shop(${waiting}) + Archived[1]`, 'folded again by its name');
  // And by the keyboard: Enter and Space on its arrow.
  await inChannels('Unfold Archived').focus();
  await page.keyboard.press('Enter');
  await until(listed, `+ design + general + Shop(${waiting}) − Archived + old`, 'Archived unfolded by Enter');
  await inChannels('Fold Archived').focus();
  await page.keyboard.press('Space');
  await until(listed, `+ design + general + Shop(${waiting}) + Archived[1]`, 'and folded by Space');

  // A group made there is unfolded for what is put straight into it; what is pinned is a branch, folded.
  await channelsPanel.getByRole('button', { name: 'New group', exact: true }).click();
  await page.getByLabel('Name').fill('Work');
  await page.getByRole('dialog').getByRole('button', { name: 'Create', exact: true }).click();
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  const rowOf = (name) => channelsPanel.locator('.grouped-channel', { has: page.getByRole('button', { name: `# ${name}`, exact: true }) });
  const choose = async (label) => {
    const menu = page.getByRole('menu');
    await menu.waitFor();
    await menu.getByText(label, { exact: true }).click();
    await menu.waitFor({ state: 'detached' });
  };
  await rowOf('design').click({ button: 'right' });
  await choose('Work');
  await until(listed, `+ general + Shop(${waiting}) − Work + design + Archived[1]`, 'the group made unfolded, with what was put in it');
  await untipped(channelsPanel, 'More for Work', 'a group of your own');
  await untipped(channelsPanel, 'More for #design', "a channel's row");
  await rowOf('general').click({ button: 'right' });
  await choose('Pin to the top');
  await until(listed, `+ Pinned + Shop(${waiting}) − Work + design + Archived[1]`, 'what is pinned a branch of its own, folded');
  // Who is in a channel folds under it as it always did.
  await inChannels('Unfold design').click();
  await channelsPanel.getByRole('button', { name: 'People', exact: true }).waitFor();
  // Who is in it is unfolded as a channel is first unfolded, and is folded here by hand, for Expand all to unfold again.
  await inChannels('People').click();
  await until(listed, `+ Pinned + Shop(${waiting}) − Work − design +People −Agents + Archived[1]`, 'who is in a channel folded by hand');

  // ---- Collapse all and Expand all of the Channels panel
  await stays(channelsPanel.getByRole('button', { name: 'Collapse all', exact: true }), () => channelsPanel.getByRole('button', { name: 'Collapse all', exact: true }).click(), 'Collapse all is pressed');
  await until(listed, `+ Pinned + Shop(${waiting}) + Work + Archived[1]`, 'every branch folded by Collapse all');
  // And every channel: a branch unfolded by hand shows its channels folded, and one of them unfolded by hand shows
  // who is in it as a channel nobody touched does (`folds.ts`).
  await inChannels('Unfold Work').click();
  await until(listed, `+ Pinned + Shop(${waiting}) − Work + design + Archived[1]`, 'the channel under a branch folded by Collapse all');
  await inChannels('Unfold design').click();
  await until(listed, `+ Pinned + Shop(${waiting}) − Work − design −People −Agents + Archived[1]`, 'a channel unfolded after Collapse all showing who is in it, as one nobody touched');
  await inChannels('People').click();
  await inChannels('Fold Work').click();
  await until(listed, `+ Pinned + Shop(${waiting}) + Work + Archived[1]`, 'folded again, who is in design folded by hand under it');
  await page.mouse.move(900, 700);
  await page.screenshot({ path: `${out}/folding-4-channels-folded.png` });
  await stays(channelsPanel.getByRole('button', { name: 'Expand all', exact: true }), () => channelsPanel.getByRole('button', { name: 'Expand all', exact: true }).click(), 'Expand all is pressed');
  // Every branch unfolded, and every channel with who is in it, what was folded by hand of that among it.
  const allOpen = () => `− Pinned − general −People −Agents − Shop − shop-talk(${waiting}) −People −Agents −Bots − Work − design −People −Agents − Archived − old −People −Agents`;
  await until(listed, allOpen(), 'every branch and every channel unfolded by Expand all, with who is in each');
  await page.mouse.move(900, 700);
  await page.screenshot({ path: `${out}/folding-5-channels-expanded.png` });
  // Each panel's buttons are its own: the Resources panel's folds are as they were, and its Collapse all leaves these.
  await toResources();
  await until(drawn, resourcesWere, "the Resources panel's folds as they were, whatever the Channels panel's buttons did");
  await press('Collapse all');
  await until(drawn, '+Pinned +Projects +Factory +Blueprints', 'the Resources panel folded');
  await toChannels();
  await until(listed, allOpen(), "the Channels panel's folds as they were, whatever the Resources panel's buttons did");

  // ---- kept when the page is read again, and the panel's box
  await inChannels('Fold Work').click();
  const channelsKept = await listed();
  await page.reload();
  await page.locator('.rail').waitFor();
  await toChannels();
  // How many wait unread is the page's own count of what it heard said, and begins again with the page.
  const uncounted = (text) => text.replace(/\(\d+\)/g, '');
  await until(async () => uncounted(await listed()), uncounted(channelsKept), "the Channels panel's folds as they were left, when the page is read again");
  await cli('post', 'shop-talk', `@${bot} help`);
  await channelsPanel.locator('.grouped-channel .count').waitFor();
  waiting = await channelsPanel.locator('.grouped-channel .count').innerText();
  await channelsPanel.getByRole('button', { name: 'Collapse all', exact: true }).click();
  await until(listed, `+ Pinned + Shop(${waiting}) + Work + Archived[1]`, 'everything folded before typing');
  const channelsBox = channelsPanel.locator('.panel-seek input');
  for (const [words, held] of [
    ['talk', `− Shop + shop-talk(${waiting})`],
    ['design', '− Work + design'],
    ['old', '− Archived + old'],
    ['general', '− Pinned + general'],
  ]) {
    await channelsBox.fill(words);
    await until(listed, held, `"${words}" holding open the branch it found something in, folded though everything was`);
  }
  // A branch held open is not folded by its arrow, which changes nothing that is kept.
  await inChannels('Fold Pinned').click({ force: true });
  await until(listed, '− Pinned + general', 'the arrow of a branch held open folding nothing');
  await channelsBox.press('Escape');
  await until(listed, `+ Pinned + Shop(${waiting}) + Work + Archived[1]`, 'the folds as they were once the box is emptied');

  // ---- the Topics panel: each page of a wiki that holds pages a fold, folded as it starts
  const topicsPanel = page.locator('nav.sidebar[aria-label="Topics"]');
  const toTopics = async () => {
    const topicsRail = page.locator('.rail').getByRole('button', { name: 'Topics', exact: true });
    if ((await topicsRail.getAttribute('aria-pressed')) !== 'true') await topicsRail.click();
    await topicsPanel.waitFor();
  };
  const wikis = () =>
    topicsPanel.locator('.nav').evaluate((nav) =>
      [...nav.querySelectorAll('.nav-item, .nav-twist')]
        .filter((one) => one.getBoundingClientRect().height > 0)
        .map((one) => (one.matches('.nav-twist') ? (one.getAttribute('aria-expanded') === 'true' ? '−' : '+') : (one.querySelector('.truncate')?.textContent.trim() ?? '')))
        .join(' '),
    );
  await toTopics();
  await until(wikis, '+ Handbook', 'a wiki with a page under it folded as it starts');
  await topicsPanel.getByRole('button', { name: 'Unfold Handbook', exact: true }).click();
  await until(wikis, '− Handbook + Deploying', 'the wiki unfolded by its arrow, the page under it that holds a page folded');
  await topicsPanel.getByRole('button', { name: 'Expand all', exact: true }).click();
  await until(wikis, '− Handbook − Deploying Rolling back', 'every page that holds pages unfolded by Expand all');
  // Kept as the page's folds are, by what the panel names each page's: read again with the page (the page is not read
  // again here, since how many wait unread in the channels is the page's own count and begins again with it).
  const keptPages = await page.evaluate(() => JSON.parse(localStorage.getItem('client.turned') ?? '[]').filter((one) => one.startsWith('channels:topics:page:')).length);
  expect(`both pages that hold pages kept as unfolded, got ${keptPages}`, keptPages === 2);
  await topicsPanel.getByRole('button', { name: 'Collapse all', exact: true }).click();
  await until(wikis, '+ Handbook', 'every page folded by Collapse all');
  await topicsPanel.locator('.panel-seek input').fill('rolling');
  await until(wikis, '− Handbook − Deploying Rolling back', 'a page found by the box with what it is under held open, folded though it was');
  await topicsPanel.locator('.panel-seek input').press('Escape');
  await until(wikis, '+ Handbook', 'the folds as they were once the box is emptied');
  await topicsPanel.getByRole('button', { name: 'Expand all', exact: true }).click();
  await until(wikis, '− Handbook − Deploying Rolling back', 'unfolded again to be looked at');
  await toChannels();
  await until(listed, `+ Pinned + Shop(${waiting}) + Work + Archived[1]`, "the Channels panel's folds as they were, whatever the Topics panel's buttons did");

  // ---- by the boxes: the buttons inside each panel's head, and nothing over anything else
  await channelsPanel.getByRole('button', { name: 'Expand all', exact: true }).click();
  await until(listed, allOpen(), 'everything unfolded to be looked at');
  await toResources();
  await press('Expand all');
  // Read again with the page, the factory's blueprints are unfolded with the rest once they are listed.
  await panel.locator('.nav-note', { hasText: 'Reading what there is' }).waitFor({ state: 'detached' });
  await press('Expand all');
  await panel.locator('.nav-item[aria-expanded="true"]', { hasText: 'misc' }).waitFor();
  const boxes = async (where) => {
    for (const [name, nav] of [['Resources', panel], ['Channels', channelsPanel], ['Topics', topicsPanel]]) {
      if (name === 'Resources') await toResources();
      else if (name === 'Topics') await toTopics();
      else await toChannels();
      const head = await nav.locator('.brand').evaluate((brand) => {
        const frame = brand.getBoundingClientRect();
        return [...brand.querySelectorAll('button')].map((one) => {
          const at = one.getBoundingClientRect();
          return { name: one.getAttribute('aria-label'), inside: at.left >= frame.left - 0.5 && at.right <= frame.right + 0.5 && at.width > 0 };
        });
      });
      expect(`Collapse all and Expand all in the ${name} panel's head, inside it ${where}: ${JSON.stringify(head)}`, ['Collapse all', 'Expand all'].every((one) => head.some((button) => button.name === one && button.inside)));
      const crowded = await pageCrowded(page, `nav.sidebar[aria-label="${name}"]`);
      expect(`nothing of the ${name} panel over anything else ${where}: ${crowded.join('; ')}`, crowded.length === 0);
      // One tree: every fold's line from its mark, a heading's as much as a project's, a line across to each thing under it, the same step at every level.
      const apart = await joined(page, `nav.sidebar[aria-label="${name}"]`);
      expect(`the ${name} panel to be one tree ${where}: ${apart.join('; ')}`, apart.length === 0);
      const cut = await namesCut(page, `nav.sidebar[aria-label="${name}"]`);
      expect(`no name of the ${name} panel cut to nothing ${where}: ${cut.join('; ')}`, cut.length === 0);
    }
  };
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await boxes(`at ${width} wide`);
  }
  // Dragged as narrow as it goes, what each panel is called gives way before anything that is pressed leaves it.
  await page.setViewportSize({ width: 1280, height: 860 });
  const edge = page.locator('.side-edge');
  const grip = await edge.boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + 300);
  await page.mouse.down();
  await page.mouse.move(60, grip.y + 300, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 180);
  await page.screenshot({ path: `${out}/folding-6-narrow.png` });
  await boxes('with the panel as narrow as it goes');
  await toResources();
  await page.screenshot({ path: `${out}/folding-6-narrow-resources.png` });
  await page.setViewportSize({ width: 390, height: 800 });
  // On a phone the side panel is put away while something is shown, and brought back by its button, with the rail.
  // It is put away a moment after the window is made narrow: asked straight away whether it shows, it still did,
  // was not brought back, and was gone a moment later (four times in six at the largest size and the most compact density).
  await page.getByRole('button', { name: 'Expand the side panel' }).click();
  await sideSettled(page);
  await toResources();
  await boxes("at a phone's width");
  await toResources();
  await press('Collapse all');
  await until(drawn, '+Pinned +Projects +Factory +Blueprints', "everything folded by a touch at a phone's width");
  await page.screenshot({ path: `${out}/folding-3-phone.png` });
  await toChannels();
  await channelsPanel.getByRole('button', { name: 'Collapse all', exact: true }).click();
  await until(listed, `+ Pinned + Shop(${waiting}) + Work + Archived[1]`, "every branch of the channels folded by a touch at a phone's width");
  await channelsPanel.getByRole('button', { name: 'Expand all', exact: true }).click();
  await until(listed, allOpen(), "and unfolded by a touch, every channel with who is in it");
  await page.screenshot({ path: `${out}/folding-7-channels-phone.png` });

  // ---- a phone, by touch: a project's name tapped shows its page, and the panel brought back has it unfolded
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: process.env.SCHEME ?? 'dark' });
  const small = await phone.newPage();
  small.on('pageerror', (err) => problems.push(err.message));
  await small.goto(url);
  await small.getByLabel('Email').fill('ada@example.com');
  await small.getByLabel('Password').fill('correct horse');
  await small.getByRole('button', { name: 'Sign in', exact: true }).last().tap();
  const smallPanel = small.locator('nav.sidebar[aria-label="Resources"]');
  const bringBack = async () => {
    await small.getByRole('button', { name: 'Expand the side panel' }).or(smallPanel).first().waitFor();
    if ((await smallPanel.count()) === 0) await small.getByRole('button', { name: 'Expand the side panel' }).tap();
    await sideSettled(small);
    const smallRail = small.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await smallRail.getAttribute('aria-pressed')) !== 'true') await smallRail.tap();
    await smallPanel.waitFor();
  };
  await bringBack();
  // A browser that kept nothing: Projects unfolded, and Shop folded in it.
  await smallPanel.getByRole('button', { name: 'Fold Projects', exact: true }).waitFor();
  await smallPanel.getByRole('button', { name: 'Unfold Shop', exact: true }).waitFor();
  await smallPanel.locator('[data-project="shop"] > .nav-row .nav-item').tap();
  await small.locator('.view-head h1', { hasText: /^Shop$/ }).waitFor();
  await bringBack();
  await smallPanel.getByRole('button', { name: 'Fold Shop', exact: true }).waitFor();
  await smallPanel.locator('[data-project="shop"] .tree .nav-item', { has: small.locator('.truncate', { hasText: /^Board$/ }) }).waitFor();
  await small.screenshot({ path: `${out}/folding-9-project-phone.png` });
  await phone.close();

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
