// What runs in a channel, under its settings in the Channel tab of the
// Tools panel: each thing applied for it (a discussion, a project's own
// status updates), opened by its name, with Settings… for its params as a
// form made from its blueprint, as the apply drawer makes one, and
// Stop… behind a question. What is worked out is `runsin.ts`; which
// things run here is the daemon's to say, read once for the channel and
// read again as something is applied, changed or ended (nothing polls),
// and only for whoever may read what is applied.

import { signal } from '@preact/signals';
import { useEffect, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Channel, Fields, Instance } from '../api.ts';
import { covers } from '../holds.ts';
import { hashed } from '../kept.ts';
import { againOf, askedOf, heardOf, offeredOf, saidOf, stopSaid } from '../runsin.ts';
import { attempt, drawer, me, openTab, toast } from '../store.ts';
import type { Drawer } from '../store.ts';
import { Modal, useEvents } from '../ui.tsx';
import { Applying } from './applying.tsx';

/** What runs in each channel, by its id, as it was last read. */
const running = signal<Record<string, Instance[]>>({});
/** A reading on its way for a channel, and whether another was asked for meanwhile: one at a time, and the last asked made. */
const reading = new Map<string, boolean>();

async function read(channel: string): Promise<void> {
  if (reading.has(channel)) {
    reading.set(channel, true);
    return;
  }
  reading.set(channel, false);
  try {
    const said = await call<{ instances: Instance[] }>('GET', `/channels/${encodeURIComponent(channel)}/running`).catch(() => null);
    if (said) running.value = { ...running.value, [channel]: said.instances };
  } finally {
    const again = reading.get(channel);
    reading.delete(channel);
    if (again) void read(channel);
  }
}

/** What is said of the daemon's refusal on a page: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** Opens the form of what `instance` is applied with, made from its blueprint as it now is, in the drawer. */
async function openSettings(instance: Instance, channel: Channel): Promise<void> {
  const described = await attempt(() => api.blueprints(instance.project));
  if (!described) return;
  const blueprint = described.find((one) => one.name === instance.blueprint);
  if (!blueprint) {
    toast(`${instance.blueprint} is not a blueprint ${instance.project ? 'its project' : 'the factory'} has now, so its settings are not offered here: stop it, or apply it again from its page.`, true);
    return;
  }
  const { asked, kept } = askedOf(blueprint.params, instance, channel);
  const key = `runsin:${instance.id}`;
  const mine: Drawer = {
    title: `Settings of ${instance.blueprint} in ${hashed(channel)}`,
    body: () => (
      <Applying
        key={key}
        kept={key}
        params={asked}
        known={instance.params}
        onClose={() => drawer.value === mine && (drawer.value = null)}
        onApply={async (values: Fields) => {
          const applied = await attempt(() => api.apply(againOf(instance, values, kept)));
          if (!applied) return false;
          toast(applied.changed ? `${instance.blueprint} goes by its new settings from now.` : `${instance.blueprint} was applied with these already; nothing changed.`);
          void read(channel.id);
          return true;
        }}
      />
    ),
  };
  drawer.value = mine;
}

/** Asks whether `instance` is to be stopped, saying what stops and what stays. */
function AskStop({ instance, channel, onClose }: { instance: Instance; channel: Channel; onClose: () => void }) {
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const words = stopSaid(instance, hashed(channel));
  const stop = async () => {
    setBusy(true);
    setWrong(null);
    try {
      await api.stopInstance(instance.id);
      toast(`Stopped ${instance.blueprint}.`);
      void read(channel.id);
      onClose();
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>{words.title}</h2>
      <div class="stack">
        <p style="margin:0">{words.says}</p>
        <p class="faint kept-note">{words.stays}</p>
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="button" disabled={busy} onClick={() => void stop()}>
            Stop it
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** One thing that runs here: its name, opening it, what is said of it, and what may be done to it. */
function Line({ instance, channel, scopes }: { instance: Instance; channel: Channel; scopes: string[] }) {
  const [stopping, setStopping] = useState(false);
  const offered = offeredOf(instance, scopes);
  const under = saidOf(instance);
  return (
    <div class="runsin-line" data-instance={instance.id}>
      <button type="button" class="runsin-name" title={`Open ${instance.blueprint}`} onClick={() => openTab({ kind: 'instance', id: instance.id })}>
        <span class="truncate">{instance.blueprint}</span>
        {under && <span class="faint truncate runsin-said">{under}</span>}
      </button>
      {(offered.change || offered.stop) && (
        <span class="runsin-acts">
          {offered.change && (
            <button type="button" class="ghost" aria-label={`Settings of ${instance.blueprint}`} onClick={() => void openSettings(instance, channel)}>
              Settings…
            </button>
          )}
          {offered.stop && (
            <button type="button" class="ghost" aria-label={`Stop ${instance.blueprint}`} onClick={() => setStopping(true)}>
              Stop…
            </button>
          )}
        </span>
      )}
      {stopping && <AskStop instance={instance} channel={channel} onClose={() => setStopping(false)} />}
    </div>
  );
}

/** What runs in `channel`, for whoever may read what is applied: nothing where nothing runs there. */
export function RunsIn({ channel }: { channel: Channel }) {
  const scopes = me.value?.principal.scopes ?? [];
  const reads = covers(scopes, 'blueprints:read');
  useEffect(() => {
    if (reads) void read(channel.id);
  }, [channel.id, reads]);
  const listed = running.value[channel.id] ?? [];
  useEvents(
    ['instance.started', 'instance.updated', 'instance.finished'],
    (event) => {
      if (!reads) return;
      const ids = (running.value[channel.id] ?? []).map((one) => one.id);
      if (heardOf(event.topic, event.data as Record<string, unknown> | undefined, event.source?.instance, ids)) void read(channel.id);
    },
    [channel.id, reads],
    () => reads && void read(channel.id),
  );
  if (!reads || listed.length === 0) return null;
  return (
    <section class="runsin" aria-label="What runs in this channel">
      <h3>Running here</h3>
      {listed.map((one) => (
        <Line key={one.id} instance={one} channel={channel} scopes={scopes} />
      ))}
    </section>
  );
}
