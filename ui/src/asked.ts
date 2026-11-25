// A change somebody asked for that they may not make themselves, as the
// part of a told line that names it shows it (`told::asks`,
// `docs/approvals.md`): each cell from what it held when it was asked,
// what it holds now where that is otherwise, to what it is to hold, with
// each scope it takes and whether whoever reads holds it; and whether
// they may decide it, which is the daemon's to say. Nothing here asks
// anything or holds anything of the page, so that its tests can read it.

/** A scope a cell takes, and whether whoever reads holds it. */
export interface Takes {
  scope: string;
  held: boolean;
}

/** A cell of an ask, as the daemon shows it now. */
export interface AskedCell {
  name: string;
  before: unknown;
  now: unknown;
  after: unknown;
  takes: Takes[];
}

/** An ask, as whoever reads is shown it (`Asks::shown`). */
export interface Change {
  ask: string;
  project: string;
  record: string;
  collection: string;
  title: unknown;
  asker: { actor: string; name: string };
  why: string;
  cells: AskedCell[];
  /** What changing the record at all takes, beside each cell (nothing, from a daemon before this). */
  takes?: Takes[];
  status: 'waiting' | 'approved' | 'declined' | 'stale' | 'withdrawn' | 'expired';
  decided: { by: string; name: string; at: number; why: string } | null;
  at: number;
  may: boolean;
  why_not?: string | null;
}

/** The part of a told line that names an ask: what it comes to now, or why the reader is shown nothing of it. */
export interface ChangePart {
  kind: 'change';
  project: string;
  change?: Change;
  ask?: string;
  refused?: string;
}

/** A value of a cell as a row says it: words as they are, nothing as a dash, anything else as it is written. */
export function valueSaid(value: unknown): string {
  if (value === null || value === undefined || value === '') return '—';
  if (typeof value === 'string') return value;
  if (Array.isArray(value)) return value.map(valueSaid).join(', ');
  return JSON.stringify(value);
}

/** A row a cell: what it held when asked, what it holds now where that is otherwise, and what it is to hold. */
export function rowsOf(change: Change): { name: string; was: string; now: string | null; to: string; takes: Takes[] }[] {
  return change.cells.map((cell) => {
    const was = valueSaid(cell.before);
    const now = valueSaid(cell.now);
    return { name: cell.name, was, now: now === was ? null : now, to: valueSaid(cell.after), takes: cell.takes };
  });
}

/** What the part is headed with: whose change of what. */
export function headOf(change: Change): string {
  const title = typeof change.title === 'string' && change.title.trim() !== '' ? ` (${change.title})` : '';
  return `${change.asker.name} asks to change ${change.record}${title} of ${change.collection}`;
}

/** How the ask stands, in words: what waits, or what it came to and by whom. */
export function standing(change: Change): string {
  const by = change.decided?.name;
  const why = change.decided?.why?.trim() ? `: ${change.decided.why.trim()}` : '';
  switch (change.status) {
    case 'waiting':
      return 'It waits for somebody who could make it to decide.';
    case 'approved':
      return `${by || 'Somebody'} approved it, and it is made.`;
    case 'declined':
      return `${by || 'Somebody'} declined it${why}.`;
    case 'stale':
      return 'It was not made: the record was changed since it was asked.';
    case 'withdrawn':
      // By the factory, saying why: whoever asked was taken away.
      return change.decided?.by === 'system' && why ? `It was withdrawn${why}.` : 'Whoever asked withdrew it.';
    case 'expired':
      return 'It was not decided in seven days and goes no further.';
  }
}

/** What whoever reads is offered: Approve and Decline, or neither and why where the daemon says. */
export function offered(change: Change): { decide: boolean; why: string | null } {
  if (change.status !== 'waiting') return { decide: false, why: null };
  return change.may ? { decide: true, why: null } : { decide: false, why: change.why_not ?? null };
}

/** What an approval came to, said once the daemon has answered; anything else is said by how it stands, once. */
export function decidedSaid(change: Change): string | null {
  return change.status === 'approved' ? `Approved: ${change.record} is changed, as ${change.asker.name}'s.` : null;
}

/** What changing the record at all takes, said where the reader does not hold all of it: nothing otherwise. */
export function wholeSaid(change: Change): string | null {
  const takes = change.takes ?? [];
  if (takes.length === 0 || takes.every((one) => one.held)) return null;
  const said = takes.map((one) => (one.held ? one.scope : `${one.scope} (not yours)`));
  return `Changing it at all takes ${said.join(', ')}.`;
}
