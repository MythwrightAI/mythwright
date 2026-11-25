// Somebody named with @ in what is said on a card or any record, and the
// card's drawer showing what was said apart from what happened.
//
//   DAEMON=../target/debug/factory node scripts/tagged.mjs [out-dir]
//
// Two people: Ada, who owns the factory, on a page of her own and from a
// terminal; and Nell, on a page that is never read again. A card's drawer
// has one row of tabs, Details, Comments, History and Attachments, and no second row
// under its form: Details first and showing, the form with nothing said
// beside it; Comments the box with what was said, every line of which is
// something said, and no line of what happened; History what happened,
// with no box and nothing said. Which shows is kept by the tab the drawer
// is beside: another tab shown and the board again, the drawer is back on
// History. In the box, an @ offers the people and the agents from
// outside, never Ada herself, narrowed by what is typed, under the box,
// moving nothing and over nothing of the drawer's edge; the arrows go
// through it, Enter puts the one marked in, and Escape puts the list away
// and leaves the drawer. Said, Nell is told on the rail of her page, and
// her box says who said it, on what and the words; pressed, it opens the
// card on its board with its drawer on Comments at that comment, marked
// and in view, and the line is read. Said on a row of another table from a
// terminal, it opens the row's form over its table, the same way; said
// on a note kept private, she is told nothing. In a channel, a card's line
// opens the card in the Tools panel beside it at that comment, marked and
// in view, with no drawer; and with the card showing there already,
// another line of it marks its own comment. A row of another table has
// Comments and History as two tabs under its form; whoever said each thing with their picture beside their name,
// pressed beside a channel read about in its Tools panel. A question asked on the card is said in its Comments
// by whoever asked, naming her, which tells her; Details says only whom it waits on and leads to the box.
// In a topic's reply box an @ offers Nell as a person and the agents, one
// the factory starts said to answer when named, and never Ada; posted, each
// name of somebody is drawn underlined in the reply, a name of nobody stays
// as it was written, and pointed at or reached by the keyboard each shows
// who it is; Nell is told on the rail of her page.
// Nothing of the drawer lies over anything else, at three
// widths and a phone's, plainly and with
// `LOOK="size=largest density=compact"`. The environment variables are
// those of walkthrough.mjs. A page of a wiki she follows, changed twice by
// Ada, is one line of her box saying who changed it, on which page and what
// the change did, which opens the page.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest, toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'tagged-data');
const home = join(out, 'tagged-home');
const hers = join(out, 'tagged-home-nell');
const looked = process.env.LOOK ? '-looked' : '';

/** What a fleet attaches does when a card's agent asks: the question said on the card, put to Nell. */
const ASK = `blueprint: ask
permissions: [board, inbox]
steps:
  ask:
    do: emit
    with: { topic: card.asked, card: shop-1, project: shop, asker: builder, question: "Which browsers do we keep?" }
  said:
    needs: [ask]
    do: record.ask
    with: { id: shop-1, says: "Which browsers do we keep?", to: ["@nell"], asker: builder, once: asked }
`;

