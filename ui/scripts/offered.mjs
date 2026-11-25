// What floats beside something being written, held by its boxes: whole in
// the window, over everything it lies on and under nothing (the panel
// beside a channel cut a thread's list off under its tabs), over
// none of what it is offered for, and nothing moved by its coming. Who can
// be named after an @ in a composer (`offeredWhole`, the arrows still
// marking in it and Escape putting it away), the composer's two menus
// (`toolMenuWhole`) and who can be named on a record (`taggedWhole`).
// `carried.mjs` asks the first two of the channel's own box, of one written
// at length and of a thread's in the panel beside it, at three widths and a
// phone's; `toolscard.mjs` the third, in the panel's card form.

/** The boxes of what the composer in `scope` is made of, and of what was said there. */
const composerAt = (page, scope) =>
  page.evaluate((scope) => {
    const at = (selector) => {
      const one = document.querySelector(`${scope} ${selector}`);
      if (!one) return null;
      const box = one.getBoundingClientRect();
      return [box.left, box.top, box.width, box.height].map((n) => Math.round(n * 10) / 10).join();
    };
    return { box: at('.composer-box'), written: at('.composer-box textarea'), list: at('.messages') };
  }, scope);

/**
 * Holds what floats (`list`, a selector of the page) by its boxes: in the window, nothing over it, over none of the
 * boxes `clear` names (selectors) nor, where `caret` names a box being written in, of the line its caret is on.
 */
async function floatsWhole(page, expect, { list, clear = [], caret = null, lines, said }) {
  // Placed before it is drawn: it is visible from the first frame it is anywhere.
  await page.waitForFunction((list) => getComputedStyle(document.querySelector(list)).visibility === 'visible', list);
  const seen = await page.evaluate(
    ([selector, clear, caret, lines]) => {
      const list = document.querySelector(selector);
      const box = list.getBoundingClientRect();
      const window = { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight };
      // What is drawn at each of its corners, in by as much as they are rounded (a point is hit only inside the
      // rounding), and in the middle of its head and its foot: the list itself, so that nothing lies over it there.
      const inset = Math.max(3, parseFloat(getComputedStyle(list).borderTopLeftRadius) || 0);
      const middle = (box.left + box.right) / 2;
      const points = [
        [box.left + inset, box.top + inset],
        [box.right - inset, box.top + inset],
        [box.left + inset, box.bottom - inset],
        [box.right - inset, box.bottom - inset],
        [middle, box.top + 2],
        [middle, box.bottom - 2],
      ];
      const under = points.map(([x, y]) => document.elementFromPoint(x, y)).filter((one) => !one || !list.contains(one)).map((one) => (one ? `${String(one.className || one.tagName)} in ${String(one.parentElement?.className)}` : 'nothing'));
      const meets = (other) => box.left < other.right - 0.5 && other.left < box.right - 0.5 && box.top < other.bottom - 0.5 && other.top < box.bottom - 0.5;
      const over = clear.filter((one) => document.querySelector(one) && meets(document.querySelector(one).getBoundingClientRect()));
      if (caret) {
        // The line the caret is on, by a copy of the box with what is before the caret written in it, laid out as it is.
        const area = document.querySelector(caret);
        const style = getComputedStyle(area);
        const copy = document.body.appendChild(document.createElement('div'));
        for (const name of ['boxSizing', 'width', 'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft', 'borderTopWidth', 'borderRightWidth', 'borderBottomWidth', 'borderLeftWidth', 'fontFamily', 'fontSize', 'fontWeight', 'letterSpacing', 'lineHeight', 'tabSize']) copy.style[name] = style[name];
        Object.assign(copy.style, { position: 'fixed', left: '0', top: '0', visibility: 'hidden', whiteSpace: 'pre-wrap', overflowWrap: 'break-word', borderStyle: 'solid' });
        copy.textContent = area.value.slice(0, area.selectionStart);
        const mark = copy.appendChild(document.createElement('span'));
        mark.textContent = '​';
        const at = area.getBoundingClientRect();
        const top = at.top + mark.offsetTop - area.scrollTop;
        copy.remove();
        if (meets({ left: at.left, right: at.right, top, bottom: top + mark.offsetHeight })) over.push('the line the caret is on');
      }
      const options = [...list.querySelectorAll(lines)];
      return {
        box: { x: box.left, y: box.top, right: box.right, bottom: box.bottom },
        window,
        under,
        over,
        // Every line is there to be read: wholly inside the list's sides, and the list scrolls to the last of them.
        sideways: options.some((one) => one.getBoundingClientRect().left < box.left - 0.5 || one.getBoundingClientRect().right > box.right + 0.5),
        reached: list.scrollHeight <= list.clientHeight + 1 || getComputedStyle(list).overflowY === 'auto',
        count: options.length,
        scrolls: document.scrollingElement.scrollWidth > document.scrollingElement.clientWidth,
      };
    },
    [list, clear, caret, lines],
  );
  expect(`${said}: something to be offered, got ${seen.count}`, seen.count > 0);
  expect(`${said}: what floats to be wholly in the window, got ${JSON.stringify(seen.box)} in ${JSON.stringify(seen.window)}`, seen.box.x >= 0 && seen.box.y >= 0 && seen.box.right <= seen.window.width + 0.5 && seen.box.bottom <= seen.window.height + 0.5);
  expect(`${said}: nothing to lie over what floats (the panel's tabs least of all), got ${seen.under.join(', ')} at its corners and edges`, seen.under.length === 0);
  expect(`${said}: what floats to lie over none of what it is offered for, got it over ${seen.over.join(', ')}`, seen.over.length === 0);
  expect(`${said}: every line of it inside its sides, and the rest reached by scrolling it`, !seen.sideways && seen.reached);
  expect(`${said}: the page not to scroll sideways`, !seen.scrolls);
  return seen;
}

