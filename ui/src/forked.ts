// A fork of a blueprint that ships, as the page offers it.
//
// The words are the daemon's: a blueprint's summary says where its text
// stands among the blueprints that ship (`blueprint::summary::forked`),
// and a line that leads somewhere says where (`opens`). What is worked
// out here is what the page does about it: where a pressed line leads
// from the page it is on, who is offered a fork, which projects one is
// offered for, and what the question asked before a fork is taken away
// says. The daemon holds each of them: a line is only not drawn where
// pressing it could do nothing. It holds nothing of the page, so that
// its tests can read it.

import { covers } from './holds.ts';
import type { Opens, Summary, SummaryLine, SummaryPart } from './summary.ts';
import type { Tab } from './tab.ts';

/** What the daemon answers when a fork is made, said of, or taken away. */
export interface Owned {
  blueprint: string;
  /** The project whose fork it is, where one was made. */
  project?: string;
  /** The file the fork is, within the project. */
  path?: string;
  /** The file that was taken away. */
  removed?: string;
  /** The version that was copied, where its text says one. */
  version?: string | null;
  /** Where the fork differs from what ships now, by version first: what ships is `was`, the fork `now`, so that what only the fork has is new, as its summary says it. */
  differs: { name: string; was: string | null; now: string | null; changed: string[]; same: boolean };
  /** What the fork's summary says of where it stands, where it was asked what it says: the parts `differs`, `moved` and `back`. */
  said?: SummaryPart[];
}

/** The page a line is pressed on. */
export interface Where {
  /** The project the page is of; none for one opened from the factory's list. */
  project: string | null;
  /** What whoever is signed in holds. */
  scopes: string[];
  /** The tables the project has, each with the kind of page it is shown as. */
  tables: Record<string, 'board' | 'library' | 'mockups' | 'table'>;
}

/** What pressing a line does: a tab is opened, or the page does one of three things. */
export type Follows =
  | { does: 'open'; tab: Tab }
  | { does: 'apply' }
  | { does: 'fork' }
  /** Asks whether the fork of the blueprint called `name` is to be taken away. */
  | { does: 'back'; name: string };

/** Whether whoever holds `scopes` is offered a fork, and going back from one: both write a file of a project's. */
export function mayFork(scopes: string[]): boolean {
  return covers(scopes, 'projects:write');
}

/**
 * Where a line leads from the page it is on; nothing where pressing it
 * could do nothing there, and the line is then words. The default opens
 * as what ships, for the project the page is of; a fork as the file it
 * is; a table as the page its kind has, once the project has it.
 */
export function follows(opens: Opens | null | undefined, where: Where): Follows | null {
  if (!opens) return null;
  const { project } = where;
  switch (opens.what) {
    case 'blueprint':
    case 'default':
      return opens.name ? { does: 'open', tab: project ? { kind: 'blueprint', name: opens.name, project } : { kind: 'blueprint', name: opens.name } } : null;
    case 'fork':
      return opens.name && project ? { does: 'open', tab: { kind: 'file', project, path: opens.name } } : null;
    case 'table': {
      const kind = opens.name && project ? where.tables[opens.name] : undefined;
      if (!kind || !opens.name || !project) return null;
      return { does: 'open', tab: kind === 'board' ? { kind, project } : { kind, project, name: opens.name } };
    }
    case 'apply':
      return { does: 'apply' };
    case 'forking':
      return mayFork(where.scopes) ? { does: 'fork' } : null;
    case 'back':
      return opens.name && project && mayFork(where.scopes) ? { does: 'back', name: opens.name } : null;
    default:
      return null;
  }
}

function everyLine(summary: Pick<Summary, 'parts'>): SummaryLine[] {
  const under = (lines: SummaryLine[]): SummaryLine[] => lines.flatMap((line) => [line, ...under(line.under)]);
  return summary.parts.flatMap((part) => under(part.lines));
}

/** The file that is the project's fork of the blueprint a summary is of, where its summary says the project has one. */
export function forkNamed(summary: Summary | null | undefined): string | null {
  if (!summary) return null;
  return everyLine(summary).find((line) => line.opens?.what === 'fork')?.opens?.name ?? null;
}

/** The blueprint that ships that a fork's summary says it was forked from, where its summary says it is a fork. */
export function defaultNamed(summary: Summary | null | undefined): string | null {
  if (!summary) return null;
  return everyLine(summary).find((line) => line.opens?.what === 'default')?.opens?.name ?? null;
}

/**
 * Whether a blueprint's page offers Fork in its head: for one that
 * ships, to whoever may write a project's files, while the project the
 * page is of has no fork of it and its summary offers one. A project
 * that keeps no blueprints of its own has a summary that offers none
 * (`blueprinted.ts`). One opened for no project asks which.
 */
export function offersFork(of: { ships: boolean; scopes: string[]; summary: Summary | null | undefined }): boolean {
  if (!of.ships || !mayFork(of.scopes) || !of.summary || forkNamed(of.summary) !== null) return false;
  return everyLine(of.summary).some((line) => line.opens?.what === 'forking');
}

/** What is said once a fork is made. */
export function forkedSaid(made: Owned, project: string): string {
  const version = made.version ? ` version ${made.version}` : '';
  return `Forked ${made.blueprint}${version} for ${project}: ${made.path ?? 'its file'} is what runs there under the name, and is the project's to change.`;
}

/** The question asked before a fork is taken away. */
export interface BackAsked {
  /** What is said, a paragraph each. */
  says: string[];
  /** Where its text would be lost, what is lost: where the fork differs from what it was forked from, as its summary says it; and where nothing says that, what moved between the fork and what ships now. */
  lost: string[];
  /** What the button that takes it away says. */
  confirm: string;
  /** Whether the daemon is told to take it away though its text says something else. */
  force: boolean;
}

/**
 * What the question says, from what the daemon said of the fork just
 * before it was asked: which file goes, what then runs, and whether
 * anything is lost by it. Only where the question said the fork's text
 * is lost is the daemon told to lose it; a fork changed between the
 * question and the answer is refused by the daemon, in its own words.
 */
export function backAsked(said: Owned, project: string): BackAsked {
  const file = said.path ?? 'its file';
  const runs = `${file} is taken away, and ${said.blueprint} as it ships is what runs in ${project} under the name again, once what is running there is applied.`;
  if (said.differs.same) {
    return { says: [runs, 'Nothing is lost by it: the fork says what ships now.'], lost: [], confirm: 'Go back to the default', force: false };
  }
  // What the project did to it is what goes: said as its summary says it, against what it was forked from.
  const differs = (said.said ?? []).filter((part) => part.kind === 'differs');
  const own = everyLine({ parts: differs }).map((line) => line.text);
  const [head, lost] = own.length > 0 ? [`${differs[0]!.title}:`, own] : ['Where it differs from the default as it ships now:', said.differs.changed];
  return {
    says: [runs, `The fork does not say what ships now, and what it says is then gone, unless the project keeps it somewhere else. ${head}`],
    lost,
    confirm: 'Remove the fork',
    force: true,
  };
}
