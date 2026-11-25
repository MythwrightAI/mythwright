// Topics as wikis, pages in a tree, as plain values: what the
// tree is read as and where a page lands when it is moved, the daemon's
// rules (`kernel::topics::tree`) held case for case to one table
// (`wikicases.json`). A wiki is a page with no page above it; a parent
// that names no page, and every page of a ring, are read as none.

/** A page as the tree is read from. */
export type Line = { id: string; parent: string | null; position: number; space: string | null };

/** The pages as one tree. */
export type Tree = { lines: Line[]; above: Map<string, string | null> };

/** Whether following what each page says is above it from `id` comes back to `id`. */
function inARing(said: Map<string, string | null>, id: string): boolean {
  const seen = new Set<string>();
  let at = said.get(id) ?? null;
  while (at !== null) {
    if (at === id) return true;
    if (seen.has(at)) return false;
    seen.add(at);
    at = said.get(at) ?? null;
  }
  return false;
}

/** The tree of `lines`. */
export function readTree(lines: Line[]): Tree {
  const said = new Map(lines.map((line) => [line.id, line.parent ? line.parent : null] as const));
  const above = new Map<string, string | null>();
  for (const line of lines) {
    const parent = said.get(line.id) ?? null;
    above.set(line.id, parent !== null && said.has(parent) && !inARing(said, line.id) ? parent : null);
  }
  return { lines, above };
}

const lineOf = (tree: Tree, id: string): Line | undefined => tree.lines.find((line) => line.id === id);
const aboveOf = (tree: Tree, id: string): string | null => tree.above.get(id) ?? null;

/** The pages from its wiki down to `id`, its wiki first. */
export function pathOf(tree: Tree, id: string): string[] {
  const path: string[] = [];
  let at: string | null = lineOf(tree, id) ? id : null;
  while (at !== null && !path.includes(at)) {
    path.push(at);
    at = aboveOf(tree, at);
  }
  return path.reverse();
}

/** The pages directly under `parent` (the wikis, for none), in the order they stand. */
export function besideOf(tree: Tree, parent: string | null): Line[] {
  return tree.lines
    .filter((line) => aboveOf(tree, line.id) === parent)
    .sort((one, other) => one.position - other.position || (one.id < other.id ? -1 : one.id > other.id ? 1 : 0));
}

/** `id` and every page under it, each before its own, in the order they stand. */
export function subtreeOf(tree: Tree, id: string): string[] {
  if (!lineOf(tree, id)) return [];
  const found: string[] = [];
  const gather = (page: string) => {
    found.push(page);
    for (const line of besideOf(tree, page)) gather(line.id);
  };
  gather(id);
  return found;
}

/** Where a page lands between two positions, or null where there is no room. */
export function between(before: number | null, after: number | null): number | null {
  const landed = before === null ? (after === null ? 1 : after - 1) : after === null ? before + 1 : before + (after - before) / 2;
  const fits = (before === null || landed > before) && (after === null || landed < after);
  return Number.isFinite(landed) && fits ? landed : null;
}

/** Where a page lands, as the daemon has it. */
export type Landing = { refused: 'gone' | 'itself' | 'elsewhere' | 'beside' } | { stays: true } | { positions: [string, number][] };

/** Where `page` lands under `under` (null for the top) before `before` (null for the end). */
export function landing(tree: Tree, page: string, under: string | null, before: string | null): Landing {
  for (const named of [page, under, before]) {
    if (named !== null && !tree.above.has(named)) return { refused: 'gone' };
  }
  if (under !== null) {
    if (pathOf(tree, under).includes(page)) return { refused: 'itself' };
    if ((lineOf(tree, under)?.space ?? null) !== (lineOf(tree, page)?.space ?? null)) return { refused: 'elsewhere' };
  }
  if (before === page) return aboveOf(tree, page) === under ? { stays: true } : { refused: 'beside' };
  if (before !== null && aboveOf(tree, before) !== under) return { refused: 'beside' };
  if (aboveOf(tree, page) === under) {
    const level = besideOf(tree, under);
    const at = level.findIndex((line) => line.id === page);
    if ((level[at + 1]?.id ?? null) === before) return { stays: true };
  }
  const level = besideOf(tree, under).filter((line) => line.id !== page);
  const found = before === null ? -1 : level.findIndex((line) => line.id === before);
  const at = found < 0 ? level.length : found;
  const position = between(at > 0 ? (level[at - 1]?.position ?? null) : null, level[at]?.position ?? null);
  if (position !== null) return { positions: [[page, position]] };
  const order = level.map((line) => line.id);
  order.splice(at, 0, page);
  const positions: [string, number][] = [[page, at + 1]];
  order.forEach((id, nth) => {
    if (id !== page && lineOf(tree, id)?.position !== nth + 1) positions.push([id, nth + 1]);
  });
  return { positions };
}

