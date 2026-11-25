// A tab shown again is as it was left.
//
//   DAEMON=../target/debug/factory node scripts/returned.mjs [out-dir]
//
// Each kind of tab is opened, made to show something more than it opens
// with (another of its views, something folded or unfolded, scrolled down
// its page, the drawer opened beside it with its own body scrolled), and
// then another tab is shown and this one again. Every frame drawn on the
// way back is looked at: none says that it is reading or opening what it
// had, and when it is there it is as it was left: scrolled where it was,
// the same view of it showing, what was folded folded, and the drawer
// open beside it as it was, there at once and not coming in again. A
// page of a wiki is among them, with a chain of its replies
// unfolded and something half written in its box. A
// card added from a terminal while the board was away is on it once it is
// back. A tab that is closed and opened again begins as a tab does, with
// nothing kept of it. Run with `TRIAL=1` it fails for nothing and says,
// for each kind of tab, what it lost. The environment variables are those
// of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { DRAWER_AT_REST } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const name = process.env.RETURNED ?? 'returned';
const data = join(out, `${name}-data`);
const home = join(out, `${name}-home`);
const trial = process.env.TRIAL === '1';

const PLAN = `blueprint: notes
description: A table of notes, for a page that is longer than the window.
permissions: [projects]
steps:
  notes:
    do: collection.ensure
    with:
      name: notes
      fields:
        title: { type: text, required: true }
        said: { type: text }
`;

// A pipeline of the project's own whose run is longer than the window: stages each of a step that writes many lines.
const LONG = `blueprint: pipelines/long
description: A run longer than the window.
steps:
${['first', 'second', 'third', 'fourth'].map((stage) => `  ${stage}: { do: shell, with: { cmd: [seq, 1, 80] } }`).join('\n')}
`;

