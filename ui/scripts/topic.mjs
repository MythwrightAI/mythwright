// What a channel is for, under what it is called in its head, as `channel.mjs`
// holds it: that check's own part, in a file of its own because the check
// was near its limit of lines. It starts no daemon and no browser: it is
// handed the page `channel.mjs` has open and the command line of its
// factory.
//
// Three channels are made, each called by seven letters and with nobody
// but whoever made it in it, so that their heads differ by nothing but what
// each is for: one that says more than two lines hold, one that says little
// and one that says nothing. Beside the last, each of the others is held by
// the boxes at three widths and a phone's: what it
// is for is under what it is called and under everything else of the
// head's first line, inside the head, smaller and fainter than the title,
// over nothing, and two lines at most; the long one cut there and said
// whole in the client's tip as the pointer rests on it, the short one whole
// and saying nothing more; nothing at the head's right (who is in it, and
// Tools) a pixel from where it stands in the one
// that says nothing; and that one drawing no line for it. Where it is cut it
// is a button, and plain words where it is not: opened whole and cut back by
// the keyboard at the usual width and by a finger at a phone's (it opens
// itself, and is not the browser's tip there), it moves nothing of the
// head, pushes down only what is under the head, and cut back every box is
// where it was. It starts no browser, but makes the phone a context of its
// own, which says it is touched.

import { over } from './attaching.mjs';
import { sideSettled } from './drawer.mjs';

const LONG =
  "Where the project's cards are, said by the factory's bot every ten minutes: what was taken up, what was handed on, what waits for somebody, what failed and why, and who has what in hand just now, so that nobody has to go and look at the board to know how the work stands; and once a day, what was done since the day before, card by card, with what each cost and how many times it was sent back before it passed";
const SHORT = 'Where releases are said';

/** Turns the side panel to a view on the rail, unless it is showing already. */
async function side(page, name) {
  const button = page.getByRole('button', { name, exact: true });
  if ((await button.getAttribute('aria-pressed')) !== 'true') await button.click();
}

/** The head of the channel showing, by its boxes. */
const measured = (page) =>
  page.locator('.channel > .split > .view-head').evaluate((head) => {
    const box = (one) => {
      if (!one) return null;
      const b = one.getBoundingClientRect();
      return { x: b.left, y: b.top, right: b.right, bottom: b.bottom };
    };
    const topic = head.querySelector('.channel-topic');
    const words = topic?.querySelector('.topic-words');
    const faint = document.createElement('span');
    faint.style.color = 'var(--faint)';
    head.append(faint);
    const faintColour = getComputedStyle(faint).color;
    faint.remove();
    const style = topic && getComputedStyle(topic);
    const high = words ? parseFloat(getComputedStyle(words).lineHeight) : 0;
    return {
      head: box(head),
      title: { ...box(head.querySelector('h1')), size: parseFloat(getComputedStyle(head.querySelector('h1')).fontSize) },
      right: {
        participants: box(head.querySelector('.participants')),
        tools: box(head.querySelector('.context-fold')),
      },
      others: [...head.children].filter((one) => one !== topic && one.getBoundingClientRect().width > 0).map((one) => ({ ...box(one), of: one.className || one.tagName })),
      under: box(head.parentElement.querySelector(':scope > .messages')),
      topic: topic && {
        ...box(topic),
        text: topic.textContent,
        tag: topic.tagName,
        open: topic.getAttribute('aria-expanded'),
        focused: document.activeElement === topic,
        size: parseFloat(style.fontSize),
        lineHeight: high,
        whole: words.scrollHeight,
        colour: style.color,
        faint: faintColour,
        cut: words.scrollHeight > 2 * high + 1,
      },
      sideways: document.scrollingElement.scrollWidth > document.scrollingElement.clientWidth,
    };
  });

