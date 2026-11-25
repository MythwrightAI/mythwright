// A project's features in the client: a table that is a board of its own,
// a feature read whole with its plan, something said on it, and one
// accepted by being carried to where features are built.
//
//   DAEMON=../target/debug/factory node scripts/features.mjs [out-dir]
//
// The agents here are the built-in assistant, which plans nothing, so a
// plan is written in by an agent's own credential as its agent would
// have left it. Accepted, the feature comes to the cards its plan lists,
// each saying in its drawer and its history that the factory's bot added
// it from what that agent wrote, by way of the features' blueprint, and
// not whoever accepted it.
//
// And how a project comes by its features from its board's page: a line
// of the gear's menu there, a switch, for whoever may apply a blueprint.
// Works has a board and a fleet and no features: the line is off and says
// what features are, by the boxes on the screen and moving nothing;
// pressed, the project has them, with the menu still open and the page
// not read again, and the side panel lists them. On, it is drawn as on
// and is not turned off there. What looks after the features is stopped
// where everything running is, after which the line is off again with the
// table still there, and gives them back, asked for once however often
// it is pressed before the answer is in. Ledger has a board and no
// fleet: its line is closed and says why. Somebody who may not apply is
// offered no line while it is off, and is told where it is on. And the
// menu by its boxes again at 600 wide and at a phone's width. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readFile, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'features-data');
const home = join(out, 'features-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const PLAN = 'First what a cart holds, then the page that shows one.\n\n```tickets\n[\n  { "title": "The cart\'s model", "body": "What a cart holds." },\n  { "title": "The cart\'s page", "after": [1] }\n]\n```';

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('apply', 'std/features', '--project', 'shop', '--param', 'project=shop');
  await cli('record', 'add', 'shop', 'features', 'title=A shopping cart', 'body=People want to buy more than one thing.', 'status=backlog');
  await cli('record', 'add', 'shop', 'features', 'title=Gift wrapping', 'status=backlog');
  // As an agent would have left it: a plan, written with the agent's own credential, and waiting to be
  // accepted. Its cards then say they came of what that agent wrote.
  await cli('agent', 'add', 'planner', '--project', 'shop', '--adapter', 'builtin');
  // An agent writes a plan when its feature asks it to, so after what looks after the feature has begun:
  // written as soon as the feature was added, under load it came before that was listening, nobody heard
  // who wrote it, and its cards said only that the features' blueprint added them (three runs in
  // thirty-two, four at once on two processors).
  const featureBegun = async () =>
    JSON.parse((await cli('instances', '--all', '--json')).stdout).instances.some(
      (one) => one.key === 'feature-shop-features-1' && one.state?.begun === true,
    );
  for (let tries = 0; tries < 300 && !(await featureBegun()); tries += 1) await new Promise((done) => setTimeout(done, 100));
  if (!(await featureBegun())) throw new Error('what looks after the feature never began');
  const token = (await readFile(join(data, 'slots', 'planner', 'token'), 'utf8')).trim();
  const planned = await fetch(`${url}/api/records/shop-features-1`, {
    method: 'PATCH',
    headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
    body: JSON.stringify({ fields: { plan: PLAN } }),
  });
  if (!planned.ok) throw new Error(`the agent could not write the plan: ${await planned.text()}`);
  await cli('record', 'set', 'shop-features-1', 'status=planned');
  // One with what a project usually starts with, which is a board and a fleet and no features; and one with a board alone.
  await cli('project', 'create', 'Works');
  await cli('project', 'create', '--bare', 'Ledger');
  await cli('apply', 'std/board', '--project', 'ledger');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1500, height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(page);
  const nav = page.locator('.nav');
  const twist = nav.getByRole('button', { name: 'Unfold Shop', exact: true });
  if (await twist.count()) await twist.click();
  await nav.getByRole('button', { name: /Features$/ }).click();

  // The features are a board of their own: a column for what becomes of one.
  const columns = page.locator('.board .column');
  await columns.first().waitFor();
  const heads = (await page.locator('.board .column-head .grow').allInnerTexts()).map((one) => one.trim().toLowerCase());
  expect(`a column for each thing that becomes of a feature, got ${heads.join()}`, heads.join() === 'backlog,planned,building,done');
  expect('the planned one to be in its column', (await columns.nth(1).locator('.card', { hasText: 'A shopping cart' }).count()) === 1);
  expect('the other to be where features begin', (await columns.nth(0).locator('.card', { hasText: 'Gift wrapping' }).count()) === 1);
  const tabs = (await page.getByRole('tablist', { name: 'Views' }).getByRole('tab').allInnerTexts()).map((one) => one.trim());
  expect(`its views to be the board and a list, got ${tabs.join()}`, tabs.join() === 'Features,List');
  await page.screenshot({ path: `${out}/features-0-board.png` });

  // Read whole, a feature shows its plan as the words it is, and something can be said on it.
  await columns.nth(1).locator('.card', { hasText: 'A shopping cart' }).click();
  const drawer = drawerAtRest(page);
  await drawer.getByLabel('Plan', { exact: true }).waitFor();
  expect('its plan to be shown as the words it is', (await drawer.getByLabel('Plan', { exact: true }).inputValue()).startsWith('First what a cart holds'));
  // What is said on it is under Comments, in the one row of tabs at the top of its form.
  await drawer.getByRole('tab', { name: 'Comments', exact: true }).click();
  await drawer.getByLabel('Say something on this feature').fill('Should the cart remember what was in it?');
  await drawer.getByRole('button', { name: 'Say', exact: true }).click();
  await drawer.locator('.journey-words', { hasText: 'Should the cart remember' }).waitFor();
  await page.screenshot({ path: `${out}/features-1-read.png` });
  await drawer.getByRole('button', { name: 'Close' }).click();

  // Carried to where features are built, it is accepted: its plan's tickets become cards.
  await columns.nth(1).locator('.card', { hasText: 'A shopping cart' }).dragTo(columns.nth(2));
  await columns.nth(2).locator('.card', { hasText: 'A shopping cart' }).waitFor();
  // Its cards are added, then each is told what it waits on, then the one that waits on nothing is made
  // ready, which puts it after the other where cards are listed by column. So what is waited for is the
  // second saying what it waits on, and each is found by what it is called, wherever it is by then: read
  // as soon as there were two, the second had not been told yet, or the two were listed the other way round.
  let cards = [];
  const called = (title) => cards.find((one) => one.title === title);
  const placed = () => cards.length >= 2 && (called("The cart's page")?.more?.after ?? []).length > 0;
  for (let tries = 0; tries < 100 && !placed(); tries += 1) {
    cards = JSON.parse((await cli('board', 'shop', '--json')).stdout).items ?? [];
    if (!placed()) await page.waitForTimeout(100);
  }
  const [first, second] = [called("The cart's model"), called("The cart's page")];
  expect(`the feature to have come to two cards, got ${cards.map((one) => one.title).join()}`, cards.length === 2 && !!first && !!second);
  expect(`the second to wait on the first, got ${JSON.stringify(second.more?.after)}`, JSON.stringify(second.more?.after) === JSON.stringify([first.id]));
  await page.screenshot({ path: `${out}/features-2-accepted.png` });

  // And as a table, where a view of it was declared.
  await page.getByRole('tablist', { name: 'Views' }).getByRole('tab', { name: 'List', exact: true }).click();
  await page.locator('.grid tbody tr', { hasText: 'Gift wrapping' }).waitFor();
  await page.screenshot({ path: `${out}/features-3-list.png` });

  // A card it came to says, in its drawer and its history, that the factory's bot added it from what the agent
  // wrote, by way of the features' blueprint, which is said after as detail: not Ada, who applied the features and
  // accepted the plan, and lent only the authority, and not the blueprint as who did it.
  await page.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }).click();
  await page.locator('.board .card', { hasText: "The cart's model" }).click();
  const card = drawerAtRest(page);
  const from = 'conductor added it to backlog from what planner wrote';
  // What happened to it is under History, beside what was said on it.
  await card.getByRole('tab', { name: /^History/ }).click();
  const addedLine = card.locator('.journey li', { hasText: from });
  await addedLine.waitFor();
  const said = await addedLine.locator('.journey-said').innerText();
  expect(`the history to say the bot did it, the blueprint after it, got "${said}"`, said === `${from} · via std/feature`);
  expect('the blueprint to be drawn as detail', (await addedLine.locator('.journey-via.faint').innerText()).trim() === '· via std/feature');
  const added = await card.locator('p.faint', { hasText: /added .* by / }).innerText();
  expect(`the drawer to say who added it, got ${added}`, added.endsWith('by conductor from what planner wrote · via std/feature'));
  expect('nobody but the bot and the agent to be said to have added it', (await card.locator('.journey li', { hasText: 'Ada added it' }).count()) === 0 && (await card.locator('.journey li', { hasText: 'std/feature added it' }).count()) === 0);
  await page.screenshot({ path: `${out}/features-3-added-by.png` });
  await addedLine.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/features-3-added-history.png` });
  await card.getByRole('button', { name: 'Close' }).click();

  // ---- features given a project from its board's page
  const signIn = async (on) => {
    await on.getByLabel('Email').fill('ada@example.com');
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  const under = (id, on = page) => on.locator(`.nav [data-project="${id}"]`);
  // Its line has its menu after it for whoever may arrange the project (`views/outline.tsx`).
  const listsFeatures = (id, on = page) => under(id, on).locator(':scope > .tree > button.nav-item, :scope > .tree > .nav-line > button.nav-item', { hasText: /Features$/ });
  // The blueprints a project uses, as its list in the side panel says them: unfolded, so that one coming or going is seen.
  const uses = async (id, on = page) => {
    // The project unfolded by its arrow where it is folded, as everything of the panel starts.
    await unfold(on, under(id, on).locator(':scope > .nav-row > button.nav-twist'));
    const list = under(id, on).locator(':scope > .tree > div', { has: on.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item', { hasText: 'Blueprints' }) });
    const fold = list.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item');
    await fold.waitFor();
    if ((await fold.getAttribute('aria-expanded')) !== 'true') await fold.click();
    return list;
  };
  // What is drawn of the gear's menu on a page: its lines by their words, and where everything is.
  const head = (on = page) => {
    const gear = on.locator('.view-head .head-acts').getByRole('button', { name: 'More for this page', exact: true });
    const menu = on.locator('.row-menu');
    const line = menu.getByRole('menuitemcheckbox', { name: 'Features', exact: true });
    // A line that is pressed by what it is called and the faint words after it; one that only says something by its words.
    const lines = () =>
      menu.locator(':scope > *').evaluateAll((all) =>
        all.map((one) => (one.matches('button') ? [...one.querySelectorAll('.truncate, .hint')].map((part) => part.textContent).join(' ') : one.textContent).replace(/\s+/g, ' ').trim()),
      );
    return { gear, menu, line, lines };
  };
  const board = async (id, called, on = page) => {
    await on.evaluate((to) => (location.hash = to), `/board/${id}`);
    await on.getByRole('heading', { name: `${called} board`, level: 1 }).waitFor();
    await on.locator('.board .column').first().waitFor();
  };
  // Where the page under the menu is: nothing of it moves for the menu being opened or its switch pressed.
  const places = (on = page) =>
    on.evaluate(() => ['.view-head', '.board .column', '.nav'].map((one) => JSON.stringify(document.querySelector(one)?.getBoundingClientRect())).join(' '));
  // The menu by its boxes: on the screen, under its gear, every line inside it and none over another. A board's gear
  // holds its tables and its tools too, and where the screen has not the height for all of it under the gear the menu
  // scrolls in itself: the line that is held here and what it says are whole inside it as it opens, every line is
  // between its sides, and its last line is inside it once it is scrolled to its end.
  const framed = async (where, on = page) => {
    const { gear, menu } = head(on);
    const got = await menu.evaluate((box) => {
      const of = (one) => {
        const at = one.getBoundingClientRect();
        return { left: at.left, right: at.right, top: at.top, bottom: at.bottom };
      };
      const all = [...box.children];
      const held = all.findIndex((one) => one.matches('[role="menuitemcheckbox"]'));
      const first = { menu: of(box), lines: all.map(of), held, scrolls: box.scrollHeight - box.clientHeight, wide: window.innerWidth, tall: window.innerHeight, sideways: document.documentElement.scrollWidth - window.innerWidth };
      box.scrollTop = box.scrollHeight;
      const last = of(all.at(-1));
      box.scrollTop = 0;
      return { ...first, last };
    });
    const from = await gear.boundingBox();
    const told = JSON.stringify(got);
    expect(`the gear's menu to be on the screen ${where}: ${told}`, got.menu.left >= 0 && got.menu.top >= 0 && got.menu.right <= got.wide && got.menu.bottom <= got.tall);
    expect(`the gear's menu to be under its gear ${where}: ${told}`, got.menu.top >= from.y + from.height - 0.5);
    for (const [at, one] of got.lines.entries()) {
      // What is under the menu's foot is reached by scrolling it; the line held here and what it says are not.
      const shown = got.scrolls <= 0 || at === got.held || at === got.held + 1;
      expect(`every line of the menu to be inside it ${where}: ${told}`, one.left >= got.menu.left - 0.5 && one.right <= got.menu.right + 0.5 && one.top >= got.menu.top - 0.5 && (!shown || one.bottom <= got.menu.bottom + 0.5));
      for (const other of got.lines.slice(at + 1)) expect(`no line of the menu to lie over another ${where}: ${told}`, one.bottom <= other.top + 0.5 || other.bottom <= one.top + 0.5);
    }
    expect(`the menu's last line to be inside it once it is scrolled to its end ${where}: ${told}`, got.last.top >= got.menu.top - 0.5 && got.last.bottom <= got.menu.bottom + 0.5);
    expect(`the page not to scroll sideways ${where}`, got.sideways <= 0);
  };
  const SAYS = 'Work bigger than a card, and cards put in order together: an agent says how it would be done and which cards wait on which, and you accept it.';
  const ON = 'On for this project. Stopping its blueprint, under Activity, ends what it does and keeps what it made.';

  // Somebody who may not apply a blueprint, on a page of their own: what they hold, as the daemon says it, without that.
  const reader = await browser.newContext({ viewport: { width: 1500, height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const lesser = await reader.newPage();
  lesser.on('pageerror', (err) => problems.push(err.message));
  await lesser.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const who = await answer.json();
    who.principal.scopes = ['projects', 'records', 'board', 'channels', 'search', 'blueprints:read', 'fleet:read', 'agents:read'];
    await route.fulfill({ response: answer, json: who });
  });
  await lesser.goto(`${url}/#/board/works`);
  await signIn(lesser);
  const theirPanel = lesser.getByRole('button', { name: 'Resources', exact: true });
  if ((await theirPanel.getAttribute('aria-pressed')) !== 'true') await theirPanel.click();
  await unfoldAll(lesser);
  await board('works', 'Works', lesser);
  const theirUses = await uses('works', lesser);
  const theirs = head(lesser);
  await theirs.gear.click();
  await theirs.menu.waitFor();
  expect(`somebody who may not apply to be offered no line for what the project has not got; got ${(await theirs.lines()).join(' | ')}`, (await theirs.line.count()) === 0 && !(await theirs.lines()).some((one) => /Features/.test(one)));
  await lesser.keyboard.press('Escape');
  await theirs.menu.waitFor({ state: 'detached' });

  // Works has a board and a fleet and no features: the line is there, off, and says what they are.
  await board('works', 'Works');
  expect('Works to list no features before it has any', (await listsFeatures('works').count()) === 0);
  const { gear, menu, line, lines } = head();
  const before = await places();
  await gear.click();
  await line.waitFor();
  expect('the line to be a switch that is off and can be pressed', (await line.getAttribute('aria-checked')) === 'false' && (await line.getAttribute('aria-disabled')) === null);
  const offered = await lines();
  const at = offered.indexOf('Features');
  expect(
    `the gear to offer features after the page's address, saying what they are, and the project's card times and A/B testing after them and before what every table has; got ${offered.join(' | ')}`,
    at === 1 && offered[0] === 'Copy link' && offered[2] === SAYS && offered[3] === 'Card times' && /^How long each card took/.test(offered[4]) && offered[5] === 'A/B testing' && /^A card worked several ways at once/.test(offered[6]) && /^Export as CSV/.test(offered[7]),
  );
  await framed('with the line off');
  expect('nothing of the page to move for the menu being opened', (await places()) === before);
  await page.screenshot({ path: `${out}/features-4-gear-off.png` });

  // Pressed, the project has its features: the switch is on with the menu still open, the side panel
  // lists them, and the page was not read again.
  await page.evaluate(() => (window.asItWas = true));
  await line.click();
  await menu.locator('[role="menuitemcheckbox"][aria-checked="true"]', { hasText: 'Features' }).waitFor();
  await listsFeatures('works').waitFor();
  expect('the page not to have been read again', await page.evaluate(() => window.asItWas === true));
  expect('nothing to be said to have gone wrong', (await page.locator('.toast.bad').count()) === 0);
  await framed('with the line just turned on');
  expect('nothing of the page to move for the switch being pressed', (await places()) === before);
  const fromTerminal = JSON.parse((await cli('instances', '--project', 'works', '--json')).stdout).instances.filter((one) => one.blueprint === 'std/features');
  expect(`one instance of the features' blueprint to be alive for Works, told where its fleet reports; got ${JSON.stringify(fromTerminal.map((one) => one.params))}`, fromTerminal.length === 1 && fromTerminal[0].params.channel === 'works' && fromTerminal[0].params.project === 'works');
  await page.screenshot({ path: `${out}/features-5-gear-turned-on.png` });
  // Pressed again where the menu was opened while it was off, nothing is applied: it says that it is on.
  await line.click();
  await page.locator('.toast', { hasText: ON }).waitFor();
  expect('the switch to stay on', (await line.getAttribute('aria-checked')) === 'true');
  // What it said is gone before anything else is looked at: it is said of this project and no other.
  await page.locator('.toast', { hasText: ON }).waitFor({ state: 'detached' });
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });

  // Opened again it is drawn as on, and is not turned off there: it says where that is done.
  await gear.click();
  await line.waitFor();
  expect('the line of a project that has its features to be on and closed', (await line.getAttribute('aria-checked')) === 'true' && (await line.getAttribute('aria-disabled')) === 'true');
  expect(`the closed line to say where it is turned off, got ${await line.getAttribute('title')}`, /stopped under Activity/.test((await line.getAttribute('title')) ?? ''));
  expect(`the menu to say that it is on and what stopping it comes to; got ${(await lines()).join(' | ')}`, (await lines())[2] === ON);
  await line.click({ force: true });
  expect('a press on it to change nothing and leave the menu open', (await line.getAttribute('aria-checked')) === 'true' && (await menu.count()) === 1);
  await framed('with the line on');
  await page.screenshot({ path: `${out}/features-6-gear-on.png` });
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  expect('one instance of it still, however often the line was pressed', JSON.parse((await cli('instances', '--project', 'works', '--all', '--json')).stdout).instances.filter((one) => one.blueprint === 'std/features').length === 1);

  // Somebody who may not apply is told that it is on, on a page that followed it, and has nothing to press.
  await listsFeatures('works', lesser).waitFor();
  await theirUses.getByText('std/features', { exact: true }).waitFor();
  await theirs.gear.click();
  await theirs.menu.waitFor();
  expect(`somebody who may not apply to be told the project has its features and offered no switch; got ${(await theirs.lines()).join(' | ')}`, (await theirs.line.count()) === 0 && (await theirs.lines())[1] === 'Features: on for this project.');
  await framed('for somebody who may not apply', lesser);
  await lesser.screenshot({ path: `${out}/features-7-gear-told.png` });
  await reader.close();

  // What looks after the features is stopped where everything running is. The table and what is in it
  // stay, the line is off again, and pressed it gives the project its features back over what it kept.
  const used = await uses('works');
  await used.getByText('std/features', { exact: true }).waitFor();
  await cli('record', 'add', 'works', 'features', 'title=A loyalty card', 'status=building');
  await cli('stop', fromTerminal[0].id);
  await used.getByText('std/features', { exact: true }).waitFor({ state: 'detached' });
  expect('the features to be listed still once what looks after them is stopped', (await listsFeatures('works').count()) === 1);
  await gear.click();
  await line.waitFor();
  expect('the line to be off again, and pressed', (await line.getAttribute('aria-checked')) === 'false' && (await line.getAttribute('aria-disabled')) === null);
  // Pressed twice before the first press is answered, the daemon is asked once: its answer is held
  // here until the second press has been made, so that the two are in that order on any machine.
  let asked = 0;
  let reached;
  let letGo;
  const begun = new Promise((there) => (reached = there));
  const held = new Promise((go) => (letGo = go));
  await page.route('**/api/projects/works/starts', async (route) => {
    asked += 1;
    reached();
    await held;
    await route.continue();
  });
  await line.click();
  await begun;
  await line.click();
  expect('the switch not to say it is on before the daemon has answered', (await line.getAttribute('aria-checked')) === 'false');
  // A menu opened meanwhile has the line closed, saying that it is being given, and a press there asks nothing.
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  await gear.click();
  await line.waitFor();
  expect(
    `the line to be closed while it is being given, saying so; got ${await line.getAttribute('title')}`,
    (await line.getAttribute('aria-disabled')) === 'true' && (await line.getAttribute('title')) === 'It is being given: its blueprint is being applied.',
  );
  await line.click({ force: true });
  letGo();
  await menu.locator('[role="menuitemcheckbox"][aria-checked="true"]', { hasText: 'Features' }).waitFor();
  await used.getByText('std/features', { exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  // Read once the answer is in and what came of it is listed: a second asking would have reached the daemon long since.
  const alive = JSON.parse((await cli('instances', '--project', 'works', '--json')).stdout).instances.filter((one) => one.blueprint === 'std/features');
  expect(`the daemon to have been asked once for three presses, and one instance of it to be alive; asked ${asked} times, ${alive.length} alive`, asked === 1 && alive.length === 1);
  await page.unroute('**/api/projects/works/starts');
  const kept = JSON.parse((await cli('record', 'list', 'works', 'features', '--json')).stdout).records ?? [];
  expect(`the feature written before to be there still, got ${kept.map((one) => one.data?.title).join()}`, kept.length === 1 && kept[0].data?.title === 'A loyalty card');
  expect('nothing to be said to have gone wrong', (await page.locator('.toast.bad').count()) === 0);

  // Ledger has a board and no fleet: the line is closed, and says what is lacking where it would say what features are.
  await board('ledger', 'Ledger');
  await gear.click();
  await line.waitFor();
  const why = 'A feature is planned by one of the fleet and comes to cards of the board: a fleet is not running for this project.';
  expect('the line of a project with no fleet to be off and closed', (await line.getAttribute('aria-checked')) === 'false' && (await line.getAttribute('aria-disabled')) === 'true');
  expect(`it to say why, to whoever points at it and in the menu; got ${await line.getAttribute('title')} and ${(await lines()).join(' | ')}`, (await line.getAttribute('title')) === why && (await lines())[2] === why);
  await line.click({ force: true });
  expect('a press on it to give the project nothing', (await line.getAttribute('aria-checked')) === 'false' && (await listsFeatures('ledger').count()) === 0);
  await framed('with the line closed');
  await page.screenshot({ path: `${out}/features-8-gear-no-fleet.png` });
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });

  // Narrower, and at a phone's width: the menu is on the screen with the line and what it says whole inside it.
  for (const width of [600, 390]) {
    await page.setViewportSize({ width, height: 844 });
    for (const [id, called, on] of [
      ['works', 'Works', 'true'],
      ['ledger', 'Ledger', 'false'],
    ]) {
      await page.evaluate((to) => (location.hash = to), `/board/${id}`);
      await page.getByRole('heading', { name: `${called} board`, level: 1 }).waitFor();
      await gear.click();
      await line.waitFor();
      expect(`the line to say how ${called} stands at ${width} wide`, (await line.getAttribute('aria-checked')) === on && (await line.getAttribute('aria-disabled')) === 'true');
      await framed(`on the board of ${called} at ${width} wide`);
      if (width === 390) await page.screenshot({ path: `${out}/features-9-gear-phone-${id}.png` });
      await page.keyboard.press('Escape');
      await menu.waitFor({ state: 'detached' });
    }
  }
  await page.setViewportSize({ width: 1500, height: 900 });

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
