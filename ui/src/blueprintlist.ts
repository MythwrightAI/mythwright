// The factory's blueprints as the side panel lists them: in two levels.
//
// Most blueprints that ship are never applied by a person: they are
// parts, which something a person applies runs. So under each family
// (`std`, and each package that brings any) the list shows first what
// somebody applies, and under each of those, folded, its parts: every
// blueprint it runs, all the way down, with how many there are. A part
// that several run is under each. Nothing is renamed or moved for it: a
// blueprint is found by its name by everything that runs.
//
// Which is which is the daemon's to say and is written nowhere here: it
// hands, with each blueprint that is no project's, what that one names
// (`runs`), worked out from the texts (`blueprint::names`), and whether
// somebody applies it though something runs it (`applied`). What is
// here is functions of that answer, with nothing of the page in them:
// what is applied (`tops`), the parts of one (`parts`), what one is a
// part of (`partOf`), the tree the panel draws (`tree`) and the tree
// held to what somebody typed (`sought`). The first four are the
// daemon's own functions over the same values, and both are held to one
// table of cases (`blueprintparts.json`).
//
// A daemon from before hands no `runs`: nothing names anything, every
// blueprint is one somebody applies, and the list is flat as it was.
//
// What somebody applies though something that ships runs it too (the
// daemon says `applied` of it) is listed in a group of its own at the
// end of its family, called misc, rather than among the rest: for now,
// until where such blueprints belong is settled. It is still under
// whatever runs it as well.

/** A blueprint as the list is handed it. */
export interface Listed {
  name: string;
  description?: string | null;
  /** What it names, by name: the blueprints it runs itself. Nothing said by a daemon from before, and of a project's own. */
  runs?: string[];
  /** Somebody applies it, though something that ships runs it too. */
  applied?: boolean;
}

/** Whether anything but itself names it. */
function isNamed(name: string, all: Listed[]): boolean {
  return all.some((one) => one.name !== name && (one.runs ?? []).includes(name));
}

/** What somebody applies, by name: what nothing else names, and what is said to be applied all the same. */
export function tops(all: Listed[]): string[] {
  return all.filter((one) => one.applied || !isNamed(one.name, all)).map((one) => one.name).sort(byName);
}

/** Every part of one, by name: what it names and what those name, all the way down; never itself. */
export function parts(name: string, all: Listed[]): string[] {
  const by = new Map(all.map((one) => [one.name, one.runs ?? []]));
  const found = new Set<string>();
  const next = [name];
  for (let one = next.pop(); one !== undefined; one = next.pop()) {
    for (const part of by.get(one) ?? []) {
      if (part === name || !by.has(part) || found.has(part)) continue;
      found.add(part);
      next.push(part);
    }
  }
  return [...found].sort(byName);
}

/** What one is a part of: each blueprint somebody applies that it is reached from. */
export function partOf(name: string, all: Listed[]): string[] {
  return tops(all).filter((top) => parts(top, all).includes(name));
}

/** What is neither applied nor a part of anything applied: texts that name each other in a ring nothing else names. */
export function unreached(all: Listed[]): string[] {
  const reached = new Set(tops(all));
  for (const top of [...reached]) for (const part of parts(top, all)) reached.add(part);
  return all.map((one) => one.name).filter((name) => !reached.has(name)).sort(byName);
}

