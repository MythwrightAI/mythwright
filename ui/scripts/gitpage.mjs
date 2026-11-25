// A project's Git page: a project whose folder is a git repository listed
// with a Git line, beside its Board and right before its Repo, and one
// whose folder is no repository with none, nor one whose folder the
// factory made and made a repository so that work can branch it, whose
// page is opened from its own page; the page opened from there in tabs in
// its head, git's own first in their order (Changes, Commits,
// Branches, Remotes, each with how many where that is known) and then a
// tab for each part a made-up package that serves the repository's remote
// shows, as the package calls it, Changes showing and one panel drawn:
// the branch the folder is on and how it stands against the one it
// follows, what is changed and what of it is staged, the commits of what
// the folder is on and, chosen, of another branch, the branches its own
// and a remote's, and the remotes with no password written into an
// address; nothing of the package read until one of its tabs is shown,
// each part read as its tab is first shown and not again when it is shown
// again, one its program answers with a row a link that opens a tab of
// its own and one whose address is no page with none, and one its program
// refuses, not signed in, its tab saying so in the package's own sentence
// with what to do; a commit, and a file staged, made by somebody's own git
// and shown with the page never read again and no part read for it; Read
// again reading git and the part that shows and no other, which is read
// as it is next shown; the tab that showed showing again, with what it
// read, after another page was shown; the folder that was none made a
// repository, its Git line coming with the page never read again; and,
// each tab showing in turn, the tab whole in sight in its row, the row and
// Refresh inside the page, the row scrolling sideways in itself at a
// phone's width, and nothing of the page over anything else, past the
// window's sides or scrolling sideways at 1280, 900 and 640 wide and a
// phone's.
//
//   DAEMON=../target/debug/factory node scripts/gitpage.mjs [out-dir]
//
// The environment variables are those of walkthrough.mjs. Nothing here
// reaches the network: the package's program is this check's own Node,
// told what to print, and the remotes are addresses nothing asks.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { pageCrowded } from './crowded.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll } from './folded.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'gitpage-data');
const home = join(out, 'gitpage-home');
const shop = join(out, 'gitpage-shop');
const plain = join(out, 'gitpage-plain');
const manifest = join(out, 'gitpage-package.yaml');

/** What the made-up package's program prints for its pull requests: one row with a page to read it at, one whose address is none. */
const ROWS = [
  { number: 12, title: 'A cart that keeps what is in it', by: 'grace', labels: ['shop', 'cart'], url: 'http://127.0.0.1:9/pull/12' },
  { number: 13, title: 'Prices in pence', by: 'bo', labels: [], url: 'javascript:alert(1)' },
];

/** A package for a service there is not, which serves a repository whose remote is at example.org; its program is Node. */
const PACKAGE = `extension: sample
title: Sample Host
description: Not a real service.
program: ${JSON.stringify(process.execPath)}
actions:
  pulls:
    description: List the open pull requests.
    args: ["-e", ${JSON.stringify(`process.stdout.write(${JSON.stringify(JSON.stringify(ROWS))})`)}]
    answer: json
    out:
      rows: \${ answer }
    repeatable: true
  checks:
    description: List what was checked.
    args: ["-e", "console.error('sign in first'); process.exit(4)"]
    fails:
      - when: \${ code != 0 }
        says: "The program is not signed in on this device. Sign in, and read this again."
    out:
      rows: \${ answer }
    repeatable: true
repository:
  serves: \${ contains(remote.url, "example.org") }
  parts:
    - name: pulls
      title: Pull requests
      action: pulls
      columns:
        - { field: number, title: "#" }
        - { field: title, title: Title }
        - { field: by, title: By }
        - { field: labels, title: Labels }
      link: url
    - name: checks
      title: Checks
      action: checks
      columns:
        - { field: name, title: Check }
`;

await mkdir(out, { recursive: true });
for (const dir of [data, home, shop, plain]) await rm(dir, { recursive: true, force: true });
await mkdir(home, { recursive: true });
// A home of its own, so that nothing here reads or writes what a factory already on this machine keeps.
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'share') };
// Git as the check asks it: who commits is what the repository says, never whoever runs the check.
const gitEnv = Object.fromEntries(Object.entries(env).filter(([name]) => !/^GIT_(AUTHOR|COMMITTER)_/.test(name)));
const git = (dir, ...args) => run('git', ['-C', dir, ...args], { env: gitEnv });
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], {
  cwd: repo,
  env,
  stdio: ['ignore', 'pipe', 'inherit'],
});
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });

