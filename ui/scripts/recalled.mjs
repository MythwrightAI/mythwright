// What the reader sent before, put back in a channel's box by Up as a
// terminal's history is (`ui/src/recalled.ts`).
//
//   DAEMON=../target/debug/factory node scripts/recalled.mjs [out-dir]
//
// Up in an empty box puts back the last thing the reader sent in that
// channel, with the caret at its end, and again goes further back, stopping
// at the first; Down goes forward and, past the newest, gives back an empty
// box. What somebody else said is no part of it, and what the reader said
// from a terminal, which the page heard of by an event, is. A message of
// two lines recalled has its own Up first: the caret goes to its first line
// and the next Up goes further back. What was half written before Up is
// given back past the newest. While the names offered after an @ are open
// the arrows move through them and nothing is recalled, and in a message of
// several lines being written Up moves the caret a line. A recalled message
// sent says it again, and nothing of the one it came from changes. A
// thread's box goes back through what the reader said in that thread alone.
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { openAdding } from './adding.mjs';
import { sized } from './looked.ts';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'recalled-data');
const home = join(out, 'recalled-home');

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
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: sized(1280), height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
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
  const channels = page.getByRole('button', { name: 'Channels', exact: true });
  if ((await channels.getAttribute('aria-pressed')) !== 'true') await channels.click();
  await page.getByRole('button', { name: 'New channel' }).click();
  await page.getByLabel('Name').fill('general');
  await page.getByRole('button', { name: 'Create', exact: true }).click();
  await page.getByRole('heading', { name: '# general' }).waitFor();

  // A bot of the channel, which answers only where it is named: somebody else who says something there.
  const adding = await openAdding(page);
  await adding.getByRole('radio', { name: /^A bot/ }).check();
  await adding.getByLabel('Name').fill('Parrot');
  await adding.getByLabel('Answers with what this command prints').fill('echo "heard: $MESSAGE"');
  await adding.getByRole('button', { name: 'Add bot', exact: true }).click();
  await adding.waitFor({ state: 'detached' });

  const inChannel = page.locator('.channel > .split .messages .message');
  const box = page.getByLabel('Message');
  const state = (area) => area.evaluate((el) => ({ text: el.value, from: el.selectionStart, to: el.selectionEnd }));
  const said = async (area, text) => {
    await area.click();
    for (const [index, line] of text.split('\n').entries()) {
      if (index > 0) await page.keyboard.press('Shift+Enter');
      await page.keyboard.type(line);
    }
    await page.keyboard.press('Enter');
  };
  const key = async (area, name, text, caret, why) => {
    await page.keyboard.press(name);
    const now = await state(area);
    expect(`${why}: the box to hold ${JSON.stringify(text)} with the caret at ${caret}, got ${JSON.stringify(now)}`, now.text === text && now.from === caret && now.to === caret);
  };

  await said(box, 'first');
  await inChannel.filter({ hasText: 'first' }).waitFor();
  await said(box, 'second');
  await inChannel.filter({ hasText: 'second' }).waitFor();
  await said(box, 'line one\nline two');
  await inChannel.filter({ hasText: 'line two' }).waitFor();
  await said(box, '@parrot hi');
  await inChannel.filter({ hasText: 'heard: @parrot hi' }).waitFor();
  expect('the box to be empty once each was sent', (await state(box)).text === '');

  // Up from an empty box: the last the reader sent, and not the bot's answer after it.
  await box.click();
  await key(box, 'ArrowUp', '@parrot hi', 10, 'Up in an empty box');
  expect('no names to be offered for a recalled mention that is finished', (await page.locator('.mentions').count()) === 0);
  await page.screenshot({ path: `${out}/recalled-1-up.png` });
  // Two lines: Up is the box's until the caret is on the first.
  await key(box, 'ArrowUp', 'line one\nline two', 17, 'Up again');
  await page.keyboard.press('ArrowUp');
  const onFirst = await state(box);
  expect(`Up in a recalled message of two lines to move the caret to its first line, got ${JSON.stringify(onFirst)}`, onFirst.text === 'line one\nline two' && onFirst.from <= 8);
  await page.screenshot({ path: `${out}/recalled-2-two-lines.png` });
  await key(box, 'ArrowUp', 'second', 6, 'Up on its first line');
  await key(box, 'ArrowUp', 'first', 5, 'Up again');
  await key(box, 'ArrowUp', 'first', 5, 'Up at the first thing sent');
  // Down comes back the way it went, and past the newest the box is empty.
  await key(box, 'ArrowDown', 'second', 6, 'Down');
  await key(box, 'ArrowDown', 'line one\nline two', 17, 'Down again');
  await key(box, 'ArrowDown', '@parrot hi', 10, 'Down on the last line');
  await key(box, 'ArrowDown', '', 0, 'Down past the newest');
  expect('the box to be one line tall again', (await box.evaluate((el) => el.value.split('\n').length)) === 1);

  // What was said from a terminal arrives by an event, and is the reader's as much as what was said here.
  await cli('post', 'general', 'from the terminal');
  await inChannel.filter({ hasText: 'from the terminal' }).waitFor();
  await box.click();
  await key(box, 'ArrowUp', 'from the terminal', 17, 'Up after a message from the terminal arrived');
  await key(box, 'ArrowDown', '', 0, 'Down');

  // What was half written is kept, and given back past the newest.
  await page.keyboard.type('half wri');
  await page.keyboard.press('Home');
  await key(box, 'ArrowUp', 'from the terminal', 17, 'Up with the caret at the start of what was half written');
  await key(box, 'ArrowUp', '@parrot hi', 10, 'Up again');
  await page.screenshot({ path: `${out}/recalled-3-half-written.png` });
  await key(box, 'ArrowDown', 'from the terminal', 17, 'Down');
  await key(box, 'ArrowDown', 'half wri', 8, 'Down past the newest');
  // With words before the caret on its one line, Up is the box's.
  await page.keyboard.press('ArrowUp');
  expect('Up after half written words to be no recall', (await state(box)).text === 'half wri');

  // While names are offered after an @, the arrows move through them and nothing is recalled.
  await page.keyboard.press('End');
  await page.keyboard.type(' @');
  const list = page.locator('.mentions[role=listbox]');
  await list.waitFor();
  const marked = () => list.locator('[role=option][aria-selected=true] strong').innerText();
  const names = await list.locator('[role=option] strong').allTextContents();
  expect(`two names to be offered, got ${names}`, names.length >= 2);
  const at = await marked();
  await page.keyboard.press('ArrowUp');
  expect(`Up to mark another name, got ${await marked()} after ${at}`, (await marked()) !== at && (await marked()) === names.at(-1));
  expect('what is written not to change', (await state(box)).text === 'half wri @');
  await page.screenshot({ path: `${out}/recalled-4-names.png` });
  await page.keyboard.press('ArrowDown');
  expect('Down to mark the first again', (await marked()) === names[0]);
  expect('what is written still not to change', (await state(box)).text === 'half wri @');
  await page.keyboard.press('Escape');
  await list.waitFor({ state: 'detached' });

  // In a message of several lines being written, Up moves the caret a line.
  await box.fill('');
  await page.keyboard.type('one');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.type('two');
  await page.keyboard.press('ArrowUp');
  const moved = await state(box);
  expect(`Up on the second line to move the caret to the first, got ${JSON.stringify(moved)}`, moved.text === 'one\ntwo' && moved.from <= 3);
  await page.screenshot({ path: `${out}/recalled-5-several-lines.png` });

  // Sent again, a recalled message is said anew, and the one it came from is as it was.
  await box.fill('');
  await key(box, 'ArrowUp', 'from the terminal', 17, 'Up before sending');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  expect('the box to hold what was said second last', (await state(box)).text === 'line one\nline two');
  const before = await inChannel.count();
  await box.fill('');
  await key(box, 'ArrowUp', 'from the terminal', 17, 'Up');
  await page.keyboard.press('Enter');
  await page.waitForFunction((n) => document.querySelectorAll('.channel > .split .messages .message').length > n, before);
  expect('the box to be empty once it is sent', (await state(box)).text === '');
  expect('it to be said twice now', (await inChannel.filter({ hasText: 'from the terminal' }).count()) === 2);
  await key(box, 'ArrowUp', 'from the terminal', 17, 'Up after it was sent again');
  await key(box, 'ArrowUp', '@parrot hi', 10, 'Up past the same words sent twice running, which are one');
  await box.fill('');

  // A thread goes back through what was said in it, and the channel's what was said in the channel.
  const first = inChannel.filter({ hasText: 'first' }).first();
  await first.hover();
  await first.getByRole('button', { name: 'Reply in thread' }).click();
  const thread = page.locator('.thread');
  await thread.waitFor();
  await thread.locator('.thread-count', { hasText: '0 replies' }).waitFor();
  const reply = thread.getByLabel('Reply');
  await reply.click();
  await page.keyboard.press('ArrowUp');
  expect('nothing recalled in a thread nobody said anything in', (await state(reply)).text === '');
  await said(reply, 'reply one');
  await thread.locator('.thread-count', { hasText: '1 reply' }).waitFor();
  await said(reply, 'reply two');
  await thread.locator('.thread-count', { hasText: '2 replies' }).waitFor();
  await reply.click();
  await key(reply, 'ArrowUp', 'reply two', 9, 'Up in the thread');
  await key(reply, 'ArrowUp', 'reply one', 9, 'Up again in the thread');
  await key(reply, 'ArrowUp', 'reply one', 9, 'Up at the first thing said in the thread');
  await page.screenshot({ path: `${out}/recalled-6-thread.png` });
  await key(reply, 'ArrowDown', 'reply two', 9, 'Down in the thread');
  await key(reply, 'ArrowDown', '', 0, 'Down past the newest in the thread');
  // Where the window is narrow for the text the thread's panel lies over the channel: the box is reached by the keyboard.
  await box.focus();
  await key(box, 'ArrowUp', 'from the terminal', 17, 'Up in the channel, with no reply of the thread in it');

  // On a phone nothing is recalled: there are no arrows to speak of.
  await page.getByRole('button', { name: 'Close the thread' }).click();
  await thread.waitFor({ state: 'detached' });
  await page.setViewportSize({ width: 390, height: 780 });
  // There the tools panel has the screen: it is put away to show the channel. The page comes to its phone's layout a
  // moment after the window is narrowed, so what is there is waited for, and not asked at once: asked at once, the
  // panel was still beside the channel, nothing was pressed, and it then had the screen for good (once in a few runs
  // at the largest size of text).
  const away = page.getByRole('button', { name: 'Back to the channel' });
  await away.or(page.locator('.channel:not(:has(.context))')).first().waitFor();
  if (await away.isVisible()) await away.click();
  // Gone, and nothing of it still on its way out.
  await page.locator('.context').waitFor({ state: 'detached' });
  await page.waitForFunction(() => document.getAnimations().every((one) => one.playState !== 'running'));
  const narrow = page.getByLabel('Message');
  await narrow.waitFor({ state: 'visible' });
  await narrow.fill('');
  await narrow.click();
  await page.keyboard.press('ArrowUp');
  expect('nothing recalled at a phone\'s width', (await state(narrow)).text === '');
  await page.screenshot({ path: `${out}/recalled-7-phone.png` });

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
