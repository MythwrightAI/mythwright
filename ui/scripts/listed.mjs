// The factory's blueprints in two levels, as `lists.mjs` holds them: that
// check's own part for the list under Blueprints, in a file of its own.
// It starts no daemon and no browser: it is handed the page `lists.mjs`
// has open on the Resources panel, the address of its daemon, where its
// pictures go, and how that check says what it expected.
//
// Under each family the list shows what somebody applies and no part;
// one that has parts says how many, and unfolded shows them, each opened
// as any blueprint is. Which is which is the daemon's to say: what the
// page lists is held to what the daemon answers when it is asked the
// same (`runs` and `applied` of each), by the one rule, and not to a list
// of names written here. A part's page says what it is a part of, and
// that line leads there. The panel's box, under its head, holds the list
// to what is typed as it holds everything else the panel lists
// (`sought.mjs`): a word only a part matches shows the part under each
// thing it is a part of, with that unfolded and its family too, though
// both were folded; and emptied, the folds are as they were. A package's blueprints are listed
// the same way. Nothing of the list lies over anything else of it or
// leaves the panel, with the panel as wide as it usually is, dragged as
// narrow as it goes, and at a phone's width; and the panel is one tree
// there (`joined.mjs`), each family hung from its heading's line.

import { unfold } from './folded.mjs';
import { joined, namesCut } from './joined.mjs';
import { sideSettled } from './drawer.mjs';

/** A family's fold: the line called what the family is, with what is under it. */
const familyOf = (page, name) =>
  page.locator('.nav').locator(':scope > .tree > div', { has: page.locator(':scope > button.nav-item', { hasText: new RegExp(`^\\s*${name}\\s*$`) }) });

/** What is listed under a family: each line by its name, the faint words at its end, whether it folds, and the parts shown under it. */
const drawnOf = (family) =>
  family.locator(':scope > .tree > *').evaluateAll((lines) =>
    lines.map((line) => {
      const item = line.matches('.nav-item') ? line : line.querySelector(':scope > .nav-row > .nav-item, :scope > button.nav-item');
      return {
        name: item?.querySelector('.truncate')?.textContent.trim() ?? '',
        hint: item?.querySelector('.nav-hint')?.textContent.trim() ?? '',
        folds: !line.matches('.nav-item'),
        parts: [...line.querySelectorAll(':scope > .tree > .nav-item .truncate')].map((one) => one.textContent.trim()),
      };
    }),
  );

/** The families that are listed, by name, in order. */
const familiesOf = (page) =>
  page.locator('.nav > .tree > div > button.nav-item[aria-expanded]').evaluateAll((all) => all.map((one) => one.querySelector('.truncate')?.textContent.trim() ?? ''));

/** What the daemon says of the blueprints there are, asked as the page asks. */
const asked = (page) => page.evaluate(async () => (await (await fetch('/api/blueprints', { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } })).json()).blueprints);

/** The one rule, over what the daemon answered: what somebody applies, and every part of one. */
function ruled(all) {
  const runs = new Map(all.map((one) => [one.name, one.runs ?? []]));
  const named = (name) => all.some((one) => one.name !== name && (one.runs ?? []).includes(name));
  const partsOf = (name) => {
    const found = new Set();
    const next = [name];
    while (next.length) {
      for (const part of runs.get(next.pop()) ?? []) {
        if (part === name || !runs.has(part) || found.has(part)) continue;
        found.add(part);
        next.push(part);
      }
    }
    return [...found].sort();
  };
  const tops = all.filter((one) => one.applied || !named(one.name)).map((one) => one.name).sort();
  // What is applied though something runs it too, which is listed apart, in misc.
  const misc = all.filter((one) => one.applied && named(one.name)).map((one) => one.name).sort();
  return { tops, misc, partsOf };
}

const counted = (many) => (many === 0 ? '' : `${many} ${many === 1 ? 'part' : 'parts'}`);

