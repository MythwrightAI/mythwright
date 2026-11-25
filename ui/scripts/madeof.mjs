// What a blueprint makes, as a table with each table's data model, for `summary.mjs`, which runs it on its pages.
//
//   - what the fleet makes is a table with a row for each thing made,
//     saying what it is, what it is called and, for a table, how many
//     fields it has, as the daemon's own summary hands each (`made`,
//     `model`); a table's model folded under its row and unfolded by its
//     name pressed and by its count, the fields the daemon's;
//   - pointed at, a table's name shows its model in a card over the page,
//     moving nothing, under its name and over nothing else of its line,
//     wholly in the window, staying as the pointer crosses to it and put
//     away by Escape; reached by the keyboard, the same;
//   - a project's own blueprint that makes two tables, one of whose
//     fields names a record of the other, has a part Data model listing
//     both with their fields, the field that names the other leading to
//     that one's model;
//   - by the boxes at three widths and a phone's, where its rows stand a
//     cell under another.

/** A project's own blueprint that makes two tables, the second of which names records of the first. */
export const TABLES = `blueprint: tables
description: Keep who works and what they work on.
permissions: [projects, records]

steps:
  people:
    do: collection.ensure
    with:
      name: people
      fields:
        title: { type: text, required: true }
        level: { type: choice, of: [low, high] }
  jobs:
    do: collection.ensure
    with:
      name: jobs
      beside: people
      about: worker
      fields:
        title: { type: text, required: true }
        worker: { type: record, of: people }
`;

/** What is drawn of the card over the page, and of the name it is about, by their boxes; and whether it moved anything. */
const cardLaid = (on) =>
  on.evaluate(() => {
    const card = document.querySelector('body > .floated.summary-model-card');
    if (!card || card.style.visibility === 'hidden') return null;
    const box = (el) => {
      const at = el.getBoundingClientRect();
      return { left: at.left, right: at.right, top: at.top, bottom: at.bottom };
    };
    const name = document.activeElement?.classList.contains('summary-model-name') ? document.activeElement : document.querySelector('.summary-model-name:hover');
    return {
      card: box(card),
      name: name ? box(name) : null,
      position: getComputedStyle(card).position,
      fields: [...card.querySelectorAll('.summary-field > dt')].map((one) => one.textContent),
      wide: innerWidth,
      tall: innerHeight,
    };
  });

const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;

/** Every box of the Summary showing, to tell that nothing moved. */
const boxes = (on) => on.evaluate(() => [...document.querySelectorAll('.summary-page *')].filter((one) => one.getClientRects().length > 0).map((one) => JSON.stringify(one.getBoundingClientRect())).join('|'));

/**
 * Holds what the fleet makes on the page `on`, which shows std/fleet's
 * Summary as it was opened, to the daemon's own summary of it (`handed`).
 */
