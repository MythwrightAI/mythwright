// A topic's history: each revision of what it says (its title, its article
// and its summary, whole each time, by whatever door it was changed), newest
// first, with who made it, when, and what it changed; one revision shown as
// it read then; two compared, part by part, with the words added and removed
// marked; and one restored by whoever may change the topic, which sets all
// three back through the door any record is changed by and so becomes a
// revision of its own. A third tab of an article, beside Article and Talk,
// and shown in a thread's place by History at its head (`topicpage.tsx`).
//
// It reads the revisions as it is drawn and again whenever the topic is
// written (`GET /api/topics/<topic>/revisions`); what it works out from them
// is in `revisions.ts`, and the difference between two texts in `worddiff.ts`.

import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { dateIn, worn } from '../clock.ts';
import { changedSaid, choosing, compared, doorSaid, historyOf, pair, partName, restoring, revisionOf } from '../revisions.ts';
import type { HistoryLine, HistoryOf, Revision } from '../revisions.ts';
import type { TopicWhole } from '../topicpage.ts';
import { topicsApi } from '../topicsread.ts';
import { Modal, Prose } from '../ui.tsx';
import { ago, classes, clock } from '../util.ts';
import { counted } from '../worddiff.ts';
import type { Piece } from '../worddiff.ts';
import { useKept } from './returned.tsx';

interface Props {
  topic: TopicWhole;
  mayChange: boolean;
  /** Whether it is a tab's panel beside Article and Talk, or shown in a thread's place. */
  tabbed: boolean;
  /** The revision shown, by its number; nothing for the list. */
  shown: number | null;
  onShow: (nth: number | null) => void;
  /** Sets the topic's fields, as its page sets them. */
  onRestore: (fields: Record<string, unknown>) => Promise<boolean>;
}

/** When a revision was made: how long ago, and the moment whole on hover. */
function When({ at }: { at: number }) {
  return (
    <time dateTime={new Date(at).toISOString()} title={exactly(at)}>
      {ago(at)}
    </time>
  );
}

/** Who made a revision, when, and by which door where it was not a person's own hand. */
function Made({ revision }: { revision: Revision }) {
  const door = doorSaid(revision);
  return (
    <>
      <span class="topic-history-who">{revision.who}</span>
      <span class="faint">
        {' · '}
        <When at={revision.at} />
        {door && <span title={revision.through ?? undefined}>{` · ${door}`}</span>}
      </span>
    </>
  );
}

/** The moment a revision was made, whole. */
const exactly = (at: number) => `${dateIn(at, worn.value.zone)}, ${clock(at)}`;

/** What a revision that says all an earlier one said is said to be. */
const restoredSaid = (restored: Revision) => `Restored from revision ${restored.nth}, ${exactly(restored.at)}`;

/** What changed, with what was added and removed marked, and said as such to whoever hears the page. */
function Pieces({ pieces }: { pieces: Piece[] }) {
  return (
    <div class="topic-diff">
      {pieces.map((piece, at) =>
        piece.kind === 'same' ? (
          piece.text
        ) : piece.kind === 'added' ? (
          <ins key={at} class="topic-diff-added">
            <span class="topic-diff-said">added: </span>
            {piece.text}
          </ins>
        ) : (
          <del key={at} class="topic-diff-removed">
            <span class="topic-diff-said">removed: </span>
            {piece.text}
          </del>
        ),
      )}
    </div>
  );
}

