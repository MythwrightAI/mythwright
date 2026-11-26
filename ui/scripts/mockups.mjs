// A project's mockups: the gallery of small live pages with variants
// beside what they vary, one opened at a phone's width, two side by side,
// one moved on and spoken on, a variant made, a file changed by another
// program showing by itself, and one opened at its own address. And what
// a mockup's script can reach: its own files, and nothing of the daemon's
// or of whoever is looking.
//
//   DAEMON=../target/debug/factory node scripts/mockups.mjs [out-dir]
//
// Shop has three mockups and `std/mockups` applied to it; Depot has
// none, and no Mockups line. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
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
const data = join(out, 'mockups-data');
const home = join(out, 'mockups-home');

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

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  for (const name of ['Shop', 'Depot']) await cli('project', 'create', '--bare', name);
  const mockups = join(data, 'projects', 'shop', 'mockups');
  const page_of = (heading, extra = '') =>
    `<!doctype html><html><head><meta charset="utf-8"><title>${heading}</title><link rel="stylesheet" href="style.css"></head>` +
    `<body><h1>${heading}</h1><p id="ran">the script has not run</p><img src="art/mark.svg" alt="its mark" width="48" height="48">${extra}<script src="app.js"></script></body></html>\n`;
  const put = async (folder, files) => {
    await mkdir(join(mockups, folder, 'art'), { recursive: true });
    for (const [name, text] of Object.entries(files)) await writeFile(join(mockups, folder, name), text);
  };
  const beside = {
    'style.css': 'body { font: 18px system-ui; margin: 2rem; } h1 { color: teal; }\n',
    'app.js': "document.getElementById('ran').textContent = 'the script ran';\n",
    'art/mark.svg': '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="teal"/></svg>\n',
  };
  await put('pricing', { 'index.html': page_of('Three plans'), ...beside });
  await put('pricing-bold', { 'index.html': page_of('Three plans, bold'), ...beside, 'style.css': 'body { font: 700 20px system-ui; margin: 2rem; background: #111; color: #fff; }\n', 'mockup.yaml': 'title: Pricing, bold\nof: pricing\n' });
  await writeFile(join(mockups, 'pricing', 'mockup.yaml'), 'title: Pricing\n');
  // A mockup that tries what it should not be able to do, and writes down what it got.
  const probe = `<!doctype html><html><head><meta charset="utf-8"><title>Probe</title></head><body><pre id="result">working</pre><script>
(async () => {
  const got = [];
  try { got.push('cookie: ' + JSON.stringify(document.cookie)); } catch { got.push('cookie: refused'); }
  try { localStorage.setItem('a', 'b'); got.push('storage: open'); } catch { got.push('storage: refused'); }
  for (const path of ['/api/me', '/api/projects', '/api/collections/shop/mockups/records']) {
    try { const answer = await fetch(path, { credentials: 'include' }); got.push(path + ': ' + answer.status); } catch { got.push(path + ': refused'); }
  }
  try { const answer = await fetch('/api/system/stop', { method: 'POST', credentials: 'include' }); got.push('stop: ' + answer.status); } catch { got.push('stop: refused'); }
  try { got.push('own: ' + (await (await fetch('data.json')).json()).said); } catch { got.push('own: refused'); }
  try { got.push('parent: ' + (window.parent === window ? 'itself' : window.parent.document.title)); } catch { got.push('parent: refused'); }
  document.getElementById('result').textContent = got.join('\\n');
})();
</script></body></html>\n`;
  await put('probe', { 'index.html': probe, 'data.json': '{ "said": "its own file" }\n' });
  await cli('apply', 'std/mockups', '--project', 'shop');

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
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await side(page, 'Resources');
  const nav = page.locator('.nav');
  await nav.getByText('Depot', { exact: true }).waitFor();

  // Mockups is under the project that has some, and not under the one that has none.
  const project = (name) => nav.locator(':scope > .tree > [data-project]', { has: page.locator(':scope > .nav-row', { hasText: name }) });
  await project('Shop').getByRole('button', { name: / Mockups$/ }).waitFor();
  // Depot is drawn too: what it lacks is not lacking because nothing of it is there yet. Nobody said a
  // folder for either, so neither lists any Repo.
  await project('Depot').locator(':scope > .nav-row').waitFor();
  expect('Depot, which has no mockups, to have no Mockups line', (await project('Depot').getByRole('button', { name: / Mockups$/ }).count()) === 0);
  expect('neither project, each in a folder the factory made, to list any Repo', (await nav.getByRole('button', { name: 'Repo', exact: true }).count()) === 0);
  await project('Shop').getByRole('button', { name: / Mockups$/ }).click();
  await page.getByRole('heading', { name: 'Shop mockups', level: 1 }).waitFor();

  // The gallery: each as it is now, small and live, a variant beside what it varies.
  const shown = (title) => page.frameLocator(`iframe[title="${title}"]`);
  await shown('Pricing').getByText('the script ran').waitFor();
  await shown('Pricing, bold').getByRole('heading', { name: 'Three plans, bold' }).waitFor();
  await shown('Probe').getByText('own: its own file').waitFor();
  expect('three mockups in two groups', (await page.locator('.mockup-card').count()) === 3 && (await page.locator('.mockup-group').count()) === 2);
  const group = page.locator('.mockup-group', { has: page.locator('.mockup-line', { hasText: 'Pricing, bold' }) });
  expect('the variant to be beside what it varies', (await group.locator('.mockup-card').count()) === 2 && (await group.locator('.mockup-card.variant').count()) === 1);
  const mark = await shown('Pricing').locator('img').evaluate((img) => img.complete && img.naturalWidth > 0);
  expect('the picture beside the page to be shown in it', mark);
  await page.screenshot({ path: `${out}/mockups-0-gallery.png` });

  // One opened, as wide as a phone.
  await page.getByRole('button', { name: 'Open Pricing', exact: true }).click();
  await page.getByRole('heading', { name: 'Pricing', level: 1 }).waitFor();
  await page.getByRole('radio', { name: 'Phone' }).click();
  await shown('Pricing').getByText('the script ran').waitFor();
  const across = await page.locator('.mockup-holder').first().evaluate((holder) => holder.getBoundingClientRect().width);
  expect(`it to be shown 390 across, got ${across}`, Math.round(across) === 390);
  await page.screenshot({ path: `${out}/mockups-1-phone.png` });

  // Two side by side, each at the width chosen.
  await page.getByLabel('Beside it').selectOption({ label: 'Beside Pricing, bold' });
  await shown('Pricing, bold').getByText('the script ran').waitFor();
  expect('two on the stage', (await page.locator('.mockup-pane').count()) === 2);
  await page.screenshot({ path: `${out}/mockups-2-beside.png` });

  // Moved on and spoken on, and its history says so.
  await page.getByLabel('How far it has got').selectOption('review');
  await page.getByRole('button', { name: 'Comments', exact: true }).click();
  const drawer = drawerAtRest(page);
  const comments = drawer.getByRole('region', { name: 'Comments' });
  await comments.getByText('Nobody has commented on it yet.').waitFor();
  await comments.getByLabel('Add a comment').fill('The middle plan should stand out.');
  await comments.getByRole('button', { name: 'Comment', exact: true }).click();
  await comments.locator('.comment-words', { hasText: 'The middle plan should stand out.' }).waitFor();
  // What happened to it is after what was said on it, and holds none of what was said.
  await drawer.locator('.journey').getByText('changed its status').waitFor();
  expect('what was said to be among the comments and not said again in the history', (await drawer.locator('.journey .journey-spoken').count()) === 0);
  const [said, happened] = [await comments.boundingBox(), await drawer.locator('.journey').boundingBox()];
  expect(`the comments to be over the history, and they are at ${JSON.stringify([said, happened])}`, said && happened && said.y + said.height <= happened.y + 0.5);
  await page.screenshot({ path: `${out}/mockups-2-comments.png` });
  await drawer.getByRole('button', { name: 'Close' }).click();
  const kept = await readFile(join(mockups, 'pricing', 'mockup.yaml'), 'utf8');
  expect(`its stage to be kept beside its page, got ${kept}`, kept.includes('status: review'));

  // A file changed by another program: the page is drawn again by itself.
  await writeFile(join(mockups, 'pricing', 'index.html'), page_of('Four plans'));
  await shown('Pricing').getByRole('heading', { name: 'Four plans' }).waitFor();

  // A variant, made here as a copy to change.
  await page.getByLabel('Beside it').selectOption({ label: 'Alone' });
  await page.getByRole('button', { name: 'Variant', exact: true }).click();
  await page.getByLabel('What it is called').fill('Pricing, lighter');
  await page.getByRole('button', { name: 'Make it', exact: true }).click();
  await page.getByRole('heading', { name: 'Pricing, lighter', level: 1 }).waitFor();
  await shown('Pricing, lighter').getByRole('heading', { name: 'Four plans' }).waitFor();
  const copied = await readFile(join(mockups, 'pricing-lighter', 'mockup.yaml'), 'utf8');
  expect(`the variant to say what it is a variant of, got ${copied}`, copied.includes('of: pricing'));

  // What a mockup's script can reach, in the client's frame: its own
  // file, and nothing of the daemon's or of this page's.
  await page.getByRole('button', { name: '‹ All' }).click();
  await page.getByRole('button', { name: 'Open Probe', exact: true }).click();
  const told = async (frames) => {
    const result = frames.locator('#result');
    await result.getByText('parent:').waitFor();
    return result.innerText();
  };
  const held = (got, where) => {
    expect(`${where}: the session's cookie not to be read, got\n${got}`, got.includes('cookie: refused') || got.includes('cookie: ""'));
    expect(`${where}: no storage of this daemon's pages, got\n${got}`, got.includes('storage: refused'));
    for (const path of ['/api/me', '/api/projects', '/api/collections/shop/mockups/records']) expect(`${where}: ${path} not to be reached, got\n${got}`, got.includes(`${path}: refused`));
    expect(`${where}: the daemon not to be told to stop, got\n${got}`, got.includes('stop: refused'));
    expect(`${where}: its own file to be read, got\n${got}`, got.includes('own: its own file'));
  };
  const framed = await told(shown('Probe'));
  held(framed, 'in the frame');
  expect(`in the frame: the page around it not to be reached, got\n${framed}`, framed.includes('parent: refused'));
  console.log(`what a mockup's script got, in the client's frame:\n${framed}`);

  // At its own address, where the browser holds the session for this
  // daemon: the same, since it is put in a place of its own there too.
  const alone = await context.newPage();
  // Opened at its bare address, it is sent through the client, which says beside the cookie what this
  // page shows and sends it on: from there to a way in, which needs nothing.
  await alone.goto(`${url}/mockups/shop-mockups:probe`);
  await alone.waitForURL(/\/mockups\/in\//);
  expect(`its address to lead to a way in, got ${alone.url()}`, alone.url().includes('/mockups/in/') && !alone.url().includes('by='));
  const direct = await told(alone);
  held(direct, 'at its own address');
  console.log(`and at its own address:\n${direct}`);
  await alone.goto(`${url}/mockups/shop-mockups:pricing-bold`);
  await alone.getByText('the script ran').waitFor();
  console.log(`a mockup opened at its address is shown at ${alone.url()}`);
  await alone.screenshot({ path: `${out}/mockups-3-address.png` });
  // The daemon is still there, and still the owner's.
  const me = await page.evaluate(() => fetch('/api/me', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } }).then((answer) => answer.status));
  expect(`the client's own session to be as it was, got ${me}`, me === 200);

  // Nothing of it is there for someone who is not signed in: they come to where they sign in.
  const stranger = await browser.newContext();
  const outside = await stranger.newPage();
  await outside.goto(`${url}/mockups/shop-mockups:pricing`);
  await outside.locator('.opens-with').waitFor();
  expect(`someone not signed in to be led to no way in, got ${outside.url()}`, !outside.url().includes('/mockups/in/'));

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
