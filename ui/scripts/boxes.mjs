// An article's boxes: an info box with a picture, a series and a note at the top.
//
//   DAEMON=../target/debug/factory node scripts/boxes.mjs [out-dir]
//
// "Frogs", a page of the wiki "Amphibians" with no status, says in its post
// a note for its top, an info box of key facts with a picture attached to
// it, and a series listing it, "Toads" and "Newts"; "Toads" says a series
// placed at its end. Read by somebody who may only read, on a wide window,
// the note is in italics over the text and the boxes, the boxes stand at the text's right
// with nothing of either over the other, the info box is a titled table
// with its picture drawn, the series marks "Frogs" as the page it is on,
// and the post's text draws none of them again; on a phone they stand
// above the text, under the note, within the window. Pressed, the series' "Toads" opens
// its page, whose Linked from says "Frogs" links to it, and whose series
// stands at the end of its article. A topic whose info box has no title
// shows it as its code with a line saying what is wrong. Whoever may write
// the page is shown the boxes from what is saved and their code in the
// editor once Edit is pressed, and `/info` there puts in an info box to be
// filled in. "Tadpoles", with four headings, a note, an info box and a
// series, has its Contents at the text's left and its boxes at its right
// at 1440 wide, none over another; at 1100 its Contents folded above the
// text and the boxes beside it; at 900 and on a phone the note, the boxes,
// the folded Contents and the text, each under the last. Its Contents list
// the four headings of its text and nothing written in a box. Nothing
// is fetched from anywhere but the daemon, and nothing on the page lies
// over anything else, plainly and with `LOOK="size=largest
// density=compact"`, in the dark and (`SCHEME=light`) the light. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
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
const data = join(out, 'boxes-data');
const home = join(out, 'boxes-home');
const looked = process.env.LOOK ? '-looked' : '';
const scheme = process.env.SCHEME ?? 'dark';
const shot = (name) => `${out}/boxes-${name}${looked}${scheme === 'light' ? '-light' : ''}.png`;

