// Replaces the client under an open page, with the daemon running throughout,
// and takes pictures of what the page does about it; and a page whose client
// did not arrive whole as it opened, which loads itself once more and, where
// that fails too, says so.
//
//   DAEMON=../target/debug/factory node scripts/update.mjs [out-dir]
//
// The daemon serves a copy of dist/, so nothing here touches the real build.
// Each "new build" is the real one under another id, put in place the way
// build.mjs does it: files first, then the entry page in one step. The last
// ones are whole builds of their own (every file under a name of that
// build's) kept and tidied by the build's own rule, so that a page left
// open across several of them is shown to have every part of its own. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { access, cp, mkdir, readFile, readdir, rename, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { generations, leftovers } from './generations.ts';
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
const client = join(out, 'update-client');
const data = join(out, 'update-data');
const home = join(out, 'update-home');

const original = await readFile('dist/index.html', 'utf8');
const script = original.match(/src="\/(assets\/app-[^"]+\.js)"/)[1];

/** Puts a build called `id` in place of the one being served. */
async function publish(id, live = false) {
  const renamed = `assets/app-${id}.js`;
  await writeFile(join(client, renamed), `${await readFile(join('dist', script), 'utf8')}\n// ${id}\n`);
  const page = original
    .replace(script, renamed)
    .replace(/name="build" content="[^"]*" data-live="[^"]*"/, `name="build" content="${id}" data-live="${live ? 'yes' : 'no'}"`);
  await writeFile(join(client, 'index.html.new'), page);
  await rename(join(client, 'index.html.new'), join(client, 'index.html'));
}

/**
 * Puts a whole build called `id` in place as build.mjs does: every file of
 * the real build under a name of this one's (and what names it in the
 * others named so), then the list of builds kept, then the entry page, and
 * then every file no kept build lists taken away.
 */
async function wholeBuild(id) {
  const kept = JSON.parse(await readFile(join(client, 'builds.json'), 'utf8'));
  const named = (file) => file.replace(/^(assets\/[^.]+)\./, `$1-${id}.`);
  const base = (file) => file.slice('assets/'.length);
  const names = new RegExp(real.files.map((file) => base(file).replace(/[.]/g, '\\.')).join('|'), 'g');
  const renamed = (text) => text.replace(names, (name) => base(named(`assets/${name}`)));
  for (const file of real.files) await writeFile(join(client, named(file)), renamed(await readFile(join('dist', file), 'utf8')));
  const now = generations(kept, { id, at: Date.now(), files: real.files.map(named) });
  await writeFile(join(client, 'builds.json'), JSON.stringify(now));
  const page = renamed(original).replace(/name="build" content="[^"]*" data-live="[^"]*"/, `name="build" content="${id}" data-live="no"`);
  await writeFile(join(client, 'index.html.new'), page);
  await rename(join(client, 'index.html.new'), join(client, 'index.html'));
  const present = (await readdir(join(client, 'assets'))).map((name) => `assets/${name}`);
  await Promise.all(leftovers(now, present).map((file) => rm(join(client, file), { force: true })));
}
const originalBuild = original.match(/name="build" content="([^"]*)"/)[1];
const real = JSON.parse(await readFile('dist/builds.json', 'utf8')).find((build) => build.id === originalBuild);

