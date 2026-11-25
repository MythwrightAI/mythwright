// The drawer, once it is where it will be.
//
// The drawer slides in from the right edge when it opens, so for a
// moment it is on the page and not yet in its place: a box read then, of
// the drawer or of anything in it, says where it was on its way, and a
// picture taken then shows it part of the way out. The drawer says when
// it is at rest (`data-rest`, set by the client once what slid it in is
// over, and at once where nothing slides). A check that measures the
// drawer, or anything in it, or takes its picture, finds it by this:
// what is found is at rest, and waiting for it is waiting for that.
// Never a pause in its place.

/** How the drawer is found once it is at rest, for a check that finds things by a selector. */
export const DRAWER_AT_REST = '.drawer[data-rest]';

/** The drawer of a page, once it is at rest: found, it has come in; waited for to be gone, it is gone. */
export const drawerAtRest = (page) => page.locator(DRAWER_AT_REST);

// What slides a drawer in, held part of the way in the moment the drawer is put on the page, so that it
// is looked at on its way whatever the machine is doing; what is said of it then is kept as `window.slid`,
// and `window.unwatched()` stops looking. Where nothing moves it, it says so and that it is at rest.
export const watchComing = (page) =>
  page.evaluate(() => {
    window.slid = null;
    const seen = new MutationObserver(() => {
      const drawer = document.querySelector('.drawer');
      if (!drawer || window.slid) return;
      const [slide] = drawer.getAnimations();
      const rested = 'rest' in drawer.dataset;
      if (!slide) return void (window.slid = { name: 'none', rested });
      const frames = slide.effect.getKeyframes();
      const moved = [...new Set(frames.flatMap((frame) => Object.keys(frame)))].filter((key) => !['offset', 'computedOffset', 'easing', 'composite'].includes(key)).sort();
      const took = slide.effect.getComputedTiming().duration;
      window.slid = { name: slide.animationName, took, eased: frames[0].easing, moved, rested };
      slide.pause();
      // Early in it: it is eased, and has most of its way behind it by half its time.
      slide.currentTime = took / 6;
    });
    seen.observe(document.querySelector('.main'), { childList: true });
    window.unwatched = () => seen.disconnect();
  });

// The still copy a drawer that is put away goes out as, held half way out in the moment it is put on the
// page; what is said of it then is kept as `window.left`. A check that is to read the page while a copy goes
// out over it asks for it held so, and plays it on afterwards.
export const watchLeaving = (page) =>
  page.evaluate(() => {
    window.left = null;
    const seen = new MutationObserver(() => {
      const copy = document.querySelector('.drawer-leaving');
      if (!copy || window.left) return;
      const [slide] = copy.getAnimations();
      const timing = slide?.effect.getComputedTiming();
      window.left = {
        name: slide?.animationName ?? 'none',
        took: timing?.duration,
        backwards: timing?.direction === 'reverse',
        inert: copy.inert,
        hidden: copy.getAttribute('aria-hidden'),
        pressed: getComputedStyle(copy).pointerEvents,
        // What it is a picture of is said by its box; what it holds is a closed tree of its own, which nothing on the page finds.
        heading: copy.dataset.of,
        found: copy.querySelectorAll('*').length + document.querySelectorAll('.drawer-leaving *').length,
      };
      slide?.pause();
      if (slide) slide.currentTime = timing.duration / 2;
      seen.disconnect();
    });
    seen.observe(document.querySelector('.main'), { childList: true });
  });

// ---- the Tools panel beside a channel, which comes and goes as the drawer does ----------------------------
// It takes room beside the channel: the room is taken at once and the panel slides into it, and closed, a
// still copy of it slides out over the room it gave back (`views/toolsslide.ts`). It says when it is at rest
// as the drawer does, and a check that measures the panel, or anything in it, or takes its picture, waits
// for that.

/** How the Tools panel is found once it is at rest. */
export const TOOLS_AT_REST = '.context[data-rest]';

