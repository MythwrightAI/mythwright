// What somebody may do, laid out by area.
//
// A credential holds scopes: an area (`board`, everything there), an area
// and one thing done in it (`board:read`), or `*`, which is everything.
// Written as a list that is short and says little to whoever has not
// read the rule: `board` beside `blueprints:read` does not look like
// "reads and changes the board, only reads blueprints". Here the list
// is turned into one row for each area with what is held there in
// words, and a row that was changed is turned back into scopes.
//
// The areas and what can be done in each are the daemon's (`scopes` of
// `/api/meta`); nothing here knows one area from another, and a scope
// the list does not know is kept and shown as it is written.

/** One area of what can be held, as the daemon lists it. */
export interface Area {
  area: string;
  /** What holding it allows, where the area is one thing with no parts. */
  does?: string | null;
  /** What may be done there, each with what it allows. Holding the area holds every one. */
  verbs: { verb: string; does: string }[];
}

/** What is held of one area. */
export interface Held {
  area: string;
  /** What may be done there, of what the area has, in the area's own order. For an area that is one thing: `['all']` or nothing. */
  verbs: string[];
  /** Whether that is everything the area has. */
  whole: boolean;
  /** What is held of it that is no part the area lists (`emit:shop.sold`): kept as written. */
  only: string[];
  /** What the daemon says of the area, for whoever points at it. */
  says: string;
}

/** A choice of a row: what would be held of its area, and how that reads. */
export interface Choice {
  verbs: string[];
  reads: string;
}

/** How one thing done in an area is said: the daemon's word, but for `write`, which a person calls changing. */
function said(verb: string): string {
  return verb === 'write' ? 'change' : verb;
}

/** What is held of an area, in words. */
export function reads(area: Area, verbs: string[], only: string[] = []): string {
  const parts = area.verbs.length === 0 ? (verbs.length > 0 ? ['yes'] : []) : area.verbs.filter((one) => verbs.includes(one.verb)).map((one) => said(one.verb));
  const narrow = only.map((word) => `only ${word}`);
  const all = [...parts, ...narrow];
  if (all.length === 0) return 'nothing';
  return all.length === 1 ? all[0]! : `${all.slice(0, -1).join(', ')} and ${all[all.length - 1]}`;
}

/**
 * The scopes as a table: a row for every area there is, in the daemon's
 * order, then a row for each scope whose area the list does not know.
 */
export function tableOf(scopes: string[], areas: Area[]): Held[] {
  const everything = scopes.includes('*');
  const rows: Held[] = areas.map((area) => {
    const parts = area.verbs.map((one) => one.verb);
    const bare = everything || scopes.includes(area.area);
    const narrow = scopes.filter((scope) => scope.startsWith(`${area.area}:`)).map((scope) => scope.slice(area.area.length + 1));
    const verbs = parts.length === 0 ? (bare ? ['all'] : []) : parts.filter((verb) => bare || narrow.includes(verb));
    return {
      area: area.area,
      verbs,
      whole: bare || (parts.length > 0 && verbs.length === parts.length),
      only: bare ? [] : narrow.filter((word) => !parts.includes(word)),
      says: area.does ?? area.verbs.map((one) => `${said(one.verb)}: ${one.does}`).join('; '),
    };
  });
  const known = new Set(areas.map((area) => area.area));
  for (const scope of scopes) {
    const area = scope.split(':')[0]!;
    if (scope === '*' || known.has(area)) continue;
    const row = rows.find((one) => one.area === area);
    const word = scope.includes(':') ? scope.slice(area.length + 1) : null;
    if (row) {
      if (word === null) row.whole = true;
      else row.only.push(word);
    } else {
      rows.push({ area, verbs: word === null ? ['all'] : [], whole: word === null, only: word === null ? [] : [word], says: 'Not something this factory lists; shown as it is written.' });
    }
  }
  return rows;
}

/**
 * What can be chosen for an area: nothing, then each further thing with
 * all before it (read; read and change), which is how a person thinks of
 * it. What is held some other way (change and not read) is offered too,
 * so that the row shows what is so.
 */
export function choices(area: Area, held: string[]): Choice[] {
  if (area.verbs.length === 0) {
    return [
      { verbs: [], reads: 'nothing' },
      { verbs: ['all'], reads: 'yes' },
    ];
  }
  const parts = area.verbs.map((one) => one.verb);
  const steps: string[][] = [[], ...parts.map((_, at) => parts.slice(0, at + 1))];
  if (!steps.some((step) => same(step, held))) steps.push(parts.filter((verb) => held.includes(verb)));
  return steps.map((verbs) => ({ verbs, reads: reads(area, verbs) }));
}

