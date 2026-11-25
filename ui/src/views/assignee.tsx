// Whom a card is assigned to, drawn the same wherever a card's assignee is
// said: its name, in the agents' colour for an agent, and what kind of
// somebody it is (`assignedTo`, over what the page has read of agents and
// people). `said` puts "Assigned to" before it, where it stands alone as a
// property; `brief` keeps it to one line at the end of a row, the name
// giving way and the kind whole.

import { assignedTo } from '../assigned.ts';
import type { Assigned } from '../assigned.ts';
import { usePeople } from '../peopleknown.ts';
import { agents, me } from '../store.ts';
import { classes } from '../util.ts';

/** Whom `assignee` names, as the page knows them now. */
export function useAssigned(assignee: string): Assigned {
  const people = usePeople();
  return assignedTo(assignee, agents.value, me.value ? { actor: me.value.actor, name: me.value.account?.name ?? null } : null, people);
}

/** The words for whom a card is assigned to, as a tooltip says them whole. */
export function assignedWords(who: Assigned, said = true): string {
  return `${said ? 'Assigned to ' : ''}${who.name}${who.kind ? ` (${who.kind})` : ''}`;
}

export function Assignee({ assignee, said = false, brief = false, class: extra }: { assignee: string; said?: boolean; brief?: boolean; class?: string }) {
  const who = useAssigned(assignee);
  return (
    <span class={classes('assigned', brief && 'assigned-brief', extra)} title={assignedWords(who)} data-assigned={who.kind ?? ''}>
      {said && (
        <>
          <span class="faint">Assigned to</span>{' '}
        </>
      )}
      <span class={classes('assignee-name', who.agent && 'agent')}>{who.name}</span>
      {who.kind && (
        <>
          {' '}
          <span class="faint">({who.kind})</span>
        </>
      )}
    </span>
  );
}
