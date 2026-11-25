// What is attached to a card, in the Attachments tab of its drawer
// (`views/recordfiles.tsx`, over `ui/src/recordfiles.ts`).
//
//   DAEMON=../target/debug/factory node scripts/attached.mjs [out-dir]
//
// Ada, who owns the factory, on a page that is never read again, and
// from a terminal. A card with a log attached from the terminal: its
// drawer's one row of tabs says Details, Comments, History and
// Attachments (1). Opened, each is said with how big it is, who added it
// and when; the log is read in place. A picture pasted into the tab and a
// file dropped on it are added, and the tab says how many; the picture is
// drawn in place, a video is given what plays it, and anything else is
// saved and said to be. One attached from the terminal is listed with
// the page not read again, and one taken away goes into the Trash with
// Undo, which gives it back, and is there by its file and its card. Somebody who may only
// read the board is offered nothing to add or take away. A row of another
// table has the same tabs at the top of its form, Attachments (1) among
// them, and the log listed there. Nothing of the
// drawer lies over anything else, at three widths and a phone's, plainly
// and with `LOOK="size=largest density=compact"`. The environment
// variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'attached-data');
const home = join(out, 'attached-home');
const looked = process.env.LOOK ? '-looked' : '';

/** A picture a pixel across, which a browser draws. */
/** A table of the project's own, whose rows carry what is attached as a card does. */
const NOTES = `blueprint: notes
permissions: [projects, records]
steps:
  notes:
    do: collection.ensure
    with:
      name: notes
      fields:
        title: { type: text, required: true }
        done: { type: flag }
`;

