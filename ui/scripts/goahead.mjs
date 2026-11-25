// A run that waits for a person to let it go on, decided on the page of
// the line that tells her (`views/gopart.tsx`, over `ui/src/goasked.ts`;
// `approval.ask`, `told::goes`).
//
//   DAEMON=../target/debug/factory node scripts/goahead.mjs [out-dir]
//
// Ada owns the factory; Grace is a member, who holds `blueprints:run`.
// Ada applies, from a terminal, a text that builds, waits for a go-ahead
// and then ships. Grace is told by a line of her inbox, on a page opened
// once and never read again; the line's page shows whose run of what,
// what it says, the address to look at as a link, how long it waits, and
// Approve and Decline, and what it is about the run that waits, whose
// page it opens as a tab. Ada is not told, and shown it from a terminal she
// may not let her own run go on, and the daemon refuses her. Grace
// approves it there: the page says so, and the run ships. A second run,
// declined on the page with why: the run fails saying it. A third run
// opened by its address, its page having Approve and Decline where the
// stage waits (`RunApproval`). And the part by
// its boxes at three widths and a phone's, nothing of it over anything
// else or past the page, the buttons a finger's size on the phone. With
// `LOOK="size=largest density=compact"` the same at that look. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
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
const data = join(out, 'goahead-data');
const home = join(out, 'goahead-home');
const hers = join(out, 'goahead-home-grace');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const grace = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: { ...env, HOME: hers, XDG_CONFIG_HOME: join(hers, 'config') } });

