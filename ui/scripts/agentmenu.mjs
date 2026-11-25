// What is pressed on an agent's line of the Members page's Agents tab: the menu at its
// end (`More` in `views/grouped.tsx`, given the keyboard), whose first
// line is the agent's setup, its history after it, and whose last, for
// whoever may, is taking it away. `agents.mjs` and `external.mjs` go through it.

/** An agent's line of the list. */
export const agentLine = (on, name) => on.locator(`.agent-line[data-agent="${name}"]`);

/** What opens the menu at the end of an agent's line. */
export const menuOpener = (on, name) => agentLine(on, name).getByRole('button', { name: `More that can be done with ${name}`, exact: true });

/** Opens the menu at the end of an agent's line, by `press` (`click`, `tap`). */
export async function openMenu(on, name, press = 'click') {
  await menuOpener(on, name)[press]();
  await on.getByRole('menu').waitFor();
}

/**
 * Opens an agent's history from the menu at the end of its line, where it is the second line, after Edit…: pressed by the pointer
 * (`click`, `tap`), or by the keyboard (`keys`: Enter on the menu, which takes the keyboard at its first line, the arrow down to it, and
 * Enter there), which `expect` holds.
 */
export async function openHistory(on, name, how, expect) {
  const first = on.getByRole('menu').getByRole('menuitem', { name: 'History', exact: true });
  if (how === 'keys') {
    await menuOpener(on, name).focus();
    await on.keyboard.press('Enter');
    await first.waitFor();
    await on.waitForFunction(() => document.activeElement?.closest('.row-menu') !== null, null, { timeout: 5000 }).catch(() => undefined);
    const edit = on.getByRole('menu').getByRole('menuitem', { name: 'Edit…', exact: true });
    expect(`the menu to take the keyboard at its first line, Edit…, for ${name}`, await edit.evaluate((one) => document.activeElement === one));
    await on.keyboard.press('ArrowDown');
    expect(`the arrow to bring the keyboard to History, for ${name}`, await first.evaluate((one) => document.activeElement === one));
    await on.keyboard.press('Enter');
  } else {
    await menuOpener(on, name)[how]();
    await first[how]();
  }
  await on.getByRole('menu').waitFor({ state: 'detached' });
}

/** Opens the question that removes an agent, from the menu at the end of its line, and answers the dialog. */
export async function askRemoval(on, name, press = 'click') {
  await openMenu(on, name, press);
  await on.getByRole('menuitem', { name: 'Remove…', exact: true })[press]();
  const dialog = on.getByRole('dialog');
  await dialog.getByRole('heading', { name: `Remove ${name}?` }).waitFor();
  return dialog;
}
