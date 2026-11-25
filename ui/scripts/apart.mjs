// What a check holds by the boxes of the Members page's Agents tab and of Settings: the
// parts of each line of a list, and the parts of the Theme panel, none
// over another or past where it is drawn, and nothing that scrolls the
// page sideways. Each answers what is wrong, in a sentence a thing, and
// nothing where all is well; the check says what it was looking at. And
// the way to each, as `agents.mjs` goes it: the Members page from its line
// under Factory, at its Agents tab, of every workspace; Settings from the rail and back.
//
// Both are held so by `agents.mjs` at three widths and a phone's, and by
// `phone.mjs` at two phones'.

import { sideSettled } from './drawer.mjs';

/**
 * The parts of each line found by `lines` (selectors of `parts`): none
 * over another, none past its line, no line past the window's sides.
 */
export const linesCrowded = (page, lines, parts) =>
  page.evaluate(
    ([lines, parts]) => {
      const said = [];
      const all = [...document.querySelectorAll(lines)];
      if (all.length === 0) said.push('there is no line');
      for (const row of all) {
        const frame = row.getBoundingClientRect();
        const called = row.dataset.agent ?? row.closest('[data-turn]')?.dataset.turn ?? '';
        if (frame.left < -0.5 || frame.right > window.innerWidth + 0.5) said.push(`${called} is past the window's side`);
        const boxes = parts.flatMap((part) => [...row.querySelectorAll(part)].map((one) => [part, one.getBoundingClientRect()])).filter(([, box]) => box.width > 0 && box.height > 0);
        for (const [part, box] of boxes) {
          if (box.left < frame.left - 0.5 || box.right > frame.right + 0.5 || box.top < frame.top - 0.5 || box.bottom > frame.bottom + 0.5) said.push(`${part} of ${called} is past its line`);
        }
        for (let one = 0; one < boxes.length; one += 1) {
          for (let other = one + 1; other < boxes.length; other += 1) {
            const [a, b] = [boxes[one][1], boxes[other][1]];
            if (Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5) said.push(`${boxes[one][0]} of ${called} lies over ${boxes[other][0]}`);
          }
        }
      }
      const wide = document.documentElement.scrollWidth - window.innerWidth;
      if (wide > 0) said.push(`the page is ${wide} wider than the window`);
      return said;
    },
    [lines, parts],
  );

/**
 * Settings' Theme panel: its heading, what is said beside it and each
 * theme, each as far as its words reach and not only as far as its box
 * (a heading squeezed to nothing kept its words, which lay over the note
 * beside it), none over another and none past the panel's sides.
 */
export const themeCrowded = (page) =>
  page.evaluate(() => {
    const said = [];
    const panel = [...document.querySelectorAll('.settings-panels > .panel')].find((one) => one.querySelector('.panel-head > h2')?.textContent === 'Theme');
    if (!panel) return ['there is no Theme panel'];
    const frame = panel.getBoundingClientRect();
    const reach = (one) => {
      const box = one.getBoundingClientRect();
      const words = document.createRange();
      words.selectNodeContents(one);
      let text = words.getBoundingClientRect();
      // Words a part cuts off where its box ends are not drawn past it.
      if (getComputedStyle(one).overflowX !== 'visible') text = new DOMRect(Math.max(text.left, box.left), text.top, Math.max(0, Math.min(text.right, box.right) - Math.max(text.left, box.left)), text.height);
      const across = text.width > 0;
      const down = text.height > 0;
      return {
        left: Math.min(box.left, across ? text.left : box.left),
        right: Math.max(box.right, across ? text.right : box.right),
        top: Math.min(box.top, down ? text.top : box.top),
        bottom: Math.max(box.bottom, down ? text.bottom : box.bottom),
      };
    };
    const parts = [...panel.querySelectorAll('.panel-head > *, .theme-samples > *, .theme-actions > button')].map((one) => [one.textContent.trim().slice(0, 24), reach(one)]).filter(([, at]) => at.right > at.left && at.bottom > at.top);
    if (parts.length < 3) said.push(`only ${parts.length} parts were found`);
    for (const [name, at] of parts) if (at.left < frame.left - 0.5 || at.right > frame.right + 0.5) said.push(`"${name}" is past the panel's side`);
    for (let one = 0; one < parts.length; one += 1) {
      for (let other = one + 1; other < parts.length; other += 1) {
        const [a, b] = [parts[one][1], parts[other][1]];
        if (Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5) said.push(`"${parts[one][0]}" lies over "${parts[other][0]}"`);
      }
    }
    const wide = document.documentElement.scrollWidth - window.innerWidth;
    if (wide > 0) said.push(`the page is ${wide} wider than the window`);
    return said;
  });

