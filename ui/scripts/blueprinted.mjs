// Where a project keeps blueprints of its own, from the gear of its page,
// on a page that is never read again: a folder taken up as a project that
// keeps every blueprint that ships under `blueprints/std/` (as this
// program's own source does), and none of them read as its own, since a
// project that says nothing keeps its own where the factory keeps them,
// and none of its repository's files are read as one: the board's page
// offers Fork and says nothing of a fork. The gear saying where they are,
// kept by the factory chosen, a folder of the repository too and none;
// none chosen, and the board's page, open beside it and not read again,
// saying the project keeps none and offering no Fork; a folder of the
// repository asked in a dialog, one that leads out of the project refused
// there in the daemon's sentence with the dialog staying, and
// `blueprints` taken, chosen in the gear and said the same by the
// terminal, the board saying the project runs its own fork of it from
// there; what is under it moved into the factory from the gear, the
// project keeping its own where the factory keeps them again, the fork
// still its own and now a file of the factory's data folder, and the
// repository's file left where it was; its own listed under its
// Blueprints in the side panel; a new one made from the gear, a name that
// ships refused, kept by the factory and opened, where it is said leading
// back to that list, which has it, the bar saying it by its name under the
// project (`/shop/blueprints/tools/release`), which opens it as its file
// in a page opened there, and the fork of the board opened by its name
// under the open page, which is never read again, as the project's file
// and not what ships; nothing of the gear's menu, cut short
// or outside the window at any look, or of the dialogs over anything else
// at three widths and a phone's; and
// somebody who may not take a folder up, told where they are and offered
// nothing, and refused by the route.
//
//   DAEMON=../target/debug/factory node scripts/blueprinted.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { linesOf, menuOf, openGear } from './gear.mjs';
import { resourcesOf, unfold, unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'blueprinted-data');
const home = join(out, 'blueprinted-home');
const source = join(out, 'blueprinted-source');

