// A long article read as a paper is: its tables, its wide diagrams, its
// Contents kept in view, Linked from with a series gathered, and serif
// letters chosen for reading.
//
//   DAEMON=../target/debug/factory node scripts/reading.mjs [out-dir]
//
// Ada owns the factory and makes from a terminal a wiki, Meditation, whose
// article has a note, an info box, six headings, a table (a link to a page
// in one of its cells, a footnote's marker in another, numbers set at the
// right), a wide flowchart, a state diagram laid out sideways, a gantt of
// eight weeks, a pie and a mind map of seven branches, a series at its end
// and its sources. Three pages under it carry the same series box, and a
// page beside it links to it from its words.
//
// At 1440 wide: the table is drawn with its headings, its rules and its
// right-aligned figures, and its link opens the page; every diagram is
// inside its column and its words read (at least 10.5px as drawn), the
// gantt scrolling in its box with its right edge marked, then its left once
// scrolled, and opened whole over the page by Open (as large as it was
// drawn, Escape putting it away) and by pressing it; the pie's slices are
// told from each other and from the page, and the mind map has no empty
// band and nothing cut off; the Contents stay in view after 3000px of
// scrolling, the section read marked; Linked from lists the page that links
// from its words and gathers the three of the series into one folded line,
// which unfolds to them. Edit then Save, after a letter typed and taken
// back, keeps the post byte for byte. Aa's Typeface: Serif draws the
// article's words and headings in serif, kept after a reload, and Sans puts
// it back. At a phone's width the table and the diagrams scroll in their
// own boxes and the page never sideways. Nothing of the page lies over
// anything else, and nothing is fetched from anywhere but the daemon. The
// environment variables are those of walkthrough.mjs (`SCHEME`, `LOOK`).
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
const data = join(out, 'reading-data');
const home = join(out, 'reading-home');
const scheme = process.env.SCHEME ?? 'dark';
const shot = (name) => `${out}/reading-${name}${process.env.LOOK ? '-looked' : ''}${scheme === 'light' ? '-light' : ''}.png`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env, maxBuffer: 1 << 24 });
const made = async (...args) => JSON.parse((await cli('topic', 'make', ...args, '--json')).stdout).id;

/** A paragraph long enough that a section is much of a window. */
const words = (what, lines = 8) => Array.from({ length: lines }, (_, nth) => `${what}, sentence ${nth + 1}: what the studies say of it is written here at some length.`).join(' ');
const fence = (lang, ...lines) => ['```' + lang, ...lines, '```'].join('\n');

