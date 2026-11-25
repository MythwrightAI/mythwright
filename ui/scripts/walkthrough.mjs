// Walks the first-run story in a real browser and saves a picture of each
// step. This is how the client is looked at when there is no screen.
//
//   DAEMON=../target/debug/factory node scripts/walkthrough.mjs [out-dir]
//       starts a fresh daemon of its own, walks, and stops it
//   LINK=<what `mw open --link` said> node scripts/walkthrough.mjs [out-dir] http://127.0.0.1:7711
//       walks a daemon that is already running; it must be fresh (nothing
//       made in it but its owner) and started with `--adapter builtin`
//
// Environment:
//   BROWSER        a Chromium binary; defaults to the newest headless shell
//                  Playwright has downloaded on this machine
//   BROWSER_LIBS   directories of shared libraries the browser needs, for
//                  machines where they are not installed system-wide
//   BROWSER_FLAGS  extra flags handed to the browser, e.g. "--disable-gpu"
//   SCHEME         "light" to take the pictures in the light theme
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { openAdding } from './adding.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { gearLine, gearOf, headButtons } from './gear.mjs';
import { pickColour } from './picked.mjs';
import { saysWhere } from './where.ts';

/** Turns the side panel to a view on the rail, unless it is showing already: Resources with what it has read unfolded, as everything there starts folded (`folded.mjs`). */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  if (name === 'Resources') await unfoldAll(page);
}


const out = resolve(process.argv[2] ?? 'shots');
const flags = (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean);

async function findBrowser() {
  if (process.env.BROWSER) return process.env.BROWSER;
  const cache = join(homedir(), '.cache/ms-playwright');
  const shells = (await readdir(cache)).filter((name) => name.startsWith('chromium_headless_shell')).sort();
  if (shells.length === 0) throw new Error('no browser found; set BROWSER to a Chromium binary');
  return join(cache, shells.at(-1), 'chrome-headless-shell-linux64/chrome-headless-shell');
}

/** Starts a daemon with nothing in it and waits until it is listening: in a home of its own, whose command line the daemon's first start signs in as the owner it made. */
async function startDaemon(binary) {
  const data = join(out, 'daemon-data');
  const home = join(out, 'daemon-home');
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
  await mkdir(home, { recursive: true });
  const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
  const child = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { env, stdio: ['ignore', 'pipe', 'inherit'] });
  endsWithThis(child);
  const url = await saysWhere(child);
  // The owner the start made is named, as the story's Ada; her workspace follows.
  await promisify(execFile)(binary, ['register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse', '--url', url], { env });
  return {
    url,
    // What signs the browser in: a link the command line asks for, good once.
    link: async () => (await promisify(execFile)(binary, ['open', '--link', '--url', url], { env })).stdout.trim(),
    stop: async () => {
      child.kill('SIGTERM');
      await new Promise((gone) => child.on('exit', gone));
      for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
    },
  };
}

await mkdir(out, { recursive: true });
const daemon = process.env.DAEMON ? await startDaemon(process.env.DAEMON) : null;
const base = daemon?.url ?? process.argv[3] ?? 'http://127.0.0.1:7700';

const browser = await chromium.launch({
  executablePath: await findBrowser(),
  args: flags,
  env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
});
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, colorScheme: process.env.SCHEME === 'light' ? 'light' : 'dark' });
const problems = [];
page.on('pageerror', (err) => problems.push(`page error: ${err.message}`));
page.on('console', (message) => {
  if (message.type() === 'error') problems.push(`console: ${message.text()}`);
});

let step = 0;
async function shot(name) {
  step += 1;
  await page.waitForTimeout(250);
  await page.screenshot({ path: join(out, `${String(step).padStart(2, '0')}-${name}.png`) });
  console.log(`${step}. ${name}`);
}
const say = async (text) => {
  await page.getByLabel('Message').fill(text);
  await page.keyboard.press('Enter');
};

// 3-4. Nobody registers: the address alone says to run `mw open`, whose link opens the app signed in as the
// owner the daemon's first start made.
await page.goto(base);
await page.locator('.opens-with').waitFor();
await shot('greeted-by-mw-open');
await page.goto(daemon ? await daemon.link() : process.env.LINK);
await page.getByText('Setup Guide').waitFor();
await shot('start-page');

// 5. First channel, and an agent brought into it.
await side(page, 'Channels');
await page.getByRole('button', { name: 'New channel' }).click();
await page.getByLabel('Name').fill('general');
await page.getByRole('button', { name: 'Create', exact: true }).click();
await page.getByRole('heading', { name: '# general' }).waitFor();
await openAdding(page);
await page.getByRole('radio', { name: /^An agent/ }).check();
await shot('add-an-agent');
await page.getByRole('button', { name: 'Add agent', exact: true }).click();
await page.getByText('builder is listening').waitFor();

// 6-7. Ask it for a project; it appears under Resources.
await say('Please set up a new project called Riverside Shop');
await side(page, 'Resources');
await page.locator('.nav').getByText('Riverside Shop').waitFor();
await page.locator('.messages .avatar.agent').first().waitFor();
await shot('project-scaffolded');

// The project's name opens its own page. An agent adds a project alone, and nobody said a folder for
// it: nothing is listed under it yet, no Repo that nobody asked for among them, and there is no
// arrow that would unfold nothing.
await page.locator('.nav').getByText('Riverside Shop', { exact: true }).click();
await page.getByRole('heading', { name: 'Riverside Shop', level: 1 }).waitFor();
await page.locator('.tile', { hasText: 'Board' }).waitFor();
await page.locator('iframe.page-frame').waitFor();
await page.waitForTimeout(400);
await shot('project-home');
const nav = page.locator('.nav');
const theirs = nav.locator('[data-project="riverside-shop"]');
if ((await nav.getByText('Repo', { exact: true }).count()) !== 0) problems.push('a project nobody said a folder for has a Repo listed under it');
if ((await theirs.locator(':scope > .tree > *').count()) !== 0) problems.push('a project that was added alone has something listed under it');
if ((await page.getByRole('button', { name: /^(Fold|Unfold) Riverside Shop$/ }).count()) !== 0) problems.push('a project with nothing under it has an arrow that would unfold nothing');
// Nothing stands between a project and what it has: no line there says what kind of project it is.
if ((await nav.getByText('App', { exact: true }).count()) !== 0) problems.push('a project is said to have an app');

// A project has no board until a blueprint makes one. Until then there is
// no board to go to under it, and its own page says so.
if ((await nav.getByText('Board', { exact: true }).count()) !== 0) problems.push('a project with no board has one listed under it');
await page.locator('.tile', { hasText: 'Board' }).click();
await page.getByText('Riverside Shop has no board yet').waitFor();
await shot('no-board-yet');

// Its files are the folder the factory made for it, reached from its own page.
await theirs.locator(':scope > .nav-row .nav-item').click();
await page.getByRole('button', { name: 'Its repo', exact: true }).click();
await page.getByRole('heading', { name: 'Riverside Shop repo', level: 1 }).waitFor();
await page.locator('.files tbody').getByText('README.md').first().waitFor();
if ((await nav.getByText('Repo', { exact: true }).count()) !== 0) problems.push('opening the files of a project put a Repo line under it');

