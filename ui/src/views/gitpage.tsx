// A project's Git page: its repository as the daemon reads it from git (the
// branch its folder is on, what is changed, the commits of a branch, its
// branches, its remotes), and each part an installed package says it
// shows of a repository it serves, as the package's program answered it:
// a tab each, in the page's head, git's own first (`gittabs.ts`). What is
// said in words is `gitpage.ts`.
//
// It reads once and follows: the daemon says `project.git.changed` as the
// system says the repository's own folders changed (a commit, a branch, a
// file staged, a fetch), and a file written through the daemon is said as
// any is; each has git read again, one reading at a time. A package's
// parts reach the service it is for, so each is read when its tab is
// first shown and when Refresh is pressed while it shows, and never
// because the repository changed: a part whose tab is never opened is
// never read. Which tab shows, and what each read, is kept by the page's
// tab, so a tab shown again is as it was left.

import { useEffect, useRef } from 'preact/hooks';
import { ApiError } from '../api.ts';
import {
  type Branch,
  type Part,
  type Read,
  type Seen,
  OneAtATime,
  branchesBy,
  cellSaid,
  changeSaid,
  headSaid,
  linkOf,
  noneSaid,
  readPart,
  readSeen,
  short,
} from '../gitpage.ts';
import { gitAsked, sayGitShowing } from '../gitshown.ts';
import { type GitTab, gitTabs, readKey, showing, toRead } from '../gittabs.ts';
import { projects } from '../store.ts';
import { Empty, NotTrusted, useEvents } from '../ui.tsx';
import { ago } from '../util.ts';
import { PageActs, Titled } from './pagehead.tsx';
import { useKept, useKeptRef } from './returned.tsx';
import { reveal, sideways } from './sideways.ts';
import { headWord, ThingWas } from './thingwas.tsx';

export { GIT_GLYPH } from '../gitpage.ts';

/** What one part came to: being read, its rows, or why not. */
type Shown = { reading: true } | { read: Read } | { failed: string };

const filesSaid = (count: number) => (count === 1 ? '1 file' : `${count} files`);

const why = (err: unknown) => (err instanceof ApiError || err instanceof Error ? err.message : String(err));

