// Finding in a text the editor shows: the bar that Ctrl and F opens over
// a blueprint's text and over a file opened as code.
//
//   DAEMON=../target/debug/factory node scripts/find.mjs [out-dir]
//
// It goes by what the page shows and by the boxes: what the bar says was
// found, where the match is, where the bar is and that no line moved for
// it. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
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
const data = join(out, 'find-data');
const home = join(out, 'find-home');
// The project's folder is one that was said, so that its Repo is listed under it: a folder of its own, beside the data.
const shop = join(out, 'find-shop');

// A project's own blueprint, long enough that most of it is not drawn. Its
// second line runs under where the bar is; `harbour` is said of every step,
// and `lighthouse` once, far down. The daemon lays it out before it is
// written, as saving does, so that what is saved later is this but for what
// was replaced in it.
const STEPS = 90;
let OWN = `blueprint: berths
description: Brings each boat of the harbour in by its turn, which the keeper of the Seawall says.
params:
  quay: { type: string, description: which quay the boats come in to }
  boats: 3
steps:
${Array.from({ length: STEPS }, (_, nth) => `  berth_${nth + 1}:\n    description: Brings boat ${nth + 1} in to the harbour${nth === 77 ? ', past the lighthouse' : ''}.\n    do: noop`).join('\n')}
`;
// A file that is code and no blueprint.
const CODE = `${Array.from({ length: 160 }, (_, nth) => `fn berth_${nth + 1}() -> u32 {\n    let harbour = ${nth + 1};\n    harbour${nth === 140 ? ' + lighthouse()' : ''}\n}\n`).join('\n')}`;

