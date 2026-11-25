// Beginning with no registration: a fresh daemon,
// whose first start made its owner and signed its own command line in;
// the address alone, with no session, saying in one line to run `mw open`
// with a sign-in for an account that has a password under it; the link
// `mw open --link` makes opened, and the app signed in with no form, the
// link taken off the address; that link opened again refused in the
// daemon's sentence on the same page; and the owner giving the account a
// password and a name on their Profile, signing in by it in a browser that holds
// nothing. Each page by the boxes at three widths and a phone's, plainly
// and with `LOOK="size=largest density=compact"`.
//
//   DAEMON=../target/debug/factory node scripts/firstrun.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'firstrun-data');
const home = join(out, 'firstrun-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// A home of its own, so that nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
let said = '';
daemon.stdout.on('data', (chunk) => (said += String(chunk)));
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};
const WIDTHS = [
  [1280, 860],
  [900, 800],
  [640, 800],
  [390, 844],
];

let failed = null;
try {
  // The command line that started it is the owner's already, and nobody registered.
  expect('the command line to be the owner the start made, whom nobody has named', (await cli('whoami')).stdout.startsWith('Unnamed'));
  expect('the workspace made with them to be called so too', (await cli('workspace', 'list')).stdout.includes('Unnamed workspace'));
  const link = async () => (await cli('open', '--link')).stdout.trim();

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const opened = async () => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    return page;
  };
  // Where the page is narrow the side panel lies over it, as it does on a phone, until it is put away: the page
  // beside it is held by the boxes there, and the whole of it where there is room for both.
  const byTheBoxes = async (page, what, name, narrowly = 'body') => {
    for (const [width, height] of WIDTHS) {
      await page.setViewportSize({ width, height });
      const wrong = await pageCrowded(page, width < 900 ? narrowly : 'body');
      expect(`nothing of ${what} over anything else or past the window at ${width}: ${wrong.join('; ')}`, wrong.length === 0);
      const sideways = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
      expect(`${what} not to scroll sideways at ${width}`, !sideways);
      await page.screenshot({ path: join(out, `firstrun-${name}-${width}.png`) });
    }
    await page.setViewportSize({ width: 1280, height: 860 });
  };

  // The address alone: one line that says to run `mw open`, and a sign-in under it.
  const nobody = await opened();
  await nobody.goto(url);
  const line = nobody.locator('.opens-with');
  await line.waitFor();
  const words = (await line.innerText()).replace(/\s+/g, ' ');
  expect(`the page to say to run \`mw open\` here, got "${words}"`, words.includes('mw open') && words.includes('on this machine'));
  expect('a sign-in by email and password under it', (await nobody.getByLabel('Email').count()) === 1 && (await nobody.getByLabel('Password').count()) === 1);
  expect('no rail before anybody is signed in', (await nobody.locator('.rail').count()) === 0);
  await byTheBoxes(nobody, 'the page with no session', 'nobody');

  // Opened at the link: the app, signed in, with no form, and the secret gone from the address.
  const first = await link();
  expect(`a link to this daemon, got ${first}`, first.startsWith(`${url}/#once=once_`));
  const page = await opened();
  await page.goto(first);
  await page.locator('.rail').waitFor();
  expect('no form on the way in', (await page.getByLabel('Password').count()) === 0);
  expect(`the link taken off the address, got ${page.url()}`, !page.url().includes('once'));
  await page.getByText('Setup Guide').first().waitFor();
  await byTheBoxes(page, 'the app signed in by the link', 'signed-in', '.main');

  // Opened again, it is spent: the page says so in the daemon's sentence, over the sign-in.
  const again = await opened();
  await again.goto(first);
  await again.locator('.opens-with').waitFor();
  const refused = again.getByRole('alert');
  await refused.waitFor();
  const why = await refused.innerText();
  expect(`the used link refused in the daemon's sentence, got "${why}"`, why.includes('used already') && why.includes('mw open'));
  expect(`the refused link taken off the address too, got ${again.url()}`, !again.url().includes('once'));
  await byTheBoxes(again, 'a used link refused', 'refused');

  // A password set on the Profile by the owner, who had none, and a browser that holds nothing signed in by it.
  await page.getByRole('button', { name: /^You: / }).click();
  const you = page.locator('.you-panel');
  await you.waitFor();
  await you.getByLabel('Email').fill('ada@example.com');
  await you.getByLabel('Password').fill('correct horse');
  await you.getByRole('button', { name: 'Set password' }).click();
  await you.getByText('Set: you can sign in with your email and this password too', { exact: false }).waitFor();
  await you.getByRole('button', { name: 'Set password' }).waitFor({ state: 'detached' });
  expect('no way to set a password once there is one', (await you.getByRole('button', { name: 'Set password' }).count()) === 0);
  await you.screenshot({ path: join(out, 'firstrun-password-set.png') });
  // Named there, the owner takes the name, and the workspace made with them follows it, by the id it was made with.
  const name = you.getByLabel('Name');
  await name.fill('Ada');
  await name.press('Enter');
  await page.getByRole('button', { name: 'You: Ada' }).waitFor();
  const spaces = (await cli('workspace', 'list')).stdout;
  expect(`the workspace to be called after her, got:\n${spaces}`, spaces.startsWith('unnamed-workspace') && spaces.includes("Ada's workspace"));
  const elsewhere = await opened();
  await elsewhere.goto(url);
  await elsewhere.getByLabel('Email').fill('ada@example.com');
  await elsewhere.getByLabel('Password').fill('correct horse');
  await elsewhere.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await elsewhere.locator('.rail').waitFor();

  await browser.close();
  expect(`the start to say it made the owner, got:\n${said}`, said.includes('owner   Unnamed, made now'));
  expect(`the start never to print a link where nobody is at a terminal, got:\n${said}`, !said.includes('once_'));
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
