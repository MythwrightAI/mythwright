// Comparing ways of doing something, in the client: started from the
// page of the blueprint whose starting is compared, looked at where the
// measurements are, and ended saying which way goes on.
//
//   DAEMON=../target/debug/factory node scripts/compared.mjs [out-dir]
//
// A shop has a piece of work done for each thing that is wanted, by a
// blueprint of its own called `work`. Somebody has written another way of
// doing it, `my-work`, which takes two steps where `work` takes one. From
// the page of `work` they start a comparison: each thing goes one way and
// keeps to it, and what is made of each is counted by the tone it was made
// in. Four things are wanted. Measurements then shows the comparison: each
// way with what it starts and hands, and one table with a column for each
// way: how many things it was given, what came of them, and everything
// measured under it, with no way marked as the better one. It is
// ended saying that everything goes their own way from now on, a fifth
// thing goes that way uncounted, and it is said again that everything is
// as it is written. Then Settings: how what is running comes by a changed
// blueprint, where having them followed says what it gives up before it
// is turned on; and, with texts followed, a way that follows its file,
// each text it ran counted in a column of its own. Last, what somebody
// decides from: a project whose fleet works cards, with two who work, each
// a stand-in that says what its turn cost and how much it read and wrote,
// and one who reviews and sends one card back. Each card goes to one of
// the two, and the comparison shows under each what came of its cards and
// how long after, what was chosen, what it cost in all and a card, how many of
// its cards were accepted of those it was given, and what it cost for each of those;
// and what came after a card's work was merged, under that card's way: the first
// card's work reverted by a card a person put in review with the revert on its branch,
// and the second's file changed again by a card that says it fixes it.
// Then two prompts for whoever reviews: one way reads words of the
// project's own where the reviewer's seat links its file, and each way's
// column says which words it was run with.
// Last, a gradual rollout: a comparison whose new way is given more of the
// things by a rule while what is counted of them holds, the shares said
// again by the rule and then by somebody from a terminal, each with why,
// on a page that is not read again.
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { appendFile, chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { fleetTook } from './fleettook.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';
import { pressTesting } from './testing.mjs';

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
const data = join(out, 'compared-data');
const home = join(out, 'compared-home');

const SHOP = `blueprint: shop
description: Has the work done for each thing that is wanted.
on:
  wanted:
    event: shop.wanted
    steps:
      done: { call: work, as: "work-\${ event.data.id }", with: { id: \${ event.data.id } } }
`;

const WORK = `blueprint: work
version: 1
description: The work, done in one step.
params:
  id: { type: int }
  tone: { type: string, default: plain }
steps:
  made: { do: emit, with: { topic: shop.made, id: \${ params.id }, tone: "\${ params.tone }" } }
`;

const MINE = `blueprint: my-work
version: 1
description: The same work, looked over before it is made.
params:
  id: { type: int }
  tone: { type: string, default: plain }
steps:
  looked: { do: emit, with: { topic: shop.looked, id: \${ params.id } } }
  made: { needs: [looked], do: emit, with: { topic: shop.made, id: \${ params.id }, tone: "\${ params.tone }" } }
`;

// Whoever works and whoever reviews in the last part: one program, told who it is. It reads what it
// is asked and answers in a line that says what the turn cost and how much was read and written, as
// an agent's program does. Each piece of work is a file of its own, so that no two cards' branches
// meet in one. The second review it is asked for sends the card back.
const STANDIN = `#!/bin/sh
who=$1
here=$(dirname "$0")
cat >/dev/null
if [ "$who" = looks ]; then
  n=$(cat "$here/reviews" 2>/dev/null || echo 0); n=$((n + 1)); echo "$n" > "$here/reviews"
  if [ "$n" = 2 ]; then say='It leaves the till open.\\nDECISION: send_back'; else say='It does what the card asks.\\nDECISION: approve'; fi
  printf '{"kind":"end","text":"%s","cost":0.02,"in":4000,"out":300}\\n' "$say"
  exit 0
fi
echo "changed by $who" > "change-$$.md"
if [ "$who" = big ]; then cost=0.41; read=120000; wrote=9000; else cost=0.09; read=80000; wrote=5000; fi
printf '{"kind":"end","text":"I changed a file.\\\\nDECISION: done","cost":%s,"in":%s,"out":%s}\\n' "$cost" "$read" "$wrote"
`;
const packaged = (name, program) => `extension: ${name}
title: ${name}
description: A stand-in that says what its turn cost.
agent:
  program: ${program}
  args:
    - ["${name}"]
  input: \${ turn.prompt }
  output:
    - when: \${ event.kind == "end" }
      reply: \${ event.text }
      cost_usd: \${ event.cost }
      input_tokens: \${ event.in }
      output_tokens: \${ event.out }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// No walls: the stand-ins of the last part write where they are, and are no agent's real program.
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off'], {
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
  await cli('project', 'create', '--bare', 'Shop');
  // The project's own blueprints, where the factory keeps them (`@blueprints/` of the project's).
  const blueprints = join(data, 'blueprints', 'projects', 'shop');
  await mkdir(blueprints, { recursive: true });
  for (const [name, text] of [['shop', SHOP], ['work', WORK], ['my-work', MINE]]) await writeFile(join(blueprints, `${name}.bp`), text);
  await cli('project', 'scan', 'shop');
  await cli('apply', join(blueprints, 'shop.bp'), '--project', 'shop');
  // A thing is wanted, and its work is done before the next is asked for.
  const want = async (id) => {
    await cli('emit', 'shop.wanted', `id=${id}`);
    for (let tries = 0; tries < 100; tries += 1) {
      const made = await cli('events', '--topics', 'shop.made', '--limit', '50', '--json');
      // One event a line.
      if (made.stdout.split('\n').filter(Boolean).some((line) => JSON.parse(line).data.id === id)) return;
      await new Promise((later) => setTimeout(later, 100));
    }
    throw new Error(`the work for ${id} was never done`);
  };

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
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

  // The page of the blueprint whose starting is to be compared: the project's own `work`.
  await side(page, 'Resources');
  const nav = page.locator('.nav');
  const project = (name) => nav.locator(':scope > .tree > [data-project]', { has: page.locator(':scope > .nav-row', { hasText: name }) });
  const list = project('Shop').locator(':scope > .tree > div', { has: page.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item', { hasText: 'Blueprints' }) });
  await list.waitFor();
  const head = list.locator(':scope > button.nav-item, :scope > .nav-line > button.nav-item');
  if ((await head.getAttribute('aria-expanded')) !== 'true') await head.click();
  await list.getByText('work', { exact: true }).click();
  // Compare on real work is a line of Testing in the blueprint's head, offered once its text checks: the menu says what
  // there was when it was opened, so it is opened once the head says the text checks.
  const viewHead = page.locator('.view-head').locator('visible=true').last();
  const compare = {
    click: async () => {
      await viewHead.getByText('checks', { exact: true }).waitFor();
      await pressTesting(page, viewHead, 'Compare on real work');
    },
  };
  await compare.click();

  // Started from there, in the drawer at the side, so that the blueprint's text is still read beside it: what it
  // is called, what a thing is, and the ways. One is the work as it is written, one their own.
  const asked = drawerAtRest(page);
  await asked.getByRole('heading', { name: 'Compare ways of work' }).waitFor();
  expect('the ways to be asked in the drawer and in no dialog over the page', (await page.getByRole('dialog').count()) === 0);
  await asked.getByLabel('Called').fill('tones');
  await asked.getByLabel('A thing is').selectOption({ label: 'what id says' });
  await asked.getByLabel('Way 1', { exact: true }).fill('theirs');
  await asked.getByLabel('Way 2', { exact: true }).fill('mine');
  await asked.getByLabel('What way 2 starts').selectOption('my-work');
  await asked.getByLabel('What way 2 hands').fill('tone=kind');
  // A way may also read another file where one is linked; nothing here links one, and it is left empty.
  expect('each way to be asked what it reads', (await asked.getByLabel('What way 2 reads').inputValue()) === '');
  // What comes of each thing is counted too: here an event of the shop's own, by the tone the thing was made in.
  expect('nothing to be counted of a thing that is no card unless it is said', (await asked.getByLabel('Counted of each thing').inputValue()) === '');
  await asked.getByLabel('Counted of each thing').selectOption('said');
  await asked.getByLabel('Events counted').fill('made=shop.made:id:tone');
  await page.screenshot({ path: `${out}/compared-0-started-from-a-blueprint.png` });

  // Nothing of the form lies over anything else of it, and nothing has to be scrolled sideways for: wide, narrower,
  // and at a phone's width, where the drawer has the screen; with two ways and with four. The boxes are each
  // label, each explanation, each box written in and each thing pressed; what the form is left by stays at the
  // drawer's foot over whatever is under it, so those are held to each other and to being in sight.
  const laid = () =>
    page.evaluate(() => {
      const drawer = document.querySelector('.drawer');
      const body = drawer.querySelector('.drawer-body');
      const form = drawer.querySelector('.comparing');
      const box = (el) => {
        const r = el.getBoundingClientRect();
        const said = (el.getAttribute('aria-label') ?? el.textContent ?? '').trim().slice(0, 40);
        return { what: `${el.tagName.toLowerCase()} "${said}"`, left: r.left, right: r.right, top: r.top, bottom: r.bottom };
      };
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const pairs = (boxes) => boxes.flatMap((a, at) => boxes.slice(at + 1).filter((b) => over(a, b)).map((b) => `${a.what} over ${b.what}`));
      const foot = form.querySelector('.comparing-foot');
      const fields = [...form.querySelectorAll('.comparing-about, .comparing-label, .comparing-said, .comparing-way-called, input, select, textarea, button')].filter((el) => !foot.contains(el));
      const left = [...foot.querySelectorAll('button')];
      const within = body.getBoundingClientRect();
      const outside = [...fields, ...left].map(box).filter((one) => one.left < within.left - 0.5 || one.right > within.right + 0.5);
      const seen = foot.getBoundingClientRect();
      return {
        fields: fields.length,
        over: [...pairs(fields.map(box)), ...pairs(left.map(box))],
        outside: outside.map((one) => one.what),
        sideways: body.scrollWidth - body.clientWidth,
        page: document.documentElement.scrollWidth - window.innerWidth,
        foot: seen.bottom <= window.innerHeight + 0.5 && seen.top >= 0 && seen.bottom <= drawer.getBoundingClientRect().bottom + 0.5,
      };
    });
  const holds = async (how) => {
    const found = await laid();
    expect(`no two things of the form to lie over each other ${how}, got ${found.over.join('; ')}`, found.over.length === 0);
    expect(`everything of the form to be within the drawer ${how}, got ${found.outside.join('; ')}`, found.outside.length === 0);
    expect(`the drawer not to scroll sideways ${how}, by ${found.sideways}`, found.sideways <= 0);
    expect(`the page not to scroll sideways ${how}, by ${found.page}`, found.page <= 0);
    expect(`what the form is left by to be in reach at the drawer's foot ${how}`, found.foot);
    return found.fields;
  };
  const widths = [1350, 1024, 390];
  for (const width of widths) {
    await page.setViewportSize({ width, height: 860 });
    await asked.locator('.comparing').waitFor();
    await asked.locator('.drawer-body').evaluate((body) => (body.scrollTop = 0));
    const counted = await holds(`with two ways at ${width} wide`);
    expect(`the form with two ways to have its fields, got ${counted}`, counted > 20);
    await page.screenshot({ path: `${out}/compared-0a-two-ways-at-${width}.png` });
  }
  // Two more ways: each a block of its own under the last, and what takes one out is in its head.
  await page.setViewportSize({ width: 1350, height: 860 });
  await asked.getByRole('button', { name: 'Another way', exact: true }).click();
  await asked.getByRole('button', { name: 'Another way', exact: true }).click();
  expect('a way added to be called by a letter no way has', (await asked.getByLabel('Way 3', { exact: true }).inputValue()) === 'a' && (await asked.getByLabel('Way 4', { exact: true }).inputValue()) === 'b');
  // A rule its shares grow by, asked under the ways: its parts, the ways chosen among those written.
  await asked.getByLabel('Its shares grow by a rule').check();
  const grows = await asked.getByLabel('Grows', { exact: true }).locator('option').allInnerTexts();
  expect(`the way that grows to be chosen among the ways, got ${JSON.stringify(grows)}`, grows.join() === 'theirs,mine,a,b');
  expect('the rule to begin on the last way grown against the first', (await asked.getByLabel('Grows', { exact: true }).inputValue()) === 'b' && (await asked.getByLabel('Held against').inputValue()) === 'theirs');
  for (const width of widths) {
    await page.setViewportSize({ width, height: 860 });
    await asked.getByLabel('Way 4', { exact: true }).waitFor();
    await asked.locator('.drawer-body').evaluate((body) => (body.scrollTop = 0));
    await holds(`with four ways at ${width} wide`);
    await page.screenshot({ path: `${out}/compared-0b-four-ways-at-${width}.png` });
    // Gone through to its end, the last way and what adds another are whole and what it is left by is still there.
    await asked.getByRole('button', { name: 'Another way', exact: true }).scrollIntoViewIfNeeded();
    await holds(`with four ways at ${width} wide, at its end`);
    await page.screenshot({ path: `${out}/compared-0c-four-ways-at-${width}-at-its-end.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await asked.getByLabel('Its shares grow by a rule').uncheck();
  // At a phone's width the side panel is put away; it is shown again for what follows.
  await side(page, 'Resources');
  await asked.getByRole('button', { name: 'Remove way 4', exact: true }).click();
  await asked.getByRole('button', { name: 'Remove way 3', exact: true }).click();
  expect('the two ways there were to be as they were written', (await asked.getByLabel('Way 2', { exact: true }).inputValue()) === 'mine' && (await asked.getByLabel('What way 2 hands').inputValue()) === 'tone=kind' && (await asked.getByLabel('Way 3', { exact: true }).count()) === 0);

  // Another tab shown puts the drawer away, and what was written is there when it is asked for again.
  await page.locator('.tab', { hasText: 'Start' }).first().click();
  await asked.waitFor({ state: 'detached' });
  await page.locator('.tab', { hasText: 'work' }).first().click();
  await compare.click();
  await asked.getByRole('heading', { name: 'Compare ways of work' }).waitFor();
  // What a thing is and what a way starts are chosen from what the project has, which is read as the drawer opens.
  const written = async () => [
    await asked.getByLabel('Called').inputValue(),
    await asked.getByLabel('A thing is').inputValue(),
    await asked.getByLabel('Way 1', { exact: true }).inputValue(),
    await asked.getByLabel('What way 2 starts').inputValue(),
    await asked.getByLabel('What way 2 hands').inputValue(),
    await asked.getByLabel('Events counted').inputValue(),
  ];
  const meant = 'tones|id|theirs|my-work|tone=kind|made=shop.made:id:tone';
  let kept = [];
  for (let tries = 0; tries < 50 && kept.join('|') !== meant; tries += 1) {
    if (tries > 0) await new Promise((later) => setTimeout(later, 100));
    kept = await written();
  }
  expect(`what was written to outlive the tab being changed, got ${JSON.stringify(kept)}`, kept.join('|') === meant);
  // Escape puts it away as it does any drawer, and nothing written is lost by it.
  await page.keyboard.press('Escape');
  await asked.waitFor({ state: 'detached' });
  await compare.click();
  expect('what was written to outlive the drawer being put away', (await asked.getByLabel('Way 2', { exact: true }).inputValue()) === 'mine');
  for (let tries = 0; tries < 50 && (await written()).join('|') !== meant; tries += 1) await new Promise((later) => setTimeout(later, 100));
  expect('each choice to say again what was chosen once the project has been read', (await written()).join('|') === meant);
  await asked.getByRole('button', { name: 'Start comparing', exact: true }).click();
  await asked.waitFor({ state: 'detached' });
  await page.getByText('Comparing ways of work as tones.').waitFor();

  // Four things are wanted: each goes one of the ways, turn about.
  for (const id of [1, 2, 3, 4]) await want(id);

  // Where the measurements are: what is compared, each way, and what was measured under each, side by side.
  await nav.getByText('Measurements', { exact: true }).click();
  await page.getByRole('heading', { name: 'Measurements', level: 1 }).waitFor();
  const panel = page.locator('.compared[data-comparison="tones"]');
  await panel.waitFor();
  const said = (await panel.locator('.panel-head').innerText()).replace(/\s+/g, ' ');
  expect(`it to say what is compared, what it counts and how it stands, got "${said}"`, said.includes('ways of work in shop') && said.includes('each id to one way') && said.includes('counting made') && said.includes('going'));
  const ways = await panel.locator('tr.compared-way').evaluateAll((rows) => rows.map((row) => [...row.querySelectorAll('td')].map((cell) => cell.textContent.trim()).join(' | ')));
  expect(`each way with its share, how many went and what it does, got ${JSON.stringify(ways)}`, ways.join(' / ') === 'theirs | 1 | 2 | as it is written / mine | 1 | 2 | starts my-work, tone=kind');
  // One table to decide from: a column for each way under the text it ran, how many things it was given, what came of them and what was measured.
  const counts = panel.locator('table.compared-counts');
  const cells = async (row) => (await row.locator('th, td').allInnerTexts()).map((cell) => cell.replace(/\s+/g, ' ').trim());
  const figures = async (what) => cells(counts.locator('tbody tr', { has: page.locator('td:first-child', { hasText: new RegExp(`^${what}$`) }) }));
  const heads = await cells(counts.locator('thead tr'));
  expect(`a column for each way, under the text it ran, got ${JSON.stringify(heads)}`, heads.length === 3 && heads[0] === '' && /^theirs version 1 · [0-9a-f]{8}$/.test(heads[1]) && /^mine version 1 · [0-9a-f]{8}$/.test(heads[2]));
  const given = await figures('Things given');
  expect(`how many things each way was given, got ${JSON.stringify(given)}`, given.join() === 'Things given,2,2');
  // What came of them, by the tone each was made in: counted under the way its thing went, with how long after.
  const plain = await figures('made plain');
  const kind = await figures('made kind');
  expect(`what was made under each way, got ${JSON.stringify([plain, kind])}`, plain[1] === '2' && plain[3] === '-' && kind[1] === '-' && kind[2] === '2');
  // One step a thing the one way and two the other: counted, each under its way.
  const steps = await figures('step');
  expect(`the steps under each way to be 2 and 4, got ${JSON.stringify(steps)}`, steps[1] === '2' && steps[3] === '4');
  const kinds = (await counts.locator('tr.compared-kind th:first-child').allInnerTexts()).map((cell) => cell.trim().toLowerCase());
  expect(`the rows to be headed by what they are, got ${JSON.stringify(kinds)}`, kinds.join() === 'came of them,measured');
  const named = (await cells(counts.locator('tr.compared-kind').first())).map((cell) => cell.toLowerCase());
  expect(`each column to say what its two figures are, got ${JSON.stringify(named)}`, named.join() === 'came of them,how many,after,how many,after');
  expect('no way to be marked as the better one', (await panel.locator('.badge, .ok, .failed, .better, .worse').count()) === 0 && !/better|worse|wins|best/i.test(await panel.innerText()));
  await page.screenshot({ path: `${out}/compared-1-side-by-side.png` });

  // Ended, saying which way everything goes from now on.
  await panel.getByRole('button', { name: 'End it', exact: true }).click();
  const ending = page.getByRole('dialog');
  await ending.getByRole('heading', { name: 'End tones' }).waitFor();
  await ending.getByLabel('From now on').selectOption('mine');
  await page.screenshot({ path: `${out}/compared-2-ending.png` });
  await ending.getByRole('button', { name: 'End it', exact: true }).click();
  await ending.waitFor({ state: 'detached' });
  await panel.locator('.panel-head', { hasText: 'ended; everything goes mine' }).waitFor();
  // A fifth thing goes that way, and is not counted as part of what was compared.
  await want(5);
  const fifth = JSON.parse((await cli('instances', '--all', '--json')).stdout).instances.find((one) => one.key === 'work-5');
  expect(`the fifth thing to have gone their own way, got ${fifth?.blueprint}`, fifth?.blueprint === 'my-work');
  const shown = JSON.parse((await cli('compare', 'show', 'tones', '--json')).stdout);
  expect(`nothing more to be counted under it, got ${JSON.stringify(shown.comparison.ways.map((way) => way.went))}`, shown.comparison.ways.map((way) => way.went).join() === '2,2');
  await page.screenshot({ path: `${out}/compared-3-ended.png` });

  // Said again: everything is as it is written, and it says nothing more of how the work is started.
  await panel.getByRole('button', { name: 'Say again which way goes on', exact: true }).click();
  await page.getByRole('dialog').getByLabel('From now on').selectOption({ label: 'as it is written' });
  await page.getByRole('dialog').getByRole('button', { name: 'Say so', exact: true }).click();
  await panel.locator('.panel-head', { hasText: 'ended; everything goes as it is written' }).waitFor();
  expect('nothing more to be said of it from here once it says nothing of how the work is started', (await panel.getByRole('button').count()) === 0);
  await want(6);
  const sixth = JSON.parse((await cli('instances', '--all', '--json')).stdout).instances.find((one) => one.key === 'work-6');
  expect(`the sixth thing to have been done as it is written, got ${sixth?.blueprint}`, sixth?.blueprint === 'work');

  // How what is running comes by a changed blueprint is the factory's, said in Settings. Having them followed says what it gives up before it is turned on.
  await page.locator('.rail').getByRole('button', { name: 'Settings', exact: true }).click();
  const setting = page.locator('.texts-setting');
  await setting.waitFor();
  expect('nothing to have been said of it yet', (await setting.getByLabel('Blueprints are').inputValue()) === 'usual');
  await setting.getByLabel('Blueprints are').selectOption('follow');
  const asks = setting.locator('.texts-asking');
  await asks.waitFor();
  expect('what following gives up to be said before it is turned on', (await asks.innerText()).includes('an agent among them, changes what runs with the authority of whoever applied it'));
  expect('nothing to have been changed by choosing', JSON.parse((await cli('texts', '--json')).stdout).come === 'usual');
  // The setting is down the page: it is brought into view to be looked at.
  await setting.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/compared-4-following-asked.png` });
  await asks.getByRole('button', { name: 'Have them followed', exact: true }).click();
  await setting.locator('.texts-standing', { hasText: 'taken as it changes' }).waitFor();
  expect('texts to be followed now', JSON.parse((await cli('texts', '--json')).stdout).come === 'follow');

  // Where they are followed a way follows its file: their own way is written again while it is compared, and what each text did is counted apart.
  await cli('compare', 'start', 'again', '--project', 'shop', '--of', 'work', '--per', 'id', '--way', 'theirs', '--way', 'mine=my-work');
  for (const id of [11, 12]) await want(id);
  await writeFile(join(blueprints, 'my-work.bp'), MINE.replace('version: 1', 'version: 2').replace('topic: shop.looked', 'topic: shop.looked_twice'));
  await cli('project', 'scan', 'shop');
  for (const id of [13, 14]) await want(id);
  await page.locator('.nav').getByText('Measurements', { exact: true }).click();
  const followed = page.locator('.compared[data-comparison="again"]');
  await followed.waitFor();
  const texts = (await followed.locator('table.compared-counts thead th.compare-value').allInnerTexts()).map((cell) => cell.replace(/\s+/g, ' ').trim());
  expect(`a column for each text each way ran, the one it runs now said, got ${JSON.stringify(texts)}`, texts.length === 3 && /^theirs version 1 · [0-9a-f]{8}$/.test(texts[0]) && /^mine version 1 · [0-9a-f]{8}$/.test(texts[1]) && /^mine version 2 · [0-9a-f]{8}, now$/.test(texts[2]));
  const each = (await followed.locator('table.compared-counts tbody tr', { has: page.locator('td:first-child', { hasText: /^step$/ }) }).locator('td').allInnerTexts()).map((cell) => cell.trim());
  expect(`the steps under each text apart, got ${JSON.stringify(each)}`, each[1] === '2' && each[3] === '2' && each[5] === '2');
  const shared = (await followed.locator('table.compared-counts tr.compared-given td').allInnerTexts()).map((cell) => cell.trim());
  expect(`how many things each text was given, got ${JSON.stringify(shared)}`, shared.join() === 'Things given,2,1,1');
  await page.screenshot({ path: `${out}/compared-5-a-way-that-follows-its-file.png` });

  // What somebody decides from. A project whose fleet works cards: two who work, each a stand-in that says what its turn cost, and one who reviews.
  const standin = join(home, 'standin.sh');
  await writeFile(standin, STANDIN);
  await chmod(standin, 0o755);
  for (const name of ['big', 'small', 'looks']) {
    await mkdir(join(home, name), { recursive: true });
    await writeFile(join(home, name, 'extension.yaml'), packaged(name, standin));
    await cli('extension', 'add', join(home, name));
  }
  await cli('project', 'create', '--bare', 'Works');
  const works = join(data, 'projects/works');
  const git = (...args) => run('git', args, { cwd: works, env });
  await git('init', '--quiet', '.');
  await git('config', 'user.name', 'Ada');
  await git('config', 'user.email', 'ada@example.com');
  await git('add', '-A');
  await git('commit', '--quiet', '--allow-empty', '-m', 'first');
  await cli('apply', 'std/board', '--project', 'works');
  await cli('fleet', 'init', 'works');
  await cli('apply', 'fleet', '--project', 'works');
  // Its rows: nobody plans, two work, one reviews.
  await cli('record', 'set', 'works-agents-1', 'count=0');
  await cli('record', 'set', 'works-agents-2', 'title=big', 'role=worker', 'adapter=big', 'wall=off');
  await cli('record', 'add', 'works', 'agents', 'title=small', 'role=worker', 'count=1', 'adapter=small', 'permission=edit', 'wall=off');
  await cli('record', 'set', 'works-agents-3', 'count=1', 'adapter=looks', 'wall=off');
  // The fleet brings its agents to what the rows now say. A card made ready before the one it is
  // handed to is there is taken and then waits for somebody to say so: it is waited for.
  await fleetTook(cli, 'works');
  for (let tries = 0; ; tries += 1) {
    const up = (await cli('agents')).stdout.split('\n').filter((line) => / active /.test(line)).map((line) => line.split(/\s+/)[0]);
    if (['works-big-1', 'works-small-1', 'works-reviewer-1'].every((name) => up.includes(name))) break;
    if (tries > 300) throw new Error(`the fleet never came to its rows: ${up.join(', ')}`);
    await new Promise((later) => setTimeout(later, 200));
  }
  // Each card goes to one of the two, turn about; a board's cards are counted the usual way with nothing said.
  const started = await cli('compare', 'start', 'models', '--project', 'works', '--of', 'std/lane-round', '--per', 'item', '--also', 'std/review', '--way', 'big', '--with', 'big.agent=works-big-1', '--way', 'small', '--with', 'small.agent=works-small-1');
  expect(`the terminal to say what it counts, got ${started.stdout}`, started.stdout.includes('It counts what comes of each thing: moved, verdict, asked, left, after.'));
  expect(`the terminal to say what is a card being accepted, got ${started.stdout}`, started.stdout.includes('A thing is accepted where it comes to moved done or verdict passed.'));
  const titles = ['Add a cart', 'Add a till', 'Add a shelf', 'Add a door'];
  for (const title of titles) await cli('board', 'add', 'works', title);
  for (let card = 1; card <= titles.length; card += 1) await cli('board', 'move', `works-${card}`, 'ready');
  for (let tries = 0; ; tries += 1) {
    const cards = JSON.parse((await cli('board', 'works', '--json')).stdout).items ?? [];
    if (cards.length === titles.length && cards.every((card) => card.status === 'done')) break;
    if (tries > 900) {
      const said = (await cli('events', '--topics', 'board.item.moved,card.*,review.*,agent.turn.finished,run.failed', '--limit', '80', '--json')).stdout;
      throw new Error(`the cards were never done: ${JSON.stringify(cards.map((card) => [card.id, card.status]))}\n${said}`);
    }
    await new Promise((later) => setTimeout(later, 200));
  }
  // What came after a card's work was merged. Two cards a person works and puts in review, which are
  // given no way: one whose branch reverts the first card's merge, and one that says it fixes the
  // second and changes its file again. Each merge says which card it merged, in its last line.
  const mergeOf = async (card) => (await git('log', '--first-parent', '--merges', '--format=%H', `--grep=^Item: ${card}$`)).stdout.trim();
  const [first, second] = [await mergeOf('works-1'), await mergeOf('works-2')];
  expect(`each card's merge to say which card, got ${first} and ${second}`, /^[0-9a-f]{40}$/.test(first) && /^[0-9a-f]{40}$/.test(second));
  const byHand = async (title, card, work) => {
    await cli('board', 'add', 'works', title);
    const place = join(works, '.lanes', card);
    await git('worktree', 'add', '--quiet', '-b', `lane/${card}`, place);
    await work((...args) => run('git', args, { cwd: place, env }), place);
    await cli('board', 'move', card, 'review');
  };
  await byHand('Undo the cart', 'works-5', (there) => there('revert', '--no-edit', '-m', '1', first));
  await byHand('Fix the till', 'works-6', async (there, place) => {
    await cli('board', 'set', 'works-6', 'fixes=works-2');
    const files = (await git('diff', '--name-only', `${second}^1`, second)).stdout.trim().split('\n');
    for (const file of files) await appendFile(join(place, file), 'put right\n');
    await there('commit', '--quiet', '-am', 'Fix the till');
  });
  for (let tries = 0; ; tries += 1) {
    const cards = JSON.parse((await cli('board', 'works', '--json')).stdout).items ?? [];
    if (['works-5', 'works-6'].every((id) => cards.some((card) => card.id === id && card.status === 'done'))) break;
    if (tries > 900) throw new Error(`the two cards put in review were never done: ${JSON.stringify(cards.map((card) => [card.id, card.status]))}`);
    await new Promise((later) => setTimeout(later, 200));
  }

  // The terminal's one screen says the same as the page.
  let screen = '';
  for (let tries = 0; ; tries += 1) {
    screen = (await cli('compare', 'show', 'models')).stdout;
    if (['after reverted', 'after changed_again', 'after fixed'].every((line) => screen.split('\n').some((row) => row.startsWith(line)))) break;
    if (tries > 300) throw new Error(`what came after the merges was never counted:\n${screen}`);
    await new Promise((later) => setTimeout(later, 200));
  }
  for (const line of ['things given', 'came of them', 'moved review>done', 'verdict passed', 'chosen', 'agent.turn approve', 'added up', 'agent.cost_usd', 'agent.input_tokens', 'agent.output_tokens', 'accepted', 'for what was accepted', 'after reverted', 'after changed_again', 'after fixed'])
    expect(`the terminal to have a line for ${line}, got\n${screen}`, screen.split('\n').some((row) => row.startsWith(line)));
  await writeFile(join(out, 'compared-6-what-came-of-the-work.txt'), screen);

  // The page was open while the work went on: it is read again when work finishes, at most every fifteen seconds, so it is opened again here.
  await page.reload();
  await page.locator('.rail').waitFor();
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Measurements', { exact: true }).click();
  const decided = page.locator('.compared[data-comparison="models"]');
  await decided.waitFor();
  const table = decided.locator('table.compared-counts');
  // A row by what it is of, and of what kind where two kinds have a row of it (what was spent, in all and over what was accepted).
  const row = async (what, kind) => (await table.locator(kind ? `tbody tr[data-kind="${kind}"]` : 'tbody tr', { has: page.locator('td:first-child', { hasText: new RegExp(`^${what}$`) }) }).locator('td').allInnerTexts()).map((cell) => cell.trim());
  const cards = await row('Things given');
  expect(`each of the two to have been given two cards, got ${JSON.stringify(cards)}`, cards.join() === 'Things given,2,2');
  const done = await row('moved review>done');
  expect(`every card to have come to done under its way, with how long after, got ${JSON.stringify(done)}`, done[1] === '2' && done[3] === '2' && /\d/.test(done[2]) && /\d/.test(done[4]));
  const back = await row('verdict failed');
  expect(`the one card sent back to be counted under its way alone, got ${JSON.stringify(back)}`, back.filter((cell) => cell === '1').length === 1 && back.includes('-'));
  // What it cost: in all, and a card. The one who is big costs 0.41 a turn and the small one 0.09, and each review 0.02.
  const cost = await row('agent.cost_usd', 'spent');
  const all = [cost[1], cost[3]].map((said) => Number(said.replace('$', '')));
  expect(`what each way cost in all and a card, in money, got ${JSON.stringify(cost)}`, cost.every((cell, at) => at === 0 || cell.startsWith('$')) && all[0] > all[1] && Math.abs(Number(cost[2].replace('$', '')) - all[0] / 2) < 0.001);
  const read = await row('agent.input_tokens', 'spent');
  expect(`what each way read in all and a card, got ${JSON.stringify(read)}`, /^\d{3},\d{3}$/.test(read[1]) && /^\d{3},\d{3}$/.test(read[3]));
  // What came after each card's work was merged, under the way the card went, once for each card that did it.
  // A way with none of it has one cell that says so; one with some, how many and how long after.
  const after = async (what) => (await row(`after ${what}`)).slice(1).filter((cell) => /^\d+$/.test(cell)).map(Number);
  const sum = (cells) => cells.reduce((all, one) => all + one, 0);
  const [reverted, again, fixed] = [await after('reverted'), await after('changed_again'), await after('fixed')];
  expect(`the first card's work reverted once, got ${JSON.stringify(reverted)}`, sum(reverted) === 1);
  expect(`the files of the first and the second changed again, once each, got ${JSON.stringify(again)}`, sum(again) === 2);
  expect(`the second said to be fixed once, got ${JSON.stringify(fixed)}`, sum(fixed) === 1);
  const chosen = await row('agent.turn approve');
  expect(`what those who reviewed chose under each way, got ${JSON.stringify(chosen)}`, chosen[1] === '2' && chosen[3] === '2');
  // How many of each way's cards were accepted (each came to done, its review passed), over how many it was given.
  const accepted = await row('accepted', 'accepted');
  expect(`each way's two cards to have been accepted, of the two it was given, got ${JSON.stringify(accepted)}`, accepted.join() === 'accepted,2,100%,2,100%');
  // And what each spent, over how many of its cards were accepted: what was spent on the one sent back is in it.
  const perAccepted = await row('agent.cost_usd', 'per_accepted');
  const spent = [perAccepted[1], perAccepted[2], perAccepted[3], perAccepted[4]].map((said) => Number(said.replace('$', '')));
  expect(`what each way spent over the cards accepted, in money, got ${JSON.stringify(perAccepted)}`, perAccepted.slice(1).every((cell) => cell.startsWith('$')) && spent[0] === all[0] && spent[2] === all[1] && Math.abs(spent[1] - all[0] / 2) < 0.001 && Math.abs(spent[3] - all[1] / 2) < 0.001);
  const headed = (await table.locator('tr.compared-kind th:first-child').allInnerTexts()).map((cell) => cell.trim().toLowerCase());
  expect(`the six kinds of row in their order, got ${JSON.stringify(headed)}`, headed.join() === 'came of them,chosen,added up,accepted,for what was accepted,measured');
  const legend = await decided.locator('.compared-legend').innerText();
  expect(`the legend to say what accepted and each accepted are, got "${legend}"`, legend.includes('Of those given is how many') && legend.includes('each accepted is what the way spent in all'));
  expect('no way to be marked as the better one', (await decided.locator('.badge, .ok, .failed, .better, .worse').count()) === 0 && !/better|worse|wins|best|score/i.test(await decided.innerText()));
  // The whole of the table in one picture: the window is made tall enough to hold it.
  await page.setViewportSize({ width: 1280, height: 1500 });
  await decided.scrollIntoViewIfNeeded();
  await decided.screenshot({ path: `${out}/compared-6-what-came-of-the-work.png` });
  // Stopped before the daemon is: nothing of the fleet's is left at work.
  await cli('compare', 'end', 'models');

  // Two prompts for whoever reviews, with nothing else changed. Other words of the project's own are read by one way where the seat links its file; each card's review goes one way.
  await mkdir(join(data, 'blueprints', 'projects', 'works', 'prompts'), { recursive: true });
  await writeFile(join(data, 'blueprints', 'projects', 'works', 'prompts', 'strict.md'), 'Card ${item}: ${title}\n\nBe strict with it. ${where}\n');
  await cli('project', 'scan', 'works');
  await cli('compare', 'start', 'prompts', '--project', 'works', '--of', 'std/review', '--per', 'item', '--way', 'usual', '--way', 'strict', '--prompt', 'strict.std/prompts/review-seat.md=prompts/strict.md');
  for (const title of ['Add a sign', 'Add a bell']) await cli('board', 'add', 'works', title);
  // After the four worked and the two a person put in review.
  for (const card of [7, 8]) await cli('board', 'move', `works-${card}`, 'ready');
  for (let tries = 0; ; tries += 1) {
    const cards = JSON.parse((await cli('board', 'works', '--json')).stdout).items ?? [];
    if (cards.length === 8 && cards.every((card) => card.status === 'done')) break;
    if (tries > 900) throw new Error(`the two cards were never done: ${JSON.stringify(cards.map((card) => [card.id, card.status]))}`);
    await new Promise((later) => setTimeout(later, 200));
  }
  // The terminal says which words each way ran, and the page the same.
  const two = (await cli('compare', 'show', 'prompts')).stdout;
  expect(`the terminal to say what the way reads, got\n${two}`, two.includes('reads prompts/strict.md for std/prompts/review-seat.md'));
  const ran = two.split('\n').find((row) => row.startsWith('std/prompts/review-seat.md')) ?? '';
  expect(`each column to say which words it was run with, got "${ran}"`, /as it is linked\s+prompts\/strict\.md [0-9a-f]{8}$/.test(ran));
  await writeFile(join(out, 'compared-7-two-prompts.txt'), two);
  await page.reload();
  await page.locator('.rail').waitFor();
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Measurements', { exact: true }).click();
  const prompted = page.locator('.compared[data-comparison="prompts"]');
  await prompted.waitFor();
  const does = await prompted.locator('tr.compared-way').evaluateAll((rows) => rows.map((row) => [...row.querySelectorAll('td')].map((cell) => cell.textContent.trim()).join(' | ')));
  expect(`the way to say what it reads, got ${JSON.stringify(does)}`, does[0].endsWith('as it is written') && does[1].endsWith('reads prompts/strict.md for std/prompts/review-seat.md'));
  const words = (await prompted.locator('table.compared-counts thead .compared-reads').allInnerTexts()).map((line) => line.trim());
  expect(`the column of the way that reads other words to say which, got ${JSON.stringify(words)}`, words.length === 1 && /^reads prompts\/strict\.md · [0-9a-f]{8}$/.test(words[0]));
  const reviews = (await prompted.locator('table.compared-counts tr.compared-given td').allInnerTexts()).map((cell) => cell.trim());
  expect(`each way to have been given one card's review, got ${JSON.stringify(reviews)}`, reviews.join() === 'Things given,1,1');
  // Wide enough for both columns whole: what the way reads makes its line of the first table a long one.
  await page.setViewportSize({ width: 1700, height: 1500 });
  await prompted.scrollIntoViewIfNeeded();
  await prompted.screenshot({ path: `${out}/compared-7-two-prompts.png` });
  // Ended saying the project's words go on, and then as it is written: nothing of it is left in force.
  await cli('compare', 'end', 'prompts', '--goes', 'strict');
  await cli('compare', 'end', 'prompts');

  // A gradual rollout: a new setup given a part of the things that grows while what is counted of them holds,
  // by a rule written from a terminal; the shares said again by the rule and by somebody, each with why, on a
  // page that is not read again, with nothing of it over anything else or scrolled across.
  await cli('compare', 'end', 'again');
  await cli('compare', 'start', 'rollout', '--project', 'shop', '--of', 'work', '--per', 'id', '--way', 'usual', '--way', 'new', '--with', 'new.tone=bright', '--count', 'made=shop.made:id:tone', '--grows', 'new', '--against', 'usual', '--holds', 'made=dull', '--over', '1', '--step', '25', '--most', '75');
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.locator('.nav').getByText('Measurements', { exact: true }).click();
  const rolled = page.locator('.compared[data-comparison="rollout"]');
  await rolled.locator('.compared-rule').waitFor();
  expect('the rule to be said in the daemon\'s words', (await rolled.locator('.compared-rule').innerText()).includes('new is given 25 more of every hundred things at a time, up to 75'));
  expect('each way to be given half to begin with', (await rolled.locator('.compared-parts').innerText()) === 'Shared out usual 50%, new 50%.');
  for (const id of [21, 22]) await want(id);
  // Neither made anything dull: the new setup is given three of every four from here, said with why.
  await rolled.locator('.compared-changes li').first().waitFor();
  await rolled.locator('.compared-parts', { hasText: 'usual 25%, new 75%' }).waitFor();
  const raised = await rolled.locator('.compared-changes li').first().innerText();
  expect(`the change to say why, got ${raised}`, /its rule/.test(raised) && raised.includes('so its share goes from 50% to 75%, as far as its rule takes it.'));
  expect('the rule to say it is done', (await rolled.locator('.compared-rule').innerText()).includes('It has taken its way as far as it goes.'));
  await rolled.scrollIntoViewIfNeeded();
  await rolled.screenshot({ path: `${out}/compared-8-rolled-out.png` });
  // Said again by somebody from a terminal, with why: on this page, which is not read again, the newest first.
  await cli('compare', 'share', 'rollout', 'usual=3', 'new=1', '--why', 'Held back while the bright ones are looked at.');
  await rolled.locator('.compared-changes li', { hasText: 'Held back while the bright ones are looked at.' }).waitFor();
  const changes = await rolled.locator('.compared-changes li').allInnerTexts();
  expect(`two changes, the newest first, got ${JSON.stringify(changes)}`, changes.length === 2 && changes[0].includes('user:') && changes[1].includes('its rule'));
  await rolled.locator('.compared-parts', { hasText: 'usual 75%, new 25%' }).waitFor();
  // Changed from the comparison's view by whoever may: a most of all of it refused in the daemon's sentence with the
  // dialog staying, then one that goes back a step to a least; and taken away.
  await rolled.getByRole('button', { name: 'Change its rule…', exact: true }).click();
  const ruling = page.getByRole('dialog');
  await ruling.getByRole('heading', { name: 'Change the rule of rollout' }).waitFor();
  expect('the rule to be written as it is kept', (await ruling.getByLabel('Grows', { exact: true }).inputValue()) === 'new' && (await ruling.getByLabel('Coming to').inputValue()) === 'dull' && (await ruling.getByLabel('Least', { exact: true }).inputValue()) === '');
  await ruling.getByLabel('Most', { exact: true }).fill('100');
  await ruling.getByRole('button', { name: 'Change it', exact: true }).click();
  await ruling.locator('.comparing-wrong', { hasText: 'all of it is ending the comparison' }).waitFor();
  for (const [width, height] of [[1280, 860], [640, 860], [390, 844]]) {
    await page.setViewportSize({ width, height });
    const found = await ruling.evaluate((dialog) => {
      // What the dialog is left by stays at its foot over whatever is scrolled under it, as a dialog's foot does: the
      // two are held each to itself, and the foot to being in sight.
      const foot = dialog.querySelector('.modal-foot');
      const of = (els) => els.map((el) => ({ what: (el.getAttribute('aria-label') ?? el.textContent ?? '').trim().slice(0, 30), box: el.getBoundingClientRect() }));
      const fields = of([...dialog.querySelectorAll('.comparing-label, .comparing-said, input, select, button, h2, .comparing-wrong')].filter((el) => !foot.contains(el)));
      const left = of([...foot.querySelectorAll('button')]);
      const pairs = (boxes) => boxes.flatMap((a, at) => boxes.slice(at + 1).filter((b) => Math.min(a.box.right, b.box.right) - Math.max(a.box.left, b.box.left) > 0.5 && Math.min(a.box.bottom, b.box.bottom) - Math.max(a.box.top, b.box.top) > 0.5).map((b) => `${a.what} over ${b.what}`));
      const over = [...pairs(fields), ...pairs(left), ...(foot.getBoundingClientRect().bottom > window.innerHeight + 0.5 ? ['the foot out of sight'] : [])];
      const boxes = [...fields, ...left];
      const outside = boxes.filter((one) => one.box.left < -0.5 || one.box.right > window.innerWidth + 0.5).map((one) => one.what);
      return { over, outside, page: document.documentElement.scrollWidth - window.innerWidth };
    });
    expect(`nothing of the rule's dialog over anything else at ${width} wide, got ${found.over.join('; ')}`, found.over.length === 0);
    expect(`the rule's dialog in the window at ${width} wide, got ${found.outside.join('; ')}`, found.outside.length === 0 && found.page <= 0);
    if (width === 390) await page.screenshot({ path: `${out}/compared-10-rule-dialog-phone.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await ruling.getByLabel('Most', { exact: true }).fill('90');
  await ruling.getByLabel('Least', { exact: true }).fill('10');
  await ruling.getByRole('button', { name: 'Change it', exact: true }).click();
  await ruling.waitFor({ state: 'detached' });
  await rolled.locator('.compared-rule', { hasText: 'up to 90' }).waitFor();
  expect('the rule changed to go back to its least', (await rolled.locator('.compared-rule').innerText()).includes('and goes back as much each time it did not, to 10 at least.'));
  expect('the terminal to say the same', (await cli('compare', 'show', 'rollout')).stdout.includes('to 10 at least'));

  const rolledScreen = (await cli('compare', 'show', 'rollout')).stdout;
  expect(`the command line to say the same, got ${rolledScreen}`, rolledScreen.includes('Its things are shared out usual 75%, new 25%.') && rolledScreen.includes('by the rule: `new` came to `dull`') && rolledScreen.includes('Held back while the bright ones are looked at.'));
  for (const [width, height] of [[1280, 860], [900, 860], [640, 860], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await rolled.locator('.compared-rolled').scrollIntoViewIfNeeded();
    const found = await rolled.evaluate((panel) => {
      const part = panel.querySelector('.compared-rolled');
      const box = part.getBoundingClientRect();
      const lines = [...part.querySelectorAll('p, li')].map((line) => line.getBoundingClientRect());
      const over = lines.some((a, at) => lines.slice(at + 1).some((b) => a.bottom > b.top + 1 && b.bottom > a.top + 1));
      const outside = lines.some((line) => line.left < box.left - 1 || line.right > box.right + 1);
      return { over, outside, sideways: part.scrollWidth - part.clientWidth, page: document.documentElement.scrollWidth - document.documentElement.clientWidth };
    });
    const how = `at ${width} wide`;
    expect(`no line of the shares over another ${how}`, !found.over);
    expect(`every line of the shares inside them ${how}`, !found.outside);
    expect(`the shares not to scroll sideways ${how}, by ${found.sideways}`, found.sideways <= 0);
    expect(`the page not to scroll sideways ${how}, by ${found.page}`, found.page <= 0);
    // Every comparison's panel is no wider than the page it is on, whatever a way does or how many columns it has.
    const view = await rolled.evaluate((panel) => {
      const tab = panel.closest('.view');
      const within = tab.getBoundingClientRect();
      const wider = [...tab.querySelectorAll('.compared')].filter((one) => one.getBoundingClientRect().right > within.right + 0.5).map((one) => one.dataset.comparison);
      // What reaches past it, where anything does, said with the failure.
      const past = [...tab.querySelectorAll('*')].filter((el) => el.getBoundingClientRect().right > within.right + 0.5 && ![...el.children].some((child) => child.getBoundingClientRect().right > within.right + 0.5)).slice(0, 6).map((el) => `${el.tagName}.${el.className} ${Math.round(el.getBoundingClientRect().width)} ${(el.textContent ?? '').slice(0, 30)}`);
      return { sideways: tab.scrollWidth - tab.clientWidth, wider, past };
    });
    expect(`Measurements not to scroll sideways ${how}, by ${view.sideways}: ${view.past.join(' | ')}`, view.sideways <= 0);
    expect(`no comparison wider than Measurements ${how}, got ${view.wider.join(', ')}`, view.wider.length === 0);
    if (width === 390) await rolled.locator('.compared-rolled').screenshot({ path: `${out}/compared-9-rolled-phone.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await rolled.getByRole('button', { name: 'Change its rule…', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Take the rule away', exact: true }).click();
  await rolled.getByRole('button', { name: 'Give it a rule…', exact: true }).waitFor();
  expect('the rule taken away', (await rolled.locator('.compared-rule').count()) === 0);
  await cli('compare', 'end', 'rollout');

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
