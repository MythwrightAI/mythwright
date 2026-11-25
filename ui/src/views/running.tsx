// The id of something running, drawn: the one thing that draws one,
// wherever one is written (`running.ts` says why).
//
// Beside the id is what it is, and the two are pressed to open it. One
// that names nothing any more is struck through and said to be gone, and
// is nothing to press. What an id is, is read once, the first time one
// is drawn, and never again: nothing here asks on a timer.

import { api, ApiError } from '../api.ts';
import { asked, known, named, parts, tell } from '../running.ts';
import type { Known } from '../running.ts';
import { openTab } from '../store.ts';

/** Reads what an id is, once. One the daemon says is not there is gone; one that could not be read is left as its id. */
function learn(id: string): void {
  if (asked.has(id)) return;
  asked.add(id);
  api.instance(id).then(
    ({ instance }) => tell(id, { blueprint: instance.blueprint, key: instance.key, project: instance.project }),
    (err) => {
      if (err instanceof ApiError && err.status === 404) tell(id, 'gone');
    },
  );
}

/** What an id is, as far as it has been read; asked about, the first time, by being asked for here. */
export function whatIs(id: string): Known | 'gone' | undefined {
  const what = known.value[id];
  if (what === undefined) learn(id);
  return what;
}

/**
 * One id. `plain` is for where it stands inside something that is pressed
 * for something else (a line of a list): the same words, and nothing of
 * its own to press.
 */
export function Running({ id, plain = false }: { id: string; plain?: boolean }) {
  const what = whatIs(id);
  if (what === 'gone') {
    return (
      <span class="running-id gone" title="Nothing is kept under this id any more: what it named is gone.">
        <code>{id}</code> <span class="running-gone">gone</span>
      </span>
    );
  }
  const inside = (
    <>
      <code>{id}</code>
      {what && ' '}
      {what && <span class="running-is">{named(what)}</span>}
    </>
  );
  if (plain) return <span class="running-id">{inside}</span>;
  return (
    <button
      type="button"
      class="running-id"
      title={what ? `Open ${named(what)}` : 'Open what is running under this id'}
      onClick={(event) => {
        // Where it stands in something else that is pressed, this is the press.
        event.stopPropagation();
        openTab({ kind: 'instance', id });
      }}
    >
      {inside}
    </button>
  );
}

/** A text, with each id of something running in it drawn as what it is. */
export function WithRunning({ text, plain = false }: { text: string; plain?: boolean }) {
  return <>{parts(text).map((part, at) => (typeof part === 'string' ? part : <Running key={at} id={part.id} plain={plain} />))}</>;
}
