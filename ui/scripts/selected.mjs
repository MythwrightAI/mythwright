// Comments on a selection of a document's text: words
// chosen in the block editor by dragging, with nothing drawn under the
// pointer while it is held and a bar with Comment over them once it lets
// go; the comment begun there, opened beside the text in the margin where
// the page has room, written and sent; the words marked, pointed at with
// nothing moved, pressed open again and drawn as the open one; at 1280,
// 1440 and 1600 wide the page having made room beside its column before
// anything is pressed, so that a mark pressed stays under the pointer and
// nothing moves as its card opens or is put away; one said from a terminal marked
// on the page with nothing read again; the text edited round the words and
// saved, the mark still on them; the file's source marking the same words,
// a mark pressed there opening its comments in the panel beside the
// document (`views/doctools.tsx`); the words rewritten, the comment listed
// in that panel's Comments saying what it was about and that it is no
// longer there, never dropped; the page as published drawing no marks,
// every comment listed there saying what it is on; a narrow tab, where a
// comment begun opens in the panel; nothing of it over anything else or
// scrolling sideways at three widths and a phone's, where a mark tapped
// opens in the panel, which has the screen; and the other of dark and light.
//
//   DAEMON=../target/debug/factory node scripts/selected.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { DOC_TOOLS_AT_REST } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'selected-data');
const home = join(out, 'selected-home');
const scheme = process.env.SCHEME ?? 'dark';
const other = scheme === 'dark' ? 'light' : 'dark';