/** Sprints, whose rows keep what is said on them, and notes kept private. */
const PLAN = `blueprint: plan
permissions: [projects, records]
steps:
  sprints:
    do: collection.ensure
    with:
      name: sprints
      fields: { title: { type: text, required: true } }
      history: [record.*]
  notes:
    do: collection.ensure
    with:
      name: notes
      reads: private:notes
      fields: { title: { type: text, required: true } }
      history: [record.*]
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
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
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await writeFile(join(home, 'plan.bp'), PLAN);
  await cli('apply', join(home, 'plan.bp'), '--project', 'shop');
  await cli('board', 'add', 'shop', 'Fix the login', '--label', 'web');
  await cli('board', 'move', 'shop-1', 'ready');
  await cli('record', 'comment', 'shop-1', 'Safari is the one that fails.');
  await cli('record', 'add', 'shop', 'sprints', 'title=November');
  await cli('record', 'add', 'shop', 'notes', 'title=Pricing');
  await cli('channel', 'create', 'general');
  await cli('topic', 'make', 'Deploying');
  await nell('topic', 'follow', 'Deploying');

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
    lastPage = page;
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const at = (page, hash) => page.evaluate((to) => (location.hash = to), hash);
  const tabOf = (drawer, name) => drawer.getByRole('tab', { name: new RegExp(`^${name}`) });
  const crowded = async (page, what) => {
    const wrong = await pageCrowded(page, '.drawer');
    expect(`nothing of the drawer over anything else ${what}${looked}, got ${wrong.join('; ')}`, wrong.length === 0);
  };

  // Nell's page is opened once, and is never read again from here to the end.
  const theirs = await signedIn('nell@example.com');
  const tray = (unread) => theirs.locator('.rail .rail-inbox').and(theirs.getByRole('button', { name: unread ? `Inbox, ${unread} unread` : 'Inbox', exact: true }));
  await tray(0).waitFor();
  let loads = 0;
  theirs.on('load', () => (loads += 1));

  // ── Ada's card: Details, Comments and History, one row of tabs ──────────
  const page = await signedIn('ada@example.com');
  await at(page, '#/board/shop');
  await page.locator('.board .card', { hasText: 'Fix the login' }).click();
  const drawer = drawerAtRest(page);
  await tabOf(drawer, 'Details').waitFor();
  const names = (await drawer.locator('.card-tabs [role="tablist"] [role="tab"]').allInnerTexts()).map((one) => one.trim());
  expect(`Details, Comments, History and then Attachments in one row, got ${JSON.stringify(names)}`, JSON.stringify(names) === JSON.stringify(['Details', 'Comments', 'History', 'Attachments']));
  expect('one row of tabs, and none under the form', (await drawer.getByRole('tablist').count()) === 1);
  expect('Details showing, with the form', (await tabOf(drawer, 'Details').getAttribute('aria-selected')) === 'true' && (await drawer.locator('form.card-form').getByLabel('Title', { exact: true }).isVisible()));
  expect('nothing said and no box beside the form', (await drawer.getByLabel('Say something on this card').count()) === 0 && (await drawer.locator('.journey-spoken').count()) === 0);
  await crowded(page, 'on Details at 1280');
  await tabOf(drawer, 'Comments').click();
  expect('Comments showing, the form put away', (await tabOf(drawer, 'Comments').getAttribute('aria-selected')) === 'true' && !(await drawer.locator('form.card-form').isVisible()));
  const box = drawer.getByLabel('Say something on this card');
  await box.waitFor();
  await drawer.locator('.journey-spoken', { hasText: 'Safari is the one that fails.' }).waitFor();
  expect('nothing of what happened among the comments', (await drawer.locator('.talked-panel li:not(.journey-spoken)').count()) === 0);
  await crowded(page, 'on Comments at 1280');
  await page.screenshot({ path: `${out}/tagged-0-comments${looked}.png` });
  await tabOf(drawer, 'History').click();
  await drawer.locator('.journey-said', { hasText: 'Ada added it to backlog' }).waitFor();
  expect('no box and nothing said in History', (await box.count()) === 0 && (await drawer.locator('.journey-spoken').count()) === 0);
  await crowded(page, 'on History at 1280');
  await page.screenshot({ path: `${out}/tagged-1-history${looked}.png` });
  // Kept by the tab the drawer is beside: another tab shown and the board again, it is back on History.
  await at(page, '#/reference');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await page.locator('.tab[data-key="board:shop"]').click();
  await drawerAtRest(page).waitFor();
  expect('History still showing once the board is shown again', (await tabOf(drawer, 'History').getAttribute('aria-selected')) === 'true');
  await tabOf(drawer, 'Comments').click();

  // ── Naming somebody with @ ─────────────────────────────────────────────
  await box.click();
  const before = await drawer.boundingBox();
  await page.keyboard.type('@ne');
  const offered = page.locator('.tag-offered');
  await offered.waitFor();
  const offers = (await offered.getByRole('option').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
  expect(`Nell offered and nobody else for "ne", got ${JSON.stringify(offers)}`, offers.length === 1 && offers[0].includes('@nell'));
  // Under the box, over nothing of the drawer's edge, and nothing moved.
  const [boxAt, listAt, drawn] = [await box.boundingBox(), await offered.boundingBox(), await drawer.boundingBox()];
  // Under the box, or over it where the window has no room under it: wholly in the window, over none of the box.
  const clearOf = listAt.y >= boxAt.y + boxAt.height - 0.5 || listAt.y + listAt.height <= boxAt.y + 0.5;
  const inWindow = listAt.y >= 0 && listAt.y + listAt.height <= page.viewportSize().height + 0.5;
  expect(`the list clear of the box, in the window and inside the drawer, got ${JSON.stringify({ boxAt, listAt, drawn })}`, clearOf && inWindow && listAt.x >= drawn.x - 0.5 && listAt.x + listAt.width <= drawn.x + drawn.width + 0.5);
  expect('nothing of the drawer to move as the list comes', JSON.stringify(before) === JSON.stringify(drawn));
  await page.screenshot({ path: `${out}/tagged-2-offered${looked}.png` });
  // Escape puts the list away and leaves the drawer.
  await page.keyboard.press('Escape');
  await offered.waitFor({ state: 'detached' });
  expect('the drawer to stay', (await page.locator('.drawer').count()) === 1);
  // Typed again from nothing, Ada is never offered: she is who writes.
  await box.fill('');
  await page.keyboard.type('@');
  await offered.waitFor();
  const everyone = (await offered.getByRole('option').allInnerTexts()).join(' ');
  expect(`Ada not offered to herself, got ${everyone}`, !everyone.includes('@ada') && everyone.includes('@nell'));
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('Enter');
  await offered.waitFor({ state: 'detached' });
  await page.keyboard.type('the login fails on Safari, can you look?');
  expect(`the name put in by Enter, got "${await box.inputValue()}"`, (await box.inputValue()) === '@nell the login fails on Safari, can you look?');
  await drawer.getByRole('button', { name: 'Say', exact: true }).click();
  await drawer.locator('.journey-spoken', { hasText: 'can you look?' }).waitFor();

  // ── Nell is told, on a page never read again ────────────────────────────
  await tray(1).waitFor();
  await theirs.locator('.rail .rail-inbox').click();
  const line = theirs.locator('.inbox-line[data-kind="commented"]');
  await line.waitFor();
  const said = (await line.innerText()).replace(/\s+/g, ' ');
  expect(`her line to say who said it, on what and the words, got "${said}"`, said.includes('Ada') && said.includes('on Fix the login') && said.includes('@nell the login fails on Safari'));
  const wrong = await pageCrowded(theirs, '.inbox-side');
  expect(`nothing of her box over anything else${looked}, got ${wrong.join('; ')}`, wrong.length === 0);
  await theirs.screenshot({ path: `${out}/tagged-3-told${looked}.png` });
  await line.locator('.inbox-open').click();
  const hersOpen = drawerAtRest(theirs);
  const marked = hersOpen.locator('.journey-spoken.journey-at');
  await marked.waitFor();
  expect('the card opened on its board', (await theirs.locator('.board .card', { hasText: 'Fix the login' }).count()) === 1);
  expect('Comments showing at that comment', (await tabOf(hersOpen, 'Comments').getAttribute('aria-selected')) === 'true' && (await marked.innerText()).includes('can you look?'));
  const markedAt = await marked.boundingBox();
  const view = theirs.viewportSize();
  expect(`the comment in view, got ${JSON.stringify(markedAt)}`, markedAt.y >= 0 && markedAt.y + markedAt.height <= view.height);
  await tray(0).waitFor();
  await theirs.screenshot({ path: `${out}/tagged-4-opened${looked}.png` });

  // ── A row of another table, and a note kept private ─────────────────────
  const [sprint, note] = ['shop-sprints-1', 'shop-notes-1'];
  await cli('record', 'comment', note, '@nell the prices for next year');
  await cli('record', 'comment', sprint, '@nell is this sprint too full?');
  await tray(1).waitFor();
  // The box is the side panel still: pressing the line opened a tab, and left the panel where it was. It holds
  // still while it is pointed at, so the pointer is taken off it first.
  if ((await theirs.locator('.inbox-side').count()) === 0) await theirs.locator('.rail .rail-inbox').click();
  await theirs.mouse.move(640, 500);
  const lines = theirs.locator('.inbox-line[data-kind="commented"]');
  await lines.nth(1).waitFor();
  expect('the private note told her nothing', (await lines.count()) === 2 && (await theirs.locator('.inbox-line', { hasText: 'prices' }).count()) === 0);
  await theirs.locator('.inbox-line', { hasText: 'sprint too full' }).locator('.inbox-open').click();
  const rowOpen = drawerAtRest(theirs);
  await rowOpen.locator('.journey-spoken.journey-at', { hasText: 'sprint too full' }).waitFor();
  // Its form in the one row of tabs at its top, as a card's, on Comments at the comment.
  const rowNames = (await rowOpen.locator('.record-tabs [role="tablist"] [role="tab"]').allInnerTexts()).map((one) => one.trim());
  expect(`the row opened in its form, its one row of tabs Details, Comments, History and Attachments, got ${JSON.stringify(rowNames)}`, JSON.stringify(rowNames) === JSON.stringify(['Details', 'Comments', 'History', 'Attachments']) && (await rowOpen.getByRole('tablist').count()) === 1);
  expect('Comments showing at the comment, with the box', (await tabOf(rowOpen, 'Comments').getAttribute('aria-selected')) === 'true' && (await rowOpen.getByLabel('Say something on this sprint').count()) === 1);
  // Details is the row's fields, Comments put away, and back on Comments the comment is still there.
  await tabOf(rowOpen, 'Details').click();
  await rowOpen.getByRole('button', { name: 'Remove this sprint', exact: true }).waitFor();
  expect('Details showing the form and no comment', (await rowOpen.locator('.journey-spoken').count()) === 0);
  const wrongRow = await pageCrowded(theirs, '.drawer');
  expect(`nothing of the row's form over anything else${looked}, got ${wrongRow.join('; ')}`, wrongRow.length === 0);
  await theirs.screenshot({ path: `${out}/tagged-5b-row-details${looked}.png` });
  await tabOf(rowOpen, 'Comments').click();
  await rowOpen.locator('.journey-spoken', { hasText: 'sprint too full' }).waitFor();
  await theirs.screenshot({ path: `${out}/tagged-5-row${looked}.png` });

  // ── A card's comment opened beside a channel, in the Tools panel ────────
  // In a channel, a card is shown in the Tools panel beside it, as a card named anywhere is: at the comment too.
  await theirs.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).click();
  await theirs.getByRole('navigation', { name: 'Channels' }).locator('.grouped-channel', { has: theirs.locator('.nav-item .truncate', { hasText: /^general$/ }) }).locator('.nav-item').first().click();
  await theirs.getByLabel('Message').waitFor();
  const inPanel = async (words, what) => {
    await tray(1).waitFor();
    // Pressed again, the rail's place would put the box away.
    if ((await theirs.locator('.inbox-side').count()) === 0) await theirs.locator('.rail .rail-inbox').click();
    await theirs.mouse.move(640, 500);
    await theirs.locator('.inbox-line', { hasText: words }).locator('.inbox-open').click();
    const tools = toolsAtRest(theirs);
    const at = tools.locator('.journey-spoken.journey-at', { hasText: words });
    await at.waitFor();
    expect(`${what}: Comments showing at the comment, and it alone marked`, (await tabOf(tools, 'Comments').getAttribute('aria-selected')) === 'true' && (await tools.locator('.journey-at').count()) === 1);
    expect(`${what}: no drawer opened for it`, (await theirs.locator('.drawer').count()) === 0);
    const [box, frame] = [await at.boundingBox(), await tools.boundingBox()];
    expect(`${what}: the comment in view in the panel, got ${JSON.stringify({ box, frame })}`, box.y >= frame.y - 0.5 && box.y + box.height <= frame.y + frame.height + 0.5);
  };
  await cli('record', 'comment', 'shop-1', '@nell one more thing on the login');
  await inPanel('one more thing on the login', 'opened beside the channel');
  await theirs.screenshot({ path: `${out}/tagged-6-panel${looked}.png` });
  // And while that card is showing there, another line of it opens it at its own comment.
  await cli('record', 'comment', 'shop-1', '@nell and the last of it, I promise');
  await inPanel('the last of it', 'with the card showing there already');
  // Whoever said it, with their picture beside their name, pressed beside the channel is read about there, in
  // the Tools panel where the card is open, and no drawer is opened for it.
  const saidBy = toolsAtRest(theirs).locator('.journey-spoken', { hasText: 'the last of it' }).locator('button.journey-by');
  await saidBy.locator('.avatar.face').waitFor();
  await saidBy.click();
  await toolsAtRest(theirs).locator('.context-crumbs', { hasText: 'Ada' }).waitFor();
  expect('no drawer opened for whoever said it', (await theirs.locator('.drawer').count()) === 0);
  expect('her page never read again', loads === 0);

  // ── A page she follows ──────────────────────────────────────────
  // Changed by Ada, the page she follows is one line of her box, saying who, on what and what the change did;
  // changed again it is the same line, raised; pressed, it opens the page.
  for (const body of ['Tag, then push.', 'Tag, then push, then watch.']) await cli('record', 'set', '@-topics-1', `body=${body}`);
  await tray(1).waitFor();
  if ((await theirs.locator('.inbox-side').count()) === 0) await theirs.locator('.rail .rail-inbox').click();
  await theirs.mouse.move(640, 500);
  const follows = theirs.locator('.inbox-line[data-kind="followed"]');
  await follows.filter({ hasText: 'Revised the page' }).waitFor();
  expect('one line of the page, saying where and what was done', (await follows.count()) === 1 && (await follows.locator('.inbox-where', { hasText: 'on Deploying' }).count()) === 1 && (await follows.locator('.inbox-from', { hasText: 'Ada' }).count()) === 1);
  const wrongFollowed = await pageCrowded(theirs, '.inbox-side');
  expect(`nothing of a followed page's line over anything else${looked}, got ${wrongFollowed.join('; ')}`, wrongFollowed.length === 0);
  await theirs.screenshot({ path: `${out}/tagged-6a-followed${looked}.png` });
  await follows.locator('.inbox-open').click();
  await theirs.locator('.topic-page-title').first().waitFor();
  expect('the page she follows opened', (await theirs.locator('.topic-page-title').first().evaluate((one) => one.value ?? one.innerText)) === 'Deploying');
  await tray(0).waitFor();
  await theirs.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).click();
  await theirs.getByRole('navigation', { name: 'Channels' }).locator('.grouped-channel', { has: theirs.locator('.nav-item .truncate', { hasText: /^general$/ }) }).locator('.nav-item').first().click();
  await theirs.getByLabel('Message').waitFor();

  // ── A question asked on a card, put to Nell ──────────────────
  // Something asked on the card is said there by whoever asked, naming whom it is put to: being named is what tells
  // her, and her line opens the card at the question. Details says only whom it waits on, and leads to Comments with
  // the box having the keyboard; there is no second box.
  await writeFile(join(home, 'ask.bp'), ASK);
  await cli('apply', join(home, 'ask.bp'), '--project', 'shop');
  await inPanel('Which browsers do we keep?', 'a question put to her');
  await theirs.screenshot({ path: `${out}/tagged-6b-asked${looked}.png` });
  await tabOf(drawer, 'Details').click();
  const waits = drawer.getByRole('button', { name: 'Waiting on an answer from @nell: answer in Comments', exact: true });
  await waits.waitFor();
  expect('no box to answer in on Details', (await drawer.getByLabel('Your answer').count()) === 0 && (await drawer.getByRole('tabpanel', { name: 'Details' }).getByText('Which browsers').count()) === 0);
  await crowded(page, 'with a question waited on, on Details at 1280');
  await page.screenshot({ path: `${out}/tagged-6c-waits${looked}.png` });
  await waits.click();
  const answering = drawer.getByLabel('Your answer');
  await answering.waitFor();
  await page.waitForFunction(() => document.activeElement?.tagName === 'TEXTAREA' && document.activeElement.closest('.card-answering') !== null);
  expect('Comments showing, with the question in it once', (await tabOf(drawer, 'Comments').getAttribute('aria-selected')) === 'true' && (await drawer.locator('.journey-spoken', { hasText: 'Which browsers' }).count()) === 1);
  await crowded(page, 'answering, on Comments at 1280');
  await page.screenshot({ path: `${out}/tagged-6d-answering${looked}.png` });
  await tabOf(drawer, 'Details').click();

  // ── Nothing over anything else at three widths and a phone's ────────────
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await drawer.waitFor();
    await crowded(page, `at ${width}`);
    // What it waits on, whole inside the drawer.
    const [line, frame] = [await waits.boundingBox(), await drawer.boundingBox()];
    expect(`what it waits on inside the drawer at ${width}, got ${JSON.stringify({ line, frame })}`, line.x >= frame.x - 0.5 && line.x + line.width <= frame.x + frame.width + 0.5);
  }
  await page.screenshot({ path: `${out}/tagged-7-narrow${looked}.png` });
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  await at(phone, '#/board/shop');
  await phone.locator('.board .card', { hasText: 'Fix the login' }).tap();
  const small = drawerAtRest(phone);
  await tabOf(small, 'Details').waitFor();
  await small.getByRole('button', { name: /^Waiting on an answer from @nell/ }).waitFor();
  await crowded(phone, 'on a phone, on Details');
  // Pressed by touch, what it waits on leads to Comments, where it is answered.
  await small.getByRole('button', { name: /^Waiting on an answer from @nell/ }).tap();
  await small.getByLabel('Your answer').waitFor();
  expect('Comments showing on a phone', (await tabOf(small, 'Comments').getAttribute('aria-selected')) === 'true');
  await crowded(phone, 'on a phone, on Comments');
  await tabOf(small, 'History').tap();
  await small.locator('.journey-said', { hasText: 'Ada added it to backlog' }).waitFor();
  await crowded(phone, 'on a phone, on History');
  await phone.screenshot({ path: `${out}/tagged-8-phone${looked}.png` });

  // ── Naming somebody in a topic's replies ─────────────────────────────────
  await cli('agent', 'add', 'scout');
  await cli('topic', 'make', 'Which linker', '--body', 'The build links for minutes.');
  const unread = async () => Number(/(\d+) unread/.exec((await theirs.locator('.rail .rail-inbox').getAttribute('aria-label')) ?? '')?.[1] ?? 0);
  const toldBefore = await unread();
  const forum = await signedIn('ada@example.com');
  // Its post says something, so it is an article: its replies are on its talk, which its address names.
  await forum.goto(`${url}/topic/${encodeURIComponent('Which linker')}#talk`);
  const topicAt = forum.locator('.topic-page');
  const replyBox = topicAt.getByLabel('Reply to the topic', { exact: true });
  await replyBox.click();
  await forum.keyboard.type('@');
  const inTopic = forum.locator('.tag-offered');
  await inTopic.waitFor();
  const offeredThere = (await inTopic.getByRole('option').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
  const nellThere = offeredThere.find((one) => one.includes('@nell'));
  const scoutThere = offeredThere.find((one) => one.includes('@scout'));
  expect(`Nell offered as herself and scout as an agent that answers, never Ada, got ${JSON.stringify(offeredThere)}`, nellThere?.includes('Nell') && scoutThere?.includes('answers when named') && !offeredThere.some((one) => one.includes('@ada')));
  const [boxThere, listThere] = [await replyBox.boundingBox(), await inTopic.boundingBox()];
  expect(`the list clear of the reply box, got ${JSON.stringify({ boxThere, listThere })}`, listThere.y >= boxThere.y + boxThere.height - 0.5 || listThere.y + listThere.height <= boxThere.y + 0.5);
  await forum.screenshot({ path: `${out}/tagged-9-topic-offered${looked}.png` });
  await forum.keyboard.type('ne');
  await forum.keyboard.press('Enter');
  await forum.keyboard.type('and @sc');
  await inTopic.getByRole('option', { name: /@scout/ }).waitFor();
  await forum.keyboard.press('Tab');
  await forum.keyboard.type('which linker is faster? Not @nobody.');
  expect(`both names put in, got "${await replyBox.inputValue()}"`, (await replyBox.inputValue()) === '@nell and @scout which linker is faster? Not @nobody.');
  await topicAt.getByRole('button', { name: 'Post', exact: true }).click();
  const posted = topicAt.locator('.topic-reply-words', { hasText: 'which linker is faster? Not' }).first();
  const nellNamed = posted.locator('.mention', { hasText: '@nell' });
  const scoutNamed = posted.locator('.mention', { hasText: '@scout' });
  await nellNamed.waitFor();
  await scoutNamed.waitFor();
  expect('a name of nobody left as it was written', (await posted.locator('.mention', { hasText: '@nobody' }).count()) === 0 && (await posted.innerText()).includes('Not @nobody.'));
  const decoration = await nellNamed.evaluate((one) => getComputedStyle(one).textDecorationLine);
  expect(`a name of somebody underlined, got ${decoration}`, decoration.includes('underline'));
  const nellSays = await nellNamed.getAttribute('aria-label');
  expect(`a name of somebody to say who it is to whoever hears it, got ${nellSays}`, nellSays === '@nell: Nell, person');
  // Pointed at, the card of who it is.
  await nellNamed.hover();
  const card = forum.locator('.who-card');
  await card.waitFor();
  const cardSays = (await card.innerText()).replace(/\s+/g, ' ');
  expect(`Nell's card to say who she is, got "${cardSays}"`, cardSays.includes('Nell') && cardSays.includes('human'));
  await forum.screenshot({ path: `${out}/tagged-10-topic-named${looked}.png` });
  // Reached by the keyboard, the same of the agent.
  await forum.mouse.move(2, 2);
  await card.waitFor({ state: 'detached' });
  await nellNamed.focus();
  await card.waitFor();
  await forum.keyboard.press('Tab');
  await forum.waitForFunction(() => document.activeElement?.classList.contains('mention') && document.activeElement.firstChild?.textContent === '@scout');
  const agentCard = forum.locator('.who-card', { hasText: 'scout' });
  await agentCard.waitFor();
  const agentSays = (await agentCard.innerText()).replace(/\s+/g, ' ');
  expect(`scout's card to say it is an agent, how it stands and what it runs on, got "${agentSays}"`, /agent/.test(agentSays) && /idle|working|stopped/.test(agentSays) && agentSays.includes('Runs on'));
  expect(`the agent's name to say who it is, got ${await scoutNamed.getAttribute('aria-label')}`, (await scoutNamed.getAttribute('aria-label'))?.startsWith('@scout: scout, '));
  await forum.screenshot({ path: `${out}/tagged-11-topic-agent${looked}.png` });
  // Nell is told, on a page never read again.
  await theirs.waitForFunction((was) => Number(/(\d+) unread/.exec(document.querySelector('.rail .rail-inbox')?.getAttribute('aria-label') ?? '')?.[1] ?? 0) > was, toldBefore);
  expect('her page never read again', loads === 0);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/tagged-failed${looked}.png` }).catch(() => {});
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
