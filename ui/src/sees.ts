// What an agent's program sees of the device it runs on, as the daemon
// says it (`agents::allowed::seen`): its work behind a wall, or the whole
// device, and why. The words are the daemon's, worked out from the slot
// and from whether the factory holds walls while it runs, so that the
// card about an agent and the command line say one thing: a factory
// started holding none (`--walls off`) shows every program the device,
// whatever its slot asks for.

/** What the daemon says an agent's program sees: what, how, and whether it is behind a wall now. */
export interface Seen {
  what: string;
  how: string;
  walled: boolean;
}

/** What an agent's program sees, in words (what, and how that is), with what it may do with the repository after where it is behind a wall and that is not the usual. */
export function sees(slot: { sees?: Seen; git?: string }): [string, string] {
  const seen = slot.sees;
  if (!seen) return ['what the daemon has not said', 'read again'];
  const git = seen.walled && slot.git && slot.git !== 'read' ? `; git ${slot.git}` : '';
  return [seen.what, `${seen.how}${git}`];
}
