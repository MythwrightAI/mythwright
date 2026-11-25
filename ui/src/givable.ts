// Who a card can be given to, where it is given from its drawer.
//
// A card is worked in its project, by agents that are there: an agent
// of a project of another workspace has nothing to do with it, and
// listing every agent of the factory under every card offered them all
// the same (a card of one workspace was offered the agents of four
// others). So what is offered is the agents of the card's own project
// first, then those of the other projects of its workspace, then the
// agents that are no project's, which are the factory's own and work
// wherever they are asked. Whoever has the card already is offered
// whatever it is of, since a form shows what is so.
//
// The daemon holds the same (`agents::assigned`): a card given to an
// agent of another workspace's project is refused by whichever way it
// is given, so this is what is offered and that is what is held. Until
// the workspaces are read the page cannot say, offers every agent, and
// the daemon answers for it.

import { spaceOf, type Workspace } from './workspaces.ts';

/** An agent as the list of agents hands one out, as far as this reads it. */
export interface Given {
  name: string;
  project: string | null;
}

/**
 * The agents a card of `project` is offered, in the order they are
 * listed. Until the workspaces are read nothing says which projects
 * belong together, and every agent is offered, as before.
 */
export function givable<A extends Given>(agents: A[], project: string, all: Workspace[], has: string | null = null): A[] {
  const space = spaceOf(project, all);
  if (!space) return agents;
  const beside = new Set(space.projects);
  // A project no workspace names is listed in the usual one (`spaceOf`), and so are its agents.
  const within = (one: string) => one === project || beside.has(one) || spaceOf(one, all)?.id === space.id;
  const rank = (agent: A) => (agent.project === project ? 0 : agent.project === null ? 2 : 1);
  return agents
    .filter((agent) => agent.project === null || within(agent.project) || `agent:${agent.name}` === has)
    .map((agent, at) => ({ agent, at }))
    .sort((a, b) => rank(a.agent) - rank(b.agent) || a.at - b.at)
    .map(({ agent }) => agent);
}
