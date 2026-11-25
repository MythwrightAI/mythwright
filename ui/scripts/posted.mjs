// A file of a project posted in a channel as that file and not a copy
// (`views/attached.tsx`, `posted.ts`, `docshown.ts`): posted from a
// terminal with `--attach`, as an agent does, it is a chip with the
// file's name over where it is and how big; pressed, it is read in place
// in the Docs tab of the Tools panel beside the channel, with Open as a
// tab, and added to the channel's documents, which a second page never
// read again follows; pressed by somebody who may not write in the
// channel, it is read there and not added. A copy attached from outside
// the project offers Add to project…, which asks a folder of the
// project (its library first) and a name, writes it there, and the
// message names that file from then on, on both pages, and opens the
// same way. Nothing of the chips, the panel or the dialog lies over
// anything else or past the window at three widths and a phone's. Run
// plainly and with `LOOK="size=largest density=compact"`.
//
//   DAEMON=../target/debug/factory node scripts/posted.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { access, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { toolsAtRest } from './drawer.mjs';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'posted-data');
const home = join(out, 'posted-home');
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
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  // Shop, with its library, and a file written among its files as an agent writes one.
  await cli('project', 'create', 'Shop', '--without', 'fleet');
  const written = join(home, 'distribution.md');
  await writeFile(written, '# Distribution\n\nBy the river, and by road when the river is low.\n');
  await cli('file', 'write', 'shop', 'docs/distribution.md', '--from', written);
  await cli('channel', 'create', 'pantry', '--project', 'shop');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  if (!joined.ok) throw new Error(`expected Grace to join, got ${joined.status}`);
  // Posted as the project's file, from where it is, as the command line is handed it.
  const projects = JSON.parse((await cli('project', 'list', '--json')).stdout);
  const shop = (projects.projects ?? projects).find((one) => one.id === 'shop');
  const folder = shop?.path ?? join(data, 'projects', 'shop');
  await access(join(folder, 'docs', 'distribution.md'));
  const posted = (await cli('post', 'pantry', '--attach', join(folder, 'docs', 'distribution.md'), 'The', 'distribution.')).stdout.trim();
  if (!posted.includes("with the project's file `docs/distribution.md` of shop")) throw new Error(`expected it posted as the project's file, got ${posted}`);

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
  /** The channel opened from the Channels side panel. */
  const pantry = async (page) => {
    const button = page.getByRole('button', { name: 'Channels', exact: true });
    if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
    await unfoldChannels(page);
    await page.getByRole('navigation', { name: 'Channels' }).getByText('pantry', { exact: true }).click();
    await page.getByRole('heading', { name: '# pantry' }).waitFor();
  };
  const tools = (page) => page.getByRole('complementary', { name: 'Tools' });
  /** The lines of the Docs tab's list, as `called | where`. */
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
  /** Back from a document read in place to the list, where it is read in place. */
  const back = async (panel) => {
    const way = panel.getByRole('button', { name: 'Back to the documents' });
    if ((await way.count()) > 0) await way.click();
    await panel.locator('.channel-docs').waitFor();
  };
  const chip = (page, file) => page.locator(`.messages .attached-posted[data-file="${file}"], .attached-posted[data-file="${file}"]`).first();

  // ---- the chip, pressed: read in place in Docs, added to the channel's documents, and followed on a second page
  const page = await opened('ada@example.com');
  lastPage = page;
  await pantry(page);
  const theirs = await opened('grace@example.com');
  let reread = 0;
  // A page loaded again, and not the address bar following the tab showing, which the browser calls a navigation too.
  theirs.on('domcontentloaded', () => (reread += 1));
  await pantry(theirs);
  if ((await tools(theirs).count()) === 0) await theirs.getByRole('button', { name: 'Expand the tools panel' }).click();
  await tools(theirs).getByRole('tab', { name: 'Docs' }).click();
  await tools(theirs).getByText('This channel keeps no documents to hand yet.').waitFor();
  reread = 0;

  const distribution = chip(page, 'shop/docs/distribution.md');
  await distribution.waitFor();
  const said = (await distribution.innerText()).replace(/\s+/g, ' ').trim();
  expect(`the chip to say the file's name over where it is and how big, got ${said}`, /^distribution\.md shop\/docs\/distribution\.md · \d+ B$/.test(said));
  expect('the chip to be something pressed, not a copy to save', (await distribution.evaluate((one) => one.tagName)) === 'BUTTON');
  await page.screenshot({ path: `${out}/posted-0-chip${looked}.png` });
  await distribution.click();
  const panel = tools(page);
  await toolsAtRest(page).waitFor();
  expect('the Docs tab chosen', (await panel.getByRole('tab', { name: 'Docs' }).getAttribute('aria-selected')) === 'true');
  await panel.frameLocator('iframe.page-frame').getByText('By the river').waitFor();
  await page.screenshot({ path: `${out}/posted-1-read${looked}.png` });
  await until("the second page to list it among the channel's documents", () => lines(tools(theirs)), ['distribution.md | shop/docs/distribution.md']);
  expect('the second page never read again', reread === 0);
  await theirs.screenshot({ path: `${out}/posted-1-listed-theirs${looked}.png` });
  // Open as a tab, and back to the channel.
  await panel.getByRole('button', { name: 'Open as a tab', exact: true }).click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'distribution.md' }).first().waitFor();
  await page.locator('.tab', { hasText: 'pantry' }).first().click();
  await panel.waitFor();
  await back(panel);
  await until("it to be among the channel's documents", () => lines(panel), ['distribution.md | shop/docs/distribution.md']);

  // ---- somebody who may not write in the channel reads it there, and nothing is added
  await cli('file', 'write', 'shop', 'docs/prices.md', '--from', written);
  await cli('post', 'pantry', '--attach', join(folder, 'docs', 'prices.md'), 'The', 'prices.');
  const member = ['projects', 'channels:read', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search', 'inbox'];
  const less = await opened('grace@example.com', member);
  await pantry(less);
  const prices = chip(less, 'shop/docs/prices.md');
  await prices.waitFor();
  expect('no Add to project… offered to whoever may not write in the channel', (await less.getByRole('button', { name: 'Add to project…' }).count()) === 0);
  await prices.click();
  await tools(less).frameLocator('iframe.page-frame').getByText('By the river').waitFor();
  await back(tools(less));
  await until('nothing added by whoever may not write in the channel', () => lines(tools(less)), ['distribution.md | shop/docs/distribution.md']);
  await less.screenshot({ path: `${out}/posted-2-not-theirs${looked}.png` });

  // ---- a copy from outside the project, added to the project's library, named from then on and opened the same way
  const brief = join(home, 'brief.md');
  await writeFile(brief, '# Brief\n\nQuinces first, then medlars.\n');
  await cli('post', 'pantry', '--attach', brief, 'A', 'brief.');
  const copy = page.locator('.attached-copy', { hasText: 'brief.md' }).first();
  await copy.waitFor();
  await copy.getByRole('button', { name: 'Add to project…' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('heading', { name: 'Add brief.md to a project' }).waitFor();
  const options = () => dialog.getByLabel('Folder').locator('option').allInnerTexts();
  await until("the project's library offered first, then its top and its folders", async () => (await options()).slice(0, 2), ['docs (library)', 'The top of the project']);
  await page.screenshot({ path: `${out}/posted-3-adding${looked}.png` });
  const asked = await pageCrowded(page, '.modal');
  expect(`nothing of the dialog over anything else at 1280: ${asked.join('; ')}`, asked.length === 0);
  // A name another file has is refused in the daemon's sentence, with the dialog staying.
  await dialog.getByLabel('Name').fill('distribution.md');
  await dialog.getByRole('button', { name: 'Add it', exact: true }).click();
  await dialog.getByRole('alert').waitFor();
  await page.screenshot({ path: `${out}/posted-4-refused${looked}.png` });
  await dialog.getByLabel('Name').fill('brief.md');
  await dialog.getByRole('button', { name: 'Add it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  const placed = chip(page, 'shop/docs/brief.md');
  await placed.waitFor();
  await chip(theirs, 'shop/docs/brief.md').waitFor();
  expect('the second page still never read again', reread === 0);
  expect('the copy is no longer offered', (await page.locator('.attached-copy', { hasText: 'brief.md' }).count()) === 0);
  await placed.click();
  await tools(page).frameLocator('iframe.page-frame').getByText('Quinces first').waitFor();
  await back(tools(page));
  await until('the added file among the documents', () => lines(tools(page)), ['distribution.md | shop/docs/distribution.md', 'brief.md | shop/docs/brief.md']);
  await page.screenshot({ path: `${out}/posted-5-added${looked}.png` });

  // ---- by the boxes: the channel with its chips and the panel beside it, and the dialog, at three widths and a phone's
  for (const [width, height] of [
    [1280, 860],
    [900, 800],
    [640, 800],
    [390, 844],
  ]) {
    await page.setViewportSize({ width, height });
    const awayButton = page.getByRole('button', { name: 'Back to the channel' });
    // Where the panel lies over the channel, as on a phone, it is put away once it is where it will be. The page
    // hears that it is that narrow a moment after the window is made so (`phone`), so the way back is waited for.
    const open = (await tools(page).count()) > 0;
    if (open) await toolsAtRest(page).waitFor();
    if (open && width <= 760) await awayButton.waitFor();
    if ((await awayButton.count()) > 0) await awayButton.click();
    await chip(page, 'shop/docs/brief.md').waitFor();
    // The pointer off the messages: what is done with a message is shown over it while it is pointed at.
    await page.mouse.move(1, height - 1);
    const channel = await pageCrowded(page);
    expect(`nothing of the channel's chips over anything else or outside the window at ${width}: ${channel.join('; ')}`, channel.length === 0);
    await page.screenshot({ path: `${out}/posted-6-chips-${width}${looked}.png` });
    await chip(page, 'shop/docs/distribution.md').click();
    await toolsAtRest(page).waitFor();
    await tools(page).frameLocator('iframe.page-frame').getByText('By the river').waitFor();
    const read = await pageCrowded(page, 'aside.context');
    expect(`nothing of the panel over anything else or outside the window at ${width}: ${read.join('; ')}`, read.length === 0);
    await page.screenshot({ path: `${out}/posted-6-read-${width}${looked}.png` });
    await back(tools(page));
    // Where the panel lies over the channel, as on a phone, it is put away once it is where it will be.
    if ((await tools(page).count()) > 0) await toolsAtRest(page).waitFor();
    if ((await awayButton.count()) > 0) await awayButton.click();
    await cli('post', 'pantry', '--attach', brief, `Again at ${width}.`);
    const again = page.locator('.message', { hasText: `Again at ${width}.` }).locator('.attached-copy');
    await again.waitFor();
    await again.getByRole('button', { name: 'Add to project…' }).click();
    await until('its folders to be read', async () => (await page.getByRole('dialog').getByLabel('Folder').locator('option').allInnerTexts())[0], 'docs (library)');
    const modal = await pageCrowded(page, '.modal');
    expect(`nothing of the dialog over anything else or outside the window at ${width}: ${modal.join('; ')}`, modal.length === 0);
    await page.screenshot({ path: `${out}/posted-7-dialog-${width}${looked}.png` });
    await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  }

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/posted-failed${looked}.png` }).catch(() => {});
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
