// A run as a pipeline run, and a project's pipelines (`views/pipelinerun.tsx`
// over `ui/src/staged.ts`; `views/pipelines.tsx` over `ui/src/pipelines.ts`).
//
//   DAEMON=../target/debug/factory node scripts/pipelines.mjs [out-dir]
//
// A project's own pipeline of a stage that is a task of two steps (the
// first printing three hundred lines), a command that fails and is let
// fail, two beside it (one writing twenty thousand lines), and a `call` of
// another pipeline; and beside it something else applied for the project.
// The Pipelines line under the project in the side panel, its page listing
// both pipelines each with Run and never run, and under them, folded,
// everything else that runs there; Run pressed, the run's page opened as a
// tab called what it is a run of, its stages in the order of the graph's
// layers, each said by what it says it is for with its key, its status and
// how long it took, the task's two steps under it, every one of the three
// hundred lines of the first in its box, the last five hundred of the
// twenty thousand drawn and no more with the window before them drawn by
// Show earlier, the failure said with what it wrote, the call unfolded
// into its child's run in place; folded and unfolded, and as it was left
// when another tab was shown and it again; its graph lit; the pipeline's
// row then with a mark of that run, which opens its page; the run's
// address copied and opened in a second page by somebody signing in, the
// tab opened and the bar saying its address; a line told about the
// run with a link to its page, as the preview's is, the link opening the
// run here as a tab and no page of the browser's, and what it is about
// opening it too; the run's row on Activity opening it; the project's
// triggers kept (`std/pipeline`), Run asking them as whoever presses, the
// run they begin under the row's key opened as a tab, and one they refuse
// (a pipeline that says nothing of what it runs with) said why on the page
// and in the presser's inbox; Run pressed at one moment on two pages, each
// opening the run its own press began and not the other's (`asked` of
// `pipeline.asked`, `/api/pipelines/begun`); nothing of either
// page over anything else or scrolling sideways at three widths and a
// phone's, plainly and with `LOOK="size=largest density=compact"`. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'pipelines-data');
const home = join(out, 'pipelines-home');

