// A topic's documents as its page draws them: its post (the article) and
// its summary, each read as it is by anybody, and written in the block
// editor a library's document is written in by whoever may change the topic
// once they press Edit (`DocField`); and the summary in a band of its own
// (`Summary`), drawn only where it says something or is being written,
// which whoever may change it takes away again by emptying it.
//
// How a document is read and written is here alone: the article
// (`topicarticle.tsx`) and the summary both draw a `DocField`. Which of
// them is open to be written is the page's (`Editing` in `topicediting.ts`,
// kept by its tab), so that Edit at its head opens them, and anything else
// may open the article at one of its headings (`TopicPage`'s `edit`).

import { useEffect, useRef, useState } from 'preact/hooks';
import { Modal, Prose } from '../ui.tsx';
import { classes } from '../util.ts';
import { slugsOf } from '../contents.ts';
import { Writer } from '../write/writer.tsx';
import type { Writer as Handle } from '../write/prosemirror.ts';
import { picturingOf } from './picturein.tsx';
import { openOf } from '../topicediting.ts';
import type { DocName, Editing } from '../topicediting.ts';
import { useKept, useKeptRef } from './returned.tsx';

/**
 * One of the topic's documents (its summary, its post): read as it is by
 * anybody, and written as blocks by whoever may change it once it is opened
 * (`open`), and while it says nothing, there being nothing to read, from
 * which typing in it opens it. What is
 * typed is kept by the tab until it is saved or left, and a document with
 * typing kept opens with it; the editor is given what is saved again only
 * where nothing is being typed and the saved text is another than what it
 * shows (somebody else changed it), since an editor handed a text anew
 * throws its cursor away. Save keeps it and returns to reading; Cancel
 * leaves it, asking first where something typed would go.
 */
