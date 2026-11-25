// The top of a channel's Channel tab in the Tools panel: its details,
// what it is called and what it says it is for (shown under its head),
// and who manages it; and, where a person in it is read about, making
// them one of its managers or no longer one. A channel's details are changed where they are read, and only by
// whoever may.
//
// Who may is the daemon's rule (`talk::managed`), worked out the same way
// here (`managed.ts`), so that a field is drawn only for whoever the
// daemon would let change it and anybody else reads the details as words.
// Each field is said when Enter is pressed or it is left, and only where
// it is something else than the channel has and than was said last, so
// that the two ask once; what the daemon refuses is said under it in its
// sentence, with what was written left as it was. Nothing is changed here
// when the daemon answers: the channels are read again on what is said of
// it (`channel.renamed`, `channel.described`, `channel.managers` in the
// store), on this page as on any other.

import { useEffect, useRef, useState } from 'preact/hooks';
import type { Channel, Member } from '../api.ts';
import { mayArchive } from '../archived.ts';
import { calledOf, hashed } from '../kept.ts';
import { describeChannel, describing, managedSaid, managerLine, manages, MOST_TOPIC, setManager } from '../managed.ts';
import { MOST, renameChannel, renaming } from '../renamed.ts';
import { me } from '../store.ts';

/** What the daemon said, as a page says it: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** Who reads this, as the rule goes by. */
function asking() {
  const who = me.value;
  return who ? { actor: who.actor, scopes: who.principal.scopes } : null;
}

/** Whether whoever reads this may change `channel`'s details as things stand: of those who may write in it, a manager, and of one archived nobody. */
export function mayChangeDetails(channel: Channel): boolean {
  const by = asking();
  return !!by && !channel.archived && mayArchive(channel, by) && manages(channel, by);
}

/** The part of the Channel tab that says, and for whoever may changes, what a channel is called and is for. */
export function ChannelDetails({ channel }: { channel: Channel }) {
  const may = mayChangeDetails(channel);
  const topic = channel.topic.trim();
  return (
    <section class="channel-details" aria-label="Details of this channel">
      <h3>Details</h3>
      {may ? (
        <>
          <DetailField
            key={`name:${channel.id}`}
            id={`channel-name-${channel.id}`}
            label={channel.kept ? 'Name' : 'Channel name'}
            now={calledOf(channel)}
            most={channel.kept ? MOST : undefined}
            check={(typed) => {
              const asked = renaming(channel, typed);
              return { to: asked.called, changes: asked.changes, says: asked.says };
            }}
            save={(to) => renameChannel(channel.id, to)}
          />
          <DetailField
            key={`topic:${channel.id}`}
            id={`channel-topic-${channel.id}`}
            label="Description"
            now={channel.topic}
            most={MOST_TOPIC}
            placeholder="What it is for"
            check={(typed) => {
              const asked = describing(channel.topic, typed);
              return { to: asked.topic, changes: asked.changes, says: asked.says };
            }}
            save={(to) => describeChannel(channel.id, to)}
          />
        </>
      ) : (
        <dl class="channel-details-words">
          <dt>{channel.kept ? 'Name' : 'Channel name'}</dt>
          <dd class="channel-details-name">{hashed(channel)}</dd>
          <dt>Description</dt>
          <dd class="channel-details-topic">{topic ? topic : <span class="faint">It says nothing of what it is for.</span>}</dd>
        </dl>
      )}
      <p class="faint channel-managed">
        {managedSaid(channel, me.value?.actor ?? null)}
        {channel.archived && manages(channel, asking() ?? { actor: '', scopes: [] }) ? ' It is archived: give it back to change it.' : ''}
      </p>
    </section>
  );
}

interface Checked {
  /** What the channel would have, or nothing where what is typed comes to nothing it takes. */
  to: string | null;
  changes: boolean;
  /** What is said under the field, where anything is. */
  says: string | null;
}

/** One detail written where it is read, said on Enter or as the field is left, once for both. */
function DetailField(props: {
  id: string;
  label: string;
  now: string;
  most?: number;
  placeholder?: string;
  check: (typed: string) => Checked;
  save: (to: string) => Promise<unknown>;
}) {
  const [typed, setTyped] = useState(props.now);
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const field = useRef<HTMLInputElement>(null);
  // What was last asked for, answered or still on its way.
  const asked = useRef<string | null>(null);
  // Changed from elsewhere (a terminal, another page): the field says so, unless it is being written in.
  useEffect(() => {
    if (document.activeElement !== field.current) setTyped(props.now);
    asked.current = null;
  }, [props.now]);
  const now = props.check(typed);
  const save = async () => {
    if (now.to === null || !now.changes || now.to === asked.current) return;
    const wanted = now.to;
    asked.current = wanted;
    setBusy(true);
    setWrong(null);
    try {
      await props.save(wanted);
      setTyped(wanted);
    } catch (refused) {
      asked.current = null;
      setWrong(said(refused));
    } finally {
      setBusy(false);
    }
  };
  const under = wrong ?? now.says;
  return (
    <form
      class="channel-detail"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <label for={props.id}>{props.label}</label>
      <input
        ref={field}
        id={props.id}
        value={typed}
        maxLength={props.most}
        placeholder={props.placeholder}
        aria-invalid={wrong ? 'true' : undefined}
        aria-busy={busy ? 'true' : undefined}
        aria-describedby={under ? `${props.id}-says` : undefined}
        onInput={(event) => {
          setTyped(event.currentTarget.value);
          setWrong(null);
        }}
        onBlur={() => void save()}
      />
      {under && (
        <p id={`${props.id}-says`} role={wrong ? 'alert' : undefined} class={wrong ? 'error-text channel-detail-says' : 'faint channel-detail-says'}>
          {under}
        </p>
      )}
    </form>
  );
}

/**
 * On a person's page among a channel's members: whether they manage it,
 * and for whoever manages it, making them one or no longer one, the last
 * manager staying (said why, and not pressed).
 */
export function ManagerLine({ channel, member }: { channel: Channel; member: Member }) {
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const by = asking();
  const line = by && !channel.archived && mayArchive(channel, by) ? managerLine(channel, member, by) : null;
  const theirs = channel.conversation || member.manages === true;
  const press = async () => {
    if (!line || line.closed || busy) return;
    setBusy(true);
    setWrong(null);
    try {
      await setManager(channel.id, member.actor, line.manages);
    } catch (refused) {
      setWrong(said(refused));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div class="channel-manager">
      <p class="muted">{theirs ? `Manages ${hashed(channel)}.` : `Does not manage ${hashed(channel)}.`}</p>
      {line && (
        <button type="button" class="channel-manager-act" aria-disabled={line.closed || busy ? 'true' : undefined} title={line.closed ?? undefined} onClick={() => void press()}>
          {line.label}
        </button>
      )}
      {line?.closed && <p class="faint">{line.closed}</p>}
      {wrong && (
        <p role="alert" class="error-text">
          {wrong}
        </p>
      )}
    </div>
  );
}
