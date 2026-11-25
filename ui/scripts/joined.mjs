// Whether a side panel's lists are one tree, by the boxes:
// every fold's line runs down from the middle of the mark
// that unfolded it, a part's heading as much as a project or a folder; a
// short line runs across from it to each thing under it, at that thing's
// row and up to its mark; the line runs on to the last of them; and every
// level is the same step further in than the one over it. What is drawn is
// the panel's one drawing of the lines (`.tree` in `folding.css`), read here
// from where the browser put each of them.
//
// `joined(page, panel)` answers what is not so, a sentence each, of what
// is drawn in the panel that `panel` finds; and `namesCut` the names that
// are cut to nothing, which a narrow panel must not do.

/** What is not joined in the panel `panel` (a selector) finds, a sentence each: nothing where it is one tree. */
export async function joined(page, panel) {
  return page.evaluate((panel) => {
    const nav = document.querySelector(panel);
    if (!nav) return [`no panel ${panel}`];
    const wrong = [];
    const nameOf = (one) => (one?.querySelector('.truncate')?.textContent ?? one?.textContent ?? '').trim().slice(0, 40);
    const drawn = (one) => {
      const box = one.getBoundingClientRect();
      return box.width > 0 && box.height > 0;
    };
    // A thing's own mark: the arrow or the glyph of its first row, or the place kept for an arrow where there is
    // nothing under it to fold (a member's line, a project with nothing listed), and never one of what is under it.
    const MARKS = '.twist, .glyph, .member-gap, span.nav-twist';
    const markOf = (one) => {
      if (!one) return null;
      const mark = one.matches(MARKS)
        ? one
        : [...one.querySelectorAll(MARKS)].find((each) => {
            const under = each.closest('.tree');
            return !under || !one.contains(under);
          });
      if (!mark || !drawn(mark)) return null;
      const box = mark.getBoundingClientRect();
      if (!mark.matches('.member-gap, span.nav-twist')) return { left: box.left, width: box.width, top: box.top, bottom: box.bottom };
      // The place kept for an arrow is where an arrow would stand: as far in as an arrow's button keeps before it, an arrow wide.
      const left = box.left + parseFloat(getComputedStyle(mark).paddingLeft);
      return { left, width: 14, top: box.top, bottom: box.bottom };
    };
    const middle = (box) => box.left + box.width / 2;
    const steps = [];
    for (const tree of nav.querySelectorAll('.tree')) {
      if (!drawn(tree) || tree.children.length === 0) continue;
      const box = tree.getBoundingClientRect();
      const before = getComputedStyle(tree, '::before');
      // What unfolded it is what stands just over it: a heading, a row, or a line with a menu after it.
      const over = tree.previousElementSibling;
      const mark = markOf(over);
      const lineX = box.left + parseFloat(before.left) + parseFloat(before.borderLeftWidth) / 2;
      const lineTop = box.top + parseFloat(before.top);
      const said = nameOf(over);
      if (!mark) {
        wrong.push(`what is under ${said || 'a fold'} hangs from no mark`);
        continue;
      }
      if (Math.abs(lineX - middle(mark)) > 1) wrong.push(`the line under ${said} runs down at ${lineX.toFixed(1)}, not from the middle of its mark at ${middle(mark).toFixed(1)}`);
      if (lineTop < mark.top - 0.5 || lineTop > mark.bottom + 0.5) wrong.push(`the line under ${said} begins at ${lineTop.toFixed(1)}, not at its mark (${mark.top.toFixed(1)} to ${mark.bottom.toFixed(1)})`);
      const children = [...tree.children].filter(drawn);
      let reached = box.top;
      children.forEach((child, nth) => {
        const at = child.getBoundingClientRect();
        const down = getComputedStyle(child, '::before');
        const across = getComputedStyle(child, '::after');
        const downX = at.left + parseFloat(down.left) + parseFloat(down.borderLeftWidth) / 2;
        if (down.content === 'none' || Math.abs(downX - lineX) > 0.5) wrong.push(`the line under ${said} does not run on beside ${nameOf(child)}`);
        const downTop = at.top + parseFloat(down.top);
        const downEnd = downTop + parseFloat(down.height);
        if (downTop > reached + 0.5) wrong.push(`the line under ${said} is broken over ${nameOf(child)}`);
        reached = Math.max(reached, downEnd);
        const y = at.top + parseFloat(across.top);
        const from = at.left + parseFloat(across.left);
        const to = from + parseFloat(across.width);
        if (across.content === 'none' || Math.abs(from - lineX) > 1) wrong.push(`no line across from the line under ${said} to ${nameOf(child)}`);
        if (nth === children.length - 1 && downEnd < y - 0.5) wrong.push(`the line under ${said} stops at ${downEnd.toFixed(1)}, over its last, ${nameOf(child)}, at ${y.toFixed(1)}`);
        const own = markOf(child);
        if (own) {
          if (y < own.top - 0.5 || y > own.bottom + 0.5) wrong.push(`the line across to ${nameOf(child)} meets it at ${y.toFixed(1)}, beside its mark (${own.top.toFixed(1)} to ${own.bottom.toFixed(1)})`);
          // It stops short of the mark by a pixel or two, where the mark's own ground begins (`.twist`); a mark narrower
          // than an arrow is met where an arrow's edge would be, its middle being where an arrow's is.
          const edge = middle(own) - 7;
          if (to > edge + 1 || edge - to > 3) wrong.push(`the line across to ${nameOf(child)} ends at ${to.toFixed(1)}, not at its mark at ${edge.toFixed(1)}`);
          // A line that opens something (its glyph) and a fold (its arrow) stand alike; who is in a channel is said, not opened, and is no step.
          if (!child.matches('.nav-who') && !child.querySelector(':scope > .nav-who')) steps.push({ step: middle(own) - middle(mark), of: `${said} to ${nameOf(child)}` });
        } else if (y < at.top || y > at.bottom) wrong.push(`the line across to ${nameOf(child)} meets nothing of it`);
      });
    }
    // The same step in at every level.
    if (steps.length > 0) {
      const least = Math.min(...steps.map((one) => one.step));
      for (const one of steps) if (one.step - least > 1) wrong.push(`the step from ${one.of} is ${one.step.toFixed(1)}, where the least is ${least.toFixed(1)}`);
    }
    return [...new Set(wrong)];
  }, panel);
}

/**
 * The names drawn in the panel `panel` finds that are cut to nothing: left
 * narrower than a letter and the ellipsis after it (a twelfth of an em
 * short of an em of the name's own letters), each with how wide it was left.
 */
export async function namesCut(page, panel) {
  return page.evaluate((panel) => {
    const nav = document.querySelector(panel);
    return [...(nav?.querySelectorAll('.nav .truncate') ?? [])]
      .filter((one) => one.getBoundingClientRect().height > 0 && one.textContent.trim() !== '')
      .filter((one) => one.getBoundingClientRect().width < Math.min(parseFloat(getComputedStyle(one).fontSize) * (11 / 12), one.scrollWidth))
      .map((one) => `${one.textContent.trim().slice(0, 30)} (${one.getBoundingClientRect().width.toFixed(1)} px)`);
  }, panel);
}
