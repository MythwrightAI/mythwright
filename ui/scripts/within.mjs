// What is written inside a step's arguments, explained and offered: the
// keys of a field of a table, the few words one of them may be.
//
//   DAEMON=../target/debug/factory node scripts/within.mjs [out-dir]
//
// The language knows a step's arguments by name and no further; what is
// inside one is its action's to read, and an action may say what it reads
// there. This holds what the editor makes of that, by what the page shows
// and by the boxes: a key of a field of the board that ships pointed at,
// in a card that lies over the page, moves nothing and stays as the
// pointer crosses to it; one of a key's few words; a key nobody describes,
// of which nothing is said; and, in a project's own blueprint, the keys a
// field does not have yet offered as it is written, in braces and as a
// block, one chosen, and its words after it. The environment variables
// are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
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
const data = join(out, 'within-data');
const home = join(out, 'within-home');
// The project's folder is one that was said, so that its Repo is listed under it.
const shop = join(out, 'within-shop');

// A table of the project's own: one field as any is written, and one
// with a key no action says anything of, which is the action's to take
// or refuse and never the editor's.
const SHELVES = `blueprint: shelves
description: The shelves of the shop.
permissions: [projects]

steps:
  shelves:
    do: collection.ensure
    with:
      name: shelves
      fields:
        title: { type: text, required: true }
        tint: { type: text, colour: red }
`;

