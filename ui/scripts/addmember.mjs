// The one dialog the line at the foot of who is in a channel opens, Add or
// invite channel member (`adding.mjs`),
// held by what the page shows and by the boxes: run by `external.mjs`,
// which plays its agent from outside and hands this its page, its helpers
// and a factory with Shop, its channels `yard` (open) and `porch` (kept to
// its members), the agents helper, walls (from outside), quarry-hand and
// rover, on a plan with more than one person.
//
// Held here: the list of the agents there are under an agent's name
// (`offeredUnder`: what each line says, an agent from outside said to be
// external, narrowed by what is typed, the arrows, Enter and Escape, and
// by its boxes at three widths and a phone's); a person chosen, what they
// hold, given more from the dialog and held so by the daemon, an
// invitation to register shown once with Copy and said not to bring them
// in, and added; a bot, with nothing to set; an agent made with what it
// may ask changed in the dialog and held so by the daemon; and somebody
// who holds less offered less, with a model's usual one and its name one
// line (`choices` in `models.ts`); and somebody who may change agents and
// not apply, offered no choice of which agent and inviting alone, an
// agent this factory runs that is there brought in as a member is, and
// the route that would bring it in by applying its text refusing them.

import { unfoldChannels } from './folded.mjs';
import { openAdding } from './adding.mjs';

/** Asks the daemon from inside the page, as the page asks it. */
const asked = (page, path) =>
  page.evaluate(async (path) => {
    const answer = await fetch(path, { headers: { 'x-page-key': localStorage.getItem('page-key') ?? '' } });
    return answer.json();
  }, path);

/** Every agent, as the daemon says it, with what each holds. */
export const agentsSaid = async (page) => (await asked(page, '/api/agents')).agents;

/**
 * The list under an agent's name, as a box: in the window, under the field
 * and as wide as it, over the dialog, and no line over another or past its
 * side. What is wrong, said.
 */
const listWrong = (page) =>
  page.evaluate(() => {
    const said = [];
    const list = document.querySelector('.agents-offered');
    const field = document.querySelector('.modal input[role="combobox"]');
    if (!list || !field) return ['there is no list'];
    const box = list.getBoundingClientRect();
    const at = field.getBoundingClientRect();
    if (box.left < -0.5 || box.top < -0.5 || box.right > window.innerWidth + 0.5 || box.bottom > window.innerHeight + 0.5) said.push(`the list is outside the window: ${JSON.stringify(box)}`);
    if (box.top < at.bottom - 0.5 && box.bottom > at.top + 0.5) said.push('the list lies over the field');
    if (Math.abs(box.left - at.left) > 0.5 || Math.abs(box.width - at.width) > 0.5) said.push(`the list is not as wide as the field and level with it: ${JSON.stringify([box.left, box.width, at.left, at.width])}`);
    if (list.scrollWidth > list.clientWidth + 1) said.push('the list scrolls sideways');
    const lines = [...list.querySelectorAll('[role="option"], .agents-offered-new')].map((one) => [one.dataset.agent ?? 'its last line', one.getBoundingClientRect()]);
    for (const [name, line] of lines) {
      if (line.left < box.left - 0.5 || line.right > box.right + 0.5) said.push(`${name} is past the list's side`);
      // Over the dialog: what is drawn at the middle of each line that is in view is that line.
      if (line.top < box.top || line.bottom > box.bottom) continue;
      const hit = document.elementFromPoint(line.left + line.width / 2, line.top + line.height / 2);
      if (!hit || !list.contains(hit)) said.push(`${name} is under something`);
    }
    for (let one = 0; one < lines.length; one += 1)
      for (let other = one + 1; other < lines.length; other += 1) {
        const [a, b] = [lines[one][1], lines[other][1]];
        if (a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5) said.push(`${lines[one][0]} lies over ${lines[other][0]}`);
      }
    return said;
  });

