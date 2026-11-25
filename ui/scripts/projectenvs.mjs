// A project's environments (`dev`, `staging`, `prod`): their page, opened
// from the gear of the project's head while it has none and from its line
// in the side panel once it has one; an environment added there, opened
// in the drawer beside the page with its row marked, its variables written
// and marked for agents, a secret set in a field that hides it (Escape
// putting its question away and not the drawer), said by name with who
// set it and when, handed to agents as a variable and not, and taken away
// behind a question that says what names it; the drawer kept for the tab
// while another is shown, given back with what was being typed, and put
// away by Escape; one made from a terminal listed with the page not read
// again; one taken away from a terminal while its drawer is open, and
// while its tab was away, putting the drawer away; Apply closed with why
// in the drawer of a blueprint applied for the project until an
// environment is chosen, and what it made said to run under it on
// Activity and on its own page, and the environment's last run said as
// what was applied under it, with no Run, beside what runs under it, and
// one with nothing under it saying the command that names the blueprint it
// is loaded by; an environment with something running
// under it not taken away, and one with nothing taken away from the foot
// of its drawer, which goes with it; that no answer the page was handed
// and nothing drawn holds a secret's value; that an agent's credential is
// shown no entry that is not marked for agents; and nothing of the page or
// the drawer over anything else or scrolling sideways at three widths and
// a phone's, with no head of the table broken inside a word beside it.
// And what an environment holds to (`views/envrules.tsx`, over
// `ui/src/envrules.ts`): the runs that acted under it listed in its drawer,
// the newest first and the first its last run, each opening its run's page;
// who approves and which branches it runs said there by whoever may change
// its secrets, a person chosen and the branches written; held by the daemon
// where something is applied under it from a terminal (Ada, whom it does
// not name, refused; Grace, of a branch it does not run, refused; of one it
// runs, applied and listed with the page not read again); and Grace, who
// may not say them, shown them in words with nothing to change them by;
// nothing of the drawer over anything else at three widths and a phone's.
//
//   DAEMON=../target/debug/factory node scripts/projectenvs.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { fillsPage } from './filled.mjs';
import { unfoldAll } from './folded.mjs';
import { gearLine } from './gear.mjs';
import { headsBroken } from './heads.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'projectenvs-data');
const home = join(out, 'projectenvs-home');
const graceHome = join(out, 'projectenvs-grace');
// What no answer and nothing drawn may ever hold.
const VALUE = 'sk_test_projectenvs_never_shown';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// A home of its own, so that nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const graceEnv = { ...process.env, HOME: graceHome, XDG_CONFIG_HOME: join(graceHome, 'config'), XDG_DATA_HOME: join(graceHome, 'share') };
const grace = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: graceEnv });

const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};

