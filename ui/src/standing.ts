// How the stream of events stands, and what is said of that.
//
// The page reads once and then follows the daemon's events. While they
// arrive there is nothing to say of it: that is how the page is meant to
// be, and a mark that is always there is one more thing to wonder about.
// What a person needs telling is the other thing, that what they see has
// stopped being kept up, and only for as long as it is so.

/** What the browser last said of the stream. */
export type Heard = 'opened' | 'failed' | 'closed';

/**
 * Where that leaves it: not yet asked for or put away on purpose
 * (`waiting`), arriving (`live`), or asked for and not arriving (`lost`).
 */
export type Standing = 'waiting' | 'live' | 'lost';

/**
 * A stream that was closed here was closed on purpose (somebody signed
 * out, or it is being opened again from another place): nothing was lost.
 * Only the browser saying that it could not be had is.
 */
export function standingAfter(heard: Heard): Standing {
  return heard === 'opened' ? 'live' : heard === 'failed' ? 'lost' : 'waiting';
}

/** What is said where the page is headed, or nothing. */
export function saidOf(standing: Standing): string | null {
  return standing === 'lost' ? 'The daemon is not answering, so what happens is not arriving. Trying again.' : null;
}