const TEXT = `# Installing it

Read this first, all of it.

## On Linux

Then run the second step, which is short.
It takes a **minute**, no more.

## On Windows

Use the installer, and wait for it.
`;
const RECORD = 'shop-docs:install';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
// What the page showed where it failed.
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  const docs = join(data, 'projects', 'shop', 'docs');
  await rm(docs, { recursive: true, force: true });
  await mkdir(docs, { recursive: true });
  const file = join(docs, 'install.md');
  await writeFile(file, TEXT);
  await cli('apply', 'std/docs', '--project', 'shop');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1800, height: 900 }, colorScheme: scheme });
  const page = await context.newPage();
  lastPage = page;
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
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(page);
  await page.locator('.nav').getByRole('button', { name: /(^|\s)Docs$/ }).click();
  await page.locator('.library-line', { hasText: 'Installing it' }).click();
  const doc = page.locator('.writer .ProseMirror');
  await doc.locator('h1', { hasText: 'Installing it' }).waitFor();
  const bar = page.locator('.select-bar');
  const card = page.locator('.writer-card');
  const marks = page.locator('.writer .commented');
  // The speech bubble at the end of the head's row, which opens and puts away the panel beside the document.
  const bubble = page.locator('.view-head .head-acts .head-tool');
  const tools = page.locator(DOC_TOOLS_AT_REST);
  const putAway = async () => {
    await bubble.click();
    await page.locator('.doc-tools').waitFor({ state: 'detached' });
    await page.locator('.doc-tools-leaving').waitFor({ state: 'detached' });
  };

  /** Where `needle` is in the block of the editor that says `inside`: the first and last boxes of its words. */
  const wordsAt = (needle, inside, root = '.writer .ProseMirror') =>
    page.evaluate(
      ([needle, inside, root]) => {
        const block = [...document.querySelector(root).querySelectorAll('p, h1, h2, h3, li')].find((one) => one.textContent.includes(inside));
        const walk = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
        for (let text = walk.nextNode(); text; text = walk.nextNode()) {
          const at = text.textContent.indexOf(needle);
          if (at < 0) continue;
          const range = document.createRange();
          range.setStart(text, at);
          range.setEnd(text, at + needle.length);
          const boxes = [...range.getClientRects()].map(({ x, y, width, height }) => ({ x, y, width, height }));
          return { first: boxes[0], last: boxes.at(-1) };
        }
        return null;
      },
      [needle, inside, root],
    );
  /** Chooses words as a person does, by dragging over them; while the pointer is held nothing is drawn over them. */
  const choose = async (needle, inside) => {
    const at = await wordsAt(needle, inside);
    expect(`"${needle}" in the block that says "${inside}"`, at);
    await page.mouse.move(at.first.x + 1, at.first.y + at.first.height / 2);
    await page.mouse.down();
    await page.mouse.move(at.last.x + at.last.width / 2, at.last.y + at.last.height / 2, { steps: 4 });
    await page.mouse.move(at.last.x + at.last.width - 1, at.last.y + at.last.height / 2, { steps: 2 });
    expect('no bar while the pointer is held', await bar.isHidden());
    await page.mouse.up();
    await bar.waitFor();
    return at;
  };
  const box = (one) => one.boundingBox();
  const over = (a, b) => a.x < b.x + b.width - 0.5 && a.x + a.width > b.x + 0.5 && a.y < b.y + b.height - 0.5 && a.y + a.height > b.y + 0.5;
  const crowded = async (where, within = 'body') => {
    const said = await pageCrowded(page, within);
    expect(`nothing over anything else ${where}: ${said.join('; ')}`, said.length === 0);
  };

  // ---- words chosen, and a comment begun on them ----------------------------------------
  const chosenAt = await choose('the second step', 'which is short. It takes');
  const barBox = await box(bar);
  expect(`the bar over the words and not on them, at ${JSON.stringify(barBox)}`, barBox.y + barBox.height <= chosenAt.first.y + 0.5);
  expect('the bar in the window', barBox.x >= 0 && barBox.x + barBox.width <= 1800);
  await page.screenshot({ path: `${out}/selected-1-chosen.png` });
  await bar.getByRole('button', { name: 'Comment' }).click();
  await card.waitFor();
  const column = await box(doc);
  const cardBox = await box(card);
  expect(`the card beside the text and over none of it, at ${JSON.stringify(cardBox)} with the text at ${JSON.stringify(column)}`, cardBox.x >= column.x + column.width);
  expect(`the card at the height of the words, at ${cardBox.y} for ${chosenAt.first.y}`, Math.abs(cardBox.y - chosenAt.first.y) < 12);
  expect('the card saying what it is on', (await card.locator('.words-thread-on').textContent()) === 'the second step');
  await page.waitForFunction(() => document.activeElement?.closest('.writer-card') && document.activeElement.getAttribute('aria-label') === 'Add a comment');
  await page.keyboard.type('Which step is the second?');
  await page.keyboard.press('Enter');
  await card.locator('.comment-words', { hasText: 'Which step is the second?' }).waitFor();
  await marks.first().waitFor();
  expect(`the mark over the words, reading "${await marks.first().textContent()}"`, (await marks.first().textContent()) === 'the second step');
  const marked = await marks.first().evaluate((one) => one.closest('p').textContent);
  expect(`the words marked where they were chosen, in "${marked}"`, marked.includes('It takes'));
  expect('no panel opened beside the document for it', (await page.locator('.doc-tools').count()) === 0);
  await page.waitForFunction(() => document.querySelector('.view-head .head-tool')?.getAttribute('aria-label') === 'Comments and details, 1 comment');
  await crowded('with the card open');
  await page.screenshot({ path: `${out}/selected-2-marked.png` });

  // The file holds nothing of it.
  expect('the file as it was', (await readFile(file, 'utf8')) === TEXT);

  // ---- the mark pointed at and pressed ----------------------------------------------------
  await page.keyboard.press('Escape');
  await card.waitFor({ state: 'detached' });
  const placed = () => page.evaluate(() => [...document.querySelectorAll('.writer .ProseMirror > *')].map((one) => one.getBoundingClientRect().toJSON()));
  const before = await placed();
  const markBox = await box(marks.first());
  await page.mouse.move(markBox.x + markBox.width / 2, markBox.y + markBox.height / 2);
  await page.waitForTimeout(150);
  expect('nothing moved as the mark is pointed at', JSON.stringify(await placed()) === JSON.stringify(before));
  await page.mouse.click(markBox.x + markBox.width / 2, markBox.y + markBox.height / 2);
  await card.waitFor();
  await card.locator('.comment-words', { hasText: 'Which step is the second?' }).waitFor();
  expect('nothing moved as the mark is pressed', JSON.stringify(await placed()) === JSON.stringify(before));
  // The mark whose card is open is drawn as that, and as no other once it is put away.
  expect('the pressed mark drawn as the open one', (await page.locator('.writer .commented.open').count()) === 1);
  // A press elsewhere in the text puts it away.
  const away = await box(doc.locator('h1'));
  await page.mouse.click(away.x + 10, away.y + away.height / 2);
  await card.waitFor({ state: 'detached' });
  await page.waitForFunction(() => document.querySelectorAll('.writer .commented.open').length === 0);

  // ---- room made beside the column at the widths a page is usually seen at ------------------
  // Too narrow for a card beside a column in the middle, the page has made room to the column's right already, so a
  // mark pressed stays where it was under the pointer, and nothing moves as its card is put away.
  for (const width of [1280, 1440, 1600]) {
    await page.setViewportSize({ width, height: 860 });
    await page.waitForFunction(() => document.querySelector('.writer.page')?.classList.contains('margined'));
    await page.waitForTimeout(150);
    const mark = marks.filter({ hasText: 'the second step' });
    const was = await box(mark);
    const laid = await placed();
    await page.mouse.click(was.x + was.width / 2, was.y + was.height / 2);
    await card.waitFor();
    await card.locator('.comment-words', { hasText: 'Which step is the second?' }).waitFor();
    expect(`at ${width} wide, the mark still under the pointer`, JSON.stringify(await box(mark)) === JSON.stringify(was));
    expect(`at ${width} wide, nothing of the text moved as the card opened`, JSON.stringify(await placed()) === JSON.stringify(laid));
    expect(`at ${width} wide, the pressed mark drawn as the open one`, (await mark.getAttribute('class')).includes('open'));
    const text = await box(doc);
    const beside = await box(card);
    const scroller = await box(page.locator('.writer.page'));
    expect(`at ${width} wide, the card beside the text and over none of it, at ${JSON.stringify(beside)} with the text at ${JSON.stringify(text)}`, beside.x >= text.x + text.width);
    expect(`at ${width} wide, the card inside the page, at ${JSON.stringify(beside)} in ${JSON.stringify(scroller)}`, beside.x + beside.width <= scroller.x + scroller.width);
    expect(`at ${width} wide, the card at the height of the mark`, Math.abs(beside.y - was.y) < 12);
    expect(`at ${width} wide, no drawer and no panel`, (await page.locator('.drawer, .doc-tools').count()) === 0);
    await crowded(`at ${width} wide with the card open`);
    if (width === 1440) await page.screenshot({ path: `${out}/selected-8-1440.png` });
    await page.keyboard.press('Escape');
    await card.waitFor({ state: 'detached' });
    expect(`at ${width} wide, nothing moved as the card was put away`, JSON.stringify(await placed()) === JSON.stringify(laid));
  }
  await page.setViewportSize({ width: 1800, height: 900 });
  await page.waitForTimeout(150);

  // ---- one said from a terminal, on a page that is not read again ---------------------------
  await cli('record', 'comment', RECORD, '--on', 'no more', '--before', 'minute, ', '--under', 'Installing it', '--under', 'On Linux', 'Is', 'it', 'really?');
  await page.waitForFunction(() => [...document.querySelectorAll('.writer .commented')].some((one) => one.textContent === 'no more'));

  // ---- the text edited round the words, and saved ------------------------------------------
  const start = await wordsAt('Then run', 'It takes');
  await page.mouse.click(start.first.x + 1, start.first.y + start.first.height / 2);
  await page.waitForTimeout(100);
  await page.keyboard.press('Home');
  await page.keyboard.type('Once it is down, ');
  await page.keyboard.press('Control+s');
  await page.getByText('unsaved').waitFor({ state: 'detached' });
  expect('the edit in the file', (await readFile(file, 'utf8')).includes('Once it is down, Then run the second step'));
  await page.waitForFunction(() => {
    const mark = [...document.querySelectorAll('.writer .commented')].find((one) => one.textContent === 'the second step');
    return mark?.closest('p')?.textContent.startsWith('Once it is down');
  });

  // Every comment marked in the text, listed in the panel's Comments, each saying what it is on: what the keyboard reaches them by.
  await bubble.click();
  await tools.locator('.doc-comments > .comment-list .comment-on', { hasText: 'on “no more”' }).waitFor();
  await tools.locator('.doc-comments > .comment-list .comment-on', { hasText: 'on “the second step”' }).waitFor();
  await crowded('with the panel beside the document open');
  await page.screenshot({ path: `${out}/selected-9-listed.png` });
  await putAway();

  // ---- the file's source marks the same words ---------------------------------------------
  await page.getByRole('tab', { name: 'Source', exact: true }).click();
  const sourceMarks = page.locator('.cm-commented');
  await sourceMarks.first().waitFor();
  const inSource = await sourceMarks.allTextContents();
  expect(`the source marking the same words, got ${JSON.stringify(inSource)}`, inSource.includes('the second step') && inSource.includes('no more'));
  await page.screenshot({ path: `${out}/selected-3-source.png` });
  await sourceMarks.filter({ hasText: 'the second step' }).click();
  await tools.waitFor();
  await tools.locator('.words-thread .comment-words', { hasText: 'Which step is the second?' }).waitFor();
  expect('Comments to be the tab showing', (await tools.getByRole('tab', { name: 'Comments', exact: true }).getAttribute('aria-selected')) === 'true');
  await page.locator('.cm-commented.cm-open', { hasText: 'the second step' }).waitFor();
  await crowded('with a mark of the source opened in the panel beside the document');
  // Its card put away, every comment is still there under it, and the mark is no longer drawn as the open one.
  await tools.getByRole('button', { name: 'Close the comments', exact: true }).click();
  await tools.locator('.words-thread').waitFor({ state: 'detached' });
  await page.waitForFunction(() => document.querySelectorAll('.cm-commented.cm-open').length === 0);
  await putAway();

  // ---- the words rewritten: the comment stands over the text, never dropped ----------------
  await page.getByRole('tab', { name: 'Write', exact: true }).click();
  await marks.first().waitFor();
  await choose('second step', 'Once it is down');
  await page.keyboard.type('next one');
  await page.keyboard.press('Control+s');
  await page.getByText('unsaved').waitFor({ state: 'detached' });
  await bubble.click();
  const gone = tools.locator('.doc-comments > .comment-list .comment', { hasText: 'Which step is the second?' });
  await gone.waitFor();
  expect(`the comment saying what it was about, got "${await gone.locator('.comment-on').textContent()}"`, (await gone.locator('.comment-on').textContent()) === 'on “the second step”, which is no longer in the text');
  expect('no mark on words that are gone', (await marks.allTextContents()).every((words) => words !== 'the second step'));
  await crowded('with a comment whose words are gone');
  await page.screenshot({ path: `${out}/selected-4-rewritten.png` });

  // ---- the page as it is published draws no marks -------------------------------------------
  await page.getByRole('tab', { name: 'Page', exact: true }).click();
  await page.locator('iframe.page-frame').waitFor();
  expect('no mark on the page as it is published', (await page.locator('.commented, .cm-commented').count()) === 0);
  const onPage = await tools.locator('.doc-comments > .comment-list .comment-on').allTextContents();
  expect(`every comment on words listed beside the page, got ${JSON.stringify(onPage)}`, onPage.includes('on “no more”') && onPage.includes('on “the second step”, which is no longer in the text'));
  await page.getByRole('tab', { name: 'Write', exact: true }).click();
  await marks.first().waitFor();
  await putAway();

  // ---- three widths, and a narrow one where a comment opens in the panel -------------------
  for (const width of [1800, 1280, 860]) {
    await page.setViewportSize({ width, height: 860 });
    await page.waitForTimeout(150);
    await crowded(`at ${width} wide`);
  }
  await choose('the installer', 'Use the installer');
  await bar.getByRole('button', { name: 'Comment' }).click();
  await tools.waitFor();
  expect('no card in the margin of a narrow tab', (await card.count()) === 0);
  await page.waitForFunction(() => document.activeElement?.closest('.doc-tools .words-thread') && document.activeElement.getAttribute('aria-label') === 'Add a comment');
  await page.keyboard.type('Which installer?');
  await page.keyboard.press('Enter');
  await tools.locator('.words-thread .comment-words', { hasText: 'Which installer?' }).waitFor();
  await tools.locator('.doc-comments > .comment-list .comment-words', { hasText: 'Which installer?' }).waitFor();
  await crowded('with a comment begun in the panel beside the document of a narrow tab');
  await page.screenshot({ path: `${out}/selected-5-narrow.png` });
  await putAway();

  // ---- a phone -----------------------------------------------------------------------
  const phone = await browser.newContext({ storageState: await context.storageState(), viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const small = await phone.newPage();
  small.on('pageerror', (err) => problems.push(`at a phone's width: ${err.message}`));
  await small.goto(url);
  const smallMark = small.locator('.writer .commented', { hasText: 'no more' });
  await smallMark.waitFor();
  const phoneSaid = await pageCrowded(small);
  expect(`nothing over anything else on a phone: ${phoneSaid.join('; ')}`, phoneSaid.length === 0);
  await smallMark.tap();
  await small.locator(DOC_TOOLS_AT_REST).locator('.words-thread .comment-words', { hasText: 'Is it really?' }).waitFor();
  const phoneDrawer = await pageCrowded(small, '.doc-tools');
  expect(`nothing over anything else in the panel beside the document on a phone: ${phoneDrawer.join('; ')}`, phoneDrawer.length === 0);
  await small.screenshot({ path: `${out}/selected-6-phone.png` });

  // ---- the other of dark and light ------------------------------------------------------
  const lit = await browser.newContext({ storageState: await context.storageState(), viewport: { width: 1800, height: 900 }, colorScheme: other });
  const light = await lit.newPage();
  light.on('pageerror', (err) => problems.push(`in ${other}: ${err.message}`));
  await light.goto(url);
  const lightMark = light.locator('.writer .commented', { hasText: 'no more' });
  await lightMark.waitFor();
  await lightMark.click();
  await light.locator('.writer-card .comment-words', { hasText: 'Is it really?' }).waitFor();
  await light.screenshot({ path: `${out}/selected-7-${other}.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/selected-failed.png` }).catch(() => {});
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
