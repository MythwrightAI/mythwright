// What a bot says again and again in a channel, folded under the latest of
// it (`src/folded.ts`), which `channel.mjs` runs: a blueprint's
// status applied from a terminal five times is one message with "4 earlier
// updates" under it; one that names somebody stands by itself and ends the
// run before it; what comes after folds again; pressed, the earlier ones
// are drawn over the latest with the press staying under the pointer, and
// it is kept by the tab while another is shown; and the press over nothing
// else of the message at three widths and a phone's. And the status's ✅
// and ❌ drawn in letters that have them (`EMOJI` in `src/looks.ts`), a
// look a browser kept from before them worn with them at once.
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

export async function updates({ page, out, expect, cli, inChannel }) {
  // A status said each time it is asked, as one said every ten minutes is: one step of one running blueprint, again and again.
  await writeFile(
    join(out, 'status.bp'),
    [
      'blueprint: status',
      'description: Says how the work stands each time it is asked.',
      'permissions: [channels]',
      'on:',
      '  pinged:',
      '    event: status.pinged',
      '    steps:',
      '      heard:',
      '        do: channel.post',
      '        with:',
      '          channel: general',
      '          body: "Listening for status."',
      '  asked:',
      '    event: status.asked',
      '    steps:',
      '      told:',
      '        do: channel.post',
      '        with:',
      '          channel: general',
      '          body: "${event.data.said}"',
      '',
    ].join('\n'),
  );
  await cli('apply', join(out, 'status.bp'));
  // Its handlers listen a moment after it is applied: asked until it is heard, by another step than the status's.
  for (let tries = 0; (await inChannel.filter({ hasText: 'Listening for status.' }).count()) === 0; tries += 1) {
    if (tries > 30) throw new Error('expected the status to be listening');
    await cli('emit', 'status.pinged');
    await page.waitForTimeout(300);
  }
  const say = (said) => cli('emit', 'status.asked', `said=${said}`);
  const status = (n) => `Status ${n}: ✅ build passed, ❌ deploy waiting.`;
  for (const n of [1, 2, 3, 4, 5]) {
    await say(status(n));
    await inChannel.filter({ hasText: `Status ${n}:` }).first().waitFor({ state: 'attached' });
  }
  await say('Status 6: ❌ the deploy failed, @ada please look.');
  await inChannel.filter({ hasText: 'Status 6:' }).waitFor({ state: 'attached' });
  for (const n of [7, 8]) {
    await say(status(n));
    await inChannel.filter({ hasText: `Status ${n}:` }).first().waitFor({ state: 'attached' });
  }
  const shown = (text) => inChannel.filter({ hasText: text });
  await shown('Status 8:').waitFor();
  // By what each holds and not by its words as drawn: a message out of sight is not laid out, and has none drawn.
  const drawn = async () => (await inChannel.evaluateAll((all) => all.map((one) => one.querySelector('.body')?.textContent.match(/^Status (\d):/)?.[1]).filter(Boolean))).join(',');
  expect(`the status said five times to be its latest, the one that names somebody by itself, and the two after it their latest, got ${await drawn()}`, (await drawn()) === '5,6,8');
  const fifth = shown('Status 5:');
  const press = fifth.locator('.earlier-updates');
  expect(`the latest of five to say how many came before it, got "${await press.innerText()}"`, (await press.innerText()) === '4 earlier updates' && (await press.getAttribute('aria-expanded')) === 'false');
  expect('the one that names somebody to have nothing folded under it', (await shown('Status 6:').locator('.earlier-updates').count()) === 0);
  expect(`the two after it to fold under the second, got "${await shown('Status 8:').locator('.earlier-updates').innerText()}"`, (await shown('Status 8:').locator('.earlier-updates').innerText()) === '1 earlier update');
  // Its letters end with the systems' emoji, whatever the look's letters are.
  const letters = await fifth.locator('.body').evaluate((one) => getComputedStyle(one).fontFamily);
  expect(`what a message is written in to end with the systems' emoji, got ${letters}`, letters.includes('Noto Color Emoji') && letters.includes('Segoe UI Emoji'));
  // A look a browser kept from before the emoji ended each stack of letters is worn with them at once, as the page is first
  // drawn (its letters as the page puts them on, before anything else of the page is run).
  const old = '"Iowan Old Style", "Palatino Linotype", Palatino, "URW Palladio L", P052, Georgia, Cambria, serif';
  // A browser of its own, which kept that look before the page was first read.
  const elsewhere = await page.context().browser().newContext();
  const early = await elsewhere.newPage();
  await early.addInitScript((stack) => {
    try {
      localStorage.setItem('client.theme.look', JSON.stringify({ '--sans': stack }));
    } catch {
      /* a document that keeps nothing */
    }
    window.firstSans = null;
    // On the document and not its root, which is not yet the page's own as this is run.
    new MutationObserver(() => {
      window.firstSans ??= document.documentElement?.style.getPropertyValue('--sans') || null;
    }).observe(document, { subtree: true, attributes: true, attributeFilter: ['style'] });
  }, old);
  await early.goto(page.url());
  const firstSans = await early.evaluate(() => window.firstSans);
  await elsewhere.close();
  expect(`a kept look's letters to be worn with the systems' emoji as the page is first drawn, got ${firstSans}`, firstSans === `${old}, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji", emoji`);
  await fifth.scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${out}/channel-21-updates-folded.png` });

  // The press under the pointer stays under it as what came before is drawn over it.
  await press.hover();
  const was = await press.boundingBox();
  await press.click();
  await shown('Status 1:').waitFor();
  const now = await press.boundingBox();
  expect(`the press to stay under the pointer, from ${JSON.stringify(was)} to ${JSON.stringify(now)}`, Math.abs(now.y - was.y) <= 1 && Math.abs(now.x - was.x) <= 1);
  expect(`all five said, in order, got ${await drawn()}`, (await drawn()) === '1,2,3,4,5,6,8');
  expect(`the press to say it hides them now, got "${await press.innerText()}"`, (await press.innerText()) === 'Hide 4 earlier updates' && (await press.getAttribute('aria-expanded')) === 'true');
  await page.screenshot({ path: `${out}/channel-22-updates-shown.png` });

  // Kept by the tab while another is shown.
  const here = await page.locator('.tab[aria-selected="true"]').first().getAttribute('data-key');
  const other = page.locator('.tab:not([aria-selected="true"])').first();
  if ((await other.count()) > 0) {
    await other.click();
    await page.locator(`.tab[data-key="${here}"][aria-selected="false"]`).waitFor();
    await page.locator(`.tab[data-key="${here}"]`).click();
    await shown('Status 8:').waitFor();
    expect(`what was shown to be shown again with its tab, got ${await drawn()}`, (await drawn()) === '1,2,3,4,5,6,8');
  }

  // The press is under the words of its message and over nothing else of it, at three widths and a phone's.
  const { width, height } = page.viewportSize();
  for (const wide of [1280, 900, 640, 390]) {
    await page.setViewportSize({ width: wide, height: 860 });
    const fifthNow = shown('Status 5:');
    await fifthNow.scrollIntoViewIfNeeded();
    const laid = await fifthNow.evaluate((message) => {
      const box = (one) => one.getBoundingClientRect();
      const press = message.querySelector('.earlier-updates');
      const words = message.querySelector('.body');
      const [p, w, m] = [box(press), box(words), box(message)];
      return { under: p.top >= w.bottom - 0.5, inside: p.left >= m.left - 0.5 && p.right <= m.right + 0.5 && p.bottom <= m.bottom + 0.5, window: p.right <= innerWidth && p.left >= 0, sideways: document.documentElement.scrollWidth - innerWidth };
    });
    expect(`at ${wide} wide the press to be under the words, inside its message and the window, got ${JSON.stringify(laid)}`, laid.under && laid.inside && laid.window && laid.sideways <= 0);
    await page.screenshot({ path: `${out}/channel-23-updates-${wide}.png` });
  }
  await page.setViewportSize({ width, height });
  // Folded again.
  await shown('Status 5:').locator('.earlier-updates').click();
  await shown('Status 1:').waitFor({ state: 'detached' });
  expect(`folded again, got ${await drawn()}`, (await drawn()) === '5,6,8');
}
