// Full screen, and Open larger in a topic: what a view (an attached page)
// and a diagram are looked at closely by.
//
//   DAEMON=../target/debug/factory node scripts/fullscreen.mjs [out-dir]
//
// A topic's article has a view, a page that counts, as a figure, and a
// `mermaid` diagram. Under the view, beside its caption, are Open larger
// and Full screen. Open larger shows it in a dialog, which has Full screen
// too, and Escape there leaves full screen and leaves the dialog open.
// Full screen puts the view's box (the client's, never the page in it)
// into the browser's full screen: the frame is the screen's size and so is
// the page inside it, the page's background is the client's, and a small
// Exit full screen stands in its corner. Exit full screen and Escape each
// leave it, and the view is where and as tall as it was. Across all of it
// the frame is the one it was: its page was not made again, and its count
// went on. The frame's box allows scripts and nothing more. A diagram's
// Full screen fills the screen with it the same way. In a channel a view's
// Full screen is beside its Open larger, and does the same; in the tools
// panel too. Nothing of the topic's page lies over anything else, plainly
// and with `LOOK="size=largest density=compact"`, in the dark and
// (`SCHEME=light`) the light. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'fullscreen-data');
const home = join(out, 'fullscreen-home');
const looked = process.env.LOOK ? '-looked' : '';
const scheme = process.env.SCHEME ?? 'dark';
const shot = (name) => `${out}/fullscreen-${name}${looked}${scheme === 'light' ? '-light' : ''}.png`;

const TITLE = 'How the till counts';
/** A page that counts, and says how large it is drawn: which page it is, by a number it drew once. */
const VIEW = `<!doctype html><style>body{color:var(--text)} b{color:var(--accent)}</style>
<p><b>Counting</b>: <span id="ticks">0</span></p><p id="size"></p>
<script>
  window.born = Math.random();
  let ticks = 0;
  setInterval(() => (document.getElementById('ticks').textContent = String(++ticks)), 40);
  const said = () => (document.getElementById('size').textContent = innerWidth + ' by ' + innerHeight);
  addEventListener('resize', said);
  said();
</script>`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });

