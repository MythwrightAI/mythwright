// Members: everybody in the factory, a page of its own under Factory in
// the Resources panel, opened by the Members line's name (its arrow
// unfolds the same people and agents under it, `direct.tsx`). Two tabs
// in its head, the people and then the agents, as the members of a
// workspace are set out in the settings of tools for working together.
//
// People is who has an account here, as the Members line lists them
// (`everybody`): whoever reads this first, said `you`, then everybody
// else by name. Each line has what its line in the panel offers, by the
// one function (`offeredFor`): who they are, a message, and for whoever
// manages the factory what they hold; a message is also a button of its
// own at the line's end. Whoever manages the factory is offered inviting
// somebody as Settings' People offers it, one and the same
// (`invite.tsx`). The daemon says nothing more of a person to whoever
// reads channels (`GET /people` is who and what they are called), so
// nothing more is said here.
//
// Agents is what was the Agents page, moved (`agents.tsx`): its choice of
// whose agents are shown stands in the page's head while the tab shows.
//
// Which tab shows is kept by the page's tab (`useKept`), so the tab shown
// again is as it was left, and is said to the address bar
// (`membersshown.ts`); an address, or a press elsewhere that asks for the
// agents, brings the page to the tab it asks for.

import { useEffect, useRef } from 'preact/hooks';
import type { MemberLine } from '../direct.ts';
import { lineTitle } from '../direct.ts';
import { MEMBERS_CALLED, MEMBERS_TABS, type MembersTab, membersAsked, membersTabOf, sayMembersShowing } from '../membersshown.ts';
import { me, tabs } from '../store.ts';
import { Empty } from '../ui.tsx';
import { AgentsNarrow, AgentsTab } from './agents.tsx';
import { everybody, offeredFor, readPeople, waitingWith } from './direct.tsx';
import { More } from './grouped.tsx';
import { useInviting } from './invite.tsx';
import { useKept } from './returned.tsx';
import { reveal, sideways } from './sideways.ts';
import { Face } from './signs.tsx';

/** `at` is the tab it was kept at, where that was said: one kept from when the agents were a page of their own. */
export function MembersView({ at }: { at?: string }) {
  const [tab, setTab] = useKept<MembersTab>('members.tab', () => membersAsked.peek()?.at ?? membersTabOf(at));
  const row = useRef<HTMLDivElement>(null);
  // A tab somebody asked for (an address, an agent's name pressed elsewhere): taken once, here or as the page is next drawn.
  const asking = membersAsked.value;
  useEffect(() => {
    if (!asking) return;
    setTab(asking.at);
    membersAsked.value = null;
  }, [asking]);
  // Where it was kept at a tab, that is now the page's to keep: the tab is kept as any other is from here on.
  useEffect(() => {
    if (at !== undefined) tabs.value = tabs.value.map((one) => (one.kind === 'members' ? { kind: 'members' } : one));
  }, [at]);
  useEffect(() => sayMembersShowing(tab), [tab]);
  useEffect(() => reveal(row.current), [tab]);
  return (
    <div class="members-page">
      <div class="view-head">
        <h1>Members</h1>
        <div class="view-tabs tabs-ruled members-tabs">
          <div class="view-tabs-list" role="tablist" aria-label="Members" ref={row} onWheel={sideways}>
            {MEMBERS_TABS.map((one) => (
              <button key={one} type="button" role="tab" class="view-tab members-tab" data-members-tab={one} aria-selected={one === tab} aria-controls={`members-panel-${one}`} onClick={() => setTab(one)}>
                {MEMBERS_CALLED[one]}
              </button>
            ))}
          </div>
          {/* Whose agents are shown, at the end of the tabs' row while the agents show: the head stays as it is from one tab to the other. */}
          {tab === 'agents' && <AgentsNarrow />}
        </div>
      </div>
      <div role="tabpanel" id={`members-panel-${tab}`} aria-label={MEMBERS_CALLED[tab]} class="members-panel" data-members-tab={tab}>
        {tab === 'people' ? <PeopleTab /> : <AgentsTab />}
      </div>
    </div>
  );
}

/** Who has an account here, each with what can be done with them, and inviting somebody for whoever may. */
function PeopleTab() {
  const scopes = me.value!.principal.scopes;
  const manages = scopes.includes('*') || scopes.includes('admin');
  const inviting = useInviting();
  // Read anew as the tab is shown, as the Members line is as it is unfolded, so that whoever registered since is listed.
  useEffect(() => void readPeople(), []);
  const people = everybody.value.filter((line) => line.kind !== 'agent');
  return (
    <div class="view-body stack people-page" style="gap:20px">
      <div class="panel people-listed">
        <div class="panel-head">
          <h2 class="grow">People</h2>
          <span class="muted">Everybody with an account here. The agents are under Agents.</span>
          {manages && inviting.button}
        </div>
        {manages && inviting.under}
        {people.length === 0 ? (
          <Empty title="Reading who is here" />
        ) : (
          <ul>
            {people.map((line) => (
              <PersonLine key={line.actor} line={line} />
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

/** One person: who they are, how many wait in the direct message with them, a message, and their menu. */
function PersonLine({ line }: { line: MemberLine }) {
  const items = offeredFor(line);
  const message = items.find((one) => one.label.startsWith('Message'));
  const count = line.kind === 'you' ? 0 : waitingWith(line.actor);
  return (
    <li class="person-line" data-actor={line.actor} title={lineTitle(line)}>
      <Face name={line.name} />
      <strong class="person-name truncate">{line.name}</strong>
      {line.said && <span class="faint person-said">{line.said}</span>}
      {count ? (
        <span class="person-waiting" title={`${count} unread in the direct message with ${line.name}`}>
          {count}
        </span>
      ) : null}
      <span class="person-acts">
        {message && (
          <button type="button" class="person-message" aria-label={`Message ${line.name}`} onClick={() => message.act?.()}>
            Message
          </button>
        )}
        <More label={`More that can be done with ${line.name}`} items={() => offeredFor(line)} keys />
      </span>
    </li>
  );
}
