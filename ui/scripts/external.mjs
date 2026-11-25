// An agent the factory does not start: invited from the Agents tab, by
// the one button and dialog that add or invite one, chosen as from
// outside by the keyboard on the cards at its head; joined
// from a terminal with a home of its own, listed as external, given a
// card from the page and taking one itself (the card's face saying it is
// assigned to it, and that it is external), what it may ask changed
// beside its channel and held to at its next command, stopped; and
// somebody who may not add agents, who is offered no inviting.
//
// And an agent this factory starts, added from the same page by the
// same dialog, which begins on it, with what a channel adds one with: asked which project of the workspace
// showing it works in, beginning on the one the page is narrowed to,
// and under which environment where that project has some, closed with
// why until one is chosen and for a name an agent has; listed in its
// band with the page not read again; what the daemon refuses said in
// the dialog, which stays; by its boxes at three widths and a phone's;
// and not offered to somebody who may not add agents; somebody who may
// invite one and not apply its text offered the dialog with no choice in
// it and inviting alone, and refused adding one by the daemon.
//
// And from a channel kept to its members, where Add an agent asks first
// which it is: one from outside that is there, offered by its name as
// external and added with nothing else asked; and one invited there,
// with what the Agents tab asks, the daemon's refusal said in the
// dialog, which stays, brought into the channel at once, its invitation
// and command shown once, listed among the members as invited, joined by
// the built program from a home of its own and said to have joined with
// the page not read again; each dialog by its boxes at three widths and
// a phone's; and neither offered to somebody who may not add agents.
//
//   DAEMON=../target/debug/factory node scripts/external.mjs [out-dir]
//
// The agent's commands are the built program's, run with a home under
// the pictures' folder that is not the home the owner's commands have:
// what it keeps of its credential is there, and nothing of anybody's is
// read or written anywhere else. The environment variables are those of
// walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, stat } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { agentsPage } from './apart.mjs';
import { drawerAtRest, toolsAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { unfoldAll, unfoldChannels } from './folded.mjs';
import { addLine, addingDialog, openAdding as addingFromItsLine } from './adding.mjs';
import { addMember, agentsSaid, offeredUnder } from './addmember.mjs';
import { saysWhere } from './where.ts';

const run = promisify(execFile);
const out = resolve(process.argv[2] ?? 'shots');
const binary = resolve(process.env.DAEMON ?? '../target/debug/factory');
const repo = resolve('..');
const data = join(out, 'external-data');
const home = join(out, 'external-home');
// Where whoever is that agent runs its commands: another home, as on another machine.
const outside = join(out, 'external-outside');

await mkdir(out, { recursive: true });
for (const dir of [data, home, outside]) await rm(dir, { recursive: true, force: true });
for (const dir of [home, outside]) await mkdir(dir, { recursive: true });
// The daemon and each command line are given a home of their own, so that
// nothing here reads or writes what a factory already on this machine keeps.
const homed = (dir) => ({ ...process.env, HOME: dir, XDG_CONFIG_HOME: join(dir, 'config'), XDG_DATA_HOME: join(dir, 'share') });
const [env, theirs] = [homed(home), homed(outside)];
const daemon = spawn(binary, ['serve', '--data', data, '--listen', '127.0.0.1:0', '--adapter', 'builtin'], { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] });
endsWithThis(daemon);
const url = await saysWhere(daemon);
const cli = (...args) => run(binary, [...args, '--url', url], { cwd: repo, env });
// What the product and its command are called, as the daemon says them: nothing here spells either.
const meta = await (await fetch(`${url}/api/meta`)).json();
/** A command asked as the agent, from its own home: what it printed, or why it was refused. */
const asWalls = (...args) =>
  run(binary, ['--as', 'walls', ...args, '--url', url], { cwd: repo, env: theirs }).then(
    (ran) => ({ ok: true, said: ran.stdout }),
    (refused) => ({ ok: false, said: String(refused.stderr ?? refused.message) }),
  );
/** A command of the agent's home handed `said` on standard input. */
const handed = (said, ...args) =>
  new Promise((done) => {
    const child = spawn(binary, [...args, '--url', url], { cwd: repo, env: theirs, stdio: ['pipe', 'pipe', 'pipe'] });
    let printed = '';
    for (const stream of [child.stdout, child.stderr]) stream.on('data', (chunk) => (printed += chunk));
    child.on('close', (code) => done({ ok: code === 0, said: printed }));
    child.stdin.end(`${said}\n`);
  });

