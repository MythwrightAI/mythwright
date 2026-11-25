// What a person is given beyond what their role holds (`people::given`):
// what can be offered to them, by whom, and how it is said.
//
// The scopes there are are the daemon's (`scopes` of `/api/meta`), and
// what each account holds as things stand is said with it (`holds` of
// `/api/accounts`): nothing here knows one scope from another. The daemon
// holds the rule (whoever gives holds it and `admin`; the owner holds
// everything); this offers only what it would let through.

import type { Account } from './api.ts';
import { type Area, covers } from './holds.ts';

/** An account as `/api/accounts` hands it, with what it was given and what it holds now. */
export type Listed = Account & { given?: string[]; holds?: string[] };

/** A scope that may be offered, with what holding it allows. */
export interface Offer {
  scope: string;
  does: string;
}

/** Every scope there is, as one is written to be held: an area that is one thing by itself, else each thing done in it. */
export function linesOf(areas: Area[]): Offer[] {
  return areas.flatMap((area) =>
    area.verbs.length === 0 ? [{ scope: area.area, does: area.does ?? '' }] : area.verbs.map((verb) => ({ scope: `${area.area}:${verb.verb}`, does: verb.does })),
  );
}

/**
 * What somebody who holds `mine` may give `account`: each scope there is
 * that they hold and the account does not, and nothing where they do not
 * manage the factory or the account is its owner's, who holds everything.
 */
export function offered(areas: Area[], mine: string[], account: Listed): Offer[] {
  if (account.role === 'owner' || !covers(mine, 'admin')) return [];
  const theirs = account.holds ?? [];
  return linesOf(areas).filter((line) => covers(mine, line.scope) && !covers(theirs, line.scope));
}

/** What an account holds, in words for its line: by its role, with `and` where what it was given follows. */
export function saidOf(account: Listed): string {
  if (account.role === 'owner') return 'Holds everything, as the owner';
  return (account.given ?? []).length === 0 ? `Holds what a ${account.role} holds` : `Holds what a ${account.role} holds, and`;
}
