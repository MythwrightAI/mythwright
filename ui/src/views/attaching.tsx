// What is waiting to go with the next message, drawn over the box one
// writes in and outside it, and what a paste into that box comes to.
//
// The strip lies over the foot of what was said and takes no room of its
// own (`.composer-over` in `composer.css`): nothing the reader is looking
// at moves as it comes and goes, and what is written keeps its room
// whatever is attached. Under the channel the list it lies over is given
// as much more room at its foot while it is there, which moves nothing
// either and lets the last thing said be scrolled up clear of it.
//
// What a paste carries is read while the paste is being said, since a
// clipboard's items are gone after it, and is decided by `pasted.ts`. A
// picture of a kind no browser shows (what WSLg hands over from Windows
// may be a BMP or a TIFF) is drawn again here as a PNG before it goes, and
// goes as it came where it cannot be drawn. Nothing is fetched: a picture
// named only by an address somewhere else is said not to be there.

import { useLayoutEffect, useRef } from 'preact/hooks';
import type { Attachment } from '../api.ts';
import { api } from '../api.ts';
import { dataOf, drawnAgain, PICTURES, pasteOf, pasteSaid, pastedName } from '../pasted.ts';
import { fileSize } from '../util.ts';

export function Clip() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M10.6 4.2 5.5 9.3a1.6 1.6 0 0 0 2.3 2.3l5.4-5.4a3 3 0 0 0-4.3-4.3L3.4 7.4a4.4 4.4 0 0 0 6.2 6.2l4.1-4.1" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  );
}

/** What is waiting to go with the message: a picture as a small one of itself, anything else by its name, each with what takes it off. */
export function Attaching({ attached, arriving, onRemove }: { attached: Attachment[]; arriving: number; onRemove: (one: Attachment) => void }) {
  const strip = useRef<HTMLDivElement>(null);
  const shown = attached.length > 0 || arriving > 0;
  // How tall it is, said to what holds the composer, where the list it lies over gives itself as much room at its foot.
  useLayoutEffect(() => {
    const here = strip.current;
    const holder = here?.closest('.composer:not(.wide)')?.parentElement;
    if (!here || !holder) return;
    const told = new ResizeObserver(() => holder.style.setProperty('--waiting', `${Math.ceil(here.getBoundingClientRect().height) + 4}px`));
    told.observe(here);
    return () => {
      told.disconnect();
      holder.style.removeProperty('--waiting');
    };
  }, [shown]);
  if (!shown) return null;
  return (
    <div class="attaching" aria-label="Attached" ref={strip}>
      {attached.map((one) => {
        const drop = (
          <button class="ghost icon" aria-label={`Remove ${one.name}`} title="Do not attach it" onClick={() => onRemove(one)}>
            ×
          </button>
        );
        return PICTURES.includes(one.media) ? (
          <span key={one.id} class="attaching-picture" title={`${one.name} · ${fileSize(one.size)}`}>
            <img src={api.attachmentUrl(one.id)} alt={one.name} />
            {drop}
          </span>
        ) : (
          <span key={one.id} class="attaching-one">
            <Clip />
            <span class="truncate">{one.name}</span>
            <span class="faint">{fileSize(one.size)}</span>
            {drop}
          </span>
        );
      })}
      {arriving > 0 && <span class="faint">{arriving === 1 ? 'Attaching a file' : `Attaching ${arriving} files`}</span>}
    </div>
  );
}

/** A picture of a kind no browser shows, drawn again as a PNG; as it came where it cannot be. */
async function drawn(file: File): Promise<File> {
  if (!drawnAgain(file.type)) return file;
  try {
    const bitmap = await createImageBitmap(file);
    const canvas = document.createElement('canvas');
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const drawing = canvas.getContext('2d');
    if (!drawing) return file;
    drawing.drawImage(bitmap, 0, 0);
    bitmap.close();
    const blob = await new Promise<Blob | null>((done) => canvas.toBlob(done, 'image/png'));
    return blob ? new File([blob], file.name, { type: 'image/png' }) : file;
  } catch {
    return file;
  }
}

/** Each file as it is to go: drawn again where it must be, and called something worth reading. */
async function prepared(files: File[]): Promise<File[]> {
  const at = new Date();
  const taken: string[] = [];
  const ready: File[] = [];
  for (const file of files) {
    const one = await drawn(file);
    const name = pastedName(one.name, one.type, at, taken);
    taken.push(name);
    ready.push(name === one.name ? one : new File([one], name, { type: one.type, lastModified: one.lastModified }));
  }
  return ready;
}

/**
 * What a paste into the composer does: files and pictures are attached,
 * with no text put in beside them; anything else is the browser's to
 * put in, and the line under the box says what was not there to attach.
 */
export function pasted(event: ClipboardEvent, attach: (files: File[]) => Promise<void>, say: (line: string | null) => void) {
  const clipboard = event.clipboardData;
  if (!clipboard) return;
  const paste = pasteOf({
    files: [...clipboard.files],
    // Asked for now: what the items hold is gone once the paste has been said.
    items: [...clipboard.items].map((item) => ({ kind: item.kind, file: item.kind === 'file' ? item.getAsFile() : null })),
    html: clipboard.getData('text/html'),
    text: clipboard.getData('text/plain'),
  });
  say(pasteSaid(paste));
  if (paste.take === 'files') {
    event.preventDefault();
    void prepared(paste.files).then(attach);
  } else if (paste.take === 'inline') {
    // Decoded here and now, so that where none can be read the text is still the browser's to put in.
    const files = paste.sources.flatMap((src) => {
      const data = dataOf(src);
      return data ? [new File([data.bytes as BlobPart], '', { type: data.type })] : [];
    });
    if (files.length === 0) return;
    event.preventDefault();
    void prepared(files).then(attach);
  }
}