export async function fleetMade({ on, handed, expect, shot, scheme, touch = false }) {
  const press = (what) => (touch ? what.tap() : what.click());
  const makes = handed.parts.find((part) => part.kind === 'makes').lines;
  const tables = makes.filter((line) => line.model?.fields?.length);
  expect(`the fleet to make tables with fields, as the daemon says, got ${tables.length}`, tables.length >= 3);
  const part = on.locator('.summary[data-of="std/fleet"] .summary-part[data-part="makes"]');
  const table = part.locator('table.summary-made');
  await table.waitFor();
  // A long list shows its first rows until the rest is asked for.
  const more = part.locator('button.summary-more-rows[aria-expanded="false"]');
  if ((await more.count()) > 0) await press(more);
  expect('what it makes to be a table with its headings', (await table.locator('thead th').allTextContents()).map((one) => one.trim()).join('|') === 'What|Called|Made of|From');
  // A row for each thing made, what it is and what it is called as the daemon hands each.
  const drawn = await part.locator('tr.summary-made-row').evaluateAll((all) => all.map((row) => [row.querySelector('.summary-made-what')?.textContent ?? '', row.querySelector('.summary-made-name .summary-said > .summary-words')?.textContent ?? '', row.querySelector('.summary-made-of')?.textContent ?? '']));
  for (const line of tables) {
    const name = line.made.name.replaceAll('`', '');
    const row = drawn.find(([what, called]) => what === line.made.what && called === name);
    const count = line.model.fields.length;
    expect(`a row for the table ${name}, saying it is ${line.made.what} with ${count} fields, got ${JSON.stringify(drawn)}`, row && row[2] === `${count} fields`);
  }
  if (!touch) {
    // ---- pointed at, its model in a card over the page --------------------------------
    const agents = tables.find((line) => line.made.called === 'agents');
    const name = part.locator('tr.summary-made-row', { has: on.locator('.summary-made-name .summary-said > .summary-words', { hasText: /^agents$/ }) }).locator('.summary-model-name');
    await name.scrollIntoViewIfNeeded();
    const before = await boxes(on);
    await name.hover();
    await on.locator('body > .floated.summary-model-card').waitFor();
    await on.waitForFunction(() => document.querySelector('body > .floated.summary-model-card')?.style.visibility !== 'hidden');
    const shown = await cardLaid(on);
    expect(`the card to lie over the page, got ${shown?.position}`, shown && shown.position === 'fixed');
    expect(`the card to say the table's fields as the daemon does, got ${shown.fields.join(', ')}`, shown.fields.join('|') === agents.model.fields.map((field) => field.name).join('|'));
    expect(`all of the card in the window, got ${JSON.stringify(shown.card)}`, shown.card.left >= 0 && shown.card.top >= 0 && shown.card.right <= shown.wide && shown.card.bottom <= shown.tall);
    expect(`the card over nothing of the name it is about, got ${JSON.stringify(shown)}`, shown.name && !over(shown.card, shown.name));
    expect('nothing of the Summary to move for the card', (await boxes(on)) === before);
    await shot(`summary-model-card-${scheme}`);
    // It stays as the pointer crosses to it.
    const card = on.locator('body > .floated.summary-model-card');
    const [at, to] = [await name.boundingBox(), await card.boundingBox()];
    await on.mouse.move(Math.min(Math.max(at.x + 4, to.x + 6), to.x + to.width - 6), to.y + 6, { steps: 6 });
    await on.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 6 });
    await on.waitForTimeout(500);
    expect('the card to stay while the pointer is on it', (await card.count()) === 1);
    await on.keyboard.press('Escape');
    await card.waitFor({ state: 'detached' });
    // Reached by the keyboard, the same; Escape puts it away.
    await on.mouse.move(0, 0);
    await name.focus();
    await card.waitFor();
    const keyed = await cardLaid(on);
    expect(`the card shown for the keyboard as for the pointer, got ${JSON.stringify(keyed)}`, keyed && keyed.fields.length === agents.model.fields.length && !over(keyed.card, keyed.name));
    await on.keyboard.press('Escape');
    await card.waitFor({ state: 'detached' });
    // Pressed, its model unfolded under its row, and folded again.
    await press(name);
    const row = part.locator('tr.summary-model-row');
    await row.waitFor();
    expect('the model unfolded under its row to be the daemon\'s', (await row.locator('.summary-field > dt').allInnerTexts()).join('|') === agents.model.fields.map((field) => field.name).join('|'));
    await shot(`summary-model-unfolded-${scheme}`);
    await press(name);
    await row.waitFor({ state: 'detached' });
  }
}

/** Every row of what it makes by its cells' boxes: cut off by nothing, and at a phone's width a cell under another. */
export async function madeLaid({ on, expect, when, stacked }) {
  const now = await on.evaluate(() => {
    const shown = [...document.querySelectorAll('.summary')].find((one) => one.getClientRects().length > 0);
    const table = shown.querySelector('table.summary-made');
    if (!table) return null;
    const column = shown.querySelector('.summary-page').getBoundingClientRect();
    const rows = [...table.querySelectorAll('tr.summary-made-row')].map((row) => [...row.children].map((cell) => cell.getBoundingClientRect()).filter((one) => one.height > 0));
    return { column: { left: column.left, right: column.right }, rows: rows.map((cells) => cells.map((one) => ({ left: one.left, right: one.right, top: one.top, bottom: one.bottom }))) };
  });
  if (!now) return;
  for (const cells of now.rows) {
    for (const cell of cells) expect(`a cell of what it makes inside the column ${when}, got ${JSON.stringify(cell)} in ${JSON.stringify(now.column)}`, cell.left >= now.column.left - 0.5 && cell.right <= now.column.right + 0.5);
    cells.forEach((a, at) => cells.slice(at + 1).forEach((b) => expect(`no cell of a row of what it makes over another ${when}`, !over(a, b))));
    if (stacked) cells.slice(1).forEach((b, at) => expect(`each cell under the one before it ${when}`, b.top >= cells[at].bottom - 0.5));
  }
}

/** Holds the part Data model of a project's own blueprint that makes two tables, on `on`, which shows its Summary. */
export async function dataModel({ on, handed, expect, shot, scheme }) {
  const part = on.locator('.summary[data-of="tables"] .summary-part[data-part="model"]');
  await part.waitFor();
  const made = handed.parts.find((one) => one.kind === 'makes').lines.filter((line) => line.model?.fields?.length);
  const listed = await part.locator('[data-model]').evaluateAll((all) => all.map((one) => [one.dataset.model, [...one.querySelectorAll('.summary-field > dt')].map((dt) => dt.textContent).join(',')]));
  expect(`the Data model to list each table with its fields as the daemon says, got ${JSON.stringify(listed)}`, JSON.stringify(listed) === JSON.stringify(made.map((line) => [line.made.called, line.model.fields.map((field) => field.name).join(',')])));
  const link = part.locator('[data-model="jobs"] .summary-field[data-field="worker"] button.summary-field-kind');
  expect('the field that names a record of the other table to lead there', (await link.count()) === 1 && (await link.innerText()).includes('people'));
  await link.click();
  await on.waitForFunction(() => document.activeElement?.dataset?.model === 'people');
  await shot(`summary-data-model-${scheme}`);
}
