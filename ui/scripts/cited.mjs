// Links between pages and sources in a topic.
//
//   DAEMON=../target/debug/factory node scripts/cited.mjs [out-dir]
//
// Ada opens the post of "Frogs" by Edit and writes in the block editor:
// `[[chy` offers the pages by title, Enter puts in a link to "Chytrid
// fungus", and `[^1]`
// with a line `[^1]: …` under it is a footnote, drawn as a raised number
// whose source shows in a card while it is pointed at. Saved, the post is
// the text `[Chytrid fungus](topic:<id>)` and the footnote as written,
// and its article ends with References. Pressed, the link opens "Chytrid
// fungus", whose article says it is linked from "Frogs". The page linked
// to is renamed and moved, and the link, read by somebody who may only
// read, still leads there, with its words as written; its marker,
// focused, shows its source, and the source's address is a link that is
// never fetched. In a reply, `[[amph` offers "Amphibians" and the link
// is posted; a link to a page that is not there is drawn as a page you
// cannot open, without the words written. Each topic says something, so
// each is an article: its replies are on Talk. A question is a thread, with
// no tabs, and its post's sources are listed under it, over its replies.
// A link to a page, pointed at a
// moment, shows a card of where it leads: the page's title, its wiki and
// what its summary (or, with none, its article) says, read once however
// often it is pointed at; reached by the keyboard it shows the same, and
// Escape puts it away. A link to a reply shows who said it and what; a
// link to a page one cannot open shows none; on a phone the card fits the
// window from the keyboard, and a tap opens the page with no card. Nothing on the page lies over
// anything else, at 1280 wide and a phone's, plainly and with `LOOK="size=largest density=compact"`,
// in the dark and (`SCHEME=light`) the light. The environment variables are
// those of walkthrough.mjs.
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
const data = join(out, 'cited-data');
const home = join(out, 'cited-home');
const looked = process.env.LOOK ? '-looked' : '';
const scheme = process.env.SCHEME ?? 'dark';
const shot = (name) => `${out}/cited-${name}${looked}${scheme === 'light' ? '-light' : ''}.png`;

const SOURCE = 'Ada Lovelace, *Notes*, Taylor, 1843. https://example.org/notes (checked 2026-11-25)';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const made = async (...args) => JSON.parse((await cli('topic', 'make', ...args, '--json')).stdout).id;

