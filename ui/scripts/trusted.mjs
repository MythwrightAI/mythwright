// A project made of a folder that was already there, trusted from the gear
// of its page and taken back there.
//
//   DAEMON=../target/debug/factory node scripts/trusted.mjs [out-dir]
//
// Shop is made of a folder of the check's own (`project create --path`),
// so it is not trusted. Ada, who owns the factory, sees that at the head
// of its page and in its row of the side panel, and its gear has a line
// that trusts it; Nell, who registered by an invitation and does not
// manage the factory, has no such line. The line opens a question that
// says what trusting does; Cancel, and Escape, change nothing. Trust it
// trusts it: the mark goes on that page, on a second page of Ada's and
// on Nell's, none of them read again, and the command line says so too.
// The gear then offers taking it back, whose question says the
// blueprints that ship are used again, and Stop trusting brings the mark
// back everywhere. What the daemon refuses is said in its own sentence
// with the question staying. Nothing of the menu or the question lies
// over anything else of it at three widths and a phone's. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { linesOf, menuOf, openGear } from './gear.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'trusted-data');
const home = join(out, 'trusted-home');
const hers = join(out, 'trusted-home-nell');
const shop = join(out, 'trusted-shop');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers, shop]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers, shop]) await mkdir(dir, { recursive: true });
await writeFile(join(shop, 'README.md'), '# Shop\n\nA folder that was here before the factory.\n');
// The daemon and the command line are given homes of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env: as(home),
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  // What the command line says of Shop: its line of the list ends in `not trusted` while it is not.
  const trusted = async () => !(await cli('project', 'list')).stdout.split('\n').some((one) => /\bshop\b/.test(one) && one.includes('not trusted'));

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
  // Somebody's page, signed in and showing Shop's page: each in a browser of their own, so that each has their own session.
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(`${url}/#/project/shop`);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    // The address waited for the sign-in, and is opened once everything is read.
    await page.locator('.view-head h1', { hasText: 'Shop' }).waitFor();
    // The side panel turned to Resources, where Shop has its row; at a phone's width it starts put away.
    if (!how.isMobile) {
      await page.locator('.rail').getByRole('button', { name: 'Resources', exact: true }).click();
      // Everything the Resources panel lists starts folded (`folded.mjs`).
      await unfoldAll(page);
    }
    return page;
  };
  // The mark at the head of Shop's page, and the one in its row of the side panel.
  const headMark = (page) => page.locator('.view-head .not-trusted');
  const rowMark = (page) => page.locator('[data-project="shop"] > .nav-row .not-trusted');
  const TRUST = 'Trust this project…';
  const STOP = 'Stop trusting…';
  // The boxes of what `selector` picks inside `within`: none lies over another, each is inside `within`, and that inside the window.
  const apart = async (within, selector, what) => {
    const found = await within.evaluate((box, picked) => {
      const frame = box.getBoundingClientRect();
      const boxes = [...box.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      if (frame.left < -0.5 || frame.right > window.innerWidth + 0.5 || frame.top < -0.5 || frame.bottom > window.innerHeight + 0.5) wrong.push(`it leaves the window: ${JSON.stringify(frame)}`);
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < frame.left - 0.5 || one.box.right > frame.right + 0.5 || one.box.top < frame.top - 0.5 || one.box.bottom > frame.bottom + 0.5) wrong.push(`${one.said} leaves it`);
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
  const MENU = 'button, .menu-said';
  const ASKED = 'h2, p, .modal-foot button';
  const WIDTHS = [1280, 1000, 800];
  // A dialog listens for Escape, and what floats for a press outside it, a moment after it is drawn.
  const settled = (page) => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const putAway = async (page) => {
    await page.keyboard.press('Escape');
    await menuOf(page).waitFor({ state: 'detached' });
  };

  expect('Shop, made of a folder that was already there, not to be trusted', (await trusted()) === false);
  const page = await signedIn('ada@example.com');
  const second = await signedIn('ada@example.com');
  const theirs = await signedIn('nell@example.com');

  // ── Not trusted, and a line to trust it for whoever manages the factory ──
  for (const [one, whose] of [[page, "Ada's page"], [second, "Ada's second page"], [theirs, "Nell's page"]]) {
    await headMark(one).waitFor();
    await rowMark(one).waitFor();
    expect(`the side panel row to say it once, on ${whose}`, (await rowMark(one).count()) === 1);
  }
  await openGear(page);
  let lines = await linesOf(page);
  expect(`the gear to offer trusting Shop, once, after Copy link; got ${lines.join(' | ')}`, lines[0] === 'Copy link' && lines.filter((one) => one === TRUST).length === 1 && !lines.includes(STOP));
  const line = menuOf(page).getByRole('menuitem', { name: TRUST });
  expect('the line to have a drawing before it', (await line.locator('.menu-sign svg').count()) === 1);
  // The menu is where it was opened: it is opened again at each width.
  for (const width of WIDTHS) {
    await putAway(page);
    await page.setViewportSize({ width, height: 860 });
    await settled(page);
    await openGear(page);
    await apart(menuOf(page), MENU, `the gear's menu at ${width}`);
  }
  await putAway(page);
  await page.setViewportSize({ width: 1280, height: 860 });
  await settled(page);
  await openGear(page);
  await page.screenshot({ path: `${out}/trusted-0-gear.png` });

  // Somebody who does not manage the factory is offered no line, and the route refuses them.
  await openGear(theirs);
  const hersLines = await linesOf(theirs);
  expect(`Nell's gear to offer nothing of trusting; got ${hersLines.join(' | ')}`, !hersLines.includes(TRUST) && !hersLines.includes(STOP));
  await putAway(theirs);
  const refused = await theirs.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    return (await fetch('/api/projects/shop/trust', { method: 'POST', headers: mine, body: '{"trusted":true}' })).status;
  });
  expect(`the route to refuse a member, got ${refused}`, refused === 403);

  // ── The question ────────────────────────────────────────────────────────
  await line.click();
  await menuOf(page).waitFor({ state: 'detached' });
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('heading', { name: 'Trust Shop?' }).waitFor();
  const asked = (await dialog.innerText()).replace(/\s+/g, ' ');
  expect(`the question to say what trusting does, got "${asked}"`, asked.includes('Its own files named like a blueprint that ships will be used in place of the one that ships.') && asked.includes('Anyone who can write to its folder can then change what those blueprints do.'));
  expect('the question to be answered by Trust it or Cancel', (await dialog.getByRole('button', { name: 'Trust it', exact: true }).count()) === 1 && (await dialog.getByRole('button', { name: 'Cancel', exact: true }).count()) === 1);
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await apart(dialog, ASKED, `the question at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/trusted-1-asked.png` });
  // Cancel changes nothing, and nor does Escape.
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await openGear(page);
  await menuOf(page).getByRole('menuitem', { name: TRUST }).click();
  await dialog.getByRole('heading', { name: 'Trust Shop?' }).waitFor();
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });
  expect('Shop to be untrusted still after Cancel and Escape', (await trusted()) === false && (await headMark(page).count()) === 1);

  // What the daemon refuses is said in its own sentence, and the question stays.
  await page.route('**/api/projects/shop/trust', (route) => route.fulfill({ status: 403, contentType: 'application/json', body: JSON.stringify({ error: 'The daemon will not have it just now.' }) }), { times: 1 });
  await openGear(page);
  await menuOf(page).getByRole('menuitem', { name: TRUST }).click();
  await dialog.getByRole('button', { name: 'Trust it', exact: true }).click();
  await dialog.locator('.error-text').waitFor();
  const why = (await dialog.locator('.error-text').innerText()).trim();
  expect(`the daemon's sentence to be said in the question, got "${why}"`, why.includes('The daemon will not have it just now.'));
  expect('the question to stay', (await dialog.count()) === 1);

  // ── Trusted ─────────────────────────────────────────────────────────────
  await dialog.getByRole('button', { name: 'Trust it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await page.getByText('Shop is trusted.').waitFor();
  for (const [one, whose] of [[page, "Ada's page"], [second, "Ada's second page"], [theirs, "Nell's page"]]) {
    await headMark(one).waitFor({ state: 'detached' });
    await rowMark(one).waitFor({ state: 'detached' });
    expect(`the project's page to be open still on ${whose}`, (await one.locator('.view-head h1', { hasText: 'Shop' }).count()) === 1);
  }
  expect('the daemon to say Shop is trusted', (await trusted()) === true);
  await page.screenshot({ path: `${out}/trusted-2-after.png` });
  await second.screenshot({ path: `${out}/trusted-3-second.png` });

  // ── And taken back ──────────────────────────────────────────────────────
  await openGear(page);
  lines = await linesOf(page);
  expect(`the gear to offer taking it back and not trusting it again; got ${lines.join(' | ')}`, lines.filter((one) => one === STOP).length === 1 && !lines.includes(TRUST));
  await menuOf(page).getByRole('menuitem', { name: STOP }).click();
  await dialog.getByRole('heading', { name: 'Stop trusting Shop?' }).waitFor();
  const back = (await dialog.innerText()).replace(/\s+/g, ' ');
  expect(`the question to say the blueprints that ship are used again, got "${back}"`, back.includes('The blueprints that ship will be used again in place of its own files of the same name.'));
  await page.screenshot({ path: `${out}/trusted-4-asked-back.png` });
  await dialog.getByRole('button', { name: 'Stop trusting', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  for (const one of [page, second, theirs]) {
    await headMark(one).waitFor();
    await rowMark(one).waitFor();
  }
  expect('the daemon to say Shop is not trusted', (await trusted()) === false);
  await page.screenshot({ path: `${out}/trusted-5-back.png` });

  // ── A phone's width ─────────────────────────────────────────────────────
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await openGear(phone, 'tap');
  await apart(menuOf(phone), MENU, "the gear's menu on a phone");
  await phone.screenshot({ path: `${out}/trusted-6-phone-gear.png` });
  await menuOf(phone).getByRole('menuitem', { name: TRUST }).tap();
  const small = phone.getByRole('dialog');
  await small.getByRole('heading', { name: 'Trust Shop?' }).waitFor();
  await settled(phone);
  await apart(small, ASKED, 'the question on a phone');
  await phone.screenshot({ path: `${out}/trusted-7-phone-asked.png` });
  await small.getByRole('button', { name: 'Cancel', exact: true }).tap();
  await small.waitFor({ state: 'detached' });
  expect('Shop to be untrusted still', (await trusted()) === false);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers, shop]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
