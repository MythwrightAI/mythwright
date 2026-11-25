// A check of the client run as the desktop app's window shows it: with tabs.
//
// In a browser the client shows one thing at a time, in place, and the
// browser's own tabs are its tabs; in the desktop app's window it keeps
// tabs of its own, split into groups, a set for each workspace
// (`src/inplace.ts`). Which it is in, the window says as it loads the page
// (`window.desktop`, with `window.ipc` to ask it things: `src/desktop.ts`).
// The checks were written while every page had tabs, and press a tab by its
// name, close one, split a group: so every page a check's browser opens is
// told it is in the window, as each of its documents begins, by stand-ins
// for those two that say the system draws the window's bar (`system`), so
// that the page draws none and looks as a browser's page does but for its
// tabs. It is one place, read by every check through `where.ts`, so that
// no check is written for it. `window.mjs` says it is framed otherwise
// itself, and is left so.
//
// The checks of the browser's own way say so (`IN_PLACE`): their pages are
// told nothing, and are a browser's.

import { chromium } from 'playwright-core';
import type { Browser, BrowserContext } from 'playwright-core';

/** The checks of the browser's way, where one thing shows at a time in place: told nothing, as a browser is. */
const IN_PLACE = ['inplace.mjs'];

/** Whether this check is of the browser's way. */
export const inPlace = IN_PLACE.some((check) => (process.argv[1] ?? '').endsWith(`/${check}`));

/** What the window says to the page before any of its own is run, as a stand-in: only in the top document, and only where nothing said otherwise (`window.mjs`). */
function window(): void {
  const here = globalThis as unknown as { top: unknown; ipc?: unknown; desktop?: unknown };
  if (here.top !== here || here.desktop) return;
  here.ipc = { postMessage: () => undefined };
  here.desktop = { frame: 'system' };
}

if (!inPlace) {
  const launch = chromium.launch.bind(chromium);
  chromium.launch = async (...asked: Parameters<typeof chromium.launch>): Promise<Browser> => {
    const browser = await launch(...asked);
    // A page made by `newPage` is made in a context of its own by `newContext`, so that is where it is told.
    const newContext = browser.newContext.bind(browser);
    browser.newContext = async (...options: Parameters<Browser['newContext']>): Promise<BrowserContext> => {
      const context = await newContext(...options);
      await context.addInitScript(window);
      return context;
    };
    return browser;
  };
}