const FLOW = fence(
  'mermaid',
  'flowchart LR',
  '  A[Notice the breath at the nostrils] --> B[Count each out-breath to ten] --> C[Notice the mind has wandered]',
  '  C --> D[Name where it went, gently] --> E[Return to the count at one] --> F[Widen to the whole body breathing]',
  '  F --> G[Rest in open awareness] --> H[Close with three slow breaths]',
);
const STATES = fence('mermaid', 'stateDiagram-v2', '  direction LR', '  [*] --> Settling', '  Settling --> Attending', '  Attending --> Wandering', '  Wandering --> Noticing', '  Noticing --> Returning', '  Returning --> Attending', '  Attending --> Resting', '  Resting --> [*]');
const WEEKS = ['Breath', 'Body scan', 'Sitting', 'Walking', 'Sounds', 'Thoughts', 'Kindness', 'Review'];
const GANTT = fence('mermaid', 'gantt', '  title Eight weeks of practice', '  dateFormat YYYY-MM-DD', '  section Course', ...WEEKS.map((week, nth) => `  ${week} : w${nth + 1}, ${nth === 0 ? '2026-01-05' : `after w${nth}`}, 7d`));
const PIE = fence('mermaid', 'pie title Time by practice', '  "Breath" : 32', '  "Body scan" : 21', '  "Walking" : 14', '  "Sounds" : 12', '  "Kindness" : 11', '  "Open awareness" : 10');
const MIND = fence('mermaid', 'mindmap', '  root((Meditation))', '    Breath', '      Counting', '    Body', '      Scanning', '    Movement', '      Walking', '    Senses', '      Sounds', '    Mind', '      Thoughts', '    Heart', '      Kindness', '    Rest', '      Open awareness');

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  const meditation = await made('Meditation');
  const studies = await made('Studies', '--body', 'A list of the studies.');
  const under = [];
  for (const title of ['Breath', 'Body scan', 'Walking']) under.push(await made(title, '--under', meditation));
  const SERIES = ['```series', 'title: Meditation', `[Meditation](topic:${meditation})`, '## Practices', ...['Breath', 'Body scan', 'Walking'].map((title, nth) => `[${title}](topic:${under[nth]})`), '```'].join('\n');
  for (const [nth, title] of ['Breath', 'Body scan', 'Walking'].entries()) await cli('record', 'set', under[nth], `body=${title}, one of the practices.\n\n${SERIES}\n`);
  await cli('record', 'set', studies, `body=The studies of [meditation](topic:${meditation}), one by one.`);
  const TABLE = [
    '| Practice | Minutes a day | Studies | Effect |',
    '|:---------|:-------------:|--------:|-------:|',
    `| [Breath](topic:${studies}) | 10 | 12[^1] | 0.41 |`,
    '| **Body scan** | 20–45 | 7 | 0.33 |',
    '| Walking \\| outdoors | 15 | 4 | 0.18 |',
  ].join('\n');
  const MEDITATION = [
    fence('note', `This page is about meditation as a practice. For the studies alone, see [Studies](topic:${studies}).`),
    '',
    fence('infobox', 'title: Meditation', 'Kind: A practice of attention', 'Studied since: 1970s', `Studies: [Studies](topic:${studies})`),
    '',
    'Meditation is a family of practices that train attention.[^1]',
    '',
    '# Background',
    '',
    words('Background'),
    '',
    '## History',
    '',
    words('History', 12),
    '',
    '# Methods',
    '',
    words('Methods'),
    '',
    TABLE,
    '',
    words('After the table'),
    '',
    '## Diagrams',
    '',
    FLOW,
    '',
    STATES,
    '',
    GANTT,
    '',
    '# Results',
    '',
    PIE,
    '',
    words('Results', 14),
    '',
    MIND,
    '',
    '# Discussion',
    '',
    words('Discussion', 30),
    '',
    fence('series', 'title: Meditation', 'place: end', ...['Breath', 'Body scan', 'Walking'].map((title, nth) => `[${title}](topic:${under[nth]})`)),
    '',
    '[^1]: Ada Lovelace, *Notes*, Taylor, 1843.',
    '',
  ].join('\n');
  await cli('record', 'set', meditation, `body=${MEDITATION}`);
  const bodyNow = async () => JSON.parse((await cli('topic', 'show', meditation, '--json')).stdout).topic?.data?.body ?? '';
  const set = await bodyNow();
  if (set !== MEDITATION) throw new Error(`expected the post kept as it was set, got ${JSON.stringify(set.slice(0, 200))}`);

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
  const DESK = { viewport: { width: 1440, height: 900 } };
  const PHONE = { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 };
  const signedIn = async (to, how = DESK) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    page.on('request', (asked) => !asked.url().startsWith(url) && !/^(data|blob|about):/.test(asked.url()) && elsewhere.push(asked.url()));
    await page.goto(to);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  const shown = async (locator, what) => {
    await locator.waitFor({ timeout: 15000 }).catch(() => {
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
    const sideways = await on.evaluate(() => [document.documentElement, document.querySelector('.topic-page'), document.querySelector('.topic-page')?.closest('.view')].map((one) => (one ? one.scrollWidth - one.clientWidth : 0)));
    expect(`nothing but its own boxes to scroll sideways ${what}, got ${sideways.join(', ')}px`, sideways.every((one) => one <= 0));
  };
  const at = (title, after = '') => `${url}/topic/${encodeURIComponent(title)}${after}`;
  const article = (on) => on.locator(`.topic-page[data-topic="${meditation}"]`);
  /** Waits until every diagram of the article is drawn and fitted to its column. */
  const drawn = async (on, what) => {
    await on
      .waitForFunction(() => {
        const all = [...document.querySelectorAll('.topic-post .diagram')];
        return all.length === 5 && all.every((one) => one.dataset.drawn !== 'not-yet' && one.querySelector('svg[data-fitted]'));
      }, null, { timeout: 30000 })
      .catch(async () => {
        const said = await on.evaluate(() => [...document.querySelectorAll('.topic-post .diagram, .topic-post .diagram-failed')].map((one) => `${one.dataset.drawn ?? 'failed'} ${one.querySelector('svg')?.dataset.fitted === undefined ? 'unfitted' : 'fitted'}`));
        throw new Error(`expected the five diagrams drawn ${what}, got ${JSON.stringify(said)}`);
      });
  };
  /** Each diagram: its kind, how wide its box shows and its drawing is, whether it scrolls, and the size its words are drawn at. */
  const diagrams = (on) =>
    on.evaluate(() =>
      [...document.querySelectorAll('.topic-post .diagram')].map((one) => {
        const box = one.querySelector('.diagram-box');
        const svg = box.querySelector('svg');
        const scale = svg.getBoundingClientRect().width / svg.viewBox.baseVal.width;
        const sizes = [...svg.querySelectorAll('text, foreignObject div, foreignObject span')].map((each) => parseFloat(getComputedStyle(each).fontSize) * scale).filter((size) => size > 0).sort((a, b) => a - b);
        const column = one.closest('.topic-post').getBoundingClientRect();
        const frame = one.querySelector('.diagram-frame');
        return {
          kind: svg.getAttribute('aria-roledescription') ?? '',
          shows: box.clientWidth,
          wide: box.scrollWidth,
          inside: box.getBoundingClientRect().left >= column.left - 1 && box.getBoundingClientRect().right <= column.right + 1,
          scrolls: frame.hasAttribute('data-scrolls'),
          right: frame.hasAttribute('data-more-right'),
          left: frame.hasAttribute('data-more-left'),
          text: sizes[Math.floor((sizes.length - 1) / 2)] ?? 0,
        };
      }),
    );

  // ── The article at 1440 ─────────────────────────────────────────────────
  const page = await signedIn(at('Meditation'));
  const read = article(page);
  const table = read.locator('.topic-post table.prose-table');
  await shown(table, 'the table drawn');
  expect('its four headings', (await table.locator('thead th').allInnerTexts()).join('|') === 'Practice|Minutes a day|Studies|Effect');
  expect('its three rows', (await table.locator('tbody tr').count()) === 3);
  expect('a pipe of a cell drawn as its words', (await table.locator('tbody tr').nth(2).locator('td').first().innerText()) === 'Walking | outdoors');
  expect('the bold in a cell', (await table.locator('tbody tr').nth(1).locator('td strong').innerText()) === 'Body scan');
  expect('its footnote marker a raised number', (await table.locator('sup.cite, .cite').count()) >= 1);
  const aligned = await table.locator('tbody tr').first().locator('td').evaluateAll((cells) => cells.map((cell) => getComputedStyle(cell).textAlign));
  expect(`left, centre, right and right, got ${aligned}`, aligned.join() === 'left,center,right,right');
  const rules = await table.evaluate((one) => [getComputedStyle(one).borderTopWidth, getComputedStyle(one.querySelector('th')).borderBottomWidth, getComputedStyle(one.querySelector('td')).borderLeftWidth]);
  expect(`a rule over it and under its headings and none between columns, got ${rules}`, parseFloat(rules[0]) >= 1 && parseFloat(rules[1]) >= 1 && parseFloat(rules[2]) === 0);
  await drawn(page, 'at 1440');
  await laid(page, 'at the top of the article');
  await page.screenshot({ path: shot('01-top') });
  await table.scrollIntoViewIfNeeded();
  await page.screenshot({ path: shot('02-table') });

  // The diagrams: inside the column, their words read; the gantt, too wide to read fitted, scrolls with its edge marked.
  const seen = await diagrams(page);
  for (const one of seen) expect(`the ${one.kind} inside its column with its words read, got ${JSON.stringify(one)}`, one.inside && one.text >= 10.5);
  const gantt = seen.find((one) => one.kind === 'gantt');
  expect(`the gantt scrolling in its box with its right edge marked, got ${JSON.stringify(gantt)}`, gantt && gantt.scrolls && gantt.wide > gantt.shows && gantt.right && !gantt.left);
  const ganttBox = read.locator('.diagram').filter({ has: page.locator('svg[aria-roledescription="gantt"]') });
  await ganttBox.scrollIntoViewIfNeeded();
  await page.screenshot({ path: shot('03-gantt') });
  await ganttBox.locator('.diagram-box').evaluate((box) => box.scrollTo({ left: box.scrollWidth }));
  await page.waitForFunction(() => {
    const frame = [...document.querySelectorAll('.topic-post .diagram-frame')].find((one) => one.querySelector('svg[aria-roledescription="gantt"]'));
    return frame?.hasAttribute('data-more-left') && !frame.hasAttribute('data-more-right');
  }, null, { timeout: 5000 }).catch(() => {
    throw new Error('expected the gantt scrolled to its end to mark its left edge, and its right no more');
  });
  await page.screenshot({ path: shot('04-gantt-scrolled') });
  // Opened whole by Open, as large as it was drawn; Escape puts it away. Pressing it opens it too.
  await ganttBox.getByRole('button', { name: /^Open .* whole$/ }).click();
  const whole = page.locator('.modal.diagram-whole');
  await shown(whole.locator('svg'), 'the gantt opened whole');
  const wholeSize = await whole.locator('svg').evaluate((svg) => [svg.getBoundingClientRect().width, svg.viewBox.baseVal.width]);
  expect(`the gantt whole as large as it was drawn, got ${wholeSize}`, Math.abs(wholeSize[0] - wholeSize[1]) <= 2);
  await page.screenshot({ path: shot('05-gantt-whole') });
  await page.keyboard.press('Escape');
  await gone(whole, 'the gantt put away by Escape');
  await ganttBox.locator('.diagram-box').click();
  await shown(whole.locator('svg'), 'the gantt opened whole by pressing it');
  await whole.getByRole('button', { name: 'Close' }).click();
  await gone(whole, 'the gantt put away by Close');

  // The pie's slices told apart and from the page; the mind map with no empty band and nothing cut off.
  const pie = await read.locator('svg[aria-roledescription="pie"]').evaluate((svg) => {
    const rgb = (colour) => {
      const probe = document.createElement('span');
      probe.style.color = colour;
      document.body.append(probe);
      const said = getComputedStyle(probe).color.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number);
      probe.remove();
      return said;
    };
    const lum = ([r, g, b]) => [r, g, b].map((c) => (c / 255 <= 0.03928 ? c / 255 / 12.92 : ((c / 255 + 0.055) / 1.055) ** 2.4)).reduce((sum, c, at) => sum + c * [0.2126, 0.7152, 0.0722][at], 0);
    const contrast = (a, b) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);
    const page = rgb(getComputedStyle(document.querySelector('.view')).backgroundColor);
    const slices = [...svg.querySelectorAll('path.pieCircle')].map((one) => rgb(getComputedStyle(one).fill));
    return { count: slices.length, distinct: new Set(slices.map(String)).size, least: Math.min(...slices.map((one) => contrast(one, page))), opacity: Math.min(...[...svg.querySelectorAll('path.pieCircle')].map((one) => Number(getComputedStyle(one).opacity))) };
  });
  expect(`six slices, each its own colour, told from the page and drawn whole, got ${JSON.stringify(pie)}`, pie.count === 6 && pie.distinct === 6 && pie.least >= 1.5 && pie.opacity === 1);
  const mind = await read.locator('svg[aria-roledescription="mindmap"]').evaluate((svg) => {
    const box = svg.viewBox.baseVal;
    const drew = svg.getBBox();
    return { top: drew.y - box.y, left: drew.x - box.x, right: box.x + box.width - (drew.x + drew.width), bottom: box.y + box.height - (drew.y + drew.height) };
  });
  expect(`the mind map's box fitted to what it draws, got ${JSON.stringify(mind)}`, Object.values(mind).every((edge) => edge >= -1 && edge <= 20));
  await read.locator('svg[aria-roledescription="mindmap"]').scrollIntoViewIfNeeded();
  await page.screenshot({ path: shot('06-mindmap') });
  await read.locator('svg[aria-roledescription="pie"]').scrollIntoViewIfNeeded();
  await page.screenshot({ path: shot('07-pie') });

  // ── Contents in view after 3000px ───────────────────────────────────────
  await page.evaluate(() => document.querySelector('.topic-page').closest('.view').scrollTo({ top: 0 }));
  const contents = read.locator('nav.contents');
  await shown(contents.locator('.contents-entry').first(), 'Contents beside the article');
  await page.evaluate(() => document.querySelector('.topic-page').closest('.view').scrollBy({ top: 3000 }));
  await page.waitForFunction(() => document.querySelector('.topic-page').closest('.view').scrollTop >= 2999, null, { timeout: 5000 });
  await shown(contents.locator('.contents-entry[aria-current="location"]'), 'the section being read marked after scrolling');
  const kept = await page.evaluate(() => {
    const nav = document.querySelector('nav.contents').getBoundingClientRect();
    const view = document.querySelector('.topic-page').closest('.view').getBoundingClientRect();
    return { top: nav.top - view.top, bottom: nav.bottom, height: view.bottom };
  });
  expect(`the Contents in view after 3000px, got ${JSON.stringify(kept)}`, kept.top >= -1 && kept.top <= 40 && kept.bottom <= kept.height + 1);
  await laid(page, 'scrolled 3000px');
  await page.screenshot({ path: shot('08-contents-kept') });

  // ── Linked from ─────────────────────────────────────────────────────────
  const linked = read.locator('.topic-linked');
  await linked.scrollIntoViewIfNeeded();
  await shown(linked.locator('.topic-pages-line', { hasText: 'Studies' }), 'Studies listed, linking from its words');
  const fold = linked.locator('.topic-linked-fold');
  await shown(fold, 'the series gathered into one line');
  expect(`the line saying how many and which series, got ${await fold.innerText()}`, /3 pages of the series Meditation/.test(await fold.innerText()));
  expect('the series folded, its pages not listed', (await fold.getAttribute('aria-expanded')) === 'false' && (await linked.locator('.topic-pages-line').count()) === 1);
  await fold.click();
  await shown(linked.locator('.topic-linked-series .topic-pages-line', { hasText: 'Walking' }), 'the pages of the series once unfolded');
  expect('the three of the series', (await linked.locator('.topic-linked-series .topic-pages-line').count()) === 3);
  await laid(page, 'with Linked from unfolded');
  await page.screenshot({ path: shot('09-linked-from') });

  // ── The table's link, then Edit and Save keeping the post byte for byte ─
  await page.goto(at('Meditation'));
  await shown(table, 'the table again');
  await table.locator('a', { hasText: 'Breath' }).click();
  await shown(page.locator(`.topic-page[data-topic="${studies}"]`), 'the page the table links to opened');
  await page.goto(at('Meditation'));
  await shown(table, 'the table again');
  await read.getByRole('button', { name: 'Edit', exact: true }).first().click();
  const editor = read.locator('.topic-post .ProseMirror');
  await shown(editor.locator('pre.writer-table'), 'the table in the editor as its text');
  expect('the table kept whole in the editor', (await editor.locator('pre.writer-table').innerText()) === TABLE);
  await page.screenshot({ path: shot('10-editing') });
  await editor.locator('p').last().click();
  await page.keyboard.press('End');
  await page.keyboard.type('x');
  await page.keyboard.press('Backspace');
  await read.locator('.topic-doc-acts').getByRole('button', { name: 'Save', exact: true }).click();
  await gone(editor, 'the editor closed once saved');
  const saved = await bodyNow();
  const differs = [...MEDITATION].findIndex((letter, at) => saved[at] !== letter);
  expect(`the post saved byte for byte, got ${saved.length} characters for ${MEDITATION.length}, first apart at ${JSON.stringify(saved.slice(differs - 40, differs + 40))}`, saved === MEDITATION);

  // ── Typeface ────────────────────────────────────────────────────────────
  const face = () => read.locator('.topic-post .prose-text').first().evaluate((one) => getComputedStyle(one).fontFamily);
  const headingFace = () => read.locator('.topic-post .prose-heading').first().evaluate((one) => getComputedStyle(one).fontFamily);
  expect('sans letters before anything is chosen', !/Iowan|Georgia|serif$/.test(await face()));
  await page.getByRole('button', { name: 'Appearance' }).click();
  await page.locator('[data-appearance="typeface=serif"]').click();
  await page.waitForFunction(() => /Iowan Old Style/.test(getComputedStyle(document.querySelector('.topic-post .prose-text')).fontFamily), null, { timeout: 5000 }).catch(() => {
    throw new Error('expected the article in serif once Serif is chosen');
  });
  expect('its headings in the same face', /Iowan Old Style/.test(await headingFace()));
  await page.keyboard.press('Escape');
  await page.screenshot({ path: shot('11-serif') });
  await page.reload();
  await shown(table, 'the article read again');
  expect('serif kept after a reload', /Iowan Old Style/.test(await face()));
  expect('the page round the article in its own letters', !/Iowan/.test(await page.locator('.tabs, .side, body').first().evaluate((one) => getComputedStyle(one).fontFamily)));
  await laid(page, 'in serif');

  // ── A phone ─────────────────────────────────────────────────────────────
  const phone = await signedIn(at('Meditation'), PHONE);
  const small = article(phone);
  await shown(small.locator('.topic-post table.prose-table'), 'the table on a phone');
  expect('serif there too, kept with the account', /Iowan Old Style/.test(await small.locator('.topic-post .prose-text').first().evaluate((one) => getComputedStyle(one).fontFamily)));
  await drawn(phone, 'on a phone');
  const narrow = await diagrams(phone);
  for (const one of narrow) expect(`the ${one.kind} in its own box on a phone, its words read, got ${JSON.stringify(one)}`, one.inside && one.text >= 10.5 && (!one.scrolls || one.right));
  const tableBox = await small.locator('.prose-table-box').evaluate((box) => ({ wide: box.scrollWidth, shows: box.clientWidth, right: box.parentElement.hasAttribute('data-more-right') }));
  expect(`the table scrolling in its own box on a phone, marked, got ${JSON.stringify(tableBox)}`, tableBox.wide <= tableBox.shows || tableBox.right);
  await small.locator('.topic-post table.prose-table').scrollIntoViewIfNeeded();
  await laid(phone, "with the table at a phone's width");
  await phone.screenshot({ path: shot('12-phone-table') });
  await small.locator('svg[aria-roledescription="gantt"]').scrollIntoViewIfNeeded();
  await laid(phone, "with the gantt at a phone's width");
  await phone.screenshot({ path: shot('13-phone-gantt') });
  await small.locator('svg[aria-roledescription="mindmap"]').scrollIntoViewIfNeeded();
  await phone.screenshot({ path: shot('14-phone-mindmap') });

  // Sans again, for whoever reads next.
  await page.getByRole('button', { name: 'Appearance' }).click();
  await page.locator('[data-appearance="typeface=sans"]').click();
  await page.waitForFunction(() => !/Iowan/.test(getComputedStyle(document.querySelector('.topic-post .prose-text')).fontFamily), null, { timeout: 5000 }).catch(() => {
    throw new Error('expected sans again once Sans is chosen');
  });

  await browser.close();
  if (elsewhere.length) throw new Error(`fetched from elsewhere: ${elsewhere.join(', ')}`);
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
