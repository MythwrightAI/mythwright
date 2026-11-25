// The line that says who is writing in a channel or a thread of it, over
// where one writes there.
//
// Words that small, in the quietest colour there is, came and went unseen.
// So while somebody is writing three dots swell in turn before the words,
// and the words are a shade stronger. The line's room is there whether or
// not anybody is writing and the dots are drawn inside it, so nothing else
// moves as it comes and goes; whoever asked for less movement is shown
// the dots still.

import { projectOfCard, typingLine } from '../typing.ts';
import type { AtWork } from '../typing.ts';
import { projects, showCard } from '../store.ts';

export function Typing({ names, edge }: { names: string[]; edge?: boolean }) {
  const says = typingLine(names);
  return (
    <div class={edge ? 'typing-line typing-edge' : 'typing-line'} aria-live="polite">
      {says && (
        // Said by the words; the dots are for the eye.
        <span class="typing-dots" aria-hidden="true">
          <i />
          <i />
          <i />
        </span>
      )}
      {says}
    </div>
  );
}

/**
 * Who is at work for the channel and not writing in it (an agent on a
 * card its fleet reports here), in a quiet line of its own: never the
 * line of who is typing, whose room it lies over at the right, so that
 * it takes none and moves nothing. Each says what it is doing, and one on
 * a card is pressed to open the card.
 */
export function WorkingLine({ at }: { at: AtWork[] }) {
  return (
    <div class="working-room" aria-live="polite">
      {at.length > 0 && (
        <div class="working-line">
          {at.map((one) => {
            const project = one.card ? projectOfCard(one.card, projects.value.map((each) => each.id)) : null;
            return one.card && project ? (
              <button key={one.name} class="ghost link working-on" title={`${one.words}: open the card`} onClick={() => showCard(one.card!, project)}>
                {one.words}
              </button>
            ) : (
              <span key={one.name} class="working-on">
                {one.words}
              </span>
            );
          })}
        </div>
      )}
    </div>
  );
}