let failed = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  const amphibians = await made('Amphibians', '--body', 'Frogs, toads and newts.');
  const fungus = await made('Chytrid fungus', '--under', amphibians, '--body', 'A fungus of the skin.');
  const frogs = await made('Frogs', '--under', amphibians, '--body', 'Many frogs died.\n');
  const toads = await made('Toads', '--under', amphibians, '--body', 'The article about toads.');
  await cli('record', 'set', toads, 'summary=Warty frogs of **dry** places.');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const elsewhere = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  /** Ada's page at a topic; with `holds`, the page is told she holds only those, as somebody who may only read. */
  const signedIn = async (topic, how = { viewport: { width: 1280, height: 860 } }, holds = null) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    lastPage = page;
    page.on('pageerror', (err) => problems.push(err.message));
    // Nothing is fetched from anywhere but the daemon.
    page.on('request', (asked) => !asked.url().startsWith(url) && !/^(data|blob|about):/.test(asked.url()) && elsewhere.push(asked.url()));
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
    await page.goto(`${url}/topic/${encodeURIComponent(topic)}`);
    await page.locator('.topic-page').waitFor();
    return page;
  };
  const crowded = async (on, what) => {
    const wrong = await pageCrowded(on, '.topic-page');
    expect(`nothing of the topic's page over anything else ${what}${looked}, got ${wrong.join('; ')}`, wrong.length === 0);
  };
  const bodyOf = async (id) => JSON.parse((await cli('topic', 'show', id, '--json')).stdout).topic?.data?.body ?? '';

  // ── The post, written: a link typed with [[, and a footnote ─────────────
  const page = await signedIn('Frogs');
  const post = page.locator('.topic-post');
  const editor = post.locator('.ProseMirror');
  // Read until Edit is pressed.
  await page.locator('.topic-page-head').getByRole('button', { name: 'Edit', exact: true }).click();
  await editor.locator('p').first().click();
  await page.keyboard.press('End');
  await page.keyboard.type(' Of [[chy');
  const menu = post.locator('.page-menu');
  await menu.getByRole('option', { name: /Chytrid fungus/ }).waitFor();
  expect('the page offered with its wiki', (await menu.getByRole('option').first().innerText()).includes('in Amphibians'));
  await page.keyboard.press('Enter');
  await menu.waitFor({ state: 'hidden' });
  await page.keyboard.type('.[^1]');
  await post.locator('.writer-cite[data-n="1"]').waitFor();
  await page.keyboard.press('Enter');
  await page.keyboard.type(`[^1]: ${SOURCE}`);
  await post.locator('.writer-note[data-n="1"]').waitFor();
  // Pointed at, the marker shows its source, in the editor as on the page.
  await post.locator('.writer-cite').hover();
  const editorCard = page.locator('.source-card');
  await editorCard.waitFor();
  expect('the source in the card', (await editorCard.innerText()).includes('Ada Lovelace'));
  await page.screenshot({ path: shot('1-writing') });
  await post.getByRole('button', { name: 'Save', exact: true }).click();
  await post.getByText('Not saved yet').waitFor({ state: 'detached' });
  const body = await bodyOf(frogs);
  expect(`the post kept as text with its link and its footnote, got ${JSON.stringify(body)}`, body.includes(`Many frogs died. Of [Chytrid fungus](topic:${fungus}).[^1]`) && body.includes(`[^1]: ${SOURCE}`));
  // Its article ends with References, the source numbered and its address a link.
  const references = page.locator('.topic-references');
  await references.locator('li[data-note="1"]').waitFor();
  expect('the source in References', (await references.innerText()).includes('Ada Lovelace'));
  expect('its address a link', (await references.locator('a[href="https://example.org/notes"]').count()) === 1);
  await references.getByRole('button', { name: 'Back to where source 1 is used' }).click();
  await crowded(page, 'written');

  // ── Followed: the page it links to is linked from this one ──────────────
  // Saved, the post is read again, its link a link to follow.
  await post.locator('.topic-doc-read a.page-link').click();
  await page.locator(`.topic-page[data-topic="${fungus}"]`).waitFor();
  const linkedFrom = page.locator(`.topic-page[data-topic="${fungus}"] .topic-linked`);
  await linkedFrom.getByRole('link', { name: /Frogs/ }).waitFor();
  expect('Linked from saying its wiki', (await linkedFrom.innerText()).includes('in Amphibians'));
  await page.screenshot({ path: shot('2-linked-from') });

  // ── Renamed and moved, and read by somebody who may only read ───────────
  await cli('record', 'set', fungus, 'title=Batrachochytrium');
  await cli('topic', 'move', fungus, '--top');
  const READS = ['projects:read', 'channels', 'records:read', 'board:read', 'search'];
  const reader = await signedIn('Frogs', undefined, READS);
  const link = reader.locator('.topic-post .topic-doc-read a.page-link');
  await link.waitFor();
  expect('the words as written, leading to the page as it is now', (await link.innerText()) === 'Chytrid fungus' && (await link.getAttribute('title')) === 'Batrachochytrium');
  const mark = reader.locator('.topic-post .cite-mark');
  expect('the marker a raised number', (await mark.innerText()) === '1' && (await mark.evaluate((one) => getComputedStyle(one.parentElement).verticalAlign)) === 'super');
  await mark.focus();
  const card = reader.locator('.source-card');
  await card.waitFor();
  expect('focused, the marker shows its source', (await card.innerText()).includes('Ada Lovelace'));
  expect('the source drawn once, at the end of the article', (await reader.locator('.topic-post .prose-notes').count()) === 0 && (await reader.locator('.topic-references li').count()) === 1);
  await reader.screenshot({ path: shot('3-read') });
  await crowded(reader, 'read');
  await mark.blur();
  await link.click();
  await reader.locator(`.topic-page[data-topic="${fungus}"]`).waitFor();
  expect('the page opened under its new title', (await reader.locator('.topic-page-title').first().innerText()).includes('Batrachochytrium'));

  // ── A reply: [[ in its box, and a link to a page that is not there ──────
  await page.goto(`${url}/topic/Frogs`);
  await page.locator(`.topic-page[data-topic="${frogs}"]`).waitFor();
  // Its post says something, so it is an article: its replies are on Talk.
  await page.locator('.topic-tabs').getByRole('tab', { name: /^Talk/ }).click();
  await page.locator('.topic-page[data-shows="talk"]').waitFor();
  const replyBox = page.getByLabel('Reply to the topic', { exact: true });
  await replyBox.click();
  await replyBox.pressSequentially('See [[amph');
  const offered = page.locator('.tag-offered');
  await offered.getByRole('option', { name: /Amphibians/ }).waitFor();
  await replyBox.press('Enter');
  expect('the link written in the box', (await replyBox.inputValue()) === `See [Amphibians](topic:${amphibians})`);
  await replyBox.pressSequentially(' and [A secret study](topic:@-topics-999).');
  await page.locator('.topic-reply-box').first().getByRole('button', { name: 'Post', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('.topic-reply-box textarea')?.value === '');
  const words = page.locator('.topic-reply-words').last();
  await words.locator('a.page-link').waitFor();
  expect('the reply links the page', (await words.locator('a.page-link').innerText()) === 'Amphibians');
  const none = words.locator('.page-link-none');
  expect('a page that is not there said to be one you cannot open, its words not drawn', (await none.innerText()) === 'a page you cannot open' && !(await words.innerText()).includes('secret'));
  await page.locator('.topic-replies-part').screenshot({ path: shot('4-reply') });

  // ── A question: a thread, whose sources show under its post all the same ──
  const asked = await made('Why do frogs croak', '--status', 'open', '--body', `To call a mate.[^1]\n\n[^1]: ${SOURCE}\n`);
  await page.goto(`${url}/topic/${encodeURIComponent(asked)}`);
  const thread = page.locator(`.topic-page[data-topic="${asked}"][data-shows="thread"]`);
  await thread.locator('.topic-references li[data-note="1"]').waitFor();
  expect('a thread with no tabs', (await thread.locator('.topic-tabs').count()) === 0);
  const under = await thread.evaluate((one) => {
    const post = one.querySelector('.topic-post').getBoundingClientRect();
    const refs = one.querySelector('.topic-references').getBoundingClientRect();
    const replies = one.querySelector('.topic-replies-part').getBoundingClientRect();
    return refs.top >= post.bottom - 1 && replies.top >= refs.bottom - 1;
  });
  expect("a question's References under its post and over its replies", under);
  await page.screenshot({ path: shot('5-thread') });
  await crowded(page, 'on a thread');

  // ── A link's card: where it leads, without leaving the page ─────────────
  const seq = JSON.parse((await cli('topic', 'reply', frogs, 'I saw three tree frogs today, all green.', '--json')).stdout).seq;
  await cli('topic', 'reply', frogs, `Read [Toads](topic:${toads}), [what was seen](topic:${frogs}#${seq}) and [the secret](topic:@-topics-999).`);
  const fungusRead = [];
  reader.on('request', (asked) => asked.url().endsWith(`/api/topics/${encodeURIComponent(fungus)}`) && fungusRead.push(asked.url()));
  await reader.goto(`${url}/topic/Frogs`);
  await reader.locator(`.topic-page[data-topic="${frogs}"]`).waitFor();
  const away = () => reader.mouse.move(2, 2);
  const preview = reader.locator('.page-card');
  // A page with no summary is read whole for its article, once however often its link is pointed at.
  const toFungus = reader.locator('.topic-post .topic-doc-read a.page-link');
  let readBefore = null;
  for (const _ of [1, 2]) {
    await toFungus.hover();
    await preview.getByText('A fungus of the skin.').waitFor();
    expect('the card says the title as it is now', (await preview.innerText()).includes('Batrachochytrium'));
    if (readBefore === null) readBefore = fungusRead.length;
    await away();
    await preview.waitFor({ state: 'detached' });
  }
  expect(`the page not read again for its card, got ${fungusRead.length - readBefore} more`, fungusRead.length === readBefore);
  // The replies' links are on Talk, "Frogs" being an article.
  await reader.locator('.topic-tabs').getByRole('tab', { name: /^Talk/ }).click();
  await reader.locator('.topic-page[data-shows="talk"]').waitFor();
  const linking = reader.locator('.topic-reply-words').last();
  const toToads = linking.getByRole('link', { name: 'Toads', exact: true });
  await toToads.waitFor();
  await toToads.hover();
  await preview.waitFor();
  const said = await preview.innerText();
  expect(`the card says the page, its wiki, its summary plainly and its replies, got ${JSON.stringify(said)}`, said.includes('Toads') && said.includes('in Amphibians') && said.includes('Warty frogs of dry places.') && said.includes('No replies') && said.includes('Open'));
  expect('the link described by its card', (await toToads.getAttribute('aria-describedby')) === (await preview.locator('.page-card-in').getAttribute('id')));
  await reader.screenshot({ path: shot('6-preview') });
  await crowded(reader, 'with a card');
  await away();
  await preview.waitFor({ state: 'detached' });
  // Reached by the keyboard, the same; Escape puts it away and leaves the keyboard where it was.
  await reader.keyboard.press('Shift');
  await toToads.focus();
  await preview.getByText('Warty frogs').waitFor();
  await reader.keyboard.press('Escape');
  await preview.waitFor({ state: 'detached' });
  expect('the link keeps the keyboard', await toToads.evaluate((one) => document.activeElement === one));
  await toToads.blur();
  // A link to a reply says who said it and what.
  await linking.getByRole('link', { name: 'what was seen' }).hover();
  await preview.getByText('I saw three tree frogs today').waitFor();
  const ofReply = await preview.innerText();
  expect(`the reply's card says who said it, on its page, got ${JSON.stringify(ofReply)}`, ofReply.includes('Ada said') && ofReply.includes('Frogs'));
  await reader.screenshot({ path: shot('7-reply-preview') });
  await away();
  await preview.waitFor({ state: 'detached' });
  // A page one cannot open is not a link and shows no card: pointed at, then a link beside it, only that link's card is there.
  const secret = linking.locator('.page-link-none');
  expect('a page one cannot open is no link', (await secret.evaluate((one) => one.tagName)) === 'SPAN');
  await secret.hover();
  await toToads.hover();
  await preview.getByText('Warty frogs').waitFor();
  expect('one card, of the link that is one', (await preview.count()) === 1 && !(await preview.innerText()).includes('secret'));
  await away();
  await preview.waitFor({ state: 'detached' });

  // ── On a phone ───────────────────────────────────────────────────────────
  const phone = await signedIn('Frogs', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true }, READS);
  await phone.locator('.topic-references li').waitFor();
  await phone.locator('.topic-post .cite-mark').focus();
  await phone.locator('.source-card').waitFor();
  const fits = await phone.evaluate(() => {
    const box = document.querySelector('.source-card').getBoundingClientRect();
    return box.left >= 0 && box.right <= document.documentElement.clientWidth;
  });
  expect('the source card within the window on a phone', fits);
  await phone.screenshot({ path: shot('8-phone') });
  await phone.locator('.topic-post .cite-mark').blur();
  await crowded(phone, 'on a phone');
  // A link's card on a phone, from the keyboard, fits the window; a tap opens the page and shows no card.
  await phone.locator('.topic-tabs').getByRole('tab', { name: /^Talk/ }).click();
  await phone.locator('.topic-page[data-shows="talk"]').waitFor();
  const phoneToads = phone.locator('.topic-reply-words').last().getByRole('link', { name: 'Toads', exact: true });
  await phone.keyboard.press('Shift');
  await phoneToads.focus();
  const phoneCard = phone.locator('.page-card');
  await phoneCard.getByText('Warty frogs').waitFor();
  const cardFits = await phone.evaluate(() => {
    const box = document.querySelector('.page-card').getBoundingClientRect();
    return box.left >= 0 && box.right <= document.documentElement.clientWidth && box.top >= 0 && box.bottom <= document.documentElement.clientHeight;
  });
  expect("the link's card within the window on a phone", cardFits);
  await phone.screenshot({ path: shot('9-phone-preview') });
  await phone.keyboard.press('Escape');
  await phoneCard.waitFor({ state: 'detached' });
  await phoneToads.tap();
  await phone.locator(`.topic-page[data-topic="${toads}"]`).waitFor();
  expect('a tap opens the page with no card', (await phoneCard.count()) === 0);
  expect(`nothing fetched from anywhere but the daemon, got ${JSON.stringify(elsewhere)}`, elsewhere.length === 0);

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
console.log(`the pictures are in ${out}`);
