// Where a picture a person puts in a topic's text is kept: attached to the
// topic, which is a record (`POST /api/records/<id>/attachments`), and so
// read by whoever may read the topic and nobody else. The text names it by
// the attachment's id (`attachment:<id>`, `pictures.ts`), which neither a
// revision nor a new title changes. The post's editor takes a picture
// pasted, dropped or chosen (`write/pictured.ts`); a reply's box does the
// same here, putting each on a line of its own where the caret is.

import { useRef, useState } from 'preact/hooks';
import { upload } from '../api.ts';
import { ATTACHED, captionOf, CHOSEN, mediaOf, takenIn, withPictures } from '../pictures.ts';
import { refusedBefore } from '../recordfiles.ts';
import { attempt, toast } from '../store.ts';
import type { Picturing } from '../write/pictured.ts';
import { Clip, pasted } from './attaching.tsx';

/** What a file attached to a record is, as the daemon hands it back. */
interface Attached {
  id: string;
  name: string;
  media: string;
}

/** Attaches a file to `record`: what it is, or `null` where it was refused, which is said. */
async function attachTo(record: string, file: File): Promise<Attached | null> {
  const refused = refusedBefore(file);
  if (refused) {
    toast(refused, true);
    return null;
  }
  return (await attempt(() => upload<Attached>(`/records/${encodeURIComponent(record)}/attachments`, file))) ?? null;
}

/** How the editor of a text of `record` takes pictures. */
export const picturingOf = (record: string): Picturing => ({ record, attach: (files) => attachedAll(record, files) });

/** Those of `files` that are put in a text, attached to `record`; any other is said not to be. */
async function attachedAll(record: string, files: File[]): Promise<Attached[]> {
  const taken: Attached[] = [];
  for (const file of files) {
    if (!takenIn(mediaOf(file))) {
      toast(`${file.name || 'That file'} is not a picture or a page, so it is not put in the text.`, true);
      continue;
    }
    const one = await attachTo(record, file);
    if (one) taken.push(one);
  }
  return taken;
}

/**
 * What lets a box of plain text (a reply's) take pictures: pasted or
 * dropped on it, or chosen with its button, each is attached to `record`
 * and written in where the caret is. `text` is what the box holds now.
 */
export function usePicturesIn(record: string, text: string, onText: (text: string) => void) {
  const held = useRef<HTMLElement>(null);
  const latest = useRef(text);
  latest.current = text;
  const [arriving, setArriving] = useState(0);
  const take = async (files: File[]) => {
    if (files.length === 0) return;
    setArriving((now) => now + 1);
    const taken = await attachedAll(record, files);
    setArriving((now) => now - 1);
    if (taken.length === 0) return;
    const area = held.current?.querySelector('textarea');
    const at = area ? area.selectionEnd : latest.current.length;
    const done = withPictures(
      latest.current,
      at,
      taken.map((one) => ({ caption: captionOf(one.name, one.media), to: `${ATTACHED}${one.id}`, place: null })),
    );
    onText(done.text);
    if (!area) return;
    // Put in at once with the caret after them, as a name chosen is (`TagBox`).
    area.value = done.text;
    area.setSelectionRange(done.caret, done.caret);
    area.focus();
  };
  const chooser = useRef<HTMLInputElement>(null);
  return {
    held,
    arriving: arriving > 0,
    onPaste: (event: ClipboardEvent) => pasted(event, take, (line) => line && toast(line, true)),
    onDragOver: (event: DragEvent) => {
      if (event.dataTransfer?.types.includes('Files')) event.preventDefault();
    },
    onDrop: (event: DragEvent) => {
      const files = [...(event.dataTransfer?.files ?? [])];
      if (files.length === 0) return;
      event.preventDefault();
      void take(files);
    },
    /** The button that chooses a picture, an SVG or a page from the device, and the chooser it opens. */
    chooseButton: (
      <>
        <input
          ref={chooser}
          type="file"
          hidden
          multiple
          accept={CHOSEN}
          onChange={(event) => {
            const files = [...(event.currentTarget.files ?? [])];
            event.currentTarget.value = '';
            void take(files);
          }}
        />
        <button type="button" class="ghost topic-reply-picture" title="Put in a picture, a diagram as SVG, or a page" aria-label="Attach a picture or a page" onClick={() => chooser.current?.click()}>
          <Clip />
        </button>
      </>
    ),
  };
}
