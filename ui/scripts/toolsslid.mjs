// The Tools panel beside a channel comes in and goes out as the drawer does (`views/toolsslide.ts`), which
// `tools.mjs` runs with a channel open beside its panel: opened by its button, the room taken at once (the
// channel laid out once and not again as the panel moves) and the panel sliding into it by where it is drawn
// alone, in the drawer's time and easing, held part of the way to be looked at, nothing scrolled sideways,
// and at rest only once it is; closed, the room given back at once and a still copy of it going out the way
// it came over the channel's right edge, nothing to press or reach; a channel shown again with its panel, as
// its tab is shown again, having it there at once and not coming in again; the same on a phone, where it has
// the screen; and where less movement is asked for, nothing moving and no copy put there.
import { toolsAtRest, watchToolsComing, watchToolsLeaving } from './drawer.mjs';

/** Plays on whatever was held on its way, the panel or its copy. */
const playOn = (page) =>
  page.evaluate(() => {
    window.toolsUnwatched?.();
    for (const one of document.querySelectorAll('.channel > .context, .context-leaving')) for (const moving of one.getAnimations()) moving.play();
  });

/** The channel's own part (what is said and written there): where it stands and how wide, which is what a reader is looking at. */
const behind = (page) =>
  page.evaluate(() => {
    const split = document.querySelector('.channel > .split').getBoundingClientRect();
    return [split.x, split.width].map(Math.round).join();
  });

/** How often the channel has been drawn. */
const drawn = (page) => page.evaluate(() => window.__drawn?.channel ?? 0);

/** Whether anything of the page was scrolled sideways, or could be. */
const sideways = (page) =>
  page.evaluate(() => {
    const main = document.querySelector('.main');
    const channel = document.querySelector('.channel');
    return [main.scrollLeft, channel.scrollLeft, document.documentElement.scrollWidth - window.innerWidth, document.documentElement.scrollLeft].filter((one) => one > 0);
  });

/** The panel's box against the channel's. */
const boxes = (page) =>
  page.evaluate(() => {
    const channel = document.querySelector('.channel').getBoundingClientRect();
    const panel = document.querySelector('.channel > .context, .context-leaving').getBoundingClientRect();
    return { channel: { x: channel.x, right: channel.right, width: channel.width }, panel: { x: panel.x, right: panel.right, width: panel.width, y: panel.y, height: panel.height } };
  });

