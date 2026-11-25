// The line of a menu that opens what its row opens in a tab of the
// browser's own, as "Open link in new tab" does for a link: in a browser
// alone, where one thing shows at a time and the browser's tabs are the
// tabs (`inplace.ts`). The desktop window has its own tabs, and no line.

import { openApart } from '../address.ts';
import { inPlace } from '../inplace.ts';
import type { Tab } from '../tab.ts';
import type { Offered } from './rowmenu.tsx';
import { signs } from './signs.tsx';

/** The line that opens `tab` in a new tab of the browser's, as a list of one or of none. */
export function newTabLines(tab: Tab): Offered[] {
  return inPlace ? [{ label: 'Open in a new tab', mark: signs.out(), act: () => openApart(tab) }] : [];
}

/** `lines` with the line that opens `tab` in a new tab put before what takes something away, which stays the last. */
export function withNewTab(tab: Tab, lines: Offered[]): Offered[] {
  const at = lines.findIndex((line) => line.look === 'grave');
  return at < 0 ? [...lines, ...newTabLines(tab)] : [...lines.slice(0, at), ...newTabLines(tab), ...lines.slice(at)];
}
