// An environment loaded from a branch, in its drawer: a project that is a
// repository with branches of its own and a remote beside it with one
// more, made by the check; the blueprint it is loaded by said in the
// drawer, a name the daemon refuses said in its sentence; the branches
// offered, the remote's come by only once Fetch is pressed and said by
// what came; a branch chosen and Run, the deploy's stages appearing and
// ending as it goes, each by what it says of itself, with the page never
// read again, what it gave out shown and its address a link; the last run
// said in the environments' table; run again for a branch whose build
// fails, the step that failed shown with what it said and the table
// saying so, with no head of the table broken inside a word beside the
// open drawer; an environment that names no blueprint it is loaded by
// saying so with the command that names one, and once something is
// applied under it from a terminal by a key of its own, with no Run, its
// last run being that, said as what was applied, beside what runs under it
// and never "never run", the page not read again; a project that is no
// repository offering no Run and saying why; and nothing of the drawer over anything else or scrolling
// sideways at 1280, 900, 640 and 390 wide. And the environment's branches
// said in its drawer (`views/envrules.tsx`): Run of one it does not name
// refused in the daemon's sentence with nothing started and its last run as
// it was, the deploy's runs listed under it the newest first, the failed one
// its last run, and its line opening the run's page; and said as any again.
//
//   DAEMON=../target/debug/factory node scripts/deployed.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs. Nothing here
// reaches the network: the remote is a repository in the check's folder.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { headsBroken } from './heads.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'deployed-data');
const home = join(out, 'deployed-home');
const shop = join(out, 'deployed-shop');
const remote = join(out, 'deployed-remote.git');
const other = join(out, 'deployed-other');
const plain = join(out, 'deployed-plain');

/** The project's own deploy: a few seconds over several steps, and a build that fails for the branch called `broken`. */
const DEPLOY = `blueprint: deploy
description: Build the branch it is handed and serve it.
params:
  branch: { type: string }
  commit: { type: string }
steps:
  checked:
    description: The branch checked out.
    do: wait
    with: { for: 2s }
  built:
    description: The branch built.
    needs: [checked]
    do: wait
    with: { for: 2s }
  broke:
    description: The build that does not build.
    needs: [built]
    when: \${ params.branch == "broken" }
    do: fail
    with: { message: "the branch broken does not build: src/main.rs is not there" }
  served:
    description: The build served.
    needs: [built]
    when: \${ params.branch != "broken" }
    do: wait
    with: { for: 1s }
outputs:
  address: http://127.0.0.1:7801/
`;

