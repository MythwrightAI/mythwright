// Whether a page of tables fills its tab: the widest
// thing of its body reaching as far from the tab's right edge as the body
// begins from its left, within a pixel, at 1280 and at 1600 wide; and at a
// phone's width nothing of the page scrolling sideways. A table too wide
// for what it is in scrolls in its own panel, which is not the page
// scrolling.
//
// `body` is the part of the page that holds its tables (`.view-body`
// usually); its parent is the tab's page, measured by what it shows of
// itself (`clientWidth`), so that a scrollbar is not taken for a gutter.
// The window is put back as it was once it is measured, and the side
// panel with it, which a phone's width puts away.

/** How the body's widest child stands in its page: the gutters at either side, in whole pixels to a tenth. */
const across = (body) =>
  body.evaluate((one) => {
    const page = one.parentElement;
    const rim = page.getBoundingClientRect();
    const left = rim.left + page.clientLeft;
    const right = left + page.clientWidth;
    const kids = [...one.children].filter((kid) => {
      const box = kid.getBoundingClientRect();
      return box.width > 0 && box.height > 0 && getComputedStyle(kid).position !== 'absolute' && getComputedStyle(kid).position !== 'fixed';
    });
    const boxes = kids.map((kid) => kid.getBoundingClientRect());
    const tenth = (n) => Math.round(n * 10) / 10;
    return {
      left: tenth(Math.min(...boxes.map((box) => box.left)) - left),
      right: tenth(right - Math.max(...boxes.map((box) => box.right))),
      page: Math.round(page.clientWidth),
    };
  });

/** Whether anything of the page goes past its right edge, so that it would scroll sideways. */
const sideways = (body) =>
  body.evaluate((one) => {
    const page = one.parentElement;
    const doc = document.documentElement;
    return { page: page.scrollWidth - page.clientWidth, body: one.scrollWidth - one.clientWidth, window: doc.scrollWidth - doc.clientWidth };
  });

/**
 * Holds `body` to filling its page at 1280 and 1600 wide and to scrolling
 * nothing sideways at 390, saying `what` it is where it does not. Waits
 * for `ready` (a locator) after each width is set, where one is given.
 * At a phone's width the side panel may have the screen: `again` is what
 * shows the page there once more, asked where the body is not drawn.
 * `shot` is where a picture of each width is written, its width after it.
 */
export async function fillsPage(page, what, body, { ready, again, shot, widths = [1280, 1600], phone = 390 } = {}) {
  const was = page.viewportSize();
  // Which place of the rail had the side panel: a phone's width puts the panel away, and it is put back after.
  const rail = page.locator('.rail button[aria-pressed]');
  const pressed = await rail.evaluateAll((all) => all.findIndex((one) => one.getAttribute('aria-pressed') === 'true'));
  const seen = [];
  for (const width of widths) {
    await page.setViewportSize({ width, height: was.height });
    if (ready) await ready.first().waitFor();
    await body.waitFor();
    // Laid out again for the width: two frames.
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    const at = await across(body);
    seen.push({ width, ...at });
    if (shot) await page.screenshot({ path: `${shot}-${width}.png` });
    if (!(at.left >= 0 && Math.abs(at.left - at.right) <= 1)) {
      throw new Error(`expected ${what} to fill its page at ${width} wide, the right gap the left's, and its gaps were ${at.left} at the left and ${at.right} at the right of ${at.page}`);
    }
  }
  await page.setViewportSize({ width: phone, height: 844 });
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  if (again && !(await body.isVisible())) await again();
  if (ready) await ready.first().waitFor();
  await body.waitFor();
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  const past = await sideways(body);
  if (shot) await page.screenshot({ path: `${shot}-${phone}.png` });
  if (past.page > 1 || past.body > 1 || past.window > 1) throw new Error(`expected nothing of ${what} to scroll sideways at a phone's width, and it went past by ${JSON.stringify(past)}`);
  await page.setViewportSize(was);
  if (pressed >= 0 && (await rail.nth(pressed).getAttribute('aria-pressed')) !== 'true') await rail.nth(pressed).click();
  await body.waitFor();
  return seen;
}
