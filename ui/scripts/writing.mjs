// Writes in a document as blocks, saves it, and looks at it as a page.
//
//   DAEMON=../target/debug/factory node scripts/writing.mjs [out-dir]
//
// The document is a Markdown file before and after: what is checked at the
// end is the file. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { unfold, unfoldAll } from './folded.mjs';
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
const data = join(out, 'writing-data');
const home = join(out, 'writing-home');
// The project's folder is one that was said, so that its Repo is listed under it: a folder of its own, beside the data.
const shop = join(out, 'writing-shop');

const BEFORE = `---
publish: true
---
# Riverside Shop

A shop for *well made* things.

- mugs
- knives
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
  const page = await browser.newPage({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const problems = [];
  page.on('pageerror', (err) => problems.push(err.message));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const file = (path, content) =>
    page.evaluate(
      async ([path, content]) => {
        const at = '/api/projects/shop/file';
        const mine = { 'x-page-key': localStorage.getItem('page-key') ?? '' };
        if (content === null) return (await (await fetch(`${at}?path=${encodeURIComponent(path)}`, { headers: mine })).json()).content;
        const put = { method: 'PUT', headers: { ...mine, 'content-type': 'application/json' }, body: JSON.stringify({ path, content }) };
        if (!(await fetch(at, put)).ok) throw new Error(`could not write ${path}`);
        return content;
      },
      [path, content],
    );

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  await file('about.md', BEFORE);
  await file('prices.md', '# Prices\n\n| Item | Price |\n|---|---|\n| Mug | 9 |\n');
  await file('framed.md', '# Framed\n\n<div class="frame">Held in a frame.</div>\n');
  await side(page, 'Resources');
  // A project's files are under it, folded away until they are wanted: unfolded by their arrow where Expand all did not, their folder never having been looked at.
  await unfold(page, 'Shop repo');
  await page.locator('.nav').getByText('about.md').click();

  // It opens as blocks, with the settings at its head out of sight.
  const doc = page.locator('.writer .ProseMirror');
  await doc.locator('h1', { hasText: 'Riverside Shop' }).waitFor();
  expect('the settings block not to be shown as writing', !(await doc.innerText()).includes('publish'));
  expect('opening a document not to count as changing it', (await page.getByText('unsaved').count()) === 0);
  await page.screenshot({ path: `${out}/writing-1-opened.png` });

  // Typing: Markdown's own marks turn into blocks as they are typed.
  await doc.locator('li').last().click();
  await page.keyboard.press('End');
  // The editor hears where a click landed a moment after it, as a browser
  // reports it. A person is never faster than that; a script is.
  await page.waitForTimeout(150);
  await page.keyboard.press('Enter');
  await page.keyboard.press('Enter');
  await page.keyboard.type('## Plan');
  await page.keyboard.press('Enter');
  await page.keyboard.type('- first');
  await page.keyboard.press('Enter');
  await page.keyboard.type('second, in **bold**');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Enter');
  // The menu: `/` offers the blocks by name.
  await page.keyboard.type('/');
  await page.locator('.block-menu button').first().waitFor({ timeout: 5000 }).catch(async (err) => {
    // Say what the editor looked like, which a timeout alone does not.
    await page.screenshot({ path: `${out}/writing-failed.png` });
    const menu = await page.evaluate(() => {
      const box = document.querySelector('.block-menu');
      const line = window.getSelection()?.anchorNode?.parentElement;
      return { menu: box ? { hidden: box.hidden, children: box.children.length } : null, line: line?.outerHTML.slice(0, 200) };
    });
    throw new Error(`${err.message}\n${JSON.stringify(menu)}`);
  });
  await page.screenshot({ path: `${out}/writing-2-menu.png` });
  await page.keyboard.type('quo');
  expect('the menu to narrow to what was typed', (await page.locator('.block-menu button').count()) === 1);
  await page.keyboard.press('Enter');
  await page.keyboard.type('Said by someone.');
  expect('a heading', (await doc.locator('h2', { hasText: 'Plan' }).count()) === 1);
  expect('a list of two under it', (await doc.locator('h2 + ul > li').count()) === 2);
  expect('bold where it was marked', (await doc.locator('li strong', { hasText: 'bold' }).count()) === 1);
  expect('a quote from the menu', (await doc.locator('blockquote', { hasText: 'Said by someone.' }).count()) === 1);
  expect('the menu gone once it was used', await page.locator('.block-menu').isHidden());
  await page.screenshot({ path: `${out}/writing-3-written.png` });

  // Saved, it is the same file with the new blocks added, and nothing else moved.
  await page.getByText('unsaved').waitFor();
  await page.keyboard.press('Control+s');
  await page.waitForFunction(() => !document.body.innerText.includes('unsaved'));
  const after = await file('about.md', null);
  console.log(after);
  expect('what was there to be untouched', after.startsWith(BEFORE.trimEnd()));
  expect('the new blocks, as Markdown', after.includes('## Plan\n\n- first\n- second, in **bold**') && after.includes('> Said by someone.'));

  // As a page: built by the daemon, the way publishing would build it.
  await page.getByRole('tab', { name: 'Page' }).click();
  const frame = page.locator('iframe.page-frame');
  await frame.waitFor();
  const html = await frame.getAttribute('srcdoc');
  expect('the page to hold what was written', html.includes('<h2>Plan</h2>') && html.includes('<strong>bold</strong>') && html.includes('<blockquote>'));
  expect('the settings not to be on the page', !html.includes('publish: true'));
  await page.frameLocator('iframe.page-frame').locator('h2', { hasText: 'Plan' }).waitFor();
  await page.screenshot({ path: `${out}/writing-4-page.png` });
  await page.getByRole('tab', { name: 'Source' }).click();
  await page.locator('.cm-content', { hasText: '## Plan' }).waitFor();
  await page.getByRole('tab', { name: 'Write' }).click();
  await doc.locator('h2', { hasText: 'Plan' }).waitFor();

  // A table is opened as blocks, kept as the text it was written as.
  await page.locator('.nav').getByText('prices.md').click();
  await doc.locator('h1', { hasText: 'Prices' }).waitFor();
  await doc.locator('pre.writer-table', { hasText: '| Mug | 9 |' }).waitFor();
  expect('a table not to be taken for something blocks cannot keep', (await page.locator('.doc-note').count()) === 0);
  await page.screenshot({ path: `${out}/writing-5-table.png` });

  // A document with something blocks cannot keep is not opened as blocks.
  await page.locator('.nav').getByText('framed.md').click();
  await page.locator('.doc-note', { hasText: 'raw HTML' }).waitFor();
  await page.locator('.cm-content', { hasText: 'Held in a frame.' }).waitFor();
  await page.screenshot({ path: `${out}/writing-6-raw.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
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