export function GitView({ project: id }: { project: string }) {
  const project = projects.value.find((one) => one.id === id);
  const [seen, setSeen] = useKept<Seen | null>(`git:${id}`, null);
  const [failed, setFailed] = useKept<string | null>(`git-failed:${id}`, null);
  // The branch whose commits are listed, whole as it is listed; `HEAD` for what the folder is on.
  const [branch, setBranch] = useKept<string>(`git-branch:${id}`, 'HEAD');
  const [parts, setParts] = useKept<Record<string, Shown>>(`git-parts:${id}`, {});
  // The tab that shows, by what it is known by; nothing for the first.
  const [chosen, setChosen] = useKept<string | null>(`git-tab:${id}`, null);
  // Which asking for a tab was brought to it, so that a tab shown again asks for nothing.
  const answered = useKeptRef<number>(`git-asked:${id}`, 0);
  const row = useRef<HTMLDivElement>(null);
  const asked = useRef(branch);
  asked.current = branch;
  // Which reading of the parts is the latest: one begun before Refresh is not drawn after it.
  const generation = useKeptRef<number>(`git-generation:${id}`, 0);

  const reading = useRef<OneAtATime | null>(null);
  if (!reading.current) {
    reading.current = new OneAtATime(async () => {
      try {
        setSeen(await readSeen(id, asked.current));
        setFailed(null);
      } catch (err) {
        // A branch that is gone since it was chosen: what the folder is on, from now.
        if (asked.current !== 'HEAD' && err instanceof ApiError && err.status === 404) {
          setBranch('HEAD');
          asked.current = 'HEAD';
          reading.current?.ask();
          return;
        }
        setFailed(why(err));
      }
    });
  }
  const readAgain = () => reading.current?.ask();

  const tabs = gitTabs(seen);
  const shown = showing(tabs, chosen);
  const readIt = (tab: GitTab) => {
    const key = readKey(tab);
    if (!key || !tab.package || !tab.part) return;
    const at = generation.current;
    const kept = (said: Shown) => at === generation.current && setParts((was) => ({ ...was, [key]: said }));
    setParts((was) => ({ ...was, [key]: { reading: true } }));
    readPart(id, tab.package, tab.part.name)
      .then((read) => kept({ read }))
      .catch((err) => kept({ failed: why(err) }));
  };

  useEffect(readAgain, [id, branch]);
  // A part is read when its tab first shows, and where Refresh forgot what it read.
  const unread = toRead(shown, parts);
  useEffect(() => {
    if (unread && shown) readIt(shown);
  }, [shown?.key, unread]);
  // A tab somebody asked for by an address.
  const asking = gitAsked.value;
  useEffect(() => {
    if (!asking || asking.project !== id || asking.asked <= answered.current) return;
    answered.current = asking.asked;
    setChosen(asking.at);
  }, [asking?.asked]);
  useEffect(() => reveal(row.current), [shown?.key, tabs.length]);
  // What the address bar says of it after the `#`: the tab that shows, and nothing for the first.
  const showingAt = shown && shown !== tabs[0] ? shown.key : '';
  useEffect(() => sayGitShowing(id, showingAt), [id, showingAt]);
  useEvents(['project.git.changed', 'project.file.changed'], (event) => event.data?.project === id && readAgain(), [id]);

  if (!project) return <Empty title="This project is gone" />;
  // Git read again, and every part forgotten: the one that shows is read again now, each other as it is next shown.
  const everything = () => {
    readAgain();
    generation.current += 1;
    setParts({});
  };
  return (
    <div class="git-page">
      <div class="view-head">
        <Titled>
          <h1>
            {project.name} {headWord(id, 'git', 'git')}
          </h1>
          <ThingWas project={id} thing="git" />
          <NotTrusted project={id} />
          {seen?.repository && <span class="faint truncate git-head-said">{headSaid(seen.head)}</span>}
        </Titled>
        <span class="grow" />
        <PageActs tab={{ kind: 'git', project: id, ...(shown && shown !== tabs[0] ? { at: shown.key } : {}) }} />
        {seen?.repository && (
          <div class="view-tabs tabs-ruled git-tabs">
            <div class="view-tabs-list" role="tablist" aria-label="What is shown of the repository" ref={row} onWheel={sideways}>
              {tabs.map((tab) => (
                <button
                  key={tab.key}
                  type="button"
                  role="tab"
                  class="view-tab git-tab"
                  data-git-tab={tab.key}
                  aria-selected={tab === shown}
                  aria-controls={`git-panel-${tab.key}`}
                  onClick={() => setChosen(tab.key)}
                >
                  {tab.title}
                  {tab.whose && <span class="faint">{tab.packageTitle}</span>}
                  {tab.count !== undefined && <span class="faint git-count">{tab.count}</span>}
                </button>
              ))}
            </div>
            <span class="grow" />
            <button onClick={everything}>Refresh</button>
          </div>
        )}
      </div>
      <div class="view-body stack git-body">
        {failed && <p class="error git-failed">{failed}</p>}
        {seen === null && !failed ? (
          <div class="empty">Reading the repository</div>
        ) : seen && !seen.repository ? (
          <Empty title="No git repository here">
            <p class="muted">{seen.said}</p>
          </Empty>
        ) : seen && shown ? (
          <div role="tabpanel" id={`git-panel-${shown.key}`} aria-label={shown.title} class="git-tab-panel" data-git-tab={shown.key}>
            {shown.own === 'changes' ? (
              <Changes seen={seen} />
            ) : shown.own === 'commits' ? (
              <Commits seen={seen} branch={branch} choose={setBranch} />
            ) : shown.own === 'branches' ? (
              <Branches seen={seen} />
            ) : shown.own === 'remotes' ? (
              <Remotes seen={seen} />
            ) : shown.part && shown.package ? (
              <PartShown tab={shown} part={shown.part} shown={parts[readKey(shown) ?? '']} />
            ) : null}
          </div>
        ) : null}
      </div>
    </div>
  );
}

function Changes({ seen }: { seen: Seen }) {
  const changes = seen.changes ?? [];
  return (
    <section class="panel git-panel" data-part="changes">
      <div class="panel-head">
        <h2>What is changed</h2>
        <span class="muted">{changes.length === 0 ? 'nothing' : filesSaid(changes.length + (seen.more ?? 0))}</span>
      </div>
      {changes.length === 0 ? (
        <p class="muted git-pad">Nothing is changed: the working tree is as the last commit has it.</p>
      ) : (
        <ul class="git-list">
          {changes.map((change) => (
            <li key={change.path} class="git-line">
              <code class="git-path">{change.was ? `${change.was} → ${change.path}` : change.path}</code>
              <span class="muted">{changeSaid(change)}</span>
            </li>
          ))}
          {(seen.more ?? 0) > 0 && <li class="git-line muted">and {seen.more} more</li>}
        </ul>
      )}
    </section>
  );
}

