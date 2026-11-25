// A table's columns as the people who look at it change them, from the
// page: a column added, called something else, retyped, its options
// changed, copied and taken away.
//
//   DAEMON=../target/debug/factory node scripts/columned.mjs [out-dir]
//
// A project has a board with a fleet, a table of stock its own blueprint
// declares, and a ledger only the factory's owner may change. Held here:
//
//   - a column is added from the `+` after the last head and from Insert
//     left and Insert right of a column's menu, by a small form under what
//     opened it that moves nothing, lies over nothing of itself and is
//     wholly in the window; it stands where it was put, for a second open
//     page that is not read again;
//   - a column is called something else at the head of its menu, on both
//     pages, and is still called by its name where a record is written by
//     it; a name another column has is said and not asked;
//   - one somebody added is made another kind where every row fits, and is
//     refused in the daemon's sentence where a row does not;
//   - what a choice offers is added to, called something else (the rows
//     that chose it say the new word), put in order, and one a row still
//     chooses is not taken away; a declared choice offers its order alone;
//   - a column is copied beside itself without what its rows say, and is
//     taken away behind a question that says what goes;
//   - a declared column says what reads it, offers no change of type and
//     no delete, and opens its blueprint's text;
//   - what filling a column takes is said, with whether whoever looks may;
//   - somebody who may not is offered none of it and is refused by the
//     routes: a table they may not change, and a credential that lacks the
//     scope;
//   - the fleet's agents' table with a column of somebody's, and the fleet
//     applied again;
//   - a phone's width, by the boxes; and the other of dark and light.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { gearLine } from './gear.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
// A check run several at once is given a folder of its own, and keeps its factory there.
const data = join(out, 'columned-data');
const home = join(out, 'columned-home');

