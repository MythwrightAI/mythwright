// What runs in a channel, in its Channel tab of the Tools panel.
//
//   DAEMON=../target/debug/factory node scripts/runsin.mjs [out-dir]
//
// `#project-updates` has two things applied for it: a project's own
// blueprint, `status-updates`, written through the daemon as the owner
// would, and the bot that ships (`std/bot`), applied from a terminal
// under a key of its own. Ada opens the Channel tab, where both are listed
// under its settings, each opening its own page; Settings… of the
// project's own opens a form made from its params in the drawer, as the
// apply drawer makes one, with the channel it is listed by not asked;
// a setting is changed and applied, and the same thing runs with it and
// the labels it was applied with. The
// bot's form is the same kind of form. Stop… asks first what stops,
// Cancel leaving it, and Stop it ending it, which a second page that is
// never read again and is told Ada holds nothing that applies sees go,
// with no Settings… or Stop… of its own; something applied for the
// channel from a terminal arrives on both. By the boxes at three widths
// and a phone's: nothing of the part over anything else in the panel or
// past it, nothing scrolling sideways. The environment variables are
// those of walkthrough.mjs, and `LOOK` (`looked.ts`).
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest, toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'runsin-data');
const home = join(out, 'runsin-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });

/** A project's own blueprint that says how the work stands in a channel, and stays. */
const UPDATES = `blueprint: status-updates
description: Say how the work stands in a channel every so often.
params:
  channel: { type: string, description: "The channel it says the status in." }
  every:
    type: string
    default: 10m
    description: "How often it says it."
  failures:
    type: bool
    default: true
    description: "Whether runs that failed are said."
on:
  asked:
    event: status.asked
    do: set
    with: { asked: true }
`;
const written = join(home, 'status-updates.bp');
await writeFile(written, UPDATES);

const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
let page = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', 'shop', '--bare');
  await cli('channel', 'create', 'project-updates');
  await cli('file', 'write', 'shop', '@blueprints/status-updates.bp', '--from', written);
  await cli('apply', 'status-updates', '--project', 'shop', '--param', 'channel=project-updates', '--param', 'every=10m', '--label', 'experiment=weekly');
  await cli('apply', 'std/bot', '--key', 'bot-echo', '--param', 'channel=project-updates', '--param', 'name=echo');

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
  /** A page signed in as Ada; `holds`, where it is said, is what the page is told she holds. */
  const opened = async (holds = null) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
    const one = await context.newPage();
    one.setDefaultTimeout(20_000);
    one.on('pageerror', (err) => problems.push(err.message));
    if (holds) {
      await one.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = holds;
        await route.fulfill({ response: answer, json: me });
      });
    }
    await one.goto(url);
    await one.getByLabel('Email').fill('ada@example.com');
    await one.getByLabel('Password').fill('correct horse');
    await one.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await one.locator('.rail').waitFor();
    const rail = one.locator('.rail').getByRole('button', { name: 'Channels', exact: true });
    if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
    await unfoldChannels(one);
    const row = one
      .getByRole('navigation', { name: 'Channels' })
      .locator('.grouped-channel', { has: one.locator('.nav-item .truncate', { hasText: /^project-updates$/ }) });
    await row.locator('.nav-item').first().click();
    await one.getByRole('heading', { name: '# project-updates' }).waitFor();
    if ((await one.locator('.context').count()) === 0) await one.getByRole('button', { name: 'Expand the tools panel' }).click();
    await toolsAtRest(one).waitFor();
    await one.locator('.context').getByRole('tab', { name: /^Channel\b/ }).click();
    return one;
  };
  const partOf = (one) => one.locator('.context').getByRole('region', { name: 'What runs in this channel' });
  const lineOf = (one, blueprint) => partOf(one).locator('.runsin-line', { has: one.locator('.runsin-name > .truncate', { hasText: new RegExp(`^${blueprint.replace('/', '\\/')}$`) }) });
  /** What the daemon says runs in the channel, asked from the page. */
  const runningNow = (one) =>
    one.evaluate(async () => {
      const headers = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
      const channel = (await (await fetch('/api/channels', { headers })).json()).channels.find((each) => each.name === 'project-updates');
      return (await (await fetch(`/api/channels/${channel.id}/running`, { headers })).json()).instances;
    });
  /** Nothing of the part lies over anything else in the panel or leaves it, and nothing scrolls sideways. */
  const boxed = async (one, what) => {
    const crowded = await pageCrowded(one, '.context');
    expect(`${what}: nothing of the Tools panel over anything else, got ${crowded.join('; ')}`, crowded.length === 0);
    const [panel, part] = await Promise.all([one.locator('.context').boundingBox(), partOf(one).boundingBox()]);
    expect(`${what}: the part inside the panel`, panel && part && part.x >= panel.x - 0.5 && part.x + part.width <= panel.x + panel.width + 0.5);
    for (const line of await partOf(one).locator('.runsin-line').all()) {
      const box = await line.boundingBox();
      for (const button of await line.locator('.runsin-acts button').all()) {
        const pressed = await button.boundingBox();
        expect(`${what}: ${await button.textContent()} whole inside its line`, pressed.x >= box.x - 0.5 && pressed.x + pressed.width <= box.x + box.width + 0.5);
      }
    }
    const sideways = await one.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`${what}: nothing scrolling sideways, got ${sideways}px`, sideways <= 0);
  };

  page = await opened();
  // ── Both listed under the settings, each by its blueprint ─────────────────────────────────────────
  await lineOf(page, 'status-updates').waitFor();
  await lineOf(page, 'std/bot').waitFor();
  expect('the bot said by its key', (await lineOf(page, 'std/bot').locator('.runsin-said').textContent()) === 'as bot-echo');
  expect('the part after the settings', await page.locator('.context').evaluate((panel) => {
    const settings = panel.querySelector('.channel-settings');
    const runs = panel.querySelector('.runsin');
    return !!settings && !!runs && !!(settings.compareDocumentPosition(runs) & Node.DOCUMENT_POSITION_FOLLOWING);
  }));
  await page.screenshot({ path: `${out}/runsin-1-listed.png` });

  // A page that is never read again, told Ada holds nothing that applies: the lines, and nothing to press on them.
  const other = await opened(['channels', 'blueprints:read', 'projects:read', 'inbox', 'themes', 'arranged']);
  await lineOf(other, 'status-updates').waitFor();
  expect('no Settings… or Stop… for whoever may not apply', (await partOf(other).locator('.runsin-acts').count()) === 0);

  // ── Settings…: a form made from the project's own blueprint, the channel not asked ────────────────
  const before = (await runningNow(page)).find((one) => one.blueprint === 'status-updates');
  await lineOf(page, 'status-updates').getByRole('button', { name: 'Settings of status-updates' }).click();
  await drawerAtRest(page).waitFor();
  const form = page.locator('.drawer form.applying');
  for (const param of ['every', 'failures']) {
    expect(`the form to ask ${param}`, (await form.locator('.comparing-label', { hasText: new RegExp(`^${param} `) }).count()) === 1);
  }
  expect('the form not to ask which channel: it is the one it is listed in', (await form.locator('.comparing-label', { hasText: /^channel / }).count()) === 0);
  const every = form.locator('label.comparing-field', { hasText: /^every / }).locator('input');
  expect(`the box to begin on what it was applied with, got ${await every.inputValue()}`, (await every.inputValue()) === '10m');
  await every.fill('5m');
  await page.screenshot({ path: `${out}/runsin-2-settings.png` });
  await form.getByRole('button', { name: 'Apply', exact: true }).click();
  await page.locator('.drawer').waitFor({ state: 'detached' });
  const after = (await runningNow(page)).find((one) => one.blueprint === 'status-updates');
  expect(
    `the same thing to run with what was set and the labels it was applied with, got ${JSON.stringify(after)}`,
    after.id === before.id && after.params.every === '5m' && after.params.channel === 'project-updates' && after.labels.experiment === 'weekly',
  );

  // ── The bot that ships: the same kind of form ─────────────────────────────────────────────────────
  await lineOf(page, 'std/bot').getByRole('button', { name: 'Settings of std/bot' }).click();
  await drawerAtRest(page).waitFor();
  for (const param of ['name', 'run']) {
    expect(`the bot's form to ask ${param}`, (await form.locator('.comparing-label', { hasText: new RegExp(`^${param} `) }).count()) === 1);
  }
  expect('the bot\'s form not to ask its channel', (await form.locator('.comparing-label', { hasText: /^channel / }).count()) === 0);
  await page.screenshot({ path: `${out}/runsin-3-shipped.png` });
  await form.getByRole('button', { name: 'Not now', exact: true }).click();
  await page.locator('.drawer').waitFor({ state: 'detached' });

  // ── By the boxes, at three widths and a phone's ─────────────────────────────────────────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await boxed(page, `at ${width} wide`);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  if ((await page.locator('.context').count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  const tab = page.locator('.context').getByRole('tab', { name: /^Channel\b/ });
  if ((await tab.getAttribute('aria-selected')) !== 'true') await tab.click();
  await partOf(page).scrollIntoViewIfNeeded();
  await boxed(page, "at a phone's width");
  await page.screenshot({ path: `${out}/runsin-4-phone.png` });
  await page.setViewportSize({ width: 1280, height: 860 });

  // ── Stop…: asked first, Cancel leaving it, Stop it ending it on both pages ─────────────────────────
  await lineOf(page, 'std/bot').getByRole('button', { name: 'Stop std/bot' }).click();
  const question = page.getByRole('dialog');
  await question.getByRole('heading', { name: 'Stop std/bot (bot-echo)?' }).waitFor();
  await question.getByText('says nothing more in #project-updates').waitFor();
  await page.screenshot({ path: `${out}/runsin-5-stop.png` });
  await question.getByRole('button', { name: 'Cancel', exact: true }).click();
  await question.waitFor({ state: 'detached' });
  expect('Cancel to leave it running', (await runningNow(page)).some((one) => one.key === 'bot-echo'));
  await lineOf(page, 'std/bot').getByRole('button', { name: 'Stop std/bot' }).click();
  await question.getByRole('button', { name: 'Stop it', exact: true }).click();
  await question.waitFor({ state: 'detached' });
  await lineOf(page, 'std/bot').waitFor({ state: 'detached' });
  await lineOf(other, 'std/bot').waitFor({ state: 'detached' });
  expect('the bot stopped', !(await runningNow(page)).some((one) => one.key === 'bot-echo'));

  // ── Something applied for the channel from a terminal arrives on both ──────────────────────────────
  await cli('apply', 'std/bot', '--key', 'bot-two', '--param', 'channel=project-updates', '--param', 'name=second');
  await lineOf(page, 'std/bot').waitFor();
  await lineOf(other, 'std/bot').waitFor();
  await other.screenshot({ path: `${out}/runsin-6-other.png` });

  console.log('runsin: what runs in a channel listed in its Channel tab, its settings a form from its blueprint applied again, stopped behind a question, and followed on a page never read again');
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/runsin-0-failed.png` }).catch(() => {});
} finally {
  // What was applied keeps nothing running of its own: the bot and the updates are listeners.
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
