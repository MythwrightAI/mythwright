// The page about the program, opened from its mark
// on the rail: the mark at the rail's foot over who is signed in, by the
// boxes beside the rail's other places, drawn by the mark's own two
// strokes in the colour of the rest; pressed, a tab called after what the
// daemon says the program is called, opening on What is <name>, the page
// `docs/about.md` came to, read in place with nothing in it run; Docs,
// every guide the daemon lists, the first read in place, another chosen,
// a link in one to another guide read in its place, each said by the
// address; Technical, with the version, the client's build, the system,
// where the data is kept, the packages installed and whether each is the
// one that came, and what to say in a report copied as Help's menu
// copies it; the address opening the page at a tab and at a guide; the
// tab shown again as it was left; a member shown Technical without the
// data folder; and nothing over anything else, past the window or
// scrolling sideways with each tab showing at 1280, 900 and 640 wide and
// a phone's, where the mark is reached on the rail by touch.

import { execFile, spawn } from 'node:child_process';
import { mkdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { readdir } from 'node:fs/promises';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';
import { sideSettled } from './drawer.mjs';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'about-data');
const home = join(out, 'about-home');

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joined = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  expect(`a member to have joined, got ${joined.status}`, joined.ok);
  // What the program is called is the daemon's to say, and nowhere written here.
  const meta = await (await fetch(`${url}/api/meta`)).json();
  const NAME = meta.name;
  expect('the daemon to say what the program is called', typeof NAME === 'string' && NAME.length > 0);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const asked = [];
  const opened = async (options = {}) => {
    const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: 'dark', ...options });
    await context.grantPermissions(['clipboard-read', 'clipboard-write'], { origin: url });
    const page = await context.newPage();
    page.setDefaultTimeout(20_000);
    page.on('pageerror', (err) => problems.push(err.message));
    page.on('request', (request) => asked.push(request.url()));
    return page;
  };
  const signIn = async (page, email, at = '') => {
    await page.goto(`${url}${at}`);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  };
  const page = await opened();
  await signIn(page, 'ada@example.com');
  await page.locator('.rail').waitFor();

  // ---- the rail: the mark at its foot, over who is signed in, as big as the rest ---------------------------------
  const mark = page.locator('.rail').getByRole('button', { name: `About ${NAME}`, exact: true });
  await mark.waitFor();
  const rail = await page.evaluate(() => {
    const strip = document.querySelector('.rail').getBoundingClientRect();
    const places = [...document.querySelectorAll('.rail > .rail-button')].map((one) => ({ name: one.getAttribute('aria-label'), box: one.getBoundingClientRect().toJSON() }));
    const drawn = document.querySelector('.rail .rail-about svg');
    return { strip: strip.toJSON(), places, paths: drawn.querySelectorAll('path').length, fill: drawn.getAttribute('fill'), drawn: drawn.getBoundingClientRect().toJSON(), colour: getComputedStyle(drawn).color, settingsColour: getComputedStyle(document.querySelector('.rail .rail-button[aria-label="Settings"]')).color };
  });
  const names = rail.places.map((one) => one.name.replace(/, \d+ unread$/, ''));
  const at = names.indexOf(`About ${NAME}`);
  expect(`the mark's place just over who is signed in and Settings at the rail's foot, got ${names.join(', ')}`, at === names.length - 3 && names[at + 1].startsWith('You:') && names[at + 2] === 'Settings');
  expect(`the side panel's places first, as they were, got ${names.join(', ')}`, names.slice(0, 5).join() === ['Channels', 'Topics', 'Resources', 'Inbox', 'Extensions'].join());
  const [extensions, about, settings] = [rail.places[4].box, rail.places[at].box, rail.places.at(-1).box];
  const told = JSON.stringify({ extensions, about, settings, strip: rail.strip });
  expect(`the mark's place as big as the others, in line with them and inside the rail: ${told}`, Math.abs(about.width - settings.width) < 0.5 && Math.abs(about.height - settings.height) < 0.5 && Math.abs(about.left - settings.left) < 0.5 && about.left >= rail.strip.left - 0.5 && about.right <= rail.strip.right + 0.5);
  expect(`the mark's place under the side panel's and over the foot's, none over another: ${told}`, about.top >= extensions.bottom - 0.5 && about.bottom <= rail.places[at + 1].box.top + 0.5);
  // Filled where the other signs are lines, the mark is drawn a step clearer than they are (lighter, on this dark page), so as not to read dimmer.
  const light = (colour) => colour.match(/[\d.]+/g).slice(0, 3).reduce((sum, one) => sum + Number(one), 0);
  expect(`the mark drawn by its own two strokes, lighter than the rail's other places, got ${JSON.stringify(rail)}`, rail.paths === 2 && rail.fill === 'currentColor' && light(rail.colour) > light(rail.settingsColour));
  expect(`the mark drawn at the rail's size, inside its place: ${JSON.stringify(rail.drawn)}`, rail.drawn.width >= 18 && rail.drawn.width <= 24 && rail.drawn.left >= about.left && rail.drawn.right <= about.right);
  await page.screenshot({ path: `${out}/about-1-rail.png`, clip: { x: 0, y: rail.strip.top, width: 220, height: rail.strip.height } });
  await mark.hover();
  await page.screenshot({ path: `${out}/about-1-rail-pointed.png`, clip: { x: 0, y: Math.max(0, about.top - 60), width: 260, height: 140 } });

  // ---- pressed: one tab, on What is <name> ------------------------------------------------------------------------
  const aboutPage = page.locator('.about-page');
  const tabNamed = (word) => aboutPage.locator(`[role="tab"][data-about-tab="${word}"]`);
  const showing = () => aboutPage.locator('[role="tabpanel"]').getAttribute('data-about-tab');
  const bar = () => page.evaluate(() => location.pathname + location.hash);
  const barSays = async (wanted) => {
    await page.waitForFunction((wanted) => location.pathname + location.hash === wanted, wanted, { timeout: 5000 }).catch(() => {});
    expect(`the address to say ${wanted}, got ${await bar()}`, (await bar()) === wanted);
  };
  await mark.click();
  await aboutPage.waitFor();
  // What each tab is called, without the mark before it and the way to close it after.
  const tabTitles = async () => (await page.locator('.tabs .tab').allInnerTexts()).map((one) => one.split('\n').map((line) => line.trim()).filter((line) => line && line !== '✕').at(-1));
  expect(`the tab called after the program, got ${JSON.stringify(await tabTitles())}`, (await tabTitles()).includes(`About ${NAME}`));
  expect('the page headed by what the program is called', (await aboutPage.getByRole('heading', { level: 1 }).innerText()).trim() === NAME);
  expect(`its tabs What is ${NAME}, Docs and Technical`, (await aboutPage.getByRole('tab').allInnerTexts()).map((one) => one.trim()).join(' | ') === `What is ${NAME} | Docs | Technical`);
  expect('it opening on the first', (await showing()) === 'about' && (await tabNamed('about').getAttribute('aria-selected')) === 'true');
  const frame = page.frameLocator('.about-page iframe.page-frame');
  await frame.getByRole('heading', { name: `What is ${NAME}`, level: 1 }).waitFor();
  expect('the page of docs/about.md with nothing in it that runs', (await page.locator('.about-page iframe.page-frame').getAttribute('sandbox')) === 'allow-same-origin');
  await barSays('/about');
  await page.screenshot({ path: `${out}/about-2-what.png` });
  // Pressed again, the one tab is shown and no second opened.
  await mark.click();
  expect('one tab whatever it is opened from', (await tabTitles()).filter((one) => one === `About ${NAME}`).length === 1);

  // ---- Docs: every guide the daemon lists, one read in place ------------------------------------------------------
  const daemonSays = (path) =>
    page.evaluate(async (path) => (await fetch(path, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json(), path);
  const said = await daemonSays('/api/about');
  await tabNamed('docs').click();
  expect('Docs shown', (await showing()) === 'docs');
  const guides = aboutPage.getByRole('navigation', { name: 'Guides' }).getByRole('button');
  await guides.first().waitFor();
  const listed = (await guides.allInnerTexts()).map((one) => one.trim());
  expect(`every guide the daemon lists, by what it calls itself: ${JSON.stringify(listed)}`, JSON.stringify(listed) === JSON.stringify(said.guides.map((one) => one.title)) && listed.length >= 5);
  const reading = () => aboutPage.locator('.about-read').getAttribute('data-reading');
  expect('the first read in place', (await reading()) === said.guides[0].name);
  await frame.getByRole('heading', { name: said.guides[0].title, level: 1 }).waitFor();
  // A link to a file no copy of the program carries leads nowhere, and keeps its words.
  expect('no link to the README, which no copy of the program carries', (await frame.locator('a[href*="README"]').count()) === 0);
  await barSays('/about#docs');
  await page.screenshot({ path: `${out}/about-3-docs.png` });
  // Another chosen, and said by the address.
  const views = said.guides.find((one) => one.name === 'views');
  await guides.filter({ hasText: views.title }).click();
  await frame.getByRole('heading', { name: views.title, level: 1 }).waitFor();
  expect('the guide chosen marked', (await guides.filter({ hasText: views.title }).getAttribute('aria-current')) === 'page');
  await barSays('/about#docs/views');
  // A link in a guide to another guide reads that one in its place.
  await guides.filter({ hasText: said.guides[0].title }).click();
  await frame.getByRole('heading', { name: said.guides[0].title, level: 1 }).waitFor();
  const extensionsGuide = said.guides.find((one) => one.name === 'extensions');
  await frame.locator('a[href="extensions.md"]').first().click();
  await frame.getByRole('heading', { name: extensionsGuide.title, level: 1 }).waitFor();
  expect('the guide a link leads to read in its place', (await reading()) === 'extensions');
  await barSays('/about#docs/extensions');

  // ---- Technical ----------------------------------------------------------------------------------------------------
  await tabNamed('technical').click();
  const technical = aboutPage.locator('.about-technical');
  await technical.waitFor();
  const facts = async (on) => Object.fromEntries(await on.locator('.about-fact').evaluateAll((all) => all.map((one) => [one.querySelector('dt').textContent.trim(), one.querySelector('dd').textContent.trim()])));
  await technical.locator('.about-fact[data-fact="System"]').waitFor();
  const owned = await facts(technical);
  expect(`the version, the client's build, the system and where the data is: ${JSON.stringify(owned)}`, owned.Version === meta.version && owned['Client build'] && owned.System && owned['Data folder'] === said.data && said.data.length > 0);
  expect('the packages installed said, with whether each is the one that came', (await technical.locator('[data-part="packages"]').count()) === 1);
  const report = (await technical.locator('.about-report').innerText()).trim();
  expect(`what to say in a report naming the program and its version: ${report}`, report.startsWith(`${NAME} ${meta.version}`) && report.includes('Browser:'));
  await technical.getByRole('button', { name: 'Copy', exact: true }).click();
  await page.locator('.toast', { hasText: 'Copied. Paste it into what you report.' }).waitFor();
  const clipped = await page.evaluate(() => navigator.clipboard.readText());
  expect(`what was copied what Help's menu copies, the report shown: ${clipped}`, clipped.trim() === report);
  await barSays('/about#technical');
  await page.screenshot({ path: `${out}/about-4-technical.png` });

  // ---- the tab shown again as it was left ---------------------------------------------------------------------------
  await page.locator('.rail').getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('heading', { name: 'Plan' }).waitFor();
  await page.locator('.tabs .tab', { hasText: `About ${NAME}` }).click();
  await technical.waitFor();
  expect('the tab shown again on the tab it was left at', (await showing()) === 'technical');

  // ---- an address opens it at a tab, and at a guide -----------------------------------------------------------------
  const second = await opened();
  await signIn(second, 'ada@example.com', '/about#docs/views');
  await second.locator('.about-page [role="tabpanel"][data-about-tab="docs"]').waitFor();
  await second.frameLocator('.about-page iframe.page-frame').getByRole('heading', { name: views.title, level: 1 }).waitFor();
  await second.evaluate(() => (location.hash = '#technical'));
  await second.locator('.about-page [role="tabpanel"][data-about-tab="technical"]').waitFor();
  await second.context().close();

  // ---- a member: Technical without the data folder ------------------------------------------------------------------
  const grace = await opened();
  await signIn(grace, 'grace@example.com', '/about#technical');
  const theirs = grace.locator('.about-page .about-technical');
  await theirs.locator('.about-fact[data-fact="Version"]').waitFor();
  await theirs.locator('.about-report').waitFor();
  const herFacts = await facts(theirs);
  expect(`the member shown no data folder: ${JSON.stringify(herFacts)}`, herFacts['Data folder'] === undefined && herFacts.Version === meta.version);
  const herSaid = await grace.evaluate(async () => (await fetch('/api/about', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json());
  expect('the member told nothing of where the data is', herSaid.data === undefined && !JSON.stringify(herSaid).includes(said.data));
  await grace.screenshot({ path: `${out}/about-5-member.png` });
  await grace.context().close();

  // ---- by the boxes, each tab showing, at three widths --------------------------------------------------------------
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800]]) {
    await page.setViewportSize({ width, height });
    for (const word of ['about', 'docs', 'technical']) {
      await tabNamed(word).click();
      await aboutPage.locator(`[role="tabpanel"][data-about-tab="${word}"]`).waitFor();
      if (word !== 'technical') await page.locator('.about-page iframe.page-frame').waitFor();
      const wrong = await pageCrowded(page, '.about-page');
      expect(`nothing of the page over anything else, past the window or scrolling sideways at ${width} with ${word} showing: ${wrong.join('; ')}`, wrong.length === 0);
      const sideways = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(`nothing scrolling sideways at ${width}`, sideways <= 0);
    }
    const wrongRail = await pageCrowded(page, '.rail');
    expect(`nothing of the rail over anything else at ${width}: ${wrongRail.join('; ')}`, wrongRail.length === 0);
    await page.screenshot({ path: `${out}/about-6-${width}.png` });
  }

  // ---- a phone: the mark on the rail by touch, and each tab -----------------------------------------------------------
  const phone = await opened({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  await signIn(phone, 'ada@example.com');
  const opener = phone.getByRole('button', { name: 'Expand the side panel' });
  await opener.tap();
  await sideSettled(phone);
  const onPhone = phone.locator('.rail').getByRole('button', { name: `About ${NAME}`, exact: true });
  const finger = await onPhone.boundingBox();
  expect(`the mark a finger's size on a phone: ${JSON.stringify(finger)}`, finger.width >= 40 && finger.height >= 40);
  await phone.screenshot({ path: `${out}/about-7-phone-rail.png` });
  await onPhone.tap();
  const phonePage = phone.locator('.about-page');
  await phonePage.waitFor();
  // On a phone the page is shown with the side panel put away, as anything opened from the rail is.
  await sideSettled(phone);
  expect('the side panel put away for the page on a phone', !(await phone.locator('.rail').isVisible()));
  for (const word of ['about', 'docs', 'technical']) {
    await phonePage.locator(`[role="tab"][data-about-tab="${word}"]`).tap();
    await phonePage.locator(`[role="tabpanel"][data-about-tab="${word}"]`).waitFor();
    if (word !== 'technical') await phone.locator('.about-page iframe.page-frame').waitFor();
    const wrong = await pageCrowded(phone, '.about-page');
    expect(`nothing of the page over anything else, past the window or scrolling sideways on a phone with ${word} showing: ${wrong.join('; ')}`, wrong.length === 0);
    await phone.screenshot({ path: `${out}/about-8-phone-${word}.png`, fullPage: true });
  }
  await phone.context().close();

  // Nothing of the page was fetched from anywhere but the daemon.
  const elsewhere = asked.filter((one) => !one.startsWith(url) && !one.startsWith('data:') && !one.startsWith('blob:') && one !== 'about:srcdoc');
  expect(`nothing asked of anywhere but the daemon, got ${elsewhere.join(', ')}`, elsewhere.length === 0);
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