/**
 * The list by its boxes: every line inside the panel's list, no line over
 * another, a line's name and the words at its end side by side with the
 * words whole, the box inside the list, and nothing scrolling sideways.
 */
async function laid(page, expect, where) {
  const got = await page.evaluate(() => {
    const nav = document.querySelector('.nav');
    const box = (one) => {
      const { left, right, top, bottom } = one.getBoundingClientRect();
      return { left, right, top, bottom };
    };
    const seen = (one) => one.getClientRects().length > 0;
    // A line is a row of an arrow and a name, or a button that is in no such row.
    const lines = [...nav.querySelectorAll('.nav-row, button.nav-item')].filter((one) => seen(one) && !(one.matches('button.nav-item') && one.closest('.nav-row')));
    const hints = [...nav.querySelectorAll('.nav-item')].filter(seen).flatMap((item) => {
      const [name, hint] = [item.querySelector('.truncate'), item.querySelector('.nav-hint')];
      return name && hint ? [{ says: `${name.textContent} · ${hint.textContent}`, item: box(item), name: box(name), hint: box(hint), cut: hint.scrollWidth - hint.clientWidth }] : [];
    });
    const seek = document.querySelector('.sidebar > .panel-seek input');
    return {
      nav: box(nav),
      side: box(document.querySelector('.sidebar')),
      lines: lines.map((one) => ({ says: one.textContent.trim().slice(0, 40), ...box(one) })),
      hints,
      seek: seek ? box(seek) : null,
      sideways: nav.scrollWidth - nav.clientWidth,
      page: document.documentElement.scrollWidth - window.innerWidth,
    };
  });
  const told = JSON.stringify(got);
  expect(`the list of blueprints to have lines and its box ${where}`, got.lines.length >= 2 && !!got.seek);
  const inside = (one) => one.left >= got.nav.left - 0.5 && one.right <= got.nav.right + 0.5;
  const out = got.lines.find((one) => !inside(one));
  expect(`every line of the panel to be inside its list ${where}, and this is not: ${JSON.stringify(out)} of ${JSON.stringify(got.nav)}`, !out);
  expect(`the panel's box to be inside the panel ${where}: ${told}`, got.seek.left >= got.side.left - 0.5 && got.seek.right <= got.side.right + 0.5 && got.seek.right - got.seek.left > 60 && got.seek.bottom <= got.nav.top + 0.5);
  const byTop = [...got.lines].sort((a, b) => a.top - b.top);
  const over = byTop.find((one, at) => at > 0 && one.top < byTop[at - 1].bottom - 0.5);
  expect(`no line of the panel to lie over another ${where}, and this does: ${JSON.stringify(over)}`, !over);
  expect(`some line to say how many parts it has ${where}`, got.hints.length > 0);
  const wrong = got.hints.find((one) => one.name.right > one.hint.left + 0.5 || one.hint.right > one.item.right + 0.5 || one.hint.left < one.item.left || one.cut > 0);
  expect(`a line's name and the count after it to be side by side, the count whole and in its line ${where}, and this is not: ${JSON.stringify(wrong)}`, !wrong);
  expect(`nothing of the panel or the page to scroll sideways ${where}, got ${got.sideways} and ${got.page}`, got.sideways <= 0 && got.page <= 0);
  // One tree: every family's line hung from the heading's, every part's from what runs it, by the boxes.
  const apart = await joined(page, 'nav.sidebar[aria-label="Resources"]');
  expect(`the panel to be one tree ${where}: ${apart.join('; ')}`, apart.length === 0);
  const cut = await namesCut(page, 'nav.sidebar[aria-label="Resources"]');
  expect(`no name of the panel cut to nothing ${where}: ${cut.join('; ')}`, cut.length === 0);
}

/**
 * The list under Blueprints, on `page`, whose Resources panel is showing
 * with nothing typed and no family unfolded. `pack` is a package that is
 * installed and brings blueprints, one of which runs another.
 */
