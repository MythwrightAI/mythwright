// The part of Settings for the workspace the side panel is of, as spaces.mjs
// goes through it (`views/spacesettings.tsx`): opened from the foot's
// menu, put away, held by its boxes, and drawn for somebody who may not
// change workspaces; and taking it away from the very foot of Settings,
// behind a question that opens its button only once the workspace's name
// is typed as it is written. It is a file of its own because spaces.mjs
// was at its limit of lines.

import { pageCrowded } from './crowded.mjs';

/**
 * What spaces.mjs asks of the part, on its page: `expect`, `listed` and
 * `footDoes` are its own.
 */
export function workspacePart({ page, expect, listed, footDoes }) {
  // The part of Settings for the workspace showing, opened from the foot's menu.
  const part = page.getByRole('region', { name: 'Workspace', exact: true });
  // Taking it away, at the very foot of Settings.
  const removal = page.getByRole('region', { name: 'Remove this workspace', exact: true });
  const named = part.getByLabel('Name', { exact: true });
  const settingsOf = async (nav, name) => {
    await footDoes(nav, 'Settings');
    await listed(() => named.inputValue(), name, 'the part of Settings to be of the workspace showing');
    // Opened from the menu, it is what is in view.
    await listed(
      () => part.locator('.panel-head').evaluate((head) => String(head.getBoundingClientRect().top >= 0 && head.getBoundingClientRect().bottom <= window.innerHeight)),
      'true',
      'the workspace\'s part of Settings to be in view',
    );
  };
  const closeSettings = async () => {
    await page.getByRole('button', { name: 'Close Settings', exact: true }).click();
    await page.locator('.tabs .tab', { hasText: 'Settings' }).waitFor({ state: 'detached' });
  };
  // The part by its boxes: nothing of it lies over anything else of it, all of it is inside it, it is inside the
  // window across, and nothing scrolls sideways.
  const partBoxes = async (where, of = part) => {
    await of.waitFor();
    const got = await of.evaluate((panel) => {
      const round = (figure) => Math.round(figure * 10) / 10;
      const box = (one) => {
        const at = one.getBoundingClientRect();
        return { what: one.className || one.tagName, left: round(at.left), right: round(at.right), top: round(at.top), bottom: round(at.bottom) };
      };
      const sized = (at) => at.right - at.left > 0.5 && at.bottom - at.top > 0.5;
      const view = panel.closest('.view-body')?.parentElement ?? panel.parentElement;
      return {
        screen: window.innerWidth,
        panel: box(panel),
        parts: [...panel.querySelectorAll('.panel-head h2, .space-settings-label, .space-settings-name > *, .space-settings-about, .space-settings-grave-said, .space-settings-grave button, [role="alert"]')].map(box).filter(sized),
        cut: [...panel.querySelectorAll('.space-settings-grave button, .space-settings-label')].some((one) => one.scrollWidth > one.clientWidth + 0.5),
        sideways: Math.max(panel.scrollWidth - panel.clientWidth, view.scrollWidth - view.clientWidth, document.documentElement.scrollWidth - window.innerWidth),
      };
    });
    const told = JSON.stringify(got);
    const over = (one, other) => one.left < other.right - 0.5 && other.left < one.right - 0.5 && one.top < other.bottom - 0.5 && other.top < one.bottom - 0.5;
    const lying = got.parts.flatMap((one, at) => got.parts.slice(at + 1).filter((other) => over(one, other)).map((other) => `${one.what} over ${other.what}`));
    expect(`nothing of the workspace's part of Settings to lie over anything else of it ${where}: ${lying.join(', ')} ${told}`, lying.length === 0);
    expect(`all of the part to be inside it, and it inside the window ${where}: ${told}`, got.parts.every((one) => one.left >= got.panel.left - 0.5 && one.right <= got.panel.right + 0.5 && one.top >= got.panel.top - 0.5 && one.bottom <= got.panel.bottom + 0.5) && got.panel.left >= -0.5 && got.panel.right <= got.screen + 0.5);
    expect(`nothing of the part cut short, and nothing scrolling sideways ${where}: ${told}`, !got.cut && got.sideways <= 0);
    return got;
  };
  // Taking the workspace away is the last part of Settings, under every other one, and held by its boxes as the part is.
  const atTheFoot = async (where) => {
    await removal.waitFor();
    const got = await removal.evaluate((panel) => {
      const all = [...panel.parentElement.children].filter((one) => one.classList.contains('panel'));
      const bottom = Math.max(...all.filter((one) => one !== panel).map((one) => one.getBoundingClientRect().bottom));
      return { last: all.at(-1) === panel, top: panel.getBoundingClientRect().top, bottom, parts: all.length };
    });
    expect(`taking the workspace away to be the last part of Settings, under every other ${where}: ${JSON.stringify(got)}`, got.last && got.top >= got.bottom - 0.5);
    await partBoxes(where, removal);
  };
  // The question opens its button only once the workspace's name is typed exactly, and says why it is closed until then.
  const typedName = async (going, name, { closely = false } = {}) => {
    const field = going.getByLabel('Its name, typed to say it is this one');
    const button = going.getByRole('button', { name: 'Remove workspace', exact: true });
    const why = async () => (await going.locator('#space-typed-why').innerText()).trim();
    if (closely) {
      expect('the name\'s field to have the keyboard as the question opens', await field.evaluate((input) => document.activeElement === input));
      expect(`the button to be closed with nothing typed, saying why, got "${await why()}"`, (await button.isDisabled()) && (await why()) === `Type ${name} to remove it.`);
      for (const near of [name.toLowerCase(), `${name} `, name.slice(0, -1)]) {
        await field.fill(near);
        await listed(why, `What is typed is not ${name}: type its name as it is written.`, `the button closed for "${near}", saying why`);
        expect(`the button to stay closed for "${near}"`, await button.isDisabled());
        // Enter does nothing while it is closed.
        await field.press('Enter');
        expect(`the question to stay with "${near}" typed and Enter pressed`, await going.isVisible());
      }
    }
    await field.fill(name);
    await listed(why, 'It cannot be undone.', 'the question to say it cannot be undone once the name is typed');
    expect('the button to open once the name is typed exactly', !(await button.isDisabled()));
    await questionBoxes(`with ${name} typed at ${page.viewportSize().width} wide`);
    return button;
  };
  // Nothing of the question lies over anything else of it, past the window's sides or scrolling sideways.
  const questionBoxes = async (where) => {
    const wrong = await pageCrowded(page, '.modal');
    expect(`nothing of the question that takes a workspace away to be crowded ${where}: ${wrong.join('; ')}`, wrong.length === 0);
  };
  return { part, removal, named, settingsOf, closeSettings, partBoxes, atTheFoot, typedName, questionBoxes };
}

