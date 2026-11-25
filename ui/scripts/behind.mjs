// An instance that is behind the text now there for its blueprint: said
// where it is listed and where it is opened, with why, and with what
// brings it up to date.
//
//   DAEMON=../target/debug/factory node scripts/behind.mjs [out-dir]
//
// A watcher starts a worker when something is said in a channel; none of
// its own steps names the worker. The worker's file is then saved twice,
// as a person would save it. First with another handler: the worker is
// behind, and takes the file where it is when it is applied again from
// its own page, which says that nothing of it runs again. Then with other
// steps: it is behind and stays so when the watcher is applied again,
// until it is applied again itself, from its own page, which says that
// its steps run again. Last with other steps once more and a word on how one that is
// running takes it (`on_update`): its page says it would be carried
// over, and applying it again does that, with none of its steps run. Then
// what ships is another (a package's blueprint, released again): a
// blueprint of the project's own that calls it is told what moved by
// version when it is applied from its page, and takes it from there, with
// what it keeps running under it, which begins otherwise, brought up to
// date by that one press. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
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
const data = join(out, 'behind-data');
const home = join(out, 'behind-home');

const WATCHER = `blueprint: watcher
description: Starts a worker when something is said.
on:
  said:
    event: channel.message
    steps:
      go: { spawn: worker, as: the-worker, with: { room: general } }
`;

