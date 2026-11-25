// Sandbox tests side by side, from a blueprint's page.
//
//   DAEMON=../target/debug/factory node scripts/sidebyside.mjs [out-dir]
//
// It goes by what the page shows and by the boxes, against what the
// daemon hands when it is asked the same:
//
//   - the board that ships, opened for a project that has no fork of it,
//     while another project has one that does less: Side by side under
//     Testing in the head plays the default and the other project's fork, each in a
//     sandbox of its own, and each column says what it ran, how it
//     stands, every row the daemon laid out with its figures and nothing
//     where it has none, its own sandbox as the Sandbox view draws one,
//     and at its foot the daemon's own sentence that it was a sandbox
//     test; the fork's column counts fewer steps than the default's;
//   - the sandboxes the daemon holds for the columns, and nothing of any
//     of them in the factory: nothing running, no project called sandbox;
//   - nothing of a column over another or over anything else of it,
//     outside its column or scrolling sideways, at three widths and a
//     phone's;
//   - Apply in the fork's column, which applies that text for the
//     project the page is of; Compare on real work, which opens the
//     drawer that starts a comparison with each text a way of its own,
//     sent as it was played, and the comparison started from there;
//     Fork in the default's column, which forks it for the project and
//     opens the fork;
//   - the fork's own page, its text changed and not saved: the text as it
//     stands and as it was saved played side by side, the two counted;
//   - the tab closed, and the daemon holding none of its sandboxes;
//   - a project's own blueprint with no fork and nothing unsaved: Side by
//     side pressed says, once its texts are read, that there is one text
//     to play and that changing it here would make a second, with nothing
//     pending and nothing asked of the daemon to play; the press there
//     showing the text; and nothing of what is said over anything else,
//     outside the view or scrolling sideways at three widths and a
//     phone's;
//   - and the other of dark and light, by SCHEME.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { pressTesting } from './testing.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'sidebyside-data');
const home = join(out, 'sidebyside-home');
const shop = join(out, 'sidebyside-shop');
const yard = join(out, 'sidebyside-yard');

