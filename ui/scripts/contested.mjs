// A card's A/B Testing tab, in its drawer.
//
//   DAEMON=../target/debug/factory node scripts/contested.mjs [out-dir]
//
// The shop's fleet has two roles that each write a change (`otter` and
// `slate`), a judge and a reviewer, each a stand-in of a few lines of shell.
// A/B tests are given to the project from the gear of its board's page,
// as a person gives them. A card's drawer then has a tab for them beside
// the card itself, called what `std/ab` called it: a test begun there
// from `+ New contest` opens whole in the drawer, and is set up from a
// terminal. Made ready, the card is worked by both at once, judged, the
// winner's work reviewed and merged. The tab then says, from what the
// daemon keeps and nothing worked out in the page: the test and what it
// came to; each contender's entry with its state, what the judge said of
// it, that it could see the other contenders' work (this factory holds no
// walls, so none was held apart: it is never left unsaid), and its
// branch, which opens to say where it is where the repository has it.
// A second card is then given a shadow test, set up from a terminal: the
// usual contender (`otter`) its incumbent and `slate` its challenger, judged
// by their ways, the judge taking the challenger. The tab says it is a
// shadow test and which entry is the incumbent, whose work was the one
// merged, and that the judge took the other, whose branch is kept.
// A third card's judge decides nothing, and its test asks a person: the
// question is said on the card naming whoever applied the fleet, says
// where what it names is marked and leads there, the card's A/B testing
// tab, where the test that asks and each attempt it names are marked.
// Nothing of the tab lies over anything else, leaves the drawer or
// scrolls sideways at three widths and a phone's. The attempts' own table
// says its rows are written by the test, offers nothing that adds or
// changes one, and calls each by its card and contender in the column of
// its title too. The environment
// variables are those of walkthrough.mjs; `LOOK` is worn as every check
// wears it.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { linesOf, openGear } from './gear.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'contested-data');
const home = join(out, 'contested-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };

// Each stand-in reads what it is asked and answers: a contender changes a file of its own and says so, the
// judge picks A (and of the shadow test's card, `slate`), the reviewer approves.
// What each says is one line of JSON, as an agent's program says what it answered.
const said = (text) => `printf '%s\\n' '${JSON.stringify({ text })}'`;
const stand = (name, does, reads = 'cat >/dev/null') => [name, join(home, `${name}.sh`), `#!/bin/sh\n${reads}\n${does}\n`];
const picks = `case "$asked" in\n*'Add a till'*) ${said('slate does what the card asks; otter less plainly.\nDECISION: slate')} ;;\n*'Add a lamp'*) ${said('Both are as good as each other.\nDECISION: neither')} ;;\n*) ${said('A does what the card asks; B does too, less plainly.\nDECISION: A')} ;;\nesac`;
const stands = [
  stand('maker', `echo changed >> "CHANGES-$$.md"\n${said('I changed a file.')}`),
  stand('judge', picks, 'asked=$(cat)'),
  stand('looker', said('It does what the card asks.\nDECISION: approve')),
];
const PACKAGE = (name, program) => `extension: ${name}
title: ${name}
description: A stand-in.
agent:
  program: ${program}
  input: \${ turn.prompt }
  output:
    - when: \${ event.text != null }
      reply: \${ event.text }
`;

const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const json = async (...args) => JSON.parse((await cli(...args, '--json')).stdout);

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  for (const [name, program, text] of stands) {
    await writeFile(program, text);
    await chmod(program, 0o755);
    await mkdir(join(home, name), { recursive: true });
    await writeFile(join(home, name, 'extension.yaml'), PACKAGE(name, program));
    await cli('extension', 'add', join(home, name));
  }
  // A repository, so that each contender works on a branch of its own.
  const shop = join(home, 'shop');
  await mkdir(shop, { recursive: true });
  await writeFile(join(shop, 'README.md'), '# Shop\n');
  const git = (...args) => run('git', args, { cwd: shop, env });
  await git('init', '--quiet', '.');
  await git('config', 'user.name', 'Ada');
  await git('config', 'user.email', 'ada@example.com');
  await git('add', '-A');
  await git('commit', '--quiet', '-m', 'first');
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('record', 'set', 'shop-agents-3', 'adapter=looker', 'count=1');
  for (const [role, adapter] of [
    ['otter', 'maker'],
    ['slate', 'maker'],
    ['judge', 'judge'],
  ]) {
    await cli('record', 'add', 'shop', 'agents', `title=${role}`, `role=${role}`, 'count=1', `adapter=${adapter}`, 'permission=full');
  }
  await cli('board', 'add', 'shop', 'Add a cart', '--body', 'Customers need one.');

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
  const signedIn = async (how) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const page = await signedIn({ viewport: { width: 1280, height: 860 } });
  const board = async (on, touch = false) => {
    const press = (locator) => (touch ? locator.tap() : locator.click());
    if (touch) await press(on.getByRole('button', { name: 'Expand the side panel' }));
    await sideSettled(on);
    const resources = on.getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await press(resources);
    await unfoldAll(on);
    await press(on.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }));
  };
  await board(page);

  // ── Given from the gear of the board's page, as a person gives them ──
  const menu = await openGear(page);
  await menu.getByRole('menuitemcheckbox', { name: 'A/B testing', exact: true }).click();
  await menu.locator('[role="menuitemcheckbox"][aria-checked="true"]', { hasText: 'A/B testing' }).waitFor({ timeout: 30000 });
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });

  // ── The card's drawer has a tab for its tests, called what std/ab called it ──
  const card = page.locator('.card', { hasText: 'Add a cart' });
  await card.click();
  const drawer = drawerAtRest(page);
  const tab = drawer.getByRole('tab', { name: 'A/B testing' });
  await tab.waitFor({ timeout: 20000 });
  const named = (await drawer.getByRole('tab').allInnerTexts()).map((one) => one.trim());
  expect(`Details, Comments, History and Attachments first, then the tests, in one row, got ${JSON.stringify(named)}`, named.join('|') === 'Details|Comments|History|Attachments|A/B testing');
  await tab.click();
  await drawer.getByText('No test of this card yet.').waitFor();
  await page.screenshot({ path: `${out}/contested-1-empty.png` });
  // Begun from the tab, it opens whole in the drawer, with the way back to the card.
  const arranged = async () => (await cli('events', '--topics', 'fleet.arranged', '--limit', '100', '--json')).stdout.trim().split('\n').filter(Boolean).length;
  const was = await arranged();
  await drawer.getByRole('button', { name: '+ New test' }).click();
  const tests = await json('record', 'list', 'shop', 'contests');
  const begun = (tests.records ?? tests).find((row) => row.data.card === 'shop-1');
  expect(`a test of the card begun from its tab, got ${JSON.stringify(tests)}`, begun !== undefined);
  await cli('record', 'set', begun.id, 'contenders=otter,slate', 'judges=judge');
  await page.keyboard.press('Escape');
  // Laid into the card's list, and the fleet has taken that: a card made ready before it has is worked as any card.
  for (let tries = 0; (await arranged()) < was + 1; tries += 1) {
    if (tries > 300) throw new Error("the fleet never took the test into the card's list");
    await new Promise((later) => setTimeout(later, 200));
  }

  // ── Worked both ways, judged, the winner reviewed and merged ──
  await cli('board', 'move', 'shop-1', 'ready');
  const done = page.locator('.column', { hasText: 'done' }).locator('.card', { hasText: 'Add a cart' });
  // Where it never gets there, what the repository, the card, the test and its entries came to is said
  // with the failure: the daemon and its data are gone once the check has ended.
  await done.waitFor({ timeout: 120000 }).catch(async (err) => {
    const shown = async (...args) => (await cli(...args, '--json')).stdout;
    console.error(`the repository's branches:\n${(await git('branch', '-a')).stdout}${(await git('worktree', 'list')).stdout}`);
    console.error(`the card: ${await shown('record', 'show', 'shop-1')}`);
    for (const table of ['contests', 'segments', 'entries']) console.error(`${table}: ${await shown('record', 'list', 'shop', table)}`);
    throw err;
  });
  await done.click();
  const opened = drawerAtRest(page);
  await opened.getByRole('tab', { name: 'A/B testing' }).click();
  const test = opened.locator(`.about-card .about-card-row[data-row="${begun.id}"]`);
  await test.waitFor();
  const entries = (await json('record', 'list', 'shop', 'entries')).records ?? [];
  expect(`two entries, got ${JSON.stringify(entries)}`, entries.length === 2);
  // What the test came to, as its row says.
  const testSays = await test.locator(':scope > .about-card-cells').innerText();
  expect(`the test to say it came, and its winner, got ${testSays}`, testSays.includes('came') && /winner\s+(otter|slate)/.test(testSays));
  for (const entry of entries) {
    const line = test.locator(`.about-card-named[data-row="${entry.id}"]`);
    await line.waitFor();
    const says = await line.innerText();
    expect(`${entry.id} said with its state and its verdict, got ${says}`, says.includes(entry.data.state) && says.includes(entry.data.verdict));
    // Never silent: it could see the other's work, and that is said.
    expect(`${entry.id} said to be not held apart, got ${says}`, says.includes('not held apart'));
    const scores = entry.data.scores ?? [];
    if (scores.length > 0) expect(`what the judge said of ${entry.id}, got ${says}`, says.includes('B does too, less plainly'));
    // Called by its card and its contender, and never by what the test finds it by.
    const calledAs = (await line.locator('.about-card-name').innerText()).trim();
    expect(`${entry.id} called by its card and contender, got ${calledAs}`, calledAs === `shop-1 · ${entry.data.way}`);
  }
  const testCalled = (await test.locator(':scope > .about-card-head strong').first().innerText()).trim();
  expect(`the test called by its card and not its id, got ${testCalled}`, testCalled === 'shop-1');
  const under = (await test.locator('.about-card-of').first().textContent()).trim();
  expect(`what the attempts are under called what the table is called, got ${under}`, under === 'A/B attempts');
  // The loser's branch is kept, and opens to say where it is.
  const lost = entries.find((entry) => entry.data.verdict === 'lost');
  const branch = test.locator(`.about-card-named[data-row="${lost.id}"] .about-card-branch button`);
  await branch.waitFor();
  expect(`the loser's branch drawn as itself, got ${await branch.innerText()}`, (await branch.innerText()) === lost.data.branch);
  await branch.click();
  const commit = test.locator(`.about-card-named[data-row="${lost.id}"] .about-card-commit`);
  await commit.waitFor();
  expect(`where its branch is, got ${await commit.innerText()}`, (await commit.innerText()).includes(`shop-1: ${lost.data.way}`));
  await page.screenshot({ path: `${out}/contested-2-judged.png` });

  // ── Nothing over anything else, at three widths and a phone's ──
  const laidOut = async (on, what) => {
    const wrong = await on.evaluate(() => {
      const said = [];
      const tab = document.querySelector('.drawer .about-card');
      if (!tab) return ['there is no tab of the card\'s tests'];
      const edge = (tab.closest('.drawer-body') ?? tab).getBoundingClientRect();
      const boxes = (selector) => [...tab.querySelectorAll(selector)].map((one) => ({ one, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const apart = (group, what) => {
        for (const [index, { one, box }] of group.entries()) {
          if (box.left < edge.left - 0.5 || box.right > edge.right + 0.5) said.push(`${what} ${one.textContent.trim().slice(0, 30)} leaves the drawer`);
          for (const other of group.slice(index + 1)) {
            if (one.contains(other.one) || other.one.contains(one)) continue;
            const over = Math.min(box.right, other.box.right) - Math.max(box.left, other.box.left) > 0.5 && Math.min(box.bottom, other.box.bottom) - Math.max(box.top, other.box.top) > 0.5;
            if (over) said.push(`${what}: ${one.textContent.trim().slice(0, 30)} lies over ${other.one.textContent.trim().slice(0, 30)}`);
          }
        }
      };
      apart(boxes('.about-card-head > *'), 'in a head');
      for (const cells of tab.querySelectorAll('.about-card-cells')) {
        apart([...cells.children].map((one) => ({ one, box: one.getBoundingClientRect() })), 'a cell');
        for (const cell of cells.children) apart([...cell.children].map((one) => ({ one, box: one.getBoundingClientRect() })), 'in a cell');
      }
      apart(boxes('.about-card-named'), 'an entry');
      // The row of tabs is inside the drawer and scrolls in itself where it has no room: a tab is seen as far as the row.
      const tabs = document.querySelector('.drawer .card-tabs [role="tablist"]');
      if (tabs) {
        const row = tabs.getBoundingClientRect();
        if (row.left < edge.left - 0.5 || row.right > edge.right + 0.5) said.push('the row of tabs leaves the drawer');
        const seen = (box) => {
          const [left, right] = [Math.max(box.left, row.left), Math.min(box.right, row.right)];
          return { left, right, top: box.top, bottom: box.bottom, width: right - left };
        };
        apart([...tabs.children].map((one) => ({ one, box: seen(one.getBoundingClientRect()) })).filter((one) => one.box.width > 0), 'a tab');
      }
      const scroller = tab.closest('.drawer-body') ?? tab.closest('.drawer');
      if (scroller && scroller.scrollWidth > scroller.clientWidth) said.push(`the drawer scrolls ${scroller.scrollWidth - scroller.clientWidth}px sideways`);
      if (document.documentElement.scrollWidth > window.innerWidth) said.push('the page scrolls sideways');
      return said;
    });
    expect(`nothing of the tab over anything else ${what}, got ${wrong.join('; ')}`, wrong.length === 0);
    const crowded = await pageCrowded(on, '.drawer .about-card');
    expect(`nothing of it crowded ${what}, got ${crowded.join('; ')}`, crowded.length === 0);
  };
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `at ${width} wide`);
  }
  await page.screenshot({ path: `${out}/contested-3-narrow.png` });

  // ── A shadow test: the incumbent's work merged though the judge takes the challenger ──
  await opened.getByRole('button', { name: 'Close', exact: true }).click();
  await opened.waitFor({ state: 'detached' });
  await page.setViewportSize({ width: 1280, height: 860 });
  await cli('board', 'add', 'shop', 'Add a till', '--body', 'The shop needs one.');
  const cards = await json('record', 'list', 'shop', 'board');
  const till = (cards.records ?? cards).find((row) => row.data.title === 'Add a till').id;
  const before = await arranged();
  await cli('record', 'add', 'shop', 'contests', `card=${till}`, 'contenders=slate', 'shadow=otter', 'judges=judge', 'known_by=ways');
  for (let tries = 0; (await arranged()) < before + 1; tries += 1) {
    if (tries > 300) throw new Error("the fleet never took the shadow test into the card's list");
    await new Promise((later) => setTimeout(later, 200));
  }
  await cli('board', 'move', till, 'ready');
  const tillDone = page.locator('.column', { hasText: 'done' }).locator('.card', { hasText: 'Add a till' });
  await tillDone.waitFor({ timeout: 120000 });
  await tillDone.click();
  const shadowed = drawerAtRest(page);
  await shadowed.getByRole('tab', { name: 'A/B testing' }).click();
  const shadowTests = await json('record', 'list', 'shop', 'contests');
  const shadowTest = (shadowTests.records ?? shadowTests).find((row) => row.data.card === till);
  const shadowRow = shadowed.locator(`.about-card .about-card-row[data-row="${shadowTest.id}"]`);
  await shadowRow.waitFor();
  // The tab says it is a shadow test, of which incumbent, and that the incumbent's work went on.
  const shadowSays = await shadowRow.locator(':scope > .about-card-cells').innerText();
  expect(`the test to say it is a shadow test of otter, got ${shadowSays}`, /shadow\s+otter/.test(shadowSays));
  expect(`the incumbent's work taken, got ${shadowSays}`, /winner\s+otter/.test(shadowSays) && shadowSays.includes('a shadow test') && shadowSays.includes('the judges took slate'));
  const shadowEntries = ((await json('record', 'list', 'shop', 'entries')).records ?? []).filter((entry) => entry.data.contest === shadowTest.id);
  expect(`two entries of the shadow test, got ${JSON.stringify(shadowEntries)}`, shadowEntries.length === 2);
  for (const entry of shadowEntries) {
    const says = await shadowRow.locator(`.about-card-named[data-row="${entry.id}"]`).innerText();
    const incumbent = entry.data.way === 'otter';
    expect(`${entry.id} said to be the incumbent or not, got ${says}`, says.includes('incumbent') === incumbent);
    expect(`${entry.id} said with what the judge took, got ${says}`, says.includes(incumbent ? 'lost' : 'won'));
  }
  // The challenger the judge took is never the card's: its branch is kept beside it.
  const challenger = shadowEntries.find((entry) => entry.data.way === 'slate');
  const kept = shadowRow.locator(`.about-card-named[data-row="${challenger.id}"] .about-card-branch button`);
  expect(`the challenger's branch kept, got ${await kept.innerText()}`, (await kept.innerText()) === challenger.data.branch);
  await page.screenshot({ path: `${out}/contested-5-shadow.png` });
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `with a shadow test at ${width} wide`);
  }

  // ── A test the judge decides nothing of asks a person ──
  // Its question is said on the card for the test, by the factory's bot, naming whoever applied the fleet, says that what
  // it names is marked in the card's A/B testing tab, and leads there: the test that asks and each attempt its question
  // names are marked.
  await shadowed.getByRole('button', { name: 'Close', exact: true }).click();
  await shadowed.waitFor({ state: 'detached' });
  await page.setViewportSize({ width: 1280, height: 860 });
  await cli('board', 'add', 'shop', 'Add a lamp', '--body', 'The shop needs one.');
  const lamp = ((await json('record', 'list', 'shop', 'board')).records ?? []).find((row) => row.data.title === 'Add a lamp').id;
  const ahead = await arranged();
  await cli('record', 'add', 'shop', 'contests', `card=${lamp}`, 'contenders=otter,slate', 'judges=judge');
  for (let tries = 0; (await arranged()) < ahead + 1; tries += 1) {
    if (tries > 300) throw new Error("the fleet never took the lamp's test into the card's list");
    await new Promise((later) => setTimeout(later, 200));
  }
  await cli('board', 'move', lamp, 'ready');
  const lampCard = page.locator('.column', { hasText: 'doing' }).locator('.card', { hasText: 'Add a lamp' });
  await lampCard.locator('.badge', { hasText: 'question' }).waitFor({ timeout: 120000 });
  await lampCard.click();
  const asking = drawerAtRest(page);
  await asking.getByRole('button', { name: /^Waiting on an answer from @ada/ }).waitFor();
  await asking.getByRole('tab', { name: /^Comments/ }).click();
  const question = asking.locator('.journey-spoken', { hasText: 'Which entry is taken?' });
  await question.waitFor();
  // Whose it is, the test's, is read a moment after the line.
  await question.locator('.journey-for', { hasText: 'the contest of' }).waitFor();
  const questionSays = await question.innerText();
  expect(`the question said for the test, naming Ada and saying where what it names is marked, got ${questionSays}`, questionSays.includes('for the contest of') && questionSays.includes('@ada') && questionSays.includes("Each is marked in the card's A/B testing tab."));
  await question.locator('.journey-tab').click();
  await asking.locator('[role="tab"][aria-selected="true"]', { hasText: 'A/B testing' }).waitFor();
  const lampTest = ((await json('record', 'list', 'shop', 'contests')).records ?? []).find((row) => row.data.card === lamp);
  const asks = asking.locator(`.about-card .about-card-row[data-row="${lampTest.id}"]`);
  await asks.waitFor();
  expect('the test that asks marked as asking', (await asks.getAttribute('class')).includes('about-card-asks') && (await asks.locator('.about-card-asking', { hasText: 'asks' }).count()) === 1);
  const lampEntries = ((await json('record', 'list', 'shop', 'entries')).records ?? []).filter((entry) => entry.data.contest === lampTest.id);
  expect(`two entries of the lamp's test, got ${JSON.stringify(lampEntries)}`, lampEntries.length === 2);
  for (const entry of lampEntries) {
    await asks.locator(`.about-card-named[data-row="${entry.id}"][data-asked="true"]`).locator('.about-card-asking').waitFor();
  }
  await page.screenshot({ path: `${out}/contested-8-asks.png` });
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `with a test that asks at ${width} wide`);
  }
  await asking.getByRole('button', { name: 'Close', exact: true }).click();
  await asking.waitFor({ state: 'detached' });

  // ── Each table called what std/ab calls it, its rows counted in its words, and what it is for under its head ──
  await page.setViewportSize({ width: 1280, height: 860 });
  await openGear(page);
  const gearSays = await linesOf(page);
  expect(`the tests beside the board as the A/B tests, counted as tests, got ${JSON.stringify(gearSays)}`, gearSays.includes('A/B tests 3 tests'));
  await page.keyboard.press('Escape');
  await page.goto(`${url}/#/table/shop/entries`);
  const head = page.locator('.view-head').first();
  await head.getByRole('heading', { name: /A\/B attempts$/ }).waitFor();
  await head.getByText(/^\d+ attempts?$/).waitFor();
  const headSays = (await head.innerText()).replace(/\s+/g, ' ');
  expect(`the attempts counted as attempts, got ${headSays}`, headSays.includes('6 attempts'));
  // Written by the test and nobody by hand: the head says so, and nothing here adds a row or changes one.
  await head.locator('.table-kept', { hasText: 'written by what keeps it' }).waitFor();
  expect('no + New at the foot of the attempts', (await page.locator('.grid-new').count()) === 0 && (await page.locator('table.grid tbody button.cell').count()) === 0);
  // Each called by its card and contender in the column of its title too, and never by what the test finds it by.
  const firstColumn = (await page.locator('table.grid tbody tr td.grid-title .cell-words').allInnerTexts()).map((one) => one.trim());
  expect(`each attempt called by its card and contender in its first column, got ${JSON.stringify(firstColumn)}`, firstColumn.length === 6 && firstColumn.every((one) => /^shop-\d+ · (otter|slate)$/.test(one)));
  const forWhat = head.locator('.table-said');
  expect(`what the attempts are for said under the head, got ${await forWhat.innerText()}`, (await forWhat.innerText()).startsWith('What each contender of an A/B test did'));
  for (const width of [1280, 1000, 820, 390]) {
    await page.setViewportSize({ width, height: 860 });
    const wrong = await head.evaluate((one) => {
      const said = one.querySelector('.table-said').getBoundingClientRect();
      const edge = one.getBoundingClientRect();
      const over = [...one.children].filter((other) => !other.classList.contains('table-said')).map((other) => other.getBoundingClientRect()).filter((box) => box.width > 0 && Math.min(box.bottom, said.bottom) - Math.max(box.top, said.top) > 0.5 && Math.min(box.right, said.right) - Math.max(box.left, said.left) > 0.5);
      return [...(over.length ? ['it lies over the head'] : []), ...(said.left < edge.left - 0.5 || said.right > edge.right + 0.5 ? ['it leaves the head'] : []), ...(document.documentElement.scrollWidth > innerWidth ? ['the page scrolls sideways'] : [])];
    });
    expect(`what the table is for over nothing at ${width} wide, got ${wrong.join('; ')}`, wrong.length === 0);
    await page.screenshot({ path: `${out}/contested-7-attempts-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await board(phone, true);
  await phone.locator('.column', { hasText: 'done' }).locator('.card', { hasText: 'Add a cart' }).tap();
  const small = drawerAtRest(phone);
  await small.getByRole('tab', { name: 'A/B testing' }).tap();
  await small.locator(`.about-card .about-card-row[data-row="${begun.id}"] .about-card-named`).first().waitFor();
  await laidOut(phone, "at a phone's width");
  await phone.screenshot({ path: `${out}/contested-4-phone.png` });
  // The shadow test at a phone's width too.
  await small.getByRole('button', { name: 'Close', exact: true }).tap();
  await small.waitFor({ state: 'detached' });
  await phone.locator('.column', { hasText: 'done' }).locator('.card', { hasText: 'Add a till' }).tap();
  const smallShadow = drawerAtRest(phone);
  await smallShadow.getByRole('tab', { name: 'A/B testing' }).tap();
  await smallShadow.locator(`.about-card .about-card-row[data-row="${shadowTest.id}"] .about-card-named`).first().waitFor();
  await laidOut(phone, "with a shadow test at a phone's width");
  await phone.screenshot({ path: `${out}/contested-6-shadow-phone.png` });

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
