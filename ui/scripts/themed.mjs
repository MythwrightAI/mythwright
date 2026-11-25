// The themes somebody makes, kept with their account: what a browser kept
// of them from before taken up once, and taken out of it; one made on one
// page and seen, and worn, on another that is never read again; there in a
// browser that never saw it, once its person signs in; offered to
// everybody by whoever manages the factory, listed for a member with who
// offered it, and withdrawn; a theme saved as a file by the browser alone
// and read back in on another person's page as a theme of theirs, with a
// file that is no theme's, and one the daemon refuses, said in a sentence; nothing drawn that whoever looks may not do;
// nothing of the panel over anything else or scrolling sideways at
// three widths and a phone's; and a theme that says how the client looks
// beyond its colours, each setting worn as it is chosen in the form and
// let go of where the form is left, kept, worn on the page that is not
// read again and put on before anything is drawn, carried by its file,
// a file of the version before read as one that says nothing of it, and
// nothing of the page, of the form or of its colour picker over anything
// else or scrolling sideways at the largest size, at the most compact
// density and at both together; and a
// colour chosen in the client's own picker, with the colour as it was
// beside it (`picked.mjs`), in the dark and in the light.
//
//   DAEMON=../target/debug/factory node scripts/themed.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
import { COLOURS, STARTS, lightFrom } from '../src/themes.ts';
import { pageCrowded } from './crowded.mjs';
import { saysWhere } from './where.ts';
import { STACKS } from '../src/looks.ts';
import { colourOf, openPicker, pickColour, pickedColours, pickerCrowded, settled } from './picked.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'themed-data');
const home = join(out, 'themed-home');

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

