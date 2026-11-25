// Takes pictures of the client as the desktop app's window shows it, with the
// top bar the client draws there, and checks what it asks the window to do.
//
//   DAEMON=../target/debug/factory node scripts/window.mjs [out-dir]
//
// There is no desktop window here: the page is given stand-ins for the two
// things the window offers it (`window.ipc`, `window.desktop`) and what it
// posts is read back. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
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
const data = join(out, 'window-data');
const home = join(out, 'window-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// Started as the window starts it when it finds none running: with a word
// on why, for the factory's log.
const NOTE = 'The daemon was not running, so the window started it. It keeps running when the window is closed.';
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--note', NOTE], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };

let failed = null;
try {
  await run(binary, ['register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse', '--url', url], { cwd: repo, env });
  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  for (const scheme of ['dark', 'light']) {
    const page = await browser.newPage({ viewport: { width: 1360, height: 860 }, colorScheme: scheme });
    const problems = [];
    page.on('pageerror', (err) => problems.push(err.message));
    await page.addInitScript(() => {
      window.__asked = [];
      window.ipc = { postMessage: (message) => window.__asked.push(JSON.parse(message).window) };
      window.desktop = { frame: 'page' };
    });
    const asked = () => page.evaluate(() => window.__asked.splice(0));

    await page.goto(url);
    await page.getByLabel('Email').waitFor();
    await page.screenshot({ path: `${out}/window-${scheme}-1-sign-in.png` });
    // The buttons work on the sign-in page too, which is all there is of the
    // window before anyone has signed in.
    await page.locator('.window-controls').getByRole('button', { name: 'Minimise' }).click();
    expect('the bar to work before signing in', (await asked()).join() === 'minimize');

    await page.getByLabel('Email').fill('ada@example.com');
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await page.locator('.sidebar').waitFor();
    // That the window started the daemon is not put in front of whoever
    // opened it. It is in the factory's log, looked at further down.
    expect('nothing said about the daemon having been started', (await page.locator('.toast').count()) === 0);
    await page.screenshot({ path: `${out}/window-${scheme}-2-shell.png` });

    // Signed in, there is no bar: the tabs are the top line of the window, and
    // they stop short of the window's buttons, which are on that line too.
    const strip = await page.locator('.tabs').first().boundingBox();
    const buttons = await page.locator('.window-controls').boundingBox();
    expect(`the tabs to be at the top of the window, got ${strip.y}`, strip.y === 0 && buttons.y === 0);
    const spare = buttons.x - (strip.x + strip.width);
    expect(`the tabs to end a little before the buttons, got ${spare}`, spare > 30 && spare < 50);
    // That little is always empty, and takes hold of the window.
    await page.mouse.click(buttons.x - spare / 2, 18);
    expect('a press beside the buttons to drag the window', (await asked()).join() === 'drag');
    expect('the buttons to be as tall as the tabs, less their line', Math.abs(strip.height - buttons.height - 1) < 1);
    await page.locator('.window-controls').getByRole('button', { name: 'Maximise' }).click();
    await page.locator('.window-controls').getByRole('button', { name: 'Close' }).click();
    expect('each button to ask for its own thing', (await asked()).join() === 'maximize,close');
    // Pressing on the bar itself moves the window; twice, maximises it.
    await page.mouse.move(600, 18);
    await page.mouse.down();
    await page.mouse.up();
    expect('a press on the bar to drag the window', (await asked()).join() === 'drag');
    await page.mouse.dblclick(600, 18);
    expect('a double click on the bar to maximise', (await asked()).join() === 'drag,maximize');
    // So does the head of the side panel, and the corner under the buttons' row; a tab does not.
    await page.locator('.sidebar .brand').click({ position: { x: 60, y: 12 } });
    expect('a press on the head of the side panel to drag the window', (await asked()).join() === 'drag');
    await page.locator('.tab', { hasText: 'Start' }).click();
    expect('a press on a tab not to move the window', (await asked()).length === 0);
    // The edges and corners resize.
    for (const [x, y, edge] of [[1, 400, 'w'], [1358, 400, 'e'], [600, 858, 's'], [600, 1, 'n'], [1358, 858, 'se'], [1, 1, 'nw']]) {
      await page.mouse.move(x, y);
      await page.mouse.down();
      await page.mouse.up();
      const got = await asked();
      expect(`the ${edge} edge to resize, got ${JSON.stringify(got)}`, got.length === 1 && got[0].resize === edge);
    }

    // Tabs can be picked up and put down elsewhere in the row.
    await side(page, 'Resources');
    await page.locator('.nav').getByText('Measurements', { exact: true }).click();
    await side(page, 'Resources');
    await page.locator('.nav').getByText('Activity', { exact: true }).click();
    // The factory's log says when it started, and why, as it was told. It is the factory's own and no workspace's: every workspace is what shows it.
    await page.getByLabel(/^What is shown of /).selectOption({ label: 'All workspaces' });
    const started = page.locator('.factory-log tr', { hasText: 'Started' }).first();
    await started.waitFor();
    expect(`the log to say why the daemon was started, and it says ${JSON.stringify(await started.innerText())}`, (await started.innerText()).includes(NOTE));
    await page.screenshot({ path: `${out}/window-${scheme}-3-log.png` });
    const order = () => page.locator('.tab .truncate').allTextContents();
    expect('three tabs in the order they were opened', (await order()).join() === 'Start,Measurements,Activity');
    await page.locator('.tab', { hasText: 'Activity' }).dragTo(page.locator('.tab', { hasText: 'Start' }));
    expect(`the dragged tab to land before the one it was dropped on, got ${await order()}`, (await order()).join() === 'Activity,Start,Measurements');
    await asked();
    // Split one above the other, only the upper row of tabs is the top of the window.
    const body = await page.locator('.view').first().boundingBox();
    const from = await page.locator('.tab', { hasText: 'Measurements' }).boundingBox();
    await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
    await page.mouse.down();
    await page.mouse.move(body.x + body.width / 2, body.y + body.height - 30, { steps: 5 });
    await page.mouse.up();
    const rows = await page.locator('.tabs').evaluateAll((strips) => strips.map((strip) => strip.getBoundingClientRect().width));
    expect(`two rows of tabs, the lower one the full width, got ${rows}`, rows.length === 2 && rows[1] - rows[0] > 100);
    const lower = await page.locator('.tabs').nth(1).boundingBox();
    await page.mouse.click(lower.x + lower.width - 200, lower.y + 18);
    expect('the lower row not to move the window', (await asked()).length === 0);
    await page.screenshot({ path: `${out}/window-${scheme}-4-rows.png` });

    // A dialog dims the page, bar and all, and the buttons stay usable.
    await side(page, 'Channels');
    await page.getByRole('button', { name: 'New channel' }).click();
    await page.locator('.scrim').waitFor();
    await page.screenshot({ path: `${out}/window-${scheme}-3-dialog.png` });
    await page.locator('.window-controls').getByRole('button', { name: 'Minimise' }).click();
    expect('the buttons to work over a dialog', (await asked()).join() === 'minimize');
    await page.keyboard.press('Escape');

    console.log(scheme, 'ok');
    if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
    await page.close();
  }
  // In a browser there is no bar and nothing is asked of anyone.
  const plain = await browser.newPage({ viewport: { width: 1360, height: 860 } });
  await plain.goto(url);
  await plain.getByLabel('Email').waitFor();
  expect('no bar in a browser', (await plain.locator('.titlebar').count()) === 0);
  await browser.close();
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