/** The Tools panel of a page, once it is at rest. */
export const toolsAtRest = (page) => page.locator(TOOLS_AT_REST);

// The panel beside a document (`views/doctools.tsx`), of its comments, details and history, comes and goes the
// same way beside the document's text, and says when it is at rest the same way.

/** How the panel beside a document is found once it is at rest. */
export const DOC_TOOLS_AT_REST = '.doc-tools[data-rest]';

/** The panel beside a document, once it is at rest. */
export const docToolsAtRest = (page) => page.locator(DOC_TOOLS_AT_REST);

// What slides the Tools panel in, held part of the way (a sixth of its time) in the moment it is put on the
// page, with what is said of it then kept as `window.toolsSlid` and the channel's own boxes then kept as
// `window.toolsBehind`; `window.toolsUnwatched()` stops looking. Where nothing moves it, it says so.
export const watchToolsComing = (page) =>
  page.evaluate(() => {
    window.toolsSlid = null;
    const seen = new MutationObserver(() => {
      const panel = document.querySelector('.channel > .context');
      if (!panel || window.toolsSlid) return;
      const [slide] = panel.getAnimations();
      const rested = 'rest' in panel.dataset;
      const split = panel.parentElement.querySelector(':scope > .split').getBoundingClientRect();
      window.toolsBehind = [split.x, split.width].map(Math.round).join();
      if (!slide) return void (window.toolsSlid = { name: 'none', rested });
      const frames = slide.effect.getKeyframes();
      const moved = [...new Set(frames.flatMap((frame) => Object.keys(frame)))].filter((key) => !['offset', 'computedOffset', 'easing', 'composite'].includes(key)).sort();
      const took = slide.effect.getComputedTiming().duration;
      window.toolsSlid = { name: slide.animationName, took, eased: frames[0].easing, moved, rested };
      slide.pause();
      slide.currentTime = took / 6;
    });
    seen.observe(document.querySelector('.main'), { childList: true, subtree: true });
    window.toolsUnwatched = () => seen.disconnect();
  });

// The still copy the Tools panel goes out as, held half way out in the moment it is put on the page; what is
// said of it then is kept as `window.toolsLeft`.
export const watchToolsLeaving = (page) =>
  page.evaluate(() => {
    window.toolsLeft = null;
    const seen = new MutationObserver(() => {
      const copy = document.querySelector('.context-leaving');
      if (!copy || window.toolsLeft) return;
      const [slide] = copy.getAnimations();
      const timing = slide?.effect.getComputedTiming();
      window.toolsLeft = {
        name: slide?.animationName ?? 'none',
        took: timing?.duration,
        eased: slide?.effect.getKeyframes()[0]?.easing,
        backwards: timing?.direction === 'reverse',
        inert: copy.inert,
        hidden: copy.getAttribute('aria-hidden'),
        pressed: getComputedStyle(copy).pointerEvents,
        found: copy.querySelectorAll('*').length + document.querySelectorAll('.context-leaving *').length,
      };
      slide?.pause();
      if (slide) slide.currentTime = timing.duration / 2;
      seen.disconnect();
    });
    seen.observe(document.querySelector('.main'), { childList: true, subtree: true });
  });

// ---- the side panel, which comes and goes as the drawer does when it is folded away and brought back ------
// It takes a column of the page: the column is taken at once and the panel slides into it from behind the
// rail, and folded away, a still copy of it slides out over the room it gave back (`views/sideslide.ts`). It
// says when it is at rest as the drawer does, and a check that brings it back and then measures it, or
// anything in it, or takes its picture, waits for that.

/** How the side panel is found once it is at rest. */
export const SIDE_AT_REST = '.shell > .sidebar[data-rest]';

/** The side panel of a page, once it is at rest. */
export const sideAtRest = (page) => page.locator(SIDE_AT_REST);

