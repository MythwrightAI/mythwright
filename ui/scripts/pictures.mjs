// Pictures, diagrams and views in a topic's text.
//
//   DAEMON=../target/debug/factory node scripts/pictures.mjs [out-dir]
//
// Ada, who owns the factory, writes the topic's post in the block editor:
// a picture pasted there is attached to the topic and put in on a line of
// its own, drawn as it will be read; chosen, it has a bar over it, where
// a caption is written and Side sets it to stand at the side. Saved, the
// post is the text `![How a sale goes](attachment:<id> "side")`, the `/`
// menu offers a picture or a page, and the picture is drawn in the editor
// beside the words after it. Read by somebody who may only read records:
// the figure stands at the side of the column, the words after it beside
// it, its caption under it, on a wide window; across the column, the
// words under it, at a phone's width. In a reply, on Talk (the topic says
// something, so it is an article, its replies on a tab): an SVG pasted in the
// box is put in as a picture, posted and drawn; a page chosen with the
// box's button is drawn in place as a channel draws a view, in its box,
// its own words in it; a `mermaid` block is drawn as its diagram, an SVG
// inside the column and called by its first line, drawn again when the
// page turns light or dark, whose Code shows what it was drawn from; a
// wide one is as wide as the column and, as narrow as a phone, scrolls
// sideways in its box; one that cannot be read, or that asks for the ELK
// layout, is its code with a faint line saying what is wrong; a picture
// somewhere else is a link with its caption, and nothing is fetched from
// anywhere but the daemon; one whose attachment is not there is a line
// saying so.
// Nothing on the page lies over anything else, at 1280 and 800 wide and a
// phone's, plainly and with `LOOK="size=largest density=compact"`, in the
// dark and (`SCHEME=light`) the light. The environment variables are those
// of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'pictures-data');
const home = join(out, 'pictures-home');
const looked = process.env.LOOK ? '-looked' : '';
const scheme = process.env.SCHEME ?? 'dark';
const shot = (name) => `${out}/pictures-${name}${looked}${scheme === 'light' ? '-light' : ''}.png`;

