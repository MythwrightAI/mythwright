// Who is in a channel, listed under its head, as `channel.mjs` holds it:
// that check's own part, in a file of its own because the check was near
// its limit of lines. It starts no daemon and no browser: it is handed the
// page `channel.mjs` has open on #general and the command line of its
// factory.
//
// An agent the factory does not start, with a name longer than the list is
// wide, is invited and put in the channel. The list is never wider than
// the window, is on the screen and never scrolls sideways; each name has
// what its owner is under it, fainter, and the agent says it is external;
// the long name wraps whole inside the list where it is narrower than
// the name; and nothing of a line lies
// over anything else of it, at three widths and a phone's. The list ends,
// set apart under who is in it, in the one line that adds somebody, which
// names what may be added: whole, inside the list and over nothing, with
// nothing of the channel moved by the list opening; reached by the arrows,
// Home and End and pressed by Enter, the list opened from the keyboard
// having it at the line and Escape giving the keyboard back. Somebody who
// may add only a person is offered the line saying so, and somebody who may
// add nobody has no line: each a page of its own, told what they hold, since
// a person is given less by no route here (`columned.mjs` does the same).
// And behind the @ of the composer, where the long name is offered to be
// asked, the list is as wide as its widest line up to as wide as it may be,
// a name wrapping only there.

import { over } from './attaching.mjs';
import { addLine, addingDialog } from './adding.mjs';

/** Longer than the list is wide at any look, and one word between dashes at a time. */
export const LONG = 'shop-front-services-and-returns-assistant';

/** The channel's head and what is under it, by their boxes, so that what opens over them is seen to move nothing. */
const channelBoxes = (page) =>
  page.evaluate(() => {
    const split = document.querySelector('.channel > .split');
    return [...split.querySelectorAll(':scope > .view-head > *, :scope > .messages')].map((one) => {
      const b = one.getBoundingClientRect();
      return [one.className, Math.round(b.left), Math.round(b.top), Math.round(b.right), Math.round(b.bottom)];
    });
  });

