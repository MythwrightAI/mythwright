// Takes pictures of a fleet of two devices: a hub, and a member that has
// joined it and is hosting the example key-value cache.
//
//   DAEMON=../target/debug/factory node scripts/devices.mjs [out-dir]
//
// Both daemons run on this machine, on ports of their own. Build the cache
// first: `cargo build -p kv-cache`. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
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

/** A daemon with nothing in it, and a command-line client signed in to it. */
async function device(name) {
  const data = join(out, `devices-${name}-data`);
  const home = join(out, `devices-${name}-home`);
  await rm(data, { recursive: true, force: true });
  await rm(home, { recursive: true, force: true });
  await mkdir(home, { recursive: true });
  const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
  const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--device', name], {
    cwd: repo,
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  // What is placed on a device is a service, which a daemon leaves running:
  // cut short, the check stops what was applied before the daemon.
  // Which is said once it has said where it is: until then it has started nothing.
  endsWithThis(daemon);
  const url = await saysWhere(daemon);
  endsWithThis(daemon, stopsAll(binary, url, { cwd: repo, env }));
  const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  return {
    url,
    cli,
    stop: async () => {
      const listed = await cli('instances', '--json').catch(() => ({ stdout: '{}' }));
      for (const instance of JSON.parse(listed.stdout).instances ?? []) {
        if (!instance.parent) await cli('stop', instance.id).catch(() => {});
      }
      daemon.kill('SIGTERM');
      await new Promise((gone) => daemon.on('exit', gone));
      await rm(data, { recursive: true, force: true });
      await rm(home, { recursive: true, force: true });
    },
  };
}

await mkdir(out, { recursive: true });
const hub = await device('hub');
const member = await device('workshop');
let failed = null;
try {
  // On the hub: invite the device. On the device: join with the invitation it printed, which is on a line of
  // its own, apart from the command that reads it.
  const invited = (await hub.cli('device', 'invite', 'builder-1')).stdout;
  const invitation = invited.split(/\s+/).find((word) => word.startsWith('inv_'));
  if (!invitation) throw new Error(`no invitation in what the hub printed:\n${invited}`);
  await member.cli('device', 'join', invitation);
  // The member hosts something of its own.
  const meta = await (await fetch(`${hub.url}/api/meta`)).json();
  const example = `${repo}/examples/blueprints/kv-cache.${meta.blueprint_ext}`;
  await member.cli('apply', example, '--param', 'build=false', '--param', `binary=${repo}/target/debug/kv-cache`);
  // And the hub sends it a step.
  const afar = `${out}/afar.${meta.blueprint_ext}`;
  const { writeFile } = await import('node:fs/promises');
  await writeFile(afar, 'blueprint: afar\npermissions: [shell, fleet]\nsteps:\n  look: { do: system.check, device: builder-1 }\n  say: { do: shell, device: builder-1, with: { run: "uname -sm" } }\n');
  console.log((await hub.cli('apply', afar)).stdout.trim());
  console.log((await hub.cli('device', 'list')).stdout.trim());

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const shot = async (name) => {
    await page.screenshot({ path: `${out}/devices-${name}.png` });
    console.log(name);
  };
  await page.goto(hub.url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Activity', { exact: true }).click();
  // What the hub sent the device was applied for no project, so it is no workspace's: every workspace is what shows it.
  await page.getByLabel(/^What is shown of /).selectOption({ label: 'All workspaces' });
  await page.getByRole('tab', { name: 'All', exact: true }).click();
  await page.locator('table tbody tr', { hasText: 'afar' }).first().waitFor({ timeout: 40_000 });
  await shot('1-activity');
  await page.getByRole('button', { name: 'Settings' }).click();
  await page.getByRole('heading', { name: 'Devices' }).scrollIntoViewIfNeeded();
  // The device is there by the short name of the key it proved, and this
  // one says its own, for the two to be compared.
  const row = page.locator('tr', { hasText: 'builder-1' }).last();
  await row.scrollIntoViewIfNeeded();
  const key = /^[0-9a-f]{4}(-[0-9a-f]{4}){4}$/;
  const theirs = (await row.locator('code').first().innerText()).trim();
  if (!key.test(theirs)) throw new Error(`the device's row does not say its key: "${theirs}"`);
  await page.getByRole('button', { name: 'Invite a device' }).waitFor();
  await page.evaluate(() => window.scrollBy(0, 200));
  await shot('2-settings');
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  await member.stop();
  await hub.stop();
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
