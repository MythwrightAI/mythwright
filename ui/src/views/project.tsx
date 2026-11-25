// A project's front page: how it stands.
//
// It is about the project, not its files: what is on its board, who works
// on it, what was said about it lately and how its last runs went, each
// of which it sends the reader on to. What is running for it is not a
// card here: a count of instances by the names of blueprints is the
// factory's own bookkeeping, and is read where what runs is (Activity,
// and under the project's blueprints). At its head is a place
// to find anything in the project by typing: a file by its name, a card,
// something said. Its files are a page of their own (`repo.tsx`), under
// the project in the side panel.

import { namesOf } from '../envsheld.ts';
import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Board, Channel, Hit, Instance, Message, Run } from '../api.ts';
import { coalesce } from '../events.ts';
import { byName } from '../find.ts';
import { Assignee } from './assignee.tsx';
import { agents, channels, fileVersions, lookAt, narrowTo, openTab, projects, trees, used } from '../store.ts';
import { pipelinesOf } from '../pipelines.ts';
import { projectRename } from './outline.tsx';
import type { Tab } from '../store.ts';
import { Badge, Empty, NotTrusted, useEvents } from '../ui.tsx';
import { duration, parentOf, when } from '../util.ts';
import { PagePreview } from './file.tsx';
import { KeepFolder, keptMenu, useKeptIn } from './blueprinted.tsx';
import { glyph, Titled } from './pagehead.tsx';
import { README } from './repo.tsx';
import { hashed } from '../kept.ts';
import { marked, openHit } from './search.tsx';
import { Running } from './running.tsx';
import { useKept } from './returned.tsx';
import { ProjectActs } from './trusted.tsx';
import { headWord } from './thingwas.tsx';

/** How many cards of a column are shown before the rest are counted. */
const SHOWN = 4;