const worker = (version, claims, says) => `blueprint: worker
version: ${version}
description: Claims a room and answers when it is poked.
params:
  room: { type: string }
steps:
  claim: { do: emit, with: { topic: ${claims} } }
on:
  poke:
    event: shop.poke
    steps:
      say: { do: emit, with: { topic: ${says} } }
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
  await cli('channel', 'create', 'general');
  // The project's own blueprints, where the factory keeps them (`@blueprints/` of the project's).
  const blueprints = join(data, 'blueprints', 'projects', 'shop');
  await mkdir(blueprints, { recursive: true });
  const watcher = join(blueprints, 'watcher.bp');
  // A file saved by hand is said to have changed when the project's files are next looked at.
  const save = async (text) => {
    await writeFile(join(blueprints, 'worker.bp'), text);
    await cli('project', 'scan', 'shop');
  };
  await writeFile(watcher, WATCHER);
  await save(worker(1, 'shop.claimed', 'shop.old'));
  await cli('apply', watcher, '--project', 'shop');
  await cli('post', 'general', 'go');

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

  // Everything that is alive, listed: the watcher and the worker it started.
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Activity', { exact: true }).click();
  await page.getByRole('heading', { name: 'Activity', level: 1 }).waitFor();
  const row = (name) => page.locator('tbody tr', { has: page.locator('strong', { hasText: new RegExp(`^${name}$`) }) }).first();
  await row('worker').locator('.badge.active').waitFor();
  expect('nothing to be behind while the files are as they were applied', (await page.locator('.lag-mark').count()) === 0);

  // The worker's file is saved with another handler. Nothing is applied: the worker is behind, and says so.
  await save(worker(2, 'shop.claimed', 'shop.new'));
  await row('worker').locator('.lag-mark').waitFor();
  expect('the watcher, whose file was not touched, not to be behind', (await row('watcher').locator('.lag-mark').count()) === 0);
  await page.screenshot({ path: `${out}/behind-0-listed.png` });

  // Opened, it says it at its head, and in its details which text it runs, which it is behind, and why.
  await row('worker').click();
  const mark = page.locator('.view-head .lag-mark');
  await mark.waitFor();
  expect(`the mark to say behind, got "${await mark.innerText()}"`, (await mark.innerText()).trim() === 'behind');
  await mark.click();
  const drawer = drawerAtRest(page);
  await drawer.getByText(/^Behind [0-9a-f]{8} · version 2$/).waitFor();
  const why = await drawer.locator('.lag-why').innerText();
  expect(`it to say that it takes the text where it is, got "${why}"`, why.includes('differs from the one this runs only in what it does later') && why.includes('when what started it, or it itself, is applied again'));
  // What moved between the two is said by version first, and what leaving it comes to.
  const moved = (await drawer.locator('.lag-moved').innerText()).trim();
  expect(`what moved to be said by version first, got "${moved}"`, moved === 'worker 1 to 2: the handler `poke` is changed');
  expect('it to say what leaving it comes to', (await drawer.locator('.lag-leave').innerText()).includes('Or leave it: it goes on with the text it has'));
  expect(`the text it runs to be said as version 1, got "${await drawer.locator('code').first().innerText()}"`, /^[0-9a-f]{8} · version 1$/.test((await drawer.locator('code').first().innerText()).trim()));
  // It can be brought up to date from here, and the button says that nothing of it runs again.
  const take = drawer.getByRole('button', { name: 'Apply it again (it takes the text where it is: nothing runs again)', exact: true });
  await take.waitFor();
  await page.screenshot({ path: `${out}/behind-1-opened.png` });
  await take.click();
  await drawer.waitFor({ state: 'detached' });
  await mark.waitFor({ state: 'detached' });
  await page.getByText('it took the text where it is, and nothing ran again').waitFor();
  const first = await cli('events', '--topics', 'shop.claimed', '--limit', '5');
  expect(`its steps not to have run again, got:\n${first.stdout}`, first.stdout.split('\n').filter((line) => line.includes('shop.claimed')).length === 1);

  // The file is saved with other steps. Applying the watcher again leaves the worker, which says what would bring it up to date.
  await save(worker(3, 'shop.claimed_again', 'shop.new'));
  await mark.waitFor();
  await cli('apply', watcher, '--project', 'shop');
  await mark.click();
  await drawer.getByText(/^Behind [0-9a-f]{8} · version 3$/).waitFor();
  expect('it to say that its steps would run again', (await drawer.locator('.lag-why').innerText()).includes('It runs that text only when it is itself applied again, which runs its steps again.'));
  const again = drawer.getByRole('button', { name: 'Apply it again (its steps run again)', exact: true });
  await again.waitFor();
  await page.screenshot({ path: `${out}/behind-2-again.png` });
  await again.click();
  // The details said what was so of the text it had, and are put away.
  await drawer.waitFor({ state: 'detached' });
  await mark.waitFor({ state: 'detached' });
  const claimed = await cli('events', '--topics', 'shop.claimed_again', '--limit', '5');
  expect(`its steps to have run again in the text now there, got "${claimed.stdout}"`, claimed.stdout.includes('shop.claimed_again'));

  // The file is saved with other steps once more, and this time says how one that is running comes to it.
  await save(`${worker(4, 'shop.claimed_anew', 'shop.new')}on_update:\n  - emit: { topic: shop.carried, from: "\${ was.version }" }\n`);
  await mark.waitFor();
  await mark.click();
  await drawer.getByText(/^Behind [0-9a-f]{8} · version 4$/).waitFor();
  const how = await drawer.locator('.lag-why').innerText();
  expect(`it to say that it would be carried over, got "${how}"`, how.includes('says how one that is running takes it (`on_update`)') && how.includes('which are not run again'));
  const carry = drawer.getByRole('button', { name: 'Apply it again (it is carried over: its steps do not run again)', exact: true });
  await carry.waitFor();
  await page.screenshot({ path: `${out}/behind-3-carried.png` });
  await carry.click();
  await drawer.waitFor({ state: 'detached' });
  await mark.waitFor({ state: 'detached' });
  const said = await cli('events', '--topics', 'shop.carried,shop.claimed_anew', '--limit', '5');
  expect(`what the file says under on_update to have been done, and none of its steps, got "${said.stdout}"`, said.stdout.includes('shop.carried') && !said.stdout.includes('shop.claimed_anew'));

  // What ships is another: a package's blueprint, released again under the same name. A blueprint of the project's own that calls it goes on with the one it was applied with; applied again from its page, it is told what moved, by version, and offered to take it or leave it.
  const reach = join(home, 'reach');
  await mkdir(reach, { recursive: true });
  // The round says what a file it links says, and the package brings the file with it.
  const release = async (version, more, words) => {
    await writeFile(
      join(reach, 'extension.yaml'),
      `extension: reach\ntitle: Reach\nblueprints:\n  round: |\n    blueprint: reach/round\n    version: ${version}\n    params:\n      id: { type: int, default: 0 }\n${more}    steps:\n      words: { do: text.put, with: { parts: [{ from: round.md, with: { id: "\${params.id}" } }] } }\n      made: { do: emit, with: { topic: shop.rounded, text: "\${steps.words.out.text}" } }\n  keeper: |\n    blueprint: reach/keeper\n    version: ${version}\n    steps:\n      began: { do: emit, with: { topic: shop.kept${version} } }\n    on:\n      poke: { event: shop.poke, do: emit, with: { topic: shop.poked } }\nprompts:\n  round.md: |\n    ${words}\n`,
    );
    await cli('extension', 'add', reach);
  };
  // What the round last said.
  const rounded = async () => {
    const before = (await cli('events', '--topics', 'shop.rounded', '--limit', '200', '--json')).stdout.split('\n').filter(Boolean).length;
    await cli('emit', 'shop.wanted');
    for (let tries = 0; tries < 100; tries += 1) {
      const said = (await cli('events', '--topics', 'shop.rounded', '--limit', '200', '--json')).stdout.split('\n').filter(Boolean);
      if (said.length > before) return JSON.parse(said.at(-1)).data.text;
      await new Promise((later) => setTimeout(later, 100));
    }
    throw new Error('the round was never made');
  };
  await release(1, '', 'Make ${id}.');
  // It keeps the package's keeper running too, which begins otherwise once the package is released again.
  await writeFile(join(blueprints, 'caller.bp'), 'blueprint: caller\non:\n  wanted:\n    event: shop.wanted\n    steps:\n      go: { call: reach/round, as: round }\n      keep: { spawn: reach/keeper, as: keeper }\n');
  await cli('project', 'scan', 'shop');
  await cli('apply', join(blueprints, 'caller.bp'), '--project', 'shop');
  expect('the round to say the words it was applied with', (await rounded()) === 'Make 0.');
  await release(2, '      tone: { type: string, default: plain }\n', 'Make ${id} with care.');
  // Released again with other words: what is running goes on saying the ones it reached.
  expect('the round to go on with the words it reached', (await rounded()) === 'Make 0.');
  // And the keeper goes on with the text it began with, and is behind the one that ships now.
  const activity = async () => {
    await side(page, 'Resources');
    await page.locator('.nav').getByText('Activity', { exact: true }).click();
    await page.getByRole('heading', { name: 'Activity', level: 1 }).waitFor();
  };
  await activity();
  await row('reach/keeper').locator('.lag-mark').waitFor();
  const nav = page.locator('.nav');
  const list = nav
    .locator(':scope > .tree > [data-project]', { has: page.locator(':scope > .nav-row', { hasText: 'Shop' }) })
    .locator(':scope > .tree > div', { has: page.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item', { hasText: 'Blueprints' }) });
  const head = list.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item');
  if ((await head.getAttribute('aria-expanded')) !== 'true') await head.click();
  await list.getByText('caller', { exact: true }).click();
  await page.getByRole('button', { name: /^Apply/ }).click();
  const left = page.locator('.reached-left');
  await left.waitFor();
  // Nothing of its own text changed: what is running is said as the instance it is, by its blueprint and its key.
  await page.getByText('caller · caller is already running this; nothing to do.', { exact: true }).waitFor();
  expect('no toast to name nothing', !(await page.locator('body').innerText()).includes('null is already running'));
  const ships = (await left.locator('.lag-moved').allInnerTexts()).map((line) => line.trim());
  expect(`the words it links to be said to have changed, and the blueprint by version first, got ${JSON.stringify(ships)}`, ships.join(' | ') === 'reach/keeper 1 to 2: the step `began` is changed | reach/prompts/round.md: its words changed | reach/round 1 to 2: the param `tone` is new');
  expect('it to say that what is running goes on as it was', (await left.innerText()).includes('What is running goes on with it as it was.'));
  await left.getByRole('button', { name: 'Leave it', exact: true }).waitFor();
  await page.screenshot({ path: `${out}/behind-4-what-ships.png` });
  // Taken from there: it moves, and says what to.
  await left.getByRole('button', { name: 'Take what ships now', exact: true }).click();
  await page.getByText('Moved to what ships now: reach/keeper 1 to 2: the step `began` is changed. reach/prompts/round.md: its words changed. reach/round 1 to 2: the param `tone` is new.').waitFor();
  await left.waitFor({ state: 'detached' });
  // One press at the top: what runs under it is brought up to date with it, the keeper's steps run again on what ships now.
  await activity();
  await row('reach/keeper').locator('.badge.active').waitFor();
  await row('reach/keeper').locator('.lag-mark').waitFor({ state: 'detached' });
  expect('nothing under the caller to be behind once what ships was taken at it', (await page.locator('.lag-mark').count()) === 0);
  const kept = await cli('events', '--topics', 'shop.kept2', '--limit', '5');
  expect(`the keeper to have begun again on what ships now, got "${kept.stdout}"`, kept.stdout.includes('shop.kept2'));
  await page.screenshot({ path: `${out}/behind-5-taken-whole.png` });
  expect('the round to say the words that ship now once they were taken', (await rounded()) === 'Make 0 with care.');
  // Applied once more there is nothing to say: it is on what ships.
  const once = await cli('apply', join(blueprints, 'caller.bp'), '--project', 'shop');
  expect(`nothing to be said of what ships once it was taken, got "${once.stdout}"`, !once.stdout.includes('goes on with it as it was') && (await left.count()) === 0);

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
