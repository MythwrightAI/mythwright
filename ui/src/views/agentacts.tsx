// What can be done with an agent, as the lines of a menu: its line's menu
// on the Agents tab (`agentlist.tsx`) and an agent's menu in the side
// panel's Members (`direct.tsx`) are made by this one function,
// so that the two never part. Each line is there only for whoever the
// daemon would let do it, its setup and its history for whoever reads.
//
// What two of its lines ask before anything is done (moving it to another
// project, removing it for good) is drawn once in the frame (`AgentAsked`),
// whichever menu it was asked from.

import { signal } from '@preact/signals';
import { api } from '../api.ts';
import type { Slot } from '../api.ts';
import { mayMove } from '../agentmoved.ts';
import { attempt, me, refreshAgents } from '../store.ts';
import { mayTakeAway } from '../takenaway.ts';
import { showSetup } from './agentedit.tsx';
import { showHistory } from './agenthistory.tsx';
import { MoveAgent } from './agentmoved.tsx';
import type { Offered } from './rowmenu.tsx';
import { signs } from './signs.tsx';
import { TakeAway } from './takenaway.tsx';

/** The agent somebody asked to have taken away, until they say yes or no. */
const taking = signal<string | null>(null);
/** The agent somebody is moving to another project, until they say where or leave it. */
const moving = signal<Slot | null>(null);

/** What can be done with `slot` by whoever reads this, in the order its menu offers it. */
export function agentActs(slot: Slot): Offered[] {
  const who = me.value;
  if (!who) return [];
  const scopes = who.principal.scopes;
  const admin = scopes.includes('*') || scopes.includes('admin');
  // Its setup and its history are for whoever reads, so every menu has those first: what is not the reader's to
  // change is closed in the setup, saying why.
  const items: Offered[] = [
    { label: 'Edit…', mark: signs.pencil(), act: () => showSetup(slot.name) },
    { label: 'History', mark: signs.clock(), act: () => showHistory(slot.name) },
  ];
  if (slot.active && admin) items.push({ label: 'Stop', mark: signs.stop(), act: () => void attempt(() => api.stopAgent(slot.name)).then(() => refreshAgents()) });
  // One a fleet counts has its fleet's project, and is moved in its row.
  if (mayMove(slot, scopes)) items.push({ label: 'Move to project…', mark: signs.move(), act: () => (moving.value = slot) });
  // Removing one is for good, and is asked about first: set apart from what is done every day.
  if (mayTakeAway(who.principal)) items.push({ label: 'Remove…', mark: signs.bin(), apart: true, look: 'grave', act: () => (taking.value = slot.name) });
  return items;
}

/** What is asked before an agent is moved or removed, drawn once in the frame. */
export function AgentAsked() {
  return (
    <>
      {taking.value && <TakeAway name={taking.value} onClose={() => (taking.value = null)} />}
      {moving.value && <MoveAgent slot={moving.value} onClose={() => (moving.value = null)} />}
    </>
  );
}