/** Names in the order the daemon puts them in: by their letters as they are written, whatever the reader's language. */
function byName(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** The family a blueprint is of: what its name says before its first `/`, and none for a name with no `/`. */
export function familyOf(name: string): string {
  return name.includes('/') ? name.slice(0, name.indexOf('/')) : '';
}

/** One that somebody applies, as a line of the list with what is under it. */
export interface Top {
  one: Listed;
  /** The parts that are listed under it: all of them, or those that match what was typed. */
  parts: Listed[];
  /** How many parts it has in all. */
  of: number;
}

export interface Family {
  /** What the family is called: `std`, or a package. */
  family: string;
  tops: Top[];
  /** What somebody applies though something that ships runs it too, listed apart (misc). */
  misc: Top[];
}

/** The name the group of what is applied though something runs it is listed under. */
export const MISC = 'misc';

/** Whether one is applied though something that ships runs it too, and so is listed in misc. */
function isMisc(one: Listed, all: Listed[]): boolean {
  return !!one.applied && isNamed(one.name, all);
}

/** The list in two levels: the families, the standard ones first, and what is of no family. */
export interface Tree {
  families: Family[];
  /** What somebody applies that is of no family, each with its parts. */
  loose: Top[];
}

/**
 * The list as it is drawn: each family with what somebody applies of it,
 * and under each of those its parts. What is in no list by what names
 * what (a ring) is listed with what is applied: nothing is hidden for
 * being nobody's part and nobody's to apply.
 */
export function tree(all: Listed[]): Tree {
  const by = new Map(all.map((one) => [one.name, one]));
  const top = (name: string): Top => {
    const under = parts(name, all).flatMap((part) => by.get(part) ?? []);
    return { one: by.get(name)!, parts: under, of: under.length };
  };
  const first = [...new Set([...tops(all), ...unreached(all)])].sort(byName).map(top);
  const families = [...new Set(first.map((line) => familyOf(line.one.name)).filter(Boolean))].sort(
    (a, b) => Number(b === 'std') - Number(a === 'std') || byName(a, b),
  );
  const of = (family: string, misc: boolean) => first.filter((line) => familyOf(line.one.name) === family && isMisc(line.one, all) === misc);
  return {
    families: families.map((family) => ({ family, tops: of(family, false), misc: of(family, true) })),
    loose: first.filter((line) => familyOf(line.one.name) === ''),
  };
}

/** What was typed, as the words that are looked for: none for nothing typed. */
export function wordsOf(typed: string): string[] {
  return typed.toLowerCase().split(/\s+/).filter(Boolean);
}

/** Whether every word looked for is in one or another of what something says: the one matcher of the side panel's boxes. */
export function says(words: string[], said: (string | null | undefined)[]): boolean {
  const all = said.filter(Boolean).join('\n').toLowerCase();
  return words.every((word) => all.includes(word));
}

/** Whether a blueprint is one that was looked for: each word is in its name or in what it says it is. */
export function matches(one: Listed, words: string[]): boolean {
  return says(words, [one.name, one.description]);
}

/**
 * The list held to what somebody typed: what is applied is listed where
 * it matches or a part of it does, with under it only the parts that
 * match; a part that several run is under each. A family with nothing
 * that matches is not listed. With nothing typed it is the whole list.
 */
export function sought(all: Listed[], typed: string): Tree {
  const whole = tree(all);
  const words = wordsOf(typed);
  if (words.length === 0) return whole;
  const held = (lines: Top[]): Top[] =>
    lines
      .map((line) => ({ ...line, parts: line.parts.filter((part) => matches(part, words)) }))
      .filter((line) => line.parts.length > 0 || matches(line.one, words));
  return {
    families: whole.families
      .map((family) => ({ ...family, tops: held(family.tops), misc: held(family.misc) }))
      .filter((family) => family.tops.length + family.misc.length > 0),
    loose: held(whole.loose),
  };
}

/** How many blueprints a tree lists, each line counted: a part under two is two lines. */
export function linesOf(listed: Tree): number {
  const count = (lines: Top[]) => lines.reduce((sum, line) => sum + 1 + line.parts.length, 0);
  return listed.families.reduce((sum, family) => sum + count(family.tops) + count(family.misc), count(listed.loose));
}

/** How many parts a line has, in words: nothing for one with none, and how many of them are listed where not all are. */
export function counted(line: Top): string {
  if (line.of === 0) return '';
  const all = `${line.of} ${line.of === 1 ? 'part' : 'parts'}`;
  return line.parts.length === line.of ? all : `${line.parts.length} of ${all}`;
}

/** What the fold of a blueprint's parts is kept as. */
export function foldOf(name: string): string {
  return `${PARTS}${name}`;
}
const PARTS = 'parts:';

/** Every fold of the factory's blueprints: each family, its misc where it has one, and the parts of each blueprint that has any. */
export function blueprintFolds(all: Listed[]): string[] {
  const listed = tree(all);
  return [
    ...listed.families.flatMap((family) => [`blueprints:${family.family}`, ...(family.misc.length > 0 ? [`blueprints:${family.family}:${MISC}`] : [])]),
    ...[...listed.families.flatMap((family) => [...family.tops, ...family.misc]), ...listed.loose].filter((line) => line.of > 0).map((line) => foldOf(line.one.name)),
  ];
}

/**
 * The folds a browser kept that fold nothing any more: the parts of a
 * blueprint that is not listed with parts now (its package went, or the
 * program came to run it another way). Taken out of what is kept, so
 * that nothing later called the same finds itself unfolded.
 */
export function foldsGone(kept: Iterable<string>, all: Listed[]): string[] {
  const listed = tree(all);
  const lines = [...listed.families.flatMap((family) => [...family.tops, ...family.misc]), ...listed.loose];
  const folding = new Set(lines.filter((line) => line.of > 0).map((line) => foldOf(line.one.name)));
  return [...kept].filter((fold) => fold.startsWith(PARTS) && !folding.has(fold));
}