export async function topic({ page, out, expect, cli, who }) {
  await cli('channel', 'create', 'updates', '--topic', LONG);
  await cli('channel', 'create', 'release', '--topic', SHORT);
  await cli('channel', 'create', 'nothing');
  const usual = page.viewportSize();
  const widths = [usual.width, 1100, 880, 390];
  const open = async (name) => {
    await page.setViewportSize(usual);
    await side(page, 'Channels');
    await page.getByRole('navigation', { name: 'Channels' }).getByRole('button', { name: `# ${name}`, exact: true }).click();
    await page.getByRole('heading', { name: `# ${name}` }).waitFor();
  };
  const atWidths = async (name, also) => {
    const at = {};
    await open(name);
    for (const width of widths) {
      await page.setViewportSize({ width, height: usual.height });
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
      at[width] = await measured(page);
      if (also) await also(width, at[width]);
    }
    await page.setViewportSize(usual);
    return at;
  };
  const tipFor = async (box, picture) => {
    await page.mouse.move((box.x + box.right) / 2, (box.y + box.bottom) / 2);
    const tip = page.getByRole('tooltip');
    // The client waits as long as the system's own tip does before it says anything.
    await page.waitForTimeout(700);
    const said = (await tip.count()) ? await tip.innerText() : null;
    if (said) await page.screenshot({ path: picture });
    await page.mouse.move(2, usual.height - 2);
    return said;
  };

  const plain = await atWidths('nothing', (width) => page.screenshot({ path: `${out}/channel-22-topic-none-${width}.png` }));
  for (const width of widths) expect(`at ${width} wide: a channel that says nothing of what it is for to draw no line for it`, plain[width].topic === null);

  for (const [name, said, cut] of [
    ['updates', LONG, true],
    ['release', SHORT, false],
  ]) {
    const tips = {};
    const at = await atWidths(name, async (width, now) => {
      await page.screenshot({ path: `${out}/channel-22-topic-${name}-${width}.png` });
      if (now.topic) tips[width] = await tipFor(now.topic, `${out}/channel-22-topic-${name}-${width}-pointed.png`);
    });
    for (const width of widths) {
      const of = `#${name} at ${width} wide`;
      const { head, title, right, others, topic: line } = at[width];
      expect(`${of}: what it is for to be drawn`, !!line && line.text === said);
      expect(`${of}: what it is for to be under the title and under all of the head's first line, got ${JSON.stringify(line)} under ${JSON.stringify(others)}`, others.every((one) => line.y >= one.bottom - 0.5));
      expect(`${of}: what it is for to begin where the title does, got ${line.x} and ${title.x}`, Math.abs(line.x - title.x) <= 0.5);
      expect(`${of}: what it is for to be inside the head, got ${JSON.stringify(line)} in ${JSON.stringify(head)}`, line.x >= head.x - 0.5 && line.right <= head.right + 0.5 && line.bottom <= head.bottom + 0.5);
      expect(`${of}: what it is for to lie over nothing of the head`, others.every((one) => !over(one, line)));
      expect(`${of}: what it is for to be smaller than the title, got ${line.size} and ${title.size}`, line.size < title.size);
      expect(`${of}: what it is for to be in the faint colour, got ${line.colour} for ${line.faint}`, line.colour === line.faint);
      expect(`${of}: two lines at most, got ${line.bottom - line.y} for lines of ${line.lineHeight}`, line.bottom - line.y <= 2 * line.lineHeight + 1);
      expect(`${of}: ${cut ? '' : 'not '}to be cut`, line.cut === cut);
      expect(`${of}: to be ${cut ? 'a button that opens it, closed' : 'plain words'}, got ${line.tag} ${line.open}`, cut ? line.tag === 'BUTTON' && line.open === 'false' : line.tag === 'P' && line.open === null);
      expect(`${of}: the page not to scroll sideways`, !at[width].sideways);
      for (const [what, box] of Object.entries(right)) {
        const was = plain[width].right[what];
        expect(`${of}: ${what} at the head's right to be there`, !!box && !!was);
        expect(`${of}: ${what} at the head's right not to be moved, got ${JSON.stringify(box)} where #nothing has ${JSON.stringify(was)}`, Math.abs(box.y - was.y) <= 0.5 && Math.abs(box.bottom - was.bottom) <= 0.5);
      }
      expect(`${of}: Tools to end where it does in #nothing`, Math.abs(right.tools.right - plain[width].right.tools.right) <= 0.5);
      expect(`${of}: the title to stand where it does in #nothing`, Math.abs(title.y - plain[width].title.y) <= 0.5);
      if (cut) expect(`${of}: the whole of it to be said as the pointer rests on it, got ${JSON.stringify(tips[width])}`, tips[width] === said);
      else expect(`${of}: nothing to be said as the pointer rests on it, got ${JSON.stringify(tips[width])}`, tips[width] === null);
    }
  }

  // ---- opened whole, and cut back ----------------------------------------------
  // What it is for, where it is cut, is opened whole and cut back by its own press: by the keyboard at the usual width
  // and by a finger on a phone. Opened, what is under the head goes down as much as the head grows, and nothing of the
  // head moves; cut back, every box is where it was.
  const same = (a, b) => Math.abs(a.x - b.x) <= 0.5 && Math.abs(a.y - b.y) <= 0.5 && Math.abs(a.right - b.right) <= 0.5 && Math.abs(a.bottom - b.bottom) <= 0.5;
  const heldOpen = (said, was, now) => {
    const line = now.topic;
    expect(`${said}: it to say it is open, got ${line.open}`, line.open === 'true');
    expect(`${said}: its words to be whole, ${line.bottom - line.y} tall for ${line.whole}, more than two lines of ${line.lineHeight}`, line.bottom - line.y >= line.whole - 1 && line.whole > 2 * line.lineHeight + 1);
    expect(`${said}: it to be inside the head and over nothing`, line.right <= now.head.right + 0.5 && line.bottom <= now.head.bottom + 0.5 && now.others.every((one) => !over(one, line)));
    for (const [what, box] of Object.entries(now.right)) expect(`${said}: ${what} at the head's right not to be moved, got ${JSON.stringify(box)} for ${JSON.stringify(was.right[what])}`, same(box, was.right[what]));
    expect(`${said}: the title not to be moved`, same(now.title, was.title));
    expect(`${said}: the head's top not to be moved`, Math.abs(now.head.y - was.head.y) <= 0.5);
    const grown = now.head.bottom - was.head.bottom;
    expect(`${said}: the head to grow by what was cut, ${grown}`, grown > 0 && Math.abs(grown - (line.bottom - was.topic.bottom)) <= 1);
    expect(`${said}: what is under the head to go down as much, ${now.under.y - was.under.y} for ${grown}`, Math.abs(now.under.y - was.under.y - grown) <= 1);
    expect(`${said}: the page not to scroll sideways`, !now.sideways);
  };
  const heldBack = (said, was, now) => {
    expect(`${said}: it to say it is cut again, got ${now.topic.open}`, now.topic.open === 'false');
    for (const [what, box] of [['the head', now.head], ['the title', now.title], ['what it is for', now.topic], ['what is under the head', now.under], ...Object.entries(now.right)]) {
      const before = what === 'the head' ? was.head : what === 'the title' ? was.title : what === 'what it is for' ? was.topic : what === 'what is under the head' ? was.under : was.right[what];
      expect(`${said}: ${what} to be where it was, got ${JSON.stringify(box)} for ${JSON.stringify(before)}`, same(box, before));
    }
  };

  await open('updates');
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const folded = await measured(page);
  const line = page.locator('.channel > .split > .view-head > .channel-topic');
  await line.focus();
  await page.keyboard.press('Enter');
  await page.locator('.channel > .split > .view-head > .channel-topic[aria-expanded="true"]').waitFor();
  const opened = await measured(page);
  heldOpen(`at ${usual.width} wide by the keyboard`, folded, opened);
  expect('the keyboard to stay on it as it opens', opened.topic.focused);
  await page.screenshot({ path: `${out}/channel-22-topic-updates-opened-${usual.width}.png` });
  await page.keyboard.press('Enter');
  await page.locator('.channel > .split > .view-head > .channel-topic[aria-expanded="false"]').waitFor();
  heldBack(`at ${usual.width} wide by the keyboard`, folded, await measured(page));

  // A phone, touched: a context of its own that says it is touched.
  const browser = page.context().browser();
  const touched = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2, colorScheme: process.env.SCHEME ?? 'dark' });
  try {
    const phone = await touched.newPage();
    await phone.goto(new URL(page.url()).origin);
    await phone.getByLabel('Email').fill(who.email);
    await phone.getByLabel('Password').fill(who.password);
    await phone.getByRole('button', { name: 'Sign in', exact: true }).last().tap();
    await phone.locator('.panes').waitFor();
    await phone.getByRole('button', { name: 'Expand the side panel' }).tap();
    await sideSettled(phone);
    await phone.locator('.sidebar').waitFor();
    const channels = phone.locator('.rail').getByRole('button', { name: 'Channels', exact: true });
    if ((await channels.getAttribute('aria-pressed')) !== 'true') await channels.tap();
    await phone.getByRole('navigation', { name: 'Channels' }).getByRole('button', { name: '# updates', exact: true }).tap();
    await phone.getByRole('heading', { name: '# updates' }).waitFor();
    await phone.locator('.sidebar').waitFor({ state: 'detached' });
    await phone.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    expect('the phone to be taken for something touched', await phone.evaluate(() => matchMedia('(pointer: coarse)').matches));
    const before = await measured(phone);
    expect(`at 390 wide, touched: what it is for to be a button that opens it, closed, got ${before.topic?.tag} ${before.topic?.open}`, before.topic?.tag === 'BUTTON' && before.topic.open === 'false' && before.topic.cut);
    const touchedLine = phone.locator('.channel > .split > .view-head > .channel-topic');
    await touchedLine.tap();
    await phone.locator('.channel > .split > .view-head > .channel-topic[aria-expanded="true"]').waitFor();
    heldOpen('at 390 wide, touched', before, await measured(phone));
    await phone.screenshot({ path: `${out}/channel-22-topic-updates-opened-390-touched.png` });
    await touchedLine.tap();
    await phone.locator('.channel > .split > .view-head > .channel-topic[aria-expanded="false"]').waitFor();
    heldBack('at 390 wide, touched', before, await measured(phone));
    await phone.screenshot({ path: `${out}/channel-22-topic-updates-cut-390-touched.png` });
  } finally {
    await touched.close();
  }
}
