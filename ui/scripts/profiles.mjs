// Two factories on one device, one called prod and one called dev1, and a
// third with no profile: what each page says of where it is, on a desk
// and on a phone, and the way from one to another.
//
//   DAEMON=../target/debug/factory node scripts/profiles.mjs [out-dir]
//
// The daemons have a home of their own, so the list they write themselves
// into is theirs and not the list of whoever runs this. A picture is taken
// of each thing looked at, and what must be true of it is checked: a named
// factory says which it is in the browser tab's title, on the sign-in form,
// across the top of a phone and in Settings, in words that are no button,
// and at the head of no side panel, which says what the panel is, whole; a
// factory with no name says nothing; the other is offered in Settings where
// it can be reached and only there, and going to it means signing in to
// it. The profile variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { createServer as createProxy, request as forward } from 'node:http';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { toolsAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const home = join(out, 'profiles-home');
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const PASSWORD = 'correct horse';

/** Starts a daemon and says where it answers, which for a named one is where it chose. */
async function start(args) {
  const daemon = spawn(binary, ['serve', '--adapter', 'builtin', ...args], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
  endsWithThis(daemon);
  return { daemon, url: await saysWhere(daemon) };
}

/**
 * Something in front of a factory, as a private network's proxy is: another
 * port, which leads to that factory and no other, and says it passed the
 * request on.
 */
async function inFrontOf(url) {
  const to = new URL(url);
  const proxy = createProxy((asked, answer) => {
    const passed = forward(
      { host: to.hostname, port: to.port, path: asked.url, method: asked.method, headers: { ...asked.headers, 'x-forwarded-for': '100.64.0.9' } },
      (answered) => {
        answer.writeHead(answered.statusCode, answered.headers);
        answered.pipe(answer);
      },
    );
    passed.on('error', () => answer.destroy());
    asked.pipe(passed);
  });
  // On a port the system gives it as it listens, read from the listener it keeps.
  await new Promise((listening) => proxy.listen(0, '127.0.0.1', listening));
  return { proxy, url: `http://127.0.0.1:${proxy.address().port}` };
}

const problems = [];
const expect = (what, ok) => {
  if (!ok) problems.push(`  ${what}`);
};
let taken = 0;
const picture = async (page, name) => page.screenshot({ path: join(out, `profiles-${String(++taken).padStart(2, '0')}-${name}.png`) });

async function signIn(page, who) {
  await page.getByLabel('Email').fill(`${who}@example.com`);
  await page.getByLabel('Password').fill(PASSWORD);
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.shell').waitFor();
}

/** Where on the screen something is, or nothing when it is not shown. */
const placed = (locator) => locator.first().boundingBox();

async function onADesk(browser, prod, dev, plain, product) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  page.setDefaultTimeout(15_000);

  // Before anyone has signed in, the form says which factory it is for.
  await page.goto(prod.url);
  await page.locator('.where-chip').waitFor();
  expect('the sign-in form to say prod', (await page.locator('.where-chip').innerText()) === 'prod');
  expect('the tab to be called by the profile first', (await page.title()) === `prod · ${product}`);
  await picture(page, 'sign-in-says-which');
  await signIn(page, 'ada');

  // Signed in, it is said first in the browser tab's title, and in Settings with the others; nowhere in the frame on a
  // wide screen. Each side panel's head says what the panel is (Channels, Resources, Inbox, Extensions) and not which
  // factory this is: beside the profile the title was cut short. Nothing at the head is
  // cut short or lies over anything else: at the usual width the title is whole, and dragged as narrow as it goes only
  // the title gives way, never what is pressed.
  await page.locator('.sidebar .brand').waitFor();
  expect('the tab to be called by the profile first once signed in', (await page.title()) === `prod · ${product}`);
  const heads = async (what, panelName, narrow = false) => {
    expect(`the panel to be called ${panelName} for whoever hears it read out ${what}`, (await page.getByRole('navigation', { name: panelName, exact: true }).count()) === 1);
    const head = await page.locator('.sidebar .brand').evaluate((brand) => {
      const frame = brand.getBoundingClientRect();
      // The title, and each thing pressed at the head, which keep together at its right (under the title on a phone where the two do not fit).
      const parts = [...brand.querySelectorAll(':scope > :not(.brand-acts), :scope > .brand-acts > *')].filter((one) => one.getClientRects().length > 0);
      const boxes = parts.map((one) => {
        const at = one.getBoundingClientRect();
        const cut = one.matches('button, span:first-child') ? one.scrollWidth - one.clientWidth : 0;
        return { said: one.getAttribute('aria-label') ?? one.textContent.trim(), button: one.matches('button'), left: at.left, right: at.right, top: at.top, bottom: at.bottom, width: at.width, cut };
      });
      // What the head says that is not something to press.
      const words = parts.filter((one) => !one.matches('button, input')).map((one) => one.textContent.trim()).filter(Boolean).join(' ');
      return { words, first: boxes[0]?.said, profiled: brand.querySelectorAll('.where').length, frame: { left: frame.left, right: frame.right }, boxes };
    });
    expect(`the head to say ${panelName} and nothing else ${what}, got "${head.words}"`, head.words === panelName);
    expect(`the title to be first at the head ${what}, got ${head.first}`, head.first === panelName);
    expect(`the head to say nothing of the profile ${what}`, head.profiled === 0);
    const wrong = [];
    for (const [at, one] of head.boxes.entries()) {
      if (one.left < head.frame.left - 0.5 || one.right > head.frame.right + 0.5) wrong.push(`${one.said} leaves the head`);
      if (one.cut > 0.5 && (one.button || !narrow)) wrong.push(`${one.said} is cut short by ${one.cut}px`);
      if (one.button && one.width < 20) wrong.push(`${one.said} is squeezed to ${one.width}px`);
      for (const two of head.boxes.slice(at + 1)) if (Math.min(one.right, two.right) - Math.max(one.left, two.left) > 0.5 && Math.min(one.bottom, two.bottom) - Math.max(one.top, two.top) > 0.5) wrong.push(`${one.said} lies over ${two.said}`);
    }
    expect(`nothing at the head cut short or over anything else ${what}: ${wrong.join('; ')}`, wrong.length === 0);
  };
  const nowhereInTheFrame = async (what) => {
    expect(`nothing in the frame to say prod ${what}`, (await page.locator('.shell .where').evaluateAll((all) => all.filter((one) => one.getClientRects().length > 0).length)) === 0);
    expect(`nothing on the rail to say where the page is ${what}`, (await page.locator('.rail .where').count()) === 0 && !(await page.locator('.rail').innerText()).includes('prod'));
  };
  await heads('under Channels', 'Channels');
  await nowhereInTheFrame('under Channels');
  expect('nothing across the top on a wide screen', (await placed(page.locator('.where-band'))) === null);
  await picture(page, 'side-panel-says-what-it-is');
  await page.getByRole('button', { name: 'Resources', exact: true }).click();
  await heads('under Resources', 'Resources');
  await page.getByRole('button', { name: 'Extensions', exact: true }).click();
  await heads('under Extensions', 'Extensions');
  await page.locator('.rail .rail-inbox').click();
  await heads('under the inbox', 'Inbox');
  // Dragged as narrow as it goes, the title gives way before anything that is pressed at the head does (`heads`).
  const edge = await placed(page.locator('.side-edge'));
  await page.mouse.move(edge.x + edge.width / 2, edge.y + 300);
  await page.mouse.down();
  await page.mouse.move(60, edge.y + 300, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 180);
  for (const name of ['Channels', 'Resources', 'Extensions']) {
    await page.getByRole('button', { name, exact: true }).click();
    await heads(`under ${name} dragged as narrow as it goes`, name, true);
  }
  await page.locator('.rail .rail-inbox').click();
  await heads('under the inbox dragged as narrow as it goes', 'Inbox', true);
  await picture(page, 'side-panel-narrow');
  await page.getByRole('button', { name: 'Resources', exact: true }).click();
  // With the side panel folded away nothing stands in for its head: the tabs begin with what brings it back.
  await page.getByRole('button', { name: 'Collapse the side panel' }).click();
  await sideSettled(page);
  await page.getByRole('button', { name: 'Expand the side panel' }).waitFor();
  await nowhereInTheFrame('with the side panel folded away');
  expect('the tab to be called by the profile first with the side panel folded away', (await page.title()) === `prod · ${product}`);
  await picture(page, 'side-folded-says-nothing');
  await page.getByRole('button', { name: 'Expand the side panel' }).click();
  await sideSettled(page);
  await page.locator('.sidebar .brand').waitFor();

  // The others on the device are listed in Settings, with what a profile is: this one, and the way to the other.
  const settings = async () => {
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.locator('.where-list').waitFor();
  };
  await settings();
  const list = page.locator('.where-list');
  const lines = list.locator('li');
  expect('two factories to be listed', (await lines.count()) === 2);
  expect('this one to be first, and said to be where the page is', (await lines.nth(0).innerText()).includes('You are here'));
  expect('the other to be offered at its own address', (await list.locator('a.where-place').getAttribute('href')) === dev.url);
  expect('the list to say that each is signed in to separately', (await page.locator('.where-panel').innerText()).includes('signed in to separately'));
  await picture(page, 'the-others');

  // Going to the other is going to another factory: it asks who is there.
  await list.locator('a.where-place').click();
  await page.locator('.where-chip').waitFor();
  expect('the other factory to be at its own address', page.url().startsWith(dev.url));
  expect('the other factory to ask for a sign-in of its own', (await page.getByRole('button', { name: 'Sign in', exact: true }).count()) > 0);
  expect('its form to say dev1', (await page.locator('.where-chip').innerText()) === 'dev1');
  await picture(page, 'the-other-asks-who-is-there');
  await signIn(page, 'grace');
  expect('its tab to be called dev1', (await page.title()) === `dev1 · ${product}`);
  await picture(page, 'the-other');
  // And the first is still signed in: a browser on one host holds a session with each.
  await page.goto(prod.url);
  await page.locator('.sidebar .brand').waitFor();
  expect('the first factory to be signed in still, its tab called prod', (await page.title()) === `prod · ${product}`);

  // Through a proxy, the other has no address that can honestly be given: it is listed, with why.
  const front = await inFrontOf(prod.url);
  try {
    await page.goto(front.url);
    // Another address is another page: it holds no key of the session, and
    // asks who is there. Signed in, it and the first are both signed in.
    await signIn(page, 'ada');
    await settings();
    expect('the other to be listed without a link through a proxy', (await page.locator('.where-list a').count()) === 0);
    const why = await page.locator('.where-list li').nth(1).innerText();
    expect(`the other to say why it cannot be opened through a proxy (it says: ${why})`, why.includes('This page came through') && why.includes('leads to this factory and to no other'));
    await picture(page, 'through-a-proxy');
  } finally {
    front.proxy.close();
    front.proxy.closeAllConnections();
  }

  // A factory with no profile is as it was: nothing says where it is, anywhere.
  await page.goto(plain.url);
  await signIn(page, 'ada');
  expect('nothing to say where a factory with no profile is', (await page.locator('.where').count()) === 0);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('heading', { name: 'Settings', level: 1 }).waitFor();
  expect('its settings to say nothing of a profile', (await page.locator('.where-panel, .where').count()) === 0);
  expect('its tab to be called what the product is', (await page.title()) === product);
  await picture(page, 'no-profile');
  await context.close();
}

