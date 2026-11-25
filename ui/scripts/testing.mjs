// Testing, in a blueprint's head (`src/views/testing.tsx`), for the checks
// that go through it. What tries a text without changing what runs was four
// buttons of the head (Preview, Sandbox test, Side by side, Compare ways)
// and is one, Testing, whose menu has a line for each
// (Compare ways is Compare on real work there). A check that used to press
// one of those buttons presses Testing and then the line, as a person does.
//
// The menu says what there was when it was opened: a check that reads
// whether a line is closed opens it once what it waits for is on the page.

/** Testing, in a head. */
export const testingOf = (head) => head.getByRole('button', { name: 'Testing', exact: true });

/** The menu Testing opens, drawn by the frame and not in the head. */
export const testingMenu = (page) => page.locator('.row-menu[aria-label="Testing"]');

/** Opens Testing's menu, by a press or by a touch, where it is not open, and hands the menu back. */
export async function openTesting(page, head, touch = false) {
  if ((await testingMenu(page).count()) === 0) {
    const button = testingOf(head);
    // A drawer open beside the page lies over the head's right, where Testing may be: it is then reached by the keyboard,
    // as a person reaches it, and not pressed through the drawer.
    const covered = await button.evaluate((one) => {
      const at = one.getBoundingClientRect();
      return !one.contains(document.elementFromPoint((at.left + at.right) / 2, (at.top + at.bottom) / 2));
    });
    if (covered) {
      await button.focus();
      await page.keyboard.press('Enter');
    } else await (touch ? button.tap() : button.click());
  }
  await testingMenu(page).waitFor();
  return testingMenu(page);
}

/** A line of Testing's menu, the menu opened for it. */
export const testingLine = async (page, head, name, touch = false) => (await openTesting(page, head, touch)).getByRole('menuitem', { name, exact: true });

/** Presses a line of Testing: Testing, and then the line. */
export async function pressTesting(page, head, name, touch = false) {
  const line = await testingLine(page, head, name, touch);
  await (touch ? line.tap() : line.click());
  await testingMenu(page).waitFor({ state: 'detached' });
}

/** What the menu says under a line: its word on what it does. */
export const testingNote = async (page, head, name) => {
  const menu = await openTesting(page, head);
  return menu.evaluate((box, which) => {
    const all = [...box.children];
    const at = all.findIndex((one) => one.matches('button') && one.querySelector('.truncate')?.textContent.trim() === which);
    return at < 0 ? null : (all[at + 1]?.textContent ?? '').trim();
  }, name);
};

/** Puts Testing's menu away by Escape, the keyboard going back to Testing. */
export async function closeTesting(page) {
  if ((await testingMenu(page).count()) === 0) return;
  await page.keyboard.press('Escape');
  await testingMenu(page).waitFor({ state: 'detached' });
}
