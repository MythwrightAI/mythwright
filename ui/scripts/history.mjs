// A topic's page read until Edit is pressed, and its History.
//
//   DAEMON=../target/debug/factory node scripts/history.mjs [out-dir]
//
// Ada owns the factory and makes from a terminal a topic with no status and
// a post (an article), Rolling back, and a question tracked as one, Which
// linker (a thread). Opened, the article is read as anybody reads it, with
// Edit at its head reached by the keyboard. Edit opens it to be written,
// the cursor in it; what is typed is not saved yet, and is kept while Talk
// is shown and Article again; Cancel asks before what was typed goes, and
// Keep editing keeps it; Save keeps it and the article is read again, with
// what was typed. History, the third tab, lists the two revisions newest
// first, who made each and what it changed; the two chosen and compared
// show the words added marked, said as added to whoever hears the page.
// The first revision opened shows what it said, and Restore this version,
// asked once more, brings its words back as a third revision, which says
// it restored the first. `#history` opens the history by an address, and
// `#history/2` the second revision, which the address bar says as it is
// shown. The thread has no tabs; its post is read, with Edit at its head;
// its summary is read, and written by the small Edit beside it, reached by
// the keyboard; History at its head shows its history in its place, with
// no Edit beside it, a revision there calling what it says its Post, and
// its replies again. Somebody who may only
// read is shown the article as before, no Edit, and no Restore. Nothing of
// the page lies over anything else or scrolls sideways, on a desk and at a
// phone's width. The environment variables are those of walkthrough.mjs
// (`SCHEME`, `LOOK` among them).
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
const data = join(out, 'history-data');
const home = join(out, 'history-home');
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

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  const back = await made('Rolling back', '--body', 'Roll back by the tag.');
  const linker = await made('Which linker', '--status', 'open', '--body', 'Which linker is the fastest here?');
  await cli('topic', 'reply', linker, 'mold, by a long way.');
  await cli('record', 'set', linker, 'summary=mold is the fastest.');

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
  /** A page signed in at `to`; one that `reads` is told whoever is signed in holds no `records:write`. */
  const signedIn = async (to, { how = DESK, reads = false } = {}) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    if (reads) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = me.principal.scopes.filter((scope) => !['*', 'records', 'records:write'].includes(scope)).concat(['records:read', 'channels:read', 'inbox:read']);
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
    for (let times = 0; times < 60; times += 1) {
      await on.keyboard.press('Tab');
      if (await locator.evaluate((one) => one === document.activeElement).catch(() => false)) return;
    }
    throw new Error(`expected ${what} to be reached by Tab`);
  };
  const tab = (on, name) => on.locator('.topic-tabs [role="tab"]', { hasText: name });

  // ── Read until Edit is pressed ──────────────────────────────────────────
  const page = await signedIn(at('Rolling back'));
  const article = page.locator(`.topic-page[data-topic="${back}"]`);
  await shown(article.locator('.topic-post .topic-doc-read', { hasText: 'Roll back by the tag.' }), 'the article read as it is');
  expect('no editor while it is read', (await article.locator('.topic-post .ProseMirror').count()) === 0);
  const edit = article.locator('.topic-page-head button.topic-edit');
  await shown(edit, 'Edit at the head of an article whoever may change it reads');
  expect('three tabs: Article, Talk and History', (await page.locator('.topic-tabs [role="tab"]').allInnerTexts()).map((one) => one.replace(/\s+\d+$/, '').trim()).join(' | ') === 'Article | Talk | History');
  await laid(page, 'as the article is read');
  await page.screenshot({ path: `${out}/history-01-read${looked}.png` });

  await tabTo(page, edit, 'Edit');
  await page.keyboard.press('Enter');
  const writer = article.locator('.topic-post .ProseMirror');
  await shown(writer, 'the editor once Edit is pressed');
  await page.waitForFunction(() => Boolean(document.activeElement?.closest('.topic-post .ProseMirror')), null, { timeout: 10000 }).catch(() => {
    throw new Error('expected the cursor in the article once Edit is pressed');
  });
  expect('no Edit at the head while it is written', (await edit.count()) === 0);
  await writer.locator('p').last().click();
  await page.keyboard.press('End');
  await page.keyboard.type(' Use the release tag.');
  await shown(article.locator('.topic-doc.unsaved .topic-doc-acts', { hasText: 'Not saved yet' }), 'what was typed said to be not saved yet');
  await page.screenshot({ path: `${out}/history-02-editing${looked}.png` });

  // What was typed is kept while Talk is shown, and Article again.
  await tab(page, 'Talk').click();
  await gone(writer, 'the editor put away while Talk shows');
  await tab(page, 'Article').click();
  await shown(article.locator('.topic-post .ProseMirror', { hasText: 'Use the release tag.' }), 'what was typed kept through Talk and back');
  await shown(article.locator('.topic-doc-acts', { hasText: 'Not saved yet' }), 'still not saved');

  // Cancel asks first, and Keep editing keeps it.
  await article.locator('.topic-doc-acts').getByRole('button', { name: 'Cancel', exact: true }).click();
  const discard = page.getByRole('dialog');
  await shown(discard.getByText('Discard your changes?'), 'Cancel asking before what was typed goes');
  await page.screenshot({ path: `${out}/history-03-cancel-asks${looked}.png` });
  await discard.getByRole('button', { name: 'Keep editing' }).click();
  await gone(page.locator('.modal'), 'the question put away');
  await shown(article.locator('.topic-post .ProseMirror', { hasText: 'Use the release tag.' }), 'what was typed kept by Keep editing');

  // Save keeps it, and it is read again.
  await article.locator('.topic-doc-acts').getByRole('button', { name: 'Save', exact: true }).click();
  await shown(article.locator('.topic-post .topic-doc-read', { hasText: 'Roll back by the tag. Use the release tag.' }), 'the article read again with what was saved');
  expect('no editor once it is saved', (await writer.count()) === 0);
  await shown(edit, 'Edit at the head again');

  // ── History ─────────────────────────────────────────────────────────────
  await tabTo(page, tab(page, 'History'), 'the History tab');
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => location.hash === '#history', null, { timeout: 10000 });
  const lines = page.locator('.topic-history-line');
  await page.waitForFunction(() => document.querySelectorAll('.topic-history-line').length === 2, null, { timeout: 10000 }).catch(async () => {
    throw new Error(`expected two revisions, got ${await lines.count()}`);
  });
  const said = await lines.allInnerTexts();
  expect(`the newest first, each with who made it and what it changed, got ${JSON.stringify(said)}`, /^2\s*Ada.*Article.*Current/s.test(said[0]) && /^1\s*Ada.*Made/s.test(said[1]));
  expect('when each was made, whole on hover', Boolean(await lines.first().locator('time').getAttribute('title')));
  await laid(page, 'as the history is listed');
  await page.screenshot({ path: `${out}/history-04-list${looked}.png` });

  // Two chosen, by the keyboard, and compared.
  const compare = page.getByRole('button', { name: 'Compare', exact: true });
  expect('Compare closed until two are chosen', await compare.isDisabled());
  for (const nth of [2, 1]) {
    await page.getByRole('checkbox', { name: `Choose revision ${nth} to compare` }).focus();
    await page.keyboard.press('Space');
  }
  await compare.click();
  const added = page.locator('.topic-history-change[data-part="article"] .topic-diff-added');
  await shown(added, 'the words added marked');
  const words = (await added.evaluate((one) => [...one.childNodes].filter((node) => !node.classList?.contains('topic-diff-said')).map((node) => node.textContent).join(''))).trim();
  expect(`the words added, got ${words}`, words === 'Use the release tag.');
  expect('nothing removed', (await page.locator('.topic-history-change .topic-diff-removed').count()) === 0);
  expect('the words added said as added to whoever hears the page', (await added.textContent()).startsWith('added: '));
  expect('only the article changed', (await page.locator('.topic-history-change').count()) === 1);
  const marks = await added.evaluate((one) => {
    const style = getComputedStyle(one);
    return { wash: style.backgroundColor, text: style.color, said: getComputedStyle(one.querySelector('.topic-diff-said')).clipPath };
  });
  expect(`the added words washed and readable, got ${JSON.stringify(marks)}`, marks.wash !== 'rgba(0, 0, 0, 0)' && marks.said.includes('inset'));
  await laid(page, 'as two revisions are compared');
  await page.screenshot({ path: `${out}/history-05-compared${looked}.png` });

  // The first opened, and restored.
  await page.getByRole('button', { name: '← All revisions' }).click();
  await lines.filter({ hasText: 'Made' }).locator('.topic-history-open').click();
  await page.waitForFunction(() => location.hash === '#history/1', null, { timeout: 10000 }).catch(async () => {
    throw new Error(`expected the address to say the revision shown, got ${await page.evaluate(() => location.hash)}`);
  });
  await shown(page.locator('.topic-history-part[aria-label="Article"] .topic-doc-read', { hasText: 'Roll back by the tag.' }), 'the first revision as it read');
  const restore = page.getByRole('button', { name: 'Restore this version' });
  await shown(restore, 'Restore this version on an earlier revision');
  await page.screenshot({ path: `${out}/history-06-revision${looked}.png` });
  await restore.focus();
  await page.keyboard.press('Enter');
  await shown(page.getByRole('dialog').getByText('Restore this version?'), 'restoring asked first');
  await page.screenshot({ path: `${out}/history-07-restore-asks${looked}.png` });
  await page.getByRole('dialog').getByRole('button', { name: 'Restore', exact: true }).click();
  await page.waitForFunction(() => document.querySelectorAll('.topic-history-line').length === 3, null, { timeout: 10000 }).catch(async () => {
    throw new Error(`expected a third revision once restored, got ${await lines.count()}`);
  });
  expect(`the third saying it restored the first, got ${await lines.first().innerText()}`, /Restored from revision 1/.test(await lines.first().innerText()));
  await tab(page, 'Article').click();
  await shown(article.locator('.topic-post .topic-doc-read', { hasText: /^Roll back by the tag\.$/ }), 'the first revision’s words back in the article');
  const { token } = JSON.parse((await cli('key', 'history', '--json')).stdout);
  const { revisions } = await (await fetch(`${url}/api/topics/${encodeURIComponent(back)}/revisions`, { headers: { authorization: `Bearer ${token}` } })).json();
  expect(`three revisions kept by the daemon, got ${revisions.length}`, revisions.length === 3 && revisions[2].body === 'Roll back by the tag.');

  // ── By its address ──────────────────────────────────────────────────────
  await page.goto(at('Rolling back', '#history'));
  await shown(page.locator('.topic-history-line').nth(2), 'the history opened by `#history`');
  expect('History the tab shown', (await tab(page, 'History').getAttribute('aria-selected')) === 'true');
  await page.goto(at('Rolling back', '#history/2'));
  await shown(page.locator('.topic-history-heading', { hasText: 'Revision 2' }), 'the second revision opened by `#history/2`');
  await shown(page.locator('.topic-history-part[aria-label="Article"] .topic-doc-read', { hasText: 'Use the release tag.' }), 'the second revision as it read');

  // ── A thread: no tabs, its post read until Edit, its history in its place ─
  await page.goto(at('Which linker'));
  const thread = page.locator(`.topic-page[data-topic="${linker}"]`);
  await shown(thread.locator('.topic-reply', { hasText: 'mold, by a long way.' }), 'the thread’s replies');
  expect('the thread is one still', (await thread.getAttribute('data-shows')) === 'thread');
  expect('no tabs on a thread', (await thread.locator('.topic-tabs').count()) === 0);
  // A tracked question is a thread whatever its post says: its post is read, and Edit at its head opens it.
  await shown(thread.locator('.topic-post .topic-doc-read', { hasText: 'Which linker is the fastest here?' }), 'the question’s post read on its thread');
  expect('Edit at the head of a question with a post, no editor while it is read', (await thread.locator('.topic-page-head button.topic-edit').count()) === 1 && (await thread.locator('.topic-post .ProseMirror').count()) === 0);
  // Its summary read, and written by the small Edit beside it, reached by the keyboard.
  const band = thread.locator('.topic-summary');
  await shown(band.locator('.topic-doc-read', { hasText: 'mold is the fastest.' }), 'the thread’s summary read');
  const editSummary = band.getByRole('button', { name: 'Edit summary' });
  await tabTo(page, editSummary, 'Edit summary');
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => Boolean(document.activeElement?.closest('.topic-summary .ProseMirror')), null, { timeout: 10000 }).catch(() => {
    throw new Error('expected the cursor in the summary once its Edit is pressed');
  });
  await page.keyboard.press('Control+End');
  await page.keyboard.type(' Then lld.');
  await band.getByRole('button', { name: 'Save', exact: true }).click();
  await shown(band.locator('.topic-doc-read', { hasText: 'mold is the fastest. Then lld.' }), 'the summary read again as it was saved');
  const toggle = thread.locator('.topic-page-head button.topic-history-toggle');
  await toggle.click();
  await shown(thread.locator('.topic-history-line', { hasText: 'Summary' }).first(), 'the thread’s history in its place, the summary’s change in it');
  expect('History said pressed, and no Edit while it shows', (await toggle.getAttribute('aria-pressed')) === 'true' && (await thread.locator('button.topic-edit').count()) === 0);
  await laid(page, 'as a thread’s history is shown');
  await page.screenshot({ path: `${out}/history-08-thread${looked}.png` });
  // What a question says is its post, in its history as on its thread.
  await thread.locator('.topic-history-line').last().locator('.topic-history-open').click();
  await shown(thread.locator('.topic-history-part[aria-label="Post"] .topic-doc-read', { hasText: 'Which linker is the fastest here?' }), 'the question’s first revision, what it says called its Post');
  expect('no part called Article in a question’s history', (await thread.locator('.topic-history-part[aria-label="Article"]').count()) === 0);
  await toggle.click();
  await shown(thread.locator('.topic-reply', { hasText: 'mold, by a long way.' }), 'the thread again once History is pressed again');

  // ── Somebody who may only read ──────────────────────────────────────────
  const reader = await signedIn(at('Rolling back'), { reads: true });
  const read = reader.locator(`.topic-page[data-topic="${back}"]`);
  await shown(read.locator('.topic-post .topic-doc-read', { hasText: 'Roll back by the tag.' }), 'the article read by somebody who may only read');
  expect('no Edit for them', (await read.locator('button.topic-edit').count()) === 0);
  await reader.goto(at('Rolling back', '#history/1'));
  await shown(reader.locator('.topic-history-heading', { hasText: 'Revision 1' }), 'a revision shown to them');
  expect('no Restore for them', (await reader.getByRole('button', { name: 'Restore this version' }).count()) === 0);

  // ── A phone ─────────────────────────────────────────────────────────────
  const phone = await signedIn(at('Rolling back', '#history'), { how: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 } });
  await shown(phone.locator('.topic-history-line').nth(2), 'the history on a phone');
  await laid(phone, "at a phone's width");
  await phone.screenshot({ path: `${out}/history-09-phone${looked}.png` });
  await phone.getByRole('checkbox', { name: 'Choose revision 3 to compare' }).tap();
  await phone.getByRole('checkbox', { name: 'Choose revision 2 to compare' }).tap();
  await phone.getByRole('button', { name: 'Compare', exact: true }).tap();
  await shown(phone.locator('.topic-history-change .topic-diff-removed'), 'what the restore took out, marked on a phone');
  await laid(phone, "as two are compared at a phone's width");
  await phone.screenshot({ path: `${out}/history-10-phone-compared${looked}.png` });

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