export function ProjectHome({ id }: { id: string }) {
  const project = projects.value.find((one) => one.id === id);
  // `undefined` until it is known; `null` when the project has no board.
  // What was read is shown at once when its tab shows again, and read again behind.
  const [board, setBoard] = useKept<Board | null | undefined>('project.board', undefined);
  const [applied, setApplied] = useKept<Instance[]>('project.applied', []);
  const [runs, setRuns] = useKept<Run[]>('project.runs', []);
  const kept = useKeptIn(id);
  const [said, setSaid] = useKept<{ channel: Channel; message: Message }[]>('project.said', []);
  // `undefined` while it is being read; `null` when there is none.
  const [readme, setReadme] = useKept<string | null | undefined>('project.readme', undefined);
  const about = (trees.value[id] ?? []).find((entry) => !entry.dir && parentOf(entry.path) === '' && README.test(entry.name));
  const talked = channels.value.filter((channel) => channel.project === id);
  const talkedIn = talked.map((channel) => channel.id).join();

  const reload = useMemo(
    () =>
      coalesce(() => {
        void api.boardIfAny(id).then((found) => setBoard(found?.board ?? null), () => {});
        void api.instances({ project: id, limit: 300 }).then(setApplied).catch(() => {});
        void api.runs({ project: id, limit: 6 }).then(setRuns).catch(() => {});
      }, 150),
    [id],
  );
  useEffect(reload, [reload]);
  useEvents(['board.**', 'collection.ensured', 'instance.*', 'run.started', 'run.finished'], reload, [reload], reload);
  useEffect(() => lookAt(id), [id]);

  // The last things said where the project is talked about.
  const listen = useMemo(
    () =>
      coalesce(() => {
        const about = channels.value.filter((channel) => channel.project === id);
        void Promise.all(about.map((channel) => api.messages(channel.id).then((all) => all.slice(-3).map((message) => ({ channel, message })))))
          .then((each) => setSaid(each.flat().sort((a, b) => b.message.created - a.message.created).slice(0, 5)))
          .catch(() => {});
      }, 150),
    [id],
  );
  useEffect(listen, [listen, talkedIn]);
  useEvents('channel.message', listen, [listen]);

  const version = about ? (fileVersions.value[`${id}:${about.path}`] ?? 0) : 0;
  useEffect(() => {
    if (!about) {
      setReadme(null);
      return;
    }
    // What was read before stands until what is there now has been read: nothing is drawn as being read again.
    let stale = false;
    api
      .readFile(id, about.path)
      .then((content) => !stale && setReadme(content))
      .catch(() => !stale && setReadme(null));
    return () => {
      stale = true;
    };
  }, [id, about?.path, version]);

  if (!project) return <Empty title="This project is gone" />;
  const blueprint = new Map(applied.map((one) => [one.id, one.blueprint]));
  const working = agents.value.filter((slot) => slot.project === id && slot.active);
  const cards = board?.items ?? [];
  const byColumn = (board?.columns ?? []).map((column) => `${cards.filter((card) => card.status === column).length} ${column}`);
  // The factory's view of something, narrowed to this project.
  const narrowed = (tab: Tab) => () => {
    narrowTo(id);
    openTab(tab);
  };

  return (
    <div>
      <div class="view-head">
        <Titled>
          <h1>{project.name}</h1>
          <NotTrusted project={id} />
          <code class="faint truncate">{project.path}</code>
        </Titled>
        <span class="grow" />
        {/* Its environments are a line of the gear: a project with none has no line for them in the side panel. */}
        <ProjectActs
          id={id}
          more={() => [
            { label: headWord(id, 'environments', 'Environments'), hint: environmentsHint(id), mark: glyph('◇'), act: () => openTab({ kind: 'environments', project: id }) },
            // Its pipelines are a line of the side panel once it has any: here they are reached, and the first made, before then.
            { label: headWord(id, 'pipelines', 'Pipelines'), hint: pipelinesHint(id), mark: glyph('▶'), act: () => openTab({ kind: 'pipelines', project: id }) },
            ...keptMenu(project, kept),
            // Called something else, for whoever may arrange it (`outline.tsx`): its id stays as it is.
            ...projectRename(project),
          ]}
        />
        <KeepFolder />
      </div>
      <div class="view-body stack" style="gap:20px">
        <Find project={id} name={project.name} />

        <div class="tiles">
          <Tile
            label={headWord(id, 'board', 'Board')}
            value={board ? String(cards.length) : board === null ? 'none' : '…'}
            unit={board ? (cards.length === 1 ? 'card' : 'cards') : ''}
            note={board === null ? 'a blueprint makes one' : byColumn.join(' · ')}
            onOpen={() => openTab({ kind: 'board', project: id })}
          />
          <Tile
            label="Agents"
            value={String(working.length)}
            unit={working.length === 1 ? 'agent' : 'agents'}
            note={working.length > 0 ? working.map((slot) => slot.name).join(', ') : 'none works on it yet'}
          />
          <Tile
            label="Channels"
            value={String(talked.length)}
            unit={talked.length === 1 ? 'channel' : 'channels'}
            note={talked.length > 0 ? talked.map((channel) => hashed(channel)).join(', ') : 'none is about it yet'}
            onOpen={talked[0] && (() => openTab({ kind: 'channel', id: talked[0]!.id }))}
          />
        </div>

        <Glance project={id} name={project.name} board={board} />

        {said.length > 0 && (
          <div class="panel">
            <div class="panel-head">
              <h2>Said lately</h2>
              <span class="muted">where the project is talked about</span>
            </div>
            <table class="said">
              <tbody>
                {said.map(({ channel, message }) => (
                  <tr key={message.id} class="clickable" onClick={() => openTab({ kind: 'channel', id: channel.id }, { message: message.id })}>
                    <td class="muted">{when(message.created)}</td>
                    <td class="muted">{hashed(channel)}</td>
                    <td>
                      <strong>{message.author_name}</strong>
                    </td>
                    <td>
                      <span class="said-body">{message.body}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div class="panel">
          <div class="panel-head">
            <h2>Recent runs</h2>
            <span class="muted">of what was applied for this project</span>
            <span class="grow" />
            <button class="ghost link" onClick={narrowed({ kind: 'activity' })}>
              All its activity
            </button>
            <button class="ghost link" onClick={narrowed({ kind: 'metrics' })}>
              Its measurements
            </button>
          </div>
          <table>
            <tbody>
              {runs.map((run) => (
                <tr key={run.id} class="clickable" data-run={run.id} title="Open this run as a page of its own" onClick={() => openTab({ kind: 'run', id: run.id })}>
                  <td class="muted">{when(run.started)}</td>
                  <td>{blueprint.get(run.instance) ?? <Running id={run.instance} plain />}</td>
                  <td>{run.flow === 'main' ? 'main' : run.flow.replace('on.', 'on ')}</td>
                  <td>
                    <Badge status={run.status} />
                  </td>
                  <td class="num">{run.finished ? duration(run.finished - run.started) : ''}</td>
                </tr>
              ))}
              {runs.length === 0 && (
                <tr>
                  <td class="faint">Nothing has run for it yet. Apply one of its blueprints, or ask an agent to set up its fleet.</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        <div class="panel">
          <div class="panel-head">
            <h2>{about?.name ?? 'README.md'}</h2>
            <span class="muted">what the project says it is</span>
            <span class="grow" />
            <button class="ghost link" onClick={() => openTab({ kind: 'repo', project: id })}>
              Its repo
            </button>
            {/* Its Git page, wherever its folder is a repository: one the factory made is listed under it in the side panel no more than its Repo is (`gitListed`). */}
            {project?.repository === true && (
              <button class="ghost link" onClick={() => openTab({ kind: 'git', project: id })}>
                Its Git
              </button>
            )}
            {about && (
              <button class="ghost link" onClick={() => openTab({ kind: 'file', project: id, path: about.path })}>
                Write in it
              </button>
            )}
          </div>
          {!about ? (
            <Empty title="This project has no README.md">
              <span>What a project is for is best said in one. Ask an agent to write it, or add the file to the project's folder.</span>
            </Empty>
          ) : readme == null ? (
            <div class="empty">Reading {about.name}</div>
          ) : (
            <div class="project-readme whole">
              <PagePreview
                source={readme}
                name={about.name}
                whole
                onLink={(href) => {
                  // A link to another file of the project, from where this one is.
                  const at = about.path.split('/').slice(0, -1);
                  for (const part of href.split('#')[0]!.split('?')[0]!.split('/')) {
                    if (part === '..') at.pop();
                    else if (part !== '.' && part !== '') at.push(decodeURIComponent(part));
                  }
                  if (at.length > 0) openTab({ kind: 'file', project: id, path: at.join('/') });
                }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Finding something in the project by typing. A file is found by its name
 * as it is typed, from the names the client already has; everything else
 * (what is written in files, cards, what was said) is the daemon's search,
 * asked a moment after the typing stops.
 */
function Find({ project, name }: { project: string; name: string }) {
  const [text, setText] = useState('');
  const [hits, setHits] = useState<Hit[]>([]);
  const [chosen, setChosen] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const typed = text.trim();
  const entries = trees.value[project];
  const files = useMemo(() => byName(entries ?? [], typed), [entries, typed]);

  useEffect(() => {
    setChosen(0);
    if (!typed) {
      setHits([]);
      return;
    }
    let stale = false;
    const timer = setTimeout(() => {
      void api
        .search(typed, project)
        .then((answer) => !stale && setHits(answer.hits))
        .catch(() => !stale && setHits([]));
    }, 150);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  }, [typed, project]);

  // By name first; then what the daemon found, less the files already there.
  const named = new Set(files.map((entry) => entry.path));
  const results = [
    ...files.map((entry) => ({
      key: `name:${entry.path}`,
      kind: 'file',
      title: entry.name,
      where: parentOf(entry.path),
      snippet: null as string | null,
      open: () => openTab({ kind: 'file', project, path: entry.path }),
    })),
    ...hits
      .filter((hit) => !(hit.link.path && named.has(hit.link.path)))
      .map((hit) => ({ key: hit.reference, kind: hit.kind as string, title: hit.title, where: '', snippet: hit.snippet as string | null, open: () => openHit(hit) })),
  ];
  useEffect(() => list.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }), [chosen]);

  return (
    <div class="find">
      <input
        type="search"
        class="find-input"
        value={text}
        placeholder={`Find in ${name}: a file by its name, a card, something said`}
        aria-label={`Find in ${name}`}
        onInput={(event) => setText(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === 'Escape') setText('');
          if (results.length === 0) return;
          if (event.key === 'ArrowDown') {
            event.preventDefault();
            setChosen((chosen + 1) % results.length);
          } else if (event.key === 'ArrowUp') {
            event.preventDefault();
            setChosen((chosen - 1 + results.length) % results.length);
          } else if (event.key === 'Enter') {
            event.preventDefault();
            results[Math.min(chosen, results.length - 1)]!.open();
          }
        }}
      />
      {typed !== '' && (
        <div class="find-results" ref={list} role="listbox" aria-label="What was found">
          {results.map((one, index) => (
            <button key={one.key} class="hit" role="option" aria-selected={index === chosen} onMouseMove={() => setChosen(index)} onClick={one.open}>
              <span class="row">
                <span class="badge plain">{one.kind}</span>
                <strong class="truncate">{one.title}</strong>
                {one.where && <span class="faint truncate">{one.where}</span>}
              </span>
              {one.snippet && <span class="snippet">{marked(one.snippet)}</span>}
            </button>
          ))}
          {results.length === 0 && <div class="find-none">Nothing in {name} matches.</div>}
        </div>
      )}
    </div>
  );
}

/** The project's board at a glance: each column, and the cards at the top of it. */
function Glance({ project, name, board }: { project: string; name: string; board: Board | null | undefined }) {
  const open = (item?: string) => openTab({ kind: 'board', project }, item ? { item } : undefined);
  return (
    <div class="panel">
      <div class="panel-head">
        <h2>{headWord(project, 'board', 'Board')}</h2>
        {board && <span class="muted">{board.items.length === 1 ? '1 card' : `${board.items.length} cards`}</span>}
        <span class="grow" />
        {board && (
          <button class="ghost link" onClick={() => open()}>
            Open the board
          </button>
        )}
      </div>
      {board === undefined ? (
        <div class="empty">Reading the board</div>
      ) : board === null ? (
        <Empty title={`${name} has no board yet`}>
          <span>A board is made by a blueprint, which says what its columns are and where its cards are kept.</span>
          <button class="primary" onClick={() => openTab({ kind: 'blueprint', name: 'std/board', project })}>
            Make one with a blueprint
          </button>
        </Empty>
      ) : (
        <div class="glance">
          {board.columns.map((column) => {
            const cards = board.items.filter((card) => card.status === column).sort((a, b) => a.position - b.position);
            return (
              <div key={column} class="glance-column">
                <div class="glance-head">
                  <span class="truncate">{column}</span>
                  <span class="count">{cards.length}</span>
                </div>
                {cards.slice(0, SHOWN).map((card) => (
                  <button key={card.id} class="glance-card" title={card.title} onClick={() => open(card.id)}>
                    <span class="truncate">{card.title}</span>
                    {card.assignee && <Assignee assignee={card.assignee} brief />}
                  </button>
                ))}
                {cards.length > SHOWN && (
                  <button class="ghost link" onClick={() => open()}>
                    and {cards.length - SHOWN} more
                  </button>
                )}
                {cards.length === 0 && <span class="faint">nothing</span>}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function Tile(props: { label: string; value: string; unit: string; note: string; onOpen?: (() => void) | false }) {
  const inside = (
    <>
      <span class="muted">{props.label}</span>
      <span class="value">
        {props.value} <small>{props.unit}</small>
      </span>
      <span class="faint truncate">{props.note || ' '}</span>
    </>
  );
  return props.onOpen ? (
    <button class="tile" onClick={props.onOpen}>
      {inside}
    </button>
  ) : (
    <div class="tile">{inside}</div>
  );
}

/** What the gear's line of a project's pipelines says beside its name: how many it has. */
function pipelinesHint(project: string): string {
  const many = pipelinesOf(used.value[project]).length;
  return many === 0 ? 'none yet' : `${many}`;
}

/** What the gear's line of a project's environments says beside its name: how many it has. */
function environmentsHint(project: string): string {
  const many = namesOf(project).length;
  return many === 0 ? 'none yet' : `${many}`;
}