// A first blueprint, by hand. The project uses none yet, so nothing under
// it is called Blueprints: every blueprint there is, is listed once, in a
// part of the panel of their own, each family under its name. The one
// that makes a board is among those that ship.
// Read it, apply it, say which project it is for, and the board is there.
const lists = nav.getByText('Blueprints', { exact: true });
if ((await lists.count()) !== 1) problems.push(`a project that uses no blueprint has a list of them, or the factory has none: ${await lists.count()} lists`);
await nav.getByText('std', { exact: true }).waitFor();
const factoryNames = await page.locator('.nav > h3, .nav > .tree > *').evaluateAll((all) => all.map((one) => (one.querySelector('.truncate') ?? one.firstChild ?? one).textContent.trim()));
// Under their heading the Reference comes first, which says what any of them can say, and then the families: the
// box that narrows them is the panel's, over everything it lists.
// The projects come first, then the factory's, whose last line is its Trash (`SECTIONS`): the blueprints are last.
const projectsAt = factoryNames.indexOf('Projects');
const trashAt = factoryNames.indexOf('Trash', factoryNames.indexOf('Factory', projectsAt));
const inOrder = [trashAt, ...['Blueprints', 'Reference', 'std'].map((name) => factoryNames.indexOf(name, trashAt))];
if (projectsAt < 0 || inOrder.some((at, nth) => at < 0 || (nth > 0 && at !== inOrder[nth - 1] + 1))) problems.push(`the blueprints are not in a part of their own after the projects and the factory's: ${factoryNames.join(', ')}`);
// Unfolded by its arrow, where Expand all has not unfolded it already.
await unfold(page, nav.getByRole('button', { name: 'std', exact: true }));
await nav.getByText('std/board', { exact: true }).waitFor();
if ((await nav.getByText('std/watch', { exact: true }).count()) !== 1) problems.push('the blueprints that ship are not all listed for the factory');
await shot('blueprints-there-are');
await nav.getByText('std/board', { exact: true }).click();
// A blueprint that ships opens on what it says in plain words; its text is one press from there.
await page.locator('.summary[data-of="std/board"]').waitFor();
await shot('board-blueprint-in-words');
await page.getByRole('tab', { name: 'Text' }).click();
await page.locator('.cm-content').waitFor();
await shot('board-blueprint');
await page.getByRole('button', { name: 'Apply', exact: true }).click();
// What it is applied with is asked in the drawer beside the text.
const asks = drawerAtRest(page);
await asks.locator('.applying').waitFor();
// Opened from the factory's list it is no project's yet, and nothing is
// chosen for it: it cannot be applied until somebody says which.
const which = asks.locator('select');
if ((await which.inputValue()) !== '') problems.push('a project is chosen for a blueprint before anybody chose one');
if (!(await which.locator('option').allInnerTexts()).some((one) => one.startsWith('No project'))) problems.push('a blueprint cannot be applied to no project');
await which.selectOption({ label: 'Riverside Shop' });
// What it is applied with is chosen, not typed. The columns are tags in a
// field; under it, while it has the caret, are the options: one is pressed
// to choose it or let it go, typed to make another, and dragged by its grip
// to change the order. Where the cards are kept is one of the places the
// blueprint names.
const columns = asks.getByRole('combobox', { name: 'columns' });
const field = asks.locator('.select', { has: page.getByRole('combobox', { name: 'columns' }) });
const tags = (within) => within.locator('.select-field .tag').allInnerTexts().then((all) => all.map((one) => one.replace(/\s*×\s*$/, '').trim()).join());
const options = field.getByRole('option');
if ((await tags(field)) !== 'backlog,ready,doing,review,done') problems.push(`the columns are not offered as tags: ${await tags(field)}`);
await columns.click();
await field.getByText('Select an option or create one').waitFor();
if ((await options.count()) !== 5 || (await field.locator('[aria-selected="true"]').count()) !== 5) problems.push('the options under the field are not the five columns, all chosen');
// Typing what is not there offers to make it.
await columns.fill('blocked');
await options.filter({ hasText: 'Create' }).waitFor();
await page.keyboard.press('Enter');
if ((await tags(field)) !== 'backlog,ready,doing,review,done,blocked') problems.push(`what was typed was not made and chosen, at the end: ${await tags(field)}`);
// Its cross takes it out of what is chosen, and it stays an option.
await field.getByRole('button', { name: 'Remove blocked' }).click();
if ((await tags(field)) !== 'backlog,ready,doing,review,done') problems.push(`a column's cross did not take it out: ${await tags(field)}`);
if ((await options.filter({ hasText: 'blocked' }).getAttribute('aria-selected')) !== 'false') problems.push('a column that was taken out is not kept as an option');
// The order is changed with Alt and an arrow on the one that is marked...
await columns.focus();
await options.filter({ hasText: 'ready' }).hover();
await page.keyboard.press('Alt+ArrowDown');
if ((await tags(field)) !== 'backlog,doing,ready,review,done') problems.push(`Alt and an arrow did not move a column along: ${await tags(field)}`);
await page.keyboard.press('Alt+ArrowUp');
// ...or by dragging one by its grip.
const grip = async (name) => {
  const box = await options.filter({ hasText: name }).locator('.select-grip').boundingBox();
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
};
const drag = async (name, onto) => {
  const from = await grip(name);
  const to = await grip(onto);
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(from.x, (from.y + to.y) / 2, { steps: 4 });
  await page.mouse.move(to.x, to.y, { steps: 4 });
  await page.mouse.up();
};
await drag('done', 'backlog');
if ((await tags(field)) !== 'done,backlog,ready,doing,review') problems.push(`dragging a column by its grip did not move it: ${await tags(field)}`);
await drag('done', 'review');
if ((await tags(field)) !== 'backlog,ready,doing,review,done') problems.push(`the columns could not be put back in order: ${await tags(field)}`);
// The options lie over what is under them; Escape puts them away, and only them.
await page.keyboard.press('Escape');
await field.getByText('Select an option or create one').waitFor({ state: 'detached' });
if ((await asks.count()) !== 1) problems.push('Escape in a field of the drawer put the drawer away');
// Where the cards are kept: one of the places there are, and choosing one lets the other go.
const store = asks.getByRole('combobox', { name: 'store' });
const kept = asks.locator('.select', { has: page.getByRole('combobox', { name: 'store' }) });
if ((await tags(kept)) !== 'local') problems.push(`the usual place for the cards is not the one chosen: ${await tags(kept)}`);
await store.click();
if ((await kept.getByRole('option').allInnerTexts()).map((one) => one.replace(/\s*✓\s*$/, '').trim()).join() !== 'local,files,service') problems.push('where the cards are kept is not offered as the places there are');
await kept.getByRole('option', { name: 'files' }).click();
if ((await tags(kept)) !== 'files') problems.push(`choosing a place did not let the other go: ${await tags(kept)}`);
await store.click();
await kept.getByRole('option', { name: 'local' }).click();
if ((await tags(kept)) !== 'local') problems.push(`the place could not be chosen back: ${await tags(kept)}`);
await columns.click();
await shot('board-blueprint-params');
await page.keyboard.press('Escape');
await field.getByText('Select an option or create one').waitFor({ state: 'detached' });
await asks.getByRole('button', { name: 'Apply', exact: true }).click();
// It is the project's now, and is shown as the project's: one tab of that
// name, saying whose it is, and the project lists it among what it uses.
await page.getByText('applies to Riverside Shop').waitFor();
if ((await page.locator('.tab', { hasText: 'std/board' }).count()) !== 1) problems.push('applying a blueprint from the factory\'s list left two tabs of the same name');
await page.getByRole('tab', { name: 'Node' }).click();
await page.locator('.gnode.status-ok[data-node="main/board"]').waitFor();
await shot('board-blueprint-applied');
await side(page, 'Resources');
await lists.nth(1).waitFor();
await unfold(page, nav.locator('[data-project] button.nav-item', { has: page.getByText('Blueprints', { exact: true }) }));
const uses = await nav.locator('[data-project] .tree .tree .nav-item', { hasText: /^◆/ }).allInnerTexts();
// The board, and what looks after its cards, which runs beside it.
if (uses.map((one) => one.replace(/^◆\s*/, '').trim()).join() !== 'std/board,std/cards') problems.push(`a project does not list what it uses, and only that: ${uses.join(', ')}`);
await shot('blueprints-a-project-uses');
// Those that ship are put away again: what follows is the project's.
await nav.getByText('std', { exact: true }).click();
await nav.getByText('std/watch', { exact: true }).waitFor({ state: 'detached' });
// The board is under the project now that there is one, without being
// asked for: with the columns that were chosen, and nothing on it yet.
await side(page, 'Resources');
// The arrow beside the project's name folds what is under it, now that something is.
await page.getByRole('button', { name: 'Fold Riverside Shop', exact: true }).click();
if ((await nav.getByText('Board', { exact: true }).count()) !== 0) problems.push('folding a project left what is under it showing');
await page.getByRole('button', { name: 'Unfold Riverside Shop', exact: true }).click();
await nav.getByText('Board', { exact: true }).click();
await page.locator('.column', { hasText: 'review' }).waitFor();
const made = (await page.locator('.column .column-head').allInnerTexts()).join(' ');
if (!/backlog[\s\S]*ready[\s\S]*doing[\s\S]*review[\s\S]*done/i.test(made) || /blocked/i.test(made)) problems.push(`the board does not have the columns that were chosen, in their order: ${made}`);
if ((await page.locator('.card').count()) !== 0) problems.push('a new board has a card on it that nobody put there');
await shot('board-from-blueprint');
await page.locator('.tab', { hasText: 'general' }).click();

