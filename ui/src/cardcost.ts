// What one card cost, by round, as the daemon lays it out
// (`GET /api/board/items/{id}/cost`, `deciding::spent`): each time the
// card was worked and looked over, the agents' turns of it, what their
// programs said they took, and what was decided at the round's end.
// This is what the card's drawer says of that: what all of it came to,
// counted and never scored; what each round's decision is called; how
// long a round's bar is drawn, as an agent's history draws its turns';
// and when what was read is no longer so. It holds nothing of the page,
// so that its tests can read it.

import { lengths, short } from './agenthistory.ts';
import type { Bar } from './agenthistory.ts';
import { heldIn } from './heldin.ts';
import { amount, duration } from './util.ts';

/** What some turns came to, added up by the daemon. */
export interface Spent {
  turns: number;
  sent: number;
  wrote: number;
  cost_usd: number;
  /** How many of them said nothing of what they cost, which is no cost of nought; nothing from a daemon from before. */
  cost_unsaid?: number;
  took_ms: number;
  /** How many of them said nothing of what they took. */
  unsaid: number;
}

/** One turn of a round. */
export interface CostTurn {
  id: string;
  agent: string;
  /** What it did for the card. */
  does: 'works' | 'plans' | 'looks' | 'contends' | 'judges';
  /** Which of several pieces of work on the card at once it did, where it did one. */
  way?: string;
  status: string;
  /** What it chose, where it was given words to end on; nothing where it is kept from whoever looks. */
  chose: string | null;
  sent: number;
  wrote: number;
  cost_usd: number;
  /** Whether its program said what it cost: false where it said nothing of it, which is no cost of nought. */
  cost_said?: boolean;
  asked: number;
  took_ms: number | null;
  /** Whether its program said anything of what it took. */
  says: boolean;
  /** Asked in a channel kept to its members that whoever looks does not read: its figures and nothing else. */
  kept_from?: boolean;
}

export interface Decided {
  came: 'passed' | 'back' | 'left' | 'open';
  by?: string;
  words?: string;
  /** The line of the card's history that says it, in the board's own words. */
  said?: string;
  at?: number;
}

export interface CostRound {
  n: number;
  /** How many of its turns were put in it by the order they were written, the card's turns saying nothing of what began them. */
  by_order?: number;
  turns: CostTurn[];
  spent: Spent;
  decided: Decided;
}

export interface CardCost {
  card: string;
  rounds: CostRound[];
  spent: Spent;
  /** How many turns were put in rounds by the order they were written, of all. */
  by_order?: number;
}

/** What is said where turns from before were put in rounds by the order they were written; nothing where none was. */
export function orderSaid(placed: number | undefined): string | null {
  if (!placed) return null;
  return placed === 1
    ? '1 turn from before says nothing of what began it, and is put in a round by the order it was written.'
    : `${placed} turns from before say nothing of what began them, and are put in rounds by the order they were written.`;
}

/** What was decided at the end of a round, in a word or two. */
export function cameSaid(came: Decided['came']): string {
  return { passed: 'Passed', back: 'Sent back', left: 'Left for a person', open: 'Under way' }[came];
}

/** What a turn did, in a word. */
export const DID: Record<CostTurn['does'], string> = {
  works: 'worked',
  plans: 'planned',
  looks: 'looked',
  contends: 'worked',
  judges: 'judged',
};

/**
 * How long each round's bar is, for a track `track` long: a round's
 * tokens, sent and written together, against the largest round of the
 * card's, in proportion and nothing else, as an agent's turns are drawn
 * (`lengths`). A round whose turns said no tokens has no bar.
 */
export function roundBars(rounds: Pick<CostRound, 'spent'>[], track: number): (Bar | null)[] {
  return lengths(
    rounds.map((round) => ({ input_tokens: round.spent.sent, output_tokens: round.spent.wrote })),
    track,
  );
}

/**
 * What some turns cost, in a few words: what those that said came to, and
 * that it was not reported for those that said nothing of it, which is no
 * cost of nought; nothing where none cost anything they said. Counted,
 * never estimated.
 */
export function costSaid(spent: Pick<Spent, 'turns' | 'cost_usd' | 'cost_unsaid'>): string | null {
  const unsaid = spent.cost_unsaid ?? 0;
  if (spent.turns > 0 && unsaid >= spent.turns) return 'cost not reported';
  const said = spent.cost_usd > 0 ? amount(spent.cost_usd, 'usd') : null;
  if (unsaid === 0) return said;
  const notSaid = `cost not reported for ${unsaid} ${unsaid === 1 ? 'turn' : 'turns'}`;
  return said ? `${said}, ${notSaid}` : notSaid;
}

/** What some turns came to, in a few words: how many, the tokens, the cost where there was any or that it was not reported, and how long they were at work. */
export function spentSaid(spent: Spent): string {
  const parts = [`${spent.turns} ${spent.turns === 1 ? 'turn' : 'turns'}`];
  if (spent.sent + spent.wrote > 0) parts.push(`${short(spent.sent)} sent`, `${short(spent.wrote)} written`);
  const cost = spent.turns > spent.unsaid ? costSaid(spent) : spent.cost_usd > 0 ? amount(spent.cost_usd, 'usd') : null;
  if (cost) parts.push(cost);
  if (spent.took_ms > 0) parts.push(duration(spent.took_ms));
  if (spent.turns > 0 && spent.unsaid === spent.turns) parts.push(spent.turns === 1 ? 'which said nothing of what it took' : 'none of which said what it took');
  else if (spent.unsaid > 0) parts.push(`${spent.unsaid} said nothing of what it took`);
  return parts.join(' · ');
}

/** A turn of a round, in a line: who, what it did, what it chose, and what it took. */
export function turnSaid(turn: CostTurn): string {
  const parts = [`${turn.agent} ${DID[turn.does]}${turn.way ? ` as ${turn.way}` : ''}`];
  if (turn.status === 'failed') parts.push('failed');
  else if (turn.status !== 'done') parts.push('in hand');
  if (turn.chose) parts.push(`chose ${turn.chose}`);
  if (turn.says) {
    parts.push(`${short(turn.sent + turn.wrote)} tokens`);
    if (turn.cost_said === false && (turn.status === 'done' || turn.status === 'failed')) parts.push('cost not reported');
    else if (turn.cost_usd > 0) parts.push(amount(turn.cost_usd, 'usd'));
  } else parts.push('said nothing of what it took');
  if (turn.took_ms !== null && turn.took_ms > 0) parts.push(duration(turn.took_ms));
  if (turn.kept_from) parts.push('asked in a channel kept from you');
  return parts.join(' · ');
}

/** Whether something said may change what was read of `card`'s cost: a line of its history, or a turn of one of its conversations. */
export function touches(card: string, topic: string, data: Record<string, unknown> | undefined, known: ReadonlySet<string>): boolean {
  if (!data) return false;
  if (topic === 'collection.history') return data.record === card;
  if (!topic.startsWith('agent.turn.')) return false;
  if (typeof data.turn === 'string' && known.has(data.turn)) return true;
  return heldIn(typeof data.thread === 'string' ? data.thread : null)?.card === card;
}
