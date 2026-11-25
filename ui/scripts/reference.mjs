// The Reference: everything a blueprint can say, as a page in a tab.
//
//   DAEMON=../target/debug/factory node scripts/reference.mjs [out-dir]
//
// The page is made from what the daemon describes and writes none of it:
// so this holds what it shows to what the daemon hands the page, by
// asking the daemon the same. Opened from its line in the side panel, it
// has three parts, each saying how many things it lists. What is typed in
// its box finds what says it: `quiet`, the key inside `fields` of
// `collection.ensure`, shown with its sentence under what it is a key of.
// The entry's mark copies an address, which opened in a second page shows
// the entry unfolded, marked and in view, and is then gone from the
// address bar. A word pointed at in the text of the board that ships has
// a card with one link, which opens the page at that word's entry; the
// card stays as the pointer crosses to the link. An event is found by its
// topic, with what it carries. A word says what it is for and is shown
// by its example (`retry`, a whole blueprint coloured as the editor
// colours it, with a mark that copies it), with the blueprints that ship
// that use it, one of them opened at that line; an action the same
// (`channel.post`); a word found by what only its example says; and the
// card about a word in a blueprint's text shows the example, moving
// nothing and staying as the pointer crosses to it. Help's menu leads there too. Nothing of it
// lies over anything else or scrolls sideways at three widths and a
// phone's, and it is drawn in the other of dark and light. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { sized } from './looked.ts';
import { saysWhere } from './where.ts';

/** Turns the side panel to a view on the rail, unless it is showing already: Resources with what it has read unfolded, as everything there starts folded (`folded.mjs`). */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  if (name === 'Resources') await unfoldAll(page);
}

/**
 * What is wrong of how the page is laid out: an entry's pieces over each
 * other, two entries over each other, one wider than its page, the box
 * that finds too narrow; and how far it scrolls sideways.
 */
function laidOut(on) {
  return on.evaluate(() => {
    const view = document.querySelector('.reference').closest('.view');
    const within = view.getBoundingClientRect();
    const rect = (one) => one.getBoundingClientRect();
    const lies = (a, b) => a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5;
    const wrong = [];
    // Each entry's own pieces, one under another; and what is inside it under those.
    for (const entry of document.querySelectorAll('.ref-entry')) {
      const pieces = [...entry.children].map(rect).filter((box) => box.width > 0 && box.height > 0);
      pieces.forEach((box, at) => {
        if (pieces.slice(at + 1).some((other) => lies(box, other))) wrong.push(`pieces of ${entry.dataset.entry} lie over each other`);
      });
      const line = [...entry.querySelectorAll(':scope > .ref-line > *')].map(rect).filter((box) => box.width > 0);
      line.forEach((box, at) => {
        if (line.slice(at + 1).some((other) => lies(box, other))) wrong.push(`the line of ${entry.dataset.entry} has one thing over another`);
      });
      const box = rect(entry);
      if (box.right > within.right + 0.5 || box.left < within.left - 0.5) wrong.push(`${entry.dataset.entry} is wider than its page`);
    }
    // The entries of a part, one under another.
    for (const list of document.querySelectorAll('.ref-entries, .ref-under')) {
      const boxes = [...list.children].map(rect);
      for (let at = 1; at < boxes.length; at++) if (boxes[at].top < boxes[at - 1].bottom - 0.5) wrong.push('two entries lie over each other');
    }
    // An example, and where it is used, inside its entry, one under the other, each place a line of its own.
    for (const example of document.querySelectorAll('.ref-example')) {
      const box = rect(example);
      const of = rect(example.closest('.ref-entry'));
      if (box.right > of.right + 0.5 || box.left < of.left - 0.5) wrong.push(`an example is wider than ${example.closest('.ref-entry').dataset.entry}`);
      const parts = [...example.querySelectorAll(':scope > *, .ref-used-line')].map(rect).filter((one) => one.height > 0);
      parts.forEach((one, at) => {
        if (parts.slice(at + 1).some((other) => lies(one, other) && !(other.top >= one.top && other.bottom <= one.bottom + 0.5))) wrong.push('two pieces of an example lie over each other');
      });
    }
    const top = rect(document.querySelector('.ref-top'));
    const find = rect(document.querySelector('.ref-find'));
    if (find.right > top.right + 0.5 || find.width < 120) wrong.push(`the box that finds is ${find.width} wide in a head ${top.width} wide`);
    return { wrong: [...new Set(wrong)].slice(0, 6), sideways: view.scrollWidth - view.clientWidth, page: document.documentElement.scrollWidth - window.innerWidth, entries: document.querySelectorAll('.ref-entry').length };
  });
}

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'reference-data');
const home = join(out, 'reference-home');

