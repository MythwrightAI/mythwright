// The Members line under Factory in the Resources panel (it was
// the last branch of the Channels panel): everybody here and the agents of
// the workspace the panel is of, each pressed for a menu of what can be
// done with them; and the Members page its name opens.
//
//   DAEMON=../target/debug/factory node scripts/members.mjs [out-dir]
//
// Two people, Ada, who owns the factory, and Nell, each on a page of her
// own that is never read again. The line is a fold of the Factory part,
// folded as it starts, beside Activity, Measurements and the Trash, its
// arrow folding it and its name opening the Members page: unfolded, it lists every person with an account here, Ada first saying
// that she is you, then the agents of the workspace (its projects' and the
// factory's own, and none of another workspace's project), each saying in
// faint words what it is (agent, agent (external), stopped), and never a
// bot; and the Channels panel has no Members any more. A member pressed
// opens a menu under its line, as a row's menu is (`RowMenu`), with the
// keyboard in it and Escape giving it back: Details, who they are, read
// about in the drawer (a person with the channels the reader shares with
// them, an agent as the Tools panel reads one); Message, the direct
// message there is or the dialog that asks for the first message; of an
// agent, its line's menu on the Members page's Agents tab, the same lines as that menu
// whoever reads (History and Edit… each opening the drawer); of a person,
// for whoever manages the factory, Permissions…, what they hold by area
// and what may be given them beyond their role. Nothing a reader may not
// do is offered: Nell's menus have no Permissions… and no Stop or Remove….
// A direct message written to a person is opened as a conversation of the
// two, and her menu then opens that one and never makes a second; an
// agent the factory starts, behind a wall, is written to the same way; one
// with no wall is refused in the daemon's sentence with the dialog staying
// as it was written; and an agent the factory does not start is written to
// as a person is. What Nell says there from a terminal is counted on her
// line on Ada's page, and on the Members line folded. The panel's box
// finds members by name and by the words beside one, the line held open.
// The Members page has People and then Agents, a tab each: pressing the
// line's name opens it at People and folds nothing; People lists Ada and
// Nell, each line with the panel's menu and Message, and Invite someone for
// Ada and not for Nell; Agents is the agents with the choice of whose in
// the head; each tab says its address, and the Agents page's old address
// (`/agents`) lands on the Agents tab.
// Every name is drawn in the panel's ordinary colour for a name, in the
// dark and the light, what each is said by the mark before it and the
// faint words after. Nothing of the panel, the menu or the dialog lies
// over anything else or scrolls sideways, and the menu is under its line
// and in the window with nothing moved, at three widths, with the panel
// dragged as narrow as it goes, and at a phone's width by touch. The
// environment variables are those of walkthrough.mjs, with `LOOK`.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { agentsPage } from './apart.mjs';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { saysWhere } from './where.ts';
import { drawerAtRest, sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'members-data');
const home = join(out, 'members-home');
const hers = join(out, 'members-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
// Walls are held: an agent the factory starts is then in a conversation where it is behind one. The agents run
// on the assistant that is built in, which is no program, so nothing is started behind them.
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
  await cli('channel', 'create', 'general');
  await cli('workspace', 'create', 'Front');
  await cli('project', 'create', 'shop', '--bare');
  await cli('project', 'create', 'depot', '--bare', '--workspace', 'front');
  // The factory's own (one behind a wall, one with none, one stopped, one it does not start), one of a project of
  // Ada's workspace and one of a project of Front's.
  await cli('agent', 'add', 'helper');
  await cli('agent', 'add', 'loose', '--no-wall');
  await cli('agent', 'add', 'idle');
  await cli('agent', 'stop', 'idle');
  await cli('agent', 'invite', 'scout');
  await cli('agent', 'add', 'shop-worker', '--project', 'shop');
  await cli('agent', 'add', 'depot-worker', '--project', 'depot');
  const bot = /`([^`]+)`/.exec((await cli('channel', 'speaker')).stdout)?.[1];
  // Ada, an agent and Nell in general, which Nell comes into by what she says there: a channel the two share.
  await cli('channel', 'add', 'general', 'agent:helper');
  await cli('post', 'general', `@${bot} help`);
  await nell('post', 'general', 'morning');
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
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  const resourcesOn = (on) => on.getByRole('navigation', { name: 'Resources' });
  const toResources = async (on, press = 'click') => {
    const railed = on.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    await railed.waitFor();
    if ((await railed.getAttribute('aria-pressed')) !== 'true') await railed[press]();
    await resourcesOn(on).waitFor();
  };
  const page = await signedIn('ada@example.com');
  const loads = { ada: 0 };
  page.on('load', () => (loads.ada += 1));
  await page.locator('.rail').waitFor();
  await toResources(page);
  const panel = resourcesOn(page);
  const part = (on) => resourcesOn(on).locator('.members-part');
  // The line's arrow, which folds it, and its name, which opens the Members page.
  const membersLine = (on) => part(on).locator(':scope > div > .nav-row > button.nav-twist');
  const membersName = (on) => part(on).locator(':scope > div > .nav-row > button.nav-item');
  // What the line lists, as it is read: its fold as + or −, then each member by its name, the faint words beside it in
  // brackets and how many wait in square ones.
  const LISTED = `(() => {
    const part = document.querySelector('nav[aria-label="Resources"] .members-part');
    if (!part) return '';
    const fold = part.querySelector(':scope > div > .nav-row > button.nav-twist');
    const count = part.querySelector(':scope > div > .nav-row > button.nav-item .count')?.textContent.trim();
    return [(fold.getAttribute('aria-expanded') === 'true' ? '−' : '+') + 'Members' + (count ? '[' + count + ']' : ''), ...[...part.querySelectorAll('.member-line')].map((line) => {
      const said = line.querySelector('.member-said')?.textContent.trim();
      const waits = line.querySelector('.count')?.textContent.trim();
      return line.querySelector('.truncate').textContent.trim() + (said ? ' (' + said + ')' : '') + (waits ? ' [' + waits + ']' : '');
    })].join(' | ');
  })()`;
  const lists = async (on, wanted, why) => {
    await on.waitForFunction(`${LISTED} === ${JSON.stringify(wanted)}`, null, { timeout: 8000 }).catch(async () => {
      throw new Error(`expected the Members line to list "${wanted}" ${why}, got "${await on.evaluate(LISTED)}"`);
    });
  };
  const line = (on, name) => part(on).locator('.member-line', { has: on.locator('.truncate', { hasText: new RegExp(`^${name}$`) }) });
  const head = (on) => on.locator('.view[data-focused="true"] .channel .view-head');
  // The menu that is open, a line a thing: what it says, a heading in square brackets, and set apart after `/`.
  const MENU = `(() => [...(document.querySelector('.row-menu:not(.row-beside)')?.children ?? [])].map((one) => (one.classList.contains('apart') ? '/' : '') + (one.matches('button') ? one.querySelector('.truncate').textContent.trim() : '[' + one.textContent.trim() + ']')))()`;
  const menu = (on) => on.locator('.row-menu:not(.row-beside)');
  // A member's menu opened from its line, by the line itself: under it, in the window, with nothing of the panel moved,
  // the keyboard in it. What it offers is answered.
  const opened = async (on, name, press = 'click') => {
    // Where the panel's lines are once it is still: a page just opened may still be reading what it lists.
    const tops = () => resourcesOn(on).locator('.nav').evaluate((nav) => [...nav.querySelectorAll('.nav-item')].map((one) => one.getBoundingClientRect().top).join(','));
    let before = await tops();
    for (let tries = 0; tries < 20; tries += 1) {
      await on.waitForTimeout(100);
      const now = await tops();
      if (now === before) break;
      before = now;
    }
    await line(on, name)[press]();
    await menu(on).waitFor();
    if (press !== 'tap') await on.waitForFunction(() => !!document.activeElement?.closest('.row-menu'), null, { timeout: 4000 }).catch(() => undefined);
    const where = await on.evaluate((name) => {
      const box = document.querySelector('.row-menu:not(.row-beside)').getBoundingClientRect();
      const from = [...document.querySelectorAll('.member-line')].find((one) => one.querySelector('.truncate').textContent.trim() === name).getBoundingClientRect();
      return { box: box.toJSON(), from: from.toJSON(), width: window.innerWidth, height: window.innerHeight, focused: !!document.activeElement?.closest('.row-menu') };
    }, name);
    expect(`the menu of ${name} under its line and in the window: ${JSON.stringify(where)}`, where.box.top >= where.from.bottom - 0.5 && where.box.left >= 0 && where.box.right <= where.width + 0.5 && where.box.bottom <= where.height + 0.5);
    expect(`the keyboard in the menu of ${name}`, press === 'tap' || where.focused);
    const after = await tops();
    expect(`nothing of the panel moved by the menu of ${name}`, before === after);
    const wrong = await pageCrowded(on, '.row-menu');
    expect(`nothing of the menu of ${name} over anything else of it: ${wrong.join('; ')}`, wrong.length === 0);
    return on.evaluate(MENU);
  };
  const choose = (on, label, press = 'click') => menu(on).locator(':scope > button', { has: on.locator('.truncate', { hasText: new RegExp(`^${label.replace(/[.…]/g, '\\$&')}$`) }) })[press]();
  // The conversations Ada has that are of exactly her and `with`, by asking the daemon from inside the page.
  const directs = (on, withActor) =>
    on.evaluate(async (actor) => {
      const read = await (await fetch('/api/channels', { headers: { 'x-page-key': localStorage['page-key'] } })).json();
      return (read.channels ?? read).filter((one) => one.conversation && one.members.filter((member) => !member.actor.startsWith('bot:')).length === 2 && one.members.some((member) => member.actor === actor)).map((one) => one.id);
    }, withActor);
  const nellActor = await page.evaluate(async () => {
    const read = await (await fetch('/api/people', { headers: { 'x-page-key': localStorage['page-key'] } })).json();
    return read.people.find((one) => one.name === 'Nell').actor;
  });
  const crowded = async (on, where) => {
    const wrong = await pageCrowded(on, 'nav.sidebar[aria-label="Resources"]');
    expect(`nothing of the Resources panel over anything else ${where}: ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await on.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`nothing to scroll sideways ${where}, got ${sideways}px`, sideways <= 0);
  };
  const drawerSays = async (on, title, words) => {
    const drawer = drawerAtRest(on);
    await drawer.waitFor();
    await drawer.locator('.drawer-head', { hasText: title }).waitFor();
    for (const one of words) await drawer.getByText(one, { exact: true }).first().waitFor();
  };
  const putDrawerAway = async (on) => {
    await on.waitForTimeout(200);
    await on.keyboard.press('Escape');
    await on.locator('.drawer').waitFor({ state: 'detached' });
  };
  // The Members page's tabs as they are drawn, the one showing marked `*`; and its people, each with the faint words beside them.
  const tabsOf = (on) => on.locator('.members-tabs [role="tab"]').evaluateAll((all) => all.map((one) => (one.getAttribute('aria-selected') === 'true' ? '*' : '') + one.textContent.trim()).join(' '));
  const PEOPLE = `[...document.querySelectorAll('.person-line')].map((one) => one.querySelector('.person-name').textContent.trim() + (one.querySelector('.person-said') ? ' (' + one.querySelector('.person-said').textContent.trim() + ')' : '')).join(' | ')`;
  const membersTab = (on, name) => on.locator('.members-tabs').getByRole('tab', { name, exact: true });
  const addressIs = (on, address, why) =>
    on.waitForFunction((address) => location.pathname + location.hash === address, address, { timeout: 8000 }).catch(async () => {
      throw new Error(`expected the address ${address} ${why}, got ${await on.evaluate(() => location.pathname + location.hash)}`);
    });
  /**
   * The Members page from the line's name, by whoever `on` is signed in as (`who`: the people it lists, whether it offers
   * inviting, and Nell's line's menu where it is Ada's): People first and then Agents, each by the boxes, a picture of each in `shots`.
   */
  const pageOfMembers = async (on, who, press = 'click') => {
    const folded = await membersLine(on).getAttribute('aria-expanded');
    await membersName(on)[press]();
    await on.getByRole('heading', { name: 'Members', level: 1 }).waitFor();
    // On a phone the panel is put away as the page opens, and is let go once it has gone: what it was left as is read where it is still drawn.
    await sideSettled(on);
    if (await membersLine(on).isVisible()) expect('pressing the name to neither fold nor unfold the line', (await membersLine(on).getAttribute('aria-expanded')) === folded);
    expect(`People and then Agents, People showing, got "${await tabsOf(on)}"`, (await tabsOf(on)) === '*People Agents');
    await addressIs(on, '/#/members', 'at People');
    await on.waitForFunction(`${PEOPLE} === ${JSON.stringify(who.people)}`, null, { timeout: 8000 }).catch(async () => {
      throw new Error(`expected People to list "${who.people}", got "${await on.evaluate(PEOPLE)}"`);
    });
    const invites = on.locator('.people-listed').getByRole('button', { name: 'Invite someone', exact: true });
    expect(`Invite someone ${who.invites ? 'offered' : 'not offered'} on People`, (await invites.count()) === (who.invites ? 1 : 0));
    if (who.invites) {
      await invites[press]();
      await on.locator('.people-listed .code-box').waitFor();
    }
    expect('no choice of whose agents while People shows', (await on.getByLabel(/^What is shown of /).count()) === 0);
    const people = await pageCrowded(on, '.members-page');
    expect(`nothing of People over anything else: ${people.join('; ')}`, people.length === 0);
    const wide = await on.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`nothing of People to scroll sideways, got ${wide}px`, wide <= 0);
    if (press !== 'tap') await on.mouse.move(5, 5);
    await sideSettled(on);
    await on.screenshot({ path: `${out}/${who.shots}-people.png` });
    if (who.menu) {
      // A person's line has the menu the panel's line has, and Message, which opens the conversation there is.
      const theirs = on.locator(`.person-line[data-actor="${nellActor}"]`);
      await theirs.getByRole('button', { name: 'More that can be done with Nell', exact: true })[press]();
      await menu(on).waitFor();
      const offered = (await on.evaluate(MENU)).join(' ');
      expect(`Nell's menu on People to be her line's in the panel, got "${offered}"`, offered === who.menu);
      // The menu has the keyboard, which Escape is said to.
      await on.waitForFunction(() => !!document.activeElement?.closest('.row-menu'), null, { timeout: 4000 });
      await on.keyboard.press('Escape');
      await menu(on).waitFor({ state: 'detached' });
      await theirs.getByRole('button', { name: 'Message Nell', exact: true })[press]();
      await on.locator('.channel > .split .message', { hasText: 'by Tuesday' }).waitFor();
      await membersName(on)[press]();
      await on.getByRole('heading', { name: 'Members', level: 1 }).waitFor();
    }
    await membersTab(on, 'Agents')[press]();
    // The workspace's agents, as the choice starts: shop-worker, of its project, and none of the factory's own.
    await on.locator('.agents-listed .agent-line[data-agent="shop-worker"]').waitFor();
    expect(`Agents showing, got "${await tabsOf(on)}"`, (await tabsOf(on)) === 'People *Agents');
    await addressIs(on, '/agents', 'at Agents');
    expect('the choice of whose agents in the head while Agents shows', (await on.locator('.members-page > .view-head').getByLabel(/^What is shown of /).count()) === 1);
    const agents = await pageCrowded(on, '.members-page');
    expect(`nothing of Agents over anything else: ${agents.join('; ')}`, agents.length === 0);
    if (press !== 'tap') await on.mouse.move(5, 5);
    await sideSettled(on);
    await on.screenshot({ path: `${out}/${who.shots}-agents.png` });
    // Back to People, as it was; the agents' choice gone with them.
    await membersTab(on, 'People')[press]();
    await on.locator('.person-line').first().waitFor();
    await addressIs(on, '/#/members', 'at People again');
    expect('the choice of whose agents gone with the Agents tab', (await on.getByLabel(/^What is shown of /).count()) === 0);
  };

  // ── The line, folded as it starts, beside the Factory's other lines; and none of it in the Channels panel ──
  await lists(page, '+Members', 'folded as it starts');
  const factory = await panel.locator('.nav').evaluate((nav) => {
    const heading = [...nav.querySelectorAll('h3.nav-section')].find((one) => one.textContent.trim() === 'Factory');
    return [...heading.nextElementSibling.querySelectorAll(':scope > .nav-item .truncate, :scope > .members-part > div > .nav-row > .nav-item .truncate')].map((one) => one.textContent.trim()).join(' ');
  });
  expect(`the Members line among the Factory's lines, and no Agents line, got "${factory}"`, factory === 'Activity Measurements Members Trash');
  await page.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).click();
  await page.getByRole('navigation', { name: 'Channels' }).waitFor();
  expect('no Members in the Channels panel', (await page.getByRole('navigation', { name: 'Channels' }).locator('.members-part, .member-line').count()) === 0);
  await toResources(page);
  await membersLine(page).click();
  const everybody = '−Members | Ada (you) | Nell | helper (agent) | loose (agent) | scout (agent (external)) | shop-worker (agent) | idle (stopped)';
  await lists(page, everybody, "with every person, Ada's own line first, and the agents of her workspace and the factory's own");
  expect('no bot to be listed', !!bot && (await line(page, bot).count()) === 0);
  expect("no agent of another workspace's project to be listed", (await line(page, 'depot-worker').count()) === 0);
  // Whoever it is, a name is drawn in the panel's ordinary colour for a name, in the dark and the light: what each is,
  // is said by the mark before it and the faint words after it.
  const ordinaries = [];
  for (const light of [false, true]) {
    await litAs(page, light ? 'light' : 'dark');
    const colours = await page.evaluate(() => {
      const nav = document.querySelector('nav[aria-label="Resources"]');
      const colour = (one) => getComputedStyle(one).color;
      const names = [...nav.querySelectorAll('.member-line > .truncate')].map((one) => `${one.textContent.trim()}=${colour(one)}`);
      const faint = [...nav.querySelectorAll('.member-line .member-said')].map(colour);
      const probe = document.createElement('span');
      probe.style.color = 'var(--dim)';
      nav.append(probe);
      const ordinary = colour(probe);
      probe.remove();
      return { names, faint, ordinary };
    });
    ordinaries.push(colours.ordinary);
    const other = colours.names.filter((one) => !one.endsWith(`=${colours.ordinary}`));
    expect(`every name under Members in the panel's ordinary colour (${colours.ordinary}) in the ${light ? 'light' : 'dark'}, and these are not: ${other.join(', ')}`, colours.names.length === 7 && other.length === 0);
    expect(`what is said beside a name to stay faint in the ${light ? 'light' : 'dark'}`, colours.faint.length > 0 && colours.faint.every((one) => one !== colours.ordinary));
    await page.mouse.move(900, 600);
    await page.screenshot({ path: `${out}/members-0-names-${light ? 'light' : 'dark'}.png` });
  }
  expect(`the dark and the light read apart, each in its own ordinary colour for a name, got ${ordinaries.join(' and ')}`, ordinaries[0] !== ordinaries[1]);
  await litAs(page, scheme);
  await page.screenshot({ path: `${out}/members-1-listed.png` });

  // ── What each menu offers whoever manages the factory, and the Agents tab's own for an agent ──
  const offers = {};
  for (const name of ['Ada', 'Nell', 'helper', 'idle', 'scout']) {
    offers[name] = (await opened(page, name)).join(' ');
    await page.keyboard.press('Escape');
    await menu(page).waitFor({ state: 'detached' });
    expect(`the keyboard given back to ${name}'s line by Escape`, await line(page, name).evaluate((one) => document.activeElement === one));
  }
  expect(`her own menu to be who she is and what she holds, got "${offers.Ada}"`, offers.Ada === 'Details Permissions…');
  expect(`Nell's to be who she is, a message and what she holds, got "${offers.Nell}"`, offers.Nell === 'Details Message… Permissions…');
  expect(`helper's to be who it is, a message and its line's menu on the Agents tab, got "${offers.helper}"`, offers.helper === 'Details Message… /Edit… History Stop Move to project… /Remove…');
  expect(`a stopped agent's to offer no message and no Stop, got "${offers.idle}"`, offers.idle === 'Details /Edit… History Move to project… /Remove…');
  expect(`one from outside's to have its line's, got "${offers.scout}"`, offers.scout.startsWith('Details Message… /Edit… History'));
  await opened(page, 'Nell');
  await page.screenshot({ path: `${out}/members-2-menu.png` });
  await page.keyboard.press('Escape');

  // ── Details, Permissions…, History and Edit…, each doing what it says ──
  await opened(page, 'Nell');
  await choose(page, 'Details');
  await drawerSays(page, 'Nell', ['Channels you share', '#general']);
  await page.screenshot({ path: `${out}/members-3-details.png` });
  await putDrawerAway(page);
  await opened(page, 'Nell');
  await choose(page, 'Permissions…');
  await drawerSays(page, 'Nell: permissions', ['What they may ask of the factory']);
  await drawerAtRest(page).locator('.beyond').waitFor();
  await page.screenshot({ path: `${out}/members-4-permissions.png` });
  await putDrawerAway(page);
  await opened(page, 'helper');
  await choose(page, 'Details');
  await drawerSays(page, 'helper', ['Runs on', 'What it was told it is for']);
  await putDrawerAway(page);
  await opened(page, 'helper');
  await choose(page, 'History');
  await drawerAtRest(page).locator('.drawer-head', { hasText: 'helper: history' }).waitFor();
  await putDrawerAway(page);
  await opened(page, 'helper');
  await choose(page, 'Edit…');
  await drawerAtRest(page).waitFor();
  await putDrawerAway(page);
  // By the keyboard: down to Details, and Enter.
  await line(page, 'Ada').focus();
  await page.keyboard.press('Enter');
  await menu(page).waitFor();
  await page.waitForFunction(() => document.activeElement?.closest('.row-menu') && document.activeElement.textContent.trim() === 'Details');
  await page.keyboard.press('Enter');
  await drawerSays(page, 'Ada', ['person · you', 'What you may ask of the factory']);
  await putDrawerAway(page);

  // ── A direct message with a person: written, and opened again as the same one ──
  await opened(page, 'Nell');
  await choose(page, 'Message…');
  const dialog = page.getByRole('dialog');
  await dialog.getByText('Message Nell', { exact: true }).waitFor();
  expect('the caret to be where the message is written', await dialog.getByLabel('Message').evaluate((box) => document.activeElement === box));
  expect('nothing to send while nothing is written', await dialog.getByRole('button', { name: 'Send', exact: true }).isDisabled());
  await dialog.getByLabel('Message').fill('is the quay mended?');
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    const wrong = await pageCrowded(page, '.modal');
    expect(`nothing of the dialog over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/members-5-write.png` });
  await dialog.getByRole('button', { name: 'Send', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await head(page).getByRole('img', { name: 'Conversation' }).waitFor();
  await page.locator('.channel > .split .message', { hasText: 'is the quay mended?' }).waitFor();
  const first = await directs(page, nellActor);
  expect(`one conversation of the two, got ${JSON.stringify(first)}`, first.length === 1);
  expect(`her menu to open it now, got "${(await opened(page, 'Nell')).join(' ')}"`, (await page.evaluate(MENU)).join(' ') === 'Details Message Permissions…');
  await page.keyboard.press('Escape');
  // Something else shown, and her menu's Message pressed: the same one, and no second.
  await page.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).click();
  const channelsPanel = page.getByRole('navigation', { name: 'Channels' });
  await channelsPanel.locator('.grouped-channel .nav-item', { hasText: 'general' }).click();
  await head(page).locator('h1', { hasText: 'general' }).waitFor();
  expect('it to be in no list of channels', (await channelsPanel.locator('.grouped-channel .nav-item', { hasText: 'quay' }).count()) === 0);
  await toResources(page);
  await opened(page, 'Nell');
  await choose(page, 'Message');
  await page.locator('.channel > .split .message', { hasText: 'is the quay mended?' }).waitFor();
  await head(page).getByRole('img', { name: 'Conversation' }).waitFor();
  expect('no dialog to be asked for one that is there', (await dialog.count()) === 0);
  const again = await directs(page, nellActor);
  expect(`the same conversation and no second, got ${JSON.stringify(again)}`, JSON.stringify(again) === JSON.stringify(first));
  await page.mouse.move(900, 600);
  await page.screenshot({ path: `${out}/members-6-opened.png` });

  // ── With agents: one the factory starts, one it will have in no conversation, and one it does not start ──
  await opened(page, 'helper');
  await choose(page, 'Message…');
  await dialog.getByText('Message helper', { exact: true }).waitFor();
  await dialog.getByLabel('Message').fill('what is on the board?');
  await dialog.getByRole('button', { name: 'Send', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await page.locator('.channel > .split .message', { hasText: 'what is on the board?' }).waitFor();
  // The assistant that is built in answers there, while it is showing: nothing of it waits.
  await page.locator('.channel > .split .message', { hasText: 'built-in assistant' }).waitFor({ timeout: 15000 });
  expect('a conversation with the agent to be there', (await directs(page, 'agent:helper')).length === 1);
  await opened(page, 'loose');
  await choose(page, 'Message…');
  await dialog.getByText('Message loose', { exact: true }).waitFor();
  await dialog.getByLabel('Message').fill('are you there?');
  await dialog.getByRole('button', { name: 'Send', exact: true }).click();
  const why = dialog.getByRole('alert');
  await why.waitFor();
  expect(`the daemon's sentence where it was asked, got "${await why.innerText()}"`, (await why.innerText()).includes('an agent with no wall is in no channel kept to its members'));
  expect('no mark a terminal reads commands by in it', !(await why.innerText()).includes('`'));
  expect('what was written to be there still', (await dialog.getByLabel('Message').inputValue()) === 'are you there?');
  const wrongly = await pageCrowded(page, '.modal');
  expect(`nothing of the dialog over anything else with what the daemon said: ${wrongly.join('; ')}`, wrongly.length === 0);
  await page.screenshot({ path: `${out}/members-7-refused.png` });
  expect('nothing to have been made with it', (await directs(page, 'agent:loose')).length === 0);
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });
  // Put away and asked again, what was written for it is kept.
  await opened(page, 'loose');
  await choose(page, 'Message…');
  expect('what was written for it to be kept while the dialog was away', (await dialog.getByLabel('Message').inputValue()) === 'are you there?');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await opened(page, 'scout');
  await choose(page, 'Message…');
  await dialog.getByText('Message scout', { exact: true }).waitFor();
  await dialog.getByLabel('Message').fill('the south gate, when you can');
  await dialog.getByRole('button', { name: 'Send', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await page.locator('.channel > .split .message', { hasText: 'the south gate, when you can' }).waitFor();
  expect('a conversation with the external agent to be there', (await directs(page, 'agent:scout')).length === 1);

  // ── What waits there, counted on a page that is never read again ───────
  await page.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).click();
  await channelsPanel.locator('.grouped-channel .nav-item', { hasText: 'general' }).click();
  await head(page).locator('h1', { hasText: 'general' }).waitFor();
  await toResources(page);
  const [quay] = first;
  await nell('post', quay, 'not yet');
  await lists(page, everybody.replace('| Nell |', '| Nell [1] |'), 'once Nell said something there, with Ada looking at something else');
  await nell('post', quay, 'by Tuesday');
  await lists(page, everybody.replace('| Nell |', '| Nell [2] |'), 'once she said something more');
  await membersLine(page).click();
  await lists(page, '+Members[2]', 'folded, saying what waits under it');
  await page.screenshot({ path: `${out}/members-8-waiting.png` });
  await membersLine(page).click();
  await opened(page, 'Nell');
  await choose(page, 'Message');
  await page.locator('.channel > .split .message', { hasText: 'by Tuesday' }).waitFor();
  await lists(page, everybody, 'once the direct message was read');

  // ── The Members page: its name opens it and folds nothing; People, then Agents ──
  await page.setViewportSize({ width: 1440, height: 900 });
  await pageOfMembers(page, { people: 'Ada (you) | Nell', invites: true, menu: 'Details Message Permissions…', shots: 'members-13-page' });
  await page.setViewportSize({ width: 1280, height: 860 });
  // The Agents page's address from before (a bookmark, a link, `mw open`) lands on the Agents tab, on a page of Ada's own.
  const before = await signedIn('ada@example.com');
  await before.locator('.rail').waitFor();
  await before.goto(`${url}/agents`);
  await before.getByRole('heading', { name: 'Members', level: 1 }).waitFor();
  await before.locator('.agents-listed').waitFor();
  expect(`the old address to land on Agents, got "${await tabsOf(before)}"`, (await tabsOf(before)) === 'People *Agents');
  await addressIs(before, '/agents', 'kept as it was asked');
  await before.context().close();

  // ── The panel's box ─────────────────────────────────────────────────────
  await membersLine(page).click();
  await lists(page, '+Members', 'folded again');
  const seek = panel.locator('.panel-seek input');
  for (const [words, wanted] of [
    ['nell', '−Members | Nell'],
    ['external', '−Members | scout (agent (external))'],
    ['stopped', '−Members | idle (stopped)'],
    ['you', '−Members | Ada (you)'],
    ['members nell', '−Members | Nell'],
  ]) {
    await seek.fill(words);
    await lists(page, wanted, `for "${words}", held open though folded`);
  }
  await page.screenshot({ path: `${out}/members-9-sought.png` });
  await seek.fill('activity');
  await panel.locator('.nav-item', { hasText: 'Activity' }).first().waitFor({ timeout: 5000 }).catch(async (err) => {
    await page.screenshot({ path: `${out}/members-x-activity.png` });
    throw new Error(`${err.message}: ${await panel.locator('.nav').innerText()}`);
  });
  expect('the line not to be drawn where nothing under it is found', (await part(page).count()) === 0);
  await seek.press('Escape');
  await lists(page, '+Members', 'folded as it was once the box is emptied');
  await membersLine(page).click();

  // ── By the boxes, with a menu open ──────────────────────────────────────
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await crowded(page, `at ${width} wide`);
    await opened(page, 'helper');
    await page.keyboard.press('Escape');
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  const edge = page.locator('.side-edge');
  const grip = await edge.boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + 300);
  await page.mouse.down();
  await page.mouse.move(60, grip.y + 300, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 180);
  await crowded(page, 'with the panel as narrow as it goes');
  await opened(page, 'scout');
  await page.screenshot({ path: `${out}/members-10-narrow.png` });
  await page.keyboard.press('Escape');
  expect(`Ada's page not to have been read again, was ${loads.ada} times`, loads.ada === 0);

  // ── What Nell is offered: nothing she may not do, and an agent's lines as her Agents tab has them ──
  const desk = await signedIn('nell@example.com');
  await desk.locator('.rail').waitFor();
  await toResources(desk);
  await membersLine(desk).click();
  await lists(desk, '−Members | Nell (you) | Ada [1] | helper (agent) | loose (agent) | scout (agent (external)) | shop-worker (agent) | idle (stopped)', "on Nell's page");
  const nellsAda = (await opened(desk, 'Ada')).join(' ');
  await desk.keyboard.press('Escape');
  expect(`no Permissions… on Nell's menus, got "${nellsAda}"`, nellsAda === 'Details Message');
  const nellsHelper = (await opened(desk, 'helper')).join(' ');
  await desk.keyboard.press('Escape');
  expect(`no Stop and no Remove… offered to Nell, got "${nellsHelper}"`, !/Stop|Remove/.test(nellsHelper));
  // Her Members page: the same people, and no inviting, which is the owner's.
  await pageOfMembers(desk, { people: 'Nell (you) | Ada', invites: false, shots: 'members-15-nell' });
  await agentsPage(desk, out);
  await desk.locator('.agent-line[data-agent="helper"] .row-more').click();
  await menu(desk).waitFor();
  const onPage = (await desk.evaluate(MENU)).join(' ');
  await desk.keyboard.press('Escape');
  expect(`helper's lines in Nell's menu the Agents tab's own, "${nellsHelper}" and "${onPage}"`, nellsHelper.endsWith(onPage.replace(/^Edit…/, '/Edit…')));
  await desk.context().close();

  // ── A phone's width, by touch ───────────────────────────────────────────
  const phone = await signedIn('nell@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await toResources(phone, 'tap');
  if ((await membersLine(phone).getAttribute('aria-expanded')) !== 'true') await membersLine(phone).tap();
  await lists(phone, '−Members | Nell (you) | Ada [1] | helper (agent) | loose (agent) | scout (agent (external)) | shop-worker (agent) | idle (stopped)', "on Nell's phone");
  await crowded(phone, "at a phone's width");
  await opened(phone, 'Ada', 'tap');
  await phone.screenshot({ path: `${out}/members-11-phone.png` });
  await choose(phone, 'Message', 'tap');
  await phone.locator('.channel > .split .message', { hasText: 'is the quay mended?' }).waitFor();
  await phone.locator('.channel .view-head').getByRole('img', { name: 'Conversation' }).waitFor();
  await phone.screenshot({ path: `${out}/members-12-phone-open.png` });
  // The Members page on her phone, by touch: People and Agents, nothing over anything else and nothing sideways.
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await toResources(phone, 'tap');
  await pageOfMembers(phone, { people: 'Nell (you) | Ada', invites: false, shots: 'members-16-phone' }, 'tap');

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
