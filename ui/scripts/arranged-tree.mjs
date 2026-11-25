// A project's things arranged from the side panel's menus (`src/views/outline.tsx`,
// over `src/outline.ts`; kept by the daemon for everybody).
//
//   DAEMON=../target/debug/factory node scripts/arranged-tree.mjs [out-dir]
//
// Shop has a board and three tables, one of them, its runs, declared as
// called `Test runs`. Nobody has arranged it. On Ada's page, from Shop's
// row menu, a folder is made (New… and then Folder…, a small dialog that asks its
// name and refuses one a folder beside it has before anything is asked),
// unfolded as it is made, and two tables are moved into it from each
// one's menu (Move to…, a menu beside it with the top and each folder,
// where each is marked), all drawn so on a second page of Ada's that is
// never read again. A folder made in that folder from its own menu, and
// called something else (Rename…), and moved to the top; a folder removed
// (what was in it taking its place) and given back by Undo. The project
// called something else from its row (Rename…), a name another project has
// refused before anything is asked with the dialog staying, and the new
// name on the page never read again, its tab and its page's head; a table
// called something else from its line, and from its page's gear, the side
// panel, the tab and the page's head saying it on both pages. The board
// shown as Roadmap from its line (Rename…, Escape leaving it as it was):
// in the side panel, the page's head with what it is faintly beside it,
// and the tab, on the page never read again and once that is read again,
// and the board again once the name is left empty. The
// arrangement forgotten behind a question, the panel as it was before. The
// menus and dialogs by their boxes at three widths, dragged narrow and on
// a phone, where the lines are offered by touch; somebody who does not
// hold `projects:write` (a page told so) offered none of it, and a key that
// only reads projects refused by every route. The environment variables
// are those of walkthrough.mjs; `LOOK` is worn as every check wears it.
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
import { openGear } from './gear.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'arranged-tree-data');
const home = join(out, 'arranged-tree-home');
const tables = join(out, 'arranged-tree-tables.bp');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
await writeFile(
  tables,
  `blueprint: shop-tables
permissions: [projects]
steps:
${['test-runs', 'deployments', 'notes']
  .map((name) => `  ${name.replace('-', '_')}:\n    do: collection.ensure\n    with:\n      name: ${name}\n${name === 'test-runs' ? '      title: Test runs\n' : ''}      fields: { title: { type: text, required: true } }\n`)
  .join('')}`,
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

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('project', 'create', '--bare', 'Depot');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('apply', tables, '--project', 'shop');

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

  /** Ada's page, signed in, with the Resources panel showing and everything of it unfolded; `lacking` is told it holds no `projects:write`. */
  const signedIn = async ({ how = { viewport: { width: 1280, height: 860 } }, lacking = false } = {}) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    if (lacking) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        // What a member holds, but writing projects: the daemon says who somebody is, and the page goes by it.
        me.principal.scopes = ['projects:read', 'channels', 'inbox', 'themes', 'arranged', 'clock', 'board', 'records', 'columns', 'blueprints:read'];
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
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
    // Expand all unfolds Shop, whatever it is called by then.
    await shop(page).locator(':scope > .tree').waitFor();
    return page;
  };
  const shop = (page) => resourcesOf(page).locator('[data-project="shop"]');
  /** Unfolds a folder of the outline by its line, where it is folded: its line is its arrow. */
  const unfoldFolder = async (page, name) => {
    const folder = shop(page).locator('[data-outline] > div > .nav-line > .nav-item', { hasText: name }).first();
    await unfold(page, folder);
  };
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
  const drawn = async (page, meant, what) => {
    const wanted = JSON.stringify(meant);
    const reached = await page
      .waitForFunction(`JSON.stringify((${LINES.toString()})()) === ${JSON.stringify(wanted)}`, undefined, { timeout: 15_000 })
      .then(() => true, () => false);
    if (!reached) expect(`${what}: ${wanted}, got ${JSON.stringify(await page.evaluate(LINES))}`, false);
  };
  const crowded = async (page, what, within = ['nav[aria-label="Resources"]']) => {
    const wrong = (await Promise.all(within.map((one) => pageCrowded(page, one)))).flat();
    expect(`${what}: nothing over anything else or past its sides, got ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`${what}: nothing to scroll sideways, got ${sideways}px`, sideways <= 0);
  };
  const rowMenu = (page) => page.locator('.row-menu:not(.row-beside)');
  const beside = (page) => page.locator('.row-menu.row-beside');
  /** Opens the menu at the end of a line of the panel, by the button called `label`. */
  const menuOf = async (page, label) => {
    const more = resourcesOf(page).getByRole('button', { name: label, exact: true });
    await more.waitFor();
    await press(page, more);
    await rowMenu(page).waitFor();
  };
  const line = (page, words) => rowMenu(page).getByRole('menuitem', { name: words, exact: true });
  const lines = async (page) => (await rowMenu(page).locator(':scope > button .truncate').allTextContents()).map((one) => one.trim());
  /** Moves something, from its line's menu, to a place offered beside Move to…. */
  const moveTo = async (page, label, place, shot) => {
    await menuOf(page, label);
    await press(page, line(page, 'Move to…'));
    await beside(page).waitFor();
    // Placed before it is drawn: waited for until it is.
    await page.waitForFunction(() => getComputedStyle(document.querySelector('.row-beside')).visibility !== 'hidden');
    const offered = (await beside(page).locator(':scope > button .truncate').allTextContents()).map((one) => one.trim());
    if (shot) {
      await crowded(page, `the menu beside Move to… ${shot}`, ['.row-menu:not(.row-beside)', '.row-menu.row-beside']);
      await page.screenshot({ path: `${out}/arranged-tree-${shot}.png` });
    }
    await press(page, beside(page).getByRole('menuitemradio', { name: place, exact: true }).or(beside(page).getByRole('menuitem', { name: place, exact: true })).first());
    await rowMenu(page).waitFor({ state: 'detached' });
    return offered;
  };
  /** New… and then Folder… beside it, in the menu that is open. */
  const newFolder = async (page) => {
    await press(page, line(page, 'New…'));
    await beside(page).waitFor();
    await page.waitForFunction(() => getComputedStyle(document.querySelector('.row-beside')).visibility !== 'hidden');
    await press(page, beside(page).getByRole('menuitem', { name: 'Folder…', exact: true }));
  };
  /** The small dialog that asks what something is called: typed in and saved by Enter. */
  const named = async (page, typed, shot) => {
    const dialog = page.locator('.modal');
    await dialog.waitFor();
    const field = dialog.locator('input');
    await field.fill(typed);
    if (shot) {
      await crowded(page, `the dialog ${shot}`, ['.modal']);
      await page.screenshot({ path: `${out}/arranged-tree-${shot}.png` });
    }
    await field.press('Enter');
  };

  // ── Nobody arranged it ──────────────────────────────────────────────────
  const page = await signedIn();
  const usual = ['Board', 'Deployments', 'Notes', 'Test runs', 'Blueprints'];
  await drawn(page, usual, 'Shop as nobody arranged it, the runs by what their blueprint calls them');
  const second = await signedIn();

  // ── A folder made from Shop's row, unfolded as it is made ─────────────────
  await menuOf(page, 'More for Shop');
  const offered = await lines(page);
  expect(`Shop's menu to offer a folder and a name, and nothing it is not arranged to forget, got ${offered.join(', ')}`, offered.includes('New…') && offered.includes('Rename…') && !offered.includes('Forget the arrangement…'));
  await crowded(page, "Shop's menu", ['.row-menu']);
  await newFolder(page);
  // A name a folder beside it has is refused before anything is asked; there is none yet, so a name that is nothing is.
  const dialog = page.locator('.modal');
  await dialog.waitFor();
  await dialog.locator('input').fill('   ');
  expect('Save to be closed for a name that is nothing', await dialog.getByRole('button', { name: 'Save' }).isDisabled());
  await named(page, 'Insights', 'a-new-folder');
  await dialog.waitFor({ state: 'detached' });
  await drawn(page, [...usual, 'Insights/'], 'Insights made at the end of what is drawn, and unfolded with nothing in it yet');
  await drawn(second, [...usual, 'Insights/'], 'Insights on the page never read again');

  // ── Two tables moved into it from their own lines ────────────────────────
  const places = await moveTo(page, 'Test runs: more', 'Insights', 'b-move-to');
  expect(`Move to… to offer the top and Insights, got ${places.join(', ')}`, JSON.stringify(places) === JSON.stringify(['Shop', 'Insights']));
  await moveTo(page, 'Deployments: more', 'Insights');
  await drawn(page, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments'], 'the two tables in Insights');
  await unfoldFolder(second, 'Insights');
  await drawn(second, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments'], 'the same on the page never read again');
  await page.screenshot({ path: `${out}/arranged-tree-c-moved.png` });

  // ── A folder in it, called something else and moved to the top ───────────
  await menuOf(page, 'Insights: more');
  const ofFolder = await lines(page);
  expect(`a folder's menu to offer a folder in it, a name, Move to… and Remove folder, got ${ofFolder.join(', ')}`, ['New…', 'Rename…', 'Move to…', 'Remove folder'].every((one) => ofFolder.includes(one)));
  await newFolder(page);
  await named(page, 'Runs');
  await page.locator('.modal').waitFor({ state: 'detached' });
  await drawn(page, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments', 'Insights/Runs/'], 'Runs made in Insights');
  await menuOf(page, 'Runs: more');
  await press(page, line(page, 'Rename…'));
  // Called what a folder beside it is called, it is refused before anything is asked.
  await page.locator('.modal input').fill('insights');
  await page.locator('.modal').getByRole('button', { name: 'Save' }).isDisabled();
  await named(page, 'Older runs', 'c-rename-folder');
  await page.locator('.modal').waitFor({ state: 'detached' });
  await drawn(second, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments', 'Insights/Older runs/'], 'the folder called something else on the page never read again');
  await moveTo(page, 'Older runs: more', 'Shop');
  await drawn(page, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments', 'Older runs/'], 'Older runs at the top');

  // ── A folder removed, what was in it where it was, and Undo ─────────────
  await menuOf(page, 'Insights: more');
  await press(page, line(page, 'Remove folder'));
  await drawn(page, ['Board', 'Notes', 'Blueprints', 'Test runs', 'Deployments', 'Older runs/'], 'what was in Insights where it was');
  await page.getByRole('button', { name: 'Undo' }).click();
  await drawn(page, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments', 'Older runs/'], 'Insights given back by Undo, unfolded as it was');
  // On the page never read again its fold went with it, and it is given back folded.
  await drawn(second, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Older runs/'], 'Insights given back on the page never read again');
  await unfoldFolder(second, 'Insights');
  await drawn(second, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments', 'Older runs/'], 'what was in it in it again');

  // ── The project called something else ────────────────────────────────────
  await menuOf(page, 'More for Shop');
  await press(page, line(page, 'Rename…'));
  await named(page, 'depot');
  const refused = page.locator('.modal [role="alert"]');
  // Refused before anything is asked, in the page's own words; Save is closed meanwhile.
  expect('Save closed for the name another project has', await page.locator('.modal').getByRole('button', { name: 'Save' }).isDisabled());
  await named(page, 'Shop front', 'd-rename-project');
  await page.locator('.modal').waitFor({ state: 'detached' });
  expect('nothing refused', (await refused.count()) === 0);
  await second.locator('nav[aria-label="Resources"] [data-project="shop"] > .nav-row .nav-item', { hasText: 'Shop front' }).waitFor();
  // Its page, opened, is headed by the new name, and so is its tab.
  await shop(second).locator(':scope > .nav-row .nav-item').click();
  await second.locator('.view-head h1', { hasText: 'Shop front' }).waitFor();

  // ── A table called something else, from its line and from its page ──────
  await menuOf(page, 'Notes: more');
  await press(page, line(page, 'Rename…'));
  await named(page, 'Field notes', 'e-rename-table');
  await page.locator('.modal').waitFor({ state: 'detached' });
  await drawn(second, ['Board', 'Field notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments', 'Older runs/'], 'the table called something else on the page never read again');
  await shop(second).locator('.nav-item', { hasText: 'Field notes' }).click();
  await second.locator('.view-head h1', { hasText: 'Field notes' }).waitFor();
  expect('its tab to say what it is called', (await second.locator('.tab', { hasText: 'Field notes' }).count()) > 0);
  // From its page's gear: called by its name again by leaving the name empty.
  await openGear(second);
  await crowded(second, "the table page's gear", ['.row-menu']);
  await second.locator('.row-menu').getByRole('menuitem', { name: 'Rename…', exact: true }).click();
  await named(second, '');
  await second.locator('.modal').waitFor({ state: 'detached' });
  await drawn(page, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments', 'Older runs/'], 'the table called by its name again from its page');
  await second.locator('.view-head h1', { hasText: /notes$/ }).waitFor();

  // ── The board shown as something else for everybody, kept, and as it was again ──
  const arranged = ['Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Insights/Deployments', 'Older runs/'];
  await menuOf(page, 'Board: more');
  const ofBoard = await lines(page);
  expect(`the board's menu to offer Rename… above Move to…, got ${ofBoard.join(', ')}`, ofBoard.includes('Rename…') && ofBoard.indexOf('Rename…') < ofBoard.indexOf('Move to…'));
  await press(page, line(page, 'Rename…'));
  // Escape leaves it as it was. The dialog listens for Escape a moment after it is drawn.
  await page.locator('.modal input').fill('Plans');
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  await page.keyboard.press('Escape');
  await page.locator('.modal').waitFor({ state: 'detached' });
  await drawn(page, ['Board', ...arranged], 'the board as it was when its rename is left by Escape');
  await menuOf(page, 'Board: more');
  await press(page, line(page, 'Rename…'));
  await named(page, 'Roadmap', 'e2-rename-board');
  await page.locator('.modal').waitFor({ state: 'detached' });
  await drawn(page, ['Roadmap', ...arranged], 'the board shown as Roadmap');
  await drawn(second, ['Roadmap', ...arranged], 'the board shown as Roadmap on the page never read again');
  await shop(second).locator('.nav-item', { hasText: 'Roadmap' }).click();
  const boardHead = async (on, what) => {
    await on.locator('.view-head h1', { hasText: /Roadmap$/ }).waitFor();
    await on.locator('.view-head .thing-was', { hasText: 'Board · table board' }).waitFor();
    expect(`${what}: its tab to say Roadmap`, (await on.locator('.tab', { hasText: 'Roadmap' }).count()) > 0);
    await crowded(on, `${what}: the board's head`, ['.view-head']);
  };
  await boardHead(second, 'the board page');
  await second.screenshot({ path: `${out}/arranged-tree-e3-board-named.png` });
  // Read again from the daemon, it is kept.
  await second.reload();
  await boardHead(second, 'the board page read again');
  await second.locator('nav[aria-label="Resources"] [data-project="shop"] .nav-item', { hasText: 'Roadmap' }).first().waitFor();
  // On a phone the head still says it, nothing over anything else.
  const narrow = await signedIn({ how: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 } });
  await narrow.locator('nav[aria-label="Resources"] [data-project="shop"] .nav-item', { hasText: 'Roadmap' }).first().tap();
  // The panel lies over the page on a phone: put away, the head is seen.
  if ((await narrow.getByRole('button', { name: 'Collapse the side panel' }).count()) > 0) await narrow.getByRole('button', { name: 'Collapse the side panel' }).first().tap();
  await sideSettled(narrow);
  await boardHead(narrow, "the board's page on a phone");
  await narrow.screenshot({ path: `${out}/arranged-tree-e4-board-named-phone.png` });
  await narrow.context().close();
  // Left empty, it is the board again, everywhere.
  await menuOf(page, 'Roadmap: more');
  await press(page, line(page, 'Rename…'));
  await named(page, '');
  await page.locator('.modal').waitFor({ state: 'detached' });
  await drawn(page, ['Board', ...arranged], 'the board as it always was once its name is left empty');
  await second.locator('.view-head h1', { hasText: /board$/ }).waitFor();
  expect('nothing faint beside the head once it is the board again', (await second.locator('.view-head .thing-was').count()) === 0);
  expect('its tab to say board again', (await second.locator('.tab', { hasText: 'Roadmap' }).count()) === 0);

  // ── By the boxes: three widths, dragged narrow, a phone ─────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await crowded(page, `the side panel at ${width} wide`);
    await moveTo(page, 'Test runs: more', 'Shop front', `f-${width}`);
    await moveTo(page, 'Test runs: more', 'Insights');
  }
  // Dragged as narrow as it goes.
  await page.setViewportSize({ width: 1280, height: 860 });
  const grip = await page.locator('.side-edge').boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + 300);
  await page.mouse.down();
  await page.mouse.move(60, grip.y + 300, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 180);
  await page.mouse.move(900, 700);
  await crowded(page, 'the side panel dragged as narrow as it goes');
  await moveTo(page, 'Test runs: more', 'Shop front', 'g-narrow');
  await moveTo(page, 'Test runs: more', 'Insights');
  const phone = await signedIn({ how: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 } });
  await unfoldFolder(phone, 'Insights');
  await crowded(phone, "the side panel at a phone's width");
  await moveTo(phone, 'Deployments: more', 'Shop front', 'h-phone');
  await drawn(phone, ['Board', 'Notes', 'Blueprints', 'Insights/', 'Insights/Test runs', 'Older runs/', 'Deployments'], 'Deployments moved to the end of the top by touch');

  // ── Forgotten behind a question ────────────────────────────────────────────
  await menuOf(page, 'More for Shop front');
  await press(page, line(page, 'Forget the arrangement…'));
  const question = page.locator('.modal');
  await question.waitFor();
  await crowded(page, 'the question before the arrangement is forgotten', ['.modal']);
  await page.screenshot({ path: `${out}/arranged-tree-i-forget.png` });
  await question.getByRole('button', { name: 'Forget it' }).click();
  await question.waitFor({ state: 'detached' });
  await drawn(second, ['Board', 'Deployments', 'Notes', 'Test runs', 'Blueprints'], 'Shop as it was before anybody arranged it');

  // ── Somebody who may not: offered none of it, and refused by the routes ───
  const lacking = await signedIn({ lacking: true });
  await menuOf(lacking, 'More for Shop front');
  const theirs = await lines(lacking);
  expect(`nothing that arranges or renames offered to somebody who does not write projects, got ${theirs.join(', ')}`, !['New…', 'Rename…', 'Forget the arrangement…'].some((one) => theirs.includes(one)));
  expect('no menu on a table line for them', (await resourcesOf(lacking).getByRole('button', { name: 'Notes: more', exact: true }).count()) === 0);
  const key = JSON.parse((await cli('key', 'reader', '--scope', 'projects:read', '--json')).stdout).token;
  const asked = (method, path, body) =>
    fetch(`${url}/api${path}`, { method, headers: { authorization: `Bearer ${key}`, 'content-type': 'application/json' }, body: JSON.stringify(body) }).then((one) => one.status);
  const statuses = [
    await asked('POST', '/projects/shop/outline', { op: 'make', name: 'Mine' }),
    await asked('PATCH', '/projects/shop', { name: 'Mine' }),
    await asked('PUT', '/collections/shop/notes/title', { title: 'Mine' }),
    await asked('POST', '/projects/shop/outline', { op: 'name', thing: 'board', name: 'Mine' }),
  ];
  expect(`every route to refuse a key that only reads projects, got ${statuses.join(', ')}`, statuses.every((one) => one === 403));

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
  await rm(tables, { force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
