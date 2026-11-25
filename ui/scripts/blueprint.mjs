// Writing a blueprint: its text coloured as a blueprint, what can go at a
// place offered there, and a word explained when it is pointed at.
//
//   DAEMON=../target/debug/factory node scripts/blueprint.mjs [out-dir]
//
// A step is written here the way a person would: a letter, a suggestion
// taken, the next one offered. What the editor offers comes from the
// daemon's description of the language, so this is a check of both halves
// together. And what it is applied with, asked in the drawer beside its
// text. And what is said under a graph, with its buttons, lying over no
// tile and no group's name at three widths and a phone's (`legended.mjs`).
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
import { besideDrawer } from './besidedrawer.mjs';
import { legendCrossed } from './legended.mjs';
import { closeTesting, pressTesting, testingMenu, testingOf } from './testing.mjs';
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
const data = join(out, 'blueprint-data');
const home = join(out, 'blueprint-home');
// The project's folder is one that was said, so that its Repo is listed under it: a folder of its own, beside the data.
const shop = join(out, 'blueprint-shop');

const RELEASE = `# Builds a version and puts it out.
blueprint: release
description: Build a version and put it out.
permissions: [shell, channels, blueprints]

params:
  version:
  channel: releases

state:
  releases: 0

steps:
  build:
    do: shell
    with:
      run: cargo build --release
    timeout: 30m
    retry: { max: 2, backoff: 10s }
    on_fail:
      - do: channel.post
        with: { channel: "\${params.channel}", body: "the build failed: \${self.error}" }
  host:
    call: deploy
    with:
      version: \${ params.version }
    when: \${ steps.build.ok && params.version != "" }
`;

const SHIP = `blueprint: ship
description: Put a version out, and stop at what nobody checked.
permissions: [blueprints]
steps:
  host:
    call: deploy
    as: shipped
    with: { version: v3 }
  checked:
    do: fail
    with: { message: "nobody looked at the address before this went out, and what was to look (in_1aaaaaaaaaaaaaa9) is not there: ask whoever keeps the door, and apply it again when they have said that it is theirs to open" }
    needs: [host]
`;

const DEPLOY = `blueprint: deploy
description: Put a version where people can reach it.
params:
  version: { type: string, description: which version to run }
  port: 8080
steps:
  go: { do: noop }
outputs:
  address: "127.0.0.1:\${params.port}"
`;

