// How what is read looks, chosen from the page it is read on: the
// Appearance button at the head of a topic and of a library's document
// (`views/appearance.tsx`), and the same three in Settings.
//
//   DAEMON=../target/debug/factory node scripts/appearance.mjs [out-dir]
//
// Ada, whose system asks for light, with a long topic open. Never touched,
// the topic reads exactly as it did before there was any of this: its
// column as wide as the measure every page has, and with the reading
// column's class taken off its text laid out in the same boxes and the same
// picture of it pixel for pixel, the page painted whole for each. The button is
// reached by the keyboard and opens a menu of Text, Width and Colour that
// the arrows walk; Enter on Wide widens the column by its box at once,
// with the side panel where it was; Larger makes the topic's text larger
// and nothing outside the column; Dark puts the page in the theme's dark
// list though the system asks for light; Escape puts the menu away and
// gives the keyboard back to its button. The account keeps all three, the
// page read again wears them before it reads anything, and another
// browser of hers, which kept nothing, wears them once it reads the
// account. A document of a library is as wide, and the button at its head
// puts it back to Standard. Settings shows the same three, and Light there
// is worn at once. Nothing of the topic or of the menu lies over anything
// else, and nothing scrolls sideways, at 1280, 1000 and 800 wide and at a
// phone's, touched, wide and with the largest text. The environment
// variables are those of walkthrough.mjs, `LOOK` among them.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'appearance-data');
const home = join(out, 'appearance-home');
const looked = process.env.LOOK ? '-looked' : '';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that nothing here reads or writes what a factory
// already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const PARAGRAPH =
  'A ledger of quinces is kept by the season, each crate weighed as it comes in and again as it goes out, so that what was lost between the two is known by the week and not guessed at by the year. ' +
  'Whoever keeps it writes the weight, the grower, the day and the shed, and nothing else: what is not written is not argued over later.';
