// The dialog that adds a project, as a table of what it starts with.
//
//   DAEMON=../target/debug/factory node scripts/settingup.mjs [out-dir]
//
// Opened from the `+` of Projects, it asks no workspace and says in a
// line of its head the one the side panel is of, which is where the
// project is made. What it starts with is a table: a row for each thing,
// called by its name, with what it is, its switch, every switch in one
// column, and a button that unfolds what it is set up with, which is its
// blueprint's own params as the daemon reads them (`/api/starts`), drawn
// as the drawer that applies a blueprint draws them. Unfolded, Docs is
// given another folder and says it has one setting changed; Pipelines is
// turned on and given a number that is not one, which is said with whose
// setting it is and keeps the dialog open with the row unfolded, and then
// one that is; Environments is turned on. Made, the project's library is
// at that folder, its pipelines run as many at once as was said, and it
// has its environments. Nothing of the dialog lies over anything else of
// it or leaves it, the dialog is in the window and no taller than it, and
// nothing scrolls sideways, at 1280, 900 and 640 wide and a phone's, with
// every row unfolded. Then a row of one's own: a blueprint named under the
// table, added on with its own settings, and the table saved as a setup
// with a name, which the dialog is then set up as, with every row kept
// and what was on; the project made with it set up as it says. Opened
// again, the setup is chosen at the top and its rows are its own, and
// whoever manages the factory saves the table as its default. Each by the
// boxes at three widths and a phone's, plainly and with
// `LOOK="size=largest density=compact"`. The environment variables are
// those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'settingup-data');
const home = join(out, 'settingup-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that nothing here reads or writes what a
// factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');

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
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  // The workspace is read before the dialog says it.
  await page.getByRole('button', { name: /^Workspace: / }).waitFor();

  await page.getByRole('button', { name: 'New project' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Name', { exact: true }).waitFor();
  const where = (await dialog.locator('.ask-where').innerText()).trim();
  expect(`its head to say where it is made, got "${where}"`, where === "In Ada's workspace");
  expect('no workspace to be asked', (await dialog.getByLabel(/^Workspace/).count()) === 0);
  await dialog.getByLabel('Name', { exact: true }).fill('Works');

  // A row for each thing, called by its name, in the list's order; each with what it is set up with, folded.
  const rows = dialog.locator('.setup-row');
  const titles = await rows.evaluateAll((all) => all.map((row) => row.querySelector('.grow')?.firstChild?.textContent?.trim()));
  expect(`a row for each of Board, Fleet, Docs, Pipelines, Environments and Flags, got ${titles.join(', ')}`, titles.join() === 'Board,Fleet,Docs,Pipelines,Environments,Flags');
  const fold = (title) => dialog.getByRole('button', { name: `Settings of ${title}`, exact: true });
  for (const title of titles) {
    await fold(title).waitFor();
    expect(`the settings of ${title} to be folded as it opens`, (await fold(title).getAttribute('aria-expanded')) === 'false');
  }
  const settings = (title) => dialog.getByRole('group', { name: `Settings of ${title}`, exact: true });
  for (const title of titles) {
    await fold(title).click();
    await settings(title).waitFor();
  }
  // What each is set up with is its blueprint's own params, less what it is told of the project.
  const labels = async (title) => settings(title).locator('.comparing-label').evaluateAll((all) => all.map((one) => one.firstChild.textContent.trim()));
  expect(`the board's settings, got ${(await labels('Board')).join(', ')}`, (await labels('Board')).join() === 'columns,store,database');
  expect(`the fleet's settings, got ${(await labels('Fleet')).join(', ')}`, (await labels('Fleet')).join() === 'planned,fetch,push,key');
  expect(`the library's setting, got ${(await labels('Docs')).join(', ')}`, (await labels('Docs')).join() === 'folder');
  expect(`the environments' setting, got ${(await labels('Environments')).join(', ')}`, (await labels('Environments')).join() === 'names');
  expect('a box to begin as what its blueprint says', (await settings('Docs').getByRole('textbox').inputValue()) === 'docs');

  // By the boxes: nothing over anything else of it or outside it, every switch in one column, the dialog in the
  // window, and nothing scrolling sideways.
  const PARTS =
    'h2, .ask-where, label > input, label > select, .ask-folder .faint, .setup-row .grow, .setup-row .switch, .setup-fold, .comparing-label, .comparing-said, .setup-add button, .setup-drop, .setup-save button, .setup-save-as, .setup-saved';
  const looked = async (what) => {
    const dialog = page.getByRole('dialog');
    const found = await dialog.evaluate((box, picked) => {
      const frame = box.getBoundingClientRect();
      const boxes = [...box.querySelectorAll(picked)]
        .map((one) => ({ said: (one.getAttribute('aria-label') ?? one.textContent.trim()).slice(0, 30) || one.className, box: one.getBoundingClientRect(), one }))
        .filter((one) => one.box.width > 0 && one.box.height > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < frame.left - 0.5 || one.box.right > frame.right + 0.5) wrong.push(`${one.said} leaves it`);
        for (const two of boxes.slice(index + 1)) {
          // What holds another (a row's words and the button under them) is not over it.
          if (one.one.contains(two.one) || two.one.contains(one.one)) continue;
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      const switches = [...box.querySelectorAll('.setup-row .switch')].map((one) => Math.round(one.getBoundingClientRect().left));
      const pressed = [...box.querySelectorAll('.modal-foot button')];
      const foot = pressed.map((one) => one.getBoundingClientRect());
      // What is drawn at the middle and near the corners of each of its buttons is that button, and nothing scrolled under it.
      const drawn = pressed.every((one) => {
        const at = one.getBoundingClientRect();
        const points = [[0.5, 0.5], [0.1, 0.15], [0.9, 0.15], [0.1, 0.85], [0.9, 0.85]];
        return points.every(([x, y]) => one.contains(document.elementFromPoint(at.left + at.width * x, at.top + at.height * y)));
      });
      return {
        count: boxes.length,
        wrong,
        column: new Set(switches).size,
        inWindow: frame.top >= -0.5 && frame.bottom <= window.innerHeight + 0.5 && frame.left >= -0.5 && frame.right <= window.innerWidth + 0.5,
        foot: drawn && foot.every((one) => one.top >= frame.top && one.bottom <= frame.bottom + 0.5 && one.bottom <= window.innerHeight + 0.5),
        sideways: document.documentElement.scrollWidth - window.innerWidth,
      };
    }, PARTS);
    expect(`${what}: something to be there`, found.count > 10);
    expect(`${what}: nothing over anything else or outside the dialog, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: every switch in one column, got ${found.column} columns`, found.column === 1);
    expect(`${what}: the dialog in the window and no taller than it`, found.inWindow);
    expect(`${what}: Create and Cancel in reach at its foot, with nothing over them`, found.foot);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px`, found.sideways <= 0);
  };
  for (const [width, height] of [
    [1280, 900],
    [900, 800],
    [640, 760],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    await looked(`at ${width} wide`);
    await page.screenshot({ path: `${out}/settingup-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 900 });

  // Docs given another folder: its row says one setting is changed.
  await settings('Docs').getByRole('textbox').fill('notes');
  await fold('Docs').locator('.setup-changed', { hasText: '1 set' }).waitFor();
  // Pipelines turned on and given a number that is not one: said with whose setting it is, and nothing is made.
  await dialog.getByRole('switch', { name: 'Pipelines', exact: true }).check();
  await dialog.getByRole('switch', { name: 'Environments', exact: true }).check();
  await settings('Pipelines').getByRole('textbox').fill('many');
  await dialog.getByRole('button', { name: 'Create', exact: true }).click();
  await settings('Pipelines').getByText('Pipelines: `at_once` should be a number.', { exact: true }).waitFor();
  expect('the dialog to stay with what was written', (await dialog.getByLabel('Name', { exact: true }).inputValue()) === 'Works');
  await page.screenshot({ path: `${out}/settingup-wrong.png` });
  await settings('Pipelines').getByRole('textbox').fill('3');
  await dialog.getByRole('button', { name: 'Create', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });

  // Made with what was said of each, and as its text says what nobody said.
  const asked = (path) =>
    page.evaluate(async (at) => (await fetch(at, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json(), path);
  const live = (await asked('/api/instances?project=works&live=true')).instances;
  const params = (blueprint) => live.find((one) => one.blueprint === blueprint)?.params ?? {};
  expect(`its library to be at the folder said, got ${JSON.stringify(params('std/docs'))}`, params('std/docs').folder === 'notes');
  expect(`its pipelines to run as many at once as was said, got ${JSON.stringify(params('std/pipeline'))}`, params('std/pipeline').at_once === 3);
  expect(`its board to have its columns as its text says, got ${JSON.stringify(params('std/board'))}`, params('std/board').columns === undefined || params('std/board').columns.length === 5);
  const environments = (await asked('/api/projects/works/environments')).environments.map((one) => one.name);
  expect(`its environments, got ${environments.join(', ')}`, environments.join() === 'dev,prod');
  const projects = (await asked('/api/workspaces')).workspaces;
  expect(`it to be in the workspace showing, got ${JSON.stringify(projects)}`, JSON.stringify(projects).includes('"works"'));

  // A row of one's own: a blueprint named under the table and added, on, with its own settings; and the table saved
  // as a setup with a name, which the dialog is then set up as. The side panel, put away at a phone's width, first.
  const panel = async () => {
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
    await page.getByRole('button', { name: 'New project' }).waitFor();
  };
  await panel();
  await page.getByRole('button', { name: 'New project' }).click();
  const again = page.getByRole('dialog');
  await again.getByLabel('Name', { exact: true }).fill('Lab');
  const own = again.getByLabel('A blueprint of your own', { exact: true });
  await own.fill('std/mockups');
  await again.getByRole('button', { name: 'Add row', exact: true }).click();
  await again.getByRole('switch', { name: 'mockups', exact: true }).waitFor();
  expect('a row of one’s own to be on', await again.getByRole('switch', { name: 'mockups', exact: true }).isChecked());
  expect('the field to be emptied once it is added', (await own.inputValue()) === '');
  await again.getByRole('button', { name: 'Settings of mockups', exact: true }).click();
  await again.getByRole('group', { name: 'Settings of mockups', exact: true }).waitFor();
  await again.getByRole('switch', { name: 'Docs', exact: true }).uncheck();
  await again.getByRole('button', { name: 'Save as…', exact: true }).click();
  const saving = again.getByRole('group', { name: 'Saving the table', exact: true });
  await saving.getByLabel('Its name', { exact: true }).fill('A research project');
  for (const [width, height] of [
    [1280, 900],
    [900, 800],
    [640, 760],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    await looked(`with a row of one's own and saving, at ${width} wide`);
    await page.screenshot({ path: `${out}/settingup-own-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 900 });
  await saving.getByRole('button', { name: 'Save', exact: true }).click();
  await saving.waitFor({ state: 'detached' });
  await again.getByRole('status').getByText('Saved as a-research-project.', { exact: true }).waitFor();
  const chosenSetup = again.getByLabel(/^Set up as/);
  await chosenSetup.waitFor();
  expect(`the dialog to be set up as what was saved, got ${await chosenSetup.inputValue()}`, (await chosenSetup.inputValue()) === 'a-research-project');
  const kept = (await asked('/api/setups')).setups.find((one) => one.name === 'a-research-project');
  const on = kept.rows.filter((row) => row.on).map((row) => row.name);
  expect(`the setup to keep every row with what is on, got ${on.join(', ')}`, on.join() === 'board,fleet,mockups');
  expect('no Remove for a row that is the setup’s own now', (await again.getByRole('button', { name: 'Remove mockups' }).count()) === 0);
  await again.getByRole('button', { name: 'Create', exact: true }).click();
  await again.waitFor({ state: 'detached' });
  const lab = (await asked('/api/instances?project=lab&live=true')).instances.map((one) => one.blueprint);
  expect(`the project to be set up as the setup says, got ${lab.join(', ')}`, lab.includes('std/mockups') && lab.includes('std/board') && !lab.includes('std/docs'));

  // Chosen at the top as a project is added, a setup's rows are its own; the factory's default is the usual.
  await panel();
  await page.getByRole('button', { name: 'New project' }).click();
  const third = page.getByRole('dialog');
  await third.getByLabel(/^Set up as/).waitFor();
  await third.getByRole('switch', { name: 'Docs', exact: true }).waitFor();
  expect('the default to be what ships, the library on', await third.getByRole('switch', { name: 'Docs', exact: true }).isChecked());
  await third.getByLabel(/^Set up as/).selectOption('a-research-project');
  await third.getByRole('switch', { name: 'mockups', exact: true }).waitFor();
  expect('the setup’s library off', !(await third.getByRole('switch', { name: 'Docs', exact: true }).isChecked()));
  for (const [width, height] of [
    [1280, 900],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    await page.screenshot({ path: `${out}/settingup-chosen-${width}.png` });
    await looked(`set up as a setup with a name, at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 900 });
  // Whoever manages the factory saves the table as its default: the next project starts with it unless told.
  await third.getByRole('button', { name: 'Save as…', exact: true }).click();
  await third.getByLabel(/^Save as/).selectOption('default');
  await third.getByRole('group', { name: 'Saving the table', exact: true }).getByRole('button', { name: 'Save', exact: true }).click();
  await third.getByRole('group', { name: 'Saving the table', exact: true }).waitFor({ state: 'detached' });
  const usual = (await asked('/api/setups')).usual.map((row) => `${row.name}${row.on ? '' : ' off'}`);
  expect(`the default to be the table as it was saved, got ${usual.join(', ')}`, usual.join() === 'board,fleet,docs off,features off,timed off,ab off,pipelines off,environments off,flags off,mockups');
  await third.getByRole('button', { name: 'Cancel', exact: true }).click();
  await third.waitFor({ state: 'detached' });

  expect(`no errors on the page: ${problems.join(' | ')}`, problems.length === 0);
  await browser.close();
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
