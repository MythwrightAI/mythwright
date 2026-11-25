// What the activity, the measurements and the agents show: of the
// workspace the side panel is of, what somebody chose there.
//
//   DAEMON=../target/debug/factory node scripts/narrowed.mjs [out-dir]
//
// The three pages have one choice in their heads, and it is a choice
// within the workspace the panel is of: all of that workspace, which is
// what there is with nothing chosen; one project of it; or every
// workspace, which is everything, what was applied for no project with
// it. A factory with one workspace and no agent of no project offers no
// line for every workspace where it would show the same (the agents), and
// offers it where the page has something of the factory's own (its log,
// on the activity). Then more workspaces are made from a terminal, each
// with projects that have a board, and something is applied for no
// project. The activity opened in the first workspace lists its
// projects' and not the other's; the choice's lines are the workspace,
// its projects and every workspace, in that order. The other workspace is
// chosen at the panel's foot, which shows that workspace's own tabs, the
// start page alone to begin with: each of the three pages opened there
// is of that workspace, with its lines. And where a page is the tab
// showing in both workspaces' tabs it follows the panel as it stands,
// the page never read again: the activity and the measurements each way.
// One project is chosen, and is still what is chosen when the page is
// read again and when the other workspace was visited meanwhile, each
// workspace keeping its own. Every workspace shows everything. A project
// put in the other workspace from a terminal while it was the one chosen
// leaves the pages showing the workspace. What a browser kept from
// before, one project for the whole factory, is read once as the choice
// of that project's own workspace and of no other. The activity of a
// project asked for from the gear of its page, opened by its address
// while the panel is of another workspace, puts the panel on the
// project's workspace: that workspace's tabs are shown with the activity
// among them, of the project, and the page it was asked from is a tab
// still where it was opened. The agents are of the same choice: the
// workspace's projects' agents, those of the other workspace there, one
// project's, an empty list said to be of that project, and every agent
// with those of no project last under All workspaces. And nothing of the
// three heads lies over anything else at three widths and a phone's. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { fillsPage } from './filled.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'narrowed-data');
const home = join(out, 'narrowed-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
// A project with a board and a card on it: something applied for it, which the activity lists.
const project = async (name, id) => {
  await cli('project', 'create', name, '--bare');
  await cli('apply', 'std/board', '--project', id);
  await cli('board', 'add', id, `A card of ${name}`);
};
// The workspace made with the factory's owner as it first started: the usual one, by the id it was made with (`unnamed-workspace`), whatever it is called since.
const USUAL = 'unnamed-workspace';

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('workspace', 'rename', USUAL, 'Print works');
  await project('Riverside Shop', 'riverside-shop');
  await project('Market Stall', 'market-stall');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1500, height: 940 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  // How often the page was itself read: what follows the panel does so with the page as it is.
  let pageReads = 0;
  page.on('request', (request) => {
    if (request.isNavigationRequest()) pageReads += 1;
  });

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const side = async () => {
    const railed = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await railed.getAttribute('aria-pressed')) !== 'true') await railed.click();
    await resources.waitFor();
    await unfoldAll(page);
  };
  await side();

  // Read until it says what is meant, or said to be otherwise: what the page shows follows what was done a moment after.
  const listed = async (read, wanted, what) => {
    const until = Date.now() + 10000;
    let got = await read();
    while (got !== wanted && Date.now() < until) {
      await page.waitForTimeout(60);
      got = await read();
    }
    expect(`${what}: "${wanted}", got "${got}"`, got === wanted);
  };

  // ---- the choice in a page's head
  const choice = page.getByLabel(/^What is shown of /);
  // Its lines, in order, with what stands over its projects and which is chosen.
  const lines = () =>
    choice.evaluate((select) =>
      [...select.children]
        .map((child) => {
          if (child.tagName === 'HR') return '—';
          if (child.tagName === 'OPTGROUP') return `${child.label}[${[...child.children].map((option) => `${option.textContent}${option.selected ? '*' : ''}`).join(', ')}]`;
          return `${child.textContent}${child.selected ? '*' : ''}`;
        })
        .join(' | '),
    );
  const chosen = () => choice.inputValue();
  // What this browser keeps of it, under who is signed in.
  const kept = () =>
    page.evaluate(() =>
      Object.entries(localStorage)
        .filter(([key]) => key.startsWith('client.factory.shown:user:'))
        .map(([, value]) => value)
        .join(' '),
    );

  // ---- the three pages, each a tab
  // Each workspace has its own open tabs: a page is shown from the set on the page where it is open there, and
  // opened from the side panel where it is not.
  const tab = (name) => page.locator('.tabs').getByRole('tab', { name: new RegExp(name) });
  // The agents are the Members page's Agents tab: its page's tab, or the Members line's name, and then the tab of the page.
  const open = async (name, there) => {
    const called = name === 'Agents' ? 'Members' : name;
    if ((await tab(called).count()) > 0) await tab(called).first().click();
    else if (name === 'Agents') await resources.locator('.members-part .nav-row > .nav-item').click();
    else await resources.locator('.nav').getByText(name, { exact: true }).click();
    await page.locator('.tab[aria-selected="true"]', { hasText: called }).first().waitFor();
    if (name === 'Agents') await page.locator('.members-tabs').getByRole('tab', { name: 'Agents', exact: true }).click();
    await page.locator(there).first().waitFor();
  };
  // The tabs on the page, the one showing marked.
  const tabs = () => page.locator('.tab').evaluateAll((all) => all.map((one) => `${one.querySelector('.truncate').textContent}${one.getAttribute('aria-selected') === 'true' ? '*' : ''}`).join(', '));
  // The activity: whose each instance listed is, where the list says, and whether the factory's own log is there.
  const whose = async () => {
    const table = page.locator('.view-body .panel').first().locator('table');
    const heads = (await table.locator('thead th').allTextContents()).map((head) => head.trim());
    const at = heads.indexOf('Project');
    if (at < 0) return `no column, ${await table.locator('tbody tr').count() > 0 ? 'some' : 'none'} listed`;
    const cells = await table.locator(`tbody tr td:nth-child(${at + 1})`).allTextContents();
    return [...new Set(cells.map((cell) => cell.trim() || '(nobody)'))].sort().join(', ');
  };
  const logged = () => page.locator('.factory-log').count();
  // The measurements: how many runs are counted for `of` (the projects, or `*` for everything), which what is drawn says
  // it is of once that is what was read; and whether requests to the daemon are counted.
  const runsCounted = async (of) => {
    const body = page.locator(`.view-body[data-of="${of}"]`);
    await body.waitFor();
    return Number(await body.locator('.tile', { hasText: 'Runs' }).first().locator('.value').innerText());
  };
  const requests = () => page.locator('.tile', { hasText: 'Requests' }).count();

  // ---- the foot of the side panel: which workspace it is of
  const foot = resources.locator('.space-foot .space-pick');
  const showing = () => resources.locator('.space-foot .space-said strong').innerText();
  const spaceMenu = page.getByRole('menu', { name: 'Workspace', exact: true });
  const toSpace = async (name) => {
    await foot.click();
    await spaceMenu.waitFor();
    // Opened, it has the keyboard, which is what says it is listening.
    await page.waitForFunction(() => document.activeElement?.closest('.row-menu') !== null && document.activeElement?.closest('.row-menu') !== undefined);
    await spaceMenu.getByRole('menuitemradio', { name: new RegExp(`^${name}(:|$)`) }).click();
    await spaceMenu.waitFor({ state: 'detached' });
    await listed(showing, name, `the foot to say ${name}`);
    // Off the panel, where nothing is pointed at.
    await page.mouse.move(900, 600);
  };

  // ---- one workspace, and nothing that is nobody's
  await listed(showing, 'Print works', 'the panel to be of the one workspace there is');
  // No agent is of no project: every workspace would show the same on the agents' page, and it is no line there.
  await open('Agents', '.agents-listed');
  await listed(lines, 'Print works* | Projects[Riverside Shop, Market Stall]', 'the agents\' choice with one workspace and no agent that is nobody\'s');
  expect(`the choice to say what it is of, got "${await choice.getAttribute('aria-label')}" and "${await choice.getAttribute('title')}"`, (await choice.getAttribute('aria-label')) === 'What is shown of Print works' && /All of Print works, one project of it\./.test(await choice.getAttribute('title')));
  await page.getByRole('button', { name: 'Close Members', exact: true }).click();
  await page.locator('.tabs .tab', { hasText: 'Members' }).waitFor({ state: 'detached' });
  // The activity has the factory's own log to show, which no project's is: there every workspace is a line.
  await open('Activity', '.view-head');
  await listed(lines, 'Print works* | Projects[Riverside Shop, Market Stall] | — | All workspaces', 'the activity\'s choice with one workspace');
  await listed(whose, 'market-stall, riverside-shop', 'the activity to list what was applied for the workspace\'s projects');
  expect('the factory\'s own log not to be shown for a workspace', (await logged()) === 0);
  // Something applied for no project: it is listed under every workspace, and not for the workspace.
  await writeFile(join(out, 'nightly.bp'), ['blueprint: nightly', 'description: Says how the night went, when it is asked.', 'steps:', '  told: { do: log, with: { message: "The night went well." } }', 'on:', '  asked:', '    event: night.asked', '    steps:', '      told: { do: log, with: { message: "It went well." } }', ''].join('\n'));
  await cli('apply', join(out, 'nightly.bp'));
  await listed(whose, 'market-stall, riverside-shop', 'what is nobody\'s not to be listed for a workspace');

  // ---- two workspaces
  await cli('workspace', 'create', 'Annex');
  await project('Harbour Notes', 'harbour-notes');
  await project('Lantern', 'lantern');
  await cli('workspace', 'put', 'annex', 'harbour-notes');
  await cli('workspace', 'put', 'annex', 'lantern');
  // The activity, opened in the first workspace: its projects' and not the other's.
  await listed(whose, 'market-stall, riverside-shop', 'what was applied for the first workspace\'s projects alone');
  await listed(lines, 'Print works* | Projects[Riverside Shop, Market Stall] | — | All workspaces', 'the lines of the choice, in order');
  expect(`nothing to be kept with nothing chosen, got "${await kept()}"`, (await kept()) === '');
  await page.screenshot({ path: `${out}/narrowed-1-workspace.png` });

  // The other workspace has tabs of its own, which begin at the start page: the activity opened there is of that
  // workspace, and its choice's lines are that workspace's.
  const pageBefore = pageReads;
  await toSpace('Annex');
  await listed(tabs, 'Start*', 'the other workspace to begin at the start page, with its own tabs');
  await open('Activity', '.view-head');
  await listed(whose, 'harbour-notes, lantern', 'the activity opened in the other workspace to be of that workspace');
  await listed(lines, 'Annex* | Projects[Harbour Notes, Lantern] | — | All workspaces', 'the lines of the choice in the other workspace');
  expect(`the choice to say which workspace it is of, got "${await choice.getAttribute('aria-label')}"`, (await choice.getAttribute('aria-label')) === 'What is shown of Annex');
  await page.screenshot({ path: `${out}/narrowed-2-other-workspace.png` });
  await toSpace('Print works');
  await listed(tabs, 'Start, Activity*', 'the first workspace\'s tabs to be as they were left');
  await listed(whose, 'market-stall, riverside-shop', 'the activity there to be of the first workspace');

  // The activity, opened in each: of that workspace. And where it is the tab showing in both, it follows the panel.
  await toSpace('Annex');
  await open('Activity', '.view-head');
  await listed(whose, 'harbour-notes, lantern', 'the activity to be of the workspace the panel is of');
  await toSpace('Print works');
  await open('Activity', '.view-head');
  await listed(whose, 'market-stall, riverside-shop', 'the activity in the first workspace to be of that one');
  expect('no log of the factory\'s own for a workspace', (await logged()) === 0);
  await toSpace('Annex');
  await listed(whose, 'harbour-notes, lantern', 'the activity to follow the panel');
  await toSpace('Print works');
  await listed(whose, 'market-stall, riverside-shop', 'and to follow it back');
  await page.screenshot({ path: `${out}/narrowed-3-activity.png` });
  // Activity is a page of tables, and fills its tab (`filled.mjs`).
  await fillsPage(page, 'Activity', page.locator('.view-body').first(), { ready: page.locator('.view-head'), shot: `${out}/narrowed-activity-filled` });
  // And the measurements: what was measured for each project, and for a workspace its projects' added up by the daemon.
  await open('Measurements', '.tile');
  const runsOf = async (project) => {
    await choice.selectOption(project);
    return runsCounted(project);
  };
  const [shop, stall] = [await runsOf('riverside-shop'), await runsOf('market-stall')];
  expect(`runs to be counted for each project, got ${shop} and ${stall}`, shop > 0 && stall > 0);
  expect('requests to the daemon, which are nobody\'s, not to be counted for one project', (await requests()) === 0);
  await choice.selectOption('');
  const works = await runsCounted('riverside-shop,market-stall');
  expect(`the workspace's runs to be its projects' put together, got ${works} for ${shop} and ${stall}`, works === shop + stall);
  expect('requests to the daemon not to be counted for a workspace either', (await requests()) === 0);
  await fillsPage(page, 'Measurements', page.locator('.view-body[data-of]'), { ready: page.locator('.tile'), shot: `${out}/narrowed-measurements-filled` });
  await toSpace('Annex');
  await open('Measurements', '.tile');
  const annex = await runsCounted('harbour-notes,lantern');
  expect(`the measurements in the other workspace to be of that one, got ${annex} runs with "${await chosen()}" chosen`, annex > 0 && (await chosen()) === '');
  // The tab showing in both: they follow the panel.
  await toSpace('Print works');
  const again = await runsCounted('riverside-shop,market-stall');
  expect(`the measurements to follow the panel, got ${again} runs for ${works}`, again === works);
  await page.screenshot({ path: `${out}/narrowed-4-measurements.png` });
  expect(`the page never to have been read again, read ${pageReads - pageBefore} times`, pageReads === pageBefore);

  // ---- the agents: a third page of the same choice, held to the workspace's projects in the page
  await cli('agent', 'add', 'stall-hand', '--project', 'market-stall');
  await cli('agent', 'add', 'shop-hand', '--project', 'riverside-shop');
  await cli('agent', 'add', 'harbour-hand', '--project', 'harbour-notes');
  await cli('agent', 'add', 'roamer');
  // The bands drawn, each with its agents.
  const banded = () => page.locator('.agents-of').evaluateAll((all) => all.map((band) => `${band.querySelector('h3 > .truncate').textContent}[${[...band.querySelectorAll('.agent-line')].map((row) => row.dataset.agent).join(', ')}]`).join(' | '));
  const whoseList = () => page.locator('.agents-listed .panel-head h2').innerText();
  await open('Agents', '.agents-listed');
  await listed(banded, 'Riverside Shop[shop-hand] | Market Stall[stall-hand]', 'the agents of the workspace\'s projects, and none of another\'s or of none');
  await listed(whoseList, 'Print works', 'the list to say it is of the workspace');
  await listed(lines, 'Print works* | Projects[Riverside Shop, Market Stall] | — | All workspaces', 'the agents\' choice, the same as the other two\'s');
  await page.screenshot({ path: `${out}/narrowed-4a-agents.png` });
  // The other workspace's tabs: the page opened there is of that workspace; and the tab showing in both follows the panel.
  await toSpace('Annex');
  await open('Agents', '.agents-listed');
  await listed(banded, 'Harbour Notes[harbour-hand]', 'the agents of the other workspace');
  await toSpace('Print works');
  await listed(banded, 'Riverside Shop[shop-hand] | Market Stall[stall-hand]', 'the agents to follow the panel back');
  // One project chosen there is the choice of all three pages.
  await choice.selectOption({ label: 'Market Stall' });
  await listed(banded, 'Market Stall[stall-hand]', 'one project\'s agents');
  await listed(whoseList, 'Market Stall', 'the list to say it is of the project');
  await listed(kept, `{"${USUAL}":{"kind":"project","id":"market-stall"}}`, 'the choice made on the agents\' page to be kept for its workspace');
  await open('Activity', '.view-head');
  await listed(chosen, 'market-stall', 'the choice made on the agents\' page to be the activity\'s too');
  await open('Agents', '.agents-listed');
  // A project of the workspace with no agent: said to be so, and where the rest are.
  await choice.selectOption({ label: 'Riverside Shop' });
  await cli('agent', 'remove', 'shop-hand');
  await page.getByText('No agent is of this project.').waitFor();
  await cli('agent', 'add', 'shop-hand', '--project', 'riverside-shop');
  // Every workspace: every agent, those of no project last, under the side panel's word for the factory's own.
  await choice.selectOption({ label: 'All workspaces' });
  await listed(banded, 'Riverside Shop[shop-hand] | Market Stall[stall-hand] | Harbour Notes[harbour-hand] | Factory[roamer]', 'every agent for every workspace, the factory\'s own last');
  await listed(whoseList, 'All workspaces', 'the list to say it is of every workspace');
  await page.screenshot({ path: `${out}/narrowed-4b-agents-everywhere.png` });
  await choice.selectOption('');
  await listed(kept, '', 'all of the workspace to be nothing kept, from the agents\' page as from any');
  // Its tab is put away in both, so that what follows finds each workspace's tabs as it left them.
  for (const space of ['Annex', 'Print works']) {
    await toSpace(space);
    await page.getByRole('button', { name: 'Close Members', exact: true }).click();
    await page.locator('.tabs .tab', { hasText: 'Members' }).waitFor({ state: 'detached' });
  }

  // ---- one project chosen: kept for its workspace
  await open('Measurements', '.tile');
  await choice.selectOption({ label: 'Market Stall' });
  await listed(kept, `{"${USUAL}":{"kind":"project","id":"market-stall"}}`, 'the choice to be kept for its workspace');
  // On the activity it is the same choice, and a list of one project's says no project.
  await open('Activity', '.view-head');
  await listed(chosen, 'market-stall', 'the choice to be one for the three pages');
  await listed(whose, 'no column, some listed', 'the activity of one project');
  // Read again, it is what was chosen.
  await page.reload();
  await page.locator('.view-head').waitFor();
  await listed(chosen, 'market-stall', 'the project to be chosen still when the page is read again');
  await listed(showing, 'Print works', 'the panel to be of the same workspace');
  // The other workspace visited meanwhile, with a choice of its own there.
  await toSpace('Annex');
  await open('Activity', '.view-head');
  await listed(chosen, '', 'the other workspace to be shown whole: nothing was chosen there');
  await listed(whose, 'harbour-notes, lantern', 'and its projects\' to be listed');
  await choice.selectOption({ label: 'Lantern' });
  await listed(kept, `{"${USUAL}":{"kind":"project","id":"market-stall"},"annex":{"kind":"project","id":"lantern"}}`, 'each workspace to keep its own');
  await toSpace('Print works');
  await listed(chosen, 'market-stall', 'the first workspace to be as it was left');
  await toSpace('Annex');
  await listed(chosen, 'lantern', 'and the other as it was left');
  await choice.selectOption('');
  await listed(kept, `{"${USUAL}":{"kind":"project","id":"market-stall"}}`, 'all of a workspace to be nothing kept');
  await toSpace('Print works');

  // ---- every workspace: everything, with what is nobody's
  await choice.selectOption({ label: 'All workspaces' });
  await listed(whose, '(nobody), harbour-notes, lantern, market-stall, riverside-shop', 'everything to be listed for every workspace');
  expect('the factory\'s own log to be shown with everything', (await logged()) === 1);
  await page.screenshot({ path: `${out}/narrowed-5-everything.png` });
  await open('Measurements', '.tile');
  const every = await runsCounted('*');
  expect(`every run to be counted with everything, got ${every} for ${shop + stall + annex} of the projects and one of nobody's`, every >= shop + stall + annex + 1);
  expect('requests to the daemon to be counted with everything', (await requests()) === 1);

  // ---- a project put in the other workspace while it was the one chosen
  await choice.selectOption({ label: 'Market Stall' });
  await open('Activity', '.view-head');
  await listed(chosen, 'market-stall', 'the project to be the one chosen');
  await cli('workspace', 'put', 'annex', 'market-stall');
  await listed(chosen, '', 'the page to come to the workspace when what was chosen is another workspace\'s');
  await listed(lines, 'Print works* | Projects[Riverside Shop] | — | All workspaces', 'the project to be a line no longer');
  await listed(kept, '', 'and what was kept of it to be gone');

  // ---- what a browser kept from before: one project for the whole factory
  await page.evaluate(() => localStorage.setItem('client.factory.project', 'harbour-notes'));
  await page.reload();
  await page.locator('.view-head').waitFor();
  await listed(showing, 'Print works', 'the panel to be of the workspace it was of');
  // It is another workspace's project: the workspace showing is shown whole, and not that project.
  await listed(lines, 'Print works* | Projects[Riverside Shop] | — | All workspaces', 'what was kept before not to be taken for this workspace\'s choice');
  await listed(chosen, '', 'nothing to be chosen here');
  await listed(() => page.evaluate(() => String(localStorage.getItem('client.factory.project'))), 'null', 'what was kept before to be read once and gone');
  await listed(kept, '{"annex":{"kind":"project","id":"harbour-notes"}}', 'it to be the choice of its project\'s own workspace');
  await toSpace('Annex');
  await open('Activity', '.view-head');
  await listed(chosen, 'harbour-notes', 'the project to be chosen in its own workspace');
  await listed(whose, 'no column, some listed', 'and to be what is listed there');
  await listed(lines, 'Annex | Projects[Market Stall, Harbour Notes*, Lantern] | — | All workspaces', 'the lines there, with the project that was put there');
  await choice.selectOption('');
  await toSpace('Print works');

  // ---- one project's activity asked for from the page of a project of another workspace
  // Its page is opened by its address where the reader is: a tab there, which moves nothing.
  await page.evaluate(() => (location.hash = '/project/lantern'));
  await page.getByRole('heading', { name: 'Lantern', level: 1 }).waitFor();
  await listed(showing, 'Print works', 'something of another workspace opened by its address not to move the panel');
  await listed(tabs, 'Start, Activity, Measurements, Lantern*', 'it to be a tab where it was opened');
  // Asked from its gear for the activity of that project, the three pages being of the workspace the panel is of:
  // the panel is put on the project's workspace, whose tabs are shown, and the activity is opened among those, of
  // the project. Nothing is closed: the project's page is a tab still where it was opened.
  await page.locator('.view-head .head-acts').getByRole('button', { name: 'More for this page', exact: true }).click();
  await page.locator('.row-menu').getByRole('menuitem', { name: 'Activity of Lantern' }).click();
  await listed(showing, 'Annex', 'the panel to be put on the project\'s workspace');
  await listed(tabs, 'Start, Activity*, Measurements', 'the activity to be shown among that workspace\'s tabs');
  await listed(chosen, 'lantern', 'the project to be what is chosen there');
  await listed(whose, 'no column, some listed', 'and the activity to be the project\'s');
  await listed(kept, '{"annex":{"kind":"project","id":"lantern"}}', 'the choice to be kept for the project\'s workspace and for no other');
  await page.screenshot({ path: `${out}/narrowed-6-of-a-project.png` });
  await choice.selectOption('');
  await toSpace('Print works');
  await listed(tabs, 'Start, Activity, Measurements, Lantern*', 'the first workspace\'s tabs to be as they were, the project\'s page among them');
  await open('Activity', '.view-head');
  await listed(chosen, '', 'and nothing to be chosen there');

  // ---- nothing of a head over anything else of it, or off the page
  const head = async (within, where) => {
    const got = await page.evaluate((within) => {
      const round = (figure) => Math.round(figure * 10) / 10;
      const all = [...document.querySelector(within).children]
        .map((child) => {
          const at = child.getBoundingClientRect();
          return { said: (child.getAttribute('aria-label') ?? child.textContent ?? '').trim().slice(0, 28), left: round(at.left), right: round(at.right), top: round(at.top), bottom: round(at.bottom) };
        })
        // A thing with no size (what only pushes two others apart) is nothing to lie over.
        .filter((at) => at.right - at.left > 0.5 && at.bottom - at.top > 0.5);
      const over = [];
      for (let a = 0; a < all.length; a++) {
        for (let b = a + 1; b < all.length; b++) {
          const [one, other] = [all[a], all[b]];
          if (one.left < other.right - 0.5 && other.left < one.right - 0.5 && one.top < other.bottom - 0.5 && other.top < one.bottom - 0.5) over.push(`"${one.said}" and "${other.said}"`);
        }
      }
      const room = document.querySelector('.workbench')?.getBoundingClientRect() ?? { left: 0, right: window.innerWidth };
      const off = all.filter((at) => at.left < room.left - 0.5 || at.right > Math.min(room.right, window.innerWidth) + 0.5).map((at) => at.said);
      const select = document.querySelector('select.narrow');
      return { over, off, sideways: document.documentElement.scrollWidth > window.innerWidth + 1, choice: select ? round(select.getBoundingClientRect().width) : 0 };
    }, within);
    const told = JSON.stringify(got);
    expect(`nothing of the head to lie over anything else of it ${where}: ${told}`, got.over.length === 0);
    expect(`nothing of the head to be off the page ${where}: ${told}`, got.off.length === 0 && !got.sideways);
    expect(`the choice to be drawn ${where}: ${told}`, got.choice > 40);
  };
  for (const [name, there, within] of [['Activity', '.view-head', '.view-head'], ['Measurements', '.tile', '.view-head'], ['Agents', '.agents-listed', '.view-head']]) {
    await open(name, there);
    for (const width of [1500, 1100, 820, 390]) {
      await page.setViewportSize({ width, height: width === 390 ? 844 : 940 });
      await page.locator(within).first().waitFor();
      await choice.waitFor();
      await page.waitForTimeout(250);
      await head(within, `on ${name} at ${width} wide`);
      if (width === 390) await page.screenshot({ path: `${out}/narrowed-7-phone-${name.toLowerCase()}.png` });
    }
    // The side panel is put away on a screen that narrow, and is asked for again.
    await page.setViewportSize({ width: 1500, height: 940 });
    await side();
  }

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