/**
 * Types an @ in the box of the composer in `scope` (`.channel > .split`, `.thread`, `.context-write`), and holds what
 * is offered by its boxes, with a picture of it at `shot` where one is said; then puts it away with Escape and leaves
 * the box empty. `wide` is a message being written at length in the panel beside the channel, where what is offered
 * is over none of the tools and of the line the caret is on, a few lines down.
 */
export async function offeredWhole(page, expect, scope, said, shot = null, wide = false) {
  const area = page.locator(`${scope} .composer-box textarea`);
  await area.click();
  const begun = wide ? 'one\ntwo\nthree\n' : '';
  await area.fill(begun);
  await area.press('End');
  const list = page.locator('.mentions.offered[aria-label="Mention"]');
  // What is written is as tall as it is with the @ in it (an empty box is as tall as what it says it is for): the
  // composer is measured so, with the list put away, and again as it is offered for that @ once more.
  await page.keyboard.type('@');
  await list.waitFor();
  await page.keyboard.press('Escape');
  await list.waitFor({ state: 'detached' });
  const before = await composerAt(page, scope);
  await page.keyboard.press('Backspace');
  await page.keyboard.type('@');
  await list.waitFor();
  const seen = await floatsWhole(page, expect, {
    list: '.mentions.offered[aria-label="Mention"]',
    clear: [`${scope} .composer-foot`, ...(wide ? [] : [`${scope} .composer-box textarea`])],
    caret: wide ? `${scope} .composer-box textarea` : null,
    lines: '[role=option]',
    said,
  });
  if (shot) await page.screenshot({ path: shot });
  const after = await composerAt(page, scope);
  expect(`${said}: nothing of the composer or of what was said to move as what is offered came, was ${JSON.stringify(before)}, is ${JSON.stringify(after)}`, JSON.stringify(before) === JSON.stringify(after));
  // The keys are the box's as they were: the arrows mark in the list, and Escape puts it away and nothing else.
  if (seen.count > 1) {
    await page.keyboard.press('ArrowDown');
    expect(`${said}: the arrows to mark the next one`, (await list.locator('[role=option]').nth(1).getAttribute('aria-selected')) === 'true');
  }
  await page.keyboard.press('Escape');
  await list.waitFor({ state: 'detached' });
  expect(`${said}: Escape to have put away what was offered and kept what was written, got "${await area.inputValue()}"`, (await area.inputValue()) === `${begun}@`);
  await area.fill('');
  return seen;
}

/**
 * Opens the menu a tool of the composer in `scope` opens (`Ask an agent or a bot`, `Kind of line`), holds it by its
 * boxes, over none of the tools, with nothing of the composer moved, and puts it away by its tool again.
 */
export async function toolMenuWhole(page, expect, scope, tool, menu, said, shot = null) {
  const before = await composerAt(page, scope);
  const button = page.locator(`${scope} .composer-foot`).getByRole('button', { name: tool });
  await button.click();
  const opened = page.getByRole('menu', { name: menu });
  await opened.waitFor();
  const selector = `.floated[role=menu][aria-label="${menu}"]`;
  await floatsWhole(page, expect, { list: selector, clear: [`${scope} .composer-foot`], lines: '[role=menuitem]', said: `${said}, ${menu}` });
  if (shot) await page.screenshot({ path: shot });
  const after = await composerAt(page, scope);
  expect(`${said}: nothing of the composer to move as ${menu} opened, was ${JSON.stringify(before)}, is ${JSON.stringify(after)}`, JSON.stringify(before) === JSON.stringify(after));
  // A press in it is in it: its first line is pressed without the menu having gone first.
  expect(`${said}: the first line of ${menu} to be pressed where it is`, await opened.getByRole('menuitem').first().evaluate((one) => document.elementFromPoint(one.getBoundingClientRect().left + 8, one.getBoundingClientRect().top + 8)?.closest('[role=menuitem]') === one));
  // Put away by the tool that opened it, as a person does.
  await button.click();
  await opened.waitFor({ state: 'detached' });
}

/** Types an @ in the box something is said on a record in (`area`, a locator), and holds who is offered by its boxes. */
export async function taggedWhole(page, expect, area, said, shot = null) {
  await area.click();
  await area.fill('');
  await page.keyboard.type('@');
  const list = page.locator('.tag-offered');
  await list.waitFor();
  const label = await area.getAttribute('aria-label');
  const seen = await floatsWhole(page, expect, { list: '.tag-offered', clear: [`textarea[aria-label="${label}"]`], lines: '[role=option]', said });
  if (shot) await page.screenshot({ path: shot });
  await page.keyboard.press('Escape');
  await list.waitFor({ state: 'detached' });
  await area.fill('');
  return seen;
}
