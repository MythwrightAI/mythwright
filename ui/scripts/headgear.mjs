// The board's gear, as `head.mjs` holds it: that check's own part for
// what a board once had two buttons for, in a file of its own because the
// check was at its limit of lines. It starts no daemon and no browser: it
// is handed the page `head.mjs` has open on Shop's board (a board with one
// table, `extras`, beside it and one row in that), where its pictures go,
// and how that check says what it expected.
//
// The board's head has no button for its tables or its tools. Both are
// lines of its gear, under what the gear had and over when the board was
// changed: a heading that only says, each table with how many rows it
// has, each tool with a word on what it does, and Layout. The lines are
// in the menu, in the window, none over another and none cut short;
// nothing of the page moves as the menu opens; the arrows go through the
// lines and pass the heading by. Layout hangs the layout's panel under
// the gear: on the screen, moving nothing, staying as the pointer crosses
// to it, put away by Escape (which is then for it alone), by a press
// outside it and by the gear. A table opens in the drawer and each tool
// does what it says. Then the same at each width `head.mjs` goes through,
// and on a phone.
import { drawerAtRest } from './drawer.mjs';
import { gearLine, gearOf, headButtons, linesOf, menuOf, openGear } from './gear.mjs';

/** What the layout's panel is on the page, where the gear hung it. */
const hungOn = (page) => page.locator('.view-head .layout-menu');

/** Lets the page draw: what opens over it listens for a press outside it from a moment after it is drawn, and a person is never that quick. */
const settled = (page) => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));

/** Everything of the head and of the board under it, by its boxes, but for what hangs under the gear. */
const still = (page) =>
  page
    .locator('.split')
    .first()
    .evaluate((split) =>
      [...split.querySelectorAll('.view-head *, .board, .column, .card')]
        .filter((one) => !one.closest('.layout-menu'))
        .map((one) => {
          const at = one.getBoundingClientRect();
          return [at.left, at.top, at.width, at.height].map((figure) => Math.round(figure * 10) / 10).join(',');
        })
        .join(' '),
    );

/** Holds something the gear opened to the window, and hands back where it is. */
async function onScreen(page, expect, one, where) {
  const [at, view] = [await one.boundingBox(), page.viewportSize()];
  expect(`what the gear opened to be on the screen ${where}: ${JSON.stringify(at)} in ${JSON.stringify(view)}`, at.x >= -0.5 && at.y >= -0.5 && at.x + at.width <= view.width + 0.5 && at.y + at.height <= view.height + 0.5);
  return at;
}

/** Holds the layout's panel to hanging under the gear, its right edge at the gear's, in the window. */
async function underGear(page, expect, where) {
  const [at, by] = [await onScreen(page, expect, hungOn(page), where), await gearOf(page).boundingBox()];
  expect(`the layout to hang under the gear, its right at the gear's, ${where}: ${JSON.stringify(at)} under ${JSON.stringify(by)}`, at.y >= by.y + by.height && at.y - (by.y + by.height) <= 16 && Math.abs(at.x + at.width - (by.x + by.width)) <= 4);
  return at;
}

/** What the line that has the keyboard says. */
const focused = (page) => page.evaluate(() => [...(document.activeElement?.querySelectorAll('.truncate, .hint') ?? [])].map((part) => part.textContent).join(' ').replace(/\s+/g, ' ').trim());

