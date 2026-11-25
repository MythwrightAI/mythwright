// Where a project keeps blueprints of its own: in the factory's place for
// them, which is in the factory's own data folder and never the
// project's, unless it says a folder of its repository is read too,
// before that, or that it keeps none (the daemon's
// `conductor::library::folders`). A project that keeps none has no fork
// and nothing is read in place of what ships; its fleet's blueprint is
// read whatever it says. A file of the factory's place is named by a path
// of the project's that begins `@blueprints/`, and is opened as any file
// of the project is.
//
// What a project says is read by its page (`/api/projects/<id>/blueprints`)
// and again when `project.blueprints` is said of it; it is changed by whoever the
// daemon lets take a folder up (`admin`, beside `projects:write`), from
// the gear of the project's page, which also moves the blueprints of a
// folder of its repository into the factory's place. Functions of plain
// values, with their cases in `blueprinted.test.ts`, and the requests.

import { call } from './api.ts';
import { covers } from './holds.ts';

/** Where a project keeps its own blueprints, as the daemon hands it: `folder` is the folder of its repository it said, where it said one. */
export interface KeptIn {
  said: 'usual' | 'folder' | 'none';
  folder: string | null;
}

/** Where the factory keeps a project's own, as a path of the project's. */
export const PLACE = '@blueprints';

/** Where `project` keeps its own, as the daemon says: what a daemon from before cannot say is the usual. */
export function keptIn(answer: { blueprints?: KeptIn } | null | undefined): KeptIn {
  const kept = answer?.blueprints;
  return kept ? { said: kept.said, folder: kept.said === 'folder' ? kept.folder : null } : { said: 'usual', folder: null };
}

/** Reads where `project` keeps its own blueprints. */
export function readKept(project: string): Promise<{ blueprints: KeptIn }> {
  return call('GET', `/projects/${encodeURIComponent(project)}/blueprints`);
}

/** Whether whoever holds `scopes` may change where a project keeps its own blueprints: as the daemon asks it. */
export function mayKeep(scopes: string[]): boolean {
  return covers(scopes, 'admin') && covers(scopes, 'projects:write');
}

/** Where a project keeps its own blueprints, in a few words. */
export function keptSaid(kept: KeptIn): string {
  if (kept.said === 'none') return 'It keeps no blueprints of its own';
  if (kept.said === 'folder' && kept.folder) return `Its own blueprints are under ${kept.folder}/ of its repository, and kept by the factory`;
  return 'Its own blueprints are kept by the factory';
}

/** What may be asked of where a project keeps its own: the factory's place, a folder of its repository too, none, or the blueprints of a folder moved in. */
export type Asked = { usual: true } | { folder: string } | { none: true } | { move_in: true; from?: string };

/** The lines a project's gear offers for it, each with what pressing it asks; the one that is how things are is chosen, and pressing it asks nothing. */
export function keptLines(kept: KeptIn): { label: string; chosen: boolean; asks: Asked | 'folder' }[] {
  const lines: { label: string; chosen: boolean; asks: Asked | 'folder' }[] = [{ label: 'Kept by the factory', chosen: kept.said === 'usual', asks: { usual: true } }];
  if (kept.said === 'folder' && kept.folder) lines.push({ label: `Also under ${kept.folder}/`, chosen: true, asks: { folder: kept.folder } });
  lines.push({ label: 'Also a repo folder…', chosen: false, asks: 'folder' }, { label: 'None of its own', chosen: kept.said === 'none', asks: { none: true } });
  if (kept.said === 'folder' && kept.folder) lines.push({ label: 'Move them in', chosen: false, asks: { move_in: true } });
  return lines;
}

/** A folder as it is written in the dialog, made what the daemon is sent: no slash at its end. One that is not within the project is the daemon's to refuse. */
export function folderAsked(written: string): Asked | null {
  const folder = written.trim().replace(/\/+$/, '');
  return folder === '' ? null : { folder };
}

/** Asks the daemon that the project keep its own blueprints as `asked`; what it refuses is said in its own sentence. */
export function keepIn(project: string, asked: Asked): Promise<{ blueprints: KeptIn; changed?: boolean; moved?: { from: string; to: string }[] }> {
  return call('POST', `/projects/${encodeURIComponent(project)}/blueprints`, asked);
}
