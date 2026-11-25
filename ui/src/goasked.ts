// A run that waits for a person to let it go on (`approval.ask`,
// `told::goes`), as a told line's part and the run's page show it: whose
// run of what, what it says is to go on, under which environment, the
// addresses to look at, how it stands, and whether whoever reads may let
// it go on or decline it, which is the daemon's to say. Nothing here asks
// anything or holds anything of the page, so that its tests can read it.

/** A run that waits, as whoever reads is shown it (`Asks::shown_go`). */
export interface Go {
  ask: string;
  kind: 'go';
  project: string;
  run: string;
  step: string;
  instance: string;
  blueprint: string;
  says: string;
  environment: string;
  links: string[];
  until: number;
  asker: { actor: string; name: string };
  status: 'waiting' | 'approved' | 'declined' | 'stale' | 'withdrawn' | 'expired';
  decided: { by: string; name: string; at: number; why: string } | null;
  at: number;
  may: boolean;
  why_not?: string | null;
}

/** The part of a told line that names a run that waits: what it comes to now, or why the reader is shown nothing of it. */
export interface GoPart {
  kind: 'go';
  project: string | null;
  go?: Go;
  ask?: string;
  refused?: string;
}

/** What the part is headed with: whose run of what, and under which environment. */
export function goHead(go: Go): string {
  const under = go.environment ? `, to go under ${go.environment}` : '';
  return `${go.asker.name}'s run of ${go.blueprint} waits for a go-ahead${under}`;
}

/** How it stands, in words: what waits and until when, or what it came to and by whom. */
export function goStanding(go: Go, now: number): string {
  const by = go.decided?.name || 'Somebody';
  const why = go.decided?.why?.trim() ? `: ${go.decided.why.trim()}` : '';
  switch (go.status) {
    case 'waiting':
      return `It waits for somebody who may let it go on, for ${left(go.until - now)} more.`;
    case 'approved':
      return `${by} approved it, and the run goes on.`;
    case 'declined':
      return `${by} declined to let it go on${why}.`;
    case 'expired':
      return 'Nobody let it go on in the time it waits, and it goes no further.';
    case 'withdrawn':
    case 'stale':
      return 'It waits no more.';
  }
}

/** How long is left, in the largest whole unit: `6 days`, `3 hours`, `under a minute`. */
export function left(ms: number): string {
  const minutes = Math.floor(ms / 60_000);
  if (minutes < 1) return 'under a minute';
  const [count, unit] =
    minutes < 60 ? [minutes, 'minute'] : minutes < 48 * 60 ? [Math.floor(minutes / 60), 'hour'] : [Math.floor(minutes / 1440), 'day'];
  return `${count} ${unit}${count === 1 ? '' : 's'}`;
}

/** What whoever reads is offered: Approve and Decline, or neither and why where the daemon says. */
export function goOffered(go: Go): { decide: boolean; why: string | null } {
  if (go.status !== 'waiting') return { decide: false, why: null };
  return go.may ? { decide: true, why: null } : { decide: false, why: go.why_not ?? null };
}

/** What a decision came to, said once the daemon has answered. */
export function goDecidedSaid(go: Go): string | null {
  switch (go.status) {
    case 'approved':
      return `Approved: the run of ${go.blueprint} goes on.`;
    case 'declined':
      return `Declined: the run of ${go.blueprint} goes no further.`;
    default:
      return null;
  }
}

/** The asks a run's page reads (`GET /api/asked?run=`), those that wait first and then the newest. */
export function runAsks(read: { asked?: unknown } | null | undefined): Go[] {
  const listed = Array.isArray(read?.asked) ? (read.asked as Go[]) : [];
  const goes = listed.filter((one) => one?.kind === 'go');
  return [...goes].sort((a, b) => Number(b.status === 'waiting') - Number(a.status === 'waiting') || b.at - a.at);
}
