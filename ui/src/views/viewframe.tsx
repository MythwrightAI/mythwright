// A view: a page somebody attached, drawn in a box of its own.
//
// It is drawn where its message is, in the channel, and again wherever
// else it is looked at: opened larger, in full screen, in the tools panel,
// or in a topic's text. The box and what it may do are the same in each.

import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Attachment } from '../api.ts';
import { viewColours } from '../viewcolours.ts';

/**
 * What a view may load, said to the browser inside the view's own page: its
 * own scripts and styles, the libraries served beside the client, pictures
 * it carries itself, and nothing over the network.
 */
function viewPolicy(): string {
  return [
    "default-src 'none'",
    `script-src 'unsafe-inline' ${location.origin}/lib/`,
    "style-src 'unsafe-inline'",
    'img-src data: blob:',
    'media-src data: blob:',
    'font-src data:',
    "form-action 'none'",
    "base-uri 'none'",
  ].join('; ');
}

/**
 * Keeps a channel at its foot when something in it has just grown by this
 * much: a view given its room, a picture that has arrived. Somebody reading
 * further up is left where they are.
 */
export function settle(element: HTMLElement | null, grew: number): void {
  const scroller = element?.closest('.messages');
  if (scroller && scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight <= grew + 80) scroller.scrollTop = scroller.scrollHeight;
}

/** The reader's colours, as a view is told them. */
function themeNow() {
  const style = getComputedStyle(document.documentElement);
  const colour = (name: string) => style.getPropertyValue(`--${name}`).trim();
  return {
    // Light where the page is, by the system or by whoever reads (`data-light`, `wearing.ts`).
    scheme: 'light' in document.documentElement.dataset ? 'light' : 'dark',
    colours: viewColours(colour),
  };
}

/**
 * A view: a page somebody attached, drawn in a box of its own. The box has
 * no origin, so the page has nobody's session and can read nothing of the
 * client; it runs scripts, and reaches nothing over the network. All it
 * can say to the client is how tall it is, and all it is told is the
 * reader's colours. With `fill`, it is as tall as the box it is in, not as
 * its page says (full screen); the frame stays the one it was, so its page
 * goes on as it was.
 */
export function ViewFrame({ attachment, tallest, fill = false }: { attachment: Attachment; tallest: number; fill?: boolean }) {
  // `undefined` while it is being read; `null` when it could not be.
  const [page, setPage] = useState<string | null | undefined>(undefined);
  const [height, setHeight] = useState(96);
  const frame = useRef<HTMLIFrameElement>(null);
  const was = useRef(96);
  useLayoutEffect(() => {
    if (fill) return;
    settle(frame.current, Math.max(0, height - was.current));
    was.current = height;
  }, [height, page, fill]);
  useEffect(() => {
    let stale = false;
    api.view(attachment.id).then(
      (html) => !stale && setPage(`<!doctype html><meta http-equiv="Content-Security-Policy" content="${viewPolicy()}">${html}`),
      () => !stale && setPage(null),
    );
    return () => {
      stale = true;
    };
  }, [attachment.id]);
  useEffect(() => {
    const tell = () => frame.current?.contentWindow?.postMessage({ theme: themeNow() }, '*');
    const hear = (event: MessageEvent) => {
      // Only this view's own page, saying one of the two things it may.
      if (event.source !== frame.current?.contentWindow || event.data?.view !== attachment.id) return;
      if (event.data.ready) tell();
      if (typeof event.data.height === 'number' && Number.isFinite(event.data.height)) {
        setHeight(Math.max(40, Math.min(Math.ceil(event.data.height), tallest)));
      }
    };
    const scheme = matchMedia('(prefers-color-scheme: light)');
    window.addEventListener('message', hear);
    scheme.addEventListener('change', tell);
    window.addEventListener('themechange', tell);
    return () => {
      window.removeEventListener('message', hear);
      scheme.removeEventListener('change', tell);
      window.removeEventListener('themechange', tell);
    };
  }, [attachment.id, tallest]);
  if (page === null) return <div class="faint view-missing">This page could not be shown.</div>;
  if (page === undefined) return <div class="faint view-missing">Reading {attachment.name}</div>;
  return <iframe ref={frame} class="view-frame" title={attachment.name} sandbox="allow-scripts" srcdoc={page} style={{ height: fill ? '100%' : height }} />;
}
