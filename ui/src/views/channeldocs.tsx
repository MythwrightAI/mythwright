// The Docs tab of the Tools panel beside a channel: the documents the
// channel keeps to hand, the same for everybody in it (`channeldocs.ts`).
//
// Each is a line that says what it is called and where it is, or what has
// become of it; pressed, it is read here in place, as a page is among the
// media, with the way back and the way to open it as a tab of its own.
// Whoever may write in the channel adds one from the libraries and files
// of the projects of the channel's workspace, in a list that opens in the
// panel and lies over nothing, and takes one away from its line. It is
// read once and again as the daemon says the list changed. A file of a
// project pressed where it is attached to a message is opened here
// (`docshown.ts`), and added to the list where whoever pressed may write
// in the channel; where they may not, it is read here and not added.

import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api, shown } from '../api.ts';
import type { Channel, Entry } from '../api.ts';
import { addDoc, lineOf, mayChange, offered, readDocs, removeDoc } from '../channeldocs.ts';
import { docAsked } from '../docshown.ts';
import type { ChannelDoc } from '../channeldocs.ts';
import { workspaces } from '../spaces.ts';
import { collections, me, openTab, projects } from '../store.ts';
import { useEvents, useFocus } from '../ui.tsx';
import { holdsProject, spaceOfChannel } from '../workspaces.ts';
import { PagePreview } from './file.tsx';

/** Who is signed in, as changing the list goes by. */
function writer() {
  const who = me.value;
  return who ? { actor: who.actor, scopes: who.principal.scopes, owner: who.account?.role === 'owner' } : null;
}

const message = (err: unknown) => (err instanceof Error ? err.message : String(err));

