// The factory's secrets, as their owner sees them: one added by typing it
// once, where it is named (an agent's row, a service, a running instance),
// one that is named and was never set, one replaced, and one taken away
// behind a question that says what names it. Then somebody who may not
// manage them, who is not offered the page.
//
//   DAEMON=../target/debug/factory node scripts/secrets.mjs [out-dir]
//
// The value is looked for in everything the page was sent and everything
// it shows: it is never there. A program that prints it is read back with
// the secret's name in its place. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis, stopsAll } from './ends.mjs';
import { fillsPage } from './filled.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'secrets-data');
const home = join(out, 'secrets-home');

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
// A service is started here, which a daemon leaves running: cut short,
// the check stops what it applied before its daemon.
// Which is said once it has said where it is: until then it has started nothing.
endsWithThis(daemon);
const url = await saysWhere(daemon);
endsWithThis(daemon, stopsAll(binary, url, { cwd: repo, env }));
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

// Two values, neither of them anybody's: the first is set, the second replaces it.
const FIRST = 'sk_test_first_4eC39HqLyjWDarjtT1zdp7dc';
const SECOND = 'sk_test_second_9aZ81KpQwmXNbtrY5uvh2Lfe';
const MARK = '[the secret `stripe`]';

let failed = null;
let checkout = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  // Everything the daemon sends the page is kept, to be looked through for a value at the end.
  const sent = [];
  page.on('response', (response) => {
    if (response.url().includes('/api/') && !response.url().includes('/api/events')) void response.text().then((body) => sent.push(body)).catch(() => {});
  });
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signIn = async (on, email) => {
    await on.goto(url);
    await on.getByLabel('Email').fill(email);
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  await signIn(page, 'ada@example.com');

  // The owner is offered the page from Settings, and it begins with nothing kept.
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Manage secrets', exact: true }).click();
  await page.getByRole('heading', { name: 'Secrets', level: 1 }).waitFor();
  await page.getByText('No secrets yet').waitFor();
  await page.screenshot({ path: `${out}/secrets-0-none.png` });

  // One is added by typing it once, into a field that hides it.
  await page.getByRole('button', { name: 'Add a secret', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel(/^Name/).fill('stripe');
  const typed = dialog.getByLabel(/^Value/);
  expect('the value to be typed into a field that hides it', (await typed.getAttribute('type')) === 'password');
  await typed.fill(FIRST);
  await page.screenshot({ path: `${out}/secrets-1-adding.png` });
  await dialog.getByRole('button', { name: 'Set it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  const kept = (name) => page.locator(`.secret-kept[data-secret="${name}"]`);
  await kept('stripe').getByText('by Ada').waitFor();
  await kept('stripe').getByText('Named nowhere').waitFor();
  expect('the page not to show what was typed', !(await page.content()).includes(FIRST));
  await page.screenshot({ path: `${out}/secrets-2-set.png` });

  // It is named: by an agent's row, by a service, and in the text of what is
  // running. The page is told as each comes to be, and nobody reads again.
  // The agent is also handed one nobody has set.
  await cli('agent', 'add', 'builder', '--project', 'shop', '--secret', 'PAY_KEY=stripe', '--secret', 'MAIL=mail-password');
  await kept('stripe').getByText('builder').waitFor();
  const file = join(home, 'checkout.bp');
  await writeFile(
    file,
    [
      'blueprint: checkout',
      'permissions: [fleet, shell, secrets]',
      'steps:',
      '  pay:',
      '    do: shell',
      '    env: { STRIPE_KEY: { secret: stripe } }',
      '    with: { run: \'echo "paying with $STRIPE_KEY"\' }',
      '  serve:',
      '    needs: [pay]',
      '    do: service.start',
      '    env: { STRIPE_KEY: { secret: stripe } }',
      '    with: { name: checkout, run: \'echo "serving with $STRIPE_KEY"; exec sleep 600\' }',
      '',
    ].join('\n'),
  );
  const applied = JSON.parse((await cli('apply', file, '--project', 'shop', '--json')).stdout);
  checkout = applied.instance;
  expect(`the blueprint to have been applied, got ${JSON.stringify(applied)}`, Boolean(checkout && applied.run));
  await kept('stripe').locator('.secret-named li', { hasText: 'service' }).waitFor();
  await kept('stripe').locator('.secret-named li', { hasText: 'running' }).waitFor();
  const named = (await kept('stripe').locator('.secret-named li').allInnerTexts()).map((line) => line.replace(/\s+/g, ' ').trim());
  expect(`the agent, the service and the instance that name it, got ${JSON.stringify(named)}`, named.length === 3 && named[0].includes('builder is handed it as PAY_KEY') && named[1].includes('checkout was started with it as STRIPE_KEY') && named[2].includes('checkout names it in an env, in shop'));
  // What is named and was never set is there too, said as that.
  await kept('mail-password').getByText('not set').waitFor();
  expect('the one never set to say who names it', (await kept('mail-password').innerText()).includes('builder'));
  expect('nothing to take away of one never set', (await kept('mail-password').getByRole('button', { name: 'Remove' }).count()) === 0);
  await page.screenshot({ path: `${out}/secrets-3-named.png` });
  // The secrets are a page of tables, and fill its tab (`filled.mjs`).
  await fillsPage(page, 'the Secrets page', page.locator('.view-body', { has: page.locator('.secret-named') }), { ready: kept('stripe'), shot: `${out}/secrets-filled` });

  // What the program printed of it is read back with the name in its place.
  const logs = (await cli('logs', applied.run)).stdout;
  expect(`the step's log to say the secret by name, got ${logs}`, logs.includes(`paying with ${MARK}`) && !logs.includes(FIRST));

  // Replaced: what is there is not shown, and the new one is typed as the first was.
  await kept('stripe').getByRole('button', { name: 'Replace', exact: true }).click();
  await dialog.getByRole('heading', { name: 'Replace stripe' }).waitFor();
  expect('the name not to be changed by replacing', await dialog.getByLabel(/^Name/).isDisabled());
  expect('the field to begin empty, with nothing of what is kept', (await dialog.getByLabel(/^Value/).inputValue()) === '');
  await dialog.getByLabel(/^Value/).fill(SECOND);
  await page.screenshot({ path: `${out}/secrets-4-replacing.png` });
  await dialog.getByRole('button', { name: 'Replace it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  // Said again: what was said of the first may still be showing.
  await page.getByText('stripe is set. It is not shown again.').last().waitFor();

  // Taken away, behind a question that says what names it.
  await kept('stripe').getByRole('button', { name: 'Remove', exact: true }).click();
  await dialog.getByRole('heading', { name: 'Remove stripe?' }).waitFor();
  const asked = (await dialog.locator('.secret-named li').allInnerTexts()).join(' | ');
  expect(`the question to say what names it, got ${asked}`, asked.includes('the agent builder, as PAY_KEY') && asked.includes('the service checkout, as STRIPE_KEY') && asked.includes('checkout in shop, which is running'));
  await page.screenshot({ path: `${out}/secrets-5-asked.png` });
  // A dialog listens for Escape a moment after it is drawn; left that way, nothing is taken.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });
  expect('the secret to be there still after the question was left', (await kept('stripe').getByRole('button', { name: 'Remove' }).count()) === 1);
  await kept('stripe').getByRole('button', { name: 'Remove', exact: true }).click();
  await dialog.getByRole('button', { name: 'Remove it', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  // It is still named, so it is still listed: as not set, with what names it.
  await kept('stripe').getByText('not set').waitFor();
  expect('what named it to be said still', (await kept('stripe').locator('.secret-named li').count()) === 3);
  await page.screenshot({ path: `${out}/secrets-6-taken.png` });
  const listed = (await cli('secret', 'list', '--where')).stdout;
  expect(`the command line to say the same, got ${listed}`, /stripe\s+not set/.test(listed) && listed.includes('agent builder (PAY_KEY)'));

  // Neither value was ever sent to the page or shown by it.
  await page.waitForLoadState('networkidle').catch(() => {});
  const everything = sent.join('\n') + (await page.content());
  expect('neither value in anything the page was sent or shows', !everything.includes(FIRST) && !everything.includes(SECOND));

  // Somebody who may not manage them is not offered the page, and is refused what it reads.
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a second person to have joined, got ${joined.status}`, joined.ok);
  const theirs = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const other = await theirs.newPage();
  other.on('pageerror', (err) => problems.push(err.message));
  await signIn(other, 'grace@example.com');
  await other.getByRole('button', { name: 'Settings', exact: true }).click();
  await other.getByRole('heading', { name: 'Settings', level: 1 }).waitFor();
  await other.getByRole('heading', { name: 'Theme', level: 2 }).waitFor();
  expect('a member not to be offered the page', (await other.getByRole('button', { name: 'Manage secrets' }).count()) === 0 && (await other.getByRole('heading', { name: 'Secrets' }).count()) === 0);
  const refused = await other.evaluate(async () => {
    const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
    const read = await fetch('/api/secrets?named=true', { headers: mine });
    const set = await fetch('/api/secrets/stripe', { method: 'PUT', headers: mine, body: JSON.stringify({ value: 'anything at all' }) });
    return [read.status, set.status];
  });
  expect(`a member to be refused where each is named, and setting one, got ${refused}`, refused.join() === '403,403');
  await other.screenshot({ path: `${out}/secrets-7-member.png`, fullPage: true });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  // The daemon leaves services running when it stops: the one started here is stopped with its instance.
  if (checkout) await cli('stop', checkout).catch(() => {});
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
