// An archived channel: read by whoever read it, written in by nobody, and
// given back.
//
//   DAEMON=../target/debug/factory node scripts/archived.mjs [out-dir]
//
// Two people, each on a page of their own that is never read again: Ada,
// who owns the factory, and Nell, who registered by an invitation. Ada
// pins `general` and archives it from its row's menu, behind a question
// that says what archiving does; on both pages it leaves the list for an
// Archived part at the foot of the channels, folded. Unfolded, its page
// says it is archived at its head, with Unarchive for whoever may, and
// one line where something would be written, with nothing to type in.
// An agent the factory does not start, and Nell from a terminal, are
// each refused in the daemon's sentence where they say something there,
// and nothing is counted as waiting unread on Nell's page. It is still
// found: by the panel's box, under the Archived part, and by what the
// daemon finds. Given back from its head, it is listed as it was, pinned
// where it was pinned, and written in again. Nothing of the question, the
// head, the line or the Archived part lies over anything else of it, at
// three widths and a phone's. The environment variables are those of
// walkthrough.mjs.
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
const data = join(out, 'archived-data');
const home = join(out, 'archived-home');
const hers = join(out, 'archived-home-nell');
const outside = join(out, 'archived-home-scout');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers, outside]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers, outside]) await mkdir(dir, { recursive: true });
// The daemon and each command line are given a home of their own, so that nothing here reads or writes what a
// factory already on this machine keeps.
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });
/** A command, and what it was refused with: nothing where it was not. */
const refusedBy = (ran) => ran.then(
  () => '',
  (refused) => String(refused.stderr ?? refused.message),
);
/** A command of the agent's own home handed `said` on standard input. */
const handed = (said, ...args) =>
  new Promise((done) => {
    const child = spawn(binary, [...args, '--url', url], { cwd: repo, env: as(outside), stdio: ['pipe', 'pipe', 'pipe'] });
    let printed = '';
    for (const stream of [child.stdout, child.stderr]) stream.on('data', (chunk) => (printed += chunk));
    child.on('close', (code) => done({ ok: code === 0, said: printed }));
    child.stdin.end(`${said}\n`);
  });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  for (const name of ['general', 'lobby']) await cli('channel', 'create', name);
  await cli('post', 'general', 'the quince ledger');
  // An agent this factory does not start, joined from a home of its own: it says things for itself.
  const invited = (await cli('agent', 'invite', 'scout')).stdout;
  const invitation = invited.split('\n').find((line) => line.trim().startsWith('enrol_'))?.trim();
  const joined = await handed(invitation, 'agent', 'join');
  if (!joined.ok) throw new Error(`expected the agent to join, got ${joined.said}`);
  const scout = (...args) => run(binary, ['--as', 'scout', ...args, '--url', url], { cwd: repo, env: as(outside) });
  // An agent says something only in a channel it is in: it is brought into #general first.
  await cli('channel', 'add', 'general', 'scout');
  await scout('post', 'general', 'the scout is here');

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

  // What a panel lists outside its Archived part, and what that part lists, each as it is drawn.
  const READ = (where) => `[...document.querySelectorAll('nav[aria-label="Channels"] ${where} .grouped-channel > div > .nav-row .nav-item')]
    .map((item) => item.querySelector('.truncate').textContent)
    .join(' ')`;
  const LIVE = READ('.nav > :not(.archived-part)');
  const ARCHIVED = READ('.archived-part');
  const lists = async (nav, read, wanted, whose) => {
    const there = nav.page().waitForFunction(`(${read}) === ${JSON.stringify(wanted)}`, null, { timeout: 8000 });
    await there.catch(async () => {
      throw new Error(`expected ${whose} to list "${wanted}", got "${await nav.page().evaluate(read)}"`);
    });
  };
  const row = (nav, called) => nav.locator('.grouped-channel', { has: nav.page().locator('.nav-item .truncate', { hasText: new RegExp(`^${called}$`) }) });
  const choose = async (on, label) => {
    const menu = on.getByRole('menu');
    await menu.waitFor();
    await menu.getByText(label, { exact: true }).click();
    await menu.waitFor({ state: 'detached' });
  };
  const part = (nav) => nav.locator('.archived-part');
  // What `on`'s reader finds, asked of the daemon as their page asks it.
  const finds = (on, words) =>
    on.evaluate(async (asked) => {
      const answer = await fetch(`/api/search?q=${encodeURIComponent(asked)}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      return JSON.stringify(await answer.json());
    }, words);
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
  const HEAD = 'h1, .archived-mark, .archived-head button, .context-fold';

  await lists(panel, LIVE, 'general lobby', "Ada's panel");
  await lists(other, LIVE, 'general lobby', "Nell's panel");
  // Nell reads the other channel; Ada pins `general`, which is hers to arrange and kept in her browser.
  await row(other, 'lobby').locator('.nav-item').click();
  await row(panel, 'general').click({ button: 'right' });
  await choose(page, 'Pin to the top');
  // What is pinned is a branch, folded as every branch of the panel starts (`folding.mjs`), and unfolded as a person would.
  await panel.getByRole('button', { name: 'Unfold Pinned', exact: true }).click();
  await panel.locator('.grouped.pinned .grouped-channel', { hasText: 'general' }).waitFor();

  // ── Archived from its row's menu, behind a question ──────────────────────
  await row(panel, 'general').first().click({ button: 'right' });
  await choose(page, 'Archive…');
  const question = page.getByRole('dialog');
  await question.getByText('Archive #general?', { exact: true }).waitFor();
  const says = (await question.innerText()).replace(/\s+/g, ' ');
  expect(`the question to say what archiving does, got "${says}"`, says.includes('Whoever reads it goes on reading it, and finds it by search') && says.includes('Nobody says anything there'));
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(question, 'h2, p, .row button', `the question at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/archived-0-question.png` });
  await question.getByRole('button', { name: 'Archive channel', exact: true }).click();
  await question.waitFor({ state: 'detached' });

  // On both pages, neither read again: out of the list, pinned nowhere, and under the Archived part, folded.
  for (const [nav, whose] of [
    [panel, "Ada's panel"],
    [other, "Nell's panel"],
  ]) {
    await lists(nav, LIVE, 'lobby', whose);
    await part(nav).waitFor();
    expect(`${whose}'s Archived part to be folded, saying how many`, (await part(nav).getByRole('button', { name: 'Unfold Archived' }).count()) === 1 && (await part(nav).locator('.hint').innerText()) === '1');
  }
  expect('the pin to be drawn nowhere while it is archived', (await panel.locator('.grouped.pinned').count()) === 0);
  await page.screenshot({ path: `${out}/archived-1-listed.png` });

  // ── Its page ─────────────────────────────────────────────────────────────
  await part(panel).getByRole('button', { name: 'Unfold Archived' }).click();
  await lists(panel, ARCHIVED, 'general', "Ada's Archived part");
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(part(panel).locator('.group-row'), '.nav-twist, .glyph, .truncate', `the Archived part's head at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await row(panel, 'general').locator('.nav-item').click();
  const head = page.locator('.channel .view-head');
  await head.locator('.archived-mark').waitFor();
  expect('its head to offer giving it back', (await head.getByRole('button', { name: 'Unarchive', exact: true }).count()) === 1);
  const line = page.locator('.channel > .split .archived-line');
  await line.waitFor();
  expect(`one line to say it is archived, got "${await line.innerText()}"`, (await line.innerText()).includes('This channel is archived by Ada'));
  expect('nothing to type in', (await page.locator('.channel > .split .composer').count()) === 0 && (await page.locator('.channel textarea').count()) === 0);
  await page.locator('.channel .message', { hasText: 'the quince ledger' }).waitFor();
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(head, HEAD, `an archived channel's head at ${width}`);
    await apart(line, '.sign, .archived-said, button', `the line where something would be written at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/archived-2-page.png` });

  // ── Nothing said there ──────────────────────────────────────────────────
  const fromAgent = await refusedBy(scout('post', 'general', 'the scout says sloe'));
  expect(`the agent to be refused in the daemon's sentence, got "${fromAgent}"`, fromAgent.includes('#general is archived') && fromAgent.includes('channel unarchive general'));
  const fromHer = await refusedBy(nell('post', 'general', 'and the medlar'));
  expect(`Nell to be refused in the daemon's sentence, got "${fromHer}"`, fromHer.includes('#general is archived'));
  // Something in the open for both pages to have heard: all that came before it has come by then.
  await cli('post', 'lobby', 'a marker');
  await theirs.locator('.channel .message', { hasText: 'a marker' }).waitFor();
  expect('nothing of what was refused on either page', !(await theirs.locator('body').innerText()).match(/sloe|medlar/) && !(await page.locator('body').innerText()).match(/sloe|medlar/));
  expect(`nothing counted as waiting unread on Nell's page, got ${await other.locator('.count').count()}`, (await other.locator('.count').count()) === 0);
  expect('nothing out of sight said to wait', (await theirs.locator('.space-out').count()) === 0);

  // ── Found ───────────────────────────────────────────────────────────────
  // By the panel's box, under the Archived part, held open though it was folded on her page.
  await other.getByRole('searchbox').fill('general');
  await lists(other, ARCHIVED, 'general', "Nell's Archived part, found by the box");
  expect('the part to be held open for what was found', (await part(other).getByRole('button', { name: 'Fold Archived' }).getAttribute('aria-disabled')) === 'true');
  await theirs.screenshot({ path: `${out}/archived-3-found.png` });
  await other.getByRole('searchbox').press('Escape');
  await lists(other, ARCHIVED, '', "Nell's Archived part, folded as it was");
  expect(`what was said there to be in what she finds, got ${await finds(theirs, 'quince')}`, (await finds(theirs, 'quince')).includes('quince'));

  // ── Given back ──────────────────────────────────────────────────────────
  await head.getByRole('button', { name: 'Unarchive', exact: true }).click();
  await head.locator('.archived-mark').waitFor({ state: 'detached' });
  await page.locator('.channel > .split .composer textarea').waitFor();
  await panel.locator('.grouped.pinned .grouped-channel', { hasText: 'general' }).waitFor();
  expect('the Archived part to be gone with nothing in it', (await part(panel).count()) === 0);
  await lists(other, LIVE, 'general lobby', "Nell's panel");
  const box = page.locator('.channel > .split .composer textarea');
  await box.fill('back in the ledger');
  await page.keyboard.press('Enter');
  await theirs.locator('nav[aria-label="Channels"] .count').first().waitFor();
  await page.screenshot({ path: `${out}/archived-4-back.png` });

  // ── A phone's width ─────────────────────────────────────────────────────
  await cli('channel', 'archive', 'general');
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  const small = phone.getByRole('navigation', { name: 'Channels' });
  await part(small).waitFor();
  await part(small).getByRole('button', { name: 'Unfold Archived' }).tap();
  await apart(part(small).locator('.group-row'), '.nav-twist, .glyph, .truncate', "the Archived part's head on a phone");
  await row(small, 'general').locator('.nav-item').tap();
  const smallHead = phone.locator('.channel .view-head');
  await smallHead.locator('.archived-mark').waitFor();
  await apart(smallHead, HEAD, "an archived channel's head on a phone");
  expect('Unarchive to be offered where something would be written, on a phone', await phone.locator('.channel > .split .archived-line').getByRole('button', { name: 'Unarchive' }).isVisible());
  await apart(phone.locator('.channel > .split .archived-line'), '.sign, .archived-said, button', 'the line where something would be written on a phone');
  await phone.screenshot({ path: `${out}/archived-5-phone.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers, outside]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
