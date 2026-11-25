// Reviewing a card: one that is in review, opened, can be sent back with
// what is wrong, and its history then says who sent it back and why.
//
//   DAEMON=../target/debug/factory node scripts/review.mjs [out-dir]
//
// And a card that something was asked on: it is marked on the board, the
// question is the first thing in its drawer with a box to answer in, and
// what is said on a card is drawn among its history as what was said.
// And what is said there marked public: the switch, kept with the words,
// and the mark in the history.
// And the drawer itself: how the card came to be where it is, which its
// board writes, is not offered to be filled in; and the drawer's edge is
// dragged, by the pointer and by the arrows, never wider than the window
// in a narrow one, and as wide as it was left when the page is read again.
//
// And what two said as a card was sent back is drawn as two paragraphs.
//
// And what a blueprint's step says of a card for itself is drawn under the
// factory's bot, as what its step does to the card is.
//
// And a card its round left for a person: opened, it says so, with the
// reason alone and where it is taken up again from, until somebody moves
// it.
//
// And a board whose fleet says where a card that did not pass goes: sent
// back, a card goes where the row of the stage that has it says, which
// the drawer names, and when the row says somewhere else, there.
//
// There is no lane here to take the card up again: that is the daemon's
// business and its own tests'. This is what a person sees and presses. The
// environment variables are those of walkthrough.mjs.
import { execFile, spawn } from 'node:child_process';
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { chromium } from 'playwright-core';
import { drawerAtRest } from './drawer.mjs';
import { endsWithThis } from './ends.mjs';
import { litAs } from './lit.mjs';
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
const data = join(out, 'review-data');
const home = join(out, 'review-home');

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
  await cli('apply', 'std/board', '--project', 'shop');
  await cli('board', 'add', 'shop', 'Add a cart', '--body', 'Customers need one.');
  await cli('board', 'add', 'shop', 'Write the returns policy');
  await cli('board', 'move', 'shop-1', 'review');

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
  await page.goto(url);
  await page.getByLabel('Email').fill('ada@example.com');
  await page.getByLabel('Password').fill('correct horse');
  await page.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await page.locator('.rail').waitFor();

  // The board is under its project, in the side panel.
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Shop', { exact: true }).click();
  await page.getByRole('heading', { name: 'Shop', level: 1 }).waitFor();
  await page.locator('.nav [data-project="shop"]').getByText('Board', { exact: true }).click();
  const column = (name) => page.locator('.column', { hasText: name });
  const cart = page.locator('.card', { hasText: 'Add a cart' });
  await column('review').locator('.card', { hasText: 'Add a cart' }).waitFor();
  const drawer = drawerAtRest(page);
  const box = drawer.getByLabel('Not right yet? Say what is wrong');
  const send = drawer.getByRole('button', { name: 'Send back', exact: true });
  // The drawer lies over the right of the board, so it is closed before a card under it is pressed.
  const close = async () => {
    await drawer.getByRole('button', { name: 'Close' }).click();
    await drawer.waitFor({ state: 'detached' });
  };

  // A card that is not in review is not offered it: there is nothing to send back.
  await page.locator('.card', { hasText: 'Write the returns policy' }).click();
  await drawer.getByLabel('Title').waitFor();
  expect('no Send back on a card that is not in review', (await send.count()) === 0);
  await close();

  // One in review is, with a box for what is wrong.
  await cart.click();
  await box.waitFor();
  expect('the card in review to be the one that is open', (await drawer.getByLabel('Title').inputValue()) === 'Add a cart');
  await page.screenshot({ path: `${out}/review-0-in-review.png` });

  // What is typed is kept when the drawer is closed, and is there again
  // when the card is opened again.
  const wrong = 'The total is wrong when the cart is empty.';
  await box.fill(wrong);
  await close();
  await cart.click();
  await box.waitFor();
  expect(`what was typed to still be there, got "${await box.inputValue()}"`, (await box.inputValue()) === wrong);
  await page.screenshot({ path: `${out}/review-1-what-is-wrong.png` });

  // Sent back, the card is in doing, the drawer has closed, and nothing was asked twice.
  await send.click();
  await column('doing').locator('.card', { hasText: 'Add a cart' }).waitFor();
  await drawer.waitFor({ state: 'detached' });
  expect('the card to have left review', (await column('review').locator('.card').count()) === 0);

  // Its history says who sent it back, and what they said: what happened, in the tab of its own beside the comments.
  await cart.click();
  await drawer.getByRole('tab', { name: /^History/ }).click();
  const said = drawer.locator('.journey-said', { hasText: 'saying' });
  await said.waitFor();
  expect(
    `the history to say who sent it back and why, got "${await said.innerText()}"`,
    (await said.innerText()).trim() === `Ada moved it from review to doing, saying: ${wrong}`,
  );
  expect('no Send back on a card that has been sent back', (await send.count()) === 0);
  await said.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/review-2-sent-back.png` });
  await close();

  // Sent back with nothing said, a move is only a move.
  await cli('board', 'move', 'shop-1', 'review');
  await column('review').locator('.card', { hasText: 'Add a cart' }).waitFor();
  await cart.click();
  await box.waitFor();
  expect('nothing left in the box from the last time', (await box.inputValue()) === '');
  await send.click();
  await column('doing').locator('.card', { hasText: 'Add a cart' }).waitFor();
  const history = (await cli('board', 'history', 'shop-1')).stdout.trim().split('\n').map((line) => line.replace(/^\S+\s+/, ''));
  expect(`the last move to say no more than that it was made, got "${history.at(-1)}"`, history.at(-1) === 'Ada moved it from review to doing');

  // Sent back with what two said, each apart from the other by an empty line as a council's words are: its history draws
  // them as two paragraphs, the first beside whoever sent it back and the second on a line of its own, and never run
  // together (the demonstration's card sent back by two reviewers read as one).
  await cli('board', 'move', 'shop-1', 'review');
  await column('review').locator('.card', { hasText: 'Add a cart' }).waitFor();
  await cart.click();
  await box.waitFor();
  const first = 'shop-reviewer-1: The total is wrong when the cart is empty.';
  const second = 'shop-reviewer-2: Same finding, and the tax is added twice.';
  await box.fill(`${first}\n\n${second}`);
  await send.click();
  await column('doing').locator('.card', { hasText: 'Add a cart' }).waitFor();
  await cart.click();
  await drawer.getByRole('tab', { name: /^History/ }).click();
  const twice = drawer.locator('.journey-said', { hasText: 'Same finding' });
  await twice.waitFor();
  const paragraphs = await twice.locator('.journey-para').evaluateAll((all) => all.map((one) => ({ text: one.textContent, box: one.getBoundingClientRect().toJSON() })));
  expect(`the two said to be two paragraphs, got ${JSON.stringify(paragraphs.map((one) => one.text))}`, paragraphs.length === 2 && paragraphs[0].text === `Ada moved it from review to doing, saying: ${first}` && paragraphs[1].text === second);
  expect(`the second to begin on a line of its own under the first, got ${JSON.stringify(paragraphs.map((one) => one.box))}`, paragraphs[1].box.top >= paragraphs[0].box.bottom && paragraphs[1].box.left <= paragraphs[0].box.left);
  await twice.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/review-2b-two-said.png` });
  await close();

  // ---- something asked on the card ---------------------------------------------
  const bot = /`([^`]+)`/.exec((await cli('channel', 'speaker')).stdout)?.[1];
  // A card's round says that its agent asked (`card.asked`), what looks after
  // the board's cards puts the question on the card, and what a fleet attaches
  // (`std/told`) says it on the card, naming whom its rows say it is put to,
  // which is what tells them. Here a blueprint of two steps does
  // the same, so that there is a question without an agent or a fleet: put
  // to whoever applied it, which is Ada.
  const asks = join(data, 'projects/shop/blueprints/ask.bp');
  await mkdir(join(data, 'projects/shop/blueprints'), { recursive: true });
  await writeFile(
    asks,
    'blueprint: ask\npermissions: [board, inbox]\nsteps:\n  ask:\n    do: emit\n    with: { topic: card.asked, card: shop-1, project: shop, asker: builder, question: "Which total: with tax, or without?" }\n  said:\n    needs: [ask]\n    do: record.ask\n    with: { id: shop-1, says: "Which total: with tax, or without?", to: [applier], asker: builder, once: asked }\n',
  );
  await cli('apply', asks, '--project', 'shop');
  // The card says so on the board, without being opened.
  const mark = column('doing').locator('.card', { hasText: 'Add a cart' }).locator('.badge', { hasText: 'question' });
  await mark.waitFor();
  await page.screenshot({ path: `${out}/review-3-marked.png` });

  // Opened, Details says only whom it waits on and where it is answered, in a line that leads there: no question
  // and no box of its own.
  await cart.click();
  const waiting = drawer.getByRole('button', { name: /^Waiting on an answer/ });
  // Whom it was put to comes from the card's history, read a moment after the card.
  await drawer.getByRole('button', { name: 'Waiting on an answer from @ada: answer in Comments', exact: true }).waitFor();
  expect('no box to answer in on Details', (await drawer.getByLabel('Your answer').count()) === 0);
  expect('the question not to be on Details', (await drawer.getByRole('tabpanel', { name: 'Details' }).getByText('Which total').count()) === 0);
  expect('the question not to be among the labels a person types', (await drawer.locator('.field-many', { has: page.getByRole('combobox', { name: 'Labels' }) }).locator('.tag').count()) === 0);
  expect('the question not to be shown a second time as a field of the card', (await drawer.getByLabel('Question', { exact: true }).count()) === 0);
  const answer = drawer.getByLabel('Your answer');
  // The line drawn in the accent with no ground of its own, and, pressed, Comments with the box having the keyboard
  // and outlined in the accent, in the dark and in the light.
  const accents = {};
  const asksDrawn = async (scheme) => {
    await litAs(page, scheme);
    await drawer.getByRole('tab', { name: /^Details/ }).click();
    const seen = await waiting.evaluate((line) => {
      const accent = document.createElement('span');
      accent.style.color = 'var(--accent)';
      document.body.append(accent);
      const wanted = getComputedStyle(accent).color;
      accent.remove();
      const drawn = getComputedStyle(line);
      return { wanted, ground: drawn.backgroundColor, outline: [drawn.borderTopColor, drawn.borderRightColor, drawn.borderBottomColor, drawn.borderLeftColor] };
    });
    accents[scheme] = seen.wanted;
    expect(`no ground about the line that says what it waits on, and its outline in the accent, in the ${scheme}, got ${JSON.stringify(seen)}`, seen.ground === 'rgba(0, 0, 0, 0)' && seen.outline.every((one) => one === seen.wanted));
    await page.screenshot({ path: `${out}/review-3b-asks-${scheme}.png` });
    await waiting.click();
    await answer.waitFor();
    await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Your answer' || document.activeElement?.closest('.card-answering') !== null);
    const box = await answer.evaluate((area) => {
      const drawn = getComputedStyle(area);
      return [drawn.borderTopColor, drawn.borderRightColor, drawn.borderBottomColor, drawn.borderLeftColor];
    });
    expect(`the answer's box outlined in the accent in the ${scheme}, got ${JSON.stringify(box)}`, box.every((one) => one === seen.wanted));
    await page.screenshot({ path: `${out}/review-3c-answering-${scheme}.png` });
  };
  // The other of the two first, and then the one the page was opened in, which the rest of the check goes on in.
  const opened = process.env.SCHEME ?? 'dark';
  await asksDrawn(opened === 'dark' ? 'light' : 'dark');
  await asksDrawn(opened);
  expect(`the dark and the light read apart, each in its own accent, got ${JSON.stringify(accents)}`, accents.dark !== accents.light);
  // What is typed is kept when the drawer is closed, like what is wrong with a card in review.
  await answer.fill('Without tax.');
  await close();
  await cart.click();
  await drawer.getByRole('tab', { name: /^Comments/ }).click();
  await answer.waitFor();
  expect(`the answer typed to still be there, got "${await answer.inputValue()}"`, (await answer.inputValue()) === 'Without tax.');
  // The question is under Comments, said by whoever asked it and naming whom it was put to, once.
  const spokenLines = drawer.locator('.journey-spoken');
  await spokenLines.first().waitFor();
  expect(`the question to be in Comments as something asked, naming Ada, got "${await spokenLines.last().innerText()}"`, (await spokenLines.last().innerText()).includes('builder asked') && (await spokenLines.last().innerText()).includes('@ada Which total'));
  expect('the question to be said in Comments once', (await drawer.locator('.journey-spoken', { hasText: 'Which total' }).count()) === 1);
  await page.screenshot({ path: `${out}/review-4-asked.png`, fullPage: true });
  // Being named told Ada, in her inbox.
  const inbox = await page.evaluate(async () => (await fetch('/api/inbox', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json());
  expect(`Ada to be told by being named, got ${JSON.stringify(inbox.lines)}`, inbox.lines.some((line) => line.kind === 'commented' && line.place === 'shop-1'));

  // Answered, the card stops waiting: the question leaves the top of the
  // drawer, the answer is in the history, and the mark is off the card.
  await drawer.getByRole('button', { name: 'Answer', exact: true }).click();
  await drawer.locator('.journey-spoken', { hasText: 'Without tax.' }).waitFor();
  await drawer.getByRole('tab', { name: /^Details/ }).click();
  await waiting.waitFor({ state: 'detached' });
  await drawer.getByRole('tab', { name: /^Comments/ }).click();
  // That it was the answer is said by what looks after the cards a moment
  // after it was said, and is drawn on what was said, not as a line beside it.
  await drawer.locator('.journey-spoken', { hasText: 'Ada answered' }).waitFor();
  expect(`the answer to be Ada's, got "${await spokenLines.last().innerText()}"`, (await spokenLines.last().innerText()).includes('Ada answered') && (await spokenLines.last().innerText()).includes('Without tax.'));
  expect('that it was the answer not to be a line of its own', (await drawer.locator('.journey li', { hasText: 'that answered what builder asked' }).count()) === 0);
  await mark.waitFor({ state: 'detached' });

  // With nothing asked, the same box is by the history, and what is said is only said.
  const remark = drawer.getByLabel('Say something on this card');
  await remark.fill('Thanks.');
  await drawer.getByRole('button', { name: 'Say', exact: true }).click();
  await drawer.locator('.journey-spoken', { hasText: 'Thanks.' }).waitFor();
  expect(`what was said to be said, got "${await spokenLines.last().innerText()}"`, (await spokenLines.last().innerText()).includes('Ada said'));
  expect('the box to be empty again', (await remark.inputValue()) === '');
  await spokenLines.last().scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/review-5-answered.png` });
  const kept = (await cli('board', 'history', 'shop-1')).stdout.trim().split('\n').map((line) => line.replace(/^\S+\s+/, ''));
  expect(
    `the history to end with what was asked and said, got ${JSON.stringify(kept.slice(-5))}`,
    kept.slice(-5).join('|') === `builder asked: Which total: with tax, or without? · via ask|${bot} said: @ada Which total: with tax, or without? · via ask|Ada said: Without tax.|that answered what builder asked · via std/cards|Ada said: Thanks.`,
  );
  expect(`putting the question on the card and taking it off not to be said in its history, got ${JSON.stringify(kept)}`, !kept.some((one) => one.includes('question')));

  // What is said is private unless it is marked: a switch beside sending,
  // off until it is turned on. The words and the mark are kept together
  // while the drawer is closed, said together, and the switch is off
  // again for the next thing said.
  const open = drawer.getByLabel('Public');
  expect('nothing to be marked public until somebody says so', !(await open.isChecked()));
  await remark.fill('It ships on Friday.');
  const where = await open.boundingBox();
  await open.check();
  const still = await open.boundingBox();
  expect('the switch to stay where it was pressed', where.x === still.x && where.y === still.y);
  await page.screenshot({ path: `${out}/review-6-to-be-public.png` });
  await close();
  await cart.click();
  // Opened afresh, the drawer is on Details: what was said is a tab away.
  await drawer.getByRole('tab', { name: /^Comments/ }).click();
  await remark.waitFor();
  expect(`the words to be kept, got "${await remark.inputValue()}"`, (await remark.inputValue()) === 'It ships on Friday.');
  expect('the mark to be kept with them', await open.isChecked());
  await drawer.getByRole('button', { name: 'Say', exact: true }).click();
  const told = drawer.locator('.journey-spoken', { hasText: 'It ships on Friday.' });
  await told.waitFor();
  await told.locator('.journey-public').waitFor();
  expect('what was only said not to be marked', (await drawer.locator('.journey-spoken', { hasText: 'Thanks.' }).locator('.journey-public').count()) === 0);
  expect('the box to be empty again', (await remark.inputValue()) === '');
  expect('the switch to be off again', !(await open.isChecked()));
  await told.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/review-7-said-publicly.png` });
  const last = (await cli('board', 'history', 'shop-1')).stdout.trim().split('\n').map((line) => line.replace(/^\S+\s+/, '')).at(-1);
  expect(`the history to say it was public, got "${last}"`, last === 'Ada said: It ships on Friday. (public)');

  // What a card's board keeps on it for itself (how it came to be where
  // it is) is no field of its form, though the card has been moved and
  // its table says the field is there.
  const row = await page.evaluate(async () => (await fetch('/api/records/shop-1', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json());
  expect(`the card's row to say how it came to be where it is, got ${JSON.stringify(row)}`, Array.isArray(row.data?.moved) && row.data.moved.length > 0);
  expect('nothing of how it was moved to be offered for filling in', (await drawer.getByLabel(/^Moved/).count()) === 0);
  expect('what its board keeps besides to be offered still', (await drawer.getByLabel(/^Part of/).count()) === 1);

  // ---- whoever said something, with their picture --------------------
  // Each comment has the picture of whoever said it beside their name, the same face as beside their messages in a
  // channel: left of the name, on its line, inside the comment; and each line of what happened its doer's picture
  // before its words. Pressed, a person's is who they are, where anybody is read about.
  await drawer.getByRole('tab', { name: /^Comments/ }).click();
  await spokenLines.last().waitFor();
  const faces = async (where) => {
    const wrong = await drawer.evaluate((root) =>
      [...root.querySelectorAll('.journey li')].flatMap((li) => {
        const face = li.querySelector('.journey-by .avatar.face');
        if (!face) return [`no picture on "${li.innerText.slice(0, 40)}"`];
        const [f, l] = [face.getBoundingClientRect(), li.getBoundingClientRect()];
        const name = li.querySelector('.journey-who')?.getBoundingClientRect();
        const said = [];
        if (f.left < l.left - 0.5 || f.right > l.right + 0.5 || f.top < l.top - 0.5 || f.bottom > l.bottom + 0.5) said.push(`the picture of "${li.innerText.slice(0, 30)}" outside its line`);
        if (name && (f.right > name.left + 0.5 || f.bottom < name.top || f.top > name.bottom)) said.push(`the picture of "${li.innerText.slice(0, 30)}" not just before its name`);
        if (!getComputedStyle(face, '::before').content.replace(/"/g, '').trim()) said.push(`no letters on the picture of "${li.innerText.slice(0, 30)}"`);
        return said;
      }),
    );
    expect(`a picture beside whoever said each thing ${where}: ${wrong.join('; ')}`, wrong.length === 0);
  };
  for (const width of [1280, 1000, 820]) {
    await page.setViewportSize({ width, height: 860 });
    await faces(`on Comments at ${width}`);
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  expect('who asked, a name nothing is known by, to be drawn and to open nothing', (await drawer.locator('.journey-spoken', { hasText: 'Which total' }).locator('span.journey-by').count()) === 1);
  await page.screenshot({ path: `${out}/review-7b-faces.png` });
  // The bot speaks for a record that asks (an A/B test asks on its card): drawn as the bot, with the record named after,
  // which opens its row.
  const tests = join(data, 'projects/shop/blueprints/tests.bp');
  await writeFile(tests, 'blueprint: tests\npermissions: [projects, records]\nsteps:\n  tests:\n    do: collection.ensure\n    with:\n      name: contests\n      project: shop\n      beside: board\n      about: card\n      fields:\n        card: { type: text }\n');
  await cli('apply', tests, '--project', 'shop');
  await cli('record', 'add', 'shop', 'contests', 'card=shop-1');
  const recordAsks = join(data, 'projects/shop/blueprints/testasks.bp');
  await writeFile(recordAsks, 'blueprint: testasks\nsteps:\n  ask:\n    do: emit\n    with: { topic: card.asked, card: shop-1, project: shop, asker: shop-contests-1, question: "Which way is faster?" }\n');
  await cli('apply', recordAsks, '--project', 'shop');
  const forTest = drawer.locator('.journey-spoken', { hasText: 'Which way is faster?' });
  await forTest.waitFor();
  await forTest.locator(`button.journey-by[data-actor="bot:${bot}"]`).waitFor();
  await forTest.locator('.journey-for', { hasText: 'the contest of shop-1' }).and(page.locator('button')).waitFor();
  expect(`the record's line to say the bot speaks for it, got "${await forTest.innerText()}"`, (await forTest.innerText()).startsWith(`${bot}, for the contest of shop-1 asked`));
  await faces('with a record that asked');
  await page.screenshot({ path: `${out}/review-7c-for-a-record.png` });
  await forTest.locator('button.journey-for').click();
  await drawer.getByLabel('Card', { exact: true }).waitFor();
  expect('the row of the record that asked to be opened', (await drawer.getByLabel('Card', { exact: true }).inputValue()) === 'shop-1');
  await close();
  // Ada's own picture and name, pressed and reached by the keyboard, are who she is, read about in the drawer.
  await cart.click();
  await drawer.getByRole('tab', { name: /^Comments/ }).click();
  const ada = drawer.locator('.journey-spoken', { hasText: 'Thanks.' }).locator('button.journey-by');
  await ada.focus();
  await page.keyboard.press('Enter');
  await drawer.locator('.context-about', { hasText: 'person · you' }).waitFor();
  await page.screenshot({ path: `${out}/review-7d-read-about.png` });
  await close();
  // History, what happened: each line with its doer's picture, Ada's pressed opening who she is.
  await cart.click();
  await drawer.getByRole('tab', { name: /^History/ }).click();
  await drawer.locator('.journey-said', { hasText: 'Ada moved it' }).first().waitFor();
  await faces('on History');
  // What a blueprint's step says for itself, with no action that changes the card (an `emit` that what it waited on is
  // done), is the factory's doing as much as a move its step makes: drawn under the bot with the blueprint after it, and
  // never under the picture of whoever applied the blueprint, who only lent it the authority.
  const frees = join(data, 'projects/shop/blueprints/frees.bp');
  await writeFile(frees, 'blueprint: frees\nsteps:\n  free:\n    do: emit\n    with: { topic: card.freed, card: shop-1, project: shop }\n');
  await cli('apply', frees, '--project', 'shop');
  const freed = drawer.locator('.journey li', { hasText: 'what it waited on is done' });
  await freed.locator(`button.journey-by[data-actor="bot:${bot}"]`).waitFor();
  expect(`what the step said to be said as the bot's doing with its blueprint after it, got "${await freed.innerText()}"`, (await freed.innerText()).includes('via frees') && !(await freed.innerText()).includes('Ada'));
  await faces('with what a step said for itself');
  await freed.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/review-7e-said-by-a-step.png` });
  await drawer.locator('.journey-said', { hasText: 'Ada moved it' }).first().locator('button.journey-by').click();
  await drawer.locator('.context-about', { hasText: 'person · you' }).waitFor();
  await close();
  await cart.click();
  await drawer.getByLabel('Title').waitFor();

  // The drawer's edge. Held and dragged, the drawer follows; let go, it
  // stays; and nothing under it moved.
  const edge = drawer.getByRole('separator', { name: 'Make the drawer wider or narrower' });
  const boxOf = async (what) => await what.boundingBox();
  const main = page.locator('.main');
  const under = page.locator('.column').first();
  const [was, lay] = [await boxOf(drawer), await boxOf(under)];
  expect(`the drawer to begin as wide as it usually is, got ${was.width}`, Math.round(was.width) === 400);
  const drag = async (by) => {
    const at = await boxOf(edge);
    await page.mouse.move(at.x + 2, at.y + at.height / 2);
    await page.mouse.down();
    await page.mouse.move(at.x + 2 + by / 2, at.y + at.height / 2, { steps: 4 });
    await page.mouse.move(at.x + 2 + by, at.y + at.height / 2, { steps: 4 });
  };
  await drag(-180);
  await page.waitForFunction(() => Math.round(document.querySelector('.drawer').getBoundingClientRect().width) === 580);
  await page.screenshot({ path: `${out}/review-8-edge-held.png` });
  await page.mouse.up();
  const wider = await boxOf(drawer);
  expect(`the drawer to be as wide as it was dragged to, got ${wider.width}`, Math.round(wider.width) === 580);
  expect('its right side to be where it was', Math.round(wider.x + wider.width) === Math.round(was.x + was.width));
  expect('the board under it not to have moved', JSON.stringify(await boxOf(under)) === JSON.stringify(lay));
  expect('what was written in it to be as it was', (await drawer.getByLabel('Title').inputValue()) === 'Add a cart');
  expect('the width to be kept in the browser', (await page.evaluate(() => localStorage.getItem('client.drawer.width'))) === '580');

  // Dragged past the window's left, it is as wide as the workbench and no wider.
  await drag(-2000);
  await page.mouse.up();
  const [most, bench] = [await boxOf(drawer), await boxOf(main)];
  expect(`the drawer to stay inside the workbench, got ${most.x} to ${most.x + most.width} in ${bench.x} to ${bench.x + bench.width}`, most.x >= bench.x && most.x + most.width <= bench.x + bench.width);
  await page.screenshot({ path: `${out}/review-9-edge-widest.png` });
  // Dragged past its right, it is as narrow as a form can be filled in at.
  await drag(2000);
  await page.mouse.up();
  expect(`the drawer to be no narrower than it can be, got ${(await boxOf(drawer)).width}`, Math.round((await boxOf(drawer)).width) === 300);

  // By the keyboard: the left arrow makes it wider, the right narrower, a step at a time.
  await edge.focus();
  for (let press = 0; press < 5; press += 1) await page.keyboard.press('ArrowLeft');
  await page.waitForFunction(() => Math.round(document.querySelector('.drawer').getBoundingClientRect().width) === 420);
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => Math.round(document.querySelector('.drawer').getBoundingClientRect().width) === 396);
  // Pressed twice, it is as wide as it started.
  await edge.dblclick({ position: { x: 2, y: 200 } });
  await page.waitForFunction(() => Math.round(document.querySelector('.drawer').getBoundingClientRect().width) === 400);
  await drag(-120);
  await page.mouse.up();
  expect('the drawer to be left at what it was dragged to', Math.round((await boxOf(drawer)).width) === 520);

  // The page read again, a drawer is as wide as it was left: this one, and
  // any other (the record of a card opened from the board as a table would
  // be the same drawer). And in a window narrower than that, it is as
  // wide as there is room for, with nothing scrolling sideways.
  await page.reload();
  await page.locator('.rail').waitFor();
  await page.locator('.card', { hasText: 'Write the returns policy' }).click();
  await drawer.getByLabel('Title').waitFor();
  expect(`a drawer opened after the page was read again to be as wide as it was left, got ${(await boxOf(drawer)).width}`, Math.round((await boxOf(drawer)).width) === 520);
  await page.screenshot({ path: `${out}/review-10-edge-kept.png` });
  await page.setViewportSize({ width: 820, height: 860 });
  await page.waitForFunction(() => {
    const [drawer, main] = [document.querySelector('.drawer'), document.querySelector('.main')].map((one) => one.getBoundingClientRect());
    return drawer.left >= main.left && drawer.right <= main.right;
  });
  const [narrow, room] = [await boxOf(drawer), await boxOf(main)];
  expect(`the drawer to be inside a narrow window, got ${narrow.x} to ${narrow.x + narrow.width} of ${room.x} to ${room.x + room.width}`, narrow.x >= room.x && narrow.x + narrow.width <= room.x + room.width && narrow.x + narrow.width <= 820 && narrow.width < 520);
  expect('nothing to scroll sideways', await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth));
  // Dragged there, it goes no wider than the window.
  await drag(-900);
  await page.mouse.up();
  const held = await boxOf(drawer);
  expect(`the drawer dragged in a narrow window to stay inside it, got ${held.x} to ${held.x + held.width}`, held.x >= room.x && held.x + held.width <= room.x + room.width);
  await page.screenshot({ path: `${out}/review-11-edge-narrow-window.png` });
  // On a phone the drawer has the screen, and there is no edge to drag.
  await page.setViewportSize({ width: 390, height: 844 });
  await edge.waitFor({ state: 'detached' });
  const whole = await boxOf(drawer);
  expect(`the drawer to have a phone's screen, got ${JSON.stringify(whole)}`, whole.x === 0 && Math.round(whole.width) === 390);
  await page.screenshot({ path: `${out}/review-12-drawer-on-a-phone.png` });
  await page.setViewportSize({ width: 1280, height: 860 });

  // ---- where a card that is sent back goes is its stage's to say -------------------
  // A project with a fleet, and nobody in it: what is done with a card is
  // two rows of its `stages`, and the one that reviews says a card that
  // did not pass goes to `ready`, where the other takes cards from. With
  // nobody to work, a card sent there stays to be seen.
  await close();
  const stages = [
    { title: 'work', column: 'ready', does: 'work', role: 'worker', pass: 'review' },
    { title: 'review', column: 'review', does: 'review', role: 'reviewer', pass: 'done', fail: 'ready' },
  ];
  await cli('project', 'create', '--bare', 'Depot');
  await cli('apply', 'std/board', '--project', 'depot');
  await cli('apply', 'std/fleet', '--project', 'depot', '--param', 'project=depot', '--param', 'agents=[]', '--param', `stages=${JSON.stringify(stages)}`);
  await cli('board', 'add', 'depot', 'Weigh the parcels');
  await cli('board', 'add', 'depot', 'Print the labels');
  await cli('board', 'move', 'depot-1', 'review');
  await cli('board', 'move', 'depot-2', 'ready');
  await side(page, 'Resources');
  await page.locator('.nav').getByText('Depot', { exact: true }).click();
  await page.getByRole('heading', { name: 'Depot', level: 1 }).waitFor();
  await page.locator('.nav [data-project="depot"]').getByText('Board', { exact: true }).click();
  await page.getByRole('heading', { name: 'Depot board', level: 1 }).waitFor();
  const parcels = page.locator('.card', { hasText: 'Weigh the parcels' });
  await column('review').locator('.card', { hasText: 'Weigh the parcels' }).waitFor();
  const goes = drawer.locator('.send-back .faint');

  // A card where a stage works its cards is not one that is being looked over.
  await page.locator('.card', { hasText: 'Print the labels' }).click();
  await drawer.getByLabel('Title').waitFor();
  expect('no Send back on a card that waits to be worked', (await send.count()) === 0);
  await close();

  // The one in review says where it would go, and goes there: not to doing.
  await parcels.click();
  await box.waitFor();
  expect(`the drawer to say where the stage sends it, got "${await goes.innerText()}"`, (await goes.innerText()) === 'It goes back to ready, where it is taken up again.');
  await box.fill('The scale reads in pounds.');
  await page.screenshot({ path: `${out}/review-13-where-its-stage-says.png` });
  await send.click();
  await column('ready').locator('.card', { hasText: 'Weigh the parcels' }).waitFor();
  await drawer.waitFor({ state: 'detached' });
  expect('nothing to have been put in doing', (await column('doing').locator('.card').count()) === 0);
  const sent = (await cli('board', 'history', 'depot-1')).stdout.trim().split('\n').map((line) => line.replace(/^\S+\s+/, '')).at(-1);
  expect(`the history to say where it was sent back to, got "${sent}"`, sent === 'Ada moved it from review to ready, saying: The scale reads in pounds.');
  await page.screenshot({ path: `${out}/review-14-sent-where-it-says.png` });

  // The row is changed to say another column, and the page follows: a
  // card opened after that is sent there. Nothing is read on a timer; the
  // change of the row is an event.
  const reviews = JSON.parse((await cli('record', 'list', 'depot', 'stages', '--json')).stdout).records.find((row) => row.data.does === 'review');
  await cli('record', 'set', reviews.id, 'fail=backlog');
  await cli('board', 'move', 'depot-1', 'review');
  await column('review').locator('.card', { hasText: 'Weigh the parcels' }).waitFor();
  for (let tries = 0; ; tries += 1) {
    await parcels.click();
    await box.waitFor();
    if ((await goes.innerText()) === 'It goes back to backlog, where it is taken up again.') break;
    if (tries === 50) throw new Error(`expected the drawer to follow the row, and it says "${await goes.innerText()}"`);
    await close();
    await page.waitForTimeout(100);
  }
  await send.click();
  await column('backlog').locator('.card', { hasText: 'Weigh the parcels' }).waitFor();

  // A row that names no column for it leaves it as it always was: back to doing.
  await cli('record', 'set', reviews.id, 'fail=');
  await cli('board', 'move', 'depot-1', 'review');
  await column('review').locator('.card', { hasText: 'Weigh the parcels' }).waitFor();
  for (let tries = 0; ; tries += 1) {
    await parcels.click();
    await box.waitFor();
    if ((await goes.innerText()) === 'It goes back to doing, where it is taken up again.') break;
    if (tries === 50) throw new Error(`expected the drawer to follow the row, and it says "${await goes.innerText()}"`);
    await close();
    await page.waitForTimeout(100);
  }
  await close();

  // ---- a card left for a person says why where the person looks -------------------
  // A project as it is added, its fleet on the assistant that is built
  // in, which takes a card and cannot work it: the round leaves the card
  // in doing and keeps why on its row. Opened, the card says so, with the
  // reason alone and where it is taken up again from; the reason names the
  // row whose cell gives that agent a program that can.
  await cli('project', 'create', 'Yard');
  await cli('board', 'add', 'yard', 'Sweep the yard');
  await cli('board', 'move', 'yard-1', 'ready');
  await side(page, 'Resources');
  await page.locator('.nav [data-project="yard"]').getByText('Board', { exact: true }).click();
  await page.getByRole('heading', { name: 'Yard board', level: 1 }).waitFor();
  const sweep = page.locator('.card', { hasText: 'Sweep the yard' });
  await column('doing').locator('.card', { hasText: 'Sweep the yard' }).waitFor();
  const leftSays = drawer.getByRole('group', { name: 'Left for a person' });
  let why = '';
  for (let tries = 0; ; tries += 1) {
    await sweep.click();
    await drawer.getByLabel('Title').waitFor();
    // The row is written a moment after the card is in doing: the page follows it, and is asked again until it has.
    if ((await leftSays.count()) === 1) {
      why = (await leftSays.innerText()).replace(/\s+/g, ' ').trim();
      break;
    }
    if (tries === 50) throw new Error('expected a card its round left to say so when it is opened');
    await close();
    await page.waitForTimeout(100);
  }
  expect(`the card to say that it was left and where it is taken up again from, got "${why}"`, why.startsWith('Nobody is working this card: it was left here for a person, and is taken up again when it is put back in ready '));
  expect(`the reason to be the innermost, with no step named, got "${why}"`, why.includes('ready yard-worker-1 could not do it: the built-in assistant cannot do this work; ') && !why.includes('step `'));
  expect(`the reason to name the row that gives that agent a program, got "${why}"`, why.includes('record set yard-agents-2 adapter=<package>'));
  expect('what was kept for the fleet not to be offered for filling in', (await drawer.getByLabel('Again').count()) === 0);
  const leftBox = await boxOf(leftSays);
  const drawerBox = await boxOf(drawer);
  expect(`why it was left to be inside the drawer: ${JSON.stringify(leftBox)} in ${JSON.stringify(drawerBox)}`, leftBox.x >= drawerBox.x && leftBox.x + leftBox.width <= drawerBox.x + drawerBox.width + 1);
  await page.screenshot({ path: `${out}/review-15-left-for-a-person.png` });
  await close();
  // Moved on by a person, it has been seen to, and says nothing of having been left.
  await cli('board', 'move', 'yard-1', 'review');
  await column('review').locator('.card', { hasText: 'Sweep the yard' }).waitFor();
  await sweep.click();
  await box.waitFor();
  expect('a card a person has moved since to say nothing of having been left', (await leftSays.count()) === 0);
  await close();

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
