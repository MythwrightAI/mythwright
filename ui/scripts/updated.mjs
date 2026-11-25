// A package that is installed and is not the one that came with this
// version: said where packages are listed and on its own page, and
// replaced by the one that came only when somebody who may asks.
//
//   DAEMON=../target/debug/factory node scripts/updated.mjs [out-dir]
//
// The factory is told a folder of its own for what came with it, holding
// two made-up packages whose program is nowhere, so that nothing is run
// and no package that ships is read. Both are installed, and one of them
// is then written again where it came from, as a newer program brings a
// package that says more: the installed copy is an earlier one. Ada, who
// owns the factory, is shown that it differs in its line of the side
// panel and on its page, with a button; Nell, who registered by an
// invitation, is shown the words and no button, and the route refuses
// her. The question says what is replaced and what becomes of the copy,
// and is left by Escape with nothing done. Asked for, the copy on the
// disk is the one that came with the earlier one kept beside it, and the
// mark goes on the page that asked, on a second page of Ada's and on
// Nell's, none of them read again. Nothing of the line, the page's note
// or the question lies over anything else of it at three widths and a
// phone's. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { fillsPage } from './filled.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'updated-data');
const home = join(out, 'updated-home');
const hers = join(out, 'updated-home-nell');
const came = join(out, 'updated-came');

// A package for a made-up agent, which says `wall` of a program that is nowhere.
const manifest = (name, title, wall) =>
  [
    `extension: ${name}`,
    `title: ${title}`,
    'description: Not a real agent; its program is nowhere.',
    'agent:',
    '  program: no-such-program-here',
    '  input: ${ turn.prompt }',
    '  output:',
    '    - reply: ${ event.text }',
    '  wall:',
    ...wall.map((line) => `    ${line}`),
    '',
  ].join('\n');
