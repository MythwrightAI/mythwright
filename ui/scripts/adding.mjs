// How a check comes to the one dialog that adds to a channel: as a person
// does, by the participants in the channel's head and the line at the foot
// of the list they open (`Participants` in `src/views/participants.tsx`).
// The line says what may be added (`lineCalled` in `src/memberadded.ts`),
// so it is found by where it is and not by its words.

/** The dialog the line opens, once it is open. */
export const addingDialog = (page) => page.locator('.modal', { has: page.getByRole('heading', { name: 'Add or invite channel member' }) });

/** The line that adds somebody, in the list of who is in the channel showing, once that list is open. */
export const addLine = (page) => page.getByRole('menu', { name: 'Participants' }).locator('.people-add');

/**
 * Opens the list of who is in the channel showing and presses the line at its foot, by a finger where `tap` is said,
 * and hands back the dialog it opened, open.
 */
export async function openAdding(page, { tap = false } = {}) {
  const press = (locator) => (tap ? locator.tap() : locator.click());
  await press(page.locator('.channel > .split > .view-head').getByRole('button', { name: /participants?$/ }));
  await press(addLine(page));
  const dialog = addingDialog(page);
  await dialog.waitFor();
  return dialog;
}
