// A blueprint previewed: the drawer that plays it on its graph as each
// part of its Summary is read.
//
//   DAEMON=../target/debug/factory node scripts/previewed.mjs [out-dir]
//
// It goes by what the page shows, against the daemon's own graph of the
// same text, and by the boxes:
//
//   - the board that ships, and the longest blueprint that ships (the
//     fleet), each previewed from Preview in its head: the drawer comes
//     in with the graph over a column of the Summary's parts, each in the
//     daemon's words, and every part scrolled to the top of the column in
//     turn; at each, every frame it plays is read as it is played (which
//     tiles are lit, and how, and which of its lines is lit), and held to
//     what that part is about: what it does when it is applied lights
//     only its own steps, each only once what it waits on (the daemon's
//     edges) is done or under way, and all of them; what it hears has
//     each handler's event arrive at its tile before its steps are done,
//     its condition said as it is written where it has one, and lights
//     nothing but that handler's; what it does as it ends lights what it
//     was doing as ended; any other part lights only the tiles its lines
//     are about; and a lit line is about a tile that is lit; each ending
//     played as the daemon says it is (its steps done, the last failed,
//     ended however they went, its new text taken, everything ended as it
//     is stopped); a step's own condition asked before it is done;
//   - a project's own blueprint with a diamond of waits, a handler with a
//     condition and something done as it is stopped, previewed beside its
//     text: the diamond's two lit together, a step's own condition asked
//     before it is done, its steps failing and it being stopped each
//     played as what it is, the condition said; the text
//     changed and not saved, the preview following it (what it says it is,
//     and the condition as it is now written) with the drawer still there;
//     made not to check, the preview saying so in the check's own words
//     and Preview closed saying why; put right, previewed again;
//   - each part played once, from its first frame, as it first comes to
//     be read, and held at its last: read again, it is as it was left,
//     played round by nothing; Play playing it again from the first, once;
//     Pause holding the frame it is at; Next and Previous, and the keys
//     (`.`, `,`, the arrows and Space) where the drawer has the keyboard,
//     each by what is lit, as the frame it comes to was lit as it played;
//     the track under the graph pressed and dragged to a frame; and a line
//     of a part pressed putting the playhead at its first frame, paused,
//     the line marked as now and its tile lit;
//   - put away by the drawer's own button and by Escape, Preview, under
//     Testing, saying it shows while it does and not once it is put away;
//   - where less movement is asked for: each part lit, still, with nothing
//     played and nothing moving in the drawer, and Next stepping all the
//     same, to a frame lit as it was played;
//   - nothing of the preview over anything else of it, outside the drawer
//     or scrolling sideways, and nothing said under its graph, nor what
//     plays it, over a tile or a group's name (`legended.mjs`), at three
//     widths and a phone's,
//     for the board and for the fleet;
//   - and the other of dark and light.
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest, DRAWER_AT_REST } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { legendCrossed } from './legended.mjs';
import { closeTesting, pressTesting, testingLine, testingNote } from './testing.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'previewed-data');
const home = join(out, 'previewed-home');
const shop = join(out, 'previewed-shop');

// A project's own blueprint with something for each kind of part: steps
// in a diamond of waits, the last of them with a condition of its own, a
// handler with a condition, and something done as its steps fail and as
// it is stopped.
const OWN = `blueprint: watch
version: 1
description: Keep a table of what was seen, and say so in the log.

steps:
  seen:
    do: collection.ensure
    with: { name: seen, fields: { title: { type: text } } }
  first:
    needs: [seen]
    do: log
    with: { message: first }
  second:
    needs: [seen]
    do: log
    with: { message: second }
  both:
    needs: [first, second]
    when: \${ steps.first.ok }
    do: log
    with: { message: both }

on:
  added:
    event: record.created
    when: \${ event.data.collection == "seen" }
    do: log
    with: { message: added }

on_fail:
  - do: log
    with: { message: failed }

on_stop:
  - do: log
    with: { message: stopped }
`;

