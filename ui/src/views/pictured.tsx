// A picture in what somebody wrote, drawn (`pictures.ts` reads it): a
// file attached to what the text is of (a topic), drawn at most as wide as
// the column, with its caption under it where it stands as a figure; an
// SVG the same, which is how a diagram is shown in this version; an
// attached page drawn in place as a channel draws a view, in the same box
// with the same rules (`viewframe.tsx`), and as a figure with Open larger
// and Full screen beside its caption (`viewshown.tsx`). A picture that
// cannot be read (it was removed, or is not the reader's to see) is a
// short line saying so, and one somewhere else is a link that is never
// fetched.
//
// What is attached to a record is read once for the page, whichever of its
// texts asks first, and again as something is attached to it or taken
// away (`record.attached`, `record.detached`): that is what tells a page
// from a picture. The editor draws a picture by the same function.

import { signal } from '@preact/signals';
import type { Signal } from '@preact/signals';
import { useState } from 'preact/hooks';
import { call, shown } from '../api.ts';
import type { Attachment } from '../api.ts';
import { stream } from '../events.ts';
import { drawnAs, elsewhereSaid, missingSaid, VIEW_MEDIA } from '../pictures.ts';
import type { Picture } from '../pictures.ts';
import { classes } from '../util.ts';
import { ViewFrame } from './viewframe.tsx';
import { ShownView } from './viewshown.tsx';

/** Something attached to a record, as much of it as drawing a picture needs. */
interface Listed {
  id: string;
  name: string;
  media: string;
  size: number;
}

/** How tall a view in a text may be before it scrolls. */
const VIEW_TALLEST = 480;

/** What is attached to each record a text asked about: `null` until it is read, and nothing where it may not be read. */
const listed = new Map<string, Signal<Listed[] | null>>();
let following = false;

function readListed(record: string): void {
  const kept = listed.get(record);
  if (!kept) return;
  call<{ attachments: Listed[] }>('GET', `/records/${encodeURIComponent(record)}/attachments`).then(
    (read) => (kept.value = read.attachments),
    () => (kept.value = []),
  );
}

/** What is attached to `record`, read the first time a text of it is drawn, and followed from then on. */
function listedOf(record: string): Signal<Listed[] | null> {
  if (!following) {
    following = true;
    stream.on(
      ['record.attached', 'record.detached'],
      (event) => {
        const of = event.data['id'];
        if (typeof of === 'string' && listed.has(of)) readListed(of);
      },
      // Where the stream may have missed some, each is read again.
      () => listed.forEach((_, record) => readListed(record)),
    );
  }
  let kept = listed.get(record);
  if (!kept) {
    kept = signal<Listed[] | null>(null);
    listed.set(record, kept);
    readListed(record);
  }
  return kept;
}

/** Where the file of an attachment is fetched from, by the page or its browser. */
export const pictureFile = (id: string) => shown(`/api/attachments/${encodeURIComponent(id)}`);

/**
 * A picture where it is written: as a figure of its own (`figure`), with
 * its caption under it and standing where it said, or in its line among
 * words. `record` is what the text is of, whose attachments it may name;
 * `still` is for the editor, where pressing it chooses it and opens nothing.
 */
export function Pictured({ picture, record, figure, still }: { picture: Picture; record: string | null; figure: boolean; still?: boolean }) {
  const attached = record ? listedOf(record).value : null;
  const drawn = drawnAs(picture, attached);
  // Which address could not be read: a picture led elsewhere since is tried anew.
  const [failed, setFailed] = useState<string | null>(null);
  // Only as a picture: one tried as a picture before the record's attachments were read may be a page.
  const missing = failed === picture.to && drawn.as === 'picture';
  const caption = picture.caption.trim();
  let shownAs;
  if (drawn.as === 'written') return <span class="prose-picture-written">{drawn.text}</span>;
  if (drawn.as === 'link') {
    const said = elsewhereSaid(picture);
    shownAs = (
      <a class="picture-elsewhere" href={drawn.to} target="_blank" rel="noopener noreferrer" title={said.title}>
        {said.words}
      </a>
    );
  } else if (missing) {
    shownAs = (
      <span class="faint picture-missing" role="note">
        {missingSaid(picture)}
      </span>
    );
  } else if (drawn.as === 'view') {
    const one = attached?.find((it) => it.id === drawn.id);
    const page = { id: drawn.id, name: caption || one?.name || 'A page', media: VIEW_MEDIA, size: one?.size ?? 0 };
    // As a figure, read, the ways to look at it larger are under it, beside its caption.
    if (figure && !still) return <ViewFigure page={page} place={picture.place ?? 'across'} caption={caption} saved={one?.name ?? page.name} />;
    shownAs = <ViewFrame attachment={page} tallest={VIEW_TALLEST} />;
  } else if (drawn.as === 'file') {
    shownAs = (
      <a class="picture-file" href={pictureFile(drawn.id)} download={drawn.name} title="Save a copy of it">
        {caption || drawn.name}
      </a>
    );
  } else {
    const img = <img class="picture-drawn" src={pictureFile(drawn.id)} alt={caption} title={figure ? undefined : caption || undefined} loading="lazy" onError={() => setFailed(picture.to)} />;
    // Pressed, it opens whole in a tab of its own: a diagram too wide for the column is read there.
    shownAs = still ? (
      img
    ) : (
      <a class="picture-open" href={pictureFile(drawn.id)} target="_blank" rel="noopener noreferrer" title={caption ? `${caption}: open it whole` : 'Open it whole'}>
        {img}
      </a>
    );
  }
  if (!figure) return <span class={classes('prose-picture', `drawn-${drawn.as}`)}>{shownAs}</span>;
  // What could not be read is said in its line, and needs no caption under it.
  const captioned = caption && !missing && drawn.as !== 'link';
  return (
    <figure class={classes('prose-figure', `place-${picture.place ?? 'across'}`, `drawn-${drawn.as}`, missing && 'missing')} data-place={picture.place ?? 'across'}>
      {shownAs}
      {captioned && <figcaption>{caption}</figcaption>}
    </figure>
  );
}

/** A view standing as a figure: its box, and under it its caption with Open larger and Full screen beside it. */
function ViewFigure({ page, place, caption, saved }: { page: Attachment; place: string; caption: string; saved: string }) {
  const save = (
    <a href={pictureFile(page.id)} download={saved} title="Save a copy of it">
      Save a copy of it
    </a>
  );
  return (
    <figure class={classes('prose-figure', `place-${place}`, 'drawn-view', 'offers', caption && 'captioned')} data-place={place}>
      <ShownView
        attachment={page}
        tallest={VIEW_TALLEST}
        // Before the caption, which is a figure's last: the grid puts them on one line.
        foot={(offers) => (
          <>
            <div class="prose-view-acts">{offers}</div>
            {caption && <figcaption>{caption}</figcaption>}
          </>
        )}
        opened={() => save}
      />
    </figure>
  );
}