/** How often something is in a text, as the bar counts: no match over another. */
const times = (text, words, { exact = false, whole = false } = {}) => {
  const escaped = words.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const found = text.match(new RegExp(whole ? `(?<![\\p{L}\\p{N}_])${escaped}(?![\\p{L}\\p{N}_])` : escaped, exact ? 'gu' : 'giu'));
  return found ? found.length : 0;
};

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
// A second page, signed in afresh, for the other of dark and light.
let lit = null;
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
  const scheme = process.env.SCHEME ?? 'dark';
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async (colorScheme, viewport = { width: 1280, height: 860 }) => {
    const opened = await (await browser.newContext({ viewport, colorScheme })).newPage();
    opened.on('pageerror', (err) => problems.push(err.message));
    await opened.goto(url);
    await opened.getByLabel('Email').fill('ada@example.com');
    await opened.getByLabel('Password').fill('correct horse');
    await opened.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await opened.locator('.rail').waitFor();
    return opened;
  };
  page = await signIn(scheme);

  const { where, ending } = await page.evaluate(async () => ({
    where: (await (await fetch('/api/projects', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).projects[0].path,
    ending: (await (await fetch('/api/meta')).json()).blueprint_ext,
  }));
  await mkdir(join(where, 'blueprints'), { recursive: true });
  OWN = await page.evaluate(async (source) => (await (await fetch('/api/blueprints/format', { method: 'POST', headers: { 'content-type': 'application/json', 'x-page-key': localStorage.getItem('page-key') ?? '' }, body: JSON.stringify({ source, project: 'shop' }) })).json()).formatted, OWN);
  expect('the daemon to lay the blueprint out', typeof OWN === 'string' && OWN.includes('lighthouse'));
  await writeFile(join(where, `blueprints/berths.${ending}`), OWN);
  await writeFile(join(where, 'blueprints/quay.rs'), CODE);

  // ---- what the check reads of the page ------------------------------------------
  // Everything is asked of the view that is showing. The browser has drawn
  // what was last done once it has drawn twice more: what a press leads to
  // is waited for by what the page then shows, and this only where a press
  // is to be heard before a key follows it.
  const drawn = (on = page) => on.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const bar = (on = page) => on.locator('.finder');
  const words = (on = page) => on.locator('.finder-words');
  const said = (on = page) => on.locator('.finder-count').innerText();
  /** Waits for the bar to say something of what was found, and gives which match and of how many. */
  const says = async (wanted, on = page) => {
    await on
      .waitForFunction((want) => new RegExp(want).test(document.querySelector('.finder-count')?.textContent ?? ''), wanted.source ?? `^${wanted.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`, { timeout: 8000 })
      .catch(async () => {
        throw new Error(`expected the bar to say ${wanted}, and it says "${await said(on).catch(() => 'nothing: there is no bar')}"`);
      });
    const [, at, of] = /^(\d+) of (\d+)/.exec(await said(on)) ?? [];
    return { at: Number(at), of: Number(of) };
  };
  /** Types into the bar a letter at a time, each waited for by what the bar then says of how many there are. */
  const typed = async (text, counts, on = page) => {
    let sofar = await words(on).inputValue();
    for (const letter of text) {
      await on.keyboard.type(letter);
      sofar += letter;
      await on.waitForFunction((want) => document.querySelector('.finder-words')?.value === want, sofar);
      const many = counts(sofar);
      await says(many === 0 ? 'No matches' : new RegExp(`^\\d+ of ${many}$`), on);
    }
  };
  /** Empties the bar's field, as somebody does: everything in it selected, and taken out. */
  const emptied = async (on = page) => {
    await words(on).focus();
    await on.keyboard.press('Control+a');
    await on.keyboard.press('Backspace');
    await on.waitForFunction(() => document.querySelector('.finder-words')?.value === '' && document.querySelector('.finder-count')?.textContent === '');
  };
  /** The boxes of the bar and of the text it lies over, and what is marked. */
  const looked = (on = page) =>
    on.evaluate(() => {
      const box = (el) => {
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return { left: r.left, right: r.right, top: r.top, bottom: r.bottom, width: r.width, height: r.height };
      };
      const view = [...document.querySelectorAll('.view')].find((one) => one.querySelector('.finder')) ?? document.querySelector('.view[data-focused="true"]') ?? document.querySelector('.view');
      const editor = view.querySelector('.cm-editor');
      const finder = view.querySelector('.finder');
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const called = (el) => `${el.tagName.toLowerCase()} "${(el.getAttribute('aria-label') ?? el.textContent ?? '').trim().slice(0, 30)}"`;
      const parts = finder ? [...finder.querySelectorAll('input, button, .finder-count')].map((el) => ({ what: called(el), ...box(el) })) : [];
      const within = box(editor);
      const drawer = box(document.querySelector('.drawer'));
      const selected = box(view.querySelector('.cm-searchMatch-selected'));
      const scroller = box(view.querySelector('.cm-scroller'));
      const first = view.querySelector('.cm-line');
      return {
        bar: box(finder),
        editor: within,
        scroller,
        drawer,
        gutters: box(view.querySelector('.cm-gutters')),
        parts: parts.length,
        // Nothing of the bar over anything else of it, and nothing of it outside the bar or the editor.
        crossed: parts.flatMap((a, at) => parts.slice(at + 1).filter((b) => over(a, b)).map((b) => `${a.what} over ${b.what}`)),
        outside: finder ? parts.filter((one) => one.left < box(finder).left - 0.5 || one.right > box(finder).right + 0.5 || one.top < box(finder).top - 0.5 || one.bottom > box(finder).bottom + 0.5).map((one) => one.what) : [],
        beyond: finder ? box(finder).left < within.left - 0.5 || box(finder).right > within.right + 0.5 || box(finder).top < within.top - 0.5 || box(finder).bottom > within.bottom + 0.5 : false,
        sideways: document.documentElement.scrollWidth - window.innerWidth,
        marked: view.querySelectorAll('.cm-searchMatch').length,
        selected,
        // Whether the match the caret is at is in sight, clear of the bar, and how far it is from the edges of what the text scrolls in.
        hidden: selected && finder ? over(selected, box(finder)) : false,
        above: selected ? selected.top - scroller.top : null,
        below: selected ? scroller.bottom - selected.bottom : null,
        line: parseFloat(getComputedStyle(view.querySelector('.cm-line')).lineHeight),
        lines: view.querySelectorAll('.cm-line').length,
        // Where the first line that is drawn is, with what it says: a line that moved is seen by it.
        first: first ? { says: first.textContent.slice(0, 40), top: first.getBoundingClientRect().top, left: first.getBoundingClientRect().left } : null,
        scrolled: view.querySelector('.cm-scroller').scrollTop,
        keyboard: document.activeElement?.className ?? '',
        choice: document.activeElement instanceof HTMLInputElement ? [document.activeElement.selectionStart, document.activeElement.selectionEnd, document.activeElement.value.length] : null,
        switches: finder ? [...finder.querySelectorAll('.finder-switch')].map((one) => one.getAttribute('aria-pressed')) : [],
        invalid: finder?.querySelector('.finder-words')?.getAttribute('aria-invalid') ?? null,
      };
    });
  /**
   * Waits for the match the caret is at to be in sight: drawn, inside what the text scrolls in with so many lines
   * about it, and clear of the bar. A match is scrolled to a moment after the bar says which one it is.
   */
  const inSight = async (when, about = 0, on = page) => {
    await on
      .waitForFunction((lines) => {
        const view = [...document.querySelectorAll('.view')].find((one) => one.querySelector('.finder'));
        const [match, scroller, finder] = ['.cm-searchMatch-selected', '.cm-scroller', '.finder'].map((what) => view?.querySelector(what)?.getBoundingClientRect());
        if (!match || !scroller || !finder) return false;
        const room = lines * parseFloat(getComputedStyle(view.querySelector('.cm-line')).lineHeight);
        const under = Math.min(match.right, finder.right) - Math.max(match.left, finder.left) > 0.5 && Math.min(match.bottom, finder.bottom) - Math.max(match.top, finder.top) > 0.5;
        return !under && match.top - scroller.top >= room && scroller.bottom - match.bottom >= room;
      }, about, { timeout: 8000 })
      .catch(async () => {
        const now = await looked(on);
        throw new Error(`expected the match ${when} to be in sight with ${about} lines about it and clear of the bar, got ${JSON.stringify(now.selected)} with ${now.above} above and ${now.below} below, lines ${now.line} tall, under the bar: ${now.hidden}`);
      });
    return looked(on);
  };
  const laidOut = async (when, on = page) => {
    const now = await looked(on);
    expect(`nothing of the bar to lie over anything else of it ${when}, got ${now.crossed.join('; ')}`, now.crossed.length === 0);
    expect(`nothing of the bar to be outside it ${when}, got ${now.outside.join('; ')}`, now.outside.length === 0);
    expect(`the bar to be inside the editor ${when}, got bar ${JSON.stringify(now.bar)} in ${JSON.stringify(now.editor)}`, !now.beyond);
    expect(`the bar to stand clear of the line numbers ${when}`, now.bar.left >= now.gutters.right - 0.5);
    expect(`nothing to scroll sideways ${when}, got ${now.sideways}`, now.sideways <= 0);
    return now;
  };
  /** Whether a press of a key was taken from the browser: the page keeps the key as it comes down, and is asked once it has been everywhere. */
  const taken = async (key, on = page) => {
    await on.evaluate(() => {
      window.heardKey = null;
      const hear = (event) => {
        // The keys held with it come down first, and are not what is asked about.
        if (['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) return;
        window.heardKey = event;
        window.removeEventListener('keydown', hear, true);
      };
      window.addEventListener('keydown', hear, true);
    });
    await on.keyboard.press(key);
    return on.evaluate(() => window.heardKey?.defaultPrevented ?? null);
  };
  const shot = (name, on = page) => on.screenshot({ path: `${out}/${name}.png` });

  // ---- a blueprint that ships: read, and found in --------------------------------
  const shipped = await readFile(join(repo, 'blueprints/std/fleet.bp'), 'utf8');
  const shippedLines = shipped.split('\n');
  // A word that is in one line only, far down, and nowhere else in any form.
  const far = shippedLines
    .slice(Math.floor(shippedLines.length * 0.7), -12)
    .flatMap((line) => line.match(/[a-z]{7,}/g) ?? [])
    .find((word) => times(shipped, word) === 1);
  expect('a word that is said once, far down the fleet that ships', !!far);
  await side(page, 'Resources');
  await unfold(page, page.locator('.nav button.nav-item', { has: page.getByText('std', { exact: true }) }));
  await page.locator('.nav').getByText('std/fleet', { exact: true }).click();
  await page.locator('.view-head').getByText('std/fleet', { exact: false }).first().waitFor();
  // A blueprint that ships opens on what it says in plain words: its text is one press from there.
  await page.getByRole('tab', { name: 'Text' }).last().click();
  const text = page.locator('.cm-content');
  await text.waitFor();
  await page.locator('.cm-line', { hasText: shippedLines[0] }).waitFor();
  // The page's head is another line taller once the daemon has checked the text, which moves the text down: a line
  // is said to be where it is only once the head is whole.
  await page.locator('.view-head').getByText('checks').waitFor({ timeout: 8000 });
  await drawn();
  const closed = await looked();
  expect(`most of the text not to be drawn: ${closed.lines} lines of ${shippedLines.length}`, closed.lines < shippedLines.length / 2);
  expect('no bar before it is asked for', closed.bar === null && closed.marked === 0);

  // Somebody reading it presses on it, which leaves the keyboard nowhere: a
  // text that is only read does not hold it. Ctrl and F opens the bar for
  // the text all the same, with the keyboard in its field.
  await page.locator('.cm-line').nth(3).click();
  await drawn();
  expect('Ctrl and F over a text that is read to be taken from the browser', (await taken('Control+f')) === true);
  await bar().waitFor();
  const opened = await looked();
  expect(`the keyboard to be in the bar's field, got ${opened.keyboard}`, opened.keyboard.includes('finder-words'));
  expect(`no line of the text to move as the bar opens: ${JSON.stringify(closed.first)} then ${JSON.stringify(opened.first)}`, JSON.stringify(closed.first) === JSON.stringify(opened.first) && opened.scrolled === closed.scrolled);
  expect(`the bar to be at the top right of the text, got ${JSON.stringify(opened.bar)} in ${JSON.stringify(opened.editor)}`, opened.bar.top - opened.editor.top < 12 && opened.editor.right - opened.bar.right < 24 && opened.bar.left > opened.editor.left + opened.editor.width / 3);
  // A text that may not be changed is offered no replacing, by the bar or by its key.
  expect('no replacing to be offered over a text that is only read', (await page.locator('.finder-turn').count()) === 0 && (await page.locator('.finder-with').count()) === 0);
  await page.keyboard.press('Control+h');
  await page.keyboard.press('Control+Alt+f');
  await drawn();
  expect('the key that replaces to do nothing there', (await page.locator('.finder-with').count()) === 0 && (await looked()).parts === 8);
  await laidOut('as it opens');

  // A word that is only in a line far down: found as it is typed, counted, and brought into view.
  await typed(far, (sofar) => times(shipped, sofar));
  expect(`the word far down to be the one match, got ${await said()}`, (await said()) === '1 of 1');
  await page.locator('.cm-searchMatch-selected', { hasText: far }).waitFor();
  const found = await inSight('far down', 2);
  expect(`the text to have been scrolled to it, got ${found.scrolled}`, found.scrolled > closed.scrolled + 1000);
  await shot(`find-1-far-down-1280-${scheme}`);

  // A word said many times: every one in view marked, the one the caret is at more strongly; next, the one before, and round at both ends.
  await emptied();
  const many = times(shipped, 'channel');
  await typed('channel', (sofar) => times(shipped, sofar));
  let at = (await says(new RegExp(`^\\d+ of ${many}$`))).at;
  const marks = await looked();
  expect(`the matches in view to be marked and one of them as the one the caret is at, got ${marks.marked}`, marks.marked >= 2 && marks.marked <= many && (await page.locator('.cm-searchMatch-selected').count()) === 1);
  const step = async (key, by) => {
    await page.keyboard.press(key);
    at = ((at - 1 + by + many) % many) + 1;
    await says(`${at} of ${many}`);
    await inSight(`gone to with ${key}`);
  };
  await step('Enter', 1);
  await step('Enter', 1);
  await step('Shift+Enter', -1);
  await step('F3', 1);
  await step('Shift+F3', -1);
  await step('Control+g', 1);
  await step('Control+Shift+G', -1);
  // Round at the ends: back past the first to the last, and on from the last to the first.
  for (let left = at; left > 0; left -= 1) await step('Shift+Enter', -1);
  expect(`going back past the first to come to the last, got ${at}`, at === many);
  await step('Enter', 1);
  expect(`going on from the last to come to the first, got ${at}`, at === 1);
  await bar().getByRole('button', { name: 'Next match', exact: true }).click();
  at = 2;
  await says(`2 of ${many}`);
  await bar().getByRole('button', { name: 'Previous match', exact: true }).click();
  at = 1;
  await says(`1 of ${many}`);
  await shot(`find-2-bar-1280-${scheme}`);

  // The three switches: as the letters are written, as a whole word, and as an expression.
  await bar().getByRole('button', { name: 'Match case', exact: true }).click();
  await says(new RegExp(`^\\d+ of ${times(shipped, 'channel', { exact: true })}$`));
  await emptied();
  await typed('Card', (sofar) => times(shipped, sofar, { exact: true }));
  expect('the same letters written otherwise to be another count', times(shipped, 'Card', { exact: true }) !== times(shipped, 'card', { exact: true }));
  await bar().getByRole('button', { name: 'Match case', exact: true }).click();
  await says(new RegExp(`^\\d+ of ${times(shipped, 'card')}$`));
  await bar().getByRole('button', { name: 'Whole word', exact: true }).click();
  expect('a word inside a longer one not to be the whole word', times(shipped, 'card', { whole: true }) < times(shipped, 'card'));
  await says(new RegExp(`^\\d+ of ${times(shipped, 'card', { whole: true })}$`));
  expect(`the switches to say how they stand, got ${(await looked()).switches}`, (await looked()).switches.join() === 'false,true,false');
  await bar().getByRole('button', { name: 'Whole word', exact: true }).click();
  await bar().getByRole('button', { name: 'Regular expression', exact: true }).click();
  await emptied();
  const steps = shipped.match(/^ {2}[a-z_]+:$/gimu).length;
  // An expression that does not read yet says so while it is written, and marks nothing.
  await words().focus();
  for (const letter of '^ {2}[') await page.keyboard.type(letter);
  await says('Not an expression');
  const unread = await looked();
  expect(`an expression that does not read to mark nothing and to be said of its field, got ${unread.marked} marked`, unread.marked === 0 && unread.invalid === 'true');
  await shot(`find-3-not-an-expression-1280-${scheme}`);
  for (const letter of 'a-z_]+:$') await page.keyboard.type(letter);
  await says(new RegExp(`^\\d+ of ${steps}$`));
  expect('an expression that reads not to be said to be wrong', (await looked()).invalid === 'false');
  // Put away and opened again, the bar holds the expression as it was written, and not the words it last found.
  await page.keyboard.press('Escape');
  await bar().waitFor({ state: 'detached' });
  await page.keyboard.press('Control+f');
  await bar().waitFor();
  await says(new RegExp(`^\\d+ of ${steps}$`));
  expect(`the bar to hold the expression it was put away with, got "${await words().inputValue()}" with ${(await looked()).switches}`, (await words().inputValue()) === '^ {2}[a-z_]+:$' && (await looked()).switches.join() === 'false,false,true');
  await bar().getByRole('button', { name: 'Regular expression', exact: true }).click();
  await says('No matches');
  await emptied();
  await typed('channel', (sofar) => times(shipped, sofar));

  // The keyboard goes through all of it in order: the words, the switches, next, the one before, close.
  await words().focus();
  const order = [];
  for (let nth = 0; nth < 6; nth += 1) {
    await page.keyboard.press('Tab');
    order.push(await page.evaluate(() => document.activeElement?.getAttribute('aria-label') ?? document.activeElement?.className));
  }
  expect(`Tab to go through the bar in order, got ${order}`, order.join() === 'Match case,Whole word,Regular expression,Next match,Previous match,Close');

  // Something that lies over the page lies over the bar too, and is what Escape then puts away.
  await words().focus();
  await page.keyboard.press('Control+k');
  await page.getByRole('dialog').waitFor();
  const covered = await page.evaluate(() => {
    const box = document.querySelector('.finder').getBoundingClientRect();
    const top = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
    return { bar: top?.closest('.finder') !== null, dimmed: top?.closest('.scrim') !== null };
  });
  expect(`a dialog over the page to lie over the bar, got ${JSON.stringify(covered)}`, !covered.bar && covered.dimmed);
  // The dialog listens for Escape a moment after it is drawn.
  await drawn();
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  expect('Escape under a dialog to put the dialog away and leave the bar', (await bar().count()) === 1);

  // Escape in the bar puts it away: the marks go, no line has moved, and the match it was at is still what is chosen in the text.
  await words().focus();
  at = (await says(new RegExp(`^\\d+ of ${many}$`))).at;
  await step('Enter', 1);
  const before = await looked();
  await page.keyboard.press('Escape');
  await bar().waitFor({ state: 'detached' });
  const away = await looked();
  expect(`the marks to go with the bar, got ${away.marked}`, away.marked === 0);
  expect('no line of the text to move as the bar goes', JSON.stringify(before.first) === JSON.stringify(away.first) && before.scrolled === away.scrolled);
  const chosen = await page.evaluate(() => {
    const drawn = document.querySelector('.cm-selectionBackground')?.getBoundingClientRect();
    return drawn ? { left: drawn.left, top: drawn.top, width: drawn.width } : null;
  });
  expect(`the match it was at to be what is chosen in the text, got ${JSON.stringify(chosen)} for ${JSON.stringify(before.selected)}`, chosen && Math.abs(chosen.left - before.selected.left) < 2 && Math.abs(chosen.top - before.selected.top) < 6 && Math.abs(chosen.width - before.selected.width) < 2);

  // Opened again it holds the words, selected, so that typing replaces them.
  expect('Ctrl and F to open it again', (await taken('Control+f')) === true);
  await bar().waitFor();
  await says(`${at} of ${many}`);
  const again = await looked();
  expect(`the bar to hold what was searched for, selected, got "${await words().inputValue()}" ${again.choice}`, (await words().inputValue()) === 'channel' && again.choice.join() === '0,7,7');
  // And after another tab was shown, though the view and its editor were away meanwhile.
  await page.locator('.nav').getByText('Repo', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop repo', level: 1 }).waitFor();
  await page.locator('.tab', { hasText: 'std/fleet' }).first().click();
  await page.locator('.cm-line').first().waitFor();
  expect('no bar on a view that has just come back', (await bar().count()) === 0);
  // With the keyboard on something else of the page, here a tab of the blueprint's own views.
  await page.getByRole('tab', { name: 'Text' }).last().focus();
  expect('Ctrl and F to be the text\'s with the keyboard on its page outside it', (await taken('Control+f')) === true);
  await bar().waitFor();
  await says(new RegExp(`^(\\d+ of )?${many}( matches)?$`));
  expect(`the bar of a view that came back to hold the words of its tab, got "${await words().inputValue()}"`, (await words().inputValue()) === 'channel' && (await looked()).keyboard.includes('finder-words'));
  // With nothing holding the keyboard, Escape puts the bar away, and the keys that go on to a match are heard.
  await page.locator('.cm-line').nth(2).click();
  await drawn();
  expect('F3 to go on to a match with the keyboard nowhere', (await taken('F3')) === true);
  await says(new RegExp(`^\\d+ of ${many}$`));
  expect('Escape to put the bar away with the keyboard nowhere', (await taken('Escape')) === true);
  await bar().waitFor({ state: 'detached' });

  // What is selected in a text that is only read is what the bar opens with, as in one that is written in.
  // Pressed twice in the middle of the first line's first word of two letters or more, found where the page draws
  // it, so that it is a word however big the letters are.
  // The text is scrolled to its top first, where its first line is drawn: lines out of sight are not. The match F3
  // went on to may still be on its way into view, which the editor does as it next measures, and scrolls the text back
  // there after it was put at its top (three runs in six): so it is put there until it stays, a frame or two later.
  const atTop = () =>
    page.locator('.cm-line').first().evaluate(
      (line) =>
        new Promise((done) => {
          const scroller = line.closest('.cm-scroller');
          scroller.scrollTop = 0;
          requestAnimationFrame(() => requestAnimationFrame(() => done(scroller.scrollTop)));
        }),
    );
  for (let tries = 0; (await atTop()) !== 0; tries++) expect('the text to stay at its top once it is put there', tries < 20);
  const first = page.locator('.cm-line').filter({ hasText: shippedLines[0] }).first();
  await first.scrollIntoViewIfNeeded();
  const word = await first.evaluate((line) => {
    const walk = document.createTreeWalker(line, NodeFilter.SHOW_TEXT);
    const edge = line.getBoundingClientRect();
    for (let text = walk.nextNode(); text; text = walk.nextNode()) {
      for (const found of text.textContent.matchAll(/\p{L}{2,}/gu)) {
        const range = document.createRange();
        range.setStart(text, found.index);
        range.setEnd(text, found.index + found[0].length);
        const box = range.getBoundingClientRect();
        return { x: box.left - edge.left + box.width / 2, y: box.top - edge.top + box.height / 2 };
      }
    }
    return null;
  });
  expect('a word in the first line to be pressed', word !== null);
  await first.dblclick({ position: word });
  await page.waitForFunction(() => document.querySelector('.view[data-focused="true"] .cm-selectionBackground') !== null);
  await drawn();
  await page.keyboard.press('Control+f');
  await bar().waitFor();
  const picked = await words().inputValue();
  expect(`the bar to open with the word selected in a text that is read, got "${picked}" of "${shippedLines[0]}"`, picked.length > 1 && picked !== 'channel' && shippedLines[0].includes(picked));
  await says(new RegExp(`^\\d+ of ${times(shipped, picked)}$`));
  await page.keyboard.press('Escape');
  await bar().waitFor({ state: 'detached' });

  // Where no editor is showing, the key is the browser's as it was.
  await page.getByRole('tab', { name: 'Node' }).last().click();
  await page.locator('.gnode').first().waitFor();
  expect('Ctrl and F to be left to the browser where no text is showing', (await taken('Control+f')) === false && (await bar().count()) === 0);
  await page.getByRole('tab', { name: 'Runs' }).last().click();
  await page.getByText('This blueprint has not run').waitFor();
  expect('and on its runs', (await taken('Control+f')) === false && (await bar().count()) === 0);
  await page.locator('.tab', { hasText: 'Shop repo' }).first().click();
  await page.getByRole('heading', { name: 'Shop repo', level: 1 }).waitFor();
  expect('and on a page that is no text', (await taken('Control+f')) === false && (await bar().count()) === 0);

  // ---- a project's own blueprint: found in, and changed --------------------------
  const files = page.locator('.files');
  await files.getByText('blueprints', { exact: true }).click();
  await files.getByText(`berths.${ending}`).click();
  await page.locator('.cm-line', { hasText: 'blueprint: berths' }).waitFor();
  await page.locator('.view-head').getByText('checks').waitFor({ timeout: 8000 });
  const own = await looked();
  // With the caret in the text the key is the editor's own. What is selected there is what the bar opens with, where it is one line.
  await page.locator('.cm-line', { hasText: 'quay: {' }).dblclick({ position: { x: 30, y: 8 } });
  await page.waitForFunction(() => document.activeElement?.classList.contains('cm-content') && document.querySelector('.view[data-focused="true"] .cm-selectionBackground') !== null);
  await drawn();
  expect('Ctrl and F in the text to be taken from the browser', (await taken('Control+f')) === true);
  await bar().waitFor();
  await says(new RegExp(`^\\d+ of ${times(OWN, 'quay')}$`));
  const seeded = await looked();
  expect(`the bar to open with the word that was selected, selected, got "${await words().inputValue()}"`, (await words().inputValue()) === 'quay' && seeded.keyboard.includes('finder-words') && seeded.choice.join() === '0,4,4');
  expect(`no line to move as it opens over a text that can be changed`, JSON.stringify(own.first) === JSON.stringify(seeded.first));
  expect('replacing to be offered where the text may be changed, shut until it is asked for', (await page.locator('.finder-turn').getAttribute('aria-expanded')) === 'false' && (await page.locator('.finder-with').count()) === 0);
  // Several lines selected are not what is searched for: the bar keeps what it had.
  await page.locator('.cm-line', { hasText: 'boats: 3' }).click();
  await page.waitForFunction(() => document.activeElement?.classList.contains('cm-content'));
  await page.keyboard.press('Shift+ArrowDown');
  await page.keyboard.press('Shift+ArrowDown');
  await page.keyboard.press('Control+f');
  await page.waitForFunction(() => document.activeElement?.classList.contains('finder-words'));
  expect(`several lines selected not to be searched for, got "${await words().inputValue()}"`, (await words().inputValue()) === 'quay');

  // A match in the first lines, under where the bar is: it is given room, and is not under the bar.
  await emptied();
  await typed('Seawall', (sofar) => times(OWN, sofar));
  await page.locator('.cm-searchMatch-selected', { hasText: 'Seawall' }).waitFor();
  const under = await inSight('in the first lines');
  expect(`the text to have been moved down for it, since it was under the bar: its first line was at ${own.first.top} and is at ${under.first.top}`, under.first.top > own.first.top + 5 && under.scrolled === 0);
  await shot(`find-4-first-lines-1280-${scheme}`);
  // The word far down, in this text too.
  await emptied();
  await typed('lighthouse', (sofar) => times(OWN, sofar));
  await inSight('far down its own text', 2);
  expect(`the word far down to be the one match, got ${await said()}`, (await said()) === '1 of 1');
  // Put away, the text is where it was before anything was given room.
  await page.keyboard.press('Escape');
  await bar().waitFor({ state: 'detached' });
  await page.waitForFunction(() => document.activeElement?.classList.contains('cm-content'));
  await page.keyboard.press('Control+Home');
  await page.locator('.cm-line', { hasText: 'blueprint: berths' }).waitFor();
  // Where its first line is, with how far the text is scrolled, says where the text is laid out: the caret brought to the top leaves it scrolled by a pixel.
  await page
    .waitForFunction((top) => {
      const view = document.querySelector('.view[data-focused="true"]');
      return view.querySelector('.cm-line')?.getBoundingClientRect().top + view.querySelector('.cm-scroller').scrollTop === top && view.querySelector('.cm-scroller').scrollTop < 4;
    }, own.first.top + own.scrolled, { timeout: 8000 })
    .catch(async () => {
      const now = await looked();
      throw new Error(`expected the text to be where it was once the bar is away: its first line was at ${own.first.top} and is at ${JSON.stringify(now.first)}, scrolled ${now.scrolled}`);
    });

  // What is applied with is asked in a drawer beside the text. The bar is put beside the drawer and not under it;
  // Ctrl and F in a field of the drawer is the browser's; and Escape is for one thing at a time.
  await page.locator('.view-head button.primary').last().click();
  const asks = page.locator('.drawer .applying');
  await asks.waitFor();
  await asks.getByLabel(/^quay/).focus();
  expect('Ctrl and F in a field of the drawer to be left to the browser', (await taken('Control+f')) === false && (await bar().count()) === 0);
  expect('and the keyboard to stay in that field', await page.evaluate(() => document.activeElement?.closest('.drawer') !== null));
  await page.locator('.cm-line', { hasText: 'blueprint: berths' }).click({ position: { x: 30, y: 8 } });
  await page.waitForFunction(() => document.activeElement?.classList.contains('cm-content'));
  await page.keyboard.press('Control+f');
  await bar().waitFor();
  await page.waitForFunction(() => {
    const [finder, drawer] = [document.querySelector('.finder'), document.querySelector('.drawer')];
    return finder && drawer && finder.getBoundingClientRect().right <= drawer.getBoundingClientRect().left;
  });
  const beside = await laidOut('beside a drawer');
  expect(`the bar to be beside the drawer, got ${JSON.stringify(beside.bar)} and ${JSON.stringify(beside.drawer)}`, beside.bar.right <= beside.drawer.left && beside.bar.width >= 280);
  await shot(`find-5-beside-a-drawer-1280-${scheme}`);
  // Escape in the bar: the bar and nothing else.
  await page.keyboard.press('Escape');
  await bar().waitFor({ state: 'detached' });
  expect('Escape in the bar to leave the drawer beside the text', (await asks.count()) === 1);
  await page.waitForFunction(() => document.activeElement?.classList.contains('cm-content'));
  // With something offered in the text, Escape is for that first; then for the bar; then for the drawer.
  await page.keyboard.press('Control+f');
  await bar().waitFor();
  await page.locator('.cm-line', { hasText: 'do: noop' }).first().click({ position: { x: 90, y: 8 } });
  await page.waitForFunction(() => document.activeElement?.classList.contains('cm-content'));
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Control+Space');
  const offered = page.locator('.cm-tooltip-autocomplete');
  await offered.waitFor();
  await drawn();
  await page.keyboard.press('Escape');
  await offered.waitFor({ state: 'detached' });
  expect('Escape with something offered in the text to put that away and nothing else', (await bar().count()) === 1 && (await asks.count()) === 1);
  await page.keyboard.press('Escape');
  await bar().waitFor({ state: 'detached' });
  expect('the next Escape in the text to put the bar away and leave the drawer', (await asks.count()) === 1);
  await page.keyboard.press('Escape');
  await asks.waitFor({ state: 'detached' });
  // The line that was added for it is taken back.
  await page.keyboard.press('Control+z');
  await page.waitForFunction(() => !document.querySelector('.view[data-focused="true"] .view-head')?.textContent.includes('unsaved'));

  // Replacing: this one, and every one, each taken back whole by one Ctrl and Z.
  const boats = times(OWN, 'harbour');
  const reads = (wanted) => page.locator('.cm-line', { hasText: wanted }).first().waitFor();
  await page.keyboard.press('Control+Home');
  await page.keyboard.press('Control+h');
  await page.locator('.finder-with').waitFor();
  expect('the key that replaces to open the second line of the bar', (await page.locator('.finder-turn').getAttribute('aria-expanded')) === 'true');
  await emptied();
  await typed('harbour', (sofar) => times(OWN, sofar));
  await says(`1 of ${boats}`);
  await page.locator('.finder-with').focus();
  for (const letter of 'haven') await page.keyboard.type(letter);
  await page.waitForFunction(() => document.querySelector('.finder-with')?.value === 'haven');
  await inSight('to be replaced');
  await laidOut('with its second line open');
  await shot(`find-6-replace-1280-${scheme}`);
  const order2 = [];
  await bar().getByRole('button', { name: 'Close', exact: true }).focus();
  for (let nth = 0; nth < 4; nth += 1) {
    await page.keyboard.press('Tab');
    order2.push(await page.evaluate(() => document.activeElement?.getAttribute('aria-label') ?? document.activeElement?.textContent.trim()));
  }
  expect(`Tab to go on from the first line to what replaces, got ${order2}`, order2.join() === 'Toggle replace,Replace with,Replace,Replace all');
  const whole = () => page.evaluate(async (path) => (await (await fetch(`/api/projects/shop/file?path=${encodeURIComponent(path)}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).content, `blueprints/berths.${ending}`);
  const unsaved = (on) => page.waitForFunction((want) => (document.querySelector('.view[data-focused="true"] .view-head')?.textContent.includes('unsaved') ?? false) === want, on);
  const inText = async () => {
    await bar().waitFor({ state: 'detached' });
    await page.waitForFunction(() => document.activeElement?.classList.contains('cm-content'));
  };
  // This one: the match the caret is at is replaced, and the next is gone to.
  await bar().getByRole('button', { name: 'Replace', exact: true }).click();
  await says(`1 of ${boats - 1}`);
  await unsaved(true);
  await reads('Brings each boat of the haven in');
  // Escape gives the keyboard back to the text, and one Ctrl and Z there takes the replacing back whole.
  await page.keyboard.press('Escape');
  await inText();
  await page.keyboard.press('Control+z');
  await reads('Brings each boat of the harbour in');
  await unsaved(false);
  // Every one, as one change, and taken back as one.
  await page.keyboard.press('Control+h');
  await page.locator('.finder-with').waitFor();
  await says(new RegExp(`^(\\d+ of )?${boats}( matches)?$`));
  expect(`the bar to hold what it was to be replaced with, got "${await page.locator('.finder-with').inputValue()}"`, (await page.locator('.finder-with').inputValue()) === 'haven');
  await bar().getByRole('button', { name: 'Replace all', exact: true }).click();
  await says('No matches');
  await unsaved(true);
  await reads('Brings each boat of the haven in');
  await page.keyboard.press('Escape');
  await inText();
  await page.keyboard.press('Control+z');
  await reads('Brings each boat of the harbour in');
  await unsaved(false);
  await page.keyboard.press('Control+f');
  await bar().waitFor();
  await says(new RegExp(`^(\\d+ of )?${boats}( matches)?$`));
  // Done again, and something typed in the text straight after it: both are there, since what replacing changed
  // is the editor's own change as typing is, and is never put back to what the text was a moment before.
  await bar().getByRole('button', { name: 'Replace all', exact: true }).click();
  await page.keyboard.press('Escape');
  await inText();
  await page.keyboard.press('Control+Home');
  for (const letter of '# kept') await page.keyboard.type(letter);
  await reads(/^# keptblueprint: berths$/);
  await reads('Brings each boat of the haven in');
  for (let nth = 0; nth < '# kept'.length; nth += 1) await page.keyboard.press('Backspace');
  await reads(/^blueprint: berths$/);
  // Saved from the bar: what the daemon then has is the text with every one replaced.
  await page.keyboard.press('Control+f');
  await bar().waitFor();
  await says('No matches');
  expect('Ctrl and S in the bar to save, and not to be the browser\'s', (await taken('Control+s')) === true);
  await unsaved(false);
  const kept = await whole();
  const wanted = OWN.replaceAll('harbour', 'haven');
  const differs = kept.split('\n').findIndex((line, at) => line !== wanted.split('\n')[at]);
  expect(`what the daemon has to be the text with every one replaced, got ${times(kept, 'harbour')} left and ${times(kept, 'haven')} of ${boats}, and line ${differs + 1} reading "${kept.split('\n')[differs]}" for "${wanted.split('\n')[differs]}"`, kept === wanted && kept === (await readFile(join(where, `blueprints/berths.${ending}`), 'utf8')));
  await emptied();
  await typed('haven', (sofar) => times(kept, sofar));
  await page.keyboard.press('Escape');
  await inText();

  // ---- a file opened as code -----------------------------------------------------
  await page.locator('.tab', { hasText: 'Shop repo' }).first().click();
  await files.getByText('quay.rs').click();
  await page.locator('.cm-line', { hasText: 'fn berth_1()' }).waitFor();
  // Its head says when it was changed once that is read: the lines are read once the head is whole.
  await page.locator('.view-head').getByText(/^Edited/).waitFor({ timeout: 8000 });
  await drawn();
  const code = await looked();
  // A file is read before it is changed: its text holds no keyboard then, and is found in all the same.
  await page.locator('.cm-line', { hasText: 'fn berth_1()' }).click();
  await drawn();
  expect('Ctrl and F in a file opened as code to be taken from the browser', (await taken('Control+f')) === true);
  await bar().waitFor();
  expect(`no line of the file to move as the bar opens`, JSON.stringify(code.first) === JSON.stringify((await looked()).first));
  expect('no replacing to be offered while a file is only read', (await page.locator('.finder-turn').count()) === 0);
  await typed('lighthouse', (sofar) => times(CODE, sofar));
  await page.locator('.cm-searchMatch-selected', { hasText: 'lighthouse' }).waitFor();
  const inCode = await inSight('far down a file', 2);
  expect(`a word far down a file to be found and scrolled to, got ${await said()}`, (await said()) === '1 of 1' && inCode.scrolled > 1000);
  await laidOut('over a file');
  await shot(`find-7-a-file-1280-${scheme}`);
  // Once it is to be changed, the bar that is open offers replacing.
  await page.getByRole('tab', { name: 'Edit' }).last().click();
  await page.locator('.finder-turn').waitFor();
  expect(`the bar to go on saying what it found as the file is opened for changing, got ${await said()}`, (await said()) === '1 of 1');
  await laidOut('over a file that may be changed');
  await words().focus();
  await page.keyboard.press('Escape');
  await inText();

  // ---- narrower: the same bar at 900 and at a phone's width ----------------------
  await page.locator('.tab', { hasText: 'berths' }).first().click();
  await page.locator('.cm-line').first().waitFor();
  for (const width of [900, 390]) {
    await page.setViewportSize({ width, height: 860 });
    await drawn();
    // On a phone the side panel lies over the page until it is put away.
    if (await page.locator('.side-under').count()) await page.locator('.side-under').click({ position: { x: width - 20, y: 400 } });
    await page.locator('.cm-line').first().waitFor();
    await page.locator('.cm-line').first().click({ position: { x: 20, y: 8 } });
    await page.waitForFunction(() => document.activeElement?.classList.contains('cm-content'));
    await page.keyboard.press('Control+h');
    await page.locator('.finder-with').waitFor();
    await says(new RegExp(`^(\\d+ of )?${boats}( matches)?$`));
    const narrow = await laidOut(`at ${width} wide`);
    expect(`every part of the bar to be drawn at ${width} wide, got ${narrow.parts}`, narrow.parts === 12);
    await words().focus();
    await page.keyboard.press('Enter');
    await inSight(`gone to at ${width} wide`);
    await shot(`find-8-bar-${width}-${scheme}`);
    await page.keyboard.press('Escape');
    await bar().waitFor({ state: 'detached' });
  }

  // ---- the other of dark and light -----------------------------------------------
  const other = scheme === 'dark' ? 'light' : 'dark';
  lit = await signIn(other);
  await side(lit, 'Resources');
  await unfold(lit, lit.locator('.nav button.nav-item', { has: lit.getByText('std', { exact: true }) }));
  await lit.locator('.nav').getByText('std/fleet', { exact: true }).click();
  await lit.locator('.view-head').getByText('std/fleet', { exact: false }).first().waitFor();
  await lit.getByRole('tab', { name: 'Text' }).last().click();
  await lit.locator('.cm-line', { hasText: shippedLines[0] }).waitFor();
  await lit.getByRole('tab', { name: 'Text' }).last().focus();
  await lit.keyboard.press('Control+f');
  await bar(lit).waitFor();
  await typed('channel', (sofar) => times(shipped, sofar), lit);
  await lit.keyboard.press('Enter');
  await lit.locator('.cm-searchMatch-selected').waitFor();
  await laidOut(`in the ${other} scheme`, lit);
  await shot(`find-2-bar-1280-${other}`, lit);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/find-0-failed.png` }).catch(() => {});
  await lit?.screenshot({ path: `${out}/find-0-failed-second-page.png` }).catch(() => {});
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
