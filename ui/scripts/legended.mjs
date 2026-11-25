// The band under a graph: what is said of the map, the buttons that move
// it and, under a preview, what plays it, which lie over no tile and no
// group's name.
//
// They were laid over the map, at its foot, and lay over whatever the
// graph came to be drawn under them: a group's name at a phone's width,
// the tiles of a graph fitted to a narrow place. They have a band of
// their own under the map now (`graph/view.tsx`), and a check that draws
// a graph holds that by the boxes: what is drawn of the map is read as
// far as the map shows it, since what the map cuts off at its edge is
// not seen.

/**
 * What of the band under each graph that is showing lies over a tile or
 * a group's name, is on the map or is outside the graph, each said in a
 * few words; nothing where all is well. `within` narrows it to the
 * graphs under what a selector finds (`.drawer`).
 */
export const legendCrossed = (page, within = '') =>
  page.evaluate((scope) => {
    const found = [];
    const box = (el) => el.getBoundingClientRect();
    const over = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0.5 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0.5;
    for (const canvas of document.querySelectorAll(`${scope} .canvas`.trim())) {
      if (canvas.getClientRects().length === 0) continue;
      const whole = box(canvas);
      const map = box(canvas.querySelector('.canvas-map'));
      const seen = (at) => ({ left: Math.max(at.left, map.left), right: Math.min(at.right, map.right), top: Math.max(at.top, map.top), bottom: Math.min(at.bottom, map.bottom) });
      const drawn = [...canvas.querySelectorAll('.gnode rect.box, .ggroup text')]
        .map((el) => ({ what: el.closest('.gnode')?.getAttribute('data-node') ?? el.textContent.trim(), ...seen(box(el)) }))
        .filter((one) => one.right - one.left > 0.5 && one.bottom - one.top > 0.5);
      const said = [...canvas.querySelectorAll('.canvas-legend span, .canvas-tools button, .canvas-player button, .canvas-player .player-track, .canvas-player .player-count')].map((el) => {
        const at = box(el);
        return { what: el.textContent.trim() || el.getAttribute('aria-label'), left: at.left, right: at.right, top: at.top, bottom: at.bottom };
      });
      for (const one of said) {
        for (const tile of drawn) if (over(one, tile)) found.push(`"${one.what}" over "${tile.what}"`);
        if (one.top < map.bottom - 0.5) found.push(`"${one.what}" on the map`);
        if (one.left < whole.left - 0.5 || one.right > whole.right + 0.5 || one.bottom > whole.bottom + 0.5) found.push(`"${one.what}" outside the graph`);
      }
      if (said.length === 0) found.push('no band under the graph');
    }
    return found;
  }, within);