/**
 * Somebody who may not change workspaces, on a page of their own that is
 * told they hold everything a member does but that: the foot's menu offers
 * nothing new, and Settings the name alone. And the daemon refuses a
 * credential that holds so, made from `page`, whatever a page draws.
 */
export async function mayNot({ browser, page, url, out, expect, problems, USUAL }) {
  const holds = ['projects:read', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];
  const less = await browser.newPage({ viewport: { width: 1350, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  less.on('pageerror', (err) => problems.push(err.message));
  // What a page draws goes by what the daemon says somebody holds: here, everything a member holds but changing projects.
  await less.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const me = await answer.json();
    if (!me.principal) return route.fulfill({ response: answer });
    me.principal.scopes = holds;
    await route.fulfill({ response: answer, json: me });
  });
  await less.goto(url);
  await less.getByLabel('Email').fill('ada@example.com');
  await less.getByLabel('Password').fill('correct horse');
  await less.getByRole('button', { name: 'Sign in', exact: true }).last().click();
  await less.locator('.rail').waitFor();
  const theirs = less.getByRole('navigation', { name: 'Resources' });
  if ((await less.locator('.rail').getByRole('button', { name: 'Resources', exact: true }).getAttribute('aria-pressed')) !== 'true') await less.locator('.rail').getByRole('button', { name: 'Resources', exact: true }).click();
  await theirs.locator('.space-foot .space-pick').click();
  const menu = less.getByRole('menu', { name: 'Workspace', exact: true });
  await menu.waitFor();
  const lines = (await menu.getByRole('menuitem').allInnerTexts()).map((line) => line.trim()).join(' | ');
  expect(`somebody who may not change workspaces to be offered Settings and Log out and nothing new, got "${lines}"`, lines === 'Settings | Log out');
  await menu.getByRole('menuitem', { name: 'Settings', exact: true }).click();
  const said = less.getByRole('region', { name: 'Workspace', exact: true });
  await said.waitFor();
  await said.locator('.space-settings-name strong').waitFor();
  expect('no field to change the name, and nothing that takes it away', (await said.locator('input').count()) === 0 && (await said.getByRole('button').count()) === 0);
  expect('nothing at the foot of Settings that takes it away', (await less.getByRole('region', { name: 'Remove this workspace', exact: true }).count()) === 0);
  expect(`the name of the workspace its panel is of to be said, got "${await said.locator('.space-settings-name strong').innerText()}"`, (await said.locator('.space-settings-name strong').innerText()) === (await theirs.locator('.space-foot .space-said strong').innerText()));
  await less.mouse.move(820, 560);
  await less.screenshot({ path: `${out}/spaces-15a-settings-for-somebody-who-may-not.png` });
  await less.close();
  // And the daemon refuses a credential that holds so, whatever a page draws.
  const made = await page.evaluate(
    (scopes) => fetch('/api/credentials', { method: 'POST', headers: { 'content-type': 'application/json', 'x-page-key': localStorage['page-key'] }, body: JSON.stringify({ name: 'narrow', scopes }) }).then((answer) => answer.json()),
    holds,
  );
  for (const [method, body] of [['PATCH', { name: 'Theirs' }], ['DELETE', { to: USUAL }]]) {
    const answer = await fetch(`${url}/api/workspaces/annex`, { method, headers: { authorization: `Bearer ${made.token}`, 'content-type': 'application/json' }, body: JSON.stringify(body) });
    expect(`a credential that may not change projects to be refused, ${method} got ${answer.status}`, answer.status === 403);
  }
}
