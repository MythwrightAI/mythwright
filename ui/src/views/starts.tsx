// What a project is given from the page of something it has.
//
// What a project may start with is the daemon's list (`starts.ts`), and
// each thing on it says where a person is offered it: most as the project
// is added, and some on the page of something the project then has, which
// is where the lack of it is felt. Such a page hands its gear these lines
// (`pagehead.tsx`), one switch for each thing offered there, and names
// none of them: what a thing is called, what it is and what it needs are
// the list's to say.
//
// Whether a project has a thing is read from what the store already
// follows: the blueprints alive in the project (`used`), which is what
// the daemon goes by when it is asked to give one. Turned on, the daemon
// is asked (`POST /api/projects/<id>/starts`), which applies the thing's
// blueprint as whoever asked, exactly as adding a project with it does;
// what it then keeps for the project (a table) comes to the side panel by
// what is said of it, with nothing read again here.
//
// There is no turning off here. What a thing made stays when its
// blueprint is stopped (a table and its records), and only what looks
// after that ends: nothing is taken away by it, and stopping is done where
// everything running is stopped, under Activity. So a thing that is on is
// drawn as on and closed, saying so. Stopped there, it is off here again,
// and turning it on gives it back over what it kept. And one that is being
// given is asked for once: pressed again before the answer is in, nothing
// more is asked, and a menu opened meanwhile has it closed, saying so.
//
// The lines are for whoever may apply a blueprint, which is what the
// daemon asks: somebody who may not is offered nothing while a thing is
// off, and is told that it is on where it is.

import { call } from '../api.ts';
import { covers } from '../holds.ts';
import { hasOf, offeredAt, whyNotYet } from '../starts.ts';
import type { Start } from '../starts.ts';
import { attempt, me, meta, nowUses, toast, used } from '../store.ts';
import { glyph, note } from './pagehead.tsx';
import type { Lines } from './pagehead.tsx';

/** What the daemon answers when a project is given things: each by its name with the instance that is it, those that were there already, and a sentence for each it was not given. */
type Given = Record<string, string | string[] | undefined> & { there?: string[]; left?: string[] };

/** What is being given just now, by project and by name: asked for once, however often its switch is pressed before the answer is in. */
const giving = new Set<string>();
const GIVING = 'It is being given: its blueprint is being applied.';

/** What is said of something that is on, where a line can only say it. */
const ON = 'On for this project. Stopping its blueprint, under Activity, ends what it does and keeps what it made.';

/** The lines a page of `where` hands its gear for `project`: a switch for each thing a project may be given there. */
export function startLines(project: string, where: string): Lines {
  const all = meta.value?.project_starts ?? [];
  const may = covers(me.value?.principal.scopes ?? [], 'blueprints:run');
  // As things stand whenever it is asked: a switch says how it is once it has been pressed.
  const has = () => hasOf(all, used.value[project]?.applied ?? []);
  const lines: Lines = [];
  for (const start of offeredAt(all, where)) {
    const on = () => has().includes(start.name);
    const apart = lines.length === 0;
    if (on()) {
      // Whoever could not have turned it on is only told; whoever could sees the switch, which does not turn off here.
      if (may) lines.push({ label: start.title, mark: glyph('◈'), apart, on, closed: 'It is on, and is not turned off here: its blueprint is stopped under Activity.', act: () => {} }, note(ON));
      else lines.push(note(`${start.title}: on for this project.`, apart));
      continue;
    }
    if (!may) continue;
    const lacking = whyNotYet(all, start, has());
    // One that is being given is not pressed again: in a menu opened meanwhile it is closed and says so.
    const closed = lacking ?? (giving.has(`${project}:${start.name}`) ? GIVING : undefined);
    lines.push({ label: start.title, mark: glyph('◈'), apart, on, closed, act: () => give(project, start, on) }, note(lacking ?? start.says));
  }
  return lines;
}

/** Asks the daemon to give `project` what `start` is, and says what it was not given. */
async function give(project: string, start: Start, on: () => boolean): Promise<void> {
  const key = `${project}:${start.name}`;
  // Pressed again before the first press was answered: the daemon was asked, and is not asked twice.
  if (giving.has(key)) return;
  // Pressed again in a menu that was opened while it was off: it is on, and says what a menu opened now would.
  if (on()) return toast(ON);
  giving.add(key);
  try {
    const given = await attempt(() => call<Given>('POST', `/projects/${encodeURIComponent(project)}/starts`, { names: [start.name] }));
    if (!given) return;
    for (const left of given.left ?? []) toast(left, true);
    if (typeof given[start.name] === 'string') nowUses(project, start.blueprint);
  } finally {
    giving.delete(key);
  }
}
