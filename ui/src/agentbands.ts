// The agents of a factory in bands, by the project each belongs to.
//
// An agent is a project's or nobody's, and a list of every agent there
// is reads as one of those and then another: a band for each project
// that has any, in the order the projects are listed everywhere else,
// and a last one for those that belong to no project, which are the
// factory's own. This holds nothing of the page, so that its tests can
// read it.
//
// The Agents tab is of the workspace the side panel is of, as Activity
// and Measurements are (`narrowed.ts`): it is handed the projects of what
// is chosen there, and bands only theirs. The factory's own belong to no
// project of any workspace, so they are banded only where everything is
// shown (All workspaces), as what is nobody's is shown only there.

/** One band: a project's agents, or the factory's own (`project` is nothing). */
export interface Band<A> {
  project: string | null;
  /** What the band is called: the project's name, its id where the page has not read the project, or the word for the factory's own. */
  name: string;
  agents: A[];
}

/** What the band of agents that belong to no project is called: the word the side panel has for what is the factory's own. */
export const OWN = 'Factory';

const byName = new Intl.Collator('en', { numeric: true, sensitivity: 'base' });

/**
 * The bands, each with its agents: the projects that have any in the
 * order they were given, then a project the page has not read (an agent
 * says it and no list has it yet) under its id, and last the factory's
 * own. In a band those that are not stopped come first, and then each is
 * by its name, `worker-2` before `worker-10`. A band is there only where
 * it has an agent, and one band alone is a band all the same.
 *
 * `only` is the projects shown, or nothing where everything is: held to
 * some, an agent of any other project, and one of none, is left out.
 */
export function banded<A extends { name: string; project: string | null; active: boolean }>(agents: A[], projects: { id: string; name: string }[], only: readonly string[] | null = null): Band<A>[] {
  const of = new Map<string | null, A[]>();
  for (const agent of agents) {
    if (!shows(agent, only)) continue;
    const key = agent.project || null;
    of.set(key, [...(of.get(key) ?? []), agent]);
  }
  const ordered = (list: A[]) => [...list].sort((one, other) => Number(other.active) - Number(one.active) || byName.compare(one.name, other.name));
  const bands: Band<A>[] = [];
  for (const project of projects) {
    const list = of.get(project.id);
    if (list) bands.push({ project: project.id, name: project.name, agents: ordered(list) });
    of.delete(project.id);
  }
  const unread = [...of.keys()].filter((key): key is string => key !== null).sort(byName.compare);
  for (const id of unread) bands.push({ project: id, name: id, agents: ordered(of.get(id)!) });
  const own = of.get(null);
  if (own) bands.push({ project: null, name: OWN, agents: ordered(own) });
  return bands;
}

/** Whether an agent is of what is shown: everything, or one of the projects held to. */
const shows = (agent: { project: string | null }, only: readonly string[] | null) => only === null || (!!agent.project && only.includes(agent.project));

/** How many agents what is shown leaves out: nothing where everything is shown. */
export function leftOut(agents: { project: string | null }[], only: readonly string[] | null): number {
  return agents.filter((agent) => !shows(agent, only)).length;
}