// 8-9. Ask it for the fleet. The blueprint appears among the blueprints
// the project uses, under the project's Blueprints, by its name and ahead
// of what is only applied there. The factory keeps a project's own
// blueprints, so it is no file of the project's repository, whose page
// lists none of it.
await say('Now set up the fleet for Riverside Shop');
await side(page, 'Resources');
const blueprint = nav.locator('[data-project]').getByText('fleet', { exact: true });
await blueprint.waitFor();
await page.locator('.tab', { hasText: 'Riverside Shop repo' }).click();
await page.locator('.files tbody tr').first().waitFor();
if ((await page.locator('.files tbody').getByText(/\bfleet\./).count()) !== 0) problems.push("the fleet's blueprint is listed among the project's repository's files");
await page.locator('.tab', { hasText: 'general' }).click();
// Among what the project uses, which is under the project: a blueprint somebody lately opened is listed at the panel's top too.
const order = await nav.locator('[data-project] .nav-item', { hasText: /^◆/ }).allInnerTexts();
if (!/fleet$/.test(order[0] ?? '')) problems.push(`the project's own blueprint is not listed first: ${order.join(', ')}`);
await page.waitForTimeout(600);
await shot('fleet-blueprint-appears');

// 10-11. Open it: a document that reads like YAML, with Text and Node pills.
await blueprint.click();
await page.locator('.cm-content').waitFor();
await shot('blueprint-as-text');

// 12. The node view: the same blueprint as a graph; click a node for details.
await page.getByRole('tab', { name: 'Node' }).click();
await page.locator('.gnode').first().waitFor();
// Dragging the map moves it and selects nothing: selected, every word on
// it was painted over in the selection's colour for as long as it was held.
{
  const map = await page.locator('.canvas').boundingBox();
  // The map is drawn fitted to its place, so taking hold of it at once
  // and bringing it back leaves it where it was.
  const where = () => page.locator('.canvas svg > g').first().getAttribute('transform');
  const before = await where();
  await page.mouse.move(map.x + 40, map.y + 40);
  await page.mouse.down();
  await page.mouse.move(map.x + map.width - 40, map.y + map.height - 40, { steps: 6 });
  const held = await page.evaluate(() => String(window.getSelection()));
  await page.mouse.move(map.x + 40, map.y + 40, { steps: 6 });
  await page.mouse.up();
  if ((await where()) !== before) problems.push(`the node map, dragged and brought back, is not where it was: ${before} became ${await where()}`);
  if (held !== '') throw new Error(`dragging the node map selected its text: ${JSON.stringify(held.slice(0, 80))}`);
}
await shot('blueprint-as-graph');
// The project's fleet says where it is, in one step that calls
// the fleet that ships.
await page.locator('.gnode', { hasText: 'fleet' }).first().click();
await drawerAtRest(page).waitFor();
await shot('node-details');
await page.keyboard.press('Escape');

// Apply it: the graph shows each step as it goes.
await page.getByRole('button', { name: 'Apply', exact: true }).click();
// It asks for what the file says: where the fleet is, and what may be
// asked of or sent to the repository's remote. Who is in it is not here.
// What the drawer is left by stays in reach under whatever it asks.
const choices = drawerAtRest(page);
await choices.getByLabel(/^planned/).waitFor();
if ((await choices.getByLabel(/^reviewers/).count()) !== 0) problems.push('the fleet asks how many review as it is applied, which is a row of its table');
await shot('fleet-choices');
const foot = await choices.getByRole('button', { name: 'Apply', exact: true }).boundingBox();
if (!foot || foot.y + foot.height > 900) problems.push('the fleet\'s choices are more than fit, and Apply is out of reach under them');
// An apply is answered when its steps have run, and the graph has drawn
// them by then from what was said of each: on a busy machine the answer is
// in a second after the step is drawn as done. That is made so here, by
// holding the answer until the runs are being read: whoever has gone on to
// them by then is left there, and not taken back to the graph.
let answer;
const answered = new Promise((go) => (answer = go));
await page.route('**/api/apply', async (route) => {
  const response = await route.fetch();
  await answered;
  await route.fulfill({ response });
});
await choices.getByRole('button', { name: 'Apply', exact: true }).click();
// The step is a call that takes a moment: it is drawn as going while it
// goes, and as done when it is, and not left as going by what was read of
// the run meanwhile.
await page.locator('.gnode.status-ok', { hasText: 'fleet' }).waitFor();
await shot('applied-and-running');
await page.getByRole('tab', { name: /^Runs/ }).click();
await page.locator('table tbody tr', { hasText: 'fleet' }).first().click();
answer();
// The page says that it started once the answer is in, and has by then
// shown whatever the answer has it show.
await page.getByText('Started fleet.').waitFor();
await page.unroute('**/api/apply');
if ((await page.getByRole('tab', { name: /^Runs/ }).getAttribute('aria-pressed')) !== 'true' || (await page.locator('table tbody tr', { hasText: 'fleet' }).count()) === 0)
  throw new Error('the answer to an apply took the page back to the graph from the runs somebody had gone on to meanwhile');
