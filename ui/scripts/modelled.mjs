// Which model wrote an agent's messages in a channel, said once and by
// the name its provider gives it.
//
//   DAEMON=../target/debug/factory node scripts/modelled.mjs [out-dir]
//
// An agent asked in a channel that, as an agent's program does when it
// has made something for whoever asked, posts a page with the factory's
// own command and then answers. It runs on a stand-in package that offers
// one model by an alias (`otter`, which it calls Otter 5.5) and says, as
// its turn starts, which model that is by an id no list holds
// (`stand-otter-5-5`). The answer is put down with that id, and the page,
// posted while the turn is in hand, with what the agent runs on as far as
// its provider has said (`Agents::model_in_hand`). Both are said by the
// one name (`modelOf` in `views/who.tsx`, over `ui/src/modelsaid.ts`),
// so the two read as one message: one head, saying Otter 5.5, and the id
// nowhere. Asked again, the same, now that the id is known. The
// environment variables are those of walkthrough.mjs; the stand-in is a
// few lines of shell, and no agent's real program is started.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'modelled-data');
const home = join(out, 'modelled-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(join(home, 'stand'), { recursive: true });
// The stand-in: asked which models it offers, one by an alias; given a turn, it says which model it is, posts a page with the
// factory's own command (which finds the factory and the agent's credential in what its program is handed) and answers.
const program = join(home, 'stand', 'stand');
await writeFile(
  program,
  `#!/bin/sh
if [ "$1" = models ]; then printf '{"models":[{"id":"otter","label":"Otter 5.5"}]}\\n'; exit 0; fi
cat > /dev/null
printf '{"kind":"start","model":"stand-otter-5-5"}\\n'
printf '<h1>Six ways</h1>' > page.html
"${binary}" post studio --attach page.html The page. > /dev/null || printf '{"kind":"end","text":"The page could not be posted.","bad":true}\\n'
printf '{"kind":"end","text":"Posted."}\\n'
`,
);
await chmod(program, 0o755);
await writeFile(
  join(home, 'stand', 'extension.yaml'),
  `extension: stand
title: Stand
description: A stand-in that says which model it is and posts a page before it answers.
agent:
  program: ${program}
  args:
    - ["turn"]
  input: \${ turn.prompt }
  output:
    - when: \${ event.kind == "start" }
      model: \${ event.model }
    - when: \${ event.kind == "end" }
      reply: \${ event.text }
      failed: \${ event.bad == true }
      error: \${ event.text }
  models:
    ask:
      args: ["models"]
      answer: \${ line.models != null }
      list: \${ line.models }
      id: \${ model.id }
      label: \${ model.label }
`,
);
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin', '--walls', 'off'], {
  cwd: repo,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config') };
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

let failed = null;
let page = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('extension', 'add', join(home, 'stand'));
  await cli('channel', 'create', 'studio');
  await cli('agent', 'add', 'maker', '--adapter', 'stand', '--model', 'otter', '--channel', 'studio');
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
  page = await (await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' })).newPage();
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const rail = page.locator('.rail').getByRole('button', { name: 'Channels', exact: true });
  if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
  await unfoldChannels(page);
  await page.locator('.nav').getByText('studio', { exact: true }).click();
  await page.getByRole('heading', { name: '# studio' }).waitFor();

  /** What the daemon put down of maker's messages: their words and the model each names. */
  const putDown = () =>
    page.evaluate(async () => {
      const headers = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
      const studio = (await (await fetch('/api/channels', { headers })).json()).channels.find((one) => one.name === 'studio');
      const said = (await (await fetch(`/api/channels/${studio.id}/messages`, { headers })).json()).messages;
      return said.filter((one) => one.author === 'agent:maker').map((one) => [one.body, one.model ?? null]);
    });
  /** Each of maker's messages as the page draws it: its words, and the model its head says, or nothing where it has no head. */
  const drawn = () =>
    page.evaluate(() =>
      [...document.querySelectorAll('[data-id]')]
        .filter((row) => row.querySelector('.body')?.textContent.trim().match(/^(The page\.|Posted\.)$/))
        .map((row) => [row.querySelector('.body').textContent.trim(), row.querySelector('.model')?.textContent.trim() ?? null]),
    );
  const askAndRead = async (times) => {
    await cli('post', 'studio', '@maker', 'make', 'the', 'page');
    await page.waitForFunction((times) => [...document.querySelectorAll('.body')].filter((one) => one.textContent.trim() === 'Posted.').length === times, times, { timeout: 30_000 });
    // The id the program said is known to the page once the daemon says so of the agent: until then it is asked for, once.
    await page.waitForFunction(() => ![...document.querySelectorAll('.model')].some((one) => one.textContent.includes('stand-otter')), null, { timeout: 10_000 }).catch(() => {});
  };

  await askAndRead(1);
  const first = await putDown();
  expect(`the page and the answer to be put down, the answer with the id its program said, got ${JSON.stringify(first)}`, first.length === 2 && first[0][0] === 'The page.' && first[1][0] === 'Posted.' && first[1][1] === 'stand-otter-5-5' && first[0][1] !== null);
  let rows = await drawn();
  expect(`the page and the answer to read as one message, headed once by the name its provider gives the model, got ${JSON.stringify(rows)}`, JSON.stringify(rows) === JSON.stringify([['The page.', 'Otter 5.5'], ['Posted.', null]]));
  await page.locator('[data-id]', { hasText: 'The page.' }).first().screenshot({ path: `${out}/modelled-1-one-head.png` });

  // Asked again, with the id its program says known: the same, under the new ask.
  await askAndRead(2);
  const second = (await putDown()).slice(2);
  expect(`the second page to be put down with the id its program said before, got ${JSON.stringify(second)}`, second.length === 2 && second.every(([, model]) => model === 'stand-otter-5-5'));
  rows = await drawn();
  expect(`each ask's page and answer to read as one, headed by the name, got ${JSON.stringify(rows)}`, JSON.stringify(rows) === JSON.stringify([['The page.', 'Otter 5.5'], ['Posted.', null], ['The page.', 'Otter 5.5'], ['Posted.', null]]));
  expect('the id nowhere on the page', !(await page.locator('.channel').innerText()).includes('stand-otter-5-5'));
  await page.screenshot({ path: `${out}/modelled-2-channel.png` });
  console.log('modelled: an answer and the page its agent posted read as one message, headed by the name its provider gives the model');
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/modelled-0-failed.png` }).catch(() => {});
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
