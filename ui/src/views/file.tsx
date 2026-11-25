// A file in a project. A blueprint opens as a blueprint document, a
// document to be written in, a picture as the picture it is, and anything
// else to be read: coloured, numbered, and not changed until Edit is chosen.
//
// A file that is a document of one of its project's libraries has a panel
// beside its text, opened from the speech bubble at the end of its head's
// row: what was said on it, with a line to add a comment, what it is, and
// what happened to it (`doctools.tsx`). Any other file has none of it.

import { useEffect, useRef, useState } from 'preact/hooks';
import { api, shown } from '../api.ts';
import { CodeEditor, languageOf } from '../editor/editor.tsx';
import { isPicture } from '../editor/language.ts';
import { documentAt } from '../library.ts';
import { attempt, closeTab, collections, draftOf, fileVersions, hints, isBlueprint, keepDraft, openFolder, openTab, projectName, tabKey, takeHint, toast, trees } from '../store.ts';
import { Empty, Pills } from '../ui.tsx';
import { fileSize } from '../util.ts';
import { cannotKeep } from '../write/blocks.ts';
import { Writer } from '../write/writer.tsx';
import { BlueprintDoc } from './blueprint.tsx';
import { useThreads } from './anchored.tsx';
import type { Thread } from '../anchored.ts';
import { commentBegun } from './comments.tsx';
import { comments } from '../journey.ts';
import { tabOf, toolSaid } from '../doctools.ts';
import type { DocTab } from '../doctools.ts';
import { besideStyle, DOC_SLIDES, DocTools } from './doctools.tsx';
import { FileActs } from './filehead.tsx';
import { signs } from './signs.tsx';
import { useSlid } from './toolsslide.ts';
import { showOwnBlueprints } from './ownblueprints.tsx';
import { PLACE } from '../blueprinted.ts';
import { Titled } from './pagehead.tsx';
import { useKept } from './returned.tsx';
import { Appearance } from './appearance.tsx';

export function FileView({ project, path }: { project: string; path: string }) {
  // A picture is shown, not read.
  return isPicture(path) ? <Picture project={project} path={path} /> : <TextFile project={project} path={path} />;
}

/**
 * Where a file is: its project and each folder down to it, each of which
 * opens the project's page there. One of those the factory keeps for the
 * project (`@blueprints/`) is in no folder of the project's: its way back
 * is the project's blueprints in the side panel, and the folders under it
 * are only said.
 */
export function Crumbs({ project, path }: { project: string; path: string }) {
  const parts = path.split('/');
  const name = parts.pop() ?? path;
  const kept = parts[0] === PLACE;
  return (
    <span class="crumbs truncate">
      <button class="ghost link" onClick={() => openFolder(project, '')}>
        {projectName(project)}
      </button>
      {kept && (
        <span>
          {' / '}
          <button class="ghost link" title="Where the factory keeps the project's own blueprints" onClick={() => showOwnBlueprints(project)}>
            Blueprints
          </button>
          {parts.slice(1).map((part, index) => (
            <span key={index}>{` / ${part}`}</span>
          ))}
        </span>
      )}
      {!kept && parts.map((part, index) => (
        <span key={index}>
          {' / '}
          <button class="ghost link" onClick={() => openFolder(project, parts.slice(0, index + 1).join('/'))}>
            {part}
          </button>
        </span>
      ))}
      {' / '}
      {/* The file's name is the heading of its view, though it reads as the end of a path. */}
      <h1>{name}</h1>
    </span>
  );
}

/** Where a file's own bytes are served from. */
function rawUrl(project: string, path: string): string {
  return shown(`/api/projects/${encodeURIComponent(project)}/raw?path=${encodeURIComponent(path)}`);
}

function sizeOf(project: string, path: string): string {
  const entry = (trees.value[project] ?? []).find((one) => one.path === path);
  return entry ? fileSize(entry.size) : '';
}