// ---- What the Topics panel draws ----------------------------------------------------

/** A page as the panel draws it: its line, and the pages under it in the order they stand. */
export interface Drawn<T> {
  line: T;
  pages: Drawn<T>[];
}

/** What a page is as far as the panel draws and moves it. */
export interface Paged {
  id: string;
  parent: string | null;
  position: number;
  status: string | null;
}

/**
 * The pages as the panel draws them: the tree they are read as (`readTree`,
 * a page's workspace being what `space` says of it), and each wiki in the
 * order it stands with the pages under it. A page carries its wiki's place,
 * so the wikis of a workspace are those `held` lists, and every page under
 * one is listed with it; a closed wiki stands apart (`closed`), with its
 * pages under it, and a closed page under an open one stays where it is.
 */
export function listedWikis<T extends Paged>(lines: readonly T[], space: (line: T) => string | null, held: (line: T) => boolean): { tree: Tree; listed: Drawn<T>[]; closed: Drawn<T>[] } {
  const byId = new Map(lines.map((line) => [line.id, line] as const));
  const tree = readTree(lines.map((line) => ({ id: line.id, parent: line.parent, position: line.position, space: space(line) })));
  const drawn = (id: string): Drawn<T> => ({ line: byId.get(id)!, pages: besideOf(tree, id).map((one) => drawn(one.id)) });
  const wikis = besideOf(tree, null)
    .map((one) => byId.get(one.id)!)
    .filter(held)
    .map((line) => drawn(line.id));
  return { tree, listed: wikis.filter((one) => one.line.status !== 'closed'), closed: wikis.filter((one) => one.line.status === 'closed') };
}

/** Every page of `wikis` and under them that holds pages, by id: what Expand all unfolds. */
export function holding<T extends { id: string }>(wikis: readonly Drawn<T>[]): string[] {
  return wikis.flatMap((one) => (one.pages.length > 0 ? [one.line.id, ...holding(one.pages)] : []));
}

/** Every page of `wikis` and under them, by id. */
export function everyPage<T extends { id: string }>(wikis: readonly Drawn<T>[]): string[] {
  return wikis.flatMap((one) => [one.line.id, ...everyPage(one.pages)]);
}

/** Where Move to… offers to put a page: the top, where it is a wiki of its own, or the end of a page, drawn at its depth. */
export type Move = { kind: 'top'; here: boolean } | { kind: 'page'; id: string; depth: number; here: boolean };

/**
 * Where Move to… offers to put `page`: the top, and every page of the
 * wikis of its workspace at its depth, in the order the panel draws them,
 * with the page itself and every page under it left out (it cannot go
 * there). Where it is now is marked (`here`). Nothing where it is not a
 * page there is.
 */
export function movesFor(tree: Tree, page: string): Move[] {
  const it = tree.lines.find((line) => line.id === page);
  if (!it) return [];
  const above = tree.above.get(page) ?? null;
  const moves: Move[] = [{ kind: 'top', here: above === null }];
  const walk = (id: string, depth: number) => {
    if (id === page) return;
    moves.push({ kind: 'page', id, depth, here: above === id });
    for (const one of besideOf(tree, id)) walk(one.id, depth + 1);
  };
  for (const wiki of besideOf(tree, null)) if (wiki.space === it.space) walk(wiki.id, 0);
  return moves;
}

/** What a page that is just made is called, until somebody calls it something else. */
export const UNTITLED = 'Untitled';

/** How a page is said with the pages under it: `Handbook`, or `Handbook and its 4 pages`. */
export function withPages(title: string, under: number): string {
  if (under <= 0) return title;
  return `${title} and its ${under === 1 ? 'page' : `${under} pages`}`;
}

/** Asked before a wiki is put in another workspace (`moveto.ts` asks the same of a project); `was` is the project it is of now, where it is one's. */
export function wikiQuestion(title: string, under: number, to: string, was: string | null = null): { title: string; says: string } {
  const many = under > 0;
  const left = was ? ` ${many ? 'They are' : 'It is'} ${was}'s no longer.` : '';
  return {
    title: `Move ${title} to ${to}?`,
    says: `${withPages(title, under)} ${many ? 'are' : 'is'} listed in ${to} from now on, for everybody.${left} Nothing else of ${many ? 'them' : 'it'} changes: what was said and answered stays as it is, and nobody is let in or kept out by where ${many ? 'they are' : 'it is'} listed.`,
  };
}
