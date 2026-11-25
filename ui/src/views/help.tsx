// Help, at the foot of the side panel: a round button beside the
// workspace, and the menu it opens.
//
// The menu holds the way back to the Setup Guide and the way to the
// Reference (everything a blueprint can say), a way to ask for
// support or report a problem, and what is new. The guide is the start
// page, a tab that is closed like any other: closed, nothing else in the
// client led back to it, and whoever pressed Help for help found none.
// Support is an address whoever built the product wrote down
// (`support` of what the daemon says of itself): pressed, it is opened in
// a new tab, and where none was written there is no such line. What
// somebody would say in a report (the program and its version, the
// system, the browser) is put on the clipboard for them to paste, and
// sent nowhere. What is new is the releases the daemon lists, each
// opening the notes at that release, and then all of them. The button
// carries a dot while the newest release is one this browser has not
// been shown the list at.
//
// The menu is the one the rows of the panel open (`RowMenu`), opened
// upward as the workspace's is.

import { useRef } from 'preact/hooks';
import { api } from '../api.ts';
import type { Meta } from '../api.ts';
import { supportLink, whatToSay } from '../help.ts';
import { news, releases, sawReleases, showRelease } from '../releases.ts';
import { meta, openTab, toast } from '../store.ts';
import { offer, offeredBy, putAway } from './grouped.tsx';
import type { Offered, Said } from './grouped.tsx';
import { signs } from './signs.tsx';

/** What a daemon says of itself beside what every one always said: where support is asked for, and where it runs from. One from before says neither. */
export type Told = Meta & { support?: string; runs_from?: string };

/** How many releases the menu lists before all of them are a press away. */
const LISTED = 5;

// The system of the device the daemon is on, asked once the menu is first opened: what is copied says it.
let system: Promise<{ os: string; arch: string } | null> | null = null;
export const readSystem = () => (system ??= api.fleet().then((fleet) => fleet.device.resources, () => null));

/** Puts text on the clipboard: the browser's own way, and where a page is not let use that (one served in the clear from another machine) the older one. */
async function copied(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    /* refused: the older way is tried */
  }
  const box = document.createElement('textarea');
  box.value = text;
  box.setAttribute('readonly', '');
  box.style.cssText = 'position:fixed;left:-9999px;top:0;opacity:0';
  document.body.append(box);
  box.select();
  let done = false;
  try {
    done = document.execCommand('copy');
  } catch {
    done = false;
  }
  box.remove();
  return done;
}

/** What somebody would say in a report, as the menu copies it and the page about the program shows it. */
export async function reportText(): Promise<string> {
  const told = meta.value as Told | null;
  const device = await readSystem();
  return whatToSay({ name: told?.name, version: told?.version, build: told?.client_build, runsFrom: told?.runs_from, os: device?.os, arch: device?.arch, browser: navigator.userAgent });
}

/** Puts what to say in a report on the clipboard, and says whether it could. */
export async function copyWhatToSay(): Promise<void> {
  const text = await reportText();
  if (await copied(text)) toast('Copied. Paste it into what you report.');
  else toast('It could not be copied here.', true);
}

/** What the menu offers: support where there is an address for it, the guide of first steps, what to say in a report, and what is new. */
function offered(): (Offered | Said)[] {
  const link = supportLink((meta.value as Told | null)?.support);
  const lines: (Offered | Said)[] = [];
  const guide: Offered = { label: 'Setup Guide', mark: signs.guide(), act: () => openTab({ kind: 'home' }) };
  // Everything a blueprint can say, beside the guide: where a word that was read somewhere is looked up.
  const reference: Offered = { label: 'Reference', mark: signs.sheets(), act: () => openTab({ kind: 'reference' }) };
  if (!link) lines.push(guide, reference);
  if (link) {
    // Opened by the browser as something pressed is, in a tab of its own: the page itself fetches nothing from there.
    const open = () => (link.toLowerCase().startsWith('mailto:') ? window.location.assign(link) : window.open(link, '_blank', 'noopener'));
    lines.push({ label: 'Get support or report a problem', mark: signs.out(), act: () => void open() });
    lines.push(guide, reference);
  }
  lines.push({ label: 'Copy what to say in a report', mark: signs.copy(), act: () => void copyWhatToSay() });
  const all = releases.value;
  if (all.length > 0) {
    lines.push({ said: <span class="menu-heading">What's new</span>, apart: true });
    all.slice(0, LISTED).forEach((one, at) => lines.push({ label: one.title, look: at === 0 ? 'joined-first' : 'joined', act: () => showRelease(one.at) }));
    lines.push({ label: 'View all releases', look: 'joined-last', act: () => showRelease() });
  }
  return lines;
}

/** The round button at the right of the panel's foot, with a dot on it while something is new. */
export function Help() {
  const button = useRef<HTMLButtonElement>(null);
  const open = offeredBy(button.current);
  const fresh = news.value;
  return (
    <button
      ref={button}
      class="foot-help"
      aria-haspopup="menu"
      aria-expanded={open}
      aria-label={fresh ? 'Help: there is something new' : 'Help'}
      title="Help: the Setup Guide, a way to ask for support or report a problem, and what is new in each release."
      onClick={(event) => {
        if (open) return putAway();
        const at = event.currentTarget.getBoundingClientRect();
        void readSystem();
        offer({ x: at.left, y: at.top - 4 }, offered(), { up: true, wide: 288, label: 'Help', from: event.currentTarget });
        // Shown the list, what is newest is new no longer.
        sawReleases();
      }}
    >
      {signs.question()}
      {fresh && <span class="foot-new" aria-hidden="true" />}
    </button>
  );
}
