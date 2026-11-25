// A channel's agents talking things over with each other, in the client.
//
//   DAEMON=../target/debug/factory node scripts/discussed.mjs [out-dir]
//
// `design` has two agents, `helper` and `architect`, each a stand-in
// package that answers by what it is shown: a few lines of shell, and no
// agent's real program is started. Ada opens the Channel tab of the Tools
// panel beside it, whose settings hold Discussions… (the row's menu does
// not), which opens the drawer with
// a form made from `std/discuss`'s own params; she makes `helper` the lead
// and `architect` its advisor, and applies it. She then asks the channel
// something naming nobody: only the lead answers, consulting the
// architect, who agrees in a thread under what she asked, and the lead
// answers her in the channel with what was agreed. Her inbox has a line
// of it, whose page opens the thread. Writing a message that names both
// agents offers Ask them to discuss beside Send, and naming one does
// not; a message marked as a plan from a terminal is drawn with its mark;
// and a discussion that waits for her is ended from the head of its
// thread, which says how it stands, without her answering it.
// All of it by the boxes at three widths and a phone's. The environment
// variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { chmod, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldChannels } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'discussed-data');
const home = join(out, 'discussed-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });

/** A stand-in package called `name` that says `then` where what it is asked holds `when`, and `otherwise` where not. */
async function stand(name, branches, otherwise) {
  const dir = join(home, name);
  await mkdir(dir, { recursive: true });
  const program = join(dir, 'stand');
  const cases = branches.map(([when, then]) => `case "$asked" in *'${when}'*) printf '%s\\n' '${JSON.stringify({ text: then })}'; exit 0;; esac`).join('\n');
  await writeFile(program, `#!/bin/sh\nasked=$(cat)\n${cases}\nprintf '%s\\n' '${JSON.stringify({ text: otherwise })}'\n`);
  await chmod(program, 0o755);
  await writeFile(
    join(dir, 'extension.yaml'),
    `extension: ${name}
title: ${name}
description: A stand-in that answers by what it is asked.
agent:
  program: ${program}
  args:
    - ["turn"]
  input: \${ turn.prompt }
  output:
    - when: \${ event.text != null }
      reply: \${ event.text }
`,
  );
  return dir;
}

const helper = await stand(
  'stand-helper',
  [
    ['has ended', 'We will keep sessions in one table.'],
    ['payments', 'Let me check with @architect: which provider?'],
    ['keep sessions', 'Let me check with @architect: one table?'],
  ],
  'Fine.',
);
const architect = await stand(
  'stand-architect',
  [
    ['payments', 'That is a matter of cost, which is yours to decide.\nDECISION: needs'],
    ['discussing', 'One table is right.\nDECISION: agree'],
  ],
  'Hello.',
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
  await cli('extension', 'add', helper);
  await cli('extension', 'add', architect);
  await cli('channel', 'create', 'design');
  await cli('agent', 'add', 'helper', '--adapter', 'stand-helper', '--channel', 'design');
  await cli('agent', 'add', 'architect', '--adapter', 'stand-architect', '--channel', 'design');
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
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  page = await context.newPage();
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const rail = page.locator('.rail').getByRole('button', { name: 'Channels', exact: true });
  if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
  await unfoldChannels(page);
  const panel = page.getByRole('navigation', { name: 'Channels' });
  const row = panel.locator('.grouped-channel', { has: page.locator('.nav-item .truncate', { hasText: /^design$/ }) });
  await row.locator('.nav-item').first().click();
  await page.getByRole('heading', { name: '# design' }).waitFor();

  // ── What the channel offers, from its tab of the Tools panel, as a form of std/discuss's own params ──
  await row.hover();
  await row.getByRole('button', { name: /^More for/ }).click();
  expect('no Discussions in the row menu: it is set in the Channel tab', (await page.getByRole('menu').getByText('Discussions…').count()) === 0);
  await page.keyboard.press('Escape');
  if ((await page.locator('.context').count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  const tools = page.locator('.context');
  await tools.getByRole('tab', { name: /^Channel\b/ }).click();
  await tools.getByRole('region', { name: 'Settings of this channel' }).getByText('Discussions…', { exact: true }).click();
  const form = page.locator('.drawer form.applying');
  await form.waitFor();
  for (const param of ['lead', 'advisors', 'agents', 'on', 'turns', 'agreeing', 'tell_on']) {
    expect(`the form to ask ${param}`, (await form.locator('.comparing-label', { hasText: new RegExp(`^${param} `) }).count()) === 1);
  }
  expect('the form not to ask which channel: it is the one it was opened from', (await form.locator('.comparing-label', { hasText: /^channel / }).count()) === 0);
  await form.locator('label.comparing-field', { hasText: /^lead / }).locator('input').fill('helper');
  const advisors = form.getByLabel('advisors', { exact: true });
  await advisors.click();
  await advisors.fill('architect: technical advice, and confirming a plan');
  await advisors.press('Enter');
  await page.screenshot({ path: `${out}/discussed-1-form.png` });
  await form.getByRole('button', { name: 'Apply', exact: true }).click();
  await page.locator('.drawer').waitFor({ state: 'detached' });
  // The still copy of the drawer that was put away goes out the way it came.
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });

  // ── Ask them to discuss, offered only while two who take part are named ──────────────────────────
  const box = page.locator('.composer textarea').first();
  const discuss = page.getByRole('button', { name: 'Ask them to discuss', exact: true });
  await box.fill('@helper @architect which table?');
  await discuss.waitFor();
  // Naming one of them is asking that one: nothing more is offered.
  await box.fill('@helper which table?');
  await discuss.waitFor({ state: 'detached' });
  await box.fill('@helper @architect which table?');
  await discuss.waitFor();
  const inside = async (what) => {
    const [button, composer] = await Promise.all([discuss.boundingBox(), page.locator('.composer-box').first().boundingBox()]);
    expect(`${what}: Ask them to discuss inside the composer`, button && composer && button.x >= composer.x - 0.5 && button.x + button.width <= composer.x + composer.width + 0.5);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`${what}: nothing scrolling sideways, got ${sideways}px`, sideways <= 0);
  };
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await inside(`at ${width} wide`);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await inside("at a phone's width");
  await page.screenshot({ path: `${out}/discussed-5-ask.png` });
  await page.setViewportSize({ width: 1280, height: 860 });
  await box.fill('');

  // ── A plan, marked from a terminal, drawn with its mark ──────────────────────────────────────────
  await cli('post', 'design', '--mark', 'plan', 'The plan: one table.');
  const plan = page.locator('[data-id]', { hasText: 'The plan: one table.' });
  await plan.waitFor();
  expect('the plan to be drawn with its mark', (await plan.locator('.tag', { hasText: 'Plan' }).count()) === 1);
  await plan.screenshot({ path: `${out}/discussed-6-plan.png` });

  // ── A person talks to the lead alone, which consults its advisor and answers with what was agreed ──
  await box.fill('How should we keep sessions?');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await page.locator('.body', { hasText: 'We will keep sessions in one table.' }).waitFor({ timeout: 30_000 });
  const authors = await page.evaluate(async () => {
    const headers = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
    const design = (await (await fetch('/api/channels', { headers })).json()).channels.find((one) => one.name === 'design');
    const said = (await (await fetch(`/api/channels/${design.id}/messages`, { headers })).json()).messages;
    return said.map((one) => [one.author, one.body]);
  });
  expect(`only the lead to answer in the channel, got ${JSON.stringify(authors)}`, authors.every(([author]) => author !== 'agent:architect'));
  await page.screenshot({ path: `${out}/discussed-2-answered.png` });

  // ── Her inbox has a line of it, whose page opens the thread ──────────────────────────────────────
  await page.locator('.rail .rail-inbox').click();
  const line = page.getByRole('navigation', { name: 'Inbox' }).locator('.inbox-line', { hasText: 'agreed in a discussion' });
  await line.waitFor();
  await line.click();
  const open = page.locator('.told-message button');
  await open.waitFor();
  await page.screenshot({ path: `${out}/discussed-3-told.png` });
  await open.click();
  await page.locator('.thread .body, .context .body', { hasText: 'One table is right.' }).first().waitFor();
  await page.screenshot({ path: `${out}/discussed-4-thread.png` });

  // ── A discussion that waits for her, ended from the head of its thread without answering it ─────
  await page.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).click();
  await row.locator('.nav-item').first().click();
  await page.getByRole('heading', { name: '# design' }).waitFor();
  const payments = page.locator('.composer textarea').first();
  await payments.fill('Which payments provider should we use?');
  // Sent from the channel's own box: the thread opened before has one of its own.
  await payments.press('Enter');
  const asked = page.locator('[data-id]', { hasText: 'Which payments provider should we use?' });
  // The advisor's answer, and once the discussion waits what is said of that: the thread grows as it is read.
  const replies = asked.getByText(/^\d+ repl(y|ies)$/);
  await replies.waitFor({ timeout: 30_000 });
  await replies.click();
  const head = page.locator('.discussion-head');
  await head.getByText('This discussion waits for a person to decide.').waitFor({ timeout: 30_000 });
  await page.screenshot({ path: `${out}/discussed-7-waits.png` });
  for (const width of [1280, 1000, 800, 390]) {
    await page.setViewportSize({ width, height: 860 });
    const [bar, button] = await Promise.all([head.boundingBox(), head.getByRole('button', { name: 'End discussion' }).boundingBox()]);
    expect(`at ${width} wide: End discussion inside the thread's head`, bar && button && button.x >= bar.x - 0.5 && button.x + button.width <= bar.x + bar.width + 0.5);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(`at ${width} wide: nothing scrolling sideways, got ${sideways}px`, sideways <= 0);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await head.getByRole('button', { name: 'End discussion' }).click();
  await head.waitFor({ state: 'detached' });
  const going = await page.evaluate(async () => {
    const headers = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
    const design = (await (await fetch('/api/channels', { headers })).json()).channels.find((one) => one.name === 'design');
    return (await (await fetch(`/api/channels/${design.id}/discussing`, { headers })).json()).discussions;
  });
  expect(`no discussion going or waiting once it is ended, got ${JSON.stringify(going)}`, Array.isArray(going) && going.length === 0);
  await page.screenshot({ path: `${out}/discussed-8-ended.png` });

  console.log('discussed: a lead consults its advisor in a thread and answers with what was agreed, told in the inbox; Ask them to discuss offered beside Send; a plan drawn with its mark');
  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await page?.screenshot({ path: `${out}/discussed-0-failed.png` }).catch(() => {});
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
