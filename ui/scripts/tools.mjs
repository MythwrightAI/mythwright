// The Tools panel beside a channel, and who can be asked there: what is
// said of each under its name, whole; the card over a long name with how
// the agent stands beside what it is; what a member may ask of the
// factory, by area, changed for an agent of a fleet's row where its row
// is changed; the panel's edge dragged, with what the drag costs
// counted; everybody of the channel a card in the Channel tab, with their
// picture and what they are, by the boxes at three widths, and the bot's
// pressed where it is read about; and the Board tab with two workspaces, offering the boards of
// the one the side panel is of, beginning on the channel's own; and the
// panel coming in and going out as the drawer does (`toolsslid.mjs`).
//
//   DAEMON=../target/debug/factory node scripts/tools.mjs [out-dir]
//
// A project with a long name, so that its agents have long names, with
// its board and its fleet as they are first made. The environment
// variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';
import { toolsAtRest } from './drawer.mjs';
import { toolsSlid } from './toolsslid.mjs';
import { toolsCard } from './toolscard.mjs';

/** Turns the side panel to a view on the rail, unless it is showing already: Channels with every branch unfolded, as each starts folded (`folded.mjs`). */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
  if (name === 'Channels') await unfoldChannels(page);
}

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'tools-data');
const home = join(out, 'tools-home');

