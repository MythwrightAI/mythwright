// Channels put in groups of a person's own.
//
//   DAEMON=../target/debug/factory node scripts/groups.mjs [out-dir]
//
// The panel says what it is once, in its head, where what makes a group
// and a channel is. A group is made there, a channel is put in it by its
// menu, by the pointer's other button and by being dragged onto the
// group's name, and taken out by its menu or by being let go on the
// panel's head, which says Channels and is there before anything is
// dragged, or among those in none where there are any: nothing stands in
// their place where every channel is in a group, and nothing moves as a
// drag begins, at three widths and a phone's; a group made there is unfolded for what is put in it, and folds, and
// how it was left is there after the page is read again; it is called something else, and taken away, which leaves its
// channels where they were before there were groups. A channel is pinned
// to the top, and is where it was when it is no longer pinned. What a
// person arranges is kept with their account: moved to the Trash a
// channel leaves nothing in the browser, and given back it is where it
// was. A channel is made from a group's menu and goes in that group, at
// its end, unfolded, here and on another open page; made in another
// workspace it is in the group all the same and where it went is said;
// and nothing of the menu or the dialog lies over anything else at three
// widths and a phone's. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold } from './folded.mjs';
import { pageCrowded } from './crowded.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'groups-data');
const home = join(out, 'groups-home');

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
  for (const name of ['general', 'design', 'ops', 'random']) await cli('channel', 'create', name);
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  // A context of its own, so that a second open page of the same browser can be had.
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async () => {
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  };
  await page.goto(url);
  await signIn();
  const panel = page.getByRole('navigation', { name: 'Channels' });
  await panel.waitFor();

  // What the list says, from the top: those pinned, the channels in no group, then each group with what is under it.
  const listedIn = (nav) =>
    nav.evaluate((nav) => {
      const names = (within) => [...within.querySelectorAll('.grouped-channel > div > .nav-row .nav-item .truncate')].map((one) => one.textContent);
      return [...nav.querySelectorAll('.grouped, .grouped-loose')]
        .map((part) => (part.classList.contains('grouped-loose') ? names(part).join(' ') : `${part.querySelector('.group-name .truncate').textContent}[${names(part).join(' ')}]`))
        .join(' | ');
    });
  const listed = () => listedIn(panel);
  const row = (name) => panel.locator('.grouped-channel', { has: page.getByRole('button', { name: `# ${name}`, exact: true }) });
  // Where each part of the panel is, by the boxes: what is outlined as a drop moves nothing, so the same as it is carried.
  const boxes = () =>
    panel.evaluate((nav) =>
      [...nav.querySelectorAll('.brand, .brand > span, .nav-row, .grouped-loose, .grouped')].map((part) => {
        const at = part.getBoundingClientRect();
        return `${part.className.split(' ')[0]} ${Math.round(at.x)},${Math.round(at.y)} ${Math.round(at.width)}x${Math.round(at.height)}`;
      }),
    );
  // Nothing stands in the place of the channels in no group: no line, and nothing drawn where they would be.
  const nothingInTheirPlace = async (where) => {
    expect(`no line in the place of the channels in no group ${where}`, (await panel.getByText('none outside a group').count()) === 0);
    const loose = await panel.locator('.grouped-loose').evaluateAll((parts) => parts.map((part) => [part.textContent.trim(), part.getBoundingClientRect().height]));
    expect(`nothing where the channels in no group would be ${where}, got ${JSON.stringify(loose)}`, loose.every(([text, height]) => text === '' && height === 0));
  };
  // A channel carried by the pointer onto the panel's head, which says Channels: nothing moves as the drag begins
  // or as it comes over the head, which is outlined and nothing else, and let go there the channel is in no group.
  const carriedToHead = async (name, where) => {
    const from = await row(name).getByRole('button', { name: `# ${name}`, exact: true }).boundingBox();
    const head = await panel.locator('.brand > span').first().boundingBox();
    await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
    const still = await boxes();
    await page.mouse.down();
    await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2 - 6, { steps: 3 });
    const begun = await boxes();
    expect(`nothing to move as a drag begins ${where}: ${still.filter((one, n) => one !== begun[n]).join('; ')} became ${begun.filter((one, n) => one !== still[n]).join('; ')}`, JSON.stringify(still) === JSON.stringify(begun));
    await page.mouse.move(head.x + head.width / 2, head.y + head.height / 2, { steps: 6 });
    // The browser says the pointer is over it a moment after it is: waited for, as a drag over anything is.
    const marked = await panel.locator('.brand.drop').waitFor({ timeout: 5000 }).then(() => true, () => false);
    expect(`the head to be marked as where it is let go ${where}`, marked);
    expect(`the head to be outlined ${where}`, (await panel.locator('.brand').evaluate((it) => getComputedStyle(it).outlineStyle)) === 'dashed');
    const over = await boxes();
    expect(`nothing to move as it comes over the head ${where}: ${still.filter((one, n) => one !== over[n]).join('; ')}`, JSON.stringify(still) === JSON.stringify(over));
    if (where.includes('first')) await page.screenshot({ path: `${out}/groups-2-over-head.png` });
    await page.mouse.up();
    expect(`the head not to be marked once it is let go ${where}`, (await panel.locator('.brand.drop').count()) === 0);
  };
  const choose = async (label) => {
    const menu = page.getByRole('menu');
    await menu.waitFor();
    await menu.getByText(label, { exact: true }).click();
    await menu.waitFor({ state: 'detached' });
  };
  const name = async (called, button) => {
    await page.getByLabel('Name').fill(called);
    await page.getByRole('button', { name: button, exact: true }).click();
    await page.getByRole('dialog').waitFor({ state: 'detached' });
  };

  // The daemon lists channels by name.
  expect(`the channels to be listed as they always were, got ${await listed()}`, (await listed()) === 'design general ops random');
  // The panel is headed once, by what it is, and what makes a group or a channel is in its head: no second heading is over the list.
  const said = await panel.getByText('Channels', { exact: true }).count();
  expect(`the panel to say what it is once, got ${said} times`, said === 1);
  expect('the panel to be called Channels', (await page.getByRole('navigation', { name: 'Channels', exact: true }).count()) === 1);
  // No heading over the list: the topics, which were a node beside the channels, have a place of their own.
  expect('no heading over the list', (await panel.getByRole('heading').count()) === 0);
  for (const made of ['New group', 'New channel']) {
    expect(`${made} to be in the panel's head`, (await panel.locator('.brand').getByRole('button', { name: made }).count()) === 1);
  }
  // With no group there is nowhere to leave, and nothing says so.
  expect('nothing to stand for channels in no group while they are all in none', (await panel.getByText('none outside a group').count()) === 0);
  await page.screenshot({ path: `${out}/groups-0-none.png` });

  // A group is made from the panel's head, and is there with nothing in it.
  await page.getByRole('button', { name: 'New group' }).click();
  await page.getByRole('dialog').getByText('New group', { exact: true }).waitFor();
  await name('Work', 'Create');
  expect(`a group with nothing in it yet, got ${await listed()}`, (await listed()) === 'design general ops random | Work[]');
  await panel.getByText('nothing in it yet').waitFor();

  // A channel goes into it by the button at the end of its row,
  await row('design').hover();
  await page.getByRole('button', { name: 'More for #design' }).click();
  await choose('Work');
  // by the pointer's other button,
  await row('ops').click({ button: 'right' });
  await choose('Work');
  // and into one that is made for it there and then.
  await row('random').click({ button: 'right' });
  await choose('New group…');
  await name('Later', 'Create');
  expect(`three channels in two groups, got ${await listed()}`, (await listed()) === 'general | Work[design ops] | Later[random]');
  // Its menu says where it is.
  await row('design').click({ button: 'right' });
  expect('the group a channel is in to be marked in its menu', (await page.getByRole('menuitemradio', { name: 'Work', exact: true }).getAttribute('aria-checked')) === 'true');
  expect('a channel in a group to be offered none', (await page.getByRole('menuitem', { name: 'No group', exact: true }).count()) === 1);
  // A menu, like a dialog, listens for Escape from a moment after it is drawn.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.getByRole('menu').waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/groups-1-two.png` });

  // A group folds, and is as it was left when the page is read again.
  await page.getByRole('button', { name: 'Fold Work' }).click();
  expect(`a folded group to show none of its channels, got ${await listed()}`, (await listed()) === 'general | Work[] | Later[random]');
  await page.reload();
  await panel.waitFor();
  await page.getByRole('button', { name: 'Unfold Work' }).waitFor();
  expect(`the groups to be there after the page is read again, got ${await listed()}`, (await listed()) === 'general | Work[] | Later[random]');
  await page.getByRole('button', { name: 'Unfold Work' }).click();
  expect(`and what was in it, got ${await listed()}`, (await listed()) === 'general | Work[design ops] | Later[random]');

  // A channel is dragged onto a group's name, and out again onto the panel's
  // head, which says Channels and is there before anything is dragged. With
  // every channel in a group nothing stands where those in no group would be.
  await row('general').dragTo(panel.locator('.group-row', { hasText: 'Work' }));
  expect(`a channel dragged onto a group to be in it, got ${await listed()}`, (await listed()) === ' | Work[design general ops] | Later[random]');
  await nothingInTheirPlace('with every channel in a group');
  await page.screenshot({ path: `${out}/groups-2-all-in.png` });
  await carriedToHead('ops', 'the first time');
  expect(`a channel dragged onto the head to be in none, got ${await listed()}`, (await listed()) === 'ops | Work[design general] | Later[random]');
  // And among those in none, where there are some: onto one of them.
  await row('design').dragTo(row('ops'));
  expect(`a channel let go on one in no group to be in none, got ${await listed()}`, (await listed()) === 'design ops | Work[general] | Later[random]');
  await row('design').click({ button: 'right' });
  await choose('Work');
  expect(`and put back by its menu, got ${await listed()}`, (await listed()) === 'ops | Work[design general] | Later[random]');
  // And by its menu.
  await row('design').click({ button: 'right' });
  await choose('No group');
  expect(`a channel taken out by its menu to be in none, got ${await listed()}`, (await listed()) === 'design ops | Work[general] | Later[random]');

  // At three widths and a phone's, plainly and at whatever look the check is told: every channel put in a group,
  // nothing in their place, and each taken out again by being carried onto the head. Under 760 wide the side panel
  // is laid out as on a phone; carried by a mouse there, since a finger does not carry a channel.
  for (const [width, height] of [[1280, 860], [1000, 800], [800, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    // A screen that has just become a phone's has its side panel put away (`clearSide`), a moment after it is told so.
    if (width < 760) await page.getByRole('button', { name: 'Expand the side panel' }).waitFor();
    await sideSettled(page);
    if (!(await panel.isVisible())) {
      await page.getByRole('button', { name: 'Expand the side panel' }).click();
      await sideSettled(page);
    }
    await panel.waitFor();
    for (const name of ['design', 'ops']) {
      await row(name).click({ button: 'right' });
      await choose('Work');
    }
    expect(`every channel in a group at ${width}, got ${await listed()}`, (await listed()) === ' | Work[design general ops] | Later[random]');
    await nothingInTheirPlace(`at ${width}`);
    await page.screenshot({ path: `${out}/groups-2-all-in-at-${width}.png` });
    await carriedToHead('ops', `at ${width}`);
    await carriedToHead('design', `at ${width}, again`);
    expect(`the two carried onto the head to be in none at ${width}, got ${await listed()}`, (await listed()) === 'design ops | Work[general] | Later[random]');
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  if (!(await panel.isVisible())) {
    await page.getByRole('button', { name: 'Expand the side panel' }).click();
    await sideSettled(page);
  }
  await panel.waitFor();

  // A channel is pinned to the top, from its menu: it is listed there and not where it was,
  await row('general').click({ button: 'right' });
  await choose('Pin to the top');
  await row('ops').hover();
  await page.getByRole('button', { name: 'More for #ops' }).click();
  await choose('Pin to the top');
  // What is pinned is a branch, folded as every branch of the panel starts (`folding.mjs` holds the rest of that).
  expect(`what is pinned to be a branch, folded, got ${await listed()}`, (await listed()) === 'Pinned[] | design | Work[] | Later[random]');
  await unfold(page, 'Pinned');
  expect(`the pinned channels to be at the top and nowhere else, got ${await listed()}`, (await listed()) === 'Pinned[general ops] | design | Work[] | Later[random]');
  await page.screenshot({ path: `${out}/groups-3-pinned.png` });
  // is still there after the page is read again,
  await page.reload();
  await panel.waitFor();
  await panel.getByRole('button', { name: 'Fold Pinned' }).waitFor();
  expect(`what was pinned to be pinned after the page is read again, got ${await listed()}`, (await listed()) === 'Pinned[general ops] | design | Work[] | Later[random]');
  // and is where it was when it is no longer pinned: in its group, or in none.
  await row('general').click({ button: 'right' });
  expect('a pinned channel to be offered being unpinned', (await page.getByRole('menuitem', { name: 'Unpin', exact: true }).count()) === 1);
  await choose('Unpin');
  await row('ops').click({ button: 'right' });
  await choose('Unpin');
  expect(`the channels to be where they were, got ${await listed()}`, (await listed()) === 'design ops | Work[general] | Later[random]');

  // A group is called something else, and taken away: its channels are where they were before.
  await panel.locator('.group-row', { hasText: 'Later' }).hover();
  await page.getByRole('button', { name: 'More for Later' }).click();
  await choose('Rename…');
  await page.getByRole('dialog').getByText('Rename group', { exact: true }).waitFor();
  await name('Someday', 'Rename');
  expect(`the group to have its new name, got ${await listed()}`, (await listed()) === 'design ops | Work[general] | Someday[random]');
  await page.screenshot({ path: `${out}/groups-4-renamed.png` });
  // A channel is made from a group's menu, its first line, by the dialog the panel's `+` opens, and goes in that
  // group at its end, the group unfolded for it: on this page, and on a second open page that is never read again,
  // since where it was put is the account's. The group is folded here first, and open on the other.
  const other = await context.newPage();
  other.on('pageerror', (err) => problems.push(err.message));
  await other.goto(url);
  const otherPanel = other.getByRole('navigation', { name: 'Channels' });
  await otherPanel.waitFor();
  await other.getByRole('button', { name: 'Fold Work' }).waitFor();
  await page.getByRole('button', { name: 'Fold Work' }).click();
  const workMenu = async () => {
    await panel.locator('.group-row', { hasText: 'Work' }).hover();
    await page.getByRole('button', { name: 'More for Work' }).click();
    await page.getByRole('menu').waitFor();
  };
  await workMenu();
  const lines = await page.getByRole('menu').getByRole('menuitem').allInnerTexts();
  expect(`a group's menu to offer a new channel first, got ${lines.join(', ')}`, lines.join(',') === 'New channel…,Rename…,Remove group');
  await choose('New channel…');
  const making = page.getByRole('dialog');
  await making.getByText('New channel', { exact: true }).waitFor();
  expect('the dialog to say the channel goes in the group', (await making.getByText('It goes in Work, a group of your own.').count()) === 1);
  expect('the dialog to begin on the workspace showing', (await making.getByLabel('Workspace').evaluate((choice) => choice.selectedOptions[0].textContent)) === "Ada's workspace");
  await page.screenshot({ path: `${out}/groups-6-new-in-group.png` });
  await name('zulu', 'Create');
  await page.getByRole('heading', { name: 'zulu', level: 1 }).waitFor();
  await row('zulu').waitFor();
  expect(`the channel made to be at the group's end, unfolded, got ${await listed()}`, (await listed()) === 'design ops | Work[general zulu] | Someday[random]');
  await otherPanel.locator('.grouped-channel', { has: other.getByRole('button', { name: '# zulu', exact: true }) }).waitFor();
  expect(`and so on the other page, got ${await listedIn(otherPanel)}`, (await listedIn(otherPanel)) === 'design ops | Work[general zulu] | Someday[random]');
  const arrangedNow = await page.evaluate(async () => {
    const asked = await fetch('/api/arranged', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return (await asked.json()).arrangement.groups.find((group) => group.name === 'Work').channels;
  });
  const zulu = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.find((one) => one.name === 'zulu').id;
  expect(`the account to keep it last in the group, got ${arrangedNow}`, arrangedNow.at(-1) === zulu);
  await page.screenshot({ path: `${out}/groups-7-made-in-group.png` });
  await other.screenshot({ path: `${out}/groups-7-made-in-group-other.png` });
  await other.close();

  // Made in another workspace than the one showing, it is in the group all the same, which lists only the
  // workspace showing: the panel is as it was, and where it went is said.
  await cli('workspace', 'create', 'Shop front');
  await workMenu();
  await choose('New channel…');
  await making.getByLabel('Workspace').locator('option', { hasText: 'Shop front' }).waitFor({ state: 'attached' });
  await making.getByLabel('Workspace').selectOption({ label: 'Shop front' });
  await name('elsewhere', 'Create');
  await page.getByText('#elsewhere is in Work, listed there while Shop front is showing.', { exact: true }).waitFor();
  await page.getByRole('heading', { name: 'elsewhere', level: 1 }).waitFor();
  expect(`the panel to list what it did, got ${await listed()}`, (await listed()) === 'design ops | Work[general zulu] | Someday[random]');

  // Nothing of the menu or the dialog over anything else, at three widths and a phone's.
  // Under 760 wide the side panel is laid out as on a phone, which the phone below holds.
  for (const [width, height] of [[1280, 860], [1000, 800], [800, 800]]) {
    await page.setViewportSize({ width, height });
    await workMenu();
    let wrong = await pageCrowded(page);
    expect(`nothing of a group's menu over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    await page.screenshot({ path: `${out}/groups-8-menu-at-${width}.png` });
    await choose('New channel…');
    await making.getByText('It goes in Work, a group of your own.').waitFor();
    wrong = await pageCrowded(page, '.modal');
    expect(`nothing of the dialog over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    expect(`the page not to scroll sideways at ${width}`, !(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth)));
    await page.screenshot({ path: `${out}/groups-8-dialog-at-${width}.png` });
    await page.waitForTimeout(200);
    await page.keyboard.press('Escape');
    await making.waitFor({ state: 'detached' });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  {
    const phone = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, colorScheme: process.env.SCHEME ?? 'dark' });
    phone.on('pageerror', (err) => problems.push(err.message));
    await phone.goto(url);
    await phone.getByLabel('Email').fill('ada@example.com');
    await phone.getByLabel('Password').fill('correct horse');
    await phone.getByRole('button', { name: 'Sign in', exact: true }).last().tap();
    await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
    await sideSettled(phone);
    await phone.locator('.sidebar').waitFor();
    // The panel comes as it was last shown: Channels is pressed only where it is another's, since pressed again it would close.
    const nav = phone.getByRole('navigation', { name: 'Channels' });
    if (!(await nav.isVisible())) await phone.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).tap();
    await nav.waitFor();
    await phone.getByRole('button', { name: 'More for Work' }).tap();
    await phone.getByRole('menu').waitFor();
    // On a phone the side panel lies over the page, so what is held is the panel and the menu over it.
    let wrong = [...(await pageCrowded(phone, 'nav[aria-label="Channels"]')), ...(await pageCrowded(phone, '[role="menu"]'))];
    expect(`nothing of a group's menu over anything else at a phone's width: ${wrong.join('; ')}`, wrong.length === 0);
    await phone.screenshot({ path: `${out}/groups-8-menu-at-390.png` });
    await phone.getByRole('menu').getByText('New channel…', { exact: true }).tap();
    await phone.getByRole('dialog').getByText('It goes in Work, a group of your own.').waitFor();
    wrong = await pageCrowded(phone, '.modal');
    expect(`nothing of the dialog over anything else at a phone's width: ${wrong.join('; ')}`, wrong.length === 0);
    expect("the page not to scroll sideways at a phone's width", !(await phone.evaluate(() => document.documentElement.scrollWidth > window.innerWidth)));
    await phone.screenshot({ path: `${out}/groups-8-dialog-at-390.png` });
    await phone.context().close();
  }
  await cli('channel', 'remove', 'zulu');
  await row('zulu').waitFor({ state: 'detached' });

  await panel.locator('.group-row', { hasText: 'Work' }).hover();
  await page.getByRole('button', { name: 'More for Work' }).click();
  await choose('Remove group');
  expect(`a group taken away to leave its channels in none, got ${await listed()}`, (await listed()) === 'design general ops | Someday[random]');
  // The channel itself is as it was: pressed, it opens.
  await row('general').getByRole('button', { name: '# general', exact: true }).click();
  await page.getByRole('heading', { name: 'general', level: 1 }).waitFor();

  // And one opened as something is said there shows it. What was said is
  // read once and followed from then on, and the two cross: here the
  // reading is taken, something is said, the page hears of it, and only
  // then does the reading come in. It is in neither unless the page kept it.
  {
    await cli('post', 'design', 'said before it was opened');
    // The reading of this channel and no other: the channel shown before it may still be asking for its own as this
    // begins (its heading is drawn before it asks), and held in its place, that one was waited on while this went by.
    const design = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.find((one) => one.name === 'design').id;
    const reading = (address) => address.pathname.endsWith(`/channels/${design}/messages`) && address.searchParams.has('limit');
    let held = false;
    await page.route(reading, async (route) => {
      if (held) return route.continue();
      held = true;
      const taken = await route.fetch();
      await cli('post', 'design', 'said as it was opened');
      // The page hears things in the order they were said: once it has
      // heard of a channel made afterwards, it has heard what was said.
      await cli('channel', 'create', 'afterwards');
      await row('afterwards').waitFor();
      await route.fulfill({ response: taken });
    });
    await row('design').getByRole('button', { name: '# design', exact: true }).click();
    await page.getByRole('heading', { name: 'design', level: 1 }).waitFor();
    await page.getByText('said before it was opened', { exact: true }).waitFor();
    expect('the reading of the channel to have been held while something was said there', held);
    const late = await page.getByText('said as it was opened', { exact: true }).waitFor({ timeout: 5000 }).then(() => true, () => false);
    expect('what was said as the channel was opened to be on the page', late);
    expect('and to be there once', (await page.getByText('said as it was opened', { exact: true }).count()) === 1);
    await page.unroute(reading);
    // The channel that was made to tell by is taken away again.
    await cli('channel', 'remove', 'afterwards');
    await row('afterwards').waitFor({ state: 'detached' });
  }

  // A channel is moved to the Trash from the same menu, by its owner, once
  // they have been told what goes with it. Where it was put is the
  // account's (`arranged.mjs` holds the rest of that): this browser keeps
  // nothing of it once it is gone, and given back from the Trash it is
  // where it was.
  const random = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.find((one) => one.name === 'random').id;
  const keptHere = () => page.evaluate(() => Object.values({ ...localStorage }).join('\n'));
  const keptThere = () =>
    page.evaluate(async () => {
      const asked = await fetch('/api/arranged', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      return JSON.stringify(await asked.json());
    });
  await row('random').click({ button: 'right' });
  await Promise.all([page.waitForResponse((answer) => answer.url().endsWith('/api/arranged') && answer.request().method() === 'PUT'), choose('Pin to the top')]);
  expect('the account to keep where the channel is listed', (await keptThere()).includes(random));
  await row('random').click({ button: 'right' });
  await choose('Move to Trash…');
  const going = page.getByRole('dialog');
  await going.getByText('Move #random to the Trash?', { exact: true }).waitFor();
  expect(`to be told what goes with it, got "${(await going.innerText()).replace(/\s+/g, ' ')}"`, (await going.innerText()).includes('Its messages go with it, and everything attached to them'));
  await page.screenshot({ path: `${out}/groups-5-removing.png` });
  await going.getByRole('button', { name: 'Move to Trash', exact: true }).click();
  await going.waitFor({ state: 'detached' });
  await row('random').waitFor({ state: 'detached' });
  expect(`the channel to be gone, and its group to be there with nothing in it, got ${await listed()}`, (await listed()) === 'design general ops | Someday[]');
  expect('nothing of it to be kept in this browser', !(await keptHere()).includes(random));
  expect('and where it was to be kept with the account', (await keptThere()).includes(random));
  const entry = JSON.parse((await cli('trash', '--json')).stdout).trash.find((one) => String(one.called).includes('random'));
  await cli('trash', 'restore', entry.id);
  await row('random').waitFor();
  expect(`a channel given back to be pinned where it was, got ${await listed()}`, (await listed()) === 'Pinned[random] | design general ops | Someday[]');

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
