// Whether the heads of a table keep their words whole: what `within` (a
// selector of heads) has broken inside a word, by the boxes. A head's
// words drawn on more than one line are broken; a head that is wider than
// its column keeps to one line and its table scrolls in its own box.

/** Each head under `within` whose words are drawn on more than one line, by its words; nothing where every one keeps to one. */
export const headsBroken = (page, within) =>
  page.evaluate((within) => {
    const broken = [];
    for (const head of document.querySelectorAll(within)) {
      const words = head.textContent.trim();
      if (!words) continue;
      const range = document.createRange();
      range.selectNodeContents(head);
      const lines = new Set([...range.getClientRects()].filter((box) => box.width > 0.5).map((box) => Math.round(box.top)));
      if (lines.size > 1) broken.push(`"${words}" is drawn on ${lines.size} lines`);
    }
    return broken;
  }, within);