/** The file called `name` under `dir` whose folder is called `within`, wherever it is. */
async function found(dir, within, name) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const at = join(dir, entry.name);
    if (entry.isDirectory()) {
      const deeper = await found(at, within, name).catch(() => null);
      if (deeper) return deeper;
    } else if (entry.name === name && dir.endsWith(`/${within}`)) return at;
  }
  return null;
}

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// The daemon and the command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// `UI=<folder>` has the daemon serve another build of the client: the one from before a change, to measure against.
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', ...(process.env.UI ? ['--ui', resolve(process.env.UI)] : [])], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const PROJECT = 'riverside-workshop-annex';
const ARCHITECT = `${PROJECT}-architect-1`;
const WORKER = `${PROJECT}-worker-1`;

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  // What a project starts with is what this is about: its board, and its fleet with its agents in its channel.
  await cli('project', 'create', 'Riverside Workshop Annex');

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
  // A request the page lost is said with why (the event stream is let go as a page closes): once, under load, sending what the drag is measured over failed to fetch.
  page.on('requestfailed', (asked) => asked.failure()?.errorText !== 'net::ERR_ABORTED' && console.log(`lost ${asked.method()} ${asked.url().replace(/by=\w+/, 'by=…')}: ${asked.failure()?.errorText}`));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const until = async (what, ask) => {
    let last;
    for (let tries = 0; tries < 150; tries += 1) {
      last = await ask();
      if (last === true) return;
      await page.waitForTimeout(100);
    }
    throw new Error(`expected ${what}, and it is ${JSON.stringify(last)}`);
  };

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await side(page, 'Channels');
  await page.locator('.nav').getByText(PROJECT, { exact: true }).click();
  await page.getByRole('heading', { name: `# ${PROJECT}` }).waitFor();
  // With the side panel and the Tools panel both open, the channel is at its narrowest.
  const panel = page.getByRole('complementary', { name: 'Tools' });
  if ((await panel.count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  // It comes in and goes out as the drawer does (`toolsslid.mjs`).
  await toolsSlid({ page, out, expect, cli, channel: PROJECT });

  // The panel's edge dragged 300 across in 60 steps, in a channel with two thousand things said in it and the panel on a board of
  // three hundred cards: what was counted while it was held, how wide the panel was before, while held, and where, and
  // every frame of the drag, against what a person sees as a hitch.
  // `SAID=` and `CARDS=` say other figures, for looking closely at one.
  const SAID = Number(process.env.SAID ?? 2000);
  const CARDS = Number(process.env.CARDS ?? 300);
  const across = async () => Math.round((await panel.boundingBox()).width);
  const dragged = async () => {
    // Nobody answers what is about to be said: an agent at work in the channel would draw it again for reasons of its own.
    await cli('agent', 'stop', ARCHITECT).catch(() => {});
    await cli('agent', 'stop', WORKER).catch(() => {});
    await page.evaluate(
      async ({ name, project, said, cards }) => {
        const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
        const channels = await (await fetch('/api/channels', { headers: mine })).json();
        const channel = channels.channels.find((one) => one.name === name);
        const send = async (path, body) => {
          const sent = await fetch(path, { method: 'POST', headers: mine, body: JSON.stringify(body) });
          if (!sent.ok) throw new Error(await sent.text());
        };
        // In tens at a time: it is how much is on the page that is measured, not how it came.
        for (let at = 0; at < cards; at += 10)
          await Promise.all(Array.from({ length: Math.min(10, cards - at) }, (_, one) => send(`/api/board/${project}/items`, { title: `A card waiting its turn, number ${at + one}`, status: 'backlog' })));
        for (let at = 0; at < said; at += 10)
          await Promise.all(Array.from({ length: Math.min(10, said - at) }, (_, one) => send(`/api/channels/${channel.id}/messages`, { body: `Something said, number ${at + one}, with a few more words so that it wraps when the channel is narrow.` })));
      },
      { name: PROJECT, project: PROJECT, said: SAID, cards: CARDS },
    );
    // What came last of the tens is any of the last ten.
    await page.waitForFunction((said) => document.querySelectorAll('.channel > .split .messages > .message').length >= said, SAID, { timeout: 300_000 });
    await panel.getByRole('tab', { name: 'Board' }).click();
    // A window wide enough that the panel has room for the whole of the drag.
    await page.setViewportSize({ width: 1600, height: 900 });
    const edge = panel.getByRole('separator');
    // The channel is at rest before the edge is taken hold of: what was just said there has been drawn.
    for (let last = -1, still = 0, tries = 0; still < 3 && tries < 60; tries += 1) {
      const now = await page.evaluate(() => window.__drawn?.channel ?? 0);
      still = now === last ? still + 1 : 0;
      last = now;
      await page.waitForTimeout(200);
    }
    const before = await across();
    const grip = await edge.boundingBox();
    const [x, y] = [grip.x + grip.width / 2, grip.y + 300];
    await page.mouse.move(x, y);
    await page.evaluate(() => {
      const seen = { frames: 0, slowest: 0, slow: 0, began: performance.now(), last: performance.now(), longest: 0, going: true, gaps: [] };
      window.__seen = seen;
      const tick = (now) => {
        seen.frames += 1;
        seen.slowest = Math.max(seen.slowest, now - seen.last);
        seen.gaps.push(now - seen.last);
        // A frame that took more than two of the screen's own is one a person sees as a hitch.
        if (now - seen.last > 34) seen.slow += 1;
        seen.last = now;
        if (seen.going) requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
      new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) seen.longest = Math.max(seen.longest, entry.duration);
      }).observe({ type: 'longtask', buffered: false });
      seen.drawn = window.__drawn?.channel ?? 0;
    });
    // Where the last thing said ends, against the foot of the list: it is to stay there, held or not.
    const foot = () =>
      page.evaluate(() => {
        const list = document.querySelector('.channel > .split .messages');
        const last = list.lastElementChild.getBoundingClientRect();
        return Math.round(list.getBoundingClientRect().bottom - last.bottom);
      });
    const footBefore = await foot();
    // `TRACE=<file>` keeps a trace of the browser's own work through the drag, for finding what a slow frame was spent on.
    if (process.env.TRACE) await browser.startTracing(page, { path: resolve(process.env.TRACE), categories: ['devtools.timeline', 'disabled-by-default-devtools.timeline', 'disabled-by-default-devtools.timeline.frame', 'blink', 'v8.execute', 'toplevel'] });
    await page.mouse.down();
    await page.mouse.move(x - 300, y, { steps: 60 });
    const footHeld = await foot();
    // While it is still held: what the drag itself cost.
    const cost = await page.evaluate(() => {
      const seen = window.__seen;
      const gaps = [...seen.gaps].sort((a, b) => a - b);
      return { drawn: (window.__drawn?.channel ?? 0) - seen.drawn, frames: seen.frames, slow: seen.slow, took: Math.round(performance.now() - seen.began), slowest: Math.round(seen.slowest), middle: Math.round(gaps[Math.floor(gaps.length / 2)] ?? 0), longest: Math.round(seen.longest), messages: document.querySelectorAll('.channel > .split .messages > .message').length, said: [...document.querySelectorAll('.channel > .split .messages > .message')].filter((one) => one.textContent.includes('Something said, number')).length, cards: document.querySelectorAll('.context .context-row').length, cursor: getComputedStyle(document.querySelector('.channel .message')).cursor, select: getComputedStyle(document.querySelector('.channel .message')).userSelect };
    });
    const held = await across();
    await page.mouse.up();
    if (process.env.TRACE) await browser.stopTracing();
    await page.evaluate(() => (window.__seen.going = false));
    console.log(`the edge dragged 300 across in 60 steps, ${cost.messages} messages drawn (${cost.said} of them said here) and ${cost.cards} cards listed beside them: the channel drawn again ${cost.drawn} times while it was held; ${cost.took} ms in all, ${cost.frames} frames of which ${cost.slow} took over 34 ms, the middle one ${cost.middle} ms, the slowest ${cost.slowest} ms, the longest task ${cost.longest} ms`);
    const footAfter = await foot();
    if (Math.abs(footHeld - footBefore) > 2 || Math.abs(footAfter - footBefore) > 2) throw new Error(`expected what was last said to stay at the foot of the channel while the edge is held and after, and it was ${footBefore}, ${footHeld} and ${footAfter} from it`);
    return { cost, before, held };
  };
  // `MEASURE=1` only says what the drag cost, for a build from before the edge was paced (`UI=<its folder>`).
  if (process.env.MEASURE) {
    await dragged();
    throw new Error('measured only');
  }

  // What is said of each who can be asked is under its name and whole: beside the name it was cut off where the list is narrow.
  const whole = (menu) =>
    menu.evaluate((list) => {
      const rim = list.getBoundingClientRect();
      return [...list.querySelectorAll('.asked-words')].map((words) => {
        const name = words.querySelector('.asked-name');
        const under = words.querySelector('.asked-under');
        const [top, below] = [name.getBoundingClientRect(), under.getBoundingClientRect()];
        return {
          says: `${name.textContent} / ${under.textContent}`,
          below: below.top >= top.bottom - 1,
          fits: [name, under].every((one) => one.scrollWidth <= one.clientWidth + 1 && one.getBoundingClientRect().right <= rim.right + 0.5 && getComputedStyle(one).textOverflow !== 'ellipsis'),
        };
      });
    });
  const good = (rows) => rows.length > 0 && rows.every((one) => one.below && one.fits);
  const asking = page.locator('.channel > .split .asking-button');
  const askable = page.getByRole('menu', { name: 'Agents and bots in this channel' });
  await asking.click();
  await askable.waitFor();
  const asked = await whole(askable);
  expect(`each who can be asked to have what is said of it under its name, none of it cut off, got ${JSON.stringify(asked)}`, good(asked) && asked.length === 3);
  // Each agent of the fleet is in the channel it reports in, and so can be asked there.
  expect(`each agent to be said to be idle and the factory's bot to take commands, got ${asked.map((one) => one.says).join(' | ')}`, asked[0].says.startsWith(`${ARCHITECT} / idle`) && asked[1].says.startsWith(`${WORKER} / idle`) && asked[2].says === 'conductor / bot · takes commands');
  expect('the list to open over the button, moving nothing', (await askable.boundingBox()).y + (await askable.boundingBox()).height <= (await asking.boundingBox()).y);
  await page.screenshot({ path: `${out}/tools-1-asking.png` });
  await asking.click();
  await askable.waitFor({ state: 'detached' });

  // The same where an @ is typed: the keys still move through it by member, and Tab puts the marked one in.
  const box = page.getByLabel('Message');
  await box.click();
  await page.keyboard.type('@');
  const offered = page.locator('.mentions.offered[aria-label="Mention"]');
  await offered.waitFor();
  const typed = await whole(offered);
  expect(`each offered to have what is said of it under its name, none of it cut off, got ${JSON.stringify(typed)}`, good(typed) && typed.length === 3);
  const reach = await offered.boundingBox();
  expect('the list to be no taller than the room over the box', reach.y >= 0 && reach.y + reach.height <= (await page.locator('.channel > .split .composer-box').boundingBox()).y + 1);
  await page.screenshot({ path: `${out}/tools-2-mention.png` });
  await page.keyboard.press('ArrowDown');
  expect('the arrows to move through the list by member', (await offered.locator('[role=option].marked .asked-name').innerText()) === `@${WORKER}`);
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('Tab');
  expect(`Tab to put the marked one in, got "${await box.inputValue()}"`, (await box.inputValue()) === `@${ARCHITECT} `);
  await page.keyboard.type('hello');
  await page.keyboard.press('Enter');
  const said = page.locator('.channel > .split .message .who.agent', { hasText: ARCHITECT }).first();
  await said.waitFor();
  // The card is over the picture of whoever said something, and not over their name, which is words.
  const picture = page.locator('.channel > .split .message', { has: page.locator('.who.agent', { hasText: ARCHITECT }) }).first().locator('.avatar');

  // The card about one with a long name: the name has its own line, how the agent stands is under it beside what it is, and nothing is cut off at the card's edge.
  const card = page.locator('.who-card');
  const kept = () =>
    card.evaluate((one) => {
      const rim = one.getBoundingClientRect();
      const name = one.querySelector('.who-name').getBoundingClientRect();
      const badge = one.querySelector('.who-what .badge');
      const what = one.querySelector('.who-what .muted').getBoundingClientRect();
      const out = [...one.querySelectorAll('*')].filter((each) => each.getBoundingClientRect().right > rim.right + 0.5 || each.getBoundingClientRect().left < rim.left - 0.5).map((each) => String(each.className || each.tagName));
      const clipped = [...one.querySelectorAll('.who-name, .who-what, .badge, dd')].filter((each) => each.scrollWidth > each.clientWidth + 1).map((each) => String(each.className || each.tagName));
      const at = badge?.getBoundingClientRect();
      return { out, clipped, under: at ? at.top >= name.bottom - 1 : null, beside: at ? Math.abs(at.top + at.height / 2 - (what.top + what.height / 2)) < 6 : null, says: badge?.textContent.trim() ?? null };
    });
  const pointAt = async () => {
    await page.mouse.move(10, 400);
    await card.waitFor({ state: 'detached' });
    await picture.hover();
    await card.waitFor();
  };
  const sound = (seen) => seen.out.length === 0 && seen.clipped.length === 0 && seen.under === true && seen.beside === true;
  await said.hover();
  await page.waitForTimeout(400);
  expect('no card from pointing at the name', (await card.count()) === 0);
  await picture.hover();
  await card.waitFor();
  const idle = await kept();
  expect(`an idle agent's card to keep everything inside it, with how it stands beside what it is under its name, got ${JSON.stringify(idle)}`, sound(idle) && idle.says === 'idle');
  expect('the whole of a long name to be on the card', (await card.locator('.who-name').innerText()) === ARCHITECT);
  await page.screenshot({ path: `${out}/tools-3-card.png` });
  await page.mouse.move(10, 400);
  await card.waitFor({ state: 'detached' });

  // What it may ask of the factory, by area, where it is read about (its picture pressed); changed there for an agent of a fleet's row.
  await picture.click();
  const where = panel.getByRole('navigation', { name: 'Where this is' });
  await where.getByText(ARCHITECT, { exact: true }).waitFor();
  const table = panel.locator('.holds-table');
  await table.waitFor();
  const lines = async () =>
    Object.fromEntries(
      await table.locator('tr').evaluateAll((rows) =>
        rows.map((row) => {
          const choice = row.querySelector('select');
          return [row.querySelector('th').textContent.trim(), choice ? choice.options[choice.selectedIndex].textContent.trim() : row.querySelector('td').textContent.trim()];
        }),
      ),
    );
  await until('the architect to be found in its fleet\'s row, with a choice for each area', async () => (await table.locator('select').count()) > 5 || (await table.locator('select').count()));
  const usual = await lines();
  expect(
    `what an agent usually holds to be said by area, with what is not held as plainly there, got ${JSON.stringify(usual)}`,
    usual.board === 'read and change' && usual.blueprints === 'read' && usual.fleet === 'read' && usual.search === 'yes' && usual.shell === 'nothing' && usual.admin === 'nothing',
  );
  expect(`it to say that this is the usual, got "${await panel.locator('.holds-where').innerText()}"`, /what an agent usually holds/.test(await panel.locator('.holds-where').innerText()));
  // Every choice one width, whatever it says, so that they stand in an even column; and each wholly in the panel.
  const choices = await table.locator('select').evaluateAll((all) => all.map((one) => one.getBoundingClientRect()).map((box) => [Math.round(box.left), Math.round(box.width), Math.round(box.right)]));
  const edge = await panel.evaluate((box) => Math.round(box.getBoundingClientRect().right));
  expect(`every choice as wide as every other and in line, got ${JSON.stringify(choices)}`, new Set(choices.map(([left, width]) => `${left}:${width}`)).size === 1);
  expect(`every choice inside the panel (its edge at ${edge}), got ${JSON.stringify(choices)}`, choices.every(([, , right]) => right <= edge));
  await page.screenshot({ path: `${out}/tools-4-holds.png` });
  // The board from "read and change" to "read": the row says the whole list, the fleet follows, and the agent's own key is refused a card's move.
  await table.getByLabel('What it may do with board').selectOption({ label: 'read' });
  const row = async () => JSON.parse((await cli('record', 'list', PROJECT, 'agents', '--json')).stdout).records.find((one) => one.data.title === 'architect');
  await until('the row to say what the architect holds, the board to read', async () => {
    const holds = (await row()).data.holds ?? [];
    return (holds.includes('board:read') && !holds.includes('board') && holds.includes('projects') && holds.includes('blueprints:read')) || holds;
  });
  await until('the agent to hold what its row says', async () => {
    const slot = await page.evaluate(async (name) => {
      const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
      const all = await (await fetch('/api/agents', { headers: mine })).json();
      return (all.agents ?? all.slots ?? all).find((one) => one.name === name)?.scopes ?? [];
    }, ARCHITECT);
    return (slot.includes('board:read') && !slot.includes('board')) || slot;
  });
  await until('the page to say what its row says', async () => (await lines()).board === 'read' || (await lines()).board);
  expect(`it to say that its row says so, got "${await panel.locator('.holds-where').innerText()}"`, /As its row in the fleet's agents says/.test(await panel.locator('.holds-where').innerText()));
  const made = JSON.parse((await cli('board', 'add', PROJECT, 'A shelf to put up', '--json')).stdout);
  const item = made.id ?? made.item?.id;
  const token = (await readFile(await found(data, ARCHITECT, 'token'), 'utf8')).trim();
  const reads = await run(binary, ['board', PROJECT, '--url', url, '--token', token], { cwd: repo, env });
  expect('the agent still to read the board', reads.stdout.includes('A shelf to put up') || reads.stdout.includes(item));
  const moved = await run(binary, ['board', 'move', item, 'ready', '--url', url, '--token', token], { cwd: repo, env }).then(
    () => 'moved',
    (refused) => String(refused.stderr ?? refused.message),
  );
  expect(`the agent's key to be refused a card's move, got "${moved}"`, /board:write/.test(moved));
  // Nobody is offered to give what they do not hold; the owner holds everything, so nothing is closed to them.
  expect('nothing to be closed to the owner', (await table.locator('option:disabled').count()) === 0);
  await page.screenshot({ path: `${out}/tools-4-holds-changed.png` });

  // An agent that is in no row of a fleet, added by hand: what it holds is its own, and whoever may change agents changes a line of it here too.
  await cli('agent', 'add', 'helper', '--channel', PROJECT, '--instructions', 'Be brief.');
  const helper = async () => {
    const listed = JSON.parse((await cli('agent', 'list', '--json')).stdout);
    return (listed.agents ?? listed).find((one) => one.name === 'helper');
  };
  const added = await helper();
  await where.getByRole('button', { name: 'Channel', exact: true }).click();
  await panel.locator('.context-agent', { hasText: 'helper' }).click();
  await where.getByText('helper', { exact: true }).waitFor();
  await table.waitFor();
  await until('a choice for each area of what an agent in no row holds', async () => (await table.locator('select').count()) > 5 || (await table.locator('select').count()));
  const apart = await lines();
  expect(`one added by hand to hold what an agent usually does, got ${JSON.stringify(apart)}`, apart.board === 'read and change' && apart.blueprints === 'read' && apart.shell === 'nothing');
  const whereSaid = () => panel.locator('.holds-where').innerText();
  expect(
    `it to say that a line is changed here and the command that does the same, got "${await whereSaid()}"`,
    /in no row of a fleet/.test(await whereSaid()) && /change a line here/.test(await whereSaid()) && (await whereSaid()).includes('agent add helper --scope projects') && !(await whereSaid()).includes('--scope …'),
  );
  await page.screenshot({ path: `${out}/tools-4-holds-apart.png` });
  // Given leave to apply blueprints: the agent holds it, the line reads so, the command under it says so, and nothing else of the agent is another thing.
  await table.getByLabel('What it may do with blueprints').selectOption({ label: 'read and run' });
  await until('the agent to hold leave to apply blueprints', async () => {
    const holds = (await helper()).scopes;
    return (holds.includes('blueprints') && !holds.includes('blueprints:read') && holds.includes('board')) || holds;
  });
  await until('the line to read so', async () => (await lines()).blueprints === 'read and run' || (await lines()).blueprints);
  await until('the command under it to say what it now holds', async () => (await whereSaid()).includes('--scope blueprints') && !(await whereSaid()).includes('--scope blueprints:read'));
  const given = await helper();
  const rest = ({ scopes, updated, ...others }) => JSON.stringify(others);
  expect(`nothing of the agent but what it holds to have changed, got ${rest(given)} from ${rest(added)}`, rest(given) === rest(added));
  expect('nothing to be closed to the owner there either', (await table.locator('option:disabled').count()) === 0);
  await page.screenshot({ path: `${out}/tools-4-holds-apart-given.png` });

  // Everybody of the channel is a card in the Channel tab: their picture, as it is drawn beside what they say, and
  // beside it who and what they are, whole and inside the card, and nothing over anything else, at three widths.
  await where.getByRole('button', { name: 'Channel', exact: true }).click();
  const members = panel.locator('.context-members');
  await members.locator('.context-member.bot').waitFor();
  const cards = () =>
    members.evaluate((list) => {
      const rim = list.closest('.context-body').getBoundingClientRect();
      const box = (one) => one.getBoundingClientRect();
      const meets = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      return [...list.querySelectorAll('.context-member')].map((card) => {
        const at = box(card);
        const face = card.querySelector(':scope > .avatar');
        const name = card.querySelector('.context-name');
        const what = card.querySelector('.context-member-says > .row > .faint');
        const parts = [face, ...card.querySelectorAll('.context-member-says *')].filter((one) => one && box(one).width > 0);
        return {
          kind: card.dataset.kind,
          name: name.textContent,
          what: what?.textContent ?? null,
          face: !!face && face.textContent.trim().length > 0,
          inside: at.left >= rim.left - 0.5 && at.right <= rim.right + 0.5 && parts.every((one) => box(one).left >= at.left - 0.5 && box(one).right <= at.right + 0.5),
          faceClear: !!face && [name, what].filter(Boolean).every((one) => !meets(box(face), box(one))),
          nameClear: !what || !meets(box(name), box(what)),
          whatWhole: !what || what.scrollWidth <= what.clientWidth + 1,
        };
      });
    });
  for (const width of [1600, 1280, 900]) {
    await page.setViewportSize({ width, height: 900 });
    await toolsAtRest(page).waitFor();
    const seen = await cards();
    const sound = seen.length > 0 && seen.every((one) => one.face && one.inside && one.faceClear && one.nameClear && one.whatWhole);
    expect(`at ${width} wide, everybody a card with their picture, whole, inside and over nothing, got ${JSON.stringify(seen)}`, sound);
    expect(
      `the person to say person and you, and the bot to say it is the factory's, got ${JSON.stringify(seen.map(({ kind, what }) => [kind, what]))}`,
      seen.some((one) => one.kind === 'person' && one.what === 'person · you') && seen.some((one) => one.kind === 'bot' && one.what === "the factory's bot") && seen.filter((one) => one.kind === 'agent').length >= 2,
    );
    await page.screenshot({ path: `${out}/tools-4-members-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // The bot's card, pressed, is where it is read about: what it did lately among it.
  await members.locator('.context-member.bot').click();
  await panel.getByRole('heading', { name: 'What it did lately' }).waitFor();
  await where.getByRole('button', { name: 'Channel', exact: true }).click();

  // A person: what you hold yourself, by area, and nothing there to choose.
  await panel.locator('.context-member.user', { hasText: 'Ada' }).click();
  await where.getByText('Ada', { exact: true }).waitFor();
  const mine = await lines();
  expect(`the owner to be shown holding everything, by area, got ${JSON.stringify(mine)}`, mine.board === 'read and change' && mine.admin === 'yes' && mine.blueprints === 'read and run' && !Object.values(mine).includes('nothing'));
  expect('nothing to choose of what a person holds', (await table.locator('select').count()) === 0);
  await page.screenshot({ path: `${out}/tools-5-person.png` });

  // An agent that was stopped: its card says so in the same place, and still keeps everything inside it.
  await cli('agent', 'stop', ARCHITECT);
  await until('the agent to be stopped', async () => {
    await pointAt();
    return (await kept()).says === 'stopped' || (await kept()).says;
  });
  const stopped = await kept();
  expect(`a stopped agent's card to keep everything inside it, got ${JSON.stringify(stopped)}`, sound(stopped));
  await page.screenshot({ path: `${out}/tools-3-card-stopped.png` });
  await page.mouse.move(10, 400);
  await card.waitFor({ state: 'detached' });

  // A card of the board opened in the panel: the card's own form, as the drawer has it (`toolscard.mjs`). Somebody
  // who may read the board and not change a card is a page told so (`/api/me` saying what they hold), signed in as Ada.
  const signedInLacking = async (holds) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
    const less = await context.newPage();
    less.on('pageerror', (err) => problems.push(err.message));
    await less.route('**/api/me', async (route) => {
      const answer = await route.fetch();
      const me = await answer.json();
      if (!me.principal) return route.fulfill({ response: answer });
      me.principal.scopes = holds;
      await route.fulfill({ response: answer, json: me });
    });
    await less.goto(url);
    await less.getByLabel('Email').fill('ada@example.com');
    await less.getByLabel('Password').fill('correct horse');
    await less.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await less.locator('.rail').waitFor();
    await side(less, 'Channels');
    await less.locator('.nav').getByText(PROJECT, { exact: true }).first().click();
    await less.getByRole('heading', { name: `# ${PROJECT}` }).waitFor();
    if ((await less.getByRole('complementary', { name: 'Tools' }).count()) === 0) await less.getByRole('button', { name: 'Expand the tools panel' }).click();
    await toolsAtRest(less).waitFor();
    return less;
  };
  await toolsCard({ page, out, expect, cli, url, until, title: 'A shelf to put up', signedInLacking });

  // The panel's edge, dragged in a channel with a good deal said in it and the panel on its board: what the drag costs is counted,
  // the channel is not drawn again while the edge is held, and what it is let go at is how wide the panel is, and stays.
  const { cost, before, held } = await dragged();
  expect(`the channel not to be drawn again while the edge is held, and it was ${cost.drawn} times`, cost.drawn === 0);
  // Every frame of the drag to a budget. With the two thousand messages laid out whole, the middle frame took 33 ms and
  // the slowest 633 on this machine, busy; laid out only in sight, 17 and 133 to 150, some sixty frames of 135 over 34 ms. A trace
  // of the drag (`TRACE=<file>`) said where those went: each frame the width was handed down to every message as a property they
  // inherit, and their style worked out again (`--context-width` is now the channel's alone, `@property` in `style.css`), and
  // the messages were a column of flexible boxes, which a width lays out at twice the cost of blocks. Without both: 17 and 50,
  // eight frames over 34 ms. What is left is the browser's own: laying out each message's box at the new width (about 25 ms a
  // frame under a trace) and which of them are in sight (about 5). A frame of 25 ms is a frame and a half of a 60 Hz screen.
  expect(`the drag to keep to the screen's pace, its middle frame ${cost.middle} ms (at most 25) and its slowest ${cost.slowest} ms (at most 150)`, cost.middle <= 25 && cost.slowest <= 150);
  expect(`the pointer to keep its shape over whatever it crosses while the edge is held, and nothing to be selected, got ${cost.cursor} and ${cost.select}`, cost.cursor === 'col-resize' && cost.select === 'none');
  expect(`the panel to follow the edge while it is held, from ${before} to ${held}`, Math.abs(held - (before + 300)) <= 2);
  expect(`the panel to be as wide as it was dragged to when the edge is let go, got ${await across()}`, Math.abs((await across()) - (before + 300)) <= 2);
  expect('the pointer to have its own shape again once the edge is let go', (await page.evaluate(() => getComputedStyle(document.querySelector('.channel .message')).cursor)) !== 'col-resize');
  await page.screenshot({ path: `${out}/tools-6-dragged.png` });
  const left = await across();
  await page.reload();
  await toolsAtRest(page).waitFor();
  expect(`the panel to be as wide after the page is loaded again, ${left} and ${await across()}`, (await across()) === left);
  // The arrows and pressing twice do what they did.
  await panel.getByRole('separator').focus();
  await page.keyboard.press('ArrowRight');
  expect(`an arrow to move the edge a step, got ${await across()}`, (await across()) === left - 24);
  await panel.getByRole('separator').dblclick();
  expect(`pressing the edge twice to put the panel as it started, got ${await across()}`, (await across()) === 388);

  // The Board tab lists as many of a column's cards as the panel holds, says how many more, and that opens the board at the column.
  if ((await panel.getByRole('tab', { name: 'Board', selected: true }).count()) === 0) await panel.getByRole('tab', { name: 'Board' }).click();
  const backlogHere = panel.locator('.plan-column[data-column="backlog"]');
  const more = backlogHere.locator('.plan-more');
  await more.waitFor();
  const listedHere = await backlogHere.locator('.plan-row').count();
  const inBacklog = Number((await backlogHere.locator('h3 .faint').innerText()).trim());
  const saidMore = Number((await more.innerText()).match(/^and (\d+) more$/)?.[1]);
  expect(`the column's count whole in its head, its first cards listed and how many more said, got ${listedHere} listed, "${await more.innerText()}" and ${inBacklog} in its head`, inBacklog >= CARDS && listedHere < inBacklog && listedHere + saidMore === inBacklog);
  const room = await panel.evaluate((one) => one.getBoundingClientRect().height);
  expect(`as many listed as the panel holds, ${listedHere} in ${Math.round(room)} high`, listedHere * 28 >= room * 0.8 && listedHere * 28 <= room * 2);
  await page.screenshot({ path: `${out}/tools-6b-board-more.png` });
  const channelTab = await page.locator('.tab[aria-selected="true"]').first().getAttribute('data-key');
  await more.click();
  const backlog = page.locator('.board > .column[data-column="backlog"]');
  await backlog.waitFor();
  const seenAt = await backlog.evaluate((one) => {
    const box = one.getBoundingClientRect();
    const view = one.closest('.board').getBoundingClientRect();
    return box.left >= view.left - 1 && box.left < view.right;
  });
  expect('the board opened with the column in view', seenAt);
  await page.locator(`.tab[data-key="${channelTab}"]`).click();
  await toolsAtRest(page).waitFor();

  // The Board tab with two workspaces: it offers the boards of the workspace the side panel is of, beginning on the
  // channel's own, and none of the other workspace's.
  await cli('project', 'create', 'Market Stall', '--without', 'fleet', '--without', 'docs');
  await cli('workspace', 'create', 'Annex');
  await cli('project', 'create', 'Harbour Notes', '--without', 'fleet', '--without', 'docs');
  await cli('project', 'create', 'Lantern', '--without', 'fleet', '--without', 'docs');
  await cli('workspace', 'put', 'annex', 'harbour-notes');
  await cli('workspace', 'put', 'annex', 'lantern');
  await cli('channel', 'create', 'harbour-talk', '--workspace', 'annex');
  const whose = panel.getByLabel('Whose board');
  const offeredBoards = () => whose.locator('option').allInnerTexts();
  await panel.getByRole('tab', { name: 'Board' }).click();
  await until('the Board tab to offer the first workspace\'s boards, the channel\'s own first', async () => {
    const now = (await whose.count()) ? (await offeredBoards()).join(', ') : '';
    return now === 'Riverside Workshop Annex, Market Stall' || now;
  });
  expect(`it to begin on the channel's own, got ${await whose.inputValue()}`, (await whose.inputValue()) === PROJECT);
  await page.screenshot({ path: `${out}/tools-8-boards-first.png` });
  // The other workspace chosen at the foot of the Channels panel, and its own channel opened there.
  const channelsSide = page.getByRole('navigation', { name: 'Channels' });
  await channelsSide.locator('.space-foot .space-pick').click();
  const spaceMenu = page.getByRole('menu', { name: 'Workspace', exact: true });
  await spaceMenu.waitFor();
  await page.waitForFunction(() => document.activeElement?.closest('.row-menu') !== null && document.activeElement?.closest('.row-menu') !== undefined);
  await spaceMenu.getByRole('menuitemradio', { name: /^Annex(:|$)/ }).click();
  await spaceMenu.waitFor({ state: 'detached' });
  await unfoldChannels(page);
  await channelsSide.getByText('harbour-talk', { exact: true }).click();
  await page.getByRole('heading', { name: '# harbour-talk' }).waitFor();
  if ((await panel.count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  await panel.getByRole('tab', { name: 'Board' }).click();
  await until('the Board tab to offer the other workspace\'s boards and none of the first\'s', async () => {
    const now = (await whose.count()) ? (await offeredBoards()).join(', ') : '';
    return now === 'Harbour Notes, Lantern' || now;
  });
  expect(`a channel of no project to begin on the first, got ${await whose.inputValue()}`, (await whose.inputValue()) === 'harbour-notes');
  await page.screenshot({ path: `${out}/tools-9-boards-other.png` });

  // At a phone's width the list of who can be asked is still whole.
  await page.getByRole('button', { name: 'Collapse the tools panel' }).click();
  await panel.waitFor({ state: 'detached' });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload();
  const narrow = page.locator('.asking-button').last();
  await narrow.waitFor();
  await narrow.click();
  await askable.waitFor();
  const small = await whole(askable);
  expect(`each who can be asked to be whole at a phone's width, got ${JSON.stringify(small)}`, good(small));
  const rim = await askable.boundingBox();
  expect('the list to be inside the phone\'s screen', rim.x >= 0 && rim.x + rim.width <= 390);
  await page.screenshot({ path: `${out}/tools-7-phone-asking.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  // The fleet's agents are slots and no processes of their own are left
  // running by the built-in one; the daemon is all there is to stop.
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