const TABLES = `blueprint: stock
description: A table of the project's own, and one only the owner may change.
permissions: [projects, board]
steps:
  stock:
    do: collection.ensure
    with:
      name: stock
      fields:
        title: { type: text, required: true }
        status: { type: choice, of: [todo, doing, done] }
        points: { type: number }
  ledger:
    do: collection.ensure
    with:
      name: ledger
      takes: admin
      fields:
        title: { type: text, required: true }
        sum: { type: number }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  const shop = join(data, 'projects/shop');
  // A blueprint of the project's own, where the factory keeps it (`@blueprints/` of the project's).
  const plan = join(data, 'blueprints', 'projects', 'shop', 'stock.bp');
  await mkdir(join(data, 'blueprints', 'projects', 'shop'), { recursive: true });
  await writeFile(plan, TABLES);
  await cli('project', 'scan', 'shop');
  await cli('apply', plan, '--project', 'shop');
  for (const [title, status, points] of [['Basket', 'todo', 3], ['Crate', 'doing', 5], ['Shelf', 'done', 8]]) await cli('record', 'add', 'shop', 'stock', `title=${title}`, `status=${status}`, `points=${points}`);
  await cli('record', 'add', 'shop', 'ledger', 'title=Shelving', 'sum=4');

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

  /** A page that is signed in. `lacking` is what whoever is signed in is said not to hold, for a page that is to draw as for somebody who holds less. */
  const opened = async (context, { email = 'ada@example.com', touch = false, lacking = null } = {}) => {
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    if (lacking) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        // Nobody is signed in yet: the answer is the daemon's own.
        if (!me.principal) return route.fulfill({ response: answer });
        // What a member holds, each said out, without the one thing: the daemon says who somebody is, and the page goes by it.
        me.principal.scopes = lacking.holds;
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(url);
    // Whichever the page comes to draw, where to sign in or what somebody signed in is shown: it is waited
    // for before it is asked which. Asked as soon as the page had loaded, a page that had not drawn yet was
    // taken for one signed in, and was waited on for ever (once in twelve, six at once).
    await page.locator('.panes').or(page.getByLabel('Email')).first().waitFor();
    if (await page.getByLabel('Email').count()) {
      await page.getByLabel('Email').fill(email);
      await page.getByLabel('Password').fill('correct horse');
      const signIn = page.getByRole('button', { name: 'Sign in', exact: true }).last();
      await (touch ? signIn.tap() : signIn.click());
    }
    await page.locator('.panes').waitFor();
    return page;
  };
  /** Shows one of the project's tables, from the side panel. */
  const show = async (page, table, touch = false) => {
    const press = (locator) => (touch ? locator.tap() : locator.click());
    if (touch) await press(page.getByRole('button', { name: 'Expand the side panel' }));
    await sideSettled(page);
    const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await press(resources);
    await unfoldAll(page);
    const nav = page.locator('.nav');
    await nav.getByText('Shop', { exact: true }).waitFor();
    const twist = nav.getByRole('button', { name: 'Unfold Shop', exact: true });
    if (await twist.count()) await press(twist);
    const called = table.charAt(0).toUpperCase() + table.slice(1);
    await press(nav.locator('[data-project="shop"]').getByRole('button', { name: new RegExp(`${called}$`) }).first());
    if (table !== 'board') await page.getByRole('heading', { name: `Shop ${table}`, level: 1 }).waitFor();
  };
  const grid = (page, within = '') => page.locator(`${within} table.grid`.trim());
  const head = (page, column, within = '') => grid(page, within).locator('thead th[data-column]', { has: page.locator('.grid-head .truncate', { hasText: new RegExp(`^${column}$`) }) });
  const pressed = (page, column, within = '') => head(page, column, within).locator('.grid-head');
  const menu = (page) => page.locator('.headmenu:not(.colform)');
  const form = (page) => page.locator('.colform');
  const line = (page, name) => menu(page).getByRole('menuitem', { name }).or(menu(page).getByRole('menuitemcheckbox', { name })).or(menu(page).getByRole('menuitemradio', { name }));
  /** The columns as they stand, by what each is called. */
  const columns = (page, within = '') => grid(page, within).locator('thead th[data-column] .grid-head .truncate').allInnerTexts();
  const standsAs = async (page, wanted, within = '') => {
    const stand = page.waitForFunction(([wanted, within]) => [...document.querySelectorAll(`${within} table.grid thead th[data-column] .grid-head .truncate`)].map((one) => one.textContent).join(' | ') === wanted, [wanted.join(' | '), within]);
    await stand.catch(async () => expect(`the columns to stand as ${wanted.join(' | ')}, and they stand as ${(await columns(page, within)).join(' | ')}`, false));
  };
  /** Opens the menu of a column and waits until the caret is in it. */
  const open = async (page, column, { touch = false, within = '' } = {}) => {
    await (touch ? pressed(page, column, within).tap() : pressed(page, column, within).click());
    await menu(page).waitFor();
    await page.waitForFunction(() => document.activeElement?.closest('.headmenu') !== null && document.querySelector('.headmenu').style.visibility !== 'hidden');
  };
  const shut = async (page) => {
    await page.keyboard.press('Escape');
    await menu(page).waitFor({ state: 'detached' });
  };
  /** What the lines of the open menu say, in order. */
  const said = (page) => menu(page).locator('.headmenu-line .grow').allInnerTexts();
  /** Where the things of a page are that a panel opening must not move. */
  const standing = (page) =>
    page.evaluate(() => {
      const all = [...document.querySelectorAll('.view-head h1, .view-head button, table.grid th, table.grid td, .grid-new, .tab, .rail-button')];
      const scrolls = [...document.querySelectorAll('.grid-scroll')].flatMap((one) => [one.scrollLeft, one.scrollTop]);
      return JSON.stringify([window.scrollX, window.scrollY, ...scrolls, ...all.map((one) => Object.values(one.getBoundingClientRect().toJSON()).map((n) => Math.round(n * 10)))]);
    });
  /** A panel by its boxes: all of it in the window, each thing in it inside it and none over another, and the page no wider for it. */
  const laidOut = async (page, panel, where) => {
    const at = await page.evaluate((panel) => {
      const box = (one) => {
        const { left, top, right, bottom } = one.getBoundingClientRect();
        return { left, top, right, bottom, words: (one.textContent || one.getAttribute('aria-label') || one.tagName).trim().slice(0, 30) };
      };
      const drawn = document.querySelector(panel);
      const parts = '.headmenu-line, .headmenu-column, .headmenu-whose, .colform > .headmenu-refused, .headmenu-ask, .select-option, input.options-add, .colform > input, .colform-of, .colform-foot, .takes-line';
      return { window: { width: window.innerWidth, height: window.innerHeight }, wide: document.documentElement.scrollWidth, panel: box(drawn), parts: [...drawn.querySelectorAll(parts)].map(box), sideways: drawn.scrollWidth - drawn.clientWidth };
    }, panel);
    const within = (inner, outer) => inner.left >= outer.left - 0.5 && inner.top >= outer.top - 0.5 && inner.right <= outer.right + 0.5 && inner.bottom <= outer.bottom + 0.5;
    const apart = (one, other) => one.right <= other.left + 0.5 || other.right <= one.left + 0.5 || one.bottom <= other.top + 0.5 || other.bottom <= one.top + 0.5;
    expect(`${where}: the panel to be wholly in the window, and it is at ${JSON.stringify(at.panel)} in ${JSON.stringify(at.window)}`, within(at.panel, { left: 0, top: 0, right: at.window.width, bottom: at.window.height }));
    expect(`${where}: the page not to be made wider by it, and it is ${at.wide} in ${at.window.width}`, at.wide <= at.window.width);
    expect(`${where}: the panel not to scroll sideways, and it does by ${at.sideways}`, at.sideways <= 0);
    expect(`${where}: the panel to have something in it`, at.parts.length > 1);
    for (const [index, one] of at.parts.entries()) {
      expect(`${where}: each thing to be inside the panel, and ${JSON.stringify(one)} is not in ${JSON.stringify(at.panel)}`, within(one, at.panel));
      for (const other of at.parts.slice(index + 1)) expect(`${where}: nothing over anything else, and ${JSON.stringify(one)} is over ${JSON.stringify(other)}`, apart(one, other));
    }
    return at;
  };
  /** The stock as the daemon keeps it: asked from a page as that page's own request. */
  const stock = (page, table = 'stock') =>
    page.evaluate(async (table) => {
      const answer = await fetch('/api/collections?project=shop', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      return (await answer.json()).collections.find((one) => one.name === table);
    }, table);
  const fieldOf = async (page, name, table = 'stock') => (await stock(page, table)).fields.find((one) => one.name === name);
  /** Asks the daemon something as a page's own request: how it answered. */
  const ask = (page, method, path, body) =>
    page.evaluate(
      async ([method, path, body]) => {
        const answer = await fetch(path, { method, headers: { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' }, body: body === null ? undefined : JSON.stringify(body) });
        return { status: answer.status, said: await answer.text() };
      },
      [method, path, body ?? null],
    );
  /** What the page says could not be done, once it has said so. */
  const refusal = async (page, words) => {
    const toast = page.locator('.toasts .toast', { hasText: words });
    await toast.first().waitFor();
    return toast.first().innerText();
  };
  /** Fills the form a column is added by, and adds it. */
  const addAs = async (page, name, kind, { options = [], touch = false } = {}) => {
    await form(page).waitFor();
    await form(page).getByLabel('What the column is called').fill(name);
    const kindLine = form(page).getByRole('menuitemradio', { name: kind, exact: true });
    await (touch ? kindLine.tap() : kindLine.click());
    for (const option of options) {
      await form(page).getByLabel(/^Add something .* offers$/).fill(option);
      await page.keyboard.press('Enter');
      await form(page).locator(`[data-option="${option}"]`).waitFor();
    }
    const add = form(page).getByRole('button', { name: 'Add column', exact: true });
    await (touch ? add.tap() : add.click());
    await form(page).waitFor({ state: 'detached' });
  };

  const scheme = process.env.SCHEME ?? 'dark';
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const page = await opened(context);
  await show(page, 'stock');
  await page.locator('tr[data-row="shop-stock-3"]').waitFor();
  const beside = await opened(context);
  await show(beside, 'stock');
  await beside.locator('tr[data-row="shop-stock-3"]').waitFor();
  await beside.evaluate(() => {
    window.readOnce = true;
  });
  const notReadAgain = async (what) => expect(`the other page not to have been read again ${what}`, await beside.evaluate(() => window.readOnce === true));
  const STOOD = ['Title', 'Status', 'Points'];
  expect(`the columns to stand as the blueprint declared them, got ${(await columns(page)).join(' | ')}`, (await columns(page)).join(' | ') === STOOD.join(' | '));

  // ---- a column added from the `+` after the last head
  const plus = page.getByRole('button', { name: 'Add a column', exact: true });
  expect('one place to add a column from, after the last head', (await plus.count()) === 1 && (await grid(page).locator('thead tr').last().locator('th').last().locator('.grid-add').count()) === 1);
  const before = await standing(page);
  await plus.click();
  await form(page).waitFor();
  await page.waitForFunction(() => document.querySelector('.colform').style.visibility !== 'hidden' && document.activeElement?.closest('.colform') !== null);
  expect('nothing on the page to move as the form opens', (await standing(page)) === before);
  expect('the caret to be where the column is named', await page.evaluate(() => document.activeElement?.getAttribute('aria-label') === 'What the column is called'));
  const whole = await laidOut(page, '.colform', 'the form a column is added by');
  const opener = await plus.boundingBox();
  expect(`the form to be under what opened it, and it is at ${JSON.stringify(whole.panel)} under ${JSON.stringify(opener)}`, whole.panel.top >= opener.y + opener.height && whole.panel.top <= opener.y + opener.height + 8);
  const framed = await form(page).evaluate((panel) => ({ border: getComputedStyle(panel).borderTopWidth, boxes: [...panel.querySelectorAll('input')].map((one) => getComputedStyle(one).borderTopWidth).join(' ') }));
  expect(`the form to be one panel with no frame, and nothing in it a frame of its own, got ${JSON.stringify(framed)}`, framed.border === '0px' && framed.boxes.split(' ').every((one) => one === '0px'));
  expect('nothing to be added until it is called something', await form(page).getByRole('button', { name: 'Add column', exact: true }).isDisabled());
  // A name another column reads as is said, and is not asked.
  await form(page).getByLabel('What the column is called').fill('points');
  await form(page).getByText('There is a column called Points already.').waitFor();
  expect('a name that is taken not to be asked for', await form(page).getByRole('button', { name: 'Add column', exact: true }).isDisabled());
  await laidOut(page, '.colform', 'the form, saying a name is taken');
  await form(page).getByLabel('What the column is called').fill('Notes');
  await page.screenshot({ path: `${out}/columned-01-adding.png` });
  await addAs(page, 'Notes', 'Text');
  await standsAs(page, [...STOOD, 'Notes']);
  await standsAs(beside, [...STOOD, 'Notes']);
  await notReadAgain('for a column added');
  expect('the caret to be back on what opened the form', await page.evaluate(() => document.activeElement?.classList.contains('grid-add')));
  const notes = await fieldOf(page, 'notes');
  expect(`the column to be the table's, marked as somebody's, got ${JSON.stringify(notes)}`, notes?.added === true && notes.kind === 'text' && notes.required === false);
  // Escape puts the form away and adds nothing; so does a press outside it.
  await plus.click();
  await form(page).waitFor();
  await form(page).getByLabel('What the column is called').fill('Nothing');
  await page.keyboard.press('Escape');
  await form(page).waitFor({ state: 'detached' });
  await plus.click();
  await form(page).waitFor();
  await page.locator('.view-head h1').click();
  await form(page).waitFor({ state: 'detached' });
  expect('nothing to have been added by a form that was put away', (await columns(page)).join(' | ') === [...STOOD, 'Notes'].join(' | '));

  // ---- inserted on either side of a column, from its menu
  await open(page, 'Points');
  expect(`the lines of a declared number's menu, got ${(await said(page)).join(' | ')}`, (await said(page)).join(' | ') === 'Sort ascending | Sort descending | Calculate | Freeze up to here | Hide | Move left | Move right | Insert left | Insert right | Duplicate column | What filling it takes | This column is read by stock, which declares it');
  await line(page, 'Insert left').click();
  await menu(page).waitFor({ state: 'detached' });
  await form(page).waitFor();
  const under = await pressed(page, 'Points').boundingBox();
  const put = await laidOut(page, '.colform', 'the form, from a column\'s menu');
  expect('the form to be under the head it was asked from', Math.abs(put.panel.left - under.x) <= 0.5 && put.panel.top >= under.y + under.height);
  await addAs(page, 'Weight', 'Number');
  await standsAs(page, ['Title', 'Status', 'Weight', 'Points', 'Notes']);
  await standsAs(beside, ['Title', 'Status', 'Weight', 'Points', 'Notes']);
  await open(page, 'Points');
  await line(page, 'Insert right').click();
  await form(page).waitFor();
  await form(page).getByLabel('What the column is called').fill('Mood');
  await form(page).getByRole('menuitemradio', { name: 'Select', exact: true }).click();
  expect('a choice not to be added until it offers something', await form(page).getByRole('button', { name: 'Add column', exact: true }).isDisabled());
  await addAs(page, 'Mood', 'Select', { options: ['fine', 'rushed'] });
  const NOW = ['Title', 'Status', 'Weight', 'Points', 'Mood', 'Notes'];
  await standsAs(page, NOW);
  await standsAs(beside, NOW);
  await notReadAgain('for columns inserted');
  expect('what the choice offers to be what was written', JSON.stringify((await fieldOf(page, 'mood')).of) === '["fine","rushed"]');
  // What a record is known by has nothing put before it.
  await open(page, 'Title');
  expect(`nothing to be inserted before what a record is known by, got ${(await said(page)).join(' | ')}`, (await said(page)).includes('Insert right') && !(await said(page)).includes('Insert left'));
  await shut(page);

  // ---- called something else, in place at the head of its menu
  await open(page, 'Notes');
  const name = menu(page).getByLabel('Name of Notes');
  expect('the name to be a box with what the column is called in it', (await name.inputValue()) === 'Notes');
  await menu(page).getByText('Blueprints and files call it notes.').waitFor();
  await menu(page).getByText('A column somebody added to this table', { exact: true }).waitFor();
  expect(`the lines of the menu of a column somebody added, got ${(await said(page)).join(' | ')}`, (await said(page)).join(' | ') === 'Change type | Filter | Sort ascending | Sort descending | Calculate | Freeze up to here | Hide | Move left | Insert left | Insert right | Duplicate column | Delete column | What filling it takes');
  await laidOut(page, '.headmenu', 'the menu of a column somebody added');
  await page.screenshot({ path: `${out}/columned-02-a-column-of-theirs.png` });
  // A name another column has is said under the box, and Enter asks nothing.
  await name.fill('Points');
  await menu(page).getByText('There is a column called Points already.').waitFor();
  await laidOut(page, '.headmenu', 'the menu, saying a name is taken');
  await page.keyboard.press('Enter');
  expect('the menu to stay while the name is refused', (await menu(page).count()) === 1);
  await name.fill('Remarks');
  await page.keyboard.press('Enter');
  await menu(page).waitFor({ state: 'detached' });
  const CALLED = NOW.map((one) => (one === 'Notes' ? 'Remarks' : one));
  await standsAs(page, CALLED);
  await standsAs(beside, CALLED);
  await notReadAgain('for a column called something else');
  // It is still called its name by whatever reads it: a record is written by it from a terminal.
  const named = await fieldOf(page, 'notes');
  expect(`the field to keep its name and say what it is called, got ${JSON.stringify(named)}`, named?.name === 'notes' && named.label_chosen === 'Remarks');
  await cli('record', 'add', 'shop', 'stock', 'title=Pallet', 'notes=fragile', 'mood=rushed');
  await page.locator('tr[data-row="shop-stock-4"]').getByText('fragile').waitFor();
  await beside.locator('tr[data-row="shop-stock-4"]').getByText('fragile').waitFor();
  // A declared column is called something else the same way, and its name again by writing that.
  await open(page, 'Points');
  await menu(page).getByLabel('Name of Points').fill('Score');
  await page.keyboard.press('Enter');
  await standsAs(beside, CALLED.map((one) => (one === 'Points' ? 'Score' : one)));
  expect('a declared column to keep its name', (await fieldOf(page, 'points'))?.label_chosen === 'Score');
  await open(page, 'Score');
  await menu(page).getByText('Blueprints and files call it points.').waitFor();
  await menu(page).getByLabel('Name of Score').fill('Points');
  // The caret leaving the box is the answer too.
  await line(page, 'Sort ascending').focus();
  await standsAs(beside, CALLED);
  await shut(page);
  expect('a column called what its name reads as to keep no label', (await fieldOf(page, 'points'))?.label_chosen === undefined);

  // ---- made another kind: refused where a row does not fit, and taken where every row does
  await open(page, 'Remarks');
  await line(page, /^Change type/).click();
  await menu(page).getByRole('menuitemradio', { name: 'Text', exact: true, checked: true }).waitFor();
  await laidOut(page, '.headmenu', 'the kinds a column can be made');
  await menu(page).getByRole('menuitemradio', { name: 'Number', exact: true }).click();
  const why = await refusal(page, 'does not take');
  expect(`a kind a row does not fit to be refused in a sentence that says how many, got ${why}`, why.includes('`notes` holds a `text` and 1 record(s) of `stock` say something there that a `number` does not take'));
  expect('the column to be as it was', (await fieldOf(page, 'notes'))?.kind === 'text');
  await shut(page);
  await open(page, 'Weight');
  await line(page, /^Change type/).click();
  await menu(page).getByRole('menuitemradio', { name: 'Checkbox', exact: true }).click();
  await menu(page).waitFor({ state: 'detached' });
  await page.locator('tr[data-row="shop-stock-1"] input[type="checkbox"][aria-label="Weight"]').waitFor();
  await beside.locator('tr[data-row="shop-stock-1"] input[type="checkbox"][aria-label="Weight"]').waitFor();
  await notReadAgain('for a column made another kind');
  // Another kind stands where its kind does until a view says otherwise: the columns are the same ones.
  expect(`every column to be there still, got ${(await columns(page)).join(' | ')}`, (await columns(page)).slice().sort().join(' | ') === CALLED.slice().sort().join(' | '));

  // ---- what a choice somebody made offers
  await open(page, 'Mood');
  await line(page, /^Options/).click();
  const offered = () => menu(page).locator('.options-edit [data-option]').evaluateAll((all) => all.map((one) => one.dataset.option).join(' '));
  const offers = (wanted) => page.waitForFunction((wanted) => [...document.querySelectorAll('.headmenu .options-edit [data-option]')].map((one) => one.dataset.option).join(' ') === wanted, wanted);
  expect(`the list to be what it offers, got ${await offered()}`, (await offered()) === 'fine rushed');
  await menu(page).getByLabel('Add something Mood offers').fill('Calm');
  await page.keyboard.press('Enter');
  await offers('fine rushed calm');
  // Called something else: the row that chose it says the new word, on a page that is not read again.
  await menu(page).getByRole('button', { name: 'Rename rushed', exact: true }).click();
  await menu(page).getByLabel('What rushed is called').fill('In a hurry');
  await page.keyboard.press('Enter');
  await offers('fine in-a-hurry calm');
  await beside.locator('tr[data-row="shop-stock-4"]').getByText('in-a-hurry').waitFor();
  await notReadAgain('for an option called something else');
  // Put in order by its grip's keys: Alt and an arrow.
  await menu(page).getByRole('button', { name: 'Move calm', exact: true }).focus();
  await page.keyboard.press('Alt+ArrowUp');
  await offers('fine calm in-a-hurry');
  await menu(page).getByRole('button', { name: 'Move calm', exact: true }).focus();
  await page.keyboard.press('Alt+ArrowUp');
  await offers('calm fine in-a-hurry');
  // And carried by its grip with the pointer.
  const grip = await menu(page).getByRole('button', { name: 'Move calm', exact: true }).boundingBox();
  const last = await menu(page).locator('[data-option="in-a-hurry"]').boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2);
  await page.mouse.down();
  await page.mouse.move(grip.x + grip.width / 2, last.y + last.height / 2, { steps: 8 });
  await page.mouse.up();
  await offers('fine in-a-hurry calm');
  expect('the order to be the table\'s, for everybody', JSON.stringify((await fieldOf(page, 'mood')).of) === '["fine","in-a-hurry","calm"]');
  await laidOut(page, '.headmenu', 'what a choice offers');
  await page.screenshot({ path: `${out}/columned-03-what-a-choice-offers.png` });
  // One a row still chooses stays, in the daemon's sentence; one no row chooses goes.
  await menu(page).getByRole('button', { name: 'Remove in-a-hurry', exact: true }).click();
  const still = await refusal(page, 'is still the');
  expect(`an option a row chooses not to be taken away, got ${still}`, still.includes('`in-a-hurry` is still the `mood` of 1 record(s) of `stock`'));
  await menu(page).getByRole('button', { name: 'Remove fine', exact: true }).click();
  await offers('in-a-hurry calm');
  await shut(page);
  // A declared choice offers its order, and nothing that changes what it offers.
  await open(page, 'Status');
  await line(page, /^Options/).click();
  expect('what a declared choice offers to be neither added to, renamed nor taken away', (await menu(page).getByRole('button', { name: /^(Rename|Remove) / }).count()) === 0 && (await menu(page).getByLabel(/^Add something/).count()) === 0);
  await menu(page).getByText('What it offers is said where the table is declared.', { exact: false }).waitFor();
  await menu(page).getByRole('button', { name: 'Move done', exact: true }).focus();
  await page.keyboard.press('Alt+ArrowUp');
  await page.waitForFunction(() => [...document.querySelectorAll('.headmenu .options-edit [data-option]')].map((one) => one.dataset.option).join(' ') === 'todo done doing');
  expect('the order of a declared choice to be kept as somebody\'s', JSON.stringify((await fieldOf(page, 'status')).of) === '["todo","done","doing"]');
  await shut(page);

  // ---- copied beside itself, without what its rows say
  await open(page, 'Mood');
  await line(page, 'Duplicate column').click();
  await menu(page).waitFor({ state: 'detached' });
  await page.waitForFunction(() => [...document.querySelectorAll('table.grid thead th[data-column]')].map((one) => one.dataset.column).join(' ').includes('mood mood_copy'));
  await beside.waitForFunction(() => [...document.querySelectorAll('table.grid thead th[data-column]')].map((one) => one.dataset.column).join(' ').includes('mood mood_copy'));
  const copy = await fieldOf(page, 'mood_copy');
  expect(`a copy to hold the same kind and offer the same, got ${JSON.stringify(copy)}`, copy?.kind === 'choice' && copy.added === true && JSON.stringify(copy.of) === '["in-a-hurry","calm"]');
  expect('none of what the rows say to be copied', (await page.locator('tr[data-row="shop-stock-4"] td').nth((await columns(page)).indexOf('Mood copy')).innerText()).trim() === '');

  // ---- taken away, behind a question that says what goes
  await open(page, 'Remarks');
  await line(page, 'Delete column').click();
  await menu(page).getByText('Remarks is taken away for everybody who opens this table. What 1 row says in it goes with it. It cannot be put back.').waitFor();
  await laidOut(page, '.headmenu', 'the question a column is taken away behind');
  await page.screenshot({ path: `${out}/columned-04-taken-away-behind-a-question.png` });
  await menu(page).getByRole('button', { name: 'Keep it', exact: true }).click();
  await line(page, 'Delete column').waitFor();
  expect('nothing to go until it is said to', (await columns(beside)).includes('Remarks'));
  await line(page, 'Delete column').click();
  await menu(page).getByRole('button', { name: 'Delete column', exact: true }).click();
  await menu(page).waitFor({ state: 'detached' });
  await page.waitForFunction(() => !document.querySelector('table.grid thead th[data-column="notes"]'));
  await beside.waitForFunction(() => !document.querySelector('table.grid thead th[data-column="notes"]'));
  await notReadAgain('for a column taken away');
  expect('the field to be gone from the table, and what the row said with it', (await fieldOf(page, 'notes')) === undefined && !('notes' in JSON.parse((await ask(page, 'GET', '/api/records/shop-stock-4')).said).data));

  // ---- a declared column: whose it is, and nothing that would change it
  await open(page, 'Points');
  const declared = await said(page);
  expect(`a declared column to offer no change of type and no delete, got ${declared.join(' | ')}`, !declared.includes('Change type') && !declared.includes('Delete column') && declared.includes('This column is read by stock, which declares it'));
  await menu(page).getByText('What it holds, and whether it is there, are changed where the table is declared. Blueprints and files call it points.').waitFor();
  await laidOut(page, '.headmenu', 'the menu of a declared column');
  await page.screenshot({ path: `${out}/columned-05-a-declared-column.png` });
  // What filling it takes, read from what the table says, with whether whoever looks may.
  await line(page, /^What filling it takes/).click();
  await menu(page).getByText('records:write, to change a row of this table').waitFor();
  await menu(page).getByText('You may fill it.', { exact: true }).waitFor();
  await laidOut(page, '.headmenu', 'what filling a column takes');
  await page.screenshot({ path: `${out}/columned-06-what-filling-it-takes.png` });
  await menu(page).getByRole('button', { name: /^Filling Points takes/ }).click();
  // The routes refuse what the menu does not offer.
  for (const [method, body] of [['PATCH', { kind: 'text' }], ['DELETE', null]]) {
    const refused = await ask(page, method, '/api/collections/shop/stock/fields/points', body);
    expect(`a declared column not to be retyped or taken away by ${method}, got ${refused.status} ${refused.said}`, refused.status === 409 && refused.said.includes('is declared with the table'));
  }
  // What declares it is opened from there: the blueprint's own text, which is a file of the project.
  await line(page, /^This column is read by stock/).click();
  await page.locator('.tab', { hasText: 'stock.bp' }).waitFor();
  await show(page, 'stock');
  await page.locator('tr[data-row="shop-stock-4"]').waitFor();

  // ---- somebody who may not: a table they may not change, and a credential that lacks the scope
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joinedIn = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joinedIn.status}`, joinedIn.ok);
  const theirs = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const other = await opened(theirs, { email: 'grace@example.com' });
  await show(other, 'ledger');
  await other.locator('tr[data-row="shop-ledger-1"]').waitFor();
  expect('nowhere to add a column from, for somebody who may not change the table', (await other.getByRole('button', { name: 'Add a column' }).count()) === 0);
  await open(other, 'Sum');
  expect(`somebody who may not change a table to be offered none of it, got ${(await said(other)).join(' | ')}`, (await said(other)).join(' | ') === 'Sort ascending | Sort descending | What filling it takes | This column is read by stock, which declares it');
  expect('the name to be words and no box for them', (await menu(other).locator('input').count()) === 0 && (await menu(other).locator('.headmenu-column strong').innerText()) === 'Sum');
  await line(other, /^What filling it takes/).click();
  await menu(other).getByText('admin, which this table asks of whoever changes a row').waitFor();
  await menu(other).getByText('You may not fill it.', { exact: true }).waitFor();
  await other.screenshot({ path: `${out}/columned-07-not-theirs-to-change.png` });
  await shut(other);
  for (const [method, path, body] of [['POST', '', { label: 'More', kind: 'text' }], ['PATCH', '/sum', { label: 'Total' }], ['DELETE', '/sum', null]]) {
    const refused = await ask(other, method, `/api/collections/shop/ledger/fields${path}`, body);
    expect(`the routes to refuse somebody who may not change the table, ${method} got ${refused.status}`, refused.status === 403);
  }
  // A member holds the scope: the stock is theirs to add a column to.
  await show(other, 'stock');
  await other.locator('tr[data-row="shop-stock-4"]').waitFor();
  expect('a member to be offered the place a column is added from', (await other.getByRole('button', { name: 'Add a column' }).count()) === 1);
  await theirs.close();
  // Somebody who holds what a member does but for the scope: the page offers what changes a view, and none of what changes the columns.
  const holds = ['projects', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];
  const narrow = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const less = await opened(narrow, { lacking: { holds } });
  await show(less, 'stock');
  await less.locator('tr[data-row="shop-stock-4"]').waitFor();
  expect('nowhere to add a column from, for somebody who lacks the scope', (await less.getByRole('button', { name: 'Add a column' }).count()) === 0);
  await open(less, 'Mood');
  const lacking = await said(less);
  expect(`somebody who lacks the scope to be offered what changes a view and the order of a choice, and nothing that changes the columns, got ${lacking.join(' | ')}`, lacking.includes('Hide') && lacking.includes('Options') && !['Change type', 'Insert left', 'Insert right', 'Duplicate column', 'Delete column'].some((one) => lacking.includes(one)));
  await line(less, /^Options/).click();
  expect('what a choice offers not to be added to or taken from by them', (await menu(less).getByRole('button', { name: /^(Rename|Remove) / }).count()) === 0 && (await menu(less).getByRole('button', { name: /^Move / }).count()) === 2);
  await shut(less);
  await narrow.close();
  // And the daemon refuses a credential that holds so, whatever a page draws.
  const made = JSON.parse((await ask(page, 'POST', '/api/credentials', { name: 'narrow', scopes: holds })).said);
  for (const [method, path, body] of [['POST', '', { label: 'More', kind: 'text' }], ['POST', '', { like: 'points' }], ['PATCH', '/mood', { kind: 'text' }], ['DELETE', '/mood', null]]) {
    const answer = await fetch(`${url}/api/collections/shop/stock/fields${path}`, { method, headers: { authorization: `Bearer ${made.token}`, 'content-type': 'application/json' }, body: body === null ? undefined : JSON.stringify(body) });
    const refused = await answer.text();
    expect(`a credential that lacks the scope to be refused, ${method} ${path} got ${answer.status} ${refused}`, answer.status === 403 && refused.includes('needs `columns`'));
  }
  const reordered = await fetch(`${url}/api/collections/shop/stock/fields/mood`, { method: 'PATCH', headers: { authorization: `Bearer ${made.token}`, 'content-type': 'application/json' }, body: JSON.stringify({ label: 'Temper' }) });
  expect(`what a column is called to be theirs all the same, got ${reordered.status}`, reordered.ok);
  await standsAs(page, (await columns(page)).map((one) => (one === 'Mood' ? 'Temper' : one)));

  // ---- the fleet's agents' table, with a column of somebody's, and the fleet applied again
  await show(page, 'board');
  await gearLine(page, /^Agents/);
  const DRAWER = '.drawer[data-rest] .sits-table';
  await page.locator(`${DRAWER} table.grid tr[data-row="shop-agents-3"]`).waitFor();
  await page.locator(DRAWER).getByRole('button', { name: 'Open as a tab' }).click();
  await page.getByRole('heading', { name: 'Shop agents', level: 1 }).waitFor();
  await page.keyboard.press('Escape');
  await page.locator('tr[data-row="shop-agents-3"]').waitFor();
  await page.getByRole('button', { name: 'Add a column', exact: true }).click();
  await addAs(page, 'Remarks', 'Text');
  await page.waitForFunction(() => document.querySelector('table.grid thead th[data-column="remarks"]') !== null);
  const put1 = await ask(page, 'PATCH', '/api/records/shop-agents-1', { fields: { remarks: 'ours' } });
  expect(`a row of the fleet's to say something in their column, got ${put1.status} ${put1.said}`, put1.status === 200);
  await page.locator('tr[data-row="shop-agents-1"]').getByText('ours', { exact: true }).waitFor();
  // A declared column of the fleet's says the fleet reads it, and offers nothing that would change it.
  await open(page, 'Count');
  const fleets = await said(page);
  expect(`a column the fleet declares to say so and offer no change of type and no delete, got ${fleets.join(' | ')}`, fleets.some((one) => one.startsWith('This column is read by ')) && !fleets.includes('Change type') && !fleets.includes('Delete column'));
  await laidOut(page, '.headmenu', 'the menu of a column the fleet declares');
  await shut(page);
  await page.screenshot({ path: `${out}/columned-08-the-fleets-agents.png` });
  // The fleet applied again declares its tables again: the column and what the row says are there.
  await cli('apply', 'fleet', '--project', 'shop', '--force');
  const agents = await stock(page, 'agents');
  expect('the fleet applied again to keep the column somebody added', agents.fields.some((one) => one.name === 'remarks' && one.added === true));
  expect('and what its row says there', JSON.parse((await ask(page, 'GET', '/api/records/shop-agents-1')).said).data.remarks === 'ours');
  // The page reads the table again as it is declared, and shows the column and what the row says when it has.
  await page.locator('table.grid thead th[data-column="remarks"]').waitFor();
  await page.locator('tr[data-row="shop-agents-1"]').getByText('ours', { exact: true }).waitFor();

  // ---- a phone's width, by touch: the form and the menu by their boxes
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const held = await opened(phone, { touch: true });
  await show(held, 'stock', true);
  await held.locator('.sidebar').waitFor({ state: 'detached' });
  await held.locator('tr[data-row="shop-stock-4"]').waitFor();
  const add = held.getByRole('button', { name: 'Add a column', exact: true });
  await add.scrollIntoViewIfNeeded();
  await add.tap();
  await form(held).waitFor();
  await held.waitForFunction(() => document.querySelector('.colform').style.visibility !== 'hidden');
  const small = await laidOut(held, '.colform', 'the form on a phone');
  expect(`the form to fit a phone's screen, and it is ${small.panel.right - small.panel.left} wide`, small.panel.right - small.panel.left <= 374);
  await form(held).getByLabel('What the column is called').fill('Aisle');
  await form(held).getByRole('menuitemradio', { name: 'Select', exact: true }).tap();
  await form(held).getByLabel(/^Add something .* offers$/).fill('north');
  await held.keyboard.press('Enter');
  await form(held).locator('[data-option="north"]').waitFor();
  await laidOut(held, '.colform', 'the form on a phone, with what a choice offers');
  const finger = await form(held).getByRole('menuitemradio', { name: 'Select', exact: true }).boundingBox();
  expect(`a line of the form to be large enough for a finger, and it is ${finger.height} high`, finger.height >= 39.5);
  await held.screenshot({ path: `${out}/columned-09-phone.png` });
  await form(held).getByRole('button', { name: 'Add column', exact: true }).tap();
  await form(held).waitFor({ state: 'detached' });
  await held.waitForFunction(() => document.querySelector('table.grid thead th[data-column="aisle"]') !== null);
  await page.bringToFront();
  await pressed(held, 'Aisle').scrollIntoViewIfNeeded();
  await open(held, 'Aisle', { touch: true });
  await laidOut(held, '.headmenu', 'the menu on a phone');
  await line(held, /^Options/).tap();
  await laidOut(held, '.headmenu', 'what a choice offers, on a phone');
  await held.screenshot({ path: `${out}/columned-10-phone-options.png` });
  await phone.close();

  // ---- the other of dark and light
  const otherScheme = scheme === 'dark' ? 'light' : 'dark';
  const turned = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: otherScheme });
  const lit = await opened(turned);
  await show(lit, 'stock');
  await lit.locator('tr[data-row="shop-stock-4"]').waitFor();
  await open(lit, 'Temper');
  await laidOut(lit, '.headmenu', 'in the other theme');
  await lit.screenshot({ path: `${out}/columned-11-${otherScheme}.png` });
  await line(lit, /^Options/).click();
  await lit.screenshot({ path: `${out}/columned-12-${otherScheme}-options.png` });
  await shut(lit);
  await lit.getByRole('button', { name: 'Add a column', exact: true }).click();
  await form(lit).waitFor();
  await lit.waitForFunction(() => document.querySelector('.colform').style.visibility !== 'hidden');
  await lit.screenshot({ path: `${out}/columned-13-${otherScheme}-adding.png` });
  await turned.close();

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