export async function toolsSlid({ page, out, expect, cli, channel }) {
  const panel = page.getByRole('complementary', { name: 'Tools' });
  const fold = (name) => page.getByRole('button', { name: `${name} the tools panel` });
  const opened = async (where) => {
    await watchToolsComing(page);
    const was = await drawn(page);
    await fold('Expand').click();
    await panel.waitFor();
    const slid = await page.evaluate(() => window.toolsSlid);
    expect(
      `${where}: the panel to come in by where it is drawn and nothing else, in the drawer's time and easing, got ${JSON.stringify(slid)}`,
      slid.name === 'drawer-in' && slid.took >= 240 && slid.took <= 280 && slid.moved.join() === 'transform' && slid.eased === 'cubic-bezier(0.22, 0.61, 0.36, 1)',
    );
    expect(`${where}: a panel on its way not to say that it is at rest`, !slid.rested && (await toolsAtRest(page).count()) === 0);
    return was;
  };

  // ---- opened: the room taken at once, and the panel sliding into it -----------------------------------
  await fold('Collapse').click();
  await panel.waitFor({ state: 'detached' });
  await playOn(page);
  await page.locator('.context-leaving').waitFor({ state: 'detached' });
  const wide = await behind(page);
  const was = await opened('opened');
  const taken = await page.evaluate(() => window.toolsBehind);
  const onItsWay = await boxes(page);
  expect(`the panel to be part of the way out of the channel's right edge, at ${JSON.stringify(onItsWay)}`, onItsWay.panel.x < onItsWay.channel.right - 40 && onItsWay.panel.right > onItsWay.channel.right + 40);
  expect(`the room to be taken at once, the channel narrower as the panel is put there, got ${taken} from ${wide}`, taken !== wide && (await behind(page)) === taken);
  expect(`nothing to be scrolled sideways while it comes, got ${await sideways(page)}`, (await sideways(page)).length === 0);
  await page.screenshot({ path: `${out}/tools-0a-coming.png` });
  await playOn(page);
  await toolsAtRest(page).waitFor();
  const rests = await toolsAtRest(page).evaluate((one) => [getComputedStyle(one).transform, one.getAnimations().length, Math.round(one.parentElement.getBoundingClientRect().right - one.getBoundingClientRect().right)]);
  expect(`the panel at rest to be where it always was, moved by nothing any more, got ${rests}`, rests.join() === 'none,0,0');
  expect(`the channel not to have moved as the panel came, got ${await behind(page)} from ${taken}`, (await behind(page)) === taken);
  const opening = (await drawn(page)) - was;
  expect(`the channel to be drawn once for the panel opened, and it was ${opening} times`, opening === 1);
  await page.screenshot({ path: `${out}/tools-0b-at-rest.png` });

  // ---- shown again as its tab is shown again: there at once, at rest, not coming in again ---------------
  await cli('channel', 'create', 'elsewhere');
  const side = page.getByRole('navigation', { name: 'Channels' });
  await side.getByText('elsewhere', { exact: true }).click();
  await page.getByRole('heading', { name: '# elsewhere' }).waitFor();
  await toolsAtRest(page).waitFor();
  await watchToolsComing(page);
  await side.getByText(channel, { exact: true }).click();
  await page.getByRole('heading', { name: `# ${channel}` }).waitFor();
  await toolsAtRest(page).waitFor();
  const back = await page.evaluate(() => window.toolsSlid);
  expect(`a panel given back with its channel to be there at once and at rest, not coming in, got ${JSON.stringify(back)}`, back.name === 'none' && back.rested === true);
  await page.evaluate(() => window.toolsUnwatched());

  // ---- closed: the room given back at once, and a still copy going out over it -------------------------
  await watchToolsLeaving(page);
  const heldAt = (await boxes(page)).panel;
  const before = await drawn(page);
  await fold('Collapse').click();
  await page.locator('.context-leaving').waitFor();
  const went = await page.evaluate(() => window.toolsLeft);
  expect(
    `a still copy of the panel that is closed to go out the way it came, in the drawer's time, as nothing to press or reach, got ${JSON.stringify(went)}`,
    went.name === 'drawer-in' && went.backwards && went.took >= 240 && went.took <= 280 && went.eased === 'cubic-bezier(0.22, 0.61, 0.36, 1)' && went.inert && went.hidden === 'true' && went.pressed === 'none' && went.found === 0,
  );
  expect('no panel to be found while its copy goes out', (await panel.count()) === 0 && (await page.locator('.context').count()) === 0);
  const going = await boxes(page);
  expect(`the copy to be part of the way out of the channel's right edge, the size the panel was, at ${JSON.stringify(going)} from ${JSON.stringify(heldAt)}`, going.panel.x > heldAt.x + 40 && Math.abs(going.panel.width - heldAt.width) < 1 && Math.abs(going.panel.y - heldAt.y) < 1 && Math.abs(going.panel.height - heldAt.height) < 1);
  expect(`the room to be given back at once, got ${await behind(page)} where it was ${wide} before the panel was opened`, (await behind(page)) === wide);
  expect(`nothing to be scrolled sideways while it goes, got ${await sideways(page)}`, (await sideways(page)).length === 0);
  await page.screenshot({ path: `${out}/tools-0c-going.png` });
  await playOn(page);
  await page.locator('.context-leaving').waitFor({ state: 'detached' });
  expect(`the channel not to have moved as the copy went, got ${await behind(page)}`, (await behind(page)) === wide);
  const closing = (await drawn(page)) - before;
  expect(`the channel to be drawn once for the panel closed, and it was ${closing} times`, closing === 1);

  // ---- on a phone, where it has the screen, the same ----------------------------------------------------
  const usual = page.viewportSize();
  await page.setViewportSize({ width: 390, height: 844 });
  await opened('on a phone');
  const phoneWay = (await boxes(page)).panel;
  expect(`on a phone the panel to be part of the way in from the screen's right edge, at ${JSON.stringify(phoneWay)}`, phoneWay.x > 40 && phoneWay.x < 390 - 40 && Math.round(phoneWay.width) === 390);
  expect(`nothing to be scrolled sideways on a phone, got ${await sideways(page)}`, (await sideways(page)).length === 0);
  await page.screenshot({ path: `${out}/tools-0d-phone-coming.png` });
  await playOn(page);
  await toolsAtRest(page).waitFor();
  const phoneAt = await toolsAtRest(page).boundingBox();
  expect(`on a phone the panel at rest to have the screen, got ${JSON.stringify(phoneAt)}`, phoneAt.x === 0 && phoneAt.width === 390);
  await watchToolsLeaving(page);
  await panel.getByRole('button', { name: 'Back to the channel' }).click();
  await page.locator('.context-leaving').waitFor();
  const phoneGoing = await page.locator('.context-leaving').boundingBox();
  const phoneWent = await page.evaluate(() => window.toolsLeft);
  expect(`on a phone a copy to go out the way it came over the screen, at ${JSON.stringify(phoneGoing)}, got ${JSON.stringify(phoneWent)}`, phoneWent.backwards && phoneGoing.x > 40 && Math.round(phoneGoing.width) === 390);
  await page.screenshot({ path: `${out}/tools-0e-phone-going.png` });
  await playOn(page);
  await page.locator('.context-leaving').waitFor({ state: 'detached' });
  await page.setViewportSize(usual);

  // ---- where less movement is asked for, nothing moves and no copy is put there -------------------------
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await watchToolsComing(page);
  await fold('Expand').click();
  await toolsAtRest(page).waitFor();
  const still = await page.evaluate(() => window.toolsSlid);
  expect(`a panel to be moved by nothing there, and at rest as it is drawn, got ${JSON.stringify(still)}`, still.name === 'none' && still.rested === true);
  await page.evaluate(() => window.toolsUnwatched());
  await fold('Collapse').click();
  await panel.waitFor({ state: 'detached' });
  expect('no copy of the panel to be left where nothing moves one', (await page.locator('.context-leaving').count()) === 0);
  await page.emulateMedia({ reducedMotion: 'no-preference' });

  // And it is open again, as it was found.
  await fold('Expand').click();
  await playOn(page);
  await toolsAtRest(page).waitFor();
}