await shot('runs-and-logs');

// The guide does not end in six steps done and nothing else. Help leads
// back to it, whatever is showing, and with every step done it says what
// is next, each a line with something to press that does it.
await page.getByRole('button', { name: /^Help/ }).click();
await page.getByRole('menu', { name: 'Help', exact: true }).getByRole('menuitem', { name: 'Setup Guide', exact: true }).click();
const whatNext = page.locator('[data-guide="next"]');
await whatNext.locator('[data-next]').first().waitFor();
const offeredNext = await whatNext.locator('[data-next]').evaluateAll((all) => all.map((one) => `${one.dataset.next}:${one.querySelector('button')?.innerText.trim()}`).join(' | '));
if (offeredNext !== 'card:Open the board | agent:Open its agents | experiment:Add it to Riverside Shop | device:Open Settings') problems.push(`with its steps done the guide offers "${offeredNext}"`);
if ((await page.locator('.badge', { hasText: /^(next|later)$/ }).count()) !== 0) problems.push('the guide says what is next while a step of it is still to do');
await shot('what-next');
const backToGuide = async () => {
  await page.locator('.tabs .tab', { hasText: 'Start' }).first().click();
  await whatNext.waitFor();
};
// The fleet's agents: the table whose rows say what they run on, as a page of its own.
await whatNext.locator('[data-next="agent"] button').click();
await page.locator('table.grid tr[data-row="riverside-shop-agents-2"]').waitFor();
if ((await page.locator('.drawer').count()) !== 0) problems.push('the agents were opened in the drawer, and the guide opens them as a page');
await backToGuide();
// The board, to add a card to.
await whatNext.locator('[data-next="card"] button').click();
await page.locator('.view-head h1', { hasText: /board$/ }).waitFor();
await gearOf(page).waitFor();
await backToGuide();
// The experiment: the example put among the project's own blueprints by the daemon, and opened; asked again it is only opened.
await whatNext.locator('[data-next="experiment"] button').click();
await page.locator('.tabs .tab', { hasText: 'ask.' }).waitFor();
await page.getByText("research is among Riverside Shop's blueprints.").waitFor();
await backToGuide();
const tried = whatNext.locator('[data-next="experiment"]');
await tried.locator('.badge.ok').waitFor();
if ((await tried.locator('button').innerText()).trim() !== 'Open it') problems.push('the example that was added is offered to be added again');
// And where a second device is invited.
await whatNext.locator('[data-next="device"] button').click();
await page.locator('.tabs .tab', { hasText: 'Settings' }).waitFor();
// Put away again, so that the story goes on from where it was.
await page.getByRole('button', { name: 'Close Settings', exact: true }).click();
await page.locator('.tabs .tab', { hasText: 'Settings' }).waitFor({ state: 'detached' });

// Who is in the fleet is a table of the project, there once the fleet is:
// a row for each kind of agent, changed where it is. It sits beside the
// board, and is opened from the gear of the board's head, beside it.
await side(page, 'Resources');
await nav.getByText('Board', { exact: true }).click();
await page.locator('.view-head h1', { hasText: /board$/ }).waitFor();
await gearLine(page, /^Agents/);
await drawerAtRest(page).locator('table.grid tr[data-row="riverside-shop-agents-3"]').waitFor();
const rows = (await drawerAtRest(page).locator('table.grid tbody tr[data-row] td.grid-title').allInnerTexts()).map((one) => one.replace(/\s*OPEN\s*$/i, '').trim());
if (rows.join() !== 'architect,worker,reviewer') problems.push(`the fleet's agents are first ${rows.join()}`);
await shot('fleet-agents');

// Beyond the story: every other view, in the state the story left.
await nav.getByText('Board', { exact: true }).click();
// What can be done to a board, how it is laid out among it, is lines of the gear of its head, which has no
// button of its own for them or for its tables.
const tool = (name) => gearLine(page, name);
if ((await headButtons(page)).some((one) => /^(Its tables|Tools)/.test(one))) problems.push(`the board's head still has a button for its tables or its tools: ${(await headButtons(page)).join(' | ')}`);
await tool('Add a card');
await page.getByPlaceholder('What needs doing?').fill('Add a shopping cart');
await page.keyboard.press('Enter');
await page.locator('.card', { hasText: 'Add a shopping cart' }).waitFor();
// One card is one card. And the board's five columns are all in the window
// at this width with the side panel open, each with its `+` inside it: the
// last was cut off at the window's edge, its `+` with it.
const counted = (await page.locator('.view-head .muted').first().innerText()).trim();
if (counted !== '1 card') problems.push(`a board with one card says "${counted}"`);
const columnsAt = await page.locator('.board > .column').evaluateAll((all) =>
  all.map((column) => {
    const box = column.getBoundingClientRect();
    const plus = column.querySelector('.column-head button[aria-label^="Add a card"]')?.getBoundingClientRect();
    return { left: box.left, right: box.right, plus: plus ? { left: plus.left, right: plus.right } : null };
  }),
);
if (columnsAt.length !== 5) problems.push(`the board has ${columnsAt.length} columns`);
for (const [nth, column] of columnsAt.entries()) {
  if (column.right > 1440 || column.left < 0) problems.push(`column ${nth + 1} of the board is not wholly in the window: ${JSON.stringify(column)}`);
  if (!column.plus || column.plus.left < column.left || column.plus.right > column.right) problems.push(`the + of column ${nth + 1} is not inside it: ${JSON.stringify(column)}`);
}
await shot('board');