const BODY = ['## How it is kept', PARAGRAPH, PARAGRAPH, '## What it is for', PARAGRAPH, PARAGRAPH].join('\n\n');

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  const docs = join(data, 'projects', 'shop', 'docs');
  await rm(docs, { recursive: true, force: true });
  await mkdir(docs, { recursive: true });
  await writeFile(join(docs, 'welcome.md'), `# Welcome\n\n${PARAGRAPH}\n\n${PARAGRAPH}\n`);
  await cli('apply', 'std/docs', '--project', 'shop');
  await cli('topic', 'make', 'The quince ledger', '--body', BODY);

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
  // Her system asks for light, so that Dark is seen to be chosen and not the system's.
  const signedIn = async (how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: 'light' });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const toPanel = async (on, name, tap = false) => {
    const rail = on.locator('.rail').getByRole('button', { name, exact: true });
    await rail.waitFor();
    if ((await rail.getAttribute('aria-pressed')) !== 'true') await (tap ? rail.tap() : rail.click());
    await on.getByRole('navigation', { name, exact: true }).waitFor();
  };
  const openTopic = async (on, tap = false) => {
    await toPanel(on, 'Topics', tap);
    const item = on.getByRole('navigation', { name: 'Topics', exact: true }).locator('.topic-line .nav-item', { hasText: 'The quince ledger' });
    await (tap ? item.tap() : item.click());
    await on.locator('.topic-page.reading-column .topic-post').waitFor();
  };
  const POST = '.topic-post';
  const TEXT = '.topic-post :is(.ProseMirror, .topic-doc-read)';
  const widthOf = async (on, selector) => (await on.locator(selector).first().boundingBox()).width;
  const sizeOf = (on, selector) => on.locator(selector).first().evaluate((one) => parseFloat(getComputedStyle(one).fontSize));
  const ground = (on) => on.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--bg0').trim());
  const marks = (on) => on.evaluate(() => ({ ...document.documentElement.dataset }));
  const kept = (on) =>
    on.evaluate(async () => {
      const answer = await fetch('/api/appearance', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      return answer.json();
    });
  const laid = async (on, what) => {
    // A menu is placed again as the window is told it was resized, which is as it is next drawn.
    if (await on.locator('.appearance-menu').count())
      await on
        .waitForFunction(() => {
          const box = document.querySelector('.appearance-menu')?.getBoundingClientRect();
          return !box || (box.left >= 0 && box.top >= 0 && box.right <= window.innerWidth + 0.5 && box.bottom <= window.innerHeight + 0.5);
        }, null, { timeout: 5000 })
        .catch(() => undefined);
    for (const within of ['.topic-page', '.appearance-menu']) {
      if ((await on.locator(within).count()) === 0) continue;
      const wrong = await pageCrowded(on, within);
      expect(`nothing of ${within} over anything else ${what}: ${wrong.join('; ')}`, wrong.length === 0);
    }
    const sideways = await on.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`nothing to scroll sideways ${what}, got ${sideways}px`, sideways <= 0);
    const menu = (await on.locator('.appearance-menu').count()) ? await on.locator('.appearance-menu').boundingBox() : null;
    const view = on.viewportSize();
    if (menu) expect(`the menu wholly in the window ${what}, at ${JSON.stringify(menu)}`, menu.x >= 0 && menu.y >= 0 && menu.x + menu.width <= view.width + 0.5 && menu.y + menu.height <= view.height + 0.5);
  };
  // Where two pictures differ, decoded by the page itself: 'the same', or how many pixels differ and where.
  const differing = (on, a, b) =>
    on.evaluate(
      async ([a, b]) => {
        const pixels = async (said) => {
          // Made from its bytes here, as the page may fetch nothing that is not the daemon's.
          const picture = await createImageBitmap(new Blob([Uint8Array.from(atob(said), (one) => one.charCodeAt(0))], { type: 'image/png' }));
          const canvas = new OffscreenCanvas(picture.width, picture.height);
          const drawn = canvas.getContext('2d');
          drawn.drawImage(picture, 0, 0);
          return { width: picture.width, height: picture.height, data: drawn.getImageData(0, 0, picture.width, picture.height).data };
        };
        const [one, other] = [await pixels(a), await pixels(b)];
        if (one.width !== other.width || one.height !== other.height) return `sized ${one.width}x${one.height} and ${other.width}x${other.height}`;
        let count = 0;
        let box = null;
        let most = 0;
        for (let at = 0; at < one.data.length; at += 4) {
          if (one.data[at] === other.data[at] && one.data[at + 1] === other.data[at + 1] && one.data[at + 2] === other.data[at + 2] && one.data[at + 3] === other.data[at + 3]) continue;
          count += 1;
          for (let part = 0; part < 4; part += 1) most = Math.max(most, Math.abs(one.data[at + part] - other.data[at + part]));
          const [x, y] = [(at / 4) % one.width, Math.floor(at / 4 / one.width)];
          box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        }
        return count === 0 ? 'the same' : `${count} pixels differ, by ${most} at most, within ${JSON.stringify(box)}`;
      },
      [a.toString('base64'), b.toString('base64')],
    );
  const button = (on) => on.locator('.view-head').getByRole('button', { name: 'Appearance', exact: true });
  const line = (on, name) => on.getByRole('menuitemradio', { name, exact: true });

  const page = await signedIn();
  await openTopic(page);
  await sideSettled(page);

  // ── Never touched: as it was ────────────────────────────────────────────
  expect(`the page light, as her system asks, got ${JSON.stringify(await marks(page))}`, 'light' in (await marks(page)));
  const column = await page.evaluate(() => getComputedStyle(document.querySelector('.topic-page')).getPropertyValue('--column').trim());
  expect(`the column's measure to be the one every page has, got ${column}`, column === '720px');
  const standard = await widthOf(page, POST);
  expect(`the post as wide as that measure, got ${standard}`, Math.round(standard) === 760);
  const usualSize = await sizeOf(page, TEXT);
  // The post with the class taken off is drawn the same: nothing of the column is changed by it until chosen.
  await page.evaluate(() => document.activeElement?.blur());
  // First by its boxes: each line of its text where it lies, in the font, size and colour it is written in.
  const laidOut = () =>
    page.evaluate(() => {
      const said = [];
      const texts = document.createTreeWalker(document.querySelector('.topic-page'), NodeFilter.SHOW_TEXT);
      for (let node = texts.nextNode(); node; node = texts.nextNode()) {
        if (!node.textContent.trim()) continue;
        const range = document.createRange();
        range.selectNodeContents(node);
        const style = getComputedStyle(node.parentElement);
        const lines = [...range.getClientRects()].map((one) => [one.x, one.y, one.width, one.height].join(','));
        said.push(`"${node.textContent.trim().slice(0, 24)}" ${style.font} ${style.color} at ${lines.join(' ')}`);
      }
      return said;
    });
  // Then pixel for pixel, of the whole window: an element's picture is cut at its box, which may stand between two
  // pixels and be cut a little otherwise each time. The page is painted again whole before each picture: a browser
  // paints again only what a change touched, and the faint edge a letter's first stroke casts one pixel to its left lies
  // outside the box it is painted again in, so that edge is there or not by what was painted before (the first
  // letters of the lines, one column of pixels, came out apart when other work held the browser back).
  const paintedWhole = () =>
    page.evaluate(async () => {
      const drawn = () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
      const whole = document.querySelector('.topic-page');
      whole.style.backgroundColor = 'rgb(1, 2, 3)';
      await drawn();
      whole.style.backgroundColor = '';
      await drawn();
    });
  // Taken once the page is still.
  const shot = async () => {
    await paintedWhole();
    let last = await page.screenshot({ animations: 'disabled' });
    for (let tries = 0; tries < 20; tries += 1) {
      const next = await page.screenshot({ animations: 'disabled' });
      if ((await differing(page, last, next)) === 'the same') return next;
      last = next;
    }
    throw new Error('expected the page to be still, and it changed from one picture to the next twenty times');
  };
  const [laidBefore, before] = [await laidOut(), await shot()];
  await page.evaluate(() => document.querySelector('.topic-page').classList.remove('reading-column'));
  const [laidWithout, without] = [await laidOut(), await shot()];
  await page.evaluate(() => document.querySelector('.topic-page').classList.add('reading-column'));
  const after = await shot();
  const moved = laidWithout.map((line, at) => (line === laidBefore[at] ? null : `${laidBefore[at]} became ${line}`)).filter(Boolean);
  expect(`the post's text laid out the same with and without the reading column, got ${moved.join('; ')}`, moved.length === 0 && laidWithout.length === laidBefore.length);
  const apart = [await differing(page, before, without), await differing(page, after, without)];
  expect(`the post drawn the same, pixel for pixel, with and without the reading column: ${apart.join('; ')}`, apart.every((one) => one === 'the same'));
  await page.screenshot({ path: `${out}/appearance-0-standard${looked}.png` });

  // ── By the keyboard: Wide ───────────────────────────────────────────────
  const side = await page.locator('.sidebar').boundingBox();
  await button(page).focus();
  await page.keyboard.press('Enter');
  await page.locator('.appearance-menu').waitFor();
  const focused = () => page.evaluate(() => document.activeElement?.textContent?.trim());
  // The menu is drawn, and then gives the keyboard to what is chosen in it.
  await page
    .waitForFunction(() => document.activeElement?.matches('.appearance-menu [role="menuitemradio"][aria-checked="true"]'), null, { timeout: 5000 })
    .catch(async () => {
      throw new Error(`expected the chosen text size to have the keyboard, got ${await focused()}`);
    });
  expect(`the chosen text size to be Balanced, got ${await focused()}`, (await focused()) === 'Balanced');
  for (let step = 0; step < 4; step += 1) await page.keyboard.press('ArrowDown');
  expect(`the arrows to walk to Wide, got ${await focused()}`, (await focused()) === 'Wide');
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => document.documentElement.dataset.readWidth === 'wide');
  const wide = await widthOf(page, POST);
  const room = await widthOf(page, '.topic-page');
  expect(`the post to widen to the page's room, from ${standard} to ${wide} of ${room}`, wide > standard + 100 && Math.abs(wide - Math.min(room, 1440)) < 2);
  expect('the side panel where it was', JSON.stringify(await page.locator('.sidebar').boundingBox()) === JSON.stringify(side));
  expect('Wide to be marked chosen', (await line(page, 'Wide').getAttribute('aria-checked')) === 'true' && (await line(page, 'Standard').getAttribute('aria-checked')) === 'false');
  await laid(page, 'wide, with the menu open');
  await page.screenshot({ path: `${out}/appearance-1-wide${looked}.png` });

  // ── Larger and Dark ─────────────────────────────────────────────────────
  const railSize = await sizeOf(page, '.sidebar .nav-item');
  await line(page, 'Larger').click();
  await page.waitForFunction(() => 'readText' in document.documentElement.dataset);
  const larger = await sizeOf(page, TEXT);
  // Larger is a tenth more than the usual size; the usual text was at the size the page wears (a look's, where one is told).
  const worn = await page.evaluate(() => Number(getComputedStyle(document.documentElement).getPropertyValue('--size')) || 1);
  expect(`the topic's text at the larger size, from ${usualSize} to ${larger}`, Math.abs(larger - (usualSize / worn) * 1.1) < 0.2);
  expect('nothing outside the column larger', (await sizeOf(page, '.sidebar .nav-item')) === railSize);
  const lightGround = await ground(page);
  await line(page, 'Dark').click();
  await page.waitForFunction(() => !('light' in document.documentElement.dataset));
  const darkGround = await ground(page);
  expect(`the theme's dark ground though the system asks for light, from ${lightGround} to ${darkGround}`, darkGround === '#0c0c0c' && lightGround !== darkGround);
  await laid(page, 'wide, larger and dark, with the menu open');
  await page.screenshot({ path: `${out}/appearance-2-larger-dark${looked}.png` });
  await page.keyboard.press('Escape');
  await page.locator('.appearance-menu').waitFor({ state: 'detached' });
  expect('Escape to give the keyboard back to the button', await button(page).evaluate((one) => one === document.activeElement));
  expect('the topic still open', (await page.locator('.topic-page').count()) === 1);

  // ── Kept with the account, and worn as the page is read again ───────────
  await page.waitForFunction(async () => {
    const answer = await fetch('/api/appearance', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return (await answer.json()).nth === 3;
  });
  const account = await kept(page);
  expect(`the account to keep all three, got ${JSON.stringify(account)}`, account.appearance?.text === 'larger' && account.appearance.width === 'wide' && account.appearance.colour === 'dark');
  await page.reload();
  // Before the client has drawn anything, the page wears what the browser kept.
  const early = await marks(page);
  expect(`the page read again to wear it at once, got ${JSON.stringify(early)}`, early.readWidth === 'wide' && 'readText' in early && !('light' in early));
  await page.locator('.topic-page.reading-column .topic-post').waitFor();
  expect('the post wide again', Math.abs((await widthOf(page, POST)) - wide) < 2);
  expect('its text larger again', Math.abs((await sizeOf(page, TEXT)) - larger) < 0.1);

  // Another browser of hers, which kept nothing: as her account says, once it is read.
  const other = await signedIn();
  await other.waitForFunction(() => document.documentElement.dataset.readWidth === 'wide' && !('light' in document.documentElement.dataset));
  await openTopic(other);
  expect('the other browser to read it wide', Math.abs((await widthOf(other, POST)) - wide) < 2);

  // ── A document of a library ─────────────────────────────────────────────
  await toPanel(page, 'Resources');
  await unfoldAll(page);
  await page.locator('.nav [data-project]').getByRole('button', { name: / Docs$/ }).click();
  await page.locator('.library-line', { hasText: 'Welcome' }).locator('.rows-line').click();
  const DOC = '.doc-body.reading-column .writer .ProseMirror';
  await page.locator(DOC, { hasText: 'quinces' }).waitFor();
  const docWide = await widthOf(page, DOC);
  expect(`the document wide too, got ${docWide}`, docWide > 800);
  await laid(page, 'on a document, wide');
  await page.screenshot({ path: `${out}/appearance-3-document-wide${looked}.png` });
  await button(page).click();
  await line(page, 'Standard').click();
  await page.waitForFunction(() => !document.documentElement.dataset.readWidth);
  const docStandard = await widthOf(page, DOC);
  expect(`the document back to the measure, got ${docStandard}`, docStandard <= 720.5 && docStandard > 600);
  await other.waitForFunction(() => !document.documentElement.dataset.readWidth);
  expect('the other browser to follow with nothing read again', Math.abs((await widthOf(other, POST)) - standard) < 2);
  await page.keyboard.press('Escape');

  // ── Settings ────────────────────────────────────────────────────────────
  await page.goto(`${url}/settings`);
  const panel = page.locator('.appearance-setting');
  await panel.waitFor();
  const checked = (name) => panel.getByRole('radio', { name, exact: true }).getAttribute('aria-checked');
  expect('Settings to show what is chosen', (await checked('Larger')) === 'true' && (await checked('Standard')) === 'true' && (await checked('Dark')) === 'true');
  await panel.getByRole('radio', { name: 'Light', exact: true }).click();
  await page.waitForFunction(() => 'light' in document.documentElement.dataset);
  expect('the light ground worn at once', (await ground(page)) === lightGround);
  await page.screenshot({ path: `${out}/appearance-4-settings${looked}.png` });

  // ── By the boxes ────────────────────────────────────────────────────────
  await openTopic(page);
  await button(page).click();
  await line(page, 'Wide').click();
  await line(page, 'Largest').click();
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await laid(page, `at ${width} wide, wide and largest`);
  }
  await page.screenshot({ path: `${out}/appearance-5-800${looked}.png` });
  await page.keyboard.press('Escape');
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await laid(page, `at ${width} wide, wide and largest, the menu put away`);
  }

  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.waitForFunction(() => document.documentElement.dataset.readWidth === 'wide');
  if ((await phone.locator('nav.sidebar').count()) === 0) await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  await openTopic(phone, true);
  // The panel is put away as the topic is opened: the page is touched once it has gone.
  await sideSettled(phone);
  expect('the side panel put away as the topic was opened', (await phone.locator('.shell > .sidebar').count()) === 0);
  await laid(phone, "at a phone's width, wide and largest");
  await button(phone).tap();
  await phone.locator('.appearance-menu').waitFor();
  await laid(phone, "at a phone's width with the menu open");
  const tall = await line(phone, 'Dark').boundingBox();
  expect(`a line of the menu tall enough for a finger, got ${tall.height}`, tall.height >= 40);
  await phone.screenshot({ path: `${out}/appearance-6-phone${looked}.png` });
  await line(phone, 'Standard').tap();
  await phone.waitForFunction(() => !document.documentElement.dataset.readWidth);
  await laid(phone, "at a phone's width, standard");

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