export function DocField(props: {
  name: DocName;
  /** The topic, whose attachments its pictures are. */
  record: string;
  saved: string;
  editable: boolean;
  /** Whether it is open to be written, and the press that opened it, where that one puts the cursor in it; the heading it was opened at, by its slug. */
  open: boolean;
  focus?: number;
  at?: string | null;
  /** Opens it, or leaves it. */
  onOpen: (open: boolean) => void;
  /** What is said where it says nothing: beside Save to whoever may write it, where that is not nothing, and to anybody else in its place. */
  empty: string;
  onSave: (markdown: string) => Promise<boolean>;
  /** What a button that empties it is called, where it may be emptied at once: what was typed goes with it. */
  remove?: string;
  /** Left while it says nothing: what was begun to write it is let go of. */
  onLeave?: () => void;
  /** Where it is read, a small Edit beside it opens it, called so to whoever cannot see what it stands by. */
  editLabel?: string;
  /** Saved and closed, or left: the heading of the section the cursor was in, by its slug, to be read at. */
  onBack?: (section: string | null) => void;
}) {
  const [draft, setDraft] = useKept<string | null>(`draft:${props.name}`, null);
  const showing = useKeptRef(`showing:${props.name}`, draft ?? props.saved);
  const [made, setMade] = useKept(`made:${props.name}`, 0);
  const focused = useKeptRef<number | undefined>(`focused:${props.name}`, undefined);
  const [busy, setBusy] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const writing = props.editable && (props.open || draft !== null || !props.saved.trim());
  // Opened by a press not yet taken up: the cursor goes in it, or it is brought into view at the heading it was opened at.
  const fresh = writing && props.focus !== undefined && props.focus !== focused.current;
  useEffect(() => {
    if (draft === null && props.saved !== showing.current) {
      showing.current = props.saved;
      setMade((was) => was + 1);
    }
  }, [props.saved, draft]);
  // The editor, once it is made (a moment after it is drawn, and again whenever it is drawn anew); and a press not yet
  // taken up, which puts the cursor in it, at the start of the section it was opened at where one was said.
  const editor = useRef<Handle | null>(null);
  if (!writing) editor.current = null;
  const asked = useRef<{ at: string | null } | null>(null);
  const takeUp = () => {
    const handle = editor.current;
    const want = asked.current;
    if (!handle || !want) return;
    asked.current = null;
    const nth = want.at ? slugsOf(handle.headings()).indexOf(want.at) : -1;
    if (nth >= 0) handle.toSection(nth);
    else handle.focus();
  };
  useEffect(() => {
    if (!fresh) return;
    focused.current = props.focus;
    asked.current = { at: props.at?.trim() || null };
    takeUp();
  }, [fresh]);
  // Closed, it is read at the section the cursor was in: said to the page before it is.
  const back = () => {
    const handle = editor.current;
    const nth = handle ? handle.sectionOf() : -1;
    props.onBack?.(handle && nth >= 0 ? (slugsOf(handle.headings())[nth] ?? null) : null);
  };
  const close = () => {
    back();
    props.onOpen(false);
  };
  const save = async (closing: boolean) => {
    if (busy) return;
    if (draft === null) return closing && close();
    setBusy(true);
    const kept = await props.onSave(draft);
    setBusy(false);
    if (!kept) return;
    setDraft(null);
    if (closing) close();
  };
  const leave = () => {
    setLeaving(false);
    setDraft(null);
    showing.current = props.saved;
    setMade((was) => was + 1);
    close();
    if (!props.saved.trim()) props.onLeave?.();
  };
  const remove = async () => {
    if (busy) return;
    setBusy(true);
    const kept = await props.onSave('');
    setBusy(false);
    if (kept) setDraft(null);
  };
  const removing = props.remove && (
    <button type="button" class="ghost topic-doc-remove" disabled={busy} onClick={() => void remove()}>
      {props.remove}
    </button>
  );
  if (!writing) {
    return props.saved.trim() ? (
      <div class={classes('topic-doc-read', props.editable && props.editLabel && 'editable')} data-field={props.name}>
        {props.editable && props.editLabel && (
          <button type="button" class="ghost topic-doc-edit" aria-label={props.editLabel} onClick={() => props.onOpen(true)}>
            Edit
          </button>
        )}
        {/* The post's sources are its article's References, listed at its end. */}
        <Prose text={props.saved} attached={props.record} notes={props.name === 'body' ? 'apart' : 'here'} />
      </div>
    ) : (
      <p class="faint topic-doc-empty">{props.empty}</p>
    );
  }
  // Left with nothing typed and nothing begun, a document that says nothing has nothing to go back to.
  const leavable = props.open || draft !== null || (!props.saved.trim() && props.onLeave !== undefined);
  return (
    <div class={classes('topic-doc', draft !== null && 'unsaved')} data-field={props.name}>
      <Writer
        key={made}
        markdown={draft ?? props.saved}
        elsewhere
        picturing={picturingOf(props.record)}
        linking={{ page: props.record }}
        onChange={(markdown) => {
          showing.current = markdown;
          setDraft(markdown === props.saved ? null : markdown);
          // Written in while it said nothing, it stays open as what is typed is saved, until it is saved or left.
          if (!props.open && markdown !== props.saved) props.onOpen(true);
        }}
        onSave={() => void save(false)}
        onReady={(handle) => {
          editor.current = handle;
          takeUp();
        }}
      />
      <div class="topic-doc-acts">
        {draft !== null ? <span class="faint">Not saved yet</span> : !props.saved.trim() && props.empty && <span class="faint">{props.empty}</span>}
        {removing}
        {leavable && (
          <button type="button" disabled={busy} onClick={() => (draft !== null ? setLeaving(true) : leave())}>
            Cancel
          </button>
        )}
        {(draft !== null || props.open) && (
          <button type="button" class="primary" disabled={busy} onClick={() => void save(true)}>
            {busy ? 'Saving…' : 'Save'}
          </button>
        )}
      </div>
      {leaving && (
        <Modal onClose={() => setLeaving(false)}>
          <h2>Discard your changes?</h2>
          <p style="margin:0 0 12px">What you typed here is not saved, and goes if you leave it.</p>
          <div class="row" style="justify-content:flex-end">
            <button type="button" onClick={() => setLeaving(false)}>
              Keep editing
            </button>
            <button type="button" class="primary" onClick={leave}>
              Discard
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}

/**
 * What is known now, in a band of its own: drawn by the page only where it
 * says something or whoever may change it pressed Add summary. Saved empty,
 * removed, or left before anything is saved in it, it goes (`onGone`), and
 * the head offers it again. Where it is read, whoever may change it opens
 * it by the small Edit beside it, or with the article by Edit at the head.
 */
export function Summary(props: {
  record: string;
  saved: string;
  editable: boolean;
  editing: Editing;
  onEditing: (open: boolean) => void;
  onSave: (summary: string) => Promise<boolean>;
  onGone: () => void;
}) {
  // What says nothing is kept as nothing, and not as the empty line an emptied editor gives.
  const save = async (summary: string) => {
    const said = summary.trim() ? summary : '';
    const kept = await props.onSave(said);
    if (kept && !said) props.onGone();
    return kept;
  };
  return (
    <section class="topic-summary" aria-label="Summary">
      <h2 class="topic-part-label">Summary</h2>
      <DocField
        name="summary"
        record={props.record}
        saved={props.saved}
        editable={props.editable}
        {...openOf(props.editing, 'summary')}
        onOpen={props.onEditing}
        empty="Sum up what is known now, in a few lines."
        onSave={save}
        remove="Remove summary"
        onLeave={props.onGone}
        editLabel="Edit summary"
      />
    </section>
  );
}
