// A channel with all three kinds of member: a person, an agent, a bot.
//
//   DAEMON=../target/debug/factory node scripts/channel.mjs [out-dir]
//
// Each is added from the one menu, a mention is typed with the list that is
// offered for it, the bot answers by itself, and an agent's message says
// which model wrote it; what is attached, and what a paste brings, is
// `attaching.mjs`, and who is in it and what it is for are `participants.mjs`
// and `topic.mjs`, each of which this runs. The environment variables are
// those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { toolsAtRest, sideSettled } from './drawer.mjs';
import { attaching } from './attaching.mjs';
import { participants } from './participants.mjs';
import { failedRun } from './failedrun.mjs';
import { topic } from './topic.mjs';
import { updates } from './updates.mjs';
import { addLine, addingDialog, openAdding } from './adding.mjs';
import { endsWithThis } from './ends.mjs';
import { sized } from './looked.ts';
import { saysWhere } from './where.ts';

/** Turns the side panel to a view on the rail, unless it is showing already. */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
}

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'channel-data');
const home = join(out, 'channel-home');

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
  // As wide for its text as a window of 1280 is at the usual size: the channel's head and the tools panel's tabs are
  // held to be on one line beside each other.
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
  await side(page, 'Channels');
  await page.getByRole('button', { name: 'New channel' }).click();
  await page.getByLabel('Name').fill('general');
  await page.getByRole('button', { name: 'Create', exact: true }).click();
  await page.getByRole('heading', { name: '# general' }).waitFor();
  // Who can be asked is behind one button among the composer's tools: the list it opens, and the list put away again.
  const asking = page.locator('.channel > .split .asking-button');
  const askable = page.getByRole('menu', { name: 'Agents and bots in this channel' });
  const counted = (count) => page.getByRole('button', { name: new RegExp(`${count} participants$`) });
  const badges = async () => {
    await asking.click();
    await askable.waitFor();
    const names = await askable.getByRole('menuitem').locator('strong').allTextContents();
    // Put away by the button that opened it. Escape does it too, a moment after the list is drawn: sooner than that is a script, not a person.
    await asking.click();
    await askable.waitFor({ state: 'detached' });
    return names;
  };
  // One line at the foot of who is in the channel, one dialog, and who it is chosen at its head: an agent, a person or a
  // bot. The head has no button of its own for it (`adding.mjs`).
  const adding = addingDialog(page);
  const add = async (who) => {
    await openAdding(page);
    await adding.getByRole('radio', { name: new RegExp(`^${who}`) }).check();
  };

  expect('no button in the head that adds to the channel', (await page.locator('.channel > .split > .view-head').getByRole('button', { name: /^Add/ }).count()) === 0);
  await page.locator('.channel > .split > .view-head').getByRole('button', { name: /participants?$/ }).click();
  expect(`the line to name all three that the owner may add, got "${await addLine(page).innerText()}"`, (await addLine(page).innerText()).trim() === 'Add a person, agent or bot');
  await addLine(page).click();
  await adding.waitFor();
  expect('no menu left between the line and the dialog', (await page.getByRole('menu').count()) === 0);
  const offered = await adding.getByRole('radiogroup', { name: 'Who it is' }).locator('strong').allTextContents();
  expect(`an agent, a person and a bot to be offered, got ${offered}`, offered.join() === 'An agent,A person,A bot');
  expect('it to begin on an agent', await adding.getByRole('radio', { name: /^An agent/ }).isChecked());
  await page.screenshot({ path: `${out}/channel-1-menu.png` });
  await page.keyboard.press('Escape');
  await adding.waitFor({ state: 'detached' });

  // An agent.
  await add('An agent');
  await page.getByRole('button', { name: 'Add agent', exact: true }).click();
  await page.getByText('builder is listening').waitFor();

  // A human: on the free plan there is nobody to add, and it says what would change that.
  await add('A person');
  await adding.getByText('Nobody else has an account here yet.').waitFor();
  await adding.getByText('The free plan is for one person.').waitFor();
  await page.screenshot({ path: `${out}/channel-2-human.png` });
  await adding.getByRole('button', { name: 'Done' }).click();

  // A bot: a blueprint, applied to this channel, that joins under its name.
  await add('A bot');
  await adding.getByLabel('Name').fill('Parrot');
  await adding.getByText('People will address it as @parrot.').waitFor();
  await adding.getByLabel('Answers with what this command prints').fill('echo "heard: $MESSAGE"');
  await page.screenshot({ path: `${out}/channel-3-bot.png` });
  await adding.getByRole('button', { name: 'Add bot', exact: true }).click();
  await counted(3).waitFor();
  // Behind the @ among the composer's tools are those that can be asked:
  // the agent and the bot, by the names they are asked by, and not the
  // person; and the factory's own bot, which can be written to in any
  // channel and is of it from the first thing said to it there. Everyone
  // in the channel is counted in its head, behind a few faces, and listed
  // when that is pressed.
  expect(`the agent and the bots to be who can be asked, and not the person, got ${await badges()}`, (await badges()).join() === 'builder,parrot,conductor');
  expect('no mark on the button while none of them is at work', (await page.locator('.asking-mark').count()) === 0);
  // Beside an agent's name is how it stands just now; a bot has no turns to be in the middle of.
  await asking.click();
  await askable.waitFor();
  const standing = askable.getByRole('menuitem', { name: /builder/ }).getByRole('img');
  expect(`the agent to be said to be idle, got "${await standing.getAttribute('aria-label')}"`, (await standing.getAttribute('aria-label')) === 'idle');
  expect('the list to open over the button, moving nothing', (await askable.boundingBox()).y + (await askable.boundingBox()).height <= (await asking.boundingBox()).y);
  // Yellow as the theme has it: the colour of a piece of work.
  const yellow = await page.evaluate(() => {
    const probe = document.body.appendChild(document.createElement('i'));
    probe.style.background = 'var(--card)';
    const is = getComputedStyle(probe).backgroundColor;
    probe.remove();
    return is;
  });
  const colour = await standing.evaluate((mark) => getComputedStyle(mark).backgroundColor);
  expect(`an idle agent's mark to be the theme's yellow (${yellow}), and it is ${colour}`, colour === yellow && /^rgb\((2[0-9]{2}), (1[4-9][0-9]|2[0-9]{2}), /.test(colour));
  expect('no such mark on a bot', (await askable.getByRole('menuitem', { name: /parrot/ }).getByRole('img').count()) === 0);
  await page.screenshot({ path: `${out}/channel-3-asking.png` });
  await page.keyboard.press('Escape');
  await askable.waitFor({ state: 'detached' });
  expect('Escape to put the list away', (await askable.count()) === 0);
  const everyone = page.getByRole('button', { name: /participants?$/ });
  expect(`the head to say how many are in the channel, got "${await everyone.innerText()}"`, (await everyone.innerText()).replace(/\s+/g, ' ').trim().endsWith('3 participants'));
  expect('a face for each of them, the person first', (await everyone.locator('.face').allTextContents()).join() === 'AD,BU,PA');
  await everyone.click();
  const listed = (await page.getByRole('menu', { name: 'Participants' }).locator('.person').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
  expect(`everyone to be listed with what they are, got ${listed.join(' | ')}`, listed.length === 3 && /^AD Ada human · you$/.test(listed[0]) && /^BU builder agent/.test(listed[1]) && /^PA Parrot bot$/.test(listed[2]));
  await page.screenshot({ path: `${out}/channel-3-participants.png` });
  await page.keyboard.press('Escape');
  expect('Escape to put the list away', (await page.getByRole('menu', { name: 'Participants' }).count()) === 0);

  // A second agent, set to a model, so that there is a model to show.
  await page.evaluate(async () => {
    // Asked as the page that is signed in asks: with the key it keeps.
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
    const channels = await (await fetch('/api/channels', { headers: mine })).json();
    const body = JSON.stringify({ name: 'reviewer', adapter: 'builtin', model: 'careful-1' });
    const added = await fetch(`/api/channels/${channels.channels[0].id}/agents`, { method: 'POST', headers: { ...mine, 'content-type': 'application/json' }, body });
    if (!added.ok) throw new Error(await added.text());
  });
  await counted(4).waitFor();

  // The box to write in is in the middle of the view, and no wider than is comfortable.
  const around = await page.locator('.channel > .split').boundingBox();
  const written = await page.locator('.channel > .split .composer-box').boundingBox();
  const off = Math.abs(written.x + written.width / 2 - (around.x + around.width / 2));
  expect(`the box to be centred and at most 720 wide, got ${written.width} wide and ${off} off centre`, off < 1 && written.width <= 720);

  // Typing @ offers who can be asked: agents first, then bots; not oneself.
  const box = page.getByLabel('Message');
  await box.click();
  await page.keyboard.type('@');
  const names = () => page.locator('.mentions [role=option] strong').allTextContents();
  await page.locator('.mentions').waitFor();
  expect(`the agents and the bots to be offered, the factory's own among them, got ${await names()}`, (await names()).join() === '@builder,@reviewer,@parrot,@conductor');
  expect('an agent to be offered with its model', (await page.locator('.mentions [role=option]', { hasText: '@reviewer' }).innerText()).includes('careful-1'));
  await page.screenshot({ path: `${out}/channel-4-mention.png` });
  // What is typed narrows it; Enter takes the one marked and does not send.
  await page.keyboard.type('pa');
  expect(`typing to narrow it to the bot, got ${await names()}`, (await names()).join() === '@parrot');
  await page.keyboard.press('Enter');
  expect(`the mention to be finished in the box, got "${await box.inputValue()}"`, (await box.inputValue()) === '@parrot ');
  expect('the list to be gone once one is chosen', (await page.locator('.mentions').count()) === 0);
  expect('nothing to have been sent yet', (await page.locator('.message').count()) === 0);
  await page.keyboard.type('are you there');
  await page.keyboard.press('Enter');

  // The bot answers by itself, as itself.
  const answer = page.locator('.message', { hasText: 'heard: @parrot are you there' });
  await answer.waitFor();
  expect('its answer to be marked as a bot\'s', (await answer.locator('.when', { hasText: 'bot' }).count()) === 1);
  expect('the first agent to have stayed quiet: it was not the one named', (await page.locator('.message').count()) === 2);

  // Down and Enter choose with the keys; the agent's answer says which model wrote it.
  await page.keyboard.type('thanks. @');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Tab');
  expect(`the second in the list to be taken, got "${await box.inputValue()}"`, (await box.inputValue()) === 'thanks. @reviewer ');
  await page.keyboard.type('hello');
  await page.keyboard.press('Enter');
  const reply = page.locator('.message', { has: page.locator('.who.agent', { hasText: 'reviewer' }) });
  await reply.waitFor();
  const label = await reply.locator('.when').first().innerText();
  expect(`the agent's message to say its model, got "${label}"`, label.replace(/\s+/g, ' ') === 'agent · careful-1');
  // An address is not a mention, and Escape puts the list away without sending.
  await page.keyboard.type('mail ada@example');
  expect('no list for an address', (await page.locator('.mentions').count()) === 0);
  await page.keyboard.type(' @');
  await page.locator('.mentions').waitFor();
  await page.keyboard.press('Escape');
  expect('Escape to put the list away', (await page.locator('.mentions').count()) === 0);
  expect('and to leave what was typed', (await box.inputValue()) === 'mail ada@example @');
  await page.screenshot({ path: `${out}/channel-5-thread.png` });

  // A thread: something said under a message. The agent that answered what
  // the thread is under goes on answering there, without being named again.
  const inChannel = page.locator('.channel > .split .message');
  const before = await inChannel.count();
  const asked = inChannel.filter({ hasText: 'thanks. @reviewer hello' });
  await asked.hover();
  // The button for it is on the line that says who said it and when, after
  // them: on the words of neither this message nor the one before it, and
  // making the line no taller for being shown.
  const row = await asked.boundingBox();
  const button = await asked.getByRole('button', { name: 'Reply in thread' }).boundingBox();
  const words = await asked.locator('.body').boundingBox();
  const time = await asked.locator('.when').last().boundingBox();
  expect(`the thread button to come after when it was said (that ends at ${time.x + time.width}, it starts at ${button.x})`, button.x >= time.x + time.width && button.x - (time.x + time.width) < 24);
  expect(`the thread button to be on that line (it is at ${button.y} to ${button.y + button.height}, the line at ${time.y} to ${time.y + time.height})`, button.y + button.height / 2 > time.y && button.y + button.height / 2 < time.y + time.height);
  expect(`the thread button not to lie on the message's words (it ends at ${button.y + button.height}, they start at ${words.y})`, button.y + button.height <= words.y + 2);
  expect('the thread button to be inside the message', button.y >= row.y);
  await page.mouse.move(10, 400);
  const unlit = await asked.boundingBox();
  expect(`the message to be as tall pointed at as not (${row.height} and ${unlit.height})`, Math.abs(row.height - unlit.height) < 0.5);
  await asked.hover();
  await page.screenshot({ path: `${out}/channel-5-pointed-at.png` });
  await asked.getByRole('button', { name: 'Reply in thread' }).click();
  const thread = page.locator('.thread');
  await thread.waitFor();
  await toolsAtRest(page).waitFor();
  await thread.locator('.thread-count', { hasText: '0 replies' }).waitFor();
  await thread.getByLabel('Reply').fill('and once more, without your name');
  await page.keyboard.press('Enter');
  await thread.locator('.message .who.agent', { hasText: 'reviewer' }).waitFor();
  await thread.locator('.thread-count', { hasText: '2 replies' }).waitFor();
  expect('what was said in the thread to stay out of the channel', (await inChannel.count()) === before);
  expect('the message it is under to say how much is under it', (await asked.getByRole('button', { name: '2 replies' }).count()) === 1);
  // What is written in a thread is written under the last thing said in it,
  // not at the foot of the window with the thread's emptiness in between.
  const threadLast = await thread.locator('.message').last().boundingBox();
  const threadBox = await thread.locator('.composer').boundingBox();
  const threadPane = await thread.boundingBox();
  expect(
    `the thread's composer to be straight under what was last said (that ends at ${threadLast.y + threadLast.height}, it starts at ${threadBox.y})`,
    threadBox.y >= threadLast.y + threadLast.height && threadBox.y - (threadLast.y + threadLast.height) < 40,
  );
  expect('the thread\'s composer to stay inside the thread', threadBox.y + threadBox.height <= threadPane.y + threadPane.height + 1);
  await page.screenshot({ path: `${out}/channel-6-thread.png` });
  await page.getByRole('button', { name: 'Close the thread' }).click();
  expect('the thread to close', (await thread.count()) === 0);
  await asked.getByRole('button', { name: '2 replies' }).click();
  await thread.locator('.message', { hasText: 'and once more, without your name' }).waitFor();
  await page.getByRole('button', { name: 'Close the thread' }).click();
  // Closing a thread leaves the panel it was read in open. It is folded away here, to be opened further on.
  expect('the panel to stay when its thread is closed', (await page.getByRole('complementary', { name: 'Tools' }).count()) === 1);
  await page.getByRole('button', { name: 'Collapse the tools panel' }).click();

  // Pointing at someone says who they are, and the card stays inside the window.
  const inside = async () => {
    const card = await page.locator('.who-card').boundingBox();
    const { width, height } = page.viewportSize();
    return card.x >= 0 && card.y >= 0 && card.x + card.width <= width && card.y + card.height <= height;
  };
  // The card is the picture's: a message's author is pointed at by their picture, and their name is only words.
  const pictureOf = (name, kind = '') => page.locator('.channel > .split .message', { has: page.locator(`.who${kind}`, { hasText: name }) }).first().locator('.avatar');
  const card = page.locator('.who-card');
  await page.locator('.channel > .split .message .who.agent', { hasText: 'reviewer' }).first().hover();
  await page.waitForTimeout(400);
  expect('no card from pointing at an author\'s name', (await card.count()) === 0);
  await pictureOf('reviewer', '.agent').hover();
  await card.waitFor();
  const about = (await card.innerText()).replace(/\s+/g, ' ');
  expect(`an agent's card to say what it runs on and may do, got "${about}"`, about.includes('reviewer') && about.includes('Runs on') && about.includes('careful-1') && about.includes('May do') && about.includes('edit'));
  expect('the card to be inside the window', await inside());
  await page.screenshot({ path: `${out}/channel-7-card.png` });
  // A card lies over the messages under what it is about, so the pointer leaves before it goes to the next of them.
  const away = async () => {
    await page.mouse.move(10, 400);
    await card.waitFor({ state: 'detached' });
  };
  await away();
  await page.locator('.channel > .split .message .avatar.bot').first().hover();
  await card.waitFor();
  expect('a bot\'s card to say what a bot is', (await card.innerText()).includes('A blueprint taking part in this channel') && (await card.innerText()).includes('Parrot') && (await inside()));
  // A person's card is on what they said too.
  await away();
  await page.locator('.message .who', { hasText: 'Ada' }).first().hover();
  await page.waitForTimeout(400);
  expect('no card from pointing at a person\'s name', (await card.count()) === 0);
  await pictureOf('Ada').hover();
  await card.waitFor();
  expect('a person\'s card to say it is you', (await card.innerText()).includes('you'));
  // Pressing an author's name is pressing the message: it opens no thread and nothing else.
  await away();
  await page.locator('.channel > .split .message .who', { hasText: 'Ada' }).first().click();
  await page.waitForTimeout(300);
  expect('pressing a name to open no thread, card or panel', (await page.locator('.thread').count()) === 0 && (await card.count()) === 0 && (await page.getByRole('complementary', { name: 'Tools' }).count()) === 0);
  // The card can be pointed at itself, to be read and copied from: going
  // from what it is about on to it keeps it, and it goes a moment after
  // the pointer has left both.
  await page.mouse.move(10, 400);
  await pictureOf('reviewer', '.agent').hover();
  await card.waitFor();
  const on = await card.boundingBox();
  await page.mouse.move(on.x + on.width / 2, on.y + on.height / 2, { steps: 6 });
  await page.waitForTimeout(400);
  expect('the card to stay while it is pointed at', (await card.count()) === 1 && (await card.innerText()).includes('reviewer'));
  await page.mouse.move(10, 400);
  await card.waitFor({ state: 'detached' });

  // ---- the side panel folds away -------------------------------------------
  // A button at the panel's head, across from what it is called, folds it
  // away for room. What brings it back, showing what it showed, is then
  // where the tabs begin, and not on the rail.
  const fold = page.getByRole('button', { name: 'Collapse the side panel' });
  const head = await page.locator('.sidebar .brand').boundingBox();
  const folds = await fold.boundingBox();
  expect(
    `the button that folds the panel to be at the right of its head, got ${JSON.stringify(folds)} in ${JSON.stringify(head)}`,
    folds.y >= head.y && folds.y + folds.height <= head.y + head.height && folds.x + folds.width > head.x + head.width - 24,
  );
  expect('no such button on the rail', (await page.locator('.rail').getByRole('button', { name: /the side panel/ }).count()) === 0);
  await fold.click();
  await sideSettled(page);
  expect('the side panel to be gone', (await page.locator('.sidebar').count()) === 0);
  const unfold = page.getByRole('button', { name: 'Expand the side panel' });
  const firstTab = await page.getByRole('tab').first().boundingBox();
  const unfolds = await unfold.boundingBox();
  expect(
    `what brings it back to be before the first tab, in its row, got ${JSON.stringify(unfolds)} before ${JSON.stringify(firstTab)}`,
    Math.abs(unfolds.x + unfolds.width - firstTab.x) < 2 && Math.abs(unfolds.y - firstTab.y) < 2 && Math.abs(unfolds.height - firstTab.height) < 2,
  );
  await page.screenshot({ path: `${out}/channel-7-folded.png` });
  await unfold.click();
  await sideSettled(page);
  await page.getByRole('navigation', { name: 'Channels' }).waitFor();
  await side(page, 'Resources');
  await page.getByRole('button', { name: 'Collapse the side panel' }).click();
  await sideSettled(page);
  await page.getByRole('button', { name: 'Expand the side panel' }).click();
  await sideSettled(page);
  await page.getByRole('navigation', { name: 'Resources' }).waitFor();
  await side(page, 'Channels');
  // The same from the keyboard; and the keys there are, are listed on asking.
  await page.keyboard.press('Control+Backslash');
  expect('Ctrl and backslash to fold the side panel away', (await page.locator('.sidebar').count()) === 0);
  await page.keyboard.press('Control+Backslash');
  await sideSettled(page);
  await page.getByRole('navigation', { name: 'Channels' }).waitFor();
  await page.keyboard.press('Control+Shift+Slash');
  const keys = page.getByRole('dialog');
  await keys.getByText('Keyboard shortcuts').first().waitFor();
  const shortcuts = (await keys.locator('.shortcut').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
  expect(`the shortcuts to be listed with their keys, got ${shortcuts.slice(0, 4).join(' | ')}`, shortcuts.includes('Search everything Ctrl K') && shortcuts.includes('Keyboard shortcuts Ctrl Shift /') && shortcuts.includes('Collapse or expand the side panel Ctrl \\'));
  await page.screenshot({ path: `${out}/channel-8-shortcuts.png` });
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await keys.waitFor({ state: 'detached' });

  // ---- attaching -----------------------------------------------------------
  // What goes with a message, waiting over the box and outside it, and what a paste brings (`attaching.mjs`).
  const composer = page.locator('.channel > .split .composer');
  await attaching({ page, out, expect, composer, inChannel, asked });

  // ---- views ---------------------------------------------------------------
  // A page that is attached is drawn where it is posted: here a chart, made
  // with the library that is served beside the client, in the reader's colours.
  const chart = `<h3 style="margin:0 0 8px">Build times</h3>
<div style="height:220px"><canvas id="chart"></canvas></div>
<p id="said" style="color:var(--dim)">drawing</p>
<script src="/lib/chart.js"></script>
<script>
  let chart;
  addEventListener('theme', (told) => {
    const { text, line, accent } = told.detail.colours;
    chart?.destroy();
    chart = new Chart(document.getElementById('chart'), {
      type: 'line',
      data: { labels: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri'], datasets: [{ label: 'seconds', data: [41, 38, 45, 36, 33], borderColor: accent, tension: 0.3 }] },
      options: { maintainAspectRatio: false, color: text, scales: { x: { ticks: { color: text }, grid: { color: line } }, y: { ticks: { color: text }, grid: { color: line } } } },
    });
    document.getElementById('said').textContent = 'drawn for a ' + told.detail.scheme + ' page';
  });
</script>`;
  const requests = [];
  page.on('request', (request) => requests.push(request.url()));
  await composer.getByLabel('Files to attach').setInputFiles([{ name: 'build-times.html', mimeType: 'text/html', buffer: Buffer.from(chart) }]);
  await composer.locator('.attaching-one', { hasText: 'build-times.html' }).waitFor();
  await composer.getByLabel('Message').fill('build times this week');
  await page.keyboard.press('Enter');
  const shown = inChannel.filter({ hasText: 'build times this week' }).locator('iframe.view-frame');
  await shown.waitFor();
  expect('the view to be in a box with no origin of its own, where only scripts are allowed', (await shown.getAttribute('sandbox')) === 'allow-scripts');
  const view = await (await shown.elementHandle()).contentFrame();
  await view.locator('#said', { hasText: `drawn for a ${process.env.SCHEME ?? 'dark'} page` }).waitFor();
  expect('the chart to be drawn with the library that is served here', await view.evaluate(() => typeof Chart === 'function' && Chart.getChart('chart')?.data.datasets[0].data.length === 5));
  const accent = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim());
  expect(`the view to be given the reader's colours, got "${await view.evaluate(() => document.documentElement.style.getPropertyValue('--accent'))}"`, (await view.evaluate(() => document.documentElement.style.getPropertyValue('--accent'))) === accent);
  // It is given as much room as its page takes.
  await page.waitForFunction(() => document.querySelector('.channel > .split iframe.view-frame')?.getBoundingClientRect().height > 250);
  // And the channel stays at its foot as the view takes that room.
  await page.waitForFunction(() => {
    const scroller = document.querySelector('.channel > .split .messages');
    return scroller && scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 4;
  });
  await page.mouse.move(10, 400);
  await page.screenshot({ path: `${out}/channel-11-view.png` });

  // What it cannot do. It has nobody's session and cannot read the client;
  // it reaches nothing over the network, not even this daemon; and it
  // cannot take its box anywhere else.
  const tried = await view.evaluate(async () => {
    const fails = async (what) => {
      try {
        await what();
        return 'done';
      } catch (err) {
        return 'refused';
      }
    };
    return {
      client: await fails(() => parent.document.title),
      session: await fails(() => document.cookie),
      storage: await fails(() => localStorage.getItem('client.side')),
      daemon: await fails(() => fetch('/api/me')),
      elsewhere: await fails(() => fetch('http://127.0.0.1:9/')),
      picture: await new Promise((done) => {
        const picture = new Image();
        picture.onload = () => done('done');
        picture.onerror = () => done('refused');
        picture.src = '/api/meta';
      }),
    };
  });
  expect(`a view to be refused everything outside itself, got ${JSON.stringify(tried)}`, Object.values(tried).every((how) => how === 'refused'));
  requests.length = 0;
  await view.evaluate(() => {
    location.href = 'http://example.invalid/taken-away';
  }).catch(() => {});
  await page.waitForTimeout(500);
  expect(`a view not to be able to send its box elsewhere, asked for ${requests.filter((one) => one.includes('example.invalid'))}`, !requests.some((one) => one.includes('example.invalid')));
  expect('the view asked for nothing from the daemon while it tried', !requests.some((one) => one.includes('/api/')));

  // It opens larger, and can be saved as the file it is.
  const foot = inChannel.filter({ hasText: 'build times this week' }).locator('.attached-view-foot');
  expect('a way to save the page', (await foot.getByRole('link', { name: 'Save a copy' }).getAttribute('download')) === 'build-times.html');
  expect('Full screen beside Open larger (its doing is fullscreen.mjs)', await foot.getByRole('button', { name: 'Full screen' }).isVisible());
  await foot.getByRole('button', { name: 'Open larger' }).click();
  const larger = await (await page.getByRole('dialog').locator('iframe.view-frame').elementHandle()).contentFrame();
  await larger.locator('#said', { hasText: `drawn for a ${process.env.SCHEME ?? 'dark'} page` }).waitFor();
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'detached' });

  {
    // Choosing one of those behind the @ begins asking it: its name is put
    // where the caret is, and the caret is left after it to go on writing.
    const ask = composer.getByLabel('Message');
    await ask.fill('');
    await asking.click();
    await askable.getByRole('menuitem', { name: /reviewer/ }).click();
    expect(`choosing an agent to put its name in the box, got "${await ask.inputValue()}"`, (await ask.inputValue()) === '@reviewer ');
    expect('the caret to be in the box after it', await ask.evaluate((area) => document.activeElement === area));
    expect('the list to be put away by the choice', (await askable.count()) === 0);
    await page.keyboard.type('and');
    await asking.click();
    await askable.getByRole('menuitem', { name: /parrot/ }).click();
    expect(`the next one to go after what was written, got "${await ask.inputValue()}"`, (await ask.inputValue()) === '@reviewer and @parrot ');

    // The tools under the box put their marks round what is selected, and
    // leave it selected, so that one tool can follow another.
    const tools = composer.getByRole('toolbar', { name: 'Formatting' });
    await ask.fill('ship it');
    await ask.selectText();
    await tools.getByRole('button', { name: 'Bold' }).click();
    expect(`Bold to put its marks round what is selected, got "${await ask.inputValue()}"`, (await ask.inputValue()) === '**ship it**');
    await page.keyboard.press('Control+i');
    expect(`Ctrl+I to lean what is still selected, got "${await ask.inputValue()}"`, (await ask.inputValue()) === '**_ship it_**');
    await page.keyboard.press('Control+z');
    expect(`undoing to take the last tool's work back, got "${await ask.inputValue()}"`, (await ask.inputValue()) === '**ship it**');
    await tools.getByRole('button', { name: 'Bold' }).click();
    expect(`Bold again to take its marks away, got "${await ask.inputValue()}"`, (await ask.inputValue()) === 'ship it');
    // A list is written with its tool, and goes on with Shift and Enter.
    await ask.fill('');
    await tools.getByRole('button', { name: 'Bulleted list' }).click();
    await page.keyboard.type('one');
    await page.keyboard.press('Shift+Enter');
    await page.keyboard.type('two');
    expect(`a list to go on to its next item, got ${JSON.stringify(await ask.inputValue())}`, (await ask.inputValue()) === '- one\n- two');
    await ask.selectText();
    await tools.getByRole('button', { name: 'Kind of line' }).click();
    const kinds = page.getByRole('menu', { name: 'Kind of line' });
    expect(`the kinds of line to be offered, got ${await kinds.getByRole('menuitem').allInnerTexts()}`, (await kinds.getByRole('menuitem').allInnerTexts()).join() === 'Text,Heading 1,Heading 2,Heading 3');
    const where = await ask.boundingBox();
    await page.screenshot({ path: `${out}/channel-12-tools.png` });
    expect('the kinds of line not to move the box they are over', (await ask.boundingBox()).y === where.y);
    await kinds.getByRole('menuitem', { name: 'Text' }).click();
    expect(`Text to make them plain lines again, got ${JSON.stringify(await ask.inputValue())}`, (await ask.inputValue()) === 'one\ntwo');

    // What was written with them is drawn in the channel as what it means.
    await ask.fill('## The plan\n**ship it** and _then_ ++rest++, ~~not~~ `now`\n- one\n- two\n  1. inside\n> as was said\nsee [the docs](https://example.com/docs) or https://example.com/more.');
    await page.keyboard.press('Enter');
    const said = inChannel.filter({ hasText: 'The plan' }).locator('.body');
    await said.waitFor();
    const drawn = await said.evaluate((body) => ({
      heading: body.querySelector('[role=heading]')?.textContent,
      bold: body.querySelector('strong')?.textContent,
      leaning: body.querySelector('em')?.textContent,
      under: body.querySelector('u')?.textContent,
      struck: body.querySelector('s')?.textContent,
      code: body.querySelector('code')?.textContent,
      items: [...body.querySelectorAll('[role=listitem]')].map((item) => item.textContent),
      quote: body.querySelector('blockquote')?.textContent,
      links: [...body.querySelectorAll('a')].map((link) => `${link.textContent} -> ${link.href} ${link.target} ${link.rel}`),
      marks: /[*_~`#>\[\]]|\+\+/.test(body.textContent),
    }));
    expect(
      `the message to be drawn as it was meant, got ${JSON.stringify(drawn)}`,
      drawn.heading === 'The plan' && drawn.bold === 'ship it' && drawn.leaning === 'then' && drawn.under === 'rest' && drawn.struck === 'not' && drawn.code === 'now' && drawn.items.join('|') === '•one|•two|1.inside' && drawn.quote === 'as was said' && !drawn.marks,
    );
    expect(
      `links to lead out in a window of their own, got ${drawn.links.join(' | ')}`,
      drawn.links.join(' | ') === 'the docs -> https://example.com/docs _blank noopener noreferrer | https://example.com/more -> https://example.com/more _blank noopener noreferrer',
    );
    await said.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${out}/channel-13-written.png` });
  }

  {
    // A channel folds open in the side panel to who is in it: the people in
    // one place, the agents in another with how each stands, and the bots.
    await side(page, 'Channels');
    const panel = page.getByRole('navigation', { name: 'Channels' });
    expect('a channel to start folded', (await panel.locator('.nav-who').count()) === 0);
    await panel.getByRole('button', { name: 'Unfold general' }).click();
    const within = (await panel.locator('.nav-who').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
    expect(`the channel to list who is in it by kind, got ${within.join(' | ')}`, within.join('|') === '· Ada you|builder|reviewer|◇ Parrot');
    const stands = await panel.locator('.nav-who.agent', { hasText: 'builder' }).getByRole('img').getAttribute('aria-label');
    expect(`an agent under its channel to say how it stands, got "${stands}"`, stands === 'idle');
    await page.screenshot({ path: `${out}/channel-14-tree.png` });

    // Alt and a number show a view of the rail; pressed again it stays.
    await page.locator('.view-head h1').first().click();
    await page.keyboard.press('Alt+2');
    await page.getByRole('navigation', { name: 'Resources' }).waitFor();
    await page.keyboard.press('Alt+2');
    expect('the same key again to leave the view showing', (await page.getByRole('navigation', { name: 'Resources' }).count()) === 1);
    await page.keyboard.press('Alt+1');
    await page.getByRole('navigation', { name: 'Channels' }).waitFor();
    // A slash goes to the box a message is written in, and is not written in it.
    const message = page.locator('.channel > .split').getByLabel('Message');
    await message.fill('');
    await page.locator('.view-head h1').first().click();
    await page.keyboard.press('/');
    expect('a slash to put the caret in the composer', await message.evaluate((area) => document.activeElement === area));
    expect(`the slash not to be typed, got "${await message.inputValue()}"`, (await message.inputValue()) === '');
    await page.keyboard.type('a/b');
    expect('a slash typed in the box to be a slash', (await message.inputValue()) === 'a/b');
    await message.fill('');

    // Ctrl and Tab turn through the tabs of the pane that has the keyboard, and round again; with Shift, the other way.
    const showing = async () => (await page.locator('.tabs .tab[aria-selected="true"]').first().innerText()).replace('✕', '').trim();
    const here = await showing();
    await page.keyboard.press('Control+Tab');
    const after = await showing();
    expect(`Ctrl and Tab to turn to another tab, from ${here} to ${after}`, after !== here);
    await page.keyboard.press('Control+Shift+Tab');
    expect(`Ctrl, Shift and Tab to turn back, got ${await showing()}`, (await showing()) === here);
    // And by the keys a browser leaves to the page: Ctrl and the full stop, Ctrl and the comma.
    await page.keyboard.press('Control+.');
    expect(`Ctrl and the full stop to turn to the next tab, got ${await showing()}`, (await showing()) === after);
    await page.keyboard.press('Control+,');
    expect(`Ctrl and the comma to turn back, got ${await showing()}`, (await showing()) === here);
    // Who is signed in is on the rail, over the settings, and is a page of its own.
    await page.getByRole('button', { name: 'You: Ada' }).click();
    await page.getByRole('heading', { name: 'Profile' }).waitFor();
    expect('the profile to be who is signed in, and how to sign out, here and everywhere', (await page.getByRole('button', { name: 'Sign out', exact: true }).count()) === 1 && (await page.getByRole('button', { name: 'Sign out everywhere', exact: true }).count()) === 1 && (await page.getByText('ada@example.com').count()) === 1);
    await page.screenshot({ path: `${out}/channel-15-profile.png` });
    await page.getByRole('button', { name: 'Settings' }).click();
    await page.getByRole('heading', { name: 'Theme' }).waitFor();
    expect('settings not to say again who is signed in', (await page.locator('.view:not([hidden])').getByRole('button', { name: 'Sign out' }).count()) <= 1 && (await page.getByRole('heading', { name: 'You', exact: true }).count()) === 0);
    expect('nobody at the foot of the side panel', (await page.locator('.sidebar-foot').count()) === 0);
  }

  {
    // Beside adding a member is what opens the panel on the right: a second,
    // smaller window with a few tabs, which are the reader's to choose.
    await page.locator('.tabs').getByRole('tab', { name: /general/ }).click();
    const message = page.locator('.channel > .split').getByLabel('Message');
    const wide = (await message.boundingBox()).width;
    await page.getByRole('button', { name: 'Expand the tools panel' }).click();
    await toolsAtRest(page).waitFor();
    const panel = page.getByRole('complementary', { name: 'Tools' });
    const tabs = async () => (await panel.getByRole('tab').allInnerTexts()).map((one) => one.replace('×', '').trim());
    expect(`the panel to start with its usual tabs, got ${await tabs()}`, (await tabs()).join() === 'Media,Channel,Board,Docs');
    // They are more than the panel is wide as it starts: their row scrolls sideways inside the panel, and the tab
    // showing is in sight; turned by the wheel to its end, the last of them is in sight (Docs is one
    // of the usual tabs). They are every tab there is, so there is no `+` to add one.
    const rimOf = await panel.boundingBox();
    const rowOf = await panel.getByRole('tablist', { name: 'Tools' }).boundingBox();
    const inSight = async (name) => {
      const one = await panel.getByRole('tab', { name }).boundingBox();
      return one.x >= rowOf.x - 0.5 && one.x + one.width <= rowOf.x + rowOf.width + 0.5;
    };
    expect(`no way to add a tab while every tab there is is one, and the row of tabs to end inside a panel that ends at ${rimOf.x + rimOf.width}; the row ends at ${rowOf.x + rowOf.width}`, (await panel.getByRole('button', { name: 'Add a tab' }).count()) === 0 && rowOf.x + rowOf.width <= rimOf.x + rimOf.width + 0.5);
    expect('the tab showing to be in sight as the panel starts', await inSight('Media'));
    await page.mouse.move(rowOf.x + 20, rowOf.y + rowOf.height / 2);
    await page.mouse.wheel(0, 600);
    await panel.getByRole('tab', { name: 'Docs' }).evaluate((tab) => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    expect('the last of the usual tabs in sight, the row turned to its end by the wheel', await inSight('Docs'));
    await page.mouse.wheel(0, -600);
    await panel.getByRole('tab', { name: 'Docs' }).evaluate((tab) => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    expect('the first tab in sight again, the row turned back', await inSight('Media'));
    expect('the channel still to be there beside it', (await message.boundingBox()).width > 200 && (await message.boundingBox()).width <= wide);
    // Its tabs are on the line of the channel's buttons beside them.
    const middle = async (locator) => {
      const box = await locator.boundingBox();
      return box.y + box.height / 2;
    };
    const buttons = await middle(page.locator('.channel > .split > .view-head').getByRole('button', { name: /participants?$/ }));
    const tabsAt = await middle(panel.getByRole('tab').first());
    expect(`the panel's tabs to be level with the channel's buttons, which are at ${buttons}; they are at ${tabsAt}`, Math.abs(buttons - tabsAt) <= 1);
    // What was attached in the channel is in the first of them.
    await panel.locator('.context-pictures img').first().waitFor();
    const listed = (await panel.locator('.context-row').allInnerTexts()).map((one) => one.split('\n')[0].trim());
    expect(`the pages and files attached here to be listed, got ${listed.join()}`, listed.includes('build-times.html') && listed.includes('notes.txt'));
    await page.screenshot({ path: `${out}/channel-16-context.png` });
    // A page that was attached is looked at in the panel, in the same box it has in the channel.
    await panel.locator('.context-row', { hasText: 'build-times.html' }).click();
    const inPanel = await (await panel.locator('iframe.view-frame').elementHandle()).contentFrame();
    await inPanel.locator('#said', { hasText: `drawn for a ${process.env.SCHEME ?? 'dark'} page` }).waitFor();
    expect('the page in the panel to be boxed as it is in the channel', (await panel.locator('iframe.view-frame').getAttribute('sandbox')) === 'allow-scripts');
    expect('a way to save it from there', (await panel.getByRole('link', { name: 'Save a copy' }).getAttribute('download')) === 'build-times.html');
    const edgeOf = await panel.boundingBox();
    const save = await panel.getByRole('link', { name: 'Save a copy' }).boundingBox();
    const boxed = await panel.locator('iframe.view-frame').boundingBox();
    expect(
      `the page and what saves it to be inside the panel, which ends at ${edgeOf.x + edgeOf.width}; they end at ${boxed.x + boxed.width} and ${save.x + save.width}`,
      boxed.x + boxed.width <= edgeOf.x + edgeOf.width - 8 && save.x + save.width <= edgeOf.x + edgeOf.width - 8,
    );
    await page.screenshot({ path: `${out}/channel-16-context-page.png` });
    await panel.getByRole('button', { name: 'Back to the media' }).click();
    await panel.locator('.context-pictures img').first().waitFor();
    // Everybody of the channel is one tab: people, agents and bots in one
    // list, each saying its kind, and an agent how it stands and what it is at.
    await panel.getByRole('tab', { name: /^Channel\b/ }).click();
    const names = (await panel.locator('.context-agent .context-name').allInnerTexts()).map((one) => one.trim());
    expect(`the channel's agents to be in the panel, got ${names.join()}`, names.join() === 'builder,reviewer');
    expect('an agent there to say how it stands', (await panel.locator('.context-agent').first().getByRole('img').getAttribute('aria-label')) === 'idle');
    const everybody = (await panel.locator('.context-member').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
    const kinds = everybody.map((one) => (/ person( · you)?$/.test(one) ? 'person' : / bot$/.test(one) ? 'bot' : /agent/.test(one) ? 'agent' : '?'));
    // Each a card with their picture, as it is drawn beside what they say.
    expect('everybody of the channel to be a card with their picture', (await panel.locator('.context-member').count()) === (await panel.locator('.context-member > .avatar').count()));
    expect(`people, then agents, then bots in the one list, each saying what it is, got ${everybody.join(' | ')}`, kinds.join() === [...kinds].sort((a, b) => ['person', 'agent', 'bot'].indexOf(a) - ['person', 'agent', 'bot'].indexOf(b)).join() && kinds.includes('person') && kinds.includes('agent') && kinds.includes('bot') && !kinds.includes('?'));
    expect('no tab of agents or of people beside it', !(await tabs()).includes('Agents') && !(await tabs()).includes('People'));
    await page.screenshot({ path: `${out}/channel-17-tools-members.png` });
    // A tab is taken away, and added again: the panel is as the reader left it, when it is opened again.
    await panel.getByRole('button', { name: 'Remove the Channel tab' }).click();
    expect(`the tab to be gone, got ${await tabs()}`, (await tabs()).join() === 'Media,Board,Docs');
    await panel.getByRole('button', { name: 'Add a tab' }).click();
    await panel.getByRole('menuitem', { name: /^Channel(?! members)/ }).click();
    expect(`the new tab to be there and showing, got ${await tabs()}`, (await tabs()).join() === 'Media,Board,Docs,Channel' && (await panel.getByRole('tab', { name: /^Channel\b/ }).getAttribute('aria-selected')) === 'true');
    // Who is in the channel is a list of plain rows: nobody has a card shown
    // over the list while they are pointed at, which is for where they are
    // named in what was said. An agent there is pressed to be read about.
    const named = panel.locator('.context-agent', { hasText: 'reviewer' });
    await named.hover();
    await page.waitForTimeout(300);
    expect('no card over the list of who is here', (await page.locator('.who-card').count()) === 0);
    // Pressed, somebody is read about there, under Channel, with the way back to the rest over them.
    const crumbs = panel.getByRole('navigation', { name: 'Where this is' });
    const backToEverybody = crumbs.getByRole('button', { name: 'Channel', exact: true });
    await named.click();
    await crumbs.getByText('reviewer', { exact: true }).waitFor();
    expect('an agent pressed in the list to be read about under Channel, which stays the tab', (await panel.locator('.context-facts').count()) === 1 && (await panel.getByRole('tab', { name: /^Channel\b/ }).getAttribute('aria-selected')) === 'true');
    expect(`nobody read about to be a tab of their own, got ${await tabs()}`, (await tabs()).join() === 'Media,Board,Docs,Channel');
    await page.screenshot({ path: `${out}/channel-17-tools-profile.png` });
    await backToEverybody.click();
    await panel.locator('.context-agent', { hasText: 'reviewer' }).waitFor();
    // A person is read about the same way.
    await panel.locator('.context-member', { hasText: 'Ada' }).click();
    await crumbs.getByText('Ada', { exact: true }).waitFor();
    expect('a person read about to be said to be one, and you', (await panel.locator('.context-about').innerText()).includes('person · you'));
    await backToEverybody.click();
    await page.getByRole('button', { name: 'Collapse the tools panel' }).click();
    expect('the panel to fold away', (await panel.count()) === 0);
    await page.reload();
    await page.getByRole('button', { name: 'Expand the tools panel' }).click();
    await toolsAtRest(page).waitFor();
    expect(`the panel to come back with the tabs it was left with, got ${await tabs()}`, (await tabs()).join() === 'Media,Board,Docs,Channel');
    // Its left edge is dragged to make it wider or narrower, within what
    // can be read and what leaves the channel room; and it stays as left.
    const across = async () => Math.round((await panel.boundingBox()).width);
    const edge = panel.getByRole('separator');
    const before = await across();
    const grip = await edge.boundingBox();
    await page.mouse.move(grip.x + grip.width / 2, grip.y + 200);
    await page.mouse.down();
    await page.mouse.move(grip.x + grip.width / 2 - 120, grip.y + 200, { steps: 6 });
    await page.mouse.up();
    expect(`dragging its edge left to make the panel wider by as much, from ${before} to ${await across()}`, Math.abs((await across()) - (before + 120)) <= 2);
    await page.mouse.move(grip.x - 120, grip.y + 200);
    await page.mouse.down();
    await page.mouse.move(grip.x + 900, grip.y + 200, { steps: 6 });
    await page.mouse.up();
    expect(`the panel not to be dragged narrower than it can be read at, got ${await across()}`, (await across()) === 240);
    await edge.focus();
    await page.keyboard.press('ArrowLeft');
    expect(`the arrows to move the edge too, got ${await across()}`, (await across()) === 264);
    await page.screenshot({ path: `${out}/channel-18-context-narrow.png` });
    await page.reload();
    await panel.waitFor();
    expect(`the panel to come back as wide as it was left, got ${await across()}`, (await across()) === 264);
    await edge.dblclick();
    expect(`pressing the edge twice to put it back as it started, got ${await across()}`, (await across()) === 388);
    // An agent, pressed by its picture where it said something, is read about in the panel: under
    // Channel, which is brought to the front, with what it runs on, what it
    // may do, what it was told and what it was last asked.
    const where = panel.getByRole('navigation', { name: 'Where this is' });
    await panel.getByRole('tab', { name: 'Media' }).click();
    await inChannel.filter({ has: page.locator('.who.agent', { hasText: 'reviewer' }) }).first().locator('.avatar').click();
    await where.getByText('reviewer', { exact: true }).waitFor();
    expect(`the agent to be read about under Channel, in front, got ${await tabs()}`, (await tabs()).join() === 'Media,Board,Docs,Channel' && (await panel.getByRole('tab', { name: /^Channel\b/ }).getAttribute('aria-selected')) === 'true');
    const facts = (await panel.locator('.context-facts').innerText()).replace(/\s+/g, ' ');
    // What it holds, by area: said in words while it is not yet known whether the agent is one of a fleet's rows, and
    // a moment later as what is chosen in a box, since whoever may change an agent in no row changes it here. It is
    // read as either: read as the table's words alone, the box's every choice was among them, and the check went by
    // having read it in the moment before the box was drawn.
    const holds = Object.fromEntries(
      await panel.locator('.holds-table tr').evaluateAll((rows) => rows.map((row) => [row.querySelector('th').textContent.trim(), (row.querySelector('select')?.selectedOptions[0] ?? row.querySelector('td')).textContent.trim()])),
    );
    expect(`the panel to say what the agent runs on, may do and holds, got ${facts} and ${JSON.stringify(holds)}`, /Runs on .*careful-1/.test(facts) && /May do edit/.test(facts) && holds.board === 'read and change');
    await panel.locator('.context-turn').first().waitFor();
    expect('what it was last asked to be there, with how it went', (await panel.locator('.context-turn .badge').first().innerText()).trim() === 'done');
    expect('no card left over what was pressed', (await page.locator('.who-card').count()) === 0);
    // Nothing in the panel reaches past its edge: not what is said of the agent, and not its row of tabs.
    const rim = await panel.boundingBox();
    const reach = async (locator) => {
      const box = await locator.boundingBox();
      return box.x + box.width;
    };
    expect(
      `what is in the panel to stay inside it, which ends at ${rim.x + rim.width}; a turn ends at ${await reach(panel.locator('.context-turn').first())} and the row of tabs at ${await reach(panel.getByRole('tablist', { name: 'Tools' }))}`,
      (await reach(panel.locator('.context-turn').first())) <= rim.x + rim.width - 8 && (await reach(panel.getByRole('tablist', { name: 'Tools' }))) <= rim.x + rim.width,
    );
    await page.screenshot({ path: `${out}/channel-18-context-agent.png` });
    // Another pressed takes its place, from whichever tab was showing; the way back over it leads to everyone.
    await panel.getByRole('tab', { name: 'Media' }).click();
    await inChannel.filter({ has: page.locator('.who.agent', { hasText: 'reviewer' }) }).first().locator('.avatar').click();
    await where.getByText('reviewer', { exact: true }).waitFor();
    expect('pressing somebody again to bring Channel to the front', (await panel.getByRole('tab', { name: /^Channel\b/ }).getAttribute('aria-selected')) === 'true');
    await where.getByRole('button', { name: 'Channel', exact: true }).click();
    await panel.locator('.context-agent', { hasText: 'reviewer' }).waitFor();
    expect(`going back to leave the reader's tabs as they were, got ${await tabs()}`, (await tabs()).join() === 'Media,Board,Docs,Channel');
    // A thread is read in the panel, as its first tab: opening one shows
    // it there, the reader's own tabs stay beside it, and closing it puts
    // the panel back as it was.
    await inChannel.filter({ hasText: 'thanks. @reviewer hello' }).getByRole('button', { name: /repl/ }).first().click();
    await panel.locator('.thread .message', { hasText: 'and once more, without your name' }).waitFor();
    expect(`an open thread to be the first tab, and showing, got ${await tabs()}`, (await tabs()).join() === 'Thread,Media,Board,Docs,Channel' && (await panel.getByRole('tab', { name: 'Thread' }).getAttribute('aria-selected')) === 'true');
    expect('one panel at the right of the channel, not two', (await page.locator('.channel > aside').count()) === 1);
    await panel.getByRole('tab', { name: 'Media' }).click();
    await panel.locator('.context-pictures img').first().waitFor();
    await panel.getByRole('tab', { name: 'Thread' }).click();
    await panel.getByLabel('Reply').waitFor();
    await page.screenshot({ path: `${out}/channel-19-thread-in-context.png` });
    await page.getByRole('button', { name: 'Close the thread' }).click();
    expect(`closing the thread to leave the reader's tabs, got ${await tabs()}`, (await tabs()).join() === 'Media,Board,Docs,Channel');
    // With the panel folded away, opening a thread opens it; closing the
    // thread closes the thread, and the panel stays with the reader's tabs.
    await page.getByRole('button', { name: 'Collapse the tools panel' }).click();
    expect('the panel to fold away', (await panel.count()) === 0);
    await inChannel.filter({ hasText: 'thanks. @reviewer hello' }).getByRole('button', { name: /repl/ }).first().click();
    await panel.locator('.thread').waitFor();
    await page.getByRole('button', { name: 'Close the thread' }).click();
    expect(`closing a thread to leave the panel open with the reader's tabs, got ${(await panel.count()) === 1 ? await tabs() : 'no panel'}`, (await panel.count()) === 1 && (await tabs()).join() === 'Media,Board,Docs,Channel');
    // Folding the panel away takes an open thread with it.
    await inChannel.filter({ hasText: 'thanks. @reviewer hello' }).getByRole('button', { name: /repl/ }).first().click();
    await panel.locator('.thread').waitFor();
    await page.getByRole('button', { name: 'Collapse the tools panel' }).click();
    expect('folding the panel away to close the thread in it', (await panel.count()) === 0 && (await page.locator('.thread').count()) === 0);
    await page.getByRole('button', { name: 'Expand the tools panel' }).click();
    await toolsAtRest(page).waitFor();
    expect(`the panel to come back without the thread, got ${await tabs()}`, (await tabs()).join() === 'Media,Board,Docs,Channel');
    // The board tab says there is none where no project of the workspace has one.
    await panel.getByRole('tab', { name: 'Board' }).click();
    await panel.getByText('No project of this workspace has a board yet').waitFor();
  }

  // What a blueprint says is said by the factory's one bot, which is in the
  // channel from then on, a member like any other. Which blueprint had it
  // said is no part of the channel; a message that keeps more than it says
  // (the whole of what went wrong) opens to that.
  await writeFile(
    join(out, 'plain.bp'),
    ['blueprint: plain', 'description: Says the night went well.', 'permissions: [channels]', 'steps:', '  told:', '    do: channel.post', '    with:', '      channel: general', '      body: "The nightly build passed."', ''].join('\n'),
  );
  await writeFile(
    join(out, 'nightly.bp'),
    [
      'blueprint: nightly',
      'description: Says how the night went.',
      'permissions: [channels]',
      'steps:',
      '  told:',
      '    do: channel.post',
      '    with:',
      '      channel: general',
      '      body: "The nightly build failed: two tests of the cart."',
      '      detail: "step `tests` failed: cart::totals and cart::empties"',
      '',
    ].join('\n'),
  );
  await cli('apply', join(out, 'nightly.bp'));
  const told = inChannel.filter({ hasText: 'The nightly build failed' });
  await told.waitFor();
  expect(`what a blueprint says to be under the bot's name, got "${await told.locator('.who').last().innerText()}"`, (await told.locator('.who').last().innerText()) === 'conductor');
  expect('it to be marked as a bot\'s', (await told.locator('.when', { hasText: 'bot' }).count()) === 1);
  // It is the bot's message, as anybody's is theirs: nothing in the channel says which blueprint had it said, pointed at or not.
  await told.hover();
  expect('nothing on the message to open what said it', (await told.getByRole('button', { name: 'What said this' }).count()) === 0 && (await told.locator('.said-by').count()) === 0);
  await told.getByRole('button', { name: 'Details' }).click();
  const opened = (await told.locator('.said-by').innerText()).replace(/\s+/g, ' ');
  expect(`the message opened to the whole of what went wrong, and to name no blueprint, step or run, got "${opened}"`, opened.includes('cart::totals and cart::empties') && !opened.includes('Said by') && !opened.includes('nightly') && !/run_/.test(opened));
  expect('a way from there to what was running, for whoever wants it', (await told.locator('.said-by').getByRole('button', { name: 'Open what was running' }).count()) === 1);
  // A message that keeps no more than it says has nothing to open.
  await cli('apply', join(out, 'plain.bp'));
  const plain = inChannel.filter({ hasText: 'The nightly build passed.' });
  await plain.waitFor();
  await plain.hover();
  expect('a message with nothing more to it to have nothing to open', (await plain.getByRole('button', { name: /Details|What said this/ }).count()) === 0);
  await everyone.click();
  const members = (await page.getByRole('menu', { name: 'Participants' }).locator('.person').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim());
  expect(`the bot to be among the channel's members, as a bot, got ${members.join(' | ')}`, members.some((one) => /^CO conductor bot$/.test(one)));
  await page.waitForTimeout(250);
  await page.keyboard.press('Escape');
  await page.getByRole('menu', { name: 'Participants' }).waitFor({ state: 'detached' });
  // Spoken to, it takes a command and nothing else; and says so.
  await box.click();
  await page.keyboard.type('@conductor what broke last night?');
  await page.keyboard.press('Escape');
  await page.keyboard.press('Enter');
  const taken = inChannel.filter({ hasText: 'I did not take that as a command.' });
  await taken.waitFor();
  expect('the bot to say what it does take, and who to ask instead', (await taken.innerText()).includes('board move') && (await taken.innerText()).includes('ask an agent'));
  await taken.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/channel-20-conductor.png` });

  // The bot is read about like anybody: pressed by its picture, it is
  // under Channel in the panel, with what it takes and what it said here
  // lately, each with the blueprint that had it said. That is where a
  // blueprint is named, and not in the channel.
  {
    await told.locator('.avatar').last().click();
    const panel = page.getByRole('complementary', { name: 'Tools' });
    await panel.getByRole('navigation', { name: 'Where this is' }).getByText('conductor', { exact: true }).waitFor();
    const about = (await panel.locator('.context-about').innerText()).replace(/\s+/g, ' ');
    expect(`the bot's commands to be listed where it is read about, got "${about}"`, about.includes('@conductor board move <card> <column>') && about.includes('move a card') && about.includes('@conductor status'));
    expect(`what it said here lately to say which blueprint had it said, got "${about}"`, about.includes('The nightly build failed') && about.includes('from nightly') && about.includes('from plain'));
    await page.screenshot({ path: `${out}/channel-20-conductor-profile.png` });
    // A command pressed there is begun where the message is written.
    await box.fill('');
    await panel.locator('.context-command', { hasText: 'agent list' }).click();
    expect(`a command pressed to be begun in the message, got "${await box.inputValue()}"`, (await box.inputValue()) === '@conductor agent list ');
    await box.fill('');
    // Written to, it offers what it takes: narrowed as more is typed, put in with Tab, and gone once it is past them.
    await box.click();
    await page.keyboard.type('@conductor ');
    const offers = page.getByRole('listbox', { name: 'What conductor takes' });
    await offers.waitFor();
    const offered = async () => (await offers.getByRole('option').locator('code').allInnerTexts()).map((one) => one.trim());
    expect(`all it takes to be offered once it is named, got ${(await offered()).join(' | ')}`, (await offered()).length === 38 && (await offered()).includes('board move <card> <column> [--note TEXT] [--give N]') && (await offered()).includes('board press <card> <action> [--back]'));
    await page.keyboard.type('bo');
    expect(`what is typed to narrow them, got ${(await offered()).join(' | ')}`, (await offered()).join(' | ') === 'board <project> | board columns <project> [<column>...] | board add <project> <title...> [--body TEXT] [--status COLUMN] [--label L]... [--field name=value]... | board move <card> <column> [--note TEXT] [--give N] | board give <card> <agent>|<person>|nobody | board set <card> <field=value>... | board press <card> <action> [--back] | board history <card> | board cost <card>');
    await page.screenshot({ path: `${out}/channel-20-conductor-offers.png` });
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Tab');
    expect(`Tab to put the marked one's words in, got "${await box.inputValue()}"`, (await box.inputValue()) === '@conductor board move ');
    await page.keyboard.type('shop-1 ready');
    expect('nothing to be offered once what is typed is past them', (await offers.count()) === 0);
    await box.fill('');
    // Sent while what it takes is offered, what was offered goes with what was written.
    await box.click();
    await page.keyboard.type('@conductor status');
    await offers.waitFor();
    await page.keyboard.press('Enter');
    for (let tries = 0; (await box.inputValue()) !== '' && tries < 100; tries++) await page.waitForTimeout(100);
    expect(`the message to be sent, got "${await box.inputValue()}"`, (await box.inputValue()) === '');
    expect('what the bot takes to be offered no longer once the message is sent', (await offers.count()) === 0);
    await page.getByRole('button', { name: 'Collapse the tools panel' }).click();
  }

  // What a bot says again and again, folded under the latest of it (`updates.mjs`): after what the bot did lately is read, which it would fill.
  await updates({ page, out, expect, cli, inChannel });
  // A failure said here opens the run that failed, at its step (`failedrun.mjs`).
  await failedRun({ page, out, expect, cli, binary });

  // ---- who is in it ----------------------------------------------------------
  // Listed under its head, with a long name among them (`participants.mjs`).
  await participants({ page, out, expect, cli, who: { email: 'ada@example.com', password: 'correct horse' } });

  // What a channel is for, under what it is called, with nothing at the head's right moved (`topic.mjs`).
  await topic({ page, out, expect, cli, who: { email: 'ada@example.com', password: 'correct horse' } });

  // The channel is moved to the Trash from its row's menu. A bot says it posts
  // here and is running, so the owner is told and asked once more; then
  // the channel is gone, its tab with it, and this browser keeps nothing
  // of it: not the thread that was open in it.
  const general = JSON.parse((await cli('channel', 'list', '--json')).stdout).channels.find((one) => one.name === 'general').id;
  const keptHere = () => page.evaluate(() => Object.values({ ...localStorage }).concat(Object.values({ ...sessionStorage })).join('\n'));
  await side(page, 'Channels');
  const channelRow = page.getByRole('navigation', { name: 'Channels' }).locator('.grouped-channel', { has: page.getByRole('button', { name: '# general', exact: true }) });
  await channelRow.click({ button: 'right' });
  await page.getByRole('menu').getByText('Move to Trash…', { exact: true }).click();
  const going = page.getByRole('dialog');
  await going.getByText('Move #general to the Trash?', { exact: true }).waitFor();
  expect('to be told what goes with it', (await going.innerText()).includes('Its messages go with it, and everything attached to them'));
  await going.getByRole('button', { name: 'Move to Trash', exact: true }).click();
  await going.getByRole('alert').waitFor();
  const held = (await going.getByRole('alert').innerText()).replace(/\s+/g, ' ');
  expect(`to be told what posts there and is running, got "${held}"`, held.includes('#general is where') && held.includes('std/bot') && held.includes('running'));
  expect('the channel to be there still', (await channelRow.count()) === 1);
  await page.screenshot({ path: `${out}/channel-21-removing.png` });
  await going.getByRole('button', { name: 'Move it all the same', exact: true }).click();
  await going.waitFor({ state: 'detached' });
  await channelRow.waitFor({ state: 'detached' });
  expect('its tab to be closed with it', (await page.getByRole('heading', { name: '# general' }).count()) === 0);
  expect('nothing of it to be kept in this browser', !(await keptHere()).includes(general));

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
