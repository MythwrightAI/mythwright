// A board's card opened in the Tools panel beside a channel, which `tools.mjs` runs
// with the panel open beside the channel of a project whose board has the card: the card's own form, as
// the drawer beside its board has it, every part the drawer draws there too (what it is called, what it
// says, its labels, its column, whom it is given to, its day, what it comes after, its priority and what it
// is part of; where it is and who added it; Delete and Save, under Details; Comments, with Say and Public,
// and History, in the one row of tabs beside Details; what it offers to be pressed); nothing of it over anything else, outside the panel or scrolling sideways at the
// panel's usual width, dragged as narrow as it goes and on a phone; what was typed there kept while the
// tab is away; saved, and the daemon holding what was saved; and somebody who may read the board and not
// change a card shown each part and offered nothing to change it with.
import { toolsAtRest } from './drawer.mjs';
import { taggedWhole } from './offered.mjs';

/** What a card's tabs draw to be read and filled as they stand, each by what it is called, in order: the same list wherever it is drawn. */
const shown = (tabs) =>
  tabs.evaluate((one) => {
    const named = (el) => el.getAttribute('aria-label') || el.labels?.[0]?.textContent.trim().split('\n')[0].trim() || el.textContent.trim();
    const controls = [...one.querySelectorAll('input, select, textarea, button')].filter((el) => el.getClientRects().length > 0 && !el.closest('.card-cost')).map((el) => `${el.tagName.toLowerCase()}:${named(el)}`);
    const said = [...one.querySelectorAll('.field-many > span, .journey')].filter((el) => el.getClientRects().length > 0).map((el) => (el.classList.contains('journey') ? 'history' : el.textContent.trim()));
    return { controls, said, where: [...one.querySelectorAll('p.faint')].some((p) => p.getClientRects().length > 0 && /^In \w+ · added .+ by /.test(p.textContent.trim())) };
  });

/** What a card's tabs draw under Details and under Comments, Details showing again after. */
const parts = async (on) => {
  const tabs = on.locator('.card-tabs');
  const tab = (name) => tabs.getByRole('tab', { name, exact: true });
  const row = (await tabs.locator('[role="tablist"]').first().getByRole('tab').allInnerTexts()).map((one) => one.trim());
  const details = await shown(tabs);
  await tab('Comments').click();
  await tabs.getByLabel('Say something on this card').waitFor();
  const comments = await shown(tabs);
  await tab('Details').click();
  await tabs.locator('form.card-form').getByLabel('Title', { exact: true }).waitFor();
  return { row, details, comments };
};

/** What of the card's page in the panel lies outside the panel or over another part of it, and whether anything there scrolls sideways. */
const laid = (panel) =>
  panel.evaluate((box) => {
    const rim = box.getBoundingClientRect();
    const page = box.querySelector('.context-page');
    // Every piece of words and every control, each by its own box: a line of words by its text's, a control by its own.
    const pieces = [];
    // What is cut short where a box it is in ends is seen as far as that: what is past it is not drawn.
    const clip = (from, at) => {
      let [left, right] = [at.left, at.right];
      for (let up = from; up && up !== page; up = up.parentElement)
        if (getComputedStyle(up).overflowX !== 'visible') {
          const edge = up.getBoundingClientRect();
          [left, right] = [Math.max(left, edge.left), Math.min(right, edge.right)];
        }
      return { left, right, top: at.top, bottom: at.bottom, width: right - left, height: at.height };
    };
    const walk = (node) => {
      for (const one of node.childNodes) {
        if (one.nodeType === Node.TEXT_NODE && one.textContent.trim()) {
          const range = document.createRange();
          range.selectNodeContents(one);
          for (const at of [...range.getClientRects()].map((at) => clip(one.parentElement, at))) if (at.width > 0 && at.height > 0) pieces.push([one.parentElement, at]);
        } else if (one.nodeType === Node.ELEMENT_NODE && one.getClientRects().length > 0 && !one.closest('.card-cost')) {
          if (one.matches('input, select, textarea, button')) {
            // A control scrolled out of what it scrolls in (a tab of the card's row in a narrow panel) is seen as far as that.
            const at = clip(one.parentElement, one.getBoundingClientRect());
            if (at.width > 0 && at.height > 0) pieces.push([one, at]);
          } else walk(one);
        }
      }
    };
    walk(page);
    const name = (el) => `${el.tagName.toLowerCase()}${el.className ? `.${String(el.className).split(' ')[0]}` : ''}`;
    const out = pieces.filter(([, at]) => at.left < rim.left - 0.5 || at.right > rim.right + 0.5).map(([el]) => name(el));
    const over = [];
    for (let i = 0; i < pieces.length; i += 1)
      for (let j = i + 1; j < pieces.length; j += 1) {
        const [ea, a] = pieces[i];
        const [eb, b] = pieces[j];
        if (ea.contains(eb) || eb.contains(ea)) continue;
        const across = Math.min(a.right, b.right) - Math.max(a.left, b.left);
        const down = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
        if (across > 1 && down > 1) over.push(`${name(ea)} "${(ea.textContent || ea.value || '').trim().slice(0, 20)}" and ${name(eb)} "${(eb.textContent || eb.value || '').trim().slice(0, 20)}"`);
      }
    const scrolls = [...box.querySelectorAll('*')].filter((el) => el.scrollWidth > el.clientWidth + 1 && ['auto', 'scroll'].includes(getComputedStyle(el).overflowX) && !el.closest('[role=tablist]')).map((el) => String(el.className));
    return { pieces: pieces.length, out, over, scrolls, sideways: document.documentElement.scrollWidth > window.innerWidth };
  });

