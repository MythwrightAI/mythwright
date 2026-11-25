// What is attached to a message in a channel, as `channel.mjs` holds it:
// that check's own part, in a file of its own because the check was at its
// limit of lines. It starts no daemon and no browser: it is handed the page
// `channel.mjs` has open on #general (with a person, an agent, a bot and
// the factory's own bot, `conductor`, and a thread under the message
// `asked`), and how that check says what it expected.
//
// A picture and a document are chosen with the composer's button, and are
// shown waiting over the box and outside it, flush with its left edge, a
// picture as a small one of itself: nothing on the page moves as they come
// and go, what is written keeps its room, and the last thing said can be
// scrolled up clear of them. One is taken off. A file let go over the box
// is attached, and so is what a real paste carries: a PNG, a BMP (which is
// drawn again as a PNG, as what WSLg hands over from Windows may be), and
// a picture that is only HTML holding it in itself, each called by when it
// was pasted where it had no name of its own. A picture named only by an
// address somewhere else is fetched by nobody and said, in a line under the
// box that goes with the next key, not to have been on the clipboard; so
// is a paste that brought nothing. With what the bot takes offered and what
// waits over the box, the two and the line that says who is writing stand
// one over another, none over another, at three widths and a phone's, and
// the line begins where the names of whoever said something do. The same
// over a thread's box, which has such a line too. Sent, they are part of the message; and a
// message that is only a picture is sent.

import { toolsAtRest } from './drawer.mjs';

/** A picture one pixel square, as a PNG. */
export const DOT = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';

/** A picture one pixel square, as a BMP: what no browser shows, and what a Windows clipboard may hand over. */
export const BMP = 'Qk06AAAAAAAAADYAAAAoAAAAAQAAAAEAAAABABgAAAAAAAQAAAATCwAAEwsAAAAAAAAAAAAA/wAAAA==';

/**
 * Says a paste into `area` (a locator of a box one writes in), carrying
 * what is described: files by name, type and the base 64 of their bytes,
 * and what the clipboard says as each kind of text.
 */
export async function paste(area, { files = [], said = {} }) {
  await area.evaluate(
    (box, { files, said }) => {
      const copied = new DataTransfer();
      for (const one of files) copied.items.add(new File([Uint8Array.from(atob(one.base64), (c) => c.charCodeAt(0))], one.name, { type: one.type }));
      for (const [kind, text] of Object.entries(said)) copied.setData(kind, text);
      box.focus();
      box.dispatchEvent(new ClipboardEvent('paste', { clipboardData: copied, bubbles: true, cancelable: true }));
    },
    { files, said },
  );
}

/** Where each of these is on the page, rounded to a tenth of a pixel; nothing for one that is not there. */
export const boxes = (page, selectors) =>
  page.evaluate((selectors) => {
    const at = (selector) => {
      const one = document.querySelector(selector);
      if (!one) return null;
      const box = one.getBoundingClientRect();
      const round = (figure) => Math.round(figure * 10) / 10;
      return { x: round(box.left), y: round(box.top), width: round(box.width), height: round(box.height), right: round(box.right), bottom: round(box.bottom) };
    };
    return Object.fromEntries(Object.entries(selectors).map(([name, selector]) => [name, at(selector)]));
  }, selectors);

/** Whether two boxes lie over each other by more than half a pixel. */
export const over = (a, b) => !!a && !!b && a.x < b.right - 0.5 && b.x < a.right - 0.5 && a.y < b.bottom - 0.5 && b.y < a.bottom - 0.5;

const WHERE = {
  box: '.channel > .split .composer-box',
  written: '.channel > .split .composer-box textarea',
  strip: '.channel > .split .composer-over > .attaching',
  offered: '.mentions.offered',
  typing: '.channel > .split .typing-edge',
  list: '.channel > .split .messages',
  last: '.channel > .split .messages > .message:last-child',
  note: '.channel > .split .composer-note',
};

/**
 * Where the words of the line that says who is writing begin in `scope` (the channel's own part, or a thread), and
 * where the name of whoever said something begins there: the line's own left with its padding, the first of its dots
 * where it has any, and the first author's name in its list. Nothing where either is not there.
 */
