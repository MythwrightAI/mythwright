// Forking a blueprint that ships, from its page, and what the Summary of
// the default and of the fork then say.
//
//   DAEMON=../target/debug/factory node scripts/forked.mjs [out-dir]
//
// It goes by what the page shows and by the boxes:
//
//   - a blueprint that ships says last how what it does is changed, in
//     the order to reach for them: what it may be told, pressed to apply
//     it; the rows of a table it makes whose rows set what it does,
//     pressed to open the table, and of the board's own table, whose rows
//     are its cards, nothing; and a fork;
//   - Fork, in its head, makes the fork for the project the page is of,
//     and the fork is open in the editor as a tab, a file the factory
//     keeps in its own data folder and never in the project's; from a page
//     that is of no project it asks which, and a project that keeps no
//     blueprints of its own is refused there in the daemon's own sentence,
//     with the dialog staying;
//   - the fork's Summary says first what it was forked from, by whom and
//     when, and that it is the default as it was forked; every line the
//     daemon's own summary of it has, and no other;
//   - a step changed and saved, it says where it differs, and that line
//     pressed shows the place in the fork's text;
//   - the default's Summary says first that the project runs its own
//     fork, pressed to open it, and offers no second Fork;
//   - the program bringing another version of the default, the fork says
//     that the default has moved since, by version first, and that it
//     takes none of it by itself;
//   - going back is behind a question that says what goes, what then runs
//     and what is lost; left, nothing is taken away; answered, the fork's
//     tab is closed, the default is shown in its place and says nothing of
//     a fork;
//   - somebody who may not write a project's files is offered no Fork, and
//     the lines that would fork or go back are words;
//   - nothing of the Summary or of either dialog lies over anything else,
//     is outside its place or scrolls sideways, at three widths and a
//     phone's.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { heldToTheDaemon } from './summarised.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'forked-data');
const home = join(out, 'forked-home');
// The shop's folder is one that was said, so that the fork can be looked at on the disk.
const shop = join(out, 'forked-shop');
// A folder that was there before it was taken up as a project: its files do not stand in for what ships.
const yard = join(out, 'forked-yard');
// A package whose blueprint ships, so that the program can be made to bring another version of it.
const reach = join(out, 'forked-reach');
const packaged = (version, more) =>
  `extension: reach\ntitle: Reach\nblueprints:\n  round: |\n    blueprint: reach/round\n    version: ${version}\n    description: Works a card.\n    params:\n      tone: { type: string, default: plain }\n    steps:\n      made: { do: noop }\n${more}`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop, yard, reach]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, yard, reach]) await mkdir(dir, { recursive: true });