const CHECK = `blueprint: pipelines/check
description: Builds the shop and checks it.
steps:
  build:
    description: Build it.
    task:
      compile: { do: shell, with: { cmd: [seq, 1, 300] } }
      link: { do: shell, with: { cmd: [echo, linked] }, needs: [compile] }
  tests:
    description: Run the tests.
    needs: [build]
    do: shell
    with: { cmd: [sh, -c, "echo running the tests; echo two of them broke >&2; exit 3"] }
    allow_failure: true
  lint: { do: shell, with: { cmd: [echo, lint ok] }, needs: [build] }
  bulk: { description: Say a great deal., do: shell, with: { cmd: [seq, 1, 20000] }, needs: [build] }
  deploy: { call: pipelines/deploy, as: deploy, needs: [tests, lint] }
`;
// Tells whoever applies it that a run ran, about the run and with a link to its page, as the preview's line does.
const TOLD = `blueprint: told
permissions: [inbox]
params:
  run: { type: string }
  link: { type: string }
steps:
  tell:
    do: inbox.tell
    with: { to: applier, says: "The check ran.", why: checked, about: "\${params.run}", parts: [{ kind: links, links: ["\${params.link}"] }] }
`;
// A pipeline that says what it runs with, which whoever holds that may have the project's triggers run by hand.
const QUICK = `blueprint: pipelines/quick
description: A quick look.
permissions: [shell]
steps:
  look: { description: Look., do: shell, with: { cmd: [echo, looked] } }
`;
const DEPLOY = `blueprint: pipelines/deploy
description: Ships what was built.
steps:
  ship: { description: Ship it., do: shell, with: { cmd: [echo, shipped] } }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const look = process.env.LOOK ? `-${process.env.LOOK.replace(/[^a-z]+/g, '-')}` : '';

let failed = null;
let page = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  // The project's own blueprints, where the factory keeps them (`@blueprints/` of the project's), under pipelines/.
  const kept = join(data, 'blueprints', 'projects', 'shop', 'pipelines');
  await mkdir(kept, { recursive: true });
  await writeFile(join(kept, 'check.bp'), CHECK);
  await writeFile(join(kept, 'deploy.bp'), DEPLOY);
  await cli('project', 'scan', 'shop');
  // Something else that runs for the project, which is no pipeline of its own.
  await cli('apply', 'std/board', '--project', 'shop');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  await context.grantPermissions(['clipboard-read', 'clipboard-write'], { origin: url });
  page = await context.newPage();
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async (one) => {
    await one.getByLabel('Email').fill('ada@example.com');
    await one.getByLabel('Password').fill('correct horse');
    await one.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await one.locator('.rail').waitFor();
  };
  await page.goto(url);
  await signIn(page);
  // Resources, with what it has read unfolded, as everything there starts folded (`folded.mjs`).
  const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(page);
  const shop = page.locator('[data-project="shop"]');

  // ---- the Pipelines line under the project, and its page
  await shop.locator('button.nav-item', { hasText: /^\W*Pipelines$/ }).click();
  const list = page.locator('.pipelines-page');
  await list.locator('h1', { hasText: 'Shop pipelines' }).waitFor();
  const row = (name) => list.locator(`.pipeline-row[data-pipeline="pipelines/${name}"]`);
  for (const name of ['check', 'deploy']) {
    await row(name).waitFor();
    await row(name).locator('.pipeline-strip', { hasText: 'never run' }).waitFor();
    expect(`a Run on ${name}`, (await row(name).getByRole('button', { name: 'Run', exact: true }).count()) === 1);
  }
  expect('what the check says it is for under its name', (await row('check').locator('.pipeline-said').innerText()).includes('Builds the shop'));
  const others = list.locator('.pipelines-others');
  await others.getByRole('button', { name: /Everything else that runs here/ }).click();
  await others.locator('.pipeline-row[data-blueprint="std/board"]').waitFor();
  await page.screenshot({ path: `${out}/pipelines-0-list${look}.png` });

  // ---- Run: the run's page, as a tab called what it is a run of
  await row('check').getByRole('button', { name: 'Run', exact: true }).click();
  const runPage = page.locator('.pipeline-run');
  await runPage.waitFor();
  await runPage.locator('.pipeline-facts[data-status="ok"]').waitFor({ timeout: 30_000 });
  const tabTitle = await page.locator('.tabs .tab[aria-selected="true"], .tabs .tab.active').first().innerText().catch(() => '');
  expect(`the tab called what it is a run of, got "${tabTitle}"`, tabTitle.includes('pipelines/check run'));
  const stages = runPage.locator('.pipeline-stages').first();
  const keys = await stages.evaluate((one) => [...one.children].map((stage) => [stage.dataset.stage, stage.dataset.status]));
  expect(`the stages in the order of the graph's layers, got ${JSON.stringify(keys)}`, JSON.stringify(keys.map(([key]) => key)) === JSON.stringify(['build', 'tests', 'lint', 'bulk', 'deploy']));
  expect(`each stage how it came out, got ${JSON.stringify(keys)}`, JSON.stringify(keys.map(([, status]) => status)) === JSON.stringify(['ok', 'failed', 'ok', 'ok', 'ok']));
  const stage = (key) => stages.locator(`:scope > .pipeline-stage[data-stage="${key}"]`);
  expect('a stage said by what it says it is for, with its key', (await stage('build').locator('.stage-called').innerText()) === 'Build it.' && (await stage('build').locator('.stage-key code').innerText()) === 'build');
  expect('the failure let fail said so', (await stage('tests').locator('.stage-mark').innerText()).includes('let fail'));
  expect('how long each took', (await stage('build').locator('.stage-took').innerText()).trim() !== '');
  // The task's two steps under it, and every line the first wrote.
  const steps = await stage('build').locator('.step-out').evaluateAll((all) => all.map((one) => one.dataset.step));
  expect(`the task's steps under it, got ${JSON.stringify(steps)}`, JSON.stringify(steps) === JSON.stringify(['build.compile', 'build.link']));
  const compiled = stage('build').locator('.step-out[data-step="build.compile"] .step-log');
  await compiled.and(page.locator('[data-lines="300"]')).waitFor();
  const lines = await compiled.evaluate((one) => [...one.children].map((line) => line.textContent));
  expect(`all three hundred lines, the first and the last among them, got ${lines.length}`, lines.length === 300 && lines[0] === '1' && lines.at(-1) === '300');
  const failure = stage('tests');
  expect('why it failed', (await failure.locator('.step-error').innerText()).includes('exited with code 3'));
  const wrote = await failure.locator('.step-log').innerText();
  expect(`what the failing command wrote, both streams, got "${wrote}"`, wrote.includes('running the tests') && wrote.includes('two of them broke'));
  // The call, unfolded into its child's run in place.
  const child = stage('deploy').locator('.child-run');
  await child.locator('.pipeline-stage[data-stage="ship"][data-status="ok"]').waitFor();
  expect('the child run said by what its step says', (await child.locator('.pipeline-stage[data-stage="ship"] .stage-called').innerText()) === 'Ship it.');
  await child.locator('.step-log', { hasText: 'shipped' }).waitFor();
  const runId = (await page.evaluate(() => [...document.querySelectorAll('.child-run a.run-page-link')].map((a) => a.getAttribute('href'))))[0];
  expect(`the child's run a link to its own page, got ${runId}`, typeof runId === 'string' && runId.startsWith('/run/run_'));
  // Twenty thousand lines drawn from their end a window at a time, and the window before by Show earlier.
  const bulk = stage('bulk').locator('.step-log');
  await bulk.and(page.locator('[data-lines="500"]')).waitFor();
  const window = await bulk.evaluate((one) => ({ first: one.dataset.first, drawn: one.querySelectorAll(':scope > div').length, last: one.lastElementChild?.textContent, earlier: one.querySelector('.step-log-earlier')?.textContent ?? '' }));
  expect(`the last five hundred of twenty thousand lines drawn and no more, with what came before offered, got ${JSON.stringify(window)}`, window.first === '19501' && window.drawn === 500 && window.last === '20000' && window.earlier.includes('19,000 lines before them'));
  const nodes = await runPage.evaluate((one) => one.querySelectorAll('*').length);
  expect(`the whole run's page a few thousand elements at most, got ${nodes}`, nodes < 5000);
  await bulk.locator('.step-log-earlier').click();
  await bulk.and(page.locator('[data-lines="1000"]')).waitFor();
  expect('the window before it drawn over it', (await bulk.getAttribute('data-first')) === '19001' && (await bulk.locator(':scope > div').first().innerText()) === '19001');
  await page.screenshot({ path: `${out}/pipelines-1-run${look}.png`, fullPage: true });

  // ---- folded, and as it was left when another tab was shown and it again
  await stage('build').locator('.stage-band').click();
  await stage('build').locator('.stage-inside').waitFor({ state: 'detached' });
  expect('a folded stage saying so', (await stage('build').locator('.stage-band').getAttribute('aria-expanded')) === 'false');
  await page.locator('.tabs .tab', { hasText: 'Shop pipelines' }).first().click();
  await list.waitFor();
  await page.locator('.tabs .tab', { hasText: 'pipelines/check run' }).first().click();
  await runPage.waitFor();
  expect('no frame on the way back saying it reads the run', (await page.locator('.empty', { hasText: 'Reading the run' }).count()) === 0);
  expect('the stage still folded', (await stage('build').locator('.stage-band').getAttribute('aria-expanded')) === 'false');
  await stage('build').locator('.stage-band').click();
  await stage('build').locator('.stage-inside').waitFor();

  // ---- its graph, lit
  await runPage.getByRole('tab', { name: 'Graph', exact: true }).click();
  await runPage.locator('.pipeline-graph svg, .pipeline-graph .canvas-map').first().waitFor();
  await page.screenshot({ path: `${out}/pipelines-2-graph${look}.png` });
  await runPage.getByRole('tab', { name: 'Stages', exact: true }).click();
  await stages.waitFor();

  // ---- by the boxes, at three widths and a phone's
  const boxes = async (where, within) => {
    const crowded = await pageCrowded(page, within);
    expect(`nothing over anything else ${where}: ${crowded.join('; ')}`, crowded.length === 0);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`nothing scrolling sideways ${where}, by ${sideways}`, sideways <= 0);
  };
  const bands = async (where) => {
    const wrong = await page.evaluate(() => {
      const said = [];
      for (const band of document.querySelectorAll('.pipeline-run .stage-band')) {
        const at = band.getBoundingClientRect();
        for (const part of band.children) {
          const box = part.getBoundingClientRect();
          if (box.width > 0 && (box.left < at.left - 0.5 || box.right > at.right + 0.5)) said.push(`${part.className} of a band past its sides`);
        }
      }
      for (const log of document.querySelectorAll('.pipeline-run .step-log')) if (log.scrollWidth > log.clientWidth + 1) said.push('a log scrolling sideways');
      return said;
    });
    expect(`each band's parts inside it and no log scrolling sideways ${where}: ${wrong.join('; ')}`, wrong.length === 0);
  };
  for (const [width, height] of [
    [1280, 860],
    [1000, 860],
    [820, 860],
    [390, 800],
  ]) {
    await page.setViewportSize({ width, height });
    const where = `at ${width} wide${look ? ` with ${process.env.LOOK}` : ''}`;
    await boxes(`on the run's page ${where}`, '.view:not([hidden])');
    await bands(where);
    if (width === 390) await page.screenshot({ path: `${out}/pipelines-3-phone${look}.png`, fullPage: true });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // ---- the pipeline's row with a mark of that run, which opens its page
  await page.locator('.tabs .tab', { hasText: 'Shop pipelines' }).first().click();
  const mark = row('check').locator('a.run-mark').first();
  await mark.waitFor();
  const href = await mark.getAttribute('href');
  expect(`the mark leading to the run's page, got ${href}`, href?.startsWith('/run/run_'));
  for (const [width, height] of [
    [1280, 860],
    [1000, 860],
    [820, 860],
    [390, 800],
  ]) {
    await page.setViewportSize({ width, height });
    await boxes(`on the Pipelines page at ${width} wide${look ? ` with ${process.env.LOOK}` : ''}`, '.view:not([hidden])');
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await mark.click();
  await runPage.waitFor();
  await page.waitForFunction((to) => location.pathname === to, href);

  // ---- its address, copied and opened in a second page by somebody who signs in
  await runPage.getByRole('button', { name: 'Copy link', exact: true }).click();
  const address = await page.evaluate(() => navigator.clipboard.readText());
  expect(`the address to name the run by its path, got "${address}"`, address === `${url}${href}`);
  const second = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  second.on('pageerror', (err) => problems.push(err.message));
  await second.goto(address);
  expect('the address to wait while nobody is signed in', second.url() === address);
  await signIn(second);
  await second.locator('.pipeline-run .pipeline-facts[data-status="ok"]').waitFor();
  await second.waitForFunction((to) => location.href === to, address);
  await second.locator('.pipeline-run .pipeline-stage[data-stage="deploy"] .child-run .pipeline-stage[data-stage="ship"]').waitFor();
  await second.screenshot({ path: `${out}/pipelines-4-address${look}.png` });

  // ---- a run named elsewhere opens its page: a run of Activity, and a told line about it with a link to it, as the preview's is
  const ran = href.slice('/run/'.length);
  await writeFile(join(home, 'told.bp'), TOLD);
  await cli('apply', join(home, 'told.bp'), '--param', `run=${ran}`, '--param', `link=${url}${href}`);
  const told = JSON.parse((await cli('inbox', '--json')).stdout).lines[0].id;
  const pages = context.pages().length;
  const showing = page.locator('.tab[aria-selected="true"]');
  await page.evaluate((to) => (location.hash = to), `/inbox/${told}`);
  await page.locator('.told-links a.inbox-link').waitFor();
  await page.locator('.told-links a.inbox-link').click();
  await showing.and(page.locator(`[data-key="run:${ran}"]`)).waitFor();
  expect('the link opened here as a tab, and no page of the browser\'s', context.pages().length === pages);
  await page.locator('.tab', { hasText: 'The check ran.' }).first().click();
  const about = page.locator('.inbox-page-about .inbox-page-opens');
  await about.waitFor();
  expect('what it is about said as a run', (await about.innerText()).includes('pipelines/check run'));
  await page.locator('.tab', { hasText: 'Start' }).first().click();
  await about.waitFor({ state: 'detached' });
  await page.evaluate((to) => (location.hash = to), `/inbox/${told}`);
  await about.click();
  await showing.and(page.locator(`[data-key="run:${ran}"]`)).waitFor();
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await page.locator('.nav').getByText('Activity', { exact: true }).click();
  const fromActivity = page.locator(`.view tr[data-run="${ran}"]`);
  await fromActivity.waitFor();
  await page.locator(`.tab[data-key="run:${ran}"] .tab-close, .tab[data-key="run:${ran}"] button[aria-label^="Close"]`).first().click().catch(() => {});
  await fromActivity.click();
  await showing.and(page.locator(`[data-key="run:${ran}"]`)).waitFor();
  await runPage.waitFor();

  // ---- the project's triggers kept: Run asks them, as whoever presses, and opens the run they begin, or says why not
  await writeFile(join(kept, 'quick.bp'), QUICK);
  await cli('project', 'scan', 'shop');
  await cli('apply', 'std/pipeline', '--project', 'shop', '--param', 'project=shop');
  for (const name of ['quick', 'check']) await cli('record', 'add', 'shop', 'triggers', `pipeline=pipelines/${name}`, 'on=hand');
  await page.reload();
  await page.locator('.rail').waitFor();
  await page.locator('.tabs .tab', { hasText: 'Shop pipelines' }).first().click();
  await row('quick').locator('.pipeline-strip', { hasText: 'never run' }).waitFor();
  const before = await page.locator('.tabs .tab[data-key^="run:"]').count();
  await row('quick').getByRole('button', { name: 'Run', exact: true }).click();
  await page.locator('.toast', { hasText: 'Asked for a run of quick' }).waitFor();
  const asked = page.locator('.pipeline-run .pipeline-stage[data-stage="look"]');
  await asked.waitFor({ timeout: 30_000 });
  expect('a tab of its own for the run the triggers began', (await page.locator('.tabs .tab[data-key^="run:"]').count()) === before + 1);
  const quick = JSON.parse((await cli('instances', '--all', '--json')).stdout).instances.find((one) => one.blueprint === 'pipelines/quick');
  expect(`the run under the row's key, the pipeline's name, got ${JSON.stringify(quick)}`, quick?.key === 'pipelines-quick');
  // The check says nothing of what it runs with, so nobody runs it by hand: whoever pressed is told why, on the page and in their inbox.
  await page.locator('.tabs .tab', { hasText: 'Shop pipelines' }).first().click();
  await row('check').getByRole('button', { name: 'Run', exact: true }).click();
  await page.locator('.toast.bad', { hasText: 'pipelines/check was not run' }).waitFor();
  const inbox = JSON.parse((await cli('inbox', '--json')).stdout).lines.map((line) => line.says ?? '');
  expect(`whoever pressed told why in their inbox, got ${JSON.stringify(inbox)}`, inbox.some((says) => says.includes('`pipelines/check` was not run') && says.includes('permissions')));
  await page.screenshot({ path: `${out}/pipelines-5-asked${look}.png` });

  // ---- two presses of Run at one moment, from two pages: each opens the run its own press began and never the other's
  const other = await context.newPage();
  other.on('pageerror', (err) => problems.push(err.message));
  await other.goto(url);
  await other.locator('.rail').waitFor();
  const quickOf = (one) => one.locator('.pipelines-page .pipeline-row[data-pipeline="pipelines/quick"]');
  const runTabs = async (one) => one.locator('.tabs .tab[data-key^="run:"]').evaluateAll((tabs) => tabs.map((tab) => tab.dataset.key));
  const pressed = [];
  for (const one of [page, other]) {
    await one.locator('.tabs .tab', { hasText: 'Shop pipelines' }).first().click();
    await quickOf(one).getByRole('button', { name: 'Run', exact: true }).waitFor();
    pressed.push(await runTabs(one));
  }
  await Promise.all([page, other].map((one) => quickOf(one).getByRole('button', { name: 'Run', exact: true }).click()));
  const opened = [];
  for (const [at, one] of [page, other].entries()) {
    await one.waitForFunction((had) => [...document.querySelectorAll('.tabs .tab[data-key^="run:"]')].some((tab) => !had.includes(tab.dataset.key)), pressed[at], { timeout: 30_000 });
    const now = (await runTabs(one)).filter((key) => !pressed[at].includes(key));
    expect(`one run opened by each press, got ${JSON.stringify(now)}`, now.length === 1);
    opened.push(now[0]);
  }
  expect(`each press to open its own run, got ${JSON.stringify(opened)}`, opened[0] !== opened[1]);
  const of = await page.evaluate(async (keys) => {
    const read = async (path) => (await fetch(`/api${path}`, { headers: { 'x-page-key': localStorage['page-key'] } })).json();
    const all = [];
    for (const key of keys) {
      const ran = await read(`/runs/${key.slice('run:'.length)}`);
      all.push([ran.flow, (await read(`/instances/${ran.instance}`)).instance.blueprint]);
    }
    return all;
  }, opened);
  expect(`both runs to be runs of quick's steps, got ${JSON.stringify(of)}`, of.every(([flow, blueprint]) => flow === 'main' && blueprint === 'pipelines/quick'));
  await other.close();

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/pipelines-failed${look}.png`, fullPage: true }).catch(() => {});
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
