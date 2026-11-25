// A project's library: its documents, set out by the folder each is in.
//
// A document is a file, and is written in where files are: choosing one
// opens it there, as blocks or as the page it would be. What this page
// adds is the library itself: what there is, a new document, the library
// read as pages, and which documents something was said on: a mark with
// how many, which opens the document with its comments showing over its
// text. What was said is read there, on the document's own page.

import { Fragment } from 'preact';
import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { coalesce } from '../events.ts';
import { CHANGES, changes, fileOf, folderOf, isLibrary, pathOf, placeFor, shelves, titleOf } from '../library.ts';
import type { Kept } from '../library.ts';
import { agents, attempt, collections, openTab, projectName } from '../store.ts';
import type { Field as Column, Row } from '../table.ts';
import { columns as columnsOf, titled } from '../table.ts';
import { Empty, Field, Modal, useEvents, useFocus } from '../ui.tsx';
import { ago } from '../util.ts';
import { CommentMark, useCommented } from './comments.tsx';
import { Laid, ViewsHead, useViews } from './laid.tsx';
import { libraryShare } from './shared.tsx';
import { TableActs } from './tablehead.tsx';
import { Titled } from './pagehead.tsx';
import { Turn } from './table.tsx';
import { opensInto, Within } from './within.tsx';
import { useNamed } from './others.ts';
import { useKept } from './returned.tsx';
import { headWord, ThingWas } from './thingwas.tsx';