const ship = (name) => `blueprint: ${name}
steps:
  built:
    do: emit
    with: { topic: shop.built }
  approve:
    needs: [built]
    do: approval.ask
    with: { says: The build of the shop is ready to go live, links: ["https://example.com/preview"] }
  shipped:
    needs: [approve]
    do: emit
    with: { topic: shop.shipped }
`;

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'grace@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await grace('register', '--email', 'grace@example.com', '--name', 'Grace', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  // Ada applies a text that waits for somebody to let it go on: what it asked, once it is told.
  const applied = async (name) => {
    const file = join(home, `${name}.bp`);
    await writeFile(file, ship(name));
    await cli('apply', file, '--project', 'shop', '--no-wait');
    for (let tries = 0; ; tries += 1) {
      const asked = JSON.parse((await grace('asked', '--json')).stdout).asked ?? [];
      const found = asked.find((one) => one.kind === 'go' && one.blueprint === name);
      if (found) return found;
      if (tries > 100) throw new Error(`Grace was never shown the run of ${name} waiting: ${JSON.stringify(asked)}`);
      await new Promise((later) => setTimeout(later, 100));
    }
  };
  // How a run ended, once it has.
  const ended = async (id) => {
    for (let tries = 0; ; tries += 1) {
      const found = JSON.parse((await cli('runs', '--json')).stdout).runs.find((one) => one.id === id);
      if (found && found.status !== 'running') return found;
      if (tries > 100) throw new Error(`the run ${id} never ended: ${JSON.stringify(found)}`);
      await new Promise((later) => setTimeout(later, 100));
    }
  };
  const first = await applied('ship');

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
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.setDefaultTimeout(30_000);
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.panes').waitFor();
    return page;
  };
  // The page of the line that tells of the run of `name`, opened from the box.
  const opened = async (page, name, how = 'click') => {
    if (how === 'tap') {
      await page.getByRole('button', { name: 'Expand the side panel' }).tap();
      await page.locator('.rail .rail-inbox').first().tap();
    } else if (!(await page.getByRole('navigation', { name: 'Inbox' }).isVisible())) await page.locator('.rail .rail-inbox').first().click();
    const box = page.getByRole('navigation', { name: 'Inbox' });
    // The list holds still while it is pointed at, so it is read with the pointer off it.
    if (how === 'click') await page.mouse.move(900, 700);
    const line = box.locator('.inbox-line', { hasText: `Ada's run of ${name} waits for your go-ahead` }).first();
    await line.locator('.inbox-press')[how]({ position: { x: 24, y: 10 } });
    const shown = page.locator('.inbox-page');
    await shown.locator('.told-go').waitFor();
    return shown;
  };
  // Nothing of the part over anything else or past the page, and nothing to scroll sideways.
  const apart = async (page, what) => {
    const found = await page.evaluate(() => {
      const edge = (document.querySelector('.inbox-page-message') ?? document.querySelector('.inbox-page')).getBoundingClientRect();
      const picked = '.told-go > .inbox-page-label, .told-go-says, .told-go-links a, .told-go-standing, .told-go-whynot, .told-go-acts button, .told-fail-label, .told-fail-acts button';
      const boxes = [...document.querySelectorAll(picked)].map((one) => ({ said: one.textContent.trim().slice(0, 30) || one.className, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < edge.left - 0.5 || one.box.right > edge.right + 0.5) wrong.push(`${one.said} leaves the page`);
        for (const two of boxes.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      const page = document.querySelector('.inbox-page');
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth, within: page.scrollWidth - page.clientWidth };
    });
    expect(`the run that waits ${what}: something to be there`, found.count >= 4);
    expect(`the run that waits ${what}: nothing over anything else or outside the page, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`the run that waits ${what}: nothing to scroll sideways, got ${found.sideways}px and ${found.within}px`, found.sideways <= 0 && found.within <= 0);
  };
  const part = (shown, id) => shown.locator(`.told-go[data-ask="${id}"]`);

  // ── Grace's page: whose run of what, what it says, the address, how long, and what she may do ──
  const page = await signedIn('grace@example.com');
  let loads = 0;
  page.on('load', () => (loads += 1));
  const shown = await opened(page, 'ship');
  const one = part(shown, first.ask);
  await one.locator('.inbox-page-label', { hasText: "Ada's run of ship waits for a go-ahead" }).waitFor();
  expect('what it says is to go on', (await one.locator('.told-go-says').textContent()).includes('The build of the shop is ready to go live'));
  const link = one.locator('.told-go-links a');
  expect('the address to look at a link of its own', (await link.getAttribute('href')) === 'https://example.com/preview' && (await link.getAttribute('target')) === '_blank');
  expect('it to wait, and how long for', /waits for somebody who may let it go on, for \d+ days? more/.test(await one.locator('.told-go-standing').textContent()));
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await apart(page, `at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.screenshot({ path: `${out}/goahead-1-told.png` });

  // ── What the line is about is the run that waits: pressed, that run's page opens as a tab, and the line's page is there to come back to ──
  const about = shown.locator('.inbox-page-about .inbox-page-opens');
  await about.click();
  await page.locator(`.tab[aria-selected="true"][data-key="run:${first.run}"]`).waitFor();
  await page.locator('.pipeline-run').waitFor();
  await page.locator('.tabs .tab', { hasText: "Ada's run of ship" }).first().click();
  await one.locator('.told-go-acts').waitFor();

  // ── Ada started it: she is not told, shown it she may not decide it, and refused ──
  const adas = JSON.parse((await cli('inbox', '--json')).stdout).lines;
  expect('Ada not told of her own run', !adas.some((line) => (line.says ?? '').includes('waits for your go-ahead')));
  const seen = JSON.parse((await cli('asked', first.ask, '--json')).stdout);
  expect(`Ada shown it with why she may not let it go on, got ${JSON.stringify(seen)}`, seen.may === false && /you started this run/.test(seen.why_not ?? ''));
  const refused = await cli('approve', first.ask).then(() => null, (why) => String(why.stderr));
  expect(`the daemon to refuse Ada's approval of her own run, got ${refused}`, refused !== null);

  // ── Grace approves it: the page says so, and the run goes on and ships ──
  await one.getByRole('button', { name: 'Approve', exact: true }).click();
  await one.locator('.told-done', { hasText: 'Approved: the run of ship goes on.' }).waitFor();
  await one.locator('.told-go-standing', { hasText: 'Grace approved it, and the run goes on.' }).waitFor();
  expect('nothing more to press once it is decided', (await one.getByRole('button', { name: /^(Approve|Decline)$/ }).count()) === 0);
  const shipped = await ended(first.run);
  expect(`the run to have gone on and ended well, got ${JSON.stringify(shipped)}`, shipped.status === 'ok');
  await page.screenshot({ path: `${out}/goahead-2-approved.png` });

  // ── A second run, declined with why: it fails saying it ──
  const second = await applied('ship-again');
  const again = await opened(page, 'ship-again');
  const two = part(again, second.ask);
  await two.getByRole('button', { name: 'Decline', exact: true }).click();
  await two.locator('textarea').fill('not before the opening');
  await apart(page, 'with Decline open');
  await page.screenshot({ path: `${out}/goahead-3-declining.png` });
  await two.getByRole('button', { name: 'Decline it', exact: true }).click();
  await two.locator('.told-go-standing', { hasText: 'Grace declined to let it go on: not before the opening.' }).waitFor();
  const declined = await ended(second.run);
  expect(`the run to have failed saying why, got ${JSON.stringify(declined)}`, declined.status === 'failed' && (declined.error ?? '').includes('not before the opening'));
  expect(`Grace's page never to have been read again, was ${loads} times`, loads === 0);

  // ── The run's own page, opened by its address: the stage that waits has Approve and Decline there too ──
  const third = await applied('ship-later');
  await page.evaluate((id) => {
    location.hash = `/run/${id}`;
  }, third.run);
  const waits = page.locator(`.run-approval .told-go[data-ask="${third.ask}"]`);
  await waits.getByRole('button', { name: 'Approve', exact: true }).waitFor();
  expect('Decline beside Approve on the run\'s page', (await waits.getByRole('button', { name: 'Decline', exact: true }).count()) === 1);
  expect(`the run's page opened by its address with the page not read again, was ${loads} times`, loads === 0);
  await page.screenshot({ path: `${out}/goahead-4-run-page.png` });

  // ── A phone: the third run, the buttons a finger's size ──
  await page.close();
  const phone = await signedIn('grace@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  const small = await opened(phone, 'ship-later', 'tap');
  const three = part(small, third.ask);
  await three.getByRole('button', { name: 'Approve', exact: true }).waitFor();
  await apart(phone, 'on a phone');
  const fingers = await three.locator('.told-go-acts button').evaluateAll((all) => all.map((button) => button.getBoundingClientRect().height));
  expect(`the buttons a finger's size on a phone, got ${fingers}`, fingers.every((tall) => tall >= 40));
  await phone.screenshot({ path: `${out}/goahead-5-phone.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  console.error(await grace('asked').then((said) => said.stdout, (why) => String(why)));
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
