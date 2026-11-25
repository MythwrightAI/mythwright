// Moving something of the side panel, as spaces.mjs goes through it: a
// project to another workspace, and a channel to a workspace or a
// project, each from its row's menu. It is a file of its own because
// spaces.mjs was at its limit of lines; it starts no daemon and no
// browser, and is handed the page spaces.mjs has open with what that
// check reads the panel by.
//
// A row's menu has one line for it, `Move to…`, and no workspace of its
// own: the line opens a menu beside it, on the pointer resting on it, a
// press, Enter or the right arrow, put where there is room (on its
// right, on its left, and at a phone's width over the menu under the
// line), on the screen, moving nothing, staying while the pointer
// crosses another line on its way there, and going where the pointer
// rests elsewhere. The left arrow and Escape put it away and give the
// keyboard back to its line, the menu staying. It lists the workspaces,
// the one the thing is in marked and nothing to press, and for a project
// ends with a new one set apart; for a channel the projects follow under
// a heading. Choosing asks a question that names the thing and where it
// goes and says what that means; Cancel leaves everything as it was, on
// a second page that is never read again too, and Move it moves it
// there as here. What the daemon refuses is said in its sentence with
// the question staying. A new workspace made from there is asked about
// the same way. Somebody who may not move a project or a channel is
// offered no line for it. Nothing of either menu or of the question lies
// over anything else at three widths and a phone's.
import { pageCrowded } from './crowded.mjs';
import { sideSettled } from './drawer.mjs';

/** The boxes of everything of the side panel, to say that nothing of it moved. */
const still = (page) =>
  page.locator('.sidebar').evaluate((side) =>
    [...side.querySelectorAll('.nav-row, .nav-item, .space-foot *')]
      .map((one) => {
        const at = one.getBoundingClientRect();
        return [at.left, at.top, at.width, at.height].map((figure) => Math.round(figure * 10) / 10).join(',');
      })
      .join(' '),
  );

const boxOf = async (locator) => {
  const at = await locator.boundingBox();
  return { left: at.x, top: at.y, right: at.x + at.width, bottom: at.y + at.height };
};

/** A page of its own, signed in as the owner, that is never read again after this; told it holds `scopes` where they are said. */
async function another(browser, url, problems, scopes = null, phone = false) {
  const other = await browser.newPage({ viewport: phone ? { width: 390, height: 844 } : { width: 1350, height: 860 }, hasTouch: phone, colorScheme: process.env.SCHEME ?? 'dark' });
  other.on('pageerror', (err) => problems.push(err.message));
  if (scopes)
    await other.route('**/api/me', async (route) => {
      const answer = await route.fetch();
      const me = await answer.json();
      if (!me.principal) return route.fulfill({ response: answer });
      me.principal.scopes = scopes;
      await route.fulfill({ response: answer, json: me });
    });
  await other.goto(url);
  await other.getByLabel('Email').fill('ada@example.com');
  await other.getByLabel('Password').fill('correct horse');
  await other.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await other.locator('.rail').waitFor({ state: 'attached' });
  // At a phone's width the side panel is brought in first.
  const opener = other.getByRole('button', { name: 'Expand the side panel' });
  if (phone && (await opener.count())) await opener.tap();
  await sideSettled(other);
  await other.locator('.rail').waitFor();
  const resources = other.locator('.rail').getByRole('button', { name: 'Resources', exact: true });
  if ((await resources.getAttribute('aria-pressed')) !== 'true') await resources.click();
  await other.getByRole('navigation', { name: 'Resources' }).waitFor();
  // Projects starts unfolded and may have been folded since (`folded.mjs`): unfolded, and nothing else, as a person would.
  const projects = other.getByRole('navigation', { name: 'Resources' }).getByRole('button', { name: 'Unfold Projects', exact: true });
  if ((await projects.count()) > 0) await (phone ? projects.tap() : projects.click());
  return other;
}