// A board is a table underneath, and other views of it are saved beside
// the board, a tab each: the same cards, laid out another way. It starts
// with two: the board, and its cards by what comes first.
const views = page.getByRole('tablist', { name: 'Views' });
const tabs = async () => (await views.getByRole('tab').allInnerTexts()).map((one) => one.trim());
if ((await tabs()).join() !== 'Board') problems.push(`a board starts with the views ${await tabs()}`);
const addView = async (name) => {
  await page.getByRole('button', { name: 'Add a view' }).click();
  await page.getByRole('menuitem', { name: new RegExp(`^${name}`) }).click();
  await views.getByRole('tab', { name, exact: true, selected: true }).waitFor();
};
const closeLayout = async () => {
  await page.keyboard.press('Escape');
  await page.getByRole('dialog', { name: 'Layout' }).waitFor({ state: 'detached' });
};
await addView('Table');
const heads = (await page.locator('.grid thead th[data-column]').allInnerTexts()).map((one) => one.trim());
// A card's own, and with them what the standard board keeps of a card besides: the cards it waits on, how often it was sent back, what is asked on it, what it waits for. What is kept on its row for whatever looks after the cards (who asked, where it was taken from, what its agent is next to be told) the board says is left out of a view to begin with.
const columnsAtFirst = 'Title,Priority,Status,Assignee,After,Fixes,Labels,Due,Rounds,Body,Part of,Plan,Question,Waiting';
if (heads.join() !== columnsAtFirst) problems.push(`the table's columns are ${heads.join()}`);
// How a table is drawn is said where its layout is chosen, and moves nothing under it.
await tool(/^Layout/);
const lined = () => page.locator('.grid').evaluate((grid) => grid.classList.contains('lined'));
if (!(await lined())) problems.push('a table starts without its vertical lines');
await page.getByRole('switch', { name: 'Show vertical lines' }).click();
if (await lined()) problems.push('turning the vertical lines off left them drawn');
await page.getByRole('switch', { name: 'Show vertical lines' }).click();
// Which columns the view shows is said there too. One the board leaves out is a switch away, a
// column that is shown is left out the same way, and the view comes back as it was left.
const columnsShown = page.getByRole('group', { name: 'Columns shown' });
const headsNow = async () => (await page.locator('.grid thead th[data-column]').allInnerTexts()).map((one) => one.trim()).join();
if (await columnsShown.getByRole('switch', { name: 'Taken' }).isChecked()) problems.push('what the board says is left out of a view starts shown');
if ((await columnsShown.getByRole('switch', { name: 'Title' }).count()) !== 0) problems.push('what a card is known by can be left out of a view');
await columnsShown.getByRole('switch', { name: 'Taken' }).click();
await page.locator('.grid thead th', { hasText: 'Taken' }).waitFor();
await columnsShown.getByRole('switch', { name: 'Due' }).click();
await page.locator('.grid thead th', { hasText: 'Due' }).waitFor({ state: 'detached' });
if ((await headsNow()) !== 'Title,Priority,Status,Assignee,After,Fixes,Labels,Rounds,Body,Part of,Plan,Question,Taken,Waiting') problems.push(`with a column shown and one left out the table's columns are ${await headsNow()}`);
await shot('board-table-columns');
await closeLayout();
await page.reload();
await page.locator('.grid thead th', { hasText: 'Taken' }).waitFor();
if ((await headsNow()) !== 'Title,Priority,Status,Assignee,After,Fixes,Labels,Rounds,Body,Part of,Plan,Question,Taken,Waiting') problems.push(`read again, the view does not show what it was left showing: ${await headsNow()}`);
await tool(/^Layout/);
// Put back, both switches pressed in the time one of them takes to be
// saved, which on a busy machine is how any two are pressed: the answer
// to the first save is held until the second switch has been pressed.
// The second is still worked out from the view as the first left it.
{
  let saved;
  const kept = new Promise((go) => (saved = go));
  let saves = 0;
  await page.route('**/api/collections/*/board/views', async (route) => {
    saves += 1;
    if (saves > 1) return route.continue();
    const response = await route.fetch();
    await kept;
    await route.fulfill({ response });
  });
  await columnsShown.getByRole('switch', { name: 'Taken' }).click();
  await columnsShown.getByRole('switch', { name: 'Due' }).click();
  saved();
  await page.locator('.grid thead th', { hasText: 'Due' }).waitFor();
  await page.locator('.grid thead th', { hasText: 'Taken' }).waitFor({ state: 'detached' }).catch(() => {});
  await page.unroute('**/api/collections/*/board/views');
  if (saves !== 2) problems.push(`two switches pressed came to ${saves} saves of the view`);
}
if ((await headsNow()) !== columnsAtFirst) problems.push(`put back, the table's columns are ${await headsNow()}`);
await closeLayout();
// A cell is pressed to change it, by what its field is: a choice is chosen,
// labels are chosen or made, a day is picked, and what a card is called is typed.
const line = page.locator('.grid tbody tr', { hasText: 'Add a shopping cart' });
await line.getByRole('button', { name: /^Status of/ }).click();
await page.getByRole('option', { name: 'doing' }).click();
await line.locator('.tag', { hasText: 'doing' }).waitFor();
await line.getByRole('button', { name: /^Labels of/ }).click();
await page.getByRole('combobox', { name: 'Labels' }).fill('shop');
await page.waitForTimeout(150);
await page.keyboard.press('Enter');
await gearOf(page).focus();
await line.locator('.tag', { hasText: 'shop' }).waitFor();
const now = new Date();
const due = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-15`;
await line.getByRole('button', { name: /^Due of/ }).click();
await page.locator('.grid tbody input[type=date]').fill(due);
await page.keyboard.press('Enter');
await line.getByRole('button', { name: /^Due of/ }).filter({ hasText: '15' }).waitFor();
// A new row is a new card.
await page.getByRole('button', { name: '+ New card' }).click();
await page.locator('.grid tbody').getByRole('textbox', { name: 'Title' }).fill('Write the returns policy');
await page.keyboard.press('Enter');
await page.locator('.grid tbody tr', { hasText: 'Write the returns policy' }).waitFor();
if ((await page.locator('.grid-count').innerText()).trim() !== '2 cards') problems.push(`the table counts ${await page.locator('.grid-count').innerText()}`);
// A column's head is pressed for what can be done with the column, and sorting by it is a line of that menu.
await page.locator('.grid thead th', { hasText: 'Title' }).getByRole('button', { name: 'Title', exact: true }).click();
await page.locator('.headmenu').getByRole('menuitemcheckbox', { name: 'Sort ascending' }).click();
await page.locator('.headmenu').waitFor({ state: 'detached' });
if (!(await page.locator('.grid tbody tr').first().innerText()).includes('Add a shopping cart')) problems.push('sorting by title did not put the rows in its order');
await shot('board-as-table');

// A calendar of the same cards, each on the day it is due; one with no day is under it.
await addView('Calendar');
await page.locator(`.cal-day[data-day="${due}"] .cal-card`, { hasText: 'Add a shopping cart' }).waitFor();
if ((await page.locator('.cal-nowhere .cal-card', { hasText: 'Write the returns policy' }).count()) !== 1) problems.push('a card with no day is not under the calendar');
await shot('board-as-calendar');
// A view is called what whoever saved it likes.
await tool(/^Layout/);
await page.getByLabel('What this view is called').fill('When');
await page.keyboard.press('Enter');
await views.getByRole('tab', { name: 'When', exact: true, selected: true }).waitFor();
await closeLayout();
// The views are kept with the board, not in this window: they are there when it is opened again.
await page.reload();
await views.getByRole('tab', { name: 'When', exact: true, selected: true }).waitFor();
if ((await tabs()).join() !== 'Board,Table,When') problems.push(`the views that were saved came back as ${await tabs()}`);
await page.locator(`.cal-day[data-day="${due}"] .cal-card`, { hasText: 'Add a shopping cart' }).waitFor();
// A view is taken away without touching what it was a view of.
await addView('Gallery');
await page.locator('.rows-card', { hasText: 'Add a shopping cart' }).locator('.tag', { hasText: 'shop' }).waitFor();
await shot('board-as-gallery');
await tool(/^Layout/);
await page.getByRole('button', { name: 'Remove this view' }).click();
await views.getByRole('tab', { name: 'Gallery' }).waitFor({ state: 'detached' });
// What was changed in the table is changed on the board: they are the same cards.
await views.getByRole('tab', { name: 'Board', exact: true }).click();
await page.locator('.column', { hasText: 'doing' }).locator('.card', { hasText: 'Add a shopping cart' }).waitFor();
await page.locator('.column', { hasText: 'backlog' }).locator('.card', { hasText: 'Write the returns policy' }).waitFor();

{
  // A card named in a message is pressed to see it, in the panel beside the
  // channel: its id is the link, written plainly or as code. What only looks
  // like one is not.
  await page.locator('.tabs').getByRole('tab', { name: /general/ }).click();
  const say = page.locator('.channel > .split').getByLabel('Message');
  await say.fill('look at riverside-shop-1 and `riverside-shop-2`, not utf-8');
  await page.keyboard.press('Enter');
  const told = page.locator('.message', { hasText: 'not utf-8' });
  await told.waitFor();
  const links = (await told.locator('.card-link').allInnerTexts()).map((one) => one.trim());
  if (links.join() !== 'riverside-shop-1,riverside-shop-2') problems.push(`the cards named in a message are ${links.join()}`);
  await told.locator('.card-link', { hasText: 'riverside-shop-1' }).click();
  await toolsAtRest(page).waitFor();
  const beside = page.getByRole('complementary', { name: 'Tools' });
  // The card's own form, as the drawer beside its board has it (`views/cardform.tsx`).
  const form = beside.locator('form.card-form');
  await form.getByLabel('Title', { exact: true }).waitFor();
  if ((await form.getByLabel('Title', { exact: true }).inputValue()) !== 'Add a shopping cart') problems.push('a card pressed in a message is not the one opened in the panel');
  if ((await beside.getByRole('tab', { name: 'Board' }).getAttribute('aria-selected')) !== 'true') problems.push('a card pressed in a message is not shown in the board tab of the panel');
  if ((await form.getByLabel(/^Column/).inputValue()) !== 'doing' || !(await form.innerText()).includes('In doing')) problems.push('the card in the panel does not say where it stands');
  // Under what the card says is its journey across the board, as it was kept: a line for each thing that happened to it,
  // under History, beside what was said on it.
  await beside.getByRole('tab', { name: /^History/ }).click();
  await beside.locator('.journey li', { hasText: 'moved it from backlog to doing' }).waitFor();
  const journey = (await beside.locator('.journey .journey-said').allInnerTexts()).map((one) => one.trim());
  if (journey[0] !== 'Ada added it to backlog' || !journey.includes('Ada changed its labels') || !journey.includes('Ada changed its due')) problems.push(`the card's history is ${journey.join(' | ')}`);
  await shot('card-from-a-message');
  // From there, back to the board in the panel, where every card is pressed the same way.
  await beside.getByRole('button', { name: 'Back to the board' }).click();
  await beside.locator('.context-row', { hasText: 'Write the returns policy' }).click();
  await form.getByLabel('Title', { exact: true }).waitFor();
  if ((await form.getByLabel('Title', { exact: true }).inputValue()) !== 'Write the returns policy') problems.push('a card pressed on the board in the panel is not the one opened');
  await beside.getByRole('button', { name: 'Back to the board' }).click();
  await page.getByRole('button', { name: 'Collapse the tools panel' }).click();
  await side(page, 'Resources');
  await nav.getByText('Board', { exact: true }).click();
  await page.locator('.column', { hasText: 'doing' }).locator('.card', { hasText: 'Add a shopping cart' }).waitFor();
}

