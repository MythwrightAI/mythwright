// A long topic's Contents, its headings' addresses, and each section edited
// by itself.
//
//   DAEMON=../target/debug/factory node scripts/contents.mjs [out-dir]
//
// Ada owns the factory and makes from a terminal an article with five
// headings at two levels (Handbook), and one with two (Short). On a wide
// window Handbook has Contents at the left of the article, nested as its
// headings are and numbered, which stay in view as the page scrolls, the
// section being read marked; an entry pressed, and one reached by the
// keyboard, takes the article to its heading, and the address bar says
// it (`#s/<slug>`). A heading's address opens the article there, read
// again too. The link mark beside a heading, shown as it is pointed at,
// copies its address. Edit beside a heading, reached by the keyboard,
// opens the editor with the cursor at the start of that section; what is
// typed there and saved is read again at that section. On a phone the
// Contents are a box at the top, folded until it is pressed, and an entry
// pressed folds it and takes the article there. Hide, reached by the
// keyboard, hides them, and they stay hidden after the page is read again,
// a small Contents showing them again. Short looks as before: no Contents,
// and nothing beside its headings until they are pointed at. Somebody who
// may only read has the link marks and no Edit. Nothing of the page lies
// over anything else or scrolls sideways. The environment variables are
// those of walkthrough.mjs (`SCHEME`, `LOOK` among them).
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'contents-data');
const home = join(out, 'contents-home');
const looked = `${process.env.LOOK ? '-looked' : ''}${process.env.SCHEME === 'light' ? '-light' : ''}`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const made = async (title, ...more) => JSON.parse((await cli('topic', 'make', title, ...more, '--json')).stdout).id;

