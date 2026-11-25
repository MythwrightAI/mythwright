// Takes pictures of installing the extension packages kept in this
// repository from inside the client, then of choosing what an agent runs on
// and which model.
//
//   DAEMON=../target/debug/factory node scripts/providers.mjs [out-dir]
//
// The model lists are asked of the real programs, so a picture shows a list
// only for a program that is installed and signed in here. The environment
// variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { openAdding } from './adding.mjs';
import { saysWhere } from './where.ts';

/** Turns the side panel to a view on the rail, unless it is showing already. */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
}


const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'providers-data');
const config = join(out, 'providers-config');

await mkdir(out, { recursive: true });
for (const dir of [data, config]) await rm(dir, { recursive: true, force: true });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0'], { cwd: repo, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
// Only this client's session goes elsewhere; the agents' programs are signed in under the real home.
const env = { ...process.env, XDG_CONFIG_HOME: config };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('channel', 'create', 'general');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1100, height: 760 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await side(page, 'Channels');
  await page.locator('.nav').getByText('general', { exact: true }).click();
  await openAdding(page);
  await page.getByRole('radio', { name: /^An agent/ }).check();
  const dialog = page.locator('.modal');
  // Out of the box there is only the built-in assistant, and a way to more.
  await page.screenshot({ path: `${out}/providers-0-out-of-the-box.png` });
  await dialog.getByRole('button', { name: 'Install one from Extensions' }).click();
  // The side panel turns to Extensions, where what came with the product is on offer.
  const panel = page.locator('nav[aria-label="Extensions"]');
  await panel.getByRole('button', { name: 'Install', exact: true }).first().waitFor();
  await page.screenshot({ path: `${out}/providers-extensions-before.png` });
  while ((await panel.getByRole('button', { name: 'Install', exact: true }).count()) > 0) {
    const left = await panel.getByRole('button', { name: 'Install', exact: true }).count();
    await panel.getByRole('button', { name: 'Install', exact: true }).first().click();
    await page.waitForFunction(
      (before) => document.querySelectorAll('nav[aria-label="Extensions"] .ext-row button.primary').length < before,
      left,
      { timeout: 15_000 },
    );
  }
  console.log('installed in the client:', (await panel.locator('.ext-row strong').allTextContents()).join(', '));
  // Each has a page of its own, with the models its program lists.
  await panel.locator('.ext-open').first().click();
  await page.locator('.view table tbody tr').first().waitFor({ timeout: 45_000 });
  await page.screenshot({ path: `${out}/providers-extension-page.png` });
  // Back to the workspace by the rail.
  await side(page, 'Channels');
  await page.locator('.nav').getByText('general', { exact: true }).click();
  await openAdding(page);
  await page.getByRole('radio', { name: /^An agent/ }).check();
  await dialog.getByLabel('Name').fill('builder');
  const runsOn = dialog.getByLabel('Runs on');
  const offered = await runsOn.locator('option').evaluateAll((options) => options.map((option) => [option.value, option.textContent]));
  console.log('runs on:', offered.map(([, title]) => title).join(' | '));
  let shot = 0;
  for (const [name, title] of offered) {
    await runsOn.selectOption(name);
    // Let the dialog turn to asking before waiting for it to have asked.
    await page.waitForTimeout(300);
    // The label that starts with "Model"; others mention the word further in.
    const model = dialog.locator('label').filter({ hasText: /^Model/ }).locator('select, input');
    if ((await model.count()) === 0) {
      console.log(`${title}: no model to choose`);
    } else {
      // A list that is asked of the program takes a moment to arrive.
      await page.waitForFunction(() => !document.querySelector('.modal select[disabled]'), null, { timeout: 45_000 });
      const kind = await model.first().evaluate((element) => element.tagName);
      const models = kind === 'SELECT' ? await model.first().locator('option').allTextContents() : ['(typed)'];
      console.log(`${title}: ${models.join(' | ')}`);
    }
    await page.screenshot({ path: `${out}/providers-${++shot}.png` });
  }
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, config]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
