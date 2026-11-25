// What may be set of somebody added to a channel, under who they are:
// one list of sections for each kind, worked out from the kind, what is
// being added and what whoever adds holds (`settingsOf`). The dialog that
// adds to a channel draws each section it is handed and decides nothing
// itself, so that a setting a kind comes to have is an entry here and no
// branch there. Today it is what each may ask of the factory, by area,
// drawn by the table the Tools panel draws (`HoldsTable`).
//
// The daemon holds each rule: nobody gives what they do not hold
// (`Agents::ensure`), an agent one of a fleet's rows counts is changed in
// its row, and a person is given a scope beyond their role by whoever
// holds it and `admin` (`people::given`). Here a section is changeable
// only where the daemon would let the change through.

import type { Listed } from './given.ts';
import { isExternal, offeredScopes } from './external.ts';
import { covers, mayChange } from './holds.ts';

/** Who is being added, as far as what may be set of them goes by it. */
export type Of =
  /** An agent not yet there: one this factory will make, or one invited from outside. */
  | { kind: 'agent'; coming: 'new' | 'invited'; usual: string[] }
  /** An agent that is there, brought in by its name. `held_by` says a fleet's row counts it. */
  | { kind: 'agent'; coming: 'there'; slot: { name: string; scopes: string[]; held_by?: string; adapter?: string } }
  /** A person with an account here; `account` is what the daemon says of it where whoever adds may read that, else nothing. */
  | { kind: 'person'; name: string; account: Listed | null }
  | { kind: 'bot' };

/** A section of what may be set: its heading, and what it draws. */
export type Section =
  /**
   * What it may ask of the factory, by area, beginning on `scopes`:
   * changeable where `may`, each change offering only what `giver` holds,
   * sent with the add where `sent`, and said under it.
   */
  | { section: 'holds'; title: string; scopes: string[]; may: boolean; sent: boolean; said: string }
  /** What a person was given beyond their role, with what may be given them: changed as it is pressed, by whoever may give. */
  | { section: 'given'; title: string; account: Listed };

const HOLDS = 'What it may ask of the factory';

/**
 * The sections drawn under who is being added, in the order they are
 * drawn. `mine` is what whoever adds holds.
 */
export function settingsOf(of: Of, mine: string[]): Section[] {
  switch (of.kind) {
    case 'agent': {
      if (of.coming !== 'there') {
        const usual = offeredScopes(of.usual, mine);
        const said =
          of.coming === 'new'
            ? 'It holds this once it is added. It begins on what an agent usually holds, as far as you hold that: nothing you do not hold is offered.'
            : 'It holds this once it joins. It begins on what an agent from outside usually holds, as far as you hold that: nothing you do not hold is offered.';
        return [{ section: 'holds', title: HOLDS, scopes: usual, may: true, sent: true, said }];
      }
      const slot = of.slot;
      if (slot.held_by) {
        return [{ section: 'holds', title: HOLDS, scopes: slot.scopes, may: false, sent: false, said: `${slot.name} is one of a fleet's agents: what it holds is changed in its row of the fleet's agents, and the fleet follows the row.` }];
      }
      // One this factory runs is changed by applying its text again as whoever asks (`byhand::give`), which takes applying.
      const may = mayChange(mine, false) && (isExternal({ adapter: slot.adapter ?? '' }) || covers(mine, 'blueprints:run'));
      return [
        {
          section: 'holds',
          title: HOLDS,
          scopes: slot.scopes,
          may,
          sent: may,
          said: may ? `What ${slot.name} holds now. Change a line and it holds that from when it is added.` : `What ${slot.name} holds now. Whoever may change agents changes it.`,
        },
      ];
    }
    case 'person': {
      if (!of.account) return [{ section: 'holds', title: 'What they may ask of the factory', scopes: [], may: false, sent: false, said: `What ${of.name} holds is what their role holds and what they were given beyond it, which whoever manages the factory reads and changes in Settings.` }];
      const sections: Section[] = [
        {
          section: 'holds',
          title: 'What they may ask of the factory',
          scopes: of.account.holds ?? [],
          may: false,
          sent: false,
          said: of.account.role === 'owner' ? `${of.account.name} owns the factory and holds everything.` : `What a ${of.account.role} holds, and what ${of.account.name} was given beyond it.`,
        },
      ];
      if (of.account.role !== 'owner' && covers(mine, 'admin')) sections.push({ section: 'given', title: 'Given beyond their role', account: of.account });
      return sections;
    }
    case 'bot':
      return [];
  }
}

/** The scopes a section begins on, by what it is of: so that what was changed is kept while the same thing is shown, and begun again for another. */
export function sectionKey(of: Of): string {
  switch (of.kind) {
    case 'agent':
      return of.coming === 'there' ? `agent:${of.slot.name}` : `agent:${of.coming}`;
    case 'person':
      return `person:${of.account?.id ?? of.name}`;
    default:
      return of.kind;
  }
}

/** Whether a list of scopes says what another does, in any order. */
export function sameScopes(one: string[], other: string[]): boolean {
  return one.length === other.length && one.every((scope) => other.includes(scope));
}
