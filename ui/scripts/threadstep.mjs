// A thread's composer a clear and constant step higher than the channel's beside it: run by
// `carried.mjs`, which hands it its page and the command line of its factory. A channel of its own, with
// nobody in it who answers, has a thread long enough that the thread's box is at the foot of the panel it is
// read in; the foot of that box stands `--thread-step` over the foot of the channel's, at every width, as
// either box grows with what is typed (the line of who is writing over each takes its room whether or not
// anybody is, and so moves neither foot).

import { toolsAtRest } from './drawer.mjs';
import { unfoldChannels } from './folded.mjs';

export async function threadStep({ page, expect, cli, out }) {
  await cli('channel', 'create', 'stepped');
  const root = JSON.parse((await cli('post', 'stepped', 'Where does the till go?', '--json')).stdout).id;
  for (let n = 1; n <= 24; n++) await cli('post', 'stepped', `By the door, perhaps, reply ${n}.`, '--thread', root);
  for (let n = 1; n <= 12; n++) await cli('post', 'stepped', `Something else said in the channel, ${n}.`);
  // The workspace's own channels are a branch of the panel that was not there when it was unfolded: unfolded again
  // once the page has heard of the channel, which it lists in a branch that starts folded.
  // The side panel may have been put away at a phone's width before this: it is shown, at the channels.
  const messaging = page.getByRole('button', { name: 'Channels', exact: true });
  if ((await messaging.getAttribute('aria-pressed')) !== 'true') await messaging.click();
  const stepped = page.locator('.nav').getByText('stepped', { exact: true }).first();
  for (let tried = 0; (await stepped.count()) === 0 && tried < 20; tried++) {
    await unfoldChannels(page);
    if ((await stepped.count()) === 0) await page.waitForTimeout(250);
  }
  await stepped.click();
  const asked = page.locator('.channel > .split .message', { hasText: 'Where does the till go?' });
  await asked.hover();
  await asked.getByRole('button', { name: /repl/ }).first().click();
  await page.locator('.thread .messages .message').nth(24).waitFor();
  await toolsAtRest(page).waitFor();
  // Where one writes has no edge at the desk's widths, under the channel and in the thread.
  const edges = await page.evaluate(() => [...document.querySelectorAll('.channel .composer-box')].map((box) => [getComputedStyle(box).boxShadow, getComputedStyle(box).borderTopWidth]));
  expect(`where one writes to have no edge, got ${JSON.stringify(edges)}`, edges.length === 2 && edges.every(([shadow, border]) => shadow === 'none' && border === '0px'));
  const step = await page.evaluate(() => parseFloat(getComputedStyle(document.querySelector('.channel')).getPropertyValue('--thread-step')));
  expect(`a step between the two composers, got ${step}`, step >= 8);
  const feet = () =>
    page.evaluate(() => {
      const foot = (selector) => document.querySelector(selector)?.getBoundingClientRect().bottom ?? NaN;
      return { channel: foot('.channel > .split > .composer > .composer-box'), thread: foot('.thread .composer > .composer-box') };
    });
  const usual = page.viewportSize();
  const held = async (said) => {
    const at = await feet();
    expect(`${said}: the thread's composer to stand ${step}px over the channel's, got its foot at ${at.thread} and the channel's at ${at.channel}`, Math.abs(at.channel - at.thread - step) <= 0.5);
  };
  for (const width of [1920, 1500, 1280, 1100]) {
    await page.setViewportSize({ width, height: usual.height });
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    await held(`at ${width} wide`);
  }
  await page.setViewportSize(usual);
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  // Each grown with what is typed, the one more than the other, and the other way round.
  const channelBox = page.locator('.channel > .split').getByLabel('Message');
  const threadBox = page.locator('.thread').getByLabel('Reply');
  await channelBox.fill('one\ntwo\nthree\nfour');
  await held('the channel\'s composer grown with what is typed');
  await threadBox.fill('one\ntwo\nthree\nfour\nfive\nsix');
  await held('both grown');
  await channelBox.fill('');
  await held('the thread\'s alone grown');
  await page.screenshot({ path: `${out}/carried-9-thread-step.png` });
  await threadBox.fill('');
}
