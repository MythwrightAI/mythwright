// What a blueprint told shown with parts beside its words, on the line's
// own page: a link, and the cards of a column of a board, each passed or
// failed there.
//
//   DAEMON=../target/debug/factory node scripts/reviewed.mjs [out-dir]
//
// Ada owns the factory and applied the fleet of a project called
// Storefront (so that a card's id is a long one), whose one
// worker is a stand-in that writes down what it is asked and says it did the work,
// and whose work goes to `staging` and stays there. Two cards are made
// ready from a terminal, and once both are worked into staging a
// blueprint tells her, and Grace, with a link and the cards in staging,
// which go to done when passed and back to ready when failed. Her page is
// opened once and never read again: the line pressed opens its page, which
// lists the two cards it was told of, under the cards that were in
// staging, each with whom it is given to, its column, the last thing said
// on it, its thread and Pass and Fail. Something said in a card's thread
// is said on the card and is in its thread. One card passed is approved
// by her, a line of its history with no comment written, and goes to
// done where the part says, said in a line; and it is still listed, as in
// done, with nothing to press, its row by the boxes at three widths. Fail asks why, and is closed saying so until something
// is written; failed with why, the card goes back to ready with why in
// its history, is worked again by its agent, which is shown why and what
// was said in its thread, and comes back to staging, offered Pass and
// Fail again on the page with nothing read again. Something said after that, and the card
// failed again, the stand-in is shown that on its next turn and not what
// it was shown before. Grace, as somebody who may not change the board
// (the parts read with a credential that holds less), is shown the cards
// and offered no Pass, no Fail and no box to say anything in, and that
// credential is refused by the daemon. And the page by its boxes at
// 2400, 1820, 1280 and 1100 wide and a phone's, the side panel open,
// nothing of a card's row over anything else or past the page, and the
// line's frame filling its tab with the room at either side the same and
// its words a reader's width: the cards a table, a row a card with its
// cells side by side (its id, its title, whom it is given to, its column,
// the last thing said), each centred on the row and the buttons in the
// last, and where the part is narrow (a phone) a card's cells under its
// id and title and the buttons under them. A card's title pressed opens it in the drawer
// beside the page, its own form and history, with the page staying. A
// card's id is one line at every width; its title and the last thing
// said on it are each one line where the cards are a table, a long one
// cut with an ellipsis and said whole by the tip where it is pointed at,
// and wrap between their words where they stand under it. The second
// card is given to the fleet's worker, whose name is whole wherever the
// cards are a table, the tip saying it too, and says something in its
// body, which the page never shows. With `LOOK="size=largest
// density=compact"` the same at that look, stacked at 1100. Its parts say no tab, so no row of tabs is drawn over
// them; a told line's page in tabs, with rows of a table, runs and measurements, is `toldtabs.mjs`, which it runs
// last. The environment variables are those of walkthrough.mjs.
import { execFile, spawn, spawnSync } from 'node:child_process';
import { chmod, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { fleetTook } from './fleettook.mjs';
import { LOOKED } from './looked.ts';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'reviewed-data');
const home = join(out, 'reviewed-home');
const hers = join(out, 'reviewed-home-grace');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// Writes down what it is asked, each turn under a line of its own, and says it did the work.
const asked = join(home, 'asked.txt');
const writes = join(home, 'writes.sh');
await writeFile(writes, `#!/bin/sh\necho '=== a turn' >> '${asked}'\ncat >> '${asked}'\necho 'I changed what the card asks.'\necho 'DECISION: done'\n`);
await chmod(writes, 0o755);
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--command', `writes=${writes}`], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const grace = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: { ...env, HOME: hers, XDG_CONFIG_HOME: join(hers, 'config') } });
// The turns the stand-in was given so far, oldest first.
const turns = async () => (await readFile(asked, 'utf8').catch(() => '')).split('=== a turn').slice(1);

