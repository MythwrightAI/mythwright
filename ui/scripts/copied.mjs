// A block of code in what somebody wrote, and its Copy button
// (`views/codeblock.tsx`, over `src/codeblocks.ts`).
//
//   DAEMON=../target/debug/factory node scripts/copied.mjs [out-dir]
//
// Ada says things with code in them from a terminal: in a channel, a
// fenced block with a tab and spaces at a line's end in it, beside a
// piece of code in a sentence; an indented block; a list whose item goes
// on in a paragraph under it, which is no block of code, with a block
// inside an item and one after the list; in a thread under the
// first; on a card, as a comment; and in lines of her inbox that a
// blueprint told her, one of which begins with a block, each page and
// tab called by its words and never by its code. Each block has a
// button at its top right, called Copy code, and code in a sentence has
// none. On a page with a pointer
// the button is not drawn until its block is pointed at or the keyboard
// is in it, and nothing of the block moves as it comes; Tab reaches it.
// Pressed, and by Enter, the clipboard holds the block's text exactly as
// it was written, with no line break added and its tab kept, the button
// says Copied, and then Copy again, which the check reads from what the
// button says it has done and waits on no time for. The button is over
// none of the block's text at three widths and a phone's, scrolled or
// not, and on a phone, where nothing points, it is always drawn and a
// finger's size. The same holds of the thread, of the card's history in
// its drawer and of the inbox line's page. Run plainly and with
// `LOOK="size=largest density=compact"`. The environment variables are
// those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'copied-data');
const home = join(out, 'copied-home');
const looked = process.env.LOOK ? '-looked' : '';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

// What each block holds, as written: a tab, spaces at a line's end, a long line, and no line break at its end.
const FENCED = "\tcargo build --release   \nprintf 'a\\tb' | ./tally --long-option-that-goes-on-and-on --and-another-one-after-it --and-a-third";
const SAID = `Run this:\n\`\`\`sh\n${FENCED}\n\`\`\`\nthen use \`tally\` as before.`;
const INDENTED = 'make check\n\tmake install';
const SAID_INDENTED = `Then, further in:\n\n    make check\n    \tmake install\n\nand that is all.`;
const THREADED = 'git log --oneline -3';
// A list as an agent writes one: an item's next paragraph written under it, which is the item going on and no
// block of code; a block of code inside an item, four further in than its words; and one after the list has ended.
const LISTED = `Two ways on:\n\n- ask the worker\n\n    the helper works the app board, so check with it first.\n\n- or build it:\n\n      cargo build --release\n\nThe list is over.\n\n    make install`;
const COMMENTED = 'SELECT total FROM receipts\nWHERE tax = 1;';
const TOLD_CODE = 'mw sandbox run std/board\n  --as preview';
const RELEASED = 'make release';
const TOLD = `blueprint: told
permissions: [inbox]
params:
  says: { type: string }
steps:
  tell:
    do: inbox.tell
    with: { to: applier, says: "\${params.says}", why: built }
`;