const expect = (what, ok) => {
  if (!ok) throw new Error(`expected ${what}`);
};

let failed = null;
let shot = null;
try {
  for (const dir of [shop, plain]) {
    await mkdir(dir, { recursive: true });
    await writeFile(join(dir, 'README.md'), '# A project\n');
  }
  await git(shop, 'init', '--quiet', '-b', 'main');
  await git(shop, 'config', 'user.name', 'Ada');
  await git(shop, 'config', 'user.email', 'ada@example.com');
  await git(shop, 'add', '-A');
  await git(shop, 'commit', '--quiet', '-m', 'The shop as it began');
  await git(shop, 'checkout', '--quiet', '-b', 'cart');
  await git(shop, 'commit', '--quiet', '--allow-empty', '-m', 'A cart that keeps what is in it');
  await git(shop, 'checkout', '--quiet', 'main');
  await git(shop, 'remote', 'add', 'origin', 'https://ada:hunter2@example.org/acme/shop.git');
  await git(shop, 'remote', 'add', 'mirror', 'git@example.net:acme/shop.git');
  // A remote's branch, as a fetch would have left it, with nothing fetched.
  await git(shop, 'update-ref', 'refs/remotes/origin/main', 'HEAD');
  await git(shop, 'branch', '--set-upstream-to=origin/main', 'main');
  await writeFile(join(shop, 'README.md'), '# A project, changed here\n');
  await writeFile(join(shop, 'prices.md'), 'In pence.\n');

  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  await cli('project', 'create', '--bare', 'Shop', '--path', shop);
  await cli('project', 'create', '--bare', 'Plain', '--path', plain);
  // One the factory makes a folder for, which it makes a repository so that pieces of work can branch it.
  await cli('project', 'create', '--bare', 'Yard');
  await writeFile(manifest, PACKAGE);
  await cli('extension', 'add', manifest);

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const problems = [];
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const page = await context.newPage();
  shot = () => page.screenshot({ path: `${out}/gitpage-failed.png` }).catch(() => {});
  page.on('pageerror', (err) => problems.push(err.message));
  let reads = 0;
  // A page loaded again, and not the address bar following the tab showing, which the browser calls a navigation too.
  page.on('domcontentloaded', () => (reads += 1));
  // How often the page asked the package's program, by part.
  const asked = { pulls: 0, checks: 0 };
  page.on('request', (request) => {
    const part = request.url().match(/\/git\/sample\/(pulls|checks)$/)?.[1];
    if (part) asked[part] += 1;
  });

  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();
  const nav = page.locator('.nav');
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await unfoldAll(page);
  const readsAfterSignIn = reads;

  // ---- listed under a project that is a repository, and under no other --------------------------
  const shopTree = nav.locator('[data-project="shop"]');
  // What a project lists, in the order it is drawn.
  const lines = (tree) => tree.locator('.tree').first().evaluate((under) => [...under.querySelectorAll('.nav-item .truncate')].map((one) => one.textContent.trim()));
  await shopTree.getByText('Git', { exact: true }).waitFor();
  const listed = await lines(shopTree);
  expect(`Git right before Repo, got ${listed}`, listed.indexOf('Git') >= 0 && listed.indexOf('Git') + 1 === listed.indexOf('Repo'));
  expect('no Git line under a project that is no repository', (await nav.locator('[data-project="plain"]').getByText('Git', { exact: true }).count()) === 0);
  // A project the factory made a folder for is not taken for software: it lists no Git line, as it lists no
  // Repo, and its Git page is opened from its own page.
  const yardTree = nav.locator('[data-project="yard"]');
  await yardTree.waitFor();
  expect('no Git line under a project whose folder the factory made a repository', (await yardTree.getByText('Git', { exact: true }).count()) === 0);
  await page.screenshot({ path: `${out}/gitpage-0-listed.png` });
  await nav.getByText('Yard', { exact: true }).click();
  await page.getByRole('heading', { name: 'Yard', level: 1 }).waitFor();
  await page.getByRole('button', { name: 'Its Git', exact: true }).click();
  await page.getByRole('heading', { name: 'Yard git', level: 1 }).waitFor();
  await page.locator('.git-page').getByText('On main', { exact: false }).first().waitFor();
  await page.screenshot({ path: `${out}/gitpage-0-own.png` });

  // ---- the page, in tabs ---------------------------------------------------------------------
  await shopTree.getByText('Git', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop git', level: 1 }).waitFor();
  const gitPage = page.locator('.git-page');
  await gitPage.getByText('On main, following origin/main: level with it', { exact: true }).waitFor();
  const tabRow = gitPage.getByRole('tablist', { name: 'What is shown of the repository' });
  const tab = (key) => tabRow.locator(`[data-git-tab="${key}"]`);
  const choose = async (key) => {
    await tab(key).click();
    await gitPage.locator(`[role="tabpanel"][data-git-tab="${key}"]`).waitFor();
  };
  const shownKey = () => tabRow.locator('[aria-selected="true"]').getAttribute('data-git-tab');
  // Git's own first, in their order, then each part the package shows, as the package calls it; Changes showing.
  await tab('checks').waitFor();
  const tabs = await tabRow.locator('[role="tab"]').evaluateAll((all) => all.map((one) => [one.dataset.gitTab, one.textContent.trim()]));
  expect(
    `the tabs in order, got ${JSON.stringify(tabs)}`,
    JSON.stringify(tabs) === JSON.stringify([['changes', 'Changes2'], ['commits', 'Commits'], ['branches', 'Branches3'], ['remotes', 'Remotes2'], ['pulls', 'Pull requests'], ['checks', 'Checks']]),
  );
  expect('Changes showing as the page opens', (await shownKey()) === 'changes');
  expect('one panel drawn, the tab that shows', (await gitPage.locator('[role="tabpanel"]').count()) === 1);
  const part = (name) => gitPage.locator(`[data-part="${name}"]`);
  await part('changes').getByText('README.md', { exact: true }).waitFor();
  await part('changes').locator('.git-line', { hasText: 'README.md' }).getByText('modified, not staged', { exact: true }).waitFor();
  await part('changes').locator('.git-line', { hasText: 'prices.md' }).getByText('new, not tracked', { exact: true }).waitFor();
  expect('nothing of the commits drawn while Changes shows', (await part('commits').count()) === 0);
  await page.screenshot({ path: `${out}/gitpage-1-changes.png` });

  await choose('commits');
  await part('commits').getByText('The shop as it began', { exact: true }).waitFor();
  expect('the commits of what the folder is on, and not of another branch', (await part('commits').getByText('A cart that keeps what is in it').count()) === 0);
  await choose('branches');
  for (const name of ['main', 'cart', 'origin/main']) await part('branches').locator(`[data-branch="${name}"]`).waitFor();
  await choose('remotes');
  await part('remotes').getByText('https://example.org/acme/shop.git', { exact: true }).waitFor();
  await part('remotes').getByText('git@example.net:acme/shop.git', { exact: true }).waitFor();
  expect('no password written into an address shown', !(await gitPage.innerText()).includes('hunter2'));
  expect(`no part of the package read while none of its tabs was shown, got ${JSON.stringify(asked)}`, asked.pulls === 0 && asked.checks === 0);

  // ---- what the package shows, each part read as its tab is first shown -----------------------
  await choose('pulls');
  const pulls = gitPage.locator('[data-package="sample"][data-part="pulls"]');
  await pulls.getByText('A cart that keeps what is in it', { exact: true }).waitFor();
  await pulls.getByText('Sample Host', { exact: true }).waitFor();
  await pulls.getByText('shop, cart', { exact: true }).waitFor();
  const open = pulls.getByRole('link', { name: 'Open' });
  expect(`one link, for the row whose address is a page, got ${await open.count()}`, (await open.count()) === 1);
  expect('the link where the row is read', (await open.getAttribute('href')) === 'http://127.0.0.1:9/pull/12');
  expect('opening a tab of its own with no hold on this one', (await open.getAttribute('target')) === '_blank' && (await open.getAttribute('rel')).includes('noopener'));
  expect(`Pull requests read once as its tab was shown, and Checks not, got ${JSON.stringify(asked)}`, asked.pulls === 1 && asked.checks === 0);
  await page.screenshot({ path: `${out}/gitpage-2-pulls.png` });
  // A part its program will not answer, not signed in: its tab says why and how to sign it in, and is not left out.
  await choose('checks');
  await gitPage.locator('[data-package="sample"][data-part="checks"]').getByText('The program is not signed in on this device. Sign in, and read this again.', { exact: true }).waitFor();
  expect(`each part read once, got ${JSON.stringify(asked)}`, asked.pulls === 1 && asked.checks === 1);
  await page.screenshot({ path: `${out}/gitpage-3-checks.png` });
  await choose('pulls');
  await pulls.getByText('A cart that keeps what is in it', { exact: true }).waitFor();
  expect(`a tab shown again showing what it read, read nothing again, got ${JSON.stringify(asked)}`, asked.pulls === 1 && asked.checks === 1);

  // ---- what somebody's own git does, shown with the page never read again ---------------------
  await choose('changes');
  await git(shop, 'add', 'README.md');
  await part('changes').locator('.git-line', { hasText: 'README.md' }).getByText('modified, staged', { exact: true }).waitFor();
  await git(shop, 'commit', '--quiet', '-m', 'Said what the shop is');
  await gitPage.getByText('On main, following origin/main: 1 ahead', { exact: true }).waitFor();
  await tab('changes').getByText('1', { exact: true }).waitFor();
  await choose('commits');
  await part('commits').locator('.git-line').first().getByText('Said what the shop is', { exact: true }).waitFor();
  expect(`the parts not read again for the repository changing, got ${JSON.stringify(asked)}`, asked.pulls === 1 && asked.checks === 1);
  await page.screenshot({ path: `${out}/gitpage-4-committed.png` });

  // ---- another branch's commits --------------------------------------------------------------
  await part('commits').getByLabel('Whose commits').selectOption('refs/heads/cart');
  await part('commits').getByText('A cart that keeps what is in it', { exact: true }).waitFor();
  await part('commits').getByLabel('Whose commits').selectOption('HEAD');
  await part('commits').locator('.git-line').first().getByText('Said what the shop is', { exact: true }).waitFor();

  // ---- Refresh: git, and the part that shows; every other part as it is next shown ---------
  await choose('pulls');
  const readAgain = page.waitForResponse((answer) => answer.url().endsWith('/git/sample/pulls'));
  await gitPage.getByRole('button', { name: 'Refresh', exact: true }).click();
  await readAgain;
  await pulls.getByText('A cart that keeps what is in it', { exact: true }).waitFor();
  expect(`the part that shows read again when asked, and no other, got ${JSON.stringify(asked)}`, asked.pulls === 2 && asked.checks === 1);
  await choose('checks');
  await gitPage.locator('[data-package="sample"][data-part="checks"]').getByText('The program is not signed in on this device. Sign in, and read this again.', { exact: true }).waitFor();
  expect(`a part Refresh forgot read as it is next shown, got ${JSON.stringify(asked)}`, asked.pulls === 2 && asked.checks === 2);

  // ---- which tab shows is kept by the page's tab ----------------------------------------------
  await shopTree.getByText('Repo', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop repo', level: 1 }).waitFor();
  await shopTree.getByText('Git', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop git', level: 1 }).waitFor();
  expect('the tab that showed showing again', (await shownKey()) === 'checks');
  await gitPage.locator('[data-package="sample"][data-part="checks"]').getByText('The program is not signed in on this device. Sign in, and read this again.', { exact: true }).waitFor();
  expect(`what it read shown again and nothing read for it, got ${JSON.stringify(asked)}`, asked.pulls === 2 && asked.checks === 2);

  // ---- its address says the tab that shows, and an address opens the page at a tab ------
  const bar = () => page.evaluate(() => location.pathname + location.hash);
  await page.waitForFunction(() => location.pathname + location.hash === '/shop/git#checks', null, { timeout: 5000 }).catch(() => {});
  expect(`the address to say the tab that shows, got ${await bar()}`, (await bar()) === '/shop/git#checks');
  await page.evaluate(() => (location.hash = '#pulls'));
  await gitPage.locator('[role="tabpanel"][data-git-tab="pulls"]').waitFor();
  expect('the address opening the page at the tab it names', (await shownKey()) === 'pulls');
  await choose('changes');
  await page.waitForFunction(() => location.pathname + location.hash === '/shop/git', null, { timeout: 5000 }).catch(() => {});
  expect(`the first tab said by no # at all, got ${await bar()}`, (await bar()) === '/shop/git');

  // ---- a folder made a repository ---------------------------------------------------------------
  await git(plain, 'init', '--quiet', '-b', 'main');
  await nav.locator('[data-project="plain"]').getByText('Git', { exact: true }).waitFor();

  // ---- by the boxes, each tab showing ---------------------------------------------------------
  const keys = tabs.map(([key]) => key);
  for (const [width, height] of [[1280, 860], [900, 800], [640, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    if (width === 390 && (await nav.isVisible())) await resources.click();
    for (const key of keys) {
      await choose(key);
      // The tab chosen is brought into sight once the page has drawn it chosen: that is waited for, and said where it never comes.
      const inSight = await page
        .waitForFunction(() => {
          const row = document.querySelector('.git-page [role="tablist"]');
          const [tab, frame] = [row.querySelector('[aria-selected="true"]').getBoundingClientRect(), row.getBoundingClientRect()];
          return tab.left >= frame.left - 1 && tab.right <= frame.right + 1;
        }, null, { timeout: 5000 })
        .then(() => true, () => false);
      expect(`the tab ${key} brought whole into sight in its row at ${width}`, inSight);
      // The row scrolls in itself: the tab chosen whole in sight in it, the row inside the page and Refresh whole at its end.
      const boxes = await gitPage.evaluate((page, key) => {
        const row = page.querySelector('[role="tablist"]');
        const chosen = row.querySelector(`[data-git-tab="${key}"]`).getBoundingClientRect();
        const again = [...page.querySelectorAll('.git-tabs > button')].find((one) => one.textContent === 'Refresh').getBoundingClientRect();
        const frame = row.getBoundingClientRect();
        const body = page.getBoundingClientRect();
        return { chosen: [chosen.left, chosen.right], row: [frame.left, frame.right], again: [again.left, again.right], body: [body.left, body.right], scrolls: row.scrollWidth > row.clientWidth };
      }, key);
      // To a pixel: a row is scrolled by whole pixels, and a tab may begin between two.
      const within = (inner, outer) => inner[0] >= outer[0] - 1 && inner[1] <= outer[1] + 1;
      expect(`the tab ${key} whole in sight in its row at ${width}: ${JSON.stringify(boxes)}`, within(boxes.chosen, boxes.row));
      expect(`the row and Refresh inside the page at ${width}: ${JSON.stringify(boxes)}`, within(boxes.row, boxes.body) && within(boxes.again, boxes.body) && boxes.again[0] >= boxes.row[1] - 0.5);
      if (width === 390) expect(`the tabs scrolling sideways in their row at a phone's width: ${JSON.stringify(boxes)}`, boxes.scrolls);
      const wrong = await pageCrowded(page, '.git-page');
      expect(`nothing of the page over anything else, past the window or scrolling sideways at ${width} with ${key} showing: ${wrong.join('; ')}`, wrong.length === 0);
    }
    await choose('pulls');
    await page.screenshot({ path: `${out}/gitpage-5-${width}.png`, fullPage: true });
  }
  expect(`the page never read again, read ${reads - readsAfterSignIn} more times`, reads === readsAfterSignIn);

  await browser.close();
  if (problems.length) throw new Error(`errors in the page:\n${problems.join('\n')}`);
} catch (err) {
  failed = err;
  await shot?.();
} finally {
  daemon.kill('SIGTERM');
  await new Promise((gone) => daemon.on('exit', gone));
  for (const dir of [data, home, shop, plain]) await rm(dir, { recursive: true, force: true });
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
console.log(`pictures are in ${out}`);
