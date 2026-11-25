// What a channel offers its agents of discussions (`std/discuss`), as the
// page reads it, and what it comes to for what is being written: whether
// what a person writes asks agents to discuss something, and what a
// message's mark is said as. Functions of plain values, with their cases
// in `discussing.test.ts`; what reads and follows is `views/discussing.tsx`.

/** What a channel offers, as `GET /api/channels/<id>/discussing` answers. */
export interface Offered {
  channel: string;
  /** The settings it offers, as `discussion.offer` keeps them; nothing where it offers none. */
  settings: { on?: string[]; lead?: string } | null;
  /** Everybody who takes part, the lead first. */
  taking: string[];
  /** Its discussions that are going or wait for a person, each by the message its thread hangs from. */
  discussions?: { root: string; status: string; stretch: number }[];
}

/** How the discussion of the thread that hangs from `root` stands: `going`, `waiting` for a person, or none. */
export function discussionOf(offered: Offered | null | undefined, root: string): 'going' | 'waiting' | null {
  const found = offered?.discussions?.find((one) => one.root === root);
  return found?.status === 'going' || found?.status === 'waiting' ? found.status : null;
}

/** Who `text` names with `@`, each once, as the daemon reads it (`talk::channels::mentions`). */
export function named(text: string): string[] {
  const found: string[] = [];
  for (const match of text.matchAll(/(^|[^\p{L}\p{N}])@([A-Za-z0-9_-]+)/gu)) {
    const name = (match[2] ?? '').toLowerCase();
    if (!found.includes(name)) found.push(name);
  }
  return found;
}

/**
 * Whether `text` may be sent asking agents to discuss it: where the
 * channel takes it (`asked`) and it names two or more who take part.
 * Naming them without asking has each answer, as anywhere.
 */
export function asksToDiscuss(offered: Offered | null | undefined, text: string): boolean {
  if (!offered?.settings || !(offered.settings.on ?? []).includes('asked')) return false;
  const taking = offered.taking.map((name) => name.toLowerCase());
  return named(text).filter((name) => taking.includes(name)).length >= 2;
}

/** A message's mark, as it is drawn: nothing for one that has none or a word this page does not know. */
export function markSaid(mark: string | null | undefined): string | null {
  if (mark === 'plan') return 'Plan';
  if (mark === 'discuss') return 'To discuss';
  return null;
}