/** What is applied under an environment by hand, by a key of its own and not through Run: it stays, heard nothing. */
const PREVIEW = `blueprint: preview
description: Serve the branch as it is, for a look.
steps:
  served:
    description: Served for a look.
    do: wait
    with: { for: 1s }
on:
  poke: { event: preview.poke, do: noop }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop, remote, other, plain]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// A home of its own, so that nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// Git as the check asks it: who commits is what the repository says, never whoever runs the check.
const gitEnv = Object.fromEntries(Object.entries(env).filter(([name]) => !/^GIT_(AUTHOR|COMMITTER)_/.test(name)));
const git = (dir, ...args) => run('git', ['-C', dir, ...args], { env: gitEnv });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};

let failed = null;
let shot = null;
try {
  // A repository with branches of its own, and a remote beside it with one more.
  for (const dir of [shop, plain]) {
    await mkdir(dir, { recursive: true });
    await writeFile(join(dir, 'README.md'), '# A project\n');
  }
  await git(shop, 'init', '--quiet', '-b', 'main');
  await git(shop, 'config', 'user.name', 'Ada');
  await git(shop, 'config', 'user.email', 'ada@example.com');
  await git(shop, 'add', '-A');
  await git(shop, 'commit', '--quiet', '-m', 'The shop as it began');
  await git(shop, 'branch', 'broken');
  await git(shop, 'checkout', '--quiet', '-b', 'dev');
  await git(shop, 'commit', '--quiet', '--allow-empty', '-m', 'A cart that keeps what is in it');
  await git(shop, 'checkout', '--quiet', 'main');
  await run('git', ['init', '--quiet', '--bare', remote], { env: gitEnv });
  await git(shop, 'remote', 'add', 'origin', remote);
  await run('git', ['clone', '--quiet', remote, other], { env: gitEnv });
  await git(other, 'config', 'user.name', 'Grace');
  await git(other, 'config', 'user.email', 'grace@example.com');
  await git(other, 'checkout', '--quiet', '-b', 'feature');
  await git(other, 'commit', '--quiet', '--allow-empty', '-m', 'Checkout in one step, as somebody else wrote it');
  await git(other, 'push', '--quiet', 'origin', 'feature');

  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  await cli('project', 'create', '--bare', 'Plain', '--path', plain);
  const file = join(out, 'deployed-deploy.bp');
  await writeFile(file, DEPLOY);
  await cli('file', 'write', 'shop', '@blueprints/deploy.bp', '--from', file);
  await cli('project', 'environment', 'shop', 'dev', 'add');
  await cli('project', 'environment', 'plain', 'dev', 'add');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  shot = () => page.screenshot({ path: `${out}/deployed-failed.png` }).catch(() => {});
  page.on('pageerror', (err) => problems.push(err.message));
  // The page is read once: everything after is what it hears.
  let reads = 0;
  // A page loaded again, and not the address bar following the tab showing, which the browser calls a navigation too.
  page.on('domcontentloaded', () => (reads += 1));

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const nav = page.locator('.nav');
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(page);
  let readsAfterSignIn = reads;

  // ---- the environment opened, and the blueprint it is loaded by said there ------------------
  await nav.locator('[data-project="shop"]').getByText('Environments', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop environments', level: 1 }).waitFor();
  const panel = page.locator('.envs-page');
  const row = panel.locator('tr[data-environment="dev"]');
  // Nothing has run under it and it names no blueprint it is loaded by: said, with the command that names one.
  const unsaid = /^nothing has run under it, and no blueprint loads it from a branch: `\S+ project environment shop dev deploy <blueprint>` names one$/;
  await row.locator('[data-last-run]', { hasText: unsaid }).waitFor();
  await row.click();
  const drawer = drawerAtRest(page);
  const opened = drawer.locator('[data-opened="dev"]');
  const loaded = opened.locator('.envs-loaded');
  await loaded.waitFor();
  const runButton = loaded.getByRole('button', { name: 'Run', exact: true });
  await loaded.getByText('Say the blueprint it is loaded by first.').waitFor();
  expect('Run closed until it says a blueprint', await runButton.isDisabled());
  const field = loaded.getByLabel('The blueprint it is loaded by');
  await field.fill('nowhere');
  await field.press('Enter');
  await page.getByText('there is no blueprint `nowhere` for `shop`', { exact: false }).first().waitFor();
  await field.fill('deploy');
  const said = page.waitForResponse((answer) => answer.url().endsWith('/environments/dev/deploy') && answer.request().method() === 'PUT' && answer.ok());
  await field.press('Enter');
  await said;
  await loaded.getByText('It has not been run yet.').waitFor();
  await row.locator('[data-last-run]', { hasText: /^never run$/ }).waitFor();

  // ---- the branches, the remote's only once somebody fetches ---------------------------------
  const select = loaded.getByLabel('The branch to load it from');
  const offered = async () => (await select.locator('option').allInnerTexts()).map((text) => text.split(' · ')[0]);
  await page.waitForFunction(() => document.querySelectorAll('.envs-loaded select option').length >= 3);
  expect(`the project's own branches offered, got ${await offered()}`, JSON.stringify(await offered()) === '["broken","dev","main"]');
  await page.screenshot({ path: `${out}/deployed-0-branches.png` });
  await loaded.getByRole('button', { name: 'Fetch', exact: true }).click();
  await page.getByText('Fetched. new: `origin/feature`.', { exact: true }).first().waitFor();
  await page.waitForFunction(() => [...document.querySelectorAll('.envs-loaded select option')].some((one) => one.value === 'origin/feature'));
  expect(`the remote's branch offered once fetched, got ${await offered()}`, (await offered()).includes('origin/feature'));

  // ---- Run, and the stages as they go ---------------------------------------------------------
  await select.selectOption('origin/feature');
  expect('Run open once a branch is chosen', !(await runButton.isDisabled()));
  await page.screenshot({ path: `${out}/deployed-1-chosen.png` });
  await runButton.click();
  const ran = loaded.locator('.envs-ran');
  // The stages appear and go on with nothing read again: the first under way, said by what it says of itself.
  const stage = (says) => ran.locator('tr', { has: page.locator('.step-says', { hasText: says }) });
  await stage('The branch checked out').locator('.badge.running').waitFor();
  await page.screenshot({ path: `${out}/deployed-2-running.png` });
  await stage('The branch checked out').locator('.badge.ok').waitFor();
  await stage('The branch built').locator('.badge.running').waitFor();
  await page.screenshot({ path: `${out}/deployed-3-building.png` });
  await stage('The build served').locator('.badge.ok').waitFor({ timeout: 20_000 });
  await ran.locator('.row .badge.ok').first().waitFor();
  const link = ran.locator('.envs-gave a[href="http://127.0.0.1:7801/"]');
  await link.waitFor();
  expect('the address it gave out a link that opens a tab of its own', (await link.getAttribute('target')) === '_blank');
  await ran.getByText(/^Last run of deploy: origin\/feature at [0-9a-f]{7}/).waitFor();
  await row.locator('[data-last-run]', { hasText: /^origin\/feature at [0-9a-f]{7}, went well/ }).waitFor();
  await page.screenshot({ path: `${out}/deployed-4-done.png` });

  // ---- run again for a branch whose build fails ----------------------------------------------
  await select.selectOption('broken');
  await runButton.click();
  await stage('The build that does not build').locator('.badge.failed').waitFor({ timeout: 20_000 });
  await ran.getByText('the branch broken does not build', { exact: false }).first().waitFor();
  await row.locator('[data-last-run]', { hasText: /^broken at [0-9a-f]{7}, failed/ }).waitFor();
  expect('the run that failed giving out nothing shown as gone well', (await ran.locator('.row .badge.failed').count()) > 0);
  await page.screenshot({ path: `${out}/deployed-5-failed.png` });
  // The whole of why, opened under its line.
  await stage('The build that does not build').locator('.run-gave').click();
  await ran.locator('.run-whole', { hasText: 'src/main.rs is not there' }).waitFor();
  await page.screenshot({ path: `${out}/deployed-6-why.png` });

  // ---- which branches it runs: one it does not name refused at Run, in the daemon's sentence ---
  const rules = opened.locator('[data-rules]');
  const branches = rules.getByLabel('The branches it runs, by spaces');
  await branches.fill('main origin/*');
  await branches.press('Enter');
  await rules.locator('[data-rule="branches"] [data-said]', { hasText: 'Only main and origin/* may be run under it.' }).waitFor();
  const before = await row.locator('[data-last-run]').innerText();
  await select.selectOption('broken');
  await runButton.click();
  await page.getByText('`broken` may not be run under `dev` of `shop`, which lets main, origin/* alone', { exact: false }).first().waitFor();
  expect('the last run as it was, with nothing started', (await row.locator('[data-last-run]').innerText()) === before);
  await page.screenshot({ path: `${out}/deployed-10-branch-refused.png` });
  // The deploy's runs under it, the newest first: the one that failed is its last run, and opens its page.
  const runs = opened.locator('[data-runs] [data-acted]');
  await runs.first().filter({ hasText: /^.*deploy as deploy-dev, failed, by Ada, / }).waitFor();
  expect('the first of them its last run', (await runs.first().innerText()).includes('its last run'));
  expect('the run that went well listed after it', /deploy as deploy-dev, went well, by Ada, /.test(await runs.nth(1).innerText()));
  // A link to a run's page opens it beside what is open, the bar saying its address (`/run/<id>`), and reads nothing again.
  const beforeLink = reads;
  await page.evaluate(() => (window.__stayed = true));
  await runs.first().getByRole('link').click();
  await page.locator('.pipeline-run .pipeline-facts', { hasText: 'under dev' }).waitFor();
  expect('the page not loaded again by a run\'s link', await page.evaluate(() => window.__stayed === true));
  readsAfterSignIn += reads - beforeLink;
  await page.screenshot({ path: `${out}/deployed-11-its-run.png` });
  await page.locator('.tab', { hasText: 'Shop environments' }).first().click();
  await opened.waitFor();
  await branches.fill('');
  await branches.press('Enter');
  await rules.locator('[data-rule="branches"] [data-said]', { hasText: 'Any branch may be run under it.' }).waitFor();

  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    const wrong = await pageCrowded(page, '.drawer');
    expect(`nothing of the drawer over anything else or scrolling sideways at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    if (width > 390) {
      // Beside the open drawer, the environments' page: no head broken inside a word, nothing over anything else.
      const broken = await headsBroken(page, '.envs-table th');
      expect(`no head of the table broken inside a word at ${width}: ${broken.join('; ')}`, broken.length === 0);
      const crowded = await pageCrowded(page, '.envs-page');
      expect(`nothing of the page beside the drawer over anything else at ${width}: ${crowded.join('; ')}`, crowded.length === 0);
    }
    await page.screenshot({ path: `${out}/deployed-7-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  expect(`the page never read again, read ${reads - readsAfterSignIn} more times`, reads === readsAfterSignIn);

  // ---- a project that is no repository has no branch and offers no Run -------------------------
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  if (!(await nav.isVisible())) await resources.click();
  await nav.locator('[data-project="plain"]').getByText('Environments', { exact: true }).click();
  await page.getByRole('heading', { name: 'Plain environments', level: 1 }).waitFor();

  // ---- something applied under an environment from a terminal, by a key of its own and no Run -----
  const plainRow = page.locator('.envs-page tr[data-environment="dev"]');
  await plainRow.locator('[data-last-run]', { hasText: /^nothing has run under it, and no blueprint loads it from a branch/ }).waitFor();
  const preview = join(out, 'deployed-preview.bp');
  await writeFile(preview, PREVIEW);
  await cli('apply', preview, '--project', 'plain', '--environment', 'dev', '--key', 'dev-preview');
  // Followed with the page not read again: what ran under it, said as what was applied, and never "never run" beside it.
  await plainRow.locator('[data-last-run]', { hasText: /^preview as dev-preview, (running|went well), by Ada, / }).waitFor();
  await plainRow.getByText('1 running', { exact: true }).waitFor();
  await plainRow.locator('[data-last-run]', { hasText: /went well/ }).waitFor();
  expect('nothing said never to have run beside something running', !(await page.locator('.envs-page').innerText()).includes('never run'));
  await page.screenshot({ path: `${out}/deployed-8-applied.png` });
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    const crowded = await pageCrowded(page, '.envs-page');
    expect(`nothing of the page with what ran under it over anything else at ${width}: ${crowded.join('; ')}`, crowded.length === 0);
    if (width > 390) {
      const broken = await headsBroken(page, '.envs-table th');
      expect(`no head of the table broken inside a word at ${width}: ${broken.join('; ')}`, broken.length === 0);
    }
    await page.screenshot({ path: `${out}/deployed-8-applied-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  expect(`the page never read again, read ${reads - readsAfterSignIn} more times`, reads === readsAfterSignIn);
  await page.locator('.envs-page tr[data-environment="dev"]').click();
  const bare = drawerAtRest(page).locator('[data-opened="dev"] .envs-loaded');
  await bare.waitFor();
  await bare.getByLabel('The blueprint it is loaded by').fill('std/board');
  await bare.getByLabel('The blueprint it is loaded by').press('Enter');
  await bare.getByText('`plain` is no repository', { exact: false }).waitFor();
  expect('no Run for a project that is no repository', await bare.getByRole('button', { name: 'Run', exact: true }).isDisabled());
  expect('no Fetch for a project that is no repository', (await bare.getByRole('button', { name: 'Fetch', exact: true }).count()) === 0);
  await page.screenshot({ path: `${out}/deployed-9-no-repository.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await shot?.();
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop, remote, other, plain]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