export async function participants({ page, out, expect, cli, who }) {
  await cli('agent', 'invite', LONG, '--project', 'shop');
  await cli('channel', 'add', 'general', LONG);
  const everyone = page.getByRole('button', { name: /participants?$/ });
  const list = page.getByRole('menu', { name: 'Participants' });
  const usual = page.viewportSize();
  for (const width of [usual.width, 1100, 880, 390]) {
    await page.setViewportSize({ width, height: usual.height });
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    const still = await channelBoxes(page);
    await everyone.click();
    await list.getByRole('menuitem', { name: new RegExp(LONG) }).waitFor();
    const moved = await channelBoxes(page);
    expect(`at ${width} wide: nothing of the channel to be moved by the list opening, got ${JSON.stringify(moved)} for ${JSON.stringify(still)}`, JSON.stringify(moved) === JSON.stringify(still));
    const at = await list.evaluate((menu) => {
      const box = (one) => {
        const b = one.getBoundingClientRect();
        return { x: b.left, y: b.top, right: b.right, bottom: b.bottom };
      };
      return {
        menu: { ...box(menu), scroll: menu.scrollWidth, client: menu.clientWidth },
        window: { width: document.documentElement.clientWidth, height: window.innerHeight },
        add: (() => {
          const line = menu.querySelector('.people-add');
          return line && { ...box(line), under: box(menu.querySelector('.people-list')), last: menu.lastElementChild === line, scroll: line.scrollWidth, client: line.clientWidth, edge: getComputedStyle(line).borderTopWidth, text: line.textContent };
        })(),
        lines: [...menu.querySelectorAll('.person')].map((line) => ({
          line: box(line),
          face: box(line.querySelector('.avatar')),
          name: { ...box(line.querySelector('.person-name')), text: line.querySelector('.person-name').textContent, scroll: line.querySelector('.person-name').scrollWidth, client: line.querySelector('.person-name').clientWidth },
          kind: { ...box(line.querySelector('.person-kind')), text: line.querySelector('.person-kind').textContent },
        })),
      };
    });
    const said = `at ${width} wide`;
    expect(`${said}: the list not to scroll sideways, got ${at.menu.scroll} in ${at.menu.client}`, at.menu.scroll <= at.menu.client);
    expect(`${said}: the list to be on the screen and no wider than the window, got ${JSON.stringify(at.menu)} in ${JSON.stringify(at.window)}`, at.menu.x >= -0.5 && at.menu.right <= at.window.width + 0.5 && at.menu.y >= -0.5);
    expect(`${said}: the page not to scroll sideways`, await page.evaluate(() => document.scrollingElement.scrollWidth <= document.scrollingElement.clientWidth));
    for (const one of at.lines) {
      const of = `${said}, ${one.name.text}`;
      expect(`${of}: what they are to be under the name, got ${JSON.stringify(one.kind)} under ${JSON.stringify(one.name)}`, one.kind.y >= one.name.bottom - 0.5 && Math.abs(one.kind.x - one.name.x) <= 0.5);
      expect(`${of}: the face, the name and what they are not to lie over one another`, !over(one.face, one.name) && !over(one.face, one.kind) && !over(one.name, one.kind));
      expect(`${of}: the line's parts to be inside it and inside the list`, [one.face, one.name, one.kind].every((part) => part.x >= one.line.x - 0.5 && part.right <= one.line.right + 0.5 && part.right <= at.menu.right + 0.5));
      expect(`${of}: the name to be whole and not cut, ${one.name.scroll} in ${one.name.client}`, one.name.scroll <= one.name.client + 0.5);
    }
    for (let i = 0; i < at.lines.length; i++) for (let j = i + 1; j < at.lines.length; j++) expect(`${said}: two lines not to lie over each other`, !over(at.lines[i].line, at.lines[j].line));
    const add = at.add;
    expect(`${said}: the list to end in the line that adds all three the owner may, got ${JSON.stringify(add)}`, !!add && add.last && add.text === 'Add a person, agent or bot');
    expect(`${said}: the line to be set apart under who is in it, who scroll above it where they are many, got ${JSON.stringify(add)}`, add.edge !== '0px' && add.y >= add.under.bottom - 0.5);
    expect(`${said}: the line to be inside the list and in view in it, whole and over nothing, got ${JSON.stringify(add)} in ${JSON.stringify(at.menu)}`, add.x >= at.menu.x - 0.5 && add.right <= at.menu.right + 0.5 && add.y >= at.menu.y - 0.5 && add.bottom <= at.menu.bottom + 0.5 && add.scroll <= add.client + 0.5 && !over(add.under, add));
    if (width === 390) expect(`${said}: the line to be a finger's size, got ${add.bottom - add.y}`, add.bottom - add.y >= 40);
    const long = at.lines.find((one) => one.name.text === LONG);
    expect(`${said}: the agent the factory does not start to say so, got "${long?.kind.text}"`, long?.kind.text === 'agent (external)');
    // Where the list is narrower than the name it wraps, whole; where it is not (as wide as a phone's screen) it is one line.
    expect(`${said}: its long name to be inside the list, whole, got ${JSON.stringify(long.name)}`, long.name.right <= at.menu.right + 0.5 && long.name.scroll <= long.name.client + 0.5);
    if (width !== 390) expect(`${said}: its long name to wrap, got ${JSON.stringify(long.name)}`, long.name.bottom - long.name.y > long.kind.bottom - long.kind.y + 4);
    await page.screenshot({ path: `${out}/channel-20-participants-${width}.png` });
    // A menu starts listening for Escape a moment after it is drawn.
    await page.waitForTimeout(250);
    await page.keyboard.press('Escape');
    await list.waitFor({ state: 'detached' });
  }
  await page.setViewportSize(usual);

  // ---- behind the @ ---------------------------------------------------------------------------
  // Those who can be asked, the long name among them: the list is as wide as its widest line up to
  // as wide as it may be, so that a name wraps only where the list is that wide.
  const asking = page.locator('.channel > .split .asking-button');
  await asking.click();
  const askable = page.getByRole('menu', { name: 'Agents and bots in this channel' });
  await askable.getByRole('menuitem', { name: new RegExp(LONG) }).waitFor();
  const asked = await askable.evaluate((menu) => {
    const most = parseFloat(getComputedStyle(menu).maxWidth);
    // How many lines its words take: a line a row of the boxes the text is laid out in.
    const rows = (one) => {
      const range = document.createRange();
      range.selectNodeContents(one);
      const tops = [...range.getClientRects()].filter((rect) => rect.width > 0).map((rect) => rect.top).sort((a, b) => a - b);
      return tops.filter((top, at) => at === 0 || top - tops[at - 1] > 4).length;
    };
    const lines = [...menu.querySelectorAll('.asked-name, .asked-under')].map((one) => ({ text: one.textContent, rows: rows(one) }));
    return { width: menu.getBoundingClientRect().width, most, lines };
  });
  const atMost = asked.width >= asked.most - 1;
  const wrapped = asked.lines.filter((one) => one.rows > 1);
  expect(`behind the @, a line to wrap only where the list is as wide as it may be (${asked.most}px), got ${Math.round(asked.width)}px with ${JSON.stringify(wrapped)}`, atMost || wrapped.length === 0);
  expect(`behind the @, the list to be no wider than it may be, got ${asked.width} for ${asked.most}`, asked.width <= asked.most + 0.5);
  await page.screenshot({ path: `${out}/channel-20-asking-long.png` });
  await asking.click();
  await askable.waitFor({ state: 'detached' });

  // ---- by the keyboard------------------------------------------------------------------------
  const line = addLine(page);
  const inList = () => page.evaluate(() => document.activeElement?.classList.contains('people-add') ?? false);
  await everyone.focus();
  await page.keyboard.press('Enter');
  await line.waitFor();
  await page.waitForFunction(() => document.activeElement?.classList.contains('people-add'), null, { timeout: 2_000 }).catch(() => {});
  expect('the list opened from the keyboard to have it at the line', await inList());
  for (const key of ['ArrowDown', 'ArrowUp', 'End', 'Home']) {
    await page.keyboard.press(key);
    expect(`${key} to keep the keyboard on the line, the one thing there is to press`, await inList());
  }
  await page.waitForTimeout(250);
  await page.keyboard.press('Escape');
  await list.waitFor({ state: 'detached' });
  expect('Escape to give the keyboard back to what opened the list', await everyone.evaluate((one) => document.activeElement === one));
  // Opened by the pointer, the arrows come to the line, and Enter presses it.
  await everyone.click();
  await line.waitFor();
  await page.keyboard.press('ArrowDown');
  expect('the arrows to come to the line', await inList());
  await page.keyboard.press('Enter');
  const dialog = addingDialog(page);
  await dialog.waitFor();
  expect('the list to be put away as the dialog opens', (await list.count()) === 0);
  await page.screenshot({ path: `${out}/channel-20-participants-added-by-keys.png` });
  await page.waitForTimeout(250);
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });

  // ---- somebody who may add less, and somebody who may add nobody --------------------------------
  for (const [holds, said, called] of [
    [['channels', 'projects:read', 'board:read'], 'somebody who may add only a person', 'Add a person'],
    [['channels:read', 'projects:read', 'board:read'], 'somebody who may add nobody', null],
  ]) {
    const context = await page.context().browser().newContext({ viewport: usual, colorScheme: process.env.SCHEME ?? 'dark' });
    try {
      const other = await context.newPage();
      await other.route('**/api/me', async (route) => {
        const answer = await route.fetch();
        const me = await answer.json();
        if (!me.principal) return route.fulfill({ response: answer });
        me.principal.scopes = holds;
        me.can_invite = false;
        await route.fulfill({ response: answer, json: me });
      });
      await other.goto(new URL(page.url()).origin);
      await other.getByLabel('Email').fill(who.email);
      await other.getByLabel('Password').fill(who.password);
      await other.getByRole('button', { name: 'Sign in', exact: true }).last().click();
      await other.locator('.panes').waitFor();
      const rail = other.locator('.rail').getByRole('button', { name: 'Channels', exact: true });
      if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
      await other.getByRole('navigation', { name: 'Channels' }).getByRole('button', { name: '# general', exact: true }).click();
      await other.getByRole('heading', { name: '# general' }).waitFor();
      await other.getByRole('button', { name: /participants?$/ }).click();
      const theirs = other.getByRole('menu', { name: 'Participants' });
      await theirs.locator('.person').first().waitFor();
      const lines = await theirs.locator('.people-add').allInnerTexts();
      expect(`${said} to be offered ${called ? `"${called}"` : 'no line'}, got ${JSON.stringify(lines)}`, called ? lines.length === 1 && lines[0].trim() === called : lines.length === 0);
      expect(`${said} to be offered no button in the head that adds`, (await other.locator('.channel > .split > .view-head').getByRole('button', { name: /^Add/ }).count()) === 0);
      await other.screenshot({ path: `${out}/channel-20-participants-${called ? 'a-person' : 'nobody'}.png` });
    } finally {
      await context.close();
    }
  }
}