export function History(props: Props) {
  const { topic, mayChange, shown, onShow } = props;
  // Beside Article and Talk it is an article's; in a thread's place, what the thread says is its post.
  const of: HistoryOf = props.tabbed ? 'article' : 'thread';
  const [revisions, setRevisions] = useKept<Revision[] | null>('history:read', null);
  const [failed, setFailed] = useState<string | null>(null);
  const [chosen, setChosen] = useKept<number[]>('history:chosen', []);
  const [comparing, setComparing] = useKept<[number, number] | null>('history:comparing', null);
  const [asked, setAsked] = useState<Revision | null>(null);
  const [busy, setBusy] = useState(false);
  // Of the readings on their way, only the one asked for last is taken.
  const reading = useRef(0);
  useEffect(() => {
    const mine = ++reading.current;
    topicsApi
      .revisions(topic.id)
      .then((read) => {
        if (mine !== reading.current) return;
        setRevisions(read.map(revisionOf));
        setFailed(null);
      })
      .catch((error) => mine === reading.current && setFailed(error instanceof Error ? error.message : String(error)));
  }, [topic.id, topic.updated]);

  const panel = (children: ComponentChildren) =>
    props.tabbed ? (
      <div class="topic-history" role="tabpanel" id={`topic-history-${topic.id}`} aria-labelledby={`topic-tab-history-${topic.id}`}>
        {children}
      </div>
    ) : (
      <section class="topic-history" aria-label="History">
        {children}
      </section>
    );
  if (!revisions) return panel(<p class="faint topic-history-none">{failed ? `The history could not be read: ${failed}` : 'Reading the history'}</p>);

  const lines = historyOf(revisions);
  const byNth = (nth: number) => lines.find((line) => line.revision.nth === nth) ?? null;
  const before = (nth: number) => lines.find((line) => line.revision.nth < nth) ?? null;
  const back = (
    <button
      type="button"
      class="ghost topic-history-back"
      onClick={() => {
        setComparing(null);
        onShow(null);
      }}
    >
      ← All revisions
    </button>
  );
  const compare = (older: number, newer: number) => {
    setComparing([older, newer]);
    onShow(null);
  };
  const onShowing = (nth: number) => {
    setComparing(null);
    onShow(nth);
  };
  const restore = async () => {
    if (!asked || busy) return;
    setBusy(true);
    const kept = await props.onRestore(restoring(asked));
    setBusy(false);
    if (!kept) return;
    setAsked(null);
    setComparing(null);
    onShow(null);
  };
  const restoreAsked = asked && (
    <Modal onClose={() => setAsked(null)}>
      <h2>Restore this version?</h2>
      <p style="margin:0 0 12px">
        The title, article and summary go back to what they said in revision {asked.nth}, made by {asked.who} on {exactly(asked.at)}. What they say
        now stays in the history.
      </p>
      <div class="row" style="justify-content:flex-end">
        <button type="button" onClick={() => setAsked(null)}>
          Cancel
        </button>
        <button type="button" class="primary topic-history-restore-sure" disabled={busy} onClick={() => void restore()}>
          {busy ? 'Restoring…' : 'Restore'}
        </button>
      </div>
    </Modal>
  );

  const pairShown = comparing && byNth(comparing[0]) && byNth(comparing[1]) ? comparing : null;
  if (pairShown) {
    const older = byNth(pairShown[0])!.revision;
    const newer = byNth(pairShown[1])!.revision;
    const changes = compared(older, newer);
    return panel(
      <>
        <div class="topic-history-bar">{back}</div>
        <h2 class="topic-history-heading">
          Changes from revision {older.nth} to revision {newer.nth}
        </h2>
        <p class="topic-history-pair">
          <button type="button" class="link" onClick={() => onShowing(older.nth)}>
            Revision {older.nth}
          </button>{' '}
          <Made revision={older} />
          <br />
          <button type="button" class="link" onClick={() => onShowing(newer.nth)}>
            Revision {newer.nth}
          </button>{' '}
          <Made revision={newer} />
        </p>
        {changes.length === 0 ? (
          <p class="faint">They say the same.</p>
        ) : (
          <>
            <p class="faint topic-history-legend" aria-hidden="true">
              <ins class="topic-diff-added">Added</ins> <del class="topic-diff-removed">Removed</del>
            </p>
            {changes.map((change) => {
              const { added, removed } = counted(change.pieces);
              return (
                <section key={change.part} class="topic-history-change" aria-label={partName(change.part, of)} data-part={change.part}>
                  <h3 class="topic-part-label">
                    {partName(change.part, of)} <span class="topic-history-counted">{`+${added} −${removed} words`}</span>
                  </h3>
                  <Pieces pieces={change.pieces} />
                </section>
              );
            })}
          </>
        )}
      </>,
    );
  }

  const line = shown !== null ? byNth(shown) : null;
  if (shown !== null) {
    if (!line) return panel(<><div class="topic-history-bar">{back}</div><p class="faint topic-history-none">There is no revision {shown} of this page.</p></>);
    const { revision } = line;
    const earlier = before(revision.nth);
    return panel(
      <>
        <div class="topic-history-bar">{back}</div>
        <div class="topic-history-revision-head">
          <h2 class="topic-history-heading">
            Revision {revision.nth}
            {line.latest && <span class="faint"> (current)</span>}
          </h2>
          <p class="topic-history-made">
            <Made revision={revision} />
            {line.restored && <span class="faint">{` · ${restoredSaid(line.restored)}`}</span>}
          </p>
          <div class="topic-history-acts">
            {earlier && (
              <button type="button" onClick={() => compare(earlier.revision.nth, revision.nth)}>
                Compare with the one before
              </button>
            )}
            {mayChange && !line.latest && (
              <button type="button" class="primary topic-history-restore" onClick={() => setAsked(revision)}>
                Restore this version
              </button>
            )}
          </div>
        </div>
        <section class="topic-history-part" aria-label="Title">
          <h3 class="topic-part-label">Title</h3>
          <p class="topic-history-title">{revision.title}</p>
        </section>
        {revision.summary.trim() && (
          <section class="topic-history-part" aria-label="Summary">
            <h3 class="topic-part-label">Summary</h3>
            <div class="topic-doc-read">
              <Prose text={revision.summary} attached={topic.id} />
            </div>
          </section>
        )}
        <section class="topic-history-part" aria-label={partName('article', of)}>
          <h3 class="topic-part-label">{partName('article', of)}</h3>
          {revision.body.trim() ? (
            <div class="topic-doc-read">
              <Prose text={revision.body} attached={topic.id} />
            </div>
          ) : (
            <p class="faint">{`Nothing was written in the ${partName('article', of).toLowerCase()}.`}</p>
          )}
        </section>
        {restoreAsked}
      </>,
    );
  }

  const two = pair(chosen.filter((nth) => byNth(nth)));
  return panel(
    <>
      <div class="topic-history-bar">
        <h2 class="topic-part-label">
          {lines.length === 1 ? '1 revision' : `${lines.length} revisions`}
        </h2>
        <span class="grow" />
        {lines.length > 1 && (
          <>
            <span class="faint topic-history-hint">{two ? '' : 'Choose two to compare them.'}</span>
            <button type="button" class="topic-history-compare" disabled={!two} onClick={() => two && compare(two[0], two[1])}>
              Compare
            </button>
          </>
        )}
      </div>
      <ol class="topic-history-list">
        {lines.map((one) => (
          <Line key={one.revision.nth} line={one} alone={lines.length === 1} chosen={chosen.includes(one.revision.nth)} onChoose={() => setChosen((was) => choosing(was, one.revision.nth))} onOpen={() => onShowing(one.revision.nth)} of={of} />
        ))}
      </ol>
    </>,
  );
}

/** A revision in the list: chosen to compare by its box, where there is another, and opened by its line. */
function Line({ line, alone, chosen, onChoose, onOpen, of }: { line: HistoryLine; alone: boolean; chosen: boolean; onChoose: () => void; onOpen: () => void; of: HistoryOf }) {
  const { revision } = line;
  return (
    <li class={classes('topic-history-line', chosen && 'chosen')} data-nth={revision.nth}>
      {!alone && <input type="checkbox" class="topic-history-choose" aria-label={`Choose revision ${revision.nth} to compare`} checked={chosen} onChange={onChoose} />}
      <button type="button" class="topic-history-open" onClick={onOpen}>
        <span class="topic-history-nth">{revision.nth}</span>
        <span class="topic-history-said">
          <Made revision={revision} />
          <span class="topic-history-what">
            {changedSaid(line, of)}
            {line.restored && ` · ${restoredSaid(line.restored)}`}
            {line.latest && ' · Current'}
          </span>
        </span>
      </button>
    </li>
  );
}
