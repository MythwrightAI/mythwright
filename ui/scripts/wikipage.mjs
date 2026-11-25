// A page of a wiki as a page of its own, and the addresses that open it.
//
//   DAEMON=../target/debug/factory node scripts/wikipage.mjs [out-dir]
//
// Ada owns the factory and makes from a terminal a wiki, Handbook, with a
// page three deep in it (Handbook › Deploying › Rolling back) that has
// pages under it, and two wikis both called Runbook, one with a page also
// called Rolling back and the other with Escalation; Nell registered by an
// invitation. A page is opened by its address (its wiki's title and its
// own), where it says where it is; by an address there was before (its
// title alone, its id, one written before addresses were paths), said
// the new way; by its own title under a wiki's title two wikis share; and
// by a title two pages share, or a wiki's title two wikis share, the page
// saying which they are, each a link that opens it. A page of a wiki two
// share is said by that wiki's id. Moved within its wiki from a terminal,
// the page left open says where it is now and keeps its address, which
// opens it on another page. Deploying says nothing and holds pages: a
// wiki's page, it is an article, opened on Article; Ada, who may write it,
// has its post to write in; somebody who may not (a page told it holds no
// `records:write`) is shown its path, its title as words, the pages in it
// and no post, and its replies on Talk. Named in a reply on
// the page three deep, Nell's inbox line says the page by its path (where
// it is now, which it follows on a page never read again), and
// pressed opens it on Talk at that reply, marked and in view, the chain it
// is in unfolded. Nothing of the page lies over anything else or scrolls
// sideways at 1280, 1000 and 800 wide and a phone's. The environment
// variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'wikipage-data');
const home = join(out, 'wikipage-home');
const hers = join(out, 'wikipage-home-nell');

