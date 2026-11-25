// A row of tabs with more in it than fits is moved along by the wheel, which
// on most mice only turns up and down, and brings the tab that is showing
// into sight when another is chosen.

/** Turns the wheel sideways over a row that has more than fits. Where the row fits, or the wheel is already going sideways, it is left alone. */
export function sideways(event: WheelEvent): void {
  const row = event.currentTarget as HTMLElement;
  if (row.scrollWidth <= row.clientWidth || Math.abs(event.deltaX) >= Math.abs(event.deltaY)) return;
  // A wheel that counts in lines, not in dots, turns a line's worth each.
  row.scrollLeft += event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
  event.preventDefault();
}

/**
 * Brings the tab that is showing into sight in its row, if the row has been moved off it or it was added past the
 * end. Measured by where each is drawn, and moved by whole pixels past that, so that a tab that begins between two
 * pixels is whole in sight and not short of its edge by part of one.
 */
export function reveal(row: HTMLElement | null): void {
  const chosen = row?.querySelector<HTMLElement>('[aria-selected="true"]');
  if (!row || !chosen) return;
  const [tab, frame] = [chosen.getBoundingClientRect(), row.getBoundingClientRect()];
  if (tab.left < frame.left) row.scrollLeft -= Math.ceil(frame.left - tab.left);
  else if (tab.right > frame.right) row.scrollLeft += Math.ceil(tab.right - frame.right);
}
