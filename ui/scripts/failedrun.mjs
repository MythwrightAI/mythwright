// A failure said in a channel opens the run that failed, at the step that
// failed, as `channel.mjs` holds it: that check's own part, in
// a file of its own because the check was near its limit of lines. It
// starts no daemon and no browser: it is handed the page `channel.mjs` has
// open on #general, the command line of its factory and its program.
//
// A blueprint of the night's steps, three that go well and one that fails,
// and a handler that says the failure in #general as the fleet says one
// (`slipped` of `std/fleet`: the reason, the whole of it as detail, and the
// run it is about). Opened, the message offers the run that failed first
// and what was running after it; the first opens the run's page as a tab,
// with the stage that failed marked, in view, saying why and what it
// wrote. A message about a run that is not kept says so in its place and
// offers nothing to press for it. Nothing of the details lies over
// anything else of them, and the stage is in view, at three widths and a
// phone's, plainly and at whatever look the check is run with.

import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

/** Each box of what the details hold, to hold that none lies over another. */
const boxes = (where) =>
  where.evaluate((root) =>
    [...root.querySelectorAll(':scope > *')].map((one) => {
      const b = one.getBoundingClientRect();
      return { what: one.className || one.tagName, left: b.left, top: b.top, right: b.right, bottom: b.bottom };
    }),
  );
const crossing = (all) =>
  all.flatMap((a, i) => all.slice(i + 1).filter((b) => a.left < b.right - 1 && b.left < a.right - 1 && a.top < b.bottom - 1 && b.top < a.bottom - 1).map((b) => `${a.what} over ${b.what}`));

