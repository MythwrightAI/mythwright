// A sandbox test of a blueprint, run from its page.
//
//   DAEMON=../target/debug/factory node scripts/sandboxed.mjs [out-dir]
//
// It goes by what the page shows and by the boxes:
//
//   - the board that ships, run from its page by Sandbox test under Testing in its head:
//     the view called Sandbox, each run under what it is a run of with
//     its steps by what they say of themselves, the text's graph lit
//     with a mark on each tile that ran, the table with no rows, and at
//     the foot the daemon's own sentences: what this was, and that
//     nothing was stood in;
//   - a card added by hand and moved, the handler that hears the move
//     run, the table with its row, and each thing done listed in order;
//   - a project's own text that runs a command, waits and asks an agent:
//     the stand-in that waits answered to fail, and the test saying why
//     it failed; that answer left out and the text run again; the
//     stand-in answered as usual, the clock moved on past the wait, the
//     next answered with what is written out, which is read as JSON
//     first; an event said to it, by a topic the page offers because
//     the text hears it, and the handler that hears it run; the foot
//     saying what was stood in and how often; and the file the command
//     would have written not there;
//   - the text changed on the page, not saved, and run again: the same
//     acts played against it, and what differs said;
//   - a text that does not check is not run, and says so;
//   - the daemon made to end the sandbox under the page: the page says
//     it has it no longer and runs it again;
//   - the tab closed, and the daemon holding no sandbox of it, asked
//     with the page's own key;
//   - the fleet that ships, run from its page: what goes first offered
//     from what the daemon says a fleet needs, the fleet up over its
//     board, and what it told whoever applied it said with its tables;
//   - nothing of any of it in the factory: no instance, no project, no
//     line in the side panel, on Activity or in the inbox;
//   - nothing of the view over anything else of it, outside its part or
//     scrolling sideways, the foot in view, at three widths and a
//     phone's; and the other of dark and light.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';
import { pressTesting } from './testing.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'sandboxed-data');
const home = join(out, 'sandboxed-home');
const shop = join(out, 'sandboxed-shop');
/** What the command of the project's own text would write, were it run. */
const would = join(out, 'sandboxed-would-have-run');

