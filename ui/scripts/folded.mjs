// Of what the Resources panel lists, three parts start unfolded
// (Projects, Factory, Blueprints) and everything else starts folded
// (`src/folds.ts`): Pinned and Recents, each project, and everything
// under those. A check that goes through the panel
// unfolds what it needs as a person would: by pressing Expand all at the
// panel's head (`unfoldAll`), and, for one fold of something that came
// to be after that, by its arrow (`unfold`), which leaves one that is
// open already as it is.
//
// Expand all unfolds what was read when it is pressed and nothing more,
// so `unfoldAll` presses it once the factory's blueprints are listed, and
// a check that makes a project afterwards presses it again, or unfolds
// that project.

/** The Resources panel, where it is showing. */
export const resourcesOf = (page) => page.locator('nav.sidebar[aria-label="Resources"]');

/**
 * Presses Expand all in the Resources panel, showing the panel first
 * where it is not, and then shows what was showing before. It waits for
 * every part of the panel to be unfolded, and for the factory's
 * blueprints to be read and unfolded too.
 */
export async function unfoldAll(page) {
  const panel = resourcesOf(page);
  const rail = page.locator('.rail');
  // Which view the panel shows, read as it is now and waited for nowhere: none where the panel is put away.
  await rail.waitFor({ state: 'attached' });
  const shown = await rail.evaluate((one) => one.querySelector('.rail-button[aria-pressed="true"]')?.getAttribute('aria-label') ?? null);
  const resources = rail.getByRole('button', { name: 'Resources', exact: true });
  if (shown !== 'Resources') await resources.click();
  await panel.waitFor();
  const press = () => panel.getByRole('button', { name: 'Expand all', exact: true }).click();
  await press();
  await panel.locator('h3 .section-fold[aria-expanded="false"]').first().waitFor({ state: 'detached' });
  // The factory's blueprints are listed once they are read: pressed again then, their families and parts unfold too.
  await panel.locator('.nav-note', { hasText: 'Reading what there is' }).waitFor({ state: 'detached' });
  await press();
  if (shown !== 'Resources') await (shown ? rail.getByRole('button', { name: shown, exact: true }) : resources).click();
}

/**
 * Unfolds one fold of the panel by its arrow, called `name` as its arrow
 * says (`Unfold Shop`, `Unfold Shop repo`, `Unfold the parts of
 * std/board`), or a line that folds (a family of blueprints, a project's
 * Blueprints) handed as a locator; one that is open already is left so.
 */
export async function unfold(page, what) {
  if (typeof what !== 'string') {
    if ((await what.getAttribute('aria-expanded')) !== 'true') await what.click();
    await what.and(page.locator('[aria-expanded="true"]')).waitFor();
    return;
  }
  const open = page.getByRole('button', { name: `Fold ${what}`, exact: true });
  const shut = page.getByRole('button', { name: `Unfold ${what}`, exact: true });
  await open.or(shut).first().waitFor();
  if ((await open.count()) === 0) await shut.first().click();
  await open.first().waitFor();
}

/** The Channels panel, where it is showing. */
export const channelsOf = (page) => page.locator('nav.sidebar[aria-label="Channels"]');

/**
 * Presses Expand all in the Channels panel, showing the panel first where
 * it is not, and then shows what was showing before. Every branch of the
 * panel starts folded as everything of the Resources panel does (what is
 * pinned, each group, each project's channels, what is archived), and
 * Expand all unfolds each, and every channel with who is in it. It waits
 * for no branch to be folded. A branch that comes to be after it was
 * pressed (a group made, something pinned, a project made) is folded, and
 * a check presses it again or unfolds that branch.
 */
export async function unfoldChannels(page) {
  const panel = channelsOf(page);
  const rail = page.locator('.rail');
  await rail.waitFor({ state: 'attached' });
  const shown = await rail.evaluate((one) => one.querySelector('.rail-button[aria-pressed="true"]')?.getAttribute('aria-label') ?? null);
  const channels = rail.getByRole('button', { name: 'Channels', exact: true });
  if (shown !== 'Channels') await channels.click();
  await panel.waitFor();
  await panel.getByRole('button', { name: 'Expand all', exact: true }).click();
  await panel.locator('.group-row .nav-twist[aria-expanded="false"]:not([aria-disabled]), .space-row .nav-twist[aria-expanded="false"]:not([aria-disabled])').first().waitFor({ state: 'detached' });
  if (shown !== 'Channels') await (shown ? rail.getByRole('button', { name: shown, exact: true }) : channels).click();
}
