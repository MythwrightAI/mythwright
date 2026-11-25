// A view where it is drawn, and the ways to look at it larger: Open larger,
// a dialog over the page with most of the window, and Full screen, the
// whole screen (`fullscreen.tsx`). The same wherever a view is drawn with
// room for a line under it: a channel's message (`attached.tsx`) and a
// topic's text (`pictured.tsx`). What else is under it, and what is said
// beside its name opened larger, is the place's own.

import type { ComponentChildren } from 'preact';
import { createPortal } from 'preact/compat';
import { useState } from 'preact/hooks';
import type { Attachment } from '../api.ts';
import { Modal } from '../ui.tsx';
import { fileSize } from '../util.ts';
import { FullBox, FullButton, useFullScreen } from './fullscreen.tsx';
import { ViewFrame } from './viewframe.tsx';

/**
 * A view in its box, which full screen fills, as tall as its page up to
 * `tallest`. Under it `foot`, given Open larger and Full screen to put
 * among its own; opened larger, beside its name, what `opened` gives,
 * which is handed the way to put the dialog away.
 */
export function ShownView({
  attachment,
  tallest,
  foot,
  opened,
}: {
  attachment: Attachment;
  tallest: number;
  foot: (offers: ComponentChildren) => ComponentChildren;
  opened: (close: () => void) => ComponentChildren;
}) {
  const full = useFullScreen();
  const [larger, setLarger] = useState(false);
  const close = () => setLarger(false);
  return (
    <>
      <FullBox full={full} class="view-box">
        <ViewFrame attachment={attachment} tallest={tallest} fill={full.on} />
      </FullBox>
      {foot(
        <>
          <button class="ghost link" onClick={() => setLarger(true)}>
            Open larger
          </button>
          <FullButton full={full} class="ghost link" />
        </>,
      )}
      {larger && (
        <ViewLarger attachment={attachment} onClose={close}>
          {opened(close)}
        </ViewLarger>
      )}
    </>
  );
}

/**
 * A view opened larger, over the page: most of the window's height, its
 * name and size under it, Full screen, and `children`. On the page's body,
 * since an article holds what is fixed in it to itself (a container).
 */
export function ViewLarger({ attachment, onClose, children }: { attachment: Attachment; onClose: () => void; children?: ComponentChildren }) {
  const full = useFullScreen();
  return createPortal(
    <Modal wide onClose={onClose}>
      <div class="picture-opened">
        <FullBox full={full} class="view-box">
          <ViewFrame attachment={attachment} tallest={Math.round(window.innerHeight * 0.7)} fill={full.on} />
        </FullBox>
        <div class="row">
          <strong class="truncate">{attachment.name}</strong>
          {attachment.size > 0 && <span class="faint">{fileSize(attachment.size)}</span>}
          <span class="grow" />
          <FullButton full={full} class="ghost link" />
          {children}
        </div>
      </div>
    </Modal>,
    document.body,
  );
}
