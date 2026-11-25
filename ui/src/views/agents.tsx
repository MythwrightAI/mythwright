// The agents: the Agents tab of the Members page (`members.tsx`), which
// was a page of its own under Factory beside Activity and Measurements.
//
// It is of the workspace the side panel is of, and shows what is chosen
// within it by the one choice those pages share (`Narrow`, `narrowed.ts`),
// drawn in the page's head while the tab shows: all of the workspace,
// which is its projects' agents; one project of it; or All workspaces,
// which is every agent, the factory's own (those of no project) with them.
// The agents are read as they always were, whole (`agents` in the store),
// and held here to the projects of the choice: nothing in the daemon
// learns which workspace anybody looks at. Until it is known which
// workspace that is, nothing is drawn, so that the tab does not show
// everything on its way to one.
//
// The list itself (`agentlist.tsx`) is what Settings had: the bands, the
// lines and their menus, adding and inviting an agent, its history in the drawer,
// its permissions, and removing one.

import { only } from '../narrowing.ts';
import { shown } from '../narrowed.ts';
import { agents, projects as read } from '../store.ts';
import { workspaces } from '../spaces.ts';
import { leftOut } from '../agentbands.ts';
import { AgentList } from './agentlist.tsx';
import { Narrow } from './narrow.tsx';

/** The choice of whose agents are shown, for the page's head. */
export function AgentsNarrow() {
  const of = shown.value;
  // Whether every workspace would show an agent this workspace's projects do not: it is a line of the choice only where it would.
  return <Narrow more={of.kind === 'projects' && leftOut(agents.value, only(of)) > 0} />;
}

/** The agents of what is chosen, under the page's head. */
export function AgentsTab() {
  const of = shown.value;
  const projects = only(of);
  // An empty list is said to be of what is chosen, so that it is not taken for a factory with no agents.
  const none = of.kind !== 'projects'
    ? null
    : of.project
      ? 'No agent is of this project. Those of the rest of the workspace are under its name above, and every agent is under All workspaces.'
      : 'No agent is of a project of this workspace. Those of other workspaces, and those of no project, are under All workspaces.';
  // The list's own head says whose agents it lists, under the page's: the page is Members whatever is chosen.
  const called =
    of.kind !== 'projects'
      ? 'All workspaces'
      : of.project
        ? (read.value.find((one) => one.id === of.project)?.name ?? of.project)
        : (workspaces.value.find((one) => one.id === of.space)?.name ?? of.space);
  return (
    <div class="view-body stack agents-page" style="gap:20px">
      {of.kind !== 'waiting' && <AgentList only={projects} none={none} called={called} />}
    </div>
  );
}