function Picture({ project, path }: { project: string; path: string }) {
  // Asked for again when the file changes on disk.
  const version = fileVersions.value[`${project}:${path}`] ?? 0;
  return (
    <div class="split">
      <div class="view-head">
        <Titled>
          <Crumbs project={project} path={path} />
          <span class="faint">{sizeOf(project, path)}</span>
        </Titled>
        <span class="grow" />
        <FileActs project={project} path={path} />
      </div>
      <div class="file-picture">
        <img src={`${rawUrl(project, path)}&v=${version}`} alt={path.split('/').pop()} />
      </div>
    </div>
  );
}

function TextFile({ project, path }: { project: string; path: string }) {
  // What was read, and what is written, are shown at once when its tab shows again, and read again behind.
  const [saved, setSaved] = useKept<string | null>('file.saved', null);
  const [text, setText] = useKept('file.text', '');
  const [missing, setMissing] = useState('');
  const [line, setLine] = useState<number | undefined>(undefined);
  const version = fileVersions.value[`${project}:${path}`] ?? 0;
  const tab = `file:${project}:${path}`;
  const latest = useRef({ text, saved });
  latest.current = { text, saved };

  // Read the file when it is opened, and again when it is said to have
  // changed. What is then done depends on what is on disk and what is here.
  useEffect(() => {
    let stale = false;
    api
      .readFile(project, path)
      .then((content) => {
        if (stale) return;
        const { text: mine, saved: was } = latest.current;
        // It reads the same as before: nothing changed for whoever is in the
        // middle of it, so nothing is redrawn.
        if (content === was) return;
        if (was !== null && mine !== was) {
          // There are edits here. If the disk now says exactly what is
          // here, that was this view's own save arriving as news.
          if (content === mine) setSaved(content);
          else toast(`${path} changed on disk while you were editing it. Saving will overwrite that.`, true);
          return;
        }
        // Opened again after being put away with edits in it: they come back.
        const draft = was === null ? draftOf(tab) : undefined;
        const kept = draft && draft.text !== content ? draft : null;
        if (kept && kept.base !== content) {
          toast(`${path} changed on disk since you edited it. Saving will overwrite that.`, true);
        }
        setSaved(content);
        setText(kept ? kept.text : content);
        setMissing('');
        setLoaded((count) => count + 1);
      })
      .catch((err) => !stale && setMissing(err instanceof Error ? err.message : String(err)));
    return () => {
      stale = true;
    };
  }, [project, path, version]);

  // The panel beside a document: whether it is open, which tab shows, and a mark's comments opened in it. The tab's.
  const [tools, setTools] = useKept('doc.tools', false);
  const [toolTab, setToolTab] = useKept<DocTab>('doc.tools.tab', 'comments');
  const [aside, setAside] = useKept<Thread | null>('doc.tools.thread', null);
  // How many times its comments were asked for by name (the mark on a
  // document's line in its library, a block's Comment): the panel opens on
  // them, and the line one is added by has the keyboard. As it is opened,
  // and again while it is already showing, when the asking is taken as it is given.
  const [asked, setAsked] = useState(0);
  const askComments = () => {
    setTools(true);
    setToolTab('comments');
    setAside(null);
    setAsked((times) => times + 1);
  };
  useEffect(() => {
    const hint = takeHint(tab);
    setLine(hint?.line);
    setAsked(0);
    if (hint?.comments) askComments();
  }, [tab]);
  const again = hints.value[tab]?.comments === true;
  useEffect(() => {
    if (again && takeHint(tab)) askComments();
  }, [again]);

  const [mode, setMode] = useState<Mode>(rememberedMode);
  const [editing, setEditing] = useKept('file.editing', false);
  // How many times the file has been read with something new in it. The
  // block editor is given the text once; a new reading is a new editor.
  const [loaded, setLoaded] = useKept('file.loaded', 0);
  const dirty = saved !== null && text !== saved;
  // Edits are kept outside this view: it is only on the page while its tab
  // is showing, and they must outlive it. The whole page then knows of them
  // too, so nothing reloads it over them.
  useEffect(() => {
    if (saved !== null) keepDraft(tab, text !== saved ? { text, base: saved } : null);
  }, [tab, text, saved]);
  // What the editor says is kept at once, and not only when the page is
  // next drawn: whoever asks a moment later (a save, a laying out) has
  // every key that was pressed before it asked.
  const typed = (next: string) => {
    latest.current = { ...latest.current, text: next };
    setText(next);
  };
  // A blueprint is laid out the one way, as its project says, by the
  // daemon. What comes back is only taken when nothing was typed while it
  // was asked for; a text that does not read is left as it is, and what
  // is wrong with it is what the check already shows.
  const layOut = async (): Promise<void> => {
    const before = latest.current.text;
    const after = await attempt(() => api.layOut(before, project, path));
    if (typeof after === 'string' && after !== before && latest.current.text === before) typed(after);
  };
  const save = async (): Promise<boolean> => {
    if (isBlueprint(path)) await layOut();
    const content = latest.current.text;
    // Nothing to write: asked for by something that saves first, as applying does.
    if (content === latest.current.saved) return true;
    const done = await attempt(() => api.writeFile(project, path, content));
    if (done === undefined) return false;
    latest.current = { ...latest.current, saved: content };
    setSaved(content);
    // A file under the name of what ships says where it stands now that it is saved: in use, or why not.
    const stands = (done as { stands?: { said: string; in_use: boolean } } | null)?.stands;
    if (stands) toast(stands.said, !stands.in_use);
    return true;
  };

  // A document (a Markdown file) can be written as blocks, read as the file
  // it is, or seen as the page it would be published as.
  const document = /\.(md|markdown)$/i.test(path);
  const unkept = document ? cannotKeep(text) : null;
  const shown: Mode = !document ? 'source' : mode === 'write' && unkept ? 'source' : mode;
  // Which document of which library it is, where it is one: what was said on it is that record's.
  const of = document ? documentAt(collections.value[project] ?? [], project, path) : null;
  // What was said on a few of its words: marked where they are, opened from there, in the panel where the margin has no room.
  const showing = tools && tabOf(toolTab) === 'comments' ? aside : null;
  const threads = useThreads(of?.record ?? null, text, shown, {
    thread: showing,
    show: (thread) => {
      setTools(true);
      setToolTab('comments');
      setAside(thread);
    },
  });
  const beside = of !== null && tools;
  // Coming in, it slides into the room it takes; going, a copy of it slides out over the room it gave back (`toolsslide.ts`).
  const slid = useSlid(beside, DOC_SLIDES);
  const said = of && threads.lines ? comments(threads.lines).length : 0;

  if (missing.endsWith('is not text')) {
    return (
      <div class="split">
        <div class="view-head">
          <Titled>
            <Crumbs project={project} path={path} />
            <span class="faint">{sizeOf(project, path)}</span>
          </Titled>
          <span class="grow" />
          <FileActs project={project} path={path} />
        </div>
        <Empty title="This file is not text, so there is nothing to read here">
          <a href={rawUrl(project, path)} download={path.split('/').pop()}>
            Save a copy of it
          </a>
        </Empty>
      </div>
    );
  }
  if (missing) return <Empty title="This file cannot be opened">{missing}</Empty>;
  if (saved === null) return <div class="empty">Opening {path}</div>;

  const crumb = <Crumbs project={project} path={path} />;
  if (isBlueprint(path)) {
    return (
      <BlueprintDoc
        title={path.split('/').pop()!}
        source={text}
        project={project}
        path={path}
        dirty={dirty}
        line={line}
        onChange={typed}
        onSave={save}
        onFormat={() => void layOut()}
      >
        {crumb}
        <button disabled={!dirty} onClick={() => void save()}>
          Save
        </button>
      </BlueprintDoc>
    );
  }
  // Anything else is read first. It is changed only once Edit is chosen, or
  // when it was left with changes in it that have not been saved.
  const reading = !document && !editing && !dirty;
  const lines = text === '' ? 0 : text.split('\n').length - (text.endsWith('\n') ? 1 : 0);
  const choose = (next: Mode) => {
    setMode(next);
    try {
      localStorage.setItem(DOC_MODE, next);
    } catch {
      /* not remembered */
    }
  };
  return (
    <div class="split">
      <div class="view-head">
        <Titled>
          {crumb}
          {document ? (
            <Pills
              value={shown}
              options={[
                ['write', 'Write'],
                ['source', 'Source'],
                ['page', 'Page'],
              ]}
              onChange={choose}
            />
          ) : (
            <Pills
              value={reading ? 'view' : 'edit'}
              options={[
                ['view', 'View'],
                ['edit', 'Edit'],
              ]}
              onChange={(next) => (next === 'view' && dirty ? toast('Save it first, or it would be read with changes nobody saved.', true) : setEditing(next === 'edit'))}
            />
          )}
          {dirty && <span class="faint">unsaved</span>}
          <span class="faint">
            {lines === 1 ? '1 line' : `${lines} lines`}
            {sizeOf(project, path) && ` · ${sizeOf(project, path)}`}
          </span>
        </Titled>
        <span class="grow" />
        {!reading && (
          <button disabled={!dirty} onClick={() => void save()}>
            Save
          </button>
        )}
        {document && <Appearance />}
        {/* What has happened to it and what was said on it, where it is a document, is its History: a line of its gear. */}
        <FileActs
          project={project}
          path={path}
          of={of}
          text
          dirty={dirty}
          onHistory={
            of
              ? () => {
                  setTools(true);
                  setToolTab('history');
                }
              : undefined
          }
          tool={of ? { label: toolSaid(said), title: tools ? 'Put away its comments, details and history' : 'Its comments, details and history, beside it', mark: signs.chat(), count: said, pressed: tools, onPress: () => {
                    // Opened by its own button it is every comment, and no mark's.
                    setAside(null);
                    setTools(!tools);
                  } } : undefined}
        />
      </div>
      <div class="doc-with" ref={slid.box} style={besideStyle(beside)}>
      <div class="doc-body reading-column">
        {document && mode === 'write' && unkept && (
          <p class="doc-note muted">
            This document has {unkept}, which writing in blocks cannot keep as it is yet, so it is shown as its source.
          </p>
        )}
        {shown === 'write' ? (
          <Writer
            key={`${project}:${path}:${loaded}`}
            markdown={text}
            onChange={setText}
            onSave={() => void save()}
            elsewhere={of !== null && asked > 0}
            onComment={of ? (begun) => commentBegun(of.record, begun, askComments) : undefined}
            marked={threads.marked}
            onBegin={of ? threads.begin : undefined}
            onOpen={threads.open}
            beside={threads.beside}
            opened={threads.opened}
            onMargined={threads.setMargined}
          />
        ) : shown === 'page' ? (
          <PagePreview source={text} name={path.split('/').pop() ?? path} />
        ) : (
          <CodeEditor
            value={text}
            language={languageOf(path, false)}
            readOnly={reading}
            line={line}
            onChange={setText}
            onSave={() => void save()}
            marked={of ? threads.marked : undefined}
            opened={threads.opened}
            onMark={(seqs, anchor) => threads.open(seqs, anchor, { top: 0, margin: null })}
          />
        )}
      </div>
      {beside && of && (
        <DocTools
          project={project}
          path={path}
          of={of}
          lines={threads.lines}
          textLines={lines}
          tab={tabOf(toolTab)}
          onTab={(next) => {
            setToolTab(next);
            setAside(null);
          }}
          onAway={() => setTools(false)}
          coming={slid.coming}
          asked={asked}
          thread={showing}
          threadAsked={threads.asked}
          onCloseThread={() => setAside(null)}
          isFound={threads.isFound}
        />
      )}
      </div>
    </div>
  );
}

