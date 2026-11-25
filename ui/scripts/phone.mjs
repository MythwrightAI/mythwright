// The client on a phone: everything a person does away from their desk, done
// with a finger on a screen 390 wide, and the most used of it again at 412.
//
//   DAEMON=../target/debug/factory node scripts/phone.mjs [out-dir]
//
// A project with a board, and a channel with agents, a long message, things
// attached and a thread, are made first. Then each screen is gone to as a
// person would, by touching, a picture is taken of it, and what must stay
// true of it is checked: nothing scrolls the page sideways, what is used
// most is large enough to touch, and whatever lies over the page has a way
// back. The environment variables are those of walkthrough.mjs, and WIDTH
// to go round at one width only.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { agentsPage, linesCrowded, themeCrowded } from './apart.mjs';
import { drawerAtRest, toolsAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { gearLine, menuOf, openGear } from './gear.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'phone-data');
const home = join(out, 'phone-home');
// The project's folder is one that was said, so that its Repo is listed under it: a folder of its own, beside the data.
const shop = join(out, 'phone-shop');

/** What is there before anyone looks: made by asking the daemon, as the page that is signed in. */
async function furnish(page) {
  await page.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
    const ask = async (method, path, body) => {
      const answer = await fetch(`/api${path}`, { method, headers: { ...mine, 'content-type': 'application/json' }, body: JSON.stringify(body) });
      if (!answer.ok) throw new Error(`${method} ${path}: ${await answer.text()}`);
      return answer.json();
    };
    const project = (await (await fetch('/api/projects', { headers: mine })).json()).projects[0].id;
    const channel = await ask('POST', '/channels', { name: 'general', project });
    // More agents than fit across a phone, so that their row has to cope.
    for (const name of ['builder', 'reviewer', 'planner', 'tester', 'writer']) await ask('POST', `/channels/${channel.id}/agents`, { name, adapter: 'builtin', model: name === 'reviewer' ? 'careful-1' : undefined });

    const applied = await ask('POST', '/apply', { project, blueprint: 'std/board', wait: true });
    if (applied.finished?.status !== 'ok') throw new Error(`the board was not made: ${JSON.stringify(applied)}`);
    const today = new Date();
    const day = (after) => {
      const on = new Date(today.getFullYear(), today.getMonth(), today.getDate() + after);
      return `${on.getFullYear()}-${String(on.getMonth() + 1).padStart(2, '0')}-${String(on.getDate()).padStart(2, '0')}`;
    };
    const cards = [
      ['Write the returns policy', 'backlog', 'What a customer is told when something goes back, in plain words.', ['docs'], 0],
      ['Add a shopping cart that remembers what was in it when the customer comes back a week later', 'backlog', 'Kept with the account, not the browser.', ['shop', 'storefront'], 2],
      ['Choose a payment provider', 'ready', '', ['shop'], null],
      ['Photograph the autumn range', 'doing', 'Forty pieces, on the grey ground.', [], 1],
      ['Check the checkout on a phone', 'review', '', ['shop', 'phone'], null],
      ['Register the domain', 'done', '', [], null],
      ['Order the boxes', 'done', '', [], null],
      ['Write to the first forty customers', 'backlog', 'By hand, each of them.', ['post'], null],
      ['Weigh every piece for the postage table', 'backlog', '', ['post'], null],
      ['Find a second photographer', 'backlog', '', [], null],
    ];
    for (const [title, status, body, labels, due] of cards) {
      const card = await ask('POST', `/board/${project}/items`, { title, status, body });
      await ask('PATCH', `/board/items/${card.id}`, { labels, ...(due === null ? {} : { due: day(due) }), ...(status === 'doing' ? { assignee: 'agent:builder' } : {}) });
    }
    for (const kind of ['table', 'list', 'gallery', 'calendar']) await ask('POST', `/collections/${project}/board/views`, { name: kind, kind, ...(kind === 'calendar' ? { by: 'due' } : {}) });

    const say = (body, more = {}) => ask('POST', `/channels/${channel.id}/messages`, { body, attachments: [], ...more });
    const attach = async (name, media, body) => {
      const kept = await fetch(`/api/channels/${channel.id}/attachments?name=${encodeURIComponent(name)}`, { method: 'POST', headers: { ...mine, 'content-type': media }, body });
      if (!kept.ok) throw new Error(`${name} could not be attached: ${await kept.text()}`);
      return (await kept.json()).id;
    };
    for (let nth = 1; nth <= 8; nth++) await say(`Morning note ${nth}: the shop is open and nothing is on fire.`);
    const root = await say('Who is looking at the checkout this week?');
    await say('I can, once the cart is in.', { thread: root.id });
    await say('Then it is yours.', { thread: root.id });
    await say(
      [
        '# Where we are',
        'A long one, to see how it sits on a narrow screen: the storefront is up, the cart is half done, and the returns policy is still three paragraphs of **good intentions** that nobody has read aloud to a customer.',
        '- the cart keeps what was in it',
        '- the checkout does not yet take cards',
        'The address is https://shop.example.com/a/very/long/path/that/does/not/break/anywhere/at-all/and-goes-on/for-quite-a-while/index.html?with=a&query=too and this is how it is started:',
        '```\ncargo run --release --package storefront -- --listen 0.0.0.0:8080 --catalogue ./data/autumn.json --no-telemetry\n```',
      ].join('\n'),
    );
    // A picture wider than any phone, a page with a table wider still, and a file.
    const canvas = document.createElement('canvas');
    canvas.width = 1400;
    canvas.height = 520;
    const pen = canvas.getContext('2d');
    for (let across = 0; across < 14; across++) {
      pen.fillStyle = `hsl(${across * 26} 60% ${across % 2 ? 42 : 58}%)`;
      pen.fillRect(across * 100, 0, 100, 520);
    }
    const picture = await attach('autumn-range.png', 'image/png', await new Promise((made) => canvas.toBlob(made, 'image/png')));
    const cells = Array.from({ length: 9 }, (_unused, nth) => `<td>column ${nth + 1} of the report</td>`).join('');
    const report = await attach('weekly-report.html', 'text/html', `<h2>Weekly report</h2><p>Orders by day.</p><table border="1" style="white-space:nowrap"><tr>${cells}</tr><tr>${cells}</tr></table>`);
    const notes = await attach('notes-from-the-photographer-about-the-autumn-range.txt', 'text/plain', 'grey ground, soft light');
    await say('The pictures, the report and the notes.', { attachments: [picture, report, notes] });
    // An agent answers when it is named; its message says what wrote it.
    await say('@reviewer does the checkout look right to you?');

    // The same work done two ways, so that there are measurements to set side by side.
    const trial = `blueprint: trial
description: The same work done two ways, to see which is better.
steps:
  a:
    labels: { variant: a }
    task:
      settle: { do: wait, measure: settle, with: { for: 100ms } }
  b:
    needs: []
    labels: { variant: b }
    task:
      settle: { do: wait, measure: settle, with: { for: 300ms } }
`;
    const tried = await ask('POST', '/apply', { source: trial, project, key: 'trial', wait: true });
    if (tried.finished?.status !== 'ok') throw new Error(`the trial did not run: ${JSON.stringify(tried)}`);
  });
}