export async function twoLevels({ page, out, expect, pack }) {
  const nav = page.locator('.nav');
  const all = await asked(page);
  const { tops, misc, partsOf } = ruled(all);
  expect(`the daemon to say what each blueprint runs, got ${JSON.stringify(all.find((one) => !Array.isArray(one.runs)))}`, all.every((one) => Array.isArray(one.runs)));
  const std = familyOf(page, 'std');
  const box = page.locator('.sidebar').getByRole('searchbox', { name: 'Search this panel' });
  await box.waitFor();
  expect(`the families to be the standard ones and then the package's, got ${(await familiesOf(page)).join(', ')}`, (await familiesOf(page)).join() === `std,${pack}`);

  // ---- what somebody applies, and no part ----------------------------------------
  await std.locator(':scope > button.nav-item').click();
  await std.getByText('std/board', { exact: true }).waitFor();
  const drawnFirst = await drawnOf(std);
  const first = drawnFirst.filter((one) => one.name !== 'misc');
  const meant = tops.filter((name) => name.startsWith('std/') && !misc.includes(name));
  expect(`the standard ones listed to be what somebody applies, as the daemon's answer comes to, got ${first.map((one) => one.name).join(', ')} and ${meant.join(', ')}`, first.map((one) => one.name).join() === meant.join());
  // The fleet and the board are applied; what a round is, what looks after a board's cards and what watches a rollout are parts.
  for (const name of ['std/board', 'std/fleet', 'std/store']) expect(`${name} to be listed among what is applied`, meant.includes(name));
  // What is applied though something runs it too is in a group of its own at the end, called misc, folded.
  const stdMisc = misc.filter((name) => name.startsWith('std/'));
  expect(`the slot, a plan and a pool to be applied though something runs them, got ${stdMisc.join(', ')}`, ['std/plan', 'std/pool', 'std/slot'].every((name) => stdMisc.includes(name)));
  expect(`misc to be the last line of the family, folded, got ${JSON.stringify(drawnFirst.at(-1))}`, drawnFirst.at(-1)?.name === 'misc' && drawnFirst.at(-1)?.folds);
  for (const name of stdMisc) expect(`${name} not to be listed among the rest`, !first.some((one) => one.name === name));
  for (const name of ['std/cards', 'std/lane-round', 'std/working', 'std/canary-stage', 'std/dispatch']) {
    expect(`${name}, a part, not to be listed among what is applied`, !meant.includes(name) && (await std.getByText(name, { exact: true }).count()) === 0);
  }
  expect(`fewer than half of the blueprints that ship to be listed first, got ${meant.length} of ${all.filter((one) => one.name.startsWith('std/')).length}`, meant.length * 2 < all.filter((one) => one.name.startsWith('std/')).length);
  // Each says how many parts it has, and one with none says nothing and has no arrow.
  for (const line of first) {
    const many = partsOf(line.name).length;
    expect(`${line.name} to say it has ${many} parts and to fold only where it has any, got ${JSON.stringify(line)}`, line.hint === counted(many) && line.folds === many > 0 && line.parts.length === 0);
  }
  expect('the fleet to have a good many parts', partsOf('std/fleet').length >= 20);
  // Unfolded, misc lists them, each as any applied blueprint is, and folded again.
  await std.locator(':scope > .tree > div > button.nav-item', { hasText: /^\s*misc\s*$/ }).click();
  await std.getByText('std/slot', { exact: true }).waitFor();
  const inMisc = await std.locator(':scope > .tree > div:last-child > .tree > *').evaluateAll((lines) =>
    lines.map((line) => (line.matches('.nav-item') ? line : line.querySelector(':scope > .nav-row > .nav-item'))?.querySelector('.truncate')?.textContent.trim() ?? ''),
  );
  expect(`misc to list what is applied though something runs it, got ${inMisc.join(', ')}`, inMisc.join() === stdMisc.join());
  await std.locator(':scope > .tree > div > button.nav-item', { hasText: /^\s*misc\s*$/ }).click();
  await std.getByText('std/slot', { exact: true }).waitFor({ state: 'detached' });
  await page.mouse.move(820, 560);
  await laid(page, expect, 'with the parts folded');
  await page.screenshot({ path: `${out}/lists-0b-what-is-applied.png` });

  // ---- its parts, unfolded, with their count ---------------------------------------
  await page.getByRole('button', { name: 'Unfold the parts of std/fleet', exact: true }).click();
  const fleet = async () => (await drawnOf(std)).find((one) => one.name === 'std/fleet');
  await std.getByText('std/lane-round', { exact: true }).waitFor();
  expect(`the fleet's parts to be every blueprint it runs, all the way down, got ${(await fleet()).parts.join(', ')}`, (await fleet()).parts.join() === partsOf('std/fleet').join());
  expect(`the fleet to say how many, got "${(await fleet()).hint}"`, (await fleet()).hint === `${partsOf('std/fleet').length} parts`);
  // A part that two run is under each: the slot is under the log feed and under the store.
  for (const top of ['std/log-feed', 'std/store']) {
    await page.getByRole('button', { name: `Unfold the parts of ${top}`, exact: true }).click();
    const line = (await drawnOf(std)).find((one) => one.name === top);
    expect(`a part that two run to be under each, got ${line.parts.join(', ')} under ${top}`, line.parts.includes('std/slot') && line.parts.join() === partsOf(top).join());
    await page.getByRole('button', { name: `Fold the parts of ${top}`, exact: true }).click();
  }
  // The picture is of the fleet's line with its parts under it: the list was scrolled to reach the others.
  await page.getByRole('button', { name: 'Fold the parts of std/fleet', exact: true }).evaluate((one) => one.scrollIntoView({ block: 'start' }));
  await page.mouse.move(820, 560);
  await laid(page, expect, 'with the fleet unfolded');
  await page.screenshot({ path: `${out}/lists-0c-the-fleets-parts.png` });

  // ---- a part opened, which says what it is a part of ------------------------------
  const part = std.locator(':scope > .tree > div > .tree > .nav-item', { has: page.getByText('std/cleared', { exact: true }) });
  expect('the part to be listed once, under the fleet', (await part.count()) === 1);
  await part.click();
  const summary = page.locator('.summary[data-of="std/cleared"]');
  await summary.waitFor();
  const leads = summary.locator('.summary-goes[data-opens="default"]');
  const of = partsOf('std/fleet').includes('std/cleared') ? tops.filter((top) => partsOf(top).includes('std/cleared')) : [];
  // A part of one says so in one line that leads there; a part of several says how many, a line for each.
  const partOfSaid = (await leads.allInnerTexts()).map((one) => one.replace(/`/g, '').trim());
  const leadsTo = of.length === 1 ? [`A part of ${of[0]}.`] : of;
  expect(`the part's page to say what it is a part of, each leading there, got ${partOfSaid.join(' | ')} for ${of.join(', ')}`, partOfSaid.join() === leadsTo.join());
  if (of.length > 1) await summary.getByText(`A part of ${of.length} blueprints:`, { exact: true }).waitFor();
  await page.screenshot({ path: `${out}/lists-0d-a-part-opened.png` });
  await leads.first().click();
  await page.locator(`.summary[data-of="${of[0]}"]`).waitFor();
  expect('one that somebody applies and nothing runs to say nothing of it', (await page.locator(`.summary[data-of="${of[0]}"]`).getByText(/^A part of/).count()) === 0);

  // ---- the box: a word only a part matches -----------------------------------------
  // Both folds are put away first: what is typed unfolds what it finds, and nothing it does is kept.
  await page.getByRole('button', { name: 'Fold the parts of std/fleet', exact: true }).click();
  await std.locator(':scope > button.nav-item').click();
  await std.getByText('std/board', { exact: true }).waitFor({ state: 'detached' });
  const WORD = 'lane-round';
  const said = (one) => `${one.name}\n${one.description ?? ''}`.toLowerCase();
  const matching = all.filter((one) => said(one).includes(WORD)).map((one) => one.name);
  expect(`"${WORD}" to be said by one blueprint, which is a part, got ${matching.join(', ')}`, matching.join() === 'std/lane-round' && !tops.includes('std/lane-round'));
  await box.fill(WORD);
  await std.getByText('std/lane-round', { exact: true }).first().waitFor();
  const found = await drawnOf(std);
  const parents = tops.filter((top) => partsOf(top).includes('std/lane-round'));
  expect(`the part to be shown under each thing it is a part of and nothing else to be listed, got ${JSON.stringify(found)}`, found.map((one) => one.name).join() === parents.join() && found.every((one) => one.parts.join() === 'std/lane-round'));
  expect(`each to say how many of its parts are shown, got ${found.map((one) => one.hint).join(', ')}`, found.every((one) => one.hint === `1 of ${partsOf(one.name).length} parts`));
  expect(`only the family that has it to be listed, got ${(await familiesOf(page)).join(', ')}`, (await familiesOf(page)).join() === 'std');
  // What holds them open is what was typed: the arrows do nothing meanwhile, and say so.
  expect('the arrows to be held', (await std.locator(':scope > button.nav-item').getAttribute('aria-disabled')) === 'true');
  await page.getByRole('button', { name: 'Fold the parts of std/fleet', exact: true }).click({ force: true });
  expect('a held arrow to fold nothing', (await std.getByText('std/lane-round', { exact: true }).count()) === parents.length);
  await page.mouse.move(820, 560);
  await laid(page, expect, 'with a part looked for');
  await page.screenshot({ path: `${out}/lists-0e-a-part-looked-for.png` });
  // What somebody applies is found by what it says it is; one that matches with no part that does is a line alone.
  await box.fill('columns');
  await std.getByText('std/board', { exact: true }).waitFor();
  const board = (await drawnOf(std)).find((one) => one.name === 'std/board');
  expect(`the board, found by what it says it is, to be a line with no part under it, got ${JSON.stringify(board)}`, !!board && !board.folds && board.parts.length === 0 && board.hint === '');
  // What nothing says lists nothing, and says so.
  await box.fill('no such blueprint anywhere');
  await nav.getByRole('status').getByText(/^Nothing listed here says “no such blueprint anywhere”\./).waitFor();
  expect('no family to be listed for what nothing says', (await familiesOf(page)).length === 0);
  // Escape empties the box, and the folds are as they were: both folded.
  await box.press('Escape');
  await std.locator(':scope > button.nav-item[aria-expanded="false"]').waitFor();
  expect(`the box to be empty and every family back, folded, got "${await box.inputValue()}" and ${(await familiesOf(page)).join(', ')}`, (await box.inputValue()) === '' && (await familiesOf(page)).join() === `std,${pack}`);
  expect('nothing of what was found to be listed once the box is empty', (await nav.getByText('std/lane-round', { exact: true }).count()) === 0 && (await nav.getByText('std/board', { exact: true }).count()) === 0);
  await std.locator(':scope > button.nav-item').click();
  await std.getByText('std/board', { exact: true }).waitFor();
  expect('the fleet to be folded as it was left', (await fleet()).parts.length === 0 && (await fleet()).hint === `${partsOf('std/fleet').length} parts`);
  // Unfolded, a fold is kept as it always was: there when the page is read again.
  await page.getByRole('button', { name: 'Unfold the parts of std/fleet', exact: true }).click();
  await page.reload();
  await box.waitFor();
  await std.getByText('std/lane-round', { exact: true }).waitFor();
  expect('what was typed to be kept nowhere', (await box.inputValue()) === '');

  // ---- a package's blueprints, the same way ------------------------------------------
  const theirs = familyOf(page, pack);
  await theirs.locator(':scope > button.nav-item').click();
  await theirs.getByText(`${pack}/serve`, { exact: true }).waitFor();
  const brought = await drawnOf(theirs);
  expect(`the package to list what is applied of it, with how many parts, got ${JSON.stringify(brought)}`, brought.length === 1 && brought[0].name === `${pack}/serve` && brought[0].hint === '1 part' && brought[0].folds);
  await page.getByRole('button', { name: `Unfold the parts of ${pack}/serve`, exact: true }).click();
  await theirs.getByText(`${pack}/inner`, { exact: true }).waitFor();
  expect(`its part to be under it, got ${(await drawnOf(theirs))[0].parts.join(', ')}`, (await drawnOf(theirs))[0].parts.join() === `${pack}/inner`);
  await page.mouse.move(820, 560);
  await page.screenshot({ path: `${out}/lists-0f-a-packages.png` });

  // ---- by the boxes, with the panel dragged as narrow as it goes --------------------
  const edge = page.locator('.side-edge');
  const grip = await edge.boundingBox();
  await page.mouse.move(grip.x + grip.width / 2, grip.y + 300);
  await page.mouse.down();
  await page.mouse.move(grip.x - 60, grip.y + 300, { steps: 6 });
  await page.mouse.move(60, grip.y + 300, { steps: 6 });
  await page.mouse.up();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 180);
  await page.mouse.move(820, 560);
  await laid(page, expect, 'with the panel dragged as narrow as it goes');
  await page.screenshot({ path: `${out}/lists-0g-the-panel-narrow.png` });
  await box.fill(WORD);
  await std.getByText('std/lane-round', { exact: true }).first().waitFor();
  await laid(page, expect, 'with the panel narrow and a part looked for');
  await box.fill('');
  await edge.dblclick();
  await page.waitForFunction(() => Math.round(document.querySelector('.sidebar').getBoundingClientRect().width) === 252);
  // Left as the rest of the check finds it: the families folded, and the fleet's parts with them.
  await page.getByRole('button', { name: 'Fold the parts of std/fleet', exact: true }).click();
  await page.getByRole('button', { name: `Fold the parts of ${pack}/serve`, exact: true }).click();
  await theirs.locator(':scope > button.nav-item').click();
  await std.locator(':scope > button.nav-item').click();
  await std.getByText('std/board', { exact: true }).waitFor({ state: 'detached' });
}

/** The same list at a phone's width, on a page of its own that is signed in: the panel has the screen there. */
export async function twoLevelsOnPhone({ page, out, expect }) {
  await page.getByRole('button', { name: 'Expand the side panel' }).click();
  await sideSettled(page);
  await page.locator('.sidebar').waitFor();
  const resources = page.getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  // Everything there starts folded: the part the blueprints are under is unfolded by its arrow.
  await unfold(page, 'Blueprints');
  const { partsOf } = ruled(await asked(page));
  const std = familyOf(page, 'std');
  await std.locator(':scope > button.nav-item').click();
  await page.getByRole('button', { name: 'Unfold the parts of std/fleet', exact: true }).click();
  await std.getByText('std/lane-round', { exact: true }).waitFor();
  const fleet = (await drawnOf(std)).find((one) => one.name === 'std/fleet');
  expect(`the fleet's parts to be listed at a phone's width, got ${fleet.parts.length}`, fleet.parts.join() === partsOf('std/fleet').join());
  await laid(page, expect, 'at a phone\'s width');
  await page.screenshot({ path: `${out}/lists-0h-at-390.png` });
  const box = page.locator('.sidebar').getByRole('searchbox', { name: 'Search this panel' });
  await box.fill('lane-round');
  await std.getByText('std/lane-round', { exact: true }).first().waitFor();
  await laid(page, expect, 'at a phone\'s width with a part looked for');
  await page.screenshot({ path: `${out}/lists-0i-looked-for-at-390.png` });
  // A part pressed there shows its page, with the panel out of its way.
  await std.locator(':scope > .tree > div > .tree > .nav-item').first().click();
  await page.locator('.sidebar').waitFor({ state: 'detached' });
  await page.locator('.summary[data-of="std/lane-round"]').waitFor();
}
