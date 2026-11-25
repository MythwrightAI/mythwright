// A channel's details in the Channel tab of the Tools panel, and who may
// change them.
//
//   DAEMON=../target/debug/factory node scripts/details.mjs [out-dir]
//
// Two people, each on a page of their own that is never read again: Ada,
// who owns the factory and made `general`, and Nell, whom Ada added to
// it. The Channel tab reads, in order, Details, Members, Settings (and
// Running here where something runs), each under a heading drawn the
// same way. Ada, who manages it, has its name and what it says it is for
// in fields; Nell reads them as words, is told who manages it, and her
// row's menu offers no Rename…. What Ada writes in the description is
// said as she leaves the field, under the channel's head and as the tip
// of its row on both pages; a name she writes is said as the name it
// comes to and saved on Enter. Ada makes Nell a manager from Nell's page
// among the members, which then says Nell manages it, and Nell's page has
// the fields, where a name another channel has is refused in the daemon's
// sentence with what she wrote staying, and what she says it is for is
// saved on Enter and seen on Ada's page. The last manager stays: what is
// pressed to take her off is closed, saying why, while she is the only
// one. Nothing of the tab lies over anything else or leaves the panel,
// and nothing scrolls sideways, at three widths and a phone's. The
// environment variables are those of walkthrough.mjs, and `LOOK`.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { sideSettled, toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'details-data');
const home = join(out, 'details-home');
const hers = join(out, 'details-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
// The daemon and each command line are given a home of their own, so that nothing here reads or writes what a
// factory already on this machine keeps.
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
const pages = [];
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('channel', 'create', 'general', '--topic', 'everything');
  await cli('channel', 'create', 'lobby');
  await cli('channel', 'add', 'general', 'Nell');
  await cli('post', 'general', 'the quince ledger');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const row = (one, called) =>
    one.getByRole('navigation', { name: 'Channels' }).locator('.grouped-channel', { has: one.locator('.nav-item .truncate', { hasText: new RegExp(`^${called}$`) }) });
  // Somebody's page, signed in, with `general` open and the Channel tab of the Tools panel showing.
  const opened = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: process.env.SCHEME ?? 'dark' });
    const one = await context.newPage();
    pages.push(one);
    one.setDefaultTimeout(20_000);
    one.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await one.goto(url);
    await one.getByLabel('Email').fill(email);
    await one.getByLabel('Password').fill('correct horse');
    await one.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await one.locator('.rail').waitFor();
    const rail = one.locator('.rail').getByRole('button', { name: 'Channels', exact: true });
    if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
    await unfoldChannels(one);
    await row(one, 'general').locator('.nav-item').first().click();
    await one.getByRole('heading', { name: '# general' }).waitFor();
    if ((await one.locator('.context').count()) === 0) await one.getByRole('button', { name: 'Expand the tools panel' }).click();
    await toolsAtRest(one).waitFor();
    const tab = one.locator('.context').getByRole('tab', { name: /^Channel\b/ });
    if ((await tab.getAttribute('aria-selected')) !== 'true') await tab.click();
    await details(one).waitFor();
    return one;
  };
  const details = (one) => one.locator('.context').getByRole('region', { name: 'Details of this channel' });
  const field = (one, label) => details(one).getByLabel(label, { exact: true });
  /** The topic under the channel's head, waited for. */
  const headSays = async (one, words, whose) => {
    const said = one.waitForFunction((wanted) => document.querySelector('.channel .view-head .channel-topic')?.textContent.trim() === wanted, words, { timeout: 8000 });
    await said.catch(async () => {
      throw new Error(`expected ${whose} head to say "${words}", got "${await one.locator('.channel .view-head').innerText()}"`);
    });
  };
  /** Nothing of the panel lies over anything else or leaves it, and nothing scrolls sideways. */
  const boxed = async (one, what) => {
    const crowded = await pageCrowded(one, '.context');
    expect(`${what}: nothing of the Tools panel over anything else, got ${crowded.join('; ')}`, crowded.length === 0);
    const [panel, part] = await Promise.all([one.locator('.context').boundingBox(), details(one).boundingBox()]);
    expect(`${what}: the details inside the panel`, panel && part && part.x >= panel.x - 0.5 && part.x + part.width <= panel.x + panel.width + 0.5);
    for (const input of await details(one).locator('input').all()) {
      const box = await input.boundingBox();
      expect(`${what}: a field inside the panel`, box.x >= panel.x - 0.5 && box.x + box.width <= panel.x + panel.width + 0.5);
    }
    const sideways = await one.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`${what}: nothing scrolling sideways, got ${sideways}px`, sideways <= 0);
  };

  const page = await opened('ada@example.com');
  const theirs = await opened('nell@example.com');

  // ── The tab's parts in order, each under a heading drawn the same way ───────────────────────
  const headings = (one) =>
    one.locator('.context').evaluate((panel) =>
      [...panel.querySelectorAll('.channel-details > h3, .channel-members > h3, .channel-settings > h3, .runsin > h3')].map((h) => {
        const style = getComputedStyle(h);
        return { said: h.textContent.trim(), look: [style.fontSize, style.fontWeight, style.color, style.textTransform, style.letterSpacing].join(' ') };
      }),
    );
  for (const [one, whose] of [
    [page, "Ada's"],
    [theirs, "Nell's"],
  ]) {
    const seen = await headings(one);
    const said = seen.map((one) => one.said);
    expect(`${whose} tab to read Details, Members, Settings in that order, got ${said.join(', ')}`, said.slice(0, 3).join() === 'Details,Members,Settings');
    expect(`${whose} headings drawn the same way, got ${seen.map((one) => one.look).join(' | ')}`, new Set(seen.map((one) => one.look)).size === 1);
  }

  // ── Ada manages it: fields; Nell reads words, and is offered no Rename… ───────────────────
  expect('Ada to have the name in a field', (await field(page, 'Channel name').inputValue()) === 'general');
  expect('Ada to have the description in a field', (await field(page, 'Description').inputValue()) === 'everything');
  await details(page).getByText('Managed by you.', { exact: true }).waitFor();
  expect('Nell to have no field', (await details(theirs).locator('input').count()) === 0);
  expect(`Nell to read its name, got ${await details(theirs).innerText()}`, (await details(theirs).locator('.channel-details-name').innerText()) === '#general');
  expect('Nell to read its description', (await details(theirs).locator('.channel-details-topic').innerText()) === 'everything');
  await details(theirs).getByText('Managed by Ada.', { exact: true }).waitFor();
  await row(theirs, 'general').click({ button: 'right' });
  const herMenu = theirs.getByRole('menu');
  await herMenu.waitFor();
  const lines = (await herMenu.locator('[role="menuitem"]').allInnerTexts()).map((one) => one.trim());
  expect(`no Rename… in Nell's menu, got ${lines.join(' | ')}`, !lines.includes('Rename…'));
  // Put away by a press outside it.
  await theirs.locator('.channel .view-head h1').click();
  await herMenu.waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/details-0-manager.png` });
  await theirs.screenshot({ path: `${out}/details-1-member.png` });

  // ── The description, said as Ada leaves the field ──────────────────────────────────────
  await field(page, 'Description').fill('Where the quinces are');
  await field(page, 'Channel name').focus();
  await headSays(page, 'Where the quinces are', "Ada's");
  await headSays(theirs, 'Where the quinces are', "Nell's");
  await details(theirs).locator('.channel-details-topic', { hasText: 'Where the quinces are' }).waitFor();
  const tip = await row(theirs, 'general').locator('.nav-item').first().getAttribute('title');
  expect(`the row's tip to say it, got "${tip}"`, tip === 'general: Where the quinces are');

  // ── The name, said as it comes to and saved on Enter ──────────────────────────────────
  await field(page, 'Channel name').fill('The Orchard');
  await details(page).getByText('It will be called #the-orchard.', { exact: true }).waitFor();
  await page.keyboard.press('Enter');
  await page.getByRole('heading', { name: '# the-orchard' }).waitFor();
  await theirs.getByRole('heading', { name: '# the-orchard' }).waitFor();
  await details(theirs).locator('.channel-details-name', { hasText: '#the-orchard' }).waitFor();

  // ── Nell made a manager from her page among the members ───────────────────────────────
  const members = page.locator('.context').getByRole('region', { name: 'Members of this channel' });
  await members.locator('.context-member', { hasText: 'Nell' }).click();
  const about = page.locator('.context .context-about');
  await about.getByText('Does not manage #the-orchard.', { exact: true }).waitFor();
  await page.screenshot({ path: `${out}/details-2-person.png` });
  await about.getByRole('button', { name: 'Make manager', exact: true }).click();
  await about.getByText('Manages #the-orchard.', { exact: true }).waitFor();
  await about.getByRole('button', { name: 'Not a manager', exact: true }).waitFor();
  await page.locator('.context-crumbs').getByRole('button', { name: 'Channel', exact: true }).click();
  await details(page).getByText('Managed by you and Nell.', { exact: true }).waitFor();
  // Her page, never read again, has the fields now.
  await field(theirs, 'Description').waitFor();
  await details(theirs).getByText('Managed by Ada and you.', { exact: true }).waitFor();

  // A name another channel has: refused in the daemon's sentence, what she wrote staying.
  await field(theirs, 'Channel name').fill('Lobby');
  await theirs.keyboard.press('Enter');
  const refused = details(theirs).getByRole('alert');
  await refused.waitFor();
  expect(`the daemon's sentence, got "${await refused.innerText()}"`, (await refused.innerText()) === 'there is already a channel called lobby');
  expect('what she wrote to stay', (await field(theirs, 'Channel name').inputValue()) === 'Lobby');
  await theirs.screenshot({ path: `${out}/details-3-refused.png` });
  await field(theirs, 'Channel name').fill('the-orchard');
  await field(theirs, 'Description').fill('Quinces, medlars and sloes');
  await field(theirs, 'Description').press('Enter');
  await headSays(page, 'Quinces, medlars and sloes', "Ada's");
  expect('the field Ada was not writing in to follow', (await field(page, 'Description').inputValue()) === 'Quinces, medlars and sloes');

  // ── The last manager stays ─────────────────────────────────────────────────────────────
  await cli('channel', 'manager', 'the-orchard', 'Nell', '--not');
  await details(page).getByText('Managed by you.', { exact: true }).waitFor();
  await members.locator('.context-member', { hasText: 'Ada' }).click();
  await about.getByText('Manages #the-orchard.', { exact: true }).waitFor();
  const last = about.getByRole('button', { name: 'Not a manager', exact: true });
  expect('taking the last manager off to be closed', (await last.getAttribute('aria-disabled')) === 'true');
  await about.getByText('The last manager stays: make somebody else one first.', { exact: true }).waitFor();
  await page.locator('.context-crumbs').getByRole('button', { name: 'Channel', exact: true }).click();
  // Nell, no longer one, reads words again.
  await details(theirs).locator('.channel-details-name').waitFor();
  expect('Nell to have no field once she manages nothing', (await details(theirs).locator('input').count()) === 0);

  // ── By the boxes, at three widths and a phone's ──────────────────────────────────────────
  for (const width of [1280, 1000, 800]) {
    for (const one of [page, theirs]) {
      await one.setViewportSize({ width, height: 860 });
      await boxed(one, `at ${width} wide`);
    }
  }
  await page.screenshot({ path: `${out}/details-4-narrow.png` });
  // At a phone's width the side panel lies over the page: put away first, so that the Tools panel is what is looked at.
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.getByRole('button', { name: 'Collapse the side panel' }).click();
  await sideSettled(page);
  await page.setViewportSize({ width: 390, height: 844 });
  if ((await page.locator('.context').count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  const tab = page.locator('.context').getByRole('tab', { name: /^Channel\b/ });
  if ((await tab.getAttribute('aria-selected')) !== 'true') await tab.click();
  await details(page).scrollIntoViewIfNeeded();
  await boxed(page, "at a phone's width");
  await page.screenshot({ path: `${out}/details-5-phone.png` });

  console.log("details: a channel's name and description in its Channel tab, changed by its managers and read as words by anybody else, managers named from a person's page, the last staying");
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  for (const [index, one] of pages.entries()) await one.screenshot({ path: `${out}/details-failed-${index}.png` }).catch(() => {});
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