const PIXEL = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('board', 'add', 'shop', 'Fix the till');
  const log = join(home, 'till.log');
  await writeFile(log, 'the till said no\nand then it said yes\n');
  await cli('record', 'attach', 'shop-1', log);
  const notes = join(home, 'notes.bp');
  await writeFile(notes, NOTES);
  await cli('apply', notes, '--project', 'shop');
  await cli('record', 'add', 'shop', 'notes', 'title=Till numbers');
  await cli('record', 'attach', 'shop-notes-1', log);

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
  // `holds`, where it is said, is what the page is told Ada holds: somebody who may read the board and change nothing.
  const signedIn = async (how = { viewport: { width: 1280, height: 860 } }, holds = null) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    lastPage = page;
    page.on('pageerror', (err) => problems.push(err.message));
    if (holds) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = holds;
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const opened = async (page, tap = false) => {
    await page.evaluate(() => (location.hash = '#/board/shop'));
    const card = page.locator('.board .card', { hasText: 'Fix the till' });
    await (tap ? card.tap() : card.click());
    const drawer = drawerAtRest(page);
    await drawer.getByRole('tab', { name: 'Details', exact: true }).waitFor();
    return drawer;
  };
  const tab = (drawer) => drawer.locator('.card-tabs [role="tab"][data-tab="@attachments"]');
  const crowded = async (page, what) => {
    const wrong = await pageCrowded(page, '.drawer');
    expect(`nothing of the drawer over anything else ${what}${looked}, got ${wrong.join('; ')}`, wrong.length === 0);
  };
  const listed = (drawer) => drawer.locator('.record-file .record-file-name').allInnerTexts();

  // ── The tab, its count, and what was attached from a terminal ──────────
  const page = await signedIn();
  let loads = 0;
  page.on('load', () => (loads += 1));
  const drawer = await opened(page);
  await tab(drawer).filter({ hasText: 'Attachments (1)' }).waitFor();
  const names = (await drawer.locator('.card-tabs [role="tablist"] [role="tab"]').allInnerTexts()).map((one) => one.trim());
  expect(`Details, Comments, History and Attachments (1) in one row, got ${JSON.stringify(names)}`, JSON.stringify(names) === JSON.stringify(['Details', 'Comments', 'History', 'Attachments (1)']));
  await tab(drawer).click();
  const files = drawer.locator('.record-files');
  await files.locator('.record-file').first().waitFor();
  const said = (await files.locator('.record-file-said').first().innerText()).trim();
  expect(`the log said with how big it is, who added it and when, got "${said}"`, /^\d+ bytes · Ada · /.test(said));
  await files.locator('.record-file-name', { hasText: 'till.log' }).click();
  await files.locator('.record-file-words', { hasText: 'and then it said yes' }).waitFor();
  await crowded(page, 'with the log read at 1280');
  await page.screenshot({ path: `${out}/attached-0-log${looked}.png` });

  // ── A picture pasted, and files dropped ─────────────────────────────────
  await files.locator('.record-files-add').focus();
  await page.evaluate((b64) => {
    const bytes = Uint8Array.from(atob(b64), (one) => one.charCodeAt(0));
    const carried = new DataTransfer();
    carried.items.add(new File([bytes], 'image.png', { type: 'image/png' }));
    document.querySelector('.record-files-add').dispatchEvent(new ClipboardEvent('paste', { clipboardData: carried, bubbles: true, cancelable: true }));
  }, PIXEL);
  await tab(drawer).filter({ hasText: 'Attachments (2)' }).waitFor();
  const pasted = drawer.locator('img.record-file-shown');
  await pasted.waitFor();
  await page.waitForFunction(() => document.querySelector('img.record-file-shown')?.naturalWidth === 1);
  const pastedName = (await listed(drawer))[1];
  expect(`the pasted picture called by when it was pasted, got ${pastedName}`, /^pasted-\d{8}-\d{6}\.png$/.test(pastedName));
  await page.screenshot({ path: `${out}/attached-1-pasted${looked}.png` });
  const drop = (name, type, words) =>
    page.evaluate(
      ([name, type, words]) => {
        const carried = new DataTransfer();
        carried.items.add(new File([words], name, { type }));
        const on = document.querySelector('.record-files');
        on.dispatchEvent(new DragEvent('dragover', { dataTransfer: carried, bubbles: true, cancelable: true }));
        on.dispatchEvent(new DragEvent('drop', { dataTransfer: carried, bubbles: true, cancelable: true }));
      },
      [name, type, words],
    );
  await drop('clip.mp4', 'video/mp4', 'not really a video');
  await tab(drawer).filter({ hasText: 'Attachments (3)' }).waitFor();
  await drawer.locator('video.record-file-shown[controls]').waitFor();
  await drop('till.bin', '', 'bytes');
  await tab(drawer).filter({ hasText: 'Attachments (4)' }).waitFor();
  await drawer.locator('p.record-file-shown', { hasText: 'save it to open it' }).waitFor();
  const saving = await drawer.locator('.record-file', { hasText: 'till.bin' }).locator('a.record-file-save').getAttribute('download');
  expect(`what nothing here shows saved by its name, got ${saving}`, saving === 'till.bin');
  expect(`four listed in the order they were added, got ${JSON.stringify(await listed(drawer))}`, JSON.stringify(await listed(drawer)) === JSON.stringify(['till.log', pastedName, 'clip.mp4', 'till.bin']));
  await crowded(page, 'with four attached at 1280');
  await page.screenshot({ path: `${out}/attached-2-dropped${looked}.png` });

  // ── From a terminal, followed with the page not read again ──────────────
  await writeFile(join(home, 'more.txt'), 'more words');
  await cli('record', 'attach', 'shop-1', join(home, 'more.txt'));
  await tab(drawer).filter({ hasText: 'Attachments (5)' }).waitFor();
  // Taken away, into the Trash with Undo, which gives it back; taken away again, it is in the Trash by its file and its card.
  const remove = () => drawer.locator('.record-file', { hasText: 'till.bin' }).getByRole('button', { name: 'Remove' }).click();
  await remove();
  await tab(drawer).filter({ hasText: 'Attachments (4)' }).waitFor();
  const moved = page.locator('.toast', { hasText: 'till.bin: Moved to Trash, kept 30 days.' });
  await moved.waitFor();
  await page.screenshot({ path: `${out}/attached-3-removed${looked}.png` });
  await moved.getByRole('button', { name: 'Undo', exact: true }).click();
  await tab(drawer).filter({ hasText: 'Attachments (5)' }).waitFor();
  await drawer.locator('.record-file', { hasText: 'till.bin' }).waitFor();
  await page.locator('.toast', { hasText: 'till.bin is back.' }).waitFor();
  await remove();
  await tab(drawer).filter({ hasText: 'Attachments (4)' }).waitFor();
  expect('the page never read again', loads === 0);
  const other = await signedIn();
  const resources = other.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(other);
  await other.getByRole('navigation', { name: 'Resources' }).getByRole('button', { name: /Trash$/ }).click();
  const held = other.locator('.trash-kind[data-kind="attachment"] .trash-thing', { hasText: 'till.bin' });
  await held.waitFor();
  const where = (await held.locator('.trash-where').innerText()).trim();
  expect(`the Trash to name it by its file and its card, got "${where}"`, where === 'attached to shop-1 of Shop');
  await other.screenshot({ path: `${out}/attached-4-trash${looked}.png` });
  await other.context().close();

  // ── Nothing over anything else at three widths and a phone's ────────────
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await drawer.waitFor();
    await crowded(page, `at ${width}`);
  }
  await page.screenshot({ path: `${out}/attached-5-narrow${looked}.png` });
  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  const small = await opened(phone, true);
  await tab(small).tap();
  await small.locator('.record-file').first().waitFor();
  await crowded(phone, 'on a phone');
  await phone.screenshot({ path: `${out}/attached-6-phone${looked}.png` });

  // ── A row of another table: the same one row of tabs at the top of its form ──
  await page.evaluate(() => (location.hash = '#/table/shop/notes'));
  const noteRow = page.locator('tr[data-row]', { hasText: 'Till numbers' });
  await noteRow.waitFor();
  await noteRow.hover();
  await page.getByRole('button', { name: 'Open Till numbers', exact: true }).click();
  const rowDrawer = drawerAtRest(page);
  const rowTab = rowDrawer.locator('.record-tabs [role="tab"][data-tab="@attachments"]');
  await rowTab.filter({ hasText: 'Attachments (1)' }).waitFor();
  const rowNames = (await rowDrawer.locator('.record-tabs [role="tablist"] [role="tab"]').allInnerTexts()).map((one) => one.trim());
  expect(`a row's form to have Details, Comments, History and Attachments (1) in one row, got ${JSON.stringify(rowNames)}`, JSON.stringify(rowNames) === JSON.stringify(['Details', 'Comments', 'History', 'Attachments (1)']));
  expect("Details showing first, with the row's fields", (await rowDrawer.getByRole('tab', { name: 'Details', exact: true }).getAttribute('aria-selected')) === 'true' && (await rowDrawer.getByRole('switch', { name: 'Done', exact: true }).isVisible()));
  await rowTab.click();
  await rowDrawer.locator('.record-file .record-file-name', { hasText: 'till.log' }).waitFor();
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    await crowded(page, `in a row's Attachments at ${width}`);
  }
  await page.screenshot({ path: `${out}/attached-8-row${looked}.png` });
  await page.setViewportSize({ width: 1280, height: 860 });

  // ── Somebody who may only read the board ────────────────────────────────
  const reader = await signedIn(undefined, ['projects', 'channels', 'board:read', 'records:read', 'search']);
  const theirs = await opened(reader);
  await tab(theirs).click();
  await theirs.locator('.record-file').first().waitFor();
  expect('nothing to add a file with', (await theirs.locator('.record-files-add').count()) === 0);
  expect('nothing to take one away with', (await theirs.getByRole('button', { name: 'Remove' }).count()) === 0);
  expect('each still to be saved', (await theirs.locator('a.record-file-save').count()) === 4);
  await reader.screenshot({ path: `${out}/attached-7-reader${looked}.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/attached-failed${looked}.png` }).catch(() => {});
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