await mkdir(out, { recursive: true });
for (const dir of [data, home, source]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
await mkdir(join(source, 'blueprints', 'std'), { recursive: true });
await writeFile(join(source, 'blueprints', 'std', 'board.bp'), ['blueprint: std/board', 'description: The source of the one that ships.', 'steps:', '  theirs: { do: noop }', ''].join('\n'));
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop', '--path', source);
  await cli('project', 'trust', 'shop');

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
  const signIn = async (email) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
    const opened = await context.newPage();
    opened.setDefaultTimeout(20_000);
    opened.on('pageerror', (err) => problems.push(err.message));
    await opened.goto(url);
    await opened.getByLabel('Email').fill(email);
    await opened.getByLabel('Password').fill('correct horse');
    await opened.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await opened.locator('.rail').waitFor();
    return opened;
  };
  const goes = (on, to) => on.evaluate((hash) => (location.hash = hash), to);
  const page = await signIn('ada@example.com');
  globalThis.lastPage = page;
  const showing = (on = page) => on.locator('.view').locator('visible=true');
  const summary = (on = page) => on.locator('.summary[data-of="std/board"]').locator('visible=true');
  const forkButton = (on = page) => on.locator('.view-head').locator('visible=true').getByRole('button', { name: 'Fork', exact: true });
  const crowded = async (what, within = 'body') => {
    const wrong = await pageCrowded(page, within);
    expect(`nothing of ${what} over anything else: ${wrong.join('; ')}`, wrong.length === 0);
  };

  // The board's page for the project: nothing of its repository is read as its own, so there is no fork and Fork is offered.
  await goes(page, '/blueprint/std/board?project=shop');
  await forkButton().waitFor();
  await summary().getByText(/^Fork it/).waitFor();
  expect('nothing said of a fork it runs', (await summary().getByText('This project runs its own fork of it').count()) === 0);
  await page.screenshot({ path: `${out}/blueprinted-0-not-read.png` });

  // The project's page: its gear says where its blueprints are, kept by the factory chosen.
  await goes(page, '/project/shop');
  await showing().getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await openGear(page);
  await menuOf(page).getByText('Its own blueprints are kept by the factory').waitFor();
  const lines = await linesOf(page);
  for (const line of ['Kept by the factory', 'Also a repo folder…', 'None of its own']) expect(`a line ${line}, got ${lines.join(' | ')}`, lines.includes(line));
  const chosen = await menuOf(page).locator('[aria-checked="true"]').innerText();
  expect(`the factory chosen, got ${chosen}`, chosen.includes('Kept by the factory'));
  await crowded("the gear's menu");
  await page.screenshot({ path: `${out}/blueprinted-0-usual.png` });

  // None: said, and the board's page (a tab not read again) says the project keeps none and offers no Fork.
  await menuOf(page).getByRole('menuitemradio', { name: 'None of its own' }).click();
  await page.locator('.toast', { hasText: 'Shop keeps no blueprints of its own now' }).waitFor();
  await goes(page, '/blueprint/std/board?project=shop');
  await summary().getByText('This project keeps no blueprints of its own').waitFor();
  expect('no Fork for a project that keeps none', (await forkButton().count()) === 0);
  expect('nothing said of a fork it runs', (await summary().getByText('This project runs its own fork of it').count()) === 0);
  await page.screenshot({ path: `${out}/blueprinted-1-none.png` });

  // A folder of its repository: one that leads out of the project is refused in the daemon's sentence, and the dialog stays.
  await goes(page, '/project/shop');
  await showing().getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await openGear(page);
  expect('none chosen', (await menuOf(page).locator('[aria-checked="true"]').innerText()).includes('None of its own'));
  await menuOf(page).getByRole('menuitemradio', { name: 'Also a repo folder…' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('heading', { name: "A folder of Shop's repository for its blueprints" }).waitFor();
  await dialog.getByLabel('Folder').fill('../elsewhere');
  await dialog.getByRole('button', { name: 'Read it' }).click();
  await dialog.getByRole('alert').getByText('does not stay within the project').waitFor();
  expect('the dialog to stay with what was written', (await dialog.getByLabel('Folder').inputValue()) === '../elsewhere');
  for (const [width, height] of [
    [1280, 860],
    [900, 800],
    [640, 800],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    await crowded(`the dialog at ${width}`, '.modal');
    await page.screenshot({ path: `${out}/blueprinted-2-refused-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await dialog.getByLabel('Folder').fill('blueprints');
  await dialog.getByRole('button', { name: 'Read it' }).click();
  await page.locator('.toast', { hasText: "Shop's own blueprints are under blueprints/ of its repository, and kept by the factory now" }).waitFor();
  await dialog.waitFor({ state: 'detached' });
  const said = (await cli('project', 'blueprints', 'shop')).stdout;
  expect(`the terminal to say the same, got ${said}`, said.includes('under `blueprints/` of its repository'));

  // Chosen in the gear, at three widths and a phone's, with nothing of the menu over anything else.
  for (const [width, height] of [
    [1280, 860],
    [900, 800],
    [640, 800],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    await openGear(page);
    await menuOf(page).getByText('Its own blueprints are under blueprints/ of its repository').waitFor();
    expect('the folder chosen', (await menuOf(page).locator('[aria-checked="true"]').innerText()).includes('Also under blueprints/'));
    await crowded(`the gear's menu at ${width}`);
    // No line's words are cut short, at any look: the menu is as much wider as text is larger, within the window.
    const cut = await menuOf(page).evaluate((box) => [...box.querySelectorAll('.truncate, .hint')].filter((part) => part.scrollWidth > part.clientWidth + 0.5).map((part) => part.textContent));
    expect(`no line of the gear cut short at ${width}, got ${JSON.stringify(cut)}`, cut.length === 0);
    const menuBox = await menuOf(page).boundingBox();
    expect(`the gear's menu within the window at ${width}`, menuBox.x >= 0 && menuBox.x + menuBox.width <= width + 0.5);
    await page.screenshot({ path: `${out}/blueprinted-3-folder-at-${width}.png` });
    await page.keyboard.press('Escape');
    await menuOf(page).waitFor({ state: 'detached' });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // The board's page, not read again, says the project runs its own fork from there, and offers no Fork.
  await goes(page, '/blueprint/std/board?project=shop');
  await summary().getByText('This project runs its own fork of it').first().waitFor();
  expect('no Fork while the project runs one', (await forkButton().count()) === 0);
  await page.screenshot({ path: `${out}/blueprinted-4-repository.png` });

  // Moved into the factory from the gear: kept by the factory again, the fork still the project's own, in the data folder, and the repository's file left.
  await goes(page, '/project/shop');
  await showing().getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await openGear(page);
  await menuOf(page).getByRole('menuitem', { name: 'Move them in' }).click();
  await page.locator('.toast', { hasText: "1 of Shop's own blueprints are kept by the factory now" }).waitFor();
  await openGear(page);
  await menuOf(page).getByText('Its own blueprints are kept by the factory').waitFor();
  expect('the factory chosen again', (await menuOf(page).locator('[aria-checked="true"]').innerText()).includes('Kept by the factory'));
  await page.keyboard.press('Escape');
  const kept = join(data, 'blueprints', 'projects', 'shop', 'std', 'board.bp');
  expect('the fork in the factory\'s data folder', (await readFile(kept, 'utf8')).includes('The source of the one that ships.'));
  expect('the repository\'s file left where it was', (await readFile(join(source, 'blueprints', 'std', 'board.bp'), 'utf8')).includes('theirs'));
  await goes(page, '/blueprint/std/board?project=shop');
  await summary().getByText('This project runs its own fork of it').first().waitFor();
  await page.screenshot({ path: `${out}/blueprinted-5-moved-in.png` });

  // Its own listed under its Blueprints in the side panel, from where the factory keeps them.
  await unfoldAll(page);
  const resources = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  const shop = resourcesOf(page).locator('[data-project="shop"]');
  const ownList = shop.locator(':scope > .tree > div', { has: page.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item', { hasText: 'Blueprints' }) });
  await unfold(page, ownList.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item'));
  await ownList.locator(':scope > .tree > .nav-item', { hasText: 'std/board' }).waitFor();

  // A new one of its own, from the gear: a name that ships refused before anything is asked, one made, kept by the factory and opened.
  await goes(page, '/project/shop');
  await showing().getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await openGear(page);
  await menuOf(page).getByRole('menuitem', { name: 'New blueprint…' }).click();
  const naming = page.getByRole('dialog');
  await naming.getByRole('heading', { name: "A new blueprint of Shop's own" }).waitFor();
  await naming.getByLabel('Name').fill('std/fleet');
  await naming.getByText('std/fleet ships with the factory').waitFor();
  expect('nothing to make for a name that ships', await naming.getByRole('button', { name: 'Make the blueprint' }).isDisabled());
  await naming.getByLabel('Name').fill('tools/release');
  for (const [width, height] of [
    [1280, 860],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    await crowded(`the dialog that names a blueprint at ${width}`, '.modal');
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/blueprinted-6-naming.png` });
  await naming.getByRole('button', { name: 'Make the blueprint' }).click();
  await naming.waitFor({ state: 'detached' });
  const made = join(data, 'blueprints', 'projects', 'shop', 'tools', 'release.bp');
  expect('the new one in the factory\'s data folder', (await readFile(made, 'utf8')).includes('blueprint: tools/release'));
  const crumbs = page.locator('.crumbs').locator('visible=true');
  await crumbs.getByRole('heading', { name: 'release.bp' }).waitFor();
  // Its address is its name under the project, which the bar says while its tab shows, on this page never read again.
  await page.waitForFunction((to) => location.href === to, `${url}/shop/blueprints/tools/release`);
  // The fork of the board that ships, opened by its name under the project, by an address under this open page: the
  // project's file, as its Blueprints line opens it, and no page of what ships.
  await page.evaluate(() => {
    history.replaceState(history.state, '', '/shop/blueprints/std/board');
    location.hash = 'L2';
  });
  await crumbs.getByRole('heading', { name: 'board.bp' }).waitFor();
  await page.waitForFunction((to) => location.href === to, `${url}/shop/blueprints/std/board`);
  expect('the fork, and no page of what ships', (await showing().getByText('ships with the product').count()) === 0);
  await page.screenshot({ path: `${out}/blueprinted-6-fork-by-name.png` });
  // What ships, opened for the project whose fork stands in for it, is said as that, and opens it again.
  await goes(page, '/blueprint/std/board?project=shop');
  await summary().getByText('This project runs its own fork of it').first().waitFor();
  await page.waitForFunction((to) => location.href === to, `${url}/blueprint/std/board?project=shop`);
  // A page opened at the name address of one of its own opens it as its file.
  const fresh = await page.context().newPage();
  fresh.on('pageerror', (err) => problems.push(err.message));
  await fresh.goto(`${url}/shop/blueprints/tools/release`);
  await fresh.locator('.crumbs').locator('visible=true').getByRole('heading', { name: 'release.bp' }).waitFor();
  expect(`the bar to say its name address, got ${fresh.url()}`, fresh.url() === `${url}/shop/blueprints/tools/release`);
  await fresh.close();
  await goes(page, '/file/shop/@blueprints/tools/release.bp');
  await crumbs.getByRole('heading', { name: 'release.bp' }).waitFor();
  // Where it is said leads back to the list of the project's blueprints, which has it, and to no folder of the repository.
  await crumbs.getByRole('button', { name: 'Blueprints', exact: true }).click();
  await ownList.locator(':scope > .tree > .nav-item', { hasText: 'tools/release' }).waitFor();
  const focused = await page.evaluate(() => document.activeElement?.textContent?.trim());
  expect(`the list's line to have the keyboard, got ${focused}`, focused === 'Blueprints');
  await ownList.locator('.own-new .nav-item', { hasText: 'New blueprint…' }).waitFor();
  await page.screenshot({ path: `${out}/blueprinted-7-listed.png` });

  // Somebody who may not take a folder up is told where they are and offered nothing, and the route refuses them.
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  const other = await signIn('grace@example.com');
  await goes(other, '/project/shop');
  await other.locator('.view').locator('visible=true').getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await openGear(other);
  await menuOf(other).getByText('Its own blueprints are kept by the factory').waitFor();
  expect('a member offered none of the lines', (await menuOf(other).locator('[role="menuitemradio"]').count()) === 0);
  const refused = await other.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const read = await fetch('/api/projects/shop/blueprints', { headers: mine });
    const set = await fetch('/api/projects/shop/blueprints', { method: 'POST', headers: mine, body: JSON.stringify({ none: true }) });
    return [read.status, set.status];
  });
  expect(`a member to read it and be refused saying it, got ${refused}`, refused.join() === '200,403');
  await other.screenshot({ path: `${out}/blueprinted-8-member.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await globalThis.lastPage?.screenshot({ path: `${out}/blueprinted-failed.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, source]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