const scheme = process.env.SCHEME ?? 'dark';
// What a browser kept before themes were kept with an account: one theme, chosen.
const OLD = { id: 'own-old-harbour', name: 'Old harbour', says: 'A theme made here.', colours: { ground: '#101820', text: '#e8eef2', accent: '#3fb8af', ok: '#58c08a', bad: '#f0585c', info: '#6aaaf0', agent: '#8fa8c4', card: '#f2c230' } };

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'mira@example.com', name: 'Mira', password: 'correct horse', invite: code }),
  });
  if (!joined.ok) throw new Error(`a second person was not let in: ${joined.status}`);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  // What failed is said with what the page threw meanwhile, which is often why.
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}${problems.length ? `\nerrors in the page:\n${problems.join('\n')}` : ''}`);
  };
  const open = async (viewport = { width: 1280, height: 860 }) => {
    const context = await browser.newContext({ viewport, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(err.message));
    return page;
  };
  const signIn = async (on, email) => {
    await on.goto(url);
    await on.getByLabel('Email').fill(email);
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  const settings = async (on) => {
    await on.locator('.rail').getByRole('button', { name: 'Settings', exact: true }).click();
    await on.getByRole('heading', { name: 'Plan' }).waitFor();
  };
  const panel = (on) => on.locator('.panel', { has: on.getByRole('heading', { name: 'Theme', exact: true }) });
  const radio = (on, name) => panel(on).getByRole('radio', { name: new RegExp(name) });
  const worn = (on) => on.evaluate(() => ({ theme: document.documentElement.dataset.theme, accent: getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() }));
  const asked = (on, method, path, body) =>
    on.evaluate(
      async ([method, path, body]) => {
        const headers = { 'x-page-key': localStorage.getItem('page-key') ?? '', 'content-type': 'application/json' };
        const answer = await fetch(`/api${path}`, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) });
        return { status: answer.status, body: await answer.json().catch(() => null) };
      },
      [method, path, body],
    );
  const mine = async (on) => (await asked(on, 'GET', '/themes')).body;
  // The boxes of the panel's parts: none lies over another, each is inside the panel, and nothing scrolls sideways.
  const apart = async (on, what) => {
    const found = await panel(on).evaluate((box) => {
      const frame = box.getBoundingClientRect();
      const boxes = [...box.querySelectorAll('.panel-head h2, .panel-head .faint, .theme-heading, .theme-sample, .theme-made .row button, .theme-actions button')]
        .map((one) => ({ said: (one.getAttribute('aria-label') ?? one.textContent).trim().slice(0, 30), box: one.getBoundingClientRect() }))
        .filter((one) => one.box.width > 0);
      const wrong = [];
      for (const [index, one] of boxes.entries()) {
        if (one.box.left < frame.left - 0.5 || one.box.right > frame.right + 0.5) wrong.push(`${one.said} leaves it`);
        for (const two of boxes.slice(index + 1)) {
          const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
          if (over) wrong.push(`${one.said} lies over ${two.said}`);
        }
      }
      // The cards of a row are as tall as each other, whatever each one's words come to.
      for (const row of box.querySelectorAll('.theme-samples')) {
        const byTop = new Map();
        for (const card of row.querySelectorAll(':scope > .theme-made > .theme-sample')) {
          const at = card.getBoundingClientRect();
          if (at.width === 0) continue;
          const top = Math.round(at.top);
          byTop.set(top, [...(byTop.get(top) ?? []), { said: card.textContent.trim().slice(0, 20), height: at.height }]);
        }
        for (const cards of byTop.values()) {
          const tallest = Math.max(...cards.map((card) => card.height));
          for (const card of cards) if (tallest - card.height > 1) wrong.push(`${card.said} is ${Math.round(tallest - card.height)}px shorter than the tallest card beside it`);
        }
      }
      return { count: boxes.length, wrong, sideways: document.documentElement.scrollWidth - window.innerWidth };
    });
    expect(`${what}: the panel's parts to be there`, found.count > 6);
    expect(`${what}: nothing over anything else or outside the panel, got ${found.wrong.join('; ')}`, found.wrong.length === 0);
    expect(`${what}: nothing to scroll sideways, got ${found.sideways}px`, found.sideways <= 0);
  };

  // 1: a browser that kept a theme of Mira's, chosen, from before themes were
  // the account's. Signed in, it is sent to her account, worn, and taken out
  // of the browser; and the account's choice is it.
  const old = await open();
  await old.goto(url);
  await old.evaluate((kept) => {
    localStorage.setItem('client.themes', JSON.stringify([kept]));
    localStorage.setItem('client.theme', kept.id);
  }, OLD);
  await signIn(old, 'mira@example.com');
  await old.waitForFunction(() => localStorage.getItem('client.themes') === null);
  const taken = await mine(old);
  expect(`the browser's theme to be the account's, got ${JSON.stringify(taken)}`, taken.own.length === 1 && taken.own[0].name === 'Old harbour' && taken.chosen === taken.own[0].id);
  await old.waitForFunction((id) => document.documentElement.dataset.theme === id, taken.own[0].id);
  expect('the theme taken up to be worn in its colours', (await worn(old)).accent === '#3fb8af');
  // Read again, it is taken up no more: still the one.
  await old.reload();
  await settings(old);
  await radio(old, 'Old harbour').waitFor();
  expect('what was taken up to be taken up once', (await mine(old)).own.length === 1 && (await radio(old, 'Old harbour').count()) === 1);
  await old.screenshot({ path: `${out}/themed-1-taken-up.png` });

  // 2: a second page of hers, which is never read again from here on.
  const second = await open();
  await signIn(second, 'mira@example.com');
  await settings(second);
  await radio(second, 'Old harbour').waitFor();
  expect('a member to be offered nothing to offer or withdraw', (await panel(second).getByRole('button', { name: /Offer .* to everybody|Withdraw/ }).count()) === 0);

  // 3: made on the first page, it is seen on the second, which wears it too, being what she chose.
  await panel(old).getByRole('button', { name: /Add a theme/ }).click();
  const making = old.locator('.modal');
  await making.getByLabel('What it is called').fill('Dusk');
  await making.getByLabel('Starting from').selectOption('rose-pine');
  await pickColour(old, making, 'Accent', '#2fd4c4');
  await making.getByRole('button', { name: 'Add it', exact: true }).click();
  await making.waitFor({ state: 'detached' });
  expect('the theme made to be worn where it was made', (await worn(old)).accent === '#2fd4c4');
  await radio(second, 'Dusk').waitFor();
  await second.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() === '#2fd4c4');
  expect('the page that was not read again to mark it as in use', (await radio(second, 'Dusk').getAttribute('aria-checked')) === 'true');
  await second.screenshot({ path: `${out}/themed-2-seen-elsewhere.png` });
  // What the daemon refuses is said in its sentence, and the form stays.
  const refused = await asked(old, 'POST', '/themes', { name: 'x'.repeat(61), colours: OLD.colours });
  expect(`a name too long refused in a sentence, got ${JSON.stringify(refused)}`, refused.status === 400 && refused.body.error.message.includes('60 letters at most'));

  // 4: a browser that never saw any of it, signed in as her: it is there and worn.
  const fresh = await open();
  await signIn(fresh, 'mira@example.com');
  await fresh.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() === '#2fd4c4');
  await settings(fresh);
  for (const name of ['Old harbour', 'Dusk']) await radio(fresh, name).waitFor();
  expect('the theme she chose to be in use in a fresh browser', (await radio(fresh, 'Dusk').getAttribute('aria-checked')) === 'true');
  await fresh.screenshot({ path: `${out}/themed-3-fresh-browser.png` });
  await fresh.context().close();

  // 5: the owner offers one of hers to everybody; Mira's page lists it with who offered it, and no Withdraw.
  const ada = await open();
  await signIn(ada, 'ada@example.com');
  await settings(ada);
  // With no theme of her own yet, her own are a heading and the line that makes one, with no empty row between.
  const none = panel(ada).getByRole('region', { name: 'Your own' });
  await none.getByRole('button', { name: /Add a theme/ }).waitFor();
  expect('no row of her own themes while she has none', (await none.locator('.theme-samples:visible').count()) === 0 && (await none.getByRole('button', { name: 'Read a theme from a file', exact: true }).isVisible()));
  await asked(ada, 'POST', '/themes', { name: 'Lantern', colours: { ...OLD.colours, accent: '#f2a541' } });
  await panel(ada).getByRole('button', { name: 'Offer Lantern to everybody', exact: true }).click();
  await panel(ada).getByRole('button', { name: 'Withdraw Lantern', exact: true }).waitFor();
  expect('the owner not to be offered her own again among those offered', (await panel(ada).getByRole('region', { name: 'Offered to everybody' }).count()) === 0);
  const offered = panel(second).getByRole('region', { name: 'Offered to everybody' });
  await offered.getByRole('radio', { name: /Lantern/ }).waitFor();
  expect('the member to be told who offered it', (await offered.innerText()).includes('Offered by Ada.'));
  expect('the member to be offered no Withdraw', (await offered.getByRole('button', { name: /Withdraw/ }).count()) === 0);
  await offered.getByRole('radio', { name: /Lantern/ }).click();
  await second.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() === '#f2a541');
  await second.screenshot({ path: `${out}/themed-4-offered.png` });
  await ada.screenshot({ path: `${out}/themed-5-offering.png` });

  // 6: a theme saved as a file by the browser alone, one of hers and one that ships, and read back in on
  // Ada's page as a theme of Ada's own; read again, it is the one she has; and a file that is no theme's,
  // and one the daemon refuses, each said in its sentence.
  const saved = async (on, name) => {
    const asking = [];
    const heard = (request) => request.url().includes('/api/') && asking.push(request.url());
    on.on('request', heard);
    const [download] = await Promise.all([on.waitForEvent('download'), panel(on).getByRole('button', { name: `Save ${name} as a file`, exact: true }).click()]);
    on.off('request', heard);
    expect(`saving ${name} to ask nothing of the daemon, asked ${asking.join(', ')}`, asking.length === 0);
    const path = join(out, download.suggestedFilename());
    await download.saveAs(path);
    return { file: download.suggestedFilename(), path, read: JSON.parse(await readFile(path, 'utf8')) };
  };
  const dusk = (await mine(second)).own.find((one) => one.name === 'Dusk');
  const alike = (one, two) => Object.keys(one).length === Object.keys(two).length && Object.keys(one).every((key) => one[key] === two[key]);
  const file = await saved(second, 'Dusk');
  expect(`the file to be called after the theme, got ${file.file}`, file.file === 'dusk.theme.json');
  expect(`the file to say its version, its name and its colours, got ${JSON.stringify(file.read)}`, file.read.version === 1 && file.read.name === 'Dusk' && alike(file.read.colours, dusk.colours));
  const shipped = await saved(second, 'Rosé Pine');
  expect(`a theme that ships to be saved too, got ${shipped.file}`, shipped.file === 'rosé-pine.theme.json' && shipped.read.colours.ground === '#191724');
  const readIn = async (path) => panel(ada).locator('input[type="file"]').setInputFiles(path);
  const own = panel(ada).getByRole('region', { name: 'Your own' });
  await readIn(file.path);
  await own.getByRole('radio', { name: /Dusk/ }).waitFor();
  await ada.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() === '#2fd4c4');
  const adas = await mine(ada);
  const made = adas.own.find((one) => one.name === 'Dusk');
  expect(`the file read in to be a theme of Ada's own with its colours, got ${JSON.stringify(adas.own)}`, made && made.owner !== dusk.owner && alike(made.colours, dusk.colours) && adas.chosen === made.id);
  await Promise.all([ada.waitForResponse((answer) => answer.url().endsWith('/api/themes') && answer.request().method() === 'POST'), readIn(file.path)]);
  expect('the same file read again to be the theme there is', (await mine(ada)).own.filter((one) => one.name === 'Dusk').length === 1);
  const wrongs = [
    ['not-json.theme.json', '{ "version": 1,', 'not-json.theme.json was not read. This is not a theme’s file: it is not JSON.'],
    ['later.theme.json', JSON.stringify({ ...file.read, version: 3 }), 'later.theme.json was not read. This file says it is version 3 of a theme’s file, and this page reads versions 1 and 2.'],
    ['pale.theme.json', JSON.stringify({ ...file.read, colours: { ...file.read.colours, text: file.read.colours.ground } }), 'pale.theme.json was not read. The text is too near the ground to be read on it: make one lighter or the other darker.'],
    ['long.theme.json', JSON.stringify({ ...file.read, name: 'x'.repeat(61) }), "long.theme.json was not read. A theme's name is 60 letters at most, and this one is 61."],
  ];
  for (const [name, text, says] of wrongs) {
    const path = join(out, name);
    await writeFile(path, text);
    await readIn(path);
    await panel(ada).locator('.theme-wrong').filter({ hasText: says }).waitFor();
  }
  expect('nothing to be made of a file that is no theme', (await mine(ada)).own.length === adas.own.length);
  await ada.screenshot({ path: `${out}/themed-6-read-in.png` });

  // 7: nothing of the panel over anything else, with every part drawn, at three widths and a phone's.
  for (const width of [1280, 1000, 800]) {
    await second.setViewportSize({ width, height: 860 });
    await apart(second, `at ${width}`);
  }
  await second.setViewportSize({ width: 390, height: 844 });
  await panel(second).scrollIntoViewIfNeeded();
  await apart(second, "at a phone's width");
  await second.screenshot({ path: `${out}/themed-7-phone.png`, fullPage: true });
  await second.setViewportSize({ width: 1280, height: 860 });

  // 8: withdrawn, it goes from her page, which goes back to the usual one.
  await panel(ada).getByRole('button', { name: 'Withdraw Lantern', exact: true }).click();
  await offered.waitFor({ state: 'detached' });
  await second.waitForFunction(() => document.documentElement.dataset.theme === 'grid-iron');
  await panel(ada).getByRole('button', { name: 'Offer Lantern to everybody', exact: true }).waitFor();

  // 9: a theme with a light version, made on the first page: its light colours begin as worked out from the dark
  // ones, follow them until one is changed, and stay once it is. The second page, never read again, wears it, and
  // wears the light version where the browser is told the system asks for light; one with none is the same in both.
  await panel(old).getByRole('button', { name: /Add a theme/ }).click();
  const lit = old.locator('.modal');
  await lit.getByLabel('What it is called').fill('Tide');
  await lit.getByLabel('Starting from').selectOption('tokyo-midnight');
  await lit.getByLabel('A light version').check();
  const lightBoxes = async () => Object.fromEntries(await Promise.all(COLOURS.map(async ([key, label]) => [key, await colourOf(lit, `Light ${label}`)])));
  const begun = lightFrom(STARTS['tokyo-midnight']);
  expect(`the light version to begin as worked out from the dark one, got ${JSON.stringify(await lightBoxes())}`, alike(await lightBoxes(), begun));
  await pickColour(old, lit, 'Ground', '#101418');
  expect('the light version to follow the dark one until one of its own is changed', (await colourOf(lit, 'Light Ground')) === lightFrom({ ...STARTS['tokyo-midnight'], ground: '#101418' }).ground);
  await pickColour(old, lit, 'Light Accent', '#0a6e66');
  await pickColour(old, lit, 'Accent', '#4fd1c5');
  expect('a light colour that was changed to stay', (await colourOf(lit, 'Light Accent')) === '#0a6e66');
  await lit.getByLabel('How it looks in light').waitFor();
  await old.screenshot({ path: `${out}/themed-8-light-version.png` });
  await lit.getByRole('button', { name: 'Add it', exact: true }).click();
  await lit.waitFor({ state: 'detached' });
  const tide = (await mine(old)).own.find((one) => one.name === 'Tide');
  expect(`the light version to be kept with the theme, got ${JSON.stringify(tide)}`, tide?.light?.accent === '#0a6e66' && tide.colours.accent === '#4fd1c5');
  const painted = (on) => on.evaluate(() => ['--bg0', '--text', '--accent'].map((name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim()));
  // From the dark, whichever the check began in (`SCHEME`): in the light the page rightly wears the light version.
  await litAs(second, 'dark');
  await second.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() === '#4fd1c5');
  expect('the dark version worn where the system asks for dark', JSON.stringify(await painted(second)) === JSON.stringify([tide.colours.ground, tide.colours.text, '#4fd1c5']));
  await litAs(second, 'light');
  await second.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() === '#0a6e66');
  expect(`the light version worn where the system asks for light, got ${await painted(second)}`, JSON.stringify(await painted(second)) === JSON.stringify([tide.light.ground, tide.light.text, '#0a6e66']));
  expect('the page to say it is light', (await second.evaluate(() => getComputedStyle(document.documentElement).colorScheme)) === 'light');
  expect('its sample in Settings drawn in the light version', (await radio(second, 'Tide').evaluate((box) => getComputedStyle(box).getPropertyValue('--accent').trim())) === '#0a6e66');
  await second.screenshot({ path: `${out}/themed-9-light.png` });
  // Read again in light, the page is in the light version before anything of the account is read.
  await second.reload();
  expect('the light version put on before anything is drawn', (await painted(second))[2] === '#0a6e66');
  await radio(old, 'Old harbour').click();
  await second.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() === '#3fb8af');
  await litAs(second, 'dark');
  expect('a theme with no light version the same in both', (await painted(second))[2] === '#3fb8af');
  // Saved as a file it carries its light version, and read in on Ada's page it is hers with it.
  const tideFile = await saved(old, 'Tide');
  expect(`the file to carry the light version, got ${JSON.stringify(tideFile.read)}`, tideFile.read.version === 1 && alike(tideFile.read.light, tide.light));
  await readIn(tideFile.path);
  await own.getByRole('radio', { name: /Tide/ }).waitFor();
  expect("the light version to come with the file to Ada's", alike((await mine(ada)).own.find((one) => one.name === 'Tide').light, tide.light));
  // The daemon holds the light version to being read on its own, in its own sentence.
  const pale = await asked(old, 'POST', '/themes', { name: 'Pale', colours: OLD.colours, light: { ...tide.light, text: tide.light.ground } });
  expect(`a light version that cannot be read refused, got ${JSON.stringify(pale)}`, pale.status === 400 && pale.body.error.message.startsWith('In the light version: The text is too near'));

  // 10: a theme that says how the client looks. Each setting is worn on the page as it is chosen in the form, and
  // let go of where the form is left; kept, it is worn on the page that is not read again, and put on before
  // anything is drawn; its file carries it, and a file of the version before still reads.
  const measure = (on, name) => on.evaluate((name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim(), name);
  const lettersOf = (on) => on.evaluate(() => getComputedStyle(document.body).fontSize);
  await panel(old).getByRole('button', { name: /Add a theme/ }).click();
  const looking = old.locator('.modal');
  await looking.getByLabel('Size', { exact: true }).selectOption('largest');
  await old.waitForFunction(() => getComputedStyle(document.body).fontSize === '15.6px');
  await looking.getByRole('button', { name: 'Cancel', exact: true }).click();
  await looking.waitFor({ state: 'detached' });
  await old.waitForFunction(() => getComputedStyle(document.body).fontSize === '13px');
  expect('a look left in the form to be let go of', (await measure(old, '--size')) === '1');
  await panel(old).getByRole('button', { name: /Add a theme/ }).click();
  await looking.getByLabel('What it is called').fill('Reading room');
  for (const [setting, word] of [['Letters', 'serif'], ['Size', 'largest'], ['Density', 'compact'], ['Corners', 'square']]) {
    await looking.getByLabel(setting, { exact: true }).selectOption(word);
  }
  expect('the page to wear the look as it is chosen', (await measure(old, '--density')) === '0.8' && (await measure(old, '--round')) === '0' && (await lettersOf(old)) === '15.6px');
  await old.screenshot({ path: `${out}/themed-10-look-chosen.png` });
  await looking.getByRole('button', { name: 'Add it', exact: true }).click();
  await looking.waitFor({ state: 'detached' });
  const room = (await mine(old)).own.find((one) => one.name === 'Reading room');
  const looked = { letters: 'serif', size: 'largest', density: 'compact', corners: 'square' };
  expect(`the look to be kept with the theme, got ${JSON.stringify(room)}`, room && alike(room.look, looked));
  await second.waitForFunction(() => getComputedStyle(document.body).fontSize === '15.6px');
  expect('the page that was not read again to wear the look', (await measure(second, '--sans')) === STACKS.serif && (await measure(second, '--density')) === '0.8');
  expect('its card to say what it says of its look', (await radio(second, 'Reading room').innerText()).includes('Size: largest'));
  await second.reload();
  expect('the look put on before anything is drawn', (await measure(second, '--size')) === '1.2' && (await measure(second, '--round')) === '0');
  await radio(second, 'Reading room').waitFor();
  const roomFile = await saved(old, 'Reading room');
  expect(`the file to carry the look as the version that says one, got ${JSON.stringify(roomFile.read)}`, roomFile.read.version === 2 && alike(roomFile.read.look, looked));
  await readIn(roomFile.path);
  await own.getByRole('radio', { name: /Reading room/ }).waitFor();
  expect("the look to come with the file to Ada's", alike((await mine(ada)).own.find((one) => one.name === 'Reading room').look, looked));
  await ada.waitForFunction(() => getComputedStyle(document.body).fontSize === '15.6px');
  const before = join(out, 'before.theme.json');
  await writeFile(before, JSON.stringify({ version: 1, name: 'Before', colours: OLD.colours }));
  await readIn(before);
  await own.getByRole('radio', { name: /Before/ }).waitFor();
  expect('a file of the version before to read as a theme that says nothing of its look', Object.keys((await mine(ada)).own.find((one) => one.name === 'Before').look ?? {}).length === 0);
  await ada.waitForFunction(() => getComputedStyle(document.body).fontSize === '13px');
  const unlike = await asked(old, 'POST', '/themes', { name: 'Huge', colours: OLD.colours, look: { size: 'huge' } });
  expect(`a look that is none refused, got ${JSON.stringify(unlike)}`, unlike.status === 400 && unlike.body.error.message.includes('is none of them'));

  // 11: nothing of the page over anything else, past the window's sides or scrolling sideways: at the largest size,
  // at the most compact density and at both together, at three widths and a phone's, with Settings showing and
  // with the form open over it.
  const looks = [
    ['the largest size', { size: 'largest' }],
    ['the most compact density', { density: 'compact' }],
    ['both together', { size: 'largest', density: 'compact' }],
  ];
  for (const [what, look] of looks) {
    const changed = await asked(old, 'PUT', `/themes/${room.id}`, { name: 'Reading room', colours: room.colours, look });
    expect(`the look to be changed to ${what}, got ${changed.status}`, changed.status === 200);
    await second.waitForFunction((size) => getComputedStyle(document.documentElement).getPropertyValue('--size').trim() === size, look.size === 'largest' ? '1.2' : '1');
    await second.waitForFunction((density) => getComputedStyle(document.documentElement).getPropertyValue('--density').trim() === density, look.density === 'compact' ? '0.8' : '1');
    for (const [width, height] of [[1280, 860], [1000, 760], [800, 700], [390, 844]]) {
      await second.setViewportSize({ width, height });
      await panel(second).scrollIntoViewIfNeeded();
      const wrong = await pageCrowded(second);
      expect(`at ${what} and ${width} wide, nothing over anything else, got ${wrong.join('; ')}`, wrong.length === 0);
      await apart(second, `at ${what} and ${width} wide`);
      await panel(second).getByRole('button', { name: 'Change Reading room', exact: true }).click();
      const form = second.locator('.modal');
      await form.waitFor();
      const inForm = await pageCrowded(second, '.modal');
      expect(`the form at ${what} and ${width} wide, nothing over anything else, got ${inForm.join('; ')}`, inForm.length === 0);
      if (width === 390 || width === 1280) await second.screenshot({ path: `${out}/themed-11-${what.replace(/ /g, '-')}-${width}.png` });
      // The colour picker over the form, at the look: this check wears looks of its own, so it is held here and not by `LOOK`.
      await form.getByRole('button', { name: 'Accent', exact: true }).scrollIntoViewIfNeeded();
      const picker = await openPicker(second, form, 'Accent');
      const inPicker = await pickerCrowded(second);
      expect(`the colour picker at ${what} and ${width} wide, nothing over anything else or outside the window, got ${inPicker.join('; ')}`, inPicker.length === 0);
      if (width === 390 || width === 1280) await second.screenshot({ path: `${out}/themed-11-${what.replace(/ /g, '-')}-${width}-picker.png` });
      await settled(second);
      await second.keyboard.press('Escape');
      await picker.waitFor({ state: 'detached' });
      await form.getByRole('button', { name: 'Cancel', exact: true }).click();
      await form.waitFor({ state: 'detached' });
    }
  }
  await second.setViewportSize({ width: 1280, height: 860 });

  // 12: a colour chosen in the client's own picker, with the colour as it was beside it (`picked.mjs`), in a page in
  // the dark and in one in the light, each from a theme that ships.
  for (const [lit, name, accent] of [['dark', 'Ember', '#c2410c'], ['light', 'Embers', '#b4530a']]) {
    await litAs(old, lit);
    await radio(old, 'Grid Iron').click();
    await old.waitForFunction(() => document.documentElement.dataset.theme === 'grid-iron');
    await pickedColours({ page: old, other: second, out, expect, name, scheme: lit, accent });
  }

  await browser.close();
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