let failed = null;
let shot = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'grace@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await grace('register', '--email', 'grace@example.com', '--name', 'Grace', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  // Every answer the page is handed, but for the stream of events, which never ends.
  const answers = [];
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  shot = () => page.screenshot({ path: `${out}/projectenvs-failed.png` }).catch(() => {});
  page.on('pageerror', (err) => problems.push(err.message));
  page.on('response', (answer) => {
    const at = new URL(answer.url());
    if (!at.pathname.startsWith('/api/') || at.pathname === '/api/events') return;
    answers.push(answer.text().then((text) => [at.pathname, text], () => [at.pathname, '']));
  });
  const holdsNoValue = async (when) => {
    for (const [path, text] of await Promise.all(answers)) expect(`no answer to hold a secret's value ${when}, and ${path} did`, !text.includes(VALUE));
    expect(`nothing drawn to hold a secret's value ${when}`, !(await page.content()).includes(VALUE));
  };

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const nav = page.locator('.nav');
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  const toResources = async () => {
    // At a phone's width the panel is put away, and is so still once the window is wide again.
    if ((await resources.getAttribute('aria-pressed')) !== 'true' || !(await page.locator('.nav').isVisible())) await resources.click();
  };
  await toResources();
  // Everything the Resources panel lists starts folded, the projects and what is under each among it.
  await unfoldAll(page);

  // ---- a project with none: no line in the side panel, a line of its gear -------------------
  await nav.getByText('Shop', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  expect('no Environments under a project that has none', (await nav.locator('[data-project="shop"]').getByText('Environments', { exact: true }).count()) === 0);
  await gearLine(page, /^Environments/);
  const head = page.getByRole('heading', { name: 'Shop environments', level: 1 });
  await head.waitFor();
  const panel = page.locator('.envs-page');
  await panel.getByText('A project with none runs everything as it always did').waitFor();
  await page.screenshot({ path: `${out}/projectenvs-0-none.png` });

  // ---- one added, and opened in the drawer beside the page -----------------------------------
  await panel.getByRole('button', { name: 'Add an environment', exact: true }).click();
  const naming = panel.locator('.envs-add input');
  await naming.fill('Staging Two');
  await panel.getByText('A name is lowercase letters').waitFor();
  expect('Make it closed for a name that is no name', await panel.getByRole('button', { name: 'Make it', exact: true }).isDisabled());
  await naming.fill('staging');
  await panel.getByRole('button', { name: 'Make it', exact: true }).click();
  const drawer = drawerAtRest(page);
  const opened = drawer.locator('[data-opened="staging"]');
  await opened.waitFor();
  expect(`the drawer headed by the environment's name, got ${await drawer.locator('.drawer-head h2').innerText()}`, (await drawer.locator('.drawer-head h2').innerText()) === 'staging');
  expect('nothing of it under the table any more', (await panel.locator('.envs-opened').count()) === 0);
  const row = panel.locator('tr[data-environment="staging"]');
  await panel.locator('tr[data-environment="staging"][aria-selected="true"]').waitFor();
  await nav.locator('[data-project="shop"]').getByText('Environments', { exact: true }).waitFor();

  // Variables, written in place, one marked for agents.
  await opened.getByLabel("A new variable's name").fill('region');
  await opened.getByLabel('Its value').fill('eu');
  await opened.getByRole('button', { name: 'Add a variable', exact: true }).click();
  await opened.locator('[data-var="region"]').waitFor();
  await opened.getByLabel("A new variable's name").fill('api');
  await opened.getByLabel('Its value').fill('https://shop.example');
  await opened.getByRole('button', { name: 'Add a variable', exact: true }).click();
  await opened.locator('[data-var="api"]').waitFor();
  await opened.locator('[data-var="region"]').getByLabel('For agents').check();
  await row.getByText('1 entry', { exact: true }).waitFor();
  await opened.getByLabel('The value of region').fill('eu-west');
  const written = page.waitForResponse((answer) => answer.url().endsWith('/environments/staging/vars/region') && answer.request().method() === 'PUT');
  await opened.getByLabel('The value of region').press('Tab');
  expect('a variable written in place to be kept by the daemon', (await written).ok());
  const kept = await page.evaluate(async () => {
    const asked = await fetch('/api/projects/shop/environments/staging', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return asked.json();
  });
  expect(`what was written in place to be what the daemon keeps, got ${JSON.stringify(kept.vars?.region)}`, kept.vars?.region?.value === 'eu-west');
  await row.getByText('2 variables', { exact: true }).waitFor();

  // A secret, typed once in a field that hides it, and said by name alone.
  const dialog = page.getByRole('dialog');
  // Escape is first for the question the drawer asked: it goes, and the drawer stays.
  await opened.getByRole('button', { name: 'Set a secret…', exact: true }).click();
  await dialog.getByLabel('Name').waitFor();
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });
  await opened.waitFor();
  await opened.getByRole('button', { name: 'Set a secret…', exact: true }).click();
  await dialog.getByLabel('Name').fill('stripe');
  const value = dialog.getByLabel('Value');
  expect('the value hidden as it is typed', (await value.getAttribute('type')) === 'password');
  await value.fill(VALUE);
  await dialog.getByRole('button', { name: 'Set it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  const stripe = opened.locator('[data-secret="stripe"]');
  await stripe.getByText(/^set .* by Ada$/).waitFor();
  await stripe.getByText('Not handed to agents', { exact: true }).waitFor();
  await stripe.getByLabel('What an agent is handed stripe as').fill('STRIPE_KEY');
  await stripe.getByRole('button', { name: 'Hand to agents', exact: true }).click();
  await stripe.getByText('Handed to agents as STRIPE_KEY', { exact: true }).waitFor();
  await row.getByText('2 entries', { exact: true }).waitFor();
  await stripe.getByRole('button', { name: 'Replace…', exact: true }).click();
  await dialog.getByRole('heading', { name: 'Replace stripe' }).waitFor();
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  // A second, to be taken away.
  await opened.getByRole('button', { name: 'Set a secret…', exact: true }).click();
  await dialog.getByLabel('Name').fill('hook');
  await dialog.getByLabel('Value').fill('whsec_projectenvs_hook');
  await dialog.getByRole('button', { name: 'Set it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await opened.locator('[data-secret="hook"]').waitFor();
  await page.screenshot({ path: `${out}/projectenvs-1-opened.png` });
  // Beside the open drawer the table keeps every head whole, scrolling in its own box where there is no room.
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800]]) {
    await page.setViewportSize({ width, height });
    const broken = await headsBroken(page, '.envs-table th');
    expect(`no head of the table broken inside a word at ${width}: ${broken.join('; ')}`, broken.length === 0);
    for (const within of ['.envs-page', '.drawer']) {
      const wrong = await pageCrowded(page, within);
      expect(`nothing of ${within} over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    }
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // Taken away behind a question that says what names it.
  await opened.locator('[data-secret="hook"]').getByRole('button', { name: 'Remove…', exact: true }).click();
  await dialog.getByRole('heading', { name: 'Remove hook?' }).waitFor();
  await dialog.getByText('Nothing that is running, no service and no agent names it.').waitFor();
  await page.screenshot({ path: `${out}/projectenvs-2-remove-secret.png` });
  await dialog.getByRole('button', { name: 'Remove it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await opened.locator('[data-secret="hook"]').waitFor({ state: 'detached' });

  // ---- kept for the tab while another is shown, with what was being typed; Escape puts it away
  await opened.getByLabel("A new variable's name").fill('pending');
  await opened.getByLabel('Its value').fill('half written');
  const envsTab = await page.locator('.tab[aria-selected="true"]').first().getAttribute('data-key');
  await page.locator('.tab:not([aria-selected="true"])').first().click();
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await page.locator(`.tab[data-key="${envsTab}"]`).click();
  await opened.waitFor();
  expect('the drawer given back simply there, not coming in again', (await page.locator('.drawer.drawer-back').count()) === 1);
  expect('what was being typed in the drawer given back with it', (await opened.getByLabel("A new variable's name").inputValue()) === 'pending' && (await opened.getByLabel('Its value').inputValue()) === 'half written');
  await panel.locator('tr[data-environment="staging"][aria-selected="true"]').waitFor();
  await opened.getByLabel("A new variable's name").fill('');
  await opened.getByLabel('Its value').fill('');
  await head.click();
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await panel.locator('tr[data-environment="staging"][aria-selected="false"]').waitFor();
  await row.click();
  await opened.waitFor();

  // One made from a terminal is listed, with the page never read again.
  await cli('project', 'environment', 'shop', 'prod', 'add');
  await panel.locator('tr[data-environment="prod"]').waitFor();
  await holdsNoValue('as an environment is written');

  // ---- nothing of the page or the drawer over anything else, at three widths and a phone's ---
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await opened.waitFor();
    for (const at of [opened.locator('[data-var="region"]'), opened.locator('[data-secret="stripe"]'), opened.getByRole('button', { name: 'Remove this environment…', exact: true })]) {
      await at.scrollIntoViewIfNeeded();
      const wrong = [...(await pageCrowded(page, '.envs-page')), ...(await pageCrowded(page, '.drawer'))];
      expect(`nothing of the page or the drawer over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    }
    const sideways = await page.evaluate(() => [...document.querySelectorAll('.envs-page, .drawer-body, .envs-opened, .envs-opened *')].some((one) => one.scrollWidth > one.clientWidth + 1 && getComputedStyle(one).overflowX !== 'hidden' && getComputedStyle(one).overflowX !== 'clip' && one.matches('.envs-page, .drawer-body, .envs-opened')));
    expect(`neither the page nor the drawer to scroll sideways at ${width}`, !sideways && (await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)));
    await opened.locator('[data-var="region"]').scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${out}/projectenvs-3-at-${width}.png`, fullPage: false });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // ---- taken away from a terminal: with its drawer open, and while its tab was away ----------
  await panel.locator('tr[data-environment="prod"]').click();
  await drawer.locator('[data-opened="prod"]').waitFor();
  await cli('project', 'environment', 'shop', 'prod', 'remove');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  // The environments are a page of tables, and fill its tab with no drawer open (`filled.mjs`).
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });
  await fillsPage(page, 'the environments page', page.locator('.envs-body'), { ready: panel.locator('tr[data-environment]'), shot: `${out}/projectenvs-filled` });
  await panel.locator('tr[data-environment="prod"]').waitFor({ state: 'detached' });
  await cli('project', 'environment', 'shop', 'qa', 'add');
  await panel.locator('tr[data-environment="qa"]').click();
  await drawer.locator('[data-opened="qa"]').waitFor();
  await page.locator('.tab:not([aria-selected="true"])').first().click();
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await cli('project', 'environment', 'shop', 'qa', 'remove');
  await page.locator(`.tab[data-key="${envsTab}"]`).click();
  await panel.locator('tr[data-environment="qa"]').waitFor({ state: 'detached' });
  await page.locator('.drawer').waitFor({ state: 'detached' });
  // Asked once given back: what is gone is put away and kept for nobody.
  await page.waitForFunction(() => !document.querySelector('.drawer'));
  await cli('project', 'environment', 'shop', 'prod', 'add');
  await panel.locator('tr[data-environment="prod"]').waitFor();

  // ---- an agent's credential is shown what is marked for agents and nothing more -------------
  const invited = await page.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const asked = await fetch('/api/agent/invite', { method: 'POST', headers: mine, body: JSON.stringify({ name: 'reader', project: 'shop', scopes: ['projects:read', 'secrets:use'] }) });
    return asked.json();
  });
  const joined = await (await fetch(`${url}/api/agent/join`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ invitation: invited.invitation }) })).json();
  expect(`the agent to have joined, got ${JSON.stringify(Object.keys(joined))}`, typeof joined.token === 'string');
  const seen = await fetch(`${url}/api/projects/shop/environments`, { headers: { authorization: `Bearer ${joined.token}` } });
  const seenText = await seen.text();
  expect(`the agent to read its project's environments, got ${seen.status}`, seen.ok);
  expect("an agent's credential to be shown no secret's value", !seenText.includes(VALUE));
  const staging = JSON.parse(seenText).environments.find((one) => one.name === 'staging');
  expect(`an agent shown the marked variable alone, got ${JSON.stringify(staging.vars)}`, JSON.stringify(Object.keys(staging.vars)) === '["region"]' && staging.vars.region.agents === true);
  expect(`an agent shown the marked secret alone, by name, got ${JSON.stringify(staging.secrets)}`, JSON.stringify(Object.keys(staging.secrets ?? {})) === '["stripe"]' && staging.secrets.stripe.agents === 'STRIPE_KEY');

  // ---- an apply for the project asks which, and is closed until one is chosen -----------------
  await toResources();
  if ((await nav.getByText('std/board', { exact: true }).count()) === 0) {
    await nav.getByText('Blueprints', { exact: true }).last().click();
    await nav.getByText('std', { exact: true }).click();
  }
  await nav.getByText('std/board', { exact: true }).click();
  await page.locator('.view-head').getByText('std/board', { exact: false }).first().waitFor();
  await page.locator('.view-head').getByRole('button', { name: 'Apply', exact: true }).click();
  const asks = drawerAtRest(page);
  await asks.getByRole('heading', { name: /^Apply / }).waitFor();
  expect('no environment asked before a project is chosen', (await asks.getByLabel('Environment').count()) === 0);
  await asks.getByLabel('Project').selectOption('shop');
  await asks.getByLabel('Environment').waitFor();
  const offered = await asks.getByLabel('Environment').locator('option:not([disabled])').allInnerTexts();
  expect(`the project's environments offered, got ${offered}`, JSON.stringify(offered) === '["prod","staging"]' || JSON.stringify(offered) === '["staging","prod"]');
  const apply = asks.getByRole('button', { name: 'Apply', exact: true });
  expect('Apply closed until an environment is chosen', await apply.isDisabled());
  await asks.getByText('Choose the environment it runs under: this project runs everything under one of them.').waitFor();
  await page.screenshot({ path: `${out}/projectenvs-4-apply-closed.png` });
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    const wrong = await pageCrowded(page, '.drawer');
    expect(`nothing of the drawer over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await asks.getByLabel('Environment').selectOption('staging');
  expect('Apply open once one is chosen', !(await apply.isDisabled()));
  const applied = page.waitForResponse((answer) => new URL(answer.url()).pathname === '/api/apply' && answer.request().method() === 'POST');
  await apply.click();
  expect('the board applied for the project', (await applied).ok());
  await asks.waitFor({ state: 'detached' });

  // ---- what runs says which -------------------------------------------------------------------
  await toResources();
  await nav.getByText('Activity', { exact: true }).click();
  await page.getByRole('heading', { name: 'Activity', level: 1 }).waitFor();
  const line = page.locator('tr', { hasText: 'std/board' }).first();
  await line.getByText('under staging', { exact: true }).waitFor();
  await page.screenshot({ path: `${out}/projectenvs-5-activity.png` });
  await line.click();
  await page.locator('.view-head', { hasText: 'std/board · 1 child · under staging' }).first().waitFor();
  await page.locator('.view-head').getByRole('button', { name: 'Details', exact: true }).click();
  await page.locator('[data-under]', { hasText: 'under staging' }).waitFor();
  await page.screenshot({ path: `${out}/projectenvs-6-instance.png` });

  // ---- taken away: refused while something runs under it (the board and what it started), and from the foot of its drawer where nothing does
  await toResources();
  await nav.locator('[data-project="shop"]').getByText('Environments', { exact: true }).click();
  await head.waitFor();
  await panel.locator('tr[data-environment="staging"]').getByText('2 running', { exact: true }).waitFor();
  // What it last ran is what was applied under it, with no Run: said as what was applied, beside what runs
  // under it, and the one with nothing under it and no blueprint it is loaded by says the command that names one.
  await panel.locator('tr[data-environment="staging"] [data-last-run]', { hasText: /^std\/board( as \S+)?, (running|went well), by Ada, / }).waitFor();
  await panel.locator('tr[data-environment="prod"] [data-last-run]', { hasText: /^nothing has run under it, and no blueprint loads it from a branch: `\S+ project environment shop prod deploy <blueprint>` names one$/ }).waitFor();
  expect('nothing said never to have run beside something running', !(await panel.innerText()).includes('never run'));
  await page.screenshot({ path: `${out}/projectenvs-7-last-run.png` });
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    const wrong = await pageCrowded(page, '.envs-page');
    expect(`nothing of the page saying what each last ran over anything else at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    if (width > 390) {
      const broken = await headsBroken(page, '.envs-table th');
      expect(`no head of the table broken inside a word at ${width}: ${broken.join('; ')}`, broken.length === 0);
    }
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  if ((await drawer.locator('[data-opened="staging"]').count()) === 0) await panel.locator('tr[data-environment="staging"]').click();
  await drawer.locator('[data-opened="staging"]').getByRole('button', { name: 'Remove this environment…', exact: true }).click();
  await dialog.getByRole('heading', { name: 'Remove staging?' }).waitFor();
  await dialog.getByText(/run under the environment .staging./).waitFor();
  expect('an environment with something running under it not taken away', await dialog.getByRole('button', { name: /^Remove/ }).last().isDisabled());
  await page.screenshot({ path: `${out}/projectenvs-8-in-use.png` });
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });

  // ---- the runs that acted under it, each opening its page; and what it holds to ----------------
  const staged = drawer.locator('[data-opened="staging"]');
  const runsPart = staged.locator('[data-runs]');
  await runsPart.locator('[data-acted]').first().waitFor();
  const first = runsPart.locator('[data-acted]').first();
  expect('the first run listed said to be its last run', (await first.innerText()).includes('its last run'));
  expect(`the first run said as what was applied under it, got ${await first.innerText()}`, /std\/board( as \S+)?, (running|went well), by Ada, /.test(await first.innerText()));
  await first.getByRole('link').click();
  await page.locator('.pipeline-run').waitFor();
  // The run of what acted under it: the board's own, or one of what it started under it.
  await page.locator('.pipeline-run .pipeline-facts', { hasText: 'under staging' }).waitFor();
  await page.screenshot({ path: `${out}/projectenvs-9-a-run-under-it.png` });
  await page.locator(`.tab[data-key="${envsTab}"]`).click();
  await staged.waitFor();
  const rulesPart = staged.locator('[data-rules]');
  await rulesPart.scrollIntoViewIfNeeded();
  await rulesPart.locator('[data-rule="approve"] [data-said]', { hasText: 'Nobody is named' }).waitFor();
  await rulesPart.locator('[data-rule="branches"] [data-said]', { hasText: 'Any branch may be run under it.' }).waitFor();
  await rulesPart.getByLabel('Somebody else who approves').selectOption({ label: 'Grace' });
  await rulesPart.locator('[data-approver="Grace"]').waitFor();
  await rulesPart.locator('[data-rule="approve"] [data-said]', { hasText: "Grace approves what goes under it: anybody else's run waits for a go-ahead from one of them." }).waitFor();
  const branches = rulesPart.getByLabel('The branches it runs, by spaces');
  await branches.fill('main, release/*');
  await branches.press('Enter');
  await rulesPart.locator('[data-rule="branches"] [data-said]', { hasText: 'Only main and release/* may be run under it.' }).waitFor();
  await page.waitForFunction(() => document.querySelector('[data-rules] input')?.value === 'main release/*', null, { timeout: 5000 }).catch(() => {});
  expect(`the branches written back by spaces, got ${await branches.inputValue()}`, (await branches.inputValue()) === 'main release/*');
  await page.screenshot({ path: `${out}/projectenvs-10-rules.png` });
  // Held by the daemon: Ada, whom it does not name, is refused; Grace of a branch it does not run is refused, and of one it runs is not.
  const { writeFile } = await import('node:fs/promises');
  const loads = join(out, 'projectenvs-loads.bp');
  await writeFile(loads, 'blueprint: loads\nparams:\n  branch: { type: string }\nsteps:\n  one: { do: noop }\n');
  const refused = async (who, ...args) => who(...args).then(() => '', (err) => `${err.stdout ?? ''}${err.stderr ?? ''}`);
  const adas = await refused(cli, 'apply', loads, '--project', 'shop', '--environment', 'staging', '--param', 'branch=main');
  expect(`Ada refused, as somebody it does not name, got ${adas}`, adas.includes('and you are not one of them'));
  const feature = await refused(grace, 'apply', loads, '--project', 'shop', '--environment', 'staging', '--param', 'branch=feature/x');
  expect(`a branch it does not run refused, got ${feature}`, feature.includes('`feature/x` may not be run under `staging` of `shop`'));
  expect('one it runs applied', (await refused(grace, 'apply', loads, '--project', 'shop', '--environment', 'staging', '--param', 'branch=release/2')) === '');
  await runsPart.locator('[data-acted]').first().filter({ hasText: /loads, went well, by Grace, / }).waitFor();
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    for (const at of [rulesPart, runsPart]) {
      await at.scrollIntoViewIfNeeded();
      const wrong = await pageCrowded(page, '.drawer');
      expect(`nothing of the drawer over anything else with its rules and runs at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    }
    expect(`the page not to scroll sideways at ${width}`, await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
    await rulesPart.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${out}/projectenvs-11-rules-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // Grace, who may not say them, is shown them in words and nothing to change them by.
  const graces = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const theirs = await graces.newPage();
  theirs.on('pageerror', (err) => problems.push(err.message));
  await theirs.goto(url);
  await theirs.getByLabel('Email').fill('grace@example.com');
  await theirs.getByLabel('Password').fill('correct horse');
  await theirs.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await theirs.locator('.rail').waitFor();
  if ((await theirs.getByRole('button', { name: 'Resources', exact: true }).getAttribute('aria-pressed')) !== 'true') await theirs.getByRole('button', { name: 'Resources', exact: true }).click();
  await unfoldAll(theirs);
  await theirs.locator('.nav [data-project="shop"]').getByText('Environments', { exact: true }).click();
  await theirs.locator('.envs-page tr[data-environment="staging"]').click();
  const shown = drawerAtRest(theirs).locator('[data-opened="staging"] [data-rules]');
  await shown.locator('[data-rule="approve"] [data-said]', { hasText: 'Grace approves what goes under it' }).waitFor();
  await shown.locator('[data-rule="branches"] [data-said]', { hasText: 'Only main and release/* may be run under it.' }).waitFor();
  expect('nothing to change them by for whoever may not', (await shown.locator('select, input, button').count()) === 0);
  await shown.getByText('Said by whoever may change its secrets.').waitFor();
  await theirs.screenshot({ path: `${out}/projectenvs-12-rules-read.png` });
  await graces.close();

  await panel.locator('tr[data-environment="prod"]').click();
  await drawer.locator('[data-opened="prod"]').getByRole('button', { name: 'Remove this environment…', exact: true }).click();
  await dialog.getByRole('heading', { name: 'Remove prod?' }).waitFor();
  // Pressed once what goes with it is read: the button waits until it is open.
  await dialog.getByRole('button', { name: /^Remove/ }).last().click();
  await dialog.waitFor({ state: 'detached' });
  await panel.locator('tr[data-environment="prod"]').waitFor({ state: 'detached' });
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await holdsNoValue('at the end');

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await shot?.();
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, graceHome]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
