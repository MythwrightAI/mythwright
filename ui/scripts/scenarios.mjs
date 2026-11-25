// A word of the Reference shown working, in the drawer.
//
//   DAEMON=../target/debug/factory node scripts/scenarios.mjs [out-dir]
//
// An example the daemon has a scenario of has Run it beside it; one it
// has none of has no such button. Pressed, the drawer opens and the
// example is run in a sandbox with what the scenario does to it, and it
// says what to notice in the scenario's own sentence, what was done in
// order, what happened as the Sandbox view of a blueprint's page draws
// it, and at its foot the daemon's own sentences, that it was a sandbox
// test and what was stood in. Each of three words (`retry`, `debounce`
// and the action `wait`) is held to what the daemon hands of the same
// sandbox, asked with the page's own key, and by the boxes at three
// widths and a phone's: nothing over anything else of it, nothing
// outside the drawer or scrolling sideways, and the foot whole and in
// view. Put away, the daemon has the sandbox no longer, and nothing of
// any of it is in the factory. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { DRAWER_AT_REST, drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'scenarios-data');
const home = join(out, 'scenarios-home');
const scheme = process.env.SCHEME ?? 'dark';

/** The three words pressed, each by its entry and the scenario of its example. */
const WORDS = [
  { entry: 'word/step/retry', scenario: 'retry' },
  { entry: 'word/handler/debounce', scenario: 'debounce' },
  { entry: 'action/wait', scenario: 'wait' },
];
/** A word whose example nothing runs: it has no button. */
const UNRUN = 'word/step/timeout';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
let page = null;
let small = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
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
  const signIn = async (context, at = '') => {
    const on = await context.newPage();
    on.on('pageerror', (err) => problems.push(err.message));
    await on.goto(`${url}/${at}`);
    await on.getByLabel('Email').fill('ada@example.com');
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor({ state: 'attached' });
    return on;
  };
  page = await signIn(await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme }));

  // ---- what the check reads of the page and asks of the daemon -----------------------
  const asks = (method, path, on = page) =>
    on.evaluate(
      async ([how, to]) => {
        const answer = await fetch(to, { method: how, headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
        return { status: answer.status, body: await answer.json().catch(() => null) };
      },
      [method, path],
    );
  const drawn = (on = page) => on.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const shot = (name, on = page) => on.screenshot({ path: `${out}/${name}.png` });
  const entry = (on, id) => on.locator(`[data-entry="${id}"]`);
  const runIt = (on, id) => entry(on, id).locator(':scope > .ref-inside > .ref-example .ref-run');
  // Opened under the open page, as a link is followed: a page gone to anew
  // is the page read again (the bar says a path since b431e32e, so an
  // address after a `#` is another page to the browser), and what a drawer
  // had then is the daemon's to end later, not the drawer's.
  const opened = async (on, id) => {
    await on.evaluate((to) => (location.hash = `/reference/${to}`), id);
    await entry(on, id).locator(':scope > .ref-inside > .ref-example .ref-code').waitFor();
  };
  /** Presses Run it, and waits for the drawer to be at rest with what came of the sandbox read. */
  const ran = async (on, id, press = (button) => button.click()) => {
    await press(runIt(on, id));
    await drawerAtRest(on).waitFor();
    await on.waitForFunction((at) => {
      const shown = document.querySelector(`${at} .scenario`);
      return !!shown && shown.dataset.busy === 'false' && Number(shown.dataset.answered) > 0;
    }, DRAWER_AT_REST);
    await drawn(on);
  };
  /** What the drawer says: its title, the sentence, what was done, each step with its key, what was said in its channels, and the foot. */
  const says = (on = page) =>
    on.evaluate((at) => {
      const drawer = document.querySelector(at);
      const shown = drawer.querySelector('.scenario');
      const text = (el) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();
      return {
        title: text(drawer.querySelector('.drawer-head h2')),
        name: shown.dataset.scenario,
        sandbox: shown.dataset.sandbox,
        notice: text(shown.querySelector('.scenario-notice')),
        done: [...shown.querySelectorAll('.scenario-done li')].map(text),
        outcome: text(shown.querySelector('.sandbox-outcome')),
        steps: [...shown.querySelectorAll('.sandbox-step')].map((step) => ({ key: step.dataset.step, came: step.dataset.came })),
        runs: [...shown.querySelectorAll('.sandbox-run h4')].map(text),
        said: [...shown.querySelectorAll('.sandbox-line.of-said code')].map(text),
        lit: shown.querySelectorAll('.sandbox-graph .gnode').length,
        foot: { said: text(shown.querySelector('.sandbox-said')), stood: text(shown.querySelector('.sandbox-stood-said')) },
      };
    }, DRAWER_AT_REST);
  /** The drawer's boxes: nothing of it over anything else, outside it or scrolling sideways, and its foot whole and in view. */
  const laidOut = async (when, on = page) => {
    await drawn(on);
    const now = await on.evaluate((at) => {
      const drawer = document.querySelector(at);
      const shown = drawer.querySelector('.scenario');
      const box = (el) => {
        const it = el.getBoundingClientRect();
        return { left: it.left, right: it.right, top: it.top, bottom: it.bottom, what: (el.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 40) };
      };
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const crossed = [];
      const among = (boxes) => boxes.forEach((a, n) => boxes.slice(n + 1).forEach((b) => over(a, b) && crossed.push(`"${a.what}" over "${b.what}"`)));
      const own = box(drawer);
      const body = shown.querySelector('.scenario-body');
      const foot = shown.querySelector('.sandbox-foot');
      among([...body.children].map(box));
      among([...shown.querySelectorAll('.sandbox-came > *')].map(box));
      among([...shown.querySelectorAll('.sandbox-run')].map(box));
      const outside = [];
      for (const step of shown.querySelectorAll('.sandbox-step')) {
        const [mine, cells] = [box(step), [...step.children].map(box)];
        among(cells);
        for (const one of cells) if (one.left < mine.left - 0.5 || one.right > mine.right + 0.5) outside.push(one.what);
      }
      for (const one of [...body.querySelectorAll('*')].map(box)) if (one.right - one.left > 0 && (one.left < own.left - 0.5 || one.right > own.right + 0.5)) outside.push(one.what);
      if (foot && over(box(body), box(foot))) crossed.push('the foot over what scrolls');
      const feet = foot && box(foot);
      return {
        crossed: [...new Set(crossed)],
        outside: [...new Set(outside)],
        sideways: [document.documentElement.scrollWidth - window.innerWidth, body.scrollWidth - body.clientWidth],
        foot: !!feet && feet.top >= own.top - 0.5 && feet.bottom <= Math.min(own.bottom, window.innerHeight) + 0.5 && foot.scrollHeight - foot.clientHeight <= 1,
        inside: own.left >= -0.5 && own.right <= window.innerWidth + 0.5,
      };
    }, DRAWER_AT_REST);
    expect(`nothing of the drawer to lie over anything else of it ${when}, got ${now.crossed.slice(0, 4).join('; ')}`, now.crossed.length === 0);
    expect(`everything to be inside the drawer ${when}, got ${now.outside.slice(0, 4).join('; ')}`, now.outside.length === 0 && now.inside);
    expect(`nothing to scroll sideways ${when}, got ${now.sideways}`, now.sideways.every((over) => over <= 0));
    expect(`the foot to be whole and in view ${when}`, now.foot);
  };
  const plain = (text) => text.replace(/`/g, '').replace(/\s+/g, ' ').trim();

  // ---- what the daemon has of the words shown working --------------------------------
  const scenarios = (await asks('GET', '/api/blueprints/language/scenarios')).body;
  expect(`the daemon to hand the words shown working, got ${JSON.stringify(scenarios)?.slice(0, 200)}`, Array.isArray(scenarios) && scenarios.length >= 10);
  const named = (name) => scenarios.find((one) => one.name === name);

  // ---- a word whose example nothing runs has no button --------------------------------
  await opened(page, UNRUN);
  expect(`${UNRUN} to have no Run it, since no scenario is of its example`, (await runIt(page, UNRUN).count()) === 0);

  // ---- three words, each run and read against what the daemon has of its sandbox --------
  for (const [n, word] of WORDS.entries()) {
    const scenario = named(word.scenario);
    expect(`a scenario called ${word.scenario}`, !!scenario);
    await opened(page, word.entry);
    await shot(`scenarios-${n + 1}-${word.scenario}-example-${scheme}`);
    await ran(page, word.entry);
    const now = await says();
    const name = word.entry.split('/').at(-1);
    expect(`the drawer to be called for the word, got "${now.title}"`, now.title === `${name}, run in a sandbox`);
    expect(`the drawer to run ${word.scenario}, got ${now.name}`, now.name === word.scenario);
    expect(`the scenario's own sentence of what to notice, got "${now.notice}"`, now.notice === plain(scenario.notice));
    expect(`each thing done, in order, got ${JSON.stringify(now.done)}`, now.done.length === scenario.acts.length && now.done[0].startsWith(Object.keys(scenario.acts[0])[0] === 'apply' ? 'Applied it' : ''));
    // What the page draws is what the daemon hands of the same sandbox, asked with the page's own key.
    const handed = (await asks('GET', `/api/sandboxes/${now.sandbox}`)).body;
    const steps = handed.timeline.filter((line) => line.what === 'step').map((line) => line.step);
    expect(`each step the daemon says ran, in its order, got ${JSON.stringify(now.steps)} for ${JSON.stringify(steps)}`, JSON.stringify(now.steps.map((one) => one.key)) === JSON.stringify(steps));
    const said = handed.timeline.filter((line) => line.what === 'said').map((line) => line.topic);
    expect(`what was said, in its order, got ${JSON.stringify(now.said)} for ${JSON.stringify(said)}`, JSON.stringify(now.said) === JSON.stringify(said));
    expect(`the foot to be the daemon's own sentences, got "${now.foot.said}"`, now.foot.said === handed.said && now.foot.said.startsWith('This was a sandbox test') && now.foot.stood.startsWith(handed.stood_said));
    expect('the example\'s graph, lit by what it did', now.lit > 0);
    if (word.scenario === 'retry') {
      expect(`the step to have gone well with its command asked for twice, got ${JSON.stringify(now.steps)}`, now.steps.length === 1 && now.steps[0].came === 'stood' && handed.asked.length === 2);
      expect(`the stand-in said in the foot with how often, got "${now.foot.stood}"`, now.foot.stood.includes('shell × 2'));
    }
    if (word.scenario === 'debounce') expect(`the handler's one run, got ${JSON.stringify(now.runs)}`, now.runs.filter((one) => one.includes('when changed')).length === 1 && now.said.filter((one) => one === 'record.updated').length === 3);
    if (word.scenario === 'wait') expect(`the wait answered, got "${now.outcome}"`, now.steps.map((one) => one.key).join() === 'asked,said' && now.said.join() === 'question.asked,question.answered');
    for (const width of [1280, 1000, 760]) {
      await page.setViewportSize({ width, height: 860 });
      await laidOut(`for ${word.scenario} at ${width} wide`);
    }
    await page.setViewportSize({ width: 1280, height: 860 });
    await shot(`scenarios-${n + 1}-${word.scenario}-run-${scheme}`);
    await page.locator(DRAWER_AT_REST).locator('.scenario-body').evaluate((body) => body.scrollTo(0, body.scrollHeight));
    await laidOut(`for ${word.scenario} scrolled to its end`);
    await shot(`scenarios-${n + 1}-${word.scenario}-ended-${scheme}`);
    // Put away, the daemon has the sandbox no longer.
    await page.locator('.drawer').getByRole('button', { name: 'Close', exact: true }).click();
    await page.locator('.drawer').waitFor({ state: 'detached' });
    await page.waitForFunction(async (id) => (await fetch(`/api/sandboxes/${id}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).status === 404, now.sandbox);
  }

  // ---- run again, it is run afresh, and another word's drawer ends the one before ----------
  await opened(page, WORDS[0].entry);
  await ran(page, WORDS[0].entry);
  const first = (await says()).sandbox;
  await opened(page, WORDS[2].entry);
  await ran(page, WORDS[2].entry);
  const second = (await says()).sandbox;
  expect('a word run again to have a sandbox of its own', second !== first && second !== '');
  // The drawer ends its sandbox as it is handed another, without waiting for the daemon's answer.
  await page
    .waitForFunction(async (id) => (await fetch(`/api/sandboxes/${id}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).status === 404, first, { timeout: 10000 })
    .catch(() => expect('the drawer handed another scenario to have ended the sandbox it had', false));
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await page.waitForFunction(async (id) => (await fetch(`/api/sandboxes/${id}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).status === 404, second);

  // ---- the other of dark and light ---------------------------------------------------------
  const other = scheme === 'dark' ? 'light' : 'dark';
  await litAs(page, other);
  await opened(page, WORDS[1].entry);
  await ran(page, WORDS[1].entry);
  await laidOut(`in the other of dark and light`);
  await shot(`scenarios-5-${WORDS[1].scenario}-${other}`);
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await litAs(page, scheme);

  // ---- a phone -------------------------------------------------------------------------------
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  small = await signIn(phone, `#/reference/${WORDS[0].entry}`);
  await entry(small, WORDS[0].entry).locator(':scope > .ref-inside > .ref-example .ref-code').waitFor();
  await ran(small, WORDS[0].entry, (button) => button.tap());
  const onPhone = await says(small);
  expect(`the scenario run on a phone, got "${onPhone.outcome}"`, onPhone.name === 'retry' && onPhone.foot.said.startsWith('This was a sandbox test'));
  await laidOut("at a phone's width", small);
  await shot(`scenarios-6-phone-${scheme}`, small);
  await small.locator(DRAWER_AT_REST).locator('.scenario-body').evaluate((body) => body.scrollTo(0, body.scrollHeight));
  await laidOut("at a phone's width, scrolled to its end", small);
  await shot(`scenarios-7-phone-ended-${scheme}`, small);
  await small.locator('.drawer').getByRole('button', { name: 'Close', exact: true }).tap();
  await small.locator('.drawer').waitFor({ state: 'detached' });
  await small.waitForFunction(async (id) => (await fetch(`/api/sandboxes/${id}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).status === 404, onPhone.sandbox);

  // ---- nothing of any of it in the factory ------------------------------------------------
  const projects = (await asks('GET', '/api/projects')).body;
  expect('the factory to have no project called sandbox', !(projects.projects ?? projects).some((one) => one.id === 'sandbox'));
  const instances = (await asks('GET', '/api/instances')).body;
  expect(`nothing to be running in the factory, got ${JSON.stringify(instances).slice(0, 200)}`, (instances.instances ?? instances).length === 0);
  const channels = (await asks('GET', '/api/channels')).body;
  expect(`no channel a scenario made to be in the factory, got ${JSON.stringify(channels).slice(0, 200)}`, !(channels.channels ?? channels).some((one) => one.name === 'general' || one.name === 'ops'));

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/scenarios-0-failed.png` }).catch(() => {});
  await small?.screenshot({ path: `${out}/scenarios-0-failed-phone.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
