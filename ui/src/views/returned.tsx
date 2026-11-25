// What a view keeps by its tab, so that it is drawn again as it was left (`returned.ts`).
//
// A view says what it keeps by naming it: what it read (`useKept`, in the
// place of `useState`, shown at once when the view is made again and read
// again behind), and what it only holds between drawings (`useKeptRef`).
// Where each of its parts was scrolled is kept for it by the frame, which
// draws every view and the drawer inside `Returned`: nothing a view need
// say. Outside a tab (a page drawn elsewhere, a test) nothing is kept, and
// each is the hook it stands in for.

import { createContext } from 'preact';
import type { ComponentChildren } from 'preact';
import { useContext, useLayoutEffect, useState } from 'preact/hooks';
import { forgetScope, FRAMES, partScope, putBack, recall, remember, SCROLLS_ITSELF, signOf } from '../returned.ts';
import type { Scrolled } from '../returned.ts';

/** The scope what is drawn here keeps things in: its tab's, or its tab's drawer's. */
const Scope = createContext<string | null>(null);

/** The scope of what is being drawn, where it is drawn for a tab. */
export const useScope = (): string | null => useContext(Scope);

/**
 * A part of a tab's view that keeps what it holds apart from the rest, under `name`: what is typed in a card's
 * form in the Tools panel, which is kept while the tab is away and let go of when the form is done with
 * (`forgetPart`), as the drawer's form is with the drawer.
 */
export function Part({ name, children }: { name: string; children: ComponentChildren }) {
  const scope = useContext(Scope);
  return <Scope.Provider value={scope === null ? null : partScope(scope, name)}>{children}</Scope.Provider>;
}

/** Lets go of what the part `name` of the view drawn in `scope` kept. */
export function forgetPart(scope: string | null, name: string): void {
  if (scope !== null) forgetScope(partScope(scope, name));
}

/** Like `useState`, kept under `name` by the tab: made again, the view begins with what it last had. */
export function useKept<T>(name: string, first: T | (() => T)): [T, (next: T | ((was: T) => T)) => void] {
  const scope = useContext(Scope);
  const [value, set] = useState<T>(() => {
    const kept = recall<{ is: T }>(scope, name);
    return kept ? kept.is : typeof first === 'function' ? (first as () => T)() : first;
  });
  remember(scope, name, { is: value });
  return [value, set];
}

/** Like `useRef`, and the same one each time its tab's view is made again. */
export function useKeptRef<T>(name: string, first: T): { current: T } {
  const scope = useContext(Scope);
  const [made] = useState(() => recall<{ current: T }>(scope, name) ?? { current: first });
  remember(scope, name, made);
  return made;
}

/** Each element of what is found, in order: what a part is found again by. */
const within = (root: Element): Element[] => [root, ...root.querySelectorAll('*')];
const sign = (one: Element): string => signOf(one.tagName, one.classList);

/** Where a part that was scrolled is, among what is found now. */
function partAt(root: Element, kept: Scrolled): Element | null {
  if (kept.nth < 0) return root;
  return within(root).filter((one) => sign(one) === kept.sign)[kept.nth] ?? null;
}

/**
 * Puts back where the parts of what is found were scrolled, as each comes
 * to be long enough, and holds them there, a frame at a time for a while:
 * what a view draws once it has read, or once a part of it was fetched,
 * comes a few frames after it is made. A part the view scrolled itself is
 * left where it is, and the reader scrolling, pressing or typing ends it,
 * as does a view saying it brings something into view itself (`scrollsItself`).
 * Says how to end it.
 */
function putScrollsBack(find: () => Element | null, kept: Scrolled[]): () => void {
  if (kept.length === 0 || typeof window === 'undefined') return () => {};
  let left = [...kept];
  // Those put back already, held there until it is laid out.
  const held = new Set<Scrolled>();
  let frames = 0;
  let frame = 0;
  const done = () => {
    cancelAnimationFrame(frame);
    for (const what of ['wheel', 'pointerdown', 'keydown', 'touchstart', SCROLLS_ITSELF]) window.removeEventListener(what, done, true);
  };
  const look = () => {
    const root = find();
    const last = (frames += 1) >= FRAMES;
    left = root
      ? left.filter((one) => {
          const part = partAt(root, one);
          const now = part && { top: part.scrollTop, left: part.scrollLeft, most: part.scrollHeight - part.clientHeight, mostLeft: part.scrollWidth - part.clientWidth };
          const what = putBack(one, now, held.has(one));
          if (what === 'leave') return false;
          if (what === 'there' || (what === 'wait' && !(last && part))) return true;
          part!.scrollTo(what === 'wait' ? one.left : what.left, what === 'wait' ? one.top : what.top);
          held.add(one);
          return true;
        })
      : left;
    if (left.length === 0 || last) done();
    else frame = requestAnimationFrame(look);
  };
  for (const what of ['wheel', 'pointerdown', 'keydown', 'touchstart', SCROLLS_ITSELF]) window.addEventListener(what, done, true);
  look();
  return done;
}

/**
 * What is drawn for a tab (`scope`), in what `find` finds on the page
 * (the tab's view, or the drawer's body): it keeps things in the scope, and
 * where its parts were scrolled is kept as they are scrolled and put back
 * when it is made again.
 */
export function Returned({ scope, find, children }: { scope: string; find: () => Element | null; children: ComponentChildren }) {
  useLayoutEffect(() => {
    const root = find();
    if (!root) return;
    const stop = putScrollsBack(find, recall<Scrolled[]>(scope, 'scrolled') ?? []);
    // As each part is scrolled, by the reader or by the view itself: read then, since by the time the view is
    // taken off the page what takes its place may be there beside it.
    const scrolled = new Map<Element, Scrolled>(
      (recall<Scrolled[]>(scope, 'scrolled') ?? []).flatMap((one) => {
        const part = partAt(root, one);
        return part ? [[part, one] as const] : [];
      }),
    );
    const heard = (event: Event) => {
      const part = event.target;
      // What keeps where it was scrolled to itself (the code editor, which alone knows how tall its lines are) is its own.
      if (!(part instanceof Element) || !root.contains(part) || part.closest('[data-scrolls-itself]')) return;
      const was = scrolled.get(part);
      const nth = part === root ? -1 : (was?.nth ?? within(root).filter((one) => sign(one) === sign(part)).indexOf(part));
      scrolled.set(part, { sign: part === root ? '' : sign(part), nth, top: Math.round(part.scrollTop), left: Math.round(part.scrollLeft) });
      for (const [one] of scrolled) if (!root.contains(one)) scrolled.delete(one);
      remember(scope, 'scrolled', [...scrolled.values()].filter((one) => one.top !== 0 || one.left !== 0));
    };
    root.addEventListener('scroll', heard, true);
    return () => {
      stop();
      root.removeEventListener('scroll', heard, true);
    };
  }, [scope]);
  return <Scope.Provider value={scope}>{children}</Scope.Provider>;
}
