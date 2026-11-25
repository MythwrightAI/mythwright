// What is attached to a message, where the message is drawn: pictures and
// pages shown (a page, a view, with the ways to look at it larger of
// `viewshown.tsx`), other files by name to be saved, and a file of a project
// posted as that file (`posted.ts`) as a chip that opens it in the Docs
// tab of the Tools panel beside the channel, read in place there and kept
// to hand for everybody in it (`docshown.ts`). A copy is offered to be
// added to a project, for whoever may write in the channel and write a
// project's files; once it is, the message names that file.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Attachment, Entry } from '../api.ts';
import { showDoc } from '../docshown.ts';
import { PICTURES } from '../pasted.ts';
import { chipOf, foldersOf, mayPlace, pathFor, placeAttachment, seenOf } from '../posted.ts';
import { workspaces } from '../spaces.ts';
import { channels, collections, me, projects } from '../store.ts';
import { Modal, useFocus } from '../ui.tsx';
import { fileSize } from '../util.ts';
import { holdsProject, spaceOfChannel } from '../workspaces.ts';
import { Clip } from './attaching.tsx';
import { signs } from './signs.tsx';
import { settle } from './viewframe.tsx';
import { ShownView } from './viewshown.tsx';

/** What a view is: an attachment that is a page, drawn where it is posted. */
const VIEW = 'text/html';
/** How tall a view may be in a message before it scrolls. Opened larger, it has most of the window; in full screen, all of the screen. */
const VIEW_TALLEST = 480;

/** What the daemon said, as a page says it: without the marks a terminal reads names by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** Who is signed in, as adding a copy to a project goes by. */
function writer() {
  const who = me.value;
  return who ? { actor: who.actor, scopes: who.principal.scopes, owner: who.account?.role === 'owner' } : null;
}

/** What is attached to a message of `channel`: pictures and pages shown, files by name, and a project's file as that file. */
export function Attached({ attachments, channel }: { attachments: Attachment[]; channel: string }) {
  const [opened, setOpened] = useState<Attachment | null>(null);
  const [placing, setPlacing] = useState<Attachment | null>(null);
  const known = channels.value.find((one) => one.id === channel);
  const places = !!known && mayPlace({ channel: known, by: writer() });
  const add = (one: Attachment) => (
    <button class="ghost link attached-add" onClick={() => setPlacing(one)} title="Write it among a project's files, so that this message names that file">
      Add to project…
    </button>
  );
  return (
    <div class="attached">
      {attachments.map((one) =>
        one.file ? (
          <FileChip key={one.id} one={one} channel={channel} />
        ) : one.media === VIEW ? (
          <div key={one.id} class="attached-view">
            <ShownView
              attachment={one}
              tallest={VIEW_TALLEST}
              foot={(offers) => (
                <div class="attached-view-foot">
                  <Clip />
                  <span class="truncate">{one.name}</span>
                  <span class="grow" />
                  {offers}
                  {places && add(one)}
                  <a href={api.attachmentUrl(one.id)} download={one.name}>
                    Save a copy
                  </a>
                </div>
              )}
              opened={(close) => (
                <>
                  {places && (
                    <button
                      class="ghost link"
                      onClick={() => {
                        setPlacing(one);
                        close();
                      }}
                    >
                      Add to project…
                    </button>
                  )}
                  <a href={api.attachmentUrl(one.id)} download={one.name}>
                    Save a copy of it
                  </a>
                </>
              )}
            />
          </div>
        ) : PICTURES.includes(one.media) ? (
          <button key={one.id} class="attached-picture" title={`${one.name} · ${fileSize(one.size)}`} onClick={() => setOpened(one)}>
            <img src={api.attachmentUrl(one.id)} alt={one.name} onLoad={(event) => settle(event.currentTarget, event.currentTarget.height)} />
          </button>
        ) : (
          <span key={one.id} class="attached-copy">
            <a class="attached-file" href={api.attachmentUrl(one.id)} download={one.name} title="Save a copy of it">
              <Clip />
              <span class="truncate">{one.name}</span>
              <span class="faint">{fileSize(one.size)}</span>
            </a>
            {places && add(one)}
          </span>
        ),
      )}
      {opened && (
        <Modal wide onClose={() => setOpened(null)}>
          <div class="picture-opened">
            <img src={api.attachmentUrl(opened.id)} alt={opened.name} />
            <div class="row">
              <strong class="truncate">{opened.name}</strong>
              <span class="faint">{fileSize(opened.size)}</span>
              <span class="grow" />
              {places && (
                <button
                  class="ghost link"
                  onClick={() => {
                    setPlacing(opened);
                    setOpened(null);
                  }}
                >
                  Add to project…
                </button>
              )}
              <a href={api.attachmentUrl(opened.id)} download={opened.name}>
                Save a copy of it
              </a>
            </div>
          </div>
        </Modal>
      )}
      {placing && known && <AddToProject one={placing} channel={known.id} project={known.project ?? null} onClose={() => setPlacing(null)} />}
    </div>
  );
}