let failed = null;
try {
  await cli('register', '--email', 'ada@example.com', '--name', 'Ada', '--password', 'correct horse');
  // A plan with more than one person: a second person, whom a card is given to by her name and who is
  // added to a channel from its dialog (`addmember.mjs`), and somebody invited to register from there.
  await cli('tier', 'premium');
  const code = JSON.parse((await cli('invite', '--json')).stdout).code;
  const joinedToo = await fetch(`${url}/api/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
  });
  if (!joinedToo.ok) throw new Error(`expected a second person to have joined, got ${joinedToo.status}`);
  await cli('project', 'create', 'Shop', '--bare');
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('channel', 'create', 'yard', '--project', 'shop');
  // An agent this factory starts, beside which the other is listed.
  await cli('agent', 'add', 'helper', '--project', 'shop', '--channel', 'yard');
  const first = JSON.parse((await cli('board', 'add', 'shop', 'Hang the gate', '--json')).stdout).id;
  const second = JSON.parse((await cli('board', 'add', 'shop', 'Oil the latch', '--json')).stdout).id;

  const cache = `${homedir()}/.cache/ms-playwright`;
  const shells = (await readdir(cache)).filter((entry) => entry.startsWith('chromium_headless_shell')).sort();
  const browser = await chromium.launch({
    executablePath: process.env.BROWSER ?? `${cache}/${shells.at(-1)}/chrome-headless-shell-linux64/chrome-headless-shell`,
    args: (process.env.BROWSER_FLAGS ?? '').split(' ').filter(Boolean),
    env: process.env.BROWSER_LIBS ? { ...process.env, LD_LIBRARY_PATH: process.env.BROWSER_LIBS } : process.env,
  });
  const scheme = process.env.SCHEME ?? 'dark';
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const problems = [];
  const expect = (what, ok) => {
    if (!ok) throw new Error(`expected ${what}`);
  };
  const until = async (what, ask) => {
    let last;
    for (let tries = 0; tries < 300; tries += 1) {
      last = await ask();
      if (last === true) return;
      await new Promise((later) => setTimeout(later, 100));
    }
    throw new Error(`expected ${what}, and it is ${JSON.stringify(last)}`);
  };
  const signIn = async (on) => {
    on.setDefaultTimeout(20_000);
    on.on('pageerror', (err) => problems.push(err.message));
    await on.goto(url);
    await on.getByLabel('Email').fill('ada@example.com');
    await on.getByLabel('Password').fill('correct horse');
    await on.getByRole('button', { name: 'Sign in', exact: true }).last().click();
    await on.locator('.rail').waitFor();
  };
  // The Members page's Agents tab, from the Members line under Factory, of every workspace (`apart.mjs`).
  const agentsOf = (on) => agentsPage(on, out);
  const line = (on, name) => on.locator(`.agent-line[data-agent="${name}"]`);
  const doing = async (on, name) => (await line(on, name).locator('.agent-doing').innerText()).trim();
  const runsOn = async (on, name) => (await line(on, name).locator('.agent-runs').innerText()).trim();
  /**
   * Nothing of a dialog outside the window or the dialog, nothing of it
   * over anything else of it, and nothing that scrolls sideways: by the
   * boxes of each thing it asks, each thing it says and what it is left by.
   */
  const sound = async (on, what) => {
    const wrong = await on.evaluate(() => {
      const said = [];
      const dialog = document.querySelector('.modal');
      if (!dialog) return ['there is no dialog'];
      const frame = dialog.getBoundingClientRect();
      if (frame.left < -0.5 || frame.top < -0.5 || frame.right > window.innerWidth + 0.5 || frame.bottom > window.innerHeight + 0.5) said.push(`the dialog is outside the window: ${JSON.stringify(frame)}`);
      if (document.documentElement.scrollWidth > window.innerWidth) said.push('the page scrolls sideways');
      if (dialog.scrollWidth > dialog.clientWidth + 1) said.push('the dialog scrolls sideways');
      // What is at its foot stays there over what scrolls under it: it is held to the dialog, and the rest to each other.
      const parts = [...dialog.querySelectorAll('h2, p, .field, .invite-shown, .error-text, .add-agent-way, .add-person-line, .member-setting .holds-table th')];
      const boxes = parts.map((part) => [part.className || part.tagName, part.getBoundingClientRect()]);
      for (const [name, box] of boxes) {
        if (box.width === 0 || box.height === 0) said.push(`${name} has no size`);
        if (box.left < frame.left - 0.5 || box.right > frame.right + 0.5) said.push(`${name} is past the dialog's side`);
      }
      for (let one = 0; one < boxes.length; one += 1)
        for (let other = one + 1; other < boxes.length; other += 1) {
          const [a, b] = [boxes[one][1], boxes[other][1]];
          if (a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5) said.push(`${boxes[one][0]} lies over ${boxes[other][0]}`);
        }
      for (const inner of dialog.querySelectorAll('input, select, button, code, th, td')) {
        const box = inner.getBoundingClientRect();
        if (box.left < frame.left - 0.5 || box.right > frame.right + 0.5) said.push(`${inner.tagName} "${(inner.textContent || inner.value || '').slice(0, 20)}" is past the dialog's side`);
      }
      for (const button of dialog.querySelectorAll('.modal-foot button')) {
        const box = button.getBoundingClientRect();
        if (box.bottom > window.innerHeight + 0.5 || box.top < 0) said.push(`${button.textContent.trim()} is not in the window`);
      }
      return said;
    });
    expect(`nothing of ${what} outside it, over anything else of it or scrolling sideways, got ${wrong.join('; ')}`, wrong.length === 0);
  };
  /** The same at three widths and a phone's, a picture of each, and back to the first. */
  const WIDTHS = [
    [1280, 860, 'wide'],
    [900, 700, 'middling'],
    [640, 720, 'narrow'],
    [390, 780, 'phone'],
  ];
  const atWidths = async (on, what, shot) => {
    for (const [width, height, called] of WIDTHS) {
      await on.setViewportSize({ width, height });
      await sound(on, `${what} at ${width} wide`);
      await on.screenshot({ path: `${out}/external-${shot}-${called}.png` });
    }
    await on.setViewportSize({ width: WIDTHS[0][0], height: WIDTHS[0][1] });
  };

  const page = await context.newPage();
  await signIn(page);
  await agentsOf(page);
  expect('an agent this factory starts to be listed as it always was', (await doing(page, 'helper')) === 'idle');

  // ---- invited from the dialog -------------------------------------------------------------
  // One button for both, opening one dialog that asks first which it is, beginning on one this factory runs, with the caret in its name.
  expect('no button that only adds or only invites', (await page.getByRole('button', { name: 'Invite an agent', exact: true }).count()) === 0 && (await page.getByRole('button', { name: 'Add an agent', exact: true }).count()) === 0);
  const inviting = page.getByRole('button', { name: 'Add or invite an agent', exact: true });
  await inviting.click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('heading', { name: 'Add or invite an agent' }).waitFor();
  const ways = dialog.getByRole('radiogroup', { name: 'Which agent it is' });
  const runsHere = ways.getByRole('radio', { name: /^Run by this factory/ });
  const fromOutside = ways.getByRole('radio', { name: /^From outside/ });
  expect('the two ways to be offered, beginning on one this factory runs', (await ways.getByRole('radio').count()) === 2 && (await runsHere.isChecked()));
  expect('the caret to be in the name of one this factory runs', await dialog.locator('form.add-agent').getByLabel('Name').evaluate((box) => document.activeElement === box));
  // Chosen by the keyboard on the cards, the keyboard stays there, and what is asked is what an invitation is made with.
  await runsHere.focus();
  await page.keyboard.press('ArrowDown');
  await dialog.locator('form.invite-agent').waitFor();
  expect('from outside to be chosen, with the keyboard still on the cards', (await fromOutside.isChecked()) && (await fromOutside.evaluate((box) => document.activeElement === box)));
  expect('nothing of adding one this factory runs to be asked any more', (await dialog.locator('form.add-agent').count()) === 0);
  expect(`what an agent from outside is to be said under the cards, got "${await dialog.locator('.add-agent-said').innerText()}"`, (await dialog.locator('.add-agent-said').innerText()).startsWith('For a program somebody runs for themselves'));
  expect('nothing to invite with until it has a name', await dialog.getByRole('button', { name: 'Invite', exact: true }).isDisabled());
  await dialog.getByLabel("The agent's name").fill('Walls');
  expect(`what it will be called to be said where that is not what was typed, got "${await dialog.locator('.invite-called').innerText()}"`, (await dialog.locator('.invite-called').innerText()) === 'It will be called walls.');
  await dialog.getByLabel('The project it is listed with').selectOption({ label: 'Shop' });
  // What it may ask, by area, as the Tools panel draws it: what an agent usually holds, and a line of it changed.
  const table = dialog.locator('.holds-table');
  const lines = async (within) =>
    Object.fromEntries(
      await within.locator('tr').evaluateAll((rows) =>
        rows.map((row) => {
          const choice = row.querySelector('select');
          return [row.querySelector('th').textContent.trim(), choice ? choice.options[choice.selectedIndex].textContent.trim() : row.querySelector('td').textContent.trim()];
        }),
      ),
    );
  const usual = await lines(table);
  expect(`a new one to be offered what an agent usually holds, got ${JSON.stringify(usual)}`, usual.board === 'read and change' && usual.blueprints === 'read' && usual.agents === 'read' && usual.shell === 'nothing' && usual.admin === 'nothing');
  expect(`a new one to be offered its own inbox with that, got ${JSON.stringify(usual)}`, usual.inbox === 'read and change');
  await table.getByLabel('What it may do with channels').selectOption({ label: 'read' });
  expect('a line changed to be what is asked', (await lines(table)).channels === 'read');
  await dialog.getByLabel('How long the invitation works').selectOption({ label: '12 hours' });
  await atWidths(page, 'the dialog that invites an agent', '1-invite');
  await dialog.getByRole('button', { name: 'Invite', exact: true }).click();

  // ---- the invitation and the command, shown once ------------------------------------------
  await dialog.getByRole('heading', { name: 'walls is invited' }).waitFor();
  const invitation = (await dialog.locator('.invite-code').innerText()).trim();
  const command = (await dialog.locator('.invite-join').innerText()).trim();
  expect(`an invitation to be shown, got "${invitation}"`, /^enrol_[a-z0-9]{40,}$/.test(invitation));
  expect(`the command that joins to say where this factory is, got "${command}"`, command === `${meta.cli} agent join --url ${url}`);
  const shown = await dialog.innerText();
  expect(`it to say that it lets the agent in once, within 12 hours, and is not shown again, got "${shown}"`, shown.includes('once, within the next 12 hours') && shown.includes('It will not be shown again'));
  expect('a button to copy each', (await dialog.getByRole('button', { name: 'Copy', exact: true }).count()) === 2);
  await atWidths(page, 'the dialog that shows an invitation', '2-invited');
  // It is listed already, behind the dialog, as external and invited.
  await line(page, 'walls').waitFor();
  await dialog.getByRole('button', { name: 'Done', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  expect(`it to be listed with its project, got ${await line(page, 'walls').evaluate((row) => row.closest('.agents-of').dataset.of)}`, (await line(page, 'walls').evaluate((row) => row.closest('.agents-of').dataset.of)) === 'shop');
  expect(`it to say external where another says what it runs on, got "${await runsOn(page, 'walls')}"`, (await runsOn(page, 'walls')) === 'external');
  expect(`it to say that it is invited and has yet to join, got "${await doing(page, 'walls')}"`, (await doing(page, 'walls')) === 'invited · has yet to join');
  expect(`its permissions to be what was asked, got "${await line(page, 'walls').locator('.agent-may').innerText()}"`, /channels/.test(await line(page, 'walls').locator('.agent-may').getAttribute('aria-label')));
  // Opened again, the dialog asks afresh, beginning where it always does: the invitation is nowhere to be shown a second time.
  await inviting.click();
  await dialog.getByRole('heading', { name: 'Add or invite an agent' }).waitFor();
  expect('it to begin again on one this factory runs', await runsHere.isChecked());
  await fromOutside.click();
  await dialog.locator('form.invite-agent').waitFor();
  expect('nothing of the invitation to be on the page once it was put away', !(await page.content()).includes(invitation));
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  await line(page, 'walls').scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/external-3-listed.png` });

  // ---- joined from a terminal, in a home of its own ----------------------------------------
  const nobody = await asWalls('whoami');
  expect(`a command asked as it before it joined to say that nothing is kept for it, got "${nobody.said}"`, !nobody.ok && nobody.said.includes('no credential is kept on this machine for an agent called `walls`'));
  const joined = await handed(invitation, 'agent', 'join');
  expect(`it to join with the invitation on standard input, got "${joined.said}"`, joined.ok && joined.said.startsWith(`joined ${url} as the agent \`walls\``));
  expect('neither the invitation nor a credential to be printed', !joined.said.includes(invitation) && !joined.said.includes('agt_'));
  const kept = join(outside, 'config', meta.id, 'agents', 'walls');
  const files = await readdir(kept);
  expect(`its credential to be kept in one file of its own home, got ${files}`, files.length === 1);
  expect('the file to be its owner\'s alone', ((await stat(join(kept, files[0]))).mode & 0o777) === 0o600);
  expect('what is kept to be its credential', (await readFile(join(kept, files[0]), 'utf8')).includes('"token":"agt_'));
  expect('nothing of it to be kept in the home of whoever invited it', (await readdir(join(home, 'config', meta.id)).catch(() => [])).every((name) => name !== 'agents'));
  const who = await asWalls('whoami');
  expect(`a command asked as it to be the agent's, got "${who.said}"`, who.ok && who.said.startsWith('walls (agent:walls)'));
  // The page says so with nothing read again.
  await until('the list to say that it has joined', async () => (await doing(page, 'walls')) === 'joined' || (await doing(page, 'walls')));
  const again = await handed(invitation, 'agent', 'join');
  expect(`an invitation that was used to let nobody in, got "${again.said}"`, !again.ok && again.said.includes('that invitation lets no agent in'));

  // ---- a card given to it from the page, and one it takes itself ---------------------------
  await page.getByRole('button', { name: 'Resources', exact: true }).click().catch(() => {});
  await unfoldAll(page);
  await page.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }).click();
  const column = (name) => page.locator('.column', { has: page.locator('.column-head', { hasText: name }) });
  await column('backlog').locator('.card', { hasText: 'Hang the gate' }).click();
  const drawer = drawerAtRest(page);
  await drawer.waitFor();
  const giving = drawer.getByLabel('Assigned to');
  expect(`it to be offered among the agents a card is given to, got ${await giving.locator('option').allInnerTexts()}`, (await giving.locator('option').allInnerTexts()).includes('walls (external)'));
  // A card is offered the agents of its own workspace and the factory's own, never those of another workspace's project:
  // one is made there from a terminal, and one that is no project's after it, so that the second being offered says the
  // page has heard of both.
  await cli('workspace', 'create', 'Annex');
  await cli('project', 'create', 'Quarry', '--bare', '--workspace', 'annex');
  await cli('agent', 'add', 'quarry-hand', '--project', 'quarry');
  await cli('agent', 'add', 'rover');
  await until('an agent that is no project\'s to be offered', async () => (await giving.locator('option').allInnerTexts()).includes('rover (agent)') || 'not yet');
  const givenTo = await giving.locator('option').allInnerTexts();
  expect(`no agent of another workspace's project to be offered, got ${givenTo}`, !givenTo.includes('quarry-hand (agent)'));
  expect(`the agents of its own project to be offered first, got ${givenTo}`, givenTo.indexOf('helper (agent)') < givenTo.indexOf('rover (agent)') && givenTo.includes('walls (external)'));
  await giving.selectOption({ label: 'walls (external)' });
  await drawer.getByRole('button', { name: 'Save', exact: true }).click();
  await until('the card to be given to it', async () => {
    const board = JSON.parse((await cli('board', 'shop', '--json')).stdout);
    const card = board.items.find((one) => one.id === first);
    return card.assignee === 'agent:walls' || card.assignee;
  });
  // The other it takes itself, moves and speaks on, with its own credential.
  const took = await asWalls('board', 'give', second, 'walls');
  expect(`it to give a card to itself, got "${took.said}"`, took.ok && took.said.trim() === `${second} is given to agent:walls`);
  expect('it to move the card', (await asWalls('board', 'move', second, 'doing')).ok);
  expect('it to say something on the card', (await asWalls('record', 'comment', second, 'The', 'hinges', 'are', 'on.')).ok);
  await column('doing').locator('.card', { hasText: 'Oil the latch' }).waitFor();
  // Its face says whom it is given to as a property, with what kind of somebody that is (`assignedTo`), each with nothing read again.
  const assigned = (title) => page.locator('.column .card', { hasText: title }).locator('.card-assigned');
  const cardSays = async (title, wanted) => until(`the card ${title} to say "${wanted}"`, async () => (await assigned(title).innerText().catch(() => '')).replace(/\s+/g, ' ').trim() === wanted || (await assigned(title).innerText().catch(() => '')));
  await cardSays('Oil the latch', 'Assigned to walls (external)');
  await cli('board', 'give', first, 'helper');
  await cardSays('Hang the gate', 'Assigned to helper (agent)');
  const owner = (await cli('whoami')).stdout.match(/\((user:[^)]+)\)/)?.[1];
  expect('whoever signed in to say who they are', Boolean(owner));
  await cli('board', 'give', first, owner);
  await cardSays('Hang the gate', 'Assigned to Ada (you)');
  const people = await page.evaluate(async () => (await (await fetch('/api/people', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).people);
  const grace = people.find((one) => one.name === 'Grace')?.actor;
  expect(`Grace to be among the people, got ${JSON.stringify(people)}`, Boolean(grace));
  await cli('board', 'give', first, grace);
  await cardSays('Hang the gate', 'Assigned to Grace (person)');
  // A person nothing names is somebody, and never the id of an account.
  await cli('board', 'give', first, 'user:acct_other');
  await cardSays('Hang the gate', 'Assigned to someone (person)');
  // The line is inside its card, on two lines at most with the name whole, and no card is wider than its column, at every width and a phone's.
  for (const [width, height, called] of [[1280, 860, 'wide'], [900, 700, 'middling'], [390, 780, 'phone']]) {
    await page.setViewportSize({ width, height });
    const wrong = await page.locator('.column .card').evaluateAll((cards) =>
      cards.flatMap((card) => {
        const [box, column] = [card.getBoundingClientRect(), card.closest('.column').getBoundingClientRect()];
        const said = [];
        if (box.left < column.left - 0.5 || box.right > column.right + 0.5) said.push(`${card.dataset.card} is wider than its column`);
        const line = card.querySelector('.card-assigned');
        if (line) {
          const at = line.getBoundingClientRect();
          if (at.left < box.left - 0.5 || at.right > box.right + 0.5) said.push(`${card.dataset.card}'s assignee leaves the card`);
          if (at.height > parseFloat(getComputedStyle(line).lineHeight) * 2.5) said.push(`${card.dataset.card}'s assignee is ${at.height} tall`);
          for (const part of line.children) for (const piece of part.getClientRects()) if (piece.left < box.left - 0.5 || piece.right > box.right + 0.5) said.push(`${card.dataset.card}'s "${part.textContent}" leaves the card`);
        }
        return said;
      }),
    );
    expect(`every card's assignee inside it at ${width} wide, and ${wrong.join('; ')}`, wrong.length === 0);
    await page.screenshot({ path: `${out}/external-4-assigned-${called}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await cli('board', 'give', first, 'walls');
  await cardSays('Hang the gate', 'Assigned to walls (external)');
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });
  await column('doing').locator('.card', { hasText: 'Oil the latch' }).click();
  await drawer.waitFor();
  expect(`its drawer to say it is given to the agent, got "${await drawer.getByLabel('Assigned to').inputValue()}"`, (await drawer.getByLabel('Assigned to').inputValue()) === 'agent:walls');
  // What it said is under Comments and its move under History, tabs beside Details, which shows first (`tagged.mjs`).
  await drawer.getByRole('tab', { name: /^Comments/ }).click();
  await drawer.getByText('The hinges are on.').first().waitFor();
  const spoken = await drawer.innerText();
  await drawer.getByRole('tab', { name: /^History/ }).click();
  await drawer.getByText('walls moved it from backlog to doing').first().waitFor();
  const history = await drawer.innerText();
  expect(`its comments to say what it said and its history that the agent moved it, got "${spoken}" and "${history}"`, spoken.includes('The hinges are on.') && history.includes('walls moved it from backlog to doing'));
  await page.screenshot({ path: `${out}/external-4-card.png` });
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });

  // The Agents tab says it is at work on the card it has in hand, and not on the one nobody has begun.
  await agentsOf(page);
  await until('the list to say which card it is working on', async () => (await doing(page, 'walls')) === `working on ${second}` || (await doing(page, 'walls')));
  expect('the agent this factory starts to be idle still', (await doing(page, 'helper')) === 'idle');
  await line(page, 'walls').scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/external-5-working.png` });
  // Its history has no turns, and says what it did.
  await line(page, 'walls').getByRole('button', { name: 'More that can be done with walls', exact: true }).click();
  await page.getByRole('menu').getByRole('menuitem', { name: 'History', exact: true }).click();
  await drawer.waitFor();
  await drawer.locator('.external-did li').first().waitFor();
  const did = await drawer.locator('.external-did li').evaluateAll((rows) => rows.map((row) => row.lastElementChild.textContent));
  expect(`its history to say what it did, the latest first, got ${JSON.stringify(did)}`, did[0] === `said on ${second}: The hinges are on.` && did.includes(`moved ${second} from backlog to doing`) && did.includes(`took ${second}`) && !did.some((one) => one.includes(first)));
  expect('it to say that it has no turns', (await drawer.locator('.turns-of').innerText()).includes('so it has no turns to list'));
  const beside = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(`nothing to scroll sideways with its history open, and the page is ${beside} wider than the window`, beside <= 0);
  await page.screenshot({ path: `${out}/external-6-history.png` });
  await page.keyboard.press('Escape');
  await page.locator('.drawer').waitFor({ state: 'detached' });

  // ---- what it may ask, changed beside its channel -----------------------------------------
  await cli('agent', 'add', 'walls', '--channel', 'yard');
  await page.getByRole('button', { name: 'Channels', exact: true }).click();
  // Every branch of the channels starts folded (`folded.mjs`): the project's, which holds `yard`, is unfolded as a person would.
  await unfoldChannels(page);
  await page.locator('.nav').getByText('yard', { exact: true }).click();
  await page.getByRole('heading', { name: '# yard' }).waitFor();
  const panel = page.getByRole('complementary', { name: 'Tools' });
  if ((await panel.count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  await panel.waitFor();
  await panel.getByRole('tab', { name: /^Channel\b/ }).click().catch(() => {});
  const row = panel.locator('.context-agent', { hasText: 'walls' });
  await row.waitFor();
  expect(`its line there to say that it is external and has joined, and nothing of a program, got "${await row.innerText()}"`, /agent · external/.test(await row.innerText()) && /joined/.test(await row.innerText()) && !/may (read|edit|full)/.test(await row.innerText()));
  await row.click();
  const about = panel.locator('.external-about');
  await about.waitFor();
  const facts = await about.locator('.context-facts').innerText();
  expect(`it to be said to run outside, with nothing of a wall or a model, got "${facts}"`, facts.includes('this factory starts no program for it') && !/wall|model|May do/i.test(facts));
  expect(`the cards given to it to be listed, got "${await about.locator('.external-cards').innerText()}"`, (await about.locator('.external-cards').innerText()).includes(first) && (await about.locator('.external-cards').innerText()).includes(second));
  const holds = about.locator('.holds-table');
  await until('a choice for each area of what it holds', async () => (await holds.locator('select').count()) > 5 || (await holds.locator('select').count()));
  expect(`what it holds to be what it was invited with, got ${JSON.stringify(await lines(holds))}`, (await lines(holds)).channels === 'read' && (await lines(holds)).board === 'read and change');
  await page.screenshot({ path: `${out}/external-7-tools.png` });
  // The board from "read and change" to "read": the credential it keeps is held to that at its next command.
  await holds.getByLabel('What it may do with board').selectOption({ label: 'read' });
  await until('its next move to be refused', async () => {
    const moved = await asWalls('board', 'move', second, 'review');
    return (!moved.ok && /board:write/.test(moved.said)) || moved.said;
  });
  expect('it still to read the board', (await asWalls('board', 'shop')).ok);
  // And back: allowed again, with the credential it had.
  await holds.getByLabel('What it may do with board').selectOption({ label: 'read and change' });
  await until('its next move to be let through', async () => {
    const moved = await asWalls('board', 'move', second, 'review');
    return moved.ok || moved.said;
  });
  expect('the credential it keeps not to have been replaced', (await readdir(kept)).join() === files.join());
  // Where those who can be asked are listed it is said to read what is said for itself, and not to be idle as one that is asked is.
  const asking = page.locator('.channel > .split .asking-button');
  await asking.click();
  const askable = page.getByRole('menu', { name: 'Agents and bots in this channel' });
  await askable.waitFor();
  const those = await askable.locator('.asked-words').evaluateAll((all) => all.map((one) => `${one.querySelector('.asked-name').textContent} / ${one.querySelector('.asked-under').textContent}`));
  expect(`it to be said to read what is said, and the other to be idle, got ${those.join(' | ')}`, those.includes('walls / agent · external · reads what is said for itself') && those.some((one) => one.startsWith('helper / idle')));
  await asking.click();
  await askable.waitFor({ state: 'detached' });
  // Let say things there, it says them as itself; and the card over its name says what it is, with nothing of a program.
  await holds.getByLabel('What it may do with channels').selectOption({ label: 'read and change' });
  await until('it to be let say something in the channel', async () => {
    const posted = await asWalls('post', 'yard', 'The gate is hung.');
    return posted.ok || posted.said;
  });
  const spoke = page.locator('.channel > .split .message .who.agent', { hasText: 'walls' }).first();
  await spoke.waitFor();
  // The card is over its picture beside what it said, and not over its name.
  const card = page.locator('.who-card');
  await spoke.hover();
  await page.waitForTimeout(400);
  expect('no card from pointing at its name', (await card.count()) === 0);
  await page.locator('.channel > .split .message', { has: page.locator('.who.agent', { hasText: 'walls' }) }).first().locator('.avatar').hover();
  await card.waitFor();
  const says = await card.innerText();
  expect(`the card over its name to say that it is external and has joined, and nothing of a program, got "${says}"`, /agent · external/.test(says) && says.includes('joined') && says.includes('this factory starts no program for it') && !/Sees|May do|model|idle/i.test(says));
  await page.screenshot({ path: `${out}/external-7-card-over-its-name.png` });
  await page.mouse.move(10, 400);
  await card.waitFor({ state: 'detached' });
  // A message that names it starts nothing and is answered by nobody: the agent beside it answers what names that one.
  await cli('post', 'yard', '@walls and the latch?');
  await cli('post', 'yard', '@helper and you?');
  await until('the agent this factory starts to have answered', async () => /done|failed/.test((await cli('agent', 'turns', 'helper')).stdout) || 'not yet');
  const said = JSON.parse((await cli('read', 'yard', '--json')).stdout);
  const messages = said.messages ?? said;
  const named = messages.find((one) => one.body === '@walls and the latch?');
  expect('nothing to have answered what named it', !messages.some((one) => one.reply_to === named.id) && !messages.some((one) => /was not asked/.test(one.body)));
  expect('nothing to have been asked of it', (await cli('agent', 'turns', 'walls')).stdout.trim() === '');

  // ---- added from a channel ------------------------------------------------------------------
  // The channel's Add an agent asks first which it is: one this factory runs, or one from outside, which is invited
  // there with what the Agents tab's dialog asks, brought into the channel at once and shown its invitation once.
  // A channel kept to its members: one from outside is in it as a person is.
  await cli('channel', 'create', 'porch', '--project', 'shop', '--kept');
  await unfoldChannels(page);
  await page.locator('.nav').getByText('porch', { exact: true }).click();
  await page.locator('.channel .view-head h1', { hasText: 'porch' }).waitFor();
  // The one dialog the line at the foot of who is in the channel opens, on an agent (`addmember.mjs` holds the rest of it).
  const inChannel = page.locator('.modal', { has: page.locator('form.add-agent') });
  const addingHere = async () => {
    await addingFromItsLine(page);
    await page.locator('.modal').getByRole('radio', { name: /^An agent/ }).check();
    await inChannel.waitFor();
  };
  const way = (called) => inChannel.getByRole('radio', { name: new RegExp(`^${called}`) });
  // A name typed, and the list of the agents there are put away, as a person does who goes on to what is under it.
  const nameHere = async (typed) => {
    await inChannel.getByLabel('Name', { exact: true }).fill(typed);
    await page.locator('.agents-offered').waitFor();
    await inChannel.getByLabel('Name', { exact: true }).press('Escape');
    await page.locator('.agents-offered').waitFor({ state: 'detached' });
  };
  await addingHere();
  expect('the dialog to begin on an agent this factory runs', await way('Run by this factory').isChecked());
  expect('the other to be one from outside', !(await way('From outside').isChecked()));
  // The agents there are, offered under the name with what is said of each: one from outside as external.
  await offeredUnder(page, { out, expect, inChannel, widths: WIDTHS });
  await nameHere('walls');
  const thereWords = await inChannel.locator('.add-agent-there').innerText();
  expect(`one from outside that is there to be said as that, got "${thereWords}"`, thereWords === 'walls is an agent from outside (joined). Adding it brings it into this channel, and nothing else of it changes.');
  for (const asked of ['Runs on', 'Model', 'May do, unasked', 'Standing instructions (optional)', 'Project', 'The invitation works for'])
    expect(`nothing else to be asked of it, and "${asked}" is`, (await inChannel.locator('label, .field', { hasText: new RegExp(`^${asked.replace(/[()]/g, '\\$&')}`) }).count()) === 0);
  await atWidths(page, 'the channel’s dialog adding one from outside that is there', '11-channel-there');
  await inChannel.getByRole('button', { name: 'Add agent', exact: true }).click();
  await inChannel.waitFor({ state: 'detached' });
  const membersHere = page.getByRole('complementary', { name: 'Tools' });
  if ((await membersHere.count()) === 0) await page.getByRole('button', { name: 'Expand the tools panel' }).click();
  await toolsAtRest(page).waitFor();
  await membersHere.getByRole('tab', { name: /^Channel\b/ }).click().catch(() => {});
  // Walls was being read about beside the other channel: back to everybody of this one.
  const everybodyHere = membersHere.locator('.context-crumbs button', { hasText: /^Channel$/ });
  if (await everybodyHere.count()) await everybodyHere.click();
  const memberRow = (name) => membersHere.locator('.context-agent', { hasText: name });
  await memberRow('walls').waitFor();
  expect('the daemon to have it in Shop still', /^walls\b.*\bshop\b/m.test((await cli('agent', 'list')).stdout));
  // Invited from there: the way chosen says what it means, and what is asked is the Agents tab's.
  await addingHere();
  await way('From outside').check();
  const meant = await inChannel.locator('.add-agent-said').innerText();
  expect(`one from outside to be said in a sentence, got "${meant}"`, meant.startsWith('An agent from outside is a program somebody runs for themselves') && meant.includes('This factory starts nothing for it'));
  expect('nothing of a program to be asked of it', (await inChannel.locator('label', { hasText: 'Runs on' }).count()) === 0);
  expect('nothing to invite until it has a name', await inChannel.getByRole('button', { name: 'Invite', exact: true }).isDisabled());
  expect('it to begin on the channel’s project', (await inChannel.getByLabel('The project it is listed with').inputValue()) === 'shop');
  await nameHere('Gate Keeper');
  expect(`what it will be called to be said, got "${await inChannel.locator('.invite-called').innerText()}"`, (await inChannel.locator('.invite-called').innerText()) === 'It will be called gate-keeper.');
  expect(`what an agent usually holds to be offered, got ${JSON.stringify(await lines(inChannel.locator('.holds-table')))}`, (await lines(inChannel.locator('.holds-table'))).inbox === 'read and change');
  // What it may ask is set here, as the Tools panel sets it, and is what it is invited with.
  await inChannel.locator('.member-setting .holds-table').getByLabel('What it may do with board').selectOption({ label: 'read' });
  await inChannel.getByLabel('How long the invitation works').selectOption({ label: '3 days' });
  await atWidths(page, 'the channel’s dialog inviting one from outside', '11-channel-invite');
  // What the daemon refuses is said in its sentence, with the dialog staying: a project gone as it was open.
  await cli('project', 'create', 'Shed', '--bare');
  await until('Shed to be offered', async () => (await inChannel.getByLabel('The project it is listed with').locator('option').allInnerTexts()).includes('Shed'));
  await inChannel.getByLabel('The project it is listed with').selectOption({ label: 'Shed' });
  await cli('project', 'forget', 'shed');
  await inChannel.getByRole('button', { name: 'Invite', exact: true }).click();
  await inChannel.locator('.error-text').waitFor();
  expect(`the daemon's refusal to be said, got "${await inChannel.locator('.error-text').innerText()}"`, (await inChannel.locator('.error-text').innerText()).includes('shed'));
  expect('nobody to have been invited for it', !/^gate-keeper\b/m.test((await cli('agent', 'list')).stdout));
  await atWidths(page, 'the channel’s dialog, refused', '11-channel-refused');
  await inChannel.getByLabel('The project it is listed with').selectOption({ label: 'Shop' });
  await inChannel.getByRole('button', { name: 'Invite', exact: true }).click();
  const invitedHere = page.locator('.modal .invited-agent[data-agent="gate-keeper"]');
  await invitedHere.getByRole('heading', { name: 'gate-keeper is invited' }).waitFor();
  expect(`it to be said to be in the channel as invited, got "${await invitedHere.locator('.invited-into').innerText()}"`, (await invitedHere.locator('.invited-into').innerText()) === 'gate-keeper is in porch, listed as invited until it joins.');
  const gateInvitation = (await invitedHere.locator('.invite-code').innerText()).trim();
  expect(`an invitation and the command to be shown, each with Copy, got "${gateInvitation}"`, /^enrol_[a-z0-9]{40,}$/.test(gateInvitation) && (await invitedHere.locator('.invite-join').innerText()).trim() === `${meta.cli} agent join --url ${url}` && (await invitedHere.getByRole('button', { name: 'Copy', exact: true }).count()) === 2);
  expect('it to say that it works within three days', (await invitedHere.innerText()).includes('once, within the next 3 days'));
  const gateHolds = (await agentsSaid(page)).find((one) => one.name === 'gate-keeper')?.scopes ?? [];
  expect(`it to hold what was set in the dialog, got ${gateHolds}`, gateHolds.includes('board:read') && !gateHolds.includes('board') && gateHolds.includes('channels'));
  await atWidths(page, 'the invitation shown beside a channel', '11-channel-invited');
  await invitedHere.getByRole('button', { name: 'Done', exact: true }).click();
  await invitedHere.waitFor({ state: 'detached' });
  expect('nothing of the invitation to be on the page once it was put away', !(await page.content()).includes(gateInvitation));
  await until('it to be among the channel’s members as invited', async () => /invited · has yet to join/.test((await memberRow('gate-keeper').innerText().catch(() => ''))) || (await memberRow('gate-keeper').innerText().catch(() => 'not there')));
  expect(`its line there to say it is external, got "${await memberRow('gate-keeper').innerText()}"`, /agent · external/.test(await memberRow('gate-keeper').innerText()));
  await page.screenshot({ path: `${out}/external-11-channel-member-invited.png` });
  // Joined by the built program from a home of its own, and said to have joined there with the page not read again.
  await page.evaluate(() => (window.notReadAgain = true));
  const gateJoined = await handed(gateInvitation, 'agent', 'join');
  expect(`it to join with the invitation, got "${gateJoined.said}"`, gateJoined.ok && gateJoined.said.startsWith(`joined ${url} as the agent \`gate-keeper\``));
  const gate = await run(binary, ['--as', 'gate-keeper', 'whoami', '--url', url], { cwd: repo, env: theirs });
  expect(`a command asked as it to be the agent's, got "${gate.stdout}"`, gate.stdout.startsWith('gate-keeper (agent:gate-keeper)'));
  await until('the channel to say it has joined', async () => /\bjoined\b/.test(await memberRow('gate-keeper').innerText()) || (await memberRow('gate-keeper').innerText()));
  expect('the page not to have been read again for it', await page.evaluate(() => window.notReadAgain === true));
  await page.screenshot({ path: `${out}/external-11-channel-member-joined.png` });
  // The rest of the one dialog: a person, an invitation to register, a bot, what may be set of each, and somebody who holds less.
  await addMember({ browser, page, url, out, cli, expect, until, signIn, sound, atWidths, scheme });

  // ---- moved to another project ------------------------------------------------------------
  // One added by hand, from its line's menu, by a dialog that lists the projects and says what moving does.
  await cli('project', 'create', 'Cafe', '--bare');
  await agentsOf(page);
  const menuOf = async (on, name) => {
    await line(on, name).getByRole('button', { name: `More that can be done with ${name}`, exact: true }).click();
    await on.getByRole('menu').waitFor();
    return (await on.getByRole('menuitem').allInnerTexts()).map((one) => one.trim());
  };
  const offeredHelper = await menuOf(page, 'helper');
  expect(`one added by hand to be offered moving it, got ${offeredHelper}`, offeredHelper.join('|') === 'Edit…|History|Stop|Move to project…|Remove…');
  await page.getByRole('menuitem', { name: 'Move to project…', exact: true }).click();
  const moving = page.locator('.modal .agent-move[data-agent="helper"]');
  await moving.getByRole('heading', { name: 'Move helper to another project' }).waitFor();
  const choice = moving.getByLabel('The project it is to work in');
  const choices = await choice.locator('option').allInnerTexts();
  const every = JSON.parse((await cli('project', 'list', '--json')).stdout).projects.map((one) => one.name);
  expect(`no project and every project to be offered, got ${choices} of ${every}`, choices[0] === 'No project' && [...choices.slice(1)].sort().join() === [...every].sort().join() && every.includes('Cafe'));
  expect('it to be where it is now, with nothing to move to', (await choice.inputValue()) === 'shop' && (await moving.getByRole('button', { name: 'Move it' }).isDisabled()));
  await choice.selectOption({ label: 'Cafe' });
  const movingWords = await moving.locator('.agent-move-said').allInnerTexts();
  expect(`what moving does to be said, from its next piece of work, got ${movingWords}`, movingWords[0] === 'From its next piece of work its wall shows its program Cafe’s folder, and it is listed under Cafe.' && movingWords[1] === 'Nothing else of it changes: it holds what it held.');
  await atWidths(page, 'the dialog that moves an agent', '7-move');
  await moving.getByRole('button', { name: 'Move it', exact: true }).click();
  await moving.waitFor({ state: 'detached' });
  await page.locator('.agents-of[data-of="cafe"] .agent-line[data-agent="helper"]').waitFor();
  expect('the daemon to have it in Cafe', /^helper\b.*\bcafe\b/m.test((await cli('agent', 'list')).stdout));
  // What the daemon refuses is said in its sentence, with the dialog staying: a project gone as the dialog was open.
  await cli('project', 'create', 'Attic', '--bare');
  await menuOf(page, 'helper');
  await page.getByRole('menuitem', { name: 'Move to project…', exact: true }).click();
  await until('Attic to be offered', async () => (await moving.getByLabel('The project it is to work in').locator('option').allInnerTexts()).includes('Attic'));
  await moving.getByLabel('The project it is to work in').selectOption({ label: 'Attic' });
  await cli('project', 'forget', 'attic');
  await moving.getByRole('button', { name: 'Move it', exact: true }).click();
  await moving.locator('.error-text').waitFor();
  const why = await moving.locator('.error-text').innerText();
  expect(`the daemon's refusal to be said, got "${why}"`, why.includes('attic'));
  await page.screenshot({ path: `${out}/external-7-move-refused.png` });
  await moving.getByRole('button', { name: 'Cancel', exact: true }).click();
  await moving.waitFor({ state: 'detached' });
  // Moved to no project, it works in none, and is listed so, by the page and by the daemon.
  await menuOf(page, 'helper');
  await page.getByRole('menuitem', { name: 'Move to project…', exact: true }).click();
  await moving.getByLabel('The project it is to work in').selectOption({ label: 'No project' });
  await moving.getByRole('button', { name: 'Move it', exact: true }).click();
  await moving.waitFor({ state: 'detached' });
  await page.locator('.agents-of[data-of="cafe"] .agent-line[data-agent="helper"]').waitFor({ state: 'detached' });
  const nowhere = JSON.parse((await cli('agent', 'list', '--json')).stdout).agents.find((one) => one.name === 'helper');
  expect(`the daemon to have helper in no project, got ${JSON.stringify(nowhere?.project)}`, nowhere && nowhere.project === null && nowhere.active === true);
  // One this factory does not start moves the same way, and what is said is of where it is listed.
  await menuOf(page, 'walls');
  await page.getByRole('menuitem', { name: 'Move to project…', exact: true }).click();
  const walled = page.locator('.modal .agent-move[data-agent="walls"]');
  await walled.getByLabel('The project it is to work in').selectOption({ label: 'No project' });
  const wallsSaid = (await walled.locator('.agent-move-said').allInnerTexts())[0];
  expect(`what is said of one from outside to be where it is listed, got "${wallsSaid}"`, wallsSaid === 'It is listed under no project.');
  await walled.getByRole('button', { name: 'Move it', exact: true }).click();
  await walled.waitFor({ state: 'detached' });
  await until('walls to be listed with no project', async () => /^walls\b/m.test((await cli('agent', 'list')).stdout) && !(await page.locator('.agents-of[data-of="shop"] .agent-line[data-agent="walls"]').count()));
  await cli('agent', 'move', 'walls', 'shop');
  await page.locator('.agents-of[data-of="shop"] .agent-line[data-agent="walls"]').waitFor();
  // One a fleet's row counts has its fleet's project, and no such line.
  await cli('apply', 'std/role', '--project', 'shop', '--key', 'workers', '--param', 'role=worker', '--param', 'name=shop-worker', '--param', 'count=1');
  await line(page, 'shop-worker-1').waitFor();
  const counted = await menuOf(page, 'shop-worker-1');
  expect(`one a fleet counts to be offered no moving, got ${counted}`, !counted.includes('Move to project…'));
  // A menu listens for Escape a moment after it is drawn.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.getByRole('menu').waitFor({ state: 'detached' });

  // ---- an agent this factory starts, added from the page --------------------------------------
  // The dialog a channel adds one with, asked here which project it works in, of the workspace showing, and under which
  // environment where that project has some; it is put in no channel, and listed in its band with the page not read again.
  await agentsOf(page);
  await page.evaluate(() => (window.notReadAgain = true));
  const stillThis = async () => page.evaluate(() => window.notReadAgain === true);
  const adding = page.getByRole('button', { name: 'Add or invite an agent', exact: true });
  const addDialog = page.locator('.modal', { has: page.locator('form.add-agent') });
  const addOf = (label) => addDialog.getByLabel(label);
  const openAdding = async () => {
    await adding.click();
    await addDialog.getByRole('heading', { name: 'Add or invite an agent' }).waitFor();
    expect('the dialog to begin on one this factory runs', await addDialog.getByRole('radio', { name: /^Run by this factory/ }).isChecked());
  };
  const band = (of, name) => page.locator(`.agents-of[data-of="${of}"] .agent-line[data-agent="${name}"]`);
  const head = await page.locator('.agents-listed > .panel-head .muted').innerText();
  expect(`the head to say agents are added here as from a channel, got "${head}"`, head === 'Add one here or from a channel’s members. Each works from its own slot.');
  await openAdding();
  const projectsOffered = await addOf('The project it works in').locator('option').allInnerTexts();
  expect(`no project and the projects of the workspace showing to be offered, and none of another's, got ${projectsOffered}`, projectsOffered.join('|') === 'No project|Shop|Cafe' || projectsOffered.join('|') === 'No project|Cafe|Shop');
  expect('it to begin on no project with every workspace shown', (await addOf('The project it works in').inputValue()) === '');
  expect('no environment to be asked of no project', (await addOf('The environment it works under').count()) === 0);
  expect('nothing to say what is known of a channel', (await addDialog.locator('datalist').count()) === 0);
  await addOf('Name').fill('Porter');
  await addDialog.getByRole('button', { name: 'Add agent', exact: true }).click();
  await addDialog.waitFor({ state: 'detached' });
  await band('', 'porter').waitFor();
  expect('the daemon to have it in no project and in no channel', /^porter\s+\S+\s+-\s+active/m.test((await cli('agent', 'list')).stdout));
  // Narrowed to a project, it begins there; a name an agent has already is said and not added.
  const narrowing = page.getByLabel(/^What is shown of /);
  await narrowing.selectOption('shop');
  await page.getByRole('heading', { name: 'Shop', level: 2 }).waitFor();
  await openAdding();
  expect('it to begin on the project the page is narrowed to', (await addOf('The project it works in').inputValue()) === 'shop');
  await addOf('Name').fill('Helper');
  const takenSaid = await addDialog.locator('.add-agent-closed').innerText();
  expect(`a name an agent has to be said, got "${takenSaid}"`, takenSaid.startsWith('There is already an agent called helper.') && (await addDialog.getByRole('button', { name: 'Add agent', exact: true }).isDisabled()));
  await addOf('Name').fill('Stocker');
  await addDialog.getByRole('button', { name: 'Add agent', exact: true }).click();
  await addDialog.waitFor({ state: 'detached' });
  await band('shop', 'stocker').waitFor();
  expect('the daemon to have it in Shop', /^stocker\s+\S+\s+shop\s+active/m.test((await cli('agent', 'list')).stdout));
  // A project with environments: one is chosen before it is added, and it works under that one.
  await cli('project', 'environment', 'cafe', 'staging', 'add');
  await cli('project', 'environment', 'cafe', 'prod', 'add');
  await narrowing.selectOption('cafe');
  await page.getByRole('heading', { name: 'Cafe', level: 2 }).waitFor();
  await openAdding();
  expect('it to begin on Cafe', (await addOf('The project it works in').inputValue()) === 'cafe');
  await until('its environments to be offered', async () => (await addOf('The environment it works under').count()) === 1 || 'not yet');
  const envs = await addOf('The environment it works under').locator('option').allInnerTexts();
  expect(`its environments to be offered with none chosen, got ${envs}`, envs[0] === 'Choose an environment' && [...envs.slice(1)].sort().join() === 'prod,staging' && (await addOf('The environment it works under').inputValue()) === '');
  await addOf('Name').fill('Baker');
  const choose = await addDialog.locator('.add-agent-closed').innerText();
  expect(`it to be closed until one is chosen, saying why, got "${choose}"`, choose.startsWith('Choose the environment it works under') && (await addDialog.getByRole('button', { name: 'Add agent', exact: true }).isDisabled()));
  await atWidths(page, 'the dialog that adds an agent from the page', '7-add');
  await addOf('The environment it works under').selectOption('staging');
  await addDialog.getByRole('button', { name: 'Add agent', exact: true }).click();
  await addDialog.waitFor({ state: 'detached' });
  await band('cafe', 'baker').waitFor();
  expect('the daemon to have it working under staging', /^baker\s+\S+\s+cafe under staging\s+active/m.test((await cli('agent', 'list')).stdout));
  await band('cafe', 'baker').scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/external-7-added.png` });
  // What the daemon refuses is said in its sentence, with the dialog staying: a project gone as the dialog was open.
  await cli('project', 'create', 'Loft', '--bare');
  await openAdding();
  await until('Loft to be offered', async () => (await addOf('The project it works in').locator('option').allInnerTexts()).includes('Loft'));
  await addOf('The project it works in').selectOption({ label: 'Loft' });
  await addOf('Name').fill('Joiner');
  await cli('project', 'forget', 'loft');
  await addDialog.getByRole('button', { name: 'Add agent', exact: true }).click();
  await addDialog.locator('.error-text').waitFor();
  const addRefused = await addDialog.locator('.error-text').innerText();
  expect(`the daemon's refusal to be said, got "${addRefused}"`, addRefused.includes('loft'));
  await atWidths(page, 'the dialog that adds an agent, refused', '7-add-refused');
  await addDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await addDialog.waitFor({ state: 'detached' });
  expect('no agent to have been added for it', !/^joiner\b/m.test((await cli('agent', 'list')).stdout));
  expect('the page not to have been read again for any of it', await stillThis());

  // ---- stopped ------------------------------------------------------------------------------
  await agentsOf(page);
  await line(page, 'walls').getByRole('button', { name: 'More that can be done with walls', exact: true }).click();
  await page.getByRole('menu').waitFor();
  const offered = await page.getByRole('menuitem').allInnerTexts();
  expect(`its menu to offer its history, stopping it and taking it away, as any agent's, got ${offered}`, offered.join('|') === 'Edit…|History|Stop|Move to project…|Remove…');
  await page.getByRole('menuitem', { name: 'Stop', exact: true }).click();
  await until('the list to say that it is stopped', async () => (await doing(page, 'walls')) === 'stopped' || (await doing(page, 'walls')));
  const stopped = await asWalls('whoami');
  expect(`its command to be refused, said for what it is, got "${stopped.said}"`, !stopped.ok && stopped.said.includes('the credential kept on this machine for the agent `walls` is not taken at') && !stopped.said.includes('login'));
  await line(page, 'walls').scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/external-8-stopped.png` });

  // ---- somebody who may not add agents ------------------------------------------------------
  // A person is given less by no route, so a page is told what they hold: a member, without what changes agents.
  const lesser = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const less = await lesser.newPage();
  const HOLDS = ['projects', 'channels', 'board', 'records', 'blueprints', 'agents:read', 'fleet', 'search'];
  await less.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const me = await answer.json();
    if (!me.principal) return route.fulfill({ response: answer });
    me.principal.scopes = HOLDS;
    // Nor may they invite anybody: the factory's owner may, on the plan this one is on.
    me.can_invite = false;
    await route.fulfill({ response: answer, json: me });
  });
  await signIn(less);
  await agentsOf(less);
  await line(less, 'walls').waitFor();
  expect('somebody who may not add agents to be offered neither adding nor inviting', (await less.getByRole('button', { name: 'Add or invite an agent', exact: true }).count()) === 0);
  expect(`the head to say nothing of adding to them, got "${await less.locator('.agents-listed > .panel-head .muted').innerText()}"`, (await less.locator('.agents-listed > .panel-head .muted').innerText()) === 'Each works from its own slot.');
  expect('them still to be shown that it is external', (await runsOn(less, 'walls')) === 'external');
  // Nor beside a channel, where an agent this factory runs and one from outside are both added from Add agent.
  await less.getByRole('button', { name: 'Channels', exact: true }).click();
  await unfoldChannels(less);
  await less.locator('.nav').getByText('yard', { exact: true }).click();
  await less.getByRole('heading', { name: '# yard' }).waitFor();
  // The line at the foot of who is in the channel names the one kind they may add.
  await less.locator('.channel > .split > .view-head').getByRole('button', { name: /participants?$/ }).click();
  expect(`the line that adds to name only a person for them, got "${await addLine(less).innerText()}"`, (await addLine(less).innerText()).trim() === 'Add a person');
  await addLine(less).click();
  const lessDialog = addingDialog(less);
  await lessDialog.locator('form.add-person').waitFor();
  expect('them to be offered no choice of who it is, a person being all they may add', (await lessDialog.getByRole('radiogroup', { name: 'Who it is' }).count()) === 0);
  expect('them to be offered no agent and no bot', (await lessDialog.locator('form.add-agent, form.add-bot').count()) === 0);
  expect('them to be offered no inviting', (await lessDialog.getByRole('button', { name: 'Make an invitation' }).count()) === 0);
  const theirSaid = await lessDialog.locator('.member-setting-said').innerText();
  expect(`what a person holds to be said to be read elsewhere, got "${theirSaid}"`, theirSaid.includes('whoever manages the factory reads and changes in Settings'));
  await sound(less, 'the dialog for somebody who may only add people');
  await less.screenshot({ path: `${out}/external-9-may-not-in-a-channel.png` });
  await less.keyboard.press('Escape');
  await lessDialog.waitFor({ state: 'detached' });
  await agentsOf(less);
  // Its menu offers them its history and nothing that is done with it, moving it among that.
  await line(less, 'walls').getByRole('button', { name: 'More that can be done with walls', exact: true }).click();
  await less.getByRole('menu').waitFor();
  const lessMay = (await less.getByRole('menuitem').allInnerTexts()).map((one) => one.trim());
  expect(`them to be offered its setup and its history alone, got ${lessMay}`, lessMay.join('|') === 'Edit…|History');
  await less.waitForTimeout(200);
  await less.keyboard.press('Escape');
  await less.getByRole('menu').waitFor({ state: 'detached' });
  // And the daemon refuses a credential that holds so.
  const narrow = JSON.parse((await cli('key', 'narrow', ...HOLDS.flatMap((scope) => ['--scope', scope]), '--json')).stdout).token;
  const refused = await fetch(`${url}/api/agent/invite`, { method: 'POST', headers: { 'content-type': 'application/json', authorization: `Bearer ${narrow}` }, body: JSON.stringify({ name: 'gate' }) });
  expect(`the daemon to refuse it, got ${refused.status}`, refused.status === 403 && (await refused.text()).includes('needs `agents:write`'));
  await line(less, 'walls').scrollIntoViewIfNeeded();
  await less.screenshot({ path: `${out}/external-9-may-not.png` });

  // ---- somebody who may invite one and not apply its text ------------------------------------
  // Adding one this factory runs applies `std/agent` as whoever asks, which takes `blueprints:run`: without it, inviting is all there is.
  const inviters = await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme });
  const inviter = await inviters.newPage();
  const INVITES = ['projects', 'channels', 'board', 'records', 'blueprints:read', 'agents', 'fleet', 'search'];
  await inviter.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const me = await answer.json();
    if (!me.principal) return route.fulfill({ response: answer });
    me.principal.scopes = INVITES;
    await route.fulfill({ response: answer, json: me });
  });
  await signIn(inviter);
  await agentsOf(inviter);
  await line(inviter, 'walls').waitFor();
  await inviter.getByRole('button', { name: 'Add or invite an agent', exact: true }).click();
  const theirDialog = inviter.getByRole('dialog');
  await theirDialog.getByRole('heading', { name: 'Add or invite an agent' }).waitFor();
  await theirDialog.locator('form.invite-agent').waitFor();
  expect('them to be offered no choice, and inviting alone', (await theirDialog.getByRole('radiogroup').count()) === 0 && (await theirDialog.locator('form.add-agent').count()) === 0);
  expect('the caret to be in the name', await theirDialog.getByLabel("The agent's name").evaluate((box) => document.activeElement === box));
  await atWidths(inviter, 'the dialog for somebody who may only invite', '9-invite-alone');
  await theirDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await theirDialog.waitFor({ state: 'detached' });
  const inviterKey = JSON.parse((await cli('key', 'inviter', ...INVITES.flatMap((scope) => ['--scope', scope]), '--json')).stdout).token;
  const notAdded = await fetch(`${url}/api/agents`, { method: 'POST', headers: { 'content-type': 'application/json', authorization: `Bearer ${inviterKey}` }, body: JSON.stringify({ name: 'gate' }) });
  expect(`the daemon to refuse adding one this factory runs, got ${notAdded.status}`, notAdded.status === 403 && (await notAdded.text()).includes('needs `blueprints:run`'));
  await inviters.close();

  // ---- a phone's width, and the other of dark and light -------------------------------------
  await page.setViewportSize({ width: 390, height: 780 });
  await line(page, 'walls').waitFor();
  const wide = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(`nothing to scroll sideways at a phone's width, and the page is ${wide} wider than the window`, wide <= 0);
  const parts = await line(page, 'walls').evaluate((row) => {
    const frame = row.getBoundingClientRect();
    return [...row.children].filter((part) => part.getBoundingClientRect().right > frame.right + 0.5 || part.getBoundingClientRect().left < frame.left - 0.5).map((part) => part.className);
  });
  expect(`every part of its line to be inside the line at a phone's width, got ${parts}`, parts.length === 0);
  await line(page, 'walls').scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/external-10-phone.png` });

  expect(`no error on any page, got ${problems.join('; ')}`, problems.length === 0);
  await browser.close();
  console.log(`an external agent, by what the pages show and by the boxes: pictures in ${out}`);
} catch (err) {
  failed = err;
} finally {
  await cli('down').catch(() => {});
  daemon.kill();
}
if (failed) {
  console.error(failed);
  process.exit(1);
}
