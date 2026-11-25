// Choosing a colour of a theme somebody makes, in the client's own picker
// (`views/colourpick.tsx`), as `themed.mjs` holds it: that check's part
// for the picker, in a file of its own. It starts no daemon and no
// browser: it is handed a page signed in with Settings showing, a second
// page of the same person's that is never read again, where its pictures
// go and how that check says what it expected.
//
// The picker is opened from a colour's swatch, with the colour as it was
// beside the colour now, each said in words and each its colour; it lies
// over the page and moves nothing; the colour is chosen by the area, by
// the arrows on the area and on the hue, and by the field it is written
// out in, which takes only what reads as a colour and says so otherwise;
// the theme's other colours and those that ship are there to take in a
// press; a colour that leaves the theme unreadable is said in the rule's
// own sentence as it is chosen; Escape and a press outside put it back,
// the dialog under it staying; Done and Enter keep it, and the theme kept
// with it is worn on the page that is never read again. Then by the
// boxes: nothing of the picker over anything else of it, nothing of it
// outside the window, at three widths and a phone's, in the dark and in
// the light. Also the helpers every check that chooses a theme's colour
// goes through (`pickColour`, `colourOf`), since a colour is no longer a
// box that can be filled.

import { NOT_A_COLOUR, fromArea, hexOf, hsvOf, stepped } from '../src/picking.ts';
import { STARTS, unreadable } from '../src/themes.ts';

/** The picker that is open over the page. */
export const pickerOf = (page) => page.locator('.colourpick[role="dialog"]');

/** Waits until the picker is drawn where it is to be: it is first drawn out of sight, to be measured. */
const placedIn = async (page) => {
  const picker = pickerOf(page);
  await picker.waitFor();
  await page.waitForFunction(() => {
    const one = document.querySelector('.colourpick[role="dialog"]');
    return one && getComputedStyle(one).visibility !== 'hidden';
  });
  return picker;
};

/** Opens the picker of a colour, the swatch found by its label within `within` (the form). */
export const openPicker = async (page, within, label) => {
  await within.getByRole('button', { name: label, exact: true }).click();
  return placedIn(page);
};

/** What colour a swatch of the form says it is. */
export const colourOf = (within, label) => within.getByRole('button', { name: label, exact: true }).getAttribute('data-colour');

/** Chooses a colour of the form by writing it out in the picker and keeping it with Enter. */
export const pickColour = async (page, within, label, colour) => {
  const picker = await openPicker(page, within, label);
  await picker.getByLabel('Hex', { exact: true }).fill(colour);
  await picker.getByLabel('Hex', { exact: true }).press('Enter');
  await picker.waitFor({ state: 'detached' });
};

/** Lets the page draw: what opens over it listens for a press outside it from a moment after it is drawn, and a person is never that quick. */
export const settled = (page) => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));

/** The two halves of the split swatch: what each says and the colour each is drawn in. */
const halves = (picker) =>
  picker.evaluate((box) =>
    ['.colourpick-was', '.colourpick-now'].map((one) => {
      const half = box.querySelector(one);
      return { said: half.innerText.replace(/\s+/g, ' ').trim(), background: getComputedStyle(half).backgroundColor, box: half.getBoundingClientRect().toJSON() };
    }),
  );

/** A colour written `#rrggbb` as the browser says a background: `rgb(r, g, b)`. */
const rgbOf = (hex) => `rgb(${[1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16)).join(', ')})`;

/** Every box of the page that is not the picker's, so that it can be said that the picker opening moved none of them. */
const boxesOf = (page) =>
  page.evaluate(() =>
    [...document.querySelectorAll('.modal *')]
      .filter((one) => !one.closest('.colourpick'))
      .map((one) => {
        const at = one.getBoundingClientRect();
        return [at.left, at.top, at.width, at.height].map((figure) => Math.round(figure * 10) / 10).join(',');
      })
      .join('|'),
  );

