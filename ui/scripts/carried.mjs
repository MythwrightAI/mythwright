// A channel while its fleet's agents are at work: who is working on a card,
// and who is writing there.
//
//   DAEMON=../target/debug/factory node scripts/carried.mjs [out-dir]
//
// The one who works here is a stand-in that takes its time and changes
// nothing, so that there is a while to look. Working a card is said in
// the channel its fleet reports in as working on it, in a quiet line of
// its own, and nobody is said to be typing; an agent answering what was
// said there is. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { beginsWithNames, boxes, DOT, over, paste, standOneOverAnother } from './attaching.mjs';
import { toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { offeredWhole, toolMenuWhole } from './offered.mjs';
import { unfoldChannels } from './folded.mjs';
import { threadStep } from './threadstep.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'carried-data');
const home = join(out, 'carried-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// Reads what it is asked, waits, and says it is done.
const slow = join(home, 'slow.sh');
await writeFile(slow, '#!/bin/sh\ncat >/dev/null\nsleep 7\necho It is done.\n');
await chmod(slow, 0o755);
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--command', `slow=${slow}`], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('record', 'set', 'shop-agents-2', 'adapter=slow');
  // The architect, who is talked with in the channel, takes its time answering too, so that a thread can say so.
  await cli('record', 'set', 'shop-agents-1', 'adapter=slow');
  await cli('board', 'add', 'shop', 'Add a shopping cart');
  await cli('board', 'add', 'shop', 'Add a till');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1500, height: 900 }, colorScheme: process.env.SCHEME ?? 'dark' });
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
  // While an agent is writing in a channel, the channel says so in a line
  // over where one writes, beginning where the names of whoever said
  // something begin, beside their pictures (the box one writes in is
  // narrower and centred), whose room is there whether or not anybody is
  // writing, so that nothing moves as it comes and goes.
  const channels = page.getByRole('button', { name: 'Channels', exact: true });
  if ((await channels.getAttribute('aria-pressed')) !== 'true') await channels.click();
  // Every branch of the channels starts folded (`folded.mjs`): the project's, which holds `shop`, is unfolded as a person would.
  await unfoldChannels(page);
  await page.locator('.nav').getByText('shop', { exact: true }).first().click();
  const line = page.locator('.typing-edge');
  await line.waitFor({ state: 'attached' });
  const box = page.getByLabel('Message');
  const placed = async () => {
    const [written, said, last, list] = await Promise.all([
      box.boundingBox(),
      line.boundingBox(),
      page.locator('.messages .message').last().boundingBox(),
      page.locator('.messages').boundingBox(),
    ]);
    return { written, said, last, list };
  };
  const unworked = await placed();
  expect(`nobody to be said to be writing yet, got "${await line.innerText()}"`, (await line.innerText()).trim() === '');

  // An agent working a card that its fleet reports in this channel is at work and not writing here: nobody
  // is said to be typing, and a quiet line of its own, at the other end of that line's room, says who is at work on
  // what, moving nothing; pressed, the card is opened beside the channel, where the agent's card says the same.
  await cli('board', 'move', 'shop-2', 'ready');
  const atWork = page.locator('.working-line');
  await atWork.getByText('shop-worker-1 is working on shop-2', { exact: true }).waitFor({ timeout: 15000 });
  expect(`nobody to be said to be typing while a card is worked, got "${await line.innerText()}"`, (await line.innerText()).trim() === '');
  const working = await placed();
  const quietly = await atWork.boundingBox();
  expect(
    `nothing to move as a card is worked: the box was at ${unworked.written.y} and is at ${working.written.y}, the line at ${unworked.said.y} and ${working.said.y}, the list ${unworked.list.height} and ${working.list.height} tall`,
    unworked.written.y === working.written.y && unworked.said.y === working.said.y && unworked.list.height === working.list.height,
  );
  expect(
    `the line of who is at work to lie in the room of who is typing, over where one writes: it is at ${JSON.stringify(quietly)}, that room at ${JSON.stringify(working.said)}`,
    quietly.y >= working.said.y - 0.5 && quietly.y + quietly.height <= working.said.y + working.said.height + 0.5 && quietly.x >= working.said.x - 0.5 && quietly.x + quietly.width <= working.said.x + working.said.width + 0.5,
  );
  expect('no dots in the line of who is at work', (await atWork.locator('.typing-dots').count()) === 0);
  {
    const near = await page.locator('.composer').boundingBox();
    await page.screenshot({ path: `${out}/carried-5-working.png`, clip: { x: working.list.x, y: near.y - 60, width: working.list.width, height: near.height + 60 } });
  }
  await atWork.getByRole('button', { name: 'shop-worker-1 is working on shop-2' }).click();
  const tools = page.getByRole('complementary', { name: 'Tools' });
  await toolsAtRest(page).waitFor();
  expect('the card to be opened beside the channel', (await tools.getByRole('tab', { name: /^Board\b/ }).getAttribute('aria-selected')) === 'true' && (await tools.innerText()).includes('shop-2'));
  await page.screenshot({ path: `${out}/carried-5-working-card.png` });
  await tools.getByRole('tab', { name: /^Channel\b/ }).click();
  const workerCard = tools.locator('.context-member.agent', { hasText: 'shop-worker-1' });
  await workerCard.locator('.context-doing', { hasText: 'working on shop-2' }).waitFor();
  // Read about there, the one at work says what its program sees of this device: it runs on a program, and a
  // program is kept behind a wall.
  await workerCard.click();
  const about = tools.locator('.context-about');
  await about.waitFor();
  const sees = (await about.innerText()).replace(/\s+/g, ' ');
  expect(`the agent to be said to work behind a wall, got ${sees}`, /Sees its work: behind a wall/i.test(sees));
  await tools.getByRole('navigation', { name: 'Where this is' }).getByRole('button', { name: 'Channel', exact: true }).click();
  await workerCard.waitFor();
  expect(`nobody to be typing still, got "${await line.innerText()}"`, (await line.innerText()).trim() === '');
  await page.getByRole('button', { name: 'Collapse the tools panel' }).click();
  await tools.waitFor({ state: 'detached' });
  await atWork.waitFor({ state: 'detached', timeout: 30000 });
  expect(`nobody to have been typing as the card was worked, got "${await line.innerText()}"`, (await line.innerText()).trim() === '');

  // While an agent answers what was said in the channel, it is writing there.
  const quiet = await placed();
  // Sixteen pixels as the usual height of things goes, and as tall as that is where a theme says otherwise.
  const sixteen = await page.evaluate(() => {
    const ruler = document.body.appendChild(document.createElement('div'));
    ruler.style.height = 'calc(16px * var(--tall))';
    const measured = ruler.getBoundingClientRect().height;
    ruler.remove();
    return measured;
  });
  expect(`the line's room to be there with nobody writing, ${sixteen} tall, got ${JSON.stringify(quiet.said)}`, quiet.said.height === sixteen);
  expect('no such line among what was said', (await page.locator('.messages .typing-line').count()) === 0);
  await cli('post', 'shop', '@shop-architect-1 what goes in the cart?');
  await page.waitForFunction(() => /is typing/.test(document.querySelector('.typing-edge')?.textContent ?? ''), null, { timeout: 15000 });
  expect(`the line to say who is writing, got "${await line.innerText()}"`, (await line.innerText()).trim() === 'shop-architect-1 is typing…');
  const busy = await placed();
  // Where the words begin (the dots before them), against where the name of whoever said something does, by the boxes.
  expect(`the line to be over where one writes: it is at ${JSON.stringify(busy.said)}, what is written at ${JSON.stringify(busy.written)}`, busy.said.y + busy.said.height <= busy.written.y);
  await beginsWithNames(page, expect, '.channel > .split', 'while somebody is writing in the channel');
  expect(
    `nothing to have moved as somebody began to write: the box was at ${quiet.written.y} and is at ${busy.written.y}, the line at ${quiet.said.y} and ${busy.said.y}, the list ${quiet.list.height} and ${busy.list.height} tall`,
    quiet.written.y === busy.written.y && quiet.said.y === busy.said.y && quiet.list.height === busy.list.height,
  );
  expect(
    `the last thing said to be whole above where one writes: it ends at ${busy.last.y + busy.last.height}, the list at ${busy.list.y + busy.list.height}, the line begins at ${busy.said.y}`,
    busy.last.y + busy.last.height <= busy.list.y + busy.list.height + 0.5 && busy.list.y + busy.list.height <= busy.said.y,
  );
  expect('the list to be at its end', await page.locator('.messages').evaluate((list) => list.scrollHeight - list.scrollTop - list.clientHeight < 2));
  await page.screenshot({ path: `${out}/carried-5-typing.png` });
  // A picture pasted while somebody is writing waits over the box and over the line that says so, flush with the
  // box and outside it, and moves nothing: not the line, not the box, not the list.
  await paste(box, { files: [{ name: 'image.png', type: 'image/png', base64: DOT }] });
  await page.locator('.composer-over .attaching-picture').waitFor();
  await standOneOverAnother(page, expect, 'while somebody is writing');
  const waited = await placed();
  expect(
    `nothing to move as a picture came to wait: the box was at ${busy.written.y} and is at ${waited.written.y}, the line at ${busy.said.y} and ${waited.said.y}, the list ${busy.list.height} and ${waited.list.height} tall`,
    waited.written.y === busy.written.y && waited.written.height === busy.written.height && waited.said.y === busy.said.y && waited.list.height === busy.list.height,
  );
  {
    const near = await page.locator('.composer').boundingBox();
    await page.screenshot({ path: `${out}/carried-5-typing-attaching.png`, clip: { x: busy.list.x, y: near.y - 140, width: near.x + near.width - busy.list.x, height: near.height + 140 } });
  }
  await page.getByRole('button', { name: /^Remove pasted-/ }).click();
  await page.locator('.composer-over .attaching').waitFor({ state: 'detached' });
  // Beside the words three dots move, so that the line is seen: inside
  // the line's own room, before its words, in a colour of the theme; and
  // they are seen to move while everything else stays where it is.
  const dots = () =>
    page.evaluate(() => {
      const line = document.querySelector('.typing-edge');
      const all = [...line.querySelectorAll('.typing-dots i')];
      const within = line.getBoundingClientRect();
      const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim();
      const probe = document.createElement('span');
      probe.style.color = accent;
      line.append(probe);
      const theme = getComputedStyle(probe).color;
      probe.remove();
      return {
        count: all.length,
        inside: all.every((dot) => {
          const box = dot.getBoundingClientRect();
          return box.left >= within.left - 0.5 && box.right <= within.right + 0.5 && box.top >= within.top - 0.5 && box.bottom <= within.bottom + 0.5;
        }),
        moving: all.map((dot) => getComputedStyle(dot).animationName),
        running: all.every((dot) => dot.getAnimations().some((one) => one.playState === 'running')),
        themed: all.every((dot) => getComputedStyle(dot).backgroundColor === theme),
        looks: all.map((dot) => `${getComputedStyle(dot).opacity} ${getComputedStyle(dot).transform}`).join('|'),
        said: line.textContent,
      };
    });
  const first = await dots();
  expect(`three dots beside the words while somebody is writing, got ${first.count}`, first.count === 3);
  expect('the dots to be inside the line\'s own room', first.inside);
  expect(`the dots to move, got ${first.moving.join()}`, first.running && first.moving.every((name) => name === 'typing-dot'));
  expect('the dots to be in the theme\'s own colour', first.themed);
  expect(`the dots to say nothing of their own, got "${first.said}"`, first.said.trim() === 'shop-architect-1 is typing…');
  await page.waitForFunction((was) => [...document.querySelectorAll('.typing-edge .typing-dots i')].map((dot) => `${getComputedStyle(dot).opacity} ${getComputedStyle(dot).transform}`).join('|') !== was, first.looks, { timeout: 3000 });
  const moved = await placed();
  expect(
    `nothing else to move while the dots do: the box was at ${busy.written.y} and is at ${moved.written.y}, the line at ${busy.said.y} and ${moved.said.y}, ${busy.said.height} and ${moved.said.height} tall`,
    moved.written.y === busy.written.y && moved.said.y === busy.said.y && moved.said.height === busy.said.height && moved.list.height === busy.list.height,
  );
  // Whoever asked their system for less movement is shown the dots still.
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const still = await dots();
  expect(`the dots to be still where less movement was asked for, got ${still.moving.join()}`, still.count === 3 && still.moving.every((name) => name === 'none') && !still.running);
  // Where one writes, alone, so that the line can be read in the picture.
  const near = await page.locator('.composer').boundingBox();
  await page.screenshot({ path: `${out}/carried-6-typing-still.png`, clip: { x: near.x, y: near.y - 40, width: Math.min(near.width, 640), height: near.height + 40 } });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  // And nothing moves when it has stopped.
  await page.waitForFunction(() => (document.querySelector('.typing-edge')?.textContent ?? '').trim() === '', null, { timeout: 20000 });
  const after = await placed();
  expect(`nothing to move as the writing stops: the box was at ${busy.written.y} and is at ${after.written.y}`, after.written.y === busy.written.y && after.said.y === busy.said.y);
  expect('the dots to be gone with the words', (await page.locator('.typing-edge .typing-dots').count()) === 0);
  // And so at three widths and a phone's: the line's words begin where the names do, whoever is writing.
  const usual = page.viewportSize();
  const across = async (scope, said) => {
    for (const width of [1100, 880, 390]) {
      await page.setViewportSize({ width, height: usual.height });
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
      await beginsWithNames(page, expect, scope, `${said} at ${width} wide`);
    }
    await page.setViewportSize(usual);
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  };
  await across('.channel > .split', 'in the channel');
  // Who can be named, offered after an @, and the menus of the composer's tools, whole and over everything they lie on,
  // at three widths and a phone's.
  const toolMenus = async (scope, said, called) => {
    await toolMenuWhole(page, expect, scope, /^Ask an agent or a bot/, 'Agents and bots in this channel', said, `${out}/carried-asking-${called}.png`);
    await toolMenuWhole(page, expect, scope, 'Kind of line', 'Kind of line', said, `${out}/carried-kinds-${called}.png`);
  };
  const named = async (scope, said, sizes) => {
    for (const size of sizes) {
      await page.setViewportSize(size);
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
      await offeredWhole(page, expect, scope, `${said} at ${size.width} by ${size.height}`, `${out}/carried-${said.replaceAll(' ', '-')}-${size.width}x${size.height}.png`);
      await toolMenus(scope, `${said} at ${size.width} by ${size.height}`, `${said.replaceAll(' ', '-')}-${size.width}x${size.height}`);
    }
    await page.setViewportSize(usual);
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  };
  const sizes = [usual, ...[1100, 880, 390].map((width) => ({ width, height: usual.height }))];
  await named('.channel > .split', 'named in the channel', sizes);
  // And written at length in the panel beside the channel, where it is offered under the box's tools.
  await page.locator('.channel > .split').getByRole('button', { name: 'Write in the panel at the right, where there is more room' }).click();
  await toolsAtRest(page).waitFor();
  await offeredWhole(page, expect, '.context-write', 'named in a message written at length', `${out}/carried-named-at-length.png`, true);
  await toolMenus('.context-write', 'in a message written at length', 'at-length');
  await page.getByRole('button', { name: 'Write under the channel again' }).click();
  await page.locator('.context-write').waitFor({ state: 'detached' });

  // A thread says who is writing in it the same way: over its box, beginning where the names of whoever said something
  // in it begin. The architect is asked there, and takes its time answering.
  const root = page.locator('.channel > .split .messages .message').last();
  await root.hover();
  await root.getByRole('button', { name: 'Reply in thread' }).click();
  const thread = page.locator('.thread');
  const reply = thread.getByLabel('Reply');
  await reply.waitFor();
  // A thread opened opens the panel it is read in, which comes in from the channel's edge: measured once it is at rest.
  await toolsAtRest(page).waitFor();
  const inThread = { box: '.thread .composer-box', written: '.thread .composer-box textarea', typing: '.thread .typing-edge', list: '.thread .messages' };
  const threadQuiet = await boxes(page, inThread);
  expect(`the thread's line to be there with nobody writing in it, ${sixteen} tall, over its box, got ${JSON.stringify(threadQuiet.typing)} over ${JSON.stringify(threadQuiet.box)}`, !!threadQuiet.typing && Math.abs(threadQuiet.typing.height - sixteen) <= 0.1 && threadQuiet.typing.bottom <= threadQuiet.box.y + 0.5);
  await beginsWithNames(page, expect, '.thread', 'in a thread with nobody writing');
  const rootId = await root.getAttribute('data-id');
  await cli('post', 'shop', '@shop-architect-1 what comes after the cart?', '--thread', rootId);
  await page.waitForFunction(() => /is typing/.test(document.querySelector('.thread .typing-edge')?.textContent ?? ''), null, { timeout: 30000 });
  const threadLine = page.locator('.thread .typing-edge');
  expect(`the thread's line to say who is writing there, got "${await threadLine.innerText()}"`, (await threadLine.innerText()).trim() === 'shop-architect-1 is typing…');
  // Said once, where it writes: the channel's own line does not name whoever is writing in a thread of it.
  {
    const there = await page.locator('.channel > .split .typing-edge').innerText();
    expect(`the channel's line not to name who is writing in a thread, got "${there}"`, !there.includes('shop-architect-1'));
  }
  const threadBusy = await boxes(page, inThread);
  await beginsWithNames(page, expect, '.thread', 'while somebody is writing in a thread');
  // What was asked is said in the thread too, which is taller by it, and where one writes follows it down: the line
  // keeps its place over the box and its size, and moves nothing of the box.
  const kept = (one) => ({ over: Math.round((one.box.y - one.typing.bottom) * 10) / 10, line: [one.typing.x, one.typing.width, one.typing.height], box: [one.box.x, one.box.width, one.box.height] });
  expect(
    `the thread's line to keep its place over the box as somebody began to write there, was ${JSON.stringify(kept(threadQuiet))}, is ${JSON.stringify(kept(threadBusy))}`,
    JSON.stringify(kept(threadQuiet)) === JSON.stringify(kept(threadBusy)),
  );
  await page.screenshot({ path: `${out}/carried-7-typing-thread.png` });
  {
    // On a phone's width, while it still says so.
    await page.setViewportSize({ width: 390, height: usual.height });
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    await beginsWithNames(page, expect, '.thread', 'in a thread on a phone');
    await page.screenshot({ path: `${out}/carried-8-typing-thread-phone.png` });
    await page.setViewportSize(usual);
  }
  await across('.thread', 'in a thread');
  // Named in the thread, in the panel beside the channel under its tabs: the list was cut off under them.
  // And in a window too low for it over the box, where it goes under or scrolls in itself.
  await named('.thread', 'named in a thread', [...sizes, { width: 1100, height: 420 }]);
  // A picture pasted in the thread waits over its box and over the line there, and moves neither.
  const before = await boxes(page, inThread);
  await paste(reply, { files: [{ name: 'image.png', type: 'image/png', base64: DOT }] });
  await thread.locator('.composer-over .attaching-picture').waitFor();
  const waiting = await boxes(page, { ...inThread, strip: '.thread .composer-over > .attaching' });
  expect(`what waits in the thread to lie over its line of who is writing, got ${JSON.stringify(waiting.strip)} over ${JSON.stringify(waiting.typing)}`, waiting.strip.bottom <= waiting.typing.y + 0.5 && !over(waiting.strip, waiting.typing));
  expect(`nothing in the thread to move as a picture came to wait, was ${JSON.stringify(before)}, is ${JSON.stringify(waiting)}`, ['box', 'written', 'typing'].every((name) => JSON.stringify(before[name]) === JSON.stringify(waiting[name])));
  await thread.getByRole('button', { name: /^Remove pasted-/ }).click();
  // A thread begun on a message nobody answered yet: its box is just under that message, high in the panel, with a
  // few lines between it and the panel's tabs, where the list of names was cut off under them.
  {
    const asked = page.locator('.channel > .split .messages .message', { hasText: 'what goes in the cart?' }).last();
    await asked.hover();
    await asked.getByRole('button', { name: 'Reply in thread' }).click();
    await page.waitForFunction(() => document.querySelectorAll('.thread .messages .message').length === 1);
    await toolsAtRest(page).waitFor();
    await named('.thread', 'named in a thread nobody answered yet', sizes);
  }
  // A thread's composer stands a clear step over the channel's beside it (`threadstep.mjs`).
  await threadStep({ page, expect, cli, out });

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
