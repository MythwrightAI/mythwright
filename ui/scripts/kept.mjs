// A private channel: one kept to its members.
//
//   DAEMON=../target/debug/factory node scripts/kept.mjs [out-dir]
//
// Two people, each on a page of their own that is never read again: Ada,
// who owns the factory, and Nell, who registered by an invitation. Ada
// makes a channel from the dialog with its Private switch on, and it is
// listed for her with a lock where an open one has its `#`, and for Nell
// not at all; what is said there does not arrive on Nell's page and is
// not in what she searches. An agent the daemon will not have in it is
// said why, in the daemon's sentence, where it was added. Nell is added
// and sees it, with what was said so far; she leaves, and it is gone
// from her page. An open channel is made private by somebody in it, and
// goes from the page of whoever is not. And the owner opens the first
// again, which everybody then lists and reads. Nothing of the dialog,
// of a private channel's row or of what is under its members lies over
// anything else of it or leaves its place, at three widths and a
// phone's. The environment variables are those of walkthrough.mjs.
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
const data = join(out, 'kept-data');
const home = join(out, 'kept-home');
const hers = join(out, 'kept-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
// No walls: an agent of such a factory is in no private channel, which the daemon says in a sentence this reads.
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
  for (const name of ['general', 'lobby']) await cli('channel', 'create', name);
  await cli('agent', 'add', 'helper');
  // One the factory does not start, which is offered as such where somebody is added to a channel.
  await cli('agent', 'invite', 'outsider');
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
  const page = await signedIn('ada@example.com');
  const theirs = await signedIn('nell@example.com');
  const panel = page.getByRole('navigation', { name: 'Channels' });
  const other = theirs.getByRole('navigation', { name: 'Channels' });
  for (const one of [panel, other]) await one.waitFor();

  // What a panel lists, as it is read: each channel by what it is called, a private one after `lock:`.
  const READ = `[...document.querySelectorAll('nav[aria-label="Channels"] .grouped-channel > div > .nav-row .nav-item')]
    .map((item) => (item.querySelector('.kept-lock') ? 'lock:' : '') + item.querySelector('.truncate').textContent)
    .join(' ')`;
  const listed = (nav) => nav.page().evaluate(READ);
  // Waits until a panel lists just that: a page follows what the daemon says, and is not read again.
  const lists = async (nav, wanted, whose) => {
    const there = nav.page().waitForFunction(`(${READ}) === ${JSON.stringify(wanted)}`, null, { timeout: 8000 });
    await there.catch(async () => {
      throw new Error(`expected ${whose} panel to list "${wanted}", got "${await listed(nav)}"`);
    });
  };
  const row = (nav, called) => nav.locator('.grouped-channel', { has: nav.page().locator('.nav-item .truncate', { hasText: new RegExp(`^${called}$`) }) });
  const choose = async (on, label) => {
    const menu = on.getByRole('menu');
    await menu.waitFor();
    await menu.getByText(label, { exact: true }).click();
    await menu.waitFor({ state: 'detached' });
  };
  const offered = async (nav, called) => {
    await row(nav, called).click({ button: 'right' });
    const menu = nav.page().getByRole('menu');
    await menu.waitFor();
    const lines = await menu.locator('button').allInnerTexts();
    // A menu, like a dialog, listens for Escape from a moment after it is drawn.
    await nav.page().waitForTimeout(200);
    await nav.page().keyboard.press('Escape');
    await menu.waitFor({ state: 'detached' });
    return lines.map((line) => line.trim());
  };
  // What `page`'s reader finds, asked of the daemon as their page asks it.
  const finds = (on, words) =>
    on.evaluate(async (asked) => {
      const answer = await fetch(`/api/search?q=${encodeURIComponent(asked)}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      return JSON.stringify(await answer.json());
    }, words);
  const say = async (on, words) => {
    const box = on.locator('.channel > .split .composer textarea');
    await box.fill(words);
    await on.keyboard.press('Enter');
    await on.locator('.channel > .split .message', { hasText: words }).waitFor();
  };
  // The boxes of what `selector` picks inside `within`: none lies over another, and each is inside `within`.
  const apart = async (within, selector, what) => {
    const found = await within.evaluate((box, picked) => {
      const frame = box.getBoundingClientRect();
      const boxes = [...box.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < frame.left - 0.5 || one.box.right > frame.right + 0.5) wrong.push(`${one.said} leaves it`);
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
  const DIALOG = 'h2, label > :not(.grow), .start-with .grow > span, .start-with .switch, .row button, select';

  await lists(panel, 'general lobby', "Ada's");
  await lists(other, 'general lobby', "Nell's");

  // ── Made from the dialog ────────────────────────────────────────────────
  await panel.locator('.brand').getByRole('button', { name: 'New channel' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByText('New channel', { exact: true }).waitFor();
  const privately = dialog.getByRole('switch', { name: /^Private/ });
  const alone = dialog.getByRole('switch', { name: /^Agents here are asked nowhere else/ });
  expect('the switch to be off unless somebody turns it on', !(await privately.isChecked()));
  expect('nothing to be said of its agents while it is not private', (await alone.count()) === 0);
  expect('the switch to say what it means under it', (await dialog.getByText('Only the people and agents in it can read it. Nobody else sees that it is there.').count()) === 1);
  await page.screenshot({ path: `${out}/kept-0-dialog.png` });
  await privately.check();
  await alone.waitFor();
  expect('what is said of its agents to be off unless somebody turns it on', !(await alone.isChecked()));
  const under = await dialog.evaluate((box) => {
    const [first, second] = [...box.querySelectorAll('.start-with')].map((one) => one.getBoundingClientRect());
    return second.top >= first.bottom;
  });
  expect('what is said of its agents to stand under the switch that makes it private', under);
  await dialog.getByLabel('Name').fill('Quince review');
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(dialog, DIALOG, `the dialog that makes a channel at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/kept-1-private.png` });
  await dialog.getByRole('button', { name: 'Create', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });

  // It is listed with its lock for whoever made it, opens as a tab called what it is called, and says so at its head.
  await lists(panel, 'general lobby lock:Quince review', "Ada's");
  const head = page.locator('.channel .view-head h1');
  await head.waitFor();
  expect(`its head to say what it is called, got "${await head.innerText()}"`, (await head.innerText()).trim() === 'Quince review');
  expect('its head to say that it is private', (await head.getByRole('img', { name: 'Private' }).count()) === 1);
  expect('its row to say that it is private', (await row(panel, 'Quince review').getByRole('img', { name: 'Private' }).count()) === 1);
  const tabOf = page.locator('.tab[aria-selected="true"]');
  expect('its tab to say that it is private, by a lock and no #', (await tabOf.getByRole('img', { name: 'Private' }).count()) === 1 && !(await tabOf.innerText()).includes('#'));
  expect('no id to be shown for a name', !(await page.locator('body').innerText()).includes('ch_'));
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(row(panel, 'Quince review').locator('.nav-row'), '.nav-twist, .kept-lock, .truncate, .row-more', `a private channel's row at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // Said there, and then something in the open for the other page to have heard.
  await say(page, 'the medlar ledger');
  await page.screenshot({ path: `${out}/kept-2-made.png` });
  await cli('post', 'lobby', 'first marker');
  await row(other, 'lobby').locator('.nav-item').click();
  await theirs.locator('.channel .message', { hasText: 'first marker' }).waitFor();
  // For somebody who is not in it, it is not there: not listed, on a page that was not read again; not said; not found.
  expect(`Nell's panel to list no private channel, got "${await listed(other)}"`, (await listed(other)) === 'general lobby');
  expect('nothing of it on her page', !(await theirs.locator('body').innerText()).match(/Quince|medlar/));
  expect(`nothing of it in what she finds, got ${await finds(theirs, 'medlar')}`, !(await finds(theirs, 'medlar')).match(/medlar|Quince/));
  expect('its maker to find what was said there', (await finds(page, 'medlar')).includes('medlar'));
  await theirs.screenshot({ path: `${out}/kept-3-not-there.png` });

  // ── Added ───────────────────────────────────────────────────────────────
  // From where its members are listed: an agent the daemon will not have in it is said why, there; a person is added.
  if ((await page.locator('.context').count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  const tools = page.locator('.context');
  await tools.getByRole('tab', { name: /^Channel\b/ }).click();
  await tools.getByText('Private: read by these and nobody else.').waitFor();
  expect('the only person in it to be told so', (await tools.getByText('You are the only person in it: add somebody before you leave.').count()) === 1);
  expect('no line to leave by for the last person in it', !(await offered(panel, 'Quince review')).includes('Leave'));
  await apart(tools.locator('.kept-members'), '.kept-said, .kept-add-member, p:not(.kept-said)', 'what is under the members');
  await tools.getByRole('button', { name: 'Add member', exact: true }).click();
  const adding = page.getByRole('dialog');
  await adding.getByText('Add to Quince review', { exact: true }).waitFor();
  await adding.getByText('They will read everything said here so far.').waitFor();
  const kindOf = async (name) => (await adding.locator('.kept-add', { has: page.getByRole('button', { name: `Add ${name}`, exact: true }) }).locator('.faint').innerText()).trim();
  expect(`an agent the factory does not start offered as external, got "${await kindOf('outsider')}"`, (await kindOf('outsider')) === 'agent (external)');
  expect(`an agent it starts offered as an agent, got "${await kindOf('helper')}"`, (await kindOf('helper')) === 'agent');
  expect(`a person offered as a person, got "${await kindOf('Nell')}"`, (await kindOf('Nell')) === 'person');
  await adding.getByRole('button', { name: 'Add helper', exact: true }).click();
  const why = adding.getByRole('alert');
  await why.waitFor();
  expect(`the daemon's sentence where the agent was added, got "${await why.innerText()}"`, (await why.innerText()).includes('an agent with no wall is in no channel kept to its members'));
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(adding, 'h2, .kept-add > *, .kept-note, .row button', `adding to a channel at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/kept-4-adding.png` });
  await adding.getByRole('button', { name: 'Add Nell', exact: true }).click();
  await adding.getByRole('button', { name: 'Add Nell', exact: true }).waitFor({ state: 'detached' });
  await adding.getByRole('button', { name: 'Done', exact: true }).click();
  await adding.waitFor({ state: 'detached' });
  // She sees it, on the page that was never read again, with what was said so far.
  await lists(other, 'general lobby lock:Quince review', "Nell's");
  await row(other, 'Quince review').locator('.nav-item').click();
  await theirs.locator('.channel .message', { hasText: 'the medlar ledger' }).waitFor();
  await say(theirs, 'and the damson');
  await page.locator('.channel > .split .message', { hasText: 'and the damson' }).waitFor();
  await theirs.screenshot({ path: `${out}/kept-5-added.png` });
  // Each is offered what is theirs: she leaves, and neither she nor the owner makes it private again; only the owner opens it.
  const hersOffered = await offered(other, 'Quince review');
  expect(`somebody in it to be offered leaving and not opening it, got ${hersOffered.join(' | ')}`, hersOffered.includes('Leave') && !hersOffered.includes('Open again…') && !hersOffered.includes('Make private…'));
  const ownersOffered = await offered(panel, 'Quince review');
  expect(`the owner to be offered opening it again, and leaving now that somebody else is in it, got ${ownersOffered.join(' | ')}`, ownersOffered.includes('Open again…') && ownersOffered.includes('Leave') && !ownersOffered.includes('Make private…'));

  // ── Left ────────────────────────────────────────────────────────────────
  await row(other, 'Quince review').click({ button: 'right' });
  await choose(theirs, 'Leave');
  await lists(other, 'general lobby', "Nell's");
  expect('its tab to be closed on the page of whoever left', (await theirs.locator('.channel .view-head h1').count()) === 0 || !(await theirs.locator('.channel .view-head h1').innerText()).includes('Quince'));
  await say(page, 'said after she left');
  await cli('post', 'lobby', 'second marker');
  await row(other, 'lobby').locator('.nav-item').click();
  await theirs.locator('.channel .message', { hasText: 'second marker' }).waitFor();
  // (Her page says once, as she left, that she has: that is all it says of it.)
  expect('nothing said after she left on her page', !(await theirs.locator('body').innerText()).includes('said after she left'));
  expect(`her panel to list nothing of it, got "${await listed(other)}"`, (await listed(other)) === 'general lobby');
  expect('nor what was said while she was in it, in what she finds', !(await finds(theirs, 'damson')).match(/damson|Quince/));

  // ── An open one made private ────────────────────────────────────────────
  // By somebody who is in it: Nell is not in `general`, and is offered no such line; Ada made it, and is.
  expect('somebody who is not in an open channel not to be offered making it private', !(await offered(other, 'general')).includes('Make private…'));
  await row(other, 'general').locator('.nav-item').click();
  await theirs.locator('.channel .view-head h1', { hasText: 'general' }).waitFor();
  await row(panel, 'general').click({ button: 'right' });
  await choose(page, 'Make private…');
  const keeping = page.getByRole('dialog');
  await keeping.getByText('Make #general private?', { exact: true }).waitFor();
  expect(`to be told who then reads it, got "${(await keeping.innerText()).replace(/\s+/g, ' ')}"`, (await keeping.innerText()).includes('Only you will read it from here'));
  await page.screenshot({ path: `${out}/kept-6-keeping.png` });
  await keeping.getByRole('button', { name: 'Make private', exact: true }).click();
  await keeping.waitFor({ state: 'detached' });
  await lists(panel, 'lock:general lobby lock:Quince review', "Ada's");
  // On the page of whoever is not in it, it is gone, and so is the tab it was showing in.
  await lists(other, 'lobby', "Nell's");
  await theirs.locator('.channel .view-head h1', { hasText: 'general' }).waitFor({ state: 'detached' });
  await theirs.screenshot({ path: `${out}/kept-7-kept-from.png` });

  // ── Opened again ────────────────────────────────────────────────────────
  await row(panel, 'Quince review').click({ button: 'right' });
  await choose(page, 'Open again…');
  const opening = page.getByRole('dialog');
  await opening.getByText('Open Quince review again?', { exact: true }).waitFor();
  expect('to be told who then reads it', (await opening.innerText()).includes('Anybody who may read channels will read it'));
  await page.screenshot({ path: `${out}/kept-8-opening.png` });
  await opening.getByRole('button', { name: 'Open again', exact: true }).click();
  await opening.waitFor({ state: 'detached' });
  await lists(panel, 'lock:general lobby quince-review', "Ada's");
  // Everybody lists it and reads what was said in it.
  await lists(other, 'lobby quince-review', "Nell's");
  await row(other, 'quince-review').locator('.nav-item').click();
  await theirs.locator('.channel .message', { hasText: 'said after she left' }).waitFor();
  expect('what was said in it to be in what anybody finds', (await finds(theirs, 'medlar')).includes('medlar'));
  await theirs.screenshot({ path: `${out}/kept-9-open-again.png` });

  // ── A phone's width ─────────────────────────────────────────────────────
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  // The side panel starts put away there, and lies over the page once it is brought.
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  const small = phone.getByRole('navigation', { name: 'Channels' });
  await small.waitFor();
  await lists(small, 'lock:general lobby quince-review', "the phone's");
  await apart(row(small, 'general').locator('.nav-row'), '.nav-twist, .kept-lock, .truncate, .row-more', "a private channel's row on a phone");
  await small.locator('.brand').getByRole('button', { name: 'New channel' }).tap();
  const asked = phone.getByRole('dialog');
  await asked.getByRole('switch', { name: /^Private/ }).check();
  await asked.getByRole('switch', { name: /^Agents here are asked nowhere else/ }).waitFor();
  await apart(asked, DIALOG, 'the dialog that makes a channel on a phone');
  await phone.screenshot({ path: `${out}/kept-10-phone.png` });

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