await mkdir(out, { recursive: true });
for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, hers]) await mkdir(dir, { recursive: true });
const as = (dir) => ({
  ...process.env,
  HOME: dir,
  XDG_CONFIG_HOME: join(dir, 'config'),
  XDG_DATA_HOME: join(dir, 'share'),
});
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env: as(home),
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(home) });
const nell = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env: as(hers) });
const made = async (title, ...more) => JSON.parse((await cli('topic', 'make', title, ...more, '--json')).stdout).id;
const at = (...titles) => `${url}/topic/${titles.map(encodeURIComponent).join('/')}`;

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('tier', 'premium');
  const code = (await cli('invite', 'nell@example.com')).stdout.split('\n')[0].replace('invitation code: ', '').trim();
  await nell('register', '--email', 'nell@example.com', '--name', 'Nell', '--password', 'correct horse', '--invite', code);
  const handbook = await made('Handbook', '--body', 'How we work.');
  const deploying = await made('Deploying', '--under', handbook);
  const back = await made('Rolling back', '--under', deploying, '--body', 'Roll back by the tag the deploy made.');
  await made('With a database', '--under', back);
  await made('Without one', '--under', back);
  const first = await made('Runbook');
  const other = await made('Rolling back', '--under', first);
  const second = await made('Runbook');
  const escalation = await made('Escalation', '--under', second);
  // A long chain of replies on the page three deep, and one at its end that names Nell.
  let under = null;
  for (let nth = 1; nth <= 7; nth += 1) {
    const words = nth === 7 ? '@nell which tag do we roll back to?' : `Step ${nth} of rolling back, which is one of many.`;
    under = (await cli('topic', 'reply', back, words, ...(under ? ['--to', under] : []))).stdout.trim().replace('said at ', '');
  }

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
  const DESK = { viewport: { width: 1280, height: 860 } };
  /** A page signed in at `to`; one that `reads` is told whoever is signed in holds no `records:write`, as somebody who may only read. */
  const signedIn = async (email, to, { how = DESK, reads = false } = {}) => {
    const context = await browser.newContext({ ...how, colorScheme: scheme });
    const page = await context.newPage();
    page.on('pageerror', (err) => problems.push(`${email}: ${err.message}`));
    if (reads) {
      await page.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = me.principal.scopes
          .filter((scope) => !['*', 'records', 'records:write'].includes(scope))
          .concat(['records:read', 'channels:read', 'inbox:read']);
        await route.fulfill({ response: answer, json: me });
      });
    }
    await page.goto(to);
    await page.getByLabel('Email').fill(email);
    await page.getByLabel('Password').fill('correct horse');
    await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    return page;
  };
  const shown = async (locator, what) => {
    await locator.waitFor({ timeout: 10000 }).catch(() => {
      throw new Error(`expected ${what}`);
    });
  };
  /** The page showing, by its id. */
  const pageOf = (on, id) => on.locator(`.topic-page[data-topic="${id}"]`);
  const titleOf = async (where) => {
    const title = where.locator('.topic-page-title').first();
    return title.evaluate((one) => one.value ?? one.innerText);
  };
  const pathOf = (where) => where.locator('.topic-path .topic-path-page').allInnerTexts();
  const opened = async (on, id, title, path, what) => {
    const where = pageOf(on, id);
    await shown(where.locator('.topic-page-title'), what);
    expect(`${what}: called ${title}, got ${await titleOf(where)}`, (await titleOf(where)) === title);
    await on
      .waitForFunction(
        (said) => [...document.querySelectorAll('.topic-path .topic-path-page')].map((one) => one.innerText).join(' › ') === said,
        path.join(' › '),
        { timeout: 10000 },
      )
      .catch(async () => {
        throw new Error(`${what}: its path ${path.join(' › ')}, got ${(await pathOf(where)).join(' › ')}`);
      });
    return where;
  };
  const addressed = (on, path) =>
    on.waitForFunction((said) => location.pathname === said, path, {
      timeout: 10000,
    });
  const laid = async (on, what) => {
    const wrong = await pageCrowded(on, '.topic-page');
    expect(`nothing of the page over anything else ${what}: ${wrong.join('; ')}`, wrong.length === 0);
    const sideways = await on.evaluate(() => [document.documentElement, document.querySelector('.topic-page')].map((one) => one.scrollWidth - one.clientWidth));
    expect(
      `nothing to scroll sideways ${what}, got ${sideways.join(', ')}px`,
      sideways.every((one) => one <= 0),
    );
  };

  // ── A page three deep by its address, its wiki's title and its own ──────
  const page = await signedIn('ada@example.com', at('Handbook', 'Rolling back'));
  const deep = await opened(page, back, 'Rolling back', ['Handbook', 'Deploying'], 'the page three deep opened by its address');
  await addressed(page, '/topic/Handbook/Rolling%20back');
  await shown(deep.locator('.topic-pages-line', { hasText: 'With a database' }), 'the pages in it');
  expect('the pages in it in the order they stand', (await deep.locator('.topic-pages-title').allInnerTexts()).join(' | ') === 'With a database | Without one');
  await page.screenshot({ path: `${out}/wikipage-01-deep.png` });

  // Every address there was: its title alone is two pages now, so the page says which; its id; one from before paths.
  for (const [old, what] of [
    [`${url}/topic/${encodeURIComponent(back)}`, 'its id'],
    [`${url}/#/topic/${encodeURIComponent(back)}`, 'an address from before addresses were paths'],
  ]) {
    await page.goto(old);
    await opened(page, back, 'Rolling back', ['Handbook', 'Deploying'], `the page opened by ${what}`);
    await addressed(page, '/topic/Handbook/Rolling%20back');
  }
  await page.goto(at('Rolling back'));
  const which = page.locator('.topic-page', {
    hasText: '2 pages are called “Rolling back”',
  });
  await shown(which, 'a title two pages share saying which pages are called so');
  expect(
    `each said by where it is, got ${await which.locator('.topic-pages-title').allInnerTexts()}`,
    (await which.locator('.topic-pages-title').allInnerTexts()).sort().join(' | ') === 'Handbook › Deploying › Rolling back | Runbook › Rolling back',
  );
  await page.screenshot({ path: `${out}/wikipage-02-several.png` });
  await which.locator('.topic-pages-line', { hasText: 'Runbook › Rolling back' }).click();
  await opened(page, other, 'Rolling back', ['Runbook'], 'the one pressed opened');

  // ── A wiki's title two wikis share ──────────────────────────────────────
  await page.goto(at('Runbook', 'Escalation'));
  await opened(page, escalation, 'Escalation', ['Runbook'], 'a page by its title under a wiki title two wikis share');
  await addressed(page, `/topic/${encodeURIComponent(second)}/Escalation`);
  await page.goto(at('Runbook'));
  const wikis = page.locator('.topic-page', {
    hasText: '2 pages are called “Runbook”',
  });
  await shown(wikis, 'the wiki title two wikis share saying which they are');
  await wikis.locator('.topic-pages-line').nth(0).click();
  await shown(page.locator(`.topic-page[data-topic="${first}"], .topic-page[data-topic="${second}"]`), 'one of them opened');
  await addressed(page, `/topic/${encodeURIComponent(await page.locator('.topic-page[data-topic]').getAttribute('data-topic'))}`);

  // ── Moved within its wiki, it keeps its address ─────────────────────────
  await page.goto(at('Handbook', 'Rolling back'));
  await opened(page, back, 'Rolling back', ['Handbook', 'Deploying'], 'the page three deep again');
  await cli('topic', 'move', back, '--under', handbook);
  await opened(page, back, 'Rolling back', ['Handbook'], 'the page left open saying where it is now, never read again');
  await addressed(page, '/topic/Handbook/Rolling%20back');
  const theirs = await signedIn('nell@example.com', at('Handbook', 'Rolling back'));
  await opened(theirs, back, 'Rolling back', ['Handbook'], 'the address it kept opening it on another page');
  await cli('topic', 'move', back, '--under', deploying);
  await opened(page, back, 'Rolling back', ['Handbook', 'Deploying'], 'moved back where it was');

  // ── A page that says nothing and holds pages ────────────────────────────
  await page.goto(at('Handbook', 'Deploying'));
  const writes = await opened(page, deploying, 'Deploying', ['Handbook'], 'the page that says nothing, for whoever may write it');
  expect('its post to write in', (await writes.locator('.topic-post .ProseMirror').count()) === 1);
  const reader = await signedIn('ada@example.com', at('Handbook', 'Deploying'), { reads: true });
  const reads = await opened(reader, deploying, 'Deploying', ['Handbook'], 'the page that says nothing, for whoever may only read it');
  const parts = await reads.locator('.topic-article-main').evaluate((one) => [...one.querySelectorAll(':scope > section')].map((part) => part.getAttribute('aria-label')).join(' | '));
  expect(`its path, its title and the pages in it and no post on Article, got ${parts}`, parts === 'Pages in it');
  await reads.getByRole('tab', { name: /^Talk/ }).click();
  await shown(reads.locator('.topic-talk .topic-replies-part'), 'its replies on Talk');
  expect('its title as words', (await reads.locator('h1.topic-page-title').count()) === 1);
  expect('no way to add a page', (await reads.getByRole('button', { name: '+ New page' }).count()) === 0);
  await reader.screenshot({ path: `${out}/wikipage-03-reader.png` });
  await reader.context().close();

  // ── Named on the page three deep, from the inbox ────────────────────────
  const rail = theirs.locator('.rail .rail-inbox');
  await rail.click();
  const line = theirs.locator('.inbox-line', {
    hasText: 'which tag do we roll back to',
  });
  await shown(line, "Nell's inbox line of the reply that named her");
  // Read as her page signed in, while the page was under Handbook alone: said where it is now, the box not read by hand.
  await shown(
    line.locator('.inbox-open[aria-label*="on Handbook › Deploying › Rolling back"]'),
    `the line to say the page by where it is now, got ${await line.locator('.inbox-open').getAttribute('aria-label')}`,
  );
  await line.locator('.inbox-open').click();
  const named = theirs.locator(`.topic-page[data-topic="${back}"] .topic-reply.asked[data-seq="${under}"]`);
  await shown(named, 'the page opened at the reply, marked');
  expect('the page opened on Talk', (await theirs.locator(`.topic-page[data-topic="${back}"] .topic-tab[aria-selected="true"]`).getAttribute('data-part')) === 'talk');
  await theirs.screenshot({ path: `${out}/wikipage-04-asked.png` });
  const where = await named.elementHandle();
  await theirs
    .waitForFunction(
      (one) => {
        const box = one.getBoundingClientRect();
        return box.top >= 0 && box.bottom <= window.innerHeight;
      },
      where,
      { timeout: 5000 },
    )
    .catch(async () => {
      throw new Error(`expected the reply in view, its box ${JSON.stringify(await named.boundingBox())}`);
    });

  // ── By the boxes ────────────────────────────────────────────────────────
  for (const width of [1280, 1000, 800]) {
    await page.setViewportSize({ width, height: 860 });
    await page.goto(at('Handbook', 'Rolling back'));
    await opened(page, back, 'Rolling back', ['Handbook', 'Deploying'], `the page at ${width} wide`);
    await laid(page, `at ${width} wide`);
  }
  await page.screenshot({ path: `${out}/wikipage-05-narrow.png` });
  const phone = await signedIn('nell@example.com', at('Handbook', 'Rolling back'), {
    how: {
      viewport: { width: 390, height: 844 },
      isMobile: true,
      hasTouch: true,
      deviceScaleFactor: 2,
    },
  });
  await opened(phone, back, 'Rolling back', ['Handbook', 'Deploying'], 'the page on a phone');
  await shown(
    phone.locator('.topic-post .topic-doc-read', {
      hasText: 'Roll back by the tag',
    }),
    'its post drawn on a phone',
  );
  await laid(phone, "at a phone's width");
  await phone.screenshot({ path: `${out}/wikipage-06-phone.png` });

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, hers]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
