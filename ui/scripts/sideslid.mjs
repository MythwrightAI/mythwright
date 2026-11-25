// The side panel comes in and goes out as the drawer and the Tools panel
// do (`views/sideslide.ts`): folded away by Ctrl and `\`, the
// column given back at once (the workbench laid out once and not again as
// it moves) and a still copy of the panel going out the way it came over
// the room it gave back, behind the rail, nothing to press or reach, held
// half way out to be looked at; brought back, the column taken at once
// and the panel sliding into it from behind the rail by where it is drawn
// (what is past the rail's edge cut off as it goes), in the drawer's time
// and easing, held part of the way, and at rest only once it is; another
// place of the rail chosen while it is out there at once and not coming
// in; folded away and brought back by the rail, by the button at its head
// and by the one that stands in for it; on a phone, where the rail and the
// panel lie over the page, both coming in together from the screen's edge
// and a copy of both going out there as the page beside them is pressed,
// that page dimmed and lit again in the same time, and at once with less
// movement;
// nothing scrolling sideways throughout; and where less movement is asked
// for, nothing moving and no copy put there. Run plainly and with
// `LOOK="size=largest density=compact"`.
//
//   DAEMON=../target/debug/factory node scripts/sideslid.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { sideAtRest, watchSideComing, watchSideLeaving } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'sideslid-data');
const home = join(out, 'sideslid-home');
const looked = process.env.LOOK ? '-looked' : '';

