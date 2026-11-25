// What the Setup Guide says once its steps are done: the things worth
// doing next with a project that has its board and its fleet, each a line
// that does something when it is pressed.
//
// The guide used to end in six "done" and nothing else, and somebody who
// had followed it had a fleet running with no word of what a fleet is
// for. What is next is worked out here from what the page already read,
// so that its cases can be said without a page: which of the project's
// tables says what its agents run on, whether any of its agents runs on
// more than the built-in assistant, and whether the example is among its
// files. Nothing here knows a table by what it is called: the table is
// the one beside the board that has the cell.

/** What an agent's row calls the cell that says what its agents run on: what the daemon's own action calls it. */
export const CELL = 'adapter';

/** A table of a project, as far as this goes: what it is called, what it sits beside, and its fields by name. */
export interface Table {
  name: string;
  beside?: string | null;
  fields: { name: string }[];
}

/** An agent, as far as this goes. */
export interface Agent {
  project: string | null;
  adapter: string;
  active: boolean;
}

/** The table of a project's that says what its fleet's agents run on: one that sits beside another and has the cell. Nothing, where no fleet has made one. */
export function agentsTable(tables: Table[]): string | null {
  return tables.find((table) => Boolean(table.beside) && table.fields.some((field) => field.name === CELL))?.name ?? null;
}

/** Whether any agent of `project` that is up runs on something that can do work: anything but what the daemon says is the assistant that is built in. */
export function hasRealAgent(agents: Agent[], project: string, builtin: string[]): boolean {
  return agents.some((agent) => agent.active && agent.project === project && !builtin.includes(agent.adapter));
}

/** Where the example's blueprint is among a project's files, as the daemon writes it there: in the folder a new blueprint of its own is written in (`written`, the factory's place where the project said nothing). */
export function examplePath(example: string, blueprint: string, ext: string, written: string): string {
  return `${written}/${example}/${blueprint}.${ext}`;
}

/** One thing worth doing next. */
export interface Next {
  /** What it is known by, for whoever draws it and for a check. */
  key: 'card' | 'agent' | 'experiment' | 'device';
  title: string;
  detail: string;
  /** What its button says. */
  action: string;
  /** Whether it has been done, where that can be known from what was read. */
  done: boolean;
}

/** What is known of the project the guide is about. */
export interface Stands {
  cli: string;
  /** The project's id and what it is called. */
  project: { id: string; name: string };
  /** The table that says what its agents run on, if its fleet made one. */
  agents: string | null;
  /** Whether one of its agents runs on something that can do work. */
  real: boolean;
  /** Whether the example is among its files. */
  example: boolean;
}

/** The lines, in the order they are worth doing. */
export function whatNext(stands: Stands): Next[] {
  const { cli, project } = stands;
  const rows = stands.agents ?? 'agents';
  return [
    {
      key: 'card',
      title: 'Add a card to the board',
      detail: `Write up a piece of work on ${project.name}'s board and move it to Ready: the fleet takes it when its turn comes, and it waits in Review for you.`,
      action: 'Open the board',
      done: false,
    },
    {
      key: 'agent',
      title: 'Give the fleet a real agent',
      detail: stands.real
        ? `An agent of ${project.name} runs on more than the built-in assistant. What each row's agents run on is the row's ${CELL} cell.`
        : `Its agents run on the built-in assistant, which takes a card and cannot work it. Install an agent package under Extensions, then say it in the ${CELL} cell of the row of those who work (\`${cli} record list ${project.id} ${rows}\` lists the rows, \`${cli} record set <row> ${CELL}=<package>\` tells one), and move the card to Ready again.`,
      action: stands.agents ? 'Open its agents' : 'Open Extensions',
      done: stands.real,
    },
    {
      key: 'experiment',
      title: 'Try two ways side by side',
      detail: stands.example
        ? `The example research is among ${project.name}'s blueprints. \`${cli} help example\` says how its questions are asked two ways and what came of each is read.`
        : `The example research puts questions to an agent of the project, with what the agent is told in a file and other words beside it: each question goes one of the two ways, and what comes of them is counted side by side (\`${cli} example add research --project ${project.id}\`, then \`${cli} help example\`).`,
      action: stands.example ? 'Open it' : `Add it to ${project.name}`,
      done: stands.example,
    },
    {
      key: 'device',
      title: 'Link a second device',
      detail: `A step of a blueprint can run on another machine of yours. Settings lists this factory's devices and invites one (\`${cli} device invite <name>\`); the other machine is a factory of its own first, with its first account made.`,
      action: 'Open Settings',
      done: false,
    },
  ];
}
