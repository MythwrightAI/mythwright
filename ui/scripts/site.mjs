// A library written out as pages (`std/site`), opened straight from the
// disk: the front page, a document, the links between them followed, a
// picture shown, and nothing in any of it running or fetched from
// anywhere else; and the library's contents beside every page, in the
// order a folder's front page says (`order:`), with the page showing
// marked, held by the boxes at three widths and a phone's: beside the page
// where there is room and under it where there is not, with the way to
// them at the page's head, nothing over anything else, nothing past the
// window's sides, and a finger's size on a phone.
//
//   DAEMON=../target/debug/factory node scripts/site.mjs [out-dir]
//
// With FOLDER set to a folder of documents (this repository's docs/, say)
// that folder is what is written out, every document of it; otherwise a
// small library of this check's own is, with one document that is not
// published. The environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { cp, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'site-data');
const home = join(out, 'site-home');
const given = process.env.FOLDER ? resolve(process.env.FOLDER) : null;

// A picture that is one red dot, so that a page has something to show.
const DOT = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==',
  'base64',
);

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
  await cli('project', 'create', '--bare', 'Shop');
  const project = join(data, 'projects', 'shop');
  const docs = join(project, 'docs');
  // Only what this check puts there: a new project has a page or two of its own.
  await rm(docs, { recursive: true, force: true });
  if (given) {
    await cp(given, docs, { recursive: true });
  } else {
    await mkdir(join(docs, 'guides'), { recursive: true });
    await writeFile(
      join(docs, 'README.md'),
      '---\npublic: true\norder: [guides/, faq]\n---\n\n# Welcome\n\nStart with [installing it](guides/install.md), or see [the guides](guides/).\n\n![a dot](dot.png)\n\nNot for everyone: [the plan](plan.md).\n\n<script>document.title = "ran"</script>\n',
    );
    await writeFile(
      join(docs, 'guides', 'install.md'),
      '---\npublic: true\n---\n\n# Installing it\n\nRun the installer, then come [back](../README.md).\n\n| Step | What happens |\n|---|---|\n| One | It is unpacked |\n',
    );
    await writeFile(
      join(docs, 'guides', 'usage.md'),
      '---\npublic: true\n---\n\n# Using it, every day, with a title long enough to wrap in a narrow column\n\nSee [installing it](install.md) first.\n',
    );
    await writeFile(join(docs, 'faq.md'), '---\npublic: true\n---\n\n# Asked often\n\nAsk away.\n');
    await writeFile(join(docs, 'about.md'), '---\npublic: true\n---\n\n# About us\n\nWho we are.\n');
    await writeFile(join(docs, 'plan.md'), '# The plan\n\nNot said aloud yet.\n');
    await writeFile(join(docs, 'dot.png'), DOT);
  }
  await cli('apply', 'std/docs', '--project', 'shop');
  const started = Date.now();
  const applied = await cli('apply', 'std/site', '--project', 'shop', ...(given ? ['--param', 'publish=all'] : []));
  console.log(`written out in ${Date.now() - started} ms, the command included`);
  console.log(applied.stdout.trim().split('\n').slice(-1)[0].slice(0, 400));
  const site = join(project, 'site');

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
  const asked = [];
  page.on('pageerror', (err) => problems.push(err.message));
  // Whatever a page asks for is kept, to see that all of it is a file of the folder.
  page.on('request', (request) => asked.push(request.url()));
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  // The style sheet is beside the pages, and is what lays a page out.
  const styled = () => page.evaluate(() => getComputedStyle(document.querySelector('main')).maxWidth);

  // The front page, opened as a file.
  await page.goto(pathToFileURL(join(site, 'index.html')).href);
  expect(`the front page to be laid out by the style sheet beside it, got ${await styled()}`, (await styled()) === '720px');
  await page.screenshot({ path: `${out}/site-0-front.png` });

  if (given) {
    // Three links followed, each to a file that is there.
    const links = page.locator('main li a');
    expect('the front page to list the documents', (await links.count()) >= 3);
    for (const nth of [0, 1, 2]) {
      const name = await links.nth(nth).innerText();
      await links.nth(nth).click();
      await page.locator('.page-up a').waitFor();
      expect(`the page of ${name} to be laid out`, (await styled()) === '720px');
      if (nth === 0) await page.screenshot({ path: `${out}/site-1-document.png` });
      await page.locator('.page-up a').click();
      await page.locator('main li a').first().waitFor();
    }
  } else {
    // A folder's front page is the document that is one, where it has one.
    expect('the front page to be the document that is one', (await page.locator('h1').innerText()) === 'Welcome');
    // The picture a page uses is beside it, and shows.
    const shown = await page.locator('img[alt="a dot"]').evaluate((img) => img.complete && img.naturalWidth > 0);
    expect('the picture to show', shown);
    // What is not published is words, not a link.
    expect('the plan not to be linked to', (await page.locator('main').getByRole('link', { name: 'the plan' }).count()) === 0);
    expect('its words to be there still', (await page.locator('main').innerText()).includes('the plan'));
    // Script written into a document is in the page as it was written, and does not run.
    expect(`no script to have run, but the title is "${await page.title()}"`, (await page.title()) === 'Welcome');
    // Three links followed: to a document, back to the front, and to a folder.
    await page.locator('main').getByRole('link', { name: 'installing it' }).click();
    await page.getByRole('heading', { name: 'Installing it', level: 1 }).waitFor();
    expect('the document to be laid out', (await styled()) === '720px');
    await page.screenshot({ path: `${out}/site-1-document.png` });
    await page.locator('main').getByRole('link', { name: 'back' }).click();
    await page.getByRole('heading', { name: 'Welcome', level: 1 }).waitFor();
    await page.locator('main').getByRole('link', { name: 'the guides' }).click();
    await page.getByRole('heading', { name: 'guides', level: 1 }).waitFor();
    await page.locator('main').getByRole('link', { name: 'Shop docs' }).click();
    await page.getByRole('heading', { name: 'Welcome', level: 1 }).waitFor();
    expect('the plan not to have been written out', !(await readdir(site)).includes('plan.html'));
    const front = await readFile(join(site, 'index.html'), 'utf8');
    expect('no link in the front page to begin at the top of an address', !/(href|src)="\//.test(front));
  }
  // The contents beside a page, by the boxes, at three widths and a phone's.
  const contents = async (file, width, phone) => {
    const look = await browser.newContext({
      viewport: { width, height: phone ? 844 : 860 },
      isMobile: phone,
      hasTouch: phone,
      deviceScaleFactor: phone ? 2 : 1,
      colorScheme: process.env.SCHEME ?? 'dark',
    });
    const shown = await look.newPage();
    shown.on('pageerror', (err) => problems.push(err.message));
    shown.on('request', (request) => asked.push(request.url()));
    await shown.goto(pathToFileURL(join(site, file)).href);
    const boxes = await shown.evaluate(() => {
      const box = (el) => {
        const r = el.getBoundingClientRect();
        return { left: r.left, right: r.right, top: r.top + scrollY, bottom: r.bottom + scrollY, width: r.width, height: r.height };
      };
      const nav = document.querySelector('nav.page-nav');
      const to = document.querySelector('.page-to-nav a');
      return {
        nav: nav && box(nav),
        main: box(document.querySelector('main')),
        links: [...(nav?.querySelectorAll('a') ?? [])].map((a) => ({ ...box(a), words: a.innerText, here: a.getAttribute('aria-current') === 'page', ground: getComputedStyle(a).backgroundColor })),
        to: to && to.offsetParent !== null ? box(to) : null,
        wide: document.documentElement.scrollWidth,
        window: document.documentElement.clientWidth,
        scripts: nav ? nav.querySelectorAll('script, [onclick], form, button, input').length : 0,
      };
    });
    const at = `${file} at ${width}${phone ? ' (a phone)' : ''}`;
    expect(`${at}: contents beside the page`, boxes.nav && boxes.links.length > 0);
    expect(`${at}: nothing scrolling sideways, but it is ${boxes.wide} wide in ${boxes.window}`, boxes.wide <= boxes.window);
    expect(`${at}: nothing in the contents but links`, boxes.scripts === 0);
    const marked = boxes.links.filter((link) => link.here);
    expect(`${at}: one link marked as the page showing, but ${marked.length}`, marked.length === 1);
    const ground = boxes.links.find((link) => !link.here)?.ground;
    expect(`${at}: the page showing drawn otherwise than the rest`, marked[0].ground !== ground);
    for (const link of boxes.links) {
      expect(`${at}: ${link.words} wholly in the window`, link.left >= 0 && link.right <= boxes.window + 0.5);
    }
    for (const [nth, one] of boxes.links.entries()) {
      for (const other of boxes.links.slice(nth + 1)) {
        const over = one.left < other.right - 0.5 && other.left < one.right - 0.5 && one.top < other.bottom - 0.5 && other.top < one.bottom - 0.5;
        expect(`${at}: ${one.words} and ${other.words} not over each other`, !over);
      }
    }
    const { nav, main } = boxes;
    const apart = nav.right <= main.left + 0.5 || main.right <= nav.left + 0.5 || nav.bottom <= main.top + 0.5 || main.bottom <= nav.top + 0.5;
    expect(`${at}: the contents over none of the page`, apart);
    if (width >= 960) {
      // Beside the page, at its left, with no way to them at its head.
      expect(`${at}: the contents at the page's left`, nav.right <= main.left + 0.5 && nav.top < main.top + 60);
      expect(`${at}: no way to them at the head`, boxes.to === null);
      expect(`${at}: the page's column as wide as it is`, main.width >= 600);
    } else {
      // Under the page, with the way to them at its head, which leads there.
      expect(`${at}: the contents under the page`, nav.top >= main.bottom - 0.5);
      expect(`${at}: a way to them at the head`, boxes.to && boxes.to.top < main.top + 60);
      if (phone) {
        for (const link of boxes.links) expect(`${at}: ${link.words} a finger's size, but ${link.height}`, link.height >= 44);
      }
      await shown.locator('.page-to-nav a').click();
      const inView = await shown.evaluate(() => {
        const r = document.querySelector('nav.page-nav').getBoundingClientRect();
        return r.top >= -1 && r.top < innerHeight;
      });
      expect(`${at}: the way at the head leading to the contents`, inView);
      await shown.evaluate(() => scrollTo(0, 0));
    }
    await shown.screenshot({ path: `${out}/site-contents-${file.replace(/\W+/g, '-')}-${width}${phone ? '-phone' : ''}.png`, fullPage: true });
    await look.close();
    return boxes.links;
  };
  const pages = given ? ['index.html'] : ['index.html', 'guides/install.html', 'guides/index.html'];
  for (const file of pages) {
    for (const [width, phone] of [[1280, false], [1024, false], [800, false], [390, true]]) {
      const links = await contents(file, width, phone);
      if (!given) {
        // In the order the front page says, the rest by title; the folder's page its folder's line.
        const words = links.map((link) => link.words.replace(/\s+/g, ' '));
        const wanted = ['Shop docs', 'guides', 'Installing it', 'Using it, every day, with a title long enough to wrap in a narrow column', 'Asked often', 'About us'];
        expect(`${file}: the contents in their order, but ${JSON.stringify(words)}`, JSON.stringify(words) === JSON.stringify(wanted));
        const here = { 'index.html': 'Shop docs', 'guides/install.html': 'Installing it', 'guides/index.html': 'guides' }[file];
        expect(`${file}: ${here} marked`, links.find((link) => link.here)?.words === here);
      }
    }
  }
  if (!given) {
    // A link of the contents followed, to a page that is there.
    await page.goto(pathToFileURL(join(site, 'guides/install.html')).href);
    await page.locator('nav.page-nav').getByRole('link', { name: 'Asked often' }).click();
    await page.getByRole('heading', { name: 'Asked often', level: 1 }).waitFor();
    expect('the page followed to be marked', (await page.locator('nav.page-nav a[aria-current="page"]').innerText()) === 'Asked often');
  }

  const elsewhere = asked.filter((one) => !one.startsWith(pathToFileURL(site).href));
  expect(`nothing to be asked for outside the folder, but there was ${elsewhere.join(', ')}`, elsewhere.length === 0);

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
