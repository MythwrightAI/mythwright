// A project's things arranged in the side panel, in folders of the
// arrangement's own (`src/outline.ts`, kept by the daemon for everybody).
//
//   DAEMON=../target/debug/factory node scripts/outlined.mjs [out-dir]
//
// Shop has a board and four tables, one of them, its diary, kept private.
// Nobody has arranged it, so it is drawn as it always was, line for line.
// Then, from a terminal, an Insights folder is made and three tables are
// put in it (`project outline`), and a page of Ada's that is never read
// again draws Insights under Shop, folded as everything of the panel
// starts, what nobody placed after it in the usual order, and the three
// tables in it once it is unfolded. Nell, a member, who may not read the
// diary, has Insights drawn with the two tables she may read. The panel's
// box finds a table by the name of the folder it is in (`insights runs`),
// holding Insights open though it was folded, and Escape leaves the
// folds as they were; Collapse all folds it and Expand all unfolds it. A
// folder called something else, and one removed (what was in it taking
// its place), are drawn so on the page that is never read again, and an
// arrangement forgotten draws the panel as it was before it. Nothing of
// the panel lies over anything else or scrolls sideways at three widths
// and a phone's. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { resourcesOf, unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'outlined-data');
const home = join(out, 'outlined-home');
const hers = join(out, 'outlined-home-nell');
const tables = join(out, 'outlined-tables.bp');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
await writeFile(
  tables,
  `blueprint: shop-tables
permissions: [projects]
steps:
${['test-runs', 'deployments', 'notes', 'diary']
  .map((name) => `  ${name.replace('-', '_')}:\n    do: collection.ensure\n    with:\n      name: ${name}\n${name === 'diary' ? '      reads: private:journal\n' : ''}      fields: { title: { type: text, required: true } }\n`)
  .join('')}`,
);
// The daemon and the command line are given homes of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env: as(home),
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('apply', tables, '--project', 'shop');
  const outline = (...args) => cli('project', 'outline', 'shop', ...args);

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
  // Somebody's page, signed in, with the Resources panel showing and everything of it unfolded.
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    if (how.isMobile) {
      await page.getByRole('button', { name: 'Expand the side panel' }).tap();
      await sideSettled(page);
    }
    await unfoldAll(page);
    // Expand all leaves the panel as it was: Resources is shown, as a person would to look at it.
    const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await (how.isMobile ? resources.tap() : resources.click());
    await resourcesOf(page).waitFor();
    await shop(page).locator(':scope > .tree').waitFor();
    return page;
  };
  const shop = (page) => resourcesOf(page).locator('[data-project="shop"]');
  /** What is drawn under Shop, line by line, each said with the folders it is in (`Insights/Deployments`); read in the page. */
  const LINES = () => {
    const project = document.querySelector('nav[aria-label="Resources"] [data-project="shop"] > .tree');
    if (!project) return [];
    const said = [];
    const walk = (box, within) => {
      for (const one of box.children) {
        if (one.matches('[data-outline]')) {
          const name = one.querySelector('.nav-item .truncate').textContent.trim();
          said.push(`${[...within, name].join('/')}/`);
          const under = one.querySelector(':scope > div > .tree');
          if (under) walk(under, [...within, name]);
          continue;
        }
        const item = one.matches('.nav-item') ? one : one.querySelector(':scope > .nav-row .nav-item, :scope > .nav-item, :scope > .nav-line > .nav-item');
        if (item) said.push([...within, item.querySelector('.truncate').textContent.trim()].join('/'));
      }
    };
    walk(project, []);
    return said;
  };
  // What is said comes a moment after it is written; the page is never read again for it, and is waited on as it draws.
  const drawn = async (page, meant, what) => {
    const wanted = JSON.stringify(meant);
    const reached = await page
      .waitForFunction(`JSON.stringify((${LINES.toString()})()) === ${JSON.stringify(wanted)}`, undefined, { timeout: 15_000 })
      .then(() => true, () => false);
    if (!reached) expect(`${what}: ${wanted}, got ${JSON.stringify(await page.evaluate(LINES))}`, false);
  };
  const crowded = async (page, what) => {
    const wrong = await pageCrowded(page, 'nav[aria-label="Resources"]');
    expect(`${what}: nothing of the panel over anything else or past its sides, got ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`${what}: nothing to scroll sideways, got ${sideways}px`, sideways <= 0);
  };
  // A folder's line, which has its menu after it for whoever may arrange the project.
  const folderLine = (page, name) => shop(page).locator('[data-outline] > div > .nav-item, [data-outline] > div > .nav-line > .nav-item', { hasText: name }).first();

  // ── Nobody arranged it: as it always was ────────────────────────────────
  const page = await signedIn('ada@example.com');
  const usual = ['Board', 'Deployments', 'Diary', 'Notes', 'Test-runs', 'Blueprints'];
  await drawn(page, usual, "Shop as nobody arranged it, in the order it always was");
  await page.screenshot({ path: `${out}/outlined-1-usual.png` });
  const second = await signedIn('ada@example.com');
  const theirs = await signedIn('nell@example.com');
  await drawn(theirs, ['Board', 'Deployments', 'Notes', 'Test-runs', 'Blueprints'], "Shop on Nell's page, without the diary she may not read");

  // ── Arranged from a terminal, drawn on a page that is never read again ───
  const made = (await outline('--folder', 'Insights', '--put', 'table:test-runs', 'table:deployments', 'table:diary', '--in', 'Insights')).stdout;
  expect(`the terminal to say what it made, got ${made}`, made.includes('made the folder `Insights`') && made.includes('    table:diary'));
  // A folder starts folded, as everything of the panel does; what nobody placed is after it in the usual order.
  await drawn(second, ['Insights/', 'Board', 'Notes', 'Blueprints'], 'Shop arranged, Insights folded');
  await folderLine(second, 'Insights').click();
  await drawn(second, ['Insights/', 'Insights/Test-runs', 'Insights/Deployments', 'Insights/Diary', 'Board', 'Notes', 'Blueprints'], 'Insights unfolded');
  await second.screenshot({ path: `${out}/outlined-2-arranged.png` });
  // A table in it opens as it always did.
  await shop(second).locator('.nav-item', { hasText: 'Test-runs' }).click();
  await second.locator('.view-head h1', { hasText: 'Test-runs' }).waitFor();
  // Nell's Insights holds what she may read of it.
  await folderLine(theirs, 'Insights').click();
  await drawn(theirs, ['Insights/', 'Insights/Test-runs', 'Insights/Deployments', 'Board', 'Notes', 'Blueprints'], "Insights on Nell's page");
  await theirs.screenshot({ path: `${out}/outlined-3-member.png` });

  // ── Found by the panel's box, and the folds as they were after ───────────
  await resourcesOf(page).getByRole('button', { name: 'Collapse all', exact: true }).click();
  await shop(page).locator(':scope > .tree').waitFor({ state: 'detached' });
  const box = resourcesOf(page).locator('.panel-seek input');
  await box.fill('insights runs');
  await drawn(page, ['Insights/', 'Insights/Test-runs'], 'what is found by the name of its folder, the folder held open');
  await page.screenshot({ path: `${out}/outlined-4-found.png` });
  await box.press('Escape');
  await shop(page).locator(':scope > .tree').waitFor({ state: 'detached' });
  expect('Shop folded again once the box is emptied', (await shop(page).locator(':scope > .tree').count()) === 0);
  await unfoldAll(page);
  await drawn(page, ['Insights/', 'Insights/Test-runs', 'Insights/Deployments', 'Insights/Diary', 'Board', 'Notes', 'Blueprints'], 'Expand all unfolding Insights with the rest');

  // ── Nothing over anything else, at three widths and a phone's ───────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await crowded(page, `the side panel at ${width} wide`);
  }
  await page.screenshot({ path: `${out}/outlined-5-narrow.png` });
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await drawn(phone, ['Insights/', 'Insights/Test-runs', 'Insights/Deployments', 'Insights/Diary', 'Board', 'Notes', 'Blueprints'], "Shop on a phone, Expand all having unfolded Insights");
  await crowded(phone, "the side panel at a phone's width");
  await phone.screenshot({ path: `${out}/outlined-6-phone.png` });

  // ── Called something else, removed and forgotten, from a terminal ────────
  await outline('--rename', 'Insights', '--to', 'Runs');
  await drawn(second, ['Runs/', 'Runs/Test-runs', 'Runs/Deployments', 'Runs/Diary', 'Board', 'Notes', 'Blueprints'], 'the folder called something else');
  await outline('--remove', 'Runs');
  await drawn(second, ['Test-runs', 'Deployments', 'Diary', 'Board', 'Notes', 'Blueprints'], 'what was in a folder removed where it was');
  await outline('--forget');
  await drawn(second, usual, 'Shop as it was before anybody arranged it');
  await second.screenshot({ path: `${out}/outlined-7-forgotten.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
  await rm(tables, { force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