await mkdir(out, { recursive: true });
for (const dir of [data, home]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

/** Plays on whatever was held on its way: the panel, the rail with it, or the copy. */
const playOn = (page) =>
  page.evaluate(() => {
    window.sideUnwatched?.();
    for (const one of document.querySelectorAll('.shell > .sidebar, .shell > .rail, .shell > .side-under, .side-leaving, .side-under-leaving')) for (const moving of one.getAnimations()) moving.play();
  });

/** Where things stand: the workbench, the rail, the panel or its copy. */
const boxes = (page) =>
  page.evaluate(() => {
    const of = (selector) => {
      const one = document.querySelector(selector);
      if (!one) return null;
      const box = one.getBoundingClientRect();
      return { x: Math.round(box.x), right: Math.round(box.right), width: Math.round(box.width), y: Math.round(box.y), height: Math.round(box.height) };
    };
    return { main: of('.main'), rail: of('.shell > .rail'), panel: of('.shell > .sidebar'), copy: of('.side-leaving') };
  });

/** How dimmed the page beside the panel is on a phone, by what dims it now or what stands in for that as it goes: its opacity, and nothing where neither is there. */
const dimmed = (page) =>
  page.evaluate(() => {
    const dim = document.querySelector('.shell > .side-under, .side-under-leaving');
    return dim ? Number(getComputedStyle(dim).opacity) : null;
  });

/** Whether anything of the page was scrolled sideways, or could be. */
const sideways = (page) =>
  page.evaluate(() => {
    const main = document.querySelector('.main');
    const shell = document.querySelector('.shell');
    return [main.scrollLeft, shell.scrollLeft, document.documentElement.scrollWidth - window.innerWidth, document.documentElement.scrollLeft].filter((one) => one > 0);
  });

/** Counts each time the workbench is laid out another size from here on (`window.laidOut`), which is what a reader sees move. */
const countLaidOut = (page) =>
  page.evaluate(
    () =>
      new Promise((told) => {
        window.laidOutBy?.disconnect();
        window.laidOut = 0;
        let first = true;
        window.laidOutBy = new ResizeObserver(() => {
          // The first is the observer being told where it stands, which is waited for before anything is done.
          if (first) return void ((first = false), told());
          window.laidOut += 1;
        });
        window.laidOutBy.observe(document.querySelector('.main'));
      }),
  );
const laidOut = (page) => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() => done(window.laidOut)))));

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', 'Shop', '--bare');
  await cli('channel', 'create', 'pantry', '--project', 'shop');

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
  const page = await context.newPage();
  page.setDefaultTimeout(20_000);
  page.on('pageerror', (err) => problems.push(err.message));
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const panel = page.locator('.shell > .sidebar');

  // ---- a page opened with the panel out has it there, at rest, not coming in --------------------------------
  await sideAtRest(page).waitFor();
  const opened = await page.evaluate(() => document.querySelector('.shell > .sidebar').getAnimations().length);
  expect(`a page opened with the panel out to have it there and at rest, moved by nothing, got ${opened} animations`, opened === 0);
  const usual = await boxes(page);
  await page.screenshot({ path: `${out}/sideslid-0-out${looked}.png` });

  /** Folded away by `how`: the column given back at once, and a still copy going out over it, held half way. */
  const foldedAway = async (where, how, wide) => {
    await watchSideLeaving(page);
    await countLaidOut(page);
    const was = await boxes(page);
    await how();
    await page.locator('.side-leaving').waitFor();
    const went = await page.evaluate(() => window.sideLeft);
    expect(
      `${where}: a still copy of the panel folded away to go out the way it came, in the drawer's time and easing, as nothing to press or reach, got ${JSON.stringify(went)}`,
      went.name === 'side-in' && went.backwards && went.took >= 240 && went.took <= 280 && went.eased === 'cubic-bezier(0.22, 0.61, 0.36, 1)' && went.inert && went.hidden === 'true' && went.pressed === 'none' && went.found === 0,
    );
    expect(`${where}: no panel to be found while its copy goes out`, (await panel.count()) === 0);
    const going = await boxes(page);
    expect(`${where}: the room given back at once, the workbench where the panel began, got ${JSON.stringify(going.main)} from ${JSON.stringify(was)}`, Math.abs(going.main.x - wide.main.x) <= 1 && Math.abs(going.main.width - wide.main.width) <= 1);
    expect(
      `${where}: the copy half way out behind the rail, as wide as the panel was and as tall, at ${JSON.stringify(going.copy)} from ${JSON.stringify(was.panel)}`,
      going.copy.x < was.panel.x - 20 && Math.abs(going.copy.width - was.panel.width) <= 1 && Math.abs(going.copy.height - was.panel.height) <= 1,
    );
    expect(`${where}: nothing scrolled sideways while it goes, got ${await sideways(page)}`, (await sideways(page)).length === 0);
    return { was, going };
  };
  /** Brought back by `how`: the column taken at once, and the panel sliding into it, held a sixth of the way. */
  const broughtBack = async (where, how) => {
    await watchSideComing(page);
    await countLaidOut(page);
    await how();
    await panel.waitFor();
    const slid = await page.evaluate(() => window.sideSlid);
    expect(
      `${where}: the panel to come in by where it is drawn, cut off at the rail's edge, in the drawer's time and easing, got ${JSON.stringify(slid)}`,
      slid.name === 'side-in' && slid.took >= 240 && slid.took <= 280 && slid.moved.join() === 'clipPath,transform' && slid.eased === 'cubic-bezier(0.22, 0.61, 0.36, 1)',
    );
    expect(`${where}: a panel on its way not to say that it is at rest`, !slid.rested && (await sideAtRest(page).count()) === 0);
    const coming = await boxes(page);
    expect(`${where}: the panel part of the way in, from behind the rail, at ${JSON.stringify(coming.panel)}`, coming.panel.x < usual.panel.x - 40 && coming.panel.x > usual.panel.x - usual.panel.width);
    const clip = await panel.evaluate((one) => getComputedStyle(one).clipPath);
    expect(`${where}: what is past the rail's edge cut off as it comes, got ${clip}`, clip.startsWith('inset('));
    expect(`${where}: the room taken at once, the workbench as it is with the panel out, got ${JSON.stringify(coming.main)}`, Math.abs(coming.main.x - usual.main.x) <= 1 && Math.abs(coming.main.width - usual.main.width) <= 1);
    expect(`${where}: nothing scrolled sideways while it comes, got ${await sideways(page)}`, (await sideways(page)).length === 0);
    return coming;
  };
  const rested = async (where) => {
    await playOn(page);
    await sideAtRest(page).waitFor();
    const at = await boxes(page);
    const still = await sideAtRest(page).evaluate((one) => [getComputedStyle(one).transform, getComputedStyle(one).clipPath, one.getAnimations().length]);
    expect(`${where}: the panel at rest where it always was, moved and cut by nothing, got ${still} at ${JSON.stringify(at.panel)}`, still.join() === 'none,none,0' && at.panel.x === usual.panel.x && at.panel.width === usual.panel.width);
    const times = await laidOut(page);
    expect(`${where}: the workbench laid out once for the panel brought back, and it was ${times} times`, times === 1);
  };
  const gone = async (where) => {
    await playOn(page);
    await page.locator('.side-leaving').waitFor({ state: 'detached' });
    const times = await laidOut(page);
    expect(`${where}: the workbench laid out once for the panel folded away, and it was ${times} times`, times === 1);
  };
  const ctrl = () => page.keyboard.press('Control+\\');

  // ---- by the keyboard ------------------------------------------------------------------------------------
  const wide = { main: { x: usual.rail.right, width: usual.main.right - usual.rail.right } };
  await foldedAway('folded by Ctrl and \\', ctrl, wide);
  await page.screenshot({ path: `${out}/sideslid-1-going${looked}.png` });
  await gone('folded by Ctrl and \\');
  const folded = await boxes(page);
  expect(`the workbench to have the column the panel had, got ${JSON.stringify(folded.main)}`, folded.main.x === usual.rail.right && folded.panel === null && folded.copy === null);
  await broughtBack('brought back by Ctrl and \\', ctrl);
  await page.screenshot({ path: `${out}/sideslid-2-coming${looked}.png` });
  await rested('brought back by Ctrl and \\');

  // ---- another place of the rail chosen while it is out: there at once, not coming in ---------------------
  await watchSideComing(page);
  await page.getByRole('button', { name: 'Resources', exact: true }).click();
  await page.getByRole('navigation', { name: 'Resources' }).waitFor();
  await sideAtRest(page).waitFor();
  const swapped = await page.evaluate(() => window.sideSlid);
  expect(`another panel in its place to be there at once and at rest, not coming in, got ${JSON.stringify(swapped)}`, swapped?.name === 'none' && swapped.rested === true);
  await page.evaluate(() => window.sideUnwatched());

  // ---- by the rail's place pressed again, and by the button that stands in for the panel's ----------------
  await foldedAway('folded by its place on the rail pressed again', () => page.getByRole('button', { name: 'Resources', exact: true }).click(), wide);
  await gone('folded by its place on the rail');
  await broughtBack('brought back by the button that stands in for it', () => page.getByRole('button', { name: 'Expand the side panel' }).click());
  await rested('brought back by the button');
  await foldedAway("folded by the button at the panel's head", () => panel.getByRole('button', { name: 'Collapse the side panel' }).click(), wide);
  await gone("folded by the button at the panel's head");
  await broughtBack('brought back by a place of the rail', () => page.getByRole('button', { name: 'Channels', exact: true }).click());
  await rested('brought back by a place of the rail');

  // ---- on a phone, where the rail and the panel lie over the page and come and go together ----------------
  const desk = page.viewportSize();
  await page.setViewportSize({ width: 390, height: 844 });
  // A screen that has just become that narrow puts the panel away, as something chosen in it does.
  await panel.waitFor({ state: 'detached' });
  await page.locator('.side-leaving').waitFor({ state: 'detached' });
  await watchSideComing(page);
  await page.getByRole('button', { name: 'Expand the side panel' }).first().click();
  await panel.waitFor();
  const phoneSlid = await page.evaluate(() => window.sideSlid);
  expect(`on a phone the rail and the panel to come in together, in the drawer's time, got ${JSON.stringify(phoneSlid)}`, phoneSlid.name === 'side-in-phone' && phoneSlid.rail === 'side-in-phone' && phoneSlid.took >= 240 && phoneSlid.took <= 280);
  const phoneComing = await boxes(page);
  expect(
    `on a phone the rail and the panel part of the way in from the screen's edge, side by side, at ${JSON.stringify(phoneComing)}`,
    phoneComing.rail.x < -20 && phoneComing.panel.x < phoneComing.rail.right + 1 && Math.abs(phoneComing.panel.x - phoneComing.rail.right) <= 1,
  );
  expect(`nothing scrolled sideways on a phone, got ${await sideways(page)}`, (await sideways(page)).length === 0);
  // The page beside them dimmed as they come, in their time and easing, and part of the way there with them.
  const dimming = await dimmed(page);
  expect(
    `on a phone the page beside them to be dimmed in the slide's time and easing, part of the way, got ${JSON.stringify(phoneSlid.scrim)} at ${dimming}`,
    phoneSlid.scrim?.name === 'scrim-in' && phoneSlid.scrim.took === phoneSlid.took && phoneSlid.scrim.eased === phoneSlid.eased && dimming > 0.05 && dimming < 0.95,
  );
  await page.screenshot({ path: `${out}/sideslid-3-phone-coming${looked}.png` });
  await playOn(page);
  await sideAtRest(page).waitFor();
  const phoneAt = await boxes(page);
  expect(`on a phone the rail at the screen's edge and the panel beside it at rest, got ${JSON.stringify(phoneAt)}`, phoneAt.rail.x === 0 && phoneAt.panel.x === phoneAt.rail.right);
  const dimmedAtRest = await page.evaluate(() => [Number(getComputedStyle(document.querySelector('.shell > .side-under')).opacity), document.querySelector('.shell > .side-under').getAnimations().length]);
  expect(`on a phone the page beside them wholly dimmed once they are at rest, moved by nothing, got ${dimmedAtRest}`, dimmedAtRest.join() === '1,0');
  await watchSideLeaving(page);
  // The page beside them pressed puts them away.
  await page.mouse.click(385, 420);
  await page.locator('.side-leaving').waitFor();
  const phoneWent = await page.evaluate(() => window.sideLeft);
  const phoneGoing = await boxes(page);
  expect(
    `on a phone a copy of the rail and the panel to go out together over the screen's edge, at ${JSON.stringify(phoneGoing.copy)}, got ${JSON.stringify(phoneWent)}`,
    phoneWent.backwards && phoneWent.found === 0 && phoneGoing.copy.x < -20 && Math.abs(phoneGoing.copy.width - (phoneAt.panel.right - phoneAt.rail.x)) <= 1,
  );
  // The page beside them lit again as they go: what dimmed it is gone at once, and what stands in for it fades out with them, nothing to press.
  const lighting = await dimmed(page);
  expect(
    `on a phone the page beside them lit again in the slide's time, what dims it gone and a stand-in nothing presses fading out part of the way, got ${JSON.stringify(phoneWent.scrim)} at ${lighting}`,
    (await page.locator('.shell > .side-under').count()) === 0 &&
      phoneWent.scrim?.name === 'scrim-in' && phoneWent.scrim.backwards && phoneWent.scrim.took === phoneWent.took && phoneWent.scrim.inert && phoneWent.scrim.pressed === 'none' &&
      lighting > 0.05 && lighting < 0.95,
  );
  await page.screenshot({ path: `${out}/sideslid-4-phone-going${looked}.png` });
  await playOn(page);
  await page.locator('.side-leaving').waitFor({ state: 'detached' });
  await page.locator('.side-under-leaving').waitFor({ state: 'detached' });

  // Where less movement is asked for, the page beside them is dimmed and lit again at once, with nothing standing in.
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.getByRole('button', { name: 'Expand the side panel' }).first().click();
  await sideAtRest(page).waitFor();
  const dimmedAtOnce = await page.evaluate(() => [Number(getComputedStyle(document.querySelector('.shell > .side-under')).opacity), document.querySelector('.shell > .side-under').getAnimations().length]);
  expect(`on a phone with less movement the page beside them wholly dimmed at once, moved by nothing, got ${dimmedAtOnce}`, dimmedAtOnce.join() === '1,0');
  await page.mouse.click(385, 420);
  await panel.waitFor({ state: 'detached' });
  expect('on a phone with less movement nothing standing in for the dimming, and no copy', (await page.locator('.side-under-leaving, .side-leaving').count()) === 0);
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.setViewportSize(desk);
  await page.getByRole('button', { name: 'Expand the side panel' }).first().click();
  await playOn(page);
  await sideAtRest(page).waitFor();

  // ---- where less movement is asked for, nothing moves and no copy is put there ---------------------------
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await ctrl();
  await panel.waitFor({ state: 'detached' });
  expect('no copy of the panel left where nothing moves one', (await page.locator('.side-leaving').count()) === 0);
  await watchSideComing(page);
  await ctrl();
  await sideAtRest(page).waitFor();
  const still = await page.evaluate(() => window.sideSlid);
  expect(`a panel moved by nothing there, and at rest as it is drawn, got ${JSON.stringify(still)}`, still.name === 'none' && still.rested === true);
  await page.evaluate(() => window.sideUnwatched());
  await page.emulateMedia({ reducedMotion: 'no-preference' });

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
