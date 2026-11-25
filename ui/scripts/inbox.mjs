// The inbox: the box of things addressed to whoever is signed in.
//
//   DAEMON=../target/debug/factory node scripts/inbox.mjs [out-dir]
//
// Two people: Ada, who owns the factory and says things from a terminal,
// and Nell, on a page that is never read again. The inbox is a place of
// the rail, directly below Resources, by the boxes, with nothing of it
// left at the head of any side panel. With nothing addressed to her it
// says no count and the box says that it is empty. Named in channels, in
// a thread and in a channel of another workspace, she is told on the
// rail how many wait, whichever panel shows, with nothing on the rail
// moved by the count. Pressed, the side panel is the box, as the rail's
// other places make it theirs, and another place pressed makes it that
// place's; pressed again it puts the panel away, as they do; and a page
// read again shows the box where it was left showing. The box lists
// each with who said it, where and its words, the latest first, nothing
// over anything else at three widths. The list holds still while it is
// pointed at, with what came counted at its head. A line pressed shows
// its channel at the message and reads every line that opens that
// channel; one said in a thread opens the thread beside it; one of a
// channel of another workspace opens as a tab where she is and moves
// the panel to no other workspace. A line is put away, and all of them
// read at once. A channel made private without her takes its line from
// her list and her count, and gives it back when she is added. On a
// phone the rail comes with the side panel, as it does for Resources and
// Channels, the inbox is pressed there and the box has the screen as the
// side panel does; what brings the panel says something waits. The owner's own
// page shows nothing of any of it. The environment variables are those
// of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'inbox-data');
const home = join(out, 'inbox-home');
const hers = join(out, 'inbox-home-nell');

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
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  for (const name of ['general', 'lobby']) await cli('channel', 'create', name);
  // A second workspace, with a channel of its own: the panel is of the first, which is the usual one.
  await cli('workspace', 'create', 'Back', 'office');
  await cli('channel', 'create', 'ops', '--workspace', 'back-office');
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
  // Somebody's page, signed in: each in a browser of their own, so that each has their own session.
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
  // Nell's page is opened once, and is never read again from here to the end.
  const page = await signedIn('nell@example.com');
  const owners = await signedIn('ada@example.com');
  const channels = page.getByRole('navigation', { name: 'Channels' });
  await channels.waitFor();
  await owners.getByRole('navigation', { name: 'Channels' }).waitFor();
  let loads = 0;
  page.on('load', () => (loads += 1));

  // The inbox's place on the rail says how many wait: it is called by that, for whoever hears it read out.
  const tray = (on, unread) => on.locator('.rail .rail-inbox').and(on.getByRole('button', { name: unread ? `Inbox, ${unread} unread` : 'Inbox', exact: true }));
  const waits = async (on, unread, why) => {
    await tray(on, unread)
      .waitFor({ timeout: 8000 })
      .catch(async () => {
        throw new Error(`expected the rail's inbox to say ${unread} unread ${why}, got "${await on.locator('.rail .rail-inbox').getAttribute('aria-label')}"`);
      });
    const count = (await on.locator('.rail .rail-inbox .inbox-count').textContent()).trim();
    expect(`the rail's count to be ${unread || 'nothing'} ${why}, got "${count}"`, count === (unread ? String(unread) : ''));
    expect('nothing of the inbox at the head of a side panel', (await on.locator('.sidebar .brand .rail-inbox, .sidebar .brand .inbox-count, .sidebar .brand .inbox-tray').count()) === 0);
  };
  const rail = (on, name) => on.locator('.rail').getByRole('button', { name, exact: true });
  // The rail's places by their boxes, in their order: the inbox directly below Resources and over Extensions, as big as the others,
  // in the rail, and its count over its own corner and no other place.
  const railed = async (on, what) => {
    const found = await on.evaluate(() => {
      const strip = document.querySelector('.rail').getBoundingClientRect();
      const places = [...document.querySelectorAll('.rail > .rail-button')].map((one) => ({ name: one.getAttribute('aria-label'), box: one.getBoundingClientRect() }));
      const count = document.querySelector('.rail .rail-inbox .inbox-count');
      const counted = count && count.textContent.trim() ? count.getBoundingClientRect() : null;
      return { strip: { left: strip.left, right: strip.right }, places, counted };
    });
    const names = found.places.map((one) => one.name.replace(/, \d+ unread$/, ''));
    const at = (name) => found.places[names.indexOf(name)];
    expect(`${what}: the inbox to be directly below Resources and over Extensions, got ${JSON.stringify(names)}`, names.indexOf('Inbox') === names.indexOf('Resources') + 1 && names.indexOf('Extensions') === names.indexOf('Inbox') + 1);
    // The places of the side panel in their order (Topics just after the channels), each as big as the others, one under the next.
    const order = ['Channels', 'Topics', 'Resources', 'Inbox', 'Extensions'];
    expect(`${what}: the rail's places to be ${order.join(', ')}, got ${JSON.stringify(names)}`, JSON.stringify(names.slice(0, order.length)) === JSON.stringify(order));
    order.slice(1).forEach((name, nth) => {
      const [up, down] = [at(order[nth]).box, at(name).box];
      expect(`${what}: ${name} to stand under ${order[nth]}, as big as it is and inside the rail, got ${JSON.stringify({ up, down })}`, down.top >= up.bottom - 0.5 && Math.abs(down.left - up.left) < 0.5 && Math.abs(down.width - up.width) < 0.5 && Math.abs(down.height - up.height) < 0.5 && down.left >= found.strip.left - 0.5 && down.right <= found.strip.right + 0.5);
    });
    const [over, inbox, under] = [at('Resources').box, at('Inbox').box, at('Extensions').box];
    expect(`${what}: the inbox to stand under Resources and over Extensions, as big as they are, got ${JSON.stringify({ over, inbox, under })}`, inbox.top >= over.bottom - 0.5 && under.top >= inbox.bottom - 0.5 && Math.abs(inbox.left - over.left) < 0.5 && Math.abs(inbox.width - over.width) < 0.5 && Math.abs(inbox.height - over.height) < 0.5);
    expect(`${what}: the inbox to be inside the rail`, inbox.left >= found.strip.left - 0.5 && inbox.right <= found.strip.right + 0.5);
    if (found.counted) {
      const c = found.counted;
      expect(`${what}: the count to be inside the rail, got ${JSON.stringify(c)}`, c.left >= found.strip.left - 0.5 && c.right <= found.strip.right + 0.5);
      for (const other of found.places.filter((one) => !one.name.startsWith('Inbox'))) {
        const crosses = Math.min(c.right, other.box.right) - Math.max(c.left, other.box.left) > 0.5 && Math.min(c.bottom, other.box.bottom) - Math.max(c.top, other.box.top) > 0.5;
        expect(`${what}: the count not to lie over ${other.name}`, !crosses);
      }
      expect(`${what}: the count to be over the inbox's own corner`, c.left >= inbox.left - 0.5 && c.top >= inbox.top - 0.5 && c.top < inbox.top + inbox.height / 2);
    }
    return found.places.map((one) => one.box);
  };
  const box = page.getByRole('navigation', { name: 'Inbox' });
  // What the box lists, as it is read: each line by its words, one that waits after `*`.
  const READ = `[...document.querySelectorAll('nav[aria-label="Inbox"] .inbox-line')]
    .map((line) => (line.classList.contains('unread') ? '*' : '') + line.querySelector('.inbox-words').textContent)
    .join(' | ')`;
  const lists = async (on, wanted, why) => {
    const there = on.waitForFunction(`(${READ}) === ${JSON.stringify(wanted)}`, null, { timeout: 8000 });
    await there.catch(async () => {
      throw new Error(`expected the box to list "${wanted}" ${why}, got "${await on.evaluate(READ)}"`);
    });
  };
  const line = (within, words) => within.locator('.inbox-line', { hasText: words });
  // The boxes of what `selector` picks inside `within`: none lies over another, and each is inside `within`.
  const apart = async (within, selector, what) => {
    const found = await within.evaluate((frame, picked) => {
      const edge = frame.getBoundingClientRect();
      const boxes = [...frame.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < edge.left - 0.5 || one.box.right > edge.right + 0.5) wrong.push(`${one.said} leaves it`);
        for (const two of boxes.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth, within: frame.scrollWidth - frame.clientWidth };
    }, selector);
    expect(`${what}: something to be there`, found.count > 1);
    expect(`${what}: nothing over anything else or outside its place, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px of the page and ${found.within}px of itself`, found.sideways <= 0 && found.within <= 0);
  };
  const HEAD = '.brand > span:first-child, .brand button, .inbox-says, .inbox-all';
  const LINE = '.inbox-from, .inbox-when, .inbox-where, .inbox-words, .inbox-away';
  const laidOut = async (on, what) => {
    const side = on.getByRole('navigation', { name: 'Inbox' });
    await apart(side, HEAD, `the head of the box ${what}`);
    for (const one of await side.locator('.inbox-line').all()) await apart(one, LINE, `a line of the box ${what}`);
    await apart(side.locator('.inbox-lines'), '.inbox-line', `the lines of the box ${what}`);
  };
  // The id of the message that says `words`, as a terminal reads the channel.
  const said = async (channel, words) => {
    const read = JSON.parse((await cli('read', channel, '--json')).stdout);
    return read.messages.find((message) => message.body.includes(words)).id;
  };

  // ── Nothing addressed to her ────────────────────────────────────────────
  await waits(page, 0, 'with nothing addressed to her');
  const without = await railed(page, 'with nothing waiting');
  await page.screenshot({ path: `${out}/inbox-1-empty.png` });
  await tray(page, 0).click();
  await box.waitFor();
  expect('the panel to be the box', (await channels.count()) === 0 && (await page.locator('nav.sidebar').count()) === 1);
  expect('the inbox to be the place of the rail that is pressed', (await tray(page, 0).getAttribute('aria-pressed')) === 'true' && (await rail(page, 'Channels').getAttribute('aria-pressed')) === 'false');
  expect(`an empty box to say so, got "${await box.locator('.inbox-lines').innerText()}"`, (await box.locator('.inbox-none').innerText()).startsWith('Nothing is here.'));
  expect(`its head to say nothing waits, got "${await box.locator('.inbox-says').innerText()}"`, (await box.locator('.inbox-says').innerText()) === 'Nothing unread');
  expect('nothing to read while nothing waits', await box.getByRole('button', { name: 'Mark all as read' }).isDisabled());
  expect('no way back of its own: the rail is that', (await box.getByRole('button', { name: /^Back to/ }).count()) === 0);
  // Pressed again, the panel is put away, as it is by any place of the rail; pressed once more, it is the box again.
  await tray(page, 0).click();
  await page.locator('.shell.side-closed').waitFor();
  expect('the box to be put away with the panel', (await box.count()) === 0);
  await tray(page, 0).click();
  await box.waitFor();
  // Another place pressed, the panel is that place's.
  await rail(page, 'Channels').click();
  await channels.waitFor();
  expect('Channels pressed to put the box away', (await box.count()) === 0);

  // ── Named: told on the rail, whichever panel shows ──────────────────────
  await cli('post', 'general', '@nell', 'the', 'quince', 'is', 'in');
  await waits(page, 1, 'once something names her');
  const withOne = await railed(page, 'with one waiting');
  expect(`nothing on the rail to move as the count comes, got ${JSON.stringify({ without, withOne })}`, JSON.stringify(without) === JSON.stringify(withOne));
  await page.mouse.move(900, 500);
  await page.screenshot({ path: `${out}/inbox-2-rail.png` });
  await cli('post', 'general', '--thread', await said('general', 'quince'), '@nell', 'and', 'the', 'greengage');
  await cli('post', 'lobby', '@nell', 'a', 'damson', 'with a good many words after it, so that what a line says of it is more than a line of the panel is wide and has to give way');
  await cli('post', 'ops', '@nell', 'the', 'sloe');
  // What names nobody, and what names somebody else, puts nothing in her box.
  await cli('post', 'general', 'nobody', 'is', 'named');
  await cli('post', 'general', '@ada', 'herself');
  await waits(page, 4, 'once four things name her');
  await rail(page, 'Resources').click();
  await page.getByRole('navigation', { name: 'Resources' }).waitFor();
  await waits(page, 4, 'with Resources showing as with Channels');
  await rail(page, 'Extensions').click();
  await page.getByRole('navigation', { name: 'Extensions' }).waitFor();
  await waits(page, 4, 'with Extensions showing');
  await rail(page, 'Channels').click();
  await channels.waitFor();
  // The side panel folded away, the rail stays, and says it still.
  await page.getByRole('button', { name: 'Collapse the side panel' }).click();
  await sideSettled(page);
  await page.locator('.shell.side-closed').waitFor();
  await waits(page, 4, 'with the side panel folded away');
  await railed(page, 'with the side panel folded away');
  await page.screenshot({ path: `${out}/inbox-2-rail-folded.png` });
  await page.getByRole('button', { name: 'Expand the side panel' }).first().click();
  await sideSettled(page);
  await channels.waitFor();

  // ── The box in the panel ────────────────────────────────────────────────
  await tray(page, 4).click();
  const all = '*@nell the sloe | *@nell a damson with a good many words after it, so that what a line says of it is more than a line of the panel is wide and has to give way | *@nell and the greengage | *@nell the quince is in';
  await lists(page, all, 'the latest first');
  const where = await box.locator('.inbox-where').allInnerTexts();
  expect(`each line to say where it was said, got ${JSON.stringify(where)}`, JSON.stringify(where) === JSON.stringify(['#ops', '#lobby', '#general, in a thread', '#general']));
  const from = await box.locator('.inbox-from').allInnerTexts();
  expect(`each line to say who said it, got ${JSON.stringify(from)}`, from.every((name) => name === 'Ada'));
  expect(`its head to say how many wait, got "${await box.locator('.inbox-says').innerText()}"`, (await box.locator('.inbox-says').innerText()) === '4 unread');
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `at ${width} wide`);
    await railed(page, `at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // Words longer than a line of the panel run to a second, and no further.
  const tall = await line(box, 'a damson').locator('.inbox-words').evaluate((words) => ({ lines: Math.round(words.clientHeight / parseFloat(getComputedStyle(words).lineHeight)), more: words.scrollHeight > words.clientHeight + 1 }));
  expect(`a long message to be read on two lines of its line and cut there, got ${JSON.stringify(tall)}`, tall.lines === 2 && tall.more);
  await page.mouse.move(900, 500);
  await page.screenshot({ path: `${out}/inbox-3-list.png` });

  // ── Held still while it is pointed at ───────────────────────────────────
  const first = line(box, 'the sloe');
  const before = await first.boundingBox();
  await first.hover();
  await cli('post', 'lobby', '@nell', 'the', 'rowan');
  await waits(page, 5, 'while the list is pointed at');
  await box.locator('.inbox-says', { hasText: '5 unread · 1 new' }).waitFor();
  const cut = await box.locator('.inbox-says').evaluate((says) => says.scrollWidth - says.clientWidth);
  expect(`what the head says of what came to be read whole, ${cut}px of it is cut off`, cut <= 0);
  const held = await first.boundingBox();
  expect(`nothing to move under the pointer as something comes, the first line was at ${before.y} and is at ${held.y}`, before.y === held.y && before.height === held.height);
  await lists(page, all, 'while it is pointed at');
  await page.screenshot({ path: `${out}/inbox-4-held.png` });
  await page.mouse.move(900, 500);
  await lists(page, `*@nell the rowan | ${all}`, 'once the pointer has left it');
  expect(`its head to say what waits and no more, got "${await box.locator('.inbox-says').innerText()}"`, (await box.locator('.inbox-says').innerText()) === '5 unread');

  // ── A line opened ───────────────────────────────────────────────────────
  const space = () => page.locator('.space-said').first().innerText();
  const shown = await space();
  await line(box, 'the quince is in').locator('.inbox-open').click();
  const view = page.locator('.view[data-focused="true"]');
  await view.locator('.channel .view-head h1', { hasText: 'general' }).waitFor();
  const flashed = view.locator(`.message[data-id="${await said('general', 'quince')}"]`);
  await flashed.waitFor();
  expect('the message to be marked where the channel is shown', ((await flashed.getAttribute('style')) ?? '').includes('accent-soft'));
  // Every line that opens the channel is read by its coming into view: the one in its thread with it.
  await waits(page, 3, 'once a channel two of them open came into view');
  await lists(page, '*@nell the rowan | *@nell the sloe | *@nell a damson with a good many words after it, so that what a line says of it is more than a line of the panel is wide and has to give way | @nell and the greengage | @nell the quince is in', 'with the two read where they stood');
  expect('the panel to go on showing the box', (await box.count()) === 1);
  await page.mouse.move(900, 500);
  await page.screenshot({ path: `${out}/inbox-5-opened.png` });
  // One said in a thread: the thread is open beside its channel.
  await line(box, 'and the greengage').locator('.inbox-open').click();
  const thread = page.locator('.context-thread .thread');
  await thread.locator('.message', { hasText: '@nell and the greengage' }).waitFor();
  await page.screenshot({ path: `${out}/inbox-6-thread.png` });
  // One of a channel of another workspace: a tab where she is, and the panel is of the workspace it was of.
  await line(box, 'the sloe').locator('.inbox-open').click();
  await view.locator('.channel .view-head h1', { hasText: 'ops' }).waitFor();
  await waits(page, 2, 'once the channel of another workspace came into view');
  expect(`the panel to stay of ${shown}, got ${await space()}`, (await space()) === shown);
  const tabs = await page.locator('.tab').allInnerTexts();
  expect(`both channels to be tabs of the set she is in, got ${JSON.stringify(tabs)}`, tabs.some((tab) => tab.includes('general')) && tabs.some((tab) => tab.includes('ops')));

  // ── Put away, and all read ──────────────────────────────────────────────
  await line(box, 'a damson').getByRole('button', { name: /^Put away/ }).click();
  await waits(page, 1, 'once one that waited was put away');
  await lists(page, '*@nell the rowan | @nell the sloe | @nell and the greengage | @nell the quince is in', 'with the one put away gone');
  const kept = (await nell('inbox', '--all')).stdout;
  expect(`what was put away to be hers still, got ${kept}`, /away\s+#lobby\s+\[Ada\] @nell a damson/.test(kept));
  await box.getByRole('button', { name: 'Mark all as read' }).click();
  await waits(page, 0, 'once all of it was read');
  await lists(page, '@nell the rowan | @nell the sloe | @nell and the greengage | @nell the quince is in', 'with nothing waiting');
  expect('nothing to read once nothing waits', await box.getByRole('button', { name: 'Mark all as read' }).isDisabled());

  // ── A channel made private without her ──────────────────────────────────
  await cli('post', 'lobby', '@nell', 'the', 'medlar');
  await waits(page, 1, 'once something names her again');
  await cli('channel', 'keep', 'lobby');
  await waits(page, 0, 'once the channel was kept from her');
  await lists(page, '@nell the sloe | @nell and the greengage | @nell the quince is in', 'with nothing of a channel kept from her');
  const lobby = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.find((channel) => channel.title === 'lobby').id;
  await cli('channel', 'add', lobby, 'Nell');
  await waits(page, 1, 'once she was added to it');
  await lists(page, '*@nell the medlar | @nell the rowan | @nell the sloe | @nell and the greengage | @nell the quince is in', 'with its lines back');
  expect(`a private channel to be said by what it is called, got ${JSON.stringify(await box.locator('.inbox-where').allInnerTexts())}`, (await line(box, 'the medlar').locator('.inbox-where').innerText()) === 'lobby');
  await laidOut(page, 'with a private channel in it');
  expect(`Nell's page never to have been read again, was ${loads} times`, loads === 0);

  // The owner's page shows nothing of it: the box there is the owner's own, which nothing was put in.
  await waits(owners, 0, "on the owner's page, of what was addressed to somebody else");
  await tray(owners, 0).click();
  await owners.getByRole('navigation', { name: 'Inbox' }).locator('.inbox-none').waitFor();
  // Which place of the rail was showing is kept as the others are: read again, the panel is the box still.
  await owners.reload();
  await owners.getByRole('navigation', { name: 'Inbox' }).locator('.inbox-none').waitFor();
  expect('the inbox to be the place pressed once the page was read again', (await tray(owners, 0).getAttribute('aria-pressed')) === 'true');

  // ── A phone's width ─────────────────────────────────────────────────────
  const phone = await signedIn('nell@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  // The side panel starts put away there, and the rail with it: what brings them says that something waits.
  await phone.locator('.tabs-side.tabs-side-waiting').waitFor();
  expect('no rail while the panel is away on a phone', !(await phone.locator('.rail').isVisible()));
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await phone.getByRole('navigation', { name: 'Channels' }).waitFor();
  await waits(phone, 1, 'on a phone');
  await railed(phone, 'on a phone');
  await phone.screenshot({ path: `${out}/inbox-7-phone-rail.png` });
  await tray(phone, 1).tap();
  const small = phone.getByRole('navigation', { name: 'Inbox' });
  await small.waitFor();
  await lists(phone, '*@nell the medlar | @nell the rowan | @nell the sloe | @nell and the greengage | @nell the quince is in', 'on a phone');
  await laidOut(phone, 'on a phone');
  const edges = await small.evaluate((side) => {
    const [panel, lines] = [side.getBoundingClientRect(), side.querySelector('.inbox-lines').getBoundingClientRect()];
    return { top: panel.top, bottom: window.innerHeight - panel.bottom, right: window.innerWidth - panel.right, inside: lines.right <= panel.right + 0.5 };
  });
  expect(`the box to have the screen as the side panel does, got ${JSON.stringify(edges)}`, edges.top <= 0.5 && edges.bottom <= 0.5 && edges.right >= 0 && edges.inside);
  await phone.screenshot({ path: `${out}/inbox-8-phone-list.png` });
  await line(small, 'the medlar').locator('.inbox-open').tap();
  await phone.locator('.channel .view-head h1', { hasText: 'lobby' }).waitFor();
  await phone.locator('.channel .message', { hasText: '@nell the medlar' }).waitFor();
  expect('the panel to be put away from over the page it opened', (await small.count()) === 0);
  await phone.screenshot({ path: `${out}/inbox-9-phone-opened.png` });
  // Read there, it waits nowhere: the page at the usual width says so with nothing read again.
  await waits(page, 0, 'once it was read on the phone');
  await lists(page, '@nell the medlar | @nell the rowan | @nell the sloe | @nell and the greengage | @nell the quince is in', 'once it was read on the phone');
  expect(`Nell's page never to have been read again, was ${loads} times`, loads === 0);

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