/** What spaces.mjs hands: its page and what it reads the panel by. */
export function movedTo(had) {
  const { browser, page, url, out, expect, listed, problems } = had;
  const rowMenu = (on = page) => on.locator('.row-menu:not(.row-beside)');
  const beside = (on = page) => on.locator('.row-menu.row-beside');
  const moveLine = (on = page) => rowMenu(on).getByRole('menuitem', { name: 'Move to…', exact: true });
  const lines = async (menu) =>
    (await menu.locator(':scope > button, :scope > .menu-said').evaluateAll((all) =>
      all.map((one) => `${one.querySelector('.truncate')?.textContent ?? one.textContent.trim()}${one.getAttribute('aria-checked') === 'true' ? ' ✓' : ''}${one.getAttribute('aria-disabled') === 'true' ? ' (here)' : ''}`),
    )).join(' | ');
  const focused = (on = page) => on.evaluate(() => `${document.activeElement?.closest('.row-beside') ? 'beside:' : document.activeElement?.closest('.row-menu') ? 'menu:' : 'page:'}${document.activeElement?.querySelector('.truncate')?.textContent ?? document.activeElement?.getAttribute('aria-label') ?? ''}`);
  // The whole page, or at a phone's width, where the side panel lies over the page on purpose, each of what floats by itself.
  const crowded = async (on, where, within = ['body']) => {
    const said = (await Promise.all(within.map((one) => pageCrowded(on, one)))).flat();
    expect(`nothing to lie over anything else ${where}: ${said.join('; ')}`, said.length === 0);
  };
  // The menu beside, by its boxes: beside the line that opened it (or over the menu under the line where neither side has
  // room), on the screen, its first line level with that line where it is beside it.
  const placed = async (on, where) => {
    const [menu, line, side] = [await boxOf(rowMenu(on)), await boxOf(moveLine(on)), await boxOf(beside(on))];
    const view = on.viewportSize();
    const told = JSON.stringify({ menu, line, side, view });
    expect(`the menu beside to be on the screen ${where}: ${told}`, side.left >= -0.5 && side.top >= -0.5 && side.right <= view.width + 0.5 && side.bottom <= view.height + 0.5);
    const right = side.left >= menu.right - 0.5;
    const left = side.right <= menu.left + 0.5;
    const over = !right && !left;
    if (over) expect(`the menu beside to lie under its line where there is no room beside the menu ${where}: ${told}`, side.top >= line.bottom - 0.5 || side.bottom <= view.height + 0.5);
    else expect(`the menu beside to be level with its line ${where}: ${told}`, Math.abs(side.top + 5 - line.top) <= 2 || side.bottom >= view.height - 5);
    return over ? 'over' : right ? 'right' : 'left';
  };

  /** A project's row's menu and the question: by the pointer, by the keyboard, cancelled, moved, refused, and to a new workspace. */
  async function project({ tree, more, toSpace, resources, projects, showing, offers, done, cli }) {
    // The panel is of Shop front, with Shop in it. A second page is open beside it and is never read again.
    const other = await another(browser, url, problems);
    const theirs = other.getByRole('navigation', { name: 'Resources' });
    const theirProjects = () => theirs.evaluate((nav) => [...nav.querySelectorAll('[data-project]')].map((one) => one.querySelector(':scope > .nav-row .nav-item .truncate').textContent).join(' | '));
    // A page of its own begins at the usual workspace: it is shown the one this one is of.
    const theirSpace = async (name) => {
      await theirs.locator('.space-foot .space-pick').click();
      await other.getByRole('menu', { name: 'Workspace', exact: true }).getByRole('menuitemradio', { name: new RegExp(`^${name}(:|$)`) }).click();
      await listed(() => theirs.locator('.space-foot .space-said strong').innerText(), name, `the other page to be of ${name}`);
    };
    await theirSpace('Shop front');
    await listed(theirProjects, 'Shop', 'the other page to list its project');

    // In view before anything is measured: the panel is long with everything of it unfolded, and pressing the row would scroll it.
    await tree('shop').scrollIntoViewIfNeeded();
    const before = await still(page);
    await more(tree('shop'), 'More for Shop');
    await rowMenu().waitFor();
    // Beside a folder made in it and it called something else (`views/outline.tsx`), which no workspace is.
    expect(`a project's menu to have one line that moves it and no workspace of its own, got "${await lines(rowMenu())}"`, (await lines(rowMenu())) === 'Pin to the top | New… | Rename… | Move to… | Move to Trash…');
    expect('the line to say it opens a menu', (await moveLine().getAttribute('aria-haspopup')) === 'menu' && (await moveLine().getAttribute('aria-expanded')) === 'false');
    // Rested on, it opens what it holds beside it.
    await moveLine().hover();
    await beside().waitFor();
    expect(`what it opens to be the workspaces, the one it is in marked and nothing to press, and a new one, got "${await lines(beside())}"`, (await lines(beside())) === 'Annex | Shop front ✓ (here) | Works | New workspace…');
    expect('what it opens to be a menu called what the line moves it to', (await page.getByRole('menu', { name: 'Move to', exact: true }).count()) === 1);
    expect(`the line to say it is open`, (await moveLine().getAttribute('aria-expanded')) === 'true');
    expect(`nothing of the side panel to have moved`, (await still(page)) === before);
    expect(`it to be put on the right, where there is room, got ${await placed(page, 'at 1350 wide')}`, (await placed(page, 'at 1350 wide')) === 'right');
    await crowded(page, 'with the menu beside open at 1350 wide');
    await page.screenshot({ path: `${out}/spaces-07a-move-to.png` });
    // Pressing the one it is in does nothing.
    await beside().getByRole('menuitemradio', { name: 'Shop front', exact: true }).click({ force: true });
    expect('the workspace it is in to be nothing to press', (await beside().count()) === 1 && (await page.getByRole('dialog').count()) === 0);
    // The pointer crossing the line under it on its way: what was opened stays, and the workspace it goes to is pressed.
    const from = await moveLine().boundingBox();
    await page.mouse.move(from.x + from.width - 20, from.y + from.height / 2);
    const works = beside().getByRole('menuitemradio', { name: 'Works', exact: true });
    const to = await works.boundingBox();
    await page.mouse.move(to.x + 30, to.y + to.height / 2, { steps: 12 });
    expect('the menu beside to stay as the pointer crossed to it', (await beside().count()) === 1);
    // Resting on another line of the menu instead, it goes.
    await rowMenu().getByRole('menuitem', { name: 'Pin to the top', exact: true }).hover();
    await beside().waitFor({ state: 'detached' });
    expect('the menu it came from to stay', (await rowMenu().count()) === 1);
    await moveLine().hover();
    await beside().waitFor();
    await beside().getByRole('menuitemradio', { name: 'Annex', exact: true }).click();
    await rowMenu().waitFor({ state: 'detached' });
    const asked = page.getByRole('dialog');
    await asked.getByRole('heading', { name: 'Move Shop to Annex?', exact: true }).waitFor();
    const says = (await asked.innerText()).replace(/\s+/g, ' ');
    expect(`the question to say what moving it means, got "${says}"`, says.includes('Shop is listed in Annex from now on') && says.includes('the channels that are its go with it') && says.includes('nobody is let in or kept out'));
    for (const [width, picture] of [[1350, 'spaces-07b-asked-to-move.png'], [1000, null], [820, null]]) {
      await page.setViewportSize({ width, height: 860 });
      await crowded(page, `with the question at ${width} wide`);
      if (picture) await page.screenshot({ path: `${out}/${picture}` });
    }
    await page.setViewportSize({ width: 1350, height: 860 });
    // Cancel leaves everything as it was, here and on the other page.
    await asked.getByRole('button', { name: 'Cancel', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(projects, 'Shop', 'a project not moved where the question was cancelled');
    expect(`nothing to be asked of the daemon by a cancel, and the other page to list it still, got "${await theirProjects()}"`, (await theirProjects()) === 'Shop');

    // ---- by the keyboard: the menu's arrows, the right arrow into what it opens, the left arrow and Escape back
    await tree('shop').hover();
    await tree('shop').getByRole('button', { name: 'More for Shop', exact: true }).focus();
    await page.keyboard.press('Enter');
    await rowMenu().waitFor();
    // It listens for keys from once it is drawn, a moment after it is on the page; a person is never quicker.
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    // Past a folder made in it and it called something else.
    for (let n = 0; n < 4; n += 1) await page.keyboard.press('ArrowDown');
    expect(`the arrows to reach the line, got ${await focused()}`, (await focused()) === 'menu:Move to…');
    await page.keyboard.press('ArrowRight');
    await beside().waitFor();
    await listed(() => focused(), 'beside:Annex', 'the right arrow to give the keyboard to what it opened, at its first');
    await page.keyboard.press('ArrowDown');
    expect(`the arrows to go through it, got ${await focused()}`, (await focused()) === 'beside:Shop front');
    await page.keyboard.press('ArrowLeft');
    await beside().waitFor({ state: 'detached' });
    expect(`the left arrow to put it away and give the keyboard back to its line, got ${await focused()}`, (await focused()) === 'menu:Move to…' && (await rowMenu().count()) === 1);
    await page.keyboard.press('Enter');
    await beside().waitFor();
    await listed(() => focused(), 'beside:Annex', 'Enter to open it with the keyboard in it');
    await page.keyboard.press('Escape');
    await beside().waitFor({ state: 'detached' });
    expect(`Escape to put away what it opened alone, got ${await focused()}`, (await focused()) === 'menu:Move to…' && (await rowMenu().count()) === 1);
    await page.keyboard.press('ArrowRight');
    await listed(() => focused(), 'beside:Annex', 'the right arrow to open it again');
    await page.keyboard.press('Enter');
    await asked.getByRole('heading', { name: 'Move Shop to Annex?', exact: true }).waitFor();
    await asked.getByRole('button', { name: 'Move it', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(projects, '', 'a project moved to leave the list of the workspace it was in');
    expect(`the panel to be of the workspace it was of, got "${await showing(resources)}"`, (await showing(resources)) === 'Shop front');
    await listed(theirProjects, '', 'the other page, never read again, to see it moved');
    await toSpace(resources, 'Annex');
    await listed(projects, 'Shop | Depot | Barn', 'the project in the workspace it was moved to');

    // ---- what the daemon refuses is said in its sentence, and the question stays: the workspace is gone by the time it is asked
    await cli('workspace', 'create', 'Gone', 'soon');
    await listed(() => offers(resources), `Annex* | Gone soon | Shop front | Works || ${done()}`, 'a workspace made from a terminal to be offered');
    await more(tree('shop'), 'More for Shop');
    await moveLine().click();
    await beside().getByRole('menuitemradio', { name: 'Gone soon', exact: true }).click();
    await asked.getByRole('heading', { name: 'Move Shop to Gone soon?', exact: true }).waitFor();
    await cli('workspace', 'remove', 'gone-soon');
    await asked.getByRole('button', { name: 'Move it', exact: true }).click();
    const refusal = asked.getByRole('alert');
    await refusal.waitFor();
    const answer = await page.evaluate(() =>
      fetch('/api/projects/shop/workspace', { method: 'PUT', headers: { 'content-type': 'application/json', 'x-page-key': localStorage['page-key'] }, body: JSON.stringify({ workspace: 'gone-soon' }) }).then((one) => one.json()),
    );
    const sentence = answer.error?.message ?? answer.error ?? answer.message;
    expect(`the daemon's refusal to be said in its sentence, got "${await refusal.innerText()}" where it says "${sentence}"`, typeof sentence === 'string' && (await refusal.innerText()) === sentence);
    expect('the question to stay', (await asked.count()) === 1);
    await crowded(page, 'with a refusal in the question');
    await page.screenshot({ path: `${out}/spaces-07c-a-move-refused.png` });
    await asked.getByRole('button', { name: 'Cancel', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(projects, 'Shop | Depot | Barn', 'nothing moved by what was refused');
    // Put back where it was, by the pointer.
    await more(tree('shop'), 'More for Shop');
    await moveLine().hover();
    await beside().getByRole('menuitemradio', { name: 'Shop front', exact: true }).click();
    await asked.getByRole('button', { name: 'Move it', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(projects, 'Depot | Barn', 'the project put back');

    // ---- a new workspace for a project, from what the line opens: made, and then asked about as any move is
    await toSpace(resources, 'Works');
    await more(tree('shed'), 'More for Shed');
    await moveLine().hover();
    await beside().getByRole('menuitem', { name: 'New workspace…', exact: true }).click();
    await asked.getByText('New workspace', { exact: true }).waitFor();
    await asked.getByLabel('Name').fill('Yard works');
    await asked.getByRole('button', { name: 'Create', exact: true }).click();
    await asked.getByRole('heading', { name: 'Move Shed to Yard works?', exact: true }).waitFor();
    await asked.getByRole('button', { name: 'Move it', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(projects, '', 'the project gone to the workspace made for it');
    await listed(() => offers(resources), `Annex | Shop front | Works* | Yard works || ${done()}`, 'the workspace made, and the panel of the workspace it was of');

    // ---- at a phone's width, and with the panel dragged narrow, on the page that is never read again
    await other.locator('.side-edge').waitFor();
    const edge = await other.locator('.side-edge').boundingBox();
    await other.mouse.move(edge.x + edge.width / 2, edge.y + 300);
    await other.mouse.down();
    await other.mouse.move(edge.x - 60, edge.y + 300, { steps: 6 });
    await other.mouse.move(60, edge.y + 300, { steps: 6 });
    await other.mouse.up();
    await listed(async () => String(Math.round((await theirs.boundingBox()).width)), '180', 'the side panel dragged as narrow as it goes');
    await theirSpace('Yard works');
    await listed(theirProjects, 'Shed', 'the other page to list the project where it was moved');
    const theirRow = theirs.locator('[data-project="shed"] > .nav-row');
    await theirRow.hover();
    const unmoved = await still(other);
    await theirRow.getByRole('button', { name: 'More for Shed', exact: true }).click();
    await moveLine(other).hover();
    await beside(other).waitFor();
    expect(`with the panel narrow it to be beside its menu, got ${await placed(other, 'with the panel narrow')}`, (await placed(other, 'with the panel narrow')) === 'right');
    expect('nothing of the narrow side panel to have moved', (await still(other)) === unmoved);
    await crowded(other, 'with the panel narrow');
    await other.screenshot({ path: `${out}/spaces-07d-move-to-with-the-panel-narrow.png` });
    await other.keyboard.press('Escape');
    await other.keyboard.press('Escape');
    await rowMenu(other).waitFor({ state: 'detached' });
    await other.close();
    const phone = await another(browser, url, problems, null, true);
    const onPhone = phone.getByRole('navigation', { name: 'Resources' });
    await onPhone.locator('.space-foot .space-pick').tap();
    await phone.getByRole('menu', { name: 'Workspace', exact: true }).getByRole('menuitemradio', { name: /^Yard works(:|$)/ }).tap();
    const phoneRow = onPhone.locator('[data-project="shed"] > .nav-row');
    await phoneRow.waitFor();
    const phoneStill = await still(phone);
    await phoneRow.getByRole('button', { name: 'More for Shed', exact: true }).tap();
    await moveLine(phone).tap();
    await beside(phone).waitFor();
    expect(`at a phone's width it to lie over the menu under its line, got ${await placed(phone, "at a phone's width")}`, (await placed(phone, "at a phone's width")) === 'over');
    expect("nothing of the side panel to have moved at a phone's width", (await still(phone)) === phoneStill);
    await phone.screenshot({ path: `${out}/spaces-07e-move-to-at-390.png` });
    await crowded(phone, "with the menu beside open at a phone's width", ['.row-menu:not(.row-beside)', '.row-menu.row-beside']);
    await beside(phone).getByRole('menuitemradio', { name: 'Annex', exact: true }).tap();
    const theirAsked = phone.getByRole('dialog');
    await theirAsked.getByRole('heading', { name: 'Move Shed to Annex?', exact: true }).waitFor();
    await crowded(phone, "with the question at a phone's width", ['.modal']);
    await phone.screenshot({ path: `${out}/spaces-07f-asked-at-390.png` });
    await theirAsked.getByRole('button', { name: 'Cancel', exact: true }).tap();
    await theirAsked.waitFor({ state: 'detached' });
    await phone.close();

    // ---- somebody who may not move a project or a channel is offered no line for it
    const less = await another(browser, url, problems, ['projects:read', 'channels:read', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search']);
    const lessRow = less.getByRole('navigation', { name: 'Resources' }).locator('[data-project="shop"] > .nav-row');
    await less.getByRole('navigation', { name: 'Resources' }).locator('.space-foot .space-pick').click();
    await less.getByRole('menu', { name: 'Workspace', exact: true }).getByRole('menuitemradio', { name: /^Shop front(:|$)/ }).click();
    await lessRow.hover();
    await lessRow.getByRole('button', { name: 'More for Shop', exact: true }).click();
    await rowMenu(less).waitFor();
    expect(`somebody who may not move a project to be offered no line for it, got "${await lines(rowMenu(less))}"`, (await lines(rowMenu(less))) === 'Pin to the top');
    await less.keyboard.press('Escape');
    await less.locator('.rail').getByRole('button', { name: 'Channels', exact: true }).click();
    const lessChannels = less.getByRole('navigation', { name: 'Channels' });
    await lessChannels.locator('.grouped-channel').first().click({ button: 'right' });
    await rowMenu(less).waitFor();
    expect(`nor a line that moves a channel, got "${await lines(rowMenu(less))}"`, !(await lines(rowMenu(less))).includes('Move to…'));
    await less.close();
  }

  /** A channel's row's menu: the workspaces and the projects under a heading, and the same question. */
  async function channel({ panel, row, channels, toSpace, cli, refused }) {
    const moveTo = async (name, to, picture = null) => {
      await row(name).click({ button: 'right' });
      await rowMenu().waitFor();
      await moveLine().hover();
      await beside().waitFor();
      if (picture) await page.screenshot({ path: `${out}/${picture}` });
      const offered = await lines(beside());
      await beside().locator(':scope > button', { hasText: to }).filter({ has: page.locator('.truncate', { hasText: new RegExp(`^${to}$`) }) }).first().click();
      return offered;
    };
    const asked = page.getByRole('dialog');
    const offered = await moveTo('plans', 'Annex', 'spaces-10a-a-channels-row-menu.png');
    expect(`each workspace, the one it belongs to marked, and each project under a heading, and nothing never, got "${offered}"`, offered.startsWith('Annex | Shop front ✓ (here) | Works | Projects | ') && offered.includes('Shop') && !/nothing/i.test(offered));
    await asked.getByRole('heading', { name: 'Move #plans to Annex?', exact: true }).waitFor();
    const says = (await asked.innerText()).replace(/\s+/g, ' ');
    expect(`to be told it is the workspace's own and that nobody is let in or kept out, got "${says}"`, says.includes("#plans is Annex's own from now on") && says.includes('nobody is let in or kept out'));
    await page.screenshot({ path: `${out}/spaces-10-what-a-channel-belongs-to.png` });
    await asked.getByRole('button', { name: 'Cancel', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(channels, 'plans | Shop[shop-talk]', 'a channel not moved where the question was cancelled');
    await moveTo('plans', 'Annex');
    await asked.getByRole('button', { name: 'Move it', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(channels, 'Shop[shop-talk]', 'a channel made another workspace\'s to leave the list');
    // A project's is made the workspace's own, and is the project's no more.
    await moveTo('shop-talk', 'Shop front');
    await asked.getByRole('heading', { name: 'Move #shop-talk to Shop front?', exact: true }).waitFor();
    expect('to be told it is the project\'s no longer', (await asked.innerText()).includes("It is Shop's no longer"));
    await asked.getByRole('button', { name: 'Move it', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(channels, 'shop-talk', 'a project\'s channel made the workspace\'s own');
    await cli('channel', 'belongs', 'shop-talk', '--project', 'shop');
    await toSpace(panel, 'Annex');
    await listed(channels, 'plans | Depot[depot-talk]', 'the workspace\'s own channel before its projects\'');
    // To a project, by the keyboard.
    await row('plans').click({ button: 'right' });
    await rowMenu().waitFor();
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    await moveLine().focus();
    await page.keyboard.press('ArrowRight');
    await beside().waitFor();
    for (let left = 20; left > 0 && !(await focused()).endsWith(':Depot'); left -= 1) await page.keyboard.press('ArrowDown');
    expect(`the arrows to reach a project past the heading, got ${await focused()}`, (await focused()) === 'beside:Depot');
    await page.keyboard.press('Enter');
    await asked.getByRole('heading', { name: 'Move #plans to Depot?', exact: true }).waitFor();
    expect('to be told whose the bot there is', (await asked.innerText()).includes("the factory's bot there is Depot's"));
    await asked.getByRole('button', { name: 'Move it', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(channels, 'Depot[depot-talk plans]', 'a channel made a project\'s, under it');
    // It is never taken out of everything: refused from a terminal, and it is where it was.
    await refused(['channel', 'belongs', 'plans', '--nothing'], 'never nobody\'s');
    // Put back in the workspace it was of.
    await moveTo('plans', 'Shop front');
    await asked.getByRole('button', { name: 'Move it', exact: true }).click();
    await asked.waitFor({ state: 'detached' });
    await listed(channels, 'Depot[depot-talk]', 'a channel put back in the workspace it was of');
  }

  return { project, channel };
}