export const typingBegins = (page, scope) =>
  page.evaluate((scope) => {
    const line = document.querySelector(`${scope} .typing-edge`);
    const name = document.querySelector(`${scope} .messages .message .who`);
    if (!line || !name) return null;
    const box = line.getBoundingClientRect();
    const style = getComputedStyle(line);
    return {
      begins: box.left + parseFloat(style.borderLeftWidth) + parseFloat(style.paddingLeft),
      dots: line.querySelector('.typing-dots')?.getBoundingClientRect().left ?? null,
      name: name.getBoundingClientRect().left,
    };
  }, scope);

/** That the line of who is writing in `scope` begins where the names of whoever said something there do, to a pixel. */
export async function beginsWithNames(page, expect, scope, said) {
  const at = await typingBegins(page, scope);
  expect(
    `${said}: the line of who is writing to begin where a name of whoever said something does, got ${JSON.stringify(at)}`,
    !!at && Math.abs(at.begins - at.name) <= 1 && (at.dots === null || Math.abs(at.dots - at.name) <= 1),
  );
  return at;
}

/** That what waits stands over the box, outside it and flush with its left edge, and that it, the bot's list and the line of who is writing lie over nothing of each other's or of the box. */
export async function standOneOverAnother(page, expect, said) {
  const at = await boxes(page, WHERE);
  const { width } = page.viewportSize();
  expect(`${said}: what waits to be over the box and outside it, got ${JSON.stringify(at.strip)} over ${JSON.stringify(at.box)}`, at.strip && at.strip.bottom <= at.box.y + 0.5);
  expect(`${said}: what waits to begin where the box does, got ${at.strip?.x} and ${at.box.x}`, Math.abs(at.strip.x - at.box.x) <= 0.5);
  expect(`${said}: what waits to be in the window, got ${JSON.stringify(at.strip)}`, at.strip.x >= 0 && at.strip.right <= width + 0.5 && at.strip.y >= 0);
  expect(`${said}: the line of who is writing to be between what waits and the box, got ${JSON.stringify(at.typing)}`, at.strip.bottom <= at.typing.y + 0.5 && at.typing.bottom <= at.box.y + 0.5);
  const named = ['box', 'strip', 'offered', 'typing', 'note'].filter((name) => at[name]);
  for (const a of named) for (const b of named) if (a < b) expect(`${said}: ${a} and ${b} not to lie over each other, got ${JSON.stringify(at[a])} and ${JSON.stringify(at[b])}`, !over(at[a], at[b]));
  // Nothing in the strip over anything else in it.
  const parts = await page.locator(`${WHERE.strip} > *`).evaluateAll((all) => all.map((one) => one.getBoundingClientRect()).map((b) => ({ x: b.left, y: b.top, right: b.right, bottom: b.bottom })));
  for (let i = 0; i < parts.length; i++) for (let j = i + 1; j < parts.length; j++) expect(`${said}: two of what waits not to lie over each other`, !over(parts[i], parts[j]));
  expect(`${said}: the page not to scroll sideways`, await page.evaluate(() => document.scrollingElement.scrollWidth <= document.scrollingElement.clientWidth));
  await beginsWithNames(page, expect, '.channel > .split', said);
  return at;
}

