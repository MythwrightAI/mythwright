// What is attached to a record, a card included: the Attachments tab of
// a card's drawer and of a row's form (`cardtabs.tsx`, `journey.tsx`).
// Each with who added it and when, opened in place where the page shows
// it (a picture drawn, a video played, words read) and otherwise saved;
// a file dropped on the tab, or a picture pasted into it, is added, by
// whoever may change the record, and one taken away goes into the Trash,
// with Undo. What is worked out is
// `ui/src/recordfiles.ts`, with its cases.

import { useEffect, useRef, useState } from 'preact/hooks';
import { call, shown, upload } from '../api.ts';
import { isAbout, refusedBefore, shownAs, sizeSaid } from '../recordfiles.ts';
import type { Attached } from '../recordfiles.ts';
import { attempt, toast } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { when } from '../util.ts';
import { pasted } from './attaching.tsx';
import { saidDeleted } from './trash.tsx';

/** What is attached to `record`, in the order it was added. */
export const attachedTo = (record: string) =>
  call<{ attachments: Attached[] }>('GET', `/records/${encodeURIComponent(record)}/attachments`).then((read) => read.attachments);

/** Attaches a file to `record`. */
export const attachTo = (record: string, file: File) => upload<Attached>(`/records/${encodeURIComponent(record)}/attachments`, file);

/** Takes something attached to `record` away. */
export const detachFrom = (record: string, id: string) =>
  call<{ trash?: string | null; kept?: number | null; attachment: Attached }>('DELETE', `/records/${encodeURIComponent(record)}/attachments/${encodeURIComponent(id)}`);

/** Where its file is fetched from, by the page or its browser. */
export const fileOf = (id: string) => shown(`/api/attachments/${encodeURIComponent(id)}`);

/** What is attached to `record`, read as the tab is drawn and again when something is added or taken away. */
export function useAttached(record: string): [Attached[] | null, () => void] {
  const [list, setList] = useState<Attached[] | null>(null);
  const asked = useRef(0);
  const read = () => {
    const mine = ++asked.current;
    attachedTo(record).then(
      (read) => mine === asked.current && setList(read),
      () => mine === asked.current && setList([]),
    );
  };
  useEffect(read, [record]);
  useEvents(['record.attached', 'record.detached'], (event) => isAbout(event, record) && read(), [record], read);
  return [list, read];
}

/** The tab: what is attached, and where whoever may change the record adds a file by dropping, pasting or choosing one. */
export function AttachedPanel({ record, list, may, read }: { record: string; list: Attached[] | null; may: boolean; read: () => void }) {
  const [open, setOpen] = useState<string | null>(null);
  const [over, setOver] = useState(false);
  const chooser = useRef<HTMLInputElement>(null);
  const add = async (files: File[]) => {
    for (const file of files) {
      const refused = refusedBefore(file);
      if (refused) {
        toast(refused, true);
        continue;
      }
      const kept = await attempt(() => attachTo(record, file));
      if (kept) setOpen(kept.id);
    }
    read();
  };
  return (
    <div
      class={`record-files${over ? ' over' : ''}`}
      aria-label="Attachments"
      onPaste={may ? (event) => pasted(event, add, (line) => line && toast(line)) : undefined}
      onDragOver={
        may
          ? (event) => {
              event.preventDefault();
              setOver(true);
            }
          : undefined
      }
      onDragLeave={() => setOver(false)}
      onDrop={
        may
          ? (event) => {
              event.preventDefault();
              setOver(false);
              void add([...(event.dataTransfer?.files ?? [])]);
            }
          : undefined
      }
    >
      {may && (
        // A place to paste into, which also takes the keyboard: a picture copied is pasted here.
        <div class="record-files-add" tabIndex={0} role="group" aria-label="Add a file">
          <span class="faint">Drop a file here, paste a picture, or</span>
          <button type="button" onClick={() => chooser.current?.click()}>
            Choose a file
          </button>
          <input
            ref={chooser}
            type="file"
            multiple
            hidden
            onChange={(event) => {
              const input = event.currentTarget;
              void add([...(input.files ?? [])]);
              input.value = '';
            }}
          />
        </div>
      )}
      {list === null && <p class="faint">Reading what is attached</p>}
      {list?.length === 0 && <p class="faint">Nothing is attached{may ? ' yet' : ''}.</p>}
      {list?.map((one) => (
        <section key={one.id} class="record-file" data-attachment={one.id}>
          <button type="button" class="link record-file-name truncate" aria-expanded={open === one.id} title={`Open ${one.name} here`} onClick={() => setOpen(open === one.id ? null : one.id)}>
            {one.name}
          </button>
          <div class="record-file-head">
            <span class="faint record-file-said">
              {sizeSaid(one.size)} · {one.who} · {when(one.at)}
            </span>
            <span class="grow" />
            <a class="record-file-save" href={fileOf(one.id)} download={one.name} title={`Save ${one.name}`}>
              Save
            </a>
            {may && (
              <button
                type="button"
                class="link danger record-file-remove"
                title={`Take ${one.name} away`}
                onClick={async () => {
                  const gone = await attempt(() => detachFrom(record, one.id));
                  if (gone === undefined) return;
                  // Into the Trash, with Undo, which gives it back whole.
                  saidDeleted(gone, one.name);
                  read();
                }}
              >
                Remove
              </button>
            )}
          </div>
          {open === one.id && <Opened one={one} />}
        </section>
      ))}
    </div>
  );
}

/** Something attached opened in place: drawn, played, read, or said to be saved. */
function Opened({ one }: { one: Attached }) {
  const shown = shownAs(one.media);
  const [words, setWords] = useState<string | null>(null);
  useEffect(() => {
    if (shown !== 'text') return;
    fetch(fileOf(one.id), { credentials: 'same-origin' }).then(
      (answer) => (answer.ok ? answer.text() : Promise.resolve(`It could not be read (${answer.status}).`)).then(setWords),
      () => setWords('It could not be read.'),
    );
  }, [one.id, shown]);
  if (shown === 'picture') return <img class="record-file-shown" src={fileOf(one.id)} alt={one.name} />;
  if (shown === 'video') return <video class="record-file-shown" src={fileOf(one.id)} controls preload="metadata" />;
  if (shown === 'text') return <pre class="record-file-words">{words ?? 'Reading it'}</pre>;
  return <p class="faint record-file-shown">Nothing here shows what {one.name} is: save it to open it.</p>;
}
