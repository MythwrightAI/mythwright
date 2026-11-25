// The gear of a page's head (`src/views/pagehead.tsx`), for the checks
// that go through it. What a page once had buttons of its own for is
// lines of the gear's menu: the tables that sit beside a table, under a
// heading that says Its tables, and what can be done to a board. A check
// that used to press such a button presses the gear and then the line.
//
// The menu says what there was when it was opened: a table whose rows were
// not read yet says `being read`, and goes on saying it. So a check that
// holds how many rows each has waits for the thing on the page that the
// rows put there, or asks `tablesRead`, which opens the menu as a person
// would who found it too soon: again.

/** The gear of the page's head. A table in the drawer has none. */
export const gearOf = (page) => page.locator('.view-head .head-acts').getByRole('button', { name: 'More for this page', exact: true }).first();

/** The menu the gear opens: a column's menu is a menu too, and is not this one. */
export const menuOf = (page) => page.locator('.row-menu');

/** Opens the gear's menu, by a press or by a touch (`tap`), and hands the menu back. */
export async function openGear(page, how = 'click') {
  await gearOf(page)[how]();
  await menuOf(page).waitFor();
  return menuOf(page);
}

/** What each line of the menu says: its name and the faint words after it, without the drawing before it; or what it only says. */
export const linesOf = (page) =>
  menuOf(page)
    .locator('button, .menu-said')
    .evaluateAll((all) => all.map((one) => (one.matches('button') ? [...one.querySelectorAll('.truncate, .hint')].map((part) => part.textContent).join(' ') : one.textContent).replace(/\s+/g, ' ').trim()));

/** The lines under the heading Its tables in the open menu, each by its name and how many rows it has; none where there is no heading. */
export const tablesOf = (page) =>
  menuOf(page).evaluate((box) => {
    const all = [...box.children];
    const at = all.findIndex((one) => one.matches('.menu-said') && one.textContent.trim() === 'Its tables');
    if (at < 0) return [];
    const under = [];
    for (const one of all.slice(at + 1)) {
      // They end at the first line of another kind of thing, or at something that is only said.
      if (!one.matches('button') || one.classList.contains('apart')) break;
      under.push([...one.querySelectorAll('.truncate, .hint')].map((part) => part.textContent).join(' ').replace(/\s+/g, ' ').trim());
    }
    return under;
  });

/**
 * Opens the gear's menu once every table beside this one has been read,
 * and hands back what is under Its tables. Opened sooner, a line says
 * `being read`: it is put away and opened again, a frame later.
 */
export async function tablesRead(page, how = 'click') {
  for (let tries = 0; ; tries += 1) {
    await openGear(page, how);
    const tables = await tablesOf(page);
    if (!tables.some((one) => / being read$/.test(one))) return tables;
    if (tries > 300) throw new Error(`the tables beside this one were never read: ${tables.join(' | ')}`);
    await page.keyboard.press('Escape');
    await menuOf(page).waitFor({ state: 'detached' });
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  }
}

/** Presses a line of the gear's menu, found by its name, and waits for the menu to be put away. */
export async function gearLine(page, name, how = 'click') {
  const menu = await openGear(page, how);
  await menu.getByRole('menuitem', { name })[how]();
  await menu.waitFor({ state: 'detached' });
}

/** The buttons of the page's head, each by what it is called: none of them is for its tables or, on a board, its tools. */
export const headButtons = (page) =>
  page
    .locator('.view-head')
    .first()
    .locator('button')
    .evaluateAll((all) => all.map((one) => (one.getAttribute('aria-label') ?? one.textContent).replace(/\s+/g, ' ').trim()));
