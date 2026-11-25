// Where a page is, as `head.mjs` holds it: that check's own part for a
// thing's address, in a file of its own because the check was
// at its limit of lines. It starts no daemon: it is handed the page
// `head.mjs` has open on Shop's tickets, its browser and what it does
// with them, and how that check says what it expected.
//
// A thing's address is a path that names it (`/shop/tickets`), with how
// it is shown after the `#` (`#rows`, a view by its name): copied from the
// head as the bar says it while its tab is showing, the bar following
// another tab and a view chosen; opened in a second page beside what was
// open, at a view, and by what is after the `#` alone under an open page;
// one that names nothing shows that thing's page saying so, one that makes
// no sense opens nothing, one written before addresses were paths opens
// what it did and is then said the new way; and one opened by somebody
// who is not signed in waits in the bar and is opened, at its view, once
// they are.

/** The part, handed what `head.mjs` has: its page on Shop's tickets with the head's link and gear, its browser and context, and how it opens a thing, signs in and says what it expected. */
export async function addresses({ page, context, browser, url, link, gear, menu, open, signIn, expect, problems }) {
  // ---- an address that opens the page: its path names the thing, and the bar follows the tab showing
  await link.click();
  const address = await page.evaluate(() => navigator.clipboard.readText());
  expect(`the address copied to name the table by its project and its name, got "${address}"`, address === `${url}/shop/tickets`);
  expect(`the bar to say the same as the tab shows, got ${page.url()}`, page.url() === address);
  await page.locator('.toast', { hasText: 'Copied' }).waitFor();
  await gear.click();
  await menu.getByRole('menuitem', { name: 'Copy link', exact: true }).click();
  expect('the line of the menu to copy the same', (await page.evaluate(() => navigator.clipboard.readText())) === address);
  // Another tab shown, the bar says it.
  await open('/project/shop', 'Shop');
  await page.waitForFunction((to) => location.href === to, `${url}/shop`);
  // A view by its name: chosen, the bar and what is copied say it after the #.
  await open('/table/shop/plain', 'Shop plain');
  await page.locator('.view-tab', { hasText: 'Rows' }).click();
  await page.waitForFunction((to) => location.href === to, `${url}/shop/plain#rows`);
  await link.click();
  expect('a view chosen to be said after the # of what is copied', (await page.evaluate(() => navigator.clipboard.readText())) === `${url}/shop/plain#rows`);
  // Opened in a second page of the same browser: that tab is showing, beside what the reader had open, and the bar says it.
  const second = await context.newPage();
  second.on('pageerror', (err) => problems.push(err.message));
  await second.goto(`${url}/shop/board`);
  await second.getByRole('heading', { name: 'Shop board', level: 1 }).waitFor();
  expect(`the bar to say the board once its tab is showing, got ${second.url()}`, second.url() === `${url}/shop/board`);
  expect('what the reader had open to be open still, beside it', (await second.locator('.tabs .tab', { hasText: 'Shop tickets' }).count()) === 1);
  // A view by its name, opened under an open page by what is after the # alone.
  await second.goto(`${url}/shop/plain#newest`);
  await second.getByRole('heading', { name: 'Shop plain', level: 1 }).waitFor();
  await second.locator('.view-tab[aria-selected="true"]', { hasText: 'Newest' }).waitFor();
  await second.evaluate(() => (location.hash = 'rows'));
  await second.locator('.view-tab[aria-selected="true"]', { hasText: 'Rows' }).waitFor();
  await second.waitForFunction((to) => location.href === to, `${url}/shop/plain#rows`);
  // One that names something that is not there shows that thing's own page, saying so; one that makes no sense opens nothing.
  await second.goto(`${url}/shop/nothing`);
  await second.getByText('There is no such table here').waitFor();
  const tabs = await second.locator('.tabs .tab').count();
  await second.goto(`${url}/settings/nonsense`);
  await second.waitForFunction(() => document.querySelectorAll('.tabs .tab').length > 0 && !location.pathname.startsWith('/settings'));
  expect('an address that makes no sense to open nothing', (await second.locator('.tabs .tab').count()) === tabs);
  // One written before addresses were paths opens what it always did, and the bar then says it the new way.
  await second.goto(`${url}/#/table/shop/tickets`);
  await second.getByRole('heading', { name: 'Shop tickets', level: 1 }).waitFor();
  await second.waitForFunction((to) => location.href === to, `${url}/shop/tickets`);
  await second.close();
  // Opened by somebody who is not signed in: it waits where it is, and comes to its tab once they have signed in.
  const stranger = await browser.newContext({ viewport: { width: 1350, height: 860 }, colorScheme: process.env.SCHEME ?? 'dark' });
  const theirs = await stranger.newPage();
  theirs.on('pageerror', (err) => problems.push(err.message));
  await theirs.goto(`${url}/shop/plain#newest`);
  await theirs.getByLabel('Email').waitFor();
  expect('the address to wait while nobody is signed in', theirs.url() === `${url}/shop/plain#newest`);
  await signIn(theirs);
  await theirs.getByRole('heading', { name: 'Shop plain', level: 1 }).waitFor();
  await theirs.locator('.view-tab[aria-selected="true"]', { hasText: 'Newest' }).waitFor();
  // The view a table is shown by first is one the bar need not say.
  await theirs.waitForFunction((to) => location.href === to, `${url}/shop/plain`);
  await stranger.close();
}
