// A forum's topic as a page of its own: a question, which is a thread, and
// an article, with its talk on a tab beside it.
//
//   DAEMON=../target/debug/factory node scripts/topicpage.mjs [out-dir]
//
// Two people, each on a page of their own that is never read again: Ada,
// who owns the factory and makes two topics from a terminal (which sets the
// topics up), and Nell, who registered by an invitation.
//
// The first is a question tracked as open, with a post. Ada opens it from
// the Topics panel, a place of the rail of its own, and the address bar
// says it by its title; Nell opens it by that address. Its head says what
// it is called, how it stands and where it is. A tracked question is talked
// out, so it is a thread whatever its post says: no tabs, its post drawn
// over its replies, read until Edit, and neither Add post nor Write an
// article offered. Ada adds a summary from its head, which brings its band
// at the top, writes it, opens the post by Edit and adds to it, each
// written as blocks, said not to be saved
// until it is, and saved through the door any record is changed by: the
// terminal reads both, and Nell's page shows both with nothing read again.
// Replies two deep, each by its own box: Ada's to the topic, Nell's by
// Reply under it, one from a terminal under hers; each arriving on the
// other page, drawn under what it answers. Ada marks Nell's as what
// answered the topic: it stands answered on both pages, in its head and in
// the panel, and her reply is drawn first, over the post. A chain that
// grows past five from a terminal starts folded past its third, saying how
// many are under the fold, and unfolds when that is pressed. What is half
// written in a box is there again after another tab was shown. An address
// of a reply (`#talk/r-<seq>`) opens the thread at it, marked and in view,
// with no tabs. Closed from its head, it is listed under Closed, and opened
// again; its post is as it was written. A question made with no post offers
// Add post, not Write an article, and the post written so is drawn in the
// thread, read once saved with Edit offered, and the thread stays one.
//
// The second has no status and a post, so it is an article: Article and
// Talk under its head, Article showing, Add status offered. Its summary,
// added from its head, is a band at the article's top. On Talk, which the
// address bar then says (`#talk`), replies two deep and one from a
// terminal, Talk counting them, none of them to be marked as the answer.
// What is half written in a box is there again, on Talk, after another
// tab was shown. The keyboard reaches both tabs. An address of a reply
// opens Talk at it, marked and in view, and one of the topic alone its
// article. A page made under it from a terminal is listed in Pages in it,
// and opened from there is a page of a wiki: where it is over its
// title, each page above it a link and the last the way up, the address
// bar saying its wiki's title and its own; a page made at its foot opens
// called Untitled with its title chosen, is called something else by
// what is typed there, on one line as it is typed, and is listed so on
// Nell's page and by the terminal; opened by its address on another page
// it says where it is, and every address there was (its title alone, its
// id, one written before addresses were paths) opens it and is said the
// new way. Nothing of either page lies over anything else or scrolls
// sideways at 1280, 1000 and 800 wide and a phone's. The environment
// variables are those of walkthrough.mjs.
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
const data = join(out, 'topicpage-data');
const home = join(out, 'topicpage-home');
const hers = join(out, 'topicpage-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

const TITLE = 'Why the build is slow on Windows';
const ARTICLE = 'How a release is made';
let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  await cli('channel', 'create', 'general');
  const made = async (...args) => JSON.parse((await cli('topic', 'make', ...args, '--json')).stdout).id;
  const question = await made(TITLE, '--body', 'A clean build takes ten minutes there and two here.', '--status', 'open');
  const article = await made(ARTICLE, '--body', 'Tag it, build it, sign it.');

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
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }, at = url) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    await page.goto(at);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  const shown = async (on, locator, what) => {
    await locator.waitFor({ timeout: 10000 }).catch(() => {
      throw new Error(`expected ${what}`);
    });
  };
  // What a page is called: a field whoever may change it writes in, and words for anybody else.
  const titleOf = async (where) => where.locator('.topic-page-title').first().evaluate((one) => one.value ?? one.innerText);
  const titledAs = async (where, title, what) => {
    await where.locator('.topic-page-title').first().waitFor({ timeout: 10000 }).catch(() => {
      throw new Error(`expected ${what}`);
    });
    const field = await where.locator('.topic-page-title').first().elementHandle();
    await where
      .page()
      .waitForFunction(([one, said]) => (one.value ?? one.innerText) === said, [field, title], { timeout: 10000 })
      .catch(async () => {
        throw new Error(`expected ${what}, got ${await titleOf(where)}`);
      });
  };
  // The parts of a panel of the page, or of a thread's page itself, by what each is called to whoever hears it read out.
  const partsIn = (panel) => panel.evaluate((one) => [...one.querySelectorAll(':scope > section')].map((part) => part.getAttribute('aria-label')).join(' | '));
  const laid = async (on, what) => {
    const wrong = await pageCrowded(on, '.topic-page');
    expect(`nothing of the topic's page over anything else ${what}: ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await on.evaluate(() => [document.documentElement, document.querySelector('.topic-page')].map((one) => one.scrollWidth - one.clientWidth));
    expect(`nothing to scroll sideways ${what}, got ${sideways.join(', ')}px`, sideways.every((one) => one <= 0));
  };
  const tabOf = (where, name) => where.getByRole('tablist', { name: 'Article, talk and history' }).getByRole('tab', { name: new RegExp(`^${name}`) });
  const showing = async (where, name, what) => {
    await where.locator(`.view-tab.topic-tab[aria-selected="true"][data-part="${name.toLowerCase()}"]`).waitFor({ timeout: 10000 }).catch(() => {
      throw new Error(`expected ${name} showing: ${what}`);
    });
  };
  // A thread: the page says so, and draws no tabs.
  const threaded = async (on, topic, what) => {
    await on.locator(`.topic-page[data-topic="${topic}"][data-shows="thread"]`).waitFor({ timeout: 10000 }).catch(() => {
      throw new Error(`expected a thread: ${what}`);
    });
    expect(`no tabs on a thread: ${what}`, (await on.locator('.topic-page').getByRole('tablist').count()) === 0);
  };
  const offered = async (where, name) => (await where.locator('.topic-page-head').getByRole('button', { name, exact: true }).count()) > 0;
  // The editor `editor` (a selector) has the keyboard as the page draws it: it says it is focused, and the caret is in
  // it, and where `atEnd`, after the last of the words of its last paragraph.
  const caretIn = async (on, editor, what, atEnd = false) => {
    await on
      .waitForFunction(
        ([editor, atEnd]) => {
          const one = document.querySelector(editor);
          const caret = document.getSelection();
          if (!one?.classList.contains('ProseMirror-focused') || !caret?.isCollapsed || !caret.anchorNode || !one.contains(caret.anchorNode)) return false;
          if (!atEnd) return true;
          const last = [...one.querySelectorAll('p')].at(-1);
          if (!last?.contains(caret.anchorNode)) return false;
          const after = document.createRange();
          after.setStart(caret.anchorNode, caret.anchorOffset);
          after.setEnd(last, last.childNodes.length);
          return after.toString() === '';
        },
        [editor, atEnd],
        { timeout: 10000 },
      )
      .catch(() => {
        throw new Error(`expected the caret in ${what}`);
      });
  };
  // Pressed in the editor `editor` (at `position` of `target`, or anywhere in it), and the caret in it. Where the press
  // moves the caret (`atEnd`: from where the editor put it to the end of its words), it is where the press put it once
  // the editor has heard so: the editor takes where the caret is from the page's selectionchange, which comes a moment
  // after the press, and what is typed before that goes where the caret was. Counted as the page tells it, so that
  // every listener of it, the editor's among them, has heard it once the count is read.
  const pressIn = async (on, editor, what, { target, position, atEnd = false } = {}) => {
    const was = await on.evaluate(() => {
      if (window.selectionsHeard === undefined) {
        window.selectionsHeard = 0;
        document.addEventListener('selectionchange', () => {
          window.selectionsHeard += 1;
        });
      }
      return window.selectionsHeard;
    });
    await (target ?? on.locator(editor)).click(position ? { position } : {});
    if (atEnd) {
      await on.waitForFunction((was) => window.selectionsHeard > was, was, { timeout: 10000 }).catch(() => {
        throw new Error(`expected the press in ${what} to move the caret`);
      });
    }
    await caretIn(on, editor, what, atEnd);
  };

  // ── The question: opened from the panel, and by its address ────────────
  const page = await signedIn('ada@example.com');
  const rail = page.locator('.rail').getByRole('button', { name: 'Topics', exact: true });
  await rail.waitFor();
  if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
  const panel = page.getByRole('navigation', { name: 'Topics', exact: true });
  await panel.locator('.topic-line .nav-item', { hasText: TITLE }).click();
  const ours = page.locator(`.topic-page[data-topic="${question}"]`);
  await titledAs(ours, TITLE, 'the topic opened from its line, headed by its title, written in place');
  await threaded(page, question, 'a question tracked as open, whose post says something');
  expect('no summary drawn before it says something', (await ours.locator('.topic-summary').count()) === 0);
  expect('its head to say how it stands and where it is', (await ours.locator('.topic-status').innerText()) === 'open' && (await ours.locator('.topic-page-where').innerText()).includes("Ada's workspace"));
  await shown(page, ours.locator('.topic-post .topic-doc-read', { hasText: 'two here' }), 'its post as the terminal wrote it, read, on the thread');
  expect('its post read until Edit, and Edit offered', (await ours.locator('.topic-post .ProseMirror').count()) === 0 && (await offered(ours, 'Edit')));
  expect('neither Add post nor Write an article offered on a question with a post', !(await offered(ours, 'Add post')) && !(await offered(ours, 'Write an article')));
  await page.waitForFunction((title) => location.pathname === `/topic/${encodeURIComponent(title)}`, TITLE);
  const address = await page.evaluate(() => location.href);
  await page.getByRole('tab', { name: new RegExp(TITLE.slice(0, 12)) }).first().waitFor();
  const theirs = await signedIn('nell@example.com', { viewport: { width: 1280, height: 860 } }, address);
  const hersPage = theirs.locator(`.topic-page[data-topic="${question}"]`);
  await titledAs(hersPage, TITLE, "the topic opened by its address on Nell's page");
  await threaded(theirs, question, "the question on Nell's page");

  // ── Its summary and its post, written as blocks ─────────────────────────
  await ours.locator('.topic-page-head').getByRole('button', { name: 'Add summary', exact: true }).click();
  await pressIn(page, `.topic-page[data-topic="${question}"] .topic-summary .ProseMirror`, "the question's summary");
  await page.keyboard.type('It is the linker, on Windows alone.');
  await shown(page, ours.locator('.topic-summary', { hasText: 'Not saved yet' }), 'the summary said not to be saved yet');
  await page.screenshot({ path: `${out}/topicpage-01-writing.png` });
  await ours.locator('.topic-summary').getByRole('button', { name: 'Save', exact: true }).click();
  await ours.locator('.topic-summary', { hasText: 'Not saved yet' }).waitFor({ state: 'detached' });
  await threaded(page, question, 'the question still a thread once its summary is saved');
  // Its post is read until Edit opens it, with its summary.
  await ours.locator('.topic-page-head').getByRole('button', { name: 'Edit', exact: true }).click();
  await shown(page, ours.locator('.topic-post .ProseMirror', { hasText: 'two here' }), 'Edit opening its post in the block editor, on the thread');
  expect('Edit offered no more while its post is open', !(await offered(ours, 'Edit')));
  // Pressed past the end of its last line, so that the caret is at the end whenever the editor hears where the press
  // landed: it hears that a moment after it, as a browser reports it, and a script is faster than that (crowded, End
  // was pressed before, and the caret was put back where the press landed).
  // Edit has put the caret in it before it is pressed where the words are to go.
  await caretIn(page, `.topic-page[data-topic="${question}"] .topic-post .ProseMirror`, "the question's post opened by Edit");
  const last = ours.locator('.topic-post .ProseMirror p').last();
  const box = await last.boundingBox();
  await pressIn(page, `.topic-page[data-topic="${question}"] .topic-post .ProseMirror`, "the end of the question's post", { target: last, position: { x: box.width - 2, y: box.height - 4 }, atEnd: true });
  await page.keyboard.press('Enter');
  await page.keyboard.type('The linker is the one that ships.');
  await page.keyboard.press('Control+s');
  await ours.locator('.topic-post', { hasText: 'Not saved yet' }).waitFor({ state: 'detached' });
  const kept = (await cli('record', 'show', question)).stdout;
  expect(`the terminal to read the summary and the post as saved, got ${kept}`, kept.includes('It is the linker, on Windows alone.') && kept.includes('The linker is the one that ships.') && kept.includes('two here'));
  await shown(theirs, hersPage.locator('.topic-summary', { hasText: 'It is the linker, on Windows alone.' }), "the summary on Nell's page, never read again");
  await shown(theirs, hersPage.locator('.topic-post', { hasText: 'The linker is the one that ships.' }), "the post on Nell's page");
  expect('Add summary offered no more once it says something', !(await offered(ours, 'Add summary')));
  expect(`the thread's parts, got ${await partsIn(ours)}`, (await partsIn(ours)) === 'Summary | Post | Replies');

  // ── Replies two deep, and one from a terminal, on the thread ────────────
  const top = ours.getByLabel('Reply to the topic', { exact: true });
  await top.fill('Is it the linker, or the antivirus?');
  await ours.getByRole('button', { name: 'Post', exact: true }).click();
  const first = hersPage.locator('.topic-replies > .topic-reply', { hasText: 'or the antivirus' });
  await shown(theirs, first, "Ada's reply on Nell's page, at the top of the tree");
  await first.locator('.topic-reply-acts').getByRole('button', { name: 'Reply', exact: true }).click();
  await first.getByLabel('Reply to Ada', { exact: true }).fill('The linker: it is twice as slow there.');
  await first.locator('.topic-reply-box').getByRole('button', { name: 'Reply', exact: true }).click();
  const second = ours.locator('.topic-reply[data-depth="2"]', { hasText: 'twice as slow' });
  await shown(page, second, "Nell's reply on Ada's page, under the one it answers");
  const answered = await second.getAttribute('data-seq');
  await cli('topic', 'reply', TITLE, 'Measured: 41 s of 60 is the link.', '--to', answered);
  await shown(page, ours.locator('.topic-reply[data-depth="3"]', { hasText: '41 s of 60' }), 'a reply from a terminal, three deep');
  await shown(theirs, hersPage.locator('.topic-reply[data-depth="3"]', { hasText: '41 s of 60' }), "the same on Nell's page");
  await page.screenshot({ path: `${out}/topicpage-02-replies.png` });
  const counted = await ours.locator('.topic-replies-part > .topic-part-label').textContent();
  expect(`the thread to say how many replies it has, got ${counted}`, counted === '3 replies');
  await threaded(page, question, 'the question still a thread with replies');

  // ── What answered it ────────────────────────────────────────────────────
  await second.locator(':scope > .topic-reply-body .topic-reply-acts').getByRole('button', { name: 'Mark as the answer', exact: true }).click();
  for (const [on, where] of [
    [page, ours],
    [theirs, hersPage],
  ]) {
    await shown(on, where.locator('.topic-status', { hasText: 'answered' }), 'answered in its head');
    await shown(on, where.locator('.topic-answer', { hasText: 'twice as slow' }), 'the reply that answered it drawn first');
  }
  await threaded(page, question, 'an answered question still a thread');
  const order = await partsIn(ours);
  expect(`the answer first, over the post, got ${order}`, order === 'Summary | What answered it | Post | Replies');
  await page.getByRole('navigation', { name: 'Topics', exact: true }).locator('.topic-line', { hasText: 'answered' }).waitFor();
  await page.screenshot({ path: `${out}/topicpage-03-answered.png` });

  // ── A long chain, folded past its third ─────────────────────────────────
  let under = await ours.locator('.topic-reply[data-depth="3"]', { hasText: '41 s of 60' }).getAttribute('data-seq');
  for (const words of ['Then link with the other linker.', 'Tried: 12 s.', 'Ship it.']) under = (await cli('topic', 'reply', TITLE, words, '--to', under)).stdout.trim().replace('said at ', '');
  const more = ours.locator('.topic-reply-more');
  await shown(page, more, 'a chain past five folded past its third');
  expect(`the fold to say how many it holds, got ${await more.innerText()}`, (await more.innerText()) === '3 more replies');
  expect('nothing under the fold drawn', (await ours.locator('.topic-reply', { hasText: 'Tried: 12 s.' }).count()) === 0);
  await more.click();
  await shown(page, ours.locator('.topic-reply[data-depth="6"]', { hasText: 'Ship it.' }), 'the chain unfolded, six deep');

  // ── Kept by the tab ─────────────────────────────────────────────────────
  await top.fill('half written');
  await page.getByRole('tab', { name: /Start/ }).click();
  await ours.waitFor({ state: 'detached' });
  await page.getByRole('tab', { name: new RegExp(TITLE.slice(0, 12)) }).first().click();
  await threaded(page, question, 'the question shown again');
  expect('what was half written there again', (await ours.getByLabel('Reply to the topic', { exact: true }).inputValue()) === 'half written');
  await top.fill('');

  // ── A reply of a question by its address ────────────────────────────────
  const atAnswer = await signedIn('nell@example.com', { viewport: { width: 1280, height: 860 } }, `${address}#talk/r-${answered}`);
  await threaded(atAnswer, question, 'an address of a reply of a question opening its thread');
  const markedThere = atAnswer.locator(`.topic-page .topic-reply.asked[data-seq="${answered}"]`);
  await shown(atAnswer, markedThere, 'the reply its address names, marked, on the thread');
  await atAnswer.waitForFunction((one) => {
    const box = one.getBoundingClientRect();
    return box.top >= 0 && box.bottom <= window.innerHeight;
  }, await markedThere.elementHandle());
  await atAnswer.waitForFunction(() => location.hash === '');
  await atAnswer.context().close();

  // ── Closed, and opened again ────────────────────────────────────────────
  await ours.locator('.topic-page-head').getByRole('button', { name: 'Close', exact: true }).click();
  await shown(page, ours.locator('.topic-status', { hasText: 'closed' }), 'closed in its head');
  await shown(page, panel.locator('.nav-item[aria-expanded]', { hasText: 'Closed' }), 'listed under Closed');
  await threaded(page, question, 'a closed question still a thread');
  await ours.locator('.topic-page-head').getByRole('button', { name: 'Open again', exact: true }).click();
  await shown(page, ours.locator('.topic-status', { hasText: 'open' }), 'open again');
  expect('nothing said of what answered it once it is open again', (await ours.locator('.topic-answer, .topic-answered-mark').count()) === 0);
  expect(`the post as it was written, a paragraph each, got ${await ours.locator('.topic-post .ProseMirror').innerText()}`, (await ours.locator('.topic-post .ProseMirror p').allInnerTexts()).join(' | ') === 'A clean build takes ten minutes there and two here. | The linker is the one that ships.');

  // ── A question with no post: Add post writes it in the thread ───────────
  const bare = await made('Which linker is fastest', '--status', 'open');
  await panel.locator('.topic-line .nav-item', { hasText: 'Which linker is fastest' }).click();
  const asked = page.locator(`.topic-page[data-topic="${bare}"]`);
  await titledAs(asked, 'Which linker is fastest', 'a question with no post opened from its line');
  await threaded(page, bare, 'a question with no post');
  expect('no post drawn, Add post offered and Write an article not', (await asked.locator('.topic-post').count()) === 0 && (await offered(asked, 'Add post')) && !(await offered(asked, 'Write an article')));
  await asked.locator('.topic-page-head').getByRole('button', { name: 'Add post', exact: true }).click();
  await pressIn(page, `.topic-page[data-topic="${bare}"] .topic-post .ProseMirror`, 'the post begun by Add post');
  await page.keyboard.type('Measured on the release build.');
  await asked.locator('.topic-post').getByRole('button', { name: 'Save', exact: true }).click();
  await asked.locator('.topic-post', { hasText: 'Not saved yet' }).waitFor({ state: 'detached' });
  await shown(page, asked.locator('.topic-post .topic-doc-read', { hasText: 'Measured on the release build.' }), 'the post read once Save is pressed');
  expect('Edit offered on the question once its post says something, Add post not', (await offered(asked, 'Edit')) && !(await offered(asked, 'Add post')));
  await threaded(page, bare, 'the question still a thread once its post is saved');
  expect(`the thread's parts with its post, got ${await partsIn(asked)}`, (await partsIn(asked)) === 'Post | Replies');
  expect('the terminal to read the post', (await cli('record', 'show', bare)).stdout.includes('Measured on the release build.'));

  // ── The article: no status, and a post that says something ──────────────
  await panel.locator('.topic-line .nav-item', { hasText: ARTICLE }).click();
  const art = page.locator(`.topic-page[data-topic="${article}"]`);
  await titledAs(art, ARTICLE, 'the article opened from its line');
  await showing(art, 'Article', 'a topic with no status whose post says something opened on its article');
  expect('no status in its head, and Add status offered', (await art.locator('.topic-status').count()) === 0 && (await offered(art, 'Add status')));
  await shown(page, art.locator('.topic-post .topic-doc-read', { hasText: 'sign it' }), 'its article as the terminal wrote it, read');
  expect('neither Add post nor Write an article offered on an article', !(await offered(art, 'Add post')) && !(await offered(art, 'Write an article')));
  await page.waitForFunction((title) => location.pathname === `/topic/${encodeURIComponent(title)}`, ARTICLE);
  const artAddress = await page.evaluate(() => location.href);
  await theirs.goto(artAddress);
  const hersArt = theirs.locator(`.topic-page[data-topic="${article}"]`);
  await titledAs(hersArt, ARTICLE, "the article opened by its address on Nell's page");
  await showing(hersArt, 'Article', "the article on Nell's page");

  await art.locator('.topic-page-head').getByRole('button', { name: 'Add summary', exact: true }).click();
  await pressIn(page, `.topic-page[data-topic="${article}"] .topic-summary .ProseMirror`, "the article's summary");
  await page.keyboard.type('Signing is the slow part.');
  await art.locator('.topic-summary').getByRole('button', { name: 'Save', exact: true }).click();
  await art.locator('.topic-summary', { hasText: 'Not saved yet' }).waitFor({ state: 'detached' });
  await shown(theirs, hersArt.locator('.topic-summary', { hasText: 'Signing is the slow part.' }), "the article's summary on Nell's page, never read again");
  expect(`the article's parts, got ${await partsIn(art.locator('.topic-article-main'))}`, (await partsIn(art.locator('.topic-article-main'))) === 'Summary | Article');

  // ── Replies two deep, and one from a terminal, on Talk ──────────────────
  await tabOf(art, 'Talk').click();
  await showing(art, 'Talk', 'Talk pressed');
  await page.waitForFunction(() => location.hash === '#talk');
  await tabOf(hersArt, 'Talk').click();
  const artTop = art.getByLabel('Reply to the topic', { exact: true });
  await artTop.fill('Can it be signed in parallel?');
  await art.getByRole('button', { name: 'Post', exact: true }).click();
  const firstThere = hersArt.locator('.topic-replies > .topic-reply', { hasText: 'in parallel' });
  await shown(theirs, firstThere, "Ada's reply on Talk on Nell's page");
  await firstThere.locator('.topic-reply-acts').getByRole('button', { name: 'Reply', exact: true }).click();
  await firstThere.getByLabel('Reply to Ada', { exact: true }).fill('Yes: one signer per target.');
  await firstThere.locator('.topic-reply-box').getByRole('button', { name: 'Reply', exact: true }).click();
  const secondThere = art.locator('.topic-reply[data-depth="2"]', { hasText: 'one signer per target' });
  await shown(page, secondThere, "Nell's reply on Ada's Talk, under the one it answers");
  const linkedSeq = await secondThere.getAttribute('data-seq');
  await cli('topic', 'reply', ARTICLE, 'Done: four at once.', '--to', linkedSeq);
  await shown(page, art.locator('.topic-reply[data-depth="3"]', { hasText: 'four at once' }), 'a reply from a terminal, three deep, on Talk');
  await shown(theirs, hersArt.locator('.topic-reply[data-depth="3"]', { hasText: 'four at once' }), "the same on Nell's Talk");
  await page.screenshot({ path: `${out}/topicpage-09-talk.png` });
  expect(`Talk to say how many replies it has, got ${await tabOf(art, 'Talk').innerText()}`, (await tabOf(art, 'Talk').innerText()).replace(/\s+/g, ' ') === 'Talk 3');
  expect(`Talk's parts with nothing answered, got ${await partsIn(art.locator('.topic-talk'))}`, (await partsIn(art.locator('.topic-talk'))) === 'Replies');
  expect('nothing to mark as the answer on a topic that is no question', (await art.getByRole('button', { name: 'Mark as the answer', exact: true }).count()) === 0);

  // ── Kept by the tab ─────────────────────────────────────────────────────
  await artTop.fill('half written');
  await page.getByRole('tab', { name: /Start/ }).click();
  await art.waitFor({ state: 'detached' });
  await page.getByRole('tab', { name: new RegExp(ARTICLE.slice(0, 12)) }).first().click();
  await showing(art, 'Talk', 'Talk still showing as the tab is shown again');
  expect('what was half written there again, on Talk', (await art.getByLabel('Reply to the topic', { exact: true }).inputValue()) === 'half written');

  // ── Both tabs by the keyboard ───────────────────────────────────────────
  await tabOf(art, 'Article').focus();
  await page.keyboard.press('Enter');
  await showing(art, 'Article', 'Article chosen by the keyboard');
  await page.waitForFunction(() => location.hash === '');
  await page.keyboard.press('Tab');
  expect('Tab to go from Article to Talk', await tabOf(art, 'Talk').evaluate((one) => one === document.activeElement));
  await page.keyboard.press('Enter');
  await showing(art, 'Talk', 'Talk chosen by the keyboard');

  // ── A reply by its address, and the article by its own ──────────────────
  const atReply = `${artAddress}#talk/r-${linkedSeq}`;
  const linked = await signedIn('nell@example.com', { viewport: { width: 1280, height: 860 } }, atReply);
  const linkedPage = linked.locator('.topic-page');
  await showing(linkedPage, 'Talk', 'an address of a reply opening Talk');
  const marked = linkedPage.locator(`.topic-reply.asked[data-seq="${linkedSeq}"]`);
  await shown(linked, marked, 'the reply its address names, marked');
  await linked.waitForFunction((one) => {
    const box = one.getBoundingClientRect();
    return box.top >= 0 && box.bottom <= window.innerHeight;
  }, await marked.elementHandle());
  await linked.waitForFunction(() => location.hash === '#talk');
  await linked.screenshot({ path: `${out}/topicpage-08-reply-linked.png` });
  await linked.goto(artAddress);
  await showing(linked.locator('.topic-page'), 'Article', 'the address of the topic alone opening its article');
  await linked.context().close();
  await tabOf(art, 'Article').click();
  await showing(art, 'Article', 'Article pressed');

  // ── A page of a wiki: where it is, the pages in it, its title written in place ──
  const make = async (title, under) => JSON.parse((await cli('topic', 'make', title, '--under', under, '--json')).stdout).id;
  const deploying = await make('Deploying', article);
  await make('Rolling back', deploying);
  const pagesIn = art.locator('.topic-pages');
  await shown(page, pagesIn.locator('.topic-pages-line', { hasText: 'Deploying' }), 'a page made under it from a terminal listed in Pages in it, the page not read again');
  const parts = await partsIn(art.locator('.topic-article-main'));
  expect(`the pages in it at the end of its article, got ${parts}`, parts === 'Summary | Article | Pages in it');
  await pagesIn.locator('.topic-pages-line', { hasText: 'Deploying' }).click();
  const deep = page.locator(`.topic-page[data-topic="${deploying}"]`);
  await titledAs(deep, 'Deploying', 'the page opened from its line in Pages in it');
  const pathOf = (where) => where.locator('.topic-path .topic-path-page').allInnerTexts();
  expect(`its path to be its wiki, got ${await pathOf(deep)}`, (await pathOf(deep)).join(' › ') === ARTICLE);
  await page.waitForFunction((title) => location.pathname === `/topic/${encodeURIComponent(title)}/Deploying`, ARTICLE);
  await shown(page, deep.locator('.topic-pages-line', { hasText: 'Rolling back' }), 'the page under it');
  await tabOf(hersArt, 'Article').click();
  await hersArt.locator('.topic-pages-line', { hasText: 'Deploying' }).click();
  const hersDeep = theirs.locator(`.topic-page[data-topic="${deploying}"]`);
  await titledAs(hersDeep, 'Deploying', "the same page on Nell's page");
  await page.screenshot({ path: `${out}/topicpage-06-page.png` });

  // A page made at its foot, opened with its title chosen, and written in place.
  await deep.getByRole('button', { name: '+ New page', exact: true }).click();
  const fresh = page.locator('.topic-page').filter({ has: page.locator('.topic-path-page', { hasText: 'Deploying' }) });
  await titledAs(fresh, 'Untitled', 'a page made at the foot of Pages in it, opened, called Untitled');
  await page.waitForFunction(() => {
    const one = document.activeElement;
    return one?.classList.contains('topic-title-field') && one.selectionStart === 0 && one.selectionEnd === one.value.length;
  });
  await page.keyboard.type('Staging');
  await page.screenshot({ path: `${out}/topicpage-07-titled.png` });
  // As wide as what is written in it, on one line: a short title is never wrapped by its own field.
  const lines = await fresh.locator('.topic-title-field').evaluate((one) => one.clientHeight / parseFloat(getComputedStyle(one).lineHeight));
  expect(`the title on one line as it is written, got ${lines.toFixed(2)} lines`, lines < 1.8);
  await page.keyboard.press('Enter');
  await shown(theirs, hersDeep.locator('.topic-pages-line', { hasText: 'Staging' }), "the page called so in Pages in it on Nell's page, never read again");
  const staged = (await cli('topic', 'show', 'Staging')).stdout;
  expect(`the terminal to read it under its path, got ${staged}`, staged.includes(`in ${ARTICLE} › Deploying`));
  expect(`its path to be its wiki and the page above it, got ${await pathOf(fresh)}`, (await pathOf(fresh)).join(' › ') === `${ARTICLE} › Deploying`);
  await page.waitForFunction((title) => location.pathname === `/topic/${encodeURIComponent(title)}/Staging`, ARTICLE);
  const nested = await page.evaluate(() => location.href);
  // The last of its path is the way up.
  const up = fresh.locator('.topic-path-page').last();
  expect('the last of its path to say it is the way up', (await up.getAttribute('title')) === 'Up to Deploying');
  await up.click();
  await titledAs(deep, 'Deploying', 'the way up opening the page above it');

  // A nested page by its address, and by every address there was.
  const byAddress = await signedIn('nell@example.com', { viewport: { width: 1280, height: 860 } }, nested);
  await titledAs(byAddress.locator('.topic-page'), 'Staging', 'a page three deep opened by its address, its wiki and its own title');
  expect('its path drawn there', (await pathOf(byAddress.locator('.topic-page'))).join(' › ') === `${ARTICLE} › Deploying`);
  const staging = JSON.parse((await cli('topic', 'show', 'Staging', '--json')).stdout).topic.id;
  for (const old of [`${url}/topic/Staging`, `${url}/topic/${encodeURIComponent(staging)}`, `${url}/#/topic/${encodeURIComponent(staging)}`]) {
    await byAddress.goto(old);
    await titledAs(byAddress.locator('.topic-page'), 'Staging', `the page opened by an address there was before, ${old}`);
    await byAddress.waitForFunction((title) => location.pathname === `/topic/${encodeURIComponent(title)}/Staging`, ARTICLE);
  }
  await byAddress.context().close();

  // ── By the boxes ────────────────────────────────────────────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await laid(page, `at ${width} wide, a page of a wiki`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  for (const [on, name] of [
    [address, 'the question'],
    [artAddress, 'the article'],
  ]) {
    await page.goto(on);
    await page.locator('.topic-page .topic-page-title').waitFor();
    for (const width of [1280, 1000, 800]) {
      await page.setViewportSize({ width, height: 860 });
      await laid(page, `at ${width} wide, ${name}`);
    }
    await page.setViewportSize({ width: 1280, height: 860 });
  }
  await page.goto(address);
  await threaded(page, question, 'the question opened again by its address');
  await page.screenshot({ path: `${out}/topicpage-04-whole.png`, fullPage: true });
  const phone = await signedIn('nell@example.com', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 }, address);
  await titledAs(phone.locator('.topic-page'), TITLE, 'the question on a phone');
  await phone.locator('.topic-reply', { hasText: 'Ship it.' }).waitFor({ state: 'attached' }).catch(() => undefined);
  await laid(phone, "at a phone's width, the question");
  await phone.screenshot({ path: `${out}/topicpage-05-phone.png` });
  await phone.goto(`${artAddress}#talk`);
  await showing(phone.locator('.topic-page'), 'Talk', 'the article on a phone, on Talk by its address');
  await phone.locator('.topic-reply[data-depth="3"]', { hasText: 'four at once' }).waitFor();
  await laid(phone, "at a phone's width, the article's talk");
  await phone.screenshot({ path: `${out}/topicpage-10-phone-talk.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