let failed = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  const topic = JSON.parse((await cli('topic', 'make', TITLE, '--body', 'Soon.', '--json')).stdout).id;
  const file = join(home, 'Counting.html');
  await writeFile(file, VIEW);
  const id = /ratt_\w+/.exec((await cli('record', 'attach', topic, file)).stdout)?.[0];
  if (!id) throw new Error('expected the page attached, with its id said');
  const body = ['The till counts as it goes.', '', `![Counting](attachment:${id})`, '', '```mermaid', 'flowchart LR', '  Card[A card] --> Till{The till}', '  Till --> Slip[A slip]', '```', ''].join('\n');
  await cli('record', 'set', topic, `body=${body}`);
  await cli('channel', 'create', 'pantry');
  await cli('post', 'pantry', '--attach', file, 'The', 'count.');

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
  const VIEWPORT = { width: 1280, height: 860 };
  const context = await browser.newContext({ viewport: VIEWPORT, colorScheme: scheme });
  const page = await context.newPage();
  lastPage = page;
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor({ state: 'attached' });
  await page.goto(`${url}/topic/${encodeURIComponent(TITLE)}`);
  await page.locator('.topic-page').waitFor();

  /** What is in full screen: its classes, and the boxes of it and of the frame or picture in it. */
  const fullNow = () =>
    page.evaluate(() => {
      const full = document.fullscreenElement;
      if (!full) return null;
      const box = (one) => one?.getBoundingClientRect().toJSON() ?? null;
      return { classes: full.className, box: box(full), frame: box(full.querySelector('iframe')), svg: box(full.querySelector('svg')), exit: box(full.querySelector('.full-exit')), background: getComputedStyle(full).backgroundColor };
    });
  const pageBackground = () => page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  /** Waits until the page is in full screen with a box of `kind`, and says what is. */
  const entered = async (kind) => {
    await page.waitForFunction((kind) => document.fullscreenElement?.classList.contains(kind) === true, kind);
    // The box is laid out at the screen's size a frame after it is put there.
    await page.waitForFunction(() => {
      const full = document.fullscreenElement?.getBoundingClientRect();
      return full && Math.abs(full.width - innerWidth) <= 1 && Math.abs(full.height - innerHeight) <= 1;
    });
    return fullNow();
  };
  const left = () => page.waitForFunction(() => document.fullscreenElement === null);
  /** The count a view's page has reached, and which page it is. */
  const counted = (frame) => frame.evaluate(() => ({ ticks: Number(document.getElementById('ticks').textContent), born: window.born }));
  /** Waits until the view's page has counted past `from`: it is running. */
  const countsOn = (frame, from) => frame.waitForFunction((from) => Number(document.getElementById('ticks').textContent) > from + 3, from.ticks);
  /** Puts `box`'s view into full screen by `button` and holds all that is true there, then leaves by `how`. */
  const viewFull = async ({ button, iframe, what, how, picture }) => {
    const frame = await (await iframe.elementHandle()).contentFrame();
    await frame.locator('#ticks').waitFor();
    const before = await counted(frame);
    const was = await iframe.boundingBox();
    await button.click();
    const full = await entered('view-box');
    const background = await pageBackground();
    expect(`${what}'s box in full screen, filled by its frame, the page's background behind it, got ${JSON.stringify({ full, background })}`, full.frame && Math.abs(full.frame.width - VIEWPORT.width) <= 1 && Math.abs(full.frame.height - VIEWPORT.height) <= 1 && full.background === background);
    expect(`${what}'s Exit full screen in a corner, small`, full.exit && full.exit.right >= VIEWPORT.width - 40 && full.exit.top <= 40 && full.exit.width < 200);
    // The page inside is laid out at the screen's size.
    await frame.waitForFunction((size) => document.getElementById('size').textContent === `${size.width} by ${size.height}`, VIEWPORT);
    expect(`${what}'s frame to allow scripts and nothing more`, (await iframe.getAttribute('sandbox')) === 'allow-scripts' && (await iframe.getAttribute('allow')) === null);
    await countsOn(frame, before);
    if (picture) await page.screenshot({ path: shot(picture) });
    if (how === 'Escape') await page.keyboard.press('Escape');
    else await page.getByRole('button', { name: 'Exit full screen' }).click();
    await left();
    expect(`${what}'s Exit full screen gone with full screen`, (await page.locator('.full-exit').count()) === 0);
    const after = await counted(frame);
    expect(`${what}'s page the one it was, its count gone on, got ${JSON.stringify({ before, after })}`, after.born === before.born && after.ticks > before.ticks + 3);
    await countsOn(frame, after);
    // Back where it was, as tall as its page says, once it has said so again.
    await page.waitForFunction(([one, height]) => Math.abs(one.getBoundingClientRect().height - height) <= 1, [await iframe.elementHandle(), was.height]);
  };

  // ── A topic's view ────────────────────────────────────────────────────────
  const figure = page.locator('.topic-post .prose-figure.drawn-view');
  const iframe = figure.locator('iframe.view-frame');
  await (await (await iframe.elementHandle()).contentFrame()).locator('#ticks').waitFor();
  expect('the browser to offer full screen', await page.evaluate(() => document.fullscreenEnabled));
  const acts = figure.locator('.prose-view-acts');
  const [caption, larger, fullButton] = [await figure.locator('figcaption').boundingBox(), await acts.getByRole('button', { name: 'Open larger' }).boundingBox(), await acts.getByRole('button', { name: 'Full screen' }).boundingBox()];
  const frameBox = await iframe.boundingBox();
  expect(`Open larger and Full screen under the view, beside its caption, got ${JSON.stringify({ caption, larger, fullButton, frameBox })}`, larger.y >= frameBox.y + frameBox.height - 1 && Math.abs(larger.y + larger.height / 2 - (caption.y + caption.height / 2)) <= 3 && larger.x > caption.x && fullButton.x > larger.x);
  expect(`the caption still the figure's caption, got ${await figure.locator('figcaption').innerText()}`, (await figure.locator('figcaption').innerText()).trim() === 'Counting');
  for (const width of [1280, 800]) {
    await page.setViewportSize({ width, height: VIEWPORT.height });
    const wrong = await pageCrowded(page, '.topic-page');
    expect(`nothing of the topic's page over anything else at ${width}${looked}, got ${wrong.join('; ')}`, wrong.length === 0);
  }
  await page.setViewportSize(VIEWPORT);
  await figure.screenshot({ path: shot('1-topic-view') });

  // Opened larger: in a dialog, which has Full screen too. Escape in full screen leaves it, not the dialog.
  await acts.getByRole('button', { name: 'Open larger' }).click();
  const dialog = page.getByRole('dialog');
  const inDialog = dialog.locator('iframe.view-frame');
  await (await (await inDialog.elementHandle()).contentFrame()).locator('#ticks').waitFor();
  await page.waitForFunction(() => document.querySelector('[role="dialog"] iframe.view-frame')?.getBoundingClientRect().height > 40);
  await page.screenshot({ path: shot('2-larger') });
  await viewFull({ button: dialog.getByRole('button', { name: 'Full screen' }), iframe: inDialog, what: 'the view opened larger', how: 'Escape' });
  expect('the dialog still open after Escape left full screen', await dialog.isVisible());
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });

  // In full screen from where it is in the article, left by Exit full screen and then by Escape.
  await viewFull({ button: acts.getByRole('button', { name: 'Full screen' }), iframe, what: "the topic's view", how: 'Exit', picture: '3-topic-view-full' });
  await viewFull({ button: acts.getByRole('button', { name: 'Full screen' }), iframe, what: "the topic's view", how: 'Escape' });

  // ── A diagram ─────────────────────────────────────────────────────────────
  const diagram = page.locator('.topic-post .diagram');
  await diagram.locator('.diagram-box svg').waitFor();
  const drawnAt = await diagram.locator('.diagram-box svg').boundingBox();
  await diagram.getByRole('button', { name: 'Full screen' }).click();
  const whole = await entered('diagram-full');
  expect(`the diagram filling the screen, as large as it holds it whole, got ${JSON.stringify(whole)}`, whole.svg && whole.svg.width > drawnAt.width && whole.svg.right <= VIEWPORT.width + 1 && whole.svg.bottom <= VIEWPORT.height + 1 && whole.background === (await pageBackground()));
  expect('nothing opened over it by pressing it there', (await page.locator('.diagram-box').first().click().then(() => page.getByRole('dialog').count())) === 0);
  await page.screenshot({ path: shot('4-diagram-full') });
  await page.getByRole('button', { name: 'Exit full screen' }).click();
  await left();
  await diagram.getByRole('button', { name: 'Full screen' }).click();
  await entered('diagram-full');
  await page.keyboard.press('Escape');
  await left();
  const back = await diagram.locator('.diagram-box svg').boundingBox();
  expect(`the diagram back as it was drawn, got ${JSON.stringify({ drawnAt, back })}`, Math.abs(back.width - drawnAt.width) <= 1 && Math.abs(back.height - drawnAt.height) <= 1);

  // ── A channel's view, and the tools panel ─────────────────────────────────
  const button = page.getByRole('button', { name: 'Channels', exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  await page.getByRole('navigation', { name: 'Channels' }).getByText('pantry', { exact: true }).click();
  await page.getByRole('heading', { name: '# pantry' }).waitFor();
  const foot = page.locator('.channel .attached-view-foot').first();
  const inChannel = page.locator('.channel .attached-view iframe.view-frame').first();
  await foot.getByRole('button', { name: 'Full screen' }).waitFor();
  await viewFull({ button: foot.getByRole('button', { name: 'Full screen' }), iframe: inChannel, what: "the channel's view", how: 'Exit', picture: '5-channel-full' });
  await foot.getByRole('button', { name: 'Open larger' }).click();
  expect('Full screen in the channel view opened larger', await page.getByRole('dialog').getByRole('button', { name: 'Full screen' }).isVisible());
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  const panel = page.getByRole('complementary', { name: 'Tools' });
  await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  await panel.locator('.context-row', { hasText: 'Counting.html' }).click();
  await viewFull({ button: panel.getByRole('button', { name: 'Full screen' }), iframe: panel.locator('iframe.view-frame'), what: "the tools panel's view", how: 'Escape' });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: shot('failed') }).catch(() => {});
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