export async function failedRun({ page, out, expect, cli, binary }) {
  const night = (name, about) =>
    [
      `blueprint: ${name}`,
      "description: Runs the night's steps, and says in the channel when one fails.",
      'permissions: [channels, shell]',
      'on:',
      '  went:',
      `    event: ${name}.go`,
      '    steps:',
      ...['first', 'second', 'third'].flatMap((step, at) => [
        `      ${step}:`,
        ...(at > 0 ? [`        needs: [${['first', 'second'][at - 1]}]`] : []),
        '        description: Says what the factory takes.',
        '        do: shell',
        '        with:',
        `          cmd: [${JSON.stringify(binary)}, help]`,
      ]),
      '      onward:',
      '        needs: [third]',
      '        description: Hands the night on.',
      '        do: shell',
      '        with:',
      `          cmd: [${JSON.stringify(binary)}, --no-such-flag]`,
      '  slipped:',
      '    event: run.failed',
      '    scope: own',
      '    when: ${ event.data.flow != "main" }',
      '    do: channel.post',
      '    with:',
      '      channel: general',
      `      body: "${name} failed: \${ event.data.reason ?? event.data.error }"`,
      '      detail: "${event.data.flow} of ${event.data.blueprint}: ${event.data.error}"',
      `      about_run: ${about}`,
      '',
    ].join('\n');
  await writeFile(join(out, 'failing.bp'), night('failing', `"\${ event.source.run ?? '' }"`));
  // About a run that is not there: one that went, or was never kept here.
  await writeFile(join(out, 'forgotten.bp'), night('forgotten', 'run_1m85442mg8j93j1c'));
  for (const name of ['failing', 'forgotten']) {
    await cli('apply', join(out, `${name}.bp`));
    await cli('emit', `${name}.go`);
  }
  const inChannel = page.locator('.channel .messages [data-id]');
  const said = inChannel.filter({ hasText: 'failing failed:' });
  await said.waitFor({ timeout: 30_000 });
  await said.hover();
  await said.getByRole('button', { name: 'Details' }).click();
  const details = said.locator('.said-by');
  const open = details.getByRole('button', { name: 'Open the run that failed' });
  await open.waitFor();
  await page.waitForFunction((el) => !el.disabled, await open.elementHandle());
  expect('what was running to be offered still, after the run that failed', (await details.getByRole('button', { name: 'Open what was running' }).count()) === 1);
  const order = await details.locator('button').allInnerTexts();
  expect(`the run that failed offered first, got ${order.join(' | ')}`, order[0] === 'Open the run that failed');
  // The other, about a run that is not kept, says so where it would be pressed.
  const gone = inChannel.filter({ hasText: 'forgotten failed:' });
  await gone.waitFor({ timeout: 30_000 });
  await gone.hover();
  await gone.getByRole('button', { name: 'Details' }).click();
  await gone.locator('.failed-run-gone', { hasText: 'The run that failed is no longer kept.' }).waitFor();
  expect('nothing to press for a run that is not kept', (await gone.getByRole('button', { name: 'Open the run that failed' }).count()) === 0);

  const widths = [
    [1280, 860],
    [900, 860],
    [640, 860],
    [390, 844],
  ];
  for (const [width, height] of widths) {
    await page.setViewportSize({ width, height });
    for (const one of [said, gone]) {
      await one.locator('.said-by').scrollIntoViewIfNeeded();
      const over = crossing(await boxes(one.locator('.said-by')));
      expect(`nothing of a failure's details over anything else at ${width} wide, got ${over.join('; ')}`, over.length === 0);
    }
    await said.locator('.said-by').screenshot({ path: `${out}/failedrun-details-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  await said.locator('.said-by').scrollIntoViewIfNeeded();
  await open.click();

  // The run's page, a tab of its own, the stage that failed marked and in view, with why and what it wrote.
  const run = page.locator('.pipeline-run');
  await run.waitFor();
  const stage = run.locator('.pipeline-stage[aria-current="step"]');
  await stage.waitFor();
  expect(`the stage marked to be the one that failed, got ${await stage.getAttribute('data-stage')}`, (await stage.getAttribute('data-stage')) === 'onward');
  expect('only that stage marked', (await run.locator('[aria-current="step"]').count()) === 1);
  await stage.locator('.step-error').waitFor();
  await page.waitForFunction(() => document.querySelector('.pipeline-stage[aria-current="step"] .step-log [data-first], .pipeline-stage[aria-current="step"] .step-log')?.textContent.includes('--no-such-flag'), null, { timeout: 15_000 });
  // Every step's output read, which is what pushed the stage down as it came.
  await page.waitForFunction(() => document.querySelectorAll('.pipeline-run .step-log').length >= 4 && !document.querySelector('.pipeline-run .step-log.faint'));
  const inView = async () =>
    run.evaluate((root) => {
      const marked = root.querySelector('[aria-current="step"]');
      // What the page scrolls in: the nearest part above the stage that scrolls.
      let scroller = marked.parentElement;
      while (scroller && !(/(auto|scroll)/.test(getComputedStyle(scroller).overflowY) && scroller.scrollHeight > scroller.clientHeight)) scroller = scroller.parentElement;
      const seen = (scroller ?? document.documentElement).getBoundingClientRect();
      const box = marked.getBoundingClientRect();
      // Its band, and why it failed, in sight: brought to the top, or as far as the page scrolls.
      const why = marked.querySelector('.step-error').getBoundingClientRect();
      return box.top >= Math.max(seen.top, 0) - 1 && why.bottom <= Math.min(seen.bottom, innerHeight) + 1 && (!scroller || box.top < seen.top + 60 || scroller.scrollTop >= scroller.scrollHeight - scroller.clientHeight - 1);
    });
  // Brought there once what is above it is laid out, which is a moment after the last of its output is read: waited for, a frame at a time.
  let seen = await inView();
  for (const until = Date.now() + 10_000; !seen && Date.now() < until; seen = await inView()) await page.evaluate(() => new Promise(requestAnimationFrame));
  expect('the stage that failed in view as the run is opened', seen);
  await page.screenshot({ path: `${out}/failedrun-run.png` });
  for (const [width, height] of widths) {
    await page.setViewportSize({ width, height });
    await stage.scrollIntoViewIfNeeded();
    const over = await stage.evaluate((one) => {
      const kids = [...one.querySelectorAll(':scope > .stage-band, :scope > .stage-inside')].map((el) => el.getBoundingClientRect());
      const own = one.getBoundingClientRect();
      return kids.some((b) => b.left < own.left - 1 || b.right > own.right + 1) || document.scrollingElement.scrollWidth > window.innerWidth;
    });
    expect(`the stage that failed inside itself and nothing scrolling sideways at ${width} wide`, !over);
    await page.screenshot({ path: `${out}/failedrun-run-${width}.png` });
  }
  await page.setViewportSize({ width: 1280, height: 860 });
  // Back to the channel, as it was: the run's tab closed.
  await page.locator('.tab', { has: page.getByText('failing run', { exact: true }) }).locator('.close').click();
  await page.locator('.tab', { has: page.getByText('general', { exact: true }) }).click();
  await page.locator('.channel').waitFor();
}
