// A page of a library or a mockup's address, opened where it says nothing
// beside the browser's cookie (typed, kept, copied before): the daemon
// sends the browser here, to the client's page, saying where it was going
// (`/?to=...`, `crates/api/src/through.rs`). The cookie reads nothing by
// itself, since a browser hands it to every port of the machine; this page
// has what to say beside it, and sends the browser on with it, in place of
// the address it came at, so that Back does not come here again. Somebody
// not signed in signs in here first and is sent on then.

import { pageShows, shown } from './api.ts';

/** What the daemon says, in this page's address, to send the browser on to. */
export const THROUGH = 'to';

/**
 * Where the browser is to be sent on to, from what is after the `?` of
 * this page's address: a page of a library or a mockup's address on this
 * same daemon, and nothing else, whatever the address says.
 */
export function sentOn(search: string): string | null {
  let to: string | null;
  try {
    to = new URLSearchParams(search).get(THROUGH);
  } catch {
    return null;
  }
  if (!to || to.startsWith('//') || to.includes('\\')) return null;
  return to.startsWith('/library/') || to.startsWith('/mockups/') ? to : null;
}

/**
 * Sends the browser on, where this page was opened to be sent on and has
 * what to say beside the cookie: whether it did, so that nothing else is
 * read for a page that is going.
 */
export function sendsOn(): boolean {
  const to = typeof location === 'undefined' ? null : sentOn(location.search);
  // Without what to say, the daemon would only send it back here.
  if (!to || !pageShows()) return false;
  location.replace(shown(to));
  return true;
}
