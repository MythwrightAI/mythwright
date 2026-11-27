// A blueprint said in plain words: the Summary of a blueprint's page.
//
//   DAEMON=../target/debug/factory node scripts/summary.mjs [out-dir]
//
// It goes by what the page shows and by the boxes:
//
//   - a blueprint that ships opens on its Summary, with nothing pressed:
//     a head that says what it is, its facts (version, what it may do,
//     what it makes) each under a label, and then each part under its
//     heading in the order a reader needs them; every line, every part of
//     a line taken apart and every fact the daemon's own summary has is
//     drawn somewhere once its folds are open, and nothing it did not
//     hand (`summarised.mjs`), no line holding a hole or an action's
//     name; its last part says how what it does is changed, offering the
//     rows of a table only where they set what it does (the fleet's, and
//     not the board's own, whose rows are its cards; `forked.mjs` holds
//     what that part and a fork's own say and do);
//   - what a stage hears is a card for each thing heard, counted at its
//     heading, the event first, its steps folded away as it is opened;
//     where most of them hear from said once over them, and on the card
//     of one that hears from elsewhere; a fold opened by Enter, folded by
//     Space, the next reached by Tab, one opened by the pointer, a step
//     under it pressed to show where it is written; what was unfolded as
//     it was left when the Summary is shown again and after another tab
//     was shown, and none of it once the tab was closed;
//   - what it makes and starts is a row each, the same blueprint started
//     the same way from several handlers one row that names each, what
//     it is started as and whether it is waited for, its blueprint pressed
//     to open it, and a long list its first five rows until the rest is
//     asked for; each way to change it says what is done and what is
//     said of it;
//   - nothing of it lies over anything else of it, is outside its column
//     or scrolls sideways, at three widths and a phone's, for a short
//     blueprint, for the longest that ships and for a stage unfolded;
//   - a line that is about a step, and one about a handler, is pressed and
//     the text is shown where that is written; a line about the whole is
//     nothing to press;
//   - a project's own blueprint opens on its text for whoever may change
//     it, and on its Summary for somebody who may not; the line that says
//     a command is run is marked;
//   - the Summary follows the text as it is written, before it is saved;
//   - a text that does not check has none, and the page says what is wrong
//     with it in the check's own words, with the way to them;
//   - what its runs have cost is said under it once it has run, as counts,
//     and not before;
//   - every view the head offers (Summary, Text, Node, Runs, History,
//     Sandbox, Side by side) is wholly in the window, cut off by nothing
//     and over no other, at a phone's two widths and three wider ones
//     (the last again with the side panel beside it),
//     with every button of the head in the window too, and each pressed
//     shows its view; and Testing's menu with its four lines
//     in their order, over the page, in the window, no line over another
//     or cut short and nothing of the head moved by it, at each of them.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';
import { bare, heldToTheDaemon, onPage } from './summarised.mjs';
import { closeTesting, openTesting, pressTesting } from './testing.mjs';
import { dataModel, fleetMade, madeLaid, TABLES } from './madeof.mjs';

/** Turns the side panel to a view on the rail, unless it is showing already: Resources with what it has read unfolded, as everything there starts folded (`folded.mjs`). */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  if (name === 'Resources') await unfoldAll(page);
}

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'summary-data');
const home = join(out, 'summary-home');
// The project's folder is one that was said, so that its Repo is listed under it.
const shop = join(out, 'summary-shop');

