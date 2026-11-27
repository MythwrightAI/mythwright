// What a library and a document are shared by: what Share opens on each.
//
// Share is drawn only where something is behind it. A library is read as
// pages by whoever is signed in and may read its project, and is written
// out as pages anything can serve by a blueprint that ships (`std/site`),
// which stays and writes it out again as its documents change. A document
// says whether it is among what is written out by a mark of its own
// (`public`, a line at the head of its file, which the daemon writes when
// the record is changed). A table has nothing of the kind: who reads it is
// said by its blueprint, and nothing here shares or unshares one.
//
// What is offered is read when Share is pressed, and is what there was
// then: nothing is followed for it while the page is only being looked at.

import { api, shown } from '../api.ts';
import type { Collection } from '../api.ts';
import { covers } from '../holds.ts';
import { folderOf, pagesOf } from '../library.ts';
import { attempt, me, openTab, toast } from '../store.ts';
import { copy, glyph, note } from './pagehead.tsx';
import type { Lines } from './pagehead.tsx';
import { signs } from './signs.tsx';

/** The blueprint that writes a library out, and what it does where its params say nothing. */
const WRITES = 'std/site';
const USUAL = { library: 'docs', to: 'site', publish: 'marked' };

const holds = (scope: string) => covers(me.value?.principal.scopes ?? [], scope);
const unfolded = (address: string) => window.open(address, '_blank', 'noopener');
const READ_BY = 'It is read by whoever is signed in here and may read this project.';

/** Whether a library's documents say of themselves that they are written out with it. */
export function marksPublic(library: Collection): boolean {
  return library.fields.some((field) => field.name === 'public' && field.kind === 'flag');
}

/** What a library is shared by, as things stand when it is asked. */
export async function libraryShare(project: string, library: Collection): Promise<Lines> {
  const pages = pagesOf(project, library.name);
  const running = await api.instances({ live: true, project }).catch(() => []);
  const writes = running.find((one) => one.blueprint === WRITES && String(one.params.library ?? USUAL.library) === library.name);
  const lines: Lines = [
    { label: 'Read as pages', mark: signs.out(), act: () => unfolded(shown(pages)) },
    { label: "Copy the pages' address", mark: signs.link(), act: () => void copy(`${location.origin}${pages}`, `Copied. ${READ_BY}`) },
  ];
  if (writes) {
    const [to, which] = [String(writes.params.to ?? USUAL.to), String(writes.params.publish ?? USUAL.publish) === 'all' ? 'every document' : 'the documents marked public'];
    lines.push(
      { label: 'What writes it out', mark: glyph('◈'), hint: `${to}/`, apart: true, act: () => openTab({ kind: 'instance', id: writes.id }) },
      note(`It is written out to ${to}/ in the project as pages anything can serve, ${which}, and again whenever one changes.`),
    );
    return lines;
  }
  const out = async () => {
    const applied = await attempt(() => api.apply({ project, blueprint: WRITES, params: { library: library.name }, wait: true }));
    if (!applied) return;
    if (applied.finished && applied.finished.status !== 'ok') toast(applied.finished.error ?? 'It could not be written out.', true);
    else toast(`Written out to ${USUAL.to}/ in the project, and again whenever a document changes.`);
  };
  lines.push(
    { label: 'Write it out as pages', mark: signs.sheets(), apart: true, closed: holds('blueprints:run') ? undefined : 'Writing it out is applying a blueprint, which takes blueprints:run.', act: () => void out() },
    note(`Writes the documents marked public to ${USUAL.to}/ in the project, as pages anything can serve, and again whenever one changes.`),
  );
  return lines;
}

/**
 * What a document is shared by: its mark, and its page. `file` is where
 * it is in its project; `dirty`, that what is being written in it is not
 * saved, while which the mark is not changed: the daemon writes the mark
 * into the file, under what is being written.
 */
export async function documentShare(project: string, library: Collection, record: string, file: string, dirty: boolean): Promise<Lines> {
  const folder = folderOf(library);
  const within = folder ? file.slice(folder.length + 1) : file;
  const page = `${pagesOf(project, library.name)}${within.split('/').map(encodeURIComponent).join('/')}`;
  const row = await api.record(record).catch(() => null);
  let on = row?.data.public === true;
  const closed = dirty ? 'Save what is written first: the mark is written into the file.' : holds('records:write') ? undefined : 'Changing a document takes records:write.';
  const mark = async () => {
    const next = await attempt(() => api.setRecord(record, { public: !on }));
    if (next) on = next.data.public === true;
  };
  return [
    { label: 'Public', mark: signs.globe(), on: () => on, closed, act: mark },
    note('A public document is written out when its library is, as a page anything can serve.'),
    { label: 'Open as a page', mark: signs.out(), apart: true, act: () => unfolded(shown(page)) },
    { label: "Copy the page's address", mark: signs.link(), act: () => void copy(`${location.origin}${page}`, `Copied. ${READ_BY}`) },
  ];
}
