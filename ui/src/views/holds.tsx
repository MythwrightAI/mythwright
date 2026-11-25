// What a member may ask of the factory, by area: a row for each area
// there is, with what is held there in words, and what is not held said
// as plainly as what is. For an agent that is one of a fleet's rows,
// whoever may change the row changes it here, a cell at a time: what is
// written is the row's `holds`, and the fleet follows the row. For one
// that is in no row, whoever may change agents changes it here too, and
// what is sent is what it is to hold and nothing else of it.
//
// Which areas there are, and what can be done in each, is the daemon's
// to say (`scopes` of `/api/meta`); what is held of them is worked out
// in `holds.ts`. Nothing here knows a board from a blueprint.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { api, type Slot } from '../api.ts';
import { type Area, choices, mayChange, mayGive, reads, rowOf, saidApart, saidOf, scopesAfter, tableOf } from '../holds.ts';
import { attempt, collections, me, meta } from '../store.ts';
import type { Row } from '../table.ts';
import { coalesce } from '../events.ts';
import { useEvents } from '../ui.tsx';
import { classes } from '../util.ts';

interface TableProps {
  scopes: string[];
  /** For a table that may be changed: what is done with a row's new choice. */
  onChange?: (area: string, verbs: string[]) => void;
  /** Whose scopes limit what can be given: nobody gives what they do not hold. */
  giver?: string[];
}

/** The table itself: an area, and what is held there, as words or as a choice. */
export function HoldsTable({ scopes, onChange, giver }: TableProps) {
  const areas = meta.value?.scopes ?? [];
  // A daemon from before lists no areas: the scopes are shown as they are written.
  if (areas.length === 0) return <p class="muted holds-plain">{scopes.join(', ') || 'nothing'}</p>;
  const rows = tableOf(scopes, areas);
  return (
    <table class="holds-table">
      <tbody>
        {rows.map((row) => {
          const about: Area = areas.find((one) => one.area === row.area) ?? { area: row.area, verbs: [] };
          const said = reads(about, row.verbs, row.only);
          const known = areas.some((one) => one.area === row.area);
          const offered = known ? choices(about, row.verbs) : [];
          // What is held under a word of its own is not one of the choices: it is shown, and changed in the row's table.
          const chosen = offered.findIndex((one) => one.verbs.length === row.verbs.length && one.verbs.every((verb) => row.verbs.includes(verb)));
          const plain = !onChange || !known || row.only.length > 0;
          return (
            <tr key={row.area} class={classes(said === 'nothing' && 'holds-none')}>
              <th scope="row" title={row.says}>
                {row.area}
              </th>
              <td>
                {plain ? (
                  <span class="holds-may">{said}</span>
                ) : (
                  <select
                    aria-label={`What it may do with ${row.area}`}
                    value={String(chosen)}
                    onChange={(event) => onChange!(row.area, offered[Number(event.currentTarget.value)]!.verbs)}
                  >
                    {offered.map((one, at) => {
                      // Taking away takes nothing; giving takes holding it.
                      const may = !giver || mayGive(giver, about, one.verbs.filter((verb) => !row.verbs.includes(verb)));
                      return (
                        <option key={at} value={String(at)} disabled={!may} title={may ? undefined : 'You do not hold this yourself, so it is not yours to give.'}>
                          {one.reads}
                        </option>
                      );
                    })}
                  </select>
                )}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

/**
 * An agent's: what it holds, and for one of a fleet's rows, changed here
 * by whoever may change the row. What the row leaves unsaid is what an
 * agent usually holds, and the first change writes the whole list. One in
 * no row is changed here by whoever may change agents, through its own
 * route: what it holds, whole, and nothing else of it.
 */
export function AgentHolds({ slot }: { slot: Slot }) {
  const project = slot.project;
  const has = project !== null && (collections.value[project] ?? []).some((one) => one.name === 'agents');
  const [rows, setRows] = useState<Row[] | null>(null);
  const reload = useMemo(
    () =>
      coalesce(() => {
        if (project && has) void api.records(project, 'agents').then(setRows, () => setRows([]));
        else setRows([]);
      }, 50),
    [project, has],
  );
  useEffect(reload, [reload]);
  useEvents(['record.**'], (event) => event.data?.project === project && event.data?.collection === 'agents' && reload(), [project]);
  const row = rows ? rowOf(slot.name, project, rows) : null;
  const said = row ? saidOf(row) : null;
  const usual = meta.value?.agent_scopes ?? [];
  // What its row says is what it will hold once the fleet has followed; until a row says, what it holds now.
  const scopes = said ?? slot.scopes;
  const mine = me.value?.principal.scopes ?? [];
  // Until it is known whether it is one of a row's, nothing is changed: what a row says is not what it holds now.
  const apart = rows !== null && row === null;
  const may = (row !== null || apart) && mayChange(mine, row !== null);
  const areas = meta.value?.scopes ?? [];
  const change = async (area: string, verbs: string[]) => {
    const now = scopesAfter(scopes, areas, area, verbs);
    if (row) {
      await attempt(() => api.setRecord(row.id, { holds: now }));
      reload();
      // The agent is read again when it is said to have changed, as any agent is.
    } else if (apart) await attempt(() => api.giveAgent(slot.name, now));
  };
  return (
    <>
      <h3>What it may ask of the factory</h3>
      <HoldsTable scopes={scopes} onChange={may ? change : undefined} giver={mine} />
      <p class="faint holds-where">
        {row === null
          ? rows === null
            ? 'Reading'
            : saidApart(may, meta.value?.cli ?? '', slot.name, slot.scopes)
          : said === null
            ? `This is what an agent usually holds${sameList(slot.scopes, usual) ? '' : ', as it stands now'}: its row says nothing else. ${may ? 'Change a line and the row says the whole of it.' : 'Whoever may change its row changes it there.'}`
            : `As its row in the fleet's agents says${may ? ': change a line here and the row and the fleet follow.' : '. Whoever may change that row changes it.'}`}
      </p>
    </>
  );
}

function sameList(one: string[], other: string[]): boolean {
  return one.length === other.length && one.every((scope) => other.includes(scope));
}

/** A person's own: what they hold here, read only. What another person holds is theirs and the owner's to see. */
export function PersonHolds({ you }: { you: boolean }) {
  if (!you) return null;
  const scopes = me.value?.principal.scopes ?? [];
  return (
    <>
      <h3>What you may ask of the factory</h3>
      <HoldsTable scopes={scopes} />
      <p class="faint holds-where">
        What an account holds is the factory's owner's to say. A key you make for a script holds only what you name: {meta.value?.cli ?? ''} key ci --scope board:read
      </p>
    </>
  );
}