type Mode = 'write' | 'source' | 'page';
const DOC_MODE = 'client.document';

function rememberedMode(): Mode {
  try {
    const saved = localStorage.getItem(DOC_MODE);
    return saved === 'source' || saved === 'page' ? saved : 'write';
  } catch {
    return 'write';
  }
}

/**
 * A document as the page it would be published as. The daemon builds it,
 * with the code that publishing uses, so this is that page and not a
 * likeness of it. It is shown boxed off: nothing in it can run or reach
 * the client around it.
 */
export function PagePreview({ source, name, whole, onLink }: { source: string; name: string; whole?: boolean; onLink?: (href: string) => void }) {
  const [html, setHtml] = useKept<string | null>(`page:${name}`, null);
  const [trouble, setTrouble] = useState('');
  useEffect(() => {
    let stale = false;
    api
      .pagePreview(source, name)
      .then((page) => !stale && setHtml(page))
      .catch((err) => !stale && setTrouble(err instanceof Error ? err.message : String(err)));
    return () => {
      stale = true;
    };
  }, [source, name]);
  if (trouble) return <Empty title="The page could not be built">{trouble}</Empty>;
  if (html === null) return <div class="empty">Building the page</div>;
  if (whole) return <WholePage html={html} name={name} onLink={onLink} />;
  return <iframe class="page-frame" title={`${name} as a page`} sandbox="" srcdoc={framed(html)} />;
}