/**
 * A file of a project posted as that file: its name, and under it where it
 * is and how big while it is there. Pressed, it is read in the Docs tab
 * beside the channel; one that went, or that is not the reader's to read,
 * says so and opens nothing. One that came by what was said of it is read
 * for this reader as it is first drawn.
 */
function FileChip({ one, channel }: { one: Attachment; channel: string }) {
  const [seen, setSeen] = useState<Attachment>(one);
  useEffect(() => {
    setSeen(one);
    if (one.file?.state) return;
    let stale = false;
    seenOf(one.id).then(
      (now) => !stale && setSeen(now),
      () => {},
    );
    return () => {
      stale = true;
    };
  }, [one.id, one.file?.project, one.file?.path, one.file?.state]);
  const chip = chipOf(seen);
  const file = seen.file;
  if (!chip || !file) return null;
  const words = (
    <span class="attached-posted-words">
      <span class="truncate">{chip.said}</span>
      <span class="faint truncate">{chip.note}</span>
    </span>
  );
  return chip.opens ? (
    <button
      class="attached-file attached-posted"
      data-file={`${file.project}/${file.path}`}
      title={`Read ${file.path} beside the channel`}
      onClick={() => showDoc({ channel, project: file.project, path: file.path, title: seen.name })}
    >
      {signs.sheets()}
      {words}
    </button>
  ) : (
    <span class={`attached-file attached-posted ${file.state ?? 'reading'}`} data-file={`${file.project}/${file.path}`}>
      {signs.sheets()}
      {words}
    </span>
  );
}

/**
 * Asks where among a project's files a copy goes: which project (the
 * channel's, or of a channel that is no project's, those of its
 * workspace), which of its folders or libraries, and what it is called;
 * then has the daemon write it there, which asks `projects:write`, and
 * the message name that file. What the daemon refuses is said in its
 * sentence, with the dialog staying.
 */
function AddToProject({ one, channel, project, onClose }: { one: Attachment; channel: string; project: string | null; onClose: () => void }) {
  const known = channels.value.find((entry) => entry.id === channel);
  const space = known ? spaceOfChannel(known, workspaces.value, projects.value) : null;
  const offered = useMemo(
    () => projects.value.filter((entry) => (project ? entry.id === project : !space || holdsProject({ kind: 'space', id: space.id }, entry.id, workspaces.value))),
    [project, space?.id],
  );
  const [which, setWhich] = useState(offered[0]?.id ?? '');
  const [tree, setTree] = useState<Entry[] | null>(null);
  // The first offered (a library's, where the project has one) until another is chosen.
  const [folder, setFolder] = useState<string | null>(null);
  const [name, setName] = useState(one.name);
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  useEffect(() => {
    if (!which) return;
    let stale = false;
    setTree(null);
    api.tree(which).then(
      (entries) => !stale && setTree(entries),
      () => !stale && setTree([]),
    );
    return () => {
      stale = true;
    };
  }, [which]);
  const libraries = (collections.value[which] ?? []).filter((entry) => entry.store === 'documents');
  const folders = tree ? foldersOf(tree, libraries) : [];
  useEffect(() => {
    if (tree && (folder === null || !folders.some((entry) => entry.path === folder))) setFolder(folders[0]?.path ?? '');
  }, [tree, which]);
  const path = folder === null ? null : pathFor(folder, name);
  const add = async (event: Event) => {
    event.preventDefault();
    if (!which || !path) return;
    setBusy(true);
    setWrong(null);
    try {
      await placeAttachment(one.id, which, path);
      onClose();
    } catch (refused) {
      setWrong(said(refused));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal onClose={onClose}>
      <h2>Add {one.name} to a project</h2>
      <form class="stack attached-placing" onSubmit={add}>
        {offered.length === 0 ? (
          <p class="faint">There is no project here to add it to.</p>
        ) : (
          <>
            {offered.length > 1 && (
              <label>
                Project
                <select value={which} onChange={(event) => setWhich(event.currentTarget.value)}>
                  {offered.map((entry) => (
                    <option key={entry.id} value={entry.id}>
                      {entry.name}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <label>
              Folder
              <select value={folder ?? ''} disabled={!tree} onChange={(event) => setFolder(event.currentTarget.value)}>
                {!tree && <option value="">Reading its folders</option>}
                {folders.map((entry) => (
                  <option key={entry.path} value={entry.path}>
                    {entry.said}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Name
              <input
                ref={input}
                value={name}
                onInput={(event) => {
                  setName(event.currentTarget.value);
                  setWrong(null);
                }}
              />
            </label>
            <p class="faint attached-placing-says">
              {path
                ? `It is written as ${offered.find((entry) => entry.id === which)?.name ?? which}'s file ${path}, and this message names that file from then on, for everybody who reads it.`
                : 'A name is one file\'s name, with no / in it.'}
            </p>
          </>
        )}
        {wrong && (
          <p role="alert" class="attached-placing-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || !path || !which || !tree}>
            {busy ? 'Adding it' : 'Add it'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