// A project's own blueprint: a command, a wait, an agent asked, and what it keeps of the answer.
const OWN = `blueprint: release
version: 1
description: Build what is there, wait a while, ask which way to go, and keep the answer.
permissions: [shell, agents]

state:
  way: none
  asked: nothing

steps:
  build:
    description: Build it. The rest is not said.
    do: shell
    with: { cmd: [touch, '${would}'] }
  pause:
    description: Let it settle for ten minutes.
    do: wait
    with: { for: 10m }
  ask:
    description: Ask which way to go.
    do: agent.task
    with: { prompt: "Which way?", role: worker }
  note:
    do: set
    with: { way: "\${steps.ask.out.reply}" }

on:
  asked:
    event: release.asked
    do: set
    with: { asked: "\${event.data.version}" }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop, would]) await rm(dir, { recursive: true, force: true });
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
let page = null;
let small = null;
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
  const scheme = process.env.SCHEME ?? 'dark';
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async (context, { at = '', touch = false } = {}) => {
    const opened = await context.newPage();
    opened.setDefaultTimeout(20_000);
    opened.on('pageerror', (err) => problems.push(err.message));
    await opened.goto(`${url}/${at}`);
    await opened.getByLabel('Email').fill('ada@example.com');
    await opened.getByLabel('Password').fill('correct horse');
    const press = opened.getByRole('button', { name: 'Sign in', exact: true }).last();
    await (touch ? press.tap() : press.click());
    return opened;
  };
  page = await signIn(await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme }), { at: '#/blueprint/std/board' });
  await page.locator('.rail').waitFor();

  /** Asks the daemon something as the page does, with the page's own key: what it answered, and with what. */
  const asks = (method, path, body, on = page) =>
    on.evaluate(
      async ([how, to, sent]) => {
        const answer = await fetch(to, { method: how, headers: { 'content-type': 'application/json', 'x-page-key': localStorage.getItem('page-key') ?? '' }, body: sent === null ? undefined : JSON.stringify(sent) });
        return { status: answer.status, body: await answer.json().catch(() => null) };
      },
      [method, path, body ?? null],
    );
  const ending = (await asks('GET', '/api/meta')).body.blueprint_ext;
  const laid = (await asks('POST', '/api/blueprints/format', { source: OWN, project: 'shop' })).body.formatted;
  expect('the daemon to lay the blueprint out', typeof laid === 'string' && laid.includes('agent.task'));
  // A blueprint of the project's own, where the factory keeps it (`@blueprints/` of the project's).
  await mkdir(join(data, 'blueprints', 'projects', 'shop'), { recursive: true });
  await writeFile(join(data, 'blueprints', 'projects', 'shop', `release.${ending}`), laid);

  // ---- what the check reads of the page ------------------------------------------
  const drawn = (on = page) => on.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const shot = (name, on = page) => on.screenshot({ path: `${out}/${name}.png` });
  const head = (on = page) => on.locator('.view-head').locator('visible=true').last();
  const view = (on = page) => on.locator('.sandbox').locator('visible=true');
  const pressed = async (on = page) => (await head(on).locator('.pills button[aria-pressed="true"]').innerText()).trim();
  /** How many things asked of the sandbox of the tab that is showing were answered: its button says, whichever of the page's views is showing. */
  const answered = (on = page) => on.evaluate(() => Number([...document.querySelectorAll('.view-head button[data-answered]')].find((one) => one.getClientRects().length > 0)?.dataset.answered ?? 0));
  /**
   * Does something that asks the sandbox a thing, and waits until it is
   * answered and read: the page counts what was answered, and the view
   * says while it is working. Not by the view being idle alone: it is
   * idle before what was pressed has been asked.
   */
  const asked = async (press, on = page) => {
    const was = await answered(on);
    await press();
    await on.waitForFunction((before) => {
      const count = Number([...document.querySelectorAll('.view-head button[data-answered]')].find((one) => one.getClientRects().length > 0)?.dataset.answered ?? 0);
      const shown = [...document.querySelectorAll('.sandbox')].find((one) => one.getClientRects().length > 0);
      return !!shown && count > before && shown.dataset.busy === 'false';
    }, was);
    await drawn(on);
  };
  /** Which sandbox the daemon has for the view that is showing; nothing where it has none. */
  const sandboxId = (on = page) => view(on).getAttribute('data-sandbox');
  /** What the daemon hands of that sandbox, asked with the page's own key. */
  const handed = async (on = page) => (await asks('GET', `/api/sandboxes/${await sandboxId(on)}`, null, on)).body;
  /** What the view says happened: how it stands, each run's heading with its steps, the tiles that are lit, and the foot. */
  const says = (on = page) =>
    on.evaluate(() => {
      const shown = [...document.querySelectorAll('.sandbox')].find((one) => one.getClientRects().length > 0);
      const text = (el) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();
      return {
        outcome: text(shown.querySelector('.sandbox-outcome')),
        how: [...(shown.querySelector('.sandbox-outcome')?.classList ?? [])].find((one) => ['good', 'bad', 'waits'].includes(one)) ?? null,
        runs: [...shown.querySelectorAll('.sandbox-run')].map((run) => ({
          heading: text(run.querySelector('h4')),
          steps: [...run.querySelectorAll('.sandbox-step')].map((step) => ({ key: step.dataset.step, came: step.dataset.came, word: text(step.querySelector('.badge')), says: text(step.querySelector('.step-says')), gave: text(step.querySelector('.sandbox-gave')) })),
          lines: [...run.querySelectorAll('.sandbox-line')].map(text),
        })),
        lit: [...shown.querySelectorAll('.sandbox-graph .gnode')].map((node) => ({ id: node.dataset.node, how: [...node.classList].find((one) => one.startsWith('status-'))?.slice(7) ?? null, mark: text(node.querySelector('.gbadge text')) })),
        legend: [...shown.querySelectorAll('.sandbox-legend li')].map(text),
        tables: [...shown.querySelectorAll('.sandbox-table summary')].map(text),
        talk: [...shown.querySelectorAll('.sandbox-talk li')].map(text),
        differs: [...shown.querySelectorAll('.sandbox-differs li')].map(text),
        notes: [...shown.querySelectorAll('.sandbox-note')].map(text),
        waits: [...shown.querySelectorAll('.sandbox-waits')].map(text),
        done: [...shown.querySelectorAll('.sandbox-done li > span')].map(text),
        foot: { said: text(shown.querySelector('.sandbox-said')), stood: text(shown.querySelector('.sandbox-stood-said')) },
      };
    });
  /** The boxes of the view that is showing: its three parts, and what each is made of. */
  const looked = (on = page) =>
    on.evaluate(() => {
      const shown = [...document.querySelectorAll('.sandbox')].find((one) => one.getClientRects().length > 0);
      const box = (el) => {
        const at = el.getBoundingClientRect();
        return { left: at.left, right: at.right, top: at.top, bottom: at.bottom, what: (el.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 40) };
      };
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const crossed = [];
      const among = (boxes) => boxes.forEach((a, at) => boxes.slice(at + 1).forEach((b) => over(a, b) && crossed.push(`"${a.what}" over "${b.what}"`)));
      const [acts, happened, foot] = ['.sandbox-acts', '.sandbox-happened', '.sandbox-foot'].map((part) => shown.querySelector(part));
      const within = box(shown);
      // The foot is over neither part: what scrolls, scrolls behind nothing of it.
      const body = box(shown.querySelector('.sandbox-body'));
      if (foot && over(body, box(foot))) crossed.push('the foot over what scrolls');
      // The two parts are side by side or one under the other, as tall as what is in them: never over each other.
      among([box(acts), box(happened)]);
      const outside = [];
      for (const part of [acts, happened]) {
        const own = box(part);
        // Each thing asked, and each run and part of what happened, with its neighbours and inside its part.
        for (const list of [part.querySelectorAll('.sandbox-form'), part.querySelectorAll('.sandbox-run'), part.querySelectorAll(':scope .sandbox-came > *')]) {
          const boxes = [...list].map(box);
          among(boxes);
          for (const one of boxes) if (one.left < own.left - 0.5 || one.right > own.right + 0.5) outside.push(one.what);
        }
        for (const form of part.querySelectorAll('.sandbox-form')) {
          const [mine, fields] = [box(form), [...form.children].map(box)];
          among(fields);
          for (const one of fields) if (one.left < mine.left - 0.5 || one.right > mine.right + 0.5) outside.push(one.what);
          for (const field of form.querySelectorAll('.comparing-field')) {
            const [wide, inside] = [box(field), [...field.children].filter((child) => child.tagName !== 'DATALIST').map(box)];
            among(inside);
            for (const one of inside) if (one.left < wide.left - 0.5 || one.right > wide.right + 0.5) outside.push(one.what);
          }
        }
        for (const step of part.querySelectorAll('.sandbox-step')) {
          const [mine, cells] = [box(step), [...step.children].map(box)];
          among(cells);
          for (const one of cells) if (one.left < mine.left - 0.5 || one.right > mine.right + 0.5) outside.push(one.what);
        }
      }
      const view = box(shown.closest('.view'));
      return {
        crossed,
        outside,
        // The foot is in view: inside the view's box, and so on the screen.
        foot: foot ? box(foot).bottom <= view.bottom + 0.5 && box(foot).top >= view.top && box(foot).bottom <= window.innerHeight + 0.5 && foot.scrollHeight - foot.clientHeight <= 1 : null,
        beyond: within.right > view.right + 0.5 || within.left < view.left - 0.5,
        sideways: document.documentElement.scrollWidth - window.innerWidth,
        inner: [acts, happened, shown.querySelector('.sandbox-body')].map((part) => part.scrollWidth - part.clientWidth),
        forms: acts.querySelectorAll('.sandbox-form').length,
      };
    });
  const laidOut = async (when, on = page) => {
    await drawn(on);
    const now = await looked(on);
    expect(`nothing of the view to lie over anything else of it ${when}, got ${now.crossed.slice(0, 4).join('; ')}`, now.crossed.length === 0);
    expect(`everything to be inside its part ${when}, got ${now.outside.slice(0, 4).join('; ')}`, now.outside.length === 0 && !now.beyond);
    expect(`nothing to scroll sideways ${when}, got the page ${now.sideways} and the parts ${now.inner}`, now.sideways <= 0 && now.inner.every((over) => over <= 0));
    expect(`the foot to be in view, whole, ${when}`, now.foot !== false);
    return now;
  };
  // Sandbox test is a line of Testing in the head, pressed by the pointer or by a touch.
  const sandboxTest = (on = page) => ({ click: () => pressTesting(on, head(on), 'Sandbox test'), tap: () => pressTesting(on, head(on), 'Sandbox test', true) });
  const form = (name, on = page) => view(on).getByRole('form', { name, exact: true });

  // ---- the board that ships, run from its page ------------------------------------
  await head().getByText('std/board').first().waitFor();
  await head().getByText('checks').waitFor();
  expect('no view called Sandbox before one is run', (await head().getByRole('tab', { name: 'Sandbox' }).count()) === 0);
  await asked(() => sandboxTest().click());
  expect(`the view called Sandbox to be showing, got ${await pressed()}`, (await pressed()) === 'Sandbox');
  let now = await says();
  let daemons = await handed();
  expect(`the board to be up in its sandbox, got ${now.how}: ${now.outcome}`, now.how === 'good' && now.outcome.includes('it is up'));
  expect(`each run under what it is a run of, got ${now.runs.map((one) => one.heading).join(' | ')}`, now.runs.some((one) => one.heading === 'std/board: its steps') && now.runs.some((one) => one.heading === 'std/cards (cards): when first'));
  const boards = now.runs.find((one) => one.heading === 'std/board: its steps');
  expect(`the board's steps by what they say of themselves, with how each came out in a word, got ${JSON.stringify(boards.steps)}`, boards.steps[0].key === 'board' && boards.steps[0].word === 'ran' && boards.steps[0].says.startsWith('The table a card is a row of'));
  expect(`what was said of the table under the run it was said for, got ${boards.lines.join(' | ')}`, boards.lines.some((line) => line.startsWith('said collection.ensured')));
  // The text's own graph, lit: each tile that ran has a mark as well as a colour, and the list under it says what each means.
  const ran = now.lit.filter((tile) => tile.how === 'ok');
  expect(`the tiles of the board's own steps to be lit as having run, with a mark, got ${JSON.stringify(now.lit)}`, ran.length === 2 && ran.every((tile) => tile.mark === '✓') && ran.map((tile) => tile.id).sort().join(',') === 'main/board,main/cards');
  expect(`the list under the graph to say what each mark means, got ${now.legend.join(' | ')}`, now.legend.includes('✓ ran') && now.legend.includes('S stood in') && now.legend.includes('– passed over'));
  expect(`the table with no rows, got ${now.tables.join(' | ')}`, now.tables.length === 1 && now.tables[0] === 'board 0 rows');
  // The foot is the daemon's own sentences, word for word.
  expect(`the foot to say what this was in the daemon's words, got ${now.foot.said}`, now.foot.said === daemons.said && now.foot.said.startsWith('This was a sandbox test') && now.foot.said.endsWith('passing here is not passing for real.'));
  expect(`the foot to say that nothing was stood in, got ${now.foot.stood}`, now.foot.stood === daemons.stood_said && now.foot.stood.startsWith('Nothing was stood in'));
  for (const width of [1280, 1000, 760]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(`for the board at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await shot(`sandboxed-1-board-${scheme}`);

  const boardsSandbox = await sandboxId();

  // ---- a card added by hand and moved: the handler that hears the move runs -------
  const byHand = form('Do something by hand');
  await byHand.locator('select').first().selectOption({ label: 'Add a card' });
  await byHand.getByLabel(/^title/).fill('Try it');
  await asked(() => byHand.getByRole('button', { name: 'Do it', exact: true }).click());
  await byHand.locator('select').first().selectOption({ label: 'Move a card' });
  await byHand.getByLabel(/^id/).fill('sandbox-1');
  await byHand.getByLabel(/^to \(needed\)/).fill('doing');
  await asked(() => byHand.getByRole('button', { name: 'Do it', exact: true }).click());
  now = await says();
  expect(`the table to have its row, got ${now.tables.join(' | ')}`, now.tables[0] === 'board 1 row');
  expect(`the handler that hears a card moved to have run, under what it is a run of, got ${now.runs.map((one) => one.heading).join(' | ')}`, now.runs.some((one) => one.heading === 'std/cards (cards): when moved' && one.lines.some((line) => line === 'heard board.item.moved')));
  // What was said by hand stands apart from any run.
  expect('what was said by hand to stand under no run', now.runs.some((one) => one.heading === '' && one.lines.some((line) => line.startsWith('said board.item.moved'))));
  expect(`each thing done to be listed in the order it was done, got ${now.done.join(' | ')}`, now.done.join('|') === 'Did board.add|Did board.move');
  await laidOut('with a card added and moved by hand');
  await shot(`sandboxed-2-by-hand-${scheme}`);

  // ---- a project's own text: a command, a wait and an agent asked ------------------
  await page.evaluate((to) => (location.hash = to), `/file/shop/@blueprints/release.${ending}`);
  await head().getByText('release', { exact: false }).first().waitFor();
  await head().getByText('checks').waitFor();
  await asked(() => sandboxTest().click());
  now = await says();
  expect(`the test to wait for its stand-in to be answered, got ${now.how}: ${now.outcome}`, now.how === 'waits' && now.waits.length === 1);
  expect(`the stand-in to say what it was asked, in words: the action's sentence and what it was handed, got ${now.waits[0]}`, now.waits[0].includes('asked shell, which was stood in') && now.waits[0].includes('cmd') && now.waits[0].includes('touch'));
  await laidOut('with a stand-in waiting');
  await shot(`sandboxed-3-a-stand-in-waits-${scheme}`);
  // Answered to fail: its step fails with the words it was given, and the test says so.
  const waiting = () => view().locator('.sandbox-waits');
  await waiting().getByLabel('It fails', { exact: true }).check();
  await waiting().getByLabel('Why it fails').fill('no compiler here');
  await asked(() => waiting().getByRole('button', { name: 'Answer it', exact: true }).click());
  now = await says();
  expect(`the test to say that it failed, and why, got ${now.how}: ${now.outcome}`, now.how === 'bad' && now.outcome.includes('no compiler here'));
  expect(`the failed step's tile to be lit as failed, with its mark, got ${JSON.stringify(now.lit)}`, now.lit.some((tile) => tile.id === 'main/build' && tile.how === 'failed' && tile.mark === '✕'));
  expect(`the answer to be listed, got ${now.done.join(' | ')}`, now.done.join('|') === 'Answered the 1st stand-in: it fails');
  // That answer left out, and the text run again: the stand-in waits again.
  await view().locator('.sandbox-done li button').click();
  expect('an act left out to be listed no longer', (await view().locator('.sandbox-done li').count()) === 0);
  await asked(() => view().getByRole('button', { name: 'Run again', exact: true }).click());
  expect('the stand-in to wait again', (await waiting().count()) === 1);
  // Answered as usual: a command ends with 0. Then the step that waits, waits for the clock.
  await asked(() => waiting().getByRole('button', { name: 'Answer it', exact: true }).click());
  now = await says();
  expect(`the test to wait for the clock, got ${now.how}: ${now.outcome}`, now.how === 'waits' && now.runs.some((one) => one.lines.includes('pause waits for the clock to be moved on')));
  const clock = form('Move the clock on');
  await clock.getByLabel('How long').fill('10m');
  await asked(() => clock.getByRole('button', { name: 'Move it on', exact: true }).click());
  expect('the agent asked for to be a stand-in that waits', (await waiting().count()) === 1 && (await waiting().innerText()).includes('Which way?'));
  // An answer written out is read as JSON before it is sent: one that does not read is not sent.
  await waiting().getByLabel('An answer written out', { exact: true }).check();
  await waiting().getByLabel('What it gives').fill('{ reply: left }');
  await waiting().getByRole('button', { name: 'Answer it', exact: true }).click();
  await waiting().locator('.comparing-wrong', { hasText: 'That does not read as JSON, so it was not sent' }).waitFor();
  expect('the stand-in to wait still', (await handed()).waiting.length === 1);
  await waiting().getByLabel('What it gives').fill('{ "reply": "left", "available": true }');
  await asked(() => waiting().getByRole('button', { name: 'Answer it', exact: true }).click());
  now = await says();
  daemons = await handed();
  expect(`the test to have come up, got ${now.how}: ${now.outcome}`, now.how === 'good' && now.outcome.includes('it is up'));
  // An event said to it: the topic is offered because the text hears it, and what it carries is written as a mapping.
  const saying = form('Say an event');
  expect(`the topic the text hears to be what is offered first, got ${await saying.getByLabel('Topic').inputValue()}`, (await saying.getByLabel('Topic').inputValue()) === 'release.asked');
  await saying.getByLabel(/^What it carries/).fill('{ "version": 2 }');
  await asked(() => saying.getByRole('button', { name: 'Say it', exact: true }).click());
  now = await says();
  daemons = await handed();
  expect(`the handler that hears it to have run, under what it is a run of, got ${now.runs.map((one) => one.heading).join(' | ')}`, now.runs.some((one) => one.heading === 'release: when asked' && one.lines.includes('heard release.asked')));
  expect(`what the handler kept to be what the event carried, got ${JSON.stringify(daemons.instances.at(-1).state)}`, daemons.instances.at(-1).state.asked === 2);
  expect(`the handler's tile to be lit as having heard, got ${JSON.stringify(now.lit)}`, now.lit.some((tile) => tile.id === 'on.asked/@' && tile.how === 'ok'));
  const releases = now.runs.find((one) => one.heading === 'release: its steps');
  expect(`each step to say how it came out, got ${JSON.stringify(releases?.steps)}`, releases.steps.map((step) => `${step.key}:${step.word}`).join(',') === 'build:stood in,pause:ran,ask:stood in,note:ran');
  expect(`a step to be said by what it says of itself, got ${releases.steps[0].says}`, releases.steps[0].says === 'Build it.');
  expect(`what the stand-in was asked and what it answered to be said under its step, got ${releases.steps[2].gave}`, releases.steps[2].gave.includes('agent.task was stood in') && releases.steps[2].gave.includes('what was answered here'));
  expect(`the tiles of steps a stand-in answered for to be told from those that ran, got ${JSON.stringify(now.lit)}`, now.lit.filter((tile) => tile.how === 'stood' && tile.mark === 'S').map((tile) => tile.id).sort().join(',') === 'main/ask,main/build' && now.lit.some((tile) => tile.id === 'main/note' && tile.how === 'ok'));
  expect(`what the text kept of the answer to be what was written, got ${JSON.stringify(daemons.instances[0].state)}`, daemons.instances.at(-1).state.way === 'left');
  expect(`the foot to say what was stood in, in the daemon's words, got ${now.foot.stood}`, now.foot.stood.startsWith(daemons.stood_said) && daemons.stood_said.startsWith('Stood in, and so not exercised: `agent.task`, `shell`. Asked 2 times in all'));
  expect('each action that was stood in to be counted at the foot', (await view().locator('.sandbox-stood').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').trim()).join('|') === 'agent.task × 1|shell × 1');
  expect('the command not to have been run: what it would have written is not there', !existsSync(would));
  expect(`the acts to be listed in order, got ${now.done.join(' | ')}`, now.done.join('|') === 'Answered the 1st stand-in as usual|Moved the clock on 10m|Answered the 2nd stand-in|Said release.asked');
  expect(`what differs from the run before to be said, got ${now.differs.join(' | ')}`, now.differs.some((line) => line.startsWith('`build` of release was failed the time before, and is stood in now')));
  for (const width of [1280, 1000, 760]) {
    await page.setViewportSize({ width, height: 860 });
    await laidOut(`for a project's own text at ${width} wide`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await shot(`sandboxed-4-own-text-${scheme}`);
  await view().locator('.sandbox-foot').scrollIntoViewIfNeeded();
  await view().locator('.sandbox-happened').evaluate((part) => part.scrollTo(0, part.scrollHeight));
  await laidOut('scrolled to what it ended with');
  await shot(`sandboxed-5-own-text-ended-${scheme}`);

  // ---- the text changed on the page, not saved, and run again -----------------------
  await head().getByRole('tab', { name: 'Text' }).click();
  await page.locator('.cm-content').locator('visible=true').click();
  await page.keyboard.press('Control+f');
  await page.locator('.cm-search, .finder').locator('visible=true').first().waitFor();
  await page.keyboard.type('note:');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Escape');
  // The match is chosen still: what is typed takes its place.
  await page.keyboard.type('noted:');
  await head().getByText('unsaved').waitFor();
  // The page has read the text as it is now when its graph has the step by its new name.
  await head().getByRole('tab', { name: 'Node' }).click();
  await page.locator('.gnode[data-node="main/noted"]').locator('visible=true').waitFor();
  await asked(() => sandboxTest().click());
  now = await says();
  expect(`the same acts to have been played against the text as it is now, got ${now.how}: ${now.outcome}`, now.how === 'good' && now.done.length === 4 && now.runs.some((one) => one.heading === 'release: when asked'));
  expect(`what differs to be said, got ${now.differs.join(' | ')}`, now.differs.includes('`noted` of release ran this time (ok), and did not the time before.') && now.differs.includes('`note` of release ran the time before, and did not this time.'));
  expect('the text to be unsaved still: what was run is what is written', (await head().getByText('unsaved').count()) === 1);
  expect(`the graph that is lit to be the text's as it is now, got ${JSON.stringify(now.lit)}`, now.lit.some((tile) => tile.id === 'main/noted' && tile.how === 'ok') && !now.lit.some((tile) => tile.id === 'main/note'));
  await laidOut('for a text run again');
  await shot(`sandboxed-6-run-again-${scheme}`);

  // ---- a text that does not check is not run, and says so -------------------------
  const before = await sandboxId();
  await head().getByRole('tab', { name: 'Text' }).click();
  await page.locator('.cm-content').locator('visible=true').click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.type('x');
  await page.locator('.problems .problem .error-text').locator('visible=true').first().waitFor();
  await sandboxTest().click();
  await view().locator('.comparing-wrong', { hasText: 'This text has problems, so it is not run' }).waitFor();
  expect('a text that does not check not to be run: the sandbox there was is as it was', (await sandboxId()) === before && (await asks('GET', `/api/sandboxes/${before}`)).status === 200);
  expect('nothing to press that would run it', await view().getByRole('button', { name: 'Run again', exact: true }).isDisabled());
  await head().getByRole('tab', { name: 'Text' }).click();
  await page.locator('.cm-content').locator('visible=true').click();
  // Taken out again, where it was put.
  await page.keyboard.press('Control+Home');
  await page.keyboard.press('Delete');
  await head().getByText('checks').waitFor();

  // ---- the daemon has the sandbox no longer: the page says so, and runs it again ----
  await head().getByRole('tab', { name: 'Sandbox' }).click();
  await view().waitFor();
  expect('the daemon to end the sandbox when it is asked to', (await asks('DELETE', `/api/sandboxes/${before}`)).status === 200);
  await asked(() => form('Move the clock on').getByRole('button', { name: 'Move it on', exact: true }).click());
  now = await says();
  expect(`the page to say that the daemon has the sandbox no longer, got ${now.notes.join(' | ')}`, now.notes.some((note) => note.startsWith('The daemon has this sandbox no longer') && note.includes('There is no sandbox test')));
  expect('what it had come to to be shown still', now.runs.length > 0 && (await sandboxId()) === '');
  await laidOut('for a sandbox the daemon has no longer');
  await shot(`sandboxed-7-gone-${scheme}`);
  await asked(() => view().getByRole('button', { name: 'Run it again', exact: true }).click());
  now = await says();
  const again = await sandboxId();
  expect(`it to be run again, with the acts it had, got ${now.how}: ${now.outcome} with ${now.done.length} acts`, now.how === 'good' && again !== '' && again !== before && now.notes.length === 0 && now.done.length === 4);

  // ---- the other of dark and light ---------------------------------------------------
  const other = scheme === 'dark' ? 'light' : 'dark';
  const ground = () => page.evaluate(() => getComputedStyle([...document.querySelectorAll('.sandbox')].find((one) => one.getClientRects().length > 0).closest('.view')).backgroundColor);
  const was = await ground();
  await litAs(page, other);
  await drawn();
  expect(`the page to be drawn in the other of dark and light, and its ground is ${await ground()} as it was`, (await ground()) !== was);
  await laidOut('in the other of dark and light');
  await shot(`sandboxed-8-${other}`);
  await litAs(page, scheme);

  // ---- the fleet that ships: what goes first is what the daemon says it needs ---------
  await page.evaluate(() => (location.hash = '/blueprint/std/fleet'));
  await head().getByText('std/fleet').first().waitFor();
  await head().getByText('checks').waitFor();
  await asked(() => sandboxTest().click());
  now = await says();
  expect(`the board to have been offered to go first, and applied, got ${await view().locator('.sandbox-first li code').allInnerTexts()}`, (await view().locator('.sandbox-first li code').allInnerTexts()).join(',') === 'std/board');
  // Which project it is for is not asked: a sandbox has one, and the text is told that one.
  expect('no box for which project the fleet is for', (await view().locator('.sandbox-acts .comparing-label', { hasText: /^project/ }).count()) === 0);
  // Its first step asks the device whether it has room, which is stood in and waits: from here on every one answers as usual.
  expect(`the fleet to wait for its first stand-in, got ${now.how}: ${now.waits.join(' | ')}`, now.how === 'waits' && now.waits.length === 1 && now.waits[0].includes('system.check'));
  await asked(() => view().getByRole('button', { name: 'Answer every one as usual from here on', exact: true }).click());
  now = await says();
  expect(`the fleet to be up over its board, got ${now.how}: ${now.outcome}`, now.how === 'good' && now.outcome.includes('it is up') && now.tables.includes('board 0 rows') && now.tables.includes('agents 3 rows'));
  expect(`the fleet's agents to have been stood in, got ${now.foot.stood}`, now.foot.stood.includes('`agent.ensure`'));
  // A card made ready: nobody is there to take it, since an agent is stood in, and whoever applied the fleet is told.
  const ready = form('Do something by hand');
  await ready.locator('select').first().selectOption({ label: 'Add a card' });
  await ready.getByLabel(/^title/).fill('Try it');
  await ready.getByLabel(/^status/).fill('ready');
  await asked(() => ready.getByRole('button', { name: 'Do it', exact: true }).click());
  now = await says();
  expect(`nothing to wait, and what was done to be listed, got ${now.waits.length} waiting and ${now.done.join(' | ')}`, now.waits.length === 0 && now.done.join('|') === 'Had every stand-in answer as usual from here on|Did board.add');
  expect(`whoever applied the fleet to have been told of the card left, got ${now.talk.join(' | ')}`, now.talk.some((line) => line.startsWith('whoever applied it · left') && line.includes('left for a person')));
  await laidOut('for the fleet with a card left');
  await view().locator('.sandbox-happened').evaluate((part) => part.scrollTo(0, part.scrollHeight));
  await shot(`sandboxed-12-fleet-told-${scheme}`);

  // ---- the tab closed: the daemon holds no sandbox of it ----------------------------
  expect('the daemon to hold the board\'s sandbox while its tab is open', (await asks('GET', `/api/sandboxes/${boardsSandbox}`)).status === 200);
  await page.getByRole('button', { name: 'Close std/board', exact: true }).click();
  await page.waitForFunction(async (id) => (await fetch(`/api/sandboxes/${id}`, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).status === 404, boardsSandbox);

  // ---- nothing of any of it is the factory's ------------------------------------------
  const instances = (await asks('GET', '/api/instances')).body;
  expect(`nothing to be running in the factory, got ${JSON.stringify(instances).slice(0, 200)}`, (instances.instances ?? instances).length === 0);
  const projects = (await asks('GET', '/api/projects')).body;
  expect('the factory to have no project called sandbox', !(projects.projects ?? projects).some((one) => one.id === 'sandbox'));
  expect('nobody in the factory to have been told anything', (await asks('GET', '/api/inbox?all=true')).body.lines.length === 0);
  const rail = async (name) => {
    const button = page.getByRole('button', { name, exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  };
  await rail('Resources');
  await unfoldAll(page);
  expect('nothing of a sandbox to be listed in the side panel', !(await page.locator('.nav').innerText()).toLowerCase().includes('sandbox'));
  await page.locator('.nav').getByText('Activity', { exact: true }).click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'Activity' }).first().waitFor();
  await drawn();
  const activity = await page.locator('.view').locator('visible=true').last().innerText();
  expect(`nothing of a sandbox to be on Activity, got ${activity.slice(0, 200)}`, !/sandbox|release|std\/board|std\/fleet/i.test(activity));
  await shot(`sandboxed-9-activity-${scheme}`);

  // ---- a phone ----------------------------------------------------------------------
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: scheme });
  small = await signIn(phone, { at: '#/blueprint/std/board', touch: true });
  await head(small).getByText('checks').waitFor();
  await asked(() => sandboxTest(small).tap(), small);
  now = await says(small);
  expect(`the board to be up in its sandbox on a phone, got ${now.how}: ${now.outcome}`, now.how === 'good' && now.foot.said.startsWith('This was a sandbox test'));
  await laidOut("at a phone's width", small);
  await shot(`sandboxed-10-phone-${scheme}`, small);
  await view(small).locator('.sandbox-outcome').scrollIntoViewIfNeeded();
  await laidOut("at a phone's width, scrolled to what happened", small);
  await shot(`sandboxed-11-phone-happened-${scheme}`, small);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/sandboxed-0-failed.png` }).catch(() => {});
  await small?.screenshot({ path: `${out}/sandboxed-0-failed-phone.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop, would]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