// Steps that say what they are for, one that says nothing, and a handler
// with a step whose key reads as bad news and whose words do not.
const TOLD = `blueprint: told
description: Count the shelves and fill them.

steps:
  ready:
    description: The shelves are counted before anything is put out. The rest of what it says is read where the step is opened.
    do: noop

  plain:
    needs: [ready]
    do: noop

  shelved:
    description: The shelves are filled one after another, which is said here in a sentence a good deal longer than a narrow page has room for on one line of a list.
    needs: [plain]
    do: noop

on:
  looked:
    event: instance.active
    scope: own
    when: \${ event.source.instance == instance.id }
    steps:
      wrong:
        description: The shelves are looked over. Nothing is said where nothing is amiss.
        do: noop
`;

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
let page = null;
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
  page = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();

  // Two blueprints in the project's folder: the one to be written in, and one it calls.
  const { where, ending } = await page.evaluate(async () => ({
    where: (await (await fetch('/api/projects', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).projects[0].path,
    ending: (await (await fetch('/api/meta')).json()).blueprint_ext,
  }));
  await mkdir(join(where, 'blueprints'), { recursive: true });
  await writeFile(join(where, `blueprints/release.${ending}`), RELEASE);
  await writeFile(join(where, `blueprints/deploy.${ending}`), DEPLOY);
  // Read as the project's own from that folder of its repository, which it says, as one that keeps them with its code does.
  await cli('project', 'blueprints', 'shop', 'blueprints');

  await side(page, 'Resources');
  await page.locator('.nav').getByText('Repo', { exact: true }).click();
  const files = page.locator('.files');
  await files.getByText('blueprints', { exact: true }).click();
  await files.getByText(`release.${ending}`).click();
  const text = page.locator('.cm-content');
  await text.waitFor();

  // Waits for the daemon's check to pass what is in the editor, and says what it found if it did not.
  const checks = async () => {
    await page
      .locator('.view-head')
      .getByText('checks')
      .waitFor({ timeout: 8000 })
      .catch(async () => {
        await page.screenshot({ path: `${out}/blueprint-0-failed.png` });
        throw new Error(`expected the blueprint to check, and it has: ${await page.locator('.problems').innerText().catch(() => 'no list of problems')}`);
      });
  };

  // ---- how it is coloured ---------------------------------------------------
  // Each kind of piece has a class of its own; what has one says how the text was read.
  const coloured = (kind) => page.locator(`.cm-content .bp-${kind}`).allInnerTexts();
  await page.locator('.cm-content .bp-field').first().waitFor();
  const fields = await coloured('field');
  for (const word of ['blueprint', 'permissions', 'params', 'steps', 'do', 'with', 'timeout', 'retry', 'on_fail', 'call', 'when']) {
    expect(`\`${word}\` to be coloured as a word of the language, of ${fields}`, fields.includes(word));
  }
  const names = await coloured('name');
  expect(`the steps and params to be coloured as names, got ${names}`, ['version', 'channel', 'build', 'host'].every((name) => names.includes(name)));
  const keys = await coloured('key');
  expect(`arguments to be coloured as the writer's own keys, got ${keys}`, keys.includes('run') && keys.includes('body') && keys.includes('releases'));
  expect('`version` under `with` not to be taken for the field of that name', keys.includes('version'));
  expect('the actions to be coloured as actions', (await coloured('action')).join() === 'shell,channel.post');
  expect('the blueprint called to be coloured as one', (await coloured('blueprint')).join() === 'deploy');
  expect('a length of time to be coloured as a number', (await coloured('number')).includes('30m'));
  const roots = await coloured('root');
  expect(`the names expressions start with to be told from the rest, got ${roots}`, roots.join() === 'params,self,params,steps,params');
  expect('what is inside an expression to be marked as inside it', (await page.locator('.cm-content .bp-held').count()) > 12);
  expect('a comment to be a comment', (await coloured('comment')).join() === '# Builds a version and puts it out.');
  await checks();
  await page.screenshot({ path: `${out}/blueprint-1-coloured.png` });

  // ---- what is offered --------------------------------------------------------
  const popup = page.locator('.cm-tooltip-autocomplete');
  const offered = () => popup.locator('li .cm-completionLabel').allInnerTexts();
  // The list as it stands once it holds this label, however long that takes to draw.
  const offering = async (label) => {
    await popup.locator('li .cm-completionLabel', { hasText: new RegExp(`^${label.replace(/[.$]/g, '\\$&')}$`) }).first().waitFor();
    return offered();
  };
  const line = () => page.locator('.cm-activeLine').innerText();
  // The list does not take Enter in its first moments, so that somebody
  // ending a line as it appears is not given a suggestion they never saw.
  // A script is quicker than that; a person is not.
  const take = async (label) => {
    await page.waitForFunction((wanted) => document.querySelector('.cm-tooltip-autocomplete li[aria-selected] .cm-completionLabel')?.textContent === wanted, label);
    await page.waitForTimeout(120);
    await page.keyboard.press('Enter');
  };

  // A new step, under the last one. Its name is the writer's own.
  await text.click();
  await page.keyboard.press('Control+End');
  await page.waitForFunction(() => document.querySelector('.cm-activeLine')?.textContent === '');
  await page.keyboard.type('  tell:');
  await page.keyboard.press('Enter');
  expect(`the line under a key to be one step in, and it is ${JSON.stringify(await line())}`, (await line()).replace(/\n$/, '') === '    ');

  // Asked for on an empty line, what a step may have, starting with what it does.
  await page.keyboard.press('Control+Space');
  const step = await offering('do');
  expect(`a step's fields to be offered, what it does first, got ${step.slice(0, 5)}`, step.slice(0, 5).join() === 'do,call,spawn,task,with');
  await page.screenshot({ path: `${out}/blueprint-2-fields.png` });

  // Taking `do` goes straight on to the actions this device has.
  await take('do');
  const actions = await offering('channel.post');
  expect(`the actions to be offered after \`do\`, got ${actions.length} of them`, actions.includes('shell') && actions.includes('agent.consult') && actions.length > 20);
  // The whole word, since its first letters are also in the names of
  // actions that are not a channel's (`place.changed`, `chain.next`).
  await page.keyboard.type('channel');
  await page.waitForFunction(() => [...document.querySelectorAll('.cm-tooltip-autocomplete li .cm-completionLabel')].every((label) => label.textContent.startsWith('channel.')));
  await page.keyboard.type('.p');
  await page.waitForFunction(() => document.querySelector('.cm-tooltip-autocomplete li[aria-selected] .cm-completionLabel')?.textContent === 'channel.post');
  // Beside the one that is marked: what it is, what it takes and gives back.
  const about = page.locator('.cm-completionInfo .bp-note');
  await about.waitFor();
  const said = await about.innerText();
  expect(`the marked action to be explained, and it says ${JSON.stringify(said)}`, /Takes/i.test(said) && said.includes('channel') && /Needs/i.test(said) && said.includes('channels:write'));
  await page.screenshot({ path: `${out}/blueprint-3-actions.png` });
  await take('channel.post');
  expect(`the action to be written, and the line is ${JSON.stringify(await line())}`, (await line()).trim() === 'do: channel.post');

  // Its arguments: `with` opens a line for them and offers the ones it must have first.
  await page.keyboard.press('Enter');
  await page.keyboard.type('wi');
  await offering('with');
  await take('with');
  const args = await offering('channel');
  expect(`the action's arguments, needed ones first, got ${args}`, args.slice(0, 2).join() === 'channel,body' && args.includes('thread'));
  const details = await popup.locator('li').first().innerText();
  expect(`an argument to say what it is, got ${JSON.stringify(details)}`, details.includes('string, needed'));
  await page.screenshot({ path: `${out}/blueprint-4-arguments.png` });
  await take('channel');
  await page.keyboard.type('ops');
  await page.keyboard.press('Enter');
  await page.keyboard.type('bo');
  const rest = await offering('body');
  expect(`an argument already given not to be offered again, got ${rest}`, !rest.includes('channel'));
  await take('body');

  // In an expression: the names there are, this blueprint's steps, and what a step's action gives back.
  await page.keyboard.type('"shipped ${ ');
  const starts = await offering('params');
  expect(`an expression to start with a name or a function, got ${starts.slice(0, 4)}`, starts.slice(0, 3).join() === 'params,state,steps' && starts.includes('slug'));
  await page.keyboard.type('st');
  await offering('steps');
  await page.keyboard.type('eps.');
  const above = await offering('build');
  expect(`the steps above this one, got ${above}`, above.join() === 'build,host');
  await take('build');
  await page.keyboard.type('.');
  const has = await offering('out');
  expect(`what a step has, got ${has}`, has.slice(0, 5).join() === 'ok,failed,skipped,status,out');
  await page.keyboard.type('out.');
  const gives = await offering('code');
  expect(`what \`shell\` gives back, got ${gives}`, gives.join() === 'code,stdout,stderr,duration_ms');
  await page.screenshot({ path: `${out}/blueprint-5-expression.png` });
  await take('code');
  await page.keyboard.type(' }"');
  expect(`the line as it was written, and it is ${JSON.stringify(await line())}`, (await line()).trim() === 'body: "shipped ${ steps.build.out.code }"');

  // A few words to choose from, each saying what it means; and what was written checks.
  await page.keyboard.press('Enter');
  await page.keyboard.press('Backspace');
  await page.keyboard.type('n');
  await offering('needs');
  await take('needs');
  const others = await offering('host');
  expect(`the other steps to be offered to \`needs\`, got ${others}`, others.join() === 'build,host');
  await take('build');
  expect(`the list to be written in its brackets, and the line is ${JSON.stringify(await line())}`, (await line()).trim() === 'needs: [build]');
  await checks();
  expect('nothing wrong with what was written', (await page.locator('.problems').count()) === 0);
  // A pill says what it means where it is pointed at.
  const checkSaid = await page.locator('.view-head .badge.ok', { hasText: 'checks' }).getAttribute('title');
  expect(`the pill that says it checks to say what that means, and it says ${JSON.stringify(checkSaid)}`, (checkSaid ?? '').includes('It can be applied'));

  // ---- what is said about a word ----------------------------------------------
  // What is said about one word may lie over the next, so the pointer
  // leaves the text between them, which takes the last one away.
  const note = page.locator('.cm-tooltip-hover .bp-note');
  const pointAt = async (kind, word, says) => {
    await page.mouse.move(5, 5);
    await note.waitFor({ state: 'detached' });
    await page.locator(`.cm-content .bp-${kind}`, { hasText: new RegExp(`^${word}$`) }).first().hover();
    await note.waitFor();
    expect(`pointing at \`${word}\` to say ${JSON.stringify(says)}, and it says ${JSON.stringify(await note.innerText())}`, (await note.innerText()).includes(says));
  };
  await pointAt('action', 'shell', 'Run a command and wait for it to finish.');
  await page.screenshot({ path: `${out}/blueprint-6-pointed-at.png` });
  await pointAt('field', 'timeout', 'Fail the step if it runs longer');
  expect('a field to say what goes after it', (await note.innerText()).includes('a length of time'));
  await pointAt('blueprint', 'deploy', 'Put a version where people can reach it.');
  await pointAt('member', 'ok', 'True when it finished without failing.');
  await page.mouse.move(5, 5);

  // ---- what a button says of itself ---------------------------------------------
  // The button at the right of the bar: the system's tooltip would start
  // under the pointer and be cut off by the window's edge in the desktop app.
  const apply = page.locator('.view-head button.primary');
  const title = await apply.getAttribute('title');
  await apply.hover();
  const shown = page.locator('.tip-said');
  await shown.waitFor();
  expect(`the button to say what it says, and it says ${JSON.stringify(await shown.innerText())}`, (await shown.innerText()) === title);
  const box = await shown.boundingBox();
  expect(`all of it to be inside the window, and it is at ${JSON.stringify(box)}`, box.x >= 0 && box.x + box.width <= 1280 && box.y >= 0 && box.y + box.height <= 860);
  expect('the system to have nothing to say of its own meanwhile', (await apply.getAttribute('title')) === null);
  await page.screenshot({ path: `${out}/blueprint-7-button-says.png` });
  await page.mouse.move(5, 5);
  await shown.waitFor({ state: 'detached' });
  expect('the button to be as it was written once the pointer has gone', (await apply.getAttribute('title')) === title);

  // ---- laid out the one way ------------------------------------------------------
  // A step written carelessly under the last one, and the cursor left in
  // the middle of one of its words.
  const rough = '  rough: {do:   noop,when: ${ params.channel!="" }}';
  // Its `${ }` stood bare inside braces, which YAML does not read: laid
  // out it is in quotes, and the cursor is still where it was.
  const laidOut = 'rough: { do: noop, when: \'${ params.channel!="" }\' }';
  await text.click();
  await page.keyboard.press('Control+End');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Shift+Home');
  await page.keyboard.type(rough);
  await page.keyboard.press('Escape');
  for (let left = rough.length - rough.indexOf('noop') - 2; left > 0; left -= 1) await page.keyboard.press('ArrowLeft');
  const lineIs = (wanted) => page.waitForFunction((want) => document.querySelector('.cm-activeLine')?.textContent.trim() === want, wanted, { timeout: 8000 });
  const says = async (what, wanted) => {
    await lineIs(wanted).catch(async () => {
      throw new Error(`expected ${what}, and the line the cursor is in reads ${JSON.stringify(await line())}`);
    });
  };
  // Its own key lays the whole text out, as its project says, and saves nothing.
  await page.keyboard.press('Shift+Alt+F');
  await says('the careless step laid out the one way by its key', laidOut);
  expect('laying it out not to have saved it', (await page.locator('.view-head').innerText()).includes('unsaved'));
  // One step back is the text as it was typed; and it can be laid out again.
  await page.keyboard.press('Control+z');
  await says('undoing once to give back exactly what was typed', rough.trim());
  await page.keyboard.press('Shift+Alt+F');
  await says('it laid out again', laidOut);
  // The cursor is where it was in the text: between the same two letters.
  await page.keyboard.type('X');
  await says('the cursor still in the middle of the word it was in', laidOut.replace('noop', 'noXop'));
  await page.keyboard.press('Backspace');
  await says('the word as it was', laidOut);
  await page.screenshot({ path: `${out}/blueprint-8-laid-out.png` });

  // ---- and it is the file ------------------------------------------------------
  // Saving lays it out too: spaces left at the end of a line are not kept.
  await page.keyboard.press('End');
  await page.keyboard.type('   ');
  await page.keyboard.press('Control+s');
  await page.waitForFunction(() => !document.querySelector('.view-head')?.textContent.includes('unsaved'));
  const written = await readFile(join(where, `blueprints/release.${ending}`), 'utf8');
  const tail = written.slice(written.indexOf('  tell:'));
  const wanted = `  tell:\n    do: channel.post\n    with:\n      channel: ops\n      body: "shipped \${steps.build.out.code}"\n    needs: [build]\n\n  ${laidOut}`;
  expect(`the steps as they were written, laid out the one way, to be in the file, which ends ${JSON.stringify(tail)}`, tail === `${wanted}\n`);
  expect('the rest of the file to be laid out too', written.includes('      version: ${params.version}\n') && !/[ \t]\n/.test(written));
  const checked = await cli('check', join(where, `blueprints/release.${ending}`), '--project', 'shop').then(
    (done) => done.stdout.trim(),
    (err) => `${err.stdout}${err.stderr}`,
  );
  expect(`the daemon's own check to pass it, and it says ${checked}`, checked.endsWith(': ok'));

  // ---- applying lays it out first ------------------------------------------------
  // The other file is as it was first written. Applying it saves it, and
  // what is saved and applied is laid out the one way.
  await unfold(page, page.locator('.nav button.nav-item', { has: page.getByText('Blueprints', { exact: true }) }).last());
  // The daemon's check of the text is held until the head has been measured, so that
  // it is answered after the head is drawn, as it is on a busy machine: nothing of
  // the head moves for the answer, under a pointer about to press Apply.
  let letCheck;
  const checkHeld = new Promise((go) => (letCheck = go));
  await page.route('**/api/blueprints/check', async (route) => {
    await checkHeld;
    await route.continue();
  });
  await page.locator('.nav').getByText('deploy', { exact: true }).click();
  await page.locator('.view-head').getByText('deploy', { exact: false }).first().waitFor();
  await page.locator('.cm-content').last().waitFor();
  await page.locator('.view-head').getByText('checking', { exact: true }).waitFor();
  const applyBefore = await page.locator('.view-head button.primary').last().boundingBox();
  expect('Apply to be closed while the text is being checked, since what it takes is not known yet', await page.locator('.view-head button.primary').last().isDisabled());
  letCheck();
  await page.locator('.view-head').getByText('checks', { exact: true }).waitFor();
  await page.unroute('**/api/blueprints/check');
  const applyAfter = await page.locator('.view-head button.primary').last().boundingBox();
  expect(
    `Apply to stay where it was as the text is said to check, and it moved from ${JSON.stringify(applyBefore)} to ${JSON.stringify(applyAfter)}`,
    Math.abs(applyBefore.x - applyAfter.x) < 1 && Math.abs(applyBefore.y - applyAfter.y) < 1,
  );
  await page.locator('.view-head button.primary').last().click();
  // What it is applied with is asked in the drawer at the side, and in no
  // dialog over the text: the text is still read beside it.
  const asks = drawerAtRest(page);
  await asks.getByRole('heading', { name: /^Apply / }).waitFor();
  expect('what a blueprint is applied with to be asked in the drawer and in no dialog over its text', (await page.getByRole('dialog').count()) === 0);
  const version = asks.getByLabel(/^version/);
  expect('a param the text gives a value to start with it', (await asks.getByLabel(/^port/).inputValue()) === '8080');
  expect('what the text says of a param to be said with it', (await asks.locator('.comparing-said', { hasText: 'which version to run' }).count()) === 1);
  // Each thing asked is its name, its box under that and what it means
  // under the box; nothing of the form lies over anything else of it or
  // has to be scrolled sideways for; what it is left by is at the
  // drawer's foot; and the text's lines begin clear of the drawer.
  const laid = () =>
    page.evaluate(() => {
      const drawer = document.querySelector('.drawer');
      const body = drawer.querySelector('.drawer-body');
      const form = drawer.querySelector('.applying');
      const box = (el) => {
        const r = el.getBoundingClientRect();
        const said = (el.getAttribute('aria-label') ?? el.textContent ?? '').trim().slice(0, 40);
        return { what: `${el.tagName.toLowerCase()} "${said}"`, left: r.left, right: r.right, top: r.top, bottom: r.bottom };
      };
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const pairs = (boxes) => boxes.flatMap((a, at) => boxes.slice(at + 1).filter((b) => over(a, b)).map((b) => `${a.what} over ${b.what}`));
      const foot = form.querySelector('.comparing-foot');
      const fields = [...form.querySelectorAll('.comparing-about, .comparing-label, .comparing-said, input, select, textarea, button')].filter((el) => !foot.contains(el));
      const left = [...foot.querySelectorAll('button')];
      const within = body.getBoundingClientRect();
      const seen = foot.getBoundingClientRect();
      // Each field in order down the drawer: its name, then its box, then what it means.
      const order = [...form.querySelectorAll('label.comparing-field')].filter((field) => {
        const tops = [field.querySelector('.comparing-label'), field.querySelector('input, select'), field.querySelector('.comparing-said')].filter(Boolean).map((el) => el.getBoundingClientRect());
        return tops.some((one, at) => at > 0 && one.top < tops[at - 1].bottom - 0.5);
      });
      const line = document.querySelector('.view:not([hidden]) .cm-line');
      return {
        fields: fields.length,
        over: [...pairs(fields.map(box)), ...pairs(left.map(box))],
        outside: [...fields, ...left].map(box).filter((one) => one.left < within.left - 0.5 || one.right > within.right + 0.5).map((one) => one.what),
        sideways: body.scrollWidth - body.clientWidth,
        page: document.documentElement.scrollWidth - window.innerWidth,
        foot: seen.bottom <= window.innerHeight + 0.5 && seen.top >= 0 && seen.bottom <= drawer.getBoundingClientRect().bottom + 0.5,
        order: order.length,
        text: line ? line.getBoundingClientRect().left : null,
        drawer: drawer.getBoundingClientRect().left,
      };
    });
  for (const width of [1280, 1024, 390]) {
    await page.setViewportSize({ width, height: 860 });
    await asks.locator('.applying').waitFor();
    const found = await laid();
    const how = `at ${width} wide`;
    expect(`the form to have its fields ${how}, got ${found.fields}`, found.fields >= 6);
    expect(`no two things of the form to lie over each other ${how}, got ${found.over.join('; ')}`, found.over.length === 0);
    expect(`everything of the form to be within the drawer ${how}, got ${found.outside.join('; ')}`, found.outside.length === 0);
    expect(`each thing asked to be its name, its box and what it means, in that order down ${how}`, found.order === 0);
    expect(`the drawer not to scroll sideways ${how}, by ${found.sideways}`, found.sideways <= 0);
    expect(`the page not to scroll sideways ${how}, by ${found.page}`, found.page <= 0);
    expect(`what the form is left by to be in reach at the drawer's foot ${how}`, found.foot);
    // On a phone the drawer has the screen; anywhere wider the text's lines begin to the left of it.
    if (width > 760) expect(`the text to be read beside the drawer ${how}: its lines begin at ${found.text}, the drawer at ${found.drawer}`, found.text !== null && found.text + 200 < found.drawer);
    // There the drawer has the screen, and what it is left by is large enough for a finger.
    else for (const name of ['Not now', 'Apply']) {
      const button = await asks.getByRole('button', { name, exact: true }).boundingBox();
      expect(`${name} to be large enough to touch ${how}, and it is ${Math.round(button.width)} by ${Math.round(button.height)}`, button.width >= 34 && button.height >= 34);
    }
    await page.screenshot({ path: `${out}/blueprint-8a-applied-with-at-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // At a phone's width the side panel is put away; it is shown again for what follows.
  await side(page, 'Resources');

  // ---- the head beside an open drawer ----------------------------------------------
  // The page is laid out within the room left of an open drawer, so that Testing and Apply are never under it: with
  // each drawer open, everything of the head is left of the drawer's edge, the two are pressed by the pointer, and
  // nothing scrolls sideways, at 1280, 1600 and 900 wide (with the side panel open beside the page).
  const viewHead = page.locator('.view-head').locator('visible=true').last();
  const besideEach = async (what) => {
    for (const width of [1280, 1600, 900]) {
      await page.setViewportSize({ width, height: 860 });
      await drawerAtRest(page).waitFor();
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
      const found = await besideDrawer(page);
      expect(`nothing of the head under the drawer with ${what} open at ${width} wide, and Testing and Apply pressed there, got ${found.wrong.slice(0, 4).join('; ')}`, found.wrong.length === 0 && found.pressed.length === 2);
      await page.screenshot({ path: `${out}/blueprint-8c-beside-${what.replace(/\W+/g, '-')}-${width}.png` });
      // Testing pressed by the pointer, its menu over the page, and put away.
      await testingOf(viewHead).click();
      await testingMenu(page).waitFor();
      await closeTesting(page);
      expect(`the drawer with ${what} to stay as Testing's menu is put away at ${width} wide`, (await page.locator('.main > .drawer').count()) === 1);
    }
    await page.setViewportSize({ width: 1280, height: 860 });
  };
  await besideEach('what it is applied with');
  // Previewed, and then Apply pressed by the pointer beside the preview, as somebody does.
  await pressTesting(page, viewHead, 'Preview');
  await drawerAtRest(page).locator('.preview').waitFor();
  await besideEach('its preview');
  // A step of its graph opened in the drawer.
  await page.getByRole('tab', { name: 'Node' }).click();
  await page.locator('.gnode').first().click();
  await drawerAtRest(page).locator('.drawer-body').waitFor();
  expect('a step of the graph to be opened in the drawer', (await page.locator('.main > .drawer .preview').count()) === 0);
  await besideEach('a step of its graph');
  await page.getByRole('tab', { name: 'Text' }).click();
  await pressTesting(page, viewHead, 'Preview');
  await drawerAtRest(page).locator('.preview').waitFor();
  await viewHead.locator('button.primary').click();
  await asks.getByRole('heading', { name: /^Apply / }).waitFor();
  // What is written is kept when another tab is shown, which puts the
  // drawer away, and is there when Apply is pressed again.
  await version.fill('v2');
  await asks.getByLabel(/^port/).fill('8081');
  await page.locator('.tab', { hasText: 'release' }).first().click();
  await asks.waitFor({ state: 'detached' });
  await page.locator('.tab', { hasText: 'deploy' }).first().click();
  // The drawer is the tab's, and is given back with it; Apply, beside it, is pressed by the pointer all the same, and
  // asks again. Apply is closed until the text is checked, and a press waits for it to open.
  await page.locator('.view-head button.primary').last().click();
  await asks.getByRole('heading', { name: /^Apply / }).waitFor();
  expect(`what was written to be there again, got ${await version.inputValue()} and ${await asks.getByLabel(/^port/).inputValue()}`, (await version.inputValue()) === 'v2' && (await asks.getByLabel(/^port/).inputValue()) === '8081');
  // Something that cannot be read as what the param is, is said in the drawer, which stays; put right, it is applied.
  await asks.getByLabel(/^port/).fill('eighty');
  await asks.getByRole('button', { name: 'Apply', exact: true }).click();
  await asks.getByText('`port` should be a number.').waitFor();
  await asks.getByLabel(/^port/).fill('8080');
  // The other views of the blueprint are read with the drawer still beside them.
  await page.getByRole('tab', { name: 'Node' }).click();
  await page.locator('.gnode').first().waitFor();
  expect(`the drawer to stay while another view of the blueprint is read, got ${await asks.count()} drawer and ${await page.locator('.drawer').count()} any, and ${await version.inputValue().catch(() => 'none')}`, (await asks.count()) === 1 && (await version.inputValue()) === 'v2');
  await page.screenshot({ path: `${out}/blueprint-8b-applied-with-beside-the-graph.png` });
  await page.getByRole('tab', { name: 'Text' }).click();
  await asks.getByRole('button', { name: 'Apply', exact: true }).click();
  await page.getByText('Started deploy').waitFor();
  await asks.waitFor({ state: 'detached' });
  // Applied, what was written is let go: asked again, it starts from what is running.
  await page.locator('.view-head button.primary').last().click();
  await asks.getByRole('heading', { name: /^Apply / }).waitFor();
  // What is running is read a moment after it is started, and the form follows it.
  await page.waitForFunction(() => [...document.querySelectorAll('.drawer .applying input')].some((box) => box.value === 'v2'), null, { timeout: 5000 }).catch(() => {});
  expect(`the form to start from what it was applied with, got ${await version.inputValue()}`, (await version.inputValue()) === 'v2');
  await asks.getByRole('button', { name: 'Not now', exact: true }).click();
  await asks.waitFor({ state: 'detached' });
  // The pill of what is running from it says what that means too.
  const runningSaid = await page.locator('.view-head .badge', { hasText: /^(running|starting|done)$/ }).first().getAttribute('title');
  expect(`the pill of what runs from it to say what that means, and it says ${JSON.stringify(runningSaid)}`, /^It (has been|was) applied/.test(runningSaid ?? ''));
  const deployed = await readFile(join(where, `blueprints/deploy.${ending}`), 'utf8');
  const tidy =
    'blueprint: deploy\ndescription: Put a version where people can reach it.\n\nparams:\n  version: { type: string, description: which version to run }\n  port: 8080\n\n' +
    'steps:\n  go: { do: noop }\n\noutputs:\n  address: "127.0.0.1:${params.port}"\n';
  expect(`the file that was applied to be laid out the one way, and it is ${JSON.stringify(deployed)}`, deployed === tidy);
  // What was laid out is what was saved: nothing is left unsaved by it.
  await page.locator('.tab-edited').waitFor({ state: 'detached' });
  await page.locator('.view-head').getByText('Text', { exact: true }).last().click();
  await page.locator('.cm-content').last().getByText('outputs').waitFor();
  expect('the text shown to be the one that was saved', !(await page.locator('.view-head').last().innerText()).includes('unsaved'));
  await page.screenshot({ path: `${out}/blueprint-9-applied.png` });

  // ---- a run's steps, and what each gave ----------------------------------------
  // The one step of that run went well and gave nothing: the list of its
  // steps has no column for how they came out, which would stand empty.
  const columns = () => page.locator('.runs-beside thead th').allInnerTexts().then((all) => all.map((one) => one.trim().toLowerCase()));
  await page.getByRole('tab', { name: 'Runs' }).last().click();
  await page.locator('.runs-beside tbody tr', { hasText: 'go' }).waitFor();
  expect(`no column for results where no step of the run gave anything, got ${await columns()}`, (await columns()).join() === 'step,status,took');
  await page.screenshot({ path: `${out}/blueprint-9a-steps-that-gave-nothing.png` });
  // Another blueprint calls that one and then fails. The step that called
  // says in a line what it gave, the one that failed says why, and the
  // whole of either is opened under its line by pressing it.
  await writeFile(join(where, `blueprints/ship.${ending}`), SHIP);
  await cli('apply', join(where, `blueprints/ship.${ending}`), '--project', 'shop').catch(() => {});
  await page.locator('.nav').getByText('ship', { exact: true }).click();
  await page.locator('.view-head').getByText('ship', { exact: false }).first().waitFor();
  await page.getByRole('tab', { name: 'Runs' }).last().click();
  const stepRow = (name) => page.locator('.runs-beside tbody tr.clickable', { hasText: name });
  await stepRow('checked').locator('.badge', { hasText: 'failed' }).waitFor();
  expect(`a column for results where a step gave something, got ${await columns()}`, (await columns()).join() === 'step,status,took,result');
  const gaveOut = stepRow('host').locator('.run-gave');
  const whyNot = stepRow('checked').locator('.run-gave');
  const inALine = (await gaveOut.innerText()).trim();
  expect(`the step that called to say in a line what it gave, got "${inALine}"`, /\binstance: in_\w+/.test(inALine) && inALine.includes('address: 127.0.0.1:8080') && !inALine.includes('{'));
  expect(`the step that failed to say why, got "${(await whyNot.innerText()).trim()}"`, (await whyNot.innerText()).trim().startsWith('nobody looked at the address'));
  expect('nothing to be opened until it is pressed', (await page.locator('.run-whole').count()) === 0 && (await gaveOut.getAttribute('aria-expanded')) === 'false');
  // A line too long for its place is cut, inside the list, and nothing scrolls sideways for it.
  const fitted = await page.evaluate(() => {
    const list = document.querySelector('.runs-beside table').getBoundingClientRect();
    const cells = [...document.querySelectorAll('.runs-beside .run-gave')].map((one) => one.getBoundingClientRect());
    const cut = [...document.querySelectorAll('.runs-beside .run-gave .truncate')].some((one) => one.scrollWidth > one.clientWidth);
    const pane = document.querySelector('.runs-beside').lastElementChild;
    return { inside: cells.every((one) => one.left >= list.left - 0.5 && one.right <= list.right + 0.5), cut, sideways: pane.scrollWidth - pane.clientWidth, page: document.documentElement.scrollWidth - window.innerWidth };
  });
  expect(`what is said of each step to be inside the list, the long one cut, and nothing scrolled sideways for, got ${JSON.stringify(fitted)}`, fitted.inside && fitted.cut && fitted.sideways <= 0 && fitted.page <= 0);
  await page.screenshot({ path: `${out}/blueprint-9b-what-steps-gave.png` });
  await gaveOut.click();
  const opened = page.locator('.run-whole pre');
  await opened.waitFor();
  expect(`the whole of what it gave to be opened under its line, got ${JSON.stringify(await opened.innerText())}`, (await opened.innerText()).includes('"address": "127.0.0.1:8080"') && (await gaveOut.getAttribute('aria-expanded')) === 'true');
  const [itsLine, under] = [await stepRow('host').boundingBox(), await page.locator('.run-whole').boundingBox()];
  expect('it to be under the line of the step that gave it', under.y >= itsLine.y + itsLine.height - 1 && under.y <= itsLine.y + itsLine.height + 1);
  expect('pressing for the whole not to have chosen the step for its log', (await stepRow('host').getAttribute('aria-selected')) !== 'true');
  await whyNot.click();
  await page.locator('.run-whole pre.error-text').waitFor();
  expect('the whole of why it failed to be opened under its line', (await page.locator('.run-whole pre.error-text').innerText()).includes('theirs to open') && (await page.locator('.run-whole').count()) === 2);
  await page.screenshot({ path: `${out}/blueprint-9c-the-whole-of-each.png` });
  await gaveOut.click();
  expect('pressed again, the whole of it to be put away', (await page.locator('.run-whole').count()) === 1);
  // The line itself is still pressed for the step's log.
  await stepRow('host').locator('td').first().click();
  expect('the line of a step to be pressed for its log still', (await stepRow('host').getAttribute('aria-selected')) === 'true');

  // ---- the id of something running, wherever one is written ----------------------
  // What the step that called gave names what it started by its id. The
  // id is said with what it is beside it, the blueprint and the key, in
  // the step's line and in the whole of what it gave; one that names
  // nothing is said to be gone, and is nothing to press.
  const started = /in_\w+/.exec(inALine)[0];
  await gaveOut.locator('.running-is').waitFor();
  expect(`the step's line to say what the id it gave is of, got "${(await gaveOut.innerText()).trim()}"`, (await gaveOut.innerText()).includes(`${started} deploy · shipped`));
  expect('nothing in the line to be pressed but the line', (await gaveOut.locator('button').count()) === 0);
  await gaveOut.click();
  const named = page.locator('.run-whole button.running-id');
  await named.locator('.running-is').waitFor();
  expect(`the whole of what it gave to say what the id is of, got "${(await named.innerText()).trim()}"`, (await named.innerText()).trim() === `${started} deploy · shipped`);
  const gone = page.locator('.run-whole .running-id.gone');
  await gone.waitFor();
  expect(`an id that names nothing to be said to be gone, got "${(await gone.innerText()).trim()}"`, (await gone.innerText()).trim() === 'in_1aaaaaaaaaaaaaa9 gone');
  expect('one that is gone to be nothing to press', (await gone.evaluate((one) => one.tagName)) === 'SPAN' && (await page.locator('.runs-beside button.running-id', { hasText: 'in_1aaaaaaaaaaaaaa9' }).count()) === 0);
  expect(`the step's line to say it is gone too, got "${(await whyNot.innerText()).trim()}"`, (await whyNot.innerText()).includes('in_1aaaaaaaaaaaaaa9 gone'));
  expect('no word that only begins as an id does to be taken for one', (await page.locator('.runs-beside .running-id').count()) === 5);
  await page.screenshot({ path: `${out}/blueprint-9d-ids-said-for-what-they-are.png` });
  // The same where the step is read on the graph: in what it gave in its run, opened from there.
  await page.getByRole('tab', { name: 'Node' }).last().click();
  await page.locator('.gnode', { hasText: 'host' }).last().click();
  const inDrawer = drawerAtRest(page).locator('.step-gave button.running-id');
  await inDrawer.locator('.running-is').waitFor();
  expect(`what a step gave, read on the graph, to say what the id is of, got "${(await inDrawer.innerText()).trim()}"`, (await inDrawer.innerText()).trim() === `${started} deploy · shipped`);
  await page.screenshot({ path: `${out}/blueprint-9e-an-id-on-the-graph.png` });
  // Pressed, it is opened: a tab called what it is and not by its id.
  await inDrawer.click();
  const itsTab = page.locator('.tab[aria-selected="true"]', { hasText: 'deploy · shipped' });
  await itsTab.waitFor();
  expect('no tab to be called by an id', (await page.locator('.tab', { hasText: /in_[0-9a-z]{10,}/ }).count()) === 0);
  await page.locator('.view-head').getByText('shipped', { exact: false }).last().waitFor();
  // And what started it, in its details, is said the same way and opened the same way.
  await page.getByRole('button', { name: 'Details', exact: true }).last().click();
  const by = drawerAtRest(page).locator('button.running-id');
  await by.locator('.running-is').waitFor();
  expect(`what started it to be said for what it is, got "${(await by.innerText()).trim()}"`, /^in_\w+ ship · \S+$/.test((await by.innerText()).trim()));
  await page.screenshot({ path: `${out}/blueprint-9f-what-started-it.png` });
  await by.click();
  await page.locator('.tab[aria-selected="true"]', { hasText: 'ship · ' }).waitFor();


  // ---- a run's steps, said by what each says of itself ---------------------------
  {
  // A blueprint whose steps say what they are for, but for one, with a
  // handler that runs once it is there. Its runs are listed by what each
  // is of: its own steps in words, with what the text calls them under
  // that, smaller; a handler's by its handler, which has nowhere to say
  // what it is for.
  await writeFile(join(where, `blueprints/told.${ending}`), TOLD);
  await cli('apply', join(where, `blueprints/told.${ending}`), '--project', 'shop');
  await page.locator('.nav').getByText('told', { exact: true }).click();
  await page.locator('.view-head').getByText('told', { exact: false }).first().waitFor();
  await page.getByRole('tab', { name: 'Runs' }).last().click();
  const line = (words) => page.locator('.runs-beside .run-line', { has: page.locator('.run-of', { hasText: new RegExp(`^${words}$`) }) });
  await line('Its steps').locator('.badge', { hasText: 'ok' }).waitFor();
  await line('looked').locator('.badge', { hasText: 'ok' }).waitFor();
  const listed = await page.evaluate(() =>
    [...document.querySelectorAll('.runs-beside .run-line')].map((one) => {
      const [words, called] = [one.querySelector('.run-of'), one.querySelector('.run-called')];
      return {
        words: words.textContent.trim(),
        called: called?.textContent.trim() ?? null,
        smaller: called ? parseFloat(getComputedStyle(called).fontSize) < parseFloat(getComputedStyle(words).fontSize) : null,
        under: called ? called.getBoundingClientRect().top >= words.getBoundingClientRect().bottom - 0.5 : null,
        inside: [words, called].filter(Boolean).every((el) => el.getBoundingClientRect().right <= one.getBoundingClientRect().right + 0.5 && el.getBoundingClientRect().left >= one.getBoundingClientRect().left - 0.5),
      };
    }),
  );
  const own = listed.find((one) => one.words === 'Its steps');
  expect(`a run of the text's own steps to be said in words, with what the text calls them under that, smaller, got ${JSON.stringify(listed)}`, own && own.called === 'main' && own.smaller && own.under && own.inside);
  const heard = listed.find((one) => one.words === 'looked');
  expect(`a handler's run to be called by its handler and nothing more, got ${JSON.stringify(listed)}`, heard && heard.called === null && listed.length === 2);

  // The steps of a run: what each says of itself as far as its first
  // full stop, its key under that, smaller; one that says nothing as it
  // always was.
  const steps = () =>
    page.evaluate(() => {
      const pane = document.querySelector('.runs-beside').lastElementChild;
      const box = (el) => el.getBoundingClientRect();
      const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
      const rows = [...pane.querySelectorAll('tbody tr.clickable')].map((row) => {
        const cell = row.querySelector('td');
        const [says, key] = [cell.querySelector('.step-says'), cell.querySelector('code')];
        const said = [says, key].filter(Boolean);
        // What a line says: the step's words and key, and what stands in each cell after them.
        const parts = [...said, ...[...row.querySelectorAll('td')].slice(1).flatMap((one) => [...one.children])].map(box).filter((one) => one.width > 0);
        return {
          says: says?.textContent.trim() ?? null,
          key: key.textContent.trim(),
          marked: key.className,
          under: says ? box(key).top >= box(says).bottom - 0.5 : null,
          smaller: says ? parseFloat(getComputedStyle(key).fontSize) < parseFloat(getComputedStyle(says).fontSize) : null,
          outside: said.filter((el) => box(el).left < box(cell).left - 0.5 || box(el).right > box(cell).right + 0.5 || box(el).top < box(cell).top - 0.5 || box(el).bottom > box(cell).bottom + 0.5).length,
          crossed: parts.flatMap((a, at) => parts.slice(at + 1).filter((b) => over(a, b))).length,
          tall: Math.round(says ? box(says).height : 0),
        };
      });
      const [table, within] = [box(pane.querySelector('table')), box(pane)];
      return { rows, list: table.left >= within.left - 0.5 && table.right <= within.right + 0.5, sideways: pane.scrollWidth - pane.clientWidth, page: document.documentElement.scrollWidth - window.innerWidth };
    });
  const row = (found, key) => found.rows.find((one) => one.key === key);
  await line('Its steps').click();
  await page.locator('.runs-beside tbody .step-says', { hasText: 'The shelves are counted' }).waitFor();
  const wide = await steps();
  expect(`a step to be said by what it says of itself, as far as its first full stop, got ${JSON.stringify(row(wide, 'ready'))}`, row(wide, 'ready')?.says === 'The shelves are counted before anything is put out.');
  expect(`its key to be under that, smaller, got ${JSON.stringify(row(wide, 'ready'))}`, row(wide, 'ready').under && row(wide, 'ready').smaller && row(wide, 'ready').marked === 'step-key');
  expect(`a step that says nothing of itself to be shown as it always was, got ${JSON.stringify(row(wide, 'plain'))}`, row(wide, 'plain') && row(wide, 'plain').says === null && row(wide, 'plain').marked === '');
  const long = row(wide, 'shelved');
  expect(`a step that says one long sentence to be said by the whole of it, got ${JSON.stringify(long)}`, long && long.says.startsWith('The shelves are filled one after another') && long.says.endsWith('one line of a list.') && long.under);
  expect(`no column for results where no step gave anything, as it was, got ${await columns()}`, (await columns()).join() === 'step,status,took');
  await page.screenshot({ path: `${out}/blueprint-9i-steps-that-say.png` });
  // The key is still what a line is found and pressed by, for the step's log.
  await page.locator('.runs-beside tbody tr.clickable', { has: page.locator('code', { hasText: /^plain$/ }) }).click();
  await page.getByRole('heading', { name: 'Log of plain' }).waitFor();
  await page.locator('.runs-beside tbody tr.clickable', { has: page.locator('code', { hasText: /^ready$/ }) }).locator('.step-says').click();
  await page.getByRole('heading', { name: 'Log of ready' }).waitFor();
  // Nothing a line says lies over anything else it says or is outside its
  // place, and nothing scrolls sideways for it, down to a phone's width,
  // where the words go onto more lines.
  for (const width of [1280, 1024, 390]) {
    await page.setViewportSize({ width, height: 860 });
    await page.locator('.runs-beside tbody .step-says', { hasText: 'The shelves are counted' }).waitFor();
    const found = await steps();
    const how = `at ${width} wide`;
    expect(`every step of the run to be listed ${how}, got ${found.rows.map((one) => one.key)}`, found.rows.length === 3);
    expect(`what a step says and its key to be inside the step's own place ${how}, got ${JSON.stringify(found.rows.filter((one) => one.outside > 0))}`, found.rows.every((one) => one.outside === 0));
    expect(`nothing a line says to lie over anything else it says ${how}, got ${JSON.stringify(found.rows.filter((one) => one.crossed > 0))}`, found.rows.every((one) => one.crossed === 0));
    expect(`a key to stay under what its step says ${how}`, found.rows.every((one) => one.says === null || one.under));
    expect(`the list to be inside its place and nothing to scroll sideways ${how}, got ${JSON.stringify({ list: found.list, sideways: found.sideways, page: found.page })}`, found.list && found.sideways <= 0 && found.page <= 0);
    if (width === 390) {
      expect(`a long sentence to go onto more lines where there is less room, ${row(found, 'shelved').tall} tall against ${long.tall}`, row(found, 'shelved').tall > long.tall);
      await page.screenshot({ path: `${out}/blueprint-9j-steps-that-say-narrow.png` });
    }
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // At a phone's width the side panel is put away; it is shown again for what follows.
  await side(page, 'Resources');
  // A handler's run the same: a step whose key reads as bad news is said by what it is for.
  await line('looked').click();
  await page.locator('.runs-beside tbody .step-says', { hasText: 'The shelves are looked over.' }).waitFor();
  const handled = await steps();
  expect(`a handler's step to be said by what it says of itself with its key under it, got ${JSON.stringify(handled.rows)}`, handled.rows.length === 1 && handled.rows[0].says === 'The shelves are looked over.' && handled.rows[0].key === 'wrong' && handled.rows[0].under && handled.rows[0].smaller);
  await page.screenshot({ path: `${out}/blueprint-9k-a-handlers-step-that-says.png` });
  // What a step says of itself is its text's, and a run is of the text it
  // began with. The text written in again is another text until it is
  // applied: its words are not put to the keys of a run that was of the
  // one before, which is shown by its keys as every run was. Put back as
  // it was, it is the run's text again.
  await page.getByRole('tab', { name: 'Text' }).last().click();
  await page.locator('.cm-content').last().click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.type('# Written in since.');
  await page.keyboard.press('Enter');
  await page.locator('.view-head').getByText('unsaved').waitFor();
  await page.getByRole('tab', { name: 'Runs' }).last().click();
  await page.locator('.runs-beside tbody tr.clickable code', { hasText: /^wrong$/ }).waitFor();
  await page.locator('.runs-beside tbody .step-says').waitFor({ state: 'detached' });
  const other = await steps();
  expect(`a run of another text than the one written to be shown by its keys alone, got ${JSON.stringify(other.rows)}`, other.rows.length === 1 && other.rows[0].says === null && other.rows[0].key === 'wrong' && other.rows[0].marked === '');
  await page.getByRole('tab', { name: 'Text' }).last().click();
  await page.locator('.cm-content').last().click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.press('Shift+ArrowDown');
  await page.keyboard.press('Backspace');
  await page.locator('.view-head').getByText('unsaved').waitFor({ state: 'detached' });
  await page.getByRole('tab', { name: 'Runs' }).last().click();
  await page.locator('.runs-beside tbody .step-says', { hasText: 'The shelves are looked over.' }).waitFor();
  }

  // ---- what a step's tile says -----------------------------------------------------
  {
  // A step that says nothing of itself is drawn as it always was: its key,
  // and under it what it runs, in a tile of the height tiles had.
  const tiles = () =>
    page.evaluate(() =>
      [...document.querySelectorAll('.canvas .gnode')].map((tile) => {
        const box = tile.querySelector('rect.box').getBoundingClientRect();
        const lines = [...tile.querySelectorAll('text')].map((text) => ({ says: text.textContent.trim(), kind: text.getAttribute('class') ?? '', size: parseFloat(getComputedStyle(text).fontSize), box: text.getBoundingClientRect() }));
        const inside = (one) => one.box.left >= box.left - 0.5 && one.box.right <= box.right + 0.5 && one.box.top >= box.top - 0.5 && one.box.bottom <= box.bottom + 0.5;
        const over = (a, b) => Math.min(a.box.right, b.box.right) - Math.max(a.box.left, b.box.left) > 0.5 && Math.min(a.box.bottom, b.box.bottom) - Math.max(a.box.top, b.box.top) > 0.5;
        const scale = box.width / 196;
        return {
          node: tile.getAttribute('data-node'),
          tall: Math.round(box.height / scale),
          words: lines.filter((one) => one.kind === 'says').map((one) => one.says),
          key: lines.filter((one) => one.kind === '').map((one) => one.says),
          under: lines.find((one) => one.kind === 'sub')?.says ?? '',
          smaller: lines.filter((one) => one.kind === 'sub').every((sub) => lines.filter((one) => one.kind === 'says').every((one) => sub.size < one.size)),
          outside: lines.filter((one) => !inside(one)).map((one) => one.says),
          crossed: lines.flatMap((a, at) => lines.slice(at + 1).filter((b) => over(a, b)).map((b) => `${a.says} over ${b.says}`)),
        };
      }),
    );
  await page.locator('.tab', { hasText: 'deploy.' }).first().click();
  await page.getByRole('tab', { name: 'Node' }).last().click();
  await page.locator('.gnode[data-node="main/go"]').waitFor();
  const asItWas = (await tiles()).find((one) => one.node === 'main/go');
  expect(`a step that says nothing of itself to be drawn as it was, got ${JSON.stringify(asItWas)}`, asItWas.tall === 52 && asItWas.key.join() === 'go' && asItWas.under === 'noop' && asItWas.words.length === 0 && asItWas.outside.length === 0 && asItWas.crossed.length === 0);
  // The fleet that ships says what many of its steps are for. Each such
  // tile says that, on two lines at most, with the step's key under it,
  // smaller, before what it runs; nothing a tile says is outside the tile
  // or over anything else it says; and every tile there is as tall as
  // the others.
  await unfold(page, page.locator('.nav button.nav-item', { has: page.getByText('std', { exact: true }) }));
  await page.locator('.nav').getByText('std/fleet', { exact: true }).click();
  await page.locator('.view-head').getByText('std/fleet', { exact: false }).first().waitFor();
  await page.getByRole('tab', { name: 'Node' }).last().click();
  await page.locator('.gnode').first().waitFor();
  const drawn = await tiles();
  const said = drawn.filter((one) => one.words.length > 0);
  expect(`many steps of the fleet to say what they are for, and some to say nothing, got ${said.length} of ${drawn.length}`, said.length >= 8 && said.length < drawn.length);
  expect(`a tile to say what its step is for on two lines at most, got ${JSON.stringify(said.find((one) => one.words.length > 2))}`, said.every((one) => one.words.length <= 2));
  expect(`a step's key to be under what it says, smaller and before what it runs, got ${JSON.stringify(said.find((one) => !one.smaller || !one.under.includes(' · ')))}`, said.every((one) => one.smaller && one.under.startsWith(`${one.node.split('/').pop().slice(0, 8)}`) && one.under.includes(' · ')));
  expect(`nothing a tile says to be outside it, got ${JSON.stringify(drawn.filter((one) => one.outside.length > 0).map((one) => one.outside))}`, drawn.every((one) => one.outside.length === 0));
  expect(`nothing a tile says to lie over anything else it says, got ${JSON.stringify(drawn.flatMap((one) => one.crossed))}`, drawn.every((one) => one.crossed.length === 0));
  expect(`every tile of a graph to be as tall as the others, got ${[...new Set(drawn.map((one) => one.tall))]}`, new Set(drawn.map((one) => one.tall)).size === 1 && drawn[0].tall > 52);
  const room = drawn.find((one) => one.node === 'main/agents');
  expect(`the step that keeps who is in the fleet to say so, got ${JSON.stringify(room)}`, room && room.words.join(' ') === 'Who is in the fleet.' && room.under.startsWith('agents · '));
  await page.screenshot({ path: `${out}/blueprint-9g-tiles-that-say.png` });
  // What is said under the graph, and its buttons, lie over no tile and no group's name, at three widths and a phone's.
  const wide = page.viewportSize();
  for (const [width, height] of [[1280, 860], [900, 860], [600, 860], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await page.locator('.canvas .gnode').first().waitFor();
    const band = await legendCrossed(page);
    expect(`nothing said under the graph to lie over a tile or a group's name at ${width} wide, got ${band.slice(0, 4).join('; ')}`, band.length === 0);
    await page.screenshot({ path: `${out}/blueprint-9h-graph-band-${width}.png` });
  }
  await page.setViewportSize(wide);
  // Looked at closely, where the letters can be read, the same holds: a tile is written in its own measure.
  const near = await page.locator('.gnode[data-node="main/agents"] rect.box').boundingBox();
  await page.mouse.move(near.x + near.width / 2, near.y + near.height / 2);
  await page.mouse.wheel(0, -1400);
  await page.waitForFunction((was) => document.querySelector('.gnode[data-node="main/agents"] rect.box').getBoundingClientRect().width > was * 3, near.width);
  const close = await tiles();
  expect(`nothing a tile says to be outside it looked at closely, got ${JSON.stringify(close.filter((one) => one.outside.length > 0).map((one) => one.outside))}`, close.every((one) => one.outside.length === 0 && one.crossed.length === 0));
  await page.screenshot({ path: `${out}/blueprint-9g-tiles-that-say-close.png` });
  // Opened, the step is headed by what it says of itself, with the whole of it and its key under that.
  await page.locator('.gnode[data-node="main/agents"]').click();
  const opened = drawerAtRest(page);
  await opened.getByRole('heading', { name: 'Who is in the fleet.' }).waitFor();
  expect('the whole of what it says to be in its details', (await opened.locator('.node-says').innerText()).startsWith('Who is in the fleet. A row is'));
  expect(`its key to be said under that, with what it runs, got "${(await opened.locator('.node-key').innerText()).trim()}"`, /^agents · \S+/.test((await opened.locator('.node-key').innerText()).trim()));
  await page.screenshot({ path: `${out}/blueprint-9h-a-step-opened.png` });
  await page.keyboard.press('Escape');
  }

  // Back to the text that was being written.
  await page.locator('.tab', { hasText: 'deploy' }).first().click();
  await page.getByRole('tab', { name: 'Text' }).last().click();
  await page.locator('.cm-content').last().getByText('outputs').waitFor();

  // ---- what a handler hears -------------------------------------------------------
  // A handler written under what is there: the events there are after
  // `event:`, and what the one chosen carries after `event.data.`.
  await page.locator('.cm-content').last().click();
  await page.keyboard.press('Control+End');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Shift+Home');
  await page.keyboard.type('on:');
  await page.keyboard.press('Enter');
  await page.keyboard.type('moved:');
  await page.keyboard.press('Enter');
  await page.keyboard.type('event: ');
  await page.keyboard.press('Control+Space');
  const topics = await offering('board.item.moved');
  expect(`the events there are to be offered after \`event:\`, got ${topics.length} of them`, topics.includes('step.failed') && topics.includes('channel.message') && topics.length > 40);
  expect('an event that is only passed on not to be offered: no handler hears it', !topics.includes('log.line'));
  await page.keyboard.type('board.item.mov');
  await take('board.item.moved');
  expect(`the topic to be put in, and the line is ${JSON.stringify(await line())}`, (await line()).trim() === 'event: board.item.moved');
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await page.keyboard.type('when: ${ event.data.');
  const carried = await offering('to');
  expect(`what a move carries to be offered after \`event.data.\`, got ${carried}`, ['id', 'project', 'from', 'to', 'note'].every((name) => carried.includes(name)) && !carried.includes('columns'));
  await page.screenshot({ path: `${out}/blueprint-10-event.png` });
  await page.keyboard.press('Escape');

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  // What the page looked like when it went wrong says more than the error does.
  await page?.screenshot({ path: `${out}/blueprint-0-failed.png` }).catch(() => {});
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
