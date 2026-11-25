// What is attached to a record, a card included (`studio::attaching`), as
// its Attachments tab reads, shows and follows it. This holds nothing of
// the page, so that its tests can read it; what reads and draws is
// `views/recordfiles.tsx`.

/** Something attached to a record, as the daemon hands it out. */
export interface Attached {
  id: string;
  record: string;
  name: string;
  media: string;
  size: number;
  /** Who added it, as the journal names them, and by name as they were then. */
  by: string;
  who: string;
  at: number;
}

/** How something attached is opened in place: drawn, played, read, or saved where nothing here shows it. */
export type Shown = 'picture' | 'video' | 'text' | 'saved';

/**
 * How what is of `media` is opened in place. Only what a browser draws or
 * plays without running anything is drawn or played; words are read; a
 * page, and anything else, is saved.
 */
export function shownAs(media: string): Shown {
  const kind = media.split(';')[0]!.trim().toLowerCase();
  if (['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif'].includes(kind)) return 'picture';
  if (['video/mp4', 'video/webm'].includes(kind)) return 'video';
  if (kind.startsWith('text/') && kind !== 'text/html') return 'text';
  if (['application/json', 'text/markdown'].includes(kind)) return 'text';
  return 'saved';
}

/** How big something is, in words. */
export function sizeSaid(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.ceil(bytes / 1024)} KB`;
  return bytes === 1 ? '1 byte' : `${bytes} bytes`;
}

/** What the tab is called: its name, saying how many where there are any, and nothing of a count not read yet. */
export function tabCalled(count: number | null): string {
  return count ? `Attachments (${count})` : 'Attachments';
}

/** The largest file the daemon takes (`talk::attachments::MAX_ATTACHMENT_BYTES`). */
export const MOST_BYTES = 32 * 1024 * 1024;

/** What is said of a file before it is sent, where it would be refused: too large, or nothing in it. */
export function refusedBefore(file: { name: string; size: number }): string | null {
  if (file.size === 0) return `There is nothing in ${file.name} to attach.`;
  if (file.size > MOST_BYTES) return `${file.name} is too large to attach: the most is 32 MB.`;
  return null;
}

/** Whether an event is about something attached to `record`, added or taken away. */
export function isAbout(event: { topic: string; data: Record<string, unknown> }, record: string): boolean {
  return (event.topic === 'record.attached' || event.topic === 'record.detached') && event.data['id'] === record;
}
