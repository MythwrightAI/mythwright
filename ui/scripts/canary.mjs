// Takes pictures of Activity while a canary rollout is under way.
//
//   DAEMON=../target/debug/factory node scripts/canary.mjs [out-dir]
//
// Starts a fresh daemon, hosts the example key-value cache (build it first:
// `cargo build -p kv-cache`), rolls out a second version, and photographs
// what is running while traffic is being shifted, and the measurements
// after. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { access, mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis, stopsAll } from './ends.mjs';
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
const data = join(out, 'canary-data');
const home = join(out, 'canary-home');

// The cache is hosted as built, not built here: without it, the rollout would wait a minute for a program that is not there.
const cacheProgram = join(repo, 'target/debug/kv-cache');
if (!(await access(cacheProgram).then(() => true, () => false))) {
  console.error(`${cacheProgram} is not built: build it first with \`cargo build -p kv-cache\`.`);
  process.exit(1);
}

await mkdir(out, { recursive: true });
await rm(data, { recursive: true, force: true });
await rm(home, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// What the command-line client is run with: a home of its own.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
// The blueprint here starts services, which a daemon leaves running: cut
// short, the check stops what it applied before its daemon.
// Which is said once it has said where it is: until then it has started nothing.
endsWithThis(daemon);
const url = await saysWhere(daemon);
endsWithThis(daemon, stopsAll(binary, url, { cwd: repo, env }));

// The command-line client, signed in from a home of its own.
const meta = await (await fetch(`${url}/api/meta`)).json();
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
const example = join(repo, `examples/blueprints/kv-cache.${meta.blueprint_ext}`);
const common = ['--param', 'build=false', '--param', `binary=${cacheProgram}`, '--param', 'observe=5s'];

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('apply', example, ...common);

  const cache = join(homedir(), '.cache/ms-playwright');
  const shells = (await readdir(cache)).filter((name) => name.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? join(cache, shells.at(-1), 'chrome-headless-shell-linux64/chrome-headless-shell'),
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, colorScheme: process.env.SCHEME === 'light' ? 'light' : 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const shot = async (name) => {
    await page.screenshot({ path: join(out, `canary-${name}.png`) });
    console.log(name);
  };

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Activity').click();
  // The example is applied for no project, so it is no workspace's: every workspace is what shows it, here and on the page after.
  await page.getByLabel(/^What is shown of /).selectOption({ label: 'All workspaces' });
  await page.locator('table tbody tr', { hasText: 'kv-cache' }).first().waitFor();
  await shot('1-one-version');

  // Roll out a second version and look while traffic is shared: the router
  // has said it routes to two backends at once.
  const rollout = cli('apply', example, ...common, '--param', 'version=v2');
  const routed = async () =>
    (await cli('events', '--topics', 'router.routed', '--limit', '50', '--json')).stdout
      .trim()
      .split('\n')
      .filter(Boolean)
      .some((line) => (JSON.parse(line).data?.backends ?? []).length > 1);
  for (let tries = 0; !(await routed()); tries += 1) {
    if (tries > 150) throw new Error('the router never said it shares traffic between two versions');
    await new Promise((later) => setTimeout(later, 200));
  }
  await shot('2-rolling-out');
  await rollout;
  await page.locator('table tbody tr', { hasText: 'kv-cache' }).first().waitFor();
  await shot('3-rolled-out');

  await side(page, 'Resources');
  await page.locator('.nav').getByText('Measurements').click();
  await page.locator('.tile').first().waitFor();
  await page.waitForTimeout(600);
  await shot('4-measurements');
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  // Stop what the blueprint started, then the daemon.
  const listed = await cli('instances', '--json').catch(() => ({ stdout: '{}' }));
  for (const instance of JSON.parse(listed.stdout).instances ?? []) {
    if (!instance.parent) await cli('stop', instance.id).catch(() => {});
  }
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  await rm(data, { recursive: true, force: true });
  await rm(home, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
