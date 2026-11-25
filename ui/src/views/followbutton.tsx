// The Follow button of a page's head: on a page of a wiki, a board, a
// table, a library, a channel and a project. Pressed where the thing is
// not followed it follows it (a page of a wiki asks first: this page, or
// this page and the pages under it); pressed where it is, it opens how it
// is followed: a page with its pages or alone, how often (At once,
// Hourly, Daily, Off) and Unfollow. Everything followed is listed in
// Settings, under Following (`followingpanel.tsx`).

import { useRef } from 'preact/hooks';
import { OFTEN, follow, followOf, oftenSaid, useFollowing } from '../following.ts';
import type { Followable, Often } from '../following.ts';
import { attempt, me } from '../store.ts';
import { covers } from '../holds.ts';
import { offer, offeredBy, putAway } from './grouped.tsx';
import type { Offered, Said } from './grouped.tsx';
import { signs } from './signs.tsx';

/** How wide its menu is at the usual size of text. */
const WIDE = 260;

/** Whether whoever is signed in may follow anything: a person whose inbox it would tell. */
export function mayFollow(): boolean {
  const who = me.value?.principal;
  return Boolean(who && who.kind === 'user' && covers(who.scopes, 'inbox:write'));
}

export function FollowButton({ thing }: { thing: Followable }) {
  const all = useFollowing();
  const button = useRef<HTMLButtonElement>(null);
  if (!mayFollow()) return null;
  const now = all ? followOf(all, thing) : null;
  const page = thing.kind === 'page';
  const set = (how: { follows?: boolean; pages?: boolean; often?: Often }) => () => {
    putAway();
    void attempt(() => follow(thing, how));
  };
  const lines = (): (Offered | Said)[] => {
    if (!now) {
      return [
        { label: 'This page', under: 'Told when it changes', act: set({ pages: false }) },
        { label: 'This page and the pages under it', under: 'And when any page under it changes, or one is added', act: set({ pages: true }) },
      ];
    }
    const items: (Offered | Said)[] = [];
    if (page) {
      items.push(
        { label: 'This page', chosen: !now.pages, act: set({ pages: false }) },
        { label: 'This page and the pages under it', chosen: now.pages, act: set({ pages: true }) },
      );
    }
    items.push({ said: <span class="menu-note">How often you are told</span>, apart: page });
    for (const [often, said] of OFTEN) {
      items.push({ label: said, chosen: now.often === often, hint: often === 'daily' ? now.at : undefined, act: set({ often }) });
    }
    items.push({ label: 'Unfollow', mark: signs.cross(), look: 'grave', apart: true, act: set({ follows: false }) });
    return items;
  };
  const press = () => {
    const from = button.current;
    if (!from) return;
    if (offeredBy(from)) return putAway();
    // Anything else but a page is followed at once by a press, as most things are; how is in the menu after.
    if (!now && !page) return void attempt(() => follow(thing, {}));
    const at = from.getBoundingClientRect();
    const size = Number.parseFloat(getComputedStyle(from).getPropertyValue('--size'));
    const wide = Math.min(Math.round(WIDE * Math.max(1, Number.isFinite(size) ? size : 1)), window.innerWidth - 8);
    offer({ x: Math.max(4, at.right - wide), y: at.bottom + 4 }, lines(), { from, label: now ? 'Following' : 'Follow', wide, fit: true });
  };
  const said = now ? `Following, ${oftenSaid(now.often, now.at).toLowerCase()}${now.pages ? ', with its pages' : ''}` : 'Follow: be told in your inbox when this changes';
  return (
    <button
      ref={button}
      type="button"
      class="ghost head-follow"
      data-follows={now ? now.often : undefined}
      aria-pressed={Boolean(now)}
      aria-haspopup={now || page ? 'menu' : undefined}
      aria-expanded={offeredBy(button.current)}
      title={said}
      onClick={press}
    >
      {signs.bell(Boolean(now))}
      <span class="head-follow-says">{now ? 'Following' : 'Follow'}</span>
    </button>
  );
}
