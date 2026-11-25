// Something pressed as soon as the frame is there, before it has entered.
//
// Somebody who signs in is shown the frame at once, and the tabs kept for
// them are put back only once the workspaces have been read (`tabsof.ts`):
// what they open in that moment is opened among those tabs (`among` in
// `src/tabskept.ts`). A busy machine comes to that order now and then; a
// check makes it on any machine by holding the reading of the workspaces
// until it has pressed.

/** Signs in by `signIn`, does `press` while the workspaces are not yet read, and then lets them be read and waits until the frame has entered. */
export async function beforeEntering(page, signIn, press) {
  let readThem;
  const held = new Promise((go) => (readThem = go));
  await page.route('**/api/workspaces', async (route) => {
    await held;
    await route.continue().catch(() => {});
  });
  await signIn();
  // Until they are read the foot names no workspace.
  if ((await page.locator('.space-foot .space-said').count()) !== 0) throw new Error('expected the frame not to have entered before the workspaces were read');
  await press();
  readThem();
  await page.locator('.space-foot .space-said').waitFor();
}
