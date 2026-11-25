// Settings' Groups: people and agents together, with what their members
// may ask of the factory beside what each holds itself
// (`people::groups`, over `peoplegroups.ts`). For whoever manages the
// factory, who alone changes a group. The daemon holds every rule; what
// is offered is what it would let through, a choice it would refuse is
// closed saying why, and what it refuses is said in its sentence.
//
// Read once and again as something is said of a group
// (`identity.group.*`), one reading at a time with the last asked made;
// nothing polls.

import { useEffect, useRef, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { type Area, scopesAfter } from '../holds.ts';
import { type Joiner, type PeopleGroup, addClosed, addable, countSaid, holdsSaid, joiners, mayManage, memberOf, nameRefused } from '../peoplegroups.ts';
import { agents, attempt, me, meta, toast } from '../store.ts';
import { Modal, useEvents } from '../ui.tsx';
import { HoldsTable } from './holds.tsx';
import { useKept } from './returned.tsx';

interface Props {
  /** The people with an account here, as Settings' People read them. */
  people: { id: string; name: string }[];
  /** Told when a change of a group may have changed what somebody holds. */
  changed: () => void;
}

export function PeopleGroups({ people, changed }: Props) {
  const mine = me.value?.principal.scopes ?? [];
  // Kept by the tab, so that Settings shown again draws the groups it read at once and reads them again behind (`returned.ts`).
  const [groups, setGroups] = useKept<PeopleGroup[] | null>('peoplegroups.groups', null);
  const [making, setMaking] = useState(false);
  const asked = useRef(0);
  const read = () => {
    const nth = ++asked.current;
    void call<{ groups: PeopleGroup[] }>('GET', '/groups')
      .then((answer) => nth === asked.current && setGroups(answer.groups))
      .catch(() => nth === asked.current && setGroups([]));
  };
  useEffect(read, []);
  useEvents('identity.group.*', () => {
    read();
    changed();
  });
  if (!mayManage(mine)) return null;
  const known = joiners(people, agents.value);
  return (
    <div class="panel people-groups" role="region" aria-label="Groups">
      <div class="panel-head">
        <h2>Groups</h2>
        <span class="muted grow people-groups-about">people and agents given what they may ask together</span>
        {!making && <button onClick={() => setMaking(true)}>New group</button>}
      </div>
      {making && <MakeGroup groups={groups ?? []} done={() => setMaking(false)} />}
      {groups === null && <p class="faint people-groups-said">Reading</p>}
      {groups?.length === 0 && !making && (
        <p class="faint people-groups-said">
          No group yet. A group's people and agents each hold what it holds beside what they hold themselves, from their next request.
        </p>
      )}
      {groups?.map((group) => <GroupLine key={group.id} group={group} groups={groups} known={known} mine={mine} />)}
    </div>
  );
}

/** A name for a new group, asked under the panel's head. */
function MakeGroup({ groups, done }: { groups: PeopleGroup[]; done: () => void }) {
  const [name, setName] = useState('');
  const [said, setSaid] = useState<string | null>(null);
  const refused = name.trim() === '' ? null : nameRefused(name, groups);
  const make = async (event: Event) => {
    event.preventDefault();
    if (name.trim() === '' || refused) return;
    try {
      await call('POST', '/groups', { name: name.trim(), holds: [] });
      toast(`Made ${name.trim()}`);
      done();
    } catch (error) {
      setSaid((error as Error).message);
    }
  };
  return (
    <form class="group-make" onSubmit={make}>
      <input aria-label="What the new group is called" placeholder="Group name" value={name} autoFocus onInput={(event) => setName(event.currentTarget.value)} />
      <button type="submit" disabled={name.trim() === '' || refused !== null}>
        Make it
      </button>
      <button type="button" class="ghost" onClick={done}>
        Cancel
      </button>
      {(refused ?? said) && <p class="faint group-said">{refused ?? said}</p>}
    </form>
  );
}

function GroupLine({ group, groups, known, mine }: { group: PeopleGroup; groups: PeopleGroup[]; known: Joiner[]; mine: string[] }) {
  const [name, setName] = useState(group.name);
  const [said, setSaid] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [chosen, setChosen] = useState('');
  const [removing, setRemoving] = useState(false);
  // What the daemon calls it, followed while it is not being written in.
  const writing = useRef(false);
  useEffect(() => {
    if (!writing.current) setName(group.name);
  }, [group.name]);
  const at = `/groups/${encodeURIComponent(group.id)}`;
  const rename = async () => {
    writing.current = false;
    const now = name.trim();
    if (now === group.name) return;
    const refused = nameRefused(now, groups, group.id);
    if (refused) {
      setSaid(refused);
      return;
    }
    try {
      await call('PATCH', at, { name: now });
      setSaid(null);
    } catch (error) {
      setSaid((error as Error).message);
    }
  };
  const areas: Area[] = meta.value?.scopes ?? [];
  const holds = group.holds ?? [];
  const closed = addClosed(mine, group, areas);
  const offered = addable(group, known);
  return (
    <div class="group-line" data-group={group.name}>
      <div class="group-head">
        <input
          class="group-name"
          aria-label={`What ${group.name} is called`}
          value={name}
          onInput={(event) => {
            writing.current = true;
            setName(event.currentTarget.value);
          }}
          onKeyDown={(event) => event.key === 'Enter' && void rename()}
          onBlur={() => void rename()}
        />
        <span class="faint group-count">{countSaid(group)}</span>
        <span class="grow" />
        <button class="ghost danger group-remove" onClick={() => setRemoving(true)}>
          Remove…
        </button>
      </div>
      {said && <p class="faint group-said">{said}</p>}
      <div class="group-members">
        {group.members.map((actor) => {
          const one = memberOf(actor, known);
          return (
            <span class="group-member" key={actor} data-actor={actor}>
              <span class="group-member-name">{one.name}</span>
              <span class="faint">{one.kind}</span>
              <button
                type="button"
                class="ghost icon"
                aria-label={`Take ${one.name} out of ${group.name}`}
                title={`Take ${one.name} out of ${group.name}: they hold what it holds no longer`}
                onClick={() => void attempt(() => call('DELETE', `${at}/members/${encodeURIComponent(actor)}`))}
              >
                ×
              </button>
            </span>
          );
        })}
        {offered.length > 0 && (
          <span class="group-add">
            <select
              aria-label={`Somebody to put in ${group.name}`}
              value={chosen}
              disabled={closed !== null}
              title={closed ?? undefined}
              onChange={(event) => setChosen(event.currentTarget.value)}
            >
              <option value="">Add…</option>
              {offered.map((one) => (
                <option key={one.actor} value={one.actor}>
                  {one.name} ({one.kind})
                </option>
              ))}
            </select>
            <button
              type="button"
              disabled={!chosen || closed !== null}
              onClick={async () => {
                const answer = await attempt(() => call('POST', `${at}/members`, { who: chosen }));
                if (answer) setChosen('');
              }}
            >
              Add
            </button>
          </span>
        )}
        {closed && <span class="faint group-closed">{closed}</span>}
      </div>
      <button type="button" class="ghost group-fold" aria-expanded={open} onClick={() => setOpen(!open)}>
        <span>{open ? '▾' : '▸'}</span> What its members may ask beside their own: <span class="muted">{holdsSaid(group)}</span>
      </button>
      {open && (
        <div class="group-holds">
          <HoldsTable
            scopes={holds}
            giver={mine}
            onChange={(area, verbs) => void attempt(() => call('PATCH', at, { holds: scopesAfter(holds, areas, area, verbs) }))}
          />
        </div>
      )}
      {removing && (
        <Modal onClose={() => setRemoving(false)}>
          <h2>Remove {group.name}?</h2>
          <p>
            {group.members.length === 0 ? 'Nobody is in it.' : `Whoever is in it (${countSaid(group)}) holds what it holds no longer, from their next request, and the keys its people made hold no more than they then do.`} Nobody is taken out of anything else.
          </p>
          <div class="row modal-foot">
            <span class="grow" />
            <button class="ghost" onClick={() => setRemoving(false)}>
              Cancel
            </button>
            <button
              class="danger"
              onClick={async () => {
                if (await attempt(() => call('DELETE', at))) {
                  setRemoving(false);
                  toast(`${group.name} is gone`);
                }
              }}
            >
              Remove it
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