function Commits({ seen, branch, choose }: { seen: Seen; branch: string; choose: (branch: string) => void }) {
  const commits = seen.commits ?? [];
  const branches = seen.branches ?? [];
  return (
    <section class="panel git-panel" data-part="commits">
      <div class="panel-head git-commits-head">
        <h2>Commits</h2>
        <label class="git-of">
          <select value={branch} onChange={(event) => choose((event.target as HTMLSelectElement).value)} aria-label="Whose commits">
            <option value="HEAD">{seen.head?.branch ? `${seen.head.branch}, where the folder is` : 'where the folder is'}</option>
            {branches.map((one) => (
              <option key={one.refname} value={one.refname}>
                {one.name}
              </option>
            ))}
          </select>
        </label>
      </div>
      {commits.length === 0 ? (
        <p class="muted git-pad">Nothing is committed yet.</p>
      ) : (
        <ul class="git-list">
          {commits.map((one) => (
            <li key={one.commit} class="git-line git-commit">
              <code class="faint" title={one.commit}>
                {short(one.commit).slice(0, 7)}
              </code>
              <span class="git-subject">{one.subject}</span>
              <span class="muted git-by" title={one.email}>
                {one.author}
                {one.parents.length > 1 ? ', a merge' : ''} · {ago(one.at)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function Branches({ seen }: { seen: Seen }) {
  const current = seen.head?.branch;
  const line = (one: Branch) => (
    <li key={one.refname} class="git-line git-commit" data-branch={one.name}>
      <code class="git-path">{one.name}</code>
      <span class="git-subject">
        {one.remote === undefined && one.name === current && <span class="badge plain">where the folder is</span>} {one.subject}
      </span>
      <span class="muted git-by">{ago(one.at)}</span>
    </li>
  );
  const by = branchesBy(seen.branches ?? []);
  return (
    <section class="panel git-panel" data-part="branches">
      <div class="panel-head">
        <h2>Branches</h2>
      </div>
      {by.length === 0 ? (
        <p class="muted git-pad">No branch yet: nothing is committed.</p>
      ) : (
        by.map(([where, all]) => (
          <div key={where} class="git-group">
            <h3 class="muted">{where === '' ? 'Its own' : `As ${where} has them`}</h3>
            <ul class="git-list">{all.map(line)}</ul>
          </div>
        ))
      )}
    </section>
  );
}

function Remotes({ seen }: { seen: Seen }) {
  const remotes = seen.remotes ?? [];
  return (
    <section class="panel git-panel" data-part="remotes">
      <div class="panel-head">
        <h2>Remotes</h2>
      </div>
      {remotes.length === 0 ? (
        <p class="muted git-pad">None: everything of it is here.</p>
      ) : (
        <ul class="git-list">
          {remotes.map((one) => (
            <li key={one.name} class="git-line">
              <code class="git-path">{one.name}</code>
              <span class="git-subject">{one.fetch}</span>
              {one.push && <span class="muted git-by">pushed to {one.push}</span>}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function PartShown({ tab, part, shown }: { tab: GitTab; part: Part; shown: Shown | undefined }) {
  const read = shown && 'read' in shown ? shown.read : null;
  return (
    <section class="panel git-panel" data-package={tab.package} data-part={part.name}>
      <div class="panel-head">
        <h2>{part.title}</h2>
        <span class="muted">{tab.packageTitle}</span>
      </div>
      {tab.refused ? (
        <p class="muted git-pad git-refused">{tab.refused}</p>
      ) : !shown || 'reading' in shown ? (
        <p class="muted git-pad">Reading {part.title.toLowerCase()}…</p>
      ) : 'failed' in shown ? (
        <p class="error git-pad git-why">{shown.failed}</p>
      ) : read && read.rows.length === 0 ? (
        <p class="muted git-pad">{noneSaid(part.title)}</p>
      ) : read ? (
        <div class="git-scroll">
          <table class="git-table">
            <thead>
              <tr>
                {part.columns.map((column) => (
                  <th key={column.field}>{column.title}</th>
                ))}
                {part.link && <th aria-label="Where it is read" />}
              </tr>
            </thead>
            <tbody>
              {read.rows.map((row, at) => {
                const link = linkOf(read, row);
                return (
                  <tr key={at}>
                    {part.columns.map((column) => (
                      <td key={column.field}>{cellSaid(row[column.field])}</td>
                    ))}
                    {part.link && (
                      <td>
                        {link && (
                          <a href={link} target="_blank" rel="noopener noreferrer">
                            Open
                          </a>
                        )}
                      </td>
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
          {read.more > 0 && <p class="muted git-pad">and {read.more} more</p>}
        </div>
      ) : null}
    </section>
  );
}