// A project's own blueprint, with something for each part: a table it
// makes, a step that says what it is for, one that runs a command (this
// program's own, which is on every system), and something it hears.
const OWN = `blueprint: watch
version: 1
description: Keep a table of what was seen, and say so in the log.
permissions: [projects, shell]

params:
  table: { type: string, default: seen, description: What the table is called. }

steps:
  seen:
    do: collection.ensure
    with: { name: "\${params.table}", fields: { title: { type: text } } }
  look:
    description: Say which program this is. Nothing is read or written by it.
    do: shell
    with: { cli: [version] }

on:
  added:
    event: record.created
    for: { project: "\${instance.project}" }
    do: log
    with: { message: added }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
let page = null;
let small = null;
let less = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
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
        // Nobody is signed in yet: the answer is the daemon's own.
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
  const laid = (await ask('/api/blueprints/format', { source: OWN, project: 'shop' })).formatted;
  expect('the daemon to lay the blueprint out', typeof laid === 'string' && laid.includes('record.created'));
  // A blueprint of the project's own, where the factory keeps it (`@blueprints/` of the project's).
  await mkdir(join(data, 'blueprints', 'projects', 'shop'), { recursive: true });
  const file = join(data, 'blueprints', 'projects', 'shop', `watch.${ending}`);
  await writeFile(file, laid);
  // One that makes two tables, one naming the other's records: its Summary has a part Data model.
  await writeFile(join(data, 'blueprints', 'projects', 'shop', `tables.${ending}`), (await ask('/api/blueprints/format', { source: TABLES, project: 'shop' })).formatted);

  // ---- what the check reads of the page ------------------------------------------
  const drawn = (on = page) => on.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const shot = (name, on = page) => on.screenshot({ path: `${out}/${name}.png` });
  const summary = (name, on = page) => on.locator(`.summary[data-of="${name}"]`);
  const pressed = async (on = page) => (await on.locator('.view-head .pills button[aria-pressed="true"]').last().innerText()).trim();
  /** Opens what a fold puts away: each handler's steps, and a long list of what a blueprint may be told. */
  const unfolded = async (on = page) => {
    // One at a time from the first: a fold opened is no longer one that is folded, so those counted before it move up;
    // and rows of what it makes shown by one bring folds of their own (each table's model), opened in their turn.
    const shut = on.locator('.summary-fold[aria-expanded="false"]');
    while ((await shut.count()) > 0) {
      const fold = shut.first();
      const mark = await fold.evaluate((one) => {
        one.dataset.pressing = 'yes';
        return true;
      });
      await (on === small ? fold.tap() : fold.click());
      if (mark) await on.waitForFunction(() => !document.querySelector('.summary-fold[data-pressing="yes"][aria-expanded="false"]'));
      await on.evaluate(() => document.querySelectorAll('[data-pressing]').forEach((one) => delete one.dataset.pressing));
    }
  };
  /**
   * Holds what the page draws to what the daemon's own summary of the blueprint says, both ways
   * (`summarised.mjs`), once every fold is open. Gives the daemon's parts, each with its lines as a
   * reader reads them.
   */
  const asTheDaemonSays = async (asked, when, on = page) => {
    await unfolded(on);
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
        wide: column.right - column.left,
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
    return now;
  };
  /** Whether a line of the text that reads so is in sight: drawn, and inside what the text scrolls in. */
  const inSight = (words, on = page) =>
    on.waitForFunction((want) => {
      const view = [...document.querySelectorAll('.view')].find((one) => one.querySelector('.cm-scroller')?.getClientRects().length);
      const [scroller, line] = [view?.querySelector('.cm-scroller')?.getBoundingClientRect(), [...(view?.querySelectorAll('.cm-line') ?? [])].find((one) => one.textContent === want)?.getBoundingClientRect()];
      return !!scroller && !!line && line.top >= scroller.top - 0.5 && line.bottom <= scroller.bottom + 0.5;
    }, words);
  const opens = async (name, on = page) => {
    await side(on, 'Resources');
    const nav = on.locator('.nav');
    if ((await nav.getByText(name, { exact: true }).count()) === 0) {
      await unfold(on, nav.locator('button.nav-item', { has: on.getByText('std', { exact: true }) }));
    }
    await nav.getByText(name, { exact: true }).click();
    await on.locator('.view-head').getByText(name, { exact: false }).first().waitFor();
  };

  /** Opens a blueprint by its address, as a link to it does: one that is a part of another is folded away in the side panel. */
  const openAt = async (name, on = page) => {
    await on.evaluate((to) => (location.hash = to), `/blueprint/${name}`);
    await on.locator('.view-head').getByText(name, { exact: false }).first().waitFor();
  };

  // ---- a blueprint that ships opens on what it says in plain words ----------------
  await opens('std/board');
  await summary('std/board').waitFor();
  expect(`a blueprint that ships to open on its Summary, with nothing pressed; it shows ${await pressed()}`, (await pressed()) === 'Summary');
  expect('no text to be drawn under a Summary', (await page.locator('.cm-content').count()) === 0);
  const board = await asTheDaemonSays({ name: 'std/board' }, 'for the board that ships');
  expect(`the board's parts in order, got ${board.map((part) => part.kind).join(', ')}`, board.map((part) => part.kind).join(',') === 'is,makes,does,may,change');
  const made = board[1].lines.map((line) => line.text);
  expect(`the board to say what it makes, by what its action says it makes and by the language, got ${made.join(' | ')}`, made[0] === 'A table called board.' && made[1] === 'Another blueprint, std/cards, started under it as cards and left running.');
  expect('no action to be named where a sentence should be', !board.flatMap((part) => part.lines).some((line) => /collection\.ensure/.test(line.text)));
  // Its own table's rows are its cards, which set nothing it does: changing them is not offered.
  const boardChange = board.find((part) => part.kind === 'change').lines.map((line) => line.text);
  expect(`the board to offer no rows of its own table to change, got ${boardChange.join(' | ')}`, !boardChange.some((text) => /^Change the rows|^board:/.test(text)));
  // What is set apart as written is drawn as that; and what it makes is a table of its own, a row for each.
  expect('a name to be drawn as a word of the text, in its row of what it makes', (await summary('std/board').locator('.summary-part[data-part="makes"] tr.summary-made-row[data-what="a table"] .summary-made-name code', { hasText: /^board$/ }).count()) === 1);
  expect('what it makes to be said nowhere in its head', (await summary('std/board').locator('.summary-fact[data-label="Makes"]').count()) === 0);
  await laidOut('for the board that ships');
  await shot(`summary-1-board-${scheme}`);

  // A line about the whole is nothing to press; one about a step shows where the step is written.
  const boardDrawn = (await onPage(page)).lines;
  expect('a line about the whole blueprint to be nothing to press', boardDrawn.filter((line) => line.part === 'is').every((line) => !line.pressed));
  expect('a line about a step to be something to press', boardDrawn.filter((line) => line.part === 'does').every((line) => line.pressed && line.at?.startsWith('main/')));
  await summary('std/board').locator('.summary-part[data-part="does"] .summary-line[data-at="main/cards"] button.summary-words').click();
  await page.locator('.cm-content').waitFor();
  expect(`the text to be what is shown once a line is pressed; it shows ${await pressed()}`, (await pressed()) === 'Text');
  await inSight('  cards:');
  await shot(`summary-2-board-line-pressed-${scheme}`);
  // The text was chosen, and is what this blueprint shows from here until its Summary is chosen again.
  await page.getByRole('tab', { name: 'Summary' }).last().click();
  await summary('std/board').waitFor();

  // ---- the longest that ships: what it hears, and the boxes at every width ---------
  await opens('std/fleet');
  await summary('std/fleet').waitFor();
  // The dozen things it may be told are folded away under their line, so that what it makes is read without scrolling past them.
  // Its own part's fold: a list under a later part (the files what it runs links, by part) folds too where it is long.
  const folded = summary('std/fleet').locator('.summary-part[data-part="is"] .summary-fold');
  expect('what the fleet may be told to be folded away until it is asked for', (await folded.count()) === 1 && (await folded.getAttribute('aria-expanded')) === 'false' && (await summary('std/fleet').locator('.summary-part[data-part="is"] .summary-key').count()) === 0);
  // Its head says what it makes and may do in a few words each, and what it does when it is applied is in sight under it.
  const does = await summary('std/fleet').locator('.summary-part[data-part="does"] > h2').boundingBox();
  expect(`what the fleet does to be in sight as it is opened, got ${JSON.stringify(does)}`, does && does.y + does.height < 860);
  const fleetTables = await summary('std/fleet').locator('.summary-part[data-part="makes"] tr.summary-made-row[data-what="a table"] .summary-made-name .summary-said > .summary-words').allInnerTexts();
  expect(`the tables it makes to be rows of its table of what it makes, got ${fleetTables.join(', ')}`, ['agents', 'stages'].every((table) => fleetTables.includes(table)));
  await laidOut('for the fleet that ships, as it is opened');
  await madeLaid({ on: page, expect, when: 'for the fleet that ships, as it is opened' });
  await shot(`summary-3-fleet-as-opened-${scheme}`);
  await fleetMade({ on: page, handed: (await ask('/api/blueprints/summary', { name: 'std/fleet' })).summary, expect, shot, scheme });
  const fleet = await asTheDaemonSays({ name: 'std/fleet' }, 'for the fleet that ships');
  expect('the things it may be told to be listed once they are asked for, each by its name', (await summary('std/fleet').locator('.summary-part[data-part="is"] .summary-key').count()) === 12);
  expect(`the fleet's parts in order, got ${fleet.map((part) => part.kind).join(', ')}`, fleet.map((part) => part.kind).join(',') === 'is,makes,does,hears,may,change');
  expect('the fleet to say the tables it makes', ['agents', 'stages', 'segments'].every((table) => fleet[1].lines.some((line) => line.text === `A table called ${table}.`)));
  // Their rows set what it does, and each is offered to be changed.
  const fleetChange = fleet.find((part) => part.kind === 'change').lines.map((line) => line.text);
  expect(`the fleet to offer the rows of each of its tables to change, got ${fleetChange.join(' | ')}`, fleetChange.some((text) => text.startsWith('Change the rows of a table it makes')) && ['agents', 'stages', 'segments'].every((table) => fleetChange.some((text) => text.startsWith(`${table}: `))));
  const heard = fleet[3].lines.filter((line) => line.at?.endsWith('/@'));
  expect(`each handler to be said by what happens and not by its topic alone, got ${heard.map((line) => line.text.slice(0, 40)).join(' | ')}`, heard.length === 5 && heard.every((line) => /^When (this happens|any of these happens): [a-z]/.test(line.text)));
  for (const width of [1280, 900, 600]) {
    await page.setViewportSize({ width, height: 860 });
    const now = await laidOut(`for the fleet that ships at ${width} wide`);
    await madeLaid({ on: page, expect, when: `for the fleet that ships at ${width} wide` });
    expect(`the column to be no wider than it reads well at, got ${now.wide}`, now.wide <= 780.5);
    if (width !== 1280) await shot(`summary-3-fleet-${width}-${scheme}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await shot(`summary-3-fleet-1280-${scheme}`);
  {
    // At 1280 wide with the side panel open, what is pressed in its head stays on the title's line, and the faint
    // words of where it is from give way, cut short and said whole where they are pointed at.
    const head = page.locator('.blueprint-page > .headed > .view-head').last();
    const middle = async (one) => {
      const box = await one.boundingBox();
      return box.y + box.height / 2;
    };
    const title = await head.locator('h1').boundingBox();
    const applies = await middle(head.getByRole('button', { name: 'Apply', exact: true }));
    expect(`Apply to be on the title's line at 1280 wide, its middle at ${applies} and the title from ${title.y} to ${title.y + title.height}`, applies > title.y && applies < title.y + title.height);
    const note = head.locator('.head-note');
    expect('the words of where it is from to say themselves whole where they are pointed at', (await note.getAttribute('title')) === (await note.innerText()) || (await note.getAttribute('title'))?.startsWith('ships with the product'));
  }
  // What it hears is further down than the window is tall: brought into view, as somebody scrolls to it.
  const changed = summary('std/fleet').locator('.summary-line[data-at="on.changed/@"] > .summary-said');
  await changed.scrollIntoViewIfNeeded();
  await laidOut('for the fleet that ships, scrolled to what it hears');
  await shot(`summary-4-fleet-hears-${scheme}`);
  // A handler's line shows where the handler is written.
  await changed.locator('button.summary-words').click();
  await page.locator('.cm-content').waitFor();
  await inSight('  changed:');
  await page.getByRole('tab', { name: 'Summary' }).last().click();
  await summary('std/fleet').waitFor();

  // ---- what a stage hears: a card each, its steps folded away, kept by the tab ---------
  await openAt('std/working');
  const working = summary('std/working');
  await working.waitFor();
  const cards = working.locator('.summary-part[data-part="hears"] .summary-card');
  const workingHandled = (await ask('/api/blueprints/summary', { name: 'std/working' })).summary.parts.find((part) => part.kind === 'hears').lines.filter((line) => line.at?.endsWith('/@'));
  expect(`a card for each thing it hears, got ${await cards.count()} of ${workingHandled.length}`, (await cards.count()) === workingHandled.length && workingHandled.length > 10);
  expect('the count of what it hears to be said by its heading', (await working.locator('.summary-part[data-part="hears"] > h2 .summary-count').innerText()).trim() === String(workingHandled.length));
  expect("every handler's steps to be folded away as it is opened", (await working.locator('.summary-card .summary-step').count()) === 0 && (await working.locator('.summary-unfolds[aria-expanded="true"]').count()) === 0);
  // The event first, and what it is after it, as the daemon handed its title.
  const titles = await cards.locator(':scope > .summary-said > .summary-words').allInnerTexts();
  expect(`each card to say first the event it hears, got ${titles.slice(0, 3).join(' | ')}`, titles.every((title, at) => title === bare(workingHandled[at].title)) && titles.some((title) => /^board\.item\.moved: a card was moved/.test(title)));
  // Where most hear from is said once over them; one that hears from elsewhere says so on its card.
  const FACTORY = 'Heard from its own project, and from the factory itself.';
  expect('where most of what it hears is heard from to be said once, over the cards', (await working.locator('.summary-part[data-part="hears"] .summary-from').allInnerTexts()).join('|') === FACTORY && (await cards.locator('li[data-from]', { hasText: FACTORY }).count()) === 0);
  const elsewhere = workingHandled.filter((line) => line.from && line.from !== FACTORY).length;
  expect(`each card that hears from elsewhere to say where, got ${await cards.locator('li[data-from]').count()} of ${elsewhere}`, elsewhere > 0 && (await cards.locator('li[data-from]').count()) === elsewhere);
  expect('how a card is taken one at a time to be a tag of its own', (await cards.locator('.summary-tag', { hasText: /^one at a time by the event's / }).count()) > 0);
  await laidOut('for a stage that works, as it is opened');
  await shot(`summary-11-working-${scheme}`);
  // By the keyboard: a fold opened by Enter and folded by Space, and the next reached by Tab.
  const firstFold = cards.nth(0).locator('.summary-unfolds');
  await firstFold.focus();
  await page.keyboard.press('Enter');
  await cards.nth(0).locator('.summary-step').first().waitFor();
  expect('a fold opened by the keyboard to say so', (await firstFold.getAttribute('aria-expanded')) === 'true');
  await page.keyboard.press('Space');
  await cards.nth(0).locator('.summary-step').first().waitFor({ state: 'detached' });
  await page.keyboard.press('Enter');
  await cards.nth(0).locator('.summary-step').first().waitFor();
  let reached = false;
  for (let press = 0; press < 4 && !reached; press += 1) {
    await page.keyboard.press('Tab');
    reached = await cards.nth(1).locator('.summary-unfolds').evaluate((fold) => fold === document.activeElement);
  }
  expect("the next card's fold to be reached by the keyboard", reached);
  // By the pointer, and a step under it pressed to show where it is written.
  await cards.nth(2).locator('.summary-unfolds').click();
  await cards.nth(2).locator('.summary-step').first().waitFor();
  await laidOut('for a stage that works, two handlers unfolded');
  await shot(`summary-12-working-unfolded-${scheme}`);
  await cards.nth(0).locator('.summary-step[data-at="on.looked/wrong"] button.summary-words').click();
  await page.locator('.cm-content').waitFor();
  await inSight('      wrong:');
  // Back to its Summary, and after another tab was shown: the folds are as they were left.
  const openNow = async () => (await working.locator('.summary-unfolds[aria-expanded="true"]').evaluateAll((all) => all.map((one) => one.closest('.summary-card').dataset.at))).join(',');
  await page.getByRole('tab', { name: 'Summary' }).last().click();
  await working.waitFor();
  expect(`what was unfolded to be kept when the Summary is shown again, got ${await openNow()}`, (await openNow()) === `${workingHandled[0].at},${workingHandled[2].at}`);
  await openAt('std/board');
  await summary('std/board').waitFor();
  await openAt('std/working');
  await working.waitFor();
  expect(`what was unfolded to be kept by the tab while another is shown, got ${await openNow()}`, (await openNow()) === `${workingHandled[0].at},${workingHandled[2].at}`);
  const worked = await asTheDaemonSays({ name: 'std/working' }, 'for a stage that works');
  expect(`its parts in the daemon's order, got ${worked.map((part) => part.kind).join(', ')}`, worked.map((part) => part.kind).join(',') === 'is,makes,does,hears,may,ends,change');
  for (const width of [1280, 900, 600]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(`for a stage that works, unfolded, at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // What it makes and starts: a row each, the same blueprint started the same way from several handlers one row.
  const workingMade = working.locator('.summary-part[data-part="makes"] .summary-made-row');
  const dueRow = workingMade.filter({ has: page.locator('.summary-what', { hasText: /^std\/due$/ }) });
  expect('a blueprint started the same way from several handlers to be one row', (await dueRow.count()) === 1);
  expect(`that row to say each handler it is started from, got ${await dueRow.locator('.summary-step-of').allInnerTexts()}`, (await dueRow.locator('.summary-step-of').allInnerTexts()).join(',') === 'first,staffed,listening,unmerged,tidying,unboarded,away');
  expect('that row to say what it is started as and that it is waited for', (await dueRow.locator('.summary-name').innerText()) === 'look' && (await dueRow.locator('.summary-tag').innerText()) === 'waited for');
  // Its ways to change it, each what is done and what is said of it.
  const ways = working.locator('.summary-part[data-part="change"] > .summary-lines > .summary-action');
  expect('each way to change it to say what is done and what is said of it', (await ways.count()) >= 3 && (await ways.locator(':scope > .summary-said > .summary-act').count()) === (await ways.count()) && (await ways.locator(':scope > .summary-detail').count()) === (await ways.count()));
  await working.locator('.summary-part[data-part="makes"] > h2').scrollIntoViewIfNeeded();
  await shot(`summary-13-working-makes-${scheme}`);
  // A row's blueprint pressed opens it.
  await dueRow.locator('button.summary-what').click();
  await summary('std/due').waitFor();
  await openAt('std/working');
  await working.waitFor();
  // A long list of what it makes shows its first rows until the rest is asked for.
  await openAt('std/planning');
  const planning = summary('std/planning');
  await planning.waitFor();
  const planned = planning.locator('.summary-part[data-part="makes"]');
  expect(`a long list of what it makes to show its first five rows, got ${await planned.locator('.summary-made-row').count()}`, (await planned.locator('.summary-made-row').count()) === 5 && (await planned.locator('button.summary-more-rows[aria-expanded="false"]').count()) === 1);
  await asTheDaemonSays({ name: 'std/planning' }, 'for a stage that plans, its list of what it makes opened');
  expect('the whole list once it is asked for', (await planned.locator('.summary-made-row').count()) > 7);
  await laidOut('for a stage that plans, unfolded');
  await openAt('std/working');
  await working.waitFor();
  // Closed and opened again, its folds are as they are first.
  await page.getByRole('button', { name: 'Close std/working', exact: true }).click();
  await summary('std/working').waitFor({ state: 'detached' });
  await openAt('std/working');
  await working.waitFor();
  expect('a tab closed to keep none of its folds', (await openNow()) === '');

  // ---- a project's own that makes two tables: what they are made of, together ------
  await page.evaluate((to) => (location.hash = to), `/file/shop/@blueprints/tables.${ending}`);
  await page.locator('.view-head').getByText(`tables.${ending}`, { exact: false }).first().waitFor();
  await page.getByRole('tab', { name: 'Summary' }).last().click();
  await summary('tables').waitFor();
  const tablesSaid = await asTheDaemonSays({ name: 'tables', project: 'shop' }, "for a project's own blueprint that makes two tables");
  await dataModel({ on: page, handed: { parts: tablesSaid }, expect, shot, scheme });
  for (const width of [1280, 900, 600]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(`for a blueprint that makes two tables at ${width} wide`);
    await madeLaid({ on: page, expect, when: `for a blueprint that makes two tables at ${width} wide` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // ---- a project's own: its text for whoever may change it -------------------------
  // Opened by its address, as a link to it opens it: beside what was open.
  await page.evaluate((to) => (location.hash = to), `/file/shop/@blueprints/watch.${ending}`);
  await page.locator('.view-head').getByText(`watch.${ending}`, { exact: false }).first().waitFor();
  await page.locator('.cm-content').waitFor();
  expect(`a project's own blueprint to open on its text for whoever may change it; it shows ${await pressed()}`, (await pressed()) === 'Text');
  await page.getByRole('tab', { name: 'Summary' }).last().click();
  await summary('watch').waitFor();
  const own = await asTheDaemonSays({ source: laid, project: 'shop' }, "for a project's own blueprint");
  expect(`its parts in order, got ${own.map((part) => part.kind).join(', ')}`, own.map((part) => part.kind).join(',') === 'is,makes,does,hears,may');
  expect(`what it makes to be said by the param that names it, got ${own[1].lines[0].text}`, own[1].lines[0].text === 'A table called what the param table says.');
  expect(`a step to be said by what it says of itself, as far as its first full stop, got ${own[2].lines[1].text}`, own[2].lines[1].text === 'Say which program this is.' && own[2].lines[1].key === 'look');
  const command = own[4].lines.filter((line) => line.mark === 'command');
  expect(`the command it runs to be said and marked, got ${JSON.stringify(command)}`, command.length === 1 && command[0].text === 'It runs a command: version.' && command[0].at === 'main/look');
  expect('the mark to say what it is of', (await summary('watch').locator('.summary-line[data-mark="command"] .summary-mark').innerText()).trim() === 'runs a command');
  expect('its head to say once that it runs a command', (await summary('watch').locator('.summary-head .summary-marked .summary-mark').allInnerTexts()).join('|') === 'runs a command');
  expect('nothing to be said of what its runs have cost while it has not run', own.length === 5 && (await summary('watch').locator('.summary-part[data-part="cost"]').count()) === 0);
  for (const width of [1280, 900, 600]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(`for a project's own blueprint at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await shot(`summary-5-own-${scheme}`);

  // ---- it follows the text as it is written, before it is saved ---------------------
  await page.getByRole('tab', { name: 'Text' }).last().click();
  await page.locator('.cm-content').click();
  await page.keyboard.press('Control+Home');
  for (let down = 0; down < 2; down += 1) await page.keyboard.press('ArrowDown');
  await page.keyboard.press('End');
  await page.waitForFunction(() => document.querySelector('.cm-activeLine')?.textContent?.startsWith('description: '));
  await page.keyboard.type(' Twice over.');
  await page.locator('.view-head').getByText('unsaved').waitFor();
  await page.getByRole('tab', { name: 'Summary' }).last().click();
  await summary('watch').locator('.summary-part[data-part="is"] .summary-words', { hasText: 'Keep a table of what was seen, and say so in the log. Twice over.' }).waitFor();
  expect('the text to be unsaved still: what is said is what is being written', (await page.locator('.view-head').getByText('unsaved').count()) === 1);

  // ---- a text that does not check has none, and the page says what is wrong ----------
  await page.getByRole('tab', { name: 'Text' }).last().click();
  await page.locator('.cm-content').click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.type('x');
  await page.locator('.problems .problem .error-text').first().waitFor();
  const wrong = await page.locator('.problems .problem').evaluateAll((all) => all.filter((one) => one.querySelector('.error-text')).map((one) => one.lastElementChild.textContent));
  expect('the check to say what is wrong with it', wrong.length > 0);
  await page.getByRole('tab', { name: 'Summary' }).last().click();
  await page.getByText('This cannot be said in plain words until it checks').waitFor();
  expect('no Summary of a text that does not check', (await page.locator('.summary').count()) === 0);
  const told = await page.locator('.summary-wrong li').allInnerTexts();
  // What the check sets apart as written is drawn as that here, without the marks that set it apart.
  expect(`what is wrong to be said in the check's own words, got ${told.join(' | ')} for ${wrong.join(' | ')}`, told.length === wrong.length && wrong.every((one, at) => told[at].endsWith(one.replaceAll('`', ''))));
  const none = await page.evaluate(() => {
    const parts = [...document.querySelector('.summary-none').children].map((one) => one.getBoundingClientRect());
    return { gaps: parts.slice(1).map((one, at) => one.top - parts[at].bottom), sideways: document.documentElement.scrollWidth - window.innerWidth };
  });
  expect(`what is wrong to stand directly under its heading and over the way to it, got gaps of ${none.gaps.join(', ')}`, none.gaps.every((gap) => gap >= 0 && gap <= 24) && none.sideways <= 0);
  await shot(`summary-6-does-not-check-${scheme}`);
  await page.getByRole('button', { name: 'See them in the text' }).click();
  await page.locator('.cm-content').waitFor();
  expect(`the text to be shown with its problems; it shows ${await pressed()}`, (await pressed()) === 'Text' && (await page.locator('.problems .problem').count()) > 0);
  // Put right, it has its Summary again; and saved as it was first written, it is the file's.
  await page.locator('.cm-content').click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.press('Delete');
  await page.locator('.problems .problem .error-text').first().waitFor({ state: 'detached' });
  await page.getByRole('tab', { name: 'Summary' }).last().click();
  await summary('watch').waitFor();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await page.locator('.view-head').getByText('unsaved').waitFor({ state: 'detached' });

  // ---- what its runs have cost, once it has run -------------------------------------
  await cli('apply', file, '--project', 'shop');
  const cost = summary('watch').locator('.summary-part[data-part="cost"]');
  await cost.waitFor();
  const spent = (await cost.locator('.summary-words').allInnerTexts()).map((line) => line.trim());
  expect(`what it cost to be counted and nothing more, got ${spent.join(' | ')}`, spent.length === 2 && spent[0] === 'It ran once in the last 7 days; none failed.' && /^The run took \d/.test(spent[1]));
  expect('no line of it to be anything to press', (await cost.locator('button').count()) === 0);
  await laidOut("for a project's own blueprint that has run");
  await cost.scrollIntoViewIfNeeded();
  await shot(`summary-7-own-cost-${scheme}`);
  // It is no part of what the daemon says of the text.
  await asTheDaemonSays({ name: 'watch', project: 'shop' }, "for a project's own blueprint by its name");
  // By its id: `stop` takes an instance, and was never told a project.
  for (const one of JSON.parse((await cli('instances', '--project', 'shop', '--json')).stdout).instances.filter((one) => one.blueprint === 'watch')) await cli('stop', one.id);

  // ---- somebody who may not change it: its Summary, with nothing pressed -------------
  const holds = ['projects:read', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];
  less = await signIn(await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme }), { at: `#/file/shop/@blueprints/watch.${ending}`, holds });
  await summary('watch', less).waitFor();
  expect(`a project's own blueprint to open on its Summary for somebody who may not change it; it shows ${await pressed(less)}`, (await pressed(less)) === 'Summary');
  await laidOut('for somebody who may not change it', less);

  // ---- a phone ----------------------------------------------------------------------
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  small = await signIn(phone, { at: '#/blueprint/std/fleet', touch: true });
  await summary('std/fleet', small).waitFor();
  await asTheDaemonSays({ name: 'std/fleet' }, "at a phone's width", small);
  await laidOut("at a phone's width", small);
  await madeLaid({ on: small, expect, when: "at a phone's width", stacked: true });
  await shot(`summary-8-phone-${scheme}`, small);
  await summary('std/fleet', small).locator('.summary-line[data-at="on.changed/@"] > .summary-said').scrollIntoViewIfNeeded();
  await laidOut("at a phone's width, scrolled to what it hears", small);
  await shot(`summary-9-phone-hears-${scheme}`, small);

  // ---- every view of the head, at every width ---------------------------------------
  // A project's own blueprint that has run and been applied has every view there is, once a sandbox test
  // and a play side by side were run from its tab; each is pressed at each width and shows what it is.
  const VIEWS = { Summary: '.summary', Text: '.cm-content', Node: '.canvas', Runs: '.runs-beside', History: '.history', Sandbox: '.sandbox', 'Side by side': '.beside-view' };
  /** The head's views and buttons by their boxes: each wholly in the window and not cut off by anything, no view over another. */
  const headLaid = (on) =>
    on.evaluate(() => {
      const head = [...document.querySelectorAll('.view-head')].find((one) => one.getClientRects().length > 0 && one.querySelector('.pills'));
      const box = (el) => {
        const at = el.getBoundingClientRect();
        return { el, name: el.textContent.trim(), left: at.left, right: at.right, top: at.top, bottom: at.bottom };
      };
      const views = [...head.querySelectorAll('.pills button')].map(box);
      const buttons = [...head.querySelectorAll('button')].filter((one) => !one.closest('.pills') && one.getClientRects().length > 0).map(box);
      const wrong = [];
      for (const one of [...views, ...buttons]) {
        if (one.left < -0.5 || one.right > innerWidth + 0.5 || one.top < -0.5 || one.bottom > innerHeight + 0.5) {
          wrong.push(`"${one.name}" is not wholly in the window (${Math.round(one.left)} to ${Math.round(one.right)} of ${innerWidth})`);
          continue;
        }
        // Its four edges are its own: nothing cuts it off, and nothing lies over it. (At the middle of each, since a
        // corner is rounded off and is the parent's there.)
        const [x, y] = [(one.left + one.right) / 2, (one.top + one.bottom) / 2];
        for (const [at, there] of [[one.left + 2, y], [one.right - 2, y], [x, one.top + 2], [x, one.bottom - 2]].entries()) {
          if (!one.el.contains(document.elementFromPoint(...there))) {
            wrong.push(`"${one.name}" is not to be seen at its ${['left', 'right', 'top', 'bottom'][at]} edge`);
            break;
          }
        }
      }
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      views.forEach((a, at) => views.slice(at + 1).forEach((b) => over(a, b) && wrong.push(`"${a.name}" over "${b.name}"`)));
      return { views: views.map((one) => one.name), wrong, sideways: document.documentElement.scrollWidth - innerWidth };
    });
  /** Testing's menu by its boxes: opened, in the window, its lines and what it says under them none over another, and the head as it was; put away again. */
  const testingLaid = async (on, touch, picture) => {
    const head = on.locator('.view-head').locator('visible=true').last();
    const boxes = () => head.locator('button').evaluateAll((all) => all.filter((one) => one.getClientRects().length > 0).map((one) => { const at = one.getBoundingClientRect(); return `${one.textContent.trim()}@${Math.round(at.left)},${Math.round(at.top)}`; }));
    const before = await boxes();
    const menu = await openTesting(on, head, touch);
    await drawn(on);
    const laid = await menu.evaluate((box) => {
      const wrong = [];
      const at = (el) => el.getBoundingClientRect();
      const whole = at(box);
      if (whole.left < -0.5 || whole.right > innerWidth + 0.5 || whole.top < -0.5 || whole.bottom > innerHeight + 0.5) wrong.push('the menu is not wholly in the window');
      const parts = [...box.children].map((one) => ({ what: one.textContent.trim().slice(0, 30), ...at(one).toJSON() }));
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      parts.forEach((a, k) => parts.slice(k + 1).forEach((b) => over(a, b) && wrong.push(`"${a.what}" over "${b.what}"`)));
      for (const one of box.querySelectorAll('button .truncate, .menu-note')) if (one.scrollWidth > one.clientWidth + 1) wrong.push(`"${one.textContent.trim()}" cut short`);
      if (document.elementFromPoint((whole.left + whole.right) / 2, whole.top + 6)?.closest('.row-menu') !== box) wrong.push('the menu is not over the page');
      return { wrong, lines: [...box.querySelectorAll('button .truncate')].map((one) => one.textContent.trim()) };
    });
    const after = await boxes();
    if (after.join('|') !== before.join('|')) laid.wrong.push(`the head moved: ${before.join(' ')} to ${after.join(' ')}`);
    if (picture) await shot(picture, on);
    await closeTesting(on);
    return laid;
  };
  const everyView = async (on, when, touch = false) => {
    await drawn(on);
    const now = await headLaid(on);
    expect(`every view of the head ${when}, got ${now.views.join(', ')}`, now.views.join('|') === Object.keys(VIEWS).join('|'));
    expect(`every view and button of the head wholly in the window, cut off by nothing and none over another, ${when}, got ${now.wrong.join('; ')}`, now.wrong.length === 0);
    expect(`nothing to scroll sideways ${when}, got ${now.sideways}`, now.sideways <= 0);
    const menu = await testingLaid(on, touch, `summary-10-testing-${when.replace(/\W+/g, '-')}-${scheme}`);
    expect(`Testing's four lines in their order ${when}, got ${menu.lines.join(', ')}`, menu.lines.join('|') === 'Preview|Sandbox test|Side by side|Compare on real work');
    expect(`Testing's menu over the page, wholly in the window, no line over another and nothing of the head moved ${when}, got ${menu.wrong.join('; ')}`, menu.wrong.length === 0);
    for (const [view, shows] of Object.entries(VIEWS)) {
      const tab = on.locator('.view-head .pills').getByRole('tab', { name: view, exact: true }).last();
      await (touch ? tab.tap() : tab.click());
      await on.locator(`${shows} >> visible=true`).first().waitFor();
      expect(`${view} pressed ${when} to be the view shown; it shows ${await pressed(on)}`, (await pressed(on)) === view);
    }
  };
  const allViews = async (on, touch = false) => {
    await on.evaluate((to) => (location.hash = to), `/file/shop/@blueprints/watch.${ending}`);
    await on.locator('.view-head').getByText(`watch.${ending}`, { exact: false }).first().waitFor();
    await on.locator('.view-head .pills').getByRole('tab', { name: 'History', exact: true }).waitFor();
    for (const [line, shows] of [['Sandbox test', '.sandbox'], ['Side by side', '.beside-view']]) {
      await pressTesting(on, on.locator('.view-head').locator('visible=true').last(), line, touch);
      await on.locator(`${shows} >> visible=true`).first().waitFor();
    }
  };
  await allViews(small, true);
  for (const [width, height] of [[390, 844], [412, 915]]) {
    await small.setViewportSize({ width, height });
    await everyView(small, `at ${width} wide`, true);
    await small.locator('.view-head .pills').getByRole('tab', { name: 'Summary', exact: true }).tap();
    await summary('watch', small).waitFor();
    await shot(`summary-10-views-${width}-${scheme}`, small);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await allViews(page);
  for (const width of [1280, 900, 600]) {
    await page.setViewportSize({ width, height: 860 });
    await everyView(page, `at ${width} wide`);
    await shot(`summary-10-views-${width}-${scheme}`);
  }
  // The page narrowed by the side panel beside it, as a narrow pane is.
  await page.setViewportSize({ width: 900, height: 860 });
  await side(page, 'Resources');
  await everyView(page, 'at 900 wide beside the side panel');
  await shot(`summary-10-views-900-beside-${scheme}`);
  await page.setViewportSize({ width: 1280, height: 860 });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/summary-0-failed.png` }).catch(() => {});
  await less?.screenshot({ path: `${out}/summary-0-failed-second-page.png` }).catch(() => {});
  await small?.screenshot({ path: `${out}/summary-0-failed-phone.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
