// Which factory a page is in, and the way to the others.
//
// A factory may have a profile: a name such as `prod` or `dev1`. Two
// of them look alike, and what is done in one is not done in the other, so
// a page in a factory that has one says which at all times (`views/where.tsx`).
// This is what of that can be worked out without a screen: what the tab is
// called, and what the list of the factories on a device says of each.
//
// No name means anything here. `prod` is a word somebody chose, and is
// drawn like any other.

/** A factory on this device, as the daemon lists it for the page that asked. */
export interface Profile {
  name: string;
  /** It is the one this page is in. */
  this: boolean;
  /** Where this page can open it; nothing when there is no address to give. */
  url: string | null;
  /** Why there is none, in a sentence. */
  why: string | null;
}

/** What the browser's tab is called: the profile first, since it is what differs between two tabs. */
export function titleOf(product: string, profile: string | null | undefined): string {
  return profile ? `${profile} · ${product}` : product;
}

/**
 * What the browser's tab is called in a browser, where a tab of the
 * browser's is a thing of the factory's (`inplace.ts`): the profile first
 * still, then the thing shown, then the product. The start page is the
 * product's alone, as every tab was before.
 */
export function titleShowing(product: string, profile: string | null | undefined, thing: string | null): string {
  const of = titleOf(product, profile);
  if (!thing) return of;
  return profile ? `${profile} · ${thing} · ${product}` : `${thing} · ${product}`;
}

/** A line of the list. */
export interface Place {
  /** Unlike a name, which two factories may share. */
  key: string;
  name: string;
  here: boolean;
  /** Where choosing it goes. */
  to: string | null;
  /** What is said under its name. */
  says: string;
}

/** An address as it is read: without what every address starts with. */
export function plain(url: string): string {
  return url.replace(/^https?:\/\//, '').replace(/\/$/, '');
}

/**
 * The factories as they are listed: the one the page is in first, then the
 * rest by name, and those of one name by where they are. Each says where
 * it is, or why it cannot be opened from here.
 */
export function places(profiles: Profile[]): Place[] {
  const order = (one: Profile) => `${one.this ? 0 : 1} ${one.name} ${one.url ?? ''}`;
  return [...profiles]
    .sort((a, b) => order(a).localeCompare(order(b)))
    .map((one, index) => ({
      key: `${index}:${one.name}:${one.url ?? ''}`,
      name: one.name,
      here: one.this,
      to: one.this ? null : one.url,
      says: one.this ? 'You are here.' : one.url ? plain(one.url) : (one.why ?? 'It cannot be opened from here.'),
    }));
}

/** Whether there is anywhere else to go. */
export function alone(profiles: Profile[]): boolean {
  return !profiles.some((one) => !one.this);
}
