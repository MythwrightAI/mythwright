// The page in the dark or the light, once it has followed.
//
// The page is light by a mark it puts on itself, `data-light` on the
// document (`src/wearing.ts`), as the system tells it the scheme has
// changed. That is a moment after the change: a check that has the
// browser ask for the other scheme and reads a colour at once can read
// the light the page was in, and, comparing two reads, compare the same
// light with itself. A check that switches the scheme does it by `litAs`,
// which is done once the page wears what was asked. Never a pause in its
// place.
//
// Where the reader chose always light or always dark (`appearance.ts`),
// the system's scheme does not move the page: the checks leave that
// choice as it starts, following the system.

/** Has the browser ask for `scheme` ('dark' or 'light'), and waits until the page is in it. */
export async function litAs(page, scheme) {
  await page.emulateMedia({ colorScheme: scheme });
  await page.waitForFunction((light) => document.documentElement.hasAttribute('data-light') === light, scheme === 'light');
}
