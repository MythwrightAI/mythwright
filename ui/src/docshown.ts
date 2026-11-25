// A file of a project asked to be read beside a channel: pressed where it
// is attached to a message, it is opened in the Tools panel's Docs tab,
// read in place there (`views/channeldocs.tsx`), and added to the
// channel's documents where whoever pressed may write in the channel.

import { signal } from '@preact/signals';
import { contextFront, contextOpen, contextTabs, foldContext, setContextTabs, showContextTab } from './store.ts';

/** A file of a project to be read in the Docs tab, with the channel it was pressed in; none when none is. */
export interface DocAsked {
  channel: string;
  project: string;
  path: string;
  /** What it is called. */
  title: string;
}

/** What the Docs tab is to open next, until it has. */
export const docAsked = signal<DocAsked | null>(null);

/** Opens a file of a project in the Docs tab beside `channel`, which is given that tab where the reader had taken it away. */
export function showDoc(asked: DocAsked): void {
  docAsked.value = asked;
  contextFront.value = null;
  if (!contextTabs.value.includes('docs')) setContextTabs([...contextTabs.value, 'docs']);
  showContextTab('docs');
  if (!contextOpen.value) foldContext();
}