export function Docs({ channel }: { channel: Channel }) {
  const [docs, setDocs] = useState<ChannelDoc[] | null>(null);
  const [wrong, setWrong] = useState('');
  const [open, setOpen] = useState<ChannelDoc | null>(null);
  const [adding, setAdding] = useState(false);
  // Of two readings on their way, the one asked last is what is drawn.
  const asked = useRef(0);
  const read = () => {
    const nth = ++asked.current;
    readDocs(channel.id).then(
      (now) => nth === asked.current && (setDocs(now), setWrong('')),
      (err) => nth === asked.current && setWrong(message(err)),
    );
  };
  useEffect(read, [channel.id]);
  useEvents('channel.docs', (event) => event.data?.channel === channel.id && read(), [channel.id]);
  const by = writer();
  const changes = by ? mayChange(channel, by) : false;
  const answered = (now: Promise<ChannelDoc[]>) => {
    ++asked.current;
    now.then(
      (list) => (setDocs(list), setWrong('')),
      (err) => setWrong(message(err)),
    );
  };

  // A file pressed where it is attached in this channel: read here, and kept to hand where whoever pressed may.
  const pressed = docAsked.value;
  useEffect(() => {
    if (!pressed || pressed.channel !== channel.id) return;
    docAsked.value = null;
    const doc = `${pressed.project}/${pressed.path}`;
    setOpen({ doc, kind: 'file', project: pressed.project, path: pressed.path, title: pressed.title, state: 'there', by: '', added: 0 });
    if (changes) answered(addDoc(channel.id, doc));
  }, [pressed, channel.id]);

  if (open) return <Opened one={open} onBack={() => setOpen(null)} />;
  return (
    <div class="channel-docs">
      <div class="row">
        <span class="faint grow">Kept to hand for everybody in this channel, and told to its agents.</span>
        {changes && !adding && (
          <button class="ghost" onClick={() => setAdding(true)}>
            Add a document
          </button>
        )}
      </div>
      {wrong && <p class="error-text">{wrong}</p>}
      {adding && (
        <Adding
          channel={channel}
          kept={(docs ?? []).map((one) => one.doc)}
          onAdd={(doc) => answered(addDoc(channel.id, doc))}
          onDone={() => setAdding(false)}
        />
      )}
      {docs === null && !wrong && <p class="faint">Reading the documents</p>}
      {docs?.length === 0 && <p class="faint">This channel keeps no documents to hand yet.{changes ? ' Add one from a library or a project\'s files.' : ''}</p>}
      {docs?.map((one) => {
        const { said, where } = lineOf(one);
        return (
          <div key={one.doc} class={`channel-doc ${one.state}`} data-doc={one.doc}>
            {one.state === 'there' ? (
              <button class="context-row grow" title={`Read ${said} here`} onClick={() => setOpen(one)}>
                <span class="channel-doc-words">
                  <span class="truncate">{said}</span>
                  <span class="faint truncate">{where}</span>
                </span>
              </button>
            ) : (
              <span class="context-row grow nobody-to-open">
                <span class="channel-doc-words">
                  <span class="truncate">{said}</span>
                  <span class="faint truncate">{where}</span>
                </span>
              </span>
            )}
            {changes && (
              <button class="ghost icon" aria-label={`Take ${one.title ?? one.doc} out of this channel's documents`} title="Take it out of this channel's documents" onClick={() => answered(removeDoc(channel.id, one.doc))}>
                ×
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}

/** What is drawn as a picture where it is read in place, and not as text. */
const PICTURE = /\.(png|jpe?g|gif|webp|avif|svg)$/i;

/** A document read in place: as its page where it is Markdown, else as it is written. */
function Opened({ one, onBack }: { one: ChannelDoc; onBack: () => void }) {
  const [text, setText] = useState<string | null>(null);
  const [wrong, setWrong] = useState('');
  const [project, path] = [one.project ?? '', one.path ?? ''];
  const picture = PICTURE.test(path);
  useEffect(() => {
    if (picture) return;
    let stale = false;
    api.readFile(project, path).then(
      (content) => !stale && setText(content),
      (err) => !stale && setWrong(message(err)),
    );
    return () => {
      stale = true;
    };
  }, [project, path]);
  return (
    <div class="context-page">
      <div class="row">
        <button class="ghost icon context-back" aria-label="Back to the documents" title="Back to the documents" onClick={onBack}>
          ‹
        </button>
        <strong class="truncate grow">{one.title ?? one.doc}</strong>
        <button class="ghost" onClick={() => openTab({ kind: 'file', project, path })}>
          Open as a tab
        </button>
      </div>
      {wrong && <p class="error-text">{wrong}</p>}
      {picture && <img class="channel-doc-picture" src={shown(`/api/projects/${encodeURIComponent(project)}/raw?path=${encodeURIComponent(path)}`)} alt={one.title ?? path} />}
      {!picture && text === null && !wrong && <p class="faint">Reading it</p>}
      {text !== null && (/\.(md|markdown)$/i.test(path) ? <PagePreview source={text} name={path} /> : <pre class="channel-doc-text">{text}</pre>)}
    </div>
  );
}

/** What may be added, of the channel's workspace's projects: their libraries' documents and their files. */
function Adding({ channel, kept, onAdd, onDone }: { channel: Channel; kept: string[]; onAdd: (doc: string) => void; onDone: () => void }) {
  const [typed, setTyped] = useState('');
  const field = useFocus<HTMLInputElement>();
  const [trees, setTrees] = useState<Record<string, Entry[]>>({});
  const space = spaceOfChannel(channel, workspaces.value, projects.value);
  const ofSpace = useMemo(
    () => projects.value.filter((project) => !space || holdsProject({ kind: 'space', id: space.id }, project.id, workspaces.value) || channel.project === project.id),
    [space, channel.project],
  );
  useEffect(() => {
    for (const project of ofSpace) {
      api.tree(project.id).then(
        (entries) => setTrees((now) => ({ ...now, [project.id]: entries })),
        () => setTrees((now) => ({ ...now, [project.id]: [] })),
      );
    }
  }, [ofSpace.map((project) => project.id).join()]);
  const shelves = Object.fromEntries(ofSpace.map((project) => [project.id, collections.value[project.id] ?? []]));
  const shown = offered(ofSpace, trees, shelves, kept, typed);
  const reading = ofSpace.some((project) => !(project.id in trees));
  return (
    <div class="channel-docs-adding" role="group" aria-label="Add a document">
      <div class="row">
        <input
          class="grow"
          aria-label="Find a document to add"
          placeholder="Find a document or a file"
          value={typed}
          onInput={(event) => setTyped(event.currentTarget.value)}
          onKeyDown={(event) => event.key === 'Escape' && (event.stopPropagation(), onDone())}
          ref={field}
        />
        <button class="ghost" onClick={onDone}>
          Done
        </button>
      </div>
      {reading && <p class="faint">Reading the projects' files</p>}
      {!reading && shown.length === 0 && <p class="faint">{typed ? 'Nothing found says that.' : 'Nothing more to add from the projects of this workspace.'}</p>}
      {shown.map((one) => (
        <div key={one.project} class="channel-docs-of" data-project={one.project}>
          <h3>{one.name}</h3>
          {(
            [
              ['Library', one.documents],
              ['Files', one.files],
            ] as const
          ).map(([heading, offers]) =>
            offers.length === 0 ? null : (
              <div key={heading}>
                <h4 class="faint">{heading}</h4>
                {offers.map((offer) => (
                  <button key={offer.doc} class="context-row" data-offer={offer.doc} title={`Add ${offer.where}`} onClick={() => onAdd(offer.doc)}>
                    <span class="channel-doc-words">
                      <span class="truncate">{offer.title}</span>
                      <span class="faint truncate">{offer.where}</span>
                    </span>
                  </button>
                ))}
              </div>
            ),
          )}
        </div>
      ))}
    </div>
  );
}