/** Waits for the side panel to be at rest where it is there, and for what goes out as it is folded away to be gone. */
export const sideSettled = async (page) => {
  await page.locator('.side-leaving, .side-under-leaving').first().waitFor({ state: 'detached' });
  if ((await page.locator('.shell > .sidebar').count()) > 0) await sideAtRest(page).waitFor();
};

// What slides the side panel in (and on a phone the rail with it), held part of the way (a sixth of its time)
// in the moment it is put on the page, with what is said of it then kept as `window.sideSlid`;
// `window.sideUnwatched()` stops looking. Where nothing moves it, it says so.
export const watchSideComing = (page) =>
  page.evaluate(() => {
    window.sideSlid = null;
    const seen = new MutationObserver(() => {
      const panel = document.querySelector('.shell > .sidebar');
      if (!panel || window.sideSlid) return;
      const rail = document.querySelector('.shell > .rail');
      const [slide] = panel.getAnimations();
      const rested = 'rest' in panel.dataset;
      if (!slide) return void (window.sideSlid = { name: 'none', rested });
      const frames = slide.effect.getKeyframes();
      const moved = [...new Set(frames.flatMap((frame) => Object.keys(frame)))].filter((key) => !['offset', 'computedOffset', 'easing', 'composite'].includes(key)).sort();
      const took = slide.effect.getComputedTiming().duration;
      const railSlide = rail.getAnimations()[0];
      // On a phone the page beside them is dimmed as they come, held where they are.
      const dim = document.querySelector('.shell > .side-under');
      const dimming = dim?.getAnimations()[0];
      window.sideSlid = {
        name: slide.animationName, took, eased: frames[0].easing, moved, rested,
        rail: railSlide?.animationName ?? 'none',
        scrim: dim ? { name: dimming?.animationName ?? 'none', took: dimming?.effect.getComputedTiming().duration, eased: dimming?.effect.getKeyframes()[0]?.easing } : null,
      };
      for (const one of [slide, railSlide, dimming].filter(Boolean)) {
        one.pause();
        one.currentTime = took / 6;
      }
    });
    seen.observe(document.querySelector('.shell'), { childList: true, attributes: true, attributeFilter: ['class'] });
    window.sideUnwatched = () => seen.disconnect();
  });

// The still copy the side panel goes out as, held half way out in the moment it is put on the page; what is
// said of it then is kept as `window.sideLeft`.
export const watchSideLeaving = (page) =>
  page.evaluate(() => {
    window.sideLeft = null;
    const seen = new MutationObserver(() => {
      const copy = document.querySelector('.side-leaving');
      if (!copy || window.sideLeft) return;
      const [slide] = copy.getAnimations();
      const timing = slide?.effect.getComputedTiming();
      window.sideLeft = {
        name: slide?.animationName ?? 'none',
        took: timing?.duration,
        eased: slide?.effect.getKeyframes()[0]?.easing,
        backwards: timing?.direction === 'reverse',
        inert: copy.inert,
        hidden: copy.getAttribute('aria-hidden'),
        of: copy.dataset.of,
        pressed: getComputedStyle(copy).pointerEvents,
        found: copy.querySelectorAll('*').length + document.querySelectorAll('.side-leaving *').length,
      };
      // On a phone what stands in for the dimming of the page beside them fades out with them, held where they are.
      const dim = document.querySelector('.side-under-leaving');
      const fading = dim?.getAnimations()[0];
      window.sideLeft.scrim = dim
        ? { name: fading?.animationName ?? 'none', took: fading?.effect.getComputedTiming().duration, backwards: fading?.effect.getComputedTiming().direction === 'reverse', inert: dim.inert, pressed: getComputedStyle(dim).pointerEvents }
        : null;
      for (const one of [slide, fading].filter(Boolean)) {
        one.pause();
        one.currentTime = timing.duration / 2;
      }
      seen.disconnect();
    });
    seen.observe(document.querySelector('.shell'), { childList: true });
  });