await mkdir(out, { recursive: true });
for (const dir of [client, data, home]) await rm(dir, { recursive: true, force: true });
await cp('dist', client, { recursive: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--ui', client], {
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
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const shot = async (name) => {
    await page.screenshot({ path: `${out}/update-${name}.png` });
    console.log(name);
  };
  const build = () => page.locator('meta[name="build"]').getAttribute('content');
  const starts = () =>
    page.evaluate(() => fetch('/api/events/recent?topics=system.started', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } }).then((r) => r.json()).then((body) => body.events.length));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };

  // A page whose client did not arrive whole as it opened: a browser drops what is on its way when
  // the system says its network changed, and the page drew nothing and said nothing. It loads itself
  // once more, and where that fails too it says so, once.
  const opening = async (dropping) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    const opened = await context.newPage();
    let left = dropping;
    let loads = 0;
    opened.on('request', (request) => request.isNavigationRequest() && (loads += 1));
    await opened.route('**/assets/chunk-*.js', (route) => (left-- > 0 ? route.abort('internetdisconnected') : route.continue()));
    await opened.goto(url);
    return { opened, context, loads: () => loads };
  };
  const once = await opening(1);
  await once.opened.getByLabel('Email').waitFor();
  expect(`the page to have loaded itself once more, and no more, got ${once.loads()} loads`, once.loads() === 2);
  await once.opened.screenshot({ path: `${out}/update-0-loaded-again.png` });
  await once.context.close();
  const never = await opening(Infinity);
  const unloaded = never.opened.locator('#app.empty');
  await unloaded.waitFor();
  const said = await unloaded.textContent();
  expect(`the page to say that it could not be loaded, got "${said}"`, said === 'The client could not be loaded: part of it did not arrive. Reload to try again.');
  expect(`the page to have tried once more and no more, got ${never.loads()} loads`, never.loads() === 2);
  await never.opened.screenshot({ path: `${out}/update-0-not-loaded.png` });
  await never.context.close();

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.sidebar').waitFor();
  const first = await build();
  await shot('1-before');

  // A new build arrives. The page is told, and offers; it does not reload.
  await publish('second');
  const notice = page.locator('.toast.notice', { hasText: 'A new version of the client is ready' });
  await notice.waitFor({ timeout: 15_000 });
  expect('the page to stay on the build it was loaded from', (await build()) === first);
  await shot('2-offered');

  // Still on the old build, it opens something it had not fetched yet.
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Reference', { exact: true }).click();
  await page.locator('.reference h1', { hasText: 'Reference' }).waitFor({ timeout: 40_000 });
  await shot('3-old-build-still-whole');

  await notice.getByRole('button', { name: 'Reload' }).click();
  await page.waitForFunction(() => document.querySelector('meta[name="build"]')?.getAttribute('content') === 'second');
  await page.locator('.sidebar').waitFor();
  expect('no notice once reloaded', (await page.locator('.toast.notice').count()) === 0);
  await shot('4-reloaded');

  // "Later" puts the offer away until there is another build.
  await publish('third');
  await notice.waitFor({ timeout: 15_000 });
  await notice.getByRole('button', { name: 'Later' }).click();
  expect('the notice to go when asked', (await page.locator('.toast.notice').count()) === 0);

  // A development build: a page on one reloads by itself.
  await publish('fourth', true);
  await notice.waitFor({ timeout: 15_000 });
  await page.reload();
  await page.locator('.sidebar').waitFor();
  expect('the page to be on the development build', (await build()) === 'fourth');
  await publish('fifth', true);
  await page.waitForFunction(() => document.querySelector('meta[name="build"]')?.getAttribute('content') === 'fifth', null, {
    timeout: 15_000,
  });
  await page.locator('.sidebar').waitFor();

  // A client newer than its daemon: the daemon lacks a route it asks for.
  await page.route('**/api/metrics**', (route) =>
    route.fulfill({
      status: 404,
      contentType: 'application/json',
      body: JSON.stringify({ error: { kind: 'no_such_route', message: 'this daemon has no `GET /api/metrics`' } }),
    }),
  );
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Measurements', { exact: true }).click();
  // The workspace has no project, and of no project nothing is asked: every workspace is what asks the daemon for what it measured.
  await page.getByLabel(/^What is shown of /).selectOption({ label: 'All workspaces' });
  await page.locator('.toast.notice', { hasText: 'newer than the daemon' }).waitFor({ timeout: 15_000 });
  await shot('5-daemon-behind');

  expect('the daemon to have started once and never again', (await starts()) === 1);
  // One line of JSON for each event.
  const listed = (await cli('events', '--topics', 'client.updated', '--json')).stdout.trim().split('\n');
  const builds = listed.map((line) => JSON.parse(line).data.build);
  console.log(`client.updated: ${builds.join(', ')}`);
  expect('every build to have been recorded, in order', builds.join() === 'second,third,fourth,fifth');

  // A page left open across three builds, each tidied as a build tidies,
  // opens the code editor for the first time and has it. The page on a
  // development build takes the first by itself; the next three it is
  // offered and does not take.
  await wholeBuild('g1');
  await page.waitForFunction(() => document.querySelector('meta[name="build"]')?.getAttribute('content') === 'g1', null, {
    timeout: 15_000,
  });
  await page.locator('.sidebar').waitFor();
  const asked = [];
  page.on('request', (request) => asked.push(new URL(request.url()).pathname));
  // What the daemon said of each build, as the page reads it: the page is
  // told of each, and stays where it is.
  const told = (id) =>
    page.waitForFunction(
      (id) =>
        fetch('/api/events/recent?topics=client.updated', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })
          .then((r) => r.json())
          .then((body) => body.events.some((event) => event.data.build === id)),
      id,
      { timeout: 15_000, polling: 200 },
    );
  for (const id of ['g2', 'g3', 'g4']) {
    await wholeBuild(id);
    await told(id);
    await notice.waitFor({ timeout: 15_000 });
    expect(`the page to stay on g1 while ${id} is put in place`, (await build()) === 'g1');
  }
  await side(page, 'Resources');
  await unfold(page, page.locator('.nav button.nav-item', { has: page.getByText('std', { exact: true }) }));
  await page.locator('.nav').getByText('std/fleet', { exact: true }).click();
  await page.getByRole('tab', { name: 'Text' }).last().click();
  await page.locator('.cm-content').waitFor({ timeout: 15_000 });
  const fetched = asked.filter((path) => path.startsWith('/assets/'));
  expect(`the editor to have come from g1's own files (${fetched.join(', ')})`, fetched.length > 0 && fetched.every((path) => path.includes('-g1.')));
  expect('no part of the client to be said to be missing', (await page.locator('.toast.notice', { hasText: 'could not be loaded' }).count()) === 0);
  await shot('6-three-builds-behind-with-its-editor');
  const there = (path) => access(join(client, path)).then(() => true, () => false);
  expect('the real build, four builds back, to be kept still', await there(script));
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [client, data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
