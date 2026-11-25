// The Topics panel: a forum's topics, a place of the rail of
// their own just after the channels, out of the Messaging panel where they
// were a node beside the channels, which holds the channels
// alone again and is called Channels.
//
//   DAEMON=../target/debug/factory node scripts/topics.mjs [out-dir]
//
// Two people, each on a page of their own that is never read again: Ada,
// who owns the factory, and Nell, who registered by an invitation. Ada's
// browser kept what it kept while the panel was Messaging: that panel
// shown, its Channels node folded, its Topics node unfolded and its closed
// topics unfolded. Her page shows the channels' panel, called Channels on
// the rail, at its head and for whoever hears it read out, listing the
// channels directly with no node and no heading over them, and what the
// two nodes were kept as is dropped. The rail's places are Channels,
// Topics, Resources, the inbox and Extensions, each as big as the others;
// Alt and 5 shows the topics, which their place pressed again puts away.
// Ada, with no topic yet, is offered the first topic and Nell is told there
// is none; topics made from a terminal (which sets the topics up, as whoever
// manages the factory), one of a project of the workspace, one answered
// and one closed arrive on both pages with nothing read again: each a wiki
// of its own, listed in the order they were made, each with how it stands where it is answered or closed and
// its project's name, faintly, and the closed one under Closed saying how
// many, unfolded on Ada's page as her browser kept it and folded on Nell's.
// One of another workspace is not listed, and comes into the list as the
// panel is made of that workspace at its foot. What is typed in the
// panel's box finds a topic by its title, by the first line of its
// summary, by its status and by its project, holding Closed open though it
// was folded, and one line where nothing says it; Escape leaves the folds
// as they were. Collapse all folds Closed, kept so when the page is read
// again, and the Channels panel's leaves it; Expand all unfolds it. New
// wiki at the panel's head makes one at once in the workspace,
// asking no place, opens it with its title in a field in its line, which a
// title that is nothing leaves Untitled and Enter saves, and the tab is
// called what it was called, on Nell's page listed with nothing read
// again. The wiki made has no status: none at its page's head, where Add
// status is offered, and no Mark as the answer under its reply; Open
// chosen there shows it and brings Mark as the answer, and Not tracked
// takes both away again, its line in the panel saying nothing of it. It
// is a thread, its reply drawn on the page with no tabs; Add summary at its
// head brings a band that, written and then emptied, goes; Write an article
// brings Article and Talk, Talk counting its reply, and what is written is
// an article still once the page is read again, on the tab it was on.
// Nothing of either panel lies over anything else, leaves it or scrolls
// sideways at 1280, 1000 and 800 wide, with the panel dragged as
// narrow as it goes, and at a phone's, touched; the lines are one tree.
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';
import { joined } from './joined.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'topics-data');
const home = join(out, 'topics-home');
const hers = join(out, 'topics-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', 'Shop', '--bare');
  for (const name of ['general', 'payments-desk']) await cli('channel', 'create', name);

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
  // What Ada's browser kept while the panel was Messaging: that panel showing, its Channels node folded,
  // its Topics node unfolded and its closed topics unfolded. Put there once, before the page is first read.
  const KEPT_BEFORE = ['channels:node:folded', 'channels:topics', 'channels:topics:closed'];
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }, before = null) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    if (before)
      await context.addInitScript((kept) => {
        if (sessionStorage.getItem('kept-before')) return;
        sessionStorage.setItem('kept-before', '1');
        localStorage.setItem('client.side', 'channels');
        localStorage.setItem('client.turned', JSON.stringify(kept));
      }, before);
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  const page = await signedIn('ada@example.com', undefined, KEPT_BEFORE);
  const theirs = await signedIn('nell@example.com');
  const railOf = (on, name) => on.locator('.rail').getByRole('button', { name, exact: true });
  const toPanel = async (on, name) => {
    const rail = railOf(on, name);
    await rail.waitFor();
    if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
    await on.getByRole('navigation', { name, exact: true }).waitFor();
  };
  const panel = page.getByRole('navigation', { name: 'Topics', exact: true });
  const other = theirs.getByRole('navigation', { name: 'Topics', exact: true });
  const channelsPanel = page.getByRole('navigation', { name: 'Channels', exact: true });

  // What the Topics panel draws, as one line: each topic with what is said beside it, Closed with its fold, and what
  // is said where there is nothing.
  const DRAWN = (name) => `(() => {
    const panel = document.querySelector('nav[aria-label="${name}"]');
    if (!panel) return 'no panel';
    const said = [];
    for (const one of panel.querySelectorAll('.nav h3, .grouped-channel > div > .nav-row .nav-item .truncate, .topic-line .nav-item, .nav-item[aria-expanded], .topics-first, .nav-note, .nav-who')) {
      if (one.closest('.grouped-channel .tree')) continue;
      if (one.matches('h3')) said.push('h3:' + one.textContent.trim());
      else if (one.matches('.topic-line .nav-item')) said.push('¶' + one.querySelector('.truncate').textContent + (one.querySelector('.nav-hint') ? '(' + one.querySelector('.nav-hint').textContent + ')' : ''));
      else if (one.matches('.nav-item[aria-expanded]')) said.push((one.getAttribute('aria-expanded') === 'true' ? '▾' : '▸') + one.querySelector('.truncate').textContent + (one.querySelector('.nav-hint') ? '[' + one.querySelector('.nav-hint').textContent + ']' : ''));
      else said.push(one.textContent.replace('+', '').trim());
    }
    return said.join(' | ');
  })()`;
  const drawn = (on, name = 'Topics') => on.evaluate(DRAWN(name));
  const until = async (on, wanted, what, name = 'Topics') => {
    const there = on.waitForFunction(`(${DRAWN(name)}) === ${JSON.stringify(wanted)}`, null, { timeout: 10000 });
    await there.catch(async () => {
      throw new Error(`expected ${what}: "${wanted}", got "${await drawn(on, name)}"`);
    });
  };
  const closedFold = (nav) => nav.locator('.nav-item[aria-expanded]', { hasText: 'Closed' });
  const laid = async (on, what, name = 'Topics') => {
    const wrong = await pageCrowded(on, `nav[aria-label="${name}"]`);
    expect(`nothing of the ${name} panel over anything else ${what}: ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await on.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`nothing to scroll sideways ${what}, got ${sideways}px`, sideways <= 0);
    const loose = await joined(on, `nav[aria-label="${name}"]`);
    expect(`the ${name} panel to be one tree ${what}: ${loose.join('; ')}`, loose.length === 0);
  };
  const shown = async (on, locator, what) => {
    await locator.waitFor({ timeout: 10000 }).catch(() => {
      throw new Error(`expected ${what}`);
    });
  };
  const keptNow = (on) => on.evaluate(() => JSON.parse(localStorage.getItem('client.turned') ?? '[]'));

  // ── The channels' panel: called Channels, the channels directly in it, and what the nodes were kept as dropped ──
  await toPanel(page, 'Channels');
  expect('the rail to say Channels and pressed, as Ada\'s browser kept the panel', (await railOf(page, 'Channels').getAttribute('aria-pressed')) === 'true');
  expect('no place of the rail called Messaging', (await railOf(page, 'Messaging').count()) === 0);
  expect('the panel to say what it is at its head', (await channelsPanel.locator('.brand > span').first().innerText()).trim() === 'Channels');
  expect('the panel\'s box to look in the channels', (await channelsPanel.getByRole('searchbox', { name: 'Search these channels', exact: true }).count()) === 1);
  await until(page, 'general | payments-desk', 'the channels directly in the panel, with no heading over them and no topic', 'Channels');
  await laid(page, 'as it opens', 'Channels');
  await page.screenshot({ path: `${out}/topics-01-channels.png` });

  // ── The rail: Topics just after Channels, as big as the rest; Alt and 5, and pressed again put away ──
  const rail = await page.evaluate(() => {
    const strip = document.querySelector('.rail').getBoundingClientRect();
    return { strip: { left: strip.left, right: strip.right }, places: [...document.querySelectorAll('.rail > .rail-button')].map((one) => ({ name: one.getAttribute('aria-label').replace(/, \d+ unread$/, ''), box: one.getBoundingClientRect().toJSON() })) };
  });
  const order = ['Channels', 'Topics', 'Resources', 'Inbox', 'Extensions'];
  expect(`the rail's places to be ${order.join(', ')}, got ${rail.places.map((one) => one.name).join(', ')}`, rail.places.slice(0, 5).map((one) => one.name).join() === order.join());
  rail.places.slice(1, 5).forEach((one, nth) => {
    const up = rail.places[nth].box;
    expect(`${one.name} under ${rail.places[nth].name}, as big and in the rail, got ${JSON.stringify({ up, it: one.box })}`, one.box.top >= up.bottom - 0.5 && Math.abs(one.box.width - up.width) < 0.5 && Math.abs(one.box.height - up.height) < 0.5 && Math.abs(one.box.left - up.left) < 0.5 && one.box.right <= rail.strip.right + 0.5);
  });
  expect('Topics drawn by a few strokes of its own, and no icon of the set', (await railOf(page, 'Topics').locator('svg.sign path').count()) === 1);
  await page.keyboard.press('Alt+5');
  await panel.waitFor();
  expect('Alt and 5 to show the topics, their place of the rail pressed', (await railOf(page, 'Topics').getAttribute('aria-pressed')) === 'true' && (await railOf(page, 'Channels').getAttribute('aria-pressed')) === 'false');
  expect('the panel to say what it is at its head', (await panel.locator('.brand > span').first().innerText()).trim() === 'Topics');
  expect('New topic, Collapse all and Expand all at its head', (await panel.locator('.brand').getByRole('button', { name: 'New topic', exact: true }).count()) === 1 && (await panel.locator('.brand').getByRole('button', { name: 'Collapse all', exact: true }).count()) === 1 && (await panel.locator('.brand').getByRole('button', { name: 'Expand all', exact: true }).count()) === 1);
  expect('its box under its head', (await panel.getByRole('searchbox', { name: 'Search these topics', exact: true }).count()) === 1);
  expect('the workspace foot at its foot', (await panel.locator('.space-foot').count()) === 1);
  await until(page, 'Start the first topic', 'Ada offered the first topic');
  await railOf(page, 'Topics').click();
  await panel.waitFor({ state: 'detached' });
  await sideSettled(page);
  expect('the panel put away by its place pressed again', (await page.locator('nav.sidebar').count()) === 0);
  await railOf(page, 'Topics').click();
  await panel.waitFor();
  await sideSettled(page);
  await toPanel(theirs, 'Topics');
  await until(theirs, 'No wikis here yet', 'Nell told there are none, and offered nothing she may not do');
  expect('no New topic for Nell', (await other.getByRole('button', { name: 'New topic', exact: true }).count()) === 0);

  // ── Topics made from a terminal, arriving on both pages ─────────────────
  await cli('topic', 'make', 'Why the build is slow on Windows', '--body', 'It takes ten minutes.');
  await cli('topic', 'make', 'Payment provider options', '--project', 'shop', '--status', 'open');
  await cli('topic', 'make', 'An old question');
  await cli('record', 'set', '@-topics-3', 'status=closed');
  await cli('workspace', 'create', 'Back office');
  const back = (await cli('workspace', 'list')).stdout.split('\n').find((line) => line.includes('Back office')).split(/\s+/)[0];
  await cli('topic', 'make', 'Where the ledgers are', '--workspace', back);
  const said = (await cli('topic', 'reply', 'Payment provider options', 'The one with the lowest fees.')).stdout.trim().replace('said at ', '');
  await cli('topic', 'answer', 'Payment provider options', said);
  await cli('record', 'set', '@-topics-1', 'summary=It is the linker, on Windows alone.');
  const listed = '¶Why the build is slow on Windows | ¶Payment provider options(answered · Shop) | ▸Closed[1]';
  const open = `${listed.replace('▸Closed[1]', '▾Closed[1]')} | ¶An old question`;
  await until(page, open, "the topics of the workspace, each a wiki in the order they were made, each with how it stands and its project, and Closed unfolded as Ada's browser kept it under the Topics node");
  await until(theirs, listed, "on Nell's page, never read again, Closed folded as it starts");
  await laid(page, 'with its topics');
  await page.mouse.move(900, 700);
  await page.screenshot({ path: `${out}/topics-02-the-panel.png` });
  await closedFold(other).click();
  await until(theirs, open, 'Closed unfolded by its arrow on Nell\'s page');
  await closedFold(other).click();
  await until(theirs, listed, 'and folded again');

  // ── The panel's box ──────────────────────────────────────────────────────
  const box = panel.getByRole('searchbox', { name: 'Search these topics', exact: true });
  await closedFold(panel).click();
  await until(page, listed, 'Closed folded on Ada\'s page');
  // Kept anew as the fold is turned: what the Messaging panel's two nodes were kept as was dropped as it was read, and is gone.
  const kept = await keptNow(page);
  expect(`what the Messaging panel's two nodes were kept as gone, and Closed folded, got ${JSON.stringify(kept)}`, !kept.includes('channels:node:folded') && !kept.includes('channels:topics') && !kept.includes('channels:topics:closed'));
  const typed = async (words, wanted, what) => {
    await box.fill(words);
    await until(page, wanted, what);
  };
  await typed('slow windows', '¶Why the build is slow on Windows', 'a topic found by its title');
  await typed('linker', '¶Why the build is slow on Windows', 'a topic found by the first line of its summary');
  await typed('answered', '¶Payment provider options(answered · Shop)', 'a topic found by how it stands');
  await typed('shop', '¶Payment provider options(answered · Shop)', 'a topic found by its project');
  await typed('old question', '▾Closed[1] | ¶An old question', 'a closed one under Closed, held open');
  expect('the held arrow to fold nothing', (await closedFold(panel).getAttribute('aria-disabled')) === 'true');
  await typed('topics', 'Nothing listed here says “topics”.', 'nothing found by the panel being called Topics');
  await typed('ledgers', 'Nothing listed here says “ledgers”.', 'a topic of another workspace not found, and one line saying so');
  await laid(page, 'with something typed');
  await box.press('Escape');
  await until(page, listed, 'the folds as they were once the box is emptied');

  // ── Another workspace ───────────────────────────────────────────────────
  const toSpace = async (name) => {
    await panel.locator('.space-foot .space-pick').click();
    await page.getByRole('menuitemradio', { name: new RegExp(`^${name}(:|$)`) }).click();
    await panel.locator('.space-foot .space-said strong', { hasText: name }).waitFor();
  };
  await toSpace('Back office');
  await until(page, '¶Where the ledgers are', "another workspace's topics alone");
  await toSpace("Ada's workspace");
  await until(page, listed, 'the first workspace again');

  // ── Expand all and Collapse all, each the panel's own ───────────────────
  await panel.getByRole('button', { name: 'Expand all', exact: true }).click();
  await until(page, open, 'Closed unfolded by Expand all');
  await toPanel(page, 'Channels');
  await channelsPanel.getByRole('button', { name: 'Collapse all', exact: true }).click();
  await toPanel(page, 'Topics');
  await until(page, open, "Closed left unfolded by the Channels panel's Collapse all");
  await panel.getByRole('button', { name: 'Collapse all', exact: true }).click();
  await until(page, listed, 'Closed folded by Collapse all');
  await page.reload();
  await panel.waitFor();
  await until(page, listed, 'the Topics panel showing and Closed kept folded when the page is read again');

  // ── New topic, at the panel's head: made at once, its title written in its line ─────────
  await panel.locator('.brand').getByRole('button', { name: 'New topic', exact: true }).click();
  const field = panel.getByRole('textbox', { name: 'Page title', exact: true });
  await field.waitFor();
  await page.waitForFunction((said) => [...document.querySelectorAll('.topic-page-title')].some((one) => (one.value ?? one.innerText) === said), 'Untitled');
  expect('no dialog asked for it', (await page.getByRole('dialog').count()) === 0);
  await page.waitForFunction(() => document.activeElement?.matches('.wiki-title-field') && document.activeElement.selectionStart === 0 && document.activeElement.selectionEnd === 'Untitled'.length);
  await page.screenshot({ path: `${out}/topics-03-new-wiki.png` });
  await page.keyboard.type('Card fees on refunds');
  await page.keyboard.press('Enter');
  await field.waitFor({ state: 'detached' });
  await page.getByRole('tab', { name: /Card fees on refunds/ }).first().waitFor();
  // Its title a field to whoever may write it, words to anybody else.
  await page.waitForFunction((said) => [...document.querySelectorAll('.topic-page-title')].some((one) => (one.value ?? one.innerText) === said), 'Card fees on refunds');
  const made = listed.replace(' | ▸Closed[1]', ' | ¶Card fees on refunds | ▸Closed[1]');
  await until(page, made, 'the wiki made listed last, where it was made');
  await until(theirs, made, "it listed on Nell's page, never read again");
  await page.screenshot({ path: `${out}/topics-04-made.png` });

  // ── A status, chosen at the page's head: none to begin with, Open, then Not tracked ──────────
  const wiki = page.locator('.topic-page');
  const head = wiki.locator('.topic-page-head');
  const marks = wiki.getByRole('button', { name: 'Mark as the answer', exact: true });
  await cli('topic', 'reply', 'Card fees on refunds', 'Only on partial refunds.');
  await shown(page, wiki.locator('.topic-reply', { hasText: 'Only on partial refunds.' }), 'a reply on the page made with no status');
  expect('no status at its head, and none of its replies marked as the answer', (await head.locator('.topic-status').count()) === 0 && (await marks.count()) === 0);
  const pick = async (from, name, picture) => {
    await from.click();
    const line = page.getByRole('menuitemradio', { name, exact: true });
    await line.waitFor();
    if (picture) await page.screenshot({ path: `${out}/${picture}` });
    await line.click();
  };
  await pick(head.getByRole('button', { name: 'Add status', exact: true }), 'Open', 'topics-07-choosing.png');
  await shown(page, head.locator('.topic-status', { hasText: 'open' }), 'Open chosen at its head, and shown there');
  await shown(page, marks.first(), 'Mark as the answer under its reply once it is open');
  await page.screenshot({ path: `${out}/topics-08-status.png` });
  await laid(page, 'with a status chosen');
  const crowdedHead = await pageCrowded(page, '.topic-page-head');
  expect(`nothing of the page's head over anything else with a status chosen: ${crowdedHead.join('; ')}`, crowdedHead.length === 0);
  expect(`the terminal to read it open, got ${(await cli('topic', 'list')).stdout}`, /open\s+Card fees on refunds/.test((await cli('topic', 'list')).stdout));
  await pick(head.locator('.topic-status'), 'Not tracked');
  await head.getByRole('button', { name: 'Add status', exact: true }).waitFor();
  await marks.first().waitFor({ state: 'detached' });
  expect('no status at its head once it is not tracked', (await head.locator('.topic-status').count()) === 0);
  await until(page, made, 'its line in the panel with no status beside it');

  // ── A thread: no article, its replies drawn directly; a summary added and emptied; an article written ──────────
  expect('a wiki made with nothing in it a thread: no tabs, its reply drawn on the page', (await wiki.getAttribute('data-shows')) === 'thread' && (await wiki.getByRole('tablist').count()) === 0);
  expect('no summary drawn, and none of its post', (await wiki.locator('.topic-summary, .topic-post').count()) === 0);
  await head.getByRole('button', { name: 'Add summary', exact: true }).click();
  const band = wiki.locator('.topic-summary');
  await shown(page, band.locator('.ProseMirror'), 'Add summary bringing a summary band to write in');
  await band.locator('.ProseMirror').click();
  await page.keyboard.type('Only partial refunds cost a fee.');
  await page.keyboard.press('Control+s');
  await band.locator('.topic-doc.unsaved').waitFor({ state: 'detached' });
  await shown(page, band.locator('.ProseMirror', { hasText: 'Only partial refunds cost a fee.' }), 'the summary as it was saved');
  expect('Add summary offered no more', (await head.getByRole('button', { name: 'Add summary', exact: true }).count()) === 0);
  await page.screenshot({ path: `${out}/topics-09-summary.png` });
  await band.locator('.ProseMirror').click();
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Backspace');
  await page.keyboard.press('Control+s');
  await band.waitFor({ state: 'detached' });
  await head.getByRole('button', { name: 'Add summary', exact: true }).waitFor();
  const summed = JSON.parse((await cli('topic', 'show', 'Card fees on refunds', '--json')).stdout).topic.data?.summary;
  expect(`the terminal to read no summary, got ${JSON.stringify(summed)}`, !summed);
  await head.getByRole('button', { name: 'Write an article', exact: true }).click();
  const tabs = wiki.getByRole('tablist', { name: 'Article, talk and history' });
  await shown(page, tabs.getByRole('tab', { name: 'Article', exact: true, selected: true }), 'Write an article bringing the tabs, on Article');
  expect('Talk to count its reply', (await tabs.getByRole('tab', { name: /^Talk/ }).innerText()).replace(/\s+/g, ' ') === 'Talk 1');
  await wiki.locator('.topic-post .ProseMirror').click();
  await page.keyboard.type('Card fees are charged on partial refunds only.');
  await page.keyboard.press('Control+s');
  await wiki.locator('.topic-post .topic-doc.unsaved').waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/topics-10-article.png` });
  await tabs.getByRole('tab', { name: /^Talk/ }).click();
  await shown(page, wiki.locator('.topic-talk .topic-reply', { hasText: 'Only on partial refunds.' }), 'its reply on Talk');
  await page.waitForFunction(() => location.hash === '#talk');
  await page.reload();
  await shown(page, page.locator('.topic-page .topic-tab[aria-selected="true"][data-part="talk"]'), 'an article on Talk still once the page is read again, its address saying so');
  await page.locator('.topic-page').getByRole('tab', { name: 'Article', exact: true }).click();
  await shown(page, page.locator('.topic-page .topic-post', { hasText: 'partial refunds only' }), 'the article it was written as, read again');
  await panel.waitFor();

  // ── By the boxes ────────────────────────────────────────────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await laid(page, `at ${width} wide`);
    await toPanel(page, 'Channels');
    await laid(page, `at ${width} wide`, 'Channels');
    await toPanel(page, 'Topics');
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  const edge = await page.locator('.side-edge').boundingBox();
  await page.mouse.move(edge.x + edge.width / 2, edge.y + 300);
  await page.mouse.down();
  await page.mouse.move(60, edge.y + 300, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 180);
  await laid(page, 'dragged as narrow as it goes');
  await page.screenshot({ path: `${out}/topics-05-narrow.png` });

  const phone = await signedIn('nell@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  const topicsRail = railOf(phone, 'Topics');
  if ((await topicsRail.getAttribute('aria-pressed')) !== 'true') await topicsRail.tap();
  const small = phone.getByRole('navigation', { name: 'Topics', exact: true });
  await small.locator('.topic-line', { hasText: 'Payment provider options' }).waitFor();
  await laid(phone, "at a phone's width");
  await phone.screenshot({ path: `${out}/topics-06-phone.png` });
  await small.locator('.topic-line .nav-item', { hasText: 'Payment provider options' }).tap();
  await phone.waitForFunction((said) => [...document.querySelectorAll('.topic-page-title')].some((one) => (one.value ?? one.innerText) === said), 'Payment provider options');

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
