// The factory's workspaces as the page has them: read once it is known who
// is signed in, and followed. What they are, and how the side panel's
// lists are worked out from them, is `workspaces.ts`.

import { signal } from '@preact/signals';
import { api, call } from './api.ts';
import { coalesce, stream } from './events.ts';
import { channels } from './store.ts';
import type { Belongs, Workspace } from './workspaces.ts';

const at = (id: string) => encodeURIComponent(id);

export const spaces = {
  list: () => call<{ workspaces: Workspace[] }>('GET', '/workspaces').then((read) => read.workspaces),
  create: (name: string) => call<Workspace>('POST', '/workspaces', { name }),
  rename: (id: string, name: string) => call<Workspace>('PATCH', `/workspaces/${at(id)}`, { name }),
  /** Takes one away: `to` is where its projects and its own channels go, which one that holds any is not taken away without. */
  remove: (id: string, to?: string) => call<{ id: string }>('DELETE', `/workspaces/${at(id)}`, to ? { to } : undefined),
  /** Puts a project in a workspace, out of the one it is in: it is always in one. */
  place: (project: string, workspace: string) => call<{ project: string }>('PUT', `/projects/${at(project)}/workspace`, { workspace }),
  /** Says what a channel belongs to. */
  belongs: (channel: string, to: Belongs) => call<{ channel: string }>('PUT', `/channels/${at(channel)}/belongs`, to),
};

/** The factory's workspaces, by name. Empty until they are read: a factory always has one. */
export const workspaces = signal<Workspace[]>([]);

/**
 * Whether what is there was read: until it is, and where it could not
 * be, an empty list says nothing of what there is. Whatever puts right
 * something kept about a workspace (which one somebody chose to look at)
 * waits for this, or every workspace would look gone as the page opens.
 */
export const spacesRead = signal(false);

/**
 * Whether a reading of them has been answered since somebody signed in,
 * however it went: until one is, nothing is known of whether there is
 * any. What has to know which workspace the panel is of before it shows
 * anything (the open tabs, which each workspace has its own of; what the
 * factory's pages are narrowed to) waits for this, and so not for ever
 * where they cannot be read.
 */
export const spacesAsked = signal(false);

/** Reads them, as whoever is signed in. Somebody who may not read projects has none to be shown. */
export async function readWorkspaces(): Promise<void> {
  try {
    workspaces.value = await spaces.list();
    spacesRead.value = true;
  } catch {
    spacesRead.value = false;
    workspaces.value = [];
  }
  spacesAsked.value = true;
}

/** Lets go of them, when nobody is signed in. */
export function forgetWorkspaces(): void {
  spacesRead.value = spacesAsked.value = false;
  workspaces.value = [];
}

// Several change at once (a workspace taken away says so once, but a
// project forgotten and its channels removed say so one by one): read
// once for them.
const changed = coalesce(() => void readWorkspaces());
stream.on(['workspace.created', 'workspace.renamed', 'workspace.removed', 'project.placed', 'channel.placed'], changed);
// Which project a channel is of is the channel's own, and what is listed
// under a project goes by it: read with the rest when it is said anew,
// whoever said it and wherever.
const replaced = coalesce(() => void api.channels().then((read) => (channels.value = read), () => undefined));
stream.on('channel.placed', replaced);
// What a workspace names may have gone.
stream.on(['project.removed', 'channel.removed'], () => {
  if (spacesRead.value) changed();
});
// And what was said while the page could not hear was not heard: by a
// daemon that came back as another program too, which has by then put
// in a workspace every project the one before it left in none.
stream.onStatus((up) => {
  if (up && spacesRead.value) changed();
});