/** The list under an agent's name in the dialog `inChannel`, which shows an agent this factory runs with no name typed. */
export async function offeredUnder(page, { out, expect, inChannel, widths }) {
  const field = inChannel.getByLabel('Name', { exact: true });
  const list = page.locator('.agents-offered');
  const names = () => list.getByRole('option').evaluateAll((all) => all.map((one) => one.dataset.agent));
  const last = () => list.locator('.agents-offered-new').innerText();
  const parts = () => inChannel.evaluate((dialog) => [...dialog.querySelectorAll('h2, label, p, .modal-foot')].filter((one) => !one.closest('.agents-offered')).map((one) => JSON.stringify(one.getBoundingClientRect())).join());
  expect('the list not to be there as the dialog opens', (await list.count()) === 0);
  const before = await parts();
  await field.click();
  await list.waitFor();
  expect('nothing of the dialog to be moved by the list', (await parts()) === before);
  const said = Object.fromEntries(await list.getByRole('option').evaluateAll((all) => all.map((one) => [one.dataset.agent, one.querySelector('.asked-under').textContent])));
  expect(
    `each agent to be said with where it works and what runs it, one from outside as external, got ${JSON.stringify(said)}`,
    said.walls === 'Shop · external' && said.helper === 'Shop' && said.rover === 'no project' && /^Quarry\b/.test(said['quarry-hand'] ?? ''),
  );
  expect(`its last line to say what a name no agent has comes to, got "${await last()}"`, (await last()) === 'A name no agent has makes a new one, run by this factory.');
  await page.screenshot({ path: `${out}/external-12-agents-offered.png` });
  // Narrowed by what is typed, by the name and by what is said beside it.
  await field.fill('ext');
  expect(`only the one from outside for "ext", got ${await names()}`, (await names()).join() === 'walls');
  expect(`a new one to be said for it, got "${await last()}"`, (await last()) === 'No agent is called ext: a new one is made.');
  await field.fill('no project');
  expect(`only the one of no project, got ${await names()}`, (await names()).join() === 'rover');
  // The arrows mark a line, round the list; Enter takes it; Escape puts the list away and leaves the dialog.
  await field.fill('shop');
  const shops = await names();
  expect(`Shop's two to be offered, got ${shops}`, shops.length === 2 && shops.includes('helper') && shops.includes('walls'));
  const marked = () => list.locator('[role="option"][aria-selected="true"]').getAttribute('data-agent');
  expect('none to be marked until a key marks one', (await list.locator('[role="option"][aria-selected="true"]').count()) === 0);
  await field.press('ArrowDown');
  expect(`the first to be marked, got ${await marked()}`, (await marked()) === shops[0]);
  await field.press('ArrowDown');
  expect(`the second to be marked, got ${await marked()}`, (await marked()) === shops[1]);
  await field.press('ArrowDown');
  expect(`the first again, round the list, got ${await marked()}`, (await marked()) === shops[0]);
  await field.press('ArrowUp');
  expect(`the last again, got ${await marked()}`, (await marked()) === shops[1]);
  await field.press('Enter');
  await list.waitFor({ state: 'detached' });
  expect(`Enter to take the one marked, got "${await field.inputValue()}"`, (await field.inputValue()) === shops[1]);
  expect('the dialog to stay', (await inChannel.count()) === 1);
  await field.press('ArrowDown');
  await list.waitFor();
  await field.press('Escape');
  await list.waitFor({ state: 'detached' });
  expect('Escape to put the list away first, the dialog staying', (await inChannel.count()) === 1 && (await field.inputValue()) === shops[1]);
  // By its boxes, with every agent and its last line, at three widths and a phone's.
  await field.fill('');
  await list.waitFor();
  const first = page.viewportSize();
  for (const [width, height, called] of widths) {
    await page.setViewportSize({ width, height });
    await page.waitForFunction(() => {
      const box = document.querySelector('.agents-offered')?.getBoundingClientRect();
      const at = document.querySelector('.modal input[role="combobox"]')?.getBoundingClientRect();
      return box && at && Math.abs(box.left - at.left) < 0.5 && Math.abs(box.width - at.width) < 0.5 && box.top >= at.bottom - 0.5;
    }, null, { timeout: 5_000 }).catch(() => {});
    await page.screenshot({ path: `${out}/external-12-agents-offered-${called}.png` });
    const wrong = await listWrong(page);
    if (wrong.length) console.log(await page.evaluate(() => JSON.stringify([document.querySelector('.agents-offered').getBoundingClientRect(), document.querySelector('.modal input[role="combobox"]').getBoundingClientRect(), window.innerHeight])));
    expect(`nothing of the list outside the window, over the field or over itself at ${width} wide, got ${wrong.join('; ')}`, wrong.length === 0);
    if (width < 500) {
      const short = await list.getByRole('option').evaluateAll((all) => all.map((one) => one.getBoundingClientRect().height).filter((tall) => tall < 40));
      expect(`each line a finger's size at a phone's width, got ${short}`, short.length === 0);
    }
  }
  await page.setViewportSize(first);
  await field.press('Escape');
  await list.waitFor({ state: 'detached' });
}

