// The handle beside a block of a document: pointed at, a
// block has a `+` and a grip in the margin to its left, with nothing of the
// text moved as they come and as the pointer crosses to them; the grip
// pressed opens a menu (Turn into, with what the block is marked;
// Duplicate; Move up and down; Comment; Delete), by its boxes; a block
// turned into another kind, a list into text with none of it lost and
// back; a block carried by its grip, nothing moving as the carry begins, a
// line saying where it lands, the block fainter, and let go there; Escape
// in the middle of a carry; the `+` adding a line with the `/` menu open
// on it; Ctrl and / opening the same menu for the block the cursor is in,
// Escape back to the grip and again to the text; Comment beginning a
// comment that names the block in the panel beside the document, opening
// it where it was put away, the cursor at its end; the file what each
// of it says, saved as any edit is; a narrow tab, where only the grip has
// room; and a phone's width by touch, where the grip is of the block the
// cursor is in and lies over nothing.
//
//   DAEMON=../target/debug/factory node scripts/handled.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { unfoldAll } from './folded.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'handled-data');
const home = join(out, 'handled-home');

const BEFORE = `# Riverside Shop

A shop for *well made* things.

- mugs
- knives

Last line.
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  // A document of a library, so that it takes comments.
  const docs = join(data, 'projects', 'shop', 'docs');
  await rm(docs, { recursive: true, force: true });
  await mkdir(docs, { recursive: true });
  const file = join(docs, 'about.md');
  await writeFile(file, BEFORE);
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
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  // Everything the Resources panel lists starts folded (`folded.mjs`).
  await unfoldAll(page);
  // The project's library, and not the blueprint that gives one, which Expand all lists too.
  await page.locator('.nav').getByRole('button', { name: /(^|\s)Docs$/ }).click();
  await page.locator('.library-line', { hasText: 'Riverside Shop' }).click();

  const doc = page.locator('.writer .ProseMirror');
  await doc.locator('h1', { hasText: 'Riverside Shop' }).waitFor();
  const handle = page.locator('.block-handle');
  const grip = handle.getByRole('button', { name: 'Drag to move, or open the menu' });
  const add = handle.getByRole('button', { name: 'Add a block below' });
  const menu = page.getByRole('menu', { name: 'Block' });
  const blocks = doc.locator(':scope > *');
  /** Where each block of the document is, and the handle, as the browser drew them. */
  const drawn = (on = page) =>
    on.evaluate(() => {
      const box = (one) => {
        const { x, y, width, height } = one.getBoundingClientRect();
        return { x, y, width, height };
      };
      const handle = document.querySelector('.block-handle');
      return {
        blocks: [...document.querySelectorAll('.writer .ProseMirror > *')].map(box),
        column: box(document.querySelector('.writer .ProseMirror')),
        shown: handle?.dataset.shown === 'true',
        handle: handle ? box(handle) : null,
        parts: [...(handle?.querySelectorAll('button') ?? [])].filter((one) => one.getClientRects().length > 0).map(box),
      };
    });
  const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
  /** The handle beside the text and over none of it: left of the column, and lying over no block. */
  const besideTheText = (seen, where) => {
    const over = seen.blocks.filter((one) => seen.parts.some((part) => part.x < one.x + one.width - 0.5 && part.x + part.width > one.x + 0.5 && part.y < one.y + one.height - 0.5 && part.y + part.height > one.y + 0.5));
    expect(`the handle to be left of the text ${where}, and it is at ${JSON.stringify(seen.parts)} with the column at ${JSON.stringify(seen.column)}`, seen.parts.length > 0 && seen.parts.every((part) => part.x + part.width <= seen.column.x + 0.5 && part.x >= -0.5));
    expect(`the handle to lie over no block ${where}`, over.length === 0);
  };
  const pointAt = async (index, dx = 12) => {
    const at = await blocks.nth(index).boundingBox();
    await page.mouse.move(at.x + dx, at.y + Math.min(at.height / 2, 10));
    await page.waitForFunction((index) => {
      const handle = document.querySelector('.block-handle');
      const block = document.querySelectorAll('.writer .ProseMirror > *')[index];
      if (handle?.dataset.shown !== 'true' || !block) return false;
      const a = handle.getBoundingClientRect();
      const b = block.getBoundingClientRect();
      return a.top + a.height / 2 >= b.top - 2 && a.top + a.height / 2 <= b.bottom + 2;
    }, index);
  };
  const markdown = async () => {
    await page.getByText('unsaved').waitFor();
    await page.keyboard.press('Control+s');
    await page.waitForFunction(() => !document.body.innerText.includes('unsaved'));
    return readFile(file, 'utf8');
  };

  // ---- pointed at ----------------------------------------------------------------
  // There before anything is pointed at, hidden; pointed at, beside the block, and the text where it was.
  await page.mouse.move(4, 400);
  expect('the handle to be on the page before anything is pointed at, and hidden', (await handle.count()) === 1 && !(await drawn()).shown);
  const atRest = await drawn();
  await pointAt(1);
  const pointed = await drawn();
  expect('no block to have moved as the handle came', same(atRest.blocks, pointed.blocks));
  besideTheText(pointed, 'beside the paragraph');
  expect('both the + and the grip where there is room for both', pointed.parts.length === 2);
  await page.screenshot({ path: `${out}/handled-1-hover.png` });
  // The pointer crosses from the text to the grip, a few pixels at a time: the handle stays, where it was.
  const gripAt = await grip.boundingBox();
  const from = await blocks.nth(1).boundingBox();
  for (let step = 1; step <= 8; step++) {
    await page.mouse.move(from.x + 12 - ((from.x + 12 - (gripAt.x + gripAt.width / 2)) * step) / 8, from.y + 10 + ((gripAt.y + gripAt.height / 2 - from.y - 10) * step) / 8);
    const now = await drawn();
    expect(`the handle to stay as the pointer crosses to it, step ${step}`, now.shown && same(now.handle, pointed.handle));
  }
  // Between two blocks, the nearer: just under the heading is the heading's.
  const heading = await blocks.nth(0).boundingBox();
  await page.mouse.move(heading.x + 12, heading.y + heading.height + 2);
  await page.waitForFunction(() => {
    const handle = document.querySelector('.block-handle').getBoundingClientRect();
    const first = document.querySelector('.writer .ProseMirror > *').getBoundingClientRect();
    return handle.top + handle.height / 2 <= first.bottom + 2;
  });

  // ---- the menu ------------------------------------------------------------------
  await pointAt(1);
  await grip.click();
  await menu.waitFor();
  const lines = await menu.locator('button, .menu-said').allInnerTexts();
  expect(`the menu to say Turn into and then the blocks and what is done, got ${JSON.stringify(lines)}`, same(lines.map((one) => one.replace('✓', '').trim()), ['Turn into', 'Text', 'Heading 1', 'Heading 2', 'Heading 3', 'Bulleted list', 'Numbered list', 'Quote', 'Code', 'Duplicate', 'Move up', 'Move down', 'Comment', 'Delete']));
  expect('what the block is to be marked', (await menu.getByRole('menuitemradio', { name: 'Text' }).getAttribute('aria-checked')) === 'true');
  const opened = await drawn();
  expect('nothing of the text to have moved as the menu opened', same(opened.blocks, atRest.blocks));
  const menuAt = await menu.boundingBox();
  const viewport = page.viewportSize();
  expect(`the menu to be in the window, and it is at ${JSON.stringify(menuAt)}`, menuAt.x >= 0 && menuAt.y >= 0 && menuAt.x + menuAt.width <= viewport.width && menuAt.y + menuAt.height <= viewport.height + 1);
  const gripNow = await grip.boundingBox();
  expect('the menu to open under the grip and not over it', menuAt.y >= gripNow.y + gripNow.height - 1);
  await page.screenshot({ path: `${out}/handled-2-menu.png` });

  // Turn into: the paragraph a heading, keeping what is marked in it.
  await menu.getByRole('menuitemradio', { name: 'Heading 2' }).click();
  await doc.locator('h2', { hasText: 'A shop for well made things.' }).waitFor();
  expect('what was marked in it to be marked still', (await doc.locator('h2 em', { hasText: 'well made' }).count()) === 1);
  expect('the menu put away once it was used', (await menu.count()) === 0);
  await page.screenshot({ path: `${out}/handled-4-turned.png` });
  // The list into text: an item a paragraph each, nothing of it lost; and back into a list, numbered.
  await pointAt(2);
  await grip.click();
  await menu.getByRole('menuitemradio', { name: 'Text' }).click();
  await doc.locator(':scope > p', { hasText: 'knives' }).waitFor();
  expect('each item of the list to be a paragraph of its own', (await doc.locator(':scope > p', { hasText: /^mugs$/ }).count()) === 1 && (await doc.locator('ul').count()) === 0);
  await pointAt(3);
  await grip.click();
  await menu.getByRole('menuitemradio', { name: 'Numbered list' }).click();
  await doc.locator('ol > li', { hasText: 'knives' }).waitFor();
  expect('the paragraph to be a numbered list of one', (await doc.locator('ol > li').count()) === 1);
  let saved = await markdown();
  expect(`the file to say what was turned, got:\n${saved}`, saved === '# Riverside Shop\n\n## A shop for *well made* things.\n\nmugs\n\n1. knives\n\nLast line.\n');

  // ---- carried ---------------------------------------------------------------------
  // The last line carried to the top: nothing moves as the carry begins, a line says where it lands, and the block is fainter.
  const count = await blocks.count();
  await pointAt(count - 1);
  const before = await drawn();
  const g = await grip.boundingBox();
  await page.mouse.move(g.x + g.width / 2, g.y + g.height / 2);
  await page.mouse.down();
  await page.mouse.move(g.x + g.width / 2, g.y + g.height / 2 - 6);
  const begun = await drawn();
  expect('nothing of the text to have moved as the carry began', same(begun.blocks, before.blocks));
  expect('the handle to be where it was, under the pointer, as the carry began', same(begun.handle, before.handle));
  const top = await blocks.nth(0).boundingBox();
  await page.mouse.move(g.x + g.width / 2, top.y + 4, { steps: 6 });
  const line = page.locator('.block-drop');
  await line.waitFor();
  const lineAt = await line.boundingBox();
  expect(`the line to be over the first block and as wide as the column, at ${JSON.stringify(lineAt)}`, lineAt.y + lineAt.height <= top.y + 1 && Math.abs(lineAt.width - before.column.width) <= 1);
  expect('the carried block to be drawn fainter', (await doc.locator('.block-carried').count()) === 1);
  expect('nothing of the text to have moved while it is carried', same((await drawn()).blocks, before.blocks));
  await page.screenshot({ path: `${out}/handled-3-drag.png` });
  await page.mouse.up();
  await doc.locator(':scope > :first-child', { hasText: 'Last line.' }).waitFor();
  expect('no line and nothing fainter once it is let go', (await line.isHidden()) && (await doc.locator('.block-carried').count()) === 0);
  expect('the menu not opened by the carry', (await menu.count()) === 0);
  // Escape in the middle of a carry puts it back, and moves nothing.
  await pointAt(1);
  const again = await grip.boundingBox();
  await page.mouse.move(again.x + again.width / 2, again.y + again.height / 2);
  await page.mouse.down();
  const lastBlock = await blocks.nth(count - 1).boundingBox();
  await page.mouse.move(again.x + again.width / 2, lastBlock.y + lastBlock.height - 2, { steps: 6 });
  await line.waitFor();
  await page.keyboard.press('Escape');
  await line.waitFor({ state: 'hidden' });
  await page.mouse.up();
  expect('Escape to have put the carried block back where it was', (await blocks.nth(1).innerText()).startsWith('Riverside Shop'));
  saved = await markdown();
  expect(`the file to have the last line first, got:\n${saved}`, saved.startsWith('Last line.\n\n# Riverside Shop\n'));

  // ---- + ------------------------------------------------------------------------------
  await pointAt(1);
  await add.click();
  await page.locator('.block-menu button').first().waitFor();
  expect('a line under the heading with the / menu open on it', (await blocks.nth(2).innerText()) === '/');
  await page.keyboard.type('quo');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Said under the heading.');
  await doc.locator(':scope > blockquote', { hasText: 'Said under the heading.' }).waitFor();
  expect('the quote to be under the heading', (await blocks.nth(2).evaluate((one) => one.tagName)) === 'BLOCKQUOTE');
  expect('the handle put away while writing', !(await drawn()).shown);

  // ---- the keyboard ----------------------------------------------------------------
  // Ctrl and / in a block opens its menu with the keyboard in it; Escape gives it to the grip, and again to the text.
  await doc.locator(':scope > p', { hasText: 'mugs' }).click();
  await page.waitForTimeout(150);
  await page.keyboard.press('Control+/');
  await menu.waitFor();
  expect('the keyboard to be in the menu', await page.evaluate(() => Boolean(document.activeElement?.closest('.row-menu'))));
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => document.activeElement?.classList.contains('block-grip'));
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => Boolean(document.activeElement?.closest('.ProseMirror')));
  await page.keyboard.press('Control+/');
  await menu.waitFor();
  await page.keyboard.press('End');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  expect('the arrows to go through the menu', (await page.evaluate(() => document.activeElement?.textContent)) === 'Move down');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('Enter');
  await doc.locator(':scope > p', { hasText: /^mugs$/ }).nth(1).waitFor();
  expect('the block duplicated under itself, and the cursor in the text', (await doc.locator(':scope > p', { hasText: /^mugs$/ }).count()) === 2 && (await page.evaluate(() => Boolean(document.activeElement?.closest('.ProseMirror')))));
  // Delete takes the copy away again; Move up takes the numbered list over the paragraph.
  await pointAt(5);
  await grip.click();
  await menu.getByRole('menuitem', { name: 'Delete' }).click();
  await page.waitForFunction(() => [...document.querySelectorAll('.writer .ProseMirror > p')].filter((one) => one.textContent === 'mugs').length === 1);
  await pointAt(5);
  await grip.click();
  await menu.getByRole('menuitem', { name: 'Move up' }).click();
  await page.waitForFunction(() => document.querySelectorAll('.writer .ProseMirror > *')[4]?.tagName === 'OL');
  saved = await markdown();
  expect(
    `the file to be what was done, and no more, got:\n${saved}`,
    saved === 'Last line.\n\n# Riverside Shop\n\n> Said under the heading.\n\n## A shop for *well made* things.\n\n1. knives\n\nmugs\n',
  );

  // ---- comment ---------------------------------------------------------------------
  // Begun in the panel beside the document, opened from the speech bubble at the end of its head's row.
  const bubble = page.locator('.view-head .head-acts .head-tool');
  await bubble.click();
  const adding = page.locator('.doc-tools[data-rest]').getByRole('region', { name: 'Comments' }).getByLabel('Add a comment');
  await adding.fill('Earlier words.');
  await pointAt(1);
  await grip.click();
  await menu.getByRole('menuitem', { name: 'Comment' }).click();
  await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Add a comment');
  expect(`the comment to name the block after what was written, got "${await adding.inputValue()}"`, (await adding.inputValue()) === 'Earlier words.\nOn “Riverside Shop”: ');
  await page.waitForFunction(() => {
    const area = document.activeElement;
    return area?.selectionStart === area?.value.length;
  });
  await page.keyboard.type('needs a picture.');
  await page.keyboard.press('Enter');
  await page.locator('.doc-tools .comment-words', { hasText: 'On “Riverside Shop”: needs a picture.' }).waitFor();
  // And put away, with the text as wide as it was.
  await bubble.click();
  await page.locator('.doc-tools, .doc-tools-leaving').waitFor({ state: 'detached' });
  // Begun from the block's menu with the panel put away, it is opened on its comments with the keyboard in the line.
  await pointAt(1);
  await grip.click();
  await menu.getByRole('menuitem', { name: 'Comment' }).click();
  await page.waitForFunction(() => document.activeElement?.closest('.doc-tools') && document.activeElement.getAttribute('aria-label') === 'Add a comment');
  expect(`the comment begun again to name the block, got "${await adding.inputValue()}"`, (await adding.inputValue()) === 'On “Riverside Shop”: ');
  await adding.fill('');
  await bubble.click();
  await page.locator('.doc-tools, .doc-tools-leaving').waitFor({ state: 'detached' });

  // ---- a narrow tab: the grip alone, over nothing ---------------------------------------
  await page.setViewportSize({ width: 820, height: 760 });
  await page.waitForTimeout(100);
  await pointAt(1);
  const narrow = await drawn();
  besideTheText(narrow, 'in a narrow tab');
  expect(`the grip alone where only it has room, got ${narrow.parts.length} parts`, narrow.parts.length === 1);
  await page.screenshot({ path: `${out}/handled-6-narrow.png` });

  // ---- a phone -----------------------------------------------------------------------
  const phone = await browser.newContext({ storageState: await context.storageState(), viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: process.env.SCHEME ?? 'dark' });
  const small = await phone.newPage();
  small.on('pageerror', (err) => problems.push(`at a phone's width: ${err.message}`));
  await small.goto(url);
  const smallDoc = small.locator('.writer .ProseMirror');
  await smallDoc.locator('h1', { hasText: 'Riverside Shop' }).waitFor();
  expect('nothing of the handle drawn before the text is touched', !(await drawn(small)).shown);
  await smallDoc.locator('h1').tap();
  await small.waitForFunction(() => document.querySelector('.block-handle')?.dataset.shown === 'true');
  const touched = await drawn(small);
  besideTheText(touched, "at a phone's width");
  expect('the grip alone on a phone', touched.parts.length === 1);
  expect(`the grip a finger's size, got ${JSON.stringify(touched.parts[0])}`, touched.parts[0].width >= 28 && touched.parts[0].height >= 32);
  await small.screenshot({ path: `${out}/handled-5-phone.png` });
  await small.locator('.block-handle').getByRole('button', { name: 'Drag to move, or open the menu' }).tap();
  await small.getByRole('menu', { name: 'Block' }).waitFor();
  const phoneMenu = await small.getByRole('menu', { name: 'Block' }).boundingBox();
  expect(`the menu in the window on a phone, at ${JSON.stringify(phoneMenu)}`, phoneMenu.x >= 0 && phoneMenu.x + phoneMenu.width <= 390 && phoneMenu.y + phoneMenu.height <= 845);
  await small.screenshot({ path: `${out}/handled-5-phone-menu.png` });

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