const TITLE = 'How the till works';
const WORDS = 'The till takes a card, asks the bank, and prints a slip. '.repeat(9).trim();
const BODY = `The till, step by step.\n\n${WORDS}\n\n${WORDS}\n`;
/** A diagram as SVG, with a script in it that must never run. */
const SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="360" height="160" viewBox="0 0 360 160"><script>parent.ranFromSvg = true</script><rect x="10" y="50" width="100" height="60" rx="8" fill="none" stroke="#333" stroke-width="3"/><text x="60" y="86" text-anchor="middle" font-family="sans-serif" font-size="16">Card</text><path d="M110 80h120" stroke="#333" stroke-width="3"/><rect x="230" y="50" width="120" height="60" rx="8" fill="none" stroke="#333" stroke-width="3"/><text x="290" y="86" text-anchor="middle" font-family="sans-serif" font-size="16">Bank</text></svg>`;
/** A page put in as a view: its own words, in the reader's colours. */
const VIEW = '<!doctype html><style>body{color:var(--text)} b{color:var(--accent)}</style><p><b>Sales by day</b>: 41, 38, 45.</p>';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });

let failed = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('topic', 'make', TITLE, '--body', BODY);

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
  /** Ada's page; with `holds`, the page is told she holds only those, as somebody who may only read. */
  const signedIn = async (how = { viewport: { width: 1280, height: 860 } }, holds = null) => {
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
    await page.goto(`${url}/topic/${encodeURIComponent(TITLE)}`);
    await page.locator('.topic-page').waitFor();
    return page;
  };
  const crowded = async (on, what) => {
    const wrong = await pageCrowded(on, '.topic-page');
    expect(`nothing of the topic's page over anything else ${what}${looked}, got ${wrong.join('; ')}`, wrong.length === 0);
  };
  /** Pastes a file into what has the keyboard, as a browser does with one on the clipboard: a PNG drawn here, or text of a kind. */
  const paste = (on, file) =>
    on.evaluate(async ({ name, type, text }) => {
      let blob;
      if (text === null) {
        const canvas = Object.assign(document.createElement('canvas'), { width: 480, height: 300 });
        const drawing = canvas.getContext('2d');
        drawing.fillStyle = '#2b6cb0';
        drawing.fillRect(0, 0, 480, 300);
        drawing.fillStyle = '#f6e05e';
        drawing.fillRect(40, 40, 180, 120);
        blob = await new Promise((done) => canvas.toBlob(done, 'image/png'));
      } else blob = new Blob([text], { type });
      const carried = new DataTransfer();
      carried.items.add(new File([blob], name, { type }));
      document.activeElement.dispatchEvent(new ClipboardEvent('paste', { clipboardData: carried, bubbles: true, cancelable: true }));
    }, file);
  const loaded = (picture) => picture.evaluate((img) => img.complete && img.naturalWidth > 0);

  // ── The post, written ────────────────────────────────────────────────────
  const page = await signedIn();
  const post = page.locator('.topic-post');
  const editor = post.locator('.ProseMirror');
  // Read until Edit is pressed.
  await page.locator('.topic-page-head button.topic-edit').click();
  await editor.click();
  await page.keyboard.press('Control+Home');
  await paste(page, { name: 'pasted.png', type: 'image/png', text: null });
  const inEditor = post.locator('.writer-picture');
  await inEditor.locator('img.picture-drawn').waitFor();
  await page.waitForFunction(() => document.querySelector('.topic-post .writer-picture img')?.naturalWidth > 0);
  expect('the picture put in after the line the cursor was on', (await editor.locator('p').first().innerText()).trim() === 'The till, step by step.');
  await inEditor.locator('img').click();
  const bar = inEditor.locator('.writer-picture-bar');
  await bar.waitFor();
  await bar.getByLabel('Caption').fill('How a sale goes');
  await bar.getByRole('button', { name: 'Side', exact: true }).click();
  expect('Side pressed', (await bar.getByRole('button', { name: 'Side', exact: true }).getAttribute('aria-pressed')) === 'true');
  await page.screenshot({ path: shot('1-editing') });
  await post.getByRole('button', { name: 'Save', exact: true }).click();
  await post.getByText('Not saved yet').waitFor({ state: 'detached' });
  const shown = JSON.parse((await cli('topic', 'show', TITLE, '--json')).stdout);
  const body = shown.topic?.data?.body ?? '';
  const id = /!\[How a sale goes\]\(attachment:(ratt_\w+) "side"\)/.exec(body)?.[1];
  expect(`the post to keep the picture as text with its caption and place, got ${JSON.stringify(body)}`, !!id && body.includes(WORDS));
  // Beside the words after it, in the editor too, opened again: Save returns it to reading.
  await page.locator('.topic-page-head button.topic-edit').click();
  await inEditor.locator('img.picture-drawn').waitFor();
  const [pictureThere, column] = [await inEditor.boundingBox(), await editor.boundingBox()];
  expect(`the picture at the side of the editor's column, got ${JSON.stringify({ pictureThere, column })}`, pictureThere.x > column.x + column.width * 0.4 && pictureThere.width < column.width * 0.6);
  // The `/` menu offers a picture or a page.
  await editor.locator('p').last().click();
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await page.keyboard.type('/pic');
  await page.locator('.block-menu button', { hasText: 'Picture or page' }).waitFor();
  await page.keyboard.press('Escape');
  await page.keyboard.press('Backspace');
  await page.keyboard.press('Backspace');
  await page.keyboard.press('Backspace');
  await page.keyboard.press('Backspace');
  await page.keyboard.press('Backspace');

  // ── Read, by somebody who may only read ──────────────────────────────────
  const READS = ['projects:read', 'channels', 'records:read', 'board:read', 'search'];
  const reader = await signedIn(undefined, READS);
  const figure = reader.locator('.topic-post .topic-doc-read .prose-figure');
  await figure.locator('img.picture-drawn').waitFor();
  await reader.waitForFunction(() => document.querySelector('.topic-post .prose-figure img')?.naturalWidth > 0);
  expect('its caption under it', (await figure.locator('figcaption').innerText()).trim() === 'How a sale goes');
  const sides = async (on, what) => {
    const boxes = await on.evaluate(() => {
      const read = document.querySelector('.topic-post .topic-doc-read');
      const fig = read.querySelector('.prose-figure');
      const after = fig.nextElementSibling;
      // Where the first line of the words after it is.
      const range = document.createRange();
      range.setStart(after.firstChild ?? after, 0);
      range.setEnd(after.firstChild ?? after, 1);
      const box = (one) => one.getBoundingClientRect().toJSON();
      return { column: box(read), figure: box(fig), first: range.getBoundingClientRect().toJSON(), caption: box(fig.querySelector('figcaption')), img: box(fig.querySelector('img')) };
    });
    return { ...boxes, what };
  };
  for (const width of [1280, 800]) {
    await reader.setViewportSize({ width, height: 860 });
    const at = await sides(reader, width);
    expect(`at ${width} the figure at the side of the column, the words after it beside it, got ${JSON.stringify(at)}`, at.figure.right <= at.column.right + 1 && at.figure.left > at.column.left + at.column.width * 0.4 && at.first.top < at.figure.bottom && at.first.right <= at.figure.left + 1);
    expect(`its caption under the picture, got ${JSON.stringify(at)}`, at.caption.top >= at.img.bottom - 1);
    await crowded(reader, `read at ${width}`);
  }
  await reader.setViewportSize({ width: 1280, height: 860 });
  await reader.screenshot({ path: shot('2-side') });
  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true }, READS);
  await phone.locator('.topic-post .prose-figure img.picture-drawn').waitFor();
  await phone.waitForFunction(() => document.querySelector('.topic-post .prose-figure img')?.naturalWidth > 0);
  const across = await sides(phone, 'a phone');
  expect(`on a phone the figure across the column, the words under it, got ${JSON.stringify(across)}`, across.figure.width >= across.column.width - 2 && across.first.top >= across.figure.bottom - 1);
  await crowded(phone, 'on a phone');
  await phone.locator('.topic-post').screenshot({ path: shot('3-phone') });

  // ── Replies: an SVG, a view, diagrams, somewhere else, and one that is not there ───
  // Its post says something, so it is an article: its replies are on Talk.
  await page.locator('.topic-tabs').getByRole('tab', { name: /^Talk/ }).click();
  await page.locator('.topic-page[data-shows="talk"]').waitFor();
  const replyBox = page.getByLabel('Reply to the topic', { exact: true });
  const reply = async (words, put) => {
    await replyBox.click();
    if (put) await put();
    await replyBox.press('End');
    if (words) await replyBox.pressSequentially(words);
    await page.locator('.topic-reply-box').first().getByRole('button', { name: 'Post', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('.topic-reply-box textarea')?.value === '');
  };
  await reply('The same, drawn.', async () => {
    await paste(page, { name: 'flow.svg', type: 'image/svg+xml', text: SVG });
    await page.waitForFunction(() => /!\[\]\(attachment:ratt_\w+\)\n$/.test(document.querySelector('.topic-reply-box textarea')?.value ?? ''));
  });
  const drawnSvg = page.locator('.topic-reply-words .prose-figure img.picture-drawn').first();
  await drawnSvg.waitFor();
  await page.waitForFunction(() => [...document.querySelectorAll('.topic-reply-words .prose-figure img')].some((img) => img.naturalWidth > 0));
  expect('the SVG drawn', await loaded(drawnSvg));
  expect("nothing in the SVG run", (await page.evaluate(() => window.ranFromSvg)) !== true);

  await reply('', async () => {
    await page.locator('.topic-reply-box').first().locator('input[type="file"]').setInputFiles({ name: 'Sales by day.html', mimeType: 'text/html', buffer: Buffer.from(VIEW) });
    await page.waitForFunction(() => /!\[Sales by day\]\(attachment:ratt_\w+\)\n$/.test(document.querySelector('.topic-reply-box textarea')?.value ?? ''));
  });
  const view = page.locator('.topic-reply-words .prose-figure.drawn-view').first();
  await view.locator('iframe.view-frame').waitFor();
  const frame = view.frameLocator('iframe.view-frame');
  await frame.getByText('Sales by day').waitFor();
  // In its reply's room: what follows stands under it.
  const held = await view.evaluate((one) => {
    const reply = one.closest('.topic-reply-body');
    const next = reply.querySelector('.topic-reply-acts');
    return { figure: one.getBoundingClientRect().toJSON(), words: one.closest('.topic-reply-words').getBoundingClientRect().toJSON(), acts: next.getBoundingClientRect().toJSON() };
  });
  expect(`the view inside its reply, what follows under it, got ${JSON.stringify(held)}`, held.figure.bottom <= held.words.bottom + 1 && held.acts.top >= held.figure.bottom - 1);
  expect('the view in its box, captioned', (await view.locator('figcaption').innerText()).trim() === 'Sales by day' && (await view.locator('iframe').getAttribute('sandbox')) === 'allow-scripts');

  await reply('```mermaid\nflowchart LR\n  Card[A card] --> Till{The till}\n  Till -->|asks| Bank[(The bank)]\n  Till --> Slip[A slip]\n```');
  const diagram = page.locator('.topic-reply-words .diagram').first();
  await diagram.locator(`.diagram-box svg`).waitFor();
  expect(`the diagram drawn in the page's ${scheme}`, (await diagram.getAttribute('data-drawn')) === scheme);
  const box = diagram.locator('.diagram-box');
  expect('the diagram called by its first line', (await box.getAttribute('role')) === 'img' && (await box.getAttribute('aria-label')) === 'flowchart LR');
  expect('the diagram to say what it shows', (await box.innerText()).includes('The till') && (await box.innerText()).includes('asks'));
  /** Where a diagram's SVG is, and the column of the reply it is in. */
  const drawnIn = (one) =>
    one.evaluate((whole) => {
      const svg = whole.querySelector('.diagram-box svg');
      const scroller = whole.querySelector('.diagram-box');
      return { natural: svg.viewBox.baseVal.width, svg: svg.getBoundingClientRect().toJSON(), column: whole.closest('.topic-reply-words').getBoundingClientRect().toJSON(), scrolls: scroller.scrollWidth > scroller.clientWidth + 1, page: document.scrollingElement.scrollWidth <= innerWidth + 1, box: scroller.getBoundingClientRect().toJSON() };
    });
  const flow = await drawnIn(diagram);
  expect(`the diagram an SVG inside the column, got ${JSON.stringify(flow)}`, flow.svg.width > 150 && flow.svg.height > 40 && flow.svg.left >= flow.column.left - 1 && flow.svg.right <= flow.column.right + 1 && !flow.scrolls);
  await diagram.screenshot({ path: shot('5-diagram') });
  // Drawn again in the other light, and back.
  const other = scheme === 'light' ? 'dark' : 'light';
  await litAs(page, other);
  await page.waitForFunction((now) => document.querySelector('.topic-reply-words .diagram')?.getAttribute('data-drawn') === now, other);
  await diagram.screenshot({ path: shot(`5-diagram-${other}`) });
  await litAs(page, scheme);
  await page.waitForFunction((now) => document.querySelector('.topic-reply-words .diagram')?.getAttribute('data-drawn') === now, scheme);
  // Code shows what it was drawn from, and again the diagram.
  const codeToggle = diagram.getByRole('button', { name: 'Code', exact: true });
  await codeToggle.click();
  expect('Code pressed', (await codeToggle.getAttribute('aria-pressed')) === 'true');
  expect('Code to show what the diagram was drawn from', (await diagram.locator('.code-block-text').innerText()).includes('Till -->|asks| Bank[(The bank)]'));
  expect('Code to show the code in place of the diagram', (await diagram.locator('.diagram-box').count()) === 0);
  await diagram.screenshot({ path: shot('6-diagram-code') });
  await codeToggle.click();
  await diagram.locator('.diagram-box svg').waitFor();

  // A wide one: made as small as the column, and at a phone's width, where it would be too small to read, scrolling sideways in its box.
  const chain = Array.from({ length: 5 }, (_, at) => `S${at}[Step ${at}]`).join(' --> ');
  await reply(`\`\`\`mermaid\nflowchart LR\n  ${chain}\n\`\`\``);
  const wide = page.locator('.topic-reply-words .diagram').nth(1);
  await wide.locator('.diagram-box svg').waitFor();
  const spread = await drawnIn(wide);
  expect(`a wide diagram made as small as the column, got ${JSON.stringify(spread)}`, Math.abs(spread.svg.width - spread.column.width) <= 2 && !spread.scrolls && spread.natural > spread.column.width);
  await phone.locator('.topic-tabs').getByRole('tab', { name: /^Talk/ }).click();
  await phone.locator('.topic-page[data-shows="talk"]').waitFor();
  const wideOnPhone = phone.locator('.topic-reply-words .diagram').nth(1);
  await wideOnPhone.locator('.diagram-box svg').waitFor();
  await wideOnPhone.scrollIntoViewIfNeeded();
  const narrow = await drawnIn(wideOnPhone);
  expect(`a wide diagram scrolling sideways in its box on a phone, the page not, got ${JSON.stringify(narrow)}`, narrow.scrolls && narrow.page && narrow.box.right <= narrow.column.right + 1);
  await wideOnPhone.screenshot({ path: shot('7-diagram-phone') });

  await reply('```mermaid\nflowchart TD\n  A --> \n```');
  const failed = page.locator('.topic-reply-words .diagram-failed').first();
  await failed.waitFor();
  expect('a diagram that cannot be read drawn as its code', (await failed.locator('.code-block-text').innerText()).includes('flowchart TD'));
  const why = (await failed.locator('.code-undrawn-said').innerText()).trim();
  expect(`a faint line saying what is wrong, on one line, got ${JSON.stringify(why)}`, why.startsWith('Not drawn: Parse error on line ') && !why.includes('\n'));
  expect('nothing drawn by the library of itself for it', (await page.locator('body > div[id^="ddiagram-"], body > svg[id^="diagram-"]').count()) === 0);
  // One that asks for the ELK layout, which is not built in, says so.
  await reply('```mermaid\n---\nconfig:\n  layout: elk\n---\nflowchart LR\n  A --> B\n```');
  const elk = page.locator('.topic-reply-words .diagram-failed').nth(1);
  await elk.waitFor();
  expect('a diagram asking for ELK to say it is not in this client', (await elk.locator('.code-undrawn-said').innerText()).includes('The ELK layout is not in this client'));

  await reply('![The logo](https://example.com/logo.png)\n![Gone](attachment:ratt_nothere)');
  const link = page.locator('.topic-reply-words a.picture-elsewhere').first();
  await link.waitFor();
  expect('a picture somewhere else a link with its caption', (await link.innerText()).trim() === 'The logo' && (await link.getAttribute('href')) === 'https://example.com/logo.png');
  // A picture is fetched as it comes into view.
  await page.locator('.topic-reply-words').last().scrollIntoViewIfNeeded();
  const missing = page.locator('.topic-reply-words .picture-missing').first();
  await missing.waitFor();
  expect('one that is not there said so', (await missing.innerText()).includes('“Gone” is not here'));
  await page.locator('.topic-replies').scrollIntoViewIfNeeded();
  for (const width of [1280, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await crowded(page, `with the replies at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.locator('.topic-replies-part').screenshot({ path: shot('4-replies') });
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
console.log(`pictures are in ${out}`);