const QUIET = 'action/collection.ensure/fields/quiet';
const WIDTHS = [1280, 1000, 760];
const scheme = process.env.SCHEME ?? 'dark';

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
let page = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  // As tall for its text as a window of 860 is at the usual size: what is found is held to be in it with nothing pressed.
  const context = await browser.newContext({ viewport: { width: 1280, height: sized(860) }, colorScheme: scheme, permissions: ['clipboard-read', 'clipboard-write'] });
  page = await context.newPage();
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  // What the page asks of its daemon for the language, counted: it reads once and follows nothing.
  let readings = 0;
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/api/blueprints/language') readings++;
  });
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async (on) => {
    await on.getByLabel('Email').fill('ada@example.com');
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  await page.goto(url);
  await signIn(page);

  // What the daemon describes, asked as the page asks it: what the page shows is held to this.
  const language = await page.evaluate(async () => (await fetch('/api/blueprints/language', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json());
  const described = {
    words: language.shapes.reduce((sum, shape) => sum + shape.fields.length, 0) + language.names.length + language.functions.length,
    actions: language.actions.length,
    events: language.events.length,
  };
  expect(`the daemon to describe a language worth a page, got ${JSON.stringify(described)}`, described.words > 100 && described.actions > 60 && described.events > 100);

  const entry = (on, id) => on.locator(`[data-entry="${id}"]`);
  const partOf = (on, name) => on.locator(`.ref-part[data-part="${name}"]`);
  const counts = (on) => on.locator('.ref-part').evaluateAll((parts) => Object.fromEntries(parts.map((part) => [part.dataset.part, Number(part.dataset.count)])));
  // Where an element is, and whether all of it is in what the page shows.
  const inView = (one) =>
    one.evaluate((found) => {
      const { top, bottom, left, right } = found.getBoundingClientRect();
      const view = found.closest('.view').getBoundingClientRect();
      const head = found.closest('.reference').querySelector('.ref-top').getBoundingClientRect();
      return { top, bottom, left, right, under: head.bottom, within: top >= head.bottom - 0.5 && bottom <= view.bottom + 0.5 && left >= view.left - 0.5 && right <= view.right + 0.5 };
    });

  // ---- opened from the side panel --------------------------------------------------
  await side(page, 'Resources');
  const line = page.locator('.nav').getByText('Reference', { exact: true });
  await line.waitFor();
  const began = Date.now();
  readings = 0;
  await line.click();
  await page.getByRole('heading', { name: 'Reference', level: 1 }).waitFor();
  await partOf(page, 'events').waitFor();
  const took = Date.now() - began;
  expect(`the page to be one tab called Reference, got ${await page.locator('.tab', { hasText: 'Reference' }).count()}`, (await page.locator('.tab', { hasText: 'Reference' }).count()) === 1);
  // The three parts, in their order, each saying how many things the daemon describes of that kind.
  const first = await counts(page);
  expect(`the three parts in their order, got ${JSON.stringify(first)}`, Object.keys(first).join() === 'words,actions,events');
  expect(`each part to count what the daemon describes ${JSON.stringify(described)}, got ${JSON.stringify(first)}`, JSON.stringify(first) === JSON.stringify(described));
  const heads = await page.locator('.ref-part-head').allInnerTexts();
  expect(`each part's head to say what it is and how many, got ${JSON.stringify(heads)}`, heads.map((head) => head.replace(/\s+/g, ' ').trim()).join(' | ') === `Keywords ${described.words} | Actions ${described.actions} | Events ${described.events}`);
  // Every action and every event the daemon describes is a line of its part, by its name.
  const listed = (name, kind) => partOf(page, name).locator(`.ref-entries > .ref-entry[data-kind="${kind}"] > .ref-line .ref-name`).allInnerTexts();
  const sorted = (names) => [...names].sort((a, b) => a.localeCompare(b));
  expect('every action to be listed, by name', JSON.stringify(await listed('actions', 'action')) === JSON.stringify(sorted(language.actions.map((action) => action.name))));
  expect('every event to be listed, by topic', JSON.stringify(await listed('events', 'event')) === JSON.stringify(sorted(language.events.map((event) => event.topic))));
  const sentence = language.actions.find((action) => action.name === 'collection.ensure').summary;
  expect('an action to say its sentence as the daemon says it', (await entry(page, 'action/collection.ensure').locator('> .ref-says').innerText()) === sentence.replace(/`/g, ''));
  // Folded as it opens, it is a line for each thing; how long it took to draw is said, and what it weighs.
  const drawn = await page.evaluate(() => document.querySelectorAll('.reference *').length);
  console.log(`the Reference: ${described.words} keywords, ${described.actions} actions and ${described.events} events drawn in ${took} ms as ${drawn} elements`);
  expect(`the page to be drawn at once without drawing every argument (${drawn} elements)`, drawn < 12000 && (await entry(page, QUIET).count()) === 0);
  expect(`the language to be read once for it, and it was asked for ${readings} times`, readings <= 1);
  await page.screenshot({ path: `${out}/reference-1-page.png` });

  // A part folds, and what is under it is put away; unfolded, it is as it was.
  await partOf(page, 'words').locator('.ref-part-head').click();
  expect('a folded part to draw none of its entries', (await partOf(page, 'words').locator('.ref-entry').count()) === 0);
  await partOf(page, 'words').locator('.ref-part-head').click();
  await entry(page, 'word/step').waitFor();
  // A word says what goes after it, in the daemon's words.
  await entry(page, 'word/step').locator('> .ref-line button.ref-head').click();
  const timeout = entry(page, 'word/step/timeout');
  await timeout.waitFor();
  const after = language.shapes.find((shape) => shape.name === 'step').fields.find((field) => field.name === 'timeout');
  expect(`a word to say what goes after it and its sentence, got "${await timeout.innerText()}"`, (await timeout.innerText()).includes(after.after) && (await timeout.innerText()).includes(after.help));
  await entry(page, 'word/step').locator('> .ref-line button.ref-head').click();

  // ---- found ------------------------------------------------------------------------
  const box = page.getByLabel('Find in the Reference');
  await box.fill('quiet');
  const quiet = entry(page, QUIET);
  await quiet.waitFor();
  const said = language.actions.find((action) => action.name === 'collection.ensure').args.find((arg) => arg.name === 'fields').within.keys.find((key) => key.name === 'quiet');
  expect(`the key to say its kind and its sentence, got "${await quiet.innerText()}"`, (await quiet.innerText()).includes('quiet') && (await quiet.innerText()).includes(said.type) && (await quiet.innerText()).includes(said.help));
  // It is shown under what it is a key of: the argument, under its action.
  const above = await quiet.evaluate((found) => {
    const up = [];
    for (let one = found.parentElement?.closest('[data-entry]'); one; one = one.parentElement?.closest('[data-entry]')) up.push(one.dataset.entry);
    return up;
  });
  expect(`the key to be shown inside its argument and its action, got ${above}`, above.join() === 'action/collection.ensure/fields,action/collection.ensure');
  expect('what each thing in the argument is to be said beside it', (await entry(page, 'action/collection.ensure/fields').locator('> .ref-line').innerText()).includes('a field of the collection'));
  expect('a key that does not say it not to be shown', (await entry(page, 'action/collection.ensure/fields/hidden').count()) === 0 && (await entry(page, 'action/shell').count()) === 0);
  const found = await counts(page);
  expect(`each part to say how many say it, got ${JSON.stringify(found)}`, found.actions >= 1 && found.actions < 40 && (await partOf(page, 'actions').locator('.ref-part-head').innerText()).includes(`${found.actions} found`));
  expect(`the key to be in view with nothing pressed: ${JSON.stringify(await inView(quiet))}`, (await inView(quiet)).within || found.actions > 12);
  await quiet.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/reference-2-quiet.png` });
  // Every word typed, wherever each is said; and nothing where nothing says it.
  await box.fill('news quiet');
  await quiet.waitFor();
  await box.fill('quiet zebra');
  await page.locator('.ref-none').first().waitFor();
  expect(`nothing to be found for what nothing says, got ${JSON.stringify(await counts(page))}`, Object.values(await counts(page)).every((count) => count === 0) && (await page.locator('.ref-none').count()) === 3);

  // ---- a word shown by its example, and where the blueprints there are use it ----------------
  const used = await page.evaluate(async () => (await fetch('/api/blueprints/language/used', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json());
  const RETRY = 'word/step/retry';
  const retried = language.shapes.find((shape) => shape.name === 'step').fields.find((field) => field.name === 'retry');
  await box.fill('retry');
  const retry = entry(page, RETRY);
  await retry.waitFor();
  expect(`retry to say what it is for, got "${await retry.locator('> .ref-says').innerText()}"`, (await retry.locator('> .ref-says').innerText()) === retried.help.replace(/`/g, ''));
  const code = retry.locator(':scope > .ref-inside > .ref-example .ref-code');
  await code.waitFor();
  expect(`its example to be the daemon's whole blueprint, got ${JSON.stringify(await code.innerText())}`, (await code.innerText()).trimEnd() === retried.example.trimEnd());
  expect('its example to be coloured as the editor colours a blueprint', (await code.locator('.bp-field').count()) > 3 && (await code.locator('.bp-action').count()) > 0);
  // What copies it lies over no word of it, and copies it whole.
  const copyOver = await retry.locator('.ref-example').evaluate((example) => {
    const button = example.querySelector('.ref-copy').getBoundingClientRect();
    const lies = (a, b) => a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5;
    return [...example.querySelectorAll('.ref-code-line')].filter((line) => {
      const range = document.createRange();
      range.selectNodeContents(line);
      return [...range.getClientRects()].some((box) => box.width > 0 && lies(box, button));
    }).length;
  });
  expect(`the mark that copies the example to lie over none of its words, and it lies over ${copyOver} lines`, copyOver === 0);
  await retry.locator('.ref-copy').click();
  await page.locator('.toast', { hasText: 'The example was copied.' }).waitFor();
  expect('the example to be copied whole', (await page.evaluate(() => navigator.clipboard.readText())) === retried.example);
  // Used in: the shortest blueprints that ship with it, each with where and the line.
  const places = used.words['step/retry'].places;
  const lines = retry.locator('.ref-used-line');
  expect(`retry to be said to be used where the daemon says, got ${await lines.allInnerTexts()}`, (await lines.count()) === places.length && places.length > 0);
  const firstUse = await lines.first().innerText();
  expect(`a place to say its blueprint and its line, got "${firstUse}"`, firstUse.includes(places[0].blueprint) && firstUse.includes(`line ${places[0].line}`));
  await retry.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/reference-10-retry.png` });
  // Found by a word only its example says.
  await box.fill('max_backoff: 30s');
  await retry.waitFor();
  expect('a word written only in an example to find what it is the example of', (await entry(page, 'word/step/timeout').count()) === 0 || (await entry(page, RETRY).count()) === 1);
  expect('what was found by its example to show the example', (await retry.locator('.ref-code').count()) === 1);
  // An action, with its example: unfolded, a whole blueprint that asks it.
  await box.fill('channel.post');
  const post = entry(page, 'action/channel.post');
  await post.waitFor();
  await post.locator('> .ref-line button.ref-head').click();
  const posted = language.actions.find((action) => action.name === 'channel.post');
  const postCode = post.locator(':scope > .ref-inside > .ref-example .ref-code');
  await postCode.waitFor();
  expect('an action to be shown by its example', (await postCode.innerText()).trimEnd() === posted.example.trimEnd() && posted.example.includes('do: channel.post'));
  expect(`an action to say where it is used, got ${await post.locator('.ref-used-line').count()}`, (await post.locator('.ref-used-line').count()) === used.actions['channel.post'].places.length);
  expect('an action used in more than a handful to say how many more', (await post.locator('.ref-more').innerText()).includes(String(used.actions['channel.post'].more)));
  await post.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/reference-11-channel-post.png` });
  // Nothing over anything else or scrolling sideways with examples shown, at three widths and a phone's.
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    for (const typed of ['retry', 'channel.post']) {
      await box.fill(typed);
      await entry(page, typed === 'retry' ? RETRY : 'action/channel.post').locator('.ref-code').first().waitFor();
      const got = await laidOut(page);
      expect(`nothing of an example over anything else or scrolling sideways at ${width} wide for ${typed}, got ${JSON.stringify(got)}`, got.wrong.length === 0 && got.sideways <= 0 && got.page <= 0);
    }
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await side(page, 'Resources');
  // Pressed, a place opens that blueprint's text at that line, beside the Reference.
  await box.fill('retry');
  await retry.locator('.ref-used-line').first().click();
  await page.locator('.tab', { hasText: places[0].blueprint }).waitFor();
  await page.locator('.cm-content').waitFor();
  // The line the caret was put on, which the editor marks.
  await page.locator('.cm-activeLine', { hasText: 'retry' }).waitFor();
  const opened = await page.locator('.cm-activeLine').innerText();
  expect(`the blueprint to be opened at the line where it says retry, got "${opened}"`, opened.trim().startsWith('retry'));
  expect('the Reference to be a tab still', (await page.locator('.tab', { hasText: 'Reference' }).count()) === 1);
  await page.screenshot({ path: `${out}/reference-12-used-in.png` });
  await page.locator('.tab', { hasText: 'Reference' }).click();
  await box.fill('');

  // ---- an entry's address ------------------------------------------------------------
  await box.fill('quiet');
  await quiet.waitFor();
  await quiet.locator('> .ref-line .ref-address').click();
  await page.locator('.toast', { hasText: 'The address was copied.' }).waitFor();
  const address = await page.evaluate(() => navigator.clipboard.readText());
  expect(`the address to open this page at the entry, got ${address}`, address === `${url}/reference/${QUIET}`);
  // Opened in a second page: the entry unfolded with what it is of, marked, in view, and the bar saying the same.
  const second = await context.newPage();
  second.on('pageerror', (err) => problems.push(err.message));
  await second.goto(address);
  const shown = entry(second, QUIET);
  await shown.waitFor();
  await second.waitForFunction((to) => location.href === to, address);
  await second.locator('.ref-lit').waitFor();
  expect(`the entry to be the one marked, got ${await second.locator('.ref-lit').getAttribute('data-entry')}`, (await second.locator('.ref-lit').getAttribute('data-entry')) === QUIET && (await second.locator('.ref-lit').count()) === 1);
  expect(`the entry to be in view: ${JSON.stringify(await inView(shown))}`, (await inView(shown)).within);
  expect('the whole of its action to be unfolded about it, with nothing left out', (await entry(second, 'action/collection.ensure/fields/hidden').count()) === 1 && (await entry(second, 'action/collection.ensure/views').count()) === 1);
  expect('nothing to be typed in the box of a page opened at an entry', (await second.getByLabel('Find in the Reference').inputValue()) === '');
  expect(`the tab to be the Reference, got "${await second.locator('.tab', { hasText: 'Reference' }).count()}"`, (await second.locator('.tab', { hasText: 'Reference' }).count()) === 1);
  await second.screenshot({ path: `${out}/reference-3-address.png` });
  // An address under an open page, written as addresses were before they were paths: an event, and one that names
  // nothing, which opens the page as it stands; the bar then says each the new way.
  await second.evaluate(() => (location.hash = '/reference/event/board.item.moved'));
  await second.waitForFunction(() => location.pathname === '/reference/event/board.item.moved' && location.hash === '');
  await second.locator('.ref-lit[data-entry="event/board.item.moved"]').waitFor();
  expect(`an event opened by its address to be in view: ${JSON.stringify(await inView(second.locator('.ref-lit')))}`, (await inView(second.locator('.ref-lit'))).within);
  await second.evaluate(() => (location.hash = '/reference/action/no.such.thing'));
  await second.waitForFunction(() => location.pathname === '/reference/action/no.such.thing' && location.hash === '');
  expect('an address that names nothing to leave the page as it was', (await second.locator('.ref-lit').getAttribute('data-entry')) === 'event/board.item.moved' && (await second.locator('.tab', { hasText: 'Reference' }).count()) === 1);
  await second.close();

  // ---- an event, by its topic ----------------------------------------------------------
  await box.fill('board.item.moved');
  const moved = entry(page, 'event/board.item.moved');
  await moved.waitFor();
  const event = language.events.find((one) => one.topic === 'board.item.moved');
  expect(`the event to say its sentence and who hears it, got "${await moved.innerText()}"`, (await moved.innerText()).includes(event.summary) && (await moved.locator('[data-mark="hears"]').innerText()).includes(event.hears));
  await moved.locator('> .ref-line button.ref-head').click();
  const carries = moved.locator('.ref-list[data-list="carries"]');
  await carries.waitFor();
  const carried = await carries.locator('dt').allInnerTexts();
  expect(`the event to list what it carries, got ${carried}`, JSON.stringify(carried) === JSON.stringify(event.fields.map((field) => field.name)));
  const note = event.fields.find((field) => !field.always);
  expect('what is carried only some of the time to say so', !note || (await carries.innerText()).includes(`${note.type}, sometimes`));
  await page.screenshot({ path: `${out}/reference-4-event.png` });
  // What is said only inside an entry unfolds it: what a move carries of what was said as it was moved.
  await moved.locator('> .ref-line button.ref-head').click();
  await carries.waitFor({ state: 'detached' });
  await box.fill(note ? note.help : 'board.item.moved');
  await carries.waitFor();
  await box.fill('');
  await partOf(page, 'words').waitFor();
  expect(`everything to be listed again once the box is empty, got ${JSON.stringify(await counts(page))}`, JSON.stringify(await counts(page)) === JSON.stringify(described));

  // ---- nothing over anything else, at three widths and a phone's -------------------------
  // With an action unfolded to every level, which is as much as an entry holds.
  await entry(page, 'action/collection.ensure').locator('> .ref-line button.ref-head').click();
  await entry(page, QUIET).waitFor();
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    await entry(page, 'action/collection.ensure').scrollIntoViewIfNeeded();
    const got = await laidOut(page);
    expect(`nothing of the page over anything else or scrolling sideways at ${width} wide, got ${JSON.stringify(got)}`, got.wrong.length === 0 && got.sideways <= 0 && got.page <= 0 && got.entries > 300);
    await page.screenshot({ path: `${out}/reference-5-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await side(page, 'Resources');

  // A phone: opened by its address, an action unfolded and a key found, by touch.
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const small = await phone.newPage();
  small.on('pageerror', (err) => problems.push(err.message));
  await small.goto(`${url}/reference/${QUIET}`);
  await small.getByLabel('Email').fill('ada@example.com');
  await small.getByLabel('Password').fill('correct horse');
  await small.getByRole('button', { name: 'Sign in', exact: true }).last().tap();
  // Opened by somebody who was not signed in, the address waited and is opened once they are.
  await small.locator(`.ref-lit[data-entry="${QUIET}"]`).waitFor();
  const narrow = await laidOut(small);
  expect(`nothing of the page over anything else or scrolling sideways on a phone, got ${JSON.stringify(narrow)}`, narrow.wrong.length === 0 && narrow.sideways <= 0 && narrow.page <= 0);
  expect(`the entry to be in view on a phone: ${JSON.stringify(await inView(entry(small, QUIET)))}`, (await inView(entry(small, QUIET))).within);
  await small.screenshot({ path: `${out}/reference-6-phone.png` });
  await small.getByLabel('Find in the Reference').fill('unhealthy_after');
  await entry(small, 'action/service.start/health/unhealthy_after').waitFor();
  const narrower = await laidOut(small);
  expect(`what is found to fit a phone, got ${JSON.stringify(narrower)}`, narrower.wrong.length === 0 && narrower.sideways <= 0 && narrower.page <= 0);
  // A word's example and where it is used, on a phone: the example scrolls sideways in its own box, and nothing else does.
  await small.getByLabel('Find in the Reference').fill('retry');
  await entry(small, RETRY).locator('.ref-code').waitFor();
  const phoned = await laidOut(small);
  expect(`an example to fit a phone, got ${JSON.stringify(phoned)}`, phoned.wrong.length === 0 && phoned.sideways <= 0 && phoned.page <= 0);
  await entry(small, RETRY).scrollIntoViewIfNeeded();
  await small.screenshot({ path: `${out}/reference-6-phone-retry.png` });
  await phone.close();

  // ---- from a word in a blueprint's text ---------------------------------------------------
  await unfold(page, page.locator('.nav button.nav-item', { has: page.getByText('std', { exact: true }) }));
  await page.locator('.nav').getByText('std/board', { exact: true }).click();
  await page.locator('.view-head').getByText('std/board', { exact: false }).first().waitFor();
  await page.getByRole('tab', { name: 'Text' }).last().click();
  await page.locator('.cm-content').waitFor();
  await page.locator('.view-head').getByText('checks').waitFor({ timeout: 8000 });
  // A word with an example: the card shows it under its sentence, coloured, and moves nothing.
  const steps = page.locator('.cm-content .bp-field', { hasText: /^steps$/ }).first();
  await steps.waitFor();
  await steps.evaluate((found) => found.scrollIntoView({ block: 'center', inline: 'nearest' }));
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const stepsAt = async () => JSON.stringify(await steps.evaluate((found) => [found.getBoundingClientRect().x, found.getBoundingClientRect().y, found.closest('.cm-scroller').scrollTop]));
  const stepsBefore = await stepsAt();
  await steps.hover();
  const shownBy = page.locator('.cm-tooltip-hover .bp-note-example');
  await shownBy.waitFor();
  const stepsWord = language.shapes.find((shape) => shape.name === 'blueprint').fields.find((field) => field.name === 'steps');
  expect(`the card to show the word's example, got ${JSON.stringify(await shownBy.innerText())}`, (await shownBy.innerText()).trimEnd() === stepsWord.example.trimEnd() && (await shownBy.locator('.bp-field').count()) > 0);
  const cardSays = (await page.locator('.cm-tooltip-hover').innerText()).toLowerCase();
  const sentenceAt = cardSays.indexOf(stepsWord.help.replace(/`/g, '').slice(0, 20).toLowerCase());
  expect(`the card to say the sentence over the example, got ${JSON.stringify(cardSays)}`, sentenceAt >= 0 && sentenceAt < cardSays.indexOf('example\n'));
  expect('the card to keep its link to the Reference', (await page.locator('.cm-tooltip-hover').getByRole('button', { name: 'in the Reference', exact: true }).count()) === 1);
  expect(`nothing to have moved for the card with an example, from ${stepsBefore} to ${await stepsAt()}`, (await stepsAt()) === stepsBefore);
  // It stays as the pointer crosses to it, and onto its example.
  const stepsBox = await steps.boundingBox();
  const cardBox = await page.locator('.cm-tooltip-hover').boundingBox();
  const exampleBox = await shownBy.boundingBox();
  const across = Math.min(Math.max(stepsBox.x + stepsBox.width / 2, cardBox.x + 6), cardBox.x + cardBox.width - 6);
  await page.mouse.move(across, stepsBox.y + stepsBox.height / 2, { steps: 4 });
  await page.mouse.move(across, exampleBox.y + exampleBox.height / 2, { steps: 10 });
  await page.waitForTimeout(500);
  expect('the card to stay while the pointer is on its example', (await shownBy.count()) === 1);
  await page.screenshot({ path: `${out}/reference-7-card-example.png` });
  await page.mouse.move(5, 5);
  await shownBy.waitFor({ state: 'detached' });
  // Only the lines in view are drawn: the text is scrolled until the word is.
  const scroller = page.locator('.cm-scroller').last();
  const frame = () => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const word = page.locator('.cm-content .bp-key', { hasText: /^quiet$/ }).first();
  for (let turn = 0; turn < 60 && (await word.count()) === 0; turn++) {
    await scroller.evaluate((within) => within.scrollBy(0, 240));
    await frame();
  }
  await word.evaluate((found) => found.scrollIntoView({ block: 'center', inline: 'nearest' }));
  await frame();
  const card = page.locator('.cm-tooltip-hover');
  const where = async () => JSON.stringify(await word.evaluate((found) => [found.getBoundingClientRect().x, found.getBoundingClientRect().y, found.closest('.cm-scroller').scrollTop]));
  const before = await where();
  await word.hover();
  const link = card.getByRole('button', { name: 'in the Reference', exact: true });
  await link.waitFor();
  expect('the card to say what the word means over its link', (await card.innerText()).includes('Its changes are not news'));
  expect(`nothing to have moved for the card, from ${before} to ${await where()}`, (await where()) === before);
  await page.screenshot({ path: `${out}/reference-7-card.png` });
  // The card stays as the pointer crosses to its link, which is at its foot.
  const at = await word.boundingBox();
  const over = await card.boundingBox();
  const to = await link.boundingBox();
  const x = Math.min(Math.max(at.x + at.width / 2, over.x + 6), over.x + over.width - 6);
  await page.mouse.move(x, at.y + at.height / 2, { steps: 4 });
  await page.mouse.move(x, over.y + over.height - 6, { steps: 8 });
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 8 });
  // Longer than the editor waits before it says anything of what is pointed at.
  await page.waitForTimeout(500);
  expect('the card to stay while the pointer is on its link', (await link.count()) === 1);
  await page.mouse.down();
  await page.mouse.up();
  // The Reference, at that key: unfolded with what it is of, marked and in view, in the tab that was open.
  await page.locator(`.ref-lit[data-entry="${QUIET}"]`).waitFor();
  expect(`the key to be in view: ${JSON.stringify(await inView(entry(page, QUIET)))}`, (await inView(entry(page, QUIET))).within);
  expect(`one tab of the Reference still, got ${await page.locator('.tab', { hasText: 'Reference' }).count()}`, (await page.locator('.tab', { hasText: 'Reference' }).count()) === 1);
  expect('nothing to be typed in the box', (await box.inputValue()) === '');
  await card.waitFor({ state: 'detached' });
  await page.screenshot({ path: `${out}/reference-8-from-the-card.png` });

  // ---- from Help -------------------------------------------------------------------------
  await page.getByRole('button', { name: 'Close Reference', exact: true }).click();
  await page.locator('.tab', { hasText: 'Reference' }).waitFor({ state: 'detached' });
  await page.getByRole('navigation', { name: 'Resources' }).locator('.space-foot').getByRole('button', { name: /^Help/ }).click();
  const menu = page.getByRole('menu', { name: 'Help', exact: true });
  await menu.getByRole('menuitem', { name: 'Reference', exact: true }).click();
  await page.getByRole('heading', { name: 'Reference', level: 1 }).waitFor();
  await partOf(page, 'events').waitFor();
  // Put away and opened again, the page is as it was left: what was marked is, and nothing is asked of the daemon again.
  expect(`the language not to be read again for a page that is opened again, and it was asked for ${readings} times in all`, readings <= 2);

  // ---- the other of dark and light ----------------------------------------------------------
  const other = scheme === 'dark' ? 'light' : 'dark';
  const ground = () => page.evaluate(() => getComputedStyle(document.querySelector('.reference').closest('.view')).backgroundColor);
  const was = await ground();
  await litAs(page, other);
  await box.fill('quiet');
  await entry(page, QUIET).waitFor();
  expect(`the page to be drawn in the other of dark and light, and its ground is ${await ground()} as it was`, (await ground()) !== was);
  await page.screenshot({ path: `${out}/reference-9-${other}.png` });
  await litAs(page, scheme);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  // What the page looked like when it went wrong says more than the error does.
  await page?.screenshot({ path: `${out}/reference-0-failed.png` }).catch(() => {});
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