// A frog, drawn as SVG: a picture a browser draws without fetching anything.
const FROG_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="320" height="220" viewBox="0 0 320 220">
<rect width="320" height="220" fill="#e8f3e0"/><ellipse cx="160" cy="140" rx="110" ry="60" fill="#4c9a2a"/>
<circle cx="110" cy="80" r="30" fill="#4c9a2a"/><circle cx="210" cy="80" r="30" fill="#4c9a2a"/>
<circle cx="110" cy="78" r="12" fill="#fff"/><circle cx="210" cy="78" r="12" fill="#fff"/>
<circle cx="110" cy="80" r="6" fill="#111"/><circle cx="210" cy="80" r="6" fill="#111"/></svg>`;

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
  const frogs = await made('Frogs', '--under', amphibians);
  const toads = await made('Toads', '--under', amphibians);
  const newts = await made('Newts', '--under', amphibians, '--body', 'Newts keep their tails.');
  const culture = await made('Frogs in culture', '--body', 'Frogs in stories and songs.');
  const picture = join(home, 'frog.svg');
  await writeFile(picture, FROG_SVG);
  const attached = /ratt_\w+/.exec((await cli('record', 'attach', frogs, picture)).stdout)?.[0];
  if (!attached) throw new Error('expected the picture attached, with its id said');
  const SERIES = ['```series', 'title: Amphibians', '## Frogs and toads', `[Frogs](topic:${frogs})`, `[Toads](topic:${toads})`, '## Tailed', `[Newts](topic:${newts})`, '```'].join('\n');
  const FROGS = [
    '```note',
    `This page is about frogs as animals. For frogs in stories, see [Frogs in culture](topic:${culture}).`,
    '```',
    '',
    'A frog is an amphibian with no tail. Frogs live near water, lay their eggs in it, and begin as tadpoles that breathe through gills.',
    '',
    '```infobox',
    'title: Frog',
    `picture: attachment:${attached}`,
    'caption: A common frog',
    'Order: Anura',
    'Species: 7,600+',
    `Range: every continent but Antarctica; see also [Toads](topic:${toads})`,
    'Notes: https://example.org/frogs',
    '```',
    '',
    SERIES,
    '',
    'Most frogs have long hind legs for leaping, and catch insects with a sticky tongue. Their skin is thin, and many breathe through it as much as through their lungs.',
    '',
    'Many frogs call at night in spring, each kind with a call of its own, which is how a female finds a male of her kind.',
  ].join('\n');
  await cli('record', 'set', frogs, `body=${FROGS}`);
  await cli('record', 'set', toads, `body=Toads are frogs with dry skin.\n\n${SERIES.replace('title: Amphibians', 'title: Amphibians\nplace: end')}\n`);
  const broken = await made('Broken box', '--body', 'Above.\n\n```infobox\nOrder: Anura\n```\n\nBelow.');
  // A long article with Contents, a note, an info box and a series: its sections each a few lines long.
  const said = (topic) => `${topic} ${'Tadpoles change as they grow, and what they eat changes with them. '.repeat(4)}`;
  const tadpoles = await made('Tadpoles', '--under', amphibians);
  const TADPOLES = [
    '```note',
    `This page is about young frogs. For grown ones, see [Frogs](topic:${frogs}).`,
    '```',
    '',
    'A tadpole is a frog before it is grown: it lives in water and breathes through gills.',
    '',
    '```infobox',
    'title: Tadpole',
    `picture: attachment:${attached}`,
    'caption: What it grows into',
    'Lives in: ponds and slow streams',
    'Eats: algae, then insects',
    '```',
    '',
    SERIES.replace(`[Toads](topic:${toads})`, `[Toads](topic:${toads})\n[Tadpoles](topic:${tadpoles})`),
    '',
    '## Eggs',
    '',
    said('Eggs.'),
    '',
    '## Hatching',
    '',
    said('Hatching.'),
    '',
    '## Growing legs: section',
    '',
    said('Legs.'),
    '',
    '### Becoming a frog',
    '',
    said('Grown.'),
  ].join('\n');
  await cli('record', 'set', tadpoles, `body=${TADPOLES}`);

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
  /** Where the parts of an article stand on the page, by their boxes. */
  const boxesOf = (on, id) =>
    on.evaluate((id) => {
      const article = document.querySelector(`.topic-page[data-topic="${id}"] .topic-article`);
      const box = (selector) => {
        const one = article?.querySelector(selector)?.getBoundingClientRect();
        return one ? { left: one.left, right: one.right, top: one.top, bottom: one.bottom } : null;
      };
      return {
        width: document.documentElement.clientWidth,
        boxes: box('.topic-article-boxes'),
        post: box('.topic-post'),
        text: box('.topic-post .topic-doc-read, .topic-post .topic-doc'),
        note: box('.topic-head-note'),
      };
    }, id);
  const pictureDrawn = (on) => on.locator('.info-box img.picture-drawn').evaluate((img) => img.complete && img.naturalWidth > 0);
  const READS = ['projects:read', 'channels', 'records:read', 'board:read', 'search'];

  // ── Read on a wide window: the note on top, the boxes at the side ──────
  const reader = await signedIn('Frogs', undefined, READS);
  const article = reader.locator(`.topic-page[data-topic="${frogs}"] .topic-article`);
  const info = article.locator('.topic-article-boxes .info-box');
  await info.waitFor();
  await reader.waitForFunction(() => document.querySelector('.info-box img.picture-drawn')?.complete);
  expect('the picture of the info box drawn', await pictureDrawn(reader));
  expect('the info box titled', (await info.locator('.box-title').innerText()).trim() === 'Frog');
  const rows = await info.locator('.info-box-rows tr').evaluateAll((all) => all.map((row) => [row.querySelector('th').innerText.trim(), row.querySelector('td').innerText.trim()]));
  expect(`its facts in rows, got ${JSON.stringify(rows)}`, rows.length === 4 && rows[0][0] === 'Order' && rows[0][1] === 'Anura' && rows[1][1] === '7,600+');
  expect('the caption under its picture', (await info.locator('figcaption').innerText()).trim() === 'A common frog');
  expect('a page in a fact a link to it', (await info.locator(`a.page-link[data-topic="${toads}"]`).count()) === 1);
  expect('an address in a fact a link, never fetched', (await info.locator('a[href="https://example.org/frogs"]').count()) === 1);
  const series = article.locator('.topic-article-boxes .series-box');
  expect('the series titled, with its groups', (await series.locator('.box-title').innerText()).trim() === 'Amphibians' && (await series.locator('.series-group-name').allInnerTexts()).join('|') === 'Frogs and toads|Tailed');
  const here = series.locator('li[aria-current="page"]');
  expect('the page it is on marked in the series', (await here.count()) === 1 && (await here.innerText()).trim() === 'Frogs');
  const note = article.locator('.topic-head-note .box-note');
  expect('the note in italics', (await note.evaluate((one) => getComputedStyle(one).fontStyle)) === 'italic');
  expect('the note linking the page it points to', (await note.locator(`a.page-link[data-topic="${culture}"]`).count()) === 1);
  expect("the post's text drawing none of them again", (await article.locator('.topic-post .info-box, .topic-post .series-box, .topic-post .box-note, .topic-post .code-block').count()) === 0);
  expect("the post's text there", (await article.locator('.topic-post').innerText()).includes('sticky tongue'));
  const wide = await boxesOf(reader, frogs);
  expect(`the boxes at the right of the text, beside it, got ${JSON.stringify(wide)}`, wide.boxes && wide.boxes.left >= wide.post.right - 1 && wide.boxes.top < wide.post.bottom);
  expect(`the note over the text and the boxes, got ${JSON.stringify(wide)}`, wide.note && wide.note.bottom <= wide.post.top + 1 && wide.note.bottom <= wide.boxes.top + 1);
  expect(`the boxes within the window, got ${JSON.stringify(wide)}`, wide.boxes.right <= wide.width);
  // The note is further in than the text.
  const inBy = await note.evaluate((one) => one.querySelector('p').getBoundingClientRect().left - one.closest('.topic-article').querySelector('.topic-post .prose-text').getBoundingClientRect().left);
  expect(`the note further in than the text, got ${inBy}`, inBy > 4);
  await reader.screenshot({ path: shot('1-wide') });
  await crowded(reader, 'on a wide window');

  // ── A series' page, pressed: it opens, linked from this one ─────────────
  await series.locator(`a.page-link[data-topic="${toads}"]`).click();
  const toadPage = reader.locator(`.topic-page[data-topic="${toads}"]`);
  await toadPage.waitFor();
  await toadPage.locator('.topic-linked').getByRole('link', { name: /Frogs/ }).waitFor();
  const ended = await toadPage.evaluate((one) => {
    const post = one.querySelector('.topic-post').getBoundingClientRect();
    const end = one.querySelector('.topic-boxes-end .series-box')?.getBoundingClientRect();
    return { at: end ? end.top >= post.bottom - 1 : false, side: one.querySelectorAll('.topic-article-boxes').length };
  });
  expect(`its series at the end of its article and none at its side, got ${JSON.stringify(ended)}`, ended.at && ended.side === 0);
  expect('Toads marked in its series', (await toadPage.locator('.topic-boxes-end li[aria-current="page"]').innerText()).trim() === 'Toads');
  await reader.screenshot({ path: shot('2-followed') });
  await crowded(reader, 'on the page a series led to');

  // ── A box that cannot be read: its code, and what is wrong ──────────────
  await reader.goto(`${url}/topic/${encodeURIComponent(broken)}`);
  const wrong = reader.locator(`.topic-page[data-topic="${broken}"] .topic-post .box-wrong`);
  await wrong.waitFor();
  expect('the broken box shown as its code', (await wrong.locator('.code-block-text').innerText()).includes('Order: Anura'));
  expect('with a line saying what is wrong', (await wrong.locator('.code-undrawn-said').innerText()).includes('needs a line title'));
  expect('nothing at the side for it', (await reader.locator('.topic-article-boxes').count()) === 0);
  await wrong.screenshot({ path: shot('3-broken') });

  // ── On a phone: the boxes above the text ─────────────────────────────────
  const phone = await signedIn('Frogs', { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true }, READS);
  await phone.locator('.topic-article-boxes .info-box').waitFor();
  await phone.waitForFunction(() => document.querySelector('.info-box img.picture-drawn')?.complete);
  const narrow = await boxesOf(phone, frogs);
  expect(`the note, then the boxes, above the text on a phone, got ${JSON.stringify(narrow)}`, narrow.boxes.bottom <= narrow.post.top + 1 && narrow.note.bottom <= narrow.boxes.top + 1);
  expect(`the boxes within the window on a phone, got ${JSON.stringify(narrow)}`, narrow.boxes.left >= 0 && narrow.boxes.right <= narrow.width);
  await phone.screenshot({ path: shot('4-phone'), fullPage: true });
  await crowded(phone, 'on a phone');

  // ── With Contents too: Contents at the left, the text, the boxes at the right, each folding in its turn ──
  // A heading written in a box (a series' group) is none of the article's: its Contents has the four of its text.
  const contentsAt = async (width, height = 900) => {
    const how = width < 500 ? { viewport: { width, height: 844 }, isMobile: true, hasTouch: true } : { viewport: { width, height } };
    const on = await signedIn('Tadpoles', how, READS);
    await on.locator(`.topic-page[data-topic="${tadpoles}"] .topic-article-boxes .info-box`).waitFor();
    await on.locator(`.topic-page[data-topic="${tadpoles}"] nav.contents`).waitFor();
    await on.waitForFunction(() => document.querySelector('.info-box img.picture-drawn')?.complete);
    const at = await on.evaluate((id) => {
      const article = document.querySelector(`.topic-page[data-topic="${id}"] .topic-article`);
      const box = (selector) => {
        const one = article.querySelector(selector)?.getBoundingClientRect();
        return one ? { left: Math.round(one.left), right: Math.round(one.right), top: Math.round(one.top), bottom: Math.round(one.bottom) } : null;
      };
      return {
        width: document.documentElement.clientWidth,
        article: article.getBoundingClientRect().width,
        note: box('.topic-head-note'),
        contents: box('.topic-article-contents'),
        text: box('.topic-post'),
        boxes: box('.topic-article-boxes'),
        folded: getComputedStyle(article.querySelector('nav.contents .contents-fold')).display !== 'none',
        entries: article.querySelectorAll('nav.contents .contents-entry').length,
        boxHeadings: article.querySelectorAll('.topic-article-boxes [data-section], .topic-head-note [data-section]').length,
        sections: [...article.querySelectorAll('.topic-post [data-section]')].map((one) => one.dataset.section),
      };
    }, tadpoles);
    const said = JSON.stringify(at);
    expect(`the Contents of the four headings of the text and none of a box at ${width}, got ${said}`, at.entries === 4 && at.boxHeadings === 0 && at.sections.join(' ') === 'eggs hatching growing-legs-section becoming-a-frog');
    expect(`everything within the window at ${width}, got ${said}`, [at.note, at.contents, at.text, at.boxes].every((one) => one.left >= 0 && one.right <= at.width));
    expect(`the note over everything at ${width}, got ${said}`, [at.contents, at.text, at.boxes].every((one) => at.note.bottom <= one.top + 1));
    await on.screenshot({ path: shot(`6-contents-${width}`), fullPage: true });
    await crowded(on, `with Contents at ${width}`);
    await on.context().close();
    return at;
  };
  const beside = (a, b) => a.right <= b.left + 1 && a.top < b.bottom && b.top < a.bottom;
  const above = (a, b) => a.bottom <= b.top + 1;
  // Wide: the three side by side, none over another.
  const widest = await contentsAt(1440);
  expect(`at 1440, Contents left, the text, the boxes right, none over another, got ${JSON.stringify(widest)}`, !widest.folded && beside(widest.contents, widest.text) && beside(widest.text, widest.boxes));
  // Narrower: the Contents fold above first, the boxes still beside the text.
  const middle = await contentsAt(1100);
  expect(`at 1100, the Contents folded above, the boxes at the text's right, got ${JSON.stringify(middle)}`, middle.folded && above(middle.contents, middle.text) && above(middle.contents, middle.boxes) && beside(middle.text, middle.boxes));
  // Narrower still: the boxes above the text too, under the note and over the folded Contents.
  for (const width of [900, 390]) {
    const narrow = await contentsAt(width);
    expect(`at ${width}, the note, the boxes, the folded Contents, then the text, got ${JSON.stringify(narrow)}`, narrow.folded && above(narrow.note, narrow.boxes) && above(narrow.boxes, narrow.contents) && above(narrow.contents, narrow.text));
  }

  // ── Whoever may write: the boxes from what is saved, the code in the editor once Edit is pressed ──
  const writer = await signedIn('Frogs');
  await writer.locator('.topic-article-boxes .info-box').waitFor();
  expect('no editor while the article is read', (await writer.locator('.topic-post .ProseMirror').count()) === 0);
  await writer.locator('.topic-page-head button.topic-edit').click();
  const editor = writer.locator('.topic-post .ProseMirror');
  await editor.waitFor();
  expect('the boxes still drawn beside the editor, from what is saved', await writer.locator('.topic-article-boxes .info-box').isVisible());
  expect('the code of the boxes in the editor', (await editor.innerText()).includes('title: Frog'));
  await writer.screenshot({ path: shot('5-writing') });
  await crowded(writer, 'for whoever may write');
  // `/info` on a new line puts in an info box to be filled in.
  await editor.getByText('Many frogs call at night').click();
  await writer.keyboard.press('End');
  // Typed first, as a person writes: a key pressed at once after the click could reach the editor before it is told where the cursor went.
  await writer.keyboard.type(' Some croak.');
  await writer.keyboard.press('Enter');
  await writer.keyboard.type('/info');
  const menu = writer.locator('.topic-post .block-menu');
  await menu.getByText('Info box', { exact: true }).waitFor();
  await writer.keyboard.press('Enter');
  await writer.waitForFunction(() => [...document.querySelectorAll('.topic-post .ProseMirror pre[data-params="infobox"]')].some((one) => one.innerText.includes('title: What it is about')));
  await writer.locator('.topic-post').getByText('Not saved yet').waitFor();
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