export async function attaching({ page, out, expect, composer, inChannel, asked }) {
  const picture = { name: 'board.png', mimeType: 'image/png', buffer: Buffer.from(DOT, 'base64') };
  const notes = { name: 'notes.txt', mimeType: 'text/plain', buffer: Buffer.from('remember the mugs\n') };
  const area = composer.getByLabel('Message');
  // A picture that is waiting is a small one of itself, said by its name; anything else is its name and its size.
  const waiting = async () => [
    ...(await composer.locator('.attaching-picture img').evaluateAll((all) => all.map((one) => one.alt))),
    ...(await composer.locator('.attaching-one').allInnerTexts()).map((one) => one.replace(/\s+/g, ' ').replace(/ ×$/, '').trim()),
  ];
  const requests = [];
  page.on('request', (request) => requests.push(request.url()));
  await area.fill('');
  expect('nothing to send with nothing written and nothing attached', await composer.getByRole('button', { name: 'Send' }).isDisabled());
  const still = await boxes(page, WHERE);
  await composer.getByLabel('Files to attach').setInputFiles([picture, notes]);
  await composer.locator('.attaching-one', { hasText: 'notes.txt' }).waitFor();
  expect(`what was chosen to be waiting over the composer, got ${await waiting()}`, (await waiting()).join() === 'board.png,notes.txt 18 B');
  {
    const small = await composer.locator('.attaching-picture').boundingBox();
    expect(`the picture to be a small one of itself, got ${JSON.stringify(small)}`, small.width <= 60 && small.height <= 60);
    const now = await standOneOverAnother(page, expect, 'with something waiting');
    // Nothing the reader looks at moved as it came: the box, what is written, the line of who is writing, the list and what was said last.
    for (const name of ['box', 'written', 'typing', 'list', 'last'])
      expect(`the ${name} not to move or change size as what waits came, was ${JSON.stringify(still[name])}, is ${JSON.stringify(now[name])}`, JSON.stringify(still[name]) === JSON.stringify(now[name]));
    // The last thing said can be scrolled up clear of what waits.
    await page.locator(WHERE.list).evaluate((list) => (list.scrollTop = list.scrollHeight));
    const scrolled = await boxes(page, WHERE);
    expect(`the last thing said to be scrolled up clear of what waits, it ends at ${scrolled.last.bottom}, what waits begins at ${scrolled.strip.y}`, scrolled.last.bottom <= scrolled.strip.y + 0.5);
  }
  await page.screenshot({ path: `${out}/channel-9-attaching.png` });
  await composer.getByRole('button', { name: 'Remove notes.txt' }).click();
  expect(`one taken off not to be waiting, got ${await waiting()}`, (await waiting()).join() === 'board.png');
  // A file let go over the composer is attached too.
  await composer.locator('.composer-box').evaluate((box) => {
    const carried = new DataTransfer();
    carried.items.add(new File(['remember the mugs\n'], 'notes.txt', { type: 'text/plain' }));
    box.dispatchEvent(new DragEvent('drop', { dataTransfer: carried, bubbles: true, cancelable: true }));
  });
  await composer.locator('.attaching-one', { hasText: 'notes.txt' }).waitFor();

  // A paste: a file by its own name, a picture with none (it and its address both on the clipboard, as a
  // browser copies a picture), a BMP, and a picture that is only HTML holding it in itself.
  await paste(area, { files: [{ name: 'pasted.csv', type: 'text/csv', base64: btoa('x,y\n1,2\n') }] });
  await composer.locator('.attaching-one', { hasText: 'pasted.csv' }).waitFor();
  await paste(area, { files: [{ name: 'image.png', type: 'image/png', base64: DOT }], said: { 'text/plain': 'https://example.test/cat.png', 'text/html': '<img src="https://example.test/cat.png">' } });
  await paste(area, { files: [{ name: 'image.bmp', type: 'image/bmp', base64: BMP }] });
  await paste(area, { said: { 'text/html': `<meta charset="utf-8"><img src="data:image/png;base64,${DOT}">` } });
  await page.waitForFunction(() => document.querySelectorAll('.channel > .split .attaching-picture').length === 4 && !document.querySelector('.channel > .split .attaching > .faint'));
  const pictures = (await waiting()).slice(0, 4);
  expect(`each picture pasted with no name to be called by when it was pasted, and the BMP to go as a PNG, got ${pictures}`, pictures[0] === 'board.png' && pictures.slice(1).every((name) => /^pasted-\d{8}-\d{6}(-\d+)?\.png$/.test(name)));
  expect(`none of what the clipboard said as text to be put in, got "${await area.inputValue()}"`, (await area.inputValue()) === '');
  expect('no note under the box for what was attached', (await page.locator(WHERE.note).count()) === 0);
  // The BMP was drawn again as a PNG in the page, and reads as one.
  const drawn = await composer.locator('.attaching-picture img').nth(2).evaluate((img) => (img.complete ? img.naturalWidth : new Promise((done) => (img.onload = () => done(img.naturalWidth)))));
  expect(`the BMP to be a picture one pixel wide once drawn again, got ${drawn}`, drawn === 1);
  await standOneOverAnother(page, expect, 'with four pictures and two files waiting');

  // A picture named only by where it is on somebody's page: nothing is fetched, nothing attached, and the line under the box says so.
  const ahead = requests.length;
  await paste(area, { said: { 'text/plain': 'https://example.test/cat.png', 'text/html': '<img src="https://example.test/cat.png" alt="a cat">' } });
  const note = page.locator(WHERE.note);
  await note.waitFor();
  expect(`the line to say the picture was not on the clipboard, got "${await note.innerText()}"`, /not on the clipboard/.test(await note.innerText()) && /Copy image/.test(await note.innerText()));
  {
    const at = await boxes(page, WHERE);
    expect(`the line to be under the box and in the window, got ${JSON.stringify(at.note)} under ${JSON.stringify(at.box)}`, at.note.y >= at.box.bottom - 0.5 && at.note.bottom <= page.viewportSize().height);
    for (const name of ['box', 'written', 'typing', 'list']) expect(`the ${name} not to move for the line, was ${JSON.stringify(still[name])}`, JSON.stringify(still[name]) === JSON.stringify(at[name]));
  }
  expect('nothing to have been asked of anywhere for it', requests.slice(ahead).every((one) => !one.includes('example.test')));
  expect(`nothing more to be waiting, got ${await waiting()}`, (await waiting()).length === 6);
  await page.screenshot({ path: `${out}/channel-9-attaching-not-there.png` });
  await area.press('ArrowRight');
  await note.waitFor({ state: 'detached' });
  // A paste that brought nothing at all says so, the same way.
  await paste(area, {});
  await note.waitFor();
  expect(`the line to say nothing came, got "${await note.innerText()}"`, /^Nothing came with that paste/.test(await note.innerText()));
  await area.press('ArrowRight');
  await note.waitFor({ state: 'detached' });

  // With what the bot takes offered too, the three stand one over another.
  await area.click();
  await page.keyboard.type('@conductor ');
  await page.getByRole('listbox', { name: 'What conductor takes' }).waitFor();
  await standOneOverAnother(page, expect, 'with what the bot takes offered');
  await page.screenshot({ path: `${out}/channel-9-attaching-offered.png` });
  // And so at three widths and a phone's.
  const usual = page.viewportSize();
  for (const width of [1100, 880, 390]) {
    await page.setViewportSize({ width, height: usual.height });
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    await standOneOverAnother(page, expect, `at ${width} wide with what the bot takes offered`);
    if (width === 390) {
      await page.screenshot({ path: `${out}/channel-9-attaching-phone.png` });
      // On a phone under the box is the screen's edge, and the line is said over what waits instead.
      await area.fill('');
      await page.keyboard.press('Escape');
      await paste(area, { said: { 'text/plain': 'https://example.test/cat.png', 'text/html': '<img src="https://example.test/cat.png">' } });
      await note.waitFor();
      const at = await standOneOverAnother(page, expect, 'on a phone with the line a paste said');
      expect(`the line to be over what waits on a phone, got ${JSON.stringify(at.note)} over ${JSON.stringify(at.strip)}`, at.note.bottom <= at.strip.y + 0.5 && at.note.y >= 0);
      await page.screenshot({ path: `${out}/channel-9-attaching-phone-note.png` });
      await area.press('ArrowRight');
      await note.waitFor({ state: 'detached' });
    }
  }
  await page.setViewportSize(usual);
  await area.fill('');
  await page.keyboard.press('Escape');

  // A thread's box the same: what waits over it and outside it, what is written as tall as it was.
  {
    await asked.getByRole('button', { name: '2 replies' }).click();
    const thread = page.locator('.thread');
    const reply = thread.getByLabel('Reply');
    await reply.waitFor();
    // The panel the thread is read in comes in from the channel's edge as it is opened: measured once it is at rest.
    await toolsAtRest(page).waitFor();
    const before = await boxes(page, { box: '.thread .composer-box', written: '.thread .composer-box textarea', typing: '.thread .typing-edge' });
    // The thread says who is writing in it as the channel does: a line over its box, beginning where the names do.
    expect(`the line of who is writing in the thread to be over its box, got ${JSON.stringify(before.typing)} over ${JSON.stringify(before.box)}`, !!before.typing && before.typing.bottom <= before.box.y + 0.5);
    await beginsWithNames(page, expect, '.thread', 'in a thread');
    await paste(reply, { files: [{ name: 'image.png', type: 'image/png', base64: DOT }] });
    await thread.locator('.attaching-picture').waitFor();
    const at = await boxes(page, { box: '.thread .composer-box', written: '.thread .composer-box textarea', typing: '.thread .typing-edge', strip: '.thread .composer-over > .attaching', pane: '.thread' });
    expect(`what waits in a thread to be over its box, outside it and where it begins, got ${JSON.stringify(at.strip)} over ${JSON.stringify(at.box)}`, at.strip.bottom <= at.box.y + 0.5 && Math.abs(at.strip.x - at.box.x) <= 0.5 && at.strip.right <= at.pane.right + 0.5);
    expect(`what waits in a thread to lie over the line of who is writing there, got ${JSON.stringify(at.strip)} over ${JSON.stringify(at.typing)}`, at.strip.bottom <= at.typing.y + 0.5 && !over(at.strip, at.typing));
    expect(`the thread's box and its line not to move or change for it, was ${JSON.stringify(before)}`, ['box', 'written', 'typing'].every((name) => JSON.stringify(before[name]) === JSON.stringify(at[name])));
    await page.screenshot({ path: `${out}/channel-9-attaching-thread.png` });
    await thread.getByRole('button', { name: /^Remove pasted-/ }).click();
    expect('nothing to be waiting in the thread once it is taken off', (await thread.locator('.attaching').count()) === 0);
    await page.getByRole('button', { name: 'Close the thread' }).click();
    await page.getByRole('button', { name: 'Collapse the tools panel' }).click();
  }

  // Sent, they are part of the message: the pictures shown, the others by name, to be saved.
  await area.fill('the board as it is');
  await page.keyboard.press('Enter');
  const sent = inChannel.filter({ hasText: 'the board as it is' });
  await sent.locator('.attached-picture img').first().waitFor();
  await page.waitForFunction(() => document.querySelector('.channel > .split .attached-picture img')?.naturalWidth === 1);
  expect('nothing left waiting once it is sent', (await waiting()).length === 0 && (await composer.locator('.attaching').count()) === 0);
  expect(`the four pictures to be shown in the message, got ${await sent.locator('.attached-picture').count()}`, (await sent.locator('.attached-picture').count()) === 4);
  const files = await sent.locator('.attached-file').evaluateAll((all) => all.map((one) => `${one.getAttribute('download')} ${one.getAttribute('href')?.replace(/att_\w+/, 'ID').replace(/\?by=[0-9a-f]{64}$/, '?by=PAGE')}`));
  // Each address says which page asks: a browser saves it by itself, and can send no key with it.
  expect(`the other files to be there by name, each to be saved, got ${files}`, files.join() === 'notes.txt /api/attachments/ID?by=PAGE,pasted.csv /api/attachments/ID?by=PAGE');
  await page.mouse.move(10, 400);
  await page.screenshot({ path: `${out}/channel-10-attached.png` });
  // A picture opens larger where it is.
  await sent.locator('.attached-picture').first().click();
  await page.getByRole('dialog').getByText('board.png').waitFor();
  expect('a way to save the picture', (await page.getByRole('dialog').getByRole('link', { name: 'Save a copy of it' }).getAttribute('download')) === 'board.png');
  // A dialog starts listening for Escape a moment after it is drawn, which
  // is sooner than a person can press it and later than a script does.
  await page.waitForTimeout(200);
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  // With something attached, a message need say nothing: a file, and a picture pasted and sent with Enter.
  await composer.getByLabel('Files to attach').setInputFiles([notes]);
  await composer.locator('.attaching-one', { hasText: 'notes.txt' }).waitFor();
  let said = await inChannel.count();
  await composer.getByRole('button', { name: 'Send' }).click();
  await page.waitForFunction((before) => document.querySelectorAll('.channel > .split .message').length > before, said);
  const silent = inChannel.last();
  expect('a message that is only what is attached', (await silent.locator('.attached-file').count()) === 1 && (await silent.locator('.body').count()) === 0);
  await paste(area, { files: [{ name: 'image.png', type: 'image/png', base64: DOT }] });
  await composer.locator('.attaching-picture').waitFor();
  said = await inChannel.count();
  await area.press('Enter');
  await page.waitForFunction((before) => document.querySelectorAll('.channel > .split .message').length > before, said);
  const only = inChannel.last();
  await only.locator('.attached-picture img').waitFor();
  expect('a message that is only a picture', (await only.locator('.attached-picture').count()) === 1 && (await only.locator('.body').count()) === 0);
}