async function onAPhone(browser, prod) {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  page.setDefaultTimeout(15_000);
  const band = page.locator('.where-band');
  const still = async (what) => {
    const at = await placed(band);
    expect(`the line across the top to say prod ${what}`, at && at.y === 0 && at.width === 390 && at.height >= 24 && (await band.innerText()) === 'prod');
    // Nothing lies over it: what is at its middle is the line itself.
    const top = await page.evaluate(() => document.elementFromPoint(195, 12)?.closest('.where-band') !== null);
    expect(`nothing to lie over the line across the top ${what}`, top);
    expect(`the page not to scroll sideways ${what}`, await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
  };

  await page.goto(prod.url);
  await page.locator('.where-chip').waitFor();
  expect('the sign-in form to say prod on a phone', (await page.locator('.where-chip').innerText()) === 'prod');
  await picture(page, 'phone-sign-in');
  await signIn(page, 'ada');
  await band.waitFor();
  await still('on the first screen');
  const tabs = await placed(page.locator('.tabs'));
  expect('the tabs to start under the line across the top', tabs && tabs.y >= 24);
  await picture(page, 'phone-says-which');

  // It is words there too, and nothing is pressed.
  expect('the line across the top to be no button', (await band.evaluate((one) => one.tagName)) !== 'BUTTON');

  // With the side panel over the page, it is still there, and said once.
  await page.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(page);
  await page.locator('.sidebar').waitFor();
  await still('with the side panel over the page');
  expect('the side panel not to say it a second time on a phone', (await page.locator('.sidebar .where').count()) === 0);
  // Its head says what it is, whole, with nothing cut short or over anything else.
  const head = await page.locator('.sidebar .brand').evaluate((brand) => {
    const frame = brand.getBoundingClientRect();
    // The title, and each thing pressed at the head, which keep together at its right (under the title on a phone where the two do not fit).
    const parts = [...brand.querySelectorAll(':scope > :not(.brand-acts), :scope > .brand-acts > *')].filter((one) => one.getClientRects().length > 0);
    return {
      title: parts[0]?.textContent.trim(),
      wrong: parts.flatMap((one, at) => {
        const box = one.getBoundingClientRect();
        const said = one.getAttribute('aria-label') ?? one.textContent.trim();
        const found = [];
        if (box.left < frame.left - 0.5 || box.right > frame.right + 0.5) found.push(`${said} leaves the head`);
        if (one.matches('button, span:first-child') && one.scrollWidth - one.clientWidth > 0.5) found.push(`${said} is cut short`);
        for (const two of parts.slice(at + 1)) {
          const other = two.getBoundingClientRect();
          if (Math.min(box.right, other.right) - Math.max(box.left, other.left) > 0.5 && Math.min(box.bottom, other.bottom) - Math.max(box.top, other.top) > 0.5) found.push(`${said} lies over ${two.getAttribute('aria-label') ?? two.textContent.trim()}`);
        }
        return found;
      }),
    };
  });
  expect(`the side panel's head on a phone to say Channels first, got ${head.title}`, head.title === 'Channels');
  expect(`nothing at the side panel's head on a phone cut short or over anything else: ${head.wrong.join('; ')}`, head.wrong.length === 0);
  const rail = await placed(page.locator('.rail .rail-button').first());
  expect('the top of the rail to be under the line, not behind it', rail && rail.y >= 24);
  await picture(page, 'phone-side-panel');

  // In a channel, with its tools panel over the page.
  await page.locator('.nav-item', { hasText: 'general' }).tap();
  await page.getByRole('heading', { name: '# general' }).waitFor();
  await still('in a channel');
  await page.getByRole('button', { name: 'Tools' }).tap();
  await toolsAtRest(page).waitFor();
  await page.locator('.context').waitFor();
  await still('with the tools panel over the page');
  const context_ = await placed(page.locator('.context-tabs'));
  expect('the head of the tools panel to be under the line', context_ && context_.y >= 24);
  await picture(page, 'phone-context-panel');

  // The others are listed in Settings: under the line, within the screen, and nothing scrolling sideways.
  await page.getByRole('button', { name: 'Back to the channel' }).tap();
  await page.locator('.context').waitFor({ state: 'detached' });
  await page.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(page);
  await page.locator('.rail').getByRole('button', { name: 'Settings', exact: true }).tap();
  await page.locator('.where-list').waitFor();
  await page.locator('.where-panel').scrollIntoViewIfNeeded();
  await still('with the others listed in Settings');
  const listed = await placed(page.locator('.where-panel'));
  expect(`the list to be under the line and within the screen, and it is at ${JSON.stringify(listed)}`, listed && listed.y >= 24 && listed.x >= 0 && listed.x + listed.width <= 390);
  await picture(page, 'phone-the-others');

  // Held on its side the phone is short, not narrow: it is still a phone.
  await page.setViewportSize({ width: 844, height: 390 });
  await page.waitForTimeout(300);
  const across = await placed(band);
  expect('the line across the top with the phone on its side', across && across.y === 0 && across.width === 844);
  await picture(page, 'phone-on-its-side');
  await context.close();
}

await mkdir(out, { recursive: true });
for (const old of await readdir(out)) if (/^profiles-\d+-.*\.png$/.test(old)) await rm(join(out, old));
await rm(home, { recursive: true, force: true });
await mkdir(home, { recursive: true });

const started = [];
let failed = null;
try {
  // Two by name, with nothing said of where their data is or where they listen; and one with no name.
  const prod = await start(['--profile', 'prod']);
  started.push(prod);
  const dev = await start(['--profile', 'dev1']);
  started.push(dev);
  const plain = await start(['--data', join(home, 'plain'), '--listen', '127.0.0.1:0']);
  started.push(plain);
  const cli = (...args) => run(binary, args, { cwd: repo, env });
  await cli('--profile', 'prod', 'register', '--email', 'ada@example.com', '--name', 'Ada', '--password', PASSWORD);
  await cli('--profile', 'prod', 'channel', 'create', 'general');
  await cli('--profile', 'dev1', 'register', '--email', 'grace@example.com', '--name', 'Grace', '--password', PASSWORD);
  await cli('--url', plain.url, 'register', '--email', 'ada@example.com', '--name', 'Ada', '--password', PASSWORD);
  // What the product is called is the daemon's to say.
  const product = (await (await fetch(`${plain.url}/api/meta`)).json()).name;

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  await onADesk(browser, prod, dev, plain, product);
  await onAPhone(browser, prod);
  await browser.close();
} catch (err) {
  failed = err;
} finally {
  for (const { daemon } of started) {
    if (daemon.exitCode !== null) continue;
    daemon.kill('SIGTERM');
    await new Promise((gone) => daemon.on('exit', gone));
  }
  await rm(home, { recursive: true, force: true });
}
if (failed) console.error(failed);
if (problems.length > 0) console.error(`\n${problems.length} thing(s) wrong with saying where the page is:\n${problems.join('\n')}`);
if (failed || problems.length > 0) process.exit(1);
console.log(`\neach page says where it is; ${taken} pictures are in ${out}`);