// The keys a field takes and the kinds it may be are what the daemon says
// is written inside `fields` of `collection.ensure` (`kernel::inside`),
// read once the page is signed in: the editor offers each, in that order,
// and a word added there is offered with nothing written here.
let FIELD = [];
let KINDS = [];
const WIDTHS = [1280, 1000, 760];

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
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
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  page = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
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

  ({ FIELD, KINDS } = await page.evaluate(async () => {
    const language = await (await fetch('/api/blueprints/language', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json();
    const keys = language.actions.find((one) => one.name === 'collection.ensure').args.find((one) => one.name === 'fields').within.keys;
    return { FIELD: keys.map((one) => one.name), KINDS: keys.find((one) => one.name === 'type').words.map((one) => one.name) };
  }));
  expect(`the daemon to say the keys of a field and its kinds, got ${FIELD} and ${KINDS}`, ['type', 'quiet', 'hidden'].every((key) => FIELD.includes(key)) && ['number', 'flag', 'list'].every((kind) => KINDS.includes(kind)));
  const { where, ending } = await page.evaluate(async () => ({
    where: (await (await fetch('/api/projects', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).projects[0].path,
    ending: (await (await fetch('/api/meta')).json()).blueprint_ext,
  }));
  await mkdir(join(where, 'blueprints'), { recursive: true });
  await writeFile(join(where, `blueprints/shelves.${ending}`), SHELVES);

  // Waits for the daemon's check to pass what is in the editor.
  const checks = async () => {
    await page
      .locator('.view-head')
      .getByText('checks')
      .waitFor({ timeout: 8000 })
      .catch(async () => {
        throw new Error(`expected the blueprint to check, and it has: ${await page.locator('.problems').innerText().catch(() => 'no list of problems')}`);
      });
  };

  // ---- the board that ships, as text -------------------------------------------
  // The factory's blueprints are listed under a heading, by family, with nothing to unfold above `std`.
  await side(page, 'Resources');
  await unfold(page, page.locator('.nav button.nav-item', { has: page.getByText('std', { exact: true }) }));
  await page.locator('.nav').getByText('std/board', { exact: true }).click();
  await page.locator('.view-head').getByText('std/board', { exact: false }).first().waitFor();
  // A blueprint that ships opens on what it says in plain words: its text is one press from there.
  await page.getByRole('tab', { name: 'Text' }).last().click();
  const text = page.locator('.cm-content');
  // The editor is a part of the client fetched as it is first shown. Where
  // that part is gone (the client was built twice under the page), the page
  // says so and draws no editor: that is said at once, and not after a wait.
  const missing = page.locator('.toast.notice', { hasText: 'could not be loaded' });
  await text.or(missing).first().waitFor();
  if (await missing.isVisible()) throw new Error(`expected the text of std/board in the editor, and the page says: ${await missing.locator('p').innerText()}`);
  // The head is a line taller once the daemon has checked the text, which moves the text down.
  await checks();

  // Only the lines in view are drawn: a piece of the text is scrolled to
  // until it is, and then put clear of the editor's top, where a card
  // about it has room over it.
  const scroller = page.locator('.cm-scroller').last();
  // A piece of the text by how it is coloured and what it says, in the first line that holds `within` where that is said.
  const piece = (kind, word, within) => {
    const inside = within ? page.locator('.cm-content .cm-line', { hasText: within }).first() : page.locator('.cm-content');
    return inside.locator(`.bp-${kind}`, { hasText: new RegExp(`^${word.replace(/[.$]/g, '\\$&')}$`) }).first();
  };
  const frame = () => page.evaluate(() => new Promise((drawn) => requestAnimationFrame(() => requestAnimationFrame(drawn))));
  const bring = async (kind, word, within) => {
    const one = piece(kind, word, within);
    if ((await one.count()) === 0) {
      await scroller.evaluate((box) => box.scrollTo(0, 0));
      await frame();
    }
    for (let turn = 0; turn < 60 && (await one.count()) === 0; turn++) {
      await scroller.evaluate((box) => box.scrollBy(0, 240));
      await frame();
    }
    expect(`\`${word}\` to be somewhere in the text`, (await one.count()) > 0);
    await one.evaluate((found) => found.scrollIntoView({ block: 'center', inline: 'nearest' }));
    // The editor draws the lines that came into view, which may put another element where this one was.
    await frame();
    await one.waitFor();
    return one;
  };

  const card = page.locator('.cm-tooltip-hover');
  const note = card.locator('.bp-note');
  // Where things are, to be held to when something is drawn over them.
  const boxes = (one) =>
    one.evaluate((found) => {
      const rect = (box) => {
        const { x, y, width, height } = box.getBoundingClientRect();
        return [x, y, width, height].map((value) => Math.round(value * 10) / 10).join(' ');
      };
      const scroller = found.closest('.cm-scroller');
      return {
        word: rect(found),
        line: rect(found.closest('.cm-line')),
        scroller: rect(scroller),
        head: rect(document.querySelector('.view-head')),
        scrolled: `${scroller.scrollTop} ${scroller.scrollLeft}`,
        page: `${document.documentElement.scrollWidth} ${document.documentElement.scrollHeight}`,
      };
    });
  // What is said about one word may lie over the next, so the pointer
  // leaves the text between them, which takes the last one away.
  const pointAt = async (one) => {
    await page.mouse.move(5, 5);
    await card.waitFor({ state: 'detached' });
    await one.hover();
    await note.waitFor();
    return note.innerText();
  };

  // ---- a key of a field, pointed at ---------------------------------------------
  const quiet = await bring('key', 'quiet');
  const before = await boxes(quiet);
  const said = await pointAt(quiet);
  expect(`pointing at \`quiet\` to say what it means, and it says ${JSON.stringify(said)}`, said.includes('quiet: bool') && said.includes("Its changes are not news: no line of a record's history is written for a change to it alone."));
  expect(`it to say what it is a key of, and it says ${JSON.stringify(said)}`, said.includes('In fields of collection.ensure: a field of the collection, by its name.'));
  await page.screenshot({ path: `${out}/within-1-quiet.png` });
  // It lies over the page: in no line of the text, wholly in the window, and nothing under it moved.
  const lying = await card.evaluate((box) => {
    const { left, top, right, bottom } = box.getBoundingClientRect();
    return { position: getComputedStyle(box).position, left, top, right, bottom, wide: window.innerWidth, tall: window.innerHeight };
  });
  expect(`the card to lie over the page and not in it, and it is ${JSON.stringify(lying)}`, lying.position === 'fixed' || lying.position === 'absolute');
  expect(`all of the card to be in the window, and it is at ${JSON.stringify(lying)}`, lying.left >= 0 && lying.top >= 0 && lying.right <= lying.wide && lying.bottom <= lying.tall);
  const after = await boxes(quiet);
  expect(`nothing to have moved for the card, and it went from ${JSON.stringify(before)} to ${JSON.stringify(after)}`, JSON.stringify(before) === JSON.stringify(after));
  // It stays as the pointer crosses to it, so that it can be read to its end.
  const at = await quiet.boundingBox();
  const over = await card.boundingBox();
  const x = Math.min(Math.max(at.x + at.width / 2, over.x + 6), over.x + over.width - 6);
  await page.mouse.move(x, at.y + at.height / 2, { steps: 4 });
  await page.mouse.move(x, over.y + over.height - 8, { steps: 8 });
  await page.mouse.move(over.x + over.width / 2, over.y + over.height / 2, { steps: 8 });
  // Longer than the editor waits before it says anything of what is pointed at.
  await page.waitForTimeout(500);
  expect('the card to stay while the pointer is on it', (await note.count()) === 1 && (await note.innerText()) === said);
  await page.screenshot({ path: `${out}/within-2-crossed.png` });

  // ---- one of a key's few words --------------------------------------------------
  const list = await bring('text', 'list', 'moved: { type: list');
  const word = await pointAt(list);
  expect(`pointing at \`list\` after \`type:\` to say what a list is, and it says ${JSON.stringify(word)}`, word.includes('Several words.') && word.includes('What type may be.') && word.includes('In fields of collection.ensure'));
  await page.screenshot({ path: `${out}/within-3-word.png` });
  // And the key itself lists the words it may be.
  const type = await pointAt(await bring('key', 'type', 'moved: { type: list'));
  expect(`pointing at \`type\` to list what it may be, and it says ${JSON.stringify(type)}`, /one of/i.test(type) && ['choice', 'records'].every((kind) => type.includes(kind)));
  // The argument says what is inside it under what it is.
  const whole = await pointAt(await bring('key', 'fields'));
  expect(`pointing at \`fields\` to list what a field takes, and it says ${JSON.stringify(whole)}`, /each takes/i.test(whole) && whole.includes('hidden'));
  // A way of looking at the table, and what it goes by.
  const kind = await pointAt(await bring('text', 'order', 'order: { kind: order'));
  expect(`pointing at \`order\` after \`kind:\` to say what an order is, and it says ${JSON.stringify(kind)}`, kind.includes('Its records by what comes first') && kind.includes('In views of collection.ensure'));

  // ---- what nobody describes is said nothing of -----------------------------------
  // A topic under `history`, which is a map of the writer's own: the pointer
  // rests on it for longer than the editor waits, and nothing is said.
  const nothing = async (one, what) => {
    await page.mouse.move(5, 5);
    await card.waitFor({ state: 'detached' });
    await one.hover();
    await page.waitForTimeout(700);
    // Read only where there is something to read: asked of nothing, a reading waits for it to come.
    const says = (await card.count()) === 0 ? null : await card.innerText();
    expect(`nothing to be said of ${what}, and it says ${JSON.stringify(says)}`, says === null);
  };
  await nothing(await bring('key', 'board.item.created'), 'a key of an argument that says nothing of what is inside it');
  await nothing(await bring('key', 'moved'), 'the name of a field, which is its writer\'s own');
  // Pointing still says something where there is something to say.
  await pointAt(await bring('key', 'hidden'));

  // ---- at three widths ---------------------------------------------------------------
  for (const width of WIDTHS) {
    await page.setViewportSize({ width, height: 860 });
    const one = await bring('key', 'quiet');
    const was = await boxes(one);
    const says = await pointAt(one);
    const box = await card.boundingBox();
    expect(`the card to say the same at ${width} wide, and it says ${JSON.stringify(says)}`, says === said);
    expect(`all of the card to be in the window at ${width} wide, and it is at ${JSON.stringify(box)}`, box.x >= 0 && box.y >= 0 && box.x + box.width <= width && box.y + box.height <= 860);
    const now = await boxes(one);
    expect(`nothing to have moved for the card at ${width} wide, from ${JSON.stringify(was)} to ${JSON.stringify(now)}`, JSON.stringify(was) === JSON.stringify(now));
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`the page not to scroll sideways at ${width} wide, and it does by ${sideways}`, sideways <= 0);
    await page.screenshot({ path: `${out}/within-4-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.mouse.move(5, 5);
  await card.waitFor({ state: 'detached' });

  // ---- a project's own blueprint, written in ------------------------------------------
  // The side panel was folded away at the narrowest of those widths, and stays so.
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Repo', { exact: true }).click();
  const files = page.locator('.files');
  await files.getByText('blueprints', { exact: true }).click();
  await files.getByText(`shelves.${ending}`).click();
  await page.locator('.cm-line', { hasText: 'blueprint: shelves' }).waitFor();
  await checks();

  const popup = page.locator('.cm-tooltip-autocomplete');
  const offered = () => popup.locator('li .cm-completionLabel').allInnerTexts();
  // The list as it stands once it holds this label, however long that takes to draw.
  const offering = async (label) => {
    await popup.locator('li .cm-completionLabel', { hasText: new RegExp(`^${label}$`) }).first().waitFor();
    return offered();
  };
  const line = async () => (await page.locator('.cm-activeLine').last().innerText()).replace(/\n$/, '');
  // The list does not take Enter in its first moments, so that somebody
  // ending a line as it appears is not given a suggestion they never saw.
  const take = async (label) => {
    await page.waitForFunction((wanted) => document.querySelector('.cm-tooltip-autocomplete li[aria-selected] .cm-completionLabel')?.textContent === wanted, label);
    await page.waitForTimeout(120);
    await page.keyboard.press('Enter');
  };

  // A key nobody describes is left alone here too: nothing said of it, and nothing wrong with it.
  await nothing(piece('key', 'colour'), 'a key no action names');
  expect('a key no action names not to be marked as a mistake', (await page.locator('.problems').count()) === 0);

  // A field in braces: asked for, the keys a field takes, each saying what it is.
  await page.locator('.cm-content').last().click();
  await page.keyboard.press('Control+End');
  await page.waitForFunction(() => [...document.querySelectorAll('.cm-activeLine')].at(-1)?.textContent === '');
  await page.keyboard.type('        stock: { ');
  await page.keyboard.press('Control+Space');
  const keys = await offering('type');
  expect(`the keys of a field to be offered inside its braces, got ${keys}`, keys.join() === FIELD.join());
  const about = page.locator('.cm-completionInfo .bp-note');
  await about.waitFor();
  expect(`the marked key to say what it is, and it says ${JSON.stringify(await about.innerText())}`, (await about.innerText()).includes('What it holds'));
  await page.screenshot({ path: `${out}/within-5-offered.png` });
  // One chosen: put in with its colon, and what it may be offered after it.
  await page.keyboard.type('qu');
  await take('quiet');
  const flags = await offering('true');
  expect(`true and false to be offered after a key that is either, got ${flags}`, flags.join() === 'true,false');
  await take('true');
  expect(`the key and its value to be put in, and the line is ${JSON.stringify(await line())}`, (await line()).trim() === 'stock: { quiet: true');
  // The next: what the field has is not offered again, and a key that is one of a few words is followed by them.
  await page.keyboard.type(', ');
  await page.keyboard.press('Control+Space');
  const rest = await offering('type');
  expect(`a key the field has not to be offered again, got ${rest}`, rest.join() === FIELD.filter((key) => key !== 'quiet').join());
  await page.keyboard.type('ty');
  await take('type');
  const kinds = await offering('number');
  expect(`the kinds of field to be offered after \`type:\`, got ${kinds}`, kinds.join() === KINDS.join());
  await page.screenshot({ path: `${out}/within-6-words.png` });
  await page.keyboard.type('nu');
  await take('number');
  await page.keyboard.type(' }');
  expect(`the field to be written, and the line is ${JSON.stringify(await line())}`, (await line()).trim() === 'stock: { quiet: true, type: number }');

  // Under it, where the next field would be named, nothing is offered: a
  // field is called what its writer calls it.
  await page.keyboard.press('Enter');
  await page.keyboard.press('Control+Space');
  await page.waitForTimeout(400);
  expect(`nothing to be offered where a field is named, and ${await offered()} is`, (await popup.count()) === 0);

  // A field as a block: the same keys on a line of their own, less what it has.
  await page.keyboard.type('aisle:');
  await page.keyboard.press('Enter');
  await page.keyboard.type('type: ');
  await page.keyboard.press('Control+Space');
  expect(`the kinds of field to be offered in a block too, got ${await offering('flag')}`, (await offered()).join() === KINDS.join());
  await page.keyboard.type('fl');
  await take('flag');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Control+Space');
  const under = await offering('hidden');
  expect(`the keys the block does not have to be offered under it, got ${under}`, under.join() === FIELD.filter((key) => key !== 'type').join());
  await page.keyboard.type('hid');
  await take('hidden');
  await offering('true');
  await take('true');
  expect(`the key to be written under the field, and the line is ${JSON.stringify(await line())}`, (await line()).trim() === 'hidden: true');

  // What was written is a blueprint the daemon takes, and what was put in is explained as what ships is.
  await checks();
  expect('nothing wrong with what was written', (await page.locator('.problems').count()) === 0);
  const number = await pointAt(piece('text', 'number'));
  expect(`a word that was chosen to be explained where it was put, and it says ${JSON.stringify(number)}`, number.includes('A number.'));
  await page.screenshot({ path: `${out}/within-7-written.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  // What the page looked like when it went wrong says more than the error does.
  await page?.screenshot({ path: `${out}/within-0-failed.png` }).catch(() => {});
  // And where it stood: which tab and which of a blueprint's views were showing, and what is drawn of a text.
  const stood = await page
    ?.evaluate(() => ({
      tabs: [...document.querySelectorAll('.tab')].map((tab) => `${tab.getAttribute('aria-selected') === 'true' ? '*' : ''}${tab.textContent?.trim()}`),
      heads: [...document.querySelectorAll('.view-head')].map((head) => head.textContent?.trim().slice(0, 160)),
      views: [...document.querySelectorAll('.view-head [role=tablist] [role=tab]')].map((tab) => `${tab.getAttribute('aria-pressed') === 'true' ? '*' : ''}${tab.textContent}`),
      texts: document.querySelectorAll('.cm-content').length,
      drawer: document.querySelector('.drawer')?.textContent?.trim().slice(0, 160) ?? null,
      modal: document.querySelector('.scrim')?.textContent?.trim().slice(0, 160) ?? null,
      notices: [...document.querySelectorAll('.toast')].map((toast) => toast.textContent?.trim().slice(0, 160)),
    }))
    .catch((why) => `not read: ${why}`);
  if (stood) err.message += `\nwhere the page stood: ${JSON.stringify(stood, null, 1)}`;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