/**
 * Goes round the client at one width. `whole` is every screen; otherwise
 * the ones used most. What was found wrong is added to `problems`, each
 * with the width it was found at.
 */
async function walk(browser, url, width, whole, problems) {
  const context = await browser.newContext({ viewport: { width, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  page.setDefaultTimeout(15_000);
  page.on('pageerror', (err) => problems.push(`${width}: an error in the page: ${err.message}`));
  const expect = (what, ok) => {
    if (!ok) problems.push(`${width}: expected ${what}`);
  };
  let step = 0;
  /** A picture of the screen as it is, after checking that nothing on it scrolls sideways that should not. */
  const seen = async (name) => {
    await page.waitForTimeout(250);
    const wide = await page.evaluate(() => {
      const over = [];
      if (document.documentElement.scrollWidth > window.innerWidth) over.push(`the page is ${document.documentElement.scrollWidth} wide`);
      for (const one of document.querySelectorAll('.view, .context-body, .drawer-body, .modal, .sidebar .nav, .messages')) {
        if (one.scrollWidth > one.clientWidth + 1) over.push(`.${[...one.classList].join('.')} is ${one.scrollWidth} wide in ${one.clientWidth}`);
      }
      return over;
    });
    expect(`nothing to scroll sideways at "${name}", but ${wide.join('; ')}`, wide.length === 0);
    step += 1;
    await page.screenshot({ path: `${out}/phone-${width}-${String(step).padStart(2, '0')}-${name}.png` });
    console.log(`${width}  ${step}. ${name}`);
  };
  /** Where something is, having checked that all of it is on the screen. */
  const onScreen = async (locator, what) => {
    const box = await locator.boundingBox();
    const screen = page.viewportSize();
    const ok = box && box.x >= -0.5 && box.y >= -0.5 && box.x + box.width <= screen.width + 0.5 && box.y + box.height <= screen.height + 0.5;
    expect(`${what} to be on the screen, and it is at ${JSON.stringify(box)}`, ok);
    return box ?? { x: 0, y: 0, width: 0, height: 0 };
  };
  /** Large enough for a finger, and on the screen. */
  const touchable = async (locator, what, least = 40) => {
    const box = await onScreen(locator, what);
    expect(`${what} to be at least ${least}px each way to touch, and it is ${Math.round(box.width)} by ${Math.round(box.height)}`, box.width >= least - 0.5 && box.height >= least - 0.5);
  };
  /** Whether something covers the whole screen. */
  const covers = async (locator, what) => {
    const box = await locator.boundingBox();
    const screen = page.viewportSize();
    expect(`${what} to have the whole screen, and it is at ${JSON.stringify(box)}`, box && box.x <= 0.5 && box.y <= 0.5 && box.width >= screen.width - 0.5 && box.height >= screen.height - 0.5);
  };
  const type = async (locator, text) => {
    await locator.tap();
    await page.keyboard.type(text);
  };

  // ---- signing in ------------------------------------------------------------
  await page.goto(url);
  await page.getByLabel('Email').waitFor();
  expect('the phone to be taken for something touched', await page.evaluate(() => matchMedia('(pointer: coarse)').matches));
  const field = await page.getByLabel('Email').evaluate((input) => parseFloat(getComputedStyle(input).fontSize));
  expect(`a field's letters to be at least 16px, so that touching it does not move the browser in, and they are ${field}`, field >= 16);
  await seen('sign-in');
  await type(page.getByLabel('Email'), 'ada@example.com');
  await type(page.getByLabel('Password'), 'correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().tap();
  await page.locator('.panes').waitFor();
  if (whole) await furnish(page);

  // ---- the frame: the side panel lies over the page, and gets out of the way --
  const panel = page.locator('.sidebar');
  const opener = page.getByRole('button', { name: 'Expand the side panel' });
  expect('the side panel to start put away', (await panel.count()) === 0 && !(await page.locator('.rail').isVisible()));
  await touchable(opener, 'what brings the side panel');
  await seen('start');
  await opener.tap();
  await sideSettled(page);
  await panel.waitFor();
  const nav = page.locator('.nav');
  // Every branch of the channels starts folded (`folding.mjs`): the project's, which holds `general`, is unfolded as a
  // person would, by Expand all, which is a finger's size.
  const expand = panel.getByRole('button', { name: 'Expand all', exact: true });
  await touchable(expand, 'what unfolds every branch of the channels');
  await expand.tap();
  await nav.getByText('general', { exact: true }).waitFor();
  const lying = await onScreen(panel, 'the side panel');
  expect(`some of the page to be left beside the side panel, to touch to put it away, and the panel reaches ${lying.x + lying.width} of ${width}`, lying.x + lying.width <= width - 40);
  for (const name of ['Channels', 'Resources', 'Extensions', 'Settings']) await touchable(page.locator('.rail').getByRole('button', { name, exact: true }), `${name} on the rail`);
  await touchable(page.getByRole('button', { name: 'Collapse the side panel' }), 'what puts the side panel away');
  await seen('side-panel-channels');
  // Touching the page beside it puts it away.
  await page.touchscreen.tap(width - 16, 400);
  await panel.waitFor({ state: 'detached' });
  expect('the rail to go with the side panel', !(await page.locator('.rail').isVisible()));
  // So does its own button; and it comes back showing what it showed.
  await opener.tap();
  await sideSettled(page);
  await page.locator('.rail').getByRole('button', { name: 'Extensions', exact: true }).tap();
  await page.getByRole('navigation', { name: 'Extensions' }).waitFor();
  await seen('side-panel-extensions');
  await page.getByRole('button', { name: 'Collapse the side panel' }).tap();
  await sideSettled(page);
  await panel.waitFor({ state: 'detached' });
  await opener.tap();
  await sideSettled(page);
  await page.getByRole('navigation', { name: 'Extensions' }).waitFor();
  await page.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).tap();
  // Choosing something shows it, with the panel out of its way.
  await nav.getByText('general', { exact: true }).tap();
  await page.getByRole('heading', { name: '# general' }).waitFor();
  await panel.waitFor({ state: 'detached' });

  // ---- a channel -----------------------------------------------------------------
  const channel = page.locator('.channel > .split');
  const said = channel.locator('.message');
  const box = page.getByLabel('Message');
  const send = channel.getByRole('button', { name: 'Send' });
  await said.filter({ hasText: 'does the checkout look right' }).waitFor();
  await channel.locator('.message .who.agent', { hasText: 'reviewer' }).first().waitFor();
  await seen('channel');
  await touchable(page.getByRole('tab', { name: /general/ }), 'a tab');
  await touchable(page.getByRole('tab', { name: /general/ }).getByRole('button', { name: /^Close/ }), 'what closes a tab', 30);
  const head = await onScreen(channel.locator('.view-head'), 'the head of the channel');
  expect(`the head of the channel to be a line or two, and it is ${head.height} tall`, head.height <= 96);
  for (const name of [/participants?$/, 'Expand the tools panel']) await touchable(channel.locator('.view-head').getByRole('button', { name }), `${name} in the channel's head`, 36);
  await onScreen(page.locator('.composer-box').first(), 'the box to write in');
  await touchable(send, 'what sends a message');
  await touchable(channel.getByRole('button', { name: 'Attach a file' }), 'what attaches a file');
  // Who can be asked is behind one of the tools, however many there are: the list is on the screen, each big enough to touch.
  const asking = channel.locator('.asking-button');
  const askable = page.getByRole('menu', { name: 'Agents and bots in this channel' });
  await touchable(asking, 'what lists who can be asked');
  await asking.tap();
  await onScreen(askable, 'the list of who can be asked');
  // Every agent, and after them the factory's own bot, which is asked as they are.
  const offered = await askable.getByRole('menuitem').evaluateAll((all) => all.map((one) => `${one.querySelector('.asking-bot') ? 'bot' : 'agent'}:${one.querySelector('.asked-name').textContent}`));
  expect(`every agent to be in the list, and the factory's bot after them, got ${offered}`, offered.slice(0, 5).join() === 'agent:builder,agent:reviewer,agent:planner,agent:tester,agent:writer' && offered.length === 6 && offered[5].startsWith('bot:'));
  await touchable(askable.getByRole('menuitem').first(), 'one of those who can be asked');
  await seen('channel-asking');
  await asking.tap();
  await askable.waitFor({ state: 'detached' });
  expect('the box not to have the caret before it is touched, which would bring the keyboard up', await page.evaluate(() => document.activeElement?.tagName !== 'TEXTAREA'));
  const newestAboveBox = async (what) => {
    const last = await said.last().boundingBox();
    const list = await channel.locator('.messages').boundingBox();
    const written = await channel.locator('.composer').boundingBox();
    expect(`${what} to be seen whole above the box (it is at ${JSON.stringify(last)}, the messages at ${JSON.stringify(list)}, the box at ${JSON.stringify(written)})`, last.y >= list.y - 1 && last.y + last.height <= list.y + list.height + 1 && last.y + last.height <= written.y + 1);
  };
  await newestAboveBox('the newest message');

  // Writing and sending. Under a finger Enter ends a line, and the button sends.
  const mine = `Sent from a phone ${width} wide`;
  await type(box, mine);
  await page.keyboard.press('Enter');
  await page.keyboard.type('on two lines');
  expect(`Enter to end a line and not to send, and the box holds ${JSON.stringify(await box.inputValue())}`, (await box.inputValue()) === `${mine}\non two lines`);
  await touchable(send, 'what sends, with something written');
  await seen('channel-writing');
  await send.tap();
  const sent = said.filter({ hasText: mine });
  await sent.waitFor();
  expect('the box to be empty once it is sent', (await box.inputValue()) === '');
  expect('the caret to stay in the box once it is sent, so that the keyboard stays up for the next', await box.evaluate((area) => document.activeElement === area));
  await newestAboveBox('what was just sent');
  // The tools are one line that scrolls; the last of them is reached by scrolling it, and what sends stays.
  const tools = channel.getByRole('toolbar', { name: 'Formatting' });
  expect('the tools to scroll within their line', await tools.evaluate((row) => row.scrollWidth > row.clientWidth));
  await tools.getByRole('button', { name: 'Block of code' }).scrollIntoViewIfNeeded();
  await touchable(tools.getByRole('button', { name: 'Block of code' }), 'the last of the tools', 36);
  await touchable(send, 'what sends, with the tools scrolled');
  await tools.evaluate((row) => (row.scrollLeft = 0));
  await tools.getByRole('button', { name: 'Kind of line' }).tap();
  await onScreen(page.getByRole('menu').first(), 'the menu of kinds of line');
  await seen('channel-tools-menu');
  await page.getByRole('menuitem', { name: 'Text', exact: true }).tap();
  await box.evaluate((area) => area.blur());

  // A long message, and what is attached, are no wider than the screen.
  const long = said.filter({ hasText: 'Where we are' });
  await long.scrollIntoViewIfNeeded();
  for (const part of ['.body', '.code-box']) {
    const piece = await long.locator(part).first().boundingBox();
    expect(`${part} of a long message to end before the edge of the screen, and it runs from ${piece.x} to ${piece.x + piece.width}`, piece.x >= 0 && piece.x + piece.width <= width);
  }
  await seen('channel-long-message');
  const with_ = said.filter({ hasText: 'The pictures, the report and the notes.' });
  await with_.locator('.attached-picture img').scrollIntoViewIfNeeded();
  for (const part of ['.attached-picture', '.attached-view', '.attached-file']) {
    const piece = await with_.locator(part).first().boundingBox();
    expect(`${part} to be no wider than the screen, and it runs from ${piece.x} to ${piece.x + piece.width}`, piece.x >= 0 && piece.x + piece.width <= width);
  }
  await seen('channel-attached');
  if (whole) {
    await with_.locator('.attached-picture').tap();
    const opened = page.getByRole('dialog');
    await onScreen(opened, 'a picture opened larger');
    await seen('channel-picture-opened');
    await touchable(opened.getByRole('button', { name: 'Close' }), 'what closes a picture');
    await opened.getByRole('button', { name: 'Close' }).tap();
    await opened.waitFor({ state: 'detached' });
  }

  // Who someone is: a picture touched beside what was said opens them where everybody of the channel is read about,
  // which has the whole screen here. No card is shown over the channel for it: the picture is pressed, and a card
  // would lie under the finger. The name is words: touched, it is a touch on the message, and opens nobody.
  const beside = page.getByRole('complementary', { name: 'Tools' });
  const back = page.getByRole('button', { name: 'Back to the channel' });
  const crumbs = beside.getByRole('navigation', { name: 'Where this is' });
  // The last of the messages that says who wrote it: one that follows another of theirs does not say it again.
  const name = said.locator('.who', { hasText: 'Ada' }).last();
  await name.scrollIntoViewIfNeeded();
  await name.tap();
  await page.waitForTimeout(300);
  expect('a name touched to open nobody and lay no card', (await beside.count()) === 0 && (await page.locator('.who-card').count()) === 0);
  // Touched again, the message is as it was.
  await name.tap();
  await said.filter({ has: page.locator('.who', { hasText: 'Ada' }) }).last().locator('.avatar').tap();
  await beside.locator('.context-about').waitFor();
  // It has come in from the screen's edge, as the panel it is read in does.
  await toolsAtRest(page).waitFor();
  await covers(beside, 'what is said about a person');
  const person = (await beside.locator('.context-about').innerText()).replace(/\s+/g, ' ');
  expect(`what is read about a person to say who it is, and that it is whoever is signed in, got "${person.slice(0, 120)}"`, person.startsWith('Ada person · you'));
  expect('no card to lie over the channel for a picture that was touched', (await page.locator('.who-card').count()) === 0);
  expect(`the way back to say where this is, got "${(await crumbs.innerText()).replace(/\s+/g, ' ')}"`, (await crumbs.innerText()).replace(/\s+/g, ' ') === 'Channel › Ada');
  await touchable(back, 'the way back to the channel from somebody read about');
  await seen('context-person');
  // From there, everybody of the channel: people, then agents, then bots, each saying which; and any of them read about.
  await crumbs.getByRole('button', { name: 'Channel', exact: true }).tap();
  await beside.locator('.context-agent').first().waitFor();
  const everybody = await beside.locator('button.context-member').evaluateAll((all) => all.map((one) => `${one.querySelector('.context-name').textContent}:${one.dataset.kind}`));
  expect(`everybody of the channel to be listed, people first, then agents, then bots, got ${everybody}`, everybody.slice(0, 6).join() === 'Ada:person,builder:agent,reviewer:agent,planner:agent,tester:agent,writer:agent' && everybody.slice(6).every((one) => one.endsWith(':bot')));
  // The list is under the channel's Details, in a panel that scrolls: each is brought into view as a finger would, then measured.
  for (const one of await beside.locator('button.context-member').all()) {
    await one.scrollIntoViewIfNeeded();
    await touchable(one, 'somebody in the list of members', 36);
  }
  // Each a card with their picture, and everything of it inside the card and the card on the screen.
  const outside = await beside.locator('.context-member').evaluateAll((all) =>
    all
      .filter((card) => {
        const at = card.getBoundingClientRect();
        const parts = [...card.querySelectorAll('*')].filter((one) => one.getBoundingClientRect().width > 0);
        return !card.querySelector(':scope > .avatar') || at.left < -0.5 || at.right > innerWidth + 0.5 || parts.some((one) => one.getBoundingClientRect().left < at.left - 0.5 || one.getBoundingClientRect().right > at.right + 0.5);
      })
      .map((card) => card.querySelector('.context-name')?.textContent),
  );
  expect(`everybody a card with their picture, wholly inside it and on the screen, got ${outside}`, outside.length === 0);
  await seen('context-everybody');
  await back.tap();
  await beside.waitFor({ state: 'detached' });
  await page.getByRole('heading', { name: '# general' }).waitFor();
  // Touching one of those who can be asked begins asking it.
  await asking.tap();
  await askable.getByRole('menuitem', { name: /builder/ }).tap();
  expect(`touching an agent in the list to begin asking it, and the box holds ${JSON.stringify(await box.inputValue())}`, (await box.inputValue()) === '@builder ');
  await box.fill('');
  await box.evaluate((area) => area.blur());

  // ---- a thread: the whole screen, and a way back -----------------------------------
  const thread = page.locator('.thread');
  // Touching a message shows what can be done with it; nothing has to be pointed at.
  expect('nothing to be offered on a message that was not touched', !(await sent.getByRole('button', { name: 'Reply in thread' }).isVisible()));
  await sent.locator('.body').tap();
  const reply = sent.getByRole('button', { name: 'Reply in thread' });
  await reply.waitFor();
  await touchable(reply, 'what begins a thread', 36);
  await seen('channel-message-touched');
  await reply.tap();
  await thread.waitFor();
  // It has come in from the screen's edge, as the panel it is read in does.
  await toolsAtRest(page).waitFor();
  await covers(beside, 'a thread');
  await touchable(back, 'the way back from a thread');
  await thread.locator('.thread-count', { hasText: '0 replies' }).waitFor();
  await type(thread.getByLabel('Reply'), 'and a reply under it');
  await touchable(thread.getByRole('button', { name: 'Send' }), 'what sends a reply');
  await thread.getByRole('button', { name: 'Send' }).tap();
  await thread.locator('.thread-count', { hasText: '1 reply' }).waitFor();
  await onScreen(thread.locator('.composer'), 'the box to reply in');
  await seen('thread');
  await back.tap();
  await beside.waitFor({ state: 'detached' });
  await page.getByRole('heading', { name: '# general' }).waitFor();
  // A thread that has something under it is opened from what says how much.
  const asked = said.filter({ hasText: 'Who is looking at the checkout this week?' });
  await asked.getByRole('button', { name: '2 replies' }).scrollIntoViewIfNeeded();
  await asked.getByRole('button', { name: '2 replies' }).tap();
  await thread.locator('.message', { hasText: 'Then it is yours.' }).waitFor();
  // It has come in from the screen's edge, as the panel it is read in does.
  await toolsAtRest(page).waitFor();
  await covers(beside, 'a thread opened from its replies');
  await seen('thread-of-two');
  // Closing the thread leaves the panel it was read in; the way back leaves that.
  await touchable(beside.getByRole('button', { name: 'Close the thread' }), 'what closes a thread', 28);
  await beside.getByRole('button', { name: 'Close the thread' }).tap();
  await thread.waitFor({ state: 'detached' });
  await back.tap();
  await beside.waitFor({ state: 'detached' });

  // ---- the tools panel: the whole screen, its tabs, and a way back ---------------------
  await channel.getByRole('button', { name: 'Expand the tools panel' }).tap();
  await toolsAtRest(page).waitFor();
  await beside.waitFor();
  await covers(beside, 'the tools panel');
  // Its tabs are one row that scrolls within itself: the last of them is reached by scrolling it, as a finger does.
  for (const tab of ['Media', 'Channel', 'Board', 'Docs']) {
    await beside.getByRole('tab', { name: tab }).scrollIntoViewIfNeeded();
    await touchable(beside.getByRole('tab', { name: tab }), `the ${tab} tab of the tools panel`, 36);
    // Touched on its name, as a finger does: the cross that takes a tab away is at its end, a finger's size too, and
    // the middle of a short tab is near it.
    await beside.getByRole('tab', { name: tab }).tap({ position: { x: 20, y: 18 } });
    if (tab === 'Media') await beside.locator('.context-pictures img').first().waitFor();
    if (tab === 'Channel') await beside.locator('.context-agent').first().waitFor();
    if (tab === 'Board') await beside.getByText('Choose a payment provider').waitFor();
    if (tab === 'Docs') await beside.locator('.channel-docs').waitFor();
    await seen(`context-${tab.toLowerCase().replace(' ', '-')}`);
  }
  // A card is read there, and left by its own way back: on the Board tab, which is not the last of the usual tabs.
  await beside.getByRole('tab', { name: 'Board' }).scrollIntoViewIfNeeded();
  await beside.getByRole('tab', { name: 'Board' }).tap({ position: { x: 20, y: 18 } });
  await beside.getByText('Choose a payment provider').tap();
  // The card's own form, as the drawer beside its board has it.
  await beside.locator('form.card-form').getByLabel('Title', { exact: true }).waitFor();
  await seen('context-card');
  await touchable(beside.getByRole('button', { name: 'Back to the board' }), 'the way back from a card in the panel');
  await beside.getByRole('button', { name: 'Back to the board' }).tap();
  if (whole) {
    // The usual tabs are every tab there is: one taken away is offered again by the `+`.
    await beside.getByRole('tab', { name: 'Docs' }).scrollIntoViewIfNeeded();
    await beside.getByRole('button', { name: 'Remove the Docs tab' }).tap();
    await beside.getByRole('button', { name: 'Add a tab' }).tap();
    await onScreen(beside.getByRole('menu'), 'the menu of tabs to add');
    await seen('context-add-a-tab');
    await beside.getByRole('menuitem', { name: /Docs/ }).tap();
    await beside.getByRole('tab', { name: 'Docs', selected: true }).waitFor();
  }
  await back.tap();
  await beside.waitFor({ state: 'detached' });
  // An agent's picture, touched, is read about there.
  const reviewer = said.filter({ has: page.locator('.who.agent', { hasText: 'reviewer' }) }).first().locator('.avatar');
  await reviewer.scrollIntoViewIfNeeded();
  await reviewer.tap();
  await beside.locator('.context-about').waitFor();
  // It has come in from the screen's edge, as the panel it is read in does.
  await toolsAtRest(page).waitFor();
  await covers(beside, 'what is said about an agent');
  expect('what an agent runs on to be said', (await beside.locator('.context-about').innerText()).includes('careful-1'));
  await seen('context-agent');
  await crumbs.getByRole('button', { name: 'Channel', exact: true }).tap();
  await back.tap();
  await beside.waitFor({ state: 'detached' });

  // Who is in the channel, and adding to them: a menu and a dialog that fit. The list ends in the line that adds.
  await channel.getByRole('button', { name: /participants?$/ }).tap();
  const people = page.getByRole('menu', { name: 'Participants' });
  await onScreen(people, 'the list of participants');
  await seen('channel-participants');
  await channel.locator('.view-head h1').tap();
  await people.waitFor({ state: 'detached' });
  await channel.getByRole('button', { name: /participants?$/ }).tap();
  await touchable(people.locator('.people-add'), 'the line that adds to the channel');
  await people.locator('.people-add').tap();
  const dialog = page.getByRole('dialog');
  await dialog.waitFor();
  await dialog.getByRole('radio', { name: /^An agent/ }).tap();
  await onScreen(dialog, 'the dialog that adds an agent');
  await seen('dialog-add-agent');
  // Its buttons are reached, by scrolling the dialog where it is taller than the screen.
  for (const name of ['Cancel', 'Add agent']) {
    await dialog.getByRole('button', { name, exact: true }).scrollIntoViewIfNeeded();
    await touchable(dialog.getByRole('button', { name, exact: true }), `${name} in the dialog`, 34);
  }
  await seen('dialog-add-agent-buttons');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).tap();
  await dialog.waitFor({ state: 'detached' });

  // ---- the board -------------------------------------------------------------------------
  const resources = async (...path) => {
    await opener.tap();
    await sideSettled(page);
    // It comes back showing what it last showed; touching that on the rail would put it away again.
    const rail = page.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
    if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.tap();
    await page.getByRole('navigation', { name: 'Resources' }).waitFor();
    // Everything the Resources panel lists starts folded (`folded.mjs`): its parts are unfolded and Shop in them, as a
    // person would, and nothing more, since every blueprint unfolded would put Shop's lines far down the panel.
    for (const name of ['Projects', 'Factory', 'Blueprints', 'Shop']) {
      const fold = nav.getByRole('button', { name: `Unfold ${name}`, exact: true });
      if ((await fold.count()) > 0) await fold.tap();
    }
    for (const name of path) {
      const item = nav.getByText(name, { exact: true });
      // A branch that is already unfolded is left so.
      if ((await item.count()) === 0) continue;
      const fold = nav.locator('button.nav-item', { has: page.getByText(name, { exact: true }) }).first();
      if ((await fold.getAttribute('aria-expanded')) !== 'true') await item.first().tap();
    }
  };
  await resources();
  await touchable(nav.locator('button.nav-item', { hasText: 'Board' }), 'something in the side panel', 38);
  await seen('side-panel-resources');
  await nav.getByText('Board', { exact: true }).tap();
  await panel.waitFor({ state: 'detached' });
  const board = page.locator('.board');
  const column = (called) => board.locator('.column', { has: page.locator('.column-head', { hasText: called }) });
  await column('backlog').locator('.card').first().waitFor();
  await seen('board');
  // Its columns are reached by scrolling it, not the page.
  expect('the board to scroll within itself', await board.evaluate((all) => all.scrollWidth > all.clientWidth));
  await board.evaluate((all) => (all.scrollLeft = all.scrollWidth));
  await onScreen(column('done'), 'the last column, with the board scrolled to it');
  await seen('board-last-column');
  await board.evaluate((all) => (all.scrollLeft = 0));
  // A card is opened, read with the whole screen, and closed.
  const drawer = drawerAtRest(page);
  const moving = 'Choose a payment provider';
  await column('ready').locator('.card', { hasText: moving }).scrollIntoViewIfNeeded();
  await column('ready').locator('.card', { hasText: moving }).tap();
  await drawer.waitFor();
  await covers(drawer, 'a card');
  await touchable(drawer.getByRole('button', { name: 'Close' }), 'what closes a card');
  await seen('card');
  await drawer.getByRole('button', { name: 'Close' }).tap();
  await drawer.waitFor({ state: 'detached' });
  // It is moved to another column by saying which, without being carried there; and back.
  for (const [from, to] of [['ready', 'doing'], ['doing', 'ready']]) {
    await column(from).locator('.card', { hasText: moving }).scrollIntoViewIfNeeded();
    await column(from).locator('.card', { hasText: moving }).tap();
    expect(`the card to say it is in ${from}`, (await drawer.getByLabel('Column').inputValue()) === from);
    await drawer.getByLabel('Column').selectOption(to);
    await drawer.getByRole('button', { name: 'Save' }).scrollIntoViewIfNeeded();
    await touchable(drawer.getByRole('button', { name: 'Save' }), 'what saves a card', 34);
    await drawer.getByRole('button', { name: 'Save' }).tap();
    await drawer.waitFor({ state: 'detached' });
    await column(to).locator('.card', { hasText: moving }).waitFor();
    expect(`the card to have left ${from}`, (await column(from).locator('.card', { hasText: moving }).count()) === 0);
    if (to === 'doing') {
      await column(to).locator('.card', { hasText: moving }).scrollIntoViewIfNeeded();
      await seen('card-moved');
    }
  }
  // What can be done to a board, and how it is laid out, are lines of the gear of its head, and what they open fits.
  await onScreen(await openGear(page, 'tap'), 'the board\'s gear, with its tools');
  await seen('board-tools');
  await page.locator('.view-head h1').tap();
  await menuOf(page).waitFor({ state: 'detached' });
  if (whole) {
    await gearLine(page, /^Layout/, 'tap');
    await onScreen(page.getByRole('dialog', { name: 'Layout' }), 'the menu of layouts');
    await seen('board-layout-menu');
    await page.locator('.view-head h1').tap();
    await page.getByRole('dialog', { name: 'Layout' }).waitFor({ state: 'detached' });
  }

  // The same cards laid out other ways: each read, each scrolling within itself.
  const views = page.getByRole('tablist', { name: 'Views' });
  const view = async (called) => {
    await views.getByRole('tab', { name: called, exact: true }).scrollIntoViewIfNeeded();
    await touchable(views.getByRole('tab', { name: called, exact: true }), `the ${called} view's tab`, 38);
    await views.getByRole('tab', { name: called, exact: true }).tap();
  };
  await view('Table');
  const grid = page.locator('.grid-scroll');
  await grid.locator('tbody tr').first().waitFor();
  expect('the table to scroll within itself', await grid.evaluate((all) => all.scrollWidth > all.clientWidth));
  await seen('board-as-table');
  // A row is opened by a button that is there without the row being pointed at.
  const row = grid.locator('tbody tr', { hasText: moving });
  await touchable(row.getByRole('button', { name: /^Open/ }), 'what opens a row of the table', 26);
  await row.getByRole('button', { name: /^Open/ }).tap();
  await drawer.waitFor();
  await drawer.getByRole('button', { name: 'Close' }).tap();
  await grid.evaluate((all) => (all.scrollLeft = all.scrollWidth));
  await seen('board-as-table-scrolled');
  await view('List');
  const lines = page.locator('.rows-line');
  await lines.first().waitFor();
  for (const line of await lines.all()) {
    const at = await line.boundingBox();
    expect(`a line of the list to be within the screen, and it runs from ${at.x} to ${at.x + at.width}`, at.x >= 0 && at.x + at.width <= width);
  }
  expect('a long name in the list to be read in full', await lines.filter({ hasText: 'when the customer comes back a week later' }).locator('.truncate').evaluate((words) => words.scrollWidth <= words.clientWidth));
  await seen('board-as-list');
  await lines.filter({ hasText: moving }).tap();
  await drawer.waitFor();
  await drawer.getByRole('button', { name: 'Close' }).tap();
  await view('Gallery');
  await page.locator('.rows-card').first().waitFor();
  expect('the gallery to scroll down within itself', await page.locator('.rows-page').evaluate((all) => all.scrollHeight > all.clientHeight && all.scrollWidth <= all.clientWidth));
  await seen('board-as-gallery');
  await view('Calendar');
  const month = page.locator('.cal');
  await month.locator('.cal-card').first().waitFor();
  expect('the calendar to scroll within itself', await month.evaluate((all) => all.scrollWidth > all.clientWidth));
  await onScreen(month.getByRole('button', { name: 'The month after' }), 'what turns the month');
  await seen('board-as-calendar');
  // A card on it is opened, and is given another day there: nothing is carried.
  await month.locator('.cal-day .cal-card').first().scrollIntoViewIfNeeded();
  await month.locator('.cal-day .cal-card').first().tap();
  await drawer.waitFor();
  expect('a card on the calendar to have a day that can be changed', (await drawer.getByLabel('Due').count()) === 1);
  await drawer.getByRole('button', { name: 'Close' }).tap();
  await view('Board');
  await board.waitFor();

  // ---- measurements ------------------------------------------------------------------------
  await resources();
  await nav.getByText('Measurements', { exact: true }).tap();
  await page.locator('.tile').first().waitFor();
  await page.waitForTimeout(500);
  await seen('measurements');
  // Set side by side they are wider than any phone, and scroll inside their own box.
  await page.getByLabel('Compare by').selectOption('variant');
  const compared = page.locator('.compare-scroll');
  await compared.locator('tbody tr').first().waitFor();
  expect('measurements side by side to scroll within their box', await compared.evaluate((all) => all.scrollWidth > all.clientWidth));
  await compared.scrollIntoViewIfNeeded();
  await seen('measurements-side-by-side');
  await page.getByLabel('Compare by').selectOption('');
  if (whole) {
    for (const table of await page.locator('.view-body .panel').all()) await table.scrollIntoViewIfNeeded();
    await seen('measurements-further-down');
  }

  // ---- settings ----------------------------------------------------------------------------
  await opener.tap();
  await sideSettled(page);
  await page.locator('.rail').getByRole('button', { name: 'Settings', exact: true }).tap();
  await panel.waitFor({ state: 'detached' });
  await page.getByRole('heading', { name: 'Plan' }).waitFor();
  await seen('settings');
  const crowdedTheme = await themeCrowded(page);
  expect(`nothing of the Theme panel over anything else of it, got ${JSON.stringify(crowdedTheme)}`, crowdedTheme.length === 0);
  const themes = page.getByRole('radiogroup', { name: 'Theme' });
  // Each is reached by scrolling, as what is under a theme (saving it as a file) makes the panel longer than a screen.
  for (const one of await themes.getByRole('radio').all()) {
    await one.evaluate((box) => box.scrollIntoView({ block: 'center' }));
    await touchable(one, 'a theme to choose', 60);
  }
  await themes.getByRole('radio', { name: /Tokyo Midnight/ }).tap();
  expect('touching a theme to put the page in it', (await page.evaluate(() => document.documentElement.dataset.theme)) === 'tokyo-midnight');
  await seen('settings-theme-chosen');
  await themes.getByRole('radio', { name: /Grid Iron/ }).tap();
  if (whole) {
    await page.locator('.view').evaluate((all) => (all.scrollTop = all.scrollHeight));
    await seen('settings-foot');
  }

  // ---- the agents, a page of their own under Factory, of every workspace --------------------------
  await agentsPage(page, out, 'tap');
  // The agents are a line each, three lines on a phone, and as wide as the screen: by the boxes, no part of a line over another or past it,
  // no line past the screen's sides, and nothing that scrolls sideways.
  const agents = page.locator('.agents-listed');
  await agents.locator('.agent-line[data-agent="writer"]').scrollIntoViewIfNeeded();
  const crowded = await linesCrowded(page, '.agent-line', ['.agent-name', '.agent-runs', '.agent-doing', '.agent-may', '.agent-acts']);
  const agentLines = await agents.locator('.agent-line').count();
  if (agentLines < 5) crowded.push(`only ${agentLines} lines`);
  expect(`nothing of the agents' lines over anything else, past its line or scrolling sideways, got ${JSON.stringify(crowded)}`, crowded.length === 0);
  for (const more of await agents.getByRole('button', { name: /^More that can be done with / }).all()) {
    await more.scrollIntoViewIfNeeded();
    await touchable(more, "what opens the menu of an agent's line", 34);
  }
  await agents.locator('.agent-line[data-agent="writer"]').scrollIntoViewIfNeeded();
  await seen('agents');

  if (whole) {
    // ---- the rest: not what a phone is for, but none of it broken -------------------------------
    await resources();
    await nav.getByText('Activity', { exact: true }).tap();
    await page.locator('table tbody tr').first().waitFor();
    await seen('activity');
    // What is running, and one run of it, are read from there.
    await page.locator('tr.clickable', { hasText: 'std/board' }).first().tap();
    await page.locator('.tab[aria-selected="true"]', { hasText: '◈' }).waitFor();
    await page.waitForTimeout(500);
    await seen('instance');
    await page.getByRole('tab', { name: /^Runs/ }).tap();
    await page.locator('.view table tbody tr').first().waitFor();
    await seen('instance-runs');
    await page.getByRole('tab', { name: 'History' }).tap();
    await page.locator('.history-row').first().waitFor();
    await seen('instance-history');
    await resources();
    await nav.getByText('Shop', { exact: true }).tap();
    await page.locator('.tile').first().waitFor();
    await seen('project');
    await page.locator('.view').evaluate((all) => (all.scrollTop = all.scrollHeight));
    await seen('project-foot');

    // What a blueprint is applied with is asked in the drawer, which has the screen here: it fits, and its buttons are reached.
    await resources('std');
    await nav.getByText('std/board', { exact: true }).tap();
    // A blueprint that ships opens on what it says in plain words.
    await page.locator('.summary[data-of="std/board"]').waitFor();
    await seen('blueprint');
    await page.getByRole('button', { name: /^Apply/ }).tap();
    const applying = drawerAtRest(page);
    await applying.locator('.applying').waitFor();
    await onScreen(applying, 'the drawer that applies a blueprint');
    await seen('drawer-apply');
    for (const name of ['Not now', 'Apply']) {
      await applying.getByRole('button', { name, exact: true }).scrollIntoViewIfNeeded();
      await touchable(applying.getByRole('button', { name, exact: true }), `${name} in the drawer`, 34);
    }
    await applying.getByRole('button', { name: 'Not now', exact: true }).tap();
    await applying.waitFor({ state: 'detached' });
    await page.getByRole('tab', { name: 'Node' }).tap();
    await page.locator('.gnode').first().waitFor();
    await seen('blueprint-as-graph');

    // A project's files, and one of them.
    await resources();
    await nav.getByText('Repo', { exact: true }).tap();
    await page.locator('.view tr.clickable').first().waitFor();
    await seen('files');
    await page.locator('.view tr.clickable', { hasText: 'README.md' }).tap();
    await page.waitForTimeout(600);
    await seen('file');

    // An extension, and who is signed in.
    await opener.tap();
    await sideSettled(page);
    await page.locator('.rail').getByRole('button', { name: 'Extensions', exact: true }).tap();
    await page.locator('.ext-open').first().waitFor();
    // The page of a package that is installed here, with its program on this device, asks that program which
    // models it offers. This walk starts nobody's program: a factory made a moment ago has no package installed,
    // and where one is all the same, no package's page is opened, and that is said.
    const asked = await page.evaluate(async () => {
      const read = await fetch('/api/extensions', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
      return ((await read.json()).extensions ?? []).filter((one) => one.agent && one.available && one.program).map((one) => one.name);
    });
    if (asked.length > 0) {
      console.log(`${width}  no package's page opened: ${asked.join(', ')} is installed here with its program, which the page would ask for its models`);
      await seen('extensions-listed');
      await page.getByRole('button', { name: 'Collapse the side panel' }).tap();
      await sideSettled(page);
    } else {
      await page.locator('.ext-open').first().tap();
      await panel.waitFor({ state: 'detached' });
      await page.waitForTimeout(400);
      // Not installed, its page says how it would be, and asks nothing of anything.
      expect('the page of a package that is not installed to ask its program nothing', (await page.getByText('Install it to see which models it offers.').count()) === 1);
      await seen('extension');
    }
    await opener.tap();
    await sideSettled(page);
    await page.locator('.rail').getByRole('button', { name: /^You:/ }).tap();
    await panel.waitFor({ state: 'detached' });
    await page.getByRole('heading', { name: 'You' }).waitFor();
    await seen('profile');

    // Searching: from the panel's one box, which offers what is typed to be searched for everywhere; a dialog with
    // that at its head and a way out of it.
    await opener.tap();
    await sideSettled(page);
    const talk = page.locator('.rail').getByRole('button', { name: 'Channels', exact: true });
    if ((await talk.getAttribute('aria-pressed')) !== 'true') await talk.tap();
    await type(page.getByLabel('Search these channels', { exact: true }), 'checkout');
    await page.getByRole('button', { name: 'Search everything for “checkout”', exact: true }).tap();
    expect('the search to open with what was typed in the box', (await page.getByLabel('Search', { exact: true }).inputValue()) === 'checkout');
    await page.locator('.hit').first().waitFor();
    await onScreen(dialog, 'the search');
    await seen('search');
    await touchable(dialog.getByRole('button', { name: 'Close' }), 'what closes the search');
    // What was found may be a card, which is shown in the drawer: what it is is read before it is touched, and the drawer waited for,
    // since a drawer comes in from the edge and is not there the moment the search is put away.
    const card = (await page.locator('.hit').first().locator('.badge').innerText()).trim() === 'card';
    await page.locator('.hit').first().tap();
    await dialog.waitFor({ state: 'detached' });
    await panel.waitFor({ state: 'detached' });
    if (card) await drawer.waitFor();
    await seen('search-went-there');
    if (card) {
      await drawer.getByRole('button', { name: 'Close' }).tap();
      await page.locator('.drawer').waitFor({ state: 'detached' });
    }

    // Every tab opened so far is in one row, scrolled with a finger, with the one showing in sight.
    const strip = page.getByRole('tablist').first();
    expect('the tabs to be one row that scrolls', (await page.locator('.tabs').count()) === 1 && (await strip.evaluate((row) => row.scrollWidth > row.clientWidth)));
    await onScreen(page.locator('.tab[aria-selected="true"]'), 'the tab that is showing');
    await page.getByRole('tab', { name: /general/ }).scrollIntoViewIfNeeded();
    await page.getByRole('tab', { name: /general/ }).tap();
    await page.getByRole('heading', { name: '# general' }).waitFor();

    // Held on its side the phone is short, not narrow: it is still a phone.
    await page.setViewportSize({ width: 844, height: 390 });
    await page.waitForTimeout(300);
    await onScreen(page.locator('.composer-box').first(), 'the box to write in, with the phone on its side');
    await onScreen(send, 'what sends, with the phone on its side');
    expect('no side panel beside the page with the phone on its side', (await panel.count()) === 0);
    await seen('on-its-side');
    await page.setViewportSize({ width, height: 844 });
  }
  await context.close();
}

await mkdir(out, { recursive: true });
// The pictures are numbered as they are taken: those of a walk that took other ones would be left among these.
for (const old of await readdir(out)) if (/^phone-\d+-\d+-.*\.png$/.test(old)) await rm(join(out, old));
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
const problems = [];
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
  // An iPhone's width, with everything; then an Android phone's, with what is used most.
  const only = process.env.WIDTH ? [Number(process.env.WIDTH)] : [390, 412];
  for (const [nth, width] of only.entries()) await walk(browser, url, width, nth === 0, problems);
  await browser.close();
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
}
if (failed) console.error(failed);
if (problems.length > 0) console.error(`\n${problems.length} thing(s) wrong on a phone:\n${problems.join('\n')}`);
if (failed || problems.length > 0) process.exit(1);
console.log(`\nnothing wrong on a phone; pictures are in ${out}`);
