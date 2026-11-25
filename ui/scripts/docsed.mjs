// The documents a channel keeps to hand, in the Docs tab of the Tools
// panel beside it: one of the panel's usual tabs, its row of tabs held by
// its boxes at the panel's default width, dragged narrow and on a phone;
// a library's document and a project's file added from the list of
// the channel's workspace's projects, which offers nothing of another
// workspace's; one read there in place and opened as a tab; one taken
// away; one whose file went said to be gone; a second page of somebody
// else's, never read again, following each change; somebody who may not
// write in the channel offered nothing to change; and nothing of the tab
// over anything else, outside the panel or scrolling sideways at three
// widths and a phone's. And the tabs as they are drawn: words on the panel
// with no edge or ground of their own, the one showing in the text's
// colour with the accent's line along its foot, the others dimmer, and a
// thin line under the whole row that the accent's line lies on, in the
// dark and the light; and every line of the Channel tab's settings with its drawing,
// Discussions… a speech bubble. Run plainly and with `LOOK="size=largest
// density=compact"`.
//
//   DAEMON=../target/debug/factory node scripts/docsed.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { toolsAtRest } from './drawer.mjs';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'docsed-data');
const home = join(out, 'docsed-home');
const looked = process.env.LOOK ? '-looked' : '';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
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
  await cli('tier', 'premium');
  // Shop, with its library (two documents) and a file of its own; and Lantern, in a workspace of its own, with a file.
  await cli('project', 'create', 'Shop', '--without', 'fleet');
  const notes = join(home, 'notes.md');
  await writeFile(notes, '# Notes\n\nThe quince is ripe, and the medlar is not yet.\n');
  await cli('file', 'write', 'shop', 'notes.md', '--from', notes);
  await cli('channel', 'create', 'pantry', '--project', 'shop');
  await cli('workspace', 'create', 'Annex');
  await cli('project', 'create', 'Lantern', '--without', 'fleet', '--without', 'docs');
  await cli('file', 'write', 'lantern', 'lamp.md', '--from', notes);
  await cli('workspace', 'put', 'annex', 'lantern');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  if (!joined.ok) throw new Error(`expected Grace to join, got ${joined.status}`);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  /** A page of `email`'s, signed in, in a context of its own. `holds` is what the page is told they hold, where it is to draw as for somebody who holds less. */
  const opened = async (email, holds = null) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    if (holds) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = holds;
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.rail').waitFor();
    return page;
  };
  /** The Channels side panel with every branch unfolded, the channel opened, and the Tools panel beside it on its Docs tab. */
  const docsOf = async (page) => {
    const button = page.getByRole('button', { name: 'Channels', exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
    await unfoldChannels(page);
    await page.getByRole('navigation', { name: 'Channels' }).getByText('pantry', { exact: true }).click();
    await page.getByRole('heading', { name: '# pantry' }).waitFor();
    const panel = page.getByRole('complementary', { name: 'Tools' });
    if ((await panel.count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
    await toolsAtRest(page).waitFor();
    await panel.waitFor();
    // Docs is one of the usual tabs: a browser that kept nothing has it with no adding.
    const usual = (await panel.getByRole('tab').allInnerTexts()).map((one) => one.replace('×', '').trim());
    expect(`the usual tabs to be Media, Channel, Board and Docs, got ${usual}`, usual.join() === 'Media,Channel,Board,Docs');
    await panel.getByRole('tab', { name: 'Docs' }).click();
    await panel.locator('.channel-docs').waitFor();
    return panel;
  };
  /**
   * The panel's row of tabs by its boxes: it is inside the panel and ends
   * before the `+`, which is whole inside the panel; the tab showing is in
   * sight in it; and where the row has more than it is wide, it scrolls
   * sideways in itself and nothing else does. Turned by the wheel to its
   * start and to its end, the first and the last tab are each in sight and
   * clear of the `+`, and the tab showing is brought back into sight when
   * it is chosen again.
   */
  const tabRow = async (page, panel, at) => {
    const boxes = () =>
      panel.evaluate((aside) => {
        const rim = aside.getBoundingClientRect();
        const row = aside.querySelector('.context-tabs');
        const list = row.querySelector('[role="tablist"]');
        const plus = row.querySelector('button[aria-label="Add a tab"]');
        const tabs = [...list.querySelectorAll('[role="tab"]')];
        const [r, l, p] = [row.getBoundingClientRect(), list.getBoundingClientRect(), plus?.getBoundingClientRect()];
        const seen = (tab) => {
          // Within a pixel: a row is scrolled by whole pixels, and a tab is as wide as its words, which is not.
          const box = tab.getBoundingClientRect();
          return box.left >= l.left - 1 && box.right <= l.right + 1 && (!p || box.right <= p.left + 1);
        };
        const chosen = tabs.find((tab) => tab.getAttribute('aria-selected') === 'true');
        return {
          inside: r.left >= rim.left - 0.5 && r.right <= rim.right + 0.5 && l.left >= rim.left - 0.5,
          plus: !p || (p.width > 0 && p.left >= l.right - 0.5 && p.right <= rim.right + 0.5 && p.left >= rim.left - 0.5),
          chosen: chosen ? seen(chosen) : true,
          first: seen(tabs[0]),
          last: seen(tabs.at(-1)),
          scrolls: list.scrollWidth > list.clientWidth + 1,
          sideways: Math.max(row.scrollWidth - row.clientWidth, aside.scrollWidth - aside.clientWidth),
          page: document.documentElement.scrollWidth - window.innerWidth,
        };
      });
    const settled = () => panel.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    const now = await boxes();
    expect(`the row of tabs inside the panel, ending before a whole + and with the tab showing in sight, at ${at}: ${JSON.stringify(now)}`, now.inside && now.plus && now.chosen && now.sideways <= 1 && now.page <= 0);
    expect(`every tab on the row, or the row scrolling in itself, at ${at}: ${JSON.stringify(now)}`, now.scrolls || (now.first && now.last));
    if (!now.scrolls) return;
    const list = await panel.getByRole('tablist', { name: 'Tools' }).boundingBox();
    await page.mouse.move(list.x + 12, list.y + list.height / 2);
    await page.mouse.wheel(0, -900);
    await settled();
    expect(`the first tab in sight, the row turned to its start by the wheel, at ${at}`, (await boxes()).first);
    await page.mouse.wheel(0, 900);
    await settled();
    expect(`the last tab in sight and clear of the +, the row turned to its end by the wheel, at ${at}`, (await boxes()).last);
    await page.mouse.wheel(0, -900);
    await settled();
    await panel.getByRole('tab', { name: 'Board' }).click();
    await panel.getByRole('tab', { name: 'Docs' }).click();
    await settled();
    expect(`the tab chosen brought into sight, clear of the +, at ${at}`, (await boxes()).chosen);
  };
  /**
   * The tabs as they are drawn (as a code-hosting site draws a
   * repository's): words on the panel, no tab with
   * an edge or a ground of its own; the one showing in the text's own
   * colour with a line of the accent two pixels thick along its foot, as
   * wide as the tab, and the others in the dimmer text colour with none; a
   * thin line under the whole row, from the panel's one side to its other,
   * which every tab reaches down to, so that the accent's line lies on it.
   * All of them as tall as one another and on one line. Held by the boxes
   * and the colours, in the dark and in the light (`scheme`).
   */
  const tabsDrawn = async (panel, at) => {
    // Pointed at, a tab is lit as it would be under any pointer: the pointer is taken off the tabs first.
    await panel.page().mouse.move(1, 1);
    const drawn = await panel.evaluate((aside) => {
      const none = (colour) => colour === 'transparent' || /rgba\(.*,\s*0\)$/.test(colour);
      // What a colour of the theme comes to, read as the page reads it.
      const colourOf = (token) => {
        const probe = document.createElement('span');
        probe.style.color = `var(${token})`;
        aside.append(probe);
        const colour = getComputedStyle(probe).color;
        probe.remove();
        return colour;
      };
      const [accent, text, dim, line] = ['--accent', '--text', '--dim', '--line'].map(colourOf);
      // An inset shadow as its colour and its offsets: `rgb(…) 0px -2px 0px 0px inset`.
      const shadow = (style) => {
        const found = /^(rgba?\([^)]*\)) (-?[\d.]+)px (-?[\d.]+)px (-?[\d.]+)px (-?[\d.]+)px inset$/.exec(style.boxShadow);
        return found ? { colour: found[1], x: +found[2], y: +found[3], blur: +found[4], spread: +found[5] } : null;
      };
      const row = aside.querySelector('.context-tabs');
      const rim = aside.getBoundingClientRect();
      const rowBox = row.getBoundingClientRect();
      const ruled = shadow(getComputedStyle(row));
      const tabs = [...row.querySelectorAll('[role="tab"]')].map((tab) => {
        const style = getComputedStyle(tab);
        const box = tab.getBoundingClientRect();
        const edged = ['Left', 'Top', 'Right', 'Bottom'].filter((side) => parseFloat(style[`border${side}Width`]) > 0 && !none(style[`border${side}Color`]));
        const under = shadow(style);
        return {
          tab: tab.textContent.replace('×', '').trim(),
          chosen: tab.getAttribute('aria-selected') === 'true',
          top: Math.round(box.top * 2) / 2,
          height: Math.round(box.height * 2) / 2,
          // From its foot to the foot of the row, where the line is.
          above: Math.round((rowBox.bottom - box.bottom) * 2) / 2,
          ground: none(style.backgroundColor),
          edged,
          colour: style.color === text ? 'text' : style.color === dim ? 'dim' : style.color,
          // The line along its foot, which is the tab's own and so as wide as it is: the accent, two pixels, straight down.
          underlined: !!under && under.colour === accent && under.x === 0 && under.y === -2 && under.blur === 0 && under.spread === 0,
          unlined: !under || none(under.colour),
        };
      });
      return {
        tabs,
        line: !!ruled && ruled.colour === line && ruled.y === -1 && ruled.x === 0 && ruled.blur === 0,
        across: rowBox.left <= rim.left + 1.5 && rowBox.right >= rim.right - 1.5,
        rule: getComputedStyle(row).boxShadow,
      };
    });
    const { tabs } = drawn;
    const chosen = tabs.filter((one) => one.chosen);
    const others = tabs.filter((one) => !one.chosen);
    expect(`one tab showing at ${at}, got ${JSON.stringify(tabs)}`, chosen.length === 1 && others.length >= 1);
    const [it] = chosen;
    expect(`the tab showing in the text's colour with the accent's line two pixels thick along its foot, at ${at}: ${JSON.stringify(it)}`, it.colour === 'text' && it.underlined);
    const marked = others.filter((one) => one.colour !== 'dim' || !one.unlined);
    expect(`every other tab in the dimmer text colour with no line under it, at ${at}: ${JSON.stringify(marked)}`, marked.length === 0);
    const boxed = tabs.filter((one) => one.edged.length > 0 || !one.ground);
    expect(`no tab with an edge or a ground of its own, at ${at}: ${JSON.stringify(boxed)}`, boxed.length === 0);
    expect(`a thin line of the theme's under the whole row, from the panel's one side to its other, at ${at}: ${JSON.stringify({ line: drawn.line, across: drawn.across, rule: drawn.rule })}`, drawn.line && drawn.across);
    const off = tabs.filter((one) => Math.abs(one.above) > 0.5);
    expect(`every tab reaching down to the row's line, the accent's line lying on it, at ${at}: ${JSON.stringify(off)}`, off.length === 0);
    const lined = new Set(tabs.map((one) => `${one.top}/${one.height}`));
    expect(`every tab as tall as the others and on one line, at ${at}: ${JSON.stringify(tabs.map((one) => [one.tab, one.top, one.height]))}`, lined.size === 1);
  };
  /** The lines of the list, as `called | where`. */
  const lines = (panel) =>
    panel.locator('.channel-doc').evaluateAll((all) => all.map((one) => [...one.querySelectorAll('.channel-doc-words > span')].map((part) => part.textContent.trim()).join(' | ')));
  const until = async (what, ask, wanted) => {
    let last;
    for (let tries = 0; tries < 150; tries += 1) {
      last = await ask();
      if (JSON.stringify(last) === JSON.stringify(wanted)) return;
      await new Promise((later) => setTimeout(later, 100));
    }
    throw new Error(`expected ${what}: ${JSON.stringify(wanted)}, and it is ${JSON.stringify(last)}`);
  };

  // ---- the owner's page, and Grace's, which is never read again
  const page = await opened('ada@example.com');
  const panel = await docsOf(page);
  await panel.getByText('This channel keeps no documents to hand yet.').waitFor();
  await tabRow(page, panel, 'the default width');
  await tabsDrawn(panel, `the default width, in the ${process.env.SCHEME ?? 'dark'}`);
  // Each line of the channel's settings with its drawing, Discussions… a speech bubble, so that their names begin together.
  await panel.getByRole('tab', { name: 'Channel' }).click();
  const settings = panel.getByRole('region', { name: 'Settings of this channel' });
  await settings.getByText('Discussions…', { exact: true }).waitFor();
  await tabsDrawn(panel, 'the Channel tab chosen');
  const marked = await settings.locator('.channel-setting').evaluateAll((all) =>
    all.map((line) => ({ label: line.querySelector('.grow').textContent, mark: !!line.querySelector('.channel-setting-mark svg'), left: Math.round(line.querySelector('.grow').getBoundingClientRect().left * 2) / 2 })),
  );
  expect(`every line of the channel's settings, Discussions… among them, to have its drawing and its name begin where the others' do, got ${JSON.stringify(marked)}`, marked.some((one) => one.label === 'Discussions…') && marked.every((one) => one.mark) && new Set(marked.map((one) => one.left)).size === 1);
  await page.screenshot({ path: `${out}/docsed-0-settings${looked}.png` });
  // The other of dark and light, and back to what the page was opened in.
  const scheme = process.env.SCHEME ?? 'dark';
  const other = scheme === 'dark' ? 'light' : 'dark';
  const groundOf = () => panel.evaluate((aside) => getComputedStyle(aside).backgroundColor);
  const wasGround = await groundOf();
  await litAs(page, other);
  await page.waitForFunction((was) => getComputedStyle(document.querySelector('aside.context')).backgroundColor !== was, wasGround);
  await tabsDrawn(panel, `the Channel tab chosen, in the ${other}`);
  await page.screenshot({ path: `${out}/docsed-0-settings-${other}${looked}.png` });
  await litAs(page, scheme);
  await panel.getByRole('tab', { name: 'Docs' }).click();
  await panel.locator('.channel-docs').waitFor();
  const theirs = await opened('grace@example.com');
  let reread = 0;
  // A page loaded again, and not the address bar following the tab showing, which the browser calls a navigation too.
  theirs.on('domcontentloaded', () => (reread += 1));
  const theirPanel = await docsOf(theirs);
  reread = 0;
  await page.screenshot({ path: `${out}/docsed-0-empty${looked}.png` });

  // A document of the library and a file of the project, added from the list of the workspace's projects alone.
  await panel.getByRole('button', { name: 'Add a document', exact: true }).click();
  const adding = panel.getByRole('group', { name: 'Add a document' });
  await adding.locator('[data-offer="shop-docs:overview"]').waitFor();
  const projectsOffered = await adding.locator('.channel-docs-of').evaluateAll((all) => all.map((one) => one.dataset.project));
  expect(`the projects of the channel's workspace alone to be offered, got ${projectsOffered}`, projectsOffered.join() === 'shop');
  expect('the find box to have the keyboard', await adding.getByLabel('Find a document to add').evaluate((field) => field === document.activeElement));
  const offeredHeads = await adding.locator('h4').allInnerTexts();
  expect(`the library's documents and the project's files each under its heading, got ${offeredHeads}`, offeredHeads.join() === 'Library,Files');
  expect('a document of the library not offered again as a file', (await adding.locator('[data-offer="shop/docs/overview.md"]').count()) === 0);
  await page.screenshot({ path: `${out}/docsed-1-adding${looked}.png` });
  await adding.locator('[data-offer="shop-docs:overview"]').click();
  await until('the document to be listed', () => lines(panel), ['Shop: overview | shop-docs:overview']);
  expect('what is listed not to be offered again', (await adding.locator('[data-offer="shop-docs:overview"]').count()) === 0);
  await adding.getByLabel('Find a document to add').fill('quince notes');
  await adding.getByText('Nothing found says that.').waitFor();
  await adding.getByLabel('Find a document to add').fill('notes');
  await until('one file found for what was typed', () => adding.locator('[data-offer]').evaluateAll((all) => all.map((one) => one.dataset.offer)), ['shop/notes.md']);
  await adding.locator('[data-offer="shop/notes.md"]').click();
  await until('both to be listed', () => lines(panel), ['Shop: overview | shop-docs:overview', 'notes.md | shop/notes.md']);
  await adding.getByRole('button', { name: 'Done', exact: true }).click();
  await adding.waitFor({ state: 'detached' });
  // The other page follows, never read again.
  await until('the second page to follow', () => lines(theirPanel), ['Shop: overview | shop-docs:overview', 'notes.md | shop/notes.md']);
  expect('the second page never read again', reread === 0);
  await page.screenshot({ path: `${out}/docsed-2-listed${looked}.png` });
  await theirs.screenshot({ path: `${out}/docsed-2-listed-theirs${looked}.png` });

  // One read in place, with the way back and the way to open it as a tab.
  await panel.locator('.channel-doc[data-doc="shop/notes.md"] button.context-row').click();
  await panel.frameLocator('iframe.page-frame').getByText('The quince is ripe').waitFor();
  await page.screenshot({ path: `${out}/docsed-3-read${looked}.png` });
  await panel.getByRole('button', { name: 'Open as a tab', exact: true }).click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'notes.md' }).first().waitFor();
  await page.locator('.tab', { hasText: 'pantry' }).first().click();
  await panel.waitFor();
  if ((await panel.getByRole('button', { name: 'Back to the documents' }).count()) > 0) await panel.getByRole('button', { name: 'Back to the documents' }).click();
  await panel.locator('.channel-doc').first().waitFor();

  // The file goes from a terminal: shown again, its line says it is gone, and it is not left out.
  await cli('file', 'remove', 'shop', 'notes.md');
  await panel.getByRole('tab', { name: 'Board' }).click();
  await panel.getByRole('tab', { name: 'Docs' }).click();
  await until('the file to be said to be gone', () => lines(panel), ['Shop: overview | shop-docs:overview', 'Gone | shop/notes.md']);
  await page.screenshot({ path: `${out}/docsed-4-gone${looked}.png` });
  // Taken away from its line: gone on the other page too.
  await panel.getByRole('button', { name: 'Take shop/notes.md out of this channel\'s documents' }).click();
  await until('the line to be taken away', () => lines(panel), ['Shop: overview | shop-docs:overview']);
  await until('the other page to follow', () => lines(theirPanel), ['Shop: overview | shop-docs:overview']);
  expect('the second page still never read again', reread === 0);

  // Somebody who may not write in the channel is shown the list and offered nothing to change it.
  const member = ['projects', 'channels:read', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search', 'inbox'];
  const less = await opened('grace@example.com', member);
  const lessPanel = await docsOf(less);
  await until('the list to be shown to whoever may read the channel', () => lines(lessPanel), ['Shop: overview | shop-docs:overview']);
  expect('no way to add one', (await lessPanel.getByRole('button', { name: 'Add a document' }).count()) === 0);
  expect('no way to take one away', (await lessPanel.getByRole('button', { name: /^Take / }).count()) === 0);
  await less.screenshot({ path: `${out}/docsed-5-not-theirs${looked}.png` });

  // ---- by the boxes: the list, the list a document is added from, and one read in place, at three widths and a phone's
  await panel.getByRole('button', { name: 'Add a document', exact: true }).click();
  for (const [width, height] of [
    [1280, 860],
    [900, 800],
    [640, 800],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    if ((await panel.count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
    await toolsAtRest(page).waitFor();
    await panel.locator('.channel-docs').waitFor();
    if ((await panel.getByRole('group', { name: 'Add a document' }).count()) === 0) await panel.getByRole('button', { name: 'Add a document', exact: true }).click();
    await panel.getByRole('group', { name: 'Add a document' }).locator('[data-offer]').first().waitFor();
    const wrong = await pageCrowded(page, 'aside.context');
    expect(`nothing of the Docs tab over anything else or outside the window at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
    const past = await panel.evaluate((aside) => {
      const rim = aside.getBoundingClientRect();
      const outside = [...aside.querySelectorAll('.channel-docs *')].filter((one) => {
        const box = one.getBoundingClientRect();
        return box.width > 0 && (box.left < rim.left - 0.5 || box.right > rim.right + 0.5);
      });
      const body = aside.querySelector('.context-body');
      return { outside: outside.map((one) => one.className || one.tagName).slice(0, 5), sideways: body.scrollWidth - body.clientWidth, page: document.documentElement.scrollWidth - window.innerWidth };
    });
    expect(`nothing of the tab outside the panel or scrolling sideways at ${width}, got ${JSON.stringify(past)}`, past.outside.length === 0 && past.sideways <= 1 && past.page <= 0);
    await page.screenshot({ path: `${out}/docsed-6-at-${width}${looked}.png` });
    if (width === 390) {
      await tabRow(page, panel, "a phone's width");
      await tabsDrawn(panel, "a phone's width");
    }
  }

  // Dragged as narrow as it goes, at the usual width of a window.
  await page.setViewportSize({ width: 1280, height: 860 });
  if ((await panel.count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  await panel.locator('.channel-docs').waitFor();
  const edge = await panel.getByRole('separator').boundingBox();
  await page.mouse.move(edge.x + edge.width / 2, edge.y + 200);
  await page.mouse.down();
  await page.mouse.move(edge.x + edge.width / 2 + 600, edge.y + 200, { steps: 20 });
  await page.mouse.up();
  const narrowest = Math.round((await panel.boundingBox()).width);
  expect(`the panel dragged as narrow as it goes, 240 wide, and it is ${narrowest}`, narrowest <= 242);
  await tabRow(page, panel, 'the narrowest');
  await tabsDrawn(panel, 'the narrowest');
  await page.screenshot({ path: `${out}/docsed-7-narrowest${looked}.png` });

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
