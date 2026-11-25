// The factory's blueprints in the side panel, in two levels.
//
// Under each family (`std`, and each package that brings any) the list
// shows what somebody applies, and under each of those, folded, its
// parts: the blueprints it runs, all the way down, with how many. Which
// is which is the daemon's to say (`runs` and `applied` of each, with the
// list); the tree, the counts and what a filter matches are functions of
// that (`blueprintlist.ts`). A part is opened as any blueprint is, and
// nothing is renamed: a line says the name everything else knows it by.
//
// What is typed in the box at the head of the Resources panel holds it to
// what was typed, in a name or in what a blueprint says it is: each match
// is under what it is a part of, with that unfolded for as long as
// something is typed. The box is the panel's (`panelseek.tsx`), which
// keeps what was typed, and no fold is changed by it: emptied, the folds
// are as they were.

import { useEffect } from 'preact/hooks';
import { MISC, counted, foldOf, foldsGone, sought, wordsOf } from '../blueprintlist.ts';
import type { Listed, Top } from '../blueprintlist.ts';
import { callable, learnBlueprints, turn, turned } from '../store.ts';
import { Fold, NavItem } from './navtree.tsx';

/** One blueprint as a line: its name, which opens it, and what it says it is for whoever points at it. */
function Line({ one, hint }: { one: Listed; hint?: string }) {
  return <NavItem tab={{ kind: 'blueprint', name: one.name }} glyph="◆" label={one.name} title={one.description ?? undefined} hint={hint} />;
}

/** One that somebody applies: a line, and under it its parts, folded until they are asked for or looked for. */
function Applied({ line, held }: { line: Top; held: boolean }) {
  const name = line.one.name;
  // One with no part, and one none of whose parts is what was looked for, is a line and nothing to unfold.
  if (line.parts.length === 0) return <Line one={line.one} hint={held ? undefined : counted(line) || undefined} />;
  return (
    <Fold at={foldOf(name)} label={name} tab={{ kind: 'blueprint', name }} called={`the parts of ${name}`} held={held} hint={counted(line)} title={line.one.description ?? undefined}>
      {line.parts.map((part) => (
        <Line key={part.name} one={part} />
      ))}
    </Fold>
  );
}

/**
 * The list read when the panel is first shown, whether or not its part is
 * unfolded, so that Expand all has it to unfold; and a fold of parts that
 * folds nothing any more taken out of what is kept, once it was read.
 */
export function useBlueprintsRead(): void {
  const known = callable.value[''] as Listed[] | undefined;
  useEffect(() => {
    if (known === undefined) void learnBlueprints(null);
  }, []);
  useEffect(() => {
    if (known && known.length > 0) for (const fold of foldsGone(turned.value, known)) turn(fold);
  }, [known]);
}

/**
 * Every blueprint there is, in a part of the panel of its own. They come
 * in families: those that ship (`std`), and those each package brought,
 * under the package's name. Each family is folded away until it is wanted,
 * and one that is of no family is listed as it is. Read when the panel is
 * first shown (`useBlueprintsRead`); after that the list follows what
 * comes and goes.
 */
export function AllBlueprints({ typed }: { typed: string }) {
  const known = callable.value[''] as Listed[] | undefined;
  const held = wordsOf(typed).length > 0;
  const listed = sought(known ?? [], typed);
  const lines = (tops: Top[]) => tops.map((line) => <Applied key={line.one.name} line={line} held={held} />);
  return (
    <>
      {listed.families.map((family) => (
        <Fold key={family.family} at={`blueprints:${family.family}`} label={family.family} held={held}>
          {lines(family.tops)}
          {/* What is applied though something runs it too, apart for now: folded until it is asked for or looked for. */}
          {family.misc.length > 0 && (
            <Fold at={`blueprints:${family.family}:${MISC}`} label={MISC} held={held}>
              {lines(family.misc)}
            </Fold>
          )}
        </Fold>
      ))}
      {lines(listed.loose)}
      {known === undefined && <span class="nav-note">Reading what there is…</span>}
    </>
  );
}
