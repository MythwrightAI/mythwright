// A conversation: a message written to one or several people and agents,
// in the inbox of each.
//
//   DAEMON=../target/debug/factory node scripts/conversed.mjs [out-dir]
//
// Two people, each on a page of their own that is never read again: Nell,
// who writes, and Ada, who owns the factory and is written to. Nell opens
// her inbox and the mark at its head that writes a message: the dialog
// lists who it can be to, each saying who it is and never herself or a
// bot; an agent the daemon will not have in a conversation is said why,
// in the daemon's sentence, with the dialog staying as it was written;
// sent, the conversation opens as a tab called by its subject, with an
// envelope where a channel has its `#`, and is in no list of channels.
// It arrives in Ada's inbox, counted on her rail, with nothing read by hand, as one line that
// says what it is about, who is in it and the latest thing said; opened,
// it has been read; answered, the count on the other page follows, and
// a conversation that is showing has read what comes. One of them opens
// it as a channel, behind a question that says what goes with it and
// offers the workspace showing first, and it is listed for both among
// their channels. Nothing of the dialog, the line or the conversation's
// head lies over anything else or scrolls sideways, at three widths and
// a phone's. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { toolsAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'conversed-data');
const home = join(out, 'conversed-home');
const hers = join(out, 'conversed-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
// No walls: an agent this factory starts is then in no private channel, and so in no conversation, which the daemon says in a sentence this reads.
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off'], {
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
  // An agent the factory starts, and one it does not: each is listed saying which it is.
  await cli('agent', 'add', 'helper');
  await cli('agent', 'invite', 'scout');
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
  // Each page is opened once, and is never read again from here to the end.
  const page = await signedIn('nell@example.com');
  const owners = await signedIn('ada@example.com');
  const loads = { nell: 0, ada: 0 };
  page.on('load', () => (loads.nell += 1));
  owners.on('load', () => (loads.ada += 1));
  for (const one of [page, owners]) await one.getByRole('navigation', { name: 'Channels' }).waitFor();

  // The inbox is a place of the rail, which says how many wait; and the side panel is that place's or another's.
  const tray = (on, unread) => on.locator('.rail .rail-inbox').and(on.getByRole('button', { name: unread ? `Inbox, ${unread} unread` : 'Inbox', exact: true }));
  const waits = async (on, unread, why) => {
    await tray(on, unread)
      .waitFor({ timeout: 8000 })
      .catch(async () => {
        throw new Error(`expected the rail's inbox to say ${unread} unread ${why}, got "${await on.locator('.rail .rail-inbox').getAttribute('aria-label')}"`);
      });
  };
  const toChannels = (on) => on.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).click();
  // What a panel lists of channels, as it is read: each by what it is called, a private one after `lock:`.
  const CHANNELS = `[...document.querySelectorAll('nav[aria-label="Channels"] .grouped-channel > div > .nav-row .nav-item')]
    .map((item) => (item.querySelector('.kept-lock') ? 'lock:' : '') + item.querySelector('.truncate').textContent)
    .join(' ')`;
  const channelsAre = async (on, wanted, whose) => {
    const there = on.waitForFunction(`(${CHANNELS}) === ${JSON.stringify(wanted)}`, null, { timeout: 8000 });
    await there.catch(async () => {
      throw new Error(`expected ${whose} panel to list "${wanted}", got "${await on.evaluate(CHANNELS)}"`);
    });
  };
  // What a box lists, as it is read: each line by where it is of and its words, one that waits after `*`.
  const LINES = `[...document.querySelectorAll('nav[aria-label="Inbox"] .inbox-line')]
    .map((line) => (line.classList.contains('unread') ? '*' : '') + line.querySelector('.inbox-where').textContent + ': ' + line.querySelector('.inbox-words').textContent)
    .join(' | ')`;
  const lists = async (on, wanted, why) => {
    const there = on.waitForFunction(`(${LINES}) === ${JSON.stringify(wanted)}`, null, { timeout: 8000 });
    await there.catch(async () => {
      throw new Error(`expected the box to list "${wanted}" ${why}, got "${await on.evaluate(LINES)}"`);
    });
  };
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
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth };
    }, selector);
    expect(`${what}: something to be there`, found.count > 1);
    expect(`${what}: nothing over anything else or outside its place, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px`, found.sideways <= 0);
  };
  const WIDTHS = [1280, 1000, 800];
  const COMPOSE = 'h2, .conversed-field > label, .conversed-to, .conversed-offered, form > label > input, form > label > textarea, .conversed-note, .row button';
  const CHOSEN = '.conversed-chosen, #conversed-to';
  const LINE = '.inbox-from, .inbox-when, .inbox-where, .inbox-words, .inbox-away';
  const HEAD = '.view-head > *:not(.grow)';
  const say = async (on, words) => {
    const box = on.locator('.channel > .split .composer textarea');
    await box.fill(words);
    await on.keyboard.press('Enter');
    await on.locator('.channel > .split .message', { hasText: words }).waitFor();
  };
  const head = (on) => on.locator('.view[data-focused="true"] .channel .view-head');

  // ── Written from the inbox's head ───────────────────────────────────────
  await waits(page, 0, 'with nothing addressed to her');
  await tray(page, 0).click();
  const box = page.getByRole('navigation', { name: 'Inbox' });
  await box.waitFor();
  expect(`an empty box to say what waits there, got "${await box.locator('.inbox-none').innerText()}"`, (await box.locator('.inbox-none').innerText()).includes('A message written to you'));
  await box.getByRole('button', { name: 'Write a message' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByText('New message', { exact: true }).waitFor();
  // Who it can be to: the people and the agents here, each saying who it is; never herself, and never a bot.
  const offered = dialog.getByRole('option');
  await offered.first().waitFor();
  const who = (await offered.allInnerTexts()).map((line) => line.replace(/\s+/g, ' ').trim());
  expect(`who it can be to, each saying who it is, got ${JSON.stringify(who)}`, JSON.stringify(who) === JSON.stringify(['Ada person', 'helper agent', 'scout external agent']));
  expect('the caret to be where the first of them is typed', await dialog.locator('#conversed-to').evaluate((input) => document.activeElement === input));
  expect('nothing to send while it is to nobody', await dialog.getByRole('button', { name: 'Send', exact: true }).isDisabled());
  await page.screenshot({ path: `${out}/conversed-0-compose.png` });
  // Typed, the list is those called so; Enter takes the first and sends nothing.
  await dialog.locator('#conversed-to').fill('ad');
  expect(`what is typed to narrow the list, got ${JSON.stringify(await offered.allInnerTexts())}`, (await offered.count()) === 1);
  await page.keyboard.press('Enter');
  await dialog.locator('.conversed-chosen', { hasText: 'Ada' }).waitFor();
  expect('the dialog to be there still', (await dialog.count()) === 1);
  await dialog.getByRole('option', { name: /^helper/ }).click();
  await dialog.getByLabel('Subject').fill('The quay');
  await dialog.getByLabel('Message').fill('who mends it?');
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(dialog, COMPOSE, `the dialog that writes a message at ${width}`);
    await apart(dialog.locator('.conversed-to'), CHOSEN, `who a message is to at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/conversed-1-written.png` });
  // The daemon will not have that agent in one: its sentence, where it was asked, and what was written as it was.
  await dialog.getByRole('button', { name: 'Send', exact: true }).click();
  const why = dialog.getByRole('alert');
  await why.waitFor();
  expect(`the daemon's sentence where it was asked, got "${await why.innerText()}"`, (await why.innerText()).includes('an agent with no wall is in no channel kept to its members'));
  expect('no mark a terminal reads commands by in it', !(await why.innerText()).includes('`'));
  expect('what was written to be there still', (await dialog.getByLabel('Subject').inputValue()) === 'The quay' && (await dialog.getByLabel('Message').inputValue()) === 'who mends it?');
  expect('who it was to to be there still', (await dialog.locator('.conversed-chosen').count()) === 2);
  await apart(dialog, `${COMPOSE}, [role="alert"]`, 'the dialog with what the daemon said');
  await page.screenshot({ path: `${out}/conversed-2-refused.png` });
  expect('nothing to have been made', !(await nell('inbox', '--all')).stdout.includes('quay') && !(await cli('inbox', '--all')).stdout.includes('quay'));
  // Put away and opened again, it holds what was written; without the agent, it is sent.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });
  await box.getByRole('button', { name: 'Write a message' }).click();
  expect('what was written to be kept while the dialog was away', (await dialog.getByLabel('Message').inputValue()) === 'who mends it?');
  await dialog.getByRole('button', { name: 'Take helper off' }).click();
  await dialog.getByRole('button', { name: 'Send', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });

  // It opens as a tab called by its subject, with an envelope and no `#`, and says what was written.
  const subject = head(page).locator('h1');
  await subject.waitFor();
  expect(`its head to say what it is about, got "${await subject.innerText()}"`, (await subject.innerText()).trim() === 'The quay');
  expect('its head to say that it is a conversation', (await subject.getByRole('img', { name: 'Conversation' }).count()) === 1);
  const tabOf = page.locator('.tab[aria-selected="true"]');
  expect('its tab to say that it is a conversation, by an envelope and no #', (await tabOf.getByRole('img', { name: 'Conversation' }).count()) === 1 && !(await tabOf.innerText()).includes('#'));
  await page.locator('.channel > .split .message', { hasText: 'who mends it?' }).waitFor();
  expect('no id to be shown for a name', !(await page.locator('body').innerText()).includes('ch_'));
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(head(page), HEAD, `the head of a conversation at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.mouse.move(900, 500);
  await page.screenshot({ path: `${out}/conversed-3-open.png` });
  // Whoever wrote it has no line of it, and it is in no list of channels.
  await waits(page, 0, 'for whoever wrote it');
  await toChannels(page);
  await channelsAre(page, 'general', "Nell's");
  // A dialog whose draft was sent begins empty.
  await tray(page, 0).click();
  await box.getByRole('button', { name: 'Write a message' }).click();
  expect('the next message to begin with nothing written', (await dialog.getByLabel('Message').inputValue()) === '' && (await dialog.locator('.conversed-chosen').count()) === 0);
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });

  // ── Arriving on the other page, which reads nothing again ───────────────
  await waits(owners, 1, 'once something was written to her');
  await channelsAre(owners, 'general', "Ada's");
  await tray(owners, 1).click();
  const theirs = owners.getByRole('navigation', { name: 'Inbox' });
  await lists(owners, '*The quay · with Nell: who mends it?', 'with the conversation written to her');
  const line = theirs.locator('.inbox-line');
  expect('the line to be a conversation', (await line.getAttribute('data-kind')) === 'conversation');
  expect(`the line to say who said the latest thing, got "${await line.locator('.inbox-from').innerText()}"`, (await line.locator('.inbox-from').innerText()) === 'Nell');
  for (const width of WIDTHS) {
    await owners.setViewportSize({ width, height: 860 });
    await apart(line, LINE, `a conversation's line at ${width}`);
  }
  await owners.setViewportSize({ width: 1280, height: 860 });
  await owners.mouse.move(900, 500);
  await owners.screenshot({ path: `${out}/conversed-4-line.png` });
  // Opened, it shows the conversation at what was said and has been read.
  await line.locator('.inbox-open').click();
  // The pointer off the list, which holds still while it is pointed at.
  await owners.mouse.move(900, 500);
  await head(owners).locator('h1', { hasText: 'The quay' }).waitFor();
  await owners.locator('.channel > .split .message', { hasText: 'who mends it?' }).waitFor();
  await waits(owners, 0, 'once the conversation was opened');
  await lists(owners, 'The quay · with Nell: who mends it?', 'read where it stood');

  // ── Answered: the count on the other page follows ───────────────────────
  // Nell is looking at something else, so what comes waits for her.
  await toChannels(page);
  await page.getByRole('navigation', { name: 'Channels' }).locator('.nav-item', { hasText: 'general' }).click();
  await head(page).locator('h1', { hasText: 'general' }).waitFor();
  await say(owners, 'the medlar yard does');
  await waits(page, 1, 'once it was answered');
  expect('nothing to wait among her channels for it', (await page.locator('.rail-button.waiting').count()) === 0);
  await say(owners, 'by Tuesday');
  await waits(page, 1, 'with two things said there: it is one line');
  await tray(page, 1).click();
  await lists(page, '*The quay · with Ada: by Tuesday', 'with the latest thing said');
  // A conversation that comes into view has been read; and one that is showing reads what comes.
  await page.locator('.tab', { hasText: 'The quay' }).click();
  await page.locator('.channel > .split .message', { hasText: 'by Tuesday' }).waitFor();
  await waits(page, 0, 'once the conversation came into view');
  await say(page, 'the damson gate too');
  await owners.locator('.channel > .split .message', { hasText: 'the damson gate too' }).waitFor();
  await lists(owners, 'The quay · with Nell: the damson gate too', 'read as it came, the conversation showing');
  await waits(owners, 0, 'with the conversation showing');
  await owners.mouse.move(900, 500);
  await owners.screenshot({ path: `${out}/conversed-5-answered.png` });

  // ── Opened as a channel, from its tab of the Tools panel: its head has no gear ──
  expect('no gear at the head of a conversation', (await head(owners).getByRole('button', { name: 'This conversation' }).count()) === 0);
  if ((await owners.locator('.context').count()) === 0) await owners.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(owners).waitFor();
  const tools = owners.locator('.context');
  await tools.getByRole('tab', { name: /^Channel\b/ }).click();
  const settings = tools.getByRole('region', { name: 'Settings of this channel' });
  await settings.waitFor();
  const lines = (await settings.locator('button').allInnerTexts()).map((one) => one.trim());
  expect(`what is done with a conversation, got ${JSON.stringify(lines)}`, lines.includes('Open as a channel…') && lines.includes('Leave') && lines.filter((one) => one === 'Leave').length === 1);
  await settings.getByText('Open as a channel…', { exact: true }).click();
  const asking = owners.getByRole('dialog');
  await asking.getByText('Open as a channel?', { exact: true }).waitFor();
  expect('to be told what goes with it', (await asking.innerText()).includes('Everything said in it so far goes with it') && (await asking.innerText()).includes('the 2 in it: Nell, Ada'));
  expect(`it to be called what it was about unless said, got "${await asking.getByLabel('Name').inputValue()}"`, (await asking.getByLabel('Name').inputValue()) === 'The quay');
  const places = (await asking.locator('.belongs-choice').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
  expect(`the workspace showing to be offered first, and chosen, got ${JSON.stringify(places)}`, places[0] === "Ada's workspace workspace" && places.includes('Front workspace') && (await asking.locator('.belongs-choice input').first().isChecked()));
  await asking.getByLabel('Name').fill('Quay works');
  for (const width of WIDTHS) {
    await owners.setViewportSize({ width, height: 860 });
    await apart(asking, 'h2, p, label > input:not([type="radio"]), .belongs-choice, .row button', `the question that opens it as a channel at ${width}`);
  }
  await owners.setViewportSize({ width: 1280, height: 860 });
  await owners.screenshot({ path: `${out}/conversed-6-as-channel.png` });
  await asking.getByRole('button', { name: 'Open as a channel', exact: true }).click();
  await asking.waitFor({ state: 'detached' });
  // It is a private channel for both from here, on pages that read nothing again: listed, with its lock.
  await toChannels(owners);
  await channelsAre(owners, 'general lock:Quay works', "Ada's");
  await toChannels(page);
  await channelsAre(page, 'general lock:Quay works', "Nell's");
  await head(owners).locator('h1', { hasText: 'Quay works' }).waitFor();
  expect('its head to say that it is private now', (await head(owners).getByRole('img', { name: 'Private' }).count()) === 1 && (await head(owners).getByRole('button', { name: 'This conversation' }).count()) === 0);
  expect('everything said so far to be there', (await owners.locator('.channel > .split .message').count()) === 4);
  await owners.screenshot({ path: `${out}/conversed-7-channel.png` });
  expect(`neither page to have been read again, were ${JSON.stringify(loads)}`, loads.nell === 0 && loads.ada === 0);

  // ── A phone's width ─────────────────────────────────────────────────────
  await cli('to', '@nell', '--subject', 'The gate', 'who', 'hangs', 'it,', 'and', 'when', 'is', 'the', 'latch', 'to', 'be', 'put', 'on', 'the', 'garden', 'side?');
  const phone = await signedIn('nell@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  // The side panel starts put away there, and lies over the page once it is brought.
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await phone.getByRole('navigation', { name: 'Channels' }).waitFor();
  await waits(phone, 1, 'on a phone');
  await tray(phone, 1).tap();
  const small = phone.getByRole('navigation', { name: 'Inbox' });
  await small.waitFor();
  const shown = small.locator('.inbox-line', { hasText: 'who hangs it' });
  await shown.waitFor();
  await apart(shown, LINE, "a conversation's line on a phone");
  await phone.screenshot({ path: `${out}/conversed-8-phone-line.png` });
  await small.getByRole('button', { name: 'Write a message' }).tap();
  const asked = phone.getByRole('dialog');
  await asked.getByRole('option').first().waitFor();
  await asked.getByRole('option', { name: /^Ada/ }).tap();
  await asked.getByRole('option', { name: /^scout/ }).tap();
  await asked.getByLabel('Subject').fill('The latch');
  await asked.getByLabel('Message').fill('it is on the garden side');
  await apart(asked, COMPOSE, 'the dialog that writes a message on a phone');
  await apart(asked.locator('.conversed-to'), CHOSEN, 'who a message is to on a phone');
  const tall = await asked.evaluate((dialog) => dialog.getBoundingClientRect().bottom <= window.innerHeight + 0.5 && dialog.getBoundingClientRect().top >= -0.5);
  expect('the dialog to be no taller than the screen', tall);
  await phone.screenshot({ path: `${out}/conversed-9-phone-compose.png` });
  // To a person and an agent the factory does not start: sent, and shown with the panel put away.
  await asked.getByRole('button', { name: 'Send', exact: true }).tap();
  await asked.waitFor({ state: 'detached' });
  await phone.locator('.channel .view-head h1', { hasText: 'The latch' }).waitFor();
  await phone.locator('.channel > .split .message', { hasText: 'it is on the garden side' }).waitFor();
  await apart(phone.locator('.channel .view-head'), HEAD, 'the head of a conversation on a phone');
  await phone.screenshot({ path: `${out}/conversed-10-phone-open.png` });
  await waits(owners, 1, "on the owner's page, once the phone wrote to her");

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
