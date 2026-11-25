// Browsing a project's files: folder by folder on its page, each file read
// before it is changed, pictures shown, and what is not text said to be so.
// And something new made there: a file and a folder, from the Repo page's
// head and gear and from the side panel's Repo line and folders' lines,
// each where it was pressed, a name refused before it is asked and one
// the daemon refuses said in its sentence, and nothing offered to
// whoever may not write the project's files.
//
//   DAEMON=../target/debug/factory node scripts/files.mjs [out-dir]
//
// Nothing here is about source control: it is the files as they are on
// disk. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { pageCrowded } from './crowded.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { gearOf, menuOf } from './gear.mjs';
import { saysWhere } from './where.ts';

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
const data = join(out, 'files-data');
const home = join(out, 'files-home');
// The project's folder is one that was said, so that its Repo is listed under it: a folder of its own, beside the data.
const shop = join(out, 'files-shop');

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
  const page = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
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

  // Files put straight into the project's folder, as an agent or an editor would.
  const where = await page.evaluate(async () => (await (await fetch('/api/projects', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).projects[0].path);
  await mkdir(join(where, 'src'), { recursive: true });
  await writeFile(join(where, 'src/main.rs'), 'fn main() {\n    let greeting = "hello";\n    println!("{greeting}");\n}\n');
  await writeFile(join(where, 'src/README.md'), '# Source\n\nWhere the program is.\n');
  await writeFile(join(where, 'data.json'), '{ "cards": 3 }\n');
  // The smallest picture there is, and something that is neither text nor a picture.
  const dot = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
  await writeFile(join(where, 'logo.png'), Buffer.from(dot, 'base64'));
  await writeFile(join(where, 'blob.bin'), Buffer.from([0xff, 0xfe, 0x00, 0x80, 0x81]));

  // The project's front page is about how it stands, not its code: no
  // listing of files there. What it has is a place to find one by typing.
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Shop', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  expect('no listing of files on the front page', (await page.locator('.files').count()) === 0);
  const find = page.getByLabel('Find in Shop');
  const found = page.locator('.find-results .hit');
  // A file is found by part of its name, as it is typed.
  await find.fill('mai');
  await found.filter({ hasText: 'main.rs' }).waitFor({ timeout: 8000 });
  expect(`a file to be found by part of its name, with where it is, got ${await found.first().innerText()}`, /main\.rs\s*src/.test((await found.first().innerText()).replace(/\n/g, ' ')));
  // And by what is written in it, which is the daemon's search.
  await find.fill('greeting');
  await found.filter({ hasText: 'main.rs' }).waitFor({ timeout: 8000 });
  await page.screenshot({ path: `${out}/files-0-find.png` });
  await find.fill('logo');
  await found.filter({ hasText: 'logo.png' }).waitFor();
  await find.fill('nothing-is-called-this');
  await page.locator('.find-none').waitFor();
  // The one that is marked opens with Enter.
  await find.fill('data.j');
  await found.filter({ hasText: 'data.json' }).waitFor();
  await page.keyboard.press('Enter');
  await page.locator('.cm-content', { hasText: '"cards": 3' }).waitFor();

  // Its files are a page of their own, under the project: the folder as it
  // is. It looks when it is first opened.
  await page.locator('.nav').getByText('Repo', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop repo', level: 1 }).waitFor();
  const files = page.locator('.files');
  const names = () => files.locator('tbody tr td:first-child').allInnerTexts().then((all) => all.map((one) => one.replace(/^\S+\s+/, '').trim()));
  await files.getByText('logo.png').waitFor({ timeout: 8000 }).catch(async (err) => {
    await page.screenshot({ path: `${out}/files-0-failed.png` });
    throw new Error(`${err.message}\nlisted: ${await files.innerText().catch(() => 'no listing')}`);
  });
  const top = await names();
  expect(`folders first and then files, by name, got ${top}`, top.join() === 'docs,src,blob.bin,data.json,logo.png,README.md');
  expect('each file to say how large it is', (await files.locator('tr', { hasText: 'data.json' }).innerText()).includes('15 B'));
  await page.screenshot({ path: `${out}/files-1-top.png` });

  // A folder opens in place, with the way back and its own README under it.
  await files.getByText('src', { exact: true }).click();
  await files.getByText('main.rs').waitFor();
  expect(`the folder's entries, got ${await names()}`, (await names()).join() === '..,main.rs,README.md');
  expect('where it is to be said', (await files.locator('.crumbs').innerText()).replace(/\s+/g, ' ') === 'Shop / src');
  // Its own README is under it, once that has been read and built.
  await page.waitForFunction(() => document.querySelector('iframe.page-frame')?.getAttribute('srcdoc')?.includes('<h1>Source</h1>'));
  // It is shown whole: as tall as it is, with nothing of its own to scroll,
  // so that what scrolls is the page it is on. And nothing in it can run.
  const whole = await page.locator('iframe.page-frame.whole').evaluate(async (frame) => {
    for (let tries = 0; tries < 40 && frame.clientHeight < 60; tries += 1) await new Promise((soon) => setTimeout(soon, 50));
    const inside = frame.contentDocument?.documentElement;
    return { tall: frame.clientHeight, page: Math.ceil(inside?.getBoundingClientRect().height ?? -1), scrolls: frame.getAttribute('scrolling'), sandbox: frame.getAttribute('sandbox') };
  });
  expect(`the README to be as tall as its page, got ${JSON.stringify(whole)}`, whole.page > 0 && Math.abs(whole.tall - whole.page) <= 2 && whole.scrolls === 'no');
  expect(`its frame to let nothing in it run, got ${whole.sandbox}`, whole.sandbox === 'allow-same-origin');
  await page.screenshot({ path: `${out}/files-2-folder.png` });

  // A file is read before it is changed: coloured, numbered, and not editable.
  await files.getByText('main.rs').click();
  const text = page.locator('.cm-content');
  await text.waitFor();
  expect('it to open for reading', (await text.getAttribute('contenteditable')) === 'false');
  await page.waitForFunction(() => document.querySelectorAll('.cm-content .cm-line span').length > 3);
  expect('it to say how long it is', (await page.locator('.view-head').innerText()).includes('4 lines'));
  await page.screenshot({ path: `${out}/files-3-reading.png` });
  // Edit, and it can be changed and saved.
  await page.getByRole('tab', { name: 'Edit' }).click();
  await page.waitForFunction(() => document.querySelector('.cm-content')?.getAttribute('contenteditable') === 'true');
  await text.click();
  await page.keyboard.press('Control+End');
  await page.waitForTimeout(150);
  await page.keyboard.type('// the end');
  await page.locator('.view-head').getByText('unsaved').waitFor();
  await page.keyboard.press('Control+s');
  await page.waitForFunction(() => !document.querySelector('.view-head')?.textContent.includes('unsaved'));
  const written = await readFile(join(where, 'src/main.rs'), 'utf8');
  expect(`what was typed to be in the file, which is now ${JSON.stringify(written)}`, written.includes('// the end') && written.startsWith('fn main()'));
  await page.getByRole('tab', { name: 'View' }).click();
  await page.waitForFunction(() => document.querySelector('.cm-content')?.getAttribute('contenteditable') === 'false');

  // Its path leads back: to its folder, and from there up.
  await page.locator('.view-head .crumbs').getByRole('button', { name: 'src' }).click();
  await files.getByText('main.rs').waitFor();
  await files.getByText('..').click();
  await files.getByText('logo.png').waitFor();

  // Something added while the page is open is found when it is asked to look again.
  await writeFile(join(where, 'added-later.txt'), 'hello\n');
  await files.getByRole('button', { name: 'Look again' }).click();
  await files.getByText('added-later.txt').waitFor();

  // A picture is shown as the picture it is.
  await files.getByText('logo.png').click();
  const picture = page.locator('.file-picture img');
  await picture.waitFor();
  await page.waitForFunction(() => document.querySelector('.file-picture img')?.naturalWidth === 1);
  await page.screenshot({ path: `${out}/files-4-picture.png` });

  // What is neither is said to be, and can be saved from here.
  await page.locator('.tab', { hasText: 'Shop repo' }).click();
  await files.getByText('blob.bin').click();
  await page.getByText('This file is not text').waitFor();
  expect('a way to save a copy of it', (await page.getByRole('link', { name: 'Save a copy of it' }).getAttribute('href')).includes('/raw?path=blob.bin'));

  // A blueprint is one whichever of its names its file has: under the name
  // that every editor takes for YAML it is marked as one in the listing and
  // opens as one, to be applied. A file that is YAML and no more is a file.
  await writeFile(join(where, 'greet.bp.yml'), 'blueprint: greet\nsteps:\n  said: { do: emit, with: { topic: shop.greeted } }\n');
  await writeFile(join(where, 'compose.yml'), 'services: {}\n');
  await page.locator('.tab', { hasText: 'Shop repo' }).click();
  await files.getByRole('button', { name: 'Look again' }).click();
  await files.getByText('greet.bp.yml').waitFor();
  const mark = (name) => files.locator('tr', { hasText: name }).locator('.glyph').innerText();
  expect(`a blueprint called as YAML to be marked as a blueprint, got ${await mark('greet.bp.yml')}`, (await mark('greet.bp.yml')) === '◆');
  expect(`YAML that is nobody's blueprint to be marked as a file, got ${await mark('compose.yml')}`, (await mark('compose.yml')) === '·');
  await files.getByText('greet.bp.yml').click();
  await page.getByRole('button', { name: 'Apply', exact: true }).waitFor();
  await page.screenshot({ path: `${out}/files-5-blueprint-as-yaml.png` });

  // ---- something new made in the repo
  const dialog = page.getByRole('dialog');
  const says = dialog.locator('#repo-made-says');
  const head = page.locator('.view-head');
  /** Nothing of the page lying over anything else or past its sides, at three widths and a phone's, as it stands. */
  const uncrowded = async (what) => {
    for (const [width, height] of [[1280, 860], [900, 760], [700, 760], [390, 844]]) {
      await page.setViewportSize({ width, height });
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
      const wrong = await pageCrowded(page);
      if (wrong.length) {
        await page.screenshot({ path: `${out}/files-crowded-${width}.png` });
        throw new Error(`expected nothing of ${what} over anything else at ${width} wide, got:\n${wrong.join('\n')}`);
      }
    }
    await page.setViewportSize({ width: 1280, height: 860 });
  };
  /** Names what is being made in the dialog, and makes it. */
  const make = async (name) => {
    await dialog.getByLabel('Name').fill(name);
    await dialog.getByRole('button', { name: /^Make the / }).click();
    await dialog.waitFor({ state: 'detached' });
  };
  const there = async (path) => readFile(join(where, path), 'utf8');

  // From the head of the Repo page, at its top: a file, which opens to be written in.
  await page.locator('.tab', { hasText: 'Shop repo' }).click();
  await files.getByText('logo.png').waitFor();
  expect('the Repo page to offer a new file and a new folder in its head', (await head.getByRole('button', { name: 'New file', exact: true }).count()) === 1 && (await head.getByRole('button', { name: 'New folder', exact: true }).count()) === 1);
  await head.getByRole('button', { name: 'New file', exact: true }).click();
  await dialog.getByLabel('Name').waitFor();
  expect(`the dialog to say where it goes, got ${await says.innerText()}`, (await says.innerText()).includes('at the top of the repo'));
  await dialog.getByLabel('Name').fill('plan.md');
  await says.getByText('beginning with a heading of its name', { exact: false }).waitFor();
  await page.screenshot({ path: `${out}/files-6-new-file.png` });
  await uncrowded('the dialog that names a new file');
  // A name that is there already is refused before anything is asked, and the dialog stays.
  await dialog.getByLabel('Name').fill('data.json');
  await says.getByText('There is a file called data.json here already.').waitFor();
  expect('nothing to be made of a name that is there', await dialog.getByRole('button', { name: 'Make the file' }).isDisabled());
  await dialog.getByLabel('Name').fill('a/b.md');
  await says.getByText('One name, without a slash', { exact: false }).waitFor();
  // One the page did not know of is refused by the daemon, in its sentence, with what was typed still there.
  await dialog.getByLabel('Name').fill('late.md');
  await writeFile(join(where, 'late.md'), 'written elsewhere\n');
  await dialog.getByRole('button', { name: 'Make the file' }).click();
  await dialog.getByRole('alert').getByText('there is a file called late.md in the project already: call the new one something else').waitFor();
  expect('what was typed to stay', (await dialog.getByLabel('Name').inputValue()) === 'late.md');
  expect(`the file somebody had to be as it was, got ${await there('late.md')}`, (await there('late.md')) === 'written elsewhere\n');
  await page.screenshot({ path: `${out}/files-7-refused.png` });
  await make('plan.md');
  await page.locator('.tab', { hasText: 'plan.md' }).waitFor();
  await page.getByRole('heading', { name: 'plan', exact: true }).first().waitFor();
  expect(`a Markdown file to begin with its name, got ${JSON.stringify(await there('plan.md'))}`, (await there('plan.md')) === '# plan\n');

  // From the gear, a folder: shown on the page, where the next file is made in it.
  await page.locator('.tab', { hasText: 'Shop repo' }).click();
  await files.getByText('plan.md').waitFor();
  await gearOf(page).click();
  await menuOf(page).getByRole('menuitem', { name: 'New folder…' }).click();
  await dialog.getByLabel('Name').waitFor();
  await make('notes');
  await page.waitForFunction(() => document.querySelector('.files .crumbs')?.textContent.replace(/\s+/g, ' ').trim() === 'Shop / notes');
  await files.getByText('This folder is empty.').waitFor();
  await head.getByRole('button', { name: 'New file', exact: true }).click();
  await dialog.getByLabel('Name').waitFor();
  expect(`a file made in the folder the page shows, said so, got ${await says.innerText()}`, (await says.innerText()).includes('in notes/'));
  await make('today.txt');
  await page.locator('.tab', { hasText: 'today.txt' }).waitFor();
  expect('a file that is not Markdown to be made empty', (await there('notes/today.txt')) === '');
  await page.locator('.tab', { hasText: 'Shop repo' }).click();
  await files.getByText('today.txt').waitFor();
  await page.screenshot({ path: `${out}/files-8-in-a-folder.png` });
  await uncrowded('the Repo page with what it makes in its head');

  // From the side panel: the Repo line's menu at the top, and a folder's menu in that folder.
  await side(page, 'Resources');
  const panel = page.locator('nav.sidebar[aria-label="Resources"]');
  await unfold(page, 'Shop repo');
  const shopTree = panel.locator('[data-project="shop"]');
  await shopTree.getByRole('button', { name: 'More for notes', exact: true }).click();
  expect(`a folder's menu to make a file and a folder, got ${await menuOf(page).innerText()}`, (await menuOf(page).getByRole('menuitem', { name: 'New file…' }).count()) === 1 && (await menuOf(page).getByRole('menuitem', { name: 'New folder…' }).count()) === 1);
  await page.screenshot({ path: `${out}/files-9-folder-menu.png` });
  await menuOf(page).getByRole('menuitem', { name: 'New folder…' }).click();
  await dialog.getByLabel('Name').waitFor();
  expect(`a folder made from a folder's line to go in it, got ${await says.innerText()}`, (await says.innerText()).includes('in notes/'));
  await make('2026');
  await page.waitForFunction(() => document.querySelector('.files .crumbs')?.textContent.replace(/\s+/g, ' ').trim() === 'Shop / notes / 2026');
  await shopTree.getByText('2026', { exact: true }).waitFor();
  await shopTree.getByRole('button', { name: 'More for 2026', exact: true }).click();
  await menuOf(page).getByRole('menuitem', { name: 'New file…' }).click();
  await make('march.md');
  await page.locator('.tab', { hasText: 'march.md' }).waitFor();
  expect('a file made in a folder made a moment before', (await there('notes/2026/march.md')) === '# march\n');
  await shopTree.getByText('march.md', { exact: true }).waitFor();
  await shopTree.getByRole('button', { name: 'More for Shop repo', exact: true }).click();
  await menuOf(page).getByRole('menuitem', { name: 'New folder…' }).click();
  await dialog.getByLabel('Name').waitFor();
  expect(`a folder made from the Repo line to go at its top, got ${await says.innerText()}`, (await says.innerText()).includes('at the top of the repo'));
  await dialog.getByLabel('Name').fill('Notes');
  await says.getByText('There is a folder called notes here already.').waitFor();
  await make('archive');
  await shopTree.getByText('archive', { exact: true }).waitFor();
  await page.screenshot({ path: `${out}/files-10-made-from-the-panel.png` });
  await uncrowded('the side panel with what was made in it');

  // Somebody who may not write the project's files is offered none of it, and the daemon refuses them.
  const theirs = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const reader = await theirs.newPage();
  reader.on('pageerror', (err) => problems.push(err.message));
  await reader.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const me = await answer.json();
    if (!me.principal) return route.fulfill({ response: answer });
    me.principal.scopes = ['projects:read', 'board', 'records', 'channels', 'search'];
    await route.fulfill({ response: answer, json: me });
  });
  await reader.goto(url);
  await reader.getByLabel('Email').fill('ada@example.com');
  await reader.getByLabel('Password').fill('correct horse');
  await reader.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await reader.locator('.rail').waitFor();
  await side(reader, 'Resources');
  await unfold(reader, 'Shop repo');
  const theirTree = reader.locator('nav.sidebar[aria-label="Resources"] [data-project="shop"]');
  await theirTree.getByText('notes', { exact: true }).waitFor();
  expect('no menu on the Repo line or a folder\'s for them', (await theirTree.getByRole('button', { name: /^More for (Shop repo|notes|archive)$/ }).count()) === 0 && (await theirTree.getByRole('button', { name: 'More for Shop', exact: true }).count()) === 1);
  await theirTree.getByText('Repo', { exact: true }).click();
  await reader.getByRole('heading', { name: 'Shop repo', level: 1 }).waitFor();
  expect('no button to make anything in the head for them', (await reader.locator('.view-head').getByRole('button', { name: /^New / }).count()) === 0);
  await gearOf(reader).click();
  await menuOf(reader).waitFor();
  expect(`no line to make anything in the gear for them, got ${await menuOf(reader).innerText()}`, (await menuOf(reader).getByRole('menuitem', { name: /^New / }).count()) === 0);
  await reader.screenshot({ path: `${out}/files-11-not-theirs.png` });
  await theirs.close();
  const key = await page.evaluate(async () => {
    const asked = await fetch('/api/credentials', { method: 'POST', headers: { 'content-type': 'application/json', 'x-page-key': localStorage.getItem('page-key') ?? '' }, body: JSON.stringify({ name: 'reader', scopes: ['projects:read'] }) });
    return (await asked.json()).token;
  });
  for (const [method, path, body] of [['POST', '/api/projects/shop/folders', { path: 'theirs' }], ['PUT', '/api/projects/shop/file', { path: 'theirs.md', content: '', new: true }]]) {
    const answer = await fetch(`${url}${path}`, { method, headers: { authorization: `Bearer ${key}`, 'content-type': 'application/json' }, body: JSON.stringify(body) });
    expect(`the daemon to refuse somebody who may not write the project's files, ${method} ${path} got ${answer.status}`, answer.status === 403);
  }

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
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
