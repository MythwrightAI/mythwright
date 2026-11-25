// What a new project is to start with, as whoever adds it chooses.
//
// The list is the daemon's (`project_starts` of `/api/meta`): what each
// thing is called, what it is, whether a project has it unless told
// otherwise, what it cannot be had without, and where a person is offered
// it: as the project is added, or on the page of something the project
// then has. Nothing here knows a board from a fleet; it only keeps the
// choosing straight.

/** One thing a new project may start with: the daemon's to say, and the client names none of them. */
export interface Start {
  /** What it is asked for by. */
  name: string;
  /** What it is called where a person chooses: its name, as a line of the dialog that adds a project names it. */
  title: string;
  /** How it is said in a sentence (`a board`, `docs`); a daemon from before says none, and its title is said in small letters. */
  said?: string;
  /** What it is, in a line. */
  says: string;
  /** The blueprint that is applied for it. */
  blueprint: string;
  /** Whether a project is given it unless whoever adds it says not. */
  on: boolean;
  /** What it cannot be given without, by name. */
  needs: string[];
  /** Why it cannot. */
  why: string;
  /** Where a person is offered it: as a project is added (`ADDING`), or the name of something it needs, on whose page it is offered. */
  offered?: string;
  /** The params of its blueprint that are told the project. */
  told?: string[];
  /** The table it gives the project, where it gives one. */
  table?: string;
  /** What its blueprint is handed beside what it is told, where its setup says. */
  with?: Record<string, unknown>;
}

/** Offered as a project is added, in the dialog that adds one. */
export const ADDING = 'adding';

/**
 * What of the list is offered at a place: as a project is added, or on
 * the page of something a project has, by that thing's name. One that
 * says nothing of it is offered as a project is added, as all of it was.
 */
export function offeredAt(starts: Start[], where: string): Start[] {
  return starts.filter((start) => (start.offered ?? ADDING) === where);
}

/**
 * What a project has of the list, by name: each whose blueprint is alive
 * there, of the blueprints `applied` in it. That is what the project's
 * list of blueprints goes by, and what the daemon goes by when it is
 * asked to give one.
 */
export function hasOf(starts: Start[], applied: string[]): string[] {
  return starts.filter((start) => applied.includes(start.blueprint)).map((start) => start.name);
}

/**
 * Why a project that `has` these cannot be given `start` as things are,
 * or nothing where it can. What is lacking is said as not running: a
 * project has a thing while its blueprint is alive there, and what one
 * that was stopped made (a board's table, and so its page) is still there.
 */
export function whyNotYet(starts: Start[], start: Start, has: string[]): string | null {
  const missing = lacks(start, has);
  if (missing.length === 0) return null;
  const called = missing.map((need) => saidOf(starts.find((each) => each.name === need)) ?? need);
  const reason = start.why ? `${start.why[0]!.toUpperCase()}${start.why.slice(1)}` : `${start.title} needs it`;
  return `${reason}: ${called.join(' and ')} ${called.length === 1 ? 'is' : 'are'} not running for this project.`;
}

/** How a thing is said in a sentence: what the daemon says, else its title in small letters. */
function saidOf(start: Start | undefined): string | undefined {
  return start ? (start.said ?? start.title.toLowerCase()) : undefined;
}

/** What a project starts with where nobody has turned anything off. */
export function usual(starts: Start[]): string[] {
  return starts.filter((start) => start.on).map((start) => start.name);
}

/** What `start` cannot be had without and has not got, of what was `chosen`. */
export function lacks(start: Start, chosen: string[]): string[] {
  return start.needs.filter((need) => !chosen.includes(need));
}

/**
 * What is chosen once `name` is turned on or off. Turning something off
 * turns off whatever cannot be had without it, and what cannot be had
 * without that. Nothing is turned on for somebody: what a thing needs is
 * theirs to turn on first.
 */
export function chosenAfter(starts: Start[], chosen: string[], name: string, on: boolean): string[] {
  let now = on ? [...chosen.filter((one) => one !== name), name] : chosen.filter((one) => one !== name);
  for (;;) {
    const kept = now.filter((one) => {
      const start = starts.find((each) => each.name === one);
      return !start || lacks(start, now).length === 0;
    });
    if (kept.length === now.length) break;
    now = kept;
  }
  // In the order they are listed, whatever order they were pressed in.
  return starts.map((start) => start.name).filter((one) => now.includes(one));
}

/**
 * What a project has of what somebody `wanted`: each thing they turned
 * on, less what cannot be had without something that is off. What they
 * wanted is kept apart from what they have, so that a thing turned off
 * only for the lack of another comes back when that other does, and a
 * thing they turned off themselves stays off.
 */
export function had(starts: Start[], wanted: string[]): string[] {
  let now = starts.map((start) => start.name).filter((one) => wanted.includes(one));
  for (;;) {
    const kept = now.filter((one) => {
      const start = starts.find((each) => each.name === one);
      return !start || lacks(start, now).length === 0;
    });
    if (kept.length === now.length) return kept;
    now = kept;
  }
}

/** What somebody wants once they turn `name` on or off themselves: that one thing changes, and nothing else of what they wanted. */
export function wantedAfter(starts: Start[], wanted: string[], name: string, on: boolean): string[] {
  const now = on ? [...wanted.filter((one) => one !== name), name] : wanted.filter((one) => one !== name);
  return starts.map((start) => start.name).filter((one) => now.includes(one));
}

/** Why `start` cannot be turned on as things are, or nothing where it can. */
export function whyNot(starts: Start[], start: Start, chosen: string[]): string | null {
  const missing = lacks(start, chosen);
  if (missing.length === 0) return null;
  const called = missing.map((need) => saidOf(starts.find((each) => each.name === need)) ?? need);
  const reason = start.why ? `${start.why[0]!.toUpperCase()}${start.why.slice(1)}` : `${start.title} needs it`;
  return `${reason}: turn on ${called.join(' and ')} to have one.`;
}
