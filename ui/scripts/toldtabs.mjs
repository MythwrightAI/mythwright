// A told line's page in tabs, with what the daemon reads for whoever reads
// it (`kernel::parts`, `views/toldread.tsx`), which `reviewed.mjs` runs.
//
//   DAEMON=../target/debug/factory node scripts/toldtabs.mjs [out-dir]
//
// Ada owns the factory, Grace is a member, and a project Shop has a board
// whose `staging` holds a card, a table a script keeps its runs in, which
// says its rows are not written by hand, a table kept private, and a deploy
// applied three times under one key. A blueprint tells both of them with
// the cards in staging in a tab `To test`, the deploy's runs in
// `Deployment`, the runs of two commits and the deploy's measurements in
// `Insights` with the private table's rows beside them, and a link in
// `Links`. Ada's page, opened once and never read again: what the line is
// about under its words and over the row of tabs, the four tabs in the
// order they were said, the first showing and only its parts on the page;
// `Deployment` the newest run's graph over its runs, newest first, each with what it is a run of, how
// it went, when and how long, one pressed opening the run's page as a tab;
// the line's tab shown again with `Deployment` still showing; `Insights`
// the rows of those commits alone, a choice drawn as its tag and seconds
// as a length, one pressed opening its form in the drawer with the whole
// of what it printed, and the deploy's latest beside its median; Grace,
// whose `Insights` says the private table is kept from her in its place;
// a line told with parts in no tab drawing no row of tabs; and the page by
// its boxes with each tab showing, at 1280, 900 and 640 wide and a phone's,
// nothing over anything else, past the page or scrolling sideways. With
// `LOOK="size=largest density=compact"` the same at that look.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { drawerAtRest, sideSettled } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'toldtabs-data');
const home = join(out, 'toldtabs-home');
const hers = join(out, 'toldtabs-home-grace');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const grace = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: { ...env, HOME: hers, XDG_CONFIG_HOME: join(hers, 'config') } });

