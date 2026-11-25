// Arranges tabs into groups: side by side, one above the other, and back.
//
//   DAEMON=../target/debug/factory node scripts/panes.mjs [out-dir]
//
// A tab is dragged to the edge of a group to split it, a divider is dragged
// to share the space differently, and what was typed in a document has to
// survive all of it. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

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
const data = join(out, 'panes-data');
const home = join(out, 'panes-home');
// The project's folder is one that was said, so that its Repo is listed under it: a folder of its own, beside the data.
const shop = join(out, 'panes-shop');

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
  const page = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const file = (path, content) =>
    page.evaluate(
      async ([path, content]) => {
        const at = '/api/projects/shop/file';
        const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
        if (content === null) return (await (await fetch(`${at}?path=${encodeURIComponent(path)}`, { headers: mine })).json()).content;
        const put = { method: 'PUT', headers: { ...mine, 'content-type': 'application/json' }, body: JSON.stringify({ path, content }) };
        if (!(await fetch(at, put)).ok) throw new Error(`could not write ${path}`);
        return content;
      },
      [path, content],
    );
  /** The groups as they are on the page: each one's tabs, and the one showing marked with a star. */
  const groups = () =>
    page.locator('.tabs').evaluateAll((strips) =>
      strips.map((strip) =>
        [...strip.querySelectorAll('.tab')]
          .map((tab) => `${tab.querySelector('.truncate').textContent}${tab.getAttribute('aria-selected') === 'true' ? '*' : ''}`)
          .join(','),
      ),
    );
  const said = async (what, want) => {
    const got = (await groups()).join(' | ');
    expect(`${what}: "${want}", got "${got}"`, got === want);
  };
  const tab = (name) => page.locator('.tab', { hasText: name });
  const view = (heading) => page.locator('.view', { has: page.locator('h1', { hasText: heading }) });
  /** Picks a tab up, carries it to a point, and puts it down there. */
  const carry = async (name, to, during) => {
    const from = await tab(name).boundingBox();
    await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
    await page.mouse.down();
    await page.mouse.move(to.x, to.y, { steps: 6 });
    await during?.();
    await page.mouse.up();
  };
  /** A point in a box: `across` and `down` are shares of it. */
  const point = (box, across, down) => ({ x: box.x + box.width * across, y: box.y + box.height * down });

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await file('about.md', '# About\n\nA shop.\n');
  await file('notes.md', '# Notes\n\nNothing yet.\n');
  await side(page, 'Resources');
  // A project's files are under it, folded away until they are wanted: unfolded by their arrow where Expand all did not, their folder never having been looked at.
  await unfold(page, 'Shop repo');
  await page.locator('.nav').getByText('about.md').click();
  await page.locator('.nav').getByText('notes.md').click();
  await page.locator('.writer h1', { hasText: 'Notes' }).waitFor();
  await said('one group to begin with', 'Start,about.md,notes.md*');

  // Dragged to the right edge of the group, a tab takes the right half of it.
  const whole = await view('notes.md').boundingBox();
  await carry('notes.md', point(whole, 0.95, 0.5), async () => {
    await page.locator('.landing').waitFor();
    await page.screenshot({ path: `${out}/panes-1-carrying.png` });
  });
  await said('two groups side by side', 'Start,about.md* | notes.md*');
  const left = await view('about.md').boundingBox();
  const right = await view('notes.md').boundingBox();
  expect('the two to share the width evenly', Math.abs(left.width - right.width) < 2 && right.x > left.x + left.width - 2);
  await page.screenshot({ path: `${out}/panes-2-side-by-side.png` });

  // What is typed in a document stays in it while it is moved about, looked
  // away from, and looked at again.
  await page.locator('.view .writer h1', { hasText: 'About' }).click();
  await page.keyboard.press('End');
  await page.waitForTimeout(150);
  await page.keyboard.type(' the shop');
  await view('about.md').getByText('unsaved').waitFor();
  expect('the tab to say it is not saved', (await tab('about.md').getByLabel('not saved').count()) === 1);
  await carry('about.md', point(await view('notes.md').boundingBox(), 0.5, 0.95));
  await said('a third group, under the second', 'Start* | notes.md* | about.md*');
  await page.locator('.view .writer h1', { hasText: 'About the shop' }).waitFor();
  const under = await view('about.md').boundingBox();
  const over = await view('notes.md').boundingBox();
  expect('it to be below the other, and as wide', under.y > over.y + over.height - 2 && Math.abs(under.width - over.width) < 2);
  await page.screenshot({ path: `${out}/panes-3-three.png` });
  // Another tab opens in the group last used, and hides the document.
  await page.locator('.nav').getByText('Activity', { exact: true }).click();
  await said('a new tab in the group last used', 'Start* | notes.md* | about.md,Activity*');
  expect('the document to be off the page', (await page.locator('.writer h1', { hasText: 'About' }).count()) === 0);
  await tab('about.md').click();
  await page.locator('.view .writer h1', { hasText: 'About the shop' }).waitFor();
  await view('about.md').getByText('unsaved').waitFor();

  // Closing it asks first. Saved from there, the file has what was typed.
  await tab('about.md').getByLabel('Close about.md').click();
  await page.getByRole('dialog').getByText('has changes that are not saved').waitFor();
  await page.screenshot({ path: `${out}/panes-4-asked.png` });
  await page.getByRole('button', { name: 'Keep it open' }).click();
  await said('nothing closed', 'Start* | notes.md* | about.md*,Activity');
  await tab('about.md').getByLabel('Close about.md').click();
  await page.getByRole('button', { name: 'Save and close' }).click();
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  await said('the tab closed', 'Start* | notes.md* | Activity*');
  expect('the file to hold what was typed', (await file('about.md', null)).startsWith('# About the shop'));

  // A divider is dragged to share the space differently, and stops before
  // the group it squeezes is too narrow to use.
  const divider = await page.locator('.divider-row').boundingBox();
  await page.mouse.move(divider.x + divider.width / 2, divider.y + 200);
  await page.mouse.down();
  await page.mouse.move(divider.x - 400, divider.y + 200, { steps: 6 });
  await page.mouse.up();
  const narrow = await page.locator('.view', { hasText: 'Setup Guide' }).boundingBox();
  expect(`the left group to be as narrow as a group gets, got ${narrow.width}`, Math.abs(narrow.width - 320) < 2);

  // Put down among another group's tabs, a tab joins them; a group left with none goes.
  await carry('Activity', point(await tab('notes.md').boundingBox(), 0.5, 0.5));
  await said('two groups again', 'Start* | Activity*,notes.md');
  await page.screenshot({ path: `${out}/panes-5-joined.png` });

  // The arrangement is still there after the page is loaded again.
  await page.reload();
  await page.locator('.rail').waitFor();
  await said('the same arrangement after a reload', 'Start* | Activity*,notes.md');
  const again = await page.locator('.view', { hasText: 'Setup Guide' }).boundingBox();
  expect('the divider where it was left', Math.abs(again.width - narrow.width) < 2);

  // Without dragging: the button at the end of a group's tabs.
  await page.getByRole('button', { name: 'Split', exact: true }).click();
  await said('the showing tab split off to the right', 'Start* | notes.md* | Activity*');
  await page.screenshot({ path: `${out}/panes-6-split.png` });
  await tab('Activity').getByLabel('Close Activity').click();
  await tab('notes.md').getByLabel('Close notes.md').click();
  await said('one group when the others are closed', 'Start*');
  const alone = await page.locator('.view', { hasText: 'Setup Guide' }).boundingBox();
  expect('it to have the whole width back', Math.abs(alone.width - whole.width) < 2);

  {
    // The side panel's right edge is dragged to make it wider or narrower,
    // within what can be read and what leaves the page room; it stays as
    // it was left, and pressing the edge twice puts it back.
    const sidebar = page.locator('.sidebar');
    const across = async () => Math.round((await sidebar.boundingBox()).width);
    const edge = page.getByRole('separator', { name: 'Make the side panel wider or narrower' });
    const usual = await across();
    const grip = await edge.boundingBox();
    const middle = grip.y + 300;
    await page.mouse.move(grip.x + grip.width / 2, middle);
    await page.mouse.down();
    await page.mouse.move(grip.x + grip.width / 2 + 90, middle, { steps: 6 });
    await page.mouse.up();
    expect(`dragging its edge right to make the side panel wider by as much, from ${usual} to ${await across()}`, Math.abs((await across()) - (usual + 90)) <= 2);
    const wider = await across();
    await page.mouse.move(grip.x + grip.width / 2 + 90, middle);
    await page.mouse.down();
    await page.mouse.move(0, middle, { steps: 6 });
    await page.mouse.up();
    expect(`the side panel not to be dragged narrower than a name can be read in, got ${await across()}`, (await across()) === 180);
    await edge.focus();
    await page.keyboard.press('ArrowRight');
    expect(`the arrows to move the edge too, got ${await across()}`, (await across()) === 204);
    await page.reload();
    await sidebar.waitFor();
    expect(`the side panel to come back as wide as it was left, got ${await across()}`, (await across()) === 204);
    await edge.dblclick();
    expect(`pressing the edge twice to put it back as it started (${usual}), got ${await across()}; it had been ${wider}`, (await across()) === usual);
  }

  {
    // A row with more tabs than it has room for scrolls sideways under Split, which stays whole at its end and
    // over none of a tab's words where the row is scrolled as far as it goes; a wheel turned over it moves it;
    // and the tab showing is brought in clear of Split.
    await page.setViewportSize({ width: 820, height: 860 });
    await page.getByRole('button', { name: 'Settings' }).first().click();
    await page.locator('.tab[aria-selected="true"]', { hasText: 'Settings' }).waitFor();
    await side(page, 'Resources');
    const panel = page.locator('.sidebar');
    for (const line of ['Activity', 'Measurements', 'Members', 'Trash', 'Reference']) {
      await panel.getByText(line, { exact: true }).first().click();
      await page.locator('.tab[aria-selected="true"]', { hasText: line }).waitFor();
    }
    // They were opened in the group last used, which is the narrower of two here.
    await tab('Reference').click();
    const strip = page.locator('.tabs', { has: tab('Reference') });
    const split = strip.getByRole('button', { name: 'Split', exact: true });
    const sizes = () => strip.evaluate((row) => ({ scroll: row.scrollLeft, wide: row.scrollWidth, room: row.clientWidth }));
    const { wide, room } = await sizes();
    expect(`the row to hold more tabs than it has room for (${wide} in ${room})`, wide > room);
    const clear = async (name) => {
      const [box, edge] = [await tab(name).boundingBox(), await split.boundingBox()];
      return box.x + box.width <= edge.x + 0.5 && box.x >= (await strip.boundingBox()).x - 0.5;
    };
    expect('the tab showing, the last opened, whole and clear of Split', await clear('Reference'));
    await page.screenshot({ path: `${out}/panes-9-many-tabs.png` });
    await tab('Settings').click();
    expect('Settings, the first of them opened, chosen and brought back into view', await clear('Settings'));
    const before = (await sizes()).scroll;
    const over = await strip.boundingBox();
    await page.mouse.move(over.x + over.width / 3, over.y + over.height / 2);
    await page.mouse.wheel(0, 600);
    await strip.evaluate((row, was) => new Promise((done) => { const look = () => (row.scrollLeft > was ? done() : requestAnimationFrame(look)); look(); }), before);
    const after = await sizes();
    expect(`a wheel to scroll the row as far as it goes (${after.scroll} of ${after.wide - after.room})`, Math.abs(after.scroll - (after.wide - after.room)) <= 1);
    expect('the last tab clear of Split with the row scrolled to its end', await clear('Reference'));
    await page.screenshot({ path: `${out}/panes-10-scrolled.png` });
    await page.setViewportSize({ width: 1280, height: 860 });
  }

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
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