/** What of the picker is wrong by its boxes: one part over another, a part outside the picker or the window, and the page scrolling sideways. */
export const pickerCrowded = (page) =>
  page.evaluate(() => {
    const picker = document.querySelector('.colourpick[role="dialog"]');
    if (!picker) return ['there is no picker'];
    const frame = picker.getBoundingClientRect();
    const wrong = [];
    if (frame.left < -0.5 || frame.top < -0.5 || frame.right > window.innerWidth + 0.5 || frame.bottom > window.innerHeight + 0.5) wrong.push(`the picker is outside the window (${JSON.stringify(frame.toJSON())})`);
    const parts = [...picker.querySelectorAll('.colourpick-head strong, .colourpick-was, .colourpick-now, .colourpick-area, .colourpick-hue, .colourpick-hex span, .colourpick-hex input, .colourpick-said, .colourpick-picks > span, .colourpick-pick, .colourpick-foot .faint, .colourpick-foot button')]
      .map((one) => ({ said: (one.getAttribute('aria-label') ?? one.textContent).trim().slice(0, 30), box: one.getBoundingClientRect() }))
      .filter((one) => one.box.width > 0 && one.box.height > 0);
    // What scrolls in the picker is looked at where the picker shows it, and what scrolls under its foot, which stays in
    // reach over it, is hidden there.
    const foot = picker.querySelector('.colourpick-foot').getBoundingClientRect();
    const shown = parts
      .map((one) => (one.said === 'Done' || one.said === 'Escape puts it back.' ? one : { ...one, box: new DOMRect(one.box.left, Math.max(one.box.top, frame.top), one.box.width, Math.min(one.box.bottom, foot.top) - Math.max(one.box.top, frame.top)) }))
      .filter((one) => one.box.height > 1);
    for (const [index, one] of shown.entries()) {
      if (one.box.left < frame.left - 0.5 || one.box.right > frame.right + 0.5) wrong.push(`${one.said} leaves the picker`);
      for (const two of shown.slice(index + 1)) {
        const over = Math.min(one.box.right, two.box.right) - Math.max(one.box.left, two.box.left) > 0.5 && Math.min(one.box.bottom, two.box.bottom) - Math.max(one.box.top, two.box.top) > 0.5;
        if (over) wrong.push(`${one.said} lies over ${two.said}`);
      }
    }
    // Words are not drawn past the half of the swatch they are in.
    for (const half of picker.querySelectorAll('.colourpick-was, .colourpick-now')) if (half.scrollWidth > half.clientWidth + 1) wrong.push(`the words of ${half.className} are wider than it`);
    const sideways = document.documentElement.scrollWidth - window.innerWidth;
    if (sideways > 0) wrong.push(`the page scrolls sideways by ${sideways}px`);
    if (picker.scrollWidth > picker.clientWidth + 1) wrong.push('the picker scrolls sideways');
    return wrong;
  });

/**
 * The picker, held as the head of this file says. `page` has Settings
 * showing for whoever may make themes; `other` is the same person's page,
 * never read again; `name` is what the theme made here is called, and
 * `accent` the accent it is kept with.
 */