let failed = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  await cli('channel', 'create', 'general');
  await cli('post', 'general', SAID);
  await cli('post', 'general', SAID_INDENTED);
  await cli('post', 'general', 'only `inline` code, with no block');
  await cli('post', 'general', LISTED);
  const read = JSON.parse((await cli('read', 'general', '--json')).stdout);
  const root = read.messages.find((message) => message.body.startsWith('Run this:')).id;
  await cli('post', 'general', '--thread', root, `In the thread:\n\`\`\`\n${THREADED}\n\`\`\``);
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('board', 'add', 'shop', 'Add a cart');
  await cli('record', 'comment', 'shop-1', `Which total?\n\`\`\`sql\n${COMMENTED}\n\`\`\``);
  await writeFile(join(home, 'told.bp'), TOLD);
  await cli('apply', join(home, 'told.bp'), '--param', `says=The preview is ready:\n\`\`\`\n${TOLD_CODE}\n\`\`\`\nTry it.`);
  await cli('apply', join(home, 'told.bp'), '--key', 'fenced', '--param', `says=\`\`\`\n${RELEASED}\n\`\`\`\nThe release of \`origin/dev\` is built.`);
  const lines = JSON.parse((await cli('inbox', '--json')).stdout).lines;
  const told = lines.find((line) => line.says.startsWith('The preview')).id;
  const fenced = lines.find((line) => line.says.startsWith('```')).id;

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
  const signedIn = async (how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    await context.grantPermissions(['clipboard-read', 'clipboard-write'], { origin: url });
    const page = await context.newPage();
    lastPage = page;
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const at = (page, hash) => page.evaluate((to) => (location.hash = to), hash);
  const clipboard = (page) => page.evaluate(() => navigator.clipboard.readText());
  const emptied = (page) => page.evaluate(() => navigator.clipboard.writeText('(nothing yet)'));
  const said = (block) => block.locator('.code-copy').getAttribute('data-copied');
  const backToCopy = (block) => block.locator('.code-copy[data-copied="idle"]').waitFor({ timeout: 10000 });
  // The channel's own message, which is drawn before the thread's copy of it.
  const message = (page, words) => page.locator('.message', { hasText: words }).first();

  // Where the block, its first line and its button are, and whether the button is over any of its text.
  const boxes = (block) =>
    block.evaluate((box) => {
      const pre = box.querySelector('pre');
      const button = box.querySelector('.code-copy');
      const rect = (one) => {
        const r = one.getBoundingClientRect();
        return { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
      };
      const range = document.createRange();
      range.selectNodeContents(pre);
      const shownIn = rect(pre);
      const lines = [...range.getClientRects()].filter((r) => r.width > 0.5 && r.bottom > shownIn.top && r.top < shownIn.bottom);
      const by = button.getBoundingClientRect();
      const over = lines.filter((r) => Math.min(r.right, by.right) - Math.max(r.left, by.left) > 0.5 && Math.min(r.bottom, by.bottom) - Math.max(r.top, by.top) > 0.5);
      const style = getComputedStyle(button);
      return {
        block: rect(box),
        pre: rect(pre),
        first: lines[0] ? { left: lines[0].left, top: lines[0].top, right: lines[0].right, bottom: lines[0].bottom } : null,
        button: rect(button),
        over: over.length,
        shown: Number(style.opacity) > 0 && style.visibility !== 'hidden',
        inside: by.left >= box.getBoundingClientRect().left - 0.5 && by.right <= box.getBoundingClientRect().right + 0.5 && by.top >= box.getBoundingClientRect().top - 0.5,
      };
    });
  const same = (one, two) => JSON.stringify(one) === JSON.stringify(two);
  const clear = async (block, what) => {
    const seen = await boxes(block);
    expect(`the button ${what} to be inside its block and over none of its text, got ${JSON.stringify(seen)}`, seen.inside && seen.over === 0);
    // Scrolled to its end, where it scrolls, the button is over none of it either.
    await block.locator('pre').evaluate((pre) => (pre.scrollTop = pre.scrollHeight));
    const scrolled = await boxes(block);
    await block.locator('pre').evaluate((pre) => (pre.scrollTop = 0));
    expect(`the button ${what}, its block scrolled, over none of its text, got ${JSON.stringify(scrolled)}`, scrolled.over === 0);
    return seen;
  };
  // The block copied by a press: what is on the clipboard, what the button says, and back to Copy.
  const copies = async (page, block, text, what, by = 'press') => {
    await emptied(page);
    if (by === 'press') await block.getByRole('button', { name: 'Copy code', exact: true }).click();
    else if (by === 'tap') await block.getByRole('button', { name: 'Copy code', exact: true }).tap();
    else await page.keyboard.press('Enter');
    await block.locator('.code-copy[data-copied="copied"]').waitFor();
    const got = await clipboard(page);
    expect(`the clipboard to hold ${what} exactly as written, got ${JSON.stringify(got)}`, got === text);
    expect(`the button to say Copied, got ${await block.locator('.code-copy').getAttribute('data-word')}`, (await block.locator('.code-copy').getAttribute('data-word')) === 'Copied');
    await backToCopy(block);
  };

  // ── A channel, with a pointer ───────────────────────────────────────────
  const page = await signedIn();
  const panel = page.getByRole('navigation', { name: 'Channels' });
  await panel.waitFor();
  await panel.locator('.grouped-channel', { has: page.locator('.nav-item .truncate', { hasText: /^general$/ }) }).locator('.nav-item').first().click();
  const first = message(page, 'Run this:').locator('.code-block');
  await first.waitFor();
  expect('code in a sentence to have no button', (await message(page, 'only').locator('.code-copy').count()) === 0 && (await message(page, 'only').locator('code').count()) === 1);
  expect('the sentence the first block is in to keep its piece of code with no button', (await message(page, 'Run this:').locator('.code-copy').count()) === 1);
  expect('the button to be called Copy code', (await first.getByRole('button', { name: 'Copy code', exact: true }).count()) === 1);
  await page.mouse.move(2, 2);
  const still = await boxes(first);
  expect(`the button not drawn while nothing points at its block, got ${JSON.stringify(still)}`, !still.shown);
  await first.hover();
  const pointed = await boxes(first);
  expect('the button drawn while its block is pointed at', pointed.shown);
  expect(`nothing of the block to move as the button comes, from ${JSON.stringify(still)} to ${JSON.stringify(pointed)}`, same({ ...still, shown: 0 }, { ...pointed, shown: 0 }));
  await clear(first, 'pointed at');
  await page.screenshot({ path: `${out}/copied-0-pointed${looked}.png` });
  await copies(page, first, FENCED, 'the fenced block');
  expect('nothing of the block to have moved once copied', same({ ...(await boxes(first)), shown: 0 }, { ...still, shown: 0 }));

  // By the keyboard: Tab from what comes before it reaches it, it is drawn, and Enter copies.
  await page.mouse.move(2, 2);
  const indented = message(page, 'Then, further in').locator('.code-block');
  await indented.waitFor();
  await indented.evaluate((box) => {
    const button = box.querySelector('.code-copy');
    const tabbable = [...document.querySelectorAll('button, a[href], input, select, textarea, [tabindex]')].filter(
      (one) => one.tabIndex >= 0 && !one.disabled && one.getClientRects().length > 0,
    );
    tabbable[tabbable.indexOf(button) - 1].focus();
  });
  await page.keyboard.press('Tab');
  expect('Tab to reach the button', await indented.locator('.code-copy').evaluate((button) => document.activeElement === button));
  expect('the button drawn while the keyboard is on it', (await boxes(indented)).shown);
  await page.screenshot({ path: `${out}/copied-1-keyboard${looked}.png` });
  await copies(page, indented, INDENTED, 'the indented block, without what put it further in', 'key');
  await clear(indented, 'of the indented block');

  // A list: the item's next paragraph is drawn in the item and as no block, and the two blocks of code are each what it holds.
  const listed = message(page, 'Two ways on');
  await listed.scrollIntoViewIfNeeded();
  const going = listed.locator('.prose-item', { hasText: 'ask the worker' });
  expect('the item to hold its next paragraph', (await going.innerText()).includes('the helper works the app board'));
  expect('the paragraph under an item to be no block of code', (await listed.locator('.code-block', { hasText: 'the helper works' }).count()) === 0);
  const inItem = listed.locator('.code-block', { hasText: 'cargo build' });
  const afterList = listed.locator('.code-block', { hasText: 'make install' });
  expect(`two blocks of code in the list's message, got ${await listed.locator('.code-block').count()}`, (await listed.locator('.code-block').count()) === 2);
  await copies(page, inItem, 'cargo build --release', 'the block inside an item, without what put it under the item');
  await copies(page, afterList, 'make install', 'the block after the list');
  await clear(inItem, 'of the block inside an item');
  await page.screenshot({ path: `${out}/copied-1-listed${looked}.png` });

  // At three widths, plainly: over no text, inside its block.
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await first.scrollIntoViewIfNeeded();
    await first.hover();
    await clear(first, `at ${width} wide`);
    await clear(indented, `of the indented block at ${width} wide`);
    await page.screenshot({ path: `${out}/copied-3-${width}${looked}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // In a thread.
  await message(page, 'Run this:').hover();
  await message(page, 'Run this:').locator('button.replies').click();
  const threaded = page.locator('.thread .message', { hasText: 'In the thread' }).locator('.code-block');
  await threaded.waitFor();
  await threaded.hover();
  await clear(threaded, 'in a thread');
  await copies(page, threaded, THREADED, 'the block in the thread');
  await page.screenshot({ path: `${out}/copied-2-thread${looked}.png` });


  // ── A card's history, in its drawer ─────────────────────────────────────
  await at(page, '#/board/shop');
  await page.locator('.card', { hasText: 'Add a cart' }).click();
  const drawer = drawerAtRest(page);
  // What was said on it is under Comments, in the row of tabs beside Details.
  await drawer.getByRole('tab', { name: 'Comments', exact: true }).click();
  const commented = drawer.locator('.journey-words .code-block');
  await commented.waitFor();
  expect('the words round the block to be drawn as they were', (await drawer.locator('.journey-words', { hasText: 'Which total?' }).count()) === 1);
  await commented.hover();
  await clear(commented, "in a card's history");
  await copies(page, commented, COMMENTED, "the block in the card's history");
  await page.screenshot({ path: `${out}/copied-4-card${looked}.png` });

  // ── A line of the inbox, on its page ────────────────────────────────────
  await at(page, `#/inbox/${told}`);
  const ofTold = page.locator('.inbox-page-words .code-block');
  await ofTold.waitFor();
  expect('the words round the block to be drawn as they were', (await page.locator('.inbox-page-words', { hasText: 'The preview is ready:' }).count()) === 1 && (await page.locator('.inbox-page-words', { hasText: 'Try it.' }).count()) === 1);
  await ofTold.hover();
  await clear(ofTold, "on an inbox line's page");
  await copies(page, ofTold, TOLD_CODE, "the block on the inbox line's page");
  // Called by its words, which a block of code is not: here those before it.
  const called = async (id, title) => {
    const head = (await page.locator('.inbox-page-title').innerText()).trim();
    const tab = (await page.locator(`.tab[data-key="inbox:${id}"] .truncate`).textContent()).trim();
    expect(`the page and its tab to be called "${title}", got "${head}" and "${tab}"`, head === title && tab === title);
  };
  await called(told, 'The preview is ready:');
  await page.screenshot({ path: `${out}/copied-5-inbox${looked}.png` });
  // One that begins with a block is called by the words after it, its piece of code without its marks.
  await at(page, `#/inbox/${fenced}`);
  const ofFenced = page.locator('.inbox-page-words .code-block');
  await ofFenced.waitFor();
  await called(fenced, 'The release of origin/dev is built.');
  await ofFenced.hover();
  await clear(ofFenced, "on the page of an inbox line that begins with it");
  await copies(page, ofFenced, RELEASED, "the block an inbox line begins with");
  await page.screenshot({ path: `${out}/copied-5-inbox-fenced${looked}.png` });

  // ── A phone, where nothing points ───────────────────────────────────────
  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
  await sideSettled(phone);
  const small = phone.getByRole('navigation', { name: 'Channels' });
  await small.waitFor();
  await small.locator('.grouped-channel', { has: phone.locator('.nav-item .truncate', { hasText: /^general$/ }) }).locator('.nav-item').first().tap();
  const onPhone = message(phone, 'Run this:').locator('.code-block');
  await onPhone.waitFor();
  await onPhone.scrollIntoViewIfNeeded();
  const touched = await clear(onPhone, 'on a phone');
  expect('the button always drawn where nothing points', touched.shown);
  const tall = touched.button.bottom - touched.button.top;
  const wide = touched.button.right - touched.button.left;
  expect(`the button a finger's size on a phone, got ${wide} by ${tall}`, tall >= 34 && wide >= 44);
  expect(`the block no wider than the screen, got ${JSON.stringify(touched.block)}`, touched.block.left >= 0 && touched.block.right <= 390);
  await clear(message(phone, 'Then, further in').locator('.code-block'), 'of the indented block on a phone');
  await phone.screenshot({ path: `${out}/copied-6-phone${looked}.png` });
  await copies(phone, onPhone, FENCED, 'the fenced block, tapped on a phone', 'tap');

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/copied-failed${looked}.png` }).catch(() => {});
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