await writeFile(join(yard, 'README.md'), '# A folder that was already there\n');
await writeFile(join(reach, 'extension.yaml'), packaged(1, ''));
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
// Where the factory keeps a project's own blueprints: in its data folder, and never the project's.
const placed = (project, path) => join(data, 'blueprints', 'projects', project, path.replace(/^@blueprints\//, ''));
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
let page = null;
let small = null;
let less = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  await cli('project', 'create', '--bare', 'Yard', '--path', yard);
  await cli('project', 'blueprints', 'yard', '--none');
  await cli('extension', 'add', reach);
  // The shop has its board, and who is told what waits, so that the table that says whom, which sits
  // beside the board, is there to be opened.
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('apply', 'std/told', '--project', 'shop', '--param', 'project=shop');
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
  /** A page that is signed in, at an address of the client's where one is said. `holds` is what whoever is signed in is said to hold, for a page that is to draw as for somebody who holds less. */
  const signIn = async (context, { at = '', touch = false, holds = null } = {}) => {
    const opened = await context.newPage();
    opened.setDefaultTimeout(20_000);
    opened.on('pageerror', (err) => problems.push(err.message));
    if (holds) {
      await opened.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = holds;
        await route.fulfill({ response: answer, json: me });
      });
    }
    await opened.goto(`${url}/${at}`);
    await opened.getByLabel('Email').fill('ada@example.com');
    await opened.getByLabel('Password').fill('correct horse');
    const press = opened.getByRole('button', { name: 'Sign in', exact: true }).last();
    await (touch ? press.tap() : press.click());
    return opened;
  };
  page = await signIn(await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme }));
  await page.locator('.rail').waitFor();

  /** Asks the daemon something as the page does. */
  const ask = (path, body, on = page) =>
    on.evaluate(async ([to, sent]) => (await fetch(to, { method: 'POST', headers: { 'content-type': 'application/json', 'x-page-key': localStorage.getItem('page-key') ?? '' }, body: JSON.stringify(sent) })).json(), [path, body]);
  const ending = await page.evaluate(async () => (await (await fetch('/api/meta')).json()).blueprint_ext);
  const drawn = (on = page) => on.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const shot = (name, on = page) => on.screenshot({ path: `${out}/${name}.png` });
  /** The Summary that is showing, of the blueprint called so. */
  const summary = (name, on = page) => on.locator(`.summary[data-of="${name}"]`).locator('visible=true');
  const pressed = async (on = page) => (await on.locator('.view-head .pills button[aria-pressed="true"]').locator('visible=true').last().innerText()).trim();
  /** Goes to an address of the client's, as a link to a thing does: the thing is shown beside what was open. */
  const goes = (to, on = page) => on.evaluate((hash) => (location.hash = hash), to);
  /** What the Summary on the page says: each part's heading with its lines, as they read. */
  const says = (on = page) =>
    on.evaluate(() => {
      const shown = [...document.querySelectorAll('.summary')].find((one) => one.getClientRects().length > 0);
      return [...shown.querySelectorAll('.summary-part')].map((part) => ({
        kind: part.dataset.part,
        title: part.querySelector('h2').textContent.trim(),
        lines: [...part.querySelectorAll('.summary-line')].map((line) => ({
          // A way to change it is drawn as what is done and what is said of it, which its words are with a colon between.
          text: line.querySelector(':scope > .summary-said > .summary-act')
            ? `${line.querySelector(':scope > .summary-said > .summary-act').textContent}: ${line.querySelector(':scope > .summary-detail').textContent}`
            : line.querySelector(':scope > .summary-said > .summary-words')?.textContent ?? '',
          at: line.dataset.at ?? null,
          opens: line.querySelector(':scope > .summary-said > button.summary-goes')?.dataset.opens ?? null,
          pressed: line.querySelector(':scope > .summary-said > button.summary-words') !== null,
        })),
      }));
    });
  const part = (all, kind) => all.find((one) => one.kind === kind)?.lines ?? [];
  /** Holds what the page draws to what the daemon's own summary says, both ways (`summarised.mjs`), once every fold is open; gives the daemon's parts. */
  const asTheDaemonSays = async (asked, when, on = page) => {
    // One at a time from the first: a fold opened is no longer one that is folded.
    const shut = on.locator('.summary-fold[aria-expanded="false"]').locator('visible=true');
    for (let left = await shut.count(); left > 0; left = await shut.count()) {
      await (on === small ? shut.first().tap() : shut.first().click());
      await on.waitForFunction((count) => [...document.querySelectorAll('.summary-fold[aria-expanded="false"]')].filter((one) => one.getClientRects().length > 0).length < count, left);
    }
    const handed = (await ask('/api/blueprints/summary', asked, on)).summary;
    return heldToTheDaemon(on, handed, when, expect);
  };
  /** The boxes of the Summary that is showing: its headings and lines, and what each line is made of. */
  const looked = (on = page) =>
    on.evaluate(() => {
      const shown = [...document.querySelectorAll('.summary')].find((one) => one.getClientRects().length > 0);
      const box = (el) => {
        const at = el.getBoundingClientRect();
        return { left: at.left, right: at.right, top: at.top, bottom: at.bottom, what: (el.textContent ?? '').trim().slice(0, 36) };
      };
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const column = box(shown.querySelector('.summary-page'));
      const within = box(shown);
      const saids = [...shown.querySelectorAll('.summary-said')];
      const more = [...shown.querySelectorAll('.summary-only > li, .summary-tags, .summary-fact, .summary-by, .summary-detail, button.summary-unfolds, .summary-from')];
      const rows = [...shown.querySelectorAll('.summary-part > h2:not(.summary-hidden)'), ...saids, ...more].map(box);
      const crossed = [];
      rows.forEach((a, at) => rows.slice(at + 1).forEach((b) => over(a, b) && crossed.push(`"${a.what}" over "${b.what}"`)));
      const inside = [];
      for (const said of saids) {
        const [own, parts] = [box(said), [...said.children].map(box)];
        parts.forEach((a, at) => {
          if (a.left < own.left - 0.5 || a.right > own.right + 0.5) inside.push(`"${a.what}" is outside its line`);
          parts.slice(at + 1).forEach((b) => over(a, b) && inside.push(`"${a.what}" over "${b.what}"`));
        });
      }
      return {
        rows: rows.length,
        crossed,
        inside,
        outside: rows.filter((one) => one.left < column.left - 0.5 || one.right > column.right + 0.5).map((one) => one.what),
        beyond: column.left < within.left - 0.5 || column.right > within.right + 0.5,
        sideways: document.documentElement.scrollWidth - window.innerWidth,
        inner: shown.scrollWidth - shown.clientWidth,
      };
    });
  const laidOut = async (when, on = page) => {
    await drawn(on);
    const now = await looked(on);
    expect(`the Summary to have lines ${when}`, now.rows > 4);
    expect(`no line or heading of the Summary to lie over another ${when}, got ${now.crossed.slice(0, 4).join('; ')}`, now.crossed.length === 0);
    expect(`nothing a line says to lie over anything else it says, or outside it, ${when}, got ${now.inside.slice(0, 4).join('; ')}`, now.inside.length === 0);
    expect(`every line to be inside the column ${when}, got ${now.outside.slice(0, 4).join('; ')}`, now.outside.length === 0 && !now.beyond);
    expect(`nothing to scroll sideways ${when}, got the page ${now.sideways} and the Summary ${now.inner}`, now.sideways <= 0 && now.inner <= 0);
  };
  /** The boxes of a dialog: inside the window, with nothing of it over anything else of it or outside it. */
  const dialogLaidOut = async (when, on = page) => {
    await drawn(on);
    const now = await on.evaluate(() => {
      const modal = document.querySelector('.modal');
      const box = (el) => {
        const at = el.getBoundingClientRect();
        return { left: at.left, right: at.right, top: at.top, bottom: at.bottom, what: (el.textContent ?? '').trim().slice(0, 30) };
      };
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const [own, parts] = [box(modal), [...modal.querySelector('.forking').children].map(box)];
      const crossed = [];
      parts.forEach((a, at) => parts.slice(at + 1).forEach((b) => over(a, b) && crossed.push(`"${a.what}" over "${b.what}"`)));
      return {
        crossed,
        outside: parts.filter((one) => one.left < own.left - 0.5 || one.right > own.right + 0.5).map((one) => one.what),
        window: own.left >= -0.5 && own.right <= window.innerWidth + 0.5 && own.top >= -0.5 && own.bottom <= window.innerHeight + 0.5,
        sideways: document.documentElement.scrollWidth - window.innerWidth,
      };
    });
    expect(`nothing of the dialog to lie over anything else of it ${when}, got ${now.crossed.join('; ')}`, now.crossed.length === 0);
    expect(`everything of the dialog to be inside it, and it inside the window, ${when}, got ${now.outside.join('; ')}`, now.outside.length === 0 && now.window && now.sideways <= 0);
  };
  /** Whether a line of the text that reads so is in sight: drawn, and inside what the text scrolls in. */
  const inSight = (words, on = page) =>
    on.waitForFunction((want) => {
      const view = [...document.querySelectorAll('.view')].find((one) => one.querySelector('.cm-scroller')?.getClientRects().length);
      const [scroller, line] = [view?.querySelector('.cm-scroller')?.getBoundingClientRect(), [...(view?.querySelectorAll('.cm-line') ?? [])].find((one) => one.textContent === want)?.getBoundingClientRect()];
      return !!scroller && !!line && line.top >= scroller.top - 0.5 && line.bottom <= scroller.bottom + 0.5;
    }, words);
  const head = (on = page) => on.locator('.view-head').locator('visible=true');
  const forkButton = (on = page) => head(on).getByRole('button', { name: 'Fork', exact: true });
  // What applies it says so by what applying would do: the shop's board is running.

  // ---- one that ships says how what it does is changed ----------------------------
  await goes('/blueprint/std/board?project=shop');
  await summary('std/board').waitFor();
  expect(`a blueprint that ships to open on its Summary; it shows ${await pressed()}`, (await pressed()) === 'Summary');
  const board = await asTheDaemonSays({ name: 'std/board', project: 'shop' }, 'for the board that ships');
  expect(`how it is changed to be its last part, got ${board.map((one) => one.kind).join(', ')}`, board.at(-1).kind === 'change' && board.at(-1).title === 'How to change what it does');
  const change = part(board, 'change');
  // Its own table's rows are its cards, which set nothing it does: no row is offered.
  expect(
    `what it may be told, then a fork, and no rows of its own table, got ${change.map((line) => `${line.opens}: ${line.text.slice(0, 28)}`).join(' | ')}`,
    change.length === 2 && change[0].opens === 'apply' && /^Tell it something else/.test(change[0].text) && change[1].opens === 'forking' && /^Fork it:/.test(change[1].text),
  );
  expect('nothing of what it is to say anything of a fork', part(board, 'is').every((line) => line.opens === null && !/fork/i.test(line.text)));
  expect('Fork to be in its head, beside what is there', (await forkButton().count()) === 1 && (await head().getByRole('button', { name: /^Apply( changes)?$/ }).count()) === 1);
  await summary('std/board').locator('.summary-part[data-part="change"]').scrollIntoViewIfNeeded();
  await laidOut('for the board that ships');
  await shot(`forked-1-ships-how-to-change-${scheme}`);
  // What it may be told: the page's own Apply, asked in the drawer beside it.
  await summary('std/board').locator('button.summary-goes[data-opens="apply"]').click();
  await page.locator('.drawer[data-rest]').getByText('Apply std/board').waitFor();
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  // The rows of a table whose rows set what it does: who is told what waits, beside the board, opened.
  await goes('/blueprint/std/told?project=shop');
  await summary('std/told').waitFor();
  const told = part(await asTheDaemonSays({ name: 'std/told', project: 'shop' }, 'for what tells of what waits'), 'change');
  expect(
    `what it may be told, then the rows of the table that says whom, then a fork, got ${told.map((line) => `${line.opens}: ${line.text.slice(0, 28)}`).join(' | ')}`,
    told.length === 4 && told[0].opens === 'apply' && /^Change the rows of the table it makes/.test(told[1].text) && told[2].opens === 'table' && /^tells: Who is told what/.test(told[2].text) && told[3].opens === 'forking',
  );
  await summary('std/told').locator('button.summary-goes[data-opens="table"]').click();
  await page.locator('.view-head h1', { hasText: 'Shop tells' }).waitFor();
  await page.locator('table.grid tr[data-row]').first().waitFor();
  await goes('/blueprint/std/board?project=shop');
  await summary('std/board').waitFor();

  // ---- forked from the page: the fork is open in the editor -------------------------
  const file = `@blueprints/std/board.${ending}`;
  await forkButton().click();
  await page.locator('.toast', { hasText: `Forked std/board version` }).waitFor();
  await head().getByText(`board.${ending}`, { exact: false }).first().waitFor();
  await page.locator('.cm-content').locator('visible=true').waitFor();
  expect(`the fork to be open in the editor; it shows ${await pressed()}`, (await pressed()) === 'Text');
  const ships = (await page.evaluate(async () => (await (await fetch('/api/blueprints/source?name=std/board', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).source));
  const onDisk = await readFile(placed('shop', file), 'utf8');
  expect('the fork on the disk to be the default, to the letter', onDisk === ships);
  // Where it is said leads to the project's blueprints in the side panel, which list it, and to no folder of its repository.
  const crumbs = page.locator('.crumbs').locator('visible=true');
  const crumbed = await crumbs.innerText();
  expect(`the fork said to be under the project's Blueprints, got ${crumbed}`, /Shop\s*\/\s*Blueprints\s*\/\s*std\s*\/\s*board\./.test(crumbed));
  expect('no folder of the repository called @blueprints to lead to', (await crumbs.getByRole('button', { name: '@blueprints' }).count()) === 0);
  await crumbs.getByRole('button', { name: 'Blueprints', exact: true }).click();
  const listed = page.locator('nav.sidebar[aria-label="Resources"] [data-project="shop"] .tree .nav-item', { hasText: 'std/board' });
  await listed.first().waitFor();
  expect('the list to be in view', await listed.first().isVisible());
  await shot(`forked-2-fork-open-in-the-editor-${scheme}`);

  // ---- the fork says what it was forked from ----------------------------------------
  await page.getByRole('tab', { name: 'Summary' }).locator('visible=true').last().click();
  await summary('std/board').waitFor();
  const fork = await asTheDaemonSays({ source: onDisk, project: 'shop', path: file }, 'for the fork');
  const version = /^version: (\S+)$/m.exec(ships)[1];
  const today = new Date().toISOString().slice(0, 10);
  const from = part(fork, 'is')[0];
  expect(`the fork to say first what it was forked from, by whom and when, got "${from.text}"`, from.opens === 'default' && (from.text === `Forked from std/board version ${version} by Ada on ${today}.` || /^Forked from std\/board version \S+ by Ada on \d{4}-\d\d-\d\d\.$/.test(from.text)));
  expect(`its parts in order, got ${fork.map((one) => one.kind).join(', ')}`, fork.map((one) => one.kind).join(',') === 'is,differs,makes,does,may,back');
  expect(`a fork nobody has changed to say that it is the default as it was forked, got ${part(fork, 'differs').map((line) => line.text).join(' | ')}`, part(fork, 'differs').length === 1 && part(fork, 'differs')[0].text === 'Nowhere: it is the default as it was forked.');
  expect('going back to be offered last, and to lose nothing', part(fork, 'back').length === 1 && part(fork, 'back')[0].opens === 'back' && /Nothing is lost by it/.test(part(fork, 'back')[0].text));
  expect('no Fork in the head of a fork', (await forkButton().count()) === 0);
  for (const width of [1280, 900, 600]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(`for a fork nobody has changed at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await shot(`forked-3-fork-says-what-it-was-forked-from-${scheme}`);

  // ---- a step changed and saved: where it differs, and the place ---------------------
  await page.getByRole('tab', { name: 'Text' }).locator('visible=true').last().click();
  const text = page.locator('.cm-content').locator('visible=true');
  await text.click();
  // Found as somebody finds it, and written in at the end of its line: the step that starts the cards' blueprint.
  await page.keyboard.press('Control+f');
  await page.locator('.cm-search, .finder').locator('visible=true').first().waitFor();
  await page.keyboard.type('spawn: std/cards');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Escape');
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await page.keyboard.type('allow_failure: true');
  await head().getByText('unsaved').waitFor();
  // Before it is saved, the Summary already says where what is being written differs.
  await page.getByRole('tab', { name: 'Summary' }).locator('visible=true').last().click();
  await summary('std/board').locator('.summary-part[data-part="differs"] .summary-words', { hasText: 'The step cards is changed.' }).waitFor();
  await page.getByRole('button', { name: 'Save', exact: true }).locator('visible=true').click();
  await head().getByText('unsaved').waitFor({ state: 'detached' });
  const saved = await readFile(placed('shop', file), 'utf8');
  expect('the fork to be saved as it was written', saved.includes('    allow_failure: true\n') && saved !== ships);
  const differs = await asTheDaemonSays({ source: saved, project: 'shop', path: file }, 'for the fork that was changed');
  const moved = part(differs, 'differs');
  expect(`the fork to say where it differs, by step, got ${JSON.stringify(moved)}`, moved.length === 1 && moved[0].text === 'The step cards is changed.' && moved[0].at === 'main/cards' && moved[0].pressed);
  expect('nothing to be said of the default having moved, which it has not', part(differs, 'moved').length === 0);
  expect(`going back to say what is lost, got "${part(differs, 'back')[0].text}"`, /What the fork's text says is then gone/.test(part(differs, 'back')[0].text));
  await laidOut('for the fork that was changed');
  await shot(`forked-4-fork-says-where-it-differs-${scheme}`);
  await summary('std/board').locator('.summary-part[data-part="differs"] .summary-line[data-at="main/cards"] button.summary-words').click();
  await page.locator('.cm-content').locator('visible=true').waitFor();
  expect(`the fork's text to be shown where it differs; it shows ${await pressed()}`, (await pressed()) === 'Text');
  await inSight('  cards:');
  await shot(`forked-5-the-place-in-the-forks-text-${scheme}`);

  // ---- the default says that the project runs its own fork ---------------------------
  await goes('/blueprint/std/board?project=shop');
  const runsOwn = summary('std/board').locator('.summary-part[data-part="is"] button.summary-goes[data-opens="fork"]');
  await runsOwn.waitFor();
  const theirs = await asTheDaemonSays({ source: ships, project: 'shop', ships: true }, 'for the default of a project that has a fork');
  expect(`the default to say first that this project runs its own fork, got "${part(theirs, 'is')[0].text}"`, part(theirs, 'is')[0].text === `This project runs its own fork of it, ${file}: that file, and not this text, is what runs here under the name.`);
  expect('no second fork to be offered: not in its head, and not as its last line', (await forkButton().count()) === 0 && part(theirs, 'change').at(-1).opens === 'fork' && part(theirs, 'change').every((line) => line.opens !== 'forking'));
  await laidOut('for the default of a project that has a fork');
  await shot(`forked-6-default-says-the-project-runs-its-own-${scheme}`);
  await runsOwn.click();
  await head().getByText(`board.${ending}`, { exact: false }).first().waitFor();

  // ---- going back, behind its question -----------------------------------------------
  await page.getByRole('tab', { name: 'Summary' }).locator('visible=true').last().click();
  await summary('std/board').locator('button.summary-goes[data-opens="back"]').click();
  const question = page.locator('.modal .forking[data-back="std/board"]');
  await question.getByRole('button', { name: 'Remove the fork' }).waitFor();
  const asked = (await question.locator('p').allInnerTexts()).join(' ');
  expect(`the question to say what goes and what then runs, got ${asked}`, asked.includes(`${file} is taken away, and std/board as it ships is what runs in Shop under the name again`));
  expect(`the question to say what is lost, got ${asked}`, asked.includes('what it says is then gone, unless the project keeps it somewhere else') && (await question.locator('.forking-lost li').allInnerTexts()).some((line) => line.includes('cards')));
  await dialogLaidOut('for the question before a fork is taken away');
  await shot(`forked-7-going-back-is-behind-a-question-${scheme}`);
  // A dialog starts listening for Escape a moment after it is drawn.
  await drawn();
  await question.getByRole('button', { name: 'Keep the fork' }).click();
  await question.waitFor({ state: 'detached' });
  expect('nothing to be taken away by a question that was left', (await readFile(placed('shop', file), 'utf8')) === saved);

  // ---- the program brings another version of a default that was forked ---------------
  await cli('own', 'reach/round', '--project', 'shop');
  const round = `@blueprints/reach/round.${ending}`;
  await goes(`/file/shop/${round}`);
  await head().getByText(`round.${ending}`, { exact: false }).first().waitFor();
  await page.getByRole('tab', { name: 'Summary' }).locator('visible=true').last().click();
  await summary('reach/round').locator('.summary-part[data-part="differs"]').waitFor();
  expect('nothing to be said of a default that has not moved', (await summary('reach/round').locator('.summary-part[data-part="moved"]').count()) === 0);
  await writeFile(join(reach, 'extension.yaml'), packaged(2, '      tidy: { do: noop }\n'));
  const roundText = await readFile(placed('shop', round), 'utf8');
  await cli('extension', 'add', reach);
  // The page is not read again: it hears that a package was added, and asks again where its text stands.
  await summary('reach/round').locator('.summary-part[data-part="moved"]').waitFor();
  const later = await says();
  const since = part(later, 'moved');
  expect(
    `the fork to say that the default has moved, by version first, got ${since.map((line) => line.text).join(' | ')}`,
    since.length === 3 && since[0].text === 'reach/round 1 to 2: what ships now is not what this was forked from.' && since[0].opens === 'default' && since[1].text === 'The step tidy is new.' && /^The fork takes none of it by itself\./.test(since[2].text),
  );
  expect(`the fork to say still that it was forked from version 1, got "${part(later, 'is')[0].text}"`, /^Forked from reach\/round version 1 by Ada on /.test(part(later, 'is')[0].text));
  expect('the fork to differ nowhere from what it was forked from still', part(later, 'differs')[0].text === 'Nowhere: it is the default as it was forked.');
  expect('the fork on the disk to be as it was', (await readFile(placed('shop', round), 'utf8')) === roundText);
  await laidOut('for a fork whose default has moved');
  await shot(`forked-8-the-default-has-moved-${scheme}`);

  // ---- forked from the factory's list: which project, and one that keeps none -------
  await goes('/blueprint/std/pass-by-hand');
  await summary('std/pass-by-hand').waitFor();
  await forkButton().click();
  const which = page.locator('.modal .forking[data-fork="std/pass-by-hand"]');
  await which.getByLabel('Project').selectOption('yard');
  await which.getByRole('button', { name: 'Fork it' }).click();
  const refused = which.locator('.forking-refused');
  await refused.waitFor();
  expect(`a project that keeps none to be refused in the daemon's own sentence, got "${await refused.innerText()}"`, /keeps no blueprints of its own, so it has no fork of std\/pass-by-hand and none is made/.test((await refused.innerText()).replace(/\s+/g, ' ')));
  expect('the dialog to stay, with the project that was chosen', (await which.getByLabel('Project').inputValue()) === 'yard');
  await dialogLaidOut('for a fork that was refused');
  await shot(`forked-9-refused-for-a-project-that-keeps-none-${scheme}`);
  await which.getByLabel('Project').selectOption('shop');
  await which.getByRole('button', { name: 'Fork it' }).click();
  await which.waitFor({ state: 'detached' });
  await head().getByText(`pass-by-hand.${ending}`, { exact: false }).first().waitFor();
  expect('the fork to be where the factory keeps the own blueprints of the project that was chosen', (await readFile(placed('shop', `std/pass-by-hand.${ending}`), 'utf8')).includes('blueprint: std/pass-by-hand'));
  expect('nothing of it in the project\'s folder', await readFile(join(shop, `blueprints/std/pass-by-hand.${ending}`), 'utf8').then(() => false, () => true));

  // ---- gone back: the fork's tab closed, the default in its place --------------------
  await goes(`/file/shop/${file}`);
  await head().getByText(`board.${ending}`, { exact: false }).first().waitFor();
  await page.getByRole('tab', { name: 'Summary' }).locator('visible=true').last().click();
  await summary('std/board').locator('button.summary-goes[data-opens="back"]').click();
  await question.getByRole('button', { name: 'Remove the fork' }).click();
  await question.waitFor({ state: 'detached' });
  await page.locator('.toast', { hasText: 'is taken away' }).waitFor();
  await summary('std/board').locator('.summary-part[data-part="change"] button.summary-goes[data-opens="forking"]').waitFor();
  const back = await says();
  expect('the default to say nothing of a fork once the project has gone back', part(back, 'is').every((line) => !/fork/i.test(line.text)) && (await forkButton().count()) === 1);
  expect('the file to be gone from the disk', await readFile(placed('shop', file), 'utf8').then(() => false, () => true));
  expect("the fork's tab to be closed", (await page.locator('.tab', { hasText: `board.${ending}` }).count()) === 0);
  await shot(`forked-10-gone-back-to-the-default-${scheme}`);

  // ---- somebody who may not write a project's files -----------------------------------
  const holds = ['projects:read', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];
  less = await signIn(await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme }), { at: '#/blueprint/std/board?project=shop', holds });
  await summary('std/board', less).waitFor();
  const offered = part(await says(less), 'change');
  expect('somebody who may not to be offered no Fork in the head', (await forkButton(less).count()) === 0 && (await head(less).getByRole('button', { name: /^Apply( changes)?$/ }).count()) === 1);
  expect(`the line that says a fork to be words for them, got ${JSON.stringify(offered.at(-1))}`, /^Fork it:/.test(offered.at(-1).text) && offered.at(-1).opens === null && !offered.at(-1).pressed);
  await goes(`/file/shop/${round}`, less);
  await summary('reach/round', less).locator('.summary-part[data-part="back"]').waitFor();
  const theirBack = part(await says(less), 'back');
  expect('going back to be words for them too', theirBack.length === 1 && theirBack[0].opens === null && !theirBack[0].pressed);
  await laidOut('for somebody who may not fork', less);

  // ---- a phone -------------------------------------------------------------------------
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  small = await signIn(phone, { at: `#/file/shop/${round}`, touch: true });
  // Whoever may change a file is shown its text first: its Summary is a touch away.
  await small.getByRole('tab', { name: 'Summary' }).locator('visible=true').last().tap();
  await summary('reach/round', small).waitFor();
  expect(`a fork to say what it was forked from at a phone's width, got "${part(await says(small), 'is')[0].text}"`, /^Forked from reach\/round version 1 by Ada on /.test(part(await says(small), 'is')[0].text));
  await laidOut("for a fork at a phone's width", small);
  await shot(`forked-11-phone-fork-${scheme}`, small);
  await summary('reach/round', small).locator('button.summary-goes[data-opens="back"]').scrollIntoViewIfNeeded();
  await summary('reach/round', small).locator('button.summary-goes[data-opens="back"]').tap();
  const smallQuestion = small.locator('.modal .forking[data-back="reach/round"]');
  await smallQuestion.getByRole('button', { name: 'Keep the fork' }).waitFor();
  await smallQuestion.locator('p').nth(1).waitFor();
  await dialogLaidOut("for the question at a phone's width", small);
  await shot(`forked-12-phone-question-${scheme}`, small);
  await drawn(small);
  await smallQuestion.getByRole('button', { name: 'Keep the fork' }).tap();
  await smallQuestion.waitFor({ state: 'detached' });
  await small.evaluate((hash) => (location.hash = hash), '/blueprint/std/board?project=shop');
  await summary('std/board', small).waitFor();
  await summary('std/board', small).locator('.summary-part[data-part="change"]').scrollIntoViewIfNeeded();
  await laidOut("for one that ships at a phone's width", small);
  await shot(`forked-13-phone-how-to-change-${scheme}`, small);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/forked-0-failed.png` }).catch(() => {});
  await less?.screenshot({ path: `${out}/forked-0-failed-second-page.png` }).catch(() => {});
  await small?.screenshot({ path: `${out}/forked-0-failed-phone.png` }).catch(() => {});
} finally {
  // The board that was applied is stopped before the daemon is: it leaves nothing running.
  // By its id: `stop` takes an instance, and was never told a project.
  const boards = await cli('instances', '--project', 'shop', '--json').then((ran) => JSON.parse(ran.stdout).instances, () => []);
  for (const board of boards.filter((one) => one.blueprint === 'std/board')) await cli('stop', board.id).catch(() => {});
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop, yard, reach]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
