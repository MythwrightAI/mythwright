// What a card cost, by round, in its drawer.
//
//   DAEMON=../target/debug/factory node scripts/cardcost.mjs [out-dir]
//
// The shop's fleet works a card with stand-ins for its worker and its
// reviewer, each a few lines of shell that say what their turn took as
// an agent's program does (tokens sent and written, and what that
// cost, but for the approving turn, which says nothing of its cost and
// is said to be not reported, never as nought). The reviewer sends the work back the first time it looks and
// approves it the second, so the card goes round twice. Opened once it
// is done, its drawer says what it cost: what all of it came to, a line
// for each round with what was decided at its end and a bar as long as
// that round's tokens against the larger round, drawn in the theme's
// dimmer text colour and never its accent, the words the decision was
// said with, and each round's turns under it. Every figure is what the
// daemon lays out when it is asked the same, and what the stand-ins
// said. Nothing of it lies over anything else, past the drawer's sides
// or scrolls sideways, at three widths and a phone's, where the drawer
// has the screen. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'cardcost-data');
const home = join(out, 'cardcost-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };

/** What each stand-in's turns say they took: tokens sent to it, tokens it wrote, and what it cost; the approving turn says nothing of what it cost, as a program that reports only tokens does. */
const TOOK = { work: [1000, 200, 0.25], again: [600, 100, 0.12], back: [400, 40, 0.02], approve: [300, 30, undefined] };
const said = (text, [read, wrote, cost]) => `printf '%s\\n' '${JSON.stringify({ text, cost, in: read, out: wrote })}'`;
// The worker changes a file and says so, the second time for less.
const worked = join(home, 'worked');
const maker = join(home, 'maker.sh');
await writeFile(
  maker,
  `#!/bin/sh
cat >/dev/null
echo changed >> CHANGES.md
if [ -e '${worked}' ]; then ${said('I changed it again.', TOOK.again)}; exit 0; fi
touch '${worked}'
${said('I changed a file.', TOOK.work)}
`,
);
// The reviewer sends it back the first time it looks, and approves it after.
const looked = join(home, 'looked');
const looker = join(home, 'looker.sh');
await writeFile(
  looker,
  `#!/bin/sh
cat >/dev/null
if [ -e '${looked}' ]; then ${said('It does what the card asks.\nDECISION: approve', TOOK.approve)}; exit 0; fi
touch '${looked}'
${said('The total is wrong when the cart is empty.\nDECISION: send_back', TOOK.back)}
`,
);
for (const file of [maker, looker]) await chmod(file, 0o755);
const PACKAGE = (name, program) => `extension: ${name}
title: ${name}
description: A stand-in that says what its turn took.
agent:
  program: ${program}
  input: \${ turn.prompt }
  output:
    - when: \${ event.text != null }
      reply: \${ event.text }
      cost_usd: \${ event.cost }
      input_tokens: \${ event.in ?? 0 }
      output_tokens: \${ event.out ?? 0 }
`;

const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  for (const [name, program] of [
    ['maker', maker],
    ['looker', looker],
  ]) {
    await mkdir(join(home, name), { recursive: true });
    await writeFile(join(home, name, 'extension.yaml'), PACKAGE(name, program));
    await cli('extension', 'add', join(home, name));
  }
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('fleet', 'init', 'shop');
  await cli('apply', 'fleet', '--project', 'shop');
  await cli('record', 'set', 'shop-agents-2', 'adapter=maker');
  await cli('record', 'set', 'shop-agents-3', 'adapter=looker', 'count=1');
  await cli('board', 'add', 'shop', 'Add a cart', '--body', 'Customers need one.');

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
  const signedIn = async (how) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    // On a phone the rail comes with the side panel, and is on the page meanwhile.
    await page.locator('.rail').waitFor({ state: 'attached' });
    return page;
  };
  const page = await signedIn({ viewport: { width: 1280, height: 860 } });
  const board = async (on, touch = false) => {
    const press = (locator) => (touch ? locator.tap() : locator.click());
    if (touch) await press(on.getByRole('button', { name: 'Expand the side panel' }));
    await sideSettled(on);
    const resources = on.getByRole('button', { name: 'Resources', exact: true });
    if ((await resources.getAttribute('aria-pressed')) !== 'true') await press(resources);
    // Everything the Resources panel lists starts folded (`folded.mjs`): unfolded by Expand all, which opens
    // nothing, so that a phone's panel stays for its board to be pressed.
    await unfoldAll(on);
    await press(on.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }));
  };
  await board(page);
  const done = page.locator('.column', { hasText: 'done' }).locator('.card', { hasText: 'Add a cart' });

  // ── Worked, sent back, worked again and passed ──
  await cli('board', 'move', 'shop-1', 'ready');
  await done.waitFor({ timeout: 90000 });
  const laid = JSON.parse((await cli('board', 'cost', 'shop-1', '--json')).stdout);
  expect(`every turn placed by what began its asking and none by order, got ${laid.by_order}`, laid.by_order === 0);
  expect(`the daemon to lay out two rounds, sent back and passed, got ${JSON.stringify(laid.rounds.map((round) => round.decided.came))}`, laid.rounds.map((round) => round.decided.came).join() === 'back,passed');
  await done.click();
  const drawer = drawerAtRest(page);
  const cost = drawer.getByRole('region', { name: 'What it cost' });
  // The second round is read once its decision is in the card's history, which may come a moment after the move.
  await cost.locator('.cost-round[data-round="2"][data-came="passed"]').waitFor({ timeout: 20000 });
  await cost.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/cardcost-1-drawer.png` });

  // Every figure is the daemon's, and the stand-ins' own.
  const tokens = (...took) => took.reduce((all, [read, wrote]) => all + read + wrote, 0);
  const rounds = await cost.locator('.cost-round').evaluateAll((all) =>
    all.map((round) => ({
      came: round.getAttribute('data-came'),
      tokens: Number(round.getAttribute('data-tokens')),
      decided: round.querySelector('.cost-round-came')?.textContent,
      words: round.querySelector('.cost-round-said')?.textContent ?? '',
      turns: [...round.querySelectorAll('.cost-turns > li')].map((one) => one.textContent),
      bar: round.querySelector('.turn-bar')?.getBoundingClientRect().width ?? 0,
      track: round.querySelector('.turn-track')?.getBoundingClientRect().width ?? 0,
    })),
  );
  expect(`two rounds drawn, got ${JSON.stringify(rounds)}`, rounds.length === 2 && rounds.map((one) => one.came).join() === 'back,passed');
  expect(`the rounds' tokens to be what the stand-ins said, got ${rounds.map((one) => one.tokens)}`, rounds[0].tokens === tokens(TOOK.work, TOOK.back) && rounds[1].tokens === tokens(TOOK.again, TOOK.approve));
  expect(`the rounds' tokens to be what the daemon laid out`, rounds.every((one, at) => one.tokens === laid.rounds[at].spent.sent + laid.rounds[at].spent.wrote));
  expect(`what was decided in a word or two, got ${rounds.map((one) => one.decided)}`, rounds[0].decided === 'Sent back' && rounds[1].decided === 'Passed');
  expect(`the words it was sent back with, got "${rounds[0].words}"`, rounds[0].words.includes('The total is wrong when the cart is empty.'));
  expect(`the words it was approved with, got "${rounds[1].words}"`, rounds[1].words === 'shop-reviewer-1 approved it: It does what the card asks.');
  expect(`each round's two turns, the worker's and then the reviewer's, got ${JSON.stringify(rounds.map((one) => one.turns))}`, rounds.every((one) => one.turns.length === 2 && one.turns[0].startsWith('shop-worker-1 worked') && one.turns[1].startsWith('shop-reviewer-1 looked')));
  expect(`what each reviewer chose, got ${rounds.map((one) => one.turns[1])}`, rounds[0].turns[1].includes('chose send_back') && rounds[1].turns[1].includes('chose approve'));
  // The larger round has the whole track, and the other is in proportion to it.
  expect(`the first round's bar to have its whole track, got ${rounds[0].bar} of ${rounds[0].track}`, Math.abs(rounds[0].bar - rounds[0].track) < 1);
  const ratio = rounds[1].bar / rounds[0].bar;
  expect(`the second round's bar to be in proportion, got ${ratio}`, Math.abs(ratio - rounds[1].tokens / rounds[0].tokens) < 0.03);
  // What all of it came to.
  const figure = (name) => cost.locator(`.turns-figures [data-figure="${name}"] dd`).innerText();
  expect(`the rounds and turns counted, got ${await figure('rounds')} and ${await figure('turns')}`, (await figure('rounds')) === '2' && (await figure('turns')) === '4');
  const all = TOOK.work[2] + TOOK.again[2] + TOOK.back[2];
  expect(`what all of it cost, of the turns that said, got ${await figure('cost')}`, (await figure('cost')) === `$${all.toFixed(2)}`);
  // A turn whose program said nothing of what it cost is said to be not reported, and never as nought.
  expect(`the daemon to count the turn that said no cost, got ${JSON.stringify(laid.spent)}`, laid.spent.cost_unsaid === 1 && Math.abs(laid.spent.cost_usd - all) < 1e-9);
  expect(`the approving turn's cost said to be not reported, got ${rounds[1].turns[1]}`, rounds[1].turns[1].includes('cost not reported') && !rounds[1].turns[1].includes('$0'));
  const printed = (await cli('board', 'cost', 'shop-1')).stdout;
  expect(`the terminal to say the same, got ${printed}`, printed.includes('cost not reported') && printed.includes(`$${all.toFixed(2)} (and 1 not reported)`));
  // Drawn in the theme's dimmer text colour, and never its accent.
  const colours = await cost.evaluate((section) => {
    const probe = (colour) => {
      const one = document.createElement('span');
      one.style.background = colour;
      section.append(one);
      const got = getComputedStyle(one).backgroundColor;
      one.remove();
      return got;
    };
    const written = section.querySelector('.cost-round .turn-part.written');
    return { written: written ? getComputedStyle(written).backgroundColor : null, dim: probe('var(--dim)'), accent: probe('var(--accent)') };
  });
  expect(`the bar in the dimmer text colour, got ${JSON.stringify(colours)}`, colours.written === colours.dim && colours.written !== colours.accent);

  // ── Nothing over anything else, at three widths and a phone's ──
  const laidOut = async (on, what) => {
    const wrong = await on.evaluate(() => {
      const said = [];
      const section = document.querySelector('.drawer .card-cost');
      if (!section) return ['there is no part saying what it cost'];
      const edge = section.getBoundingClientRect();
      const parts = (selector) => [...section.querySelectorAll(selector)].map((one) => ({ one, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0);
      const apart = (group, what) => {
        for (const [index, { one, box }] of group.entries()) {
          if (box.left < edge.left - 0.5 || box.right > edge.right + 0.5) said.push(`${what} ${one.className || one.textContent} leaves the part`);
          for (const other of group.slice(index + 1)) {
            const over = Math.min(box.right, other.box.right) - Math.max(box.left, other.box.left) > 0.5 && Math.min(box.bottom, other.box.bottom) - Math.max(box.top, other.box.top) > 0.5;
            if (over) said.push(`${what}: ${one.textContent.trim().slice(0, 30)} lies over ${other.one.textContent.trim().slice(0, 30)}`);
          }
        }
      };
      apart(parts('.turns-figures > div'), 'a figure');
      for (const line of section.querySelectorAll('.cost-round-line')) apart([...line.children].map((one) => ({ one, box: one.getBoundingClientRect() })), 'a round');
      apart(parts('.cost-round'), 'a round');
      for (const round of section.querySelectorAll('.cost-round')) apart([...round.children].map((one) => ({ one, box: one.getBoundingClientRect() })).filter((one) => one.box.width > 0), 'in a round');
      const scroller = section.closest('.drawer-body') ?? section.closest('.drawer');
      if (scroller && scroller.scrollWidth > scroller.clientWidth) said.push(`the drawer scrolls ${scroller.scrollWidth - scroller.clientWidth}px sideways`);
      if (document.documentElement.scrollWidth > window.innerWidth) said.push('the page scrolls sideways');
      return said;
    });
    expect(`nothing of what it cost over anything else ${what}, got ${wrong.join('; ')}`, wrong.length === 0);
    const crowded = await pageCrowded(on, '.drawer .card-cost');
    expect(`nothing of it crowded ${what}, got ${crowded.join('; ')}`, crowded.length === 0);
  };
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await cost.scrollIntoViewIfNeeded();
    await laidOut(page, `at ${width} wide`);
  }
  await page.screenshot({ path: `${out}/cardcost-2-narrow.png` });
  const phone = await signedIn({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  await board(phone, true);
  const small = phone.locator('.column', { hasText: 'done' }).locator('.card', { hasText: 'Add a cart' });
  await small.tap();
  const there = drawerAtRest(phone).getByRole('region', { name: 'What it cost' });
  await there.locator('.cost-round[data-round="2"]').waitFor();
  await there.scrollIntoViewIfNeeded();
  await laidOut(phone, "at a phone's width");
  await phone.screenshot({ path: `${out}/cardcost-3-phone.png` });

  // A card no agent has taken a turn on has nothing of it.
  await cli('board', 'add', 'shop', 'Write the returns policy');
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });
  await page.locator('.card', { hasText: 'Write the returns policy' }).click();
  await drawerAtRest(page).getByLabel('Title').waitFor();
  expect('no part saying what it cost for a card nobody worked', (await drawerAtRest(page).getByRole('region', { name: 'What it cost' }).count()) === 0);

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