/** Opens the dialog in the channel showing, choosing who it is where there is a choice. */
const opened = async (on, who) => {
  const dialog = await openAdding(on);
  if (who) await dialog.getByRole('radio', { name: new RegExp(`^${who}`) }).check();
  return dialog;
};

const channelOf = async (on, name) => {
  // The Channels panel, by the rail, which a second press would put away; and what is under each branch unfolded.
  const rail = on.locator('.rail').getByRole('button', { name: 'Channels', exact: true });
  if ((await rail.getAttribute('aria-pressed')) !== 'true') await rail.click();
  await unfoldChannels(on);
  await on.locator('.nav').getByText(name, { exact: true }).click();
  await on.locator('.channel .view-head h1', { hasText: name }).waitFor();
};

export async function addMember({ browser, page, url, out, cli, expect, until, signIn, sound, atWidths, scheme }) {
  // Somebody else with an account here, to be added: registered here unless whoever runs this registered her first
  // (`external.mjs` does, to give her a card by her name).
  const registered = (await asked(page, '/api/accounts')).accounts.some((one) => one.email === 'grace@example.com');
  if (!registered) {
    const code = JSON.parse((await cli('invite', '--json')).stdout).code;
    const joined = await fetch(`${url}/api/auth/register`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ email: 'grace@example.com', name: 'Grace', password: 'correct horse', invite: code }),
    });
    expect(`Grace to have registered, got ${joined.status}`, joined.ok);
  }

  // ---- who it is, at the dialog's head ---------------------------------------------------------
  let dialog = await opened(page);
  const kinds = await dialog.getByRole('radiogroup', { name: 'Who it is' }).locator('strong').allTextContents();
  expect(`an agent, a person and a bot to be offered to the owner, got ${kinds}`, kinds.join() === 'An agent,A person,A bot');

  // ---- a person: chosen, what they hold, given more, invited, added ------------------------------
  await dialog.getByRole('radio', { name: /^A person/ }).check();
  const grace = dialog.getByRole('radio', { name: 'Grace', exact: true });
  await grace.waitFor();
  expect('the first person not in it to be chosen', await grace.isChecked());
  expect('Ada, who is in it, not to be offered', (await dialog.getByRole('radio', { name: 'Ada', exact: true }).count()) === 0);
  const holds = dialog.locator('.member-setting[data-setting="holds"]');
  expect('what she holds to be shown, and nothing of it to change there', (await holds.locator('.holds-table').count()) === 1 && (await holds.locator('select').count()) === 0);
  expect(`it to be said as a member's, got "${await holds.locator('.member-setting-said').innerText()}"`, (await holds.locator('.member-setting-said').innerText()) === 'What a member holds, and what Grace was given beyond it.');
  // Given more, as People in Settings gives it, and held so by the daemon.
  const give = dialog.getByLabel('A scope to give Grace');
  const scope = (await give.locator('option').evaluateAll((all) => all.map((one) => one.value))).find((one) => one && one !== 'admin');
  expect('something to be offered to give her', !!scope);
  await give.selectOption(scope);
  await dialog.getByRole('button', { name: 'Give', exact: true }).click();
  const account = async () => (await asked(page, '/api/accounts')).accounts.find((one) => one.email === 'grace@example.com');
  await until(`Grace to have been given ${scope}`, async () => (await account()).given?.includes(scope) || JSON.stringify((await account()).given));
  expect('the dialog to stay as she is given it', (await dialog.count()) === 1 && (await grace.isChecked()));
  // Somebody with no account yet: an invitation, shown this once with Copy, said not to bring them in.
  await dialog.getByRole('button', { name: 'Make an invitation', exact: true }).click();
  const shown = dialog.locator('.add-person-invite .invite-code');
  await shown.waitFor();
  expect(`an invitation to be shown, got "${await shown.innerText()}"`, /^inv_[A-Za-z0-9]+$/.test((await shown.innerText()).trim()));
  expect('it to be copied from beside it', (await dialog.locator('.add-person-invite').getByRole('button', { name: 'Copy', exact: true }).count()) === 1);
  const invitedWords = await dialog.locator('.invited-person').innerText();
  expect(`it to be said that registering does not bring them in, got "${invitedWords}"`, invitedWords.endsWith('Registering does not bring them into porch: once they have registered, add them here.'));
  await atWidths(page, 'the dialog adding a person', '12-person');
  const graceId = (await account()).id;
  await dialog.getByRole('button', { name: 'Add Grace', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  const porch = async () => (await asked(page, '/api/channels')).channels.find((one) => one.title === 'porch' || one.name === 'porch');
  await until('Grace to be in porch', async () => (await porch()).members.some((one) => one.actor === `user:${graceId}`) || (await porch()).members.map((one) => one.actor).join());

  // ---- a bot: nothing to set -----------------------------------------------------------------
  dialog = await opened(page, 'A bot');
  await dialog.locator('form.add-bot').waitFor();
  expect('nothing to be set of a bot yet', (await dialog.locator('.member-settings').count()) === 0);
  await atWidths(page, 'the dialog adding a bot', '12-bot');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });

  // ---- an agent made, with what it may ask set in the dialog ----------------------------------
  await channelOf(page, 'yard');
  dialog = await opened(page, 'An agent');
  await dialog.getByLabel('Name', { exact: true }).fill('Mason');
  await page.locator('.agents-offered').waitFor();
  await dialog.getByLabel('Name', { exact: true }).press('Escape');
  await page.locator('.agents-offered').waitFor({ state: 'detached' });
  const table = dialog.locator('.member-setting .holds-table');
  await table.getByLabel('What it may do with board').selectOption({ label: 'read' });
  await atWidths(page, 'the dialog making an agent', '12-made');
  await dialog.getByRole('button', { name: 'Add agent', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  const mason = (await agentsSaid(page)).find((one) => one.name === 'mason');
  expect(`it to hold what was set in the dialog, got ${mason?.scopes}`, !!mason && mason.scopes.includes('board:read') && !mason.scopes.includes('board'));

  // ---- somebody who holds less, offered less ----------------------------------------------------
  // A member, as a page is told what somebody holds; and an agent's package that lists a model as its usual one by a
  // word of its own and names it, which is put together with that model's own line.
  const MEMBER = ['projects', 'channels', 'board', 'records', 'blueprints', 'agents', 'fleet', 'search'];
  const theirs = await (await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme })).newPage();
  await theirs.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const me = await answer.json();
    if (!me.principal) return route.fulfill({ response: answer });
    me.principal.scopes = MEMBER;
    me.can_invite = false;
    await route.fulfill({ response: answer, json: me });
  });
  await theirs.route('**/api/agents', async (route) => {
    if (route.request().method() !== 'GET') return route.continue();
    const answer = await route.fetch();
    const said = await answer.json();
    said.providers = [...(said.providers ?? []), { name: 'stand-in', title: 'Stand-in', description: '', kind: 'extension', available: true, models: 'asked', permissions: ['read', 'edit'] }];
    await route.fulfill({ response: answer, json: said });
  });
  await theirs.route('**/api/providers/stand-in/models', (route) =>
    route.fulfill({
      json: {
        models: [
          { id: 'default', label: 'Default (recommended)', note: 'Model 5.5 · for most things', default: true },
          { id: 'm55', label: 'Model 5.5', note: 'For most things', default: false },
          { id: 'quick', label: 'Quick', note: 'For quick answers', default: false },
        ],
      },
    }),
  );
  await signIn(theirs);
  await channelOf(theirs, 'yard');
  dialog = await opened(theirs);
  const theirKinds = await dialog.getByRole('radiogroup', { name: 'Who it is' }).locator('strong').allTextContents();
  expect(`a member, who holds no shell, to be offered no bot, got ${theirKinds}`, theirKinds.join() === 'An agent,A person');
  await dialog.getByLabel('Runs on').selectOption('stand-in');
  const model = dialog.locator('label', { hasText: /^Model/ }).locator('select');
  await theirs.waitForFunction(() => [...document.querySelectorAll('.modal select')].some((one) => [...one.options].some((option) => option.textContent === 'Quick')));
  const models = await model.locator('option').allTextContents();
  expect(`the usual model and its name to be one line, got ${models}`, models.join('|') === 'Model 5.5 (recommended)|Quick|Another model');
  expect('choosing it to name no model', (await model.inputValue()) === '');
  await sound(theirs, 'the dialog for a member, with a model chosen');
  await theirs.screenshot({ path: `${out}/external-12-member-models.png` });
  // Invited from outside, it begins on what such an agent usually holds as far as they hold that, and nothing more is offered.
  await dialog.getByRole('radio', { name: /^From outside/ }).check();
  const theirTable = dialog.locator('.member-setting .holds-table');
  const inbox = theirTable.getByLabel('What it may do with inbox');
  const inboxNow = await inbox.evaluate((one) => one.options[one.selectedIndex].textContent.trim());
  expect(`what they do not hold not to be begun on, got "${inboxNow}"`, inboxNow === 'nothing');
  const offeredInbox = await inbox.locator('option').evaluateAll((all) => all.filter((one) => !one.disabled).map((one) => one.textContent.trim()));
  expect(`nothing they do not hold to be offered, got ${offeredInbox}`, offeredInbox.join() === 'nothing');
  await atWidths(theirs, 'the dialog inviting, for somebody who holds less', '12-member-invite');
  await theirs.keyboard.press('Escape');
  await dialog.waitFor({ state: 'detached' });
  await theirs.context().close();

  // ---- somebody who may change agents and not apply ---------------------------------------------
  // One this factory runs is made, and brought in by its name, by applying `std/agent` as whoever asks (`blueprints:run`): they may only invite.
  const INVITES = ['projects', 'channels', 'board', 'records', 'blueprints:read', 'agents', 'fleet', 'search'];
  const yard = (await asked(page, '/api/channels')).channels.find((one) => one.name === 'yard');
  expect('rover not to be in yard yet', !yard.members.some((one) => one.actor === 'agent:rover'));
  const key = JSON.parse((await cli('key', 'bringer', ...INVITES.flatMap((one) => ['--scope', one]), '--json')).stdout).token;
  const byName = await fetch(`${url}/api/channels/${yard.id}/agents`, { method: 'POST', headers: { 'content-type': 'application/json', authorization: `Bearer ${key}` }, body: JSON.stringify({ name: 'rover' }) });
  expect(`the daemon to refuse bringing one in by applying its text, got ${byName.status}`, byName.status === 403 && (await byName.text()).includes('needs `blueprints:run`'));
  const inviter = await (await browser.newContext({ viewport: { width: 1280, height: 860 }, colorScheme: scheme })).newPage();
  await inviter.route('**/api/me', async (route) => {
    const answer = await route.fetch();
    const me = await answer.json();
    if (!me.principal) return route.fulfill({ response: answer });
    me.principal.scopes = INVITES;
    me.can_invite = false;
    await route.fulfill({ response: answer, json: me });
  });
  await signIn(inviter);
  await channelOf(inviter, 'yard');
  dialog = await opened(inviter, 'An agent');
  await dialog.locator('form.add-agent').waitFor();
  expect('them to be offered no choice of which agent it is', (await dialog.getByRole('radiogroup', { name: 'Which agent it is' }).count()) === 0);
  expect(`what an agent from outside is to be said, got "${await dialog.locator('.add-agent-said').innerText()}"`, (await dialog.locator('.add-agent-said').innerText()).startsWith('An agent from outside is a program'));
  const name = dialog.getByLabel('Name', { exact: true });
  await name.fill('Gate');
  await inviter.locator('.agents-offered').waitFor();
  await name.press('Escape');
  await inviter.locator('.agents-offered').waitFor({ state: 'detached' });
  expect('a new one to be invited, and nothing of a program asked', (await dialog.getByRole('button', { name: 'Invite', exact: true }).count()) === 1 && (await dialog.getByLabel('Runs on').count()) === 0 && (await dialog.getByLabel('How long the invitation works').count()) === 1);
  await atWidths(inviter, 'the dialog for somebody who may only invite', '12-invite-alone');
  // One this factory runs that is there is brought in with nothing else of it changed, as a member is, by the route that takes them.
  await name.fill('rover');
  await inviter.locator('.agents-offered').waitFor();
  await name.press('Escape');
  await inviter.locator('.agents-offered').waitFor({ state: 'detached' });
  await dialog.locator('.add-agent-there').waitFor();
  expect('what it holds to be shown and nothing of it to change there', (await dialog.locator('.member-setting .holds-table select').count()) === 0);
  const asks = [];
  inviter.on('request', (request) => request.method() === 'POST' && request.url().includes('/api/channels/') && asks.push(new URL(request.url()).pathname));
  await dialog.getByRole('button', { name: 'Add agent', exact: true }).click();
  await dialog.waitFor({ state: 'detached' });
  expect(`it to be brought in as a member is, got ${asks}`, asks.length === 1 && asks[0] === `/api/channels/${yard.id}/members`);
  const now = (await asked(page, '/api/channels')).channels.find((one) => one.id === yard.id);
  expect('rover to be in yard', now.members.some((one) => one.actor === 'agent:rover'));
  await inviter.context().close();
}