const EARLIER = manifest('sample', 'Sample Agent', ['keeps: [".sample"]']);
const CAME = manifest('sample', 'Sample Agent', ['keeps: [".sample"]', 'signin: [".sample/auth"]']);
const comes = async (name, source) => {
  await mkdir(join(came, name), { recursive: true });
  await writeFile(join(came, name, 'extension.yaml'), source);
};

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers, came]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
await comes('sample', EARLIER);
await comes('steady', manifest('steady', 'Steady Agent', ['keeps: [".steady"]']));
// The daemon and the command line are given homes of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--packages', came], {
  cwd: repo,
  env: as(home),
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });
const installed = join(data, 'extensions', 'sample');

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  for (const name of ['sample', 'steady']) await cli('extension', 'add', name);
  // A newer program comes with a package that says more than the copy that is installed.
  await comes('sample', CAME);

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
  // Somebody's page, signed in: each in a browser of their own, so that each has their own session.
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    // Signed in: the rail is on the page, though put away at a phone's width.
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  // The side panel turned to Extensions, with what is installed read.
  const listing = async (on) => {
    const panel = on.getByRole('navigation', { name: 'Extensions' });
    if ((await panel.count()) === 0) await on.locator('.rail').getByRole('button', { name: 'Extensions', exact: true }).click();
    await panel.locator('.ext-row[data-extension="steady"]').waitFor();
    return panel;
  };
  const line = (panel, name) => panel.locator(`.ext-row[data-extension="${name}"]`);
  const DIFFERS = 'Differs from the one that came';
  // The boxes of what `selector` picks inside `within`: none lies over another, and each is inside `within`.
  const apart = async (within, selector, what) => {
    const found = await within.evaluate((box, picked) => {
      const frame = box.getBoundingClientRect();
      const boxes = [...box.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      if (frame.left < -0.5 || frame.right > window.innerWidth + 0.5) wrong.push('it leaves the window');
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < frame.left - 0.5 || one.box.right > frame.right + 0.5) wrong.push(`${one.said} leaves it`);
        for (const two of boxes.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth };
    }, selector);
    expect(`${what}: something to be there`, found.count > 1);
    expect(`${what}: nothing over anything else or outside its place, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px`, found.sideways <= 0);
  };
  const WIDTHS = [1280, 1000, 800];
  const LINE = '.ext-open > *, .ext-update, .ext-dot';
  const NOTE = '.ext-came-says > p, .ext-came > button';
  const ASKED = 'h2, p, .modal-foot button';

  const page = await signedIn('ada@example.com');
  const second = await signedIn('ada@example.com');
  const theirs = await signedIn('nell@example.com');
  const panel = await listing(page);
  const beside = await listing(second);
  const other = await listing(theirs);

  // ── Said where packages are listed ──────────────────────────────────────
  await line(panel, 'sample').locator('.ext-behind').waitFor();
  expect('the line of a package that differs to say so', (await line(panel, 'sample').locator('.ext-behind').innerText()).trim() === DIFFERS);
  expect('one button on that line, for whoever may install a package', (await line(panel, 'sample').getByRole('button', { name: 'Update', exact: true }).count()) === 1);
  expect('nothing said of one that is the one that came, and no button', (await line(panel, 'steady').locator('.ext-behind').count()) === 0 && (await line(panel, 'steady').getByRole('button', { name: 'Update' }).count()) === 0);
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(line(panel, 'sample'), LINE, `the line of a package that differs at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/updated-0-listed.png` });
  const listed = (await cli('extension', 'list')).stdout;
  expect(`the command line to say the same, got ${listed}`, listed.includes('not the one that came with this') && listed.includes('sample: the one that came says `agent.wall.signin` besides'));

  // ── And on its own page ─────────────────────────────────────────────────
  await line(panel, 'sample').locator('.ext-open').click();
  const note = page.locator('.ext-came');
  await note.waitFor();
  const said = (await note.locator('.ext-came-says').innerText()).replace(/\s+/g, ' ');
  expect(`the page to say that the copy is not the one that came, and where they differ, got "${said}"`, said.includes('The installed copy is not the one that came with this version of') && said.includes('the one that came says agent.wall.signin besides') && said.includes('nothing says whether this copy is an earlier one or one somebody changed'));
  expect('nothing there to call either of them the newer', !/\b(newer|older|out of date)\b/i.test(said));
  expect('the page to offer the same button', (await note.getByRole('button', { name: 'Update', exact: true }).count()) === 1);
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(note, NOTE, `what a package's page says of one that differs at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/updated-1-page.png` });
  // A package's page is a page of tables, and fills its tab (`filled.mjs`).
  await fillsPage(page, "a package's page", page.locator('.ext-body'), { ready: note, shot: `${out}/updated-filled` });
  // The page of one that is the one that came says nothing of it.
  await line(panel, 'steady').locator('.ext-open').click();
  await page.locator('.view-head h1', { hasText: 'Steady Agent' }).waitFor();
  expect('nothing said on the page of one that is the one that came', (await page.locator('.ext-came').count()) === 0);
  await line(panel, 'sample').locator('.ext-open').click();
  await note.waitFor();

  // ── Somebody who may not is shown the words and no button ───────────────
  await line(other, 'sample').locator('.ext-behind').waitFor();
  expect('a member to be offered no button on the line', (await line(other, 'sample').getByRole('button', { name: 'Update' }).count()) === 0);
  await line(other, 'sample').locator('.ext-open').click();
  await theirs.locator('.ext-came').waitFor();
  expect('nor on the page, which says the same words', (await theirs.locator('.ext-came').getByRole('button').count()) === 0 && (await theirs.locator('.ext-came').innerText()).includes('The installed copy is not the one that came'));
  const refused = await theirs.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    return (await fetch('/api/extensions/sample/update', { method: 'POST', headers: mine, body: '{}' })).status;
  });
  expect(`the route to refuse a member, got ${refused}`, refused === 403);
  await theirs.screenshot({ path: `${out}/updated-2-member.png` });

  // ── A phone's width ─────────────────────────────────────────────────────
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  // The side panel starts put away there, and lies over the page once it is brought.
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await phone.locator('.rail').getByRole('button', { name: 'Extensions', exact: true }).tap();
  const small = phone.getByRole('navigation', { name: 'Extensions' });
  await line(small, 'sample').locator('.ext-behind').waitFor();
  await apart(line(small, 'sample'), LINE, 'the line of a package that differs on a phone');
  await phone.screenshot({ path: `${out}/updated-3-phone-listed.png` });
  await line(small, 'sample').getByRole('button', { name: 'Update', exact: true }).tap();
  const tapped = phone.getByRole('dialog');
  await tapped.getByRole('heading', { name: 'Update Sample Agent?' }).waitFor();
  await apart(tapped, ASKED, 'the question on a phone');
  await phone.screenshot({ path: `${out}/updated-4-phone-asked.png` });
  await tapped.getByRole('button', { name: 'Cancel', exact: true }).tap();
  await tapped.waitFor({ state: 'detached' });
  await line(small, 'sample').locator('.ext-open').tap();
  await phone.locator('.ext-came').waitFor();
  await apart(phone.locator('.ext-came'), NOTE, "what a package's page says of one that differs on a phone");
  await phone.screenshot({ path: `${out}/updated-5-phone-page.png` });

  // ── The question ────────────────────────────────────────────────────────
  await note.getByRole('button', { name: 'Update', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('heading', { name: 'Update Sample Agent?' }).waitFor();
  const asked = (await dialog.innerText()).replace(/\s+/g, ' ');
  expect(`the question to say what is replaced, got "${asked}"`, asked.includes('The installed copy of this package is replaced by the one that came with this version of'));
  expect(`and where the two differ, got "${asked}"`, asked.includes('Against the installed copy, the one that came says agent.wall.signin besides.'));
  expect(`and what becomes of the copy, got "${asked}"`, asked.includes('Changes made to the installed copy are lost from what is installed') && asked.includes('extension.was.yaml'));
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(dialog, ASKED, `the question at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/updated-6-asked.png` });
  // A dialog listens for Escape a moment after it is drawn; left that way, nothing is replaced.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });
  expect('the copy to be as it was after the question was left', (await readFile(join(installed, 'extension.yaml'), 'utf8')) === EARLIER);
  expect('the line to say it differs still', (await line(panel, 'sample').locator('.ext-behind').count()) === 1);

  // ── Asked for ───────────────────────────────────────────────────────────
  await line(panel, 'sample').getByRole('button', { name: 'Update', exact: true }).click();
  await dialog.getByRole('button', { name: 'Update', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await page.getByText('Sample Agent is now the one that came with this version.').waitFor();
  // The mark goes where it was asked, on a second page and on somebody else's, none of them read again.
  for (const [nav, whose] of [[panel, "Ada's page"], [beside, "Ada's second page"], [other, "Nell's page"], [small, 'the phone']]) {
    await line(nav, 'sample').locator('.ext-behind').waitFor({ state: 'detached' });
    expect(`no button left on the line, on ${whose}`, (await line(nav, 'sample').getByRole('button', { name: 'Update' }).count()) === 0);
  }
  await note.waitFor({ state: 'detached' });
  await theirs.locator('.ext-came').waitFor({ state: 'detached' });
  expect('the package to be listed still, with its page open', (await page.locator('.view-head h1', { hasText: 'Sample Agent' }).count()) === 1);
  await page.screenshot({ path: `${out}/updated-7-taken.png` });
  await second.screenshot({ path: `${out}/updated-8-second.png` });
  // What is installed is the one that came, and the copy it replaced is kept beside it.
  expect('the installed package to be the one that came', (await readFile(join(installed, 'extension.yaml'), 'utf8')) === CAME);
  expect('the copy it replaced to be kept beside it', (await readFile(join(installed, 'extension.was.yaml'), 'utf8')) === EARLIER);
  const after = (await cli('extension', 'list')).stdout;
  expect(`the command line to say nothing differs, got ${after}`, !after.includes('not the one'));
  const again = (await cli('extension', 'update', 'sample')).stdout.trim();
  expect(`asked again from a terminal, to be told there is nothing to take, got ${again}`, again === '`sample` is already the one that came with this installation');

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers, came]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