// A project's own blueprint that ships under no name: it has no default and no fork, so one text.
const OWN = `blueprint: greet
version: 1
description: Say that somebody was greeted.

steps:
  hello:
    description: Say hello.
    do: emit
    with: { topic: greet.said, who: somebody }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop, yard]) await rm(dir, { recursive: true, force: true });
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
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  await cli('project', 'create', '--bare', 'Yard', '--path', yard);
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
  const signIn = async (context, { at = '', touch = false } = {}) => {
    const opened = await context.newPage();
    opened.setDefaultTimeout(20_000);
    opened.on('pageerror', (err) => problems.push(err.message));
    await opened.goto(`${url}/${at}`);
    await opened.getByLabel('Email').fill('ada@example.com');
    await opened.getByLabel('Password').fill('correct horse');
    const press = opened.getByRole('button', { name: 'Sign in', exact: true }).last();
    await (touch ? press.tap() : press.click());
    return opened;
  };
  page = await signIn(await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme }));
  await page.locator('.rail').waitFor();

  /** Asks the daemon something as the page does, with the page's own key: what it answered, and with what. */
  const asks = (method, path, body, on = page) =>
    on.evaluate(
      async ([how, to, sent]) => {
        const answer = await fetch(to, { method: how, headers: { 'content-type': 'application/json', 'x-page-key': localStorage.getItem('page-key') ?? '' }, body: sent === null ? undefined : JSON.stringify(sent) });
        return { status: answer.status, body: await answer.json().catch(() => null) };
      },
      [method, path, body ?? null],
    );

  // The shop forks the board and has its fork start nothing for its cards: a fork that does less.
  const forked = await asks('POST', '/api/projects/shop/own', { blueprint: 'std/board' });
  expect(`the shop to have a fork of the board, got ${JSON.stringify(forked.body)}`, forked.status === 200 && typeof forked.body.path === 'string');
  const shopFork = forked.body.path;
  // Written where the factory keeps the project's own (`@blueprints/` of the project's), in its data folder.
  const copied = await readFile(join(data, 'blueprints', 'projects', 'shop', shopFork.replace(/^@blueprints\//, '')), 'utf8');
  const less = copied.replace(/\n {2}cards:\n(?: {4}.*\n)+/, '\n');
  expect('the fork to be written without its step for the cards', less !== copied && !less.includes('spawn: std/cards'));
  const wrote = await asks('PUT', '/api/projects/shop/file', { path: shopFork, content: less });
  expect(`the fork to be saved through the daemon, got ${wrote.status}`, wrote.status === 200);

  // ---- what the check reads of the page ------------------------------------------
  const drawn = (on = page) => on.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const shot = (name, on = page) => on.screenshot({ path: `${out}/${name}.png` });
  const head = (on = page) => on.locator('.view-head').locator('visible=true').last();
  const view = (on = page) => on.locator('.beside-view').locator('visible=true');
  // Side by side is a line of Testing in the head, pressed by the pointer or by a touch.
  const besideButton = (on = page) => ({ click: () => pressTesting(on, head(on), 'Side by side'), tap: () => pressTesting(on, head(on), 'Side by side', true) });
  const answered = (on = page) => on.evaluate(() => Number([...document.querySelectorAll('.view-head button[data-beside-answered]')].find((one) => one.getClientRects().length > 0)?.dataset.besideAnswered ?? 0));
  /** What the page asked the daemon to play, the last time it asked. */
  let sent = null;
  const listen = (on) =>
    on.on('request', (request) => {
      if (request.method() === 'POST' && request.url().endsWith('/api/sandboxes/beside')) sent = request.postDataJSON();
    });
  listen(page);
  /** Does something that has the texts played, and waits until they are answered and read: the page counts each playing. */
  const played = async (press, on = page) => {
    const was = await answered(on);
    await press();
    await on.waitForFunction((before) => {
      const count = Number([...document.querySelectorAll('.view-head button[data-beside-answered]')].find((one) => one.getClientRects().length > 0)?.dataset.besideAnswered ?? 0);
      const shown = [...document.querySelectorAll('.beside-view')].find((one) => one.getClientRects().length > 0);
      return !!shown && count > before && shown.dataset.busy === 'false';
    }, was);
    await drawn(on);
  };
  /** What the view says: the texts offered and chosen, and each column with its counts, how it stands and its foot. */
  const says = (on = page) =>
    on.evaluate(() => {
      const shown = [...document.querySelectorAll('.beside-view')].find((one) => one.getClientRects().length > 0);
      const text = (el) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();
      return {
        texts: [...shown.querySelectorAll('.beside-text')].map((one) => ({ text: text(one), chosen: one.querySelector('input').checked })),
        notes: [...shown.querySelectorAll('.beside-choose .sandbox-note')].map(text),
        columns: [...shown.querySelectorAll('.beside-column')].map((column) => ({
          called: column.dataset.called,
          sandbox: column.dataset.sandbox,
          heading: text(column.querySelector('h3')),
          ran: text(column.querySelector('.beside-ran')),
          outcome: text(column.querySelector('.sandbox-outcome')),
          presses: [...column.querySelectorAll('.beside-presses button')].map(text),
          rows: [...column.querySelectorAll('.beside-counts tr[data-what]')].map((row) => ({ what: row.dataset.what, kind: row.dataset.kind, cells: [...row.querySelectorAll('td.num')].map(text) })),
          heads: [...column.querySelectorAll('.beside-kind th')].map(text),
          said: text(column.querySelector('.sandbox-said')),
          stood: text(column.querySelector('.sandbox-stood-said')),
        })),
      };
    });
  /** The boxes of the view that is showing: each column, and what each is made of. */
  const looked = (on = page) =>
    on.evaluate(() => {
      const shown = [...document.querySelectorAll('.beside-view')].find((one) => one.getClientRects().length > 0);
      const box = (el) => {
        const at = el.getBoundingClientRect();
        return { left: at.left, right: at.right, top: at.top, bottom: at.bottom, what: (el.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 40) };
      };
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const crossed = [];
      const outside = [];
      const among = (boxes) => boxes.forEach((a, at) => boxes.slice(at + 1).forEach((b) => over(a, b) && crossed.push(`"${a.what}" over "${b.what}"`)));
      const inside = (within, boxes) => boxes.forEach((one) => (one.left < within.left - 0.5 || one.right > within.right + 0.5) && outside.push(one.what));
      const columns = [...shown.querySelectorAll('.beside-column')];
      among([box(shown.querySelector('.beside-choose')), ...columns.map(box)]);
      inside(box(shown), [box(shown.querySelector('.beside-choose')), ...columns.map(box)]);
      const choose = shown.querySelector('.beside-choose');
      among([...choose.children].map(box));
      inside(box(choose), [...choose.querySelectorAll('.beside-text, .beside-go > *, .beside-one > *, .beside-seconds li > *')].map(box));
      for (const line of choose.querySelectorAll('.beside-one, .beside-seconds li')) among([...line.children].map(box));
      for (const column of columns) {
        const parts = [...column.children].filter((child) => child.getClientRects().length > 0);
        among(parts.map(box));
        inside(box(column), parts.map(box));
        inside(box(column), [...column.querySelectorAll('.beside-counts td, .beside-presses button, .sandbox-step, .sandbox-foot p')].filter((one) => one.getClientRects().length > 0).map(box));
        for (const row of column.querySelectorAll('.beside-counts tr')) among([...row.children].map(box));
      }
      return {
        crossed,
        outside,
        sideways: document.documentElement.scrollWidth - window.innerWidth,
        inner: [shown, ...columns].map((part) => part.scrollWidth - part.clientWidth),
        across: new Set(columns.map((column) => Math.round(column.getBoundingClientRect().top))).size === 1 && columns.length > 1,
      };
    });
  const laidOut = async (when, on = page) => {
    await drawn(on);
    const now = await looked(on);
    expect(`nothing of the view to lie over anything else of it ${when}, got ${now.crossed.slice(0, 4).join('; ')}`, now.crossed.length === 0);
    expect(`everything to be inside its column ${when}, got ${now.outside.slice(0, 4).join('; ')}`, now.outside.length === 0);
    expect(`nothing to scroll sideways ${when}, got the page ${now.sideways} and the parts ${now.inner}`, now.sideways <= 0 && now.inner.every((over) => over <= 0));
    return now;
  };
  /** Holds each column's figures to what the daemon hands for the same texts played the same way, row for row. */
  const heldTo = async (now, why) => {
    expect(`the page to have asked for what it played (${why})`, sent !== null);
    const daemons = (await asks('POST', '/api/sandboxes/beside', sent)).body;
    expect(`the daemon to play the same (${why}), got ${JSON.stringify(daemons).slice(0, 300)}`, daemons.ok === true);
    try {
      expect(`a column for each text the daemon played, in its order (${why}), got ${now.columns.map((one) => one.called).join(' | ')}`, now.columns.map((one) => one.called).join('|') === daemons.sandboxes.map((one) => one.called).join('|'));
      for (const [at, column] of now.columns.entries()) {
        const laid = daemons.beside.rows;
        expect(`every row the daemon laid out in ${column.called}, in its order (${why}), got ${column.rows.map((row) => row.what).join(', ')}`, column.rows.map((row) => row.what).join('|') === laid.map((row) => row.what).join('|'));
        for (const [index, row] of column.rows.entries()) {
          const cell = laid[index].by[at];
          const first = row.cells[0] ?? '';
          const said = cell === null ? '-' : `${cell.count}${cell.errors > 0 && row.kind !== 'came' ? ` (${cell.errors} failed)` : ''}`;
          expect(`${column.called} to say of ${row.what} what the daemon counted (${why}): ${said}, got ${first}`, first === said);
        }
        const ran = daemons.beside.columns[at];
        expect(`${column.called} to say which text it ran (${why}), got ${column.ran}`, column.ran.includes(ran.text) && (!ran.version || column.ran.includes(`version ${ran.version}`)));
        expect(`the foot of ${column.called} to be the daemon's own sentence (${why}), got ${column.said}`, column.said === daemons.said && column.said.startsWith('This was a sandbox test'));
        expect(`the foot of ${column.called} to say what was stood in, in the daemon's words (${why})`, column.stood === daemons.sandboxes[at].shown.stood_said || column.stood.startsWith(daemons.sandboxes[at].shown.stood_said));
      }
    } finally {
      for (const one of daemons.sandboxes ?? []) await asks('DELETE', `/api/sandboxes/${one.sandbox}`);
    }
    return daemons;
  };
  const row = (column, what) => column.rows.find((one) => one.what === what);
  const counted = (column, what) => Number.parseInt(row(column, what)?.cells[0] ?? '0', 10) || 0;

  // ---- the default beside another project's fork ----------------------------------
  await page.goto(`${url}/#/blueprint/std/board?project=yard`);
  await head().getByText('checks').waitFor();
  expect('no view called Side by side before anything is played', (await head().getByRole('tab', { name: 'Side by side' }).count()) === 0);
  await played(() => besideButton().click());
  expect('the view called Side by side to be showing', (await head().locator('.pills button[aria-pressed="true"]').innerText()).trim() === 'Side by side');
  let now = await says();
  expect(`the default and the shop's fork to be offered and chosen, got ${JSON.stringify(now.texts)}`, now.texts.length === 2 && now.texts[0].text.startsWith('The default') && now.texts[1].text.startsWith('The fork in Shop') && now.texts.every((one) => one.chosen));
  expect(`a column for each, got ${now.columns.map((one) => one.heading).join(' | ')}`, now.columns.length === 2 && now.columns[0].heading === 'The default' && now.columns[1].heading === 'The fork in Shop');
  expect(`each to be up in its sandbox, got ${now.columns.map((one) => one.outcome).join(' | ')}`, now.columns.every((one) => one.outcome.includes('it is up')));
  expect(`the rows to be headed by kind, got ${now.columns[0].heads.join(' | ')}`, now.columns[0].heads.includes('Came of them') && now.columns[0].heads.includes('Measured'));
  expect(`the fork, which starts nothing for its cards, to count fewer steps, got ${counted(now.columns[0], 'step')} and ${counted(now.columns[1], 'step')}`, counted(now.columns[1], 'step') < counted(now.columns[0], 'step'));
  expect(`what to press in each column, got ${now.columns.map((one) => one.presses.join(',')).join(' | ')}`, now.columns[0].presses.join(',') === 'Apply,Fork' && now.columns[1].presses.join(',') === 'Apply');
  await heldTo(now, 'the default and a fork');
  // The daemon holds a sandbox for each column, and nothing of them is the factory's.
  for (const column of now.columns) expect(`the daemon to hold the sandbox of ${column.called}`, (await asks('GET', `/api/sandboxes/${column.sandbox}`)).status === 200);
  const running = (await asks('GET', '/api/instances')).body;
  expect(`nothing to be running in the factory, got ${JSON.stringify(running).slice(0, 200)}`, (running.instances ?? running).length === 0);
  expect('the factory to have no project called sandbox', !((await asks('GET', '/api/projects')).body.projects ?? []).some((one) => one.id === 'sandbox'));
  for (const width of [1280, 1000, 760]) {
    await page.setViewportSize({ width, height: 860 });
    const laid = await laidOut(`for the default and a fork at ${width} wide`);
    if (width === 1280) expect('the two columns to stand side by side where there is room', laid.across);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await shot(`sidebyside-1-default-and-fork-${scheme}`);
  // Each column's own sandbox, as the Sandbox view draws one.
  await view().locator('.beside-column').nth(1).locator('.beside-happened > summary').click();
  await view().locator('.beside-column').nth(1).locator('.sandbox-run').first().waitFor();
  const opened = await page.evaluate(() => {
    const column = [...document.querySelectorAll('.beside-column')].filter((one) => one.getClientRects().length > 0)[1];
    return [...column.querySelectorAll('.sandbox-run h4')].map((one) => one.textContent.trim());
  });
  expect(`what happened in the fork's sandbox to be its runs, got ${opened.join(' | ')}`, opened.includes('std/board: its steps') && !opened.some((one) => one.startsWith('std/cards')));
  await laidOut("with the fork's sandbox opened");
  await shot(`sidebyside-2-opened-${scheme}`);
  const sandboxesOfTab = now.columns.map((one) => one.sandbox);

  // ---- Apply, Compare on real work, Fork ------------------------------------------
  await view().locator('.beside-column').nth(1).getByRole('button', { name: 'Apply', exact: true }).click();
  await page.waitForFunction(async () => {
    const answer = await fetch('/api/instances?project=yard', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return (await answer.json()).instances.some((one) => one.blueprint === 'std/board');
  });
  const applied = (await asks('GET', '/api/instances?project=yard')).body.instances.find((one) => one.blueprint === 'std/board');
  expect(`the fork's text to be what was applied for the yard, got ${applied.revision} for ${now.columns[1].ran}`, now.columns[1].ran.includes(applied.revision.slice(0, 8)));
  const children = (await asks('GET', '/api/instances?project=yard')).body.instances.filter((one) => one.blueprint === 'std/cards');
  expect('nothing to be started for the cards of a board applied as the fork is', children.length === 0);

  await view().getByRole('button', { name: 'Compare on real work', exact: true }).click();
  const comparing = drawerAtRest(page).locator('.comparing');
  await comparing.waitFor();
  const ways = await comparing.locator('.comparing-way').evaluateAll((all) => all.map((one) => ({ name: one.querySelector('input[aria-label^="Way"]').value, sent: one.querySelector('.comparing-sent')?.textContent.trim() ?? null })));
  expect(`a way for each text, each starting it as it was played, got ${JSON.stringify(ways)}`, ways.length === 2 && ways[0].name === 'default' && ways[1].name === 'fork-shop' && ways[0].sent.endsWith('The default') && ways[1].sent.endsWith('The fork in Shop'));
  await comparing.locator('.comparing-sent').last().scrollIntoViewIfNeeded();
  await drawn();
  const drawerSideways = await comparing.evaluate((form) => {
    const body = form.closest('.drawer-body') ?? form.parentElement;
    return [body.scrollWidth - body.clientWidth, body.scrollLeft, ...[...form.querySelectorAll('.comparing-way-sent > *')].map((one) => (one.getBoundingClientRect().right > form.getBoundingClientRect().right + 0.5 ? 1 : 0))];
  });
  expect(`nothing of a way that starts a text played side by side to leave the drawer or scroll it sideways, got ${drawerSideways}`, drawerSideways.every((over) => over <= 0));
  await shot(`sidebyside-3-compare-${scheme}`);
  await comparing.getByLabel('Called', { exact: true }).fill('forks');
  await comparing.getByRole('button', { name: 'Start comparing', exact: true }).click();
  await page.locator('.drawer .comparing').waitFor({ state: 'detached' });
  const comparison = (await asks('GET', '/api/comparisons/forks')).body.comparison;
  expect(`the comparison to have each text as a way, sent, got ${JSON.stringify(comparison?.ways)}`, comparison.project === 'yard' && comparison.ways.length === 2 && comparison.ways.every((one) => one.sent === true && one.blueprint === 'std/board' && !('source' in one)));
  await asks('POST', '/api/comparisons/forks/end', {});

  await view().locator('.beside-column').first().getByRole('button', { name: 'Fork', exact: true }).click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'board' }).first().waitFor();
  const stands = (await asks('GET', '/api/blueprints/stands')).body.own;
  const yardFork = stands.find((one) => one.project === 'yard' && one.name === 'std/board');
  expect(`the yard to have its own fork now, got ${JSON.stringify(stands)}`, !!yardFork);

  // ---- the fork's own page: as it stands, and as it was saved -----------------------
  await head().getByText('checks').waitFor();
  await head().getByRole('tab', { name: 'Text' }).click();
  await page.locator('.cm-content').locator('visible=true').click();
  await page.keyboard.press('Control+f');
  await page.locator('.cm-search, .finder').locator('visible=true').first().waitFor();
  // The step's key is found by what is written there alone: `cards:` is
  // in a sentence of the text too, and which of two matches a search
  // chooses goes by where the caret was put, which a press in the middle of
  // the text leaves to the layout (between the two, it chose the sentence).
  await page.keyboard.type('  cards:');
  await page
    .waitForFunction(() => document.querySelector('.finder-count')?.textContent === '1 of 1', null, { timeout: 8000 })
    .catch(async () => expect(`the step's key to be the one match, got ${await page.locator('.finder-count').innerText()}`, false));
  await page.keyboard.press('Escape');
  await page.keyboard.type('  the_cards:');
  await head().getByText('unsaved').waitFor();
  await head().getByRole('tab', { name: 'Node' }).click();
  await page.locator('.gnode[data-node="main/the_cards"]').locator('visible=true').waitFor();
  await played(() => besideButton().click());
  now = await says();
  // The fork was only just made, so as it is saved it is the default to the letter: one text, said so.
  expect(
    `the text as it stands, as it was saved (which is the default to the letter) and the shop's fork offered, the first two chosen, got ${JSON.stringify(now.texts)}`,
    now.texts.length === 3 &&
      now.texts[0].text.startsWith('As it stands here') &&
      now.texts[1].text.startsWith('As it was saved') &&
      now.texts[1].text.endsWith('the same to the letter as The default') &&
      now.texts[2].text.startsWith('The fork in Shop') &&
      now.texts.map((one) => one.chosen).join() === 'true,true,false',
  );
  expect(`a column for each, got ${now.columns.map((one) => one.heading).join(' | ')}`, now.columns.map((one) => one.heading).join('|') === 'As it stands here|As it was saved');
  expect(`the two to have run two texts, got ${now.columns.map((one) => one.ran).join(' | ')}`, now.columns[0].ran !== now.columns[1].ran);
  expect('the text to be unsaved still: what was played is what is written', (await head().getByText('unsaved').count()) === 1);
  await heldTo(now, 'as it stands and as it was saved');
  await laidOut('for the text as it stands and as it was saved');
  await shot(`sidebyside-4-here-and-saved-${scheme}`);
  const sandboxesOfFork = now.columns.map((one) => one.sandbox);

  // ---- the tabs closed: the daemon holds none of their sandboxes ---------------------
  const closes = page.getByRole('button', { name: /^Close .*board/ });
  for (let left = await closes.count(); left > 0; left = await closes.count()) {
    await closes.first().click();
    // The fork's page holds what was not saved: asked about, it is let go of.
    await page.waitForFunction(
      (was) => [...document.querySelectorAll('button[aria-label^="Close "]')].filter((one) => /board/.test(one.getAttribute('aria-label'))).length < was || [...document.querySelectorAll('button')].some((one) => one.textContent.trim() === 'Discard them'),
      left,
    );
    const discard = page.getByRole('button', { name: 'Discard them', exact: true });
    if ((await discard.count()) > 0) await discard.click();
    await drawn();
  }
  await page.waitForFunction(
    async (ids) => {
      for (const id of ids) {
        const answer = await fetch(`/api/sandboxes/${id}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
        if (answer.status !== 404) return false;
      }
      return true;
    },
    [...sandboxesOfTab, ...sandboxesOfFork],
  );

  // ---- a project's own blueprint, with no fork: one text, and what would make a second --
  const ending = (await asks('GET', '/api/meta')).body.blueprint_ext;
  const greet = `@blueprints/greet.${ending}`;
  const laid = (await asks('POST', '/api/blueprints/format', { source: OWN, project: 'yard' })).body.formatted;
  expect('the daemon to lay the blueprint out', typeof laid === 'string' && laid.includes('greet.said'));
  expect('the project\'s own blueprint to be saved through the daemon', (await asks('PUT', '/api/projects/yard/file', { path: greet, content: laid })).status === 200);
  /** Presses Side by side on a page with one text, and holds what it says: nothing asked of the daemon to play, nothing pending. */
  const alone = async (on, press) => {
    sent = null;
    const was = await answered(on);
    await press();
    await on.waitForFunction((before) => {
      const count = Number([...document.querySelectorAll('.view-head button[data-beside-answered]')].find((one) => one.getClientRects().length > 0)?.dataset.besideAnswered ?? 0);
      const shown = [...document.querySelectorAll('.beside-view')].find((one) => one.getClientRects().length > 0);
      return !!shown && count > before && shown.dataset.stands === 'one' && shown.dataset.busy === 'false';
    }, was);
    await drawn(on);
    const one = await on.evaluate(() => {
      const shown = [...document.querySelectorAll('.beside-view')].find((one) => one.getClientRects().length > 0);
      const text = (el) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();
      return {
        said: text(shown.querySelector('.beside-one > p')),
        seconds: [...shown.querySelectorAll('.beside-seconds li')].map((line) => ({ press: text(line.querySelector('button')), then: text(line.querySelector('span')) })),
        texts: shown.querySelectorAll('.beside-text').length,
        reading: shown.querySelectorAll('.beside-reading, .sandbox-busy').length,
        columns: shown.querySelectorAll('.beside-column').length,
        plays: [...shown.querySelectorAll('button')].filter((one) => one.textContent.trim().startsWith('Play')).length,
      };
    });
    expect(
      `one text to play to be said, got ${one.said}`,
      one.said === 'There is one text of this blueprint to play: as it stands here, which is also as it was saved. Side by side plays the same things against two texts or more, each in a sandbox of its own.',
    );
    expect(`changing the text here to be what makes a second, and the only press, got ${JSON.stringify(one.seconds)}`, one.seconds.length === 1 && one.seconds[0].press === 'Change the text here' && one.seconds[0].then === 'it is then played as it stands here beside as it was saved');
    expect(`nothing pending, nothing to choose and nothing played, got ${JSON.stringify(one)}`, one.texts === 0 && one.reading === 0 && one.columns === 0 && one.plays === 0 && sent === null);
  };
  await page.evaluate((to) => (location.hash = to), `/file/yard/${greet}`);
  await head().getByText('greet', { exact: false }).first().waitFor();
  await head().getByText('checks').waitFor();
  await alone(page, () => besideButton().click());
  for (const width of [1280, 1000, 760]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(`for one text at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await shot(`sidebyside-5-one-text-${scheme}`);
  await view().getByRole('button', { name: 'Change the text here', exact: true }).click();
  await page.locator('.cm-content').locator('visible=true').waitFor();
  expect('the press to show the text, to be changed', (await head().locator('.pills button[aria-pressed="true"]').innerText()).trim() === 'Text');

  // ---- a phone ----------------------------------------------------------------------
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  small = await signIn(phone, { at: '#/blueprint/std/board?project=shop', touch: true });
  listen(small);
  await head(small).getByText('checks').waitFor();
  await played(() => besideButton(small).tap(), small);
  now = await says(small);
  expect(`the default and the forks to be played on a phone, got ${now.columns.map((one) => one.heading).join(' | ')}`, now.columns.length === 2 && now.columns[0].heading === 'The default');
  await laidOut("at a phone's width", small);
  await shot(`sidebyside-6-phone-${scheme}`, small);
  await view(small).locator('.beside-column').nth(1).scrollIntoViewIfNeeded();
  await laidOut("at a phone's width, scrolled to the second column", small);
  await shot(`sidebyside-7-phone-second-${scheme}`, small);
  // One text, at a phone's width.
  await small.evaluate((to) => (location.hash = to), `/file/yard/${greet}`);
  await head(small).getByText('greet', { exact: false }).first().waitFor();
  await head(small).getByText('checks').waitFor();
  await alone(small, () => besideButton(small).tap());
  await laidOut("for one text at a phone's width", small);
  await shot(`sidebyside-8-phone-one-text-${scheme}`, small);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/sidebyside-0-failed.png` }).catch(() => {});
  await small?.screenshot({ path: `${out}/sidebyside-0-failed-phone.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop, yard]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