const TABLES = `blueprint: kept-runs
permissions: [records, projects:write]
steps:
  runs:
    do: collection.ensure
    with:
      name: test-runs
      by_hand: false
      fields:
        title: { type: text, required: true }
        result: { type: choice, of: [passed, failed] }
        seconds: { type: number }
        commit: { type: text }
        printed: { type: text }
  notes:
    do: collection.ensure
    with: { name: notes, reads: "private:notes", fields: { title: { type: text, required: true } } }
`;
const DEPLOY = `blueprint: deploy
steps:
  loaded: { do: emit, with: { topic: shop.loaded } }
`;
const TOLD = `blueprint: preview-tabs
permissions: [inbox]
steps:
  told:
    do: inbox.tell
    with:
      to: [applier, "@grace"]
      says: The preview is ready to try, in tabs.
      why: preview
      about: shop-1
      parts:
        - { kind: cards, column: staging, pass_to: done, tab: To test }
        - { kind: runs, key: deploy-dev, tab: Deployment }
        - kind: rows
          table: test-runs
          saying: { commit: { in: [a1b2c3d4, e5f6a7b8] } }
          columns: [title, result, seconds, commit]
          tab: Insights
        - { kind: rows, table: notes, saying: {}, tab: Insights }
        - { kind: measures, measures: [run], labels: { blueprint: deploy }, last: 5, tab: Insights }
        - { kind: links, links: ["http://127.0.0.1:7760/"], tab: Links }
`;
const PLAIN = `blueprint: preview-plain
permissions: [inbox]
steps:
  told:
    do: inbox.tell
    with: { to: applier, says: Nothing in tabs here., why: plain, parts: [{ kind: links, links: ["http://127.0.0.1:7760/"] }] }
`;
// The end of what a run printed: in its form, never in the table.
const END = 'END-OF-WHAT-IT-PRINTED';

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'grace@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await grace('register', '--email', 'grace@example.com', '--name', 'Grace', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', '--bare', 'Shop');
  await cli('apply', 'std/board', '--project', 'shop', '--param', 'columns=["ready","staging","done"]');
  await cli('board', 'add', 'shop', 'Fix the cart');
  await cli('board', 'move', 'shop-1', 'staging');
  const file = async (name, text) => {
    const at = join(home, name);
    await writeFile(at, text);
    return at;
  };
  await cli('apply', await file('tables.bp', TABLES), '--project', 'shop');
  for (const [title, result, seconds, commit] of [
    ['20261123-0901-a', 'passed', 96, 'a1b2c3d4'],
    ['20261123-0915-b', 'failed', 3700, 'e5f6a7b8'],
    ['20261123-0930-c', 'passed', 12, '99999999'],
  ]) {
    const printed = `${title} ${'test result: ok. '.repeat(40)}${END}`;
    await cli('record', 'add', 'shop', 'test-runs', `title=${title}`, `result=${result}`, `seconds=${seconds}`, `commit=${commit}`, `printed=${printed}`);
  }
  await cli('record', 'add', 'shop', 'notes', 'title=Kept to some');
  const deploy = await file('deploy.bp', DEPLOY);
  for (let times = 0; times < 3; times += 1) await cli('apply', deploy, '--project', 'shop', '--key', 'deploy-dev', '--force');
  await cli('apply', await file('told.bp', TOLD), '--project', 'shop');
  await cli('apply', await file('plain.bp', PLAIN), '--project', 'shop');

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
  const signedIn = async (email, how = { viewport: { width: 1280, height: 860 } }) => {
    const context = await browser.newContext({ ...how, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.setDefaultTimeout(30_000);
    page.on('pageerror', (err) => problems.push(err.message));
    await page.goto(url);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.panes').waitFor();
    return page;
  };
  // The page of a line, opened from the box by what it says.
  const opened = async (page, says, how = 'click') => {
    if (how === 'tap') {
      await page.getByRole('button', { name: 'Expand the side panel' }).tap();
      await sideSettled(page);
      await page.locator('.rail .rail-inbox').first().tap();
    } else await page.locator('.rail .rail-inbox').first().click();
    const line = page.getByRole('navigation', { name: 'Inbox' }).locator('.inbox-line', { hasText: says });
    await line.locator('.inbox-press')[how]({ position: { x: 24, y: 10 } });
    const shown = page.locator('.inbox-page');
    await shown.locator('.inbox-page-words', { hasText: says }).waitFor();
    return shown;
  };
  const SAYS = 'The preview is ready to try, in tabs.';
  const tabs = (shown) => shown.locator('.told-tabs [role="tab"]');
  const choose = async (shown, name) => {
    await tabs(shown).filter({ hasText: name }).click();
    await shown.locator('.told-tabs [role="tab"][aria-selected="true"]', { hasText: name }).waitFor();
  };
  const crowded = async (page, what) => {
    const wrong = await pageCrowded(page, '.inbox-page');
    expect(`nothing of ${what} over anything else, past the page or scrolling sideways: ${wrong.join('; ')}`, wrong.length === 0);
  };

  // ---- Ada's page, read once ------------------------------------------------------------------------------------
  const page = await signedIn('ada@example.com');
  const shown = await opened(page, SAYS);
  await tabs(shown).first().waitFor();
  const named = (await tabs(shown).allInnerTexts()).map((one) => one.trim());
  expect(`the tabs in the order they were said, got ${named.join(', ')}`, named.join('|') === 'To test|Deployment|Insights|Links');
  expect('the first tab showing', (await tabs(shown).first().getAttribute('aria-selected')) === 'true');
  // What it is about, under its words and over the tabs.
  const about = await shown.locator('.inbox-page-about').boundingBox();
  const row = await shown.locator('.told-tabs').boundingBox();
  const words = await shown.locator('.inbox-page-words').boundingBox();
  expect(`what it is about under its words and over the tabs (${JSON.stringify({ words, about, row })})`, about && row && words && words.y + words.height <= about.y + 0.5 && about.y + about.height <= row.y + 0.5);
  await shown.locator('.told-cards .told-card[data-card="shop-1"]').waitFor();
  expect('only the tab showing drawn', (await shown.locator('.told-runs, .told-rows, .told-links, .told-measures').count()) === 0);
  await page.screenshot({ path: `${out}/toldtabs-1-to-test.png` });

  // Deployment: the key's runs, the newest first.
  await choose(shown, 'Deployment');
  const runs = shown.locator('.told-runs .told-run');
  await runs.nth(2).waitFor();
  expect(`three runs of the deploy, got ${await runs.count()}`, (await runs.count()) === 3);
  for (const one of await runs.all()) {
    const said = (await one.innerText()).replace(/\s+/g, ' ');
    expect(`each run said by what it is of, how it went and how long, got "${said}"`, said.startsWith('deploy as deploy-dev went well') && /\d(ms|s)$/.test(said));
  }
  // Above them, the newest as the graph of what it ran, its tile saying how its step went.
  const latest = shown.locator('.told-runs .told-run-graph');
  await latest.locator('.gnode[data-node="main/loaded"][data-status="ok"]').waitFor();
  expect('the graph of the newest run, above the list', (await latest.getAttribute('data-run')) === (await runs.first().getAttribute('data-run')));
  expect('none of the cards on the page now', (await shown.locator('.told-cards').count()) === 0);
  await page.screenshot({ path: `${out}/toldtabs-2-deployment.png` });
  const first = await runs.first().getAttribute('data-run');
  await runs.first().locator('.told-run-open').click();
  await page.locator(`.tab[data-key="run:${first}"][aria-selected="true"]`).waitFor();
  // The line's tab shown again, with Deployment showing.
  await page.locator('.tab[data-key^="inbox:"]').click();
  await shown.locator('.told-tabs [role="tab"][aria-selected="true"]', { hasText: 'Deployment' }).waitFor();
  await shown.locator('.told-runs .told-run').first().waitFor();

  // Insights: the runs of the two commits, a choice as its tag and seconds as a length; the deploy's measurements.
  await choose(shown, 'Insights');
  const rows = shown.locator('.told-rows').first().locator('.told-row');
  await rows.nth(1).waitFor();
  expect(`the rows of those commits alone, got ${await rows.count()}`, (await rows.count()) === 2);
  const second = (await rows.nth(1).innerText()).replace(/\s+/g, ' ');
  expect(`a failed run of an hour said so, got "${second}"`, second.includes('20261123-0915-b') && second.includes('1h 01m') && second.includes('e5f6a7b8'));
  expect('a choice drawn as its tag', (await rows.nth(1).locator('.tag', { hasText: 'failed' }).count()) === 1);
  expect('nothing of what it printed in the table', !(await shown.innerText()).includes(END));
  expect('the private table read by its owner', (await shown.locator('.told-rows').nth(1).locator('.told-row', { hasText: 'Kept to some' }).count()) === 1);
  const measured = (await shown.locator('.told-measure[data-measure="run"]').innerText()).replace(/\s+/g, ' ');
  expect(`the deploy's latest beside its median, got "${measured}"`, /latest; .*the median of its last \w+/.test(measured));
  await crowded(page, 'Insights');
  await page.screenshot({ path: `${out}/toldtabs-3-insights.png` });
  await rows.first().locator('.told-row-open').click();
  const drawer = drawerAtRest(page);
  await drawer.waitFor();
  await drawer.locator(`textarea, input, .field-text`).first().waitFor();
  const inForm = await drawer.evaluate((one) => [...one.querySelectorAll('textarea, input')].map((field) => field.value).join(' ') + one.innerText);
  expect('the row opened with the whole of what it printed', inForm.includes(END));
  await page.screenshot({ path: `${out}/toldtabs-4-row.png` });
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });

  // Links, and every tab by its boxes at three widths.
  await choose(shown, 'Links');
  await shown.locator('.told-links a').waitFor();
  for (const width of [1280, 900, 640]) {
    await page.setViewportSize({ width, height: 860 });
    for (const name of ['To test', 'Deployment', 'Insights', 'Links']) {
      await choose(shown, name);
      await crowded(page, `${name} at ${width} wide`);
    }
    await page.screenshot({ path: `${out}/toldtabs-5-${width}.png` });
  }

  // A line told with parts in no tab draws no row of tabs.
  await page.setViewportSize({ width: 1280, height: 860 });
  const plain = await opened(page, 'Nothing in tabs here.');
  await plain.locator('.told-links a').waitFor();
  expect('no row of tabs where every part is on the first', (await plain.locator('.told-tabs').count()) === 0);

  // ---- Grace: the private table kept from her, in its place -------------------------------------------------
  const hersPage = await signedIn('grace@example.com');
  const hersShown = await opened(hersPage, SAYS);
  await choose(hersShown, 'Insights');
  const kept = hersShown.locator('.told-rows').nth(1).locator('.told-refused');
  await kept.waitFor();
  expect(`the private table said to be kept from her, got "${await kept.innerText()}"`, (await kept.innerText()).includes('kept private'));
  expect('the rows she may read listed', (await hersShown.locator('.told-rows').first().locator('.told-row').count()) === 2);
  await hersPage.screenshot({ path: `${out}/toldtabs-6-grace.png` });

  // ---- a phone ------------------------------------------------------------------------------------------------
  const phone = await signedIn('ada@example.com', { viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 });
  const onPhone = await opened(phone, SAYS, 'tap');
  await tabs(onPhone).first().waitFor();
  for (const name of ['To test', 'Deployment', 'Insights', 'Links']) {
    await tabs(onPhone).filter({ hasText: name }).tap();
    await onPhone.locator('.told-tabs [role="tab"][aria-selected="true"]', { hasText: name }).waitFor();
    await onPhone.locator('.told-tab-panel .told-part').first().waitFor();
    await crowded(phone, `${name} on a phone`);
  }
  await phone.screenshot({ path: `${out}/toldtabs-7-phone.png` });

  expect(`no errors on the page: ${problems.join('; ')}`, problems.length === 0);
  await browser.close();
  console.log('a told line in tabs: each part where it says, read as whoever reads it may');
} catch (error) {
  failed = error;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