// The second card's title, longer than its cell has room for where the cards are a table.
const LONG = "Add a shelf of the season's books beside the cart, with a price on each and the last copy of one marked so that nobody buys it twice";
// What its body says, which the page never shows.
const BODY = 'The shelf is in the body of the card and nowhere on the page.';
const WORKER = 'storefront-worker-1';
// The widths the page is held at, the side panel open: two wide windows, where such a line is often read, and
// narrower; the cards a table at each, and at the largest size a table but at the narrowest.
const WIDTHS = [2400, 1820, 1280, 1100];
const TABLE = LOOKED ? [2400, 1820, 1280] : WIDTHS;

const TOLD = `blueprint: preview-told
permissions: [inbox]
steps:
  told:
    do: inbox.tell
    with:
      to: [applier, "@grace"]
      says: The preview is ready to try.
      why: preview
      parts:
        - { kind: links, links: ["http://127.0.0.1:7760/"] }
        - { kind: cards, column: staging, pass_to: done, fail_to: ready }
`;

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'grace@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await grace('register', '--email', 'grace@example.com', '--name', 'Grace', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Storefront');
  await cli('apply', 'std/board', '--project', 'storefront', '--param', 'columns=["backlog","ready","doing","review","staging","done"]');
  await cli('fleet', 'init', 'storefront');
  await cli('apply', 'fleet', '--project', 'storefront');
  // The worker is the stand-in, and its work goes to staging, which nothing looks over.
  const rows = async (table) => JSON.parse((await cli('record', 'list', 'storefront', table, '--json')).stdout).records;
  const called = (all, title) => all.find((row) => row.data.title === title).id;
  await cli('record', 'set', called(await rows('agents'), 'worker'), 'adapter=writes', 'wall=off');
  const stages = await rows('stages');
  await cli('record', 'set', called(stages, 'work'), 'pass=staging');
  await cli('record', 'remove', called(stages, 'review'));
  // The fleet takes the change of its rows: a card made ready before it has would go on to review.
  await fleetTook(cli, 'storefront');
  await cli('board', 'add', 'storefront', 'Fix the cart');
  await cli('board', 'add', 'storefront', LONG);
  await cli('board', 'move', 'storefront-1', 'ready');
  await cli('board', 'move', 'storefront-2', 'ready');
  // The line keeps the cards staging holds as it is told: both, once each is worked there.
  for (let tries = 0; ; tries += 1) {
    const items = JSON.parse((await cli('board', 'storefront', '--json')).stdout).items;
    if (['storefront-1', 'storefront-2'].every((id) => items.find((one) => one.id === id)?.status === 'staging')) break;
    if (tries > 450) throw new Error(`the cards never came to staging: ${items.map((one) => `${one.id} ${one.status}`).join(', ')}`);
    await new Promise((later) => setTimeout(later, 200));
  }
  // Whom it is given to, by a name that wants room in its cell, and what its body says; staging looks at neither.
  await cli('board', 'give', 'storefront-2', WORKER);
  await cli('board', 'set', 'storefront-2', `body=${BODY}`);
  const told = join(home, 'told.bp');
  await writeFile(told, TOLD);
  await cli('apply', told, '--project', 'storefront');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }, prepare = async () => {}) => {
    const context = await browser.newContext({ ...how, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.setDefaultTimeout(30_000);
    page.on('pageerror', (err) => problems.push(err.message));
    await prepare(page);
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.panes').waitFor();
    return page;
  };
  // The page of the line that says the preview is ready, opened from the box.
  const opened = async (page, how = 'click') => {
    const tray = page.locator('.rail .rail-inbox').first();
    if (how === 'tap') {
      await page.getByRole('button', { name: 'Expand the side panel' }).tap();
      await sideSettled(page);
      await page.locator('.rail .rail-inbox').first().tap();
    } else await tray.click();
    const box = page.getByRole('navigation', { name: 'Inbox' });
    const line = box.locator('.inbox-line', { hasText: 'The preview is ready to try.' });
    await line.locator('.inbox-press')[how]({ position: { x: 24, y: 10 } });
    const shown = page.locator('.inbox-page');
    await shown.locator('.inbox-page-words', { hasText: 'The preview is ready to try.' }).waitFor();
    return shown;
  };
  // The boxes of what `selector` picks inside each of `within`: none lies over another, and each is inside the page.
  const apart = async (page, within, selector, what) => {
    const found = await page.evaluate(
      ([outer, picked]) => {
        // Inside the frame the line's message is drawn in, which the parts are part of.
        const edge = (document.querySelector('.inbox-page-message') ?? document.querySelector('.inbox-page')).getBoundingClientRect();
        const wrong = [];
        let count = 0;
        for (const frame of document.querySelectorAll(outer)) {
          const boxes = [...frame.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
          count += boxes.length;
          for (const [index, one] of boxes.entries()) {
            if (one.box.left < edge.left - 0.5 || one.box.right > edge.right + 0.5) wrong.push(`${one.said} leaves the page`);
            for (const two of boxes.slice(index + 1)) {
              const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
              if (over) wrong.push(`${one.said} lies over ${two.said}`);
            }
          }
        }
        const page = document.querySelector('.inbox-page');
        return { count, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth, within: page.scrollWidth - page.clientWidth };
      },
      [within, selector],
    );
    expect(`${what}: something to be there`, found.count > 1);
    expect(`${what}: nothing over anything else or outside the page, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px of the page and ${found.within}px of itself`, found.sideways <= 0 && found.within <= 0);
  };
  const ROW = '.told-card-id, .told-card-title, .told-card-who, .told-card-column, .told-card-last, .told-card-acts button, .told-card-now';
  const PARTS = '.inbox-page-words, .told-part > .inbox-page-label, .told-goes, .told-links a, .told-table';
  // A card's cells: side by side and each centred on the row where the table has its heads, and where it is
  // narrow, its id and title on one line and the rest under them in the order they are, the buttons last. Which of
  // the two is by the part's width.
  const CELLS = ['.told-card-id', '.told-card-title', '.told-card-who', '.told-card-column', '.told-card-last', '.told-card-acts'];
  const shaped = async (page, what, wanted) => {
    const found = await page.evaluate((cells) => {
      const wide = getComputedStyle(document.querySelector('.told-table thead')).display !== 'none';
      const wrong = [];
      // Where a cell's first line of words stands: the top of its letters, whatever box they are drawn in.
      const firstLine = (row, cell) => {
        const walker = document.createTreeWalker(row.querySelector(cell), NodeFilter.SHOW_TEXT);
        for (let text = walker.nextNode(); text; text = walker.nextNode()) {
          const range = document.createRange();
          range.selectNodeContents(text);
          const box = [...range.getClientRects()].find((one) => one.width > 0);
          if (box) return box.top;
        }
        return Number.NaN;
      };
      for (const row of document.querySelectorAll('.told-card-row')) {
        const box = row.getBoundingClientRect();
        const middle = box.top + box.height / 2;
        const inner = cells.map((cell) => {
          const range = document.createRange();
          const td = row.querySelector(cell);
          range.selectNodeContents(td);
          // What is drawn of it: a title cut at its cell's edge goes on past it, unseen.
          const box = range.getBoundingClientRect();
          const edge = td.getBoundingClientRect();
          return { cell, box: { top: box.top, bottom: box.bottom, height: box.height, left: Math.max(box.left, edge.left), right: Math.min(box.right, edge.right) } };
        });
        for (const [at, one] of inner.entries()) {
          if (wide && Math.abs(one.box.top + one.box.height / 2 - middle) > 1.5) wrong.push(`${one.cell} is not centred on its row`);
          if (!wide && at === 1 && (one.box.left < inner[0].box.right - 0.5 || Math.abs(firstLine(row, cells[1]) - firstLine(row, cells[0])) > 1.5)) wrong.push(`${one.cell} is not beside ${inner[0].cell} on its first line`);
          if (!wide && at > 1 && one.box.top < Math.max(inner[0].box.bottom, inner[at - 1].box.bottom) - 0.5) wrong.push(`${one.cell} is not under ${inner[at - 1].cell}`);
        }
        if (wide && inner.some((one, at) => at > 0 && one.box.left < inner[at - 1].box.right - 0.5)) wrong.push('the cells are not side by side');
      }
      return { wide, wrong };
    }, CELLS);
    if (wanted !== undefined) expect(`the table ${what} to be ${wanted ? 'a row a card' : 'a card under its title'}`, found.wide === wanted);
    expect(`a card's cells ${what}, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
  };
  // A card's id is one line, whatever wraps beside it: its title does (an id broke mid-word over three lines).
  const whole = async (page, what) => {
    // Counted by the lines its letters stand on, which a span drawn as a block of its own does not say of itself.
    const lines = await page.locator('.told-card-id').evaluateAll((all) =>
      all.map((id) => {
        const range = document.createRange();
        range.selectNodeContents(id);
        const tops = new Set([...range.getClientRects()].filter((box) => box.width > 0).map((box) => Math.round(box.top)));
        return { id: id.textContent, lines: tops.size };
      }),
    );
    expect(`cards' ids ${what}`, lines.length > 0);
    // Its title: where the cards are a table, one line, cut with an ellipsis where it does not fit, its cell no
    // narrower than a word or two, and said whole to whoever points at it; where its cells stand under it, wrapping
    // between its words, never a letter at a time. The last thing said on it the same.
    const titles = await page.locator('.told-card-open .told-card-words, .told-card-said').evaluateAll((all) =>
      all.map((words) => {
        const range = document.createRange();
        range.selectNodeContents(words);
        const tops = new Set([...range.getClientRects()].filter((box) => box.width > 0).map((box) => Math.round(box.top)));
        const style = getComputedStyle(words);
        return {
          title: words.textContent,
          lines: tops.size,
          words: words.textContent.trim().split(/\s+/).length,
          cut: words.scrollWidth > words.clientWidth + 0.5,
          ellipsis: style.textOverflow === 'ellipsis',
          ems: words.parentElement.closest('td').getBoundingClientRect().width / parseFloat(style.fontSize),
        };
      }),
    );
    const wide = await page.locator('.told-table thead').first().evaluate((head) => getComputedStyle(head).display !== 'none');
    expect(`titles ${what}`, titles.length > 0);
    if (wide) {
      const long = titles.filter((one) => one.lines !== 1).map((one) => `"${one.title}" on ${one.lines} lines`);
      expect(`each card's title on one line ${what}, got ${long.join('; ')}`, long.length === 0);
      const plain = titles.filter((one) => one.cut && !one.ellipsis).map((one) => `"${one.title}"`);
      expect(`a title cut with an ellipsis ${what}, got ${plain.join('; ')}`, plain.length === 0);
      const squeezed = titles.filter((one) => one.ems < 5).map((one) => `"${one.title}" in ${one.ems.toFixed(1)}em`);
      expect(`a title's cell, and a last thing said's, room for a word or two ${what}, got ${squeezed.join('; ')}`, squeezed.length === 0);
    } else {
      const torn = titles.filter((one) => one.lines > one.words).map((one) => `"${one.title}" on ${one.lines} lines`);
      expect(`each card's title to wrap between its words ${what}, got ${torn.join('; ')}`, torn.length === 0);
    }
    // Whom it is given to: the name whole in its cell, or cut with an ellipsis and its kind whole, with the whole of
    // both for the tip, and its cell room for a name.
    const names = await page.locator('.told-card-who .assigned').evaluateAll((all) =>
      all.map((who) => {
        const name = who.querySelector('.assignee-name');
        const style = getComputedStyle(name);
        return { name: name.textContent, said: who.getAttribute('title') ?? '', cut: name.scrollWidth > name.clientWidth + 0.5, ellipsis: style.textOverflow === 'ellipsis', ems: who.getBoundingClientRect().width / parseFloat(style.fontSize), kind: who.lastElementChild?.textContent ?? '' };
      }),
    );
    expect(`a name in Assigned to ${what}`, names.some((one) => one.name === WORKER));
    const plainly = names.filter((one) => one.cut && !one.ellipsis).map((one) => one.name);
    expect(`a name in Assigned to cut only with an ellipsis ${what}, got ${plainly.join('; ')}`, plainly.length === 0);
    const unsaid = names.filter((one) => !one.said.includes(one.name)).map((one) => one.name);
    expect(`a name in Assigned to said whole for the tip ${what}, got ${unsaid.join('; ')}`, unsaid.length === 0);
    // Where the cards are a table, a name no longer than 20em is whole: it is given its room before the title grows.
    const cut = names.filter((one) => wide && one.cut).map((one) => `${one.name} in ${one.ems.toFixed(1)}em`);
    expect(`each name in Assigned to whole ${what}, got ${cut.join('; ')} cut`, cut.length === 0);
    // And never the card's body.
    expect(`the card's body nowhere on the page ${what}`, !(await page.locator('.inbox-page').innerText()).includes(BODY));
    const broken = lines.filter((one) => one.lines !== 1).map((one) => `${one.id} on ${one.lines} lines`);
    expect(`each card's id whole on one line ${what}, got ${broken.join('; ')}`, broken.length === 0);
  };
  // The line's frame fills its tab, the room at its right the room at its left to a pixel, and its words a reader's
  // width within it.
  const fills = async (page, what) => {
    const at = await page.evaluate(() => {
      const tab = document.querySelector('.inbox-page');
      const rim = tab.getBoundingClientRect();
      const left = rim.left + tab.clientLeft;
      const right = left + tab.clientWidth;
      const frame = document.querySelector('.inbox-page-message').getBoundingClientRect();
      const cards = document.querySelector('.told-cards').getBoundingClientRect();
      const words = document.querySelector('.inbox-page-words').getBoundingClientRect();
      return { left: frame.left - left, right: right - frame.right, tab: tab.clientWidth, cards: cards.width, frame: frame.width, words: words.width };
    });
    expect(`the line's frame to fill its tab ${what}, the right gap the left's, and its gaps were ${at.left.toFixed(1)} and ${at.right.toFixed(1)} of ${at.tab}`, at.left >= 0 && Math.abs(at.left - at.right) <= 1);
    expect(`the cards to take the frame's width ${what}, got ${at.cards.toFixed(0)} of ${at.frame.toFixed(0)}`, at.cards > at.frame - 80);
    expect(`the line's words a reader's width ${what}, got ${at.words.toFixed(0)}`, at.words <= 684.5);
  };
  const laidOut = async (page, what, wide) => {
    await fills(page, what);
    await whole(page, what);
    await shaped(page, what, wide);
    await apart(page, '.inbox-page-message', PARTS, `the page's parts ${what}`);
    await apart(page, '.told-card-row', ROW, `a card's row ${what}`);
    await apart(page, '.told-card', '.told-card-row, .told-fail, .told-thread, .told-refused', `a card unfolded ${what}`);
  };
  const card = (shown, id) => shown.locator(`.told-card[data-card="${id}"]`);

  // ── The line's page: a link, and the cards staging held as it was told ──
  const page = await signedIn('ada@example.com');
  let loads = 0;
  page.on('load', () => (loads += 1));
  const shown = await opened(page);
  const link = shown.locator('.told-links a.inbox-link');
  expect(`the link to lead where the part says, in a tab of its own`, (await link.getAttribute('href')) === 'http://127.0.0.1:7760/' && (await link.getAttribute('target')) === '_blank');
  await shown.locator('.told-cards .inbox-page-label', { hasText: 'Cards that were in staging' }).waitFor();
  const goes = await shown.locator('.told-goes').innerText();
  expect(`the part to say where cards passed and failed go, got "${goes}"`, goes === 'Passed are approved and go to done; failed go back to ready, with why.');
  await card(shown, 'storefront-1').waitFor();
  await card(shown, 'storefront-2').waitFor();
  // Its parts say no tab: one tab, and no row of tabs over them (`toldtabs.mjs` has the tabs).
  expect('no row of tabs over parts that say no tab', (await shown.locator('.told-tabs').count()) === 0);
  const first = card(shown, 'storefront-1');
  const said = async (one) => (await one.locator('.told-card-row').innerText()).replace(/\s+/g, ' ');
  const row = await said(first);
  expect(`the row to say the card, that it is given to nobody, its column and that nothing was said, got "${row}"`, row.includes('storefront-1 Fix the cart nobody') && row.includes('staging') && row.includes('nothing said'));
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `at ${width} wide`, TABLE.includes(width) ? true : undefined);
    await page.screenshot({ path: `${out}/reviewed-1-at-${width}.png` });
  }
  // Where such a line is often read: the long title cut, and pointed at, said whole by the tip.
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/reviewed-1-page.png` });
  const longCut = await card(shown, 'storefront-2').locator('.told-card-words').evaluate((words) => words.scrollWidth > words.clientWidth);
  expect('the long title cut at 1280 wide', longCut);
  await card(shown, 'storefront-2').locator('.told-card-open').hover();
  const tip = page.getByRole('tooltip');
  await tip.waitFor();
  expect(`the tip to say the title whole, got "${await tip.innerText()}"`, (await tip.innerText()).trim() === LONG);
  await page.screenshot({ path: `${out}/reviewed-1-tip.png` });
  await page.mouse.move(0, 0);
  await tip.waitFor({ state: 'detached' });
  // And whom it is given to, the whole of it.
  await card(shown, 'storefront-2').locator('.told-card-who .assigned').hover();
  await tip.waitFor();
  expect(`the tip to say whom it is given to whole, got "${await tip.innerText()}"`, (await tip.innerText()).trim() === `Assigned to ${WORKER} (agent)`);
  await page.mouse.move(0, 0);
  await tip.waitFor({ state: 'detached' });
  await page.setViewportSize({ width: WIDTHS[0], height: 860 });

  // ── A card opened beside the page: its own form and history in the drawer, the page staying ──
  await first.locator('.told-card-open').click();
  const beside = drawerAtRest(page);
  await beside.locator('form.card-form').getByLabel('Title', { exact: true }).waitFor();
  expect('the drawer to be the card\'s own form', (await beside.getByLabel('Title', { exact: true }).inputValue()) === 'Fix the cart');
  // And its history, a tab beside its Details.
  await beside.getByRole('tab', { name: 'History', exact: true }).click();
  await beside.locator('.journey').waitFor();
  await beside.getByRole('tab', { name: 'Details', exact: true }).click();
  expect('the page to stay where it is', (await shown.count()) === 1 && (await shown.locator('.inbox-page-words').count()) === 1);
  await laidOut(page, 'beside the drawer');
  await page.screenshot({ path: `${out}/reviewed-1-drawer.png` });
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });

  // ── Its thread: something said there is said on the card ──
  await first.getByRole('button', { name: 'Discuss' }).click();
  await first.getByText('Nothing was said on it since it came to staging.').waitFor();
  await first.getByLabel('Say something on storefront-1').fill('Mind the tax on shipping. The total at checkout leaves it out for every address outside the country, and the receipt then says the wrong amount as well.');
  await first.locator('.told-say').getByRole('button', { name: 'Comment' }).click();
  await first.locator('.told-said', { hasText: 'Mind the tax on shipping.' }).waitFor();
  await first.locator('.told-card-last', { hasText: 'Ada: Mind the tax on shipping.' }).waitFor();
  expect('the card to say it on itself', (await cli('board', 'history', 'storefront-1')).stdout.includes('Ada said: Mind the tax on shipping.'));
  // One line in the table, and whole for whoever points at it.
  const SAID = 'Ada: Mind the tax on shipping. The total at checkout leaves it out for every address outside the country, and the receipt then says the wrong amount as well.';
  await first.locator('.told-card-line').hover();
  const saidTip = page.getByRole('tooltip');
  await saidTip.waitFor();
  expect(`the tip to say the last thing said whole, got "${await saidTip.innerText()}"`, (await saidTip.innerText()).trim() === SAID);
  await page.screenshot({ path: `${out}/reviewed-2-said-tip.png` });
  await page.mouse.move(0, 0);
  await saidTip.waitFor({ state: 'detached' });
  await laidOut(page, 'with a thread open');
  await page.screenshot({ path: `${out}/reviewed-2-thread.png` });

  // ── Passed: it goes to done ──
  await card(shown, 'storefront-2').getByRole('button', { name: 'Pass', exact: true }).click();
  await shown.locator('.told-done', { hasText: 'storefront-2 is approved, and moved to done.' }).waitFor();
  // Still listed, where it is now, with nothing to press: the line goes on saying what it was told of.
  const passedRow = card(shown, 'storefront-2');
  await passedRow.locator('.told-card-now', { hasText: 'in done' }).waitFor();
  expect('the card passed to say it is in done', (await passedRow.locator('.told-card-column').innerText()).trim() === 'done');
  expect('no Pass or Fail for a card that moved on', (await passedRow.getByRole('button', { name: /^(Pass|Fail)$/ }).count()) === 0);
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(page, `with a card in done at ${width} wide`, TABLE.includes(width) ? true : undefined);
  }
  await page.setViewportSize({ width: WIDTHS[0], height: 860 });
  await page.screenshot({ path: `${out}/reviewed-2-done.png` });
  // Approved by whoever passed it, a line of its history, with no words said on it, and then moved where the part says.
  const passedHistory = (await cli('board', 'history', 'storefront-2')).stdout;
  expect(`the card passed to be approved and in done, got ${passedHistory}`, passedHistory.includes('Ada approved it') && passedHistory.includes('moved it from staging to done'));
  expect(`no comment written for passing it, got ${passedHistory}`, !passedHistory.includes('Ada said:') && !passedHistory.includes('Passed'));

  // ── Failed: why first, then back to ready, worked again told why and what was said ──
  const before = (await turns()).length;
  await first.getByRole('button', { name: 'Fail', exact: true }).click();
  const failIt = first.getByRole('button', { name: 'Fail it' });
  await failIt.waitFor();
  expect('Fail it to be closed with nothing written', (await failIt.getAttribute('aria-disabled')) === 'true');
  const why = await first.locator('.told-fail-why').innerText();
  expect(`the fail to say why it is closed, got "${why}"`, why === 'Say why it fails: that is what its agent is told when it next has it.');
  await laidOut(page, 'with a fail asking why');
  await page.screenshot({ path: `${out}/reviewed-3-why.png` });
  await failIt.click({ force: true });
  expect('nothing failed while it is closed', (await first.count()) === 1 && (await cli('board', 'history', 'storefront-1')).stdout.includes('from staging to ready') === false);
  await first.getByLabel('Why it fails').fill('The cart forgets the tax.');
  expect('Fail it open once something is written', (await failIt.getAttribute('aria-disabled')) === 'false');
  await failIt.click();
  await shown.locator('.told-done', { hasText: 'storefront-1 failed: moved to ready.' }).waitFor();
  const history = (await cli('board', 'history', 'storefront-1')).stdout;
  expect(`its history to say it went back to ready with why, got ${history}`, history.includes('moved it from staging to ready, saying: The cart forgets the tax.'));
  // Worked again, it comes back to staging, listed again on the page that was never read again.
  // Its thread is what was said since it came there, which is nothing yet: so it is the card come back.
  await card(shown, 'storefront-1').locator('.told-card-last', { hasText: 'nothing said' }).waitFor({ timeout: 90_000 });
  await card(shown, 'storefront-1').getByRole('button', { name: 'Pass', exact: true }).waitFor();
  const again = (await turns()).slice(before);
  expect(`the stand-in to have been given the card once more, got ${again.length} turns`, again.length === 1);
  expect(`the stand-in to be shown why it failed, got ${again[0]}`, again[0].includes('This card was sent back') && again[0].includes('The cart forgets the tax.'));
  expect(`the stand-in to be shown what was said in its thread, got ${again[0]}`, again[0].includes('What was said on the card since you last had it:') && again[0].includes('- Ada: Mind the tax on shipping.'));
  await page.screenshot({ path: `${out}/reviewed-4-back.png` });

  // ── Said again, and failed again: the stand-in is shown what was said since, and not before ──
  const back = card(shown, 'storefront-1');
  if ((await back.locator('.told-thread').count()) === 0) await back.locator('.told-card-acts button').first().click();
  await back.getByLabel('Say something on storefront-1').fill('The total is still short.');
  await back.locator('.told-say').getByRole('button', { name: 'Comment' }).click();
  await back.locator('.told-said', { hasText: 'The total is still short.' }).waitFor();
  await back.getByRole('button', { name: 'Fail', exact: true }).click();
  await back.getByLabel('Why it fails').fill('Still the tax.');
  await back.getByRole('button', { name: 'Fail it' }).click();
  await shown.locator('.told-done', { hasText: 'storefront-1 failed: moved to ready.' }).waitFor();
  await card(shown, 'storefront-1').locator('.told-card-last', { hasText: 'nothing said' }).waitFor({ timeout: 90_000 });
  const third = (await turns()).slice(before + 1);
  expect(`one turn more, got ${third.length}`, third.length === 1);
  expect(`the stand-in shown what was said since its last turn and not before, got ${third[0]}`, third[0].includes('- Ada: The total is still short.') && !third[0].includes('Mind the tax on shipping.') && third[0].includes('Still the tax.'));

  // ── Somebody who may not change the board: shown the cards and offered nothing ──
  const narrow = (await grace('key', 'review', '--scope', 'inbox', '--scope', 'board:read')).stdout.split(/\s+/).find((word) => word.startsWith('key_'));
  expect('a key of Grace that holds less', Boolean(narrow));
  const hersPage = await signedIn('grace@example.com', { viewport: { width: 1280, height: 860 } }, async (one) => {
    // What the parts come to is read as that credential reads it: the daemon's own answer for somebody who holds less.
    await one.route('**/api/inbox/*/parts', (route) => route.continue({ headers: { ...route.request().headers(), authorization: `Bearer ${narrow}` } }));
  });
  const theirs = await opened(hersPage);
  await card(theirs, 'storefront-1').waitFor();
  expect('no Pass or Fail for somebody who may not change the board', (await theirs.getByRole('button', { name: /^(Pass|Fail)$/ }).count()) === 0);
  await card(theirs, 'storefront-1').locator('.told-card-acts button').first().click();
  await card(theirs, 'storefront-1').locator('.told-thread').waitFor();
  expect('no box to say anything in for somebody who may not change the board', (await theirs.locator('.told-say').count()) === 0);
  const lines = JSON.parse((await grace('inbox', '--json')).stdout).lines;
  const hersLine = lines.find((one) => one.why === 'preview').id;
  const refused = await fetch(`${url}/api/inbox/${hersLine}/parts/2/cards/storefront-1`, { method: 'POST', headers: { authorization: `Bearer ${narrow}`, 'content-type': 'application/json' }, body: JSON.stringify({ verdict: 'pass' }) });
  expect(`the daemon to refuse that credential, got ${refused.status}`, refused.status === 403);
  await hersPage.screenshot({ path: `${out}/reviewed-5-may-not.png` });

  // ── A phone ──
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  const small = await opened(phone, 'tap');
  await card(small, 'storefront-1').waitFor();
  await card(small, 'storefront-1').getByRole('button', { name: 'Fail', exact: true }).tap();
  await card(small, 'storefront-1').locator('.told-card-acts button').first().tap();
  await card(small, 'storefront-1').locator('.told-thread').waitFor();
  await laidOut(phone, 'on a phone', false);
  await card(small, 'storefront-2').locator('.told-card-now', { hasText: 'in done' }).waitFor();
  expect('no Pass or Fail on a phone for a card that moved on', (await card(small, 'storefront-2').getByRole('button', { name: /^(Pass|Fail)$/ }).count()) === 0);
  const fingers = await card(small, 'storefront-1').locator('.told-card-acts button').evaluateAll((all) => all.map((one) => one.getBoundingClientRect().height));
  expect(`a card's buttons a finger's size on a phone, got ${fingers}`, fingers.every((tall) => tall >= 40));
  await phone.screenshot({ path: `${out}/reviewed-6-phone.png` });
  expect(`Ada's page never to have been read again, was ${loads} times`, loads === 0);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  // What the board and the fleet's tables came to, for whoever reads why.
  for (const asked of [['board', 'storefront'], ['record', 'list', 'storefront', 'stages'], ['read', 'storefront']]) {
    console.error(await cli(...asked).then((said) => said.stdout, (why) => String(why)));
  }
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
// A told line's page in tabs, with rows, runs and measurements (`toldtabs.mjs`).
if (spawnSync(process.execPath, [new URL('./toldtabs.mjs', import.meta.url).pathname, out], { stdio: 'inherit', env: process.env }).status !== 0) process.exit(1);
console.log(`pictures are in ${out}`);