export async function pickedColours({ page, other, out, expect, name, scheme, accent }) {
  await page.getByRole('button', { name: /Add a theme/ }).click();
  const form = page.locator('.modal');
  await form.getByLabel('What it is called').fill(name);
  await form.getByLabel('Starting from').selectOption('grid-iron');
  const start = STARTS['grid-iron'];

  // 1: opened from the accent, the colour as it was beside the colour now, each in words and in its colour, and nothing moved.
  const before = await boxesOf(page);
  let picker = await openPicker(page, form, 'Accent');
  expect('the swatch to say its picker is open', (await form.getByRole('button', { name: 'Accent', exact: true }).getAttribute('aria-expanded')) === 'true');
  let [was, now] = await halves(picker);
  expect(`Was and Now side by side in the accent as it was, got ${JSON.stringify([was, now])}`, was.said === `Was ${start.accent}` && now.said === `Now ${start.accent}` && was.background === rgbOf(start.accent) && now.background === rgbOf(start.accent));
  expect('Was beside Now, on one line', Math.abs(was.box.top - now.box.top) < 1 && was.box.right <= now.box.left + 0.5);
  expect('nothing of the form moved by the picker opening', (await boxesOf(page)) === before);
  expect(`the picker to be called after its colour, got ${await picker.getAttribute('aria-label')}`, (await picker.getAttribute('aria-label')) === 'Choosing Accent');
  expect('the keyboard to be on the area as it opens', await picker.locator('.colourpick-area').evaluate((one) => document.activeElement === one));
  await page.screenshot({ path: `${out}/picked-1-open-${scheme}.png` });

  // 2: by the area, pressed near its top right (its corner is rounded, and no press lands there), which is the hue at
  // nearly its strongest and brightest; the form wears it as it is chosen.
  const area = await picker.locator('.colourpick-area').boundingBox();
  const box = { left: area.x, top: area.y, width: area.width, height: area.height };
  let hsv = hsvOf(start.accent);
  const hue = Math.round(hsv.hue);
  await page.mouse.click(area.x + area.width - 1, area.y + 10);
  hsv = { ...hsv, ...fromArea(area.x + area.width - 1, area.y + 10, box) };
  const cornered = hexOf(hsv);
  [was, now] = await halves(picker);
  expect(`the area to choose the colour at its corner, got ${JSON.stringify(now)} for ${cornered}`, now.said === `Now ${cornered}` && now.background === rgbOf(cornered) && was.background === rgbOf(start.accent));
  expect('the hex field to follow the area', (await picker.getByLabel('Hex', { exact: true }).inputValue()) === cornered);
  expect('the swatch to be the colour as it is chosen', (await colourOf(form, 'Accent')) === cornered);
  const sampled = () => form.getByLabel('How it looks', { exact: true }).evaluate((box) => getComputedStyle(box).getPropertyValue('--accent').trim());
  expect('the form to wear it as it is chosen', (await sampled()) === cornered);
  // Dragged across past its left edge, the strength goes and the brightness stays.
  await page.mouse.move(area.x + area.width - 1, area.y + 10);
  await page.mouse.down();
  await page.mouse.move(area.x + area.width / 2, area.y + 10, { steps: 4 });
  await page.mouse.move(area.x - 20, area.y + 10, { steps: 4 });
  await page.mouse.up();
  hsv = { ...hsv, strength: 0 };
  expect(`dragged past its left edge, the colour to be a grey as bright, got ${await picker.getByLabel('Hex', { exact: true }).inputValue()}`, (await picker.getByLabel('Hex', { exact: true }).inputValue()) === hexOf(hsv));

  // 3: by the keys: the area across and down by a step, ten with Shift; the hue by a degree, and Home and End; the hue kept through a grey.
  await picker.locator('.colourpick-area').focus();
  for (const [key, shift] of [['ArrowRight', true], ['ArrowRight', true], ['ArrowDown', false]]) {
    await page.keyboard.press(shift ? `Shift+${key}` : key);
    hsv = stepped(hsv, 'area', key, shift);
  }
  expect(`the arrows on the area to move it by a step, got ${await picker.getByLabel('Hex', { exact: true }).inputValue()} for ${hexOf(hsv)}`, hsv.strength === 0.2 && (await picker.getByLabel('Hex', { exact: true }).inputValue()) === hexOf(hsv));
  expect('the hue kept though the colour was a grey for a while', (await picker.locator('.colourpick-hue').getAttribute('aria-valuenow')) === String(hue));
  await picker.locator('.colourpick-hue').focus();
  await page.keyboard.press('End');
  await page.keyboard.press('ArrowLeft');
  expect('the arrows on the hue to move it by a degree from its end', (await picker.locator('.colourpick-hue').getAttribute('aria-valuenow')) === '358');
  await page.keyboard.press('Home');
  await page.keyboard.press('Shift+ArrowRight');
  expect('Home and Shift with an arrow on the hue', (await picker.locator('.colourpick-hue').getAttribute('aria-valuenow')) === '10');
  expect('the colour to follow the hue', (await picker.getByLabel('Hex', { exact: true }).inputValue()) === hexOf({ ...hsv, hue: 10 }));

  // 4: by the field: what is not a colour said under it and not taken; a colour taken once it reads as one, written small.
  const hex = picker.getByLabel('Hex', { exact: true });
  const held = await hex.inputValue();
  await hex.fill('#2fd4c');
  await picker.getByText(NOT_A_COLOUR, { exact: true }).waitFor();
  expect('half a colour not taken', (await colourOf(form, 'Accent')) === held && (await hex.getAttribute('aria-invalid')) === 'true');
  await hex.press('Enter');
  expect('Enter not keeping what is not a colour', (await pickerOf(page).count()) === 1);
  await page.screenshot({ path: `${out}/picked-2-not-a-colour-${scheme}.png` });
  await hex.fill('#2FD4C4');
  await picker.getByText(NOT_A_COLOUR, { exact: true }).waitFor({ state: 'detached' });
  [, now] = await halves(picker);
  expect(`a colour written out taken, got ${JSON.stringify(now)}`, now.said === 'Now #2fd4c4' && (await colourOf(form, 'Accent')) === '#2fd4c4');

  // 5: a pick: one of the theme's own and one of a theme that ships.
  await picker.getByRole('button', { name: `Ground, ${start.ground}`, exact: true }).click();
  expect('a colour of the theme taken in a press', (await colourOf(form, 'Accent')) === start.ground);
  await picker.getByRole('button', { name: `Rosé Pine: Accent, ${STARTS['rose-pine'].accent}`, exact: true }).click();
  expect('a colour of a theme that ships taken in a press', (await colourOf(form, 'Accent')) === STARTS['rose-pine'].accent);
  expect('the pick that is the colour now marked', (await picker.getByRole('button', { name: `Rosé Pine: Accent, ${STARTS['rose-pine'].accent}`, exact: true }).getAttribute('aria-pressed')) === 'true');

  // 6: Was pressed goes back to it; then Escape puts it back with the dialog staying, and a press outside the same.
  await picker.getByRole('button', { name: /^Was / }).click();
  expect('Was pressed to go back to it', (await colourOf(form, 'Accent')) === start.accent && (await halves(picker))[1].said === `Now ${start.accent}`);
  await hex.fill('#123456');
  await settled(page);
  await page.keyboard.press('Escape');
  await picker.waitFor({ state: 'detached' });
  expect('Escape to put it back', (await colourOf(form, 'Accent')) === start.accent && (await sampled()) === start.accent);
  expect('Escape to be the picker’s alone: the dialog stays', (await form.count()) === 1);
  expect('the keyboard back on the swatch', await form.getByRole('button', { name: 'Accent', exact: true }).evaluate((one) => document.activeElement === one));
  picker = await openPicker(page, form, 'Accent');
  await picker.getByLabel('Hex', { exact: true }).fill('#654321');
  await settled(page);
  await form.getByLabel('What it is called').click();
  await picker.waitFor({ state: 'detached' });
  expect('a press outside to put it back', (await colourOf(form, 'Accent')) === start.accent);
  expect('a press outside to leave the dialog', (await form.count()) === 1);

  // 7: the text chosen too near the ground: the rule's own sentence in the picker as it is chosen, and the form cannot keep it.
  picker = await openPicker(page, form, 'Text');
  await picker.getByLabel('Hex', { exact: true }).fill('#1a1a1a');
  const sentence = unreadable({ ...start, text: '#1a1a1a' });
  await picker.locator('.colourpick-unreadable').filter({ hasText: sentence }).waitFor();
  expect('the form not to keep a theme that cannot be read', !(await form.getByRole('button', { name: 'Add it', exact: true }).isEnabled()));
  const unread = await pickerCrowded(page);
  expect(`the picker saying so, nothing over anything else or outside the window, got ${unread.join('; ')}`, unread.length === 0);
  expect('Done in view with the picker grown', await picker.getByRole('button', { name: 'Done', exact: true }).evaluate((one) => { const at = one.getBoundingClientRect(); return at.bottom <= window.innerHeight && at.top >= 0 && document.elementFromPoint(at.left + at.width / 2, at.top + at.height / 2) === one; }));
  await page.screenshot({ path: `${out}/picked-3-unreadable-${scheme}.png` });
  await picker.getByRole('button', { name: /^Was / }).click();
  await picker.locator('.colourpick-unreadable').waitFor({ state: 'detached' });
  await picker.getByRole('button', { name: 'Done', exact: true }).click();
  await picker.waitFor({ state: 'detached' });

  // 8: the light version's colours by the same picker; one put back by Escape leaves the light version worked out from the dark one.
  await form.getByLabel('A light version').check();
  picker = await openPicker(page, form, 'Light Accent');
  expect('the light accent picker to offer the light colours of the themes that ship', (await picker.getByRole('button', { name: /Rosé Pine: Accent/ }).getAttribute('aria-label')).endsWith(', #b4637a'));
  await picker.getByLabel('Hex', { exact: true }).fill('#000000');
  await settled(page);
  await page.keyboard.press('Escape');
  await picker.waitFor({ state: 'detached' });
  expect('the light version still worked out from the dark one after a picker was put back', await form.getByText('Worked out from the colours above').isVisible());

  // 9: by the boxes, at three widths and a phone's.
  for (const [width, height] of [[1280, 860], [1000, 760], [800, 700], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await form.getByRole('button', { name: 'Accent', exact: true }).scrollIntoViewIfNeeded();
    picker = await openPicker(page, form, 'Accent');
    const wrong = await pickerCrowded(page);
    expect(`the picker at ${width} wide, nothing over anything else or outside the window, got ${wrong.join('; ')}`, wrong.length === 0);
    if (width === 1280 || width === 390) await page.screenshot({ path: `${out}/picked-4-at-${width}-${scheme}.png` });
    await settled(page);
    await page.keyboard.press('Escape');
    await picker.waitFor({ state: 'detached' });
  }
  await page.setViewportSize({ width: 1280, height: 860 });

  // 10: Done keeps it, and Enter on the area too; the theme kept is worn on the page that is never read again.
  picker = await openPicker(page, form, 'Accent');
  await picker.getByLabel('Hex', { exact: true }).fill(accent);
  await picker.getByRole('button', { name: 'Done', exact: true }).click();
  await picker.waitFor({ state: 'detached' });
  expect('Done to keep it', (await colourOf(form, 'Accent')) === accent);
  picker = await openPicker(page, form, 'Good');
  await page.keyboard.press('ArrowUp');
  const kept = await picker.getByLabel('Hex', { exact: true }).inputValue();
  await page.keyboard.press('Enter');
  await picker.waitFor({ state: 'detached' });
  expect('Enter on the area to keep it', (await colourOf(form, 'Good')) === kept);
  await form.getByRole('button', { name: 'Add it', exact: true }).click();
  await form.waitFor({ state: 'detached' });
  await other.waitForFunction((accent) => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() === accent, accent);
  expect('the colour kept to be worn on the page that is never read again', (await other.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--ok').trim())) === kept);
}