await mkdir(out, { recursive: true });
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
  const other = scheme === 'dark' ? 'light' : 'dark';
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async (context) => {
    const opened = await context.newPage();
    opened.setDefaultTimeout(20_000);
    opened.on('pageerror', (err) => problems.push(err.message));
    await opened.goto(`${url}/`);
    await opened.getByLabel('Email').fill('ada@example.com');
    await opened.getByLabel('Password').fill('correct horse');
    await opened.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await opened.locator('.rail').waitFor();
    return opened;
  };
  page = await signIn(await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme }));

  /** Asks the daemon something as the page does. */
  const ask = (path, body, on = page) =>
    on.evaluate(async ([to, sent]) => (await fetch(to, { method: 'POST', headers: { 'content-type': 'application/json', 'x-page-key': localStorage.getItem('page-key') ?? '' }, body: JSON.stringify(sent) })).json(), [path, body]);
  const ending = await page.evaluate(async () => (await (await fetch('/api/meta')).json()).blueprint_ext);
  const laid = (await ask('/api/blueprints/format', { source: OWN, project: 'shop' })).formatted;
  expect('the daemon to lay the blueprint out', typeof laid === 'string' && laid.includes('record.created'));
  await mkdir(join(shop, 'blueprints'), { recursive: true });
  await writeFile(join(shop, `blueprints/watch.${ending}`), laid);

  // ---- what the check reads of the page ------------------------------------------
  const drawn = (on = page) => on.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const shot = (name, on = page) => on.screenshot({ path: `${out}/${name}.png` });
  const head = (on = page) => on.locator('.view-head').last();
  const preview = (name, on = page) => drawerAtRest(on).locator(`.preview[data-of="${name}"]`);
  /** The daemon's graph of a text, as the page is handed it. */
  const graphOf = async (source, on = page) => (await ask('/api/blueprints/check', { source, project: 'shop' }, on)).graph;
  const openAt = async (name, on = page) => {
    await on.evaluate((to) => (location.hash = to), `/blueprint/${name}`);
    await head(on).getByText(name, { exact: false }).first().waitFor();
  };
  const opened = async (name, on = page) => {
    await pressTesting(on, head(on), 'Preview');
    await preview(name, on).waitFor();
    await preview(name, on).locator('.preview-part').first().waitFor();
  };

  /**
   * Keeps, in the page, every frame the preview shows as it is shown: which part is read, which frame it is,
   * which tiles are lit and how, which line is lit and what is said over the graph.
   */
  const recording = (on = page) =>
    on.evaluate(() => {
      window.unrecorded?.();
      window.played = [];
      const take = () => {
        const shown = document.querySelector('.drawer .preview');
        if (!shown) return;
        const tiles = {};
        for (const tile of shown.querySelectorAll('.preview-graph [data-node]')) {
          const how = [...tile.classList].find((name) => name.startsWith('status-pv-'));
          if (how) tiles[tile.dataset.node] = how.slice('status-pv-'.length);
        }
        const lit = shown.querySelector('.preview-part[aria-current] .preview-line[data-lit]');
        // Which ending the lit line is of, where it is one or a thing done under one.
        const ending = lit?.closest('.preview-line[data-ending]')?.dataset.ending ?? null;
        const said = { read: shown.dataset.read, frame: shown.dataset.frame, frames: Number(shown.dataset.frames), tiles, line: lit ? (lit.dataset.at ?? '') : null, ending, says: shown.querySelector('.preview-says code')?.textContent ?? null };
        const last = window.played.at(-1);
        if (!last || JSON.stringify(last) !== JSON.stringify(said)) window.played.push(said);
      };
      const seen = new MutationObserver(take);
      seen.observe(document.querySelector('.main'), { attributes: true, subtree: true, childList: true, attributeFilter: ['data-frame', 'data-read', 'class', 'data-lit'] });
      take();
      window.unrecorded = () => seen.disconnect();
    });
  /** Brings a part to the top of the column, as somebody scrolls to it, and waits until it is the one read. */
  const readPart = async (kind, on = page) => {
    await on.evaluate((which) => document.querySelector(`.drawer .preview-part[data-part="${which}"]`).scrollIntoView({ block: 'start' }), kind);
    await on.locator(`.drawer .preview[data-read="${kind}"]`).waitFor();
  };
  /** What the preview shows now: which frame, whether it plays, and which tiles are lit and how. */
  const shownNow = (on = page) =>
    on.evaluate(() => {
      const shown = document.querySelector('.drawer .preview');
      const tiles = {};
      for (const tile of shown.querySelectorAll('.preview-graph [data-node]')) {
        const how = [...tile.classList].find((name) => name.startsWith('status-pv-'));
        if (how) tiles[tile.dataset.node] = how.slice('status-pv-'.length);
      }
      return { frame: shown.dataset.frame, playing: shown.dataset.playing, tiles };
    });
  /** What plays the preview, under its graph. */
  const control = (name, on = page) => on.locator('.drawer .canvas-player').getByRole('button', { name, exact: true });
  /** Longer than any frame is shown for: what was to come by itself has come by then. */
  const longer = () => new Promise((later) => setTimeout(later, 1800));
  /** Holds what is shown now to be frame `at` of what was played, lit as it was lit then, and paused. */
  const showsFrame = async (at, played, when, on = page) => {
    await on.locator(`.drawer .preview[data-frame="${at}"]`).waitFor();
    const now = await shownNow(on);
    expect(`frame ${at} to be shown paused ${when}, got ${now.frame}, playing ${now.playing}`, now.frame === String(at) && now.playing === 'false');
    expect(`frame ${at} lit as it was played ${when}, got ${JSON.stringify(now.tiles)} for ${JSON.stringify(played[at].tiles)}`, JSON.stringify(now.tiles) === JSON.stringify(played[at].tiles));
  };
  /** Every frame a part plays, each as it was shown, once the page has shown each of them. */
  const framesOf = async (kind, on = page) => {
    const count = Number(await on.locator('.drawer .preview').getAttribute('data-frames'));
    if (count === 0) return [];
    await on.waitForFunction(
      ([which, many]) => new Set(window.played.filter((one) => one.read === which && one.frame !== '').map((one) => one.frame)).size >= many,
      [kind, count],
      { timeout: 90_000 },
    );
    const all = await on.evaluate((which) => window.played.filter((one) => one.read === which && one.frame !== ''), kind);
    // The first time each frame was shown whole, from the first.
    const first = all.findIndex((one) => one.frame === '0');
    const byFrame = new Map();
    for (const one of all.slice(first)) if (!byFrame.has(one.frame)) byFrame.set(one.frame, one);
    return [...Array(count).keys()].map((at) => byFrame.get(String(at)));
  };

  /** Holds what a part plays to what it is about, by the daemon's own graph of the same text. */
  const heldToTheGraph = (kind, frames, graph, about, when) => {
    const nodes = new Map(graph.nodes.map((one) => [one.id, one]));
    const lit = (frame) => Object.entries(frame.tiles);
    for (const [at, frame] of frames.entries())
      expect(`frame ${at} of ${kind} ${when} to light only tiles of the graph, got ${lit(frame).map(([id]) => id).join(', ')}`, lit(frame).every(([id]) => nodes.has(id)));
    // A lit line is about a tile that is lit, where it is about one.
    for (const [at, frame] of frames.entries())
      if (frame.line) expect(`the line lit in frame ${at} of ${kind} ${when} (${frame.line}) to be about a tile lit in it`, frame.tiles[frame.line] !== undefined);
    const ever = new Set(frames.flatMap((frame) => Object.keys(frame.tiles)));
    if (kind === 'does') {
      const steps = graph.nodes.filter((one) => one.group === 'main').map((one) => one.id);
      expect(`what it does to light only its own steps ${when}`, [...ever].every((id) => nodes.get(id).group === 'main'));
      expect(`what it does to light every one of its steps ${when}, missing ${steps.filter((id) => !ever.has(id)).join(', ')}`, steps.every((id) => ever.has(id)));
      // A step that says `when` has it asked, as written, before it is done.
      for (const step of graph.nodes.filter((one) => one.group === 'main' && one.when)) {
        const asked = frames.findIndex((frame) => frame.tiles[step.id] === 'asks');
        const done = frames.findIndex((frame) => frame.tiles[step.id] === 'runs');
        expect(`${step.id}, which says when it is done, to have that asked as written before it is done ${when}`, asked >= 0 && asked < done && frames[asked].says.includes(step.when));
      }
      // Each step lit as being done only once everything it waits on is done or under way, as the daemon's edges say.
      for (const [at, frame] of frames.entries())
        for (const [id, how] of lit(frame))
          if (how === 'runs')
            for (const edge of graph.edges.filter((one) => one.to === id && one.kind !== 'fires'))
              expect(`${id} to be done only once ${edge.from} is, in frame ${at} ${when}, which has it ${frame.tiles[edge.from] ?? 'unlit'}`, ['done', 'runs'].includes(frame.tiles[edge.from]) && (edge.kind === 'starts' || frame.tiles[edge.from] === 'done' || nodes.get(edge.from).kind === 'task'));
      expect(`the last frame of what it does to have every step done ${when}`, steps.every((id) => frames.at(-1).tiles[id] === 'done'));
    } else if (kind === 'hears') {
      const triggers = about.filter((id) => id.endsWith('/@'));
      expect(`what it hears to be about its handlers ${when}`, triggers.length > 0);
      for (const trigger of triggers) {
        const group = trigger.slice(0, -2);
        const arrives = frames.findIndex((frame) => frame.tiles[trigger] === 'arrives');
        const runs = frames.findIndex((frame) => lit(frame).some(([id, how]) => nodes.get(id).group === group && how === 'runs'));
        expect(`the event of ${group} to arrive at its tile ${when}`, arrives >= 0);
        expect(`the event of ${group} to arrive before its steps are done ${when}`, runs === -1 || arrives < runs);
        const asks = nodes.get(trigger).when;
        if (asks) expect(`the condition of ${group} to be said as written ${when}`, frames.some((frame) => frame.tiles[trigger] === 'asks' && frame.says === asks));
        else expect(`${group}, which has no condition, to ask nothing ${when}`, !frames.some((frame) => frame.tiles[trigger] === 'asks'));
      }
      // Each frame lights one handler's tiles and no other's.
      for (const [at, frame] of frames.entries()) expect(`frame ${at} of what it hears to light one handler ${when}`, new Set(lit(frame).map(([id]) => nodes.get(id).group)).size <= 1);
    } else if (kind === 'ends') {
      // Each ending is played as what it is, by the daemon's word for which it is.
      const main = graph.nodes.filter((one) => one.group === 'main').map((one) => one.id);
      const all = graph.nodes.map((one) => one.id);
      const just = (frame, ids, how) => lit(frame).length === ids.length && ids.every((id) => frame.tiles[id] === how);
      expect(`every frame of what it does as it ends to be of an ending ${when}`, frames.length > 0 && frames.every((frame) => frame.ending));
      for (const [at, frame] of frames.entries()) {
        const as = frame.ending;
        const ok =
          as === 'success' ? just(frame, main, 'done')
          : as === 'finish' ? just(frame, main, 'ended')
          : as === 'update' ? just(frame, all, 'taken')
          : as === 'stop' ? just(frame, all, 'ended')
          : as === 'fail' ? lit(frame).filter(([, how]) => how === 'failed').length >= 1 && lit(frame).every(([id, how]) => main.includes(id) && ['failed', 'done'].includes(how))
          : false;
        expect(`frame ${at} of what it does as it ends to play ${as} as what it is ${when}, got ${JSON.stringify(frame.tiles)}`, ok);
      }
    } else if (kind === 'is') {
      expect(`what it is to light the whole of it ${when}`, frames.length === 1 && graph.nodes.every((one) => frames[0].tiles[one.id] === 'shown'));
    } else {
      expect(`${kind} to light only what its lines are about ${when}, got ${[...ever].filter((id) => !about.includes(id)).join(', ')}`, [...ever].every((id) => about.includes(id)));
    }
  };
  /** What a part's lines are about, as the page says each line is. */
  const aboutOf = (kind, on = page) => on.locator(`.drawer .preview-part[data-part="${kind}"] .preview-line[data-at]`).evaluateAll((all) => all.map((one) => one.dataset.at));
  /** Reads every part in turn, holding each to the graph; gives what was played of each. */
  const everyPart = async (graph, when, on = page) => {
    await recording(on);
    const kinds = await on.locator('.drawer .preview-part').evaluateAll((all) => all.map((one) => one.dataset.part));
    const played = {};
    for (const kind of kinds) {
      await readPart(kind, on);
      const frames = await framesOf(kind, on);
      expect(`every frame of ${kind} to be seen ${when}`, frames.every(Boolean));
      heldToTheGraph(kind, frames, graph, await aboutOf(kind, on), when);
      played[kind] = frames;
    }
    return { kinds, played };
  };

  /** The boxes of the preview in the drawer: what lies over what, what is outside the drawer, what scrolls sideways. */
  const looked = (on = page) =>
    on.evaluate(() => {
      const drawer = document.querySelector('.drawer[data-rest]');
      const box = (el) => {
        const at = el.getBoundingClientRect();
        return { left: at.left, right: at.right, top: at.top, bottom: at.bottom, what: (el.textContent ?? '').trim().slice(0, 36) || el.className };
      };
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const edge = box(drawer);
      const column = box(drawer.querySelector('.preview-parts'));
      // What is in the column is seen only as far as the column shows it.
      const seen = (one) => ({ ...one, top: Math.max(one.top, column.top), bottom: Math.min(one.bottom, column.bottom) });
      const fixed = [...drawer.querySelectorAll('.drawer-head, .preview-graph, .preview-says, .preview-legend li')].map(box);
      const scrolled = [...drawer.querySelectorAll('.preview-part h3, .preview-words, .preview-nothing')].map(box).map(seen).filter((one) => one.bottom - one.top > 0.5);
      const rows = [...fixed, ...scrolled];
      const crossed = [];
      rows.forEach((a, at) => rows.slice(at + 1).forEach((b) => over(a, b) && crossed.push(`"${a.what}" over "${b.what}"`)));
      const scrollers = [...drawer.querySelectorAll('.preview, .preview-parts, .preview-now, .drawer-body')];
      return {
        rows: rows.length,
        crossed,
        outside: rows.filter((one) => one.left < edge.left - 0.5 || one.right > edge.right + 0.5).map((one) => one.what),
        sideways: [document.documentElement.scrollWidth - window.innerWidth, ...scrollers.map((one) => one.scrollWidth - one.clientWidth)],
        beyond: edge.right > window.innerWidth + 0.5 || edge.left < -0.5,
      };
    });
  const laidOut = async (when, on = page) => {
    await drawn(on);
    const now = await looked(on);
    expect(`the preview to have lines ${when}`, now.rows > 4);
    expect(`nothing of the preview to lie over anything else of it ${when}, got ${now.crossed.slice(0, 4).join('; ')}`, now.crossed.length === 0);
    expect(`nothing of the preview to be outside the drawer ${when}, got ${now.outside.slice(0, 4).join('; ')}`, now.outside.length === 0 && !now.beyond);
    expect(`nothing to scroll sideways ${when}, got ${now.sideways.join(', ')}`, now.sideways.every((one) => one <= 0));
    const band = await legendCrossed(on, '.drawer');
    expect(`what is said under the graph to lie over no tile and no group's name ${when}, got ${band.slice(0, 4).join('; ')}`, band.length === 0);
  };
  /** The boxes at three widths and a phone's, each part brought to the top in turn. */
  const atEveryWidth = async (name, on, label) => {
    for (const [width, height] of [[1280, 860], [900, 860], [600, 860], [390, 844]]) {
      await on.setViewportSize({ width, height });
      await drawerAtRest(on).waitFor();
      const kinds = await on.locator('.drawer .preview-part').evaluateAll((all) => all.map((one) => one.dataset.part));
      for (const kind of kinds) {
        await readPart(kind, on);
        await laidOut(`for ${name}, reading ${kind}, at ${width} wide`, on);
      }
      await shot(`previewed-${label}-${width}-${scheme}`, on);
    }
    await on.setViewportSize({ width: 1280, height: 860 });
  };

  // ---- the board that ships ---------------------------------------------------------
  await openAt('std/board');
  await head().getByText('checks', { exact: true }).waitFor();
  await opened('std/board');
  expect('the preview to be headed by what it is of', (await drawerAtRest(page).locator('.drawer-head h2').innerText()).trim() === 'Preview of std/board');
  expect('Preview, under Testing, to say that it is showing while its preview is', (await testingNote(page, head(), 'Preview')) === 'Showing beside the page: pressed, it is put away.');
  await closeTesting(page);
  expect('Escape to put the menu away and not the preview beside it', (await page.locator(DRAWER_AT_REST).count()) === 1);
  const boardText = await readFile(join(repo, 'blueprints/std/board.bp'), 'utf8');
  const boardGraph = await graphOf(boardText);
  // Every part of the daemon's own summary is a section, in its words.
  const boardSummary = (await ask('/api/blueprints/summary', { name: 'std/board' })).summary;
  const sectionTitles = await page.locator('.drawer .preview-part h3').allTextContents();
  expect(`a section for each part of the daemon's summary, got ${sectionTitles.join(' | ')}`, sectionTitles.length === boardSummary.parts.length && boardSummary.parts.every((part) => sectionTitles.includes(part.title)));
  const words = (await page.locator('.drawer .preview-words').allTextContents()).map((one) => one.trim());
  const said = boardSummary.parts.flatMap((part) => {
    const under = (lines) => lines.flatMap((line) => [line.text.replaceAll('`', ''), ...under(line.under)]);
    return under(part.lines);
  });
  expect(`every line the daemon says to be said in the preview, and no other, got ${words.length} for ${said.length}`, words.length === said.length && said.every((one, at) => words[at] === one || words.includes(one)));
  const board = await everyPart(boardGraph, 'for the board that ships');
  expect(`the board's parts, got ${board.kinds.join(', ')}`, board.kinds.includes('does'));
  // Each part was played once as it came to be read, and is held at its last frame: read again, it is as it was left,
  // and nothing plays it round again.
  const does = board.played.does;
  await readPart('does');
  await showsFrame(does.length - 1, does, 'as what it does is read again');
  await longer();
  await showsFrame(does.length - 1, does, 'a while after, played round by nothing');
  // Play plays it again from the first, once; part of the way through, Pause holds the frame it is at.
  await control('Play').click();
  await page.locator('.drawer .preview[data-frame="1"][data-playing="true"]').waitFor({ timeout: 30_000 });
  await shot(`previewed-1-board-mid-play-${scheme}`);
  await control('Pause').click();
  const paused = Number((await shownNow()).frame);
  await showsFrame(paused, does, 'paused');
  await longer();
  await showsFrame(paused, does, 'a while after it was paused');
  // Next and Previous, each to the frame beside it, paused, and lit as it was played; at either end, no further.
  await control('Previous frame').click();
  await showsFrame(Math.max(0, paused - 1), does, 'by Previous');
  await control('Next frame').click();
  await showsFrame(paused, does, 'by Next');
  // The track under the graph: a frame pressed, and the playhead dragged along it to the last.
  const track = page.locator('.drawer .player-track');
  const box = await track.boundingBox();
  const atFrame = (at) => box.x + ((at + 0.5) / does.length) * box.width;
  await page.mouse.click(atFrame(2), box.y + box.height / 2);
  await showsFrame(2, does, 'by the track pressed');
  await page.mouse.move(atFrame(0), box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(atFrame(1), box.y + box.height / 2, { steps: 3 });
  await page.mouse.move(atFrame(does.length - 1), box.y + box.height / 2, { steps: 6 });
  await page.mouse.up();
  await showsFrame(does.length - 1, does, 'by the track dragged to its end');
  await control('Next frame').click();
  await showsFrame(does.length - 1, does, 'by Next at the last, which goes no further');
  expect('the track to say which frame of how many', (await track.getAttribute('aria-valuetext')) === `frame ${does.length} of ${does.length}`);
  await shot(`previewed-1-board-held-${scheme}`);
  // The keys, where the drawer has the keyboard: `,` and the left arrow back, `.` and the right arrow on, Space plays and pauses.
  await page.locator('.drawer .preview-now').click();
  await page.keyboard.press(',');
  await showsFrame(does.length - 2, does, 'by ,');
  await page.keyboard.press('ArrowLeft');
  await showsFrame(does.length - 3, does, 'by the left arrow');
  await page.keyboard.press('.');
  await showsFrame(does.length - 2, does, 'by .');
  await page.keyboard.press('ArrowRight');
  await showsFrame(does.length - 1, does, 'by the right arrow');
  await page.keyboard.press(' ');
  await page.locator('.drawer .preview[data-playing="true"]').waitFor();
  await page.keyboard.press(' ');
  await page.locator('.drawer .preview[data-playing="false"]').waitFor();
  // A line of the part pressed: the playhead at the first frame it is of, paused, the line marked as now and its tile lit.
  // A line with none under it, about a tile, that some frame after the first is of: the first frame it is of is where it goes.
  const leaves = await page.locator('.drawer .preview-part[data-part="does"] .preview-line[data-at]:not(:has(.preview-line)) > button.preview-words').evaluateAll((all) => all.map((one) => one.parentElement.dataset.at));
  const pressedAt = leaves.find((at) => does.findIndex((frame) => frame.line === at) > 0);
  expect(`a line of what it does that a frame after the first is of, got ${leaves.join(', ')}`, pressedAt !== undefined);
  const lineAt = does.findIndex((frame) => frame.line === pressedAt);
  const pressedLine = page.locator(`.drawer .preview-part[data-part="does"] .preview-line[data-at="${pressedAt}"] > button.preview-words`);
  await pressedLine.click();
  await showsFrame(lineAt, does, `by the line about ${pressedAt} pressed`);
  expect(`the line pressed to be marked as now`, (await pressedLine.getAttribute('aria-current')) === 'true');
  expect(`the tile the line is about to be lit`, (await shownNow()).tiles[pressedAt] !== undefined);
  // And reached by the keyboard: a line is a button, Enter on it the same as a press.
  await control('Next frame').click();
  await pressedLine.focus();
  await page.keyboard.press('Enter');
  await showsFrame(lineAt, does, `by Enter on the line about ${pressedAt}`);
  await shot(`previewed-1-board-line-pressed-${scheme}`);
  await atEveryWidth('the board', page, '2-board');

  // Put away by the drawer's own button and by Escape.
  await drawerAtRest(page).getByRole('button', { name: 'Close', exact: true }).click();
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await opened('std/board');
  // A dialog, and so the drawer, listens for Escape a moment after it is drawn.
  await drawn();
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  expect('Preview, under Testing, to say what it does once its preview is put away', (await testingNote(page, head(), 'Preview')) === 'Plays what it does on its graph; runs nothing.');
  await closeTesting(page);

  // ---- the longest that ships ------------------------------------------------------
  await openAt('std/fleet');
  await head().getByText('checks', { exact: true }).waitFor();
  await opened('std/fleet');
  const fleetGraph = await graphOf(await readFile(join(repo, 'blueprints/std/fleet.bp'), 'utf8'));
  const fleet = await everyPart(fleetGraph, 'for the fleet that ships');
  expect(`the fleet's parts to include what it hears, got ${fleet.kinds.join(', ')}`, fleet.kinds.includes('hears') && fleet.played.hears.length > 4);
  await readPart('hears');
  // Played once as it was read in turn, and held at its end: Play plays it again.
  await showsFrame(fleet.played.hears.length - 1, fleet.played.hears, 'as what the fleet hears is read again');
  await control('Play').click();
  await page.locator('.drawer .preview[data-frame="2"]').waitFor({ timeout: 30_000 });
  await shot(`previewed-3-fleet-hears-${scheme}`);
  await atEveryWidth('the fleet', page, '4-fleet');
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });

  // ---- a project's own, beside its text, as it is written ----------------------------
  // Opened by its address, as a link to it opens it, on its text for whoever may change it.
  await page.evaluate((to) => (location.hash = to), `/file/shop/blueprints/watch.${ending}`);
  await head().getByText(`watch.${ending}`, { exact: false }).first().waitFor();
  await page.locator('.cm-content').waitFor();
  await head().getByText('checks', { exact: true }).waitFor();
  await opened('watch');
  const ownGraph = await graphOf(laid);
  const own = await everyPart(ownGraph, "for a project's own");
  expect('the two steps that wait on the same one to be lit together', own.played.does.some((frame) => frame.tiles['main/first'] === 'runs' && frame.tiles['main/second'] === 'runs'));
  expect('what it does as it is stopped to be played', (own.played.ends ?? []).length >= 2);
  expect('its steps failing to be played, the step that failed lit as failed before what it then does', own.played.ends.some((frame) => frame.ending === 'fail' && Object.values(frame.tiles).includes('failed')));
  expect('its condition of its own to be asked before the step is done', own.played.does.some((frame) => frame.tiles['main/both'] === 'asks'));
  await shot(`previewed-5-own-beside-its-text-${scheme}`);
  // Its steps failing, as it is played: the step that failed lit as failed.
  await recording();
  await readPart('ends');
  await control('Play').click();
  await page.waitForFunction(() => window.played.some((one) => one.read === 'ends' && one.ending === 'fail' && Object.values(one.tiles).includes('failed')), null, { timeout: 30_000 });
  await shot(`previewed-5-own-failing-${scheme}`);
  // The text changed and not saved: the preview follows it, and stays.
  const lineOf = (text) => laid.split('\n').findIndex((one) => one.startsWith(text));
  const goTo = async (line) => {
    await page.locator('.cm-content').click();
    await page.keyboard.press('Control+Home');
    for (let down = 0; down < line; down += 1) await page.keyboard.press('ArrowDown');
    await page.keyboard.press('End');
  };
  await goTo(lineOf('description: '));
  await page.waitForFunction(() => document.querySelector('.cm-activeLine')?.textContent?.startsWith('description: '));
  await page.keyboard.type(' Twice over.');
  await head().getByText('unsaved').waitFor();
  await preview('watch').locator('.preview-part[data-part="is"] .preview-words', { hasText: 'Twice over.' }).waitFor();
  // The handler's condition, and not the step's.
  const handlerWhen = laid.split('\n').findIndex((one) => one.startsWith('    when:') && one.includes('event.data'));
  const asked = laid.split('\n')[handlerWhen];
  await goTo(handlerWhen);
  await page.waitForFunction((want) => document.querySelector('.cm-activeLine')?.textContent === want, asked);
  for (let back = 0; back < '" }'.length; back += 1) await page.keyboard.press('ArrowLeft');
  await page.keyboard.type('-or-so');
  await head().getByText('checks', { exact: true }).waitFor();
  await recording();
  await readPart('hears');
  await page.waitForFunction(() => window.played.some((one) => one.read === 'hears' && one.says === 'event.data.collection == "seen-or-so"'), null, { timeout: 30_000 });
  expect('the preview to stay beside the text as it is written', (await page.locator(DRAWER_AT_REST).count()) === 1);
  await shot(`previewed-6-own-followed-unsaved-${scheme}`);
  // Made not to check: none, in the check's own words, and Preview closed saying why.
  await page.locator('.cm-content').click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.type('x');
  await preview('watch').waitFor({ state: 'detached' });
  await drawerAtRest(page).getByText('This cannot be previewed until it checks').waitFor();
  const wrong = await page.locator('.problems .problem').evaluateAll((all) => all.filter((one) => one.querySelector('.error-text')).map((one) => one.lastElementChild.textContent));
  const told = await drawerAtRest(page).locator('.summary-wrong li').allInnerTexts();
  expect(`what is wrong to be said in the check's own words, got ${told.join(' | ')} for ${wrong.join(' | ')}`, told.length === wrong.length && wrong.every((one, at) => told[at].endsWith(one.replaceAll('`', ''))));
  const closedLine = await testingLine(page, head(), 'Preview');
  expect('Preview, under Testing, to be closed while the text does not check, saying why', (await closedLine.getAttribute('aria-disabled')) === 'true' && (await closedLine.getAttribute('title')).startsWith('It is previewed once its text checks: '));
  await closeTesting(page);
  await shot(`previewed-7-does-not-check-${scheme}`);
  await page.locator('.cm-content').click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.press('Delete');
  await preview('watch').waitFor();
  expect('Preview to be open again once the text checks', (await (await testingLine(page, head(), 'Preview')).getAttribute('aria-disabled')) === null);
  await closeTesting(page);
  await page.keyboard.press('Escape');
  await drawn();
  // The editor takes the first Escape where something is offered in it; the drawer the next.
  if ((await page.locator('.drawer').count()) > 0) await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });

  // ---- where less movement is asked for ----------------------------------------------
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openAt('std/board');
  await opened('std/board');
  for (const kind of board.kinds) {
    await readPart(kind);
    await drawn();
    const still = await page.evaluate(() => {
      const shown = document.querySelector('.drawer .preview');
      const tiles = [...shown.querySelectorAll('.preview-graph [data-node]')].filter((one) => [...one.classList].some((name) => name.startsWith('status-pv-'))).map((one) => one.dataset.node);
      return { frame: shown.dataset.frame, tiles, moving: document.querySelector('.drawer').getAnimations({ subtree: true }).length };
    });
    const shownOnce = board.played[kind];
    const lit = new Set(shownOnce.flatMap((frame) => Object.keys(frame.tiles)));
    expect(`${kind} to be still where less movement is asked for, got ${still.frame}`, still.frame === 'still');
    expect(`${kind} to show, still, every tile it lights as it plays`, still.tiles.length === lit.size && still.tiles.every((id) => lit.has(id)));
    expect(`nothing in the drawer to move where less movement is asked for, got ${still.moving}`, still.moving === 0);
    await drawn();
    const again = await page.locator('.drawer .preview').getAttribute('data-frame');
    expect(`${kind} to stay still`, again === 'still');
  }
  await readPart('does');
  // Nothing plays by itself, and Next and Previous step all the same: to a frame lit as it was played, and nothing moving.
  await control('Next frame').click();
  await showsFrame(0, board.played.does, 'by Next where less movement is asked for');
  await control('Next frame').click();
  await showsFrame(1, board.played.does, 'by Next again where less movement is asked for');
  await control('Previous frame').click();
  await showsFrame(0, board.played.does, 'by Previous where less movement is asked for');
  // A change is given a hundredth of a millisecond there (`style.css`): looked at once it has been drawn.
  await drawn();
  const moving = await page.evaluate(() => document.querySelector('.drawer').getAnimations({ subtree: true }).map((one) => `${one.animationName ?? one.transitionProperty} on ${one.effect?.target?.getAttribute?.('class') ?? one.effect?.target?.tagName}`));
  expect(`nothing in the drawer to move as it is stepped where less movement is asked for, got ${moving.join('; ')}`, moving.length === 0);
  await shot(`previewed-8-reduced-motion-${scheme}`);
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });

  // ---- the other of dark and light --------------------------------------------------
  small = await signIn(await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: other }));
  await openAt('std/fleet', small);
  await head(small).getByText('checks', { exact: true }).waitFor();
  await opened('std/fleet', small);
  await recording(small);
  await readPart('does', small);
  await small.locator('.drawer .preview[data-frame="1"]').waitFor({ timeout: 30_000 });
  await laidOut('in the other of dark and light', small);
  await shot(`previewed-9-fleet-${other}`, small);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/previewed-0-failed.png` }).catch(() => {});
  await small?.screenshot({ path: `${out}/previewed-0-failed-other.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
