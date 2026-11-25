// What a project lists under Blueprints.
//
// Not every blueprint there is: those are listed once, for the factory. A
// project lists what it uses, which is the blueprints applied there that
// are still alive and the ones that are files of its own. A project that
// uses none has no such line at all.

/** What a project uses, as the daemon says it. */
export interface Uses {
  /** The blueprints with an instance alive in the project, at any depth. */
  applied: string[];
  /** The project's own, each with the file it is and the files of its own it links. */
  own: { name: string; file: string; links?: string[] }[];
  /** Where a new one of its own is written, as a path of the project's: nothing where it keeps none, or the daemon is from before and does not say. */
  written?: string | null;
}

/** One line of a project's list: a blueprint, and its file when it is the project's own, with the files it links under it. */
export interface Listed {
  name: string;
  file: string | null;
  links: string[];
}

/**
 * The lines of a project's list: its own first, since those are what is
 * written there, then what else is applied, each by name. One that is both
 * its own and applied is said once, as the file it is.
 */
export function listed(uses: Uses | undefined): Listed[] {
  if (!uses) return [];
  const byName = (a: Listed, b: Listed) => a.name.localeCompare(b.name);
  const own: Listed[] = uses.own.map((one) => ({ name: one.name, file: one.file, links: one.links ?? [] })).sort(byName);
  const mine = new Set(own.map((one) => one.name));
  const applied: Listed[] = [...new Set(uses.applied)]
    .filter((name) => !mine.has(name))
    .map((name) => ({ name, file: null, links: [] }))
    .sort(byName);
  return [...own, ...applied];
}

/** Whether an event may have changed what some project uses: an instance came or went. */
export function changesUse(topic: string): boolean {
  return ['instance.started', 'instance.active', 'instance.finished', 'instance.stopped', 'instance.failed'].includes(topic);
}
