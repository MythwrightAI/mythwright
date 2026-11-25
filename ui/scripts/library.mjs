// A project's library: its documents by folder, one opened to be written
// in, one made, a file put there by another program showing by itself,
// and the library read as pages. And comments: a mark on a document's
// line that says how many, kept current as one is said from a terminal;
// the document opened by it with the panel beside it on its comments
// (`views/doctools.tsx`), opened and put away by the speech bubble right
// after the gear of its head; one added there; what was half written kept;
// every one listed with the newest in sight; its Details and its History,
// the last opened from the gear too; the panel kept by its tab and made
// wider by its edge; the mark pressed for a document already showing
// beside its library; the panel having the screen at a phone's width; and
// a file that is no document, which has none of it.
//
//   DAEMON=../target/debug/factory node scripts/library.mjs [out-dir]
//
// Shop has a folder of documents and `std/docs` applied to it; Depot has
// neither, and no Docs line. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { docToolsAtRest } from './drawer.mjs';
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
const data = join(out, 'library-data');
const home = join(out, 'library-home');

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
  for (const name of ['Shop', 'Depot']) await cli('project', 'create', '--bare', name);
  const docs = join(data, 'projects', 'shop', 'docs');
  // Only what this check puts there: a new project has a page or two of its own.
  await rm(docs, { recursive: true, force: true });
  await mkdir(join(docs, 'guides'), { recursive: true });
  await writeFile(join(docs, 'welcome.md'), '# Welcome\n\nStart with [installing it](guides/install.md).\n');
  await writeFile(join(docs, 'guides', 'install.md'), '# Installing it\n\nRun the installer, then come [back](../welcome.md).\n\n| Step | What happens |\n|---|---|\n| One | It is unpacked |\n| Two | It is started |\n');
  await writeFile(join(docs, 'guides', 'returns.md'), 'Thirty days, with no heading.\n');
  await cli('apply', 'std/docs', '--project', 'shop');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
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
  await side(page, 'Resources');
  const nav = page.locator('.nav');
  await nav.getByText('Depot', { exact: true }).waitFor();

  // Docs is under the project that has a library, and not under the one that has none.
  const project = (name) => nav.locator(':scope > .tree > [data-project]', { has: page.locator(':scope > .nav-row', { hasText: name }) });
  await project('Shop').getByRole('button', { name: / Docs$/ }).waitFor();
  // Depot is drawn too: what it lacks is not lacking because nothing of it is there yet.
  // Nobody said a folder for either, so neither lists any Repo.
  await project('Depot').locator(':scope > .nav-row').waitFor();
  expect('Depot, which has no library, to have no Docs line', (await project('Depot').getByRole('button', { name: / Docs$/ }).count()) === 0);
  expect('neither project, each in a folder the factory made, to list any Repo', (await nav.getByRole('button', { name: 'Repo', exact: true }).count()) === 0);
  await project('Shop').getByRole('button', { name: / Docs$/ }).click();
  await page.getByRole('heading', { name: 'Shop docs', level: 1 }).waitFor();

  // The documents by folder: the library's own first, then each folder.
  const lines = page.locator('.library-line');
  await lines.nth(2).waitFor();
  const shelves = await page.locator('.library-shelf').evaluateAll((all) =>
    all.map((shelf) => [shelf.querySelector('h2')?.textContent ?? '', [...shelf.querySelectorAll('.rows-line > .truncate:first-child')].map((one) => one.textContent)]),
  );
  expect(
    `the library's own documents first and then its folder's, got ${JSON.stringify(shelves)}`,
    JSON.stringify(shelves) === JSON.stringify([['', ['Welcome']], ['guides/', ['Installing it', 'returns']]]),
  );
  await page.screenshot({ path: `${out}/library-0-listed.png` });

  // A file another program puts in the folder is there without the page being read again.
  await writeFile(join(docs, 'guides', 'later.md'), '# Written by something else\n');
  await page.locator('.library-line', { hasText: 'Written by something else' }).waitFor();

  // ---- comments ---------------------------------------------------------------
  // A document nothing was said on shows no count. Its mark is in the line
  // all the same, in room of its own, and is drawn while the line is
  // pointed at or the mark is reached by keyboard: nothing in the line moves.
  const line = (title) => page.locator('.library-line', { hasText: title });
  const mark = (title) => line(title).locator('.comment-mark');
  const drawnAt = (title) => mark(title).evaluate((one) => getComputedStyle(one).opacity);
  const placed = async (title) => JSON.stringify([await line(title).locator('.rows-line').boundingBox(), await mark(title).boundingBox()]);
  await page.mouse.move(2, 2);
  await page.getByRole('button', { name: 'Comment on Welcome', exact: true }).waitFor();
  expect('a document nothing was said on to show no count', (await mark('Welcome').locator('.comment-count').count()) === 0);
  expect(`its mark not to be drawn while nothing points at its line, and it is at ${await drawnAt('Welcome')}`, (await drawnAt('Welcome')) === '0');
  const atRest = await placed('Welcome');
  await line('Welcome').locator('.rows-line').hover();
  expect(`its mark to be drawn while its line is pointed at, and it is at ${await drawnAt('Welcome')}`, (await drawnAt('Welcome')) === '1');
  expect('nothing in the line to move for the mark being drawn', (await placed('Welcome')) === atRest);
  await page.screenshot({ path: `${out}/library-1-pointed.png` });
  await page.mouse.move(2, 2);
  await line('Welcome').locator('.rows-line').focus();
  await page.keyboard.press('Tab');
  expect(
    'the mark to be the next thing the keyboard reaches after its line, and drawn then',
    (await page.evaluate(() => document.activeElement?.getAttribute('aria-label'))) === 'Comment on Welcome' && (await drawnAt('Welcome')) === '1',
  );
  expect('nothing in the line to move for the keyboard reaching the mark', (await placed('Welcome')) === atRest);
  await page.evaluate(() => document.activeElement?.blur());

  // Said on from a terminal, its line says how many, without the page being read again.
  await page.evaluate(() => (window.sameReading = true));
  await cli('record', 'comment', 'shop-docs:welcome', 'Does this cover Windows?');
  await page.getByRole('button', { name: '1 comment on Welcome', exact: true }).waitFor();
  expect('the count to be drawn at rest', (await drawnAt('Welcome')) === '1' && (await mark('Welcome').locator('.comment-count').innerText()) === '1');
  expect('the others to say nothing still', (await page.locator('.comment-mark.has').count()) === 1);
  expect('the page not to have been read again for it', await page.evaluate(() => window.sameReading === true));
  expect('the line to be where it was, with its count', (await placed('Welcome')) === atRest);
  await page.screenshot({ path: `${out}/library-1-counted.png` });

  /** What lies over something else among `boxes`, by the boxes themselves: each is `[what, box]`. */
  const over = (boxes) => {
    const found = [];
    for (let a = 0; a < boxes.length; a += 1) {
      for (let b = a + 1; b < boxes.length; b += 1) {
        const [first, one] = boxes[a];
        const [second, other] = boxes[b];
        const across = Math.min(one.x + one.width, other.x + other.width) - Math.max(one.x, other.x);
        const down = Math.min(one.y + one.height, other.y + other.height) - Math.max(one.y, other.y);
        if (across > 0.5 && down > 0.5) found.push(`${first} lies over ${second}`);
      }
    }
    return found;
  };
  /**
   * Holds a document's page with the panel beside it to nothing lying over anything else and nothing scrolling
   * sideways: the head over the text and the panel, the panel beside the text, its comments one under another down
   * to the line one is added by, and the speech bubble that opens it right after the gear, the last of the head's
   * row and on one line with it. On a phone the panel has the screen, and is held to being in the window.
   */
  const laidOut = async (on, where) => {
    const found = await on.evaluate(() => {
      const box = (one) => {
        const { x, y, width, height } = one.getBoundingClientRect();
        return { x, y, width, height };
      };
      const all = (selector, what) => [...document.querySelectorAll(selector)].filter((one) => one.getClientRects().length > 0).map((one, index) => [`${what} ${index + 1}`, box(one)]);
      const wide = [];
      if (document.documentElement.scrollWidth > window.innerWidth) wide.push(`the page is ${document.documentElement.scrollWidth} wide in ${window.innerWidth}`);
      for (const one of document.querySelectorAll('.view, .doc-body, .doc-tools, .doc-tools-body, .comment-add, .rows-page, .drawer-body')) {
        if (one.scrollWidth > one.clientWidth + 1) wide.push(`.${[...one.classList].join('.')} is ${one.scrollWidth} wide in ${one.clientWidth}`);
      }
      // What is said scrolls within the panel: a comment is on the page as far as the panel shows it.
      const body = document.querySelector('.doc-tools-body')?.getBoundingClientRect();
      const shown = ([what, at]) => {
        if (!body) return [what, at];
        const top = Math.max(at.y, body.top);
        return [what, { ...at, y: top, height: Math.min(at.y + at.height, body.bottom) - top }];
      };
      const acts = [...(document.querySelector('.view .head-acts')?.children ?? [])].filter((one) => one.getClientRects().length > 0).map((one) => ({ is: one.getAttribute('aria-label') ?? one.className, ...box(one) }));
      const panel = document.querySelector('.doc-tools');
      const phone = window.innerWidth <= 760;
      return {
        wide,
        phone,
        acts,
        panel: panel ? box(panel) : null,
        room: window.innerWidth,
        strip: [...all('.doc-tools .doc-comments > .comment-list > .comment', 'comment').map(shown).filter(([, at]) => at.height > 0.5), ...all('.doc-tools .doc-comments > .comment-add > *', 'what a comment is added by')],
        head: phone && panel ? [] : all('.view > .split > .view-head > *', 'what the head holds'),
        parts: phone && panel ? [] : [...all('.view > .split > .view-head', 'the head'), ...all('.doc-with > .doc-body', 'the text'), ...all('.doc-with > .doc-tools', 'the panel')],
      };
    });
    const adding = found.strip.filter(([what]) => what.startsWith('what a comment is added by'));
    const comments = found.strip.filter(([what]) => what.startsWith('comment'));
    const wrong = [...found.wide, ...over(comments), ...over([...comments.slice(-1), ...adding]), ...over(found.head), ...over(found.parts)];
    expect(`nothing of a document's page to lie over anything else or scroll sideways ${where}, but: ${wrong.join('; ')}`, wrong.length === 0);
    if (found.panel) expect(`the panel inside the window ${where}, at ${JSON.stringify(found.panel)}`, found.panel.x >= -0.5 && found.panel.x + found.panel.width <= found.room + 0.5);
    if (found.phone && found.panel) return;
    const [tool, gear] = [found.acts.at(-1), found.acts.at(-2)];
    expect(
      `the speech bubble right after the gear, the last of the head's row and on one line with it ${where}, and the row is ${JSON.stringify(found.acts)}`,
      tool && gear && /^Comments and details/.test(tool.is) && gear.is === 'More for this page' && tool.x >= gear.x + gear.width - 0.5 && Math.abs(tool.y + tool.height / 2 - (gear.y + gear.height / 2)) <= 1,
    );
  };

  // The mark opens the document with the panel beside it on its comments, and the cursor where a comment is added.
  await mark('Welcome').click();
  await page.locator('.crumbs h1', { hasText: 'welcome.md' }).waitFor();
  const tools = docToolsAtRest(page);
  const comments = tools.getByRole('region', { name: 'Comments' });
  const adding = comments.getByLabel('Add a comment');
  const bubble = page.locator('.view-head .head-acts .head-tool');
  await comments.locator('.comment-words', { hasText: 'Does this cover Windows?' }).waitFor();
  expect('the comment to say who said it', (await comments.locator('.comment-who').innerText()) === 'Ada');
  expect('Comments to be the tab showing', (await tools.getByRole('tab', { name: 'Comments', exact: true }).getAttribute('aria-selected')) === 'true');
  expect(`the bubble to be pressed and to say how many, got "${await bubble.getAttribute('aria-label')}"`, (await bubble.getAttribute('aria-pressed')) === 'true' && (await bubble.getAttribute('aria-label')) === 'Comments and details, 1 comment');
  expect('nothing said on it over its text', (await page.locator('.doc-body .comment').count()) === 0);
  await page.locator('.writer .ProseMirror', { hasText: 'Start with' }).waitFor();
  expect('the cursor to be where a comment is added', (await page.evaluate(() => document.activeElement?.getAttribute('aria-label'))) === 'Add a comment');
  await laidOut(page, 'at the usual width');
  await page.screenshot({ path: `${out}/library-1-comments.png` });

  // One added there is there at once, the newest last, and the line is empty again.
  await adding.fill('It does now.');
  await adding.press('Enter');
  await comments.locator('.comment-words', { hasText: 'It does now.' }).waitFor();
  expect(
    `the comments to be the two, oldest first, got ${await comments.locator('.comment-words').allInnerTexts()}`,
    (await comments.locator('.comment-words').allInnerTexts()).join('|') === 'Does this cover Windows?|It does now.',
  );
  // The line is emptied when the daemon has answered, which may be after the comment is shown: it is waited for.
  await page.waitForFunction(() => document.querySelector('.doc-tools textarea')?.value === '');
  await page.waitForFunction(() => document.querySelector('.view-head .head-tool')?.getAttribute('aria-label') === 'Comments and details, 2 comments');
  // One marked public, said from a terminal, says so; nothing here offers the mark, as nothing did.
  await cli('record', 'comment', 'shop-docs:welcome', '--public', 'Told to whoever asked.');
  await comments.locator('.comment', { hasText: 'Told to whoever asked.' }).locator('.journey-public').waitFor();
  expect('only what was marked public to say so', (await comments.locator('.journey-public').count()) === 1);
  expect('no switch for the mark where a comment is added', (await comments.locator('input[type=checkbox]').count()) === 0);

  // Half a comment, and another tab shown: the panel is there again on its comments, and what was half written was kept.
  await adding.fill('Half a thought');
  await page.getByRole('tab', { name: /docs/i }).first().click();
  await page.getByRole('button', { name: '3 comments on Welcome', exact: true }).waitFor();
  await page.getByRole('tab', { name: /welcome\.md/ }).first().click();
  await comments.locator('.comment-words', { hasText: 'Told to whoever asked.' }).waitFor();
  expect('what was said to be there again when its tab is shown again', (await comments.locator('.comment').count()) === 3);
  expect(`what was half written to have been kept, got "${await adding.inputValue()}"`, (await adding.inputValue()) === 'Half a thought');
  await adding.fill('');

  // More than a few: every one is listed, the newest in sight by the line one is added by.
  for (const words of ['Then say so in the heading.', 'Done: see the first line.', 'A sixth thing,\nsaid over two lines, and long enough that it has to go on to the next line at any width a page is read at.']) {
    await cli('record', 'comment', 'shop-docs:welcome', words);
  }
  await comments.locator('.comment-words', { hasText: 'A sixth thing' }).waitFor();
  expect('every comment to be listed', (await comments.locator('.comment').count()) === 6);
  const [newest, panelBody] = [await comments.locator('.comment').last().boundingBox(), await tools.locator('.doc-tools-body').boundingBox()];
  expect(`the newest to be in sight, at ${JSON.stringify(newest)} in ${JSON.stringify(panelBody)}`, newest.y >= panelBody.y - 0.5 && newest.y + newest.height <= panelBody.y + panelBody.height + 0.5);
  await laidOut(page, 'with every comment listed');
  // Read as its source and as its page, the panel stays beside it.
  for (const [mode, shows] of [['Source', '.cm-content'], ['Page', 'iframe.page-frame'], ['Write', '.writer .ProseMirror']]) {
    await page.locator('.view-head').getByRole('tab', { name: mode, exact: true }).click();
    await page.locator(shows).waitFor();
    await comments.locator('.comment-words', { hasText: 'A sixth thing' }).waitFor();
    await laidOut(page, `read as ${mode}`);
    if (mode === 'Source') await page.screenshot({ path: `${out}/library-1-source.png` });
  }
  // And where the tab is narrower: the text gives way to the panel and keeps its share.
  await page.setViewportSize({ width: 1060, height: 860 });
  for (const [mode, shows] of [['Page', 'iframe.page-frame'], ['Source', '.cm-content'], ['Write', '.writer .ProseMirror']]) {
    await page.locator('.view-head').getByRole('tab', { name: mode, exact: true }).click();
    await page.locator(shows).waitFor();
    await laidOut(page, `read as ${mode}, in a narrower tab`);
  }
  await page.screenshot({ path: `${out}/library-1-narrower.png` });
  await page.setViewportSize({ width: 1280, height: 860 });

  // Its edge, reached by the keyboard, makes it wider; and it keeps that.
  const edge = tools.getByRole('separator');
  const was = (await tools.boundingBox()).width;
  await edge.focus();
  await page.keyboard.press('ArrowLeft');
  await page.waitForFunction((before) => document.querySelector('.doc-tools').getBoundingClientRect().width > before + 20, was);
  await laidOut(page, 'made wider by its edge');

  // Details: what the document is.
  await tools.getByRole('tab', { name: 'Details', exact: true }).click();
  const details = tools.getByRole('region', { name: 'Details' });
  await details.locator('.doc-detail', { hasText: 'Comments' }).waitFor();
  // Whether it is marked public is read from its record as Details is shown, and says Reading until it is.
  await details.locator('.doc-detail', { hasText: /^Public(Yes|No)/ }).waitFor();
  const said = await details.locator('.doc-detail').evaluateAll((all) => all.map((one) => `${one.querySelector('dt').textContent}: ${one.querySelector('dd').textContent}`));
  expect(
    `Details to say where it is, its library, how big, when it was changed, that it is not public, and how many comments, got ${said.join(' | ')}`,
    said[0] === 'Path: docs/welcome.md' && said[1] === 'Library: Docs' && /^Size: (.* · )?3 lines$/.test(said[2]) && said.some((one) => /^Changed: Last edited/.test(one)) && said.includes('Public: No') && said.at(-1) === 'Comments: 6',
  );
  await laidOut(page, 'on its Details');
  await page.screenshot({ path: `${out}/library-1-details.png` });
  // And the way from there to its history, which is everything that happened to the document, in the order it did.
  await details.getByRole('button', { name: 'Its history', exact: true }).click();
  await tools.locator('.journey-words', { hasText: 'Does this cover Windows?' }).waitFor();
  expect('History to be the tab showing', (await tools.getByRole('tab', { name: 'History', exact: true }).getAttribute('aria-selected')) === 'true');
  expect('the history to hold everything said', (await tools.locator('.journey-spoken').count()) === 6);
  await laidOut(page, 'on its History');
  await page.screenshot({ path: `${out}/library-1-history.png` });

  // The bubble puts it away, and the text has the page again; the tab shown again is as it was left, and the bubble brings it back on the tab it was on.
  await bubble.click();
  await page.locator('.doc-tools').waitFor({ state: 'detached' });
  await page.locator('.doc-tools-leaving').waitFor({ state: 'detached' });
  expect('the bubble not pressed once it is put away', (await bubble.getAttribute('aria-pressed')) === 'false');
  const text = await page.locator('.doc-with > .doc-body').boundingBox();
  const all = await page.locator('.doc-with').boundingBox();
  expect(`the text as wide as the page once it is put away, at ${JSON.stringify(text)} in ${JSON.stringify(all)}`, Math.abs(text.width - all.width) <= 1);
  await page.getByRole('tab', { name: /docs/i }).first().click();
  await page.getByRole('tab', { name: /welcome\.md/ }).first().click();
  await page.locator('.writer .ProseMirror', { hasText: 'Start with' }).waitFor();
  expect('the panel still put away when its tab is shown again', (await page.locator('.doc-tools').count()) === 0);
  await bubble.click();
  await tools.locator('.journey-words', { hasText: 'Does this cover Windows?' }).waitFor();
  // History is still under the gear of the head, and opens the panel on it.
  await tools.getByRole('tab', { name: 'Comments', exact: true }).click();
  const gear = page.locator('.view-head').getByRole('button', { name: 'More for this page', exact: true });
  const history = async () => {
    await gear.click();
    await page.getByRole('menu').getByRole('menuitem', { name: 'History', exact: true }).click();
  };
  await history();
  await page.waitForFunction(() => document.querySelector('.doc-tools [role="tab"][aria-selected="true"]')?.textContent === 'History');
  await tools.getByRole('tab', { name: 'Comments', exact: true }).click();

  // The document already showing, beside its library: the mark is taken as it is pressed, with nothing opened anew.
  await page.getByRole('button', { name: 'Split', exact: true }).click();
  await page.getByRole('tab', { name: /docs/i }).first().click();
  await page.getByRole('button', { name: '6 comments on Welcome', exact: true }).waitFor();
  await comments.locator('.comment-words', { hasText: 'A sixth thing' }).waitFor();
  expect('the cursor to be nowhere in the document before its mark is pressed', (await page.evaluate(() => document.activeElement?.getAttribute('aria-label'))) !== 'Add a comment');
  await mark('Welcome').click();
  await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Add a comment');
  expect('every comment of a document that was already showing to be shown when its mark is pressed', (await comments.locator('.comment').count()) === 6);
  await page.screenshot({ path: `${out}/library-1-beside.png` });
  // One group again, and the document opened by its mark as before.
  await page.getByRole('tab', { name: /welcome\.md/ }).getByLabel('Close welcome.md').click();
  await page.locator('.crumbs h1', { hasText: 'welcome.md' }).waitFor({ state: 'detached' });
  await mark('Welcome').click();
  await comments.locator('.comment-words', { hasText: 'A sixth thing' }).waitFor();
  await page.locator('.writer .ProseMirror', { hasText: 'Start with' }).waitFor();

  // A phone: the same page at 390 wide, touched. The panel has the screen, with the way back at its head.
  const phone = await browser.newContext({ storageState: await context.storageState(), viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: process.env.SCHEME ?? 'dark' });
  const small = await phone.newPage();
  small.on('pageerror', (err) => problems.push(`at a phone's width: ${err.message}`));
  await small.goto(url);
  await small.locator('.writer .ProseMirror', { hasText: 'Start with' }).waitFor();
  await laidOut(small, "at a phone's width");
  await small.locator('.view-head .head-acts .head-tool').tap();
  const smallComments = docToolsAtRest(small).getByRole('region', { name: 'Comments' });
  await smallComments.locator('.comment-words', { hasText: 'A sixth thing' }).waitFor();
  await laidOut(small, "at a phone's width, with the panel open");
  await small.screenshot({ path: `${out}/library-1-phone-comments.png` });
  await smallComments.getByLabel('Add a comment').tap();
  await small.keyboard.type('From a phone.');
  await smallComments.getByRole('button', { name: 'Comment', exact: true }).tap();
  await smallComments.locator('.comment-words', { hasText: 'From a phone.' }).waitFor();
  await laidOut(small, "at a phone's width, once one was added there");
  await small.getByRole('button', { name: 'Back to the document', exact: true }).tap();
  await small.locator('.doc-tools').waitFor({ state: 'detached' });
  await small.locator('.doc-tools-leaving').waitFor({ state: 'detached' });
  // And the library's list there: every mark is drawn, since nothing points at a line, and each is on the screen.
  await small.getByRole('tab', { name: /docs/i }).first().tap();
  await small.getByRole('button', { name: '7 comments on Welcome', exact: true }).waitFor();
  const marks = await small.locator('.comment-mark').evaluateAll((all) => all.map((one) => ({ drawn: getComputedStyle(one).opacity, right: one.getBoundingClientRect().right, wide: window.innerWidth })));
  expect(`every mark to be drawn and on the screen at a phone's width, got ${JSON.stringify(marks)}`, marks.length >= 3 && marks.every((one) => one.drawn === '1' && one.right <= one.wide + 0.5));
  const smallLines = await small.locator('.library-line').evaluateAll((all) => all.map((one, index) => [`line ${index + 1}`, { x: one.getBoundingClientRect().x, y: one.getBoundingClientRect().y, width: one.getBoundingClientRect().width, height: one.getBoundingClientRect().height }]));
  expect(`no line of the list to lie over another at a phone's width, but: ${over(smallLines).join('; ')}`, over(smallLines).length === 0);
  const smallWide = await small.evaluate(() => [document.documentElement.scrollWidth, window.innerWidth, ...[...document.querySelectorAll('.view, .rows-page')].flatMap((one) => [one.scrollWidth, one.clientWidth + 1])]);
  expect(`nothing of the list to scroll sideways at a phone's width, got ${smallWide}`, smallWide.every((wide, index) => index % 2 === 1 || wide <= smallWide[index + 1]));
  await small.screenshot({ path: `${out}/library-1-phone-listed.png` });
  await phone.close();
  // What was added on the phone is on this page too, with nothing read again.
  await comments.locator('.comment-words', { hasText: 'From a phone.' }).waitFor();

  // A file that is no library's document has none of it. The project's files are reached from its own
  // page: no Repo is listed under a project nobody said a folder for.
  await project('Shop').locator(':scope > .nav-row .nav-item').click();
  await page.getByRole('button', { name: 'Its repo', exact: true }).click();
  await page.getByRole('heading', { name: 'Shop repo', level: 1 }).waitFor();
  await page.locator('.files tbody').getByText('README.md').first().click();
  await page.locator('.crumbs h1', { hasText: 'README.md' }).waitFor();
  await page.locator('.view-head').getByRole('tab', { name: 'Write', exact: true }).waitFor();
  expect('a file that is no document to have no comments and no bubble to open them', (await page.getByRole('region', { name: 'Comments' }).count()) === 0 && (await page.getByLabel('Add a comment').count()) === 0 && (await bubble.count()) === 0);
  await gear.click();
  await page.getByRole('menu').getByRole('menuitem', { name: 'Copy path', exact: true }).waitFor();
  expect('and no history under its gear', (await page.getByRole('menu').getByRole('menuitem', { name: 'History', exact: true }).count()) === 0);
  await page.keyboard.press('Escape');
  await page.getByRole('menu').waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/library-1-no-document.png` });
  await page.getByRole('tab', { name: /docs/i }).first().click();
  await page.getByRole('heading', { name: 'Shop docs', level: 1 }).waitFor();

  // The same documents laid out another way: a view is saved beside the
  // list, a tab each, as a board's are.
  const views = page.getByRole('tablist', { name: 'Views' });
  const tabs = async () => (await views.getByRole('tab').allInnerTexts()).map((one) => one.trim());
  expect(`a library to start as its list, got ${await tabs()}`, (await tabs()).join() === 'Library');
  // The ways it can be laid out are offered in one order, whichever it is
  // laid out as: the one chosen used to come to the front, from under the pointer.
  await page.getByRole('button', { name: /^Layout/ }).click();
  const ways = page.getByRole('dialog', { name: 'Layout' }).getByRole('radio');
  const offered = async () => (await ways.allInnerTexts()).map((one) => one.trim()).join();
  const chosen = async () => (await page.getByRole('dialog', { name: 'Layout' }).getByRole('radio', { checked: true }).innerText()).trim();
  const asList = await offered();
  expect(`a library's own look to be its list, got ${await chosen()} of ${asList}`, (await chosen()) === 'List' && asList === 'Table,List,Gallery,Calendar');
  await ways.filter({ hasText: 'Table' }).click();
  await page.locator('.grid thead th').first().waitFor();
  expect(`the layouts to be offered in the order they were once another is chosen, got ${await offered()} with ${await chosen()} chosen`, (await offered()) === asList && (await chosen()) === 'Table');
  await page.screenshot({ path: `${out}/library-2-layouts.png` });
  await ways.filter({ hasText: 'List' }).click();
  await page.locator('.library-line', { hasText: 'Installing it' }).waitFor();
  expect(`and again laid out as it was, got ${await offered()}`, (await offered()) === asList && (await chosen()) === 'List');
  await page.keyboard.press('Escape');
  await page.getByRole('dialog', { name: 'Layout' }).waitFor({ state: 'detached' });
  const addView = async (name) => {
    await page.getByRole('button', { name: 'Add a view' }).click();
    await page.getByRole('menuitem', { name: new RegExp(`^${name}`) }).click();
    await views.getByRole('tab', { name, exact: true, selected: true }).waitFor();
  };
  // The library is on the page again, and is read again: its documents are waited for before a view of them is.
  await page.locator('.library-line', { hasText: 'Installing it' }).waitFor();
  await addView('Table');
  await page.locator('.grid thead th').first().waitFor();
  const heads = (await page.locator('.grid thead th[data-column]').allInnerTexts()).map((one) => one.trim());
  expect(`the documents as a table to begin with what each is called, got ${heads.join()}`, heads[0] === 'Title' && heads.includes('Tags'));
  expect('every document to be a row of it', (await page.locator('.grid tbody tr').count()) >= 3);
  await page.screenshot({ path: `${out}/library-5-as-table.png` });
  await addView('Calendar');
  await page.screenshot({ path: `${out}/library-6-as-calendar.png` });
  await addView('Gallery');
  await page.screenshot({ path: `${out}/library-7-as-gallery.png` });
  expect(`the views to be kept as tabs, got ${await tabs()}`, (await tabs()).join() === 'Library,Table,Calendar,Gallery');
  // They are the library's, and there for whoever opens it next.
  const kept = JSON.parse((await cli('record', 'list', 'shop', 'docs', '--json')).stdout);
  void kept;
  // And the list is as it was.
  await views.getByRole('tab', { name: 'Library', exact: true }).click();
  await page.locator('.library-line', { hasText: 'Installing it' }).waitFor();

  // A new document is a file where its title and folder say, and opens to be written in.
  await page.getByRole('button', { name: 'New document', exact: true }).click();
  await page.getByLabel('Title').fill('Release notes');
  await page.getByLabel('Folder').fill('guides');
  await page.getByRole('button', { name: 'Make it', exact: true }).click();
  await page.locator('.crumbs h1', { hasText: 'release-notes.md' }).waitFor();
  expect('the new document to be a file with its title as its heading', (await readFile(join(docs, 'guides', 'release-notes.md'), 'utf8')) === '# Release notes\n');
  // Nothing was said on it yet: the panel beside it, opened by the bubble, says so over the line a comment is added by.
  expect('no panel beside a document opened anew', (await page.locator('.doc-tools').count()) === 0);
  await bubble.click();
  await adding.waitFor();
  expect('a document nothing was said on to have no comment listed', (await comments.locator('.comment').count()) === 0);
  await comments.getByText('Nobody has commented on it yet', { exact: false }).waitFor();
  await laidOut(page, 'with nothing said');
  await page.screenshot({ path: `${out}/library-2-opened.png` });
  // What happened to it is in its history: that it was written, and by whom.
  await history();
  await tools.locator('.journey-said', { hasText: 'Ada wrote it' }).waitFor();
  expect('nothing said in the history of a document nothing was said on', (await tools.locator('.journey-spoken').count()) === 0);
  await bubble.click();
  await page.locator('.doc-tools, .doc-tools-leaving').waitFor({ state: 'detached' });

  // The library as pages: its front page, a document, and the link between them.
  const pages = await context.newPage();
  await pages.goto(`${url}/library/shop/docs/`);
  await pages.getByRole('heading', { name: 'Shop docs', level: 1 }).waitFor();
  expect('the front page to list the folder', (await pages.getByRole('heading', { name: 'guides', level: 2 }).count()) === 1);
  // The contents are beside every page served, as they are beside every page written out, with the page showing marked.
  const marked = async (words) => (await pages.locator('nav.page-nav a[aria-current="page"]').innerText()) === words;
  expect('the front page served to have the contents beside it, itself marked', await marked('Shop docs'));
  await pages.screenshot({ path: `${out}/library-3-front.png` });
  await pages.locator('main').getByRole('link', { name: 'Installing it' }).click();
  await pages.getByRole('heading', { name: 'Installing it', level: 1 }).waitFor();
  expect('a document served to be marked in the contents beside it', await marked('Installing it'));
  expect('a table in a document to be drawn as one', (await pages.locator('table td', { hasText: 'It is unpacked' }).count()) === 1);
  await pages.screenshot({ path: `${out}/library-4-page.png` });
  await pages.locator('main').getByRole('link', { name: 'back' }).click();
  await pages.getByRole('heading', { name: 'Welcome', level: 1 }).waitFor();
  await pages.locator('main').getByRole('link', { name: 'Shop docs' }).click();
  await pages.getByRole('heading', { name: 'Shop docs', level: 1 }).waitFor();
  // Nothing of it is there for someone who is not signed in.
  const stranger = await browser.newContext();
  const refused = await (await stranger.newPage()).goto(`${url}/library/shop/docs/`);
  expect(`the pages to be refused to someone not signed in, got ${refused.status()}`, refused.status() === 401);

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
