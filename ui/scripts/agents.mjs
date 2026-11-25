// The agents on the Members page's Agents tab, the history of one, and one removed for
// good.
//
// The page is the Members line's under Factory in the Resources panel, and its Agents tab is of the
// workspace the panel is of by the choice Activity and Measurements share
// (`narrowed.mjs` holds that); here All workspaces is chosen, which is
// every agent there is. The list: a band for each project that has an agent, in the order the
// side panel lists the projects, and a last one for the factory's own,
// each saying how many, those that are stopped last in theirs; a line's
// parts (what it runs on, what it is doing, its permissions in a few
// words under that word, and the menu, which is all that is pressed at its end), none over another and none past its line
// at three widths; working said, with where, while a stand-in has a turn
// in hand, on a page that heard it begin and on one that read it, and
// idle after; what an agent may ask opened by area in one panel over the
// page, in the window, moving nothing, staying as the pointer crosses to
// it, put away by Escape, a press outside and its own button, and
// nothing in it to change; the menu at a line's end, each line drawn,
// its history first, Stop, and Remove… set apart under it.
//
// The history, in the drawer, from the menu by pointer and keyboard: its head's counts against what the lines
// say; each bar as long as its turn's tokens, in the order of the
// figures and none wider than its track; a turn that says no tokens with
// a dash and no bar; the card of a turn's exact figures, beside the
// drawer, moving nothing and staying as the pointer crosses to it; a
// line opened and closed by the pointer and by the keyboard, one at a
// time, and still open when the list is read again for a turn that came
// meanwhile; a turn a message asked for opening on the very words that
// were said in its channel, as words, with what it answered under them
// and the whole of what it was sent folded away behind one line, pressed
// and keyed to be read and put away; a piece of work a step asked for,
// with no words handed apart, opening as a line always did; a turn in hand; a turn handed without its words, which is a
// line with its figures that opens to nothing (the answer is made up
// here: nothing keeps a channel to its members yet), in a list as long
// as the daemon hands, which says that it is the latest and is under a
// heading for each day; an agent never
// asked; the same at a phone's width, and in the other of dark and light.
//
// The drawer itself, which the history is opened in: it comes in from
// the right edge by where it is drawn and by nothing else, held part of
// the way here to be looked at, with nothing behind it
// moved and nothing scrolled; it says that it is at rest only once it
// is, where it always was; handed other contents it is the same drawer
// and does not come in again; where less movement is asked for it
// appears, at rest as it is drawn; and on a phone it comes in the same.
//
// And removing one: the entry at the end of each line, offered to
// whoever manages the factory and not to a member, whose menu offers
// her its history and moving it, and who opens Settings as soon as the frame is there,
// before the tabs kept for her are put back (the reading of the
// workspaces held until she has), and has it, and the Agents tab of Members after; the question asked first, which says what goes with the
// agent and what stays; why not, where one is counted by a fleet or has
// work in hand; one taken away, its line gone on a second open page when
// the daemon says so, no longer a member in the Tools panel beside its
// channel, said to be gone where it was being read about and nothing to
// press where it is named in what it said.
//
// And the Theme panel, which stays in Settings, by its boxes: its
// heading, what is said beside it and each theme, each as far as its
// words reach, none over another or past the panel, at 1280, 900 and 820
// and on a phone.
//
//   DAEMON=../target/debug/factory node scripts/agents.mjs [out-dir]
//
// Every agent here is the built-in assistant, a few lines of shell that
// wait to be let go, or a few that say what their turn took as an
// agent's program does: no agent's program is started. The environment
// variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { askRemoval, menuOpener, openHistory, openMenu } from './agentmenu.mjs';
import { agentsPage, linesCrowded, themeHeld } from './apart.mjs';
import { DRAWER_AT_REST, drawerAtRest, watchComing, watchLeaving, toolsAtRest } from './drawer.mjs';
import { talliesFor, SIZES } from './agentstandin.mjs';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { fillsPage } from './filled.mjs';
import { unfoldAll, unfoldChannels } from './folded.mjs';
import { beforeEntering } from './entering.mjs';
import { saysWhere } from './where.ts';
const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'agents-data');
const home = join(out, 'agents-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// A stand-in for an agent's program that reads what it is asked and then
// waits to be let go, so that its agent has that in hand for as long as
// the check needs. It looks outside its project for the word, so the
// daemon holds no walls.
const gate = join(home, 'let-go');
const waits = join(home, 'waits.sh');
await writeFile(waits, `#!/bin/sh\ncat >/dev/null\nwhile [ ! -e '${gate}' ]; do sleep 0.1; done\necho Here it is.\n`);
await chmod(waits, 0o755);
// And one that says what its turn took, as an agent's program does: how much by the last `size=` it was asked with, which is
// the newest thing said to it. One size says nothing of tokens or cost (not reported, never nought), and one fails.
const tells = join(home, 'tells.sh');
await writeFile(
  tells,
  `#!/bin/sh
size=$(cat | grep -o 'size=[a-z]*' | tail -n 1)
case "$size" in
  size=large) cost=0.41; read=120000; wrote=9000 ;;
  size=middling) cost=0.09; read=40000; wrote=2500 ;;
  size=small) cost=0.004; read=900; wrote=100 ;;
  size=fails) printf '{"kind":"end","text":"It cannot be counted.","bad":true}\\n'; exit 0 ;;
  *) printf '{"kind":"end","text":"Nothing to count."}\\n'; exit 0 ;;
esac
printf '{"kind":"end","text":"Counted.","cost":%s,"in":%s,"out":%s}\\n' "$cost" "$read" "$wrote"
`,
);
await chmod(tells, 0o755);
// A piece of work for the one that tells what its turns took, asked by a step and by no message.
const asks = join(home, 'asks.bp');
await writeFile(
  asks,
  `blueprint: asks
version: 1
description: Asks an agent for a piece of work.
steps:
  ask: { do: agent.task, with: { slot: teller, prompt: "Count the shelf, size=small" } }
`,
);
const TALLIES = talliesFor(tells);
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--command', `waits=${waits}`, '--walls', 'off'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  // A project as one is made: its board, and its fleet with an architect in its channel and one who works. Two more with nothing
  // of that: one with two agents, and one with none, which has no band.
  await cli('project', 'create', 'Shop');
  await cli('project', 'create', '--bare', 'Atlas');
  await cli('project', 'create', '--bare', 'Barn');
  // Added by hand: one that has said something and been given a card, one that will have work in hand, two of another project, one in
  // no project, and one in none that says what its turns took and holds other than the usual.
  await cli('agent', 'add', 'helper', '--project', 'shop', '--channel', 'shop');
  await cli('agent', 'add', 'maker', '--adapter', 'waits', '--project', 'shop');
  await cli('agent', 'add', 'loner');
  await cli('agent', 'add', 'mapper', '--project', 'atlas');
  await cli('agent', 'add', 'plotter', '--project', 'atlas');
  await cli('channel', 'create', 'workshop');
  await cli('agent', 'add', 'maker', '--channel', 'workshop');
  await mkdir(join(home, 'tallies'), { recursive: true });
  await writeFile(join(home, 'tallies', 'extension.yaml'), TALLIES);
  await cli('extension', 'add', join(home, 'tallies'));
  await cli('channel', 'create', 'counting');
  await cli('agent', 'add', 'teller', '--adapter', 'tallies', '--channel', 'counting', '--scope', 'channels', '--scope', 'board:read');
  await cli('board', 'add', 'shop', 'Paint', 'the', 'door');
  await cli('board', 'give', 'shop-1', 'helper');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const scheme = process.env.SCHEME ?? 'dark';
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async (on, email) => {
    on.on('pageerror', (err) => problems.push(err.message));
    await on.goto(url);
    await on.getByLabel('Email').fill(email);
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  /** Waits for what the daemon says of something, asked of it until it is so. */
  const until = async (what, ask) => {
    let last;
    for (let tries = 0; tries < 300; tries += 1) {
      last = await ask();
      if (last === true) return;
      await new Promise((later) => setTimeout(later, 100));
    }
    throw new Error(`expected ${what}, and it is ${JSON.stringify(last)}`);
  };
  /** How many of an agent's turns have ended, well or not, as the daemon lists them. */
  const ended = async (name) => ((await cli('agent', 'turns', name)).stdout.match(/\b(done|failed)\b/g) ?? []).length;
  /** Opens the Members page's Agents tab of every workspace (`apart.mjs`). */
  const agentsOf = (on, press = 'click') => agentsPage(on, out, press);
  const line = (on, name) => on.locator(`.agent-line[data-agent="${name}"]`);
  const listed = async (on) => on.locator('.agent-line').evaluateAll((rows) => rows.map((row) => row.dataset.agent));
  /** The bands as they are drawn: whose each is, what it is called, how many it says, and its agents in order. */
  const bands = async (on) =>
    on.locator('.agents-of').evaluateAll((all) =>
      all.map((band) => ({
        of: band.dataset.of,
        name: band.querySelector('h3 > .truncate').textContent,
        count: band.querySelector('.agents-count').textContent,
        agents: [...band.querySelectorAll('.agent-line')].map((row) => row.dataset.agent),
      })),
    );
  const doing = async (on, name) => (await line(on, name).locator('.agent-doing').innerText()).trim();
  // The menu at the end of a line, the history from it by `click`, `tap` or `keys`, and the question that removes one (`agentmenu.mjs`).
  const [more, history, ask] = [openMenu, (on, name, how = 'click') => openHistory(on, name, how, expect), askRemoval];
  /** Nothing of a dialog outside the window, or under anything else of it. */
  const within = async (on, dialog, what) => {
    const box = await dialog.boundingBox();
    const size = on.viewportSize();
    expect(`${what} to be inside the window, and it is at ${JSON.stringify(box)} of ${JSON.stringify(size)}`, box.x >= 0 && box.y >= 0 && box.x + box.width <= size.width && box.y + box.height <= size.height);
    const wide = await on.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`nothing to scroll sideways under ${what}, and the page is ${wide} wider than the window`, wide <= 0);
    for (const name of await dialog.getByRole('button').allInnerTexts()) {
      const button = await dialog.getByRole('button', { name, exact: true }).boundingBox();
      expect(`\`${name}\` of ${what} to be in the window`, button && button.x >= 0 && button.x + button.width <= size.width && button.y + button.height <= size.height);
    }
  };
  /** The parts of each line by their boxes (`apart.mjs`). */
  const apart = async (on, lines, parts, what) => {
    const wrong = await linesCrowded(on, lines, parts);
    expect(`nothing of ${what} over anything else, past its line or scrolling sideways, got ${JSON.stringify(wrong)}`, wrong.length === 0);
  };
  /** Settings' Theme panel by its boxes (`apart.mjs`), with the Members tab shown again after. */
  const themed = (on, what, press = 'click') => themeHeld(on, what, expect, press);
  const LINE = ['.agent-name', '.agent-runs', '.agent-doing', '.agent-may', '.agent-acts'];
  const TURN = ['.turn-dot', '.turn-said', '.turn-track', '.turn-figure'];
  /** Where each line of the list is, to say afterwards that nothing moved. */
  const places = async (on, lines) => JSON.stringify(await on.locator(lines).evaluateAll((all) => all.map((one) => Object.values(one.getBoundingClientRect().toJSON()).map(Math.round))));
  const inWindow = async (on, what, called) => {
    const box = await what.boundingBox();
    const size = on.viewportSize();
    expect(`${called} to be in the window, and it is at ${JSON.stringify(box)} of ${JSON.stringify(size)}`, box && box.x >= 0 && box.y >= 0 && box.x + box.width <= size.width && box.y + box.height <= size.height);
  };

  // helper is asked something in its channel, and answers: what it said is there, and it was asked one piece of work.
  await cli('post', 'shop', '@helper what can you do?');
  await until('helper to have answered', async () => /\bdone\b/.test((await cli('agent', 'turns', 'helper')).stdout));
  // teller is asked five things, one after another: the largest first, then a small one, one that says no tokens, one between, and one that fails.
  const ASKED = ['large', 'small', 'none', 'middling', 'fails'];
  for (const [at, size] of ASKED.entries()) {
    await cli('post', 'counting', `@teller size=${size} how many is that?`);
    await until(`teller to have ended ${at + 1} turns`, async () => (await ended('teller')) === at + 1 || (await ended('teller')));
  }

  // ---- the owner: the list by project, and a line's parts -------------------------------
  const page = await context.newPage();
  await signIn(page, 'ada@example.com');
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(page);
  await page.locator('.nav [data-project="shop"]').waitFor();
  // The order the side panel lists the projects in is the order of the bands, with no band for a project nobody is of, and the factory's own last.
  const projects = await page.locator('.nav [data-project]').evaluateAll((all) => all.map((one) => one.dataset.project));
  expect(`the side panel to list the three projects, got ${projects}`, [...projects].sort().join() === 'atlas,barn,shop');
  await agentsOf(page);
  await line(page, 'loner').waitFor();
  await fillsPage(page, "the Members page's Agents tab", page.locator('.agents-page'), { ready: line(page, 'loner'), shot: `${out}/agents-filled` });
  const of = { shop: ['helper', 'maker', 'shop-architect-1', 'shop-worker-1'], atlas: ['mapper', 'plotter'], '': ['loner', 'teller'] };
  const order = [...projects.filter((id) => id in of), ''];
  const all = order.flatMap((id) => of[id]);
  let drawn = await bands(page);
  expect(`a band for each project that has an agent, in the side panel's order, and the factory's own last, got ${JSON.stringify(drawn)}`, drawn.map((band) => band.of).join() === order.join());
  expect(
    `each band to be called what its project is, and the last what the side panel calls the factory's own, got ${JSON.stringify(drawn)}`,
    drawn.every((band) => band.name === { shop: 'Shop', atlas: 'Atlas', '': 'Factory' }[band.of]) && (await page.locator('.nav h3', { hasText: 'Factory' }).count()) >= 1,
  );
  expect(`each band to say how many it has, got ${JSON.stringify(drawn)}`, drawn.every((band) => band.count === (of[band.of].length === 1 ? '1 agent' : `${of[band.of].length} agents`)));
  expect(`each band to hold its own agents by name, got ${JSON.stringify(drawn)}`, drawn.every((band) => band.agents.join() === of[band.of].join()));
  expect(`every agent to be listed, got ${await listed(page)}`, (await listed(page)).join() === all.join());
  for (const name of all) expect(`${name} to have the entry at the end of its line`, (await line(page, name).getByRole('button', { name: `More that can be done with ${name}`, exact: true }).count()) === 1);
  // A line: what it may ask as words that are pressed, and one quiet thing for the rest, its history among it. Nothing else is a button of the line.
  const loud = await line(page, 'helper').getByRole('button').allInnerTexts();
  expect(`the line's buttons to be what it may ask and the menu, got ${JSON.stringify(loud)}`, loud.join('|') === 'the usual|⋯');
  // What it runs on: what its provider is called, never the id of a package.
  const runs = async (name) => (await line(page, name).locator('.agent-runs').innerText()).trim();
  expect(`teller to be said to run on what its package is called, got "${await runs('teller')}"`, (await runs('teller')) === 'Tally');
  expect(`helper to be said to run on the built-in assistant, got "${await runs('helper')}"`, (await runs('helper')) === 'Built-in assistant');
  // What each is doing: a mark and a few words, and no badge.
  expect(`helper to be idle since a moment ago, got "${await doing(page, 'helper')}"`, /^idle · (just now|\dm ago)$/.test(await doing(page, 'helper')));
  expect(`teller to be idle since a moment ago, got "${await doing(page, 'teller')}"`, /^idle · (just now|\dm ago)$/.test(await doing(page, 'teller')));
  expect(`one never asked to be idle and no more, got "${await doing(page, 'loner')}"`, (await doing(page, 'loner')) === 'idle');
  expect('the mark of one that is idle', (await line(page, 'loner').locator('.agent-doing .standing.idle').count()) === 1 && (await page.locator('.agent-line .badge').count()) === 0);
  // What each may ask, short.
  const may = async (on, name) => (await line(on, name).locator('.agent-may').innerText()).trim();
  expect(`one that holds what an agent usually does to say so, got "${await may(page, 'helper')}"`, (await may(page, 'helper')) === 'the usual');
  expect(`one that holds something else to say the areas, got "${await may(page, 'teller')}"`, (await may(page, 'teller')) === '2 areas: channels, board');
  await apart(page, '.agent-line', LINE, 'the list at 1280');
  await themed(page, 'at 1280');
  // What each part of a line is, is said once over the lines, each word over its own part: what an agent may ask is its permissions.
  const heads = await page.locator('.agent-heads > span').evaluateAll((all) => all.map((one) => [one.textContent, Math.round(one.getBoundingClientRect().left)]));
  const lefts = await line(page, 'teller').evaluate((row) => ['.agent-name', '.agent-runs', '.agent-doing', '.agent-may'].map((part) => Math.round(row.querySelector(part).getBoundingClientRect().left)));
  expect(`the words over the lines, got ${JSON.stringify(heads)}`, heads.map(([word]) => word).join('|') === 'Agent|Runs on|Now|Permissions|');
  // The words of what is pressed begin a little inside it, so that it lines up with what is over it by its words.
  expect(`each word to be over its part of a line, at ${JSON.stringify(heads)} over ${lefts}`, lefts.every((left, at) => Math.abs(left - heads[at][1]) <= (at === 3 ? 6 : 1)));
  await line(page, 'helper').screenshot({ path: `${out}/agents-0a-line.png` });
  // The menu: its history first, stopping, and under it, set apart as what cannot be undone, taking away; each with a drawing before it.
  await more(page, 'helper');
  const offered = await page.getByRole('menuitem').evaluateAll((items) => items.map((item) => [item.innerText.trim(), item.className, item.querySelectorAll('.menu-sign svg').length]));
  const [said, grave] = [offered.map(([words]) => words).join('|'), offered[4]?.[1] ?? ''];
  expect(`the menu to offer Edit…, History, Stop, moving it, then set apart and grave taking away, each with its drawing, got ${JSON.stringify(offered)}`, said === 'Edit…|History|Stop|Move to project…|Remove…' && offered[2][1] === '' && grave.includes('apart') && grave.includes('menu-grave') && offered.every((item) => item[2] === 1));
  // Opened by the pointer it has the keyboard too, at its first line, and pressed again it is put away.
  await page.waitForFunction(() => document.activeElement?.closest('.row-menu') !== null, null, { timeout: 5000 }).catch(() => undefined);
  expect('the menu to have the keyboard at Edit…', await page.getByRole('menuitem', { name: 'Edit…', exact: true }).evaluate((one) => document.activeElement === one));
  await inWindow(page, page.getByRole('menu'), 'the menu of a line');
  await page.screenshot({ path: `${out}/agents-0-list.png` });
  await menuOpener(page, 'helper').click();
  await page.getByRole('menu').waitFor({ state: 'detached' });
  await more(page, 'helper');
  await page.waitForTimeout(200); // A menu listens for Escape a moment after it is drawn.
  await page.keyboard.press('Escape');
  await page.getByRole('menu').waitFor({ state: 'detached' });
  expect('Escape to give the keyboard back to what opened the menu', await menuOpener(page, 'helper').evaluate((one) => document.activeElement === one));

  // ---- what one may ask, whole: a panel over the page ------------------------------------
  const before = await places(page, '.agent-line');
  const pressed = line(page, 'teller').locator('.agent-may');
  await pressed.click();
  const holds = page.getByRole('dialog', { name: 'Permissions of teller' });
  await holds.waitFor();
  await inWindow(page, holds, 'the permissions of teller');
  expect('nothing of the list to have moved when it opened', (await places(page, '.agent-line')) === before);
  const held = Object.fromEntries(await holds.locator('.holds-table tr').evaluateAll((rows) => rows.map((row) => [row.querySelector('th').textContent, row.querySelector('td').textContent])));
  expect(`the whole by area, as the Tools panel draws it, got ${JSON.stringify(held)}`, held.board === 'read' && held.channels === 'read and change' && held.records === 'nothing' && Object.keys(held).length > 6);
  expect(`the panel to be called its permissions, got "${await holds.locator('strong').innerText()}"`, (await holds.locator('strong').innerText()) === 'Permissions of teller');
  expect('nothing there to change it by', (await holds.locator('select, input').count()) === 0 && (await holds.getByRole('button').count()) === 0);
  // It is under what was pressed, or over it where there is no room under, and stays as the pointer crosses to it.
  const [from, to] = [await pressed.boundingBox(), await holds.boundingBox()];
  const [under, over] = [to.y - (from.y + from.height), from.y - (to.y + to.height)];
  expect(`the panel to be just under what opened it or just over it, at ${JSON.stringify(to)} by ${JSON.stringify(from)}`, (under >= 0 && under < 12) || (over >= 0 && over < 12));
  await page.mouse.move(from.x + 10, from.y + from.height / 2);
  await page.mouse.move(to.x + 30, to.y + 30, { steps: 6 });
  await page.waitForTimeout(300);
  expect('the panel to be there still with the pointer on it', await holds.isVisible());
  await page.screenshot({ path: `${out}/agents-1-may-ask.png` });
  // Escape puts it away and gives the keyboard back; a press outside does; and so does what opened it.
  await page.keyboard.press('Escape');
  await holds.waitFor({ state: 'detached' });
  expect('the keyboard to be back on what opened it', await pressed.evaluate((button) => document.activeElement === button));
  await pressed.click();
  await holds.waitFor();
  await page.getByRole('heading', { name: 'Members', level: 1 }).click();
  await holds.waitFor({ state: 'detached' });
  await pressed.click();
  await holds.waitFor();
  expect('what opened it to say that it is open', (await pressed.getAttribute('aria-expanded')) === 'true');
  await pressed.click();
  await holds.waitFor({ state: 'detached' });
  expect('nothing of the list to have moved by any of it', (await places(page, '.agent-line')) === before);

  // A second page is open on the same list, and a third beside the channel, reading about helper in the Tools panel.
  const second = await context.newPage();
  second.on('pageerror', (err) => problems.push(err.message));
  await second.goto(url);
  await second.locator('.rail').waitFor();
  await agentsOf(second);
  await line(second, 'helper').waitFor();
  const third = await context.newPage();
  third.on('pageerror', (err) => problems.push(err.message));
  await third.goto(url);
  await third.locator('.rail').waitFor();
  const channels = third.getByRole('button', { name: 'Channels', exact: true });
  if ((await channels.getAttribute('aria-pressed')) !== 'true') await channels.click();
  // Every branch of the channels starts folded (`folded.mjs`): the project's, which holds `shop`, is unfolded as a person would.
  await unfoldChannels(third);
  await third.locator('.nav').getByText('shop', { exact: true }).click();
  await third.getByRole('heading', { name: '# shop' }).waitFor();
  const panel = third.getByRole('complementary', { name: 'Tools' });
  if ((await panel.count()) === 0) await third.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(third).waitFor();
  await panel.waitFor();
  await panel.getByRole('tab', { name: /^Channel\b/ }).click();
  await panel.locator('.context-agent', { hasText: 'helper' }).waitFor();
  // By its picture beside what it said, it is pressed to be read about.
  const named = third.locator('.messages .message', { has: third.locator('.who.agent', { hasText: 'helper' }) }).first().locator('.who-anchor');
  await named.waitFor();
  expect('an agent that is there to be pressed where it is named', (await named.getAttribute('role')) === 'button');
  await panel.locator('.context-agent', { hasText: 'helper' }).click();
  await panel.locator('.context-about').getByText('helper', { exact: true }).waitFor();

  // ---- Stop, from the menu: stopped where it is listed, and last in its band ---------------
  await more(page, 'mapper');
  await page.getByRole('menuitem', { name: 'Stop', exact: true }).click();
  await line(page, 'mapper').locator('.agent-doing[data-doing="stopped"]').waitFor();
  expect(`one that was stopped to say so, got "${await doing(page, 'mapper')}"`, (await doing(page, 'mapper')) === 'stopped');
  of.atlas = ['plotter', 'mapper'];
  const stopped = order.flatMap((id) => of[id]);
  expect(`one that is stopped to be last in its band, got ${await listed(page)}`, (await listed(page)).join() === stopped.join());
  // The second page was told by the daemon.
  await line(second, 'mapper').locator('.agent-doing[data-doing="stopped"]').waitFor();
  expect(`the same on a second page, got ${await listed(second)}`, (await listed(second)).join() === stopped.join());
  // One that is stopped is not offered stopping.
  await more(page, 'mapper');
  expect('a stopped one to be offered moving and taking away, and no stopping', (await page.getByRole('menuitem').allInnerTexts()).join('|') === 'Edit…|History|Move to project…|Remove…');
  // A menu listens for Escape a moment after it is drawn.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.getByRole('menu').waitFor({ state: 'detached' });
  expect('the daemon to have it still, stopped', /mapper.*stopped/.test((await cli('agent', 'list')).stdout));

  // ---- why not: one that a fleet counts ---------------------------------------------
  let dialog = await ask(page, 'shop-worker-1');
  await dialog.getByText('It cannot be removed as things stand.').waitFor();
  const counted = await dialog.locator('.take-away-why').innerText();
  expect(
    `the question to say what counts it and where it is changed, got "${counted}"`,
    counted.startsWith('shop-worker-1 is brought up by') && counted.includes('(std/role, in_') && counted.includes('in the agents table of the project shop, lower the row\'s count or take the row away'),
  );
  expect('nothing to take it away by', (await dialog.getByRole('button', { name: 'Remove it' }).count()) === 0 && (await dialog.getByRole('button', { name: 'Close', exact: true }).count()) === 1);
  await within(page, dialog, 'the question of one a fleet counts');
  await page.screenshot({ path: `${out}/agents-2-counted.png` });
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });

  // ---- working, while a stand-in has a turn in hand, and idle after -------------------
  expect(`maker to be idle and no more before it is asked, got "${await doing(page, 'maker')}"`, (await doing(page, 'maker')) === 'idle');
  await cli('post', 'workshop', '@maker three palettes, please');
  await until('maker to have that in hand', async () => /\brunning\b/.test((await cli('agent', 'turns', 'maker')).stdout));
  // The page that was open heard it begin, and says where.
  await line(page, 'maker').locator('.agent-doing[data-doing="working"]').waitFor();
  expect(`maker to be said to be at work in its channel, got "${await doing(page, 'maker')}"`, (await doing(page, 'maker')) === 'working in #workshop');
  expect('the mark of one at work', (await line(page, 'maker').locator('.agent-doing .standing.working').count()) === 1);
  // A page opened while it is at work heard nothing begin: it read it.
  await second.reload();
  await second.locator('.rail').waitFor();
  await agentsOf(second);
  await line(second, 'maker').locator('.agent-doing[data-doing="working"]').waitFor();
  expect(`the same on a page that read it, got "${await doing(second, 'maker')}"`, (await doing(second, 'maker')) === 'working in #workshop');
  await page.screenshot({ path: `${out}/agents-3-working.png` });
  // Its history has the one turn, in hand.
  const drawer = drawerAtRest(page);
  const turns = drawer.locator('.turn');
  const figure = async (what) => (await drawer.locator(`.turns-figures [data-figure="${what}"] dd`).innerText()).trim();
  const figures = async () => drawer.locator('.turns-figures > div').evaluateAll((all) => all.map((one) => `${one.querySelector('dt').textContent} ${one.querySelector('dd').textContent}`).join(', '));
  await history(page, 'maker');
  await drawer.getByRole('heading', { name: 'maker: history' }).waitFor();
  await turns.first().waitFor();
  expect(`one turn, in hand, asked in its channel, got ${await turns.first().innerText()}`, (await turns.count()) === 1 && (await turns.first().getAttribute('data-ended')) === 'in hand' && (await turns.first().locator('.turn-where').innerText()) === '#workshop');
  expect(`the head to count it as in hand, got ${await figures()}`, (await figures()) === 'Asked 1, Failed 0, In hand 1, Sent to it –, Written by it –');
  expect('a turn that says no tokens to have a dash and no bar', (await turns.first().locator('.turn-figure').innerText()) === '–' && (await turns.first().locator('.turn-bar').count()) === 0);
  expect('a list with no tokens in it to say so where the key would be', (await drawer.locator('.turns-key').innerText()) === 'No turn here says how many tokens it took.');

  // ---- why not: one with work in hand -------------------------------------------------
  await drawer.getByRole('button', { name: 'Close', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  dialog = await ask(page, 'maker');
  await dialog.getByText('It cannot be removed as things stand.').waitFor();
  const inHand = await dialog.locator('.take-away-why').innerText();
  expect(`the question to say it has work in hand and what to do, got "${inHand}"`, inHand.startsWith('maker has 1 piece of work in hand') && inHand.includes('agent stop maker'));
  expect('what is typed to be drawn as that', (await dialog.locator('.take-away-why code').count()) >= 2);
  expect('nothing to take it away by', (await dialog.getByRole('button', { name: 'Remove it' }).count()) === 0);
  await page.screenshot({ path: `${out}/agents-4-in-hand.png` });
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  expect(`nothing to have gone, got ${await listed(page)}`, (await listed(page)).join() === stopped.join());
  // Its history is open as it is let go: the line that was in hand is done, with nothing read by anybody's asking.
  await history(page, 'maker', 'keys');
  await drawer.locator('.turn[data-ended="in hand"]').waitFor();
  await writeFile(gate, '');
  await until('maker to have answered', async () => /\bdone\b/.test((await cli('agent', 'turns', 'maker')).stdout));
  await drawer.locator('.turn[data-ended="done"]').waitFor();
  expect(`the head to count nothing in hand any more, and its cost not reported and never nought, got ${await figures()}`, (await figures()) === 'Asked 1, Failed 0, Sent to it –, Written by it –, Cost not reported');
  await drawer.getByRole('button', { name: 'Close', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  // And it is idle, on the page that heard it end and on the one that read it at work.
  for (const on of [page, second]) {
    await line(on, 'maker').locator('.agent-doing[data-doing="idle"]').waitFor();
    expect(`maker to be idle since a moment ago, got "${await doing(on, 'maker')}"`, (await doing(on, 'maker')) === 'idle · just now');
  }

  // ---- the drawer comes in from the right edge, and says when it is at rest ---------------
  // What slides it in is held part of the way in the moment the drawer is put on the page, so that it is looked at on its way whatever
  // the machine is doing; and what is said of it then is kept, to be read afterwards.
  await watchComing(page);
  const behind = await places(page, '.agent-line');
  await history(page, 'teller');
  const coming = page.locator('.drawer');
  await coming.waitFor();
  const slid = await page.evaluate(() => window.slid);
  expect(
    `the drawer to come in by where it is drawn and nothing else, eased, in about a quarter of a second, got ${JSON.stringify(slid)}`,
    slid.name === 'drawer-in' && slid.took >= 240 && slid.took <= 280 && slid.moved.join() === 'transform' && slid.eased.startsWith('cubic-bezier'),
  );
  expect('a drawer on its way not to say that it is at rest', !slid.rested && (await drawer.count()) === 0);
  const room = await page.locator('.main').boundingBox();
  const onItsWay = await coming.boundingBox();
  expect(`the drawer to be part of the way out of the right edge, at ${JSON.stringify(onItsWay)} of ${JSON.stringify(room)}`, onItsWay.x < room.x + room.width - 40 && onItsWay.x + onItsWay.width > room.x + room.width + 40);
  expect('nothing of the page behind it to have moved', (await places(page, '.agent-line')) === behind);
  const pushed = await page.evaluate(() => [document.querySelector('.main').scrollLeft, document.documentElement.scrollWidth - window.innerWidth]);
  expect(`nothing to be scrolled sideways by it, got ${pushed}`, pushed[0] === 0 && pushed[1] <= 0);
  await page.screenshot({ path: `${out}/agents-4a-drawer-on-its-way.png` });
  await page.evaluate(() => {
    window.unwatched();
    document.querySelector('.drawer').getAnimations()[0].play();
  });
  await drawer.waitFor();
  const rests = await coming.evaluate((one) => [getComputedStyle(one).transform, one.getAnimations().length, Math.round(one.parentElement.getBoundingClientRect().right - one.getBoundingClientRect().right)]);
  expect(`the drawer at rest to be where it always was, moved by nothing any more, got ${rests}`, rests.join() === 'none,0,10');
  expect('nothing of the page behind it to have moved by the end', (await places(page, '.agent-line')) === behind);
  await page.screenshot({ path: `${out}/agents-4b-drawer-at-rest.png` });
  // Handed other contents while it is open, it is the same drawer and does not come in again. What opens another is under the drawer, so the keyboard presses it.
  await coming.evaluate((one) => (one.dataset.same = 'yes'));
  await history(page, 'maker', 'keys');
  await coming.getByRole('heading', { name: 'maker: history' }).waitFor();
  const handed = await coming.evaluate((one) => [one.dataset.same, 'rest' in one.dataset, one.getAnimations().length, one.querySelectorAll('.turn').length]);
  // And what it is handed is drawn afresh: under the second agent's name there is never the first one's list (which had five lines, where this one's has one or is being read).
  expect(`a drawer handed other contents to be the one that was open, at rest and not coming in again, with nothing of what it held, got ${handed}`, handed.slice(0, 3).join() === 'yes,true,0' && handed[3] <= 1);

  // ---- put away, it goes out the way it came, as a still copy ---------------------------
  // What is said of the copy is kept in the moment it is put on the page, with what moves it held there, so that it is looked at on
  // its way out whatever the machine is doing.
  await watchLeaving(page);
  const heldAt = await coming.boundingBox();
  await coming.getByRole('button', { name: 'Close', exact: true }).click();
  await page.locator('.drawer-leaving').waitFor();
  const went = await page.evaluate(() => window.left);
  expect(
    `a still copy of a drawer that is put away to go out the way it came, as nothing to press or reach, got ${JSON.stringify(went)}`,
    went.name === 'drawer-in' && went.backwards && went.took >= 240 && went.took <= 280 && went.inert && went.hidden === 'true' && went.pressed === 'none' && went.heading === 'maker: history' && went.found === 0,
  );
  expect('no drawer to be found while its copy goes out', (await page.locator('.drawer').count()) === 0 && (await drawer.count()) === 0);
  const going = await page.locator('.drawer-leaving').boundingBox();
  expect(`the copy to be part of the way out of the right edge, the size the drawer was, at ${JSON.stringify(going)} from ${JSON.stringify(heldAt)}`, going.x > heldAt.x + 40 && Math.abs(going.width - heldAt.width) < 1 && Math.abs(going.y - heldAt.y) < 1);
  expect('nothing of the page behind it to have moved as it goes', (await places(page, '.agent-line')) === behind);
  await page.screenshot({ path: `${out}/agents-4c-drawer-going-out.png` });
  // What is under the copy is pressed through it: the next drawer is opened by the pointer while the copy is still on the page.
  await history(page, 'teller');
  await drawer.getByRole('heading', { name: 'teller: history' }).waitFor();
  await page.evaluate(() => document.querySelector('.drawer-leaving')?.getAnimations()[0]?.play());
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });
  expect('one drawer, and no copy left behind', (await page.locator('.drawer').count()) === 1 && (await page.locator('.drawer-leaving').count()) === 0);

  // ---- an agent's history: what it comes to, and a line a turn with its bar ------------
  await history(page, 'teller', 'keys');
  await drawer.getByRole('heading', { name: 'teller: history' }).waitFor();
  await turns.nth(ASKED.length - 1).waitFor();
  /** Each line as it is drawn: how it ended, its tokens, its figure, how long its bar and its track are, and what it says. */
  const lines = async () =>
    turns.evaluateAll((all) =>
      all.map((one) => ({
        ended: one.dataset.ended,
        tokens: Number(one.dataset.tokens),
        figure: one.querySelector('.turn-figure').textContent.trim(),
        bar: one.querySelector('.turn-bar')?.getBoundingClientRect().width ?? null,
        parts: [...one.querySelectorAll('.turn-bar .turn-part')].map((part) => [part.className.replace('turn-part ', ''), part.getBoundingClientRect().width]),
        track: one.querySelector('.turn-track').getBoundingClientRect().width,
        right: Math.round(one.querySelector('.turn-figure').getBoundingClientRect().right),
        where: one.querySelector('.turn-where').textContent,
        when: one.querySelector('.turn-when').textContent,
      })),
    );
  let shown = await lines();
  // Newest first: what failed, the one between, the one with no tokens, the small one, the large one.
  const sizeOf = (size) => (SIZES[size] ? SIZES[size][0] + SIZES[size][1] : 0);
  const newest = [...ASKED].reverse();
  expect(`a line for each time it was asked, newest first, by its tokens, got ${JSON.stringify(shown)}`, shown.map((one) => one.tokens).join() === newest.map(sizeOf).join());
  expect(`each line to say how it ended, got ${shown.map((one) => one.ended)}`, shown.map((one) => one.ended).join() === 'failed,done,done,done,done');
  expect(`a line to say where it was asked, when and how long it took, and that it failed where it did, got ${JSON.stringify(shown[0])}`, shown.every((one) => one.where === '#counting' && /^\d{1,2}:\d\d (?:AM|PM) \S+ · \d/.test(one.when)) && shown[0].when.endsWith(' · failed') && !shown[1].when.includes('done'));
  expect(`the figures in a few letters, and a dash where a turn says no tokens, got ${shown.map((one) => one.figure)}`, shown.map((one) => one.figure).join() === '–,42.5k,–,1.0k,129k');
  // The bars: none where there are no tokens, none wider than its track, the largest the whole of it, and each as long as its turn is large.
  expect(`every track to be as wide, and the figures to end at one edge, got ${JSON.stringify(shown.map((one) => [one.track, one.right]))}`, new Set(shown.map((one) => one.track)).size === 1 && new Set(shown.map((one) => one.right)).size === 1);
  expect(`no bar where a turn says no tokens, and one where it says any, got ${JSON.stringify(shown.map((one) => one.bar))}`, shown.every((one) => (one.tokens === 0) === (one.bar === null)));
  const bars = shown.filter((one) => one.bar !== null);
  expect(`no bar wider than its track, the largest the whole of it and the smallest still seen, got ${JSON.stringify(bars.map((one) => one.bar))}`, bars.every((one) => one.bar <= one.track + 0.01 && one.bar >= 2) && bars.at(-1).bar === bars.at(-1).track);
  const bySize = [...bars].sort((one, other) => one.tokens - other.tokens);
  expect(`the bars' lengths to be in the order of their figures, got ${JSON.stringify(bySize.map((one) => [one.tokens, one.bar]))}`, bySize.every((one, at) => at === 0 || one.bar > bySize[at - 1].bar));
  // In proportion: the one between is as much of the track as its tokens are of the largest's, to the pixel.
  expect(`a bar to be as much of the track as its turn is of the largest, got ${shown[1].bar} of ${shown[1].track}`, Math.abs(shown[1].bar - Math.round((shown[1].track * sizeOf('middling')) / sizeOf('large'))) < 0.01);
  expect(`a bar to be what was sent to it and then what it wrote, each seen, got ${JSON.stringify(bars.map((one) => one.parts))}`, bars.every((one) => one.parts.length === 2 && one.parts[0][0] === 'sent' && one.parts[1][0] === 'written' && one.parts.every(([, width]) => width >= 1) && Math.abs(one.parts[0][1] + one.parts[1][1] - one.bar) < 0.01));
  // The head: counted from the same turns.
  const sent = Object.values(SIZES).reduce((sum, [read]) => sum + read, 0);
  const written = Object.values(SIZES).reduce((sum, [, wrote]) => sum + wrote, 0);
  expect(`the head to say how often it was asked and what that came to, got ${await figures()}`, (await figures()) === 'Asked 5, Failed 1, Sent to it 161k, Written by it 11.6k, Cost $0.504');
  expect('the head to say how many lines there are, and how many of them failed', Number(await figure('asked')) === shown.length && Number(await figure('failed')) === shown.filter((one) => one.ended === 'failed').length);
  const exact = async (what) => drawer.locator(`.turns-figures [data-figure="${what}"]`).getAttribute('title');
  expect(`the head's tokens to be the lines' added up, got ${await exact('sent')} and ${await exact('written')}`, (await exact('sent')) === `${sent.toLocaleString('en-US')} tokens` && (await exact('written')) === `${written.toLocaleString('en-US')} tokens` && shown.reduce((sum, one) => sum + one.tokens, 0) === sent + written);
  expect(`the head to say that this is every time, got "${await drawer.locator('.turns-of').innerText()}"`, (await drawer.locator('.turns-of').innerText()) === 'Every time it was asked.');
  const key = (await drawer.locator('.turns-key').innerText()).replace(/\s+/g, ' ').trim();
  expect(`one line of key for the bar's two parts, which says what a bar is against, got "${key}"`, key === "sent to it written by it. A bar is a turn's tokens against the largest here, 129k." && (await drawer.locator('.turns-key .turn-part').count()) === 2);
  expect('the turns to be under the day they were asked', (await drawer.locator('.turns-day > h3').evaluateAll((heads) => heads.map((head) => head.textContent))).join() === 'Today');
  await apart(page, `${DRAWER_AT_REST} .turn-line`, TURN, 'the history at 1280');
  // The bar is one quiet hue in two strengths, and no figure wears it: figures are in the text's own colour.
  const colours = async (on) =>
    on.evaluate(() => {
      const of = (selector, what = 'backgroundColor') => getComputedStyle(document.querySelector(selector))[what];
      const probe = document.body.appendChild(document.createElement('span'));
      probe.style.color = 'var(--text)';
      const text = getComputedStyle(probe).color;
      probe.style.color = 'var(--accent)';
      const accent = getComputedStyle(probe).color;
      probe.remove();
      return { sent: of('.turn-bar .turn-part.sent'), written: of('.turn-bar .turn-part.written'), figure: of('.turn-figure', 'color'), total: of('.turns-figures dd', 'color'), text, accent };
    });
  let hue = await colours(page);
  expect(`the bar's two parts to be told apart, neither the accent, and the figures in the text's colour, got ${JSON.stringify(hue)}`, hue.sent !== hue.written && hue.sent !== hue.accent && hue.written !== hue.accent && hue.figure === hue.text && hue.total === hue.text);

  // The card of a turn's exact figures: beside the drawer while its line is pointed at, moving nothing, and staying as the pointer crosses to it.
  const large = turns.nth(4);
  const unmoved = await places(page, `${DRAWER_AT_REST} .turn-line`);
  await large.locator('.turn-line').hover();
  const card = page.locator('.turn-exact');
  await card.waitFor();
  const cardSaid = (await card.locator('dl').evaluate((list) => [...list.children].map((one) => one.textContent))).join('|');
  expect(`the card to say the figures exactly, got "${cardSaid}"`, cardSaid === 'Sent to it|120,000 tokens|Written by it|9,000 tokens|Cost|$0.41');
  await inWindow(page, card, 'the card of exact figures');
  expect('nothing of the list to have moved under the card', (await places(page, `${DRAWER_AT_REST} .turn-line`)) === unmoved);
  const [cardAt, drawerAt, lineAt] = [await card.boundingBox(), await drawer.boundingBox(), await large.locator('.turn-line').boundingBox()];
  expect(`the card to be beside the drawer and over none of its lines, at ${JSON.stringify(cardAt)} beside ${JSON.stringify(drawerAt)}`, cardAt.x + cardAt.width <= drawerAt.x && Math.abs(cardAt.y - lineAt.y) < 2);
  await page.screenshot({ path: `${out}/agents-5-history-figures.png` });
  await page.mouse.move(lineAt.x + 4, lineAt.y + lineAt.height / 2);
  await page.mouse.move(cardAt.x + cardAt.width - 20, cardAt.y + cardAt.height / 2, { steps: 4 });
  await page.waitForTimeout(400);
  expect('the card to be there still with the pointer on it', (await card.count()) === 1);
  // The pointer on another line, the card is that line's; off the lines, it goes.
  await turns.nth(3).locator('.turn-line').hover();
  await page.getByText('900 tokens').waitFor();
  expect('one card at a time', (await card.count()) === 1);
  await drawer.getByRole('heading', { name: 'teller: history' }).hover();
  await card.waitFor({ state: 'detached' });

  // A line is opened under itself by a press, one at a time, and closed by another.
  await large.locator('.turn-line').click();
  const whole = drawer.locator('.turn-whole');
  await whole.waitFor();
  expect('what was pressed to say that it is open', (await large.locator('.turn-line').getAttribute('aria-expanded')) === 'true');
  // What it opens into: first the very words that were said to it in its channel, as words and in no box, with nothing of what the
  // daemon put round them; then what it answered.
  const SAID = '@teller size=large how many is that?';
  const words = whole.locator('.turn-words');
  expect(`the words that asked for it, as they were said, got "${await words.innerText()}"`, (await words.innerText()).trim() === SAID && (await whole.locator('.code-box').count()) === 0);
  expect(`what it answered, under what it was asked, got "${await whole.innerText()}"`, (await whole.locator('.turn-answer').innerText()).trim() === 'Counted.' && (await words.boundingBox()).y < (await whole.locator('.turn-answer').boundingBox()).y);
  // The whole of what it was sent is behind one small line under those, pressed to be read and again to be put away.
  const fold = whole.getByRole('button', { name: 'The whole of what it was sent', exact: true });
  const sentWhole = whole.locator('.turn-asked');
  expect('the whole of what it was sent to be folded away', (await fold.getAttribute('aria-expanded')) === 'false' && (await sentWhole.count()) === 0 && (await fold.boundingBox()).y > (await whole.locator('.turn-answer').boundingBox()).y);
  await page.screenshot({ path: `${out}/agents-6-history-open.png` });
  await fold.click();
  await sentWhole.waitFor();
  const allSent = (await sentWhole.innerText()).trim();
  expect(`the whole of what it was sent, which says where it is and ends with the words, got "${allSent}"`, allSent.startsWith('You are in the channel #counting.') && allSent.endsWith(SAID) && allSent.length > SAID.length + 60);
  expect('it to be in the box it always was, no taller than it was, and said to be unfolded', (await sentWhole.boundingBox()).height <= 122 && (await sentWhole.evaluate((box) => box.classList.contains('code-box'))) && (await fold.getAttribute('aria-expanded')) === 'true');
  expect('the words that asked to be there still, over it', (await words.innerText()).trim() === SAID && (await words.boundingBox()).y < (await sentWhole.boundingBox()).y);
  await apart(page, `${DRAWER_AT_REST} .turn-line`, TURN, 'the history with the whole of what a turn was sent unfolded');
  await page.screenshot({ path: `${out}/agents-6a-history-whole.png` });
  // By the keyboard, as the line itself: Enter folds it away, and unfolds it.
  await fold.focus();
  await page.keyboard.press('Enter');
  await sentWhole.waitFor({ state: 'detached' });
  await page.keyboard.press('Enter');
  await sentWhole.waitFor();
  await fold.click();
  await sentWhole.waitFor({ state: 'detached' });
  expect('the line to be open still with the whole folded away', (await whole.count()) === 1 && (await fold.getAttribute('aria-expanded')) === 'false');
  expect(`the open line to say its figures, got "${await whole.locator('.turn-figures').innerText()}"`, (await whole.locator('.turn-figures').innerText()) === 'Sent to it 120,000 tokens · Written by it 9,000 tokens · Cost $0.41');
  expect('no card over a line that is open', (await card.count()) === 0);
  await turns.first().locator('.turn-line').click();
  expect('one line open at a time', (await whole.count()) === 1 && (await turns.first().locator('.turn-whole').count()) === 1);
  expect(`one that failed to say why where what it answered would be, got "${await whole.innerText()}"`, (await whole.locator('.turn-answer.error-text').innerText()).includes('It cannot be counted.'));
  await turns.first().locator('.turn-line').click();
  await whole.waitFor({ state: 'detached' });
  // By the keyboard: reached, it says its figures; Enter opens it and closes it.
  await drawer.getByRole('button', { name: 'Close', exact: true }).focus();
  await page.keyboard.press('Tab');
  expect('the keyboard to be on the first line', await turns.first().locator('.turn-line').evaluate((one) => document.activeElement === one));
  await card.waitFor();
  expect(`the card of the line the keyboard is on, its cost not reported and never nought, got "${await card.innerText()}"`, (await card.innerText()).includes('no tokens said') && (await card.innerText()).includes('not reported') && !(await card.innerText()).includes('$0'));
  await page.keyboard.press('Enter');
  await whole.waitFor();
  // On to the third line: what the first opened into may take the keyboard on the way, where what was asked is long enough to scroll.
  const thirdLine = turns.nth(2).locator('.turn-line');
  for (let tabs = 0; tabs < 6 && !(await thirdLine.evaluate((one) => document.activeElement === one)); tabs += 1) await page.keyboard.press('Tab');
  expect('the keyboard to have come to the third line', await thirdLine.evaluate((one) => document.activeElement === one));
  await page.keyboard.press('Enter');
  expect('the keyboard to have opened the third line and closed the first', (await whole.count()) === 1 && (await turns.nth(2).locator('.turn-whole').count()) === 1);
  // What is open stays open while the list is read again: it is asked a sixth thing, and the drawer follows.
  await cli('post', 'counting', '@teller size=small and once more?');
  await until('teller to have ended a sixth turn', async () => (await ended('teller')) === 6 || (await ended('teller')));
  await drawer.locator('.turn').nth(5).waitFor();
  expect(`the head to count the turn that came, got ${await figures()}`, (await figure('asked')) === '6');
  expect('the line that was open to be open still, now the fourth', (await whole.count()) === 1 && (await turns.nth(3).locator('.turn-whole').count()) === 1);
  await page.keyboard.press('Enter');
  await whole.waitFor({ state: 'detached' });
  await drawer.getByRole('heading', { name: 'teller: history' }).click();
  // A piece of work a step asked for, and no message: no words are handed apart from what it was sent, which is the step's own, so
  // its line opens as a line always did, on the whole of that in its box, with nothing folded away.
  await cli('apply', asks);
  await until('teller to have ended a seventh turn', async () => (await ended('teller')) === 7 || (await ended('teller')));
  await turns.nth(6).waitFor();
  const worked = turns.first();
  expect(`the newest line to be the piece of work, got "${await worked.locator('.turn-where').innerText()}"`, (await worked.locator('.turn-where').innerText()) === 'task');
  await worked.locator('.turn-line').click();
  await whole.waitFor();
  expect(
    `a turn with no words handed apart to open on the whole of what it was sent, got "${await whole.innerText()}"`,
    (await whole.locator('.turn-asked').innerText()).trim() === 'Count the shelf, size=small' && (await whole.locator('.turn-words').count()) === 0 && (await whole.getByRole('button').count()) === 0 && (await whole.locator('.turn-answer').innerText()).trim() === 'Counted.',
  );
  await page.screenshot({ path: `${out}/agents-6b-history-task.png` });
  await worked.locator('.turn-line').click();
  await whole.waitFor({ state: 'detached' });

  // A turn asked in a channel kept to its members, as the daemon hands it to somebody who does not read that channel: its figures and
  // none of its words. Nothing keeps a channel so yet, so the daemon's answer is given that one turn here.
  await drawer.getByRole('button', { name: 'Close', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  await page.route('**/api/agents/teller/turns', async (route) => {
    const answer = await route.fetch();
    const body = await answer.json();
    const large = body.turns.at(-1);
    body.turns[body.turns.length - 1] = { ...large, thread: '', prompt: '', channel: null, reply_to: null, options: [], reply: '', error: null, decision: null, labels: {}, cwd: null, kept_from: true };
    // And earlier ones, half a day apart, until there are as many as the daemon hands at most.
    const small = body.turns[0];
    for (let back = 1; body.turns.length < 50; back += 1) body.turns.push({ ...small, id: `earlier-${back}`, created: small.created - back * 12 * 3_600_000 });
    await route.fulfill({ response: answer, json: body });
  });
  await history(page, 'teller');
  await turns.nth(6).waitFor();
  const kept = turns.nth(6);
  expect(`one whose words are kept to say so where its place would be, got "${await kept.locator('.turn-where').innerText()}"`, (await kept.locator('.turn-where').innerText()) === 'asked in a channel kept to its members');
  expect('it to be a line like any other, with its figure and its bar, counted with the rest', (await kept.locator('.turn-figure').innerText()) === '129k' && (await kept.locator('.turn-bar').count()) === 1 && (await figure('asked')) === '50');
  // As many as the daemon hands at most may not be all there are, and the head says so; and the days are each under its own heading.
  expect(`a list as long as the daemon hands to say that it is the latest, got "${await drawer.locator('.turns-of').innerText()}"`, (await drawer.locator('.turns-of').innerText()) === 'The latest 50 times it was asked. Earlier ones are not listed here.');
  const days = await drawer.locator('.turns-day > h3').evaluateAll((heads) => heads.map((head) => head.textContent));
  expect(`the days to be today, yesterday and then each by its date, got ${days}`, days.length > 20 && days[0] === 'Today' && days[1] === 'Yesterday' && days.slice(2).every((day) => /^\d{1,2} [A-Z][a-z]{2}( \d{4})?$/.test(day)) && new Set(days).size === days.length);
  expect('it to be nothing to open', (await kept.locator('button').count()) === 0 && (await kept.locator('[aria-expanded]').count()) === 0);
  await kept.locator('.turn-line').click();
  expect('a press on it to open nothing', (await whole.count()) === 0);
  await apart(page, `${DRAWER_AT_REST} .turn-line`, TURN, 'the history with a turn whose words are kept');
  await page.screenshot({ path: `${out}/agents-7-history-kept.png` });
  await page.unroute('**/api/agents/teller/turns');
  await drawer.getByRole('button', { name: 'Close', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  // One that was never asked.
  await history(page, 'loner', 'keys');
  await drawer.getByRole('heading', { name: 'loner: history' }).waitFor();
  await drawer.getByText('This agent has not been asked anything').waitFor();
  await drawer.getByRole('button', { name: 'Close', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });

  // ---- narrower: 900 wide, where a line is three, and its history ------------------------
  await page.setViewportSize({ width: 900, height: 860 });
  await line(page, 'loner').waitFor();
  await apart(page, '.agent-line', LINE, 'the list at 900');
  await themed(page, 'at 900');
  const stacked = await line(page, 'teller').evaluate((row) => ['.agent-name', '.agent-doing', '.agent-runs'].map((part) => Math.round(row.querySelector(part).getBoundingClientRect().top)));
  expect(`a line in a narrow panel to say what it is doing and what it runs on under its name, got ${stacked}`, stacked[0] < stacked[1] && stacked[1] < stacked[2]);
  // With no words over the lines there, a line says of itself which of its parts is its permissions.
  expect('no words over the lines in a narrow panel', !(await page.locator('.agent-heads').isVisible()));
  expect(`a narrow line to say that what is pressed is its permissions, got "${await line(page, 'teller').locator('.agent-may').innerText()}"`, (await line(page, 'teller').locator('.agent-may').innerText()).replace(/\s+/g, ' ') === 'Permissions 2 areas: channels, board');
  await page.screenshot({ path: `${out}/agents-8-list-900.png` });
  await line(page, 'teller').locator('.agent-may').click();
  await holds.waitFor();
  await inWindow(page, holds, 'what teller may ask, at 900');
  await page.keyboard.press('Escape');
  await holds.waitFor({ state: 'detached' });
  await history(page, 'teller');
  await turns.nth(6).waitFor();
  await apart(page, `${DRAWER_AT_REST} .turn-line`, TURN, 'the history at 900');
  await turns.nth(6).locator('.turn-line').click();
  await whole.waitFor();
  await apart(page, `${DRAWER_AT_REST} .turn-line`, TURN, 'the history at 900 with a line open');
  await page.screenshot({ path: `${out}/agents-9-history-900.png` });
  await drawer.getByRole('button', { name: 'Close', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  // At 820 the side panel leaves the page less room still: what a panel's head says goes under its heading, and no word lies over another.
  await page.setViewportSize({ width: 820, height: 860 });
  await line(page, 'loner').waitFor();
  await apart(page, '.agent-line', LINE, 'the list at 820');
  await themed(page, 'at 820');
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });
  await page.locator('.agents-page > .panel').first().scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/agents-9b-page-820.png` });
  await page.setViewportSize({ width: 1280, height: 860 });
  await line(page, 'loner').waitFor();

  // ---- the other of dark and light --------------------------------------------------------
  const other = scheme === 'dark' ? 'light' : 'dark';
  await litAs(page, other);
  await history(page, 'teller');
  await turns.nth(6).waitFor();
  await turns.nth(6).locator('.turn-line').click();
  await whole.waitFor();
  const was = hue;
  hue = await colours(page);
  expect(`the same held in the ${other} of the theme, in its own colours, got ${JSON.stringify(hue)}`, hue.text !== was.text && hue.sent !== hue.written && hue.sent !== hue.accent && hue.written !== hue.accent && hue.figure === hue.text && hue.total === hue.text);
  await page.screenshot({ path: `${out}/agents-10-${other}.png` });
  await drawer.getByRole('button', { name: 'Close', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  await litAs(page, scheme);

  // ---- where less movement is asked for, the drawer appears as it always did --------------
  const calm = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme, reducedMotion: 'reduce' });
  const still = await calm.newPage();
  await signIn(still, 'ada@example.com');
  await agentsOf(still);
  await line(still, 'teller').waitFor();
  await watchComing(still);
  await history(still, 'teller');
  await drawerAtRest(still).locator('.turn').first().waitFor();
  const appeared = await still.evaluate(() => window.slid);
  expect(`a drawer to be moved by nothing there, and at rest as it is drawn, got ${JSON.stringify(appeared)}`, appeared.name === 'none' && appeared.rested === true);
  await drawerAtRest(still).getByRole('button', { name: 'Close', exact: true }).click();
  await drawerAtRest(still).waitFor({ state: 'detached' });
  expect('no copy of a drawer to be left where nothing moves one', (await still.locator('.drawer-leaving').count()) === 0);
  await calm.close();

  // ---- the question: what goes with it, and what stays -------------------------------
  dialog = await ask(page, 'helper');
  await dialog.getByText('This cannot be put back.').waitFor();
  const goes = await dialog.locator('.take-away-goes li').allInnerTexts();
  const stays = await dialog.locator('.take-away-stays li').allInnerTexts();
  expect(
    `what goes with it to be said, got ${JSON.stringify(goes)}`,
    goes.length === 4 && goes[0].startsWith('Its slot') && goes[1].startsWith('Its credential, which is nobody’s') && goes[2] === 'Its place in #shop.' && goes[3] === 'The folder that was its own on this device: the home its program was shown.',
  );
  expect(
    `what stays to be said, got ${JSON.stringify(stays)}`,
    stays.length === 3 && stays[0].startsWith('What it said in channels') && stays[1] === 'The one piece of work it was asked, with what it answered.' && stays[2] === 'shop-1 is given to it, and stays so until somebody gives it to another.',
  );
  await dialog.getByText('An agent that is later called helper is another agent, and has none of this for its own.').waitFor();
  expect('the usual two buttons', (await dialog.getByRole('button').allInnerTexts()).join('|') === 'Cancel|Remove it');
  await within(page, dialog, 'the question');
  await page.screenshot({ path: `${out}/agents-11-asked.png` });
  // A dialog listens for Escape a moment after it is drawn; left that way, nothing is taken.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });
  expect(`helper to be there still after the question was left, got ${await listed(page)}`, (await listed(page)).includes('helper'));
  expect('the daemon to have it still', (await cli('agent', 'list')).stdout.includes('helper'));

  // ---- taken away: gone where the daemon says so, on every open page -----------------
  dialog = await ask(page, 'helper');
  await dialog.getByRole('button', { name: 'Remove it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await page.getByText('helper is removed.').waitFor();
  await line(page, 'helper').waitFor({ state: 'detached' });
  // The second page was told by the daemon, and asked nothing of anybody.
  await line(second, 'helper').waitFor({ state: 'detached' });
  const left = stopped.filter((name) => name !== 'helper');
  for (const on of [page, second]) expect(`the others to be listed still, got ${await listed(on)}`, (await listed(on)).join() === left.join());
  drawn = await bands(page);
  expect(`its band to say one fewer, got ${JSON.stringify(drawn)}`, drawn.find((band) => band.of === 'shop').count === '3 agents');
  expect('the daemon to have it no longer', !(await cli('agent', 'list')).stdout.includes('helper'));
  await page.screenshot({ path: `${out}/agents-12-gone.png` });

  // Beside its channel: where it was being read about it is said to be gone, with the way back to the others, of whom it is not one.
  await panel.locator('.context-agent-gone').waitFor();
  expect('what was open about it to say it is gone', (await panel.locator('.context-agent-gone').innerText()).startsWith('There is no agent called helper any more.'));
  await third.screenshot({ path: `${out}/agents-13-gone-in-the-panel.png` });
  await panel.locator('.context-crumbs').getByRole('button', { name: 'Channel', exact: true }).click();
  await panel.locator('.context-agent', { hasText: 'shop-architect-1' }).waitFor();
  expect('it to be a member there no longer', (await panel.locator('.context-agent', { hasText: 'helper' }).count()) === 0);
  // What it said is there under its name, which is nothing to press, and the card over it says it is gone.
  await named.waitFor();
  expect('one that was taken away to be nothing to press where it is named', (await named.getAttribute('role')) === null);
  await named.hover();
  const about = third.locator('.who-card');
  await about.getByText('gone', { exact: true }).waitFor();
  expect('the card to name it', (await about.innerText()).includes('helper'));
  await third.screenshot({ path: `${out}/agents-14-gone-where-it-spoke.png` });
  await named.click();
  expect('pressing its name to open nothing about it', (await panel.locator('.context-about, .context-agent-gone').count()) === 0);

  // ---- a member is shown the list and each history, is offered moving alone, and is refused by the daemon ---
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  const theirs = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const member = await theirs.newPage();
  // Settings pressed as soon as the frame is there, before the tabs kept for her are put back, is the tab showing after.
  await beforeEntering(member, () => signIn(member, 'grace@example.com'), () => member.getByRole('button', { name: 'Settings', exact: true }).click());
  await member.getByRole('heading', { name: 'Settings', level: 1 }).waitFor();
  expect(`Settings pressed before the frame entered to be the tab showing, got ${await member.locator('.tabs .tab').allInnerTexts()}`, (await member.locator('.tabs .tab[aria-selected="true"]').innerText()).includes('Settings'));
  expect('no agent listed in Settings any more', (await member.locator('.agent-line').count()) === 0);
  await agentsOf(member);
  await line(member, 'loner').waitFor();
  expect(`a member to be shown the agents, got ${await listed(member)}`, (await listed(member)).join() === left.join());
  // A member changes agents: of what is done with one, moving it to another project alone.
  await line(member, 'loner').getByRole('button', { name: 'More that can be done with loner', exact: true }).click();
  const memberMay = (await member.getByRole('menuitem').allInnerTexts()).map((one) => one.trim());
  expect(`a member to be offered its history and moving it, and neither stopping one nor taking one away, got ${memberMay}`, memberMay.join('|') === 'Edit…|History|Move to project…');
  await member.waitForTimeout(200);
  await member.keyboard.press('Escape');
  await member.getByRole('menu').waitFor({ state: 'detached' });
  const memberButtons = await line(member, 'teller').getByRole('button').allInnerTexts();
  expect(`a member's line to have what it may ask and its menu, got ${memberButtons}`, memberButtons.map((one) => one.trim()).join('|') === '2 areas: channels, board|⋯');
  // And her history of one from it, as anybody's.
  await history(member, 'teller');
  await drawerAtRest(member).getByRole('heading', { name: 'teller: history' }).waitFor();
  await member.keyboard.press('Escape');
  await drawerAtRest(member).waitFor({ state: 'detached' });
  const refused = await member.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const looked = await fetch('/api/agents/loner/remove', { headers: mine });
    const taken = await fetch('/api/agents/loner/remove', { method: 'POST', headers: mine });
    return [looked.status, taken.status, (await taken.json()).error?.message ?? ''];
  });
  expect(`a member to be refused by the daemon, asking and doing, got ${refused}`, refused[0] === 403 && refused[1] === 403 && refused[2].includes('(needs `admin`)'));
  await member.screenshot({ path: `${out}/agents-15-member.png`, fullPage: true });

  // ---- a phone's width: the list, a history, the question whole in the window, and one taken away ---
  const small = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  const phone = await small.newPage();
  phone.on('pageerror', (err) => problems.push(err.message));
  await phone.goto(url);
  await phone.getByLabel('Email').fill('ada@example.com');
  await phone.getByLabel('Password').fill('correct horse');
  await phone.getByRole('button', { name: 'Sign in', exact: true }).last().tap();
  await phone.locator('.panes').waitFor();
  await agentsOf(phone, 'tap');
  // A line is three on a phone, each part whole in it: nothing is a table wider than the screen, and nothing scrolls sideways.
  await line(phone, 'loner').scrollIntoViewIfNeeded();
  await apart(phone, '.agent-line', LINE, 'the list on a phone');
  await themed(phone, 'on a phone', 'tap');
  const at = await line(phone, 'loner').getByRole('button', { name: 'More that can be done with loner', exact: true }).boundingBox();
  expect(`the entry to be in the window on a phone, and it is at ${JSON.stringify(at)}`, at && at.x >= 0 && at.x + at.width <= 390);
  await line(phone, 'teller').scrollIntoViewIfNeeded();
  await phone.screenshot({ path: `${out}/agents-16-phone-list.png` });
  // What one may ask, whole, in the window.
  await line(phone, 'teller').locator('.agent-may').tap();
  const theirHolds = phone.getByRole('dialog', { name: 'Permissions of teller' });
  await theirHolds.waitFor();
  await inWindow(phone, theirHolds, 'what teller may ask, on a phone');
  await phone.getByRole('heading', { name: 'Members', level: 1 }).tap();
  await theirHolds.waitFor({ state: 'detached' });
  // Its history has the screen. A finger does not point: a line touched opens, and says its figures where it is open.
  await history(phone, 'teller', 'tap');
  const sheet = drawerAtRest(phone);
  await sheet.locator('.turn').nth(6).waitFor();
  // It has the screen, and came in as it does anywhere.
  const cameIn = await sheet.evaluate((one) => [getComputedStyle(one).animationName, getComputedStyle(one).transform, Math.round(one.getBoundingClientRect().left), Math.round(one.getBoundingClientRect().width)]);
  expect(`the drawer on a phone to have come in the same way and to have the screen, got ${cameIn}`, cameIn.join() === 'drawer-in,none,0,390');
  await apart(phone, `${DRAWER_AT_REST} .turn-line`, TURN, 'the history on a phone');
  await sheet.locator('.turn').nth(6).locator('.turn-line').tap();
  await sheet.locator('.turn-whole').waitFor();
  expect('a line touched to open, with its figures in it and no card over it', (await sheet.locator('.turn-whole .turn-figures').innerText()).startsWith('Sent to it 120,000 tokens') && (await phone.locator('.turn-exact').count()) === 0);
  await apart(phone, `${DRAWER_AT_REST} .turn-line`, TURN, 'the history on a phone with a line open');
  await phone.screenshot({ path: `${out}/agents-17-phone-history.png` });
  await sheet.getByRole('button', { name: 'Close', exact: true }).tap();
  await sheet.waitFor({ state: 'detached' });
  dialog = await ask(phone, 'loner', 'tap');
  await dialog.getByText('This cannot be put back.').waitFor();
  const alone = await dialog.locator('.take-away-goes li').allInnerTexts();
  expect(`one in no project to be told its folder holds what it wrote, got ${JSON.stringify(alone)}`, alone.length === 3 && alone[2].includes('since it works in no project, whatever it wrote there'));
  await within(phone, dialog, 'the question on a phone');
  await phone.screenshot({ path: `${out}/agents-18-phone-asked.png` });
  await dialog.getByRole('button', { name: 'Remove it', exact: true }).tap();
  await dialog.waitFor({ state: 'detached' });
  await line(phone, 'loner').waitFor({ state: 'detached' });
  // And on the pages that were open all along.
  for (const on of [page, second, member]) await line(on, 'loner').waitFor({ state: 'detached' });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  // Whatever was left waiting is let go before the daemon is stopped.
  await writeFile(gate, '').catch(() => {});
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
