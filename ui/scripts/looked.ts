// A check of the client run with a theme's look, told in words by the
// environment: `LOOK="size=largest density=compact"`.
//
// What a theme says of how the client looks (`src/looks.ts`) is what most
// changes where things stand, so a check that holds that nothing lies over
// anything else is run with one as well as plain. A check makes its own
// people and their themes, and none of them says anything of a look, so the
// look is put on every page a check's browser opens, as each of its
// documents begins, as the measures a theme saying it would put there, and
// above whatever the page itself puts there: the page's own theme is still
// worn, with its colours, and with this look in place of its own. It is one
// place, read by every check through `where.ts`, so that no check is
// written for it. Told nothing, it does nothing.

import { chromium } from 'playwright-core';
import type { Browser, BrowserContext, Page } from 'playwright-core';
import { lookOf, toldLook } from '../src/looks.ts';

/**
 * The checks that wear looks of their own and hold the page to each, which a look told from outside would stand
 * over and so undo: `themed.mjs` makes themes that say looks, waits for the page to wear each as it is chosen, and
 * holds the whole page by the boxes at the largest size, the most compact density and both. Told a look, such a
 * check is run as it is, and says so.
 */
const OWN_LOOKS = ['themed.mjs'];
const ownLooks = OWN_LOOKS.some((check) => (process.argv[1] ?? '').endsWith(`/${check}`));

/** The look this check is told to wear, by its measures; nothing where it was told none, or wears looks of its own. */
export const LOOKED = (() => {
  const look = toldLook(process.env['LOOK']);
  if (look && ownLooks) console.log(`(told the look ${process.env['LOOK']}, which this check stands aside from: it wears looks of its own)`);
  return look && !ownLooks ? lookOf(look) : null;
})();

/**
 * How wide or tall a window is that is as wide or tall, for text of the size told, as `pixels` is for text of the
 * usual size: for a check that holds things to fit beside each other on one line, or under each other in one
 * window, of so many pixels, which everything being larger makes a smaller window.
 */
export function sized(pixels: number): number {
  return Math.round(pixels * Number(LOOKED?.['--size'] ?? 1));
}

/**
 * What is run as each document of a page begins: the measures put in its head, held over what the page puts on
 * itself, in the page's own documents and in no frame of one.
 */
function wearing(measures: Record<string, string>): string {
  const said = Object.entries(measures)
    .map(([name, value]) => `${name}: ${value} !important;`)
    .join(' ');
  return `(() => {
    if (window.top !== window) return;
    const put = () => {
      const style = document.createElement('style');
      style.dataset.looked = '';
      style.textContent = ${JSON.stringify(`:root { ${said} }`)};
      document.documentElement.append(style);
    };
    // A document is begun with nothing in it: the look is put in as soon as it has its root.
    if (document.documentElement) return put();
    const waiting = new MutationObserver(() => {
      if (!document.documentElement) return;
      waiting.disconnect();
      put();
    });
    waiting.observe(document, { childList: true });
  })();`;
}

/**
 * Has every document `page` comes to begin with the look. It is run in a world of its own, beside the page's
 * scripts and not among them: put among them it would be run in every frame of the page too, and a frame that may
 * run no script (a document shown whole) would have the browser say it refused it. Nor is the page's document
 * changed on its way: one handed to the browser by the check is no longer the daemon's on this machine, and the
 * browser kept a frame of it from what the daemon serves beside it.
 */
async function wear(page: Page, source: string): Promise<void> {
  const told = await page.context().newCDPSession(page);
  await told.send('Page.enable');
  await told.send('Page.addScriptToEvaluateOnNewDocument', { source, worldName: 'looked' });
}

if (LOOKED) {
  const source = wearing(LOOKED);
  const launch = chromium.launch.bind(chromium);
  chromium.launch = async (...asked: Parameters<typeof chromium.launch>): Promise<Browser> => {
    const browser = await launch(...asked);
    // A page made by `newPage` is made in a context of its own by `newContext`, so that is where it is put on: on
    // each page the context makes before whoever asked for it has it, and on one it opens by itself as it opens.
    const newContext = browser.newContext.bind(browser);
    browser.newContext = async (...options: Parameters<Browser['newContext']>): Promise<BrowserContext> => {
      const context = await newContext(...options);
      const newPage = context.newPage.bind(context);
      const worn = new WeakMap<Page, Promise<void>>();
      const once = (page: Page) => {
        if (!worn.has(page)) worn.set(page, wear(page, source));
        return worn.get(page)!;
      };
      context.newPage = async () => {
        const page = await newPage();
        await once(page);
        return page;
      };
      context.on('page', (page) => void once(page).catch(() => undefined));
      return context;
    };
    return browser;
  };
  console.log(`(every page is worn with the look ${process.env['LOOK']}: ${JSON.stringify(LOOKED)})`);
}