/**
 * Opens the Members page from its line under Factory, at its Agents tab, with every workspace
 * chosen where that is a line, which is every agent there is (where it is
 * not, the workspace's agents are all there are); where it never comes, says where
 * the page stood as it was pressed and after, with a picture in `out`.
 */
export async function agentsPage(on, out, press = 'click') {
  const stood = async () => ({ tabs: await on.locator('.tabs .tab').allInnerTexts(), workspaceSaid: (await on.locator('.space-foot .space-said').count()) > 0 });
  const before = await stood();
  if (!(await on.locator('.rail').isVisible())) await on.getByRole('button', { name: 'Expand the side panel' })[press]();
  await sideSettled(on);
  const railed = on.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
  if ((await railed.getAttribute('aria-pressed')) !== 'true') await railed[press]();
  // Factory is a part of the panel that starts unfolded, and may have been folded since (`folded.mjs`).
  const factory = on.getByRole('navigation', { name: 'Resources' }).getByRole('button', { name: 'Unfold Factory', exact: true });
  if ((await factory.count()) > 0) await factory[press]();
  // The Members line's name, which opens its page (its arrow folds), and the page's Agents tab.
  await membersLine(on)[press]();
  try {
    await on.getByRole('heading', { name: 'Members', level: 1 }).waitFor({ timeout: 15000 });
    await membersTab(on, 'Agents')[press]();
    await on.locator('.agents-listed').waitFor({ timeout: 15000 });
    // Every workspace is a line of the choice only where it shows more than the workspace does: where it is not, the workspace is all there is.
    const choice = on.getByLabel(/^What is shown of /);
    if ((await choice.count()) === 0 || (await choice.locator('option[value="*"]').count()) === 0) return;
    if ((await choice.inputValue()) !== '*') await choice.selectOption('*');
    await on.getByRole('heading', { name: 'All workspaces', level: 2 }).waitFor({ timeout: 15000 });
  } catch (err) {
    await on.screenshot({ path: `${out}/agents-page-never-came.png` });
    throw new Error(`expected the Members page's Agents tab of every workspace, pressed with the page at ${JSON.stringify(before)}, and it is at ${JSON.stringify(await stood())}: ${err.message}`);
  }
}

/** The Members line's name under Factory, which opens the Members page. */
export const membersLine = (on) => on.getByRole('navigation', { name: 'Resources' }).locator('.members-part .nav-row > .nav-item');

/** One of the Members page's tabs, `People` or `Agents`. */
export const membersTab = (on, name) => on.locator('.members-tabs').getByRole('tab', { name, exact: true });

/** Settings' Theme panel by its boxes, Settings opened from the rail, and the Members tab shown again after. */
export async function themeHeld(on, what, expect, press = 'click') {
  if (!(await on.locator('.rail').isVisible())) await on.getByRole('button', { name: 'Expand the side panel' })[press]();
  await sideSettled(on);
  await on.locator('.rail').getByRole('button', { name: 'Settings', exact: true })[press]();
  await on.getByRole('heading', { name: 'Theme', level: 2 }).waitFor();
  const wrong = await themeCrowded(on);
  expect(`nothing of the Theme panel over anything else of it, past its sides or scrolling sideways ${what}, got ${JSON.stringify(wrong)}`, wrong.length === 0);
  await on.locator('.tabs').getByRole('tab', { name: /Members/ })[press]();
  await on.getByRole('heading', { name: 'Members', level: 1 }).waitFor();
}