// The board's columns are changed from the board, which keeps them as
// its own: its blueprint applied again keeps a column added there. And
// what the blueprint was applied with each time is kept, and can be read.
await tool('Add a column');
const shape = page.getByRole('dialog');
// It opens with the caret where the new column's name is typed.
await shape.getByText('Select an option or create one').waitFor();
await page.keyboard.type('blocked');
await shape.getByRole('option').filter({ hasText: 'Create' }).waitFor();
await page.keyboard.press('Enter');
await page.keyboard.press('Escape');
await shape.getByRole('button', { name: 'Apply', exact: true }).click();
await page.locator('.column', { hasText: 'blocked' }).waitFor();
if ((await page.locator('.card', { hasText: 'Add a shopping cart' }).count()) !== 1) problems.push('adding a column lost the card that was on the board');
// Applied again as it was, with the columns it now has said as its own, the board keeps the one added to it, where it was.
{
  const again = await page.evaluate(async () => {
    const headers = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const read = async (path) => (await fetch(path, { headers })).json();
    const tables = await read('/api/collections?project=riverside-shop');
    const holder = (tables.collections ?? tables).find((one) => one.name === 'board').owner;
    const held = await read(`/api/instances/${encodeURIComponent(holder)}`);
    const params = { ...held.instance.params, columns: ['backlog', 'ready', 'doing', 'review', 'done', 'blocked'] };
    const body = { project: 'riverside-shop', source: held.source, key: held.instance.key, params, wait: true };
    return (await fetch('/api/apply', { method: 'POST', headers, body: JSON.stringify(body) })).json();
  });
  if (again.finished?.status !== 'ok') problems.push(`the board's blueprint could not be applied again: ${JSON.stringify(again)}`);
  const kept = await page.evaluate(async () => (await (await fetch('/api/board/riverside-shop', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).columns);
  if (kept?.join() !== 'backlog,ready,doing,review,done,blocked') problems.push(`the board's blueprint applied again did not keep the columns as they were: ${JSON.stringify(kept)}`);
}
await tool('Arrange the columns');
await page.getByRole('dialog').getByRole('button', { name: 'Its blueprint and history' }).click();
await page.getByRole('tab', { name: 'History' }).click();
const versions = page.locator('.history-row');
await versions.nth(1).waitFor();
const told = (await versions.allInnerTexts()).map((one) => one.replace(/\s+/g, ' '));
// Each is called by the version its text says, which is the blueprint's and the same for both here, and by which
// time the board was applied: never by one word for the two, which read as a heading contradicting its own text.
if (!/^Blueprint version (\S+) running second time applied .*Ada.*columns: added blocked$/.test(told[0] ?? '') || !/^Blueprint version (\S+) first time applied .*what it was started with$/.test(told[1] ?? '')) problems.push(`the history does not say what each time it was applied changed: ${told.join(' | ')}`);
await versions.nth(1).click();
await page.getByRole('button', { name: 'Apply it like this again' }).waitFor();
// What the heading over the text says is the version the text under it says.
const headed = (await page.locator('.history-head .history-called').innerText()).trim();
// The text is drawn a moment after its heading.
const itsVersion = page.locator('.history-text .cm-line', { hasText: /^version:/ });
await itsVersion.waitFor({ timeout: 10_000 }).catch(() => {});
const says = /^version:\s*(\S+)/.exec((await itsVersion.count()) > 0 ? await itsVersion.first().innerText() : '')?.[1];
if (!says || headed !== `Blueprint version ${says}`) problems.push(`the heading over a text in the history says "${headed}" and the text says version ${says}`);
if (/\bVersion \d/.test(await page.locator('.history').innerText())) problems.push('something in the history is still called a version by its count');
// What it was applied with is a heading in words, folded, one line over the text and nothing like JSON; pressed, each name in
// the faint column with its value beside it, and a list folded under its name until it is pressed too. The fold is the tab's.
{
  const handed = page.locator('.history-shown .handed');
  const head = handed.locator('.handed-head');
  const heading = (await head.innerText()).trim();
  const many = Number(/^Applied with (\d+) params?$/.exec(heading)?.[1] ?? NaN);
  if (!(many > 0)) problems.push(`what it was applied with is headed "${heading}"`);
  if ((await head.getAttribute('aria-expanded')) !== 'false' || (await handed.locator('.handed-lines').count()) !== 0) problems.push('what it was applied with is not folded as the history opens');
  if ((await page.locator('.history-params').count()) !== 0) problems.push('what it was applied with is still drawn as JSON');
  const [over, text] = [await head.boundingBox(), await page.locator('.history-text').boundingBox()];
  if (text.y - (over.y + over.height) > over.height) problems.push(`the text begins ${Math.round(text.y - over.y)}px under the heading of what it was applied with, more than a line`);
  await head.click();
  const named = await handed.locator('.handed-lines > .handed-line > .handed-name').allInnerTexts();
  if (named.length !== many) problems.push(`the heading says ${many} and ${named.length} are listed: ${named.join(', ')}`);
  const list = handed.locator('.handed-lines > .handed-line', { has: page.locator('.handed-fold', { hasText: /^a list of \d+$/ }) }).first();
  if ((await list.count()) === 0) problems.push(`no list among what it was applied with: ${(await handed.innerText()).replace(/\s+/g, ' ')}`);
  else {
    if ((await list.locator('.handed-under').count()) !== 0) problems.push('a list among what it was applied with is not folded');
    await list.locator('.handed-fold').click();
    const inside = await list.locator('.handed-under > .handed-line').evaluateAll((all) => all.map((one) => `${one.querySelector('.handed-name').textContent}=${one.querySelector('.handed-value')?.textContent ?? '…'}`));
    if (inside[0]?.startsWith('#1=') !== true) problems.push(`a list unfolded says ${inside.join(', ')}`);
  }
  await shot('history-applied-with');
  // Kept by the tab: another view of the page, and back, finds it as it was left.
  await page.getByRole('tab', { name: 'Text', exact: true }).click();
  await page.getByRole('tab', { name: 'History' }).click();
  await handed.locator('.handed-lines').waitFor();
  if ((await handed.locator('.handed-under').count()) === 0) problems.push('what was unfolded of what it was applied with was not kept by the tab');
}
await shot('history');

await side(page, 'Resources');
await page.locator('.nav').getByText('Activity').click();
await page.locator('table tbody tr').first().waitFor();
await shot('activity');

await side(page, 'Resources');
await page.locator('.nav').getByText('Measurements').click();
await page.locator('.tile').first().waitFor();
await page.waitForTimeout(500);
await shot('measurements');

// The factory's views show the things of the workspace the side panel is
// of, are narrowed to one project's, or show every workspace's. It is one
// choice for all three of them.
const show = (label) => page.getByLabel('Show').selectOption({ label });
await side(page, 'Resources');
await page.locator('.nav').getByText('Activity', { exact: true }).click();
await page.locator('table tbody tr').first().waitFor();
await show('Riverside Shop');
await page.getByRole('columnheader', { name: 'Project' }).waitFor({ state: 'detached' });
await shot('activity-of-one-project');

await page.locator('.nav').getByText('Measurements', { exact: true }).click();
await page.locator('.tile').first().waitFor();
if ((await page.getByLabel('Show').inputValue()) !== 'riverside-shop') problems.push('the choice of project was not carried from the activity to the measurements');
await page.locator('.tile', { hasText: 'Runs' }).first().waitFor();
await page.waitForTimeout(500);
const ran = await page.locator('.tile', { hasText: 'Runs' }).first().locator('.value').innerText();
if (!(Number(ran) > 0)) problems.push(`measurements narrowed to a project show no runs: "${ran}"`);
if ((await page.locator('.tile', { hasText: 'Requests' }).count()) !== 0) problems.push('measurements narrowed to a project show requests to the daemon, which are nobody\'s');
await shot('measurements-of-one-project');
await show('All workspaces');
await page.locator('.tile', { hasText: 'Requests' }).first().waitFor();

{
  // Measurements set side by side. Nothing so far was labelled by a blueprint,
  // so there is nothing to compare by and nothing is offered.
  if ((await page.getByLabel('Compare by').count()) !== 0) problems.push('a comparison is offered though no blueprint has labelled anything');
  // The same work done two ways, each said to be a variant, for two experiments.
  const trial = `blueprint: trial
description: The same work done two ways, to see which is better.
permissions: [agents]
params:
  experiment: cart
steps:
  a:
    labels: { experiment: "\${params.experiment}", variant: a }
    task:
      work: { do: agent.task, with: { slot: builder, prompt: "add a card Try the \${params.experiment} one way" } }
      settle: { do: wait, measure: settle, with: { for: 300ms } }
  b:
    needs: []
    labels: { experiment: "\${params.experiment}", variant: b }
    task:
      work: { do: agent.task, with: { slot: builder, prompt: "add a card Try the \${params.experiment} another way" } }
      settle: { do: wait, measure: settle, with: { for: 900ms } }
      check: { do: fail, allow_failure: true, with: { message: not as good } }
`;
  for (const experiment of ['cart', 'search']) {
    const applied = await page.evaluate(
      async (body) => (await fetch('/api/apply', { method: 'POST', headers: { ...{ 'x-page-key': localStorage.getItem('page-key') ?? '' }, 'content-type': 'application/json' }, body: JSON.stringify(body) })).json(),
      { source: trial, project: 'riverside-shop', key: `trial-${experiment}`, params: { experiment }, wait: true },
    );
    if (applied.finished?.status !== 'ok') problems.push(`the trial of ${experiment} did not run: ${JSON.stringify(applied)}`);
  }
  await page.getByRole('button', { name: 'Refresh' }).click();
  await page.getByLabel('Compare by').waitFor();
  const offered = await page.getByLabel('Compare by').locator('option').allInnerTexts();
  if (offered.join() !== 'nothing,experiment,variant') problems.push(`what can be compared by is ${offered.join()}`);
  if ((await page.locator('.compare').count()) !== 0) problems.push('measurements are set side by side before anything was chosen');
  await page.getByLabel('Compare by').selectOption('variant');
  const sideBySide = page.locator('.compare');
  await sideBySide.locator('tbody tr').first().waitFor();
  const heads = (await sideBySide.locator('thead tr').first().locator('th').allInnerTexts()).map((head) => head.replace(/\s+/g, ' ').trim());
  if (heads.join('|').toLowerCase() !== 'measured|a|b|difference b against a') problems.push(`the variants are not side by side as columns: ${heads.join('|')}`);
  // A row is a line of cells: what was measured, five figures for each variant, and the difference.
  const line = async (called) => {
    const row = sideBySide.locator('tbody tr').filter({ has: page.locator('td:first-child', { hasText: called }) }).first();
    return (await row.locator('td').allInnerTexts()).map((cell) => cell.replace(/\s+/g, ' ').trim());
  };
  const turns = await line(/^Agent turns$/);
  if (turns.length !== 12 || turns[1] !== '2' || turns[6] !== '2') problems.push(`each variant's two turns are not counted beside each other: ${turns.join('|')}`);
  const settled = await line(/^settle$/);
  if (!/^\+\d+ms \(\+\d+%\)$/.test(settled[11] ?? '')) problems.push(`the difference between the variants is not said: ${settled.join('|')}`);
  const checked = await line(/b\.check$/);
  if (checked[1] !== 'not measured' || !/2 \(100%\)/.test(checked[3] ?? '')) problems.push(`what only one variant did, and that it failed, is not shown: ${checked.join('|')}`);
  await shot('measurements-side-by-side');
  // Narrowed to one experiment, each variant has the one turn it took for it.
  const where = page.getByLabel('Only where');
  if ((await where.locator('option').allInnerTexts()).join() !== 'anything,experiment is cart,experiment is search') problems.push('the comparison cannot be narrowed to one experiment');
  await where.selectOption('experiment:search');
  await sideBySide.getByText('where experiment is search').waitFor();
  const narrowedTurns = await line(/^Agent turns$/);
  if (narrowedTurns[1] !== '1' || narrowedTurns[6] !== '1') problems.push(`narrowed to one experiment, the turns of both are still counted: ${narrowedTurns.join('|')}`);
  await shot('measurements-side-by-side-in-one-experiment');
  // The choice outlives looking at something else, and is undone by choosing nothing.
  await page.locator('.nav').getByText('Activity', { exact: true }).click();
  await page.locator('table tbody tr').first().waitFor();
  await page.locator('.nav').getByText('Measurements', { exact: true }).click();
  await sideBySide.getByText('where experiment is search').waitFor();
  await page.getByLabel('Compare by').selectOption('');
  await sideBySide.waitFor({ state: 'detached' });
  if ((await page.getByLabel('Only where').count()) !== 0) problems.push('with nothing compared, a narrowing is still offered');
}

await page.keyboard.press('Control+k');
await page.getByLabel('Search', { exact: true }).fill('fleet');
await page.locator('.hit').first().waitFor();
// Under what was typed: how much was found, and what narrows it to one kind of thing.
{
  const says = await page.locator('.palette-found').innerText();
  if (!/^(1 result|\d+ results|\d+ or more results)$/.test(says)) problems.push(`the search says it found "${says}"`);
  if ((await page.getByLabel('Search', { exact: true }).evaluate((box) => getComputedStyle(box).borderBottomWidth)) !== '0px') problems.push('there is a line under what is typed in the search');
  await shot('search');
  const among = page.getByRole('group', { name: 'Look among' });
  await among.getByRole('button', { name: 'Files' }).click();
  await page.waitForFunction(() => [...document.querySelectorAll('.hit .badge')].every((kind) => kind.textContent === 'file'));
  if ((await among.getByRole('button', { name: 'Files' }).getAttribute('aria-pressed')) !== 'true') problems.push('narrowing the search to files does not show as chosen');
  if (!(await page.getByLabel('Search', { exact: true }).evaluate((box) => document.activeElement === box))) problems.push('narrowing the search took the caret out of it');
}
await page.keyboard.press('Escape');

await page.getByRole('button', { name: 'Settings' }).click();
await page.getByRole('heading', { name: 'Plan' }).waitFor();
await shot('settings');

// The client is in one of its themes: the one that was chosen, which is
// kept with the account, and is what the page is drawn in when it is opened
// again. A choice is waited for until the daemon has it, so that the page
// read again is not read before it.
const ground = () => page.evaluate(() => getComputedStyle(document.body).backgroundColor);
const themes = page.getByRole('radiogroup', { name: 'Theme' }).getByRole('radio');
const called = (await themes.allInnerTexts()).map((one) => one.split('\n')[0].trim());
if (called.join() !== 'Grid Iron,Rosé Pine,Tokyo Midnight') problems.push(`the themes offered are ${called.join()}`);
if ((await page.getByRole('radio', { name: /Grid Iron/ }).getAttribute('aria-checked')) !== 'true') problems.push('a client nobody has chosen a theme for is not in Grid Iron');
const grounds = [await ground()];
for (const [name, id] of [['Rosé Pine', 'rose-pine'], ['Tokyo Midnight', 'tokyo-midnight']]) {
  await Promise.all([page.waitForResponse((answer) => answer.url().endsWith('/api/themes/chosen')), page.getByRole('radio', { name: new RegExp(name) }).click()]);
  if ((await page.evaluate(() => document.documentElement.dataset.theme)) !== id) problems.push(`choosing ${name} did not put the page in it`);
  grounds.push(await ground());
  await shot(`theme-${id}`);
}
if (new Set(grounds).size !== 3) problems.push(`three themes should be three grounds, got ${grounds.join(' | ')}`);
await page.reload();
await page.getByRole('heading', { name: 'Plan' }).waitFor();
if ((await ground()) !== grounds[2]) problems.push('the theme that was chosen was not the one the page came back in');
if ((await page.getByRole('radio', { name: /Tokyo Midnight/ }).getAttribute('aria-checked')) !== 'true') problems.push('the theme in use is not marked as the one in use');

// A theme of one's own: what it is called and a few colours, with the rest
// of its list worked out from them. It is kept with the account as the
// choice is, and put on before anything is drawn (`themed.mjs` holds the
// rest of what is kept with an account).
await page.getByRole('button', { name: /Add a theme/ }).click();
const making = page.locator('.modal');
await making.getByLabel('What it is called').fill('Harbour');
await making.getByLabel('Starting from').selectOption('rose-pine');
await pickColour(page, making, 'Accent', '#2fd4c4');
await Promise.all([page.waitForResponse((answer) => answer.url().endsWith('/api/themes/chosen')), making.getByRole('button', { name: 'Add it', exact: true }).click()]);
await making.waitFor({ state: 'detached' });
const colour = (name) => page.evaluate((name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim(), name);
if ((await colour('--accent')) !== '#2fd4c4') problems.push(`a theme made here was not put on: its accent is ${await colour('--accent')}`);
if ((await page.getByRole('radio', { name: /Harbour/ }).getAttribute('aria-checked')) !== 'true') problems.push('the theme just made is not the one in use');
await shot('theme-of-ones-own');
await page.reload();
await page.getByRole('heading', { name: 'Plan' }).waitFor();
if ((await colour('--accent')) !== '#2fd4c4') problems.push('the theme that was made is not the one the page came back in');
// Text that cannot be read on its ground is not a theme, and is said.
await page.getByRole('button', { name: 'Change Harbour' }).click();
await pickColour(page, making, 'Text', '#1a1826');
if ((await making.getByRole('button', { name: 'Keep it', exact: true }).isEnabled()) || !(await making.innerText()).includes('too near the ground')) problems.push('a theme whose text cannot be read could be kept');
await making.getByRole('button', { name: 'Cancel', exact: true }).click();
// Taken away, the page is in the usual one again.
await page.getByRole('button', { name: 'Remove Harbour' }).click();
// It goes as the daemon says it has gone.
await page.getByRole('radio', { name: /Harbour/ }).waitFor({ state: 'detached' });
if ((await page.evaluate(() => document.documentElement.dataset.theme)) !== 'grid-iron' || (await colour('--accent')) !== '#dc7b32') problems.push('taking the theme in use away did not put the page back in the usual one');

await browser.close();
await daemon?.stop();
if (problems.length > 0) {
  console.error(`\n${problems.length} problem(s) in the page:\n${problems.join('\n')}`);
  process.exit(1);
}
console.log(`\nno errors in the page; pictures are in ${out}`);
