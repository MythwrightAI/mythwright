// Inviting somebody to make an account on this installation: one button,
// and under it the code it made, which is given to them and works once;
// on a plan for one person the button is not to be pressed, and says
// why. It is where Settings' People has it and on the Members page's
// People tab, one and the same (`settings.tsx`, `members.tsx`). Whoever
// is shown it is the page's to say: whoever manages the factory.

import type { ComponentChildren } from 'preact';
import { useState } from 'preact/hooks';
import { api } from '../api.ts';
import { attempt, me } from '../store.ts';
import { Field } from '../ui.tsx';

/** The button, for a panel's head, and what is said under the head: that the plan does not invite, or the code just made. */
export function useInviting(): { button: ComponentChildren; under: ComponentChildren } {
  const who = me.value!;
  const [invite, setInvite] = useState('');
  const button = (
    <button
      class="invite-someone"
      disabled={!who.can_invite}
      title={who.can_invite ? '' : 'Inviting people needs the premium plan'}
      onClick={async () => {
        const answer = await attempt(() => api.invite());
        if (answer) setInvite(answer.code);
      }}
    >
      Invite someone
    </button>
  );
  const under = (
    <>
      {!who.can_invite && <p class="muted" style="padding:10px 14px 0">The free plan is for one person. Switch plan to invite others.</p>}
      {invite && (
        <div style="padding:10px 14px 0">
          <Field label="Invitation code: give it to the person; it works once">
            <code class="code-box">{invite}</code>
          </Field>
        </div>
      )}
    </>
  );
  return { button, under };
}