// Tells whoever applies it what it is handed: a line of the inbox, which has a page of its own.
const TOLD = `blueprint: told
permissions: [inbox]
params:
  says: { type: string }
steps:
  tell:
    do: inbox.tell
    with: { to: applier, says: "\${params.says}", why: built }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env, maxBuffer: 1 << 24 });

let failed = null;
let page = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', 'Shop', '--without', 'fleet');
  const shop = join(data, 'projects/shop');
  // A project's own blueprint, where the factory keeps it (`@blueprints/` of the project's).
  const kept = join(data, 'blueprints', 'projects', 'shop');
  await mkdir(kept, { recursive: true });
  await writeFile(join(kept, 'notes.bp'), PLAN);
  await mkdir(join(kept, 'pipelines'), { recursive: true });
  await writeFile(join(kept, 'pipelines', 'long.bp'), LONG);
  await cli('project', 'scan', 'shop');
  await cli('apply', join(kept, 'notes.bp'), '--project', 'shop');
  // A run of the pipeline, whose page is a tab like any other.
  const ran = JSON.parse((await cli('apply', 'pipelines/long', '--project', 'shop', '--json')).stdout).run;
  // Each page longer than the window, so that where it was scrolled to says something.
  await Promise.all(Array.from({ length: 6 }, (_, part) => (async () => {
    for (let at = part; at < 36; at += 6) {
      await cli('board', 'add', 'shop', `Card ${at + 1}`, '--status', at % 2 ? 'ready' : 'backlog');
      await cli('record', 'add', 'shop', 'notes', `title=Note ${at + 1}`, `said=${'something said '.repeat(4)}`);
    }
  })()));
  const long = ['# A long page', '', ...Array.from({ length: 120 }, (_, at) => `Line ${at + 1} of a page that is longer than the window.\n`)].join('\n');
  await writeFile(join(shop, 'docs/long.md'), long);
  await cli('project', 'scan', 'shop');
  await cli('channel', 'create', 'talk');
  for (let at = 0; at < 40; at += 1) await cli('post', 'talk', `Message ${at + 1}, which is one of many.`);
  // A line of the inbox longer than the window, whose page is a tab like any other.
  await writeFile(join(home, 'told.bp'), TOLD);
  const words = Array.from({ length: 60 }, (_, at) => `Line ${at + 1} of what was told, longer than the window.`).join('\n');
  await cli('apply', join(home, 'told.bp'), '--param', `says=${words}`);
  const told = JSON.parse((await cli('inbox', '--json')).stdout).lines[0].id;
  // A page of a wiki with a page above it and one under it, and a chain of replies longer than the window.
  const topicMade = async (...args) => JSON.parse((await cli('topic', 'make', ...args, '--json')).stdout).id;
  const handbook = await topicMade('Handbook');
  const deploying = await topicMade('Deploying', '--under', handbook, '--body', 'How a build goes out.');
  await topicMade('Rolling back', '--under', deploying);
  let saidUnder = null;
  for (let nth = 1; nth <= 14; nth += 1) {
    const words = `Step ${nth} of going out, said at length so that the chain is longer than the window is tall.`;
    saidUnder = (await cli('topic', 'reply', deploying, words, ...(saidUnder ? ['--to', saidUnder] : []))).stdout.trim().replace('said at ', '');
  }
  const instances = JSON.parse((await cli('instances', '--json')).stdout);
  const instance = (instances.instances ?? instances).find((one) => one.blueprint === 'notes' || one.key === 'notes');
  if (!instance) throw new Error(`expected the notes blueprint to be running, got ${JSON.stringify(instances).slice(0, 300)}`);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  page = await browser.newPage({ viewport: { width: 1350, height: 760 }, colorScheme: process.env.SCHEME ?? 'dark' });
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
  const resources = page.getByRole('navigation', { name: 'Resources' });
  const side = async (called) => {
    const button = page.locator('.rail').getByRole('button', { name: called, exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
    await page.getByRole('navigation', { name: called }).waitFor();
    // Every branch of the channels starts folded, as everything of Resources does (`folded.mjs`).
    if (called === 'Channels') await unfoldChannels(page);
  };
  const view = page.locator('.view');
  const at = (hash) => page.evaluate((to) => (location.hash = to), hash);
  // A few frames drawn, so that what a view does once it is on the page has been done.
  const frames = (count = 8) => page.evaluate((many) => new Promise((done) => {
    let left = many;
    const next = () => (--left <= 0 ? done() : requestAnimationFrame(next));
    requestAnimationFrame(next);
  }), count);
  const factoryLine = async (called, there) => {
    await side('Resources');
    // Factory is a part of the panel that starts unfolded, and may have been folded since (`folded.mjs`).
    const factory = resources.getByRole('button', { name: 'Unfold Factory', exact: true });
    if ((await factory.count()) > 0) await factory.click();
    await resources.locator('.nav').getByText(called, { exact: true }).click();
    await view.locator(there).first().waitFor();
  };

  // What scrolls in a view (or the drawer), found by what it is and how many of its kind come before it.
  const scrollers = (root) =>
    page.evaluate((sel) => {
      const top = document.querySelector(sel);
      if (!top) return [];
      const sign = (one) => `${one.tagName}.${[...one.classList].sort().join('.')}`;
      const all = [top, ...top.querySelectorAll('*')];
      return all
        .filter((one) => one.scrollHeight > one.clientHeight + 40 && ['auto', 'scroll'].includes(getComputedStyle(one).overflowY))
        .map((one) => ({ sign: sign(one), nth: all.filter((other) => sign(other) === sign(one)).indexOf(one), top: Math.round(one.scrollTop), most: one.scrollHeight - one.clientHeight }));
    }, root);
  const scrollTo = (root, which, top) =>
    page.evaluate(({ sel, which: { sign, nth }, top: to }) => {
      const at = document.querySelector(sel);
      const all = [at, ...at.querySelectorAll('*')];
      const mark = (one) => `${one.tagName}.${[...one.classList].sort().join('.')}`;
      const found = all.filter((one) => mark(one) === sign)[nth];
      found.scrollTop = to;
      found.dispatchEvent(new Event('scroll'));
    }, { sel: root, which, top });
  /** Scrolls the largest scroller of what is found to most of its way down, and says which it was and how far. */
  const scrollDown = async (root) => {
    const all = await scrollers(root);
    if (all.length === 0) return null;
    const biggest = all.sort((a, b) => b.most - a.most)[0];
    const to = Math.round(biggest.most * 0.6);
    await scrollTo(root, biggest, to);
    await frames(4);
    const now = (await scrollers(root)).find((one) => one.sign === biggest.sign && one.nth === biggest.nth);
    return now ? { sign: now.sign, nth: now.nth, top: now.top } : null;
  };
  const scrolledAt = async (root, which) => (await scrollers(root)).find((one) => one.sign === which.sign && one.nth === which.nth)?.top ?? 0;

  // What is at the top of a part that scrolls, a little in from its corner: what the reader sees first there.
  const atTop = (root, which) =>
    page.evaluate(({ sel, which: { sign, nth } }) => {
      const at = document.querySelector(sel);
      const all = [at, ...at.querySelectorAll('*')];
      const mark = (one) => `${one.tagName}.${[...one.classList].sort().join('.')}`;
      const box = all.filter((one) => mark(one) === sign)[nth]?.getBoundingClientRect();
      if (!box) return '';
      const there = document.elementFromPoint(box.left + 40, box.top + 12);
      return (there?.textContent ?? '').trim().slice(0, 60);
    }, { sel: root, which });
  // The line of the code editor's text at the top of what shows of it.
  const topLine = () =>
    page.evaluate(() => {
      const scroller = document.querySelector('.view .cm-scroller');
      const top = scroller.getBoundingClientRect().top;
      // The first line more than half of which shows: a line by a pixel or two at the edge is the same line either way.
      const shows = (line) => {
        const box = line.getBoundingClientRect();
        return box.bottom > top + box.height / 2;
      };
      return [...scroller.querySelectorAll('.cm-line')].find(shows)?.textContent ?? '';
    });
  // Every frame drawn on the way back, until it is told to stop: what the view and the drawer said in each.
  const watch = () =>
    page.evaluate(() => {
      window.seen = [];
      const look = () => {
        const words = document.querySelector('.view')?.innerText ?? '';
        const drawer = document.querySelector('.main > .drawer');
        window.seen.push({
          reading: /\b(Reading|Opening)\b/.test(words.slice(0, 4000)) || words.trim() === '',
          said: words.slice(0, 80).replace(/\s+/g, ' '),
          drawer: drawer ? { rest: 'rest' in drawer.dataset, moving: drawer.getAnimations().length > 0 } : null,
        });
        if (window.seen.length < 40) requestAnimationFrame(look);
      };
      requestAnimationFrame(look);
    });
  const watched = () => page.evaluate(() => window.seen);

  const away = async () => {
    await page.locator('.tab[data-key="home"]').click();
    await page.locator('.view[data-tab="home"]').waitFor();
    await frames(4);
  };
  const back = async (key) => {
    await watch();
    await page.locator(`.tab[data-key="${key}"]`).click();
    await frames(40);
  };
  const pressed = (pills) => view.locator(`${pills} [aria-pressed="true"], ${pills} [aria-checked="true"]`).first().innerText().catch(() => '(none)');

  const CASES = [
    {
      kind: 'blueprint page',
      key: 'blueprint:shop:std/board',
      open: () => at('#/blueprint/std/board?project=shop'),
      ready: '.view-head h1',
      make: async () => {
        await view.locator('.view-head .pills').getByText('Text', { exact: true }).click();
        await view.locator('.cm-scroller').waitFor();
        await frames(6);
      },
      sub: () => pressed('.view-head .pills'),
    },
    {
      kind: 'blueprint page, a step\'s drawer',
      key: 'blueprint::std/cards',
      open: () => at('#/blueprint/std/cards'),
      ready: '.view-head h1',
      make: async () => {
        await view.locator('.view-head .pills').getByText('Node', { exact: true }).click();
        await view.locator('.gnode').first().waitFor();
        await view.locator('.gnode').first().click();
        await page.locator(DRAWER_AT_REST).waitFor();
      },
      drawer: true,
      sub: () => pressed('.view-head .pills'),
      chosen: () => view.locator('.gnode.selected').getAttribute('data-node').catch(() => '(none)'),
    },
    {
      kind: 'project blueprint, Runs',
      key: 'file:shop:@blueprints/notes.bp',
      open: () => at('#/file/shop/@blueprints/notes.bp'),
      ready: '.view-head h1',
      make: async () => {
        await view.locator('.view-head .pills').getByText('Runs', { exact: true }).click();
        await frames(6);
      },
      sub: () => pressed('.view-head .pills'),
    },
    {
      kind: 'running instance, details drawer',
      key: `instance:${instance.id}`,
      open: () => at(`#/instance/${instance.id}`),
      ready: '.view-head h1',
      make: async () => {
        await view.getByRole('button', { name: 'Details', exact: true }).click();
        await page.locator(DRAWER_AT_REST).waitFor();
      },
      drawer: true,
      sub: () => pressed('.view-head .pills'),
    },
    {
      kind: 'table, row drawer',
      key: 'table:shop:notes',
      open: () => at('#/table/shop/notes'),
      ready: '.grid-scroll',
      make: async () => {
        await view.locator('.grid-scroll').getByText('Note 3', { exact: true }).first().hover();
        await view.getByRole('button', { name: 'Open Note 3', exact: true }).click();
        await page.locator(DRAWER_AT_REST).waitFor();
      },
      drawer: true,
    },
    {
      kind: 'board, card drawer',
      key: 'board:shop',
      open: () => at('#/board/shop'),
      ready: '.board-col, .column, .card',
      make: async () => {
        await view.getByText('Card 2', { exact: true }).first().click();
        await page.locator(DRAWER_AT_REST).waitFor();
        await page.locator('.drawer').getByLabel('Title').fill('Card 2, half changed');
      },
      drawer: true,
      typed: () => page.locator('.drawer').getByLabel('Title').inputValue(),
      // Added from a terminal while the board is away, and to be on it once it is back.
      meanwhile: () => cli('board', 'add', 'shop', 'Added while away', '--status', 'backlog'),
      shows: 'Added while away',
    },
    { kind: 'library', key: 'library:shop:docs', open: () => at('#/library/shop/docs'), ready: '.view-head h1' },
    { kind: 'document', key: 'file:shop:docs/long.md', open: () => at('#/file/shop/docs/long.md'), ready: '.view-head h1, .writer' },
    {
      kind: 'channel',
      key: null,
      open: async () => {
        await side('Channels');
        await page.getByRole('navigation', { name: 'Channels' }).getByRole('button', { name: /^# talk( \d+)?$/ }).click();
      },
      ready: '.composer',
    },
    { kind: 'project page', key: 'project:shop', open: () => at('#/project/shop'), ready: '.view-head h1' },
    { kind: 'files', key: 'repo:shop', open: () => at('#/files/shop'), ready: '.view-head h1' },
    { kind: 'activity', key: 'activity', open: () => factoryLine('Activity', '.view-head'), ready: '.view-head' },
    { kind: 'measurements', key: 'metrics', open: () => factoryLine('Measurements', '.view-head'), ready: '.view-head' },
    {
      kind: 'settings',
      key: 'settings',
      open: async () => {
        await page.getByRole('button', { name: 'Settings' }).first().click();
      },
      ready: '.view-head, h1',
    },
    { kind: 'trash', key: 'trash', open: () => factoryLine('Trash', '.view-head'), ready: '.view-head' },
    {
      kind: 'pipeline run, a stage folded',
      key: `run:${ran}`,
      open: () => at(`#/run/${ran}`),
      ready: '.pipeline-stage[data-stage="fourth"][data-status="ok"]',
      make: async () => {
        await view.locator('.pipeline-stage[data-stage="second"] .stage-band').click();
        await frames(4);
      },
      folds: () => view.locator('.stage-band[aria-expanded="false"]').count(),
    },
    {
      kind: 'pipelines, what else runs unfolded',
      key: 'pipelines:shop',
      open: () => at('#/pipelines/shop'),
      ready: '.pipeline-row[data-pipeline="pipelines/long"] a.run-mark',
      make: async () => {
        await view.getByRole('button', { name: /Everything else that runs here/ }).click();
        await view.locator('.pipelines-others .pipeline-row').first().waitFor();
      },
      folds: () => view.locator('.pipelines-fold[aria-expanded="true"]').count(),
    },
    { kind: 'inbox message', key: `inbox:${told}`, open: () => at(`#/inbox/${told}`), ready: '.inbox-page-words' },
    {
      kind: 'topic, a page of a wiki on Talk with its chain unfolded and a reply half written',
      key: `topic:${deploying}`,
      open: () => at(`#/topic/${encodeURIComponent(deploying)}`),
      ready: '.topic-page .topic-path .topic-path-page',
      make: async () => {
        // Its talk, a tab of its own beside its article: shown again, the tab is on it still, or nothing below is there.
        await view.getByRole('tab', { name: /^Talk/ }).click();
        await view.locator('.topic-reply-more').first().click();
        await view.locator('.topic-reply-more').waitFor({ state: 'detached' });
        // Replies nest eight deep and no deeper: the last of the chain is beside the eighth.
        await view.locator('.topic-reply[data-depth="8"]').last().waitFor({ state: 'attached' });
        await view.getByLabel('Reply to the topic', { exact: true }).fill('half written');
      },
      folds: () => view.locator('.topic-reply-more').count(),
      typed: () => view.getByLabel('Reply to the topic', { exact: true }).inputValue(),
    },
    {
      kind: 'reference, unfolded',
      key: 'reference',
      open: () => at('#/reference'),
      ready: '.reference',
      make: async () => {
        await view.locator('.reference').getByRole('button').filter({ hasText: /^collection\.ensure/ }).first().click().catch(() => {});
        await frames(6);
      },
      folds: () => view.locator('[aria-expanded="true"]').count(),
    },
  ];

  const rows = [];
  // One kind of tab alone, where it is named (`ONLY=board`): for looking at one closely.
  for (const one of CASES.filter((each) => !process.env.ONLY || each.kind.includes(process.env.ONLY))) {
    await one.open();
    await view.locator(one.ready).first().waitFor({ timeout: 15000 });
    const key = one.key ?? (await page.locator('.tab[aria-selected="true"]').first().getAttribute('data-key'));
    await frames(10);
    await one.make?.();
    const scrolled = await scrollDown('.view');
    if (scrolled?.sign.includes('cm-scroller')) scrolled.line = await topLine();
    else if (scrolled) scrolled.seen = await atTop('.view', scrolled);
    const drawerWas = one.drawer ? { title: await page.locator('.drawer-head h2').innerText(), scrolled: await scrollDown('.main > .drawer .drawer-body') } : null;
    const sub = await one.sub?.();
    const folds = await one.folds?.();
    const typed = await one.typed?.();
    const chosen = await one.chosen?.();
    const said = (await view.innerText()).slice(0, 200);
    await page.screenshot({ path: `${out}/${name}-${rows.length}-left.png` });
    await away();
    await one.meanwhile?.();
    await back(key);
    const seen = await watched();
    const row = { kind: one.kind, lost: [] };
    // What was held of it, said with what it came to.
    row.held = [
      'no frame reading',
      scrolled && `scrolled ${scrolled.top}`,
      sub !== undefined && `view ${sub}`,
      folds !== undefined && `${folds} unfolded`,
      drawerWas && `drawer "${drawerWas.title}"${drawerWas.scrolled ? ` scrolled ${drawerWas.scrolled.top}` : ''}`,
      typed !== undefined && 'typed',
      chosen !== undefined && `chosen ${chosen}`,
      one.shows && 'changed while away',
    ].filter(Boolean);
    if (seen.some((frame) => frame.reading)) row.lost.push(`drawn as reading (${seen.filter((frame) => frame.reading).length} of ${seen.length} frames: "${seen.find((frame) => frame.reading).said}")`);
    if (scrolled && scrolled.sign.includes('cm-scroller')) {
      // The code editor knows how tall its lines are only once it has drawn them: it is where it was if the same line is at its top.
      const now = await topLine();
      if (now !== scrolled.line) row.lost.push(`the line at the top of its text ("${scrolled.line}" then "${now}")`);
    } else if (scrolled) {
      // Where it was scrolled to, or, where what was read again is taller above it and the browser kept what was in
      // view in view, the same thing at its top.
      const now = await scrolledAt('.view', scrolled);
      const seen = await atTop('.view', scrolled);
      if (Math.abs(now - scrolled.top) > 4 && seen !== scrolled.seen) row.lost.push(`scroll (${scrolled.top} then ${now}, "${scrolled.seen}" at its top then "${seen}")`);
    } else row.note = 'nothing to scroll';
    if (sub !== undefined) {
      const now = await one.sub();
      if (now !== sub) row.lost.push(`its view (${sub} then ${now})`);
    }
    if (typed !== undefined) {
      const now = await one.typed();
      if (now !== typed) row.lost.push(`what was typed in the drawer ("${typed}" then "${now}")`);
    }
    if (chosen !== undefined) {
      const now = await one.chosen();
      if (now !== chosen) row.lost.push(`what was chosen (${chosen} then ${now})`);
    }
    if (folds !== undefined) {
      const now = await one.folds();
      if (now !== folds) row.lost.push(`what was unfolded (${folds} then ${now})`);
    }
    if (drawerWas) {
      const there = await page.locator('.main > .drawer').count();
      if (!there) row.lost.push('the drawer');
      else {
        const title = await page.locator('.drawer-head h2').innerText();
        if (title !== drawerWas.title) row.lost.push(`the drawer's contents (${drawerWas.title} then ${title})`);
        if (seen.some((frame) => frame.drawer?.moving)) row.lost.push('the drawer came in again');
        if (seen.some((frame) => frame.drawer && !frame.drawer.rest && !frame.drawer.moving)) row.lost.push('the drawer did not say it was at rest');
        if (drawerWas.scrolled) {
          const now = await scrolledAt('.main > .drawer .drawer-body', drawerWas.scrolled);
          if (Math.abs(now - drawerWas.scrolled.top) > 4) row.lost.push(`the drawer's scroll (${drawerWas.scrolled.top} then ${now})`);
        }
      }
    }
    if (one.shows) {
      const shown = await view.getByText(one.shows, { exact: true }).first().waitFor({ timeout: 8000 }).then(() => true, () => false);
      if (!shown) row.lost.push(`what changed while it was away ("${one.shows}" not shown)`);
    }
    const now = (await view.innerText()).slice(0, 200);
    if (!one.shows && now !== said) row.differs = true;
    await page.screenshot({ path: `${out}/${name}-${rows.length}-back.png` });
    rows.push(row);
    console.log(`${one.kind}: ${row.lost.length ? `lost ${row.lost.join('; ')}` : 'as it was'} [${row.held.join(', ')}]`);
    // The drawer is put away before the next, which would otherwise open beside it.
    if (await page.locator('.main > .drawer').count()) {
      await page.locator('.drawer-head').getByRole('button', { name: 'Close' }).click();
      await page.locator('.main > .drawer').waitFor({ state: 'detached' });
    }
  }

  // ---- a tab closed and opened again begins as a tab does
  await at('#/board/shop');
  await view.getByText('Card 2', { exact: true }).first().click();
  await page.locator(DRAWER_AT_REST).waitFor();
  await away();
  await page.locator('.tab[data-key="board:shop"]').hover();
  await page.locator('.tab[data-key="board:shop"]').getByRole('button', { name: /^Close/ }).click();
  await page.locator('.tab[data-key="board:shop"]').waitFor({ state: 'detached' });
  await at('#/board/shop');
  await view.getByText('Card 2', { exact: true }).first().waitFor();
  await frames(20);
  const closedKept = await page.locator('.main > .drawer').count();
  console.log(`a closed tab opened again: ${closedKept ? 'kept its drawer' : 'begins with nothing kept'}`);

  // ---- a drawer of something taken away while its tab was away is put away, and kept for nobody
  await view.getByText('Card 4', { exact: true }).first().click();
  await page.locator(DRAWER_AT_REST).waitFor();
  const card = await page.locator('.drawer-head h2').innerText();
  await away();
  // Taken away by somebody else meanwhile: by the route the board's page takes one away by, as its owner.
  await page.evaluate(async (id) => {
    const done = await fetch(`/api/board/items/${encodeURIComponent(id)}`, { method: 'DELETE', headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    if (!done.ok) throw new Error(`could not take ${id} away: ${done.status}`);
  }, card);
  await page.locator('.tab[data-key="board:shop"]').click();
  const goneAway = await page.locator('.main > .drawer').waitFor({ state: 'detached', timeout: 8000 }).then(() => true, () => false);
  console.log(`a card taken away while its board was away: ${goneAway ? 'its drawer put away' : 'its drawer still there'}`);
  await away();
  await page.locator('.tab[data-key="board:shop"]').click();
  await frames(20);
  const keptGone = await page.locator('.main > .drawer').count();

  if (!trial) {
    expect('the drawer of a card taken away while its tab was away to be put away, and kept for nobody', goneAway && keptGone === 0);
    const lost = rows.filter((row) => row.lost.length);
    expect(`every tab to be as it was left, and ${lost.map((row) => `${row.kind} lost ${row.lost.join('; ')}`).join(' | ')}`, lost.length === 0);
    expect('a tab closed and opened again to begin with no drawer of its own', closedKept === 0);
  }
  expect(`no errors on the page, got ${problems.join(' | ')}`, problems.length === 0);
  await browser.close();
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/${name}-failed.png` }).catch(() => {});
} finally {
  daemon.kill('SIGTERM');
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log('returned: every tab as it was left');