function same(one: string[], other: string[]): boolean {
  return one.length === other.length && one.every((verb) => other.includes(verb));
}

/**
 * The scopes once `area` is held as `verbs` say and everything else is as
 * it was: the whole list, written the short way (the area alone where
 * everything in it is held), in the daemon's order, with what it does not
 * know after. `*` is written out as every area, less what was taken.
 */
export function scopesAfter(scopes: string[], areas: Area[], area: string, verbs: string[]): string[] {
  const out: string[] = [];
  for (const row of tableOf(scopes, areas)) {
    const about = areas.find((one) => one.area === row.area);
    const changed = row.area === area;
    const now = changed ? verbs : row.verbs;
    const parts = about?.verbs.map((one) => one.verb) ?? [];
    const whole = about ? (parts.length === 0 ? now.length > 0 : parts.every((verb) => now.includes(verb))) : row.whole;
    if (whole) out.push(row.area);
    else out.push(...parts.filter((verb) => now.includes(verb)).map((verb) => `${row.area}:${verb}`));
    // What is held of it under a word of its own stays unless the whole area now covers it or it was taken to nothing.
    if (!whole && !(changed && now.length === 0)) out.push(...row.only.map((word) => `${row.area}:${word}`));
  }
  return out;
}

/** Whether somebody who holds `held` holds `scope`, by the daemon's own rule: the scope, its area, or everything. */
export function covers(held: string[], scope: string): boolean {
  const area = scope.split(':')[0]!;
  return held.includes('*') || held.includes(scope) || held.includes(area);
}

/**
 * Whether somebody who holds `mine` may change what an agent holds. One
 * of a fleet's rows is changed by its row, which takes changing agents
 * and changing records; one in no row is changed by itself, which takes
 * changing agents. Either way nobody gives what they do not hold
 * (`mayGive`), which is asked of each thing offered.
 */
export function mayChange(mine: string[], inRow: boolean): boolean {
  return covers(mine, 'agents:write') && (!inRow || covers(mine, 'records:write'));
}

/** The command that says what an agent holds, whole: what the page does when a line is changed, from a terminal. */
export function commandFor(cli: string, agent: string, scopes: string[]): string {
  return [`${cli} agent add ${agent}`, ...scopes.map((scope) => `--scope ${scope}`)].join(' ');
}

/** What is said under what an agent in no row of a fleet holds: how it is changed, and by whom. */
export function saidApart(may: boolean, cli: string, agent: string, scopes: string[]): string {
  const command = commandFor(cli, agent, scopes);
  return may
    ? `It is in no row of a fleet, so what it holds is its own: change a line here and it holds that from the next thing it asks of the factory. The same from a terminal, where what it holds is said whole: ${command}`
    : `It is in no row of a fleet, so what it holds is its own. Whoever may change agents changes a line here, or says the whole of it from a terminal: ${command}`;
}

/** Whether somebody who holds `mine` holds everything `verbs` of `area` would give: nobody gives what they do not hold. */
export function mayGive(mine: string[], area: Area, verbs: string[]): boolean {
  if (mine.includes('*') || mine.includes(area.area)) return true;
  if (area.verbs.length === 0) return verbs.length === 0;
  return verbs.every((verb) => mine.includes(`${area.area}:${verb}`));
}

/**
 * The row of a fleet's `agents` that an agent is one of: a fleet calls
 * each after its project and its row's title with its number after it
 * (`shop-worker-2`), so the row is the one whose title is between the
 * two. Nothing, for an agent that is in no row.
 */
export function rowOf<R extends { data: Record<string, unknown> }>(agent: string, project: string | null, rows: R[]): R | null {
  if (!project || !agent.startsWith(`${project}-`)) return null;
  const rest = agent.slice(project.length + 1);
  const cut = rest.lastIndexOf('-');
  if (cut <= 0 || !/^\d+$/.test(rest.slice(cut + 1))) return null;
  const title = rest.slice(0, cut);
  return rows.find((row) => row.data.title === title) ?? null;
}

/** What a row says its agents hold, or nothing where it leaves that to the usual. */
export function saidOf(row: { data: Record<string, unknown> }): string[] | null {
  const holds = row.data.holds;
  return Array.isArray(holds) && holds.length > 0 ? holds.map(String) : null;
}