/** The board's gear at the usual width: its lines, the keyboard, the layout it hangs, and what each line does. */
export async function boardGear({ page, out, expect }) {
  const [gear, menu, hung] = [gearOf(page), menuOf(page), hungOn(page)];
  expect(`the board's head to have no button for its tables, its tools or its layout; got ${(await headButtons(page)).join(' | ')}`, !(await headButtons(page)).some((one) => /^(Its tables|Tools|Layout)/.test(one)));
  await page.locator('.column').first().waitFor();
  const before = await still(page);
  // The table beside the board has been read by the time its line says how many rows it has.
  for (let tries = 0; ; tries += 1) {
    await openGear(page);
    if ((await linesOf(page)).includes('Extras 1 extra') || tries > 300) break;
    await page.keyboard.press('Escape');
    await menu.waitFor({ state: 'detached' });
    await settled(page);
  }
  const lines = await linesOf(page);
  const tail = lines.slice(lines.indexOf('Measurements of Shop') + 1);
  expect(
    `the board's gear to say, under what it had, its tables under a heading, then its tools and Layout, and last when it was changed; got ${lines.join(' | ')}`,
    lines[0] === 'Copy link' && tail.length === 8 && tail.slice(0, 7).join(' | ') === 'Its tables | Extras 1 extra | Add a card to backlog | Add a column after done | Arrange the columns | Change their order, or take one away. | Layout Board' && /^Last edited/.test(tail[7]),
  );
  expect('nothing of the board or its head to move as the menu opens', (await still(page)) === before);
  await onScreen(page, expect, menu, "on the board's page");
  // No line lies over another, each is inside the menu, and no line's words are cut short.
  const lined = await menu.evaluate((box) => {
    const edge = box.getBoundingClientRect();
    return [...box.querySelectorAll('button, .menu-said')].map((one) => {
      const at = one.getBoundingClientRect();
      const cut = [...one.querySelectorAll('.truncate, .hint')].filter((part) => part.scrollWidth > part.clientWidth + 0.5).map((part) => part.textContent);
      return { top: at.top, bottom: at.bottom, inside: at.left >= edge.left - 0.5 && at.right <= edge.right + 0.5 && at.top >= edge.top - 0.5 && at.bottom <= edge.bottom + 0.5, cut };
    });
  });
  for (const [nth, one] of lined.entries()) {
    expect(`line ${nth} of the board's gear to be inside its menu: ${JSON.stringify(one)}`, one.inside);
    expect(`line ${nth} of the board's gear to say all of its words: ${JSON.stringify(one.cut)} are cut short`, one.cut.length === 0);
    if (nth > 0) expect(`line ${nth} of the board's gear not to lie over the one before it: ${JSON.stringify([lined[nth - 1], one])}`, one.top >= lined[nth - 1].bottom - 0.5);
  }
  // The heading only says: it is no line to press, every line that is pressed has a drawing before it, and the
  // arrows pass the heading by, from the last line of what the gear had to the first table.
  expect('the heading to be something said and nothing to press', (await menu.locator('.menu-said', { hasText: 'Its tables' }).count()) === 1 && (await menu.getByRole('menuitem', { name: /^Its tables/ }).count()) === 0);
  expect('every line that is pressed to have a drawing before it', (await menu.locator('button .menu-sign').evaluateAll((all) => all.every((one) => one.firstElementChild !== null))) && (await menu.locator('button .menu-sign').count()) === (await menu.locator('button').count()));
  await menu.getByRole('menuitem', { name: 'Measurements of Shop' }).focus();
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to pass the heading by, got ${await focused(page)}`, (await focused(page)) === 'Extras 1 extra');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go on to the first tool, got ${await focused(page)}`, (await focused(page)) === 'Add a card to backlog');
  await page.keyboard.press('End');
  expect(`End to go to the last line that is pressed, got ${await focused(page)}`, (await focused(page)) === 'Layout Board');
  await page.keyboard.press('ArrowDown');
  expect(`the arrow to go round to the first, got ${await focused(page)}`, (await focused(page)) === 'Copy link');
  await page.keyboard.press('Home');
  await page.keyboard.press('ArrowUp');
  expect(`the arrow to go round to the last, got ${await focused(page)}`, (await focused(page)) === 'Layout Board');
  await page.screenshot({ path: `${out}/head-03-gear-on-the-board.png` });

  // Layout, by the keyboard: the menu is put away and the layout's panel hangs under the gear, on the screen, with nothing moved.
  await page.keyboard.press('Enter');
  await hung.waitFor();
  await menu.waitFor({ state: 'detached' });
  expect('nothing of the board or its head to move as the layout opens', (await still(page)) === before);
  await underGear(page, expect, "on the board's page");
  expect('the gear to say something of its own is open', (await gear.getAttribute('aria-expanded')) === 'true');
  expect('the board to be laid out as a board there', (await hung.getByRole('radio', { checked: true }).innerText()).trim() === 'Board');
  // The pointer crosses from the gear to the panel: it stays.
  await gear.hover();
  await hung.getByRole('radio', { name: 'Table' }).hover();
  expect('the layout to stay as the pointer crosses to it', await hung.isVisible());
  await page.screenshot({ path: `${out}/head-03-layout-under-the-gear.png` });
  // Escape puts the panel away and nothing else, and gives the keyboard back to the gear.
  await page.keyboard.press('Escape');
  await hung.waitFor({ state: 'detached' });
  expect('Escape to give the keyboard back to the gear', await gear.evaluate((one) => one === document.activeElement));
  expect('the gear to say nothing of its own is open', (await gear.getAttribute('aria-expanded')) !== 'true');
  expect('the board to be showing still, as it was', (await page.getByRole('heading', { name: 'Shop board', level: 1 }).count()) === 1 && (await still(page)) === before);
  // By the pointer; a press outside puts it away, and so does the gear, which then shows its menu: and the Escape
  // pressed straight after is the menu's, with nothing of the panel left to take it.
  const lay = async () => {
    await gearLine(page, /^Layout/);
    await hung.waitFor();
    await settled(page);
  };
  await lay();
  await page.mouse.click(700, 600);
  await hung.waitFor({ state: 'detached' });
  await lay();
  await gear.click();
  await menu.waitFor();
  await hung.waitFor({ state: 'detached' });
  await page.keyboard.press('Escape');
  await menu.waitFor({ state: 'detached' });
  // A drawer is open beside the board, and the panel over the page: Escape is the panel's first, and the drawer
  // stays; pressed again it is the drawer's. (The gear is reached by the keyboard there: the drawer lies over it.)
  await gearLine(page, /^Extras/);
  const drawer = drawerAtRest(page);
  await drawer.locator('.sits-table table.grid').waitFor();
  expect('nothing of the right of a head in the drawer', (await drawer.locator('.head-acts').count()) === 0);
  await gear.focus();
  await page.keyboard.press('Enter');
  await menu.waitFor();
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await hung.waitFor();
  await settled(page);
  await page.keyboard.press('Escape');
  await hung.waitFor({ state: 'detached' });
  expect('Escape to put the layout away and leave the drawer beside the board', (await drawer.count()) === 1);
  await page.keyboard.press('Escape');
  await drawer.waitFor({ state: 'detached' });
  // What it changes is the view: laid out as a table, where it lists the columns and is no taller than the window
  // has room for, and back; the panel stays where it hangs.
  await lay();
  await hung.getByRole('radio', { name: 'Table' }).click();
  // The board's own table: the drawer's still copy going out has one too, for a moment.
  await page.locator('.view table.grid').waitFor();
  await hung.getByRole('group', { name: 'Columns shown' }).waitFor();
  await underGear(page, expect, 'once the board is laid out as a table');
  await hung.getByRole('radio', { name: 'Board' }).click();
  await page.locator('.column').first().waitFor();
  await page.keyboard.press('Escape');
  await hung.waitFor({ state: 'detached' });

  // Each tool does what it says: a card is written in the board's first column, and the columns are asked about,
  // with the caret where a new one is typed when one is to be added and nothing being typed when they are arranged.
  await gearLine(page, /^Add a card/);
  const writing = page.locator('.column').first().getByPlaceholder('What needs doing?');
  await writing.waitFor();
  await writing.press('Escape');
  await writing.waitFor({ state: 'detached' });
  for (const [name, typing] of [[/^Add a column/, true], [/^Arrange the columns/, false]]) {
    await gearLine(page, name);
    const asked = page.getByRole('dialog');
    await asked.getByRole('heading', { name: "The board's columns" }).waitFor();
    // What is offered there lies over the dialog's buttons; Escape puts it away, and only it.
    if (typing) {
      await asked.getByText('Select an option or create one').waitFor();
      await page.keyboard.press('Escape');
      await asked.getByText('Select an option or create one').waitFor({ state: 'detached' });
    } else expect('nothing to be typed where the columns are only arranged', (await asked.getByText('Select an option or create one').count()) === 0);
    await asked.getByRole('button', { name: 'Cancel', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
  }
  expect('the board to be as it was once each tool has been put away', (await still(page)) === before);
}

/** The board's gear and the layout it hangs, at whatever width the page is: on the screen, under the gear, over its own page, moving nothing. */
export async function boardGearAt({ page, out, expect }, where, pictured = false) {
  const [menu, hung] = [menuOf(page), hungOn(page)];
  await page.locator('.column').first().waitFor();
  const stood = await still(page);
  await openGear(page);
  await onScreen(page, expect, menu, `on the board ${where}`);
  const lines = await linesOf(page);
  expect(`the board's gear to hold its tables and its tools ${where}, got ${lines.join(' | ')}`, lines.includes('Its tables') && lines.includes('Add a card to backlog') && lines.includes('Layout Board'));
  expect(`nothing of the board to move as its gear opens ${where}`, (await still(page)) === stood);
  await menu.getByRole('menuitem', { name: /^Layout/ }).click();
  await hung.waitFor();
  const [panel, split] = [await underGear(page, expect, `on the board ${where}`), await page.locator('.split').first().boundingBox()];
  expect(`the layout to be over its own page and not past its left edge ${where}: ${JSON.stringify(panel)} in ${JSON.stringify(split)}`, panel.x >= split.x - 0.5);
  expect(`nothing of the board to move as the layout opens ${where}`, (await still(page)) === stood);
  if (pictured) await page.screenshot({ path: `${out}/head-10-narrow-layout.png` });
  await settled(page);
  await page.keyboard.press('Escape');
  await hung.waitFor({ state: 'detached' });
}

/** The board's gear on a phone's page, by touch: its lines and the layout it opens are on the screen, the layout under the gear, each put away by a touch elsewhere. */
export async function boardGearOnPhone({ page, out, expect }) {
  const [gear, menu, room] = [gearOf(page), menuOf(page), page.viewportSize()];
  const fits = async (one, what) => {
    const at = await one.boundingBox();
    expect(`${what} to be on a phone's screen: ${JSON.stringify(at)}`, at.x >= -0.5 && at.y >= -0.5 && at.x + at.width <= room.width + 0.5 && at.y + at.height <= room.height + 0.5);
    return at;
  };
  await page.locator('.column').first().waitFor();
  expect(`a phone's head of the board to have no button for its tables or its tools; got ${(await headButtons(page)).join(' | ')}`, !(await headButtons(page)).some((one) => /^(Its tables|Tools|Layout)/.test(one)));
  await openGear(page, 'tap');
  await fits(menu, "the board's gear");
  const lines = await menu.locator('button .truncate').allInnerTexts();
  expect(`a phone's gear of the board to hold its tables and its tools, got ${lines.join(' | ')}`, ['Extras', 'Add a card', 'Add a column', 'Arrange the columns', 'Layout'].every((one) => lines.includes(one)));
  await page.screenshot({ path: `${out}/head-11-phone-board-gear.png` });
  await page.locator('.view-head h1').tap();
  await menu.waitFor({ state: 'detached' });
  await gearLine(page, /^Layout/, 'tap');
  const layout = page.getByRole('dialog', { name: 'Layout' });
  await layout.waitFor();
  const [laid, by] = [await fits(layout, 'the layout'), await gear.boundingBox()];
  expect(`the layout to be under the gear on a phone: ${JSON.stringify(laid)} under ${JSON.stringify(by)}`, laid.y >= by.y + by.height && laid.y - (by.y + by.height) <= 24);
  await page.screenshot({ path: `${out}/head-11-phone-layout.png` });
  await page.locator('.view-head h1').tap();
  await layout.waitFor({ state: 'detached' });
}