export function LibraryView({ project, name }: { project: string; name: string }) {
  const library = (collections.value[project] ?? []).find((one) => one.name === name && isLibrary(one));
  // `null` while they are first being read.
  // What was read is shown at once when its tab shows again, and read again behind.
  const [rows, setRows] = useKept<Row[] | null>(`rows:${project}/${name}`, null);
  const [adding, setAdding] = useState(false);
  const reload = useMemo(() => coalesce(() => void api.records(project, name).then(setRows, () => setRows([])), 120), [project, name]);
  useEffect(reload, [reload]);
  // Read once, and again when something says it may have changed: a
  // document written here, or a file put in its folder by anything.
  useEvents(CHANGES, (event) => library && changes(event, project, library) && reload(), [project, name, library?.at]);

  // How many things were said on each document: read once for the library, and again as something is said.
  const said = useCommented(project, name);
  // The documents that are opened into what names them, where anything does.
  const [opened, setOpened] = useKept<string[]>('library.opened', []);
  const deeper = opensInto(project, name);
  // What its documents name, where a field names records of another table.
  const others = useNamed(project, library?.fields ?? [], name);
  // Its own look is the list, by folder; whoever reads it saves other views
  // of the same documents beside that: a table, a gallery, a calendar.
  const views = useViews(project, library, name, 'list');
  if (!library) {
    return (
      <Empty title="There is no library here">
        A project has one once a blueprint makes it one: <code>std/docs</code> does, of the Markdown files in a folder of the project.
      </Empty>
    );
  }
  const folder = folderOf(library);
  const open = (row: Row) => openTab({ kind: 'file', project, path: fileOf(library, row) });
  // The same, with what was said on it asked for: all of it shown, and the cursor where a comment is added.
  const comment = (row: Row) => openTab({ kind: 'file', project, path: fileOf(library, row) }, { comments: true });
  // A document named in a table of them is a file where its title says, as one made with the button is.
  const write = async (title: string | null) => {
    const place = title ? placeFor(title, '') : null;
    if (!title || !place) return;
    const fields = library.fields.some((field) => field.name === 'path') ? { title: title.trim(), path: place } : { title: title.trim() };
    const made = await attempt(() => api.addRecord(project, library.name, fields));
    if (made) openTab({ kind: 'file', project, path: fileOf(library, made) });
  };
  const byFolder = (
    <div class="rows-page">
      {rows?.length === 0 && (
        <Empty title="No documents yet">
          Write the first one here, or put a Markdown file in {folder ? <code>{folder}/</code> : 'the project'}: it is in the library as soon as it is there.
        </Empty>
      )}
      {shelves(rows ?? []).map((shelf) => (
        <section key={shelf.folder} class="library-shelf" aria-label={shelf.folder || titled(name)}>
          {shelf.folder && <h2>{shelf.folder}/</h2>}
          <div class="rows-list">
            {shelf.rows.map((row) => (
              <Fragment key={row.id}>
              <div class="library-line">
                {deeper && (
                  <Turn
                    open={opened.includes(row.id)}
                    called={titleOf(row)}
                    onTurn={() => setOpened(opened.includes(row.id) ? opened.filter((one) => one !== row.id) : [...opened, row.id])}
                  />
                )}
                <button class="rows-line" title={`Open ${fileOf(library, row)}`} onClick={() => open(row)}>
                  <span class="truncate">{titleOf(row)}</span>
                  <span class="id faint truncate">{pathOf(row).split('/').pop()}</span>
                  <span class="grow" />
                  {row.updated > 0 && <span class="faint">{ago(row.updated)}</span>}
                </button>
                <CommentMark count={said[row.id] ?? 0} title={titleOf(row)} onOpen={() => comment(row)} />
              </div>
                {deeper && opened.includes(row.id) && (
                  <div class="rows-within">
                    <Within project={project} name={name} id={row.id} />
                  </div>
                )}
              </Fragment>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
  return (
    <div class="split">
      <div class="view-head">
        <Titled>
          <h1>
            {projectName(project)} {headWord(project, `library:${name}`, name)}
          </h1>
          <ThingWas project={project} thing={`library:${name}`} />
          {rows && (
            <span class="faint">
              {rows.length} {rows.length === 1 ? 'document' : 'documents'}
            </span>
          )}
          <span class="faint truncate" title="The Markdown files of this folder of the project, as they are">
            in {folder ? `${folder}/` : 'the project itself'}
          </span>
        </Titled>
        <span class="grow" />
        {/* The library as pages, and written out as pages, is what it is shared by. */}
        <TableActs tab={{ kind: 'library', project, name }} table={library} rows={rows} share={() => libraryShare(project, library)} />
        <button class="primary" onClick={() => setAdding(true)}>
          New document
        </button>
        <ViewsHead views={views} />
      </div>
      {rows === null ? (
        <div class="rows-page">
          <p class="faint">Reading</p>
        </div>
      ) : (
        <Laid
          views={views}
          own={{ list: () => byFolder }}
          fields={columnsOf(library.fields).filter((field) => field.name !== 'body')}
          rows={rows}
          onAdd={(title) => void write(title)}
          people={agents.value.map((slot) => ({ value: `agent:${slot.name}`, name: slot.name }))}
          named={{ ...others, [name]: { fields: library.fields, rows } }}
          what="document"
          onOpen={open}
          within={opensInto(project, name) ? (row) => <Within project={project} name={name} id={row.id} /> : undefined}
          onChange={async (row: Row, field: Column, value: unknown) => {
            await attempt(() => api.setRecord(row.id, { [field.name]: value }));
            reload();
          }}
        />
      )}
      {adding && <NewDocument project={project} library={library} canPlace={library.fields.some((field) => field.name === 'path')} onClose={() => setAdding(false)} />}
    </div>
  );
}

function NewDocument({ project, library, canPlace, onClose }: { project: string; library: Kept; canPlace: boolean; onClose: () => void }) {
  const [title, setTitle] = useState('');
  const [folder, setFolder] = useState('');
  const [busy, setBusy] = useState(false);
  const focus = useFocus<HTMLInputElement>();
  const place = placeFor(title, canPlace ? folder : '');
  const top = folderOf(library);
  const make = async (event: Event) => {
    event.preventDefault();
    if (!place || busy) return;
    setBusy(true);
    const fields = canPlace ? { title: title.trim(), path: place } : { title: title.trim() };
    const made = await attempt(() => api.addRecord(project, library.name, fields));
    setBusy(false);
    if (made) {
      onClose();
      openTab({ kind: 'file', project, path: fileOf(library, made) });
    }
  };
  return (
    <Modal onClose={onClose}>
      <form class="stack" onSubmit={make}>
        <h2>A new document</h2>
        <Field label="Title">
          <input ref={focus} value={title} aria-label="Title" onInput={(event) => setTitle(event.currentTarget.value)} />
        </Field>
        {canPlace && (
          <Field label="Folder, if it goes in one">
            <input value={folder} aria-label="Folder" placeholder="guides" onInput={(event) => setFolder(event.currentTarget.value)} />
          </Field>
        )}
        <p class="faint">{place ? `It is the file ${top ? `${top}/` : ''}${place}.md, and opens to be written in.` : 'Its file is named by its title.'}</p>
        <div class="row">
          <span class="grow" />
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" disabled={!place || busy}>
            {busy ? 'Making it' : 'Make it'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
