// Deciding what a page does when the client it was loaded from is replaced.
//
// The daemon serves the client from disk and records `client.updated` when a
// new build is there; it is not restarted for this, and neither is anything
// it runs. A page never reloads under someone who is in the middle of
// something: it offers. Only a page from a development build (`npm run dev`)
// reloads by itself, and not over unsaved edits.

/** What this page was loaded from. */
export interface Loaded {
  /** The build's id, or null when the page did not come from a build. */
  build: string | null;
  /** From `npm run dev`: whoever has it open is working on the client. */
  live: boolean;
}

export type Verdict = 'current' | 'offer' | 'reload';

/** What to do on hearing that the client being served is build `served`. */
export function verdict(loaded: Loaded, served: unknown, unsaved: boolean): Verdict {
  if (!loaded.build || typeof served !== 'string' || served === '' || served === loaded.build) return 'current';
  return loaded.live && !unsaved ? 'reload' : 'offer';
}

/** Reads what the build wrote into the entry page. */
export function loadedFrom(page: { querySelector(selector: string): { getAttribute(name: string): string | null } | null }): Loaded {
  const tag = page.querySelector('meta[name="build"]');
  const build = tag?.getAttribute('content') ?? '';
  // An entry page that did not go through the build still holds the placeholder.
  return { build: build !== '' && !build.startsWith('%') ? build : null, live: tag?.getAttribute('data-live') === 'yes' };
}