// Paragraphs long enough that each section is most of a window.
const words = (what, lines = 9) => Array.from({ length: lines }, (_, nth) => `${what} line ${nth + 1}: what is known of it is written here at some length, so that the page scrolls.`).join(' ');
const HANDBOOK = [
  'The handbook of the shop, read from the top.',
  '',
  '# Getting started',
  words('Getting started'),
  '',
  '## Installing',
  words('Installing'),
  '',
  '## First run',
  words('First run'),
  '',
  '# Rolling back',
  words('Rolling back'),
  '',
  '## Rollback steps',
  words('Rollback steps', 40),
  '',
].join('\n');
const SHORT = ['Two headings only.', '', '# One', 'The first.', '', '# Two', 'The second.', ''].join('\n');

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  const handbook = await made('Handbook', '--body', HANDBOOK);
  const short = await made('Short', '--body', SHORT);

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
  const DESK = { viewport: { width: 1280, height: 860 } };
  const PHONE = { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 };
  /** A page signed in at `to`; one that `reads` is told whoever is signed in holds no `records:write`. */
  const signedIn = async (to, { how = DESK, reads = false } = {}) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    await context.grantPermissions(['clipboard-read', 'clipboard-write'], { origin: url });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    if (reads) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = me.principal.scopes.filter((scope) => !['*', 'records', 'records:write'].includes(scope)).concat(['records:read', 'channels:read', 'inbox:read', 'themes:read', 'themes:write']);
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(to);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  const shown = async (locator, what) => {
    await locator.waitFor({ timeout: 10000 }).catch(() => {
      throw new Error(`expected ${what}`);
    });
  };
  const gone = async (locator, what) => {
    await locator.waitFor({ state: 'detached', timeout: 10000 }).catch(() => {
      throw new Error(`expected ${what}`);
    });
  };
  const laid = async (on, what) => {
    const wrong = await pageCrowded(on, '.topic-page');
    expect(`nothing of the page over anything else ${what}: ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await on.evaluate(() => [document.documentElement, document.querySelector('.topic-page')].map((one) => one.scrollWidth - one.clientWidth));
    expect(`nothing to scroll sideways ${what}, got ${sideways.join(', ')}px`, sideways.every((one) => one <= 0));
  };
  const at = (title, after = '') => `${url}/topic/${encodeURIComponent(title)}${after}`;
  /** Reaches by Tab, from the page's title, what is to be pressed. */
  const tabTo = async (on, locator, what) => {
    await on.locator('.topic-page-title').first().focus();
    for (let times = 0; times < 80; times += 1) {
      await on.keyboard.press('Tab');
      if (await locator.evaluate((one) => one === document.activeElement).catch(() => false)) return;
    }
    throw new Error(`expected ${what} to be reached by Tab`);
  };
  /** Waits until the heading known by `slug` stands at the top of what scrolls, in the article read or in the editor. */
  const atTop = async (on, slug, what) => {
    await on
      .waitForFunction(
        (slug) => {
          const page = document.querySelector('.topic-page')?.closest('.view');
          const read = document.querySelector(`.topic-post [data-section="${slug}"]`);
          const heading = read ?? [...document.querySelectorAll('.topic-post .ProseMirror :is(h1, h2, h3)')].find((one) => one.textContent.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-') === slug);
          if (!page || !heading) return false;
          const from = heading.getBoundingClientRect().top - page.getBoundingClientRect().top;
          return from >= -2 && from <= 40;
        },
        slug,
        { timeout: 10000 },
      )
      .catch(async () => {
        const where = await on.evaluate((slug) => {
          const heading = document.querySelector(`.topic-post [data-section="${slug}"]`) ?? document.querySelector('.topic-post .ProseMirror');
          const page = document.querySelector('.topic-page')?.closest('.view');
          const scrolls = [];
          for (let up = heading?.parentElement; up; up = up.parentElement) if (up.scrollHeight > up.clientHeight + 1) scrolls.push(`${up.tagName.toLowerCase()}.${[...up.classList].join('.')} ${up.scrollTop}/${up.scrollHeight - up.clientHeight}`);
          return `${heading ? Math.round(heading.getBoundingClientRect().top - page.getBoundingClientRect().top) : 'no heading'}px from the top; what scrolls: ${scrolls.join(', ')}`;
        }, slug);
        throw new Error(`expected ${what}: the heading ${slug} at the top of the page, got ${where}`);
      });
  };
  const hashIs = async (on, hash, what) => {
    await on.waitForFunction((hash) => location.hash === hash, hash, { timeout: 10000 }).catch(async () => {
      throw new Error(`expected ${what}: the address to end ${hash}, got ${await on.evaluate(() => location.hash)}`);
    });
  };

  // ── Contents beside the article ─────────────────────────────────────────
  const page = await signedIn(at('Handbook'));
  const article = page.locator(`.topic-page[data-topic="${handbook}"]`);
  const contents = article.locator('nav.contents');
  await shown(contents.locator('.contents-entry').first(), 'Contents on an article with five headings');
  expect('the Contents not folded beside the article', (await contents.locator('.contents-fold').isHidden()) && (await contents.locator('.contents-title').isVisible()));
  const boxes = await page.evaluate(() => [document.querySelector('nav.contents'), document.querySelector('.topic-article-main')].map((one) => one.getBoundingClientRect()));
  expect(`the Contents at the left of the article, got ${JSON.stringify(boxes)}`, boxes[0].right <= boxes[1].left + 1 && boxes[0].top < boxes[1].top + 80);
  const entries = await contents.locator('.contents-entry').evaluateAll((all) => all.map((one) => `${one.closest('.contents-list').parentElement.closest('.contents-list') ? '  ' : ''}${one.innerText.replace(/\s+/g, ' ').trim()}`));
  expect(
    `the headings nested and numbered, got ${JSON.stringify(entries)}`,
    JSON.stringify(entries) === JSON.stringify(['1 Getting started', '  1.1 Installing', '  1.2 First run', '2 Rolling back', '  2.1 Rollback steps']),
  );
  await laid(page, 'with the Contents beside the article');
  await page.screenshot({ path: `${out}/contents-01-beside${looked}.png` });

  // An entry pressed takes the article there, the address says it, and it is marked; the Contents stay in view.
  await contents.locator('.contents-entry', { hasText: 'Rollback steps' }).click();
  await atTop(page, 'rollback-steps', 'Rollback steps pressed');
  await hashIs(page, '#s/rollback-steps', 'Rollback steps pressed');
  await shown(contents.locator('.contents-entry[aria-current="location"]', { hasText: 'Rollback steps' }), 'the section being read marked');
  const stays = await contents.evaluate((one) => {
    const box = one.getBoundingClientRect();
    return box.top >= document.querySelector('.topic-page')?.closest('.view').getBoundingClientRect().top - 1 && box.bottom > 0;
  });
  expect('the Contents in view as the page is scrolled', stays);
  await laid(page, 'scrolled to a section');
  await page.screenshot({ path: `${out}/contents-02-section${looked}.png` });

  // And one reached by the keyboard.
  const installing = contents.locator('.contents-entry', { hasText: 'Installing' });
  await tabTo(page, installing, 'the Installing entry');
  await page.keyboard.press('Enter');
  await atTop(page, 'installing', 'Installing reached by the keyboard');
  await hashIs(page, '#s/installing', 'Installing reached by the keyboard');
  await shown(contents.locator('.contents-entry[aria-current="location"]', { hasText: 'Installing' }), 'Installing marked as read');

  // ── A heading's address ─────────────────────────────────────────────────
  await page.goto(at('Handbook', '#s/first-run'));
  await atTop(page, 'first-run', 'the article opened by `#s/first-run`');
  await page.reload();
  await atTop(page, 'first-run', 'the same address read again');
  // The link mark beside a heading, shown as it is pointed at, copies its address.
  const heading = article.locator('.prose-heading-line[data-section="rolling-back"]');
  const mark = heading.locator('a.prose-heading-mark');
  expect('the link mark not shown until the heading is pointed at', Number(await mark.evaluate((one) => getComputedStyle(one.parentElement).opacity)) === 0);
  await heading.hover();
  await page.waitForFunction(() => Number(getComputedStyle(document.querySelector('[data-section="rolling-back"] .prose-heading-acts')).opacity) === 1, null, { timeout: 5000 });
  expect('the link mark named for what it does', (await mark.getAttribute('aria-label')) === 'Copy link to Rolling back');
  await mark.click();
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(`its address copied, got ${copied}`, copied === at('Handbook', '#s/rolling-back'));
  await hashIs(page, '#s/rolling-back', 'the link mark pressed');
  await page.screenshot({ path: `${out}/contents-03-link${looked}.png` });

  // ── Edit a section ──────────────────────────────────────────────────────
  const editFirstRun = article.getByRole('button', { name: 'Edit section First run' });
  await tabTo(page, editFirstRun, 'Edit beside First run');
  await page.keyboard.press('Enter');
  await shown(article.locator('.topic-post .ProseMirror'), 'the editor once Edit beside a heading is pressed');
  await page
    .waitForFunction(
      () => {
        const selection = document.getSelection();
        const block = selection?.anchorNode && (selection.anchorNode.nodeType === 1 ? selection.anchorNode : selection.anchorNode.parentElement)?.closest('.ProseMirror > *');
        return Boolean(document.activeElement?.closest('.topic-post .ProseMirror')) && selection.anchorOffset === 0 && block?.previousElementSibling?.textContent.trim() === 'First run' && block.textContent.startsWith('First run line 1');
      },
      null,
      { timeout: 10000 },
    )
    .catch(() => {
      throw new Error('expected the cursor at the start of the First run section');
    });
  await atTop(page, 'first-run', 'the editor opened at First run');
  await page.screenshot({ path: `${out}/contents-04-editing${looked}.png` });
  await page.keyboard.type('Typed in this section. ');
  await article.locator('.topic-doc-acts').getByRole('button', { name: 'Save', exact: true }).click();
  await gone(article.locator('.topic-post .ProseMirror'), 'the editor closed once saved');
  const firstRun = article.locator('.prose-heading-line[data-section="first-run"] + .prose-text');
  await shown(firstRun.filter({ hasText: /^Typed in this section\. First run line 1/ }), 'what was typed at the start of the section, read');
  await atTop(page, 'first-run', 'the article read again at First run once saved');
  await laid(page, 'read again at a section');

  // ── A phone ─────────────────────────────────────────────────────────────
  const phone = await signedIn(at('Handbook'), { how: PHONE });
  const box = phone.locator('nav.contents');
  await shown(box.locator('.contents-fold'), 'the Contents as a box on a phone');
  const above = await phone.evaluate(() => [document.querySelector('nav.contents'), document.querySelector('.topic-article-main')].map((one) => one.getBoundingClientRect()));
  expect(`the box at the top of the article, got ${JSON.stringify(above)}`, above[0].bottom <= above[1].top + 1);
  expect('folded until pressed', (await box.locator('.contents-fold').getAttribute('aria-expanded')) === 'false' && !(await box.locator('.contents-entry').first().isVisible()));
  await laid(phone, "with the Contents folded at a phone's width");
  await phone.screenshot({ path: `${out}/contents-05-phone-folded${looked}.png` });
  await box.locator('.contents-fold').tap();
  await shown(box.locator('.contents-entry', { hasText: 'Rolling back' }), 'the entries once the box is pressed');
  await laid(phone, "with the Contents open at a phone's width");
  await phone.screenshot({ path: `${out}/contents-06-phone-open${looked}.png` });
  await box.locator('.contents-entry', { hasText: 'Rolling back' }).first().tap();
  await atTop(phone, 'rolling-back', 'Rolling back pressed on a phone');
  expect('the box folded again', !(await box.locator('.contents-entry').first().isVisible()));
  await laid(phone, "at a section at a phone's width");
  await phone.screenshot({ path: `${out}/contents-07-phone-section${looked}.png` });

  // ── Hide ────────────────────────────────────────────────────────────────
  await page.goto(at('Handbook'));
  const hide = contents.getByRole('button', { name: 'Hide contents' });
  await shown(hide, 'Hide in the Contents');
  await tabTo(page, hide, 'Hide');
  await page.keyboard.press('Enter');
  const show = article.locator('.contents.is-hidden button.contents-show');
  await shown(show, 'a small Contents once they are hidden');
  expect('no entries once hidden', (await article.locator('.contents-entry').count()) === 0);
  await laid(page, 'with the Contents hidden');
  await page.screenshot({ path: `${out}/contents-08-hidden${looked}.png` });
  await page.reload();
  await shown(article.locator('.prose-heading-line').first(), 'the article read again');
  await shown(show, 'the Contents still hidden once the page is read again');
  expect('no entries after reading again', (await article.locator('.contents-entry').count()) === 0);
  await show.click();
  await shown(contents.locator('.contents-entry').first(), 'the Contents shown again');

  // ── Fewer than three headings, and somebody who may only read ───────────
  await page.goto(at('Short'));
  const few = page.locator(`.topic-page[data-topic="${short}"]`);
  await shown(few.locator('.prose-heading-line[data-section="two"]'), 'the short article read');
  expect('no Contents with two headings', (await few.locator('.contents').count()) === 0);
  expect('nothing shown beside its headings at rest', Number(await few.locator('.prose-heading-acts').first().evaluate((one) => getComputedStyle(one).opacity)) === 0);
  await page.mouse.move(0, 0);
  await page.screenshot({ path: `${out}/contents-09-short${looked}.png` });

  const reader = await signedIn(at('Handbook'), { reads: true });
  const read = reader.locator(`.topic-page[data-topic="${handbook}"]`);
  await shown(read.locator('.contents-entry').first(), 'the Contents for somebody who may only read');
  expect('link marks for them', (await read.locator('a.prose-heading-mark').count()) === 5);
  expect('no Edit beside a heading for them', (await read.locator('.prose-heading-edit').count()) === 0);

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
