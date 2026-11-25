// A wiki's pages carried to another place in its tree:
// the Topics panel hands the general carry (`src/views/treedrag.tsx`, over
// `landing` and `steppedRows` in `src/treedrag.ts`) one move of the
// daemon's for each landing (`views/wikitree.tsx`), where the daemon would
// take it (`landing` in `src/wikis.ts`). Every page can hold pages, so the
// middle of any page's line is into it.
//
//   DAEMON=../target/debug/factory node scripts/wikidrag.mjs [out-dir]
//
// A wiki, Handbook, with three pages under it, one holding a page, and a
// second wiki, Decisions, made from a terminal. On Ada's page, with every
// page unfolded: a line held and moved, nothing of the panel moved as the
// carry begins (by the boxes of every line), drawn fainter where it stands;
// what says where it lands lying over the page, a page's line outlined over
// its middle and a line between two lines over the upper part of one;
// Escape putting it back and let go where it began, each asking nothing of
// the daemon. A page let go over the middle of a page that holds pages put
// at its end, and the press that carried it opening nothing; a wiki let go
// over the middle of a page that holds none put in it, the page then
// holding it and folded; a page put before another by the upper part of its
// line; the last of a page carried out of it by the pointer further left,
// and further left still to the top, where it is a wiki of its own; each
// seen on a second page that is never read again. The keyboard: Alt and
// the arrows on a line moving it down and up its level, into the page above
// it and out again, the line keeping the keyboard and the page not going
// back. A plain press of a line still opening it. At 1280, 1000 and 800
// wide: nothing moved as a carry begins, the line inside the panel, and
// nothing over anything else. Somebody who may not write records (a page
// told so) carries nothing. The environment variables are those of
// walkthrough.mjs; `LOOK` is worn as every check wears it.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'wikidrag-data');
const home = join(out, 'wikidrag-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const made = async (...args) => (await cli('topic', 'make', ...args)).stdout.match(/@-topics-\d+/)[0];

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  const handbook = await made('Handbook');
  await made('Getting started', '--under', handbook);
  const deploying = await made('Deploying', '--under', handbook);
  await made('Rolling back', '--under', deploying);
  await made('Notes', '--under', handbook);
  await made('Decisions');

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
  const panelOf = (page) => page.getByRole('navigation', { name: 'Topics', exact: true });

  /** Ada's page, signed in, with the Topics panel showing and every page unfolded; `lacking` is told it holds no `records:write`. */
  const signedIn = async ({ lacking = false } = {}) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    // What is asked of the daemon to move a page, counted: a carry that lands nowhere asks nothing.
    page.asked = 0;
    page.on('request', (request) => {
      if (request.method() === 'POST' && request.url().endsWith('/move')) page.asked += 1;
    });
    if (lacking)
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = ['records:read', 'channels', 'inbox', 'themes', 'arranged', 'clock', 'board:read', 'projects:read', 'blueprints:read'];
        await route.fulfill({ response: answer, json: me });
      });
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    const rail = page.locator('.rail').getByRole('button', { name: 'Topics', exact: true });
    await rail.waitFor();
    if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
    await panelOf(page).locator('.topic-line').first().waitFor();
    await panelOf(page).getByRole('button', { name: 'Expand all', exact: true }).click();
    return page;
  };
  /** A page's own line, by its title: what is pressed and held, never one of what is under it. */
  const lineOf = (page, words) =>
    panelOf(page)
      .locator('.topic-line .nav-item')
      .filter({ has: page.locator(':scope > .truncate', { hasText: new RegExp(`^${words}$`) }) })
      .first();
  /** What is drawn in the panel, page by page, each said with the pages it is under (`Handbook/Notes`); read in the page. */
  const LINES = () => {
    const root = document.querySelector('nav[aria-label="Topics"] .wikis');
    if (!root) return [];
    const said = [];
    const walk = (box, within) => {
      for (const one of box.children) {
        if (!one.matches('.topic-line')) continue;
        const name = one.querySelector(':scope > :is(.nav-row, .nav-line) > .nav-item .truncate').textContent.trim();
        said.push([...within, name].join('/'));
        const under = one.querySelector(':scope > .tree');
        if (under) walk(under, [...within, name]);
      }
    };
    walk(root, []);
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
      [...document.querySelectorAll('nav[aria-label="Topics"] :is(.nav-item, .nav-row, .nav-line, .nav-twist, .row-more, h3), .workbench')].map((one) => {
        const box = one.getBoundingClientRect();
        return `${Math.round(box.left * 2)},${Math.round(box.top * 2)},${Math.round(box.width * 2)},${Math.round(box.height * 2)}`;
      }),
    );
  /** What says where a carry would land, as it is drawn now: nothing, a line, or a page outlined. */
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
  /** The row a page's line is drawn in: what the carry measures, and what a landing is said beside. */
  const rowOf = async (page, words) => {
    const row = lineOf(page, words).locator('xpath=..');
    const box = await row.boundingBox();
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
    const wrong = await pageCrowded(page, 'nav[aria-label="Topics"]');
    expect(`${what}: nothing over anything else or past its sides, got ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`${what}: nothing to scroll sideways, got ${sideways}px`, sideways <= 0);
  };
  const tabs = (page) => page.locator('.tab').allTextContents();
  const placeOf = async (page, words) => lineOf(page, words).evaluate((one) => one.closest('[data-place]').dataset.place);

  const page = await signedIn();
  const second = await signedIn();
  const start = ['Handbook', 'Handbook/Getting started', 'Handbook/Deploying', 'Handbook/Deploying/Rolling back', 'Handbook/Notes', 'Decisions'];
  await drawn(page, start, 'the wikis as the terminal made them, every page unfolded');
  await drawn(second, start, 'the same on the second page');

  // ── Taken up: nothing moves, the line fainter where it stands ─────────────
  await takeUp(page, 'Notes', 'Notes taken up');
  const faint = await lineOf(page, 'Notes').evaluate((one) => Number(getComputedStyle(one).opacity));
  expect(`Notes drawn fainter where it stands, got ${faint}`, faint < 1);
  // Over the middle of Deploying's line: Deploying outlined, over its own line.
  const deployingRow = await rowOf(page, 'Deploying');
  const into = await carryTo(page, deployingRow.x, deployingRow.y);
  expect(`Deploying outlined over its line, got ${JSON.stringify(into)} for ${JSON.stringify(deployingRow.box)}`, into?.into && into.fixed && Math.abs(into.top - deployingRow.box.y) < 1 && Math.abs(into.height - deployingRow.box.height) < 1);
  await page.screenshot({ path: `${out}/wikidrag-a-into.png` });
  // Over the upper part of Getting started: a line at its top, at its level.
  const started = await rowOf(page, 'Getting started');
  const between = await carryTo(page, started.x, started.box.y + 2);
  expect(`a line at the top of Getting started, got ${JSON.stringify(between)}`, between && !between.into && between.fixed && Math.abs(between.top + 1 - started.box.y) < 1);
  await page.screenshot({ path: `${out}/wikidrag-b-between.png` });
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

  // ── Into Deploying, at its end, and nothing opened by the press ────────────
  await takeUp(page, 'Notes', 'Notes carried into Deploying');
  await carryTo(page, deployingRow.x, deployingRow.y);
  await page.mouse.up();
  const inDeploying = ['Handbook', 'Handbook/Getting started', 'Handbook/Deploying', 'Handbook/Deploying/Rolling back', 'Handbook/Deploying/Notes', 'Decisions'];
  await drawn(page, inDeploying, 'Notes at the end of Deploying');
  await drawn(second, inDeploying, 'Notes in Deploying on the page never read again');
  expect(`one move asked of the daemon, got ${page.asked}`, page.asked === 1);
  expect('no tab of Notes opened by the press that carried it', !(await tabs(page)).some((one) => one.includes('Notes')));
  await page.screenshot({ path: `${out}/wikidrag-c-into-deploying.png` });

  // ── A wiki into a page that holds none: it holds it, folded ────────────────
  await takeUp(page, 'Decisions', 'Decisions carried into Getting started');
  const leaf = await rowOf(page, 'Getting started');
  const onto = await carryTo(page, leaf.x, leaf.y);
  expect(`Getting started outlined though it holds no page, got ${JSON.stringify(onto)}`, onto?.into);
  await page.mouse.up();
  await drawn(page, ['Handbook', 'Handbook/Getting started', 'Handbook/Deploying', 'Handbook/Deploying/Rolling back', 'Handbook/Deploying/Notes'], 'Decisions in Getting started, which holds it and is folded');
  await panelOf(page).getByRole('button', { name: 'Unfold Getting started', exact: true }).click();
  const inStarted = ['Handbook', 'Handbook/Getting started', 'Handbook/Getting started/Decisions', 'Handbook/Deploying', 'Handbook/Deploying/Rolling back', 'Handbook/Deploying/Notes'];
  await drawn(page, inStarted, 'Decisions a page of Handbook now, under Getting started');

  // ── Before Getting started, by the upper part of its line ─────────────────
  await takeUp(page, 'Rolling back', 'Rolling back carried before Getting started');
  const topOf = await rowOf(page, 'Getting started');
  await carryTo(page, topOf.x, topOf.box.y + 2);
  await page.mouse.up();
  const first = ['Handbook', 'Handbook/Rolling back', 'Handbook/Getting started', 'Handbook/Getting started/Decisions', 'Handbook/Deploying', 'Handbook/Deploying/Notes'];
  await drawn(page, first, 'Rolling back first in Handbook');
  await drawn(second, first.filter((one) => one !== 'Handbook/Getting started/Decisions'), 'Rolling back first on the page never read again, Getting started folded there');

  // ── Out of Deploying by the pointer further left, and to the top ──────────
  // Let go over the last of a page's lower part with the pointer further left than it: after that page, a level out.
  await takeUp(page, 'Rolling back', 'Rolling back carried out of Deploying');
  const last = await rowOf(page, 'Notes');
  const outward = await carryTo(page, last.box.x - 10, last.box.y + last.box.height - 3);
  expect(`the line drawn at Handbook's level, further left than Notes, got ${JSON.stringify(outward)}`, outward && !outward.into && outward.left < last.box.x);
  await page.mouse.up();
  const outOf = ['Handbook', 'Handbook/Getting started', 'Handbook/Getting started/Decisions', 'Handbook/Deploying', 'Handbook/Deploying/Notes', 'Handbook/Rolling back'];
  await drawn(page, outOf, 'Rolling back after Deploying, at the end of Handbook');
  // Further left still, over the last of Handbook: the top, where it is a wiki of its own.
  await takeUp(page, 'Notes', 'Notes carried to the top');
  const lastAgain = await rowOf(page, 'Rolling back');
  const panelBox = await panelOf(page).boundingBox();
  const top = await carryTo(page, panelBox.x + 4, lastAgain.box.y + lastAgain.box.height - 3);
  expect(`the line drawn at the top, got ${JSON.stringify(top)}`, top && !top.into);
  await page.mouse.up();
  const atTop = ['Handbook', 'Handbook/Getting started', 'Handbook/Getting started/Decisions', 'Handbook/Deploying', 'Handbook/Rolling back', 'Notes'];
  await drawn(page, atTop, 'Notes a wiki of its own, at the top');
  await drawn(second, atTop.filter((one) => one !== 'Handbook/Getting started/Decisions'), 'Notes a wiki on the page never read again');
  await page.screenshot({ path: `${out}/wikidrag-d-to-the-top.png` });

  // ── The keyboard: Alt and the arrows on a line ──────────────────────────
  const before = page.url();
  const deployingId = await placeOf(page, 'Deploying');
  await lineOf(page, 'Deploying').focus();
  await page.keyboard.press('Alt+ArrowUp');
  await drawn(page, ['Handbook', 'Handbook/Deploying', 'Handbook/Getting started', 'Handbook/Getting started/Decisions', 'Handbook/Rolling back', 'Notes'], 'Deploying up a line');
  await page.keyboard.press('Alt+ArrowDown');
  await drawn(page, atTop, 'Deploying down a line again');
  // Into the page above it, which is Getting started, at its end.
  await page.keyboard.press('Alt+ArrowRight');
  await drawn(page, ['Handbook', 'Handbook/Getting started', 'Handbook/Getting started/Decisions', 'Handbook/Getting started/Deploying', 'Handbook/Rolling back', 'Notes'], 'Deploying into Getting started');
  await page.waitForFunction((id) => document.activeElement?.closest('[data-place]')?.dataset.place === id, deployingId);
  await page.keyboard.press('Alt+ArrowLeft');
  await drawn(page, atTop, 'Deploying out of Getting started again');
  await page.waitForFunction((id) => document.activeElement?.closest('[data-place]')?.dataset.place === id, deployingId);
  expect('the page not gone back by Alt and the left arrow', page.url() === before);
  await drawn(second, atTop.filter((one) => one !== 'Handbook/Getting started/Decisions'), 'the steps on the page never read again');

  // ── A plain press still opens the line ───────────────────────────────────
  await lineOf(page, 'Notes').click();
  await page.waitForFunction((said) => [...document.querySelectorAll('.topic-page-title')].some((one) => (one.value ?? one.innerText) === said), 'Notes');

  // ── By the boxes: three widths ───────────────────────────────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await page.mouse.move(width - 10, 800);
    await crowded(page, `the side panel at ${width} wide`);
    await takeUp(page, 'Notes', `Notes taken up at ${width} wide`);
    const row = await rowOf(page, 'Getting started');
    const line = await carryTo(page, row.x, row.box.y + 2);
    const panel = await panelOf(page).boundingBox();
    expect(`the line inside the panel at ${width} wide, got ${JSON.stringify(line)} in ${JSON.stringify(panel)}`, line && line.left >= panel.x && line.right <= panel.x + panel.width + 0.5);
    await page.screenshot({ path: `${out}/wikidrag-e-${width}.png` });
    await page.keyboard.press('Escape');
    await page.mouse.up();
    await page.locator('.outline-drop').waitFor({ state: 'detached' });
  }
  await drawn(page, atTop, 'nothing moved by the carries put back');

  // ── Somebody who may not write records carries nothing ───────────────────
  const lacking = await signedIn({ lacking: true });
  const at = await middleOf(lineOf(lacking, 'Notes'));
  await lacking.mouse.move(at.x, at.y);
  await lacking.mouse.down();
  await lacking.mouse.move(at.x, at.y - 60, { steps: 5 });
  expect('nothing carried by somebody who may not write records', (await lacking.locator('.outline-carried, .outline-drop').count()) === 0);
  await lacking.mouse.up();
  await lineOf(lacking, 'Deploying').focus();
  await lacking.keyboard.press('Alt+ArrowUp');
  expect(`nothing asked of the daemon for them, got ${lacking.asked}`, lacking.asked === 0);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
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
