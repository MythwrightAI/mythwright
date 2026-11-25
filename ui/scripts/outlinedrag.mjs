// A project's things carried to another place in its tree (`src/views/outlinedrag.tsx`,
// over `landing` and `stepped` in `src/outline.ts`; kept by the daemon for everybody).
//
//   DAEMON=../target/debug/factory node scripts/outlinedrag.mjs [out-dir]
//
// Shop has a board and three tables, arranged from a terminal: Insights
// with its runs in it, Archive with nothing, then the board, two tables and
// its blueprints. On Ada's page: a line held and moved, nothing of the
// panel moved as the carry begins (by the boxes of every line), drawn
// fainter where it stands; what says where it lands lying over the page, a
// line between two lines at the line it would go before, and a folder's
// line outlined, its middle; Escape putting it back and nothing asked of
// the daemon; let go where it began, nothing asked; a table let go on
// Insights put at its end, and the press that carried it opening nothing;
// another let go on Archive folded, in it with Archive still folded; the
// board let go over the upper half of Insights put first; the last of
// Insights carried out of it by the pointer further left; each seen on a
// second page that is never read again. The keyboard: Alt and the arrows on
// a line moving it down and up its level, into the folder above it and out
// again, the line keeping the keyboard and the page not going back. A plain
// press of a line still opening it. At 1280, 1000 and 800 wide: nothing
// moved as a carry begins, the line inside the panel, and nothing over
// anything else. Somebody who does not hold `projects:write` (a page told
// so) carries nothing. The environment variables are those of
// walkthrough.mjs; `LOOK` is worn as every check wears it.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { resourcesOf, unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'outlinedrag-data');
const home = join(out, 'outlinedrag-home');
const tables = join(out, 'outlinedrag-tables.bp');

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
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('apply', tables, '--project', 'shop');
  const outline = (...args) => cli('project', 'outline', 'shop', ...args);
  await outline('--folder', 'Insights', '--put', 'table:test-runs', '--in', 'Insights');
  await outline('--folder', 'Archive');
  await outline('--put', 'board', 'table:deployments', 'table:notes', 'blueprints');

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

  /** Ada's page, signed in, with the Resources panel showing and everything of it unfolded; `lacking` is told it holds no `projects:write`. */
  const signedIn = async ({ lacking = false } = {}) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    // What is asked of the daemon to change the outline, counted: a carry that lands nowhere asks nothing.
    page.asked = 0;
    page.on('request', (request) => {
      if (request.method() === 'POST' && request.url().endsWith('/outline')) page.asked += 1;
    });
    if (lacking) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = ['projects:read', 'channels', 'inbox', 'themes', 'arranged', 'clock', 'board', 'records', 'columns', 'blueprints:read'];
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await unfoldAll(page);
    const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
    await resourcesOf(page).waitFor();
    await shop(page).locator(':scope > .tree').waitFor();
    return page;
  };
  const shop = (page) => resourcesOf(page).locator('[data-project="shop"]');
  /** The line of something drawn under Shop, by what it says: a folder's or a thing's own line, never one of what is under it. */
  const lineOf = (page, words) =>
    shop(page)
      .locator('.nav-item')
      .filter({ has: page.locator(':scope > .truncate', { hasText: new RegExp(`^${words}$`) }) })
      .first();
  /** What is drawn under Shop, line by line, each said with the folders it is in (`Insights/Notes`); read in the page. */
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
  /** The box of every line of the panel and of the rest of the page beside it, to the pixel: what a carry may not move. */
  const boxes = (page) =>
    page.evaluate(() =>
      [...document.querySelectorAll('nav[aria-label="Resources"] :is(.nav-item, .nav-row, .nav-line, .nav-twist, h3), .workbench')].map((one) => {
        const box = one.getBoundingClientRect();
        return `${Math.round(box.left * 2)},${Math.round(box.top * 2)},${Math.round(box.width * 2)},${Math.round(box.height * 2)}`;
      }),
    );
  /** What says where a carry would land, as it is drawn now: nothing, a line, or a folder outlined. */
  const dropOf = (page) =>
    page.evaluate(() => {
      const drop = document.querySelector('.outline-drop');
      if (!drop || drop.style.display === 'none') return null;
      const box = drop.getBoundingClientRect();
      return { into: drop.classList.contains('into'), top: box.top, left: box.left, right: box.right, height: box.height, fixed: getComputedStyle(drop).position === 'fixed' };
    });
  const middleOf = async (locator) => {
    const box = await locator.boundingBox();
    return { x: box.x + box.width / 2, y: box.y + box.height / 2, box };
  };
  /** Takes hold of a line and moves it past where a press is a press, checking that nothing moved as the carry began. */
  const takeUp = async (page, words, what) => {
    const at = await middleOf(lineOf(page, words));
    const before = await boxes(page);
    await page.mouse.move(at.x, at.y);
    await page.mouse.down();
    await page.mouse.move(at.x, at.y + 3, { steps: 2 });
    expect(`${what}: no carry under a few pixels`, (await page.locator('.outline-carried').count()) === 0);
    await page.mouse.move(at.x + 2, at.y + 9, { steps: 3 });
    await page.locator('.outline-carried').waitFor();
    const after = await boxes(page);
    const moved = after.filter((one, n) => one !== before[n]);
    expect(`${what}: nothing of the panel or the page moved as the carry began, got ${moved.length} of ${after.length} boxes moved`, moved.length === 0 && after.length === before.length);
    return at;
  };
  /** Moves what is carried to a point and waits for what says where it lands to be drawn there. */
  const carryTo = async (page, x, y) => {
    await page.mouse.move(x, y, { steps: 4 });
    // Drawn once a frame: the next frame has it.
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    return dropOf(page);
  };
  const crowded = async (page, what) => {
    const wrong = await pageCrowded(page, 'nav[aria-label="Resources"]');
    expect(`${what}: nothing over anything else or past its sides, got ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`${what}: nothing to scroll sideways, got ${sideways}px`, sideways <= 0);
  };
  const tabs = (page) => page.locator('.tab').allTextContents();

  const page = await signedIn();
  const second = await signedIn();
  const start = ['Insights/', 'Insights/Test runs', 'Archive/', 'Board', 'Deployments', 'Notes', 'Blueprints'];
  await drawn(page, start, 'Shop as the terminal arranged it');
  await drawn(second, start, 'the same on the second page');

  // ── Taken up: nothing moves, the line fainter where it stands ─────────────
  await takeUp(page, 'Notes', 'Notes taken up');
  const faint = await lineOf(page, 'Notes').evaluate((one) => Number(getComputedStyle(one).opacity));
  expect(`Notes drawn fainter where it stands, got ${faint}`, faint < 1);
  // Over the middle of Insights's line: Insights outlined, over its own line.
  const insights = await middleOf(lineOf(page, 'Insights'));
  const into = await carryTo(page, insights.x, insights.y);
  expect(`Insights outlined over its line, got ${JSON.stringify(into)}`, into?.into && into.fixed && Math.abs(into.top - insights.box.y) < 1 && Math.abs(into.height - insights.box.height) < 1);
  await page.screenshot({ path: `${out}/outlinedrag-a-into.png` });
  // Over the upper half of Deployments: a line at its top, at its level.
  const deployments = await middleOf(lineOf(page, 'Deployments'));
  const between = await carryTo(page, deployments.x, deployments.box.y + 3);
  expect(`a line at the top of Deployments, got ${JSON.stringify(between)}`, between && !between.into && between.fixed && Math.abs(between.top + 1 - deployments.box.y) < 1);
  await page.screenshot({ path: `${out}/outlinedrag-b-between.png` });
  // Escape puts it back, and nobody is told.
  await page.keyboard.press('Escape');
  await page.locator('.outline-drop').waitFor({ state: 'detached' });
  expect('nothing carried once Escape is pressed', (await page.locator('.outline-carried').count()) === 0);
  await page.mouse.up();
  await drawn(page, start, 'nothing moved by Escape');
  expect(`nothing asked of the daemon by Escape, got ${page.asked}`, page.asked === 0);
  expect('no tab of Notes opened by the press Escape ended', !(await tabs(page)).some((one) => one.includes('Notes')));

  // ── Let go where it began: nobody told ─────────────────────────────────────
  const notes = await takeUp(page, 'Notes', 'Notes taken up again');
  const stays = await carryTo(page, notes.x, notes.y);
  expect(`nothing said over where it is, got ${JSON.stringify(stays)}`, stays === null);
  await page.mouse.up();
  await drawn(page, start, 'nothing moved let go where it began');
  expect(`nothing asked of the daemon let go where it began, got ${page.asked}`, page.asked === 0);
  expect('no tab of Notes opened by the press that carried it', !(await tabs(page)).some((one) => one.includes('Notes')));

  // ── Into Insights, at its end, and nothing opened by the press ─────────────
  await takeUp(page, 'Notes', 'Notes carried to Insights');
  await carryTo(page, insights.x, insights.y);
  await page.mouse.up();
  const inInsights = ['Insights/', 'Insights/Test runs', 'Insights/Notes', 'Archive/', 'Board', 'Deployments', 'Blueprints'];
  await drawn(page, inInsights, 'Notes at the end of Insights');
  await drawn(second, inInsights, 'Notes in Insights on the page never read again');
  expect(`one change asked of the daemon, got ${page.asked}`, page.asked === 1);
  expect('no tab of Notes opened by the press that carried it', !(await tabs(page)).some((one) => one.includes('Notes')));
  await page.screenshot({ path: `${out}/outlinedrag-c-in-insights.png` });

  // ── Into Archive folded, which stays folded ──────────────────────────────
  const archive = lineOf(page, 'Archive');
  await archive.click();
  await archive.and(page.locator('[aria-expanded="false"]')).waitFor();
  await takeUp(page, 'Deployments', 'Deployments carried to Archive');
  const folded = await middleOf(archive);
  const over = await carryTo(page, folded.x, folded.y);
  expect(`Archive outlined though folded, got ${JSON.stringify(over)}`, over?.into);
  await page.mouse.up();
  await drawn(page, ['Insights/', 'Insights/Test runs', 'Insights/Notes', 'Archive/', 'Board', 'Blueprints'], 'Deployments in Archive, drawn in nothing while Archive is folded');
  expect('Archive not unfolded under the pointer', (await archive.getAttribute('aria-expanded')) === 'false');
  await archive.click();
  await drawn(page, ['Insights/', 'Insights/Test runs', 'Insights/Notes', 'Archive/', 'Archive/Deployments', 'Board', 'Blueprints'], 'Deployments in Archive');

  // ── The board before Insights, by the upper half of its line ────────────
  await takeUp(page, 'Board', 'the board carried to the top');
  const top = await middleOf(lineOf(page, 'Insights'));
  await carryTo(page, top.x, top.box.y + 2);
  await page.mouse.up();
  const first = ['Board', 'Insights/', 'Insights/Test runs', 'Insights/Notes', 'Archive/', 'Archive/Deployments', 'Blueprints'];
  await drawn(page, first, 'the board first');
  await drawn(second, first, 'the board first on the page never read again');

  // ── Out of Insights, after it, by the pointer further left ──────────────
  await takeUp(page, 'Test runs', 'Test runs carried out of Insights');
  const last = await middleOf(lineOf(page, 'Notes'));
  const out_ = await carryTo(page, last.box.x - 10, last.box.y + last.box.height - 3);
  expect(`the line drawn at the top level, further left than Notes, got ${JSON.stringify(out_)}`, out_ && !out_.into && out_.left < last.box.x);
  await page.mouse.up();
  const outOf = ['Board', 'Insights/', 'Insights/Notes', 'Test runs', 'Archive/', 'Archive/Deployments', 'Blueprints'];
  await drawn(page, outOf, 'Test runs after Insights');
  await drawn(second, outOf, 'Test runs after Insights on the page never read again');

  // ── The keyboard: Alt and the arrows on a line ──────────────────────────
  const before = page.url();
  await lineOf(page, 'Test runs').focus();
  await page.keyboard.press('Alt+ArrowDown');
  await drawn(page, ['Board', 'Insights/', 'Insights/Notes', 'Archive/', 'Archive/Deployments', 'Test runs', 'Blueprints'], 'Test runs down a line');
  await page.keyboard.press('Alt+ArrowUp');
  await drawn(page, outOf, 'Test runs up a line again');
  // Into the folder above it, which is Insights, at its end.
  await page.keyboard.press('Alt+ArrowRight');
  await drawn(page, ['Board', 'Insights/', 'Insights/Notes', 'Insights/Test runs', 'Archive/', 'Archive/Deployments', 'Blueprints'], 'Test runs into Insights');
  await page.waitForFunction(() => document.activeElement?.closest('[data-place]')?.dataset.place === 'table:test-runs');
  await page.keyboard.press('Alt+ArrowLeft');
  await drawn(page, outOf, 'Test runs out of Insights again');
  await page.waitForFunction(() => document.activeElement?.closest('[data-place]')?.dataset.place === 'table:test-runs');
  expect('the page not gone back by Alt and the left arrow', page.url() === before);
  await drawn(second, outOf, 'the steps on the page never read again');

  // ── A plain press still opens the line ───────────────────────────────────
  await lineOf(page, 'Test runs').click();
  await page.locator('.view-head h1', { hasText: 'Test runs' }).waitFor();

  // ── By the boxes: three widths ───────────────────────────────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await page.mouse.move(width - 10, 800);
    await crowded(page, `the side panel at ${width} wide`);
    await takeUp(page, 'Test runs', `Test runs taken up at ${width} wide`);
    const board = await middleOf(lineOf(page, 'Board'));
    const line = await carryTo(page, board.x, board.box.y + 2);
    const panel = await resourcesOf(page).boundingBox();
    expect(`the line inside the panel at ${width} wide, got ${JSON.stringify(line)} in ${JSON.stringify(panel)}`, line && line.left >= panel.x && line.right <= panel.x + panel.width + 0.5);
    await page.screenshot({ path: `${out}/outlinedrag-d-${width}.png` });
    await page.keyboard.press('Escape');
    await page.mouse.up();
    await page.locator('.outline-drop').waitFor({ state: 'detached' });
  }
  await drawn(page, outOf, 'nothing moved by the carries put back');

  // ── Somebody who may not arrange carries nothing ─────────────────────────
  const lacking = await signedIn({ lacking: true });
  const at = await middleOf(lineOf(lacking, 'Board'));
  await lacking.mouse.move(at.x, at.y);
  await lacking.mouse.down();
  await lacking.mouse.move(at.x, at.y + 30, { steps: 5 });
  expect('nothing carried by somebody who does not write projects', (await lacking.locator('.outline-carried, .outline-drop').count()) === 0);
  await lacking.mouse.up();
  await lineOf(lacking, 'Board').focus();
  await lacking.keyboard.press('Alt+ArrowDown');
  expect(`nothing asked of the daemon for them, got ${lacking.asked}`, lacking.asked === 0);

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
