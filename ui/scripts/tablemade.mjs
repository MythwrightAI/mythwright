// A table made from the side panel (`src/views/tablemade.tsx`, over
// `src/tablemade.ts`; `std/table`, applied as whoever asks).
//
//   DAEMON=../target/debug/factory node scripts/tablemade.mjs [out-dir]
//
// Shop has a board, a table of notes and a diary kept private, and an
// Insights folder made from a terminal. On Ada's page: New… on Shop's row
// opening Folder… and Table… beside it; Table… a dialog with the caret in
// the name, a name the project has a table of (the board, the notes)
// refused before anything is asked with Make it closed, and a table made
// laid out as a list, opened as a tab called what it was called, listed among
// what nobody placed in Shop, in the usual order, on a second page of Ada's that is never read
// again; the same from Insights's line, the table put in Insights on both
// pages. On Nell's page (a member, who may not read the diary): a table
// called Diary refused by the daemon in its sentence, which says nothing
// of the diary, with the dialog staying as it was typed. Somebody who may
// not apply (a page told so) offered Folder… alone. The menus and the
// dialog by their boxes at three widths and a phone's, where they are
// offered by touch. The environment variables are those of
// walkthrough.mjs; `LOOK` is worn as every check wears it.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { resourcesOf, unfold, unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'tablemade-data');
const home = join(out, 'tablemade-home');
const hers = join(out, 'tablemade-home-nell');
const tables = join(out, 'tablemade-tables.bp');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
await writeFile(
  tables,
  `blueprint: shop-tables
permissions: [projects]
steps:
  notes:
    do: collection.ensure
    with:
      name: notes
      fields: { title: { type: text, required: true } }
  diary:
    do: collection.ensure
    with:
      name: diary
      reads: private:journal
      fields: { title: { type: text, required: true } }
`,
);
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
  await cli('project', 'outline', 'shop', '--folder', 'Insights');

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
  const touching = (page) => !!page.context()._options?.hasTouch;
  const press = (page, locator) => (touching(page) ? locator.tap() : locator.click());

  /** A page signed in as `who`, with the Resources panel showing and everything of it unfolded; `lacking` is told it holds no `blueprints:run`. */
  const signedIn = async ({ who = 'ada', how = { viewport: { width: 1280, height: 860 } }, lacking = false } = {}) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    if (lacking) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        // What a member holds, but applying: the daemon says who somebody is, and the page goes by it.
        me.principal.scopes = ['projects', 'channels', 'inbox', 'themes', 'arranged', 'clock', 'board', 'records', 'columns', 'blueprints:read'];
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(url);
    await page.getByLabel('Email').fill(`${who}@example.com`);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    if (how.isMobile) {
      await page.getByRole('button', { name: 'Expand the side panel' }).tap();
      await sideSettled(page);
    }
    await unfoldAll(page);
    const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await press(page, resources);
    await resourcesOf(page).waitFor();
    await shop(page).locator(':scope > .tree').waitFor();
    return page;
  };
  const shop = (page) => resourcesOf(page).locator('[data-project="shop"]');
  /** What is drawn under Shop, line by line, each said with the folders it is in; read in the page. */
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
  const drawn = async (page, meant, what) => {
    const wanted = JSON.stringify(meant);
    const reached = await page
      .waitForFunction(`JSON.stringify((${LINES.toString()})()) === ${JSON.stringify(wanted)}`, undefined, { timeout: 15_000 })
      .then(() => true, () => false);
    if (!reached) expect(`${what}: ${wanted}, got ${JSON.stringify(await page.evaluate(LINES))}`, false);
  };
  const crowded = async (page, what, within) => {
    const wrong = (await Promise.all(within.map((one) => pageCrowded(page, one)))).flat();
    expect(`${what}: nothing over anything else or past its sides, got ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`${what}: nothing to scroll sideways, got ${sideways}px`, sideways <= 0);
  };
  const rowMenu = (page) => page.locator('.row-menu:not(.row-beside)');
  const beside = (page) => page.locator('.row-menu.row-beside');
  /** New… on the line whose menu is called `label`: what it offers beside it, which is left open. */
  const newOn = async (page, label, shot) => {
    const more = resourcesOf(page).getByRole('button', { name: label, exact: true });
    await more.waitFor();
    await press(page, more);
    await rowMenu(page).waitFor();
    await press(page, rowMenu(page).getByRole('menuitem', { name: 'New…', exact: true }));
    await beside(page).waitFor();
    // Placed before it is drawn: waited for until it is.
    await page.waitForFunction(() => getComputedStyle(document.querySelector('.row-beside')).visibility !== 'hidden');
    if (shot) {
      await crowded(page, `New… ${shot}`, ['.row-menu:not(.row-beside)', '.row-menu.row-beside']);
      await page.screenshot({ path: `${out}/tablemade-${shot}.png` });
    }
    return (await beside(page).locator(':scope > button .truncate').allTextContents()).map((one) => one.trim());
  };
  const dialog = (page) => page.locator('.modal');
  const makeIt = (page) => dialog(page).getByRole('button', { name: 'Make it', exact: true });
  /** The dialog Table… opens, from the line whose menu is called `label`. */
  const tableOn = async (page, label, shot) => {
    const offered = await newOn(page, label, shot && `${shot}-menu`);
    expect(`New… to offer a folder and a table, got ${offered.join(', ')}`, JSON.stringify(offered) === JSON.stringify(['Folder…', 'Table…']));
    await press(page, beside(page).getByRole('menuitem', { name: 'Table…', exact: true }));
    await dialog(page).waitFor();
    // The caret is in its name.
    await page.waitForFunction(() => document.activeElement?.matches('.modal input:not([type="radio"])'));
  };

  // ── Made from Shop's row ─────────────────────────────────────────────────
  const page = await signedIn();
  const second = await signedIn();
  const usual = ['Insights/', 'Board', 'Diary', 'Notes', 'Blueprints'];
  await drawn(page, usual, 'Shop as the terminal arranged it');
  await tableOn(page, 'More for Shop', 'a');
  expect('the dialog to say where it is made', (await dialog(page).locator('h2').textContent()).includes('New table in Shop'));
  const name = dialog(page).getByLabel('Table name');
  for (const taken of ['Board', 'notes']) {
    await name.fill(taken);
    expect(`Make it closed for ${taken}`, await makeIt(page).isDisabled());
    const says = await dialog(page).locator('#table-made-says').textContent();
    expect(`${taken} refused before anything is asked, got ${says}`, says.includes('has a table called') && says.includes('already'));
  }
  await name.fill('Reading list');
  await dialog(page).getByLabel('List', { exact: true }).check();
  await crowded(page, 'the dialog that makes a table', ['.modal']);
  await page.screenshot({ path: `${out}/tablemade-b-dialog.png` });
  await makeIt(page).click();
  await dialog(page).waitFor({ state: 'detached' });
  await page.locator('.view-head h1', { hasText: 'Reading list' }).waitFor();
  expect('its tab called what it was called', (await page.locator('.tab', { hasText: 'Reading list' }).count()) > 0);
  const made = ['Insights/', 'Board', 'Diary', 'Notes', 'Reading list', 'Blueprints'];
  await drawn(page, made, 'the table among what nobody placed, in the usual order');
  await drawn(second, made, 'the table on the page never read again');

  // ── Made in Insights ─────────────────────────────────────────────────────
  await tableOn(page, 'Insights: more');
  expect('the dialog to say it is made in Insights', (await dialog(page).locator('h2').textContent()).includes('New table in Insights'));
  await dialog(page).getByLabel('Table name').fill('Test runs');
  await dialog(page).getByLabel('Gallery', { exact: true }).check();
  await makeIt(page).click();
  await dialog(page).waitFor({ state: 'detached' });
  await page.locator('.view-head h1', { hasText: 'Test runs' }).waitFor();
  const inside = ['Insights/', 'Insights/Test runs', 'Board', 'Diary', 'Notes', 'Reading list', 'Blueprints'];
  await drawn(page, inside, 'the table in Insights, unfolded for it');
  await unfold(second, shop(second).locator('[data-outline] > div > .nav-line > .nav-item', { hasText: 'Insights' }));
  await drawn(second, inside, 'the table in Insights on the page never read again');
  const reader = JSON.parse((await cli('key', 'reader', '--scope', 'projects:read', '--json')).stdout).token;
  const kinds = await fetch(`${url}/api/collections?project=shop`, { headers: { authorization: `Bearer ${reader}` } }).then((one) => one.json());
  const kindOf = (table) => kinds.collections.find((one) => one.name === table)?.views?.[0]?.kind;
  expect(`each laid out as it was chosen, got ${kindOf('reading-list')} and ${kindOf('test-runs')}`, kindOf('reading-list') === 'list' && kindOf('test-runs') === 'gallery');

  // ── A member: the daemon's refusal, in words that say nothing of the diary ──
  const hers_ = await signedIn({ who: 'nell' });
  await drawn(hers_, ['Insights/', 'Insights/Test runs', 'Board', 'Notes', 'Reading list', 'Blueprints'], 'Shop on her page, with no diary');
  await tableOn(hers_, 'More for Shop');
  await dialog(hers_).getByLabel('Table name').fill('Diary');
  expect('Make it open for a name her page does not know', !(await makeIt(hers_).isDisabled()));
  await makeIt(hers_).click();
  const refused = dialog(hers_).locator('[role="alert"]');
  await refused.waitFor();
  const words = await refused.textContent();
  expect(`the daemon's refusal said in its sentence, saying nothing of the diary, got ${words}`, words.startsWith('diary cannot be made in shop') && !words.includes('already'));
  expect('the dialog staying as it was typed', (await dialog(hers_).getByLabel('Table name').inputValue()) === 'Diary');
  await crowded(hers_, 'the dialog with what the daemon refused', ['.modal']);
  await hers_.screenshot({ path: `${out}/tablemade-c-refused.png` });
  await dialog(hers_).getByRole('button', { name: 'Cancel', exact: true }).click();

  // ── Somebody who may not apply: Folder… alone ──────────────────────────
  const lacking = await signedIn({ lacking: true });
  const theirs = await newOn(lacking, 'More for Shop');
  expect(`New… to offer only a folder to somebody who may not apply, got ${theirs.join(', ')}`, JSON.stringify(theirs) === JSON.stringify(['Folder…']));

  // ── By the boxes: three widths and a phone's ─────────────────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await page.mouse.move(width - 10, 800);
    await tableOn(page, 'More for Shop', `d-${width}`);
    await crowded(page, `the dialog at ${width} wide`, ['.modal']);
    await dialog(page).getByRole('button', { name: 'Cancel', exact: true }).click();
    await dialog(page).waitFor({ state: 'detached' });
  }
  const phone = await signedIn({ how: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 } });
  await tableOn(phone, 'More for Shop', 'e-phone');
  await crowded(phone, "the dialog at a phone's width", ['.modal']);
  await phone.screenshot({ path: `${out}/tablemade-e-phone-dialog.png` });

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
