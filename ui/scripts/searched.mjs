// The search of everything, opened by Ctrl, Shift and F as by Ctrl and K.
//
//   DAEMON=../target/debug/factory node scripts/searched.mjs [out-dir]
//
// A factory with a project, Life, whose tables are rows of bugs and a
// library of guides; a wiki, Handbook, with a page under it and a reply
// said on that page; a channel and somebody else, Nell Quarry. Ctrl, Shift
// and F opens the search over the page, in the middle of it across, with
// the caret in its box, and leaves it open pressed again; Escape closes
// it. What is typed finds, each with its kind and where it is: a page of
// the wiki under the wiki, which opens its page; the reply, which opens its
// page's talk with it marked and in view; a row of a table, which opens in its
// form in the drawer; a document of the library, under its library and
// folder, which opens its file; the channel, which opens it; and Nell, who
// opens in the drawer. The arrows go through what was found and Enter
// opens the one marked; narrowing to one kind finds only that kind. Both
// keys are in the list of keyboard shortcuts. At a phone's width the
// search covers the page, its results and its kinds inside it, and
// nothing of it scrolls sideways. Plain and with `LOOK="size=largest
// density=compact"`; the environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const own = `searched-${process.pid}`;
const data = join(out, `${own}-data`);
const home = join(out, `${own}-home`);
const hers = join(out, `${own}-home-nell`);
const folder = join(out, `${own}-life`);
const looked = process.env.LOOK ? '-looked' : '';

const TABLES = `blueprint: tables
permissions: [projects]
steps:
  bugs:
    do: collection.ensure
    with:
      name: bugs
      fields: { title: { type: text, required: true }, notes: { type: text } }
  guides:
    do: collection.ensure
    with:
      name: guides
      store: documents
      at: notes/guides
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers, folder]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
await mkdir(join(folder, 'notes', 'guides', 'ops'), { recursive: true });
await writeFile(join(folder, 'notes', 'guides', 'ops', 'install.md'), `# Install\n\n${'Get the tools.\n\n'.repeat(40)}## Rolling back\n\nRestore the greengage snapshot.\n`);
const as = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env: as(home), stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });

let failed = null;
let lastPage = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell Quarry', '--password', 'correct horse', '--invite', code);
  await cli('project', 'create', 'Life', '--bare', '--path', folder);
  const plan = join(data, 'blueprints', 'projects', 'life', 'tables.bp');
  await mkdir(join(data, 'blueprints', 'projects', 'life'), { recursive: true });
  await writeFile(plan, TABLES);
  await cli('project', 'scan', 'life');
  await cli('apply', plan, '--project', 'life');
  await cli('channel', 'create', 'quarry-talk');
  const key = JSON.parse((await cli('key', 'filler', '--json')).stdout).token;
  const send = async (path, body) => {
    const answer = await fetch(`${url}/api${path}`, { method: 'POST', headers: { authorization: `Bearer ${key}`, 'content-type': 'application/json' }, body: JSON.stringify(body) });
    if (!answer.ok) throw new Error(`${path} was refused: ${answer.status} ${await answer.text()}`);
    return answer.json();
  };
  const row = await send('/collections/life/bugs/records', { fields: { title: 'Pantry door', notes: 'the hinge squeaks' } });
  const wiki = await send('/topics', { title: 'Handbook', body: 'Everything we do.' });
  const page = await send('/topics', { title: 'Rolling back', body: 'Restore the quince snapshot.', under: wiki.id });
  const reply = await send(`/topics/${page.id}/replies`, { body: 'the medlar way is faster' });
  await cli('project', 'scan', 'life');

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const scheme = process.env.SCHEME ?? 'dark';
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const signedIn = async (viewport, more = {}) => {
    const opened = await (await browser.newContext({ viewport, colorScheme: scheme, ...more })).newPage();
    opened.on('pageerror', (err) => problems.push(err.message));
    await opened.goto(url);
    await opened.getByLabel('Email').fill('ada@example.com');
    await opened.getByLabel('Password').fill('correct horse');
    await opened.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await opened.locator('.rail').waitFor({ state: 'attached' });
    lastPage = opened;
    return opened;
  };

  const desk = await signedIn({ width: 1350, height: 860 });
  const box = desk.getByLabel('Search', { exact: true });
  const modal = desk.locator('.modal.wide');
  const hits = desk.locator('.palette-results .hit');
  /** Each hit as it reads: its kind, what it is called and where it is; as said in the page, and read from here. */
  const HITS = `[...document.querySelectorAll('.palette-results .hit')].map((hit) => [...hit.querySelectorAll('.row > *')].map((one) => one.textContent.trim()).filter(Boolean).join(' | '))`;
  const read = () => desk.evaluate(HITS);
  /**
   * Waits until `ok` holds in the page as it is drawn (a function, or an expression, given `arg`), and says what
   * `readNow` reads where it never does.
   */
  const until = async (ok, what, readNow, arg) => {
    const held = await desk.waitForFunction(ok, arg, { timeout: 8000 }).then(
      () => true,
      () => false,
    );
    expect(held ? what : `${what}, got ${JSON.stringify(await readNow())}`, held);
  };
  /** Waits until what is found reads `wanted` among its lines. */
  const finds = async (typed, wanted) => {
    await box.fill(typed);
    await until(`${HITS}.includes(${JSON.stringify(wanted)})`, `"${typed}" to find "${wanted}"`, read);
    return (await read()).indexOf(wanted);
  };
  const open = async (keys = 'Control+Shift+F') => {
    await desk.keyboard.press(keys);
    await box.waitFor();
  };
  const choose = async (typed, wanted) => {
    await open();
    const at = await finds(typed, wanted);
    // Through the arrows to it, and Enter.
    for (let step = 0; step < at; step += 1) await box.press('ArrowDown');
    expect(`the arrows to mark "${wanted}"`, (await hits.nth(at).getAttribute('aria-selected')) === 'true');
    await box.press('Enter');
    await modal.waitFor({ state: 'detached' });
  };
  const shownTab = () => desk.locator('.tab[aria-selected="true"]').first().innerText();
  /** Whether the tab shown says any of `words`, in the page. */
  const tabSays = (words) => words.some((one) => document.querySelector('.tab[aria-selected="true"]')?.textContent.includes(one));
  /** Whether the drawer's head says `words`, in the page. */
  const drawerSays = (words) => document.querySelector('.drawer-head')?.textContent.includes(words) ?? false;
  const drawerHead = () => desk.locator('.drawer-head').innerText().catch(() => '');

  // Ctrl, Shift and F opens it over the page, in the middle across, with the caret in its box.
  await open();
  const laid = await modal.evaluate((one) => {
    const { left, right, top } = one.getBoundingClientRect();
    return { left, right, top, width: window.innerWidth, focused: document.activeElement?.getAttribute('aria-label') };
  });
  expect(`the search in the middle of the page across: ${JSON.stringify(laid)}`, Math.abs((laid.left + laid.right) / 2 - laid.width / 2) < 2 && laid.top > 0 && laid.focused === 'Search');
  await desk.keyboard.press('Control+Shift+F');
  expect('the search to stay open, pressed again', (await modal.count()) === 1);
  await desk.screenshot({ path: `${out}/searched-0-open${looked}.png` });
  await desk.keyboard.press('Escape');
  await modal.waitFor({ state: 'detached' });

  // A page of the wiki, under the wiki, opens its page.
  await choose('quince', 'topic | Rolling back | Handbook');
  await until(() => decodeURIComponent(location.pathname).includes('Rolling back'), 'the page of the wiki to be opened', () => desk.evaluate(() => decodeURIComponent(location.pathname)));
  // The reply opens its page's talk at it.
  await open();
  await finds('medlar', 'reply | Rolling back | Handbook');
  await desk.screenshot({ path: `${out}/searched-1-reply${looked}.png` });
  await box.press('Enter');
  // Its talk shows, with the reply marked as the one asked for, and in view.
  await until(() => location.hash.startsWith('#talk'), 'the reply to open its talk', () => desk.evaluate(() => location.hash));
  const marked = desk.locator('.topic-reply.asked[aria-current="true"]');
  await until(() => document.querySelector('.topic-reply.asked[aria-current="true"]')?.textContent.includes('medlar') ?? false, `the reply said at ${reply.seq} to be marked`, () => marked.innerText().catch(() => ''));
  const seen = await marked.evaluate((one) => {
    const { top, bottom } = one.getBoundingClientRect();
    return top >= 0 && bottom <= window.innerHeight;
  });
  expect('the reply marked to be in view', seen);
  // A row, in its form in the drawer.
  await choose('hinge', 'row | Pantry door | Life · bugs');
  await until(drawerSays, `the row ${row.id} in its form`, drawerHead, 'Pantry door');
  await desk.keyboard.press('Escape');
  await desk.locator('.drawer').waitFor({ state: 'detached' });
  // A document, under its library and folder, opens its file.
  await choose('greengage', 'document | Install | Life · guides › ops');
  await until(tabSays, 'the document to be opened', shownTab, ['install.md', 'Install']);
  // At the heading nearest what matched, in view, far down the document.
  const heading = () => {
    const one = [...document.querySelectorAll('.view[data-focused="true"] :is(h1, h2, h3, h4, h5, h6)')].find((each) => each.textContent.trim() === 'Rolling back');
    if (!one) return 'none';
    const { top, bottom } = one.getBoundingClientRect();
    return top >= 0 && bottom <= window.innerHeight ? 'in view' : `at ${Math.round(top)}`;
  };
  await until(() => {
    const one = [...document.querySelectorAll('.view[data-focused="true"] :is(h1, h2, h3, h4, h5, h6)')].find((each) => each.textContent.trim() === 'Rolling back');
    const box = one?.getBoundingClientRect();
    return box !== undefined && box.top >= 0 && box.bottom <= window.innerHeight;
  }, 'the heading nearest what matched to be in view', () => desk.evaluate(heading));
  await desk.screenshot({ path: `${out}/searched-2-document${looked}.png` });
  // The channel, by what it is called.
  await choose('quarry', 'channel | #quarry-talk');
  await until(tabSays, 'the channel to be opened', shownTab, ['quarry-talk']);
  // Somebody, in the drawer.
  await choose('nell qu', 'person | Nell Quarry');
  await until(drawerSays, 'Nell to be shown', drawerHead, 'Nell Quarry');
  await desk.keyboard.press('Escape');
  await desk.locator('.drawer').waitFor({ state: 'detached' });

  // Narrowed to one kind, only that kind; and Ctrl and K is the same search.
  await open('Control+k');
  await box.fill('quarry');
  await desk.getByRole('group', { name: 'Look among' }).getByRole('button', { name: 'People' }).click();
  await until(`${HITS}.join('\\n') === 'person | Nell Quarry'`, 'narrowed to people, only Nell', read);
  expect('the caret still in the box', await box.evaluate((one) => document.activeElement === one));
  await desk.screenshot({ path: `${out}/searched-3-narrowed${looked}.png` });
  await desk.keyboard.press('Escape');

  // Both keys are in the list of shortcuts.
  await desk.keyboard.press('Control+Shift+?');
  const listed = await desk.locator('.shortcuts').innerText();
  expect(`both keys listed, got ${listed.slice(0, 300)}`, /Search everything\s+Ctrl\s+K/.test(listed) && /as an editor searches its files\s+Ctrl\s+Shift\s+F/.test(listed));
  await desk.keyboard.press('Escape');

  // At a phone's width it covers the page, and nothing of it scrolls sideways.
  for (const width of [390, 412]) {
    const phone = await signedIn({ width, height: 800 }, { hasTouch: true, isMobile: true });
    await phone.keyboard.press('Control+Shift+F');
    const phoneBox = phone.getByLabel('Search', { exact: true });
    await phoneBox.waitFor();
    await phoneBox.fill('quarry');
    await phone.locator('.palette-results .hit').first().waitFor();
    const fits = await phone.locator('.modal.wide').evaluate((one) => {
      const { left, right } = one.getBoundingClientRect();
      const inside = [...one.querySelectorAll('.hit, .palette-tools button, .palette-input')].every((part) => {
        const box = part.getBoundingClientRect();
        return box.left >= left - 0.5 && box.right <= right + 0.5;
      });
      return { left, right, width: window.innerWidth, inside, sideways: Math.max(document.documentElement.scrollWidth - window.innerWidth, one.scrollWidth - one.clientWidth) };
    });
    expect(`the search across the phone with everything of it inside, at ${width}: ${JSON.stringify(fits)}`, fits.left >= -0.5 && fits.right <= fits.width + 0.5 && fits.right - fits.left >= fits.width - 40 && fits.inside && fits.sideways <= 0);
    await phone.screenshot({ path: `${out}/searched-4-phone-${width}${looked}.png` });
    await phone.context().close();
  }

  expect(`no errors on the page: ${problems.join('; ')}`, problems.length === 0);
  await browser.close();
} catch (err) {
  failed = err;
  await lastPage?.screenshot({ path: `${out}/searched-failed${looked}.png` }).catch(() => {});
} finally {
  daemon.kill();
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`searched: every kind found and opened where it should be${looked ? ', with a look' : ''}`);