export async function toolsCard({ page, out, expect, cli, url, until, title, signedInLacking }) {
  const panel = page.getByRole('complementary', { name: 'Tools' });
  const form = panel.locator('form.card-form');
  const opened = async (on) => {
    await on.getByRole('tab', { name: 'Board' }).click();
    await on.locator('.context-row', { hasText: title }).click();
    await on.locator('form.card-form').getByLabel('Title', { exact: true }).waitFor();
  };
  const sound = async (where) => {
    const seen = await laid(panel);
    expect(`${where}: nothing of the card's form outside the panel, over another part of it or scrolling sideways, got ${JSON.stringify(seen)}`, seen.pieces > 10 && seen.out.length === 0 && seen.over.length === 0 && seen.scrolls.length === 0 && !seen.sideways);
  };

  // ---- the card's own form, with every part the drawer has ---------------------------------------------
  await opened(panel);
  const here = await parts(panel);
  const { details, comments } = here;
  expect(`one row of tabs, Details, Comments and History first, got ${JSON.stringify(here.row)}`, here.row.slice(0, 3).join('|') === 'Details|Comments|History');
  for (const label of ['Title', 'Details', 'Column', 'Assigned to', 'Due', 'After', 'Priority', 'Part of']) expect(`the card's ${label} to be offered in the panel, got ${details.controls.join(' | ')}`, details.controls.some((one) => !one.startsWith('button:') && one.includes(`:${label}`)));
  expect(`its labels, Delete and Save under Details, got ${JSON.stringify(details)}`, details.said.includes('Labels') && details.controls.includes('button:Delete') && details.controls.includes('button:Save') && details.where);
  expect(`what was said, Say and Public under Comments, got ${JSON.stringify(comments)}`, comments.said.includes('history') && comments.controls.includes('textarea:Say something on this card') && comments.controls.includes('input:Public'));
  // And the drawer beside its board draws the same parts.
  await panel.getByRole('button', { name: 'Open on the board' }).click();
  const drawer = page.locator('.drawer[data-rest]');
  await drawer.locator('form.card-form').getByLabel('Title', { exact: true }).waitFor();
  const there = await parts(drawer);
  expect(`the panel to draw what the drawer draws of the card, got\n  ${JSON.stringify(here)}\nwhere the drawer has\n  ${JSON.stringify(there)}`, JSON.stringify(here) === JSON.stringify(there));
  await drawer.getByRole('button', { name: 'Close', exact: true }).click();
  await drawer.waitFor({ state: 'detached' });
  await page.locator('.drawer-leaving').waitFor({ state: 'detached' });
  await page.locator('.tabs').getByRole('tab', { name: /riverside-workshop-annex$/ }).first().click();
  await toolsAtRest(page).waitFor();
  await form.getByLabel('Title', { exact: true }).waitFor();
  await sound('at the panel\'s usual width');
  await page.screenshot({ path: `${out}/tools-10-card.png` });

  // ---- dragged as narrow as it goes, and on a phone ---------------------------------------------------
  const edge = panel.getByRole('separator');
  await edge.focus();
  for (let last = -1, now = 0; now !== last; ) {
    last = now;
    await page.keyboard.press('ArrowRight');
    now = Math.round((await panel.boundingBox()).width);
  }
  await sound('dragged as narrow as it goes');
  await page.screenshot({ path: `${out}/tools-10-card-narrow.png` });
  await edge.dblclick();
  const usual = page.viewportSize();
  await page.setViewportSize({ width: 390, height: 844 });
  await form.getByLabel('Title', { exact: true }).waitFor();
  await sound('on a phone');
  await page.screenshot({ path: `${out}/tools-10-card-phone.png` });
  await page.setViewportSize(usual);

  // ---- what is typed is kept while the tab is away, and saved ------------------------------------------
  const was = await form.getByLabel('Title', { exact: true }).inputValue();
  await form.getByLabel('Title', { exact: true }).fill(`${was}, level`);
  await form.getByLabel('Details', { exact: true }).fill('Two brackets and a spirit level.');
  await form.getByLabel(/^Priority/).selectOption('high');
  await page.locator('.tabs').getByRole('tab', { name: /Start/ }).click();
  await panel.waitFor({ state: 'detached' });
  await page.locator('.tabs').getByRole('tab', { name: /riverside-workshop-annex$/ }).first().click();
  await toolsAtRest(page).waitFor();
  await form.getByLabel('Title', { exact: true }).waitFor();
  const kept = [await form.getByLabel('Title', { exact: true }).inputValue(), await form.getByLabel('Details', { exact: true }).inputValue(), await form.getByLabel(/^Priority/).inputValue()];
  expect(`what was typed in the card's form to be kept while the tab was away, got ${JSON.stringify(kept)}`, kept.join('|') === `${was}, level|Two brackets and a spirit level.|high`);
  await form.getByRole('button', { name: 'Save', exact: true }).click();
  await panel.locator('.context-row', { hasText: `${was}, level` }).waitFor();
  const saved = async () => JSON.parse((await cli('record', 'list', 'riverside-workshop-annex', 'board', '--json')).stdout).records.find((one) => one.data.title === `${was}, level`);
  await until('the daemon to hold what was saved', async () => {
    const card = await saved();
    return (card && card.data.body === 'Two brackets and a spirit level.' && card.data.priority === 'high') || JSON.stringify(card?.data ?? null);
  });
  // Opened again, it is the card as it was saved, with nothing kept of what was typed before.
  await panel.locator('.context-row', { hasText: `${was}, level` }).click();
  expect('the card opened again to be as it was saved', (await form.getByLabel('Title', { exact: true }).inputValue()) === `${was}, level`);
  await page.screenshot({ path: `${out}/tools-11-card-saved.png` });
  // Who can be named on it, offered after an @ in its box, whole and over everything it lies on with the box scrolled up
  // to the panel's top, where what scrolls there would cut it off: at the usual size, a window too low for it
  // under the box, and a phone's.
  {
    // Somebody to name: who is offered is whom being named tells, and that is never whoever writes. Inviting
    // somebody takes a tier that has more than one person.
    await cli('tier', 'premium');
    const code = JSON.parse((await cli('invite', '--json')).stdout).code;
    const joined = await fetch(`${url}/api/auth/register`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email: 'nell@example.com', name: 'Nell', password: 'correct horse', invite: code }) });
    expect(`Nell to have registered, got ${joined.status}`, joined.ok);
    const usual = page.viewportSize();
    // What is said on it is under its Comments tab.
    const tabs = panel.locator('.card-tabs');
    await tabs.getByRole('tab', { name: 'Comments', exact: true }).click();
    const saying = tabs.getByLabel('Say something on this card');
    await saying.waitFor();
    for (const size of [usual, { width: 1100, height: 420 }, { width: 390, height: 844 }]) {
      await page.setViewportSize(size);
      await saying.evaluate((box) => box.scrollIntoView({ block: 'start' }));
      await taggedWhole(page, expect, saying, `named on a card in the panel at ${size.width} by ${size.height}`, `${out}/tools-11-card-named-${size.width}x${size.height}.png`);
    }
    await page.setViewportSize(usual);
    await tabs.getByRole('tab', { name: 'Details', exact: true }).click();
    await form.getByLabel('Title', { exact: true }).waitFor();
  }
  await panel.getByRole('button', { name: 'Back to the board' }).click();

  // ---- somebody who may read the board and not change a card ---------------------------------------------
  const less = await signedInLacking(['projects', 'channels', 'board:read', 'records', 'blueprints', 'agents', 'fleet', 'search']);
  const theirs = less.getByRole('complementary', { name: 'Tools' });
  await opened(theirs);
  const read = theirs.locator('form.card-form');
  const closed = await read.evaluate((one) => [...one.querySelectorAll('input, select, textarea')].filter((el) => !el.closest('.card-cost')).map((el) => [el.getAttribute('aria-label') || el.labels?.[0]?.textContent.trim().split('\n')[0], el.matches(':disabled')]));
  expect(`each part shown and closed to somebody who may not change a card, got ${JSON.stringify(closed)}`, closed.length >= 8 && closed.every(([, off]) => off));
  expect(`what it is called shown to them, got ${await read.getByLabel('Title', { exact: true }).inputValue()}`, (await read.getByLabel('Title', { exact: true }).inputValue()) === `${was}, level`);
  const offered = await read.getByRole('button').allInnerTexts();
  expect(`nothing to change it with offered to them (no Save, Delete or Say), got ${offered.join(' | ')}`, !offered.some((one) => ['Save', 'Delete', 'Say', 'Send back'].includes(one.trim())));
  await theirs.locator('.card-tabs').getByRole('tab', { name: 'Comments', exact: true }).click();
  await theirs.locator('.card-tabs .journey').waitFor();
  expect('nothing to say anything with under Comments for them', (await theirs.getByLabel('Say something on this card').count()) === 0);
  await less.screenshot({ path: `${out}/tools-12-card-read-only.png` });
  await less.context().close();
}