/**
 * Said in a page shown in a tab, where it scrolls in its frame: room for a
 * scrollbar at each side, as the block editor keeps, and as thin a
 * scrollbar as the client's, so that its column stands where the written
 * one does. Nothing of the page as it is published is changed.
 */
const FRAMED = '<style>html { scrollbar-gutter: stable both-edges; scrollbar-width: thin; }</style>';
const framed = (html: string) => (html.includes('<head>') ? html.replace('<head>', `<head>${FRAMED}`) : FRAMED + html);

/** Said in the page itself, ahead of anything it holds: nothing in it runs, whatever its frame allows. */
const INERT = `<meta http-equiv="Content-Security-Policy" content="script-src 'none'; object-src 'none'; form-action 'none'; base-uri 'none'">`;

/** A part of a page shown whole that is to be brought into view, by its id (the page's head where it names none), and which asking that is: each is answered once. */
export interface Shows {
  at: string;
  asked: number;
}

/**
 * A page shown whole, as tall as it is, so that what scrolls is the page
 * it is on and not a box within it. Its frame lets the client measure it
 * and nothing more: no script runs in it (the frame allows none, and the
 * page is told so itself as well), and a link in it is the client's to
 * follow, never the frame's.
 */
export function WholePage({ html, name, onLink, shows }: { html: string; name: string; onLink?: (href: string) => void; shows?: Shows | null }) {
  const frame = useRef<HTMLIFrameElement>(null);
  const [height, setHeight] = useState(240);
  // Whether the page has been measured, and which asking was last brought into view.
  const [measured, setMeasured] = useState(false);
  const brought = useRef(0);
  const inert = html.includes('<head>') ? html.replace('<head>', `<head>${INERT}`) : INERT + html;
  useEffect(() => {
    const box = frame.current;
    if (!box) return;
    let watching: ResizeObserver | null = null;
    const fit = () => {
      const page = box.contentDocument?.documentElement;
      if (!page) return;
      setHeight(Math.ceil(page.getBoundingClientRect().height));
      setMeasured(true);
    };
    const follow = (event: MouseEvent) => {
      const link = (event.target as Element | null)?.closest?.('a[href]');
      if (!link) return;
      // Never the frame's to follow: it would leave the page for whatever the link names.
      event.preventDefault();
      const href = link.getAttribute('href') ?? '';
      if (href.startsWith('#')) box.contentDocument?.getElementById(decodeURIComponent(href.slice(1)))?.scrollIntoView({ block: 'start' });
      else if (/^https?:\/\//i.test(href)) window.open(href, '_blank', 'noopener');
      else if (!/^[a-z][a-z0-9+.-]*:/i.test(href)) onLink?.(href);
    };
    const ready = () => {
      // A frame that is between two documents has one with nothing in it
      // yet; its `load` comes when there is, and this is asked again then.
      const inside = box.contentDocument;
      const whole = inside?.documentElement;
      if (!inside || !whole) return;
      inside.addEventListener('click', follow);
      watching?.disconnect();
      watching = new ResizeObserver(fit);
      watching.observe(whole);
      fit();
    };
    box.addEventListener('load', ready);
    ready();
    return () => {
      box.removeEventListener('load', ready);
      watching?.disconnect();
    };
  }, [inert]);
  // Once the frame is as tall as its page, and once for each asking: before that the part is not yet where it will be.
  useEffect(() => {
    const box = frame.current;
    if (!shows || !measured || !box || brought.current === shows.asked) return;
    brought.current = shows.asked;
    (shows.at ? (box.contentDocument?.getElementById(shows.at) ?? box) : box).scrollIntoView({ block: 'start' });
  }, [shows?.asked, measured, height]);
  // `allow-same-origin` and nothing else: the client may read how tall the
  // page is. It must never be joined by `allow-scripts`, which together
  // would make whatever a document holds a script of the client's own.
  return <iframe ref={frame} class="page-frame whole" title={`${name} as a page`} sandbox="allow-same-origin" scrolling="no" style={{ height: `${height}px` }} srcdoc={inert} />;
}

/**
 * A blueprint that ships with the product: read it as text or as a graph,
 * and apply it. Opened under a project it applies to that project; opened
 * from the factory's list it is nobody's yet, and applying it asks which
 * project it is for. It is not a file, so it is not edited here.
 */
export function ShippedBlueprint({ name, project }: { name: string; project?: string }) {
  const [source, setSource] = useKept<string | null>('shipped.source', null);
  const [missing, setMissing] = useState('');
  // A line it was asked to be opened at, by whatever opened it, and again while it is showing.
  const tab = tabKey(project === undefined ? { kind: 'blueprint', name } : { kind: 'blueprint', name, project });
  const [at, setAt] = useState<{ line?: number; asked: number }>({ asked: 0 });
  const hinted = hints.value[tab];
  useEffect(() => {
    const line = takeHint(tab)?.line;
    if (line) setAt((now) => ({ line, asked: now.asked + 1 }));
  }, [tab, hinted]);
  useEffect(() => {
    let stale = false;
    api
      .blueprintSource(name)
      .then((text) => !stale && setSource(text))
      .catch((err) => !stale && setMissing(err instanceof Error ? err.message : String(err)));
    return () => {
      stale = true;
    };
  }, [name]);
  if (missing) return <Empty title="This blueprint cannot be opened">{missing}</Empty>;
  if (source === null) return <div class="empty">Opening {name}</div>;
  return (
    <BlueprintDoc
      title={name}
      source={source}
      project={project ?? null}
      readOnly
      ships
      line={at.line}
      lineAsked={at.asked}
      // Applied to a project it is that project's, and is shown as that
      // in place of this: two tabs called the same would be one too many.
      asksProject={
        project === undefined
          ? (chosen) => {
              openTab({ kind: 'blueprint', name, project: chosen });
              closeTab(tabKey({ kind: 'blueprint', name }), true);
            }
          : undefined
      }
    >
      {(() => {
        const said = project === undefined ? 'ships with the product; applying it asks which project it is for' : `ships with the product; applies to ${projectName(project)}`;
        return (
          <span class="faint truncate head-note" title={said}>
            {said}
          </span>
        );
      })()}
    </BlueprintDoc>
  );
}
