// An agent's whole setup, from Edit… first in its line's menu on the
// Agents tab of Members, in the drawer (`views/agentedit.tsx`, over
// `ui/src/agentedit.ts`):
//
// - one added by hand: the form the dialog that adds one asks with,
//   filled in from its slot as it stands; what is typed kept with the
//   drawer while another tab is shown and given back with it; saved, the
//   daemon's sentence of what the change comes to said over the form's
//   foot, and what was changed held by the daemon (what it is told,
//   emptied too, its repository, the web, what it may ask);
// - one of a fleet's: its row of the fleet's agents in the row form,
//   said over it to be where its settings are;
// - one from outside: what it may ask and its project, moved and held by
//   the daemon;
// - somebody who holds less: a page told it holds no `shell` and no
//   `secrets:use` shown what would show the agent more of the device and
//   its sign-ins closed, each saying why; one told it may only see agents
//   shown all of it closed, saying why, and a credential that holds so
//   refused by the daemon;
// - the still copy the drawer goes out as, put away with something
//   chosen, typed and ticked and not saved, showing each field as it
//   stood and not as it was first drawn (`holdsAsItStands` in
//   `ui/src/still.ts`);
// - and nothing of the drawer over anything else of it, outside it or
//   scrolling sideways, at 1280, 900 and 640 wide and a phone's.
//
//   DAEMON=../target/debug/factory node scripts/agentedit.mjs [out-dir]
//
// Run it plainly and with `LOOK="size=largest density=compact"`. Every
// agent here is the built-in assistant or one from outside: no agent's
// program is started. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { openMenu } from './agentmenu.mjs';
import { agentsPage } from './apart.mjs';
import { pageCrowded } from './crowded.mjs';
import { DRAWER_AT_REST } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'agentedit-data');
const home = join(out, 'agentedit-home');
const look = process.env.LOOK ? '-looked' : '';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  // A project with its fleet, whose agents are its rows'; another with nothing; one added by hand; and one from outside.
  await cli('project', 'create', 'Shop');
  await cli('project', 'create', '--bare', 'Cafe');
  await cli('agent', 'add', 'helper', '--project', 'shop', '--instructions', 'Be brief.');
  await cli('agent', 'invite', 'walls', '--project', 'shop');

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
  const until = async (what, ask) => {
    let last;
    for (let tries = 0; tries < 300; tries += 1) {
      last = await ask();
      if (last === true) return;
      await new Promise((later) => setTimeout(later, 100));
    }
    throw new Error(`expected ${what}, and it is ${JSON.stringify(last)}`);
  };
  // The fleet's agents are there once its rows are taken.
  await until('the fleet to have its worker', async () => (await cli('agent', 'list')).stdout.includes('shop-worker-1') || (await cli('agent', 'list')).stdout);

  /** A page signed in as the owner; `holds`, where it is said, is what the page is told she holds, to draw as for somebody who holds less. */
  const opened = async (context, holds = null) => {
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    // The still copy a drawer goes out as holds what it was a picture of in a closed tree nobody is handed: the check is handed it, here alone.
    await page.addInitScript(() => {
      const attach = Element.prototype.attachShadow;
      window.closedTrees = new WeakMap();
      Element.prototype.attachShadow = function (init) {
        const tree = attach.call(this, init);
        window.closedTrees.set(this, tree);
        return tree;
      };
    });
    if (holds) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = holds;
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor();
    await agentsPage(page, out);
    return page;
  };
  /** What the daemon says of an agent, asked from the page as the page asks. */
  const slotOf = async (page, name) => {
    const all = await page.evaluate(async () => (await fetch('/api/agents', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json());
    return all.agents.find((one) => one.name === name);
  };
  const drawer = (page) => page.locator(DRAWER_AT_REST);
  /** Opens an agent's setup from the first line of its line's menu. */
  const edit = async (page, name, kind) => {
    await openMenu(page, name);
    const first = page.getByRole('menuitem').first();
    expect(`Edit… to be the first line of ${name}'s menu, got ${await first.innerText()}`, (await first.innerText()).trim() === 'Edit…');
    await first.click();
    await drawer(page).locator(`.agent-setup[data-setup="${kind}"]`).waitFor();
  };
  /** Nothing of the drawer over anything else, outside it or scrolling sideways, at three widths and a phone's. */
  const held = async (page, what) => {
    for (const [width, height] of [
      [1280, 860],
      [900, 860],
      [640, 860],
      [390, 844],
    ]) {
      await page.setViewportSize({ width, height });
      await drawer(page).waitFor();
      // Each part scrolled into view in turn: what is out of sight is looked at where it is scrolled to.
      const parts = await drawer(page).locator('.agent-setup h3, .agent-setup .setup-foot, .agent-setup .form > .row').count();
      for (let at = 0; at < Math.max(parts, 1); at += 1) {
        const part = drawer(page).locator('.agent-setup h3, .agent-setup .setup-foot, .agent-setup .form > .row').nth(at);
        if (parts > 0) await part.scrollIntoViewIfNeeded();
        const wrong = await pageCrowded(page, '.drawer');
        expect(`nothing of ${what} over anything else at ${width} wide, got ${JSON.stringify(wrong)}`, wrong.length === 0);
      }
      const wide = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(`nothing to scroll sideways at ${width} wide under ${what}, and it is ${wide} too wide`, wide <= 0);
      await page.screenshot({ path: `${out}/agentedit-${what.replace(/\W+/g, '-')}-${width}${look}.png` });
    }
    await page.setViewportSize({ width: 1280, height: 860 });
  };
  const putAway = async (page) => {
    await page.waitForTimeout(200);
    await page.keyboard.press('Escape');
    await page.locator('.drawer').waitFor({ state: 'detached' });
  };

  // ---- one added by hand: its form filled in, typed in, kept with the tab, saved ----------------------------------------------------
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const page = await opened(context);
  await edit(page, 'helper', 'hand');
  const told = drawer(page).getByLabel('Standing instructions (optional)');
  expect(`its form to begin with what it is told now, got "${await told.inputValue()}"`, (await told.inputValue()) === 'Be brief.');
  const save = drawer(page).getByRole('button', { name: 'Save', exact: true });
  expect('nothing to save until something is changed', await save.isDisabled());
  await held(page, 'one added by hand');
  await told.fill('Be thorough.');
  await drawer(page).locator('select').filter({ has: page.locator('option[value="commit"]') }).selectOption('commit');
  await drawer(page).getByLabel('May use the web unasked').check();
  // What it may ask, a line changed: search taken away.
  const search = drawer(page).getByLabel('What it may do with search', { exact: true });
  await search.selectOption({ index: 0 });
  // Another tab shown, and the agents again: the drawer is given back with what was typed.
  await page.locator('.tabs .tab', { hasText: 'Start' }).click();
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await page.locator('.tabs .tab', { hasText: 'Members' }).click();
  await drawer(page).locator('.agent-setup[data-setup="hand"]').waitFor();
  expect(`what was typed to be there again, got "${await told.inputValue()}"`, (await told.inputValue()) === 'Be thorough.');
  await save.click();
  const said = drawer(page).locator('.setup-said');
  await said.waitFor();
  const words = await said.innerText();
  expect(`the daemon's sentence of what the change comes to, got "${words}"`, words.startsWith('`helper` is changed:') && words.includes('from its next turn') && !(await said.evaluate((one) => one.classList.contains('error-text'))));
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/agentedit-saved${look}.png` });
  let slot = await slotOf(page, 'helper');
  expect(`the daemon to hold what was changed, got ${JSON.stringify(slot)}`, slot.instructions === 'Be thorough.' && slot.git === 'commit' && slot.web === true && !slot.scopes.includes('search'));
  // What it is told emptied, and held so.
  await told.fill('');
  await save.click();
  await until('what it is told to be emptied', async () => (await slotOf(page, 'helper')).instructions === '' || (await slotOf(page, 'helper')).instructions);
  // Put away with something chosen and typed and not saved, the still copy it goes out as shows each field as it stood, and never
  // each list at its first choice or each box as it was first drawn: held half way out and read in its own tree.
  await drawer(page).locator('select').filter({ has: page.locator('option[value="commit"]') }).selectOption('write');
  await told.fill('Half written.');
  const web = await drawer(page).getByLabel('May use the web unasked').isChecked();
  await drawer(page).getByLabel('May use the web unasked').setChecked(!web);
  await page.evaluate(() => {
    window.leftHeld = null;
    const seen = new MutationObserver(() => {
      const copy = document.querySelector('.drawer-leaving');
      if (!copy || window.leftHeld) return;
      const [slide] = copy.getAnimations();
      slide?.pause();
      const tree = window.closedTrees.get(copy);
      window.leftHeld = {
        selects: [...tree.querySelectorAll('select')].map((one) => one.value),
        areas: [...tree.querySelectorAll('textarea')].map((one) => one.value),
        checks: [...tree.querySelectorAll('input[type="checkbox"]')].map((one) => one.checked),
      };
      window.leftSlide = slide;
      seen.disconnect();
    });
    seen.observe(document.querySelector('.main'), { childList: true });
  });
  const shown = await drawer(page).evaluate((one) => ({
    selects: [...one.querySelectorAll('select')].map((field) => field.value),
    areas: [...one.querySelectorAll('textarea')].map((field) => field.value),
    checks: [...one.querySelectorAll('input[type="checkbox"]')].map((field) => field.checked),
  }));
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => window.leftHeld !== null);
  const copied = await page.evaluate(() => window.leftHeld);
  await page.screenshot({ path: `${out}/agentedit-leaving${look}.png` });
  expect(`the copy going out to show each field as it stood, got ${JSON.stringify(copied)} of ${JSON.stringify(shown)}`, JSON.stringify(copied) === JSON.stringify(shown) && copied.selects.includes('write') && copied.areas.includes('Half written.'));
  await page.evaluate(() => window.leftSlide?.play());
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });

  // ---- one of a fleet's: its row ------------------------------------------------------------------------------------------------------
  await edit(page, 'shop-worker-1', 'row');
  const rowSaid = await drawer(page).locator('.setup-row-said').innerText();
  expect(`its settings said to be its fleet row's, got "${rowSaid}"`, rowSaid.includes("This agent's settings are its fleet row's") && rowSaid.includes('shop-agents-'));
  await drawer(page).locator('.agent-setup .form').waitFor();
  const values = await drawer(page).locator('.agent-setup .form input, .agent-setup .form textarea').evaluateAll((all) => all.map((one) => one.value));
  expect(`the row's own form, with what its row says, got ${JSON.stringify(values)}`, values.includes('worker'));
  await held(page, 'a fleet agent');
  await putAway(page);

  // ---- one from outside: what it may ask and its project --------------------------------------------------------------------------------
  await edit(page, 'walls', 'outside');
  expect('no program asked of it', (await drawer(page).getByLabel('Standing instructions (optional)').count()) === 0);
  await held(page, 'one from outside');
  await drawer(page).getByLabel('The project it works in').selectOption('cafe');
  await drawer(page).getByRole('button', { name: 'Save', exact: true }).click();
  await drawer(page).locator('.setup-said').waitFor();
  expect(`its move said, got "${await drawer(page).locator('.setup-said').innerText()}"`, (await drawer(page).locator('.setup-said').innerText()).includes('`cafe`'));
  slot = await slotOf(page, 'walls');
  expect(`the daemon to hold its project, got ${slot.project}`, slot.project === 'cafe');
  await putAway(page);
  await context.close();

  // ---- somebody who holds less ------------------------------------------------------------------------------------------------------------
  const member = ['projects', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];
  const lessContext = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const less = await opened(lessContext, member);
  await edit(less, 'helper', 'hand');
  const reads = drawer(less).getByLabel('What else it may read there, a folder or file a line');
  expect('what would show its program more of the device to be closed', await reads.isDisabled());
  expect('and to say why', (await reads.getAttribute('title'))?.includes('shell'));
  const signins = drawer(less).getByLabel(/^What it signs in with/);
  expect('its sign-ins to be closed, saying why', (await signins.isDisabled()) && (await signins.getAttribute('title'))?.includes('secrets:use'));
  // Turning on what shows it more is closed to them, and turning it off is theirs: the web is on now, going on without the daemon off.
  expect('the web, which is on, to be theirs to turn off', !(await drawer(less).getByLabel('May use the web unasked').isDisabled()));
  expect('going on without the daemon, which is off, not to be turned on by them', await drawer(less).getByLabel(/^Goes on without the daemon/).isDisabled());
  expect('what it is told to be theirs to change', !(await drawer(less).getByLabel('Standing instructions (optional)').isDisabled()));
  await held(less, 'one added by hand for somebody who holds less');
  await lessContext.close();
  const seeing = ['agents:read', 'projects', 'channels', 'board', 'search'];
  const seeContext = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const sees = await opened(seeContext, seeing);
  await edit(sees, 'helper', 'hand');
  const why = await drawer(sees).locator('.setup-closed').first().innerText();
  expect(`all of it closed, saying why, got "${why}"`, why.includes('agents:write'));
  expect('what it is told to be closed too', await drawer(sees).getByLabel('Standing instructions (optional)').isDisabled());
  expect('nothing to save', await drawer(sees).getByRole('button', { name: /^Save/ }).isDisabled());
  await sees.screenshot({ path: `${out}/agentedit-closed${look}.png` });
  // And the daemon refuses a credential that holds so, whatever a page draws.
  const made = await sees.evaluate(
    async (scopes) =>
      (await fetch('/api/credentials', { method: 'POST', headers: { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' }, body: JSON.stringify({ name: 'sees', scopes }) })).json(),
    seeing,
  );
  const refused = await fetch(`${url}/api/agents`, {
    method: 'POST',
    headers: { authorization: `Bearer ${made.token}`, 'content-type': 'application/json' },
    body: JSON.stringify({ name: 'helper', params: { instructions: 'Mine.' } }),
  });
  expect(`a credential that may only see agents refused, got ${refused.status}`, refused.status === 403);
  await seeContext.close();
  expect(`nothing thrown on any page, got ${problems.join(' | ')}`, problems.length === 0);
  await browser.close();
  console.log(`agentedit: every check held${look ? ' at the look' : ''}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
