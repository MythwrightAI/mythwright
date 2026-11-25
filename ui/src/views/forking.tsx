// Forking a blueprint that ships, and going back from a fork, from a
// blueprint's page.
//
// A blueprint's Summary says where its text stands among the blueprints
// that ship, and some of its lines lead somewhere (`forked.ts`). This is
// what a blueprint's page does about that, in one place so that the page
// itself is told little: the Fork button in the head of one that ships,
// for whoever may write a project's files; the dialog that asks which
// project where the page is of none; the question asked before a fork is
// taken away, which says what is lost and what then runs; and where each
// pressed line goes. A fork is made, said of and taken away by the one
// route there is for it, and what the daemon refuses (a project that is
// not trusted, a fork there already, a fork changed since the question
// was asked) is said in its own sentence where it was asked, with the
// dialog staying.
//
// The page is told when a fork of its blueprint is made or taken away,
// by whoever did it and from wherever (`project.forked`,
// `project.unforked`), and when a package is added, which may bring
// another text of a default: what its Summary says of where its text
// stands is asked again.

import type { ComponentChildren } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { backAsked, follows, forkedSaid, mayFork, offersFork } from '../forked.ts';
import type { Owned, Where } from '../forked.ts';
import { isLibrary } from '../library.ts';
import { forgetThing } from '../latelykept.ts';
import { isMockups } from '../mockups.ts';
import { closeTab, collections, me, openTab, projectName, projects, tabKey, toast } from '../store.ts';
import type { Opens, Summary } from '../summary.ts';
import type { Tab } from '../tab.ts';
import { forgetTab } from '../tabsof.ts';
import { Modal, Prose, useEvents } from '../ui.tsx';

/** Makes a fork, says of one where it differs from what ships, or takes one away: the one route for all three. */
const own = (project: string, body: { blueprint: string; say?: boolean; back?: boolean; force?: boolean }) =>
  call<Owned>('POST', `/projects/${encodeURIComponent(project)}/own`, body);

const said = (err: unknown) => (err instanceof Error ? err.message : String(err));

/** The dialog that asks which project a blueprint is forked for, where its page is of none. */
function ForkFor({ name, onClose, onFork }: { name: string; onClose: () => void; onFork: (project: string) => Promise<string | null> }) {
  const [where, setWhere] = useState(projects.value[0]?.id ?? '');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const fork = async () => {
    if (busy || !where) return;
    setBusy(true);
    const refused = await onFork(where);
    // What the daemon would not do is said where it was asked, and the dialog stays.
    if (refused !== null) {
      setError(refused);
      setBusy(false);
    }
  };
  return (
    <Modal onClose={onClose}>
      <div class="stack forking" data-fork={name}>
        <h2>Fork {name}</h2>
        <p>
          Its text is copied into a project, where the copy is what runs under the name and is that project's to change. The default stays as it
          is, and the fork says what it was forked from.
        </p>
        {projects.value.length === 0 ? (
          <p class="faint">There is no project to fork it for yet.</p>
        ) : (
          <label class="stack forking-field">
            <span>For which project</span>
            <select aria-label="Project" value={where} onChange={(event) => setWhere(event.currentTarget.value)}>
              {projects.value.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>
          </label>
        )}
        {error && (
          <p class="error-text forking-refused">
            <Prose text={error} />
          </p>
        )}
        <div class="row modal-foot">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" disabled={busy || !where} onClick={() => void fork()}>
            Fork it
          </button>
        </div>
      </div>
    </Modal>
  );
}

/** The question asked before a fork is taken away: what goes, what then runs, and what is lost by it. */
function GoBack({ name, project, onClose, onGone }: { name: string; project: string; onClose: () => void; onGone: (gone: Owned) => void }) {
  const [found, setFound] = useState<Owned | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  // Asked of the daemon first, which changes nothing by being asked: what is said is what is so.
  useEffect(() => {
    let left = false;
    own(project, { blueprint: name, say: true }).then(
      (answer) => !left && setFound(answer),
      (err) => !left && setError(said(err)),
    );
    return () => {
      left = true;
    };
  }, [name, project]);
  const asked = found ? backAsked(found, projectName(project)) : null;
  const take = async () => {
    if (busy || !asked) return;
    setBusy(true);
    try {
      // Told to lose what the fork says only where the question said it would be lost.
      onGone(await own(project, { blueprint: name, back: true, force: asked.force }));
    } catch (err) {
      setError(said(err));
      setBusy(false);
    }
  };
  return (
    <Modal onClose={onClose}>
      <div class="stack forking" data-back={name}>
        <h2>Go back to the default?</h2>
        {!found && !error && <p class="faint">Reading where the fork differs from what ships</p>}
        {asked?.says.map((line) => (
          <p key={line}>{line}</p>
        ))}
        {asked && asked.lost.length > 0 && (
          <ul class="forking-lost">
            {asked.lost.map((line) => (
              <li key={line}>
                <Prose text={line} />
              </li>
            ))}
          </ul>
        )}
        {error && (
          <p class="error-text forking-refused">
            <Prose text={error} />
          </p>
        )}
        <div class="row modal-foot">
          <button type="button" onClick={onClose}>
            {asked ? 'Keep the fork' : 'Close'}
          </button>
          {asked && (
            <button class={asked.force ? 'danger' : 'primary'} disabled={busy} onClick={() => void take()}>
              {asked.confirm}
            </button>
          )}
        </div>
      </div>
    </Modal>
  );
}

export interface ForkingProps {
  /** What the blueprint is called, once its text has been read. */
  name: string | null;
  /** The project the page is of; none for one opened from the factory's list. */
  project: string | null;
  /** The page is of a blueprint that ships, as it ships. */
  ships: boolean;
  summary: Summary | null | undefined;
  /** Applies it, asking what it is to be told: what the page's own Apply does. */
  apply: () => void;
}

/**
 * What a blueprint's page has of forking: the button for its head, the
 * dialogs, what pressing a line of its Summary does, and something that
 * is another thing once a fork of it was made or taken away.
 */
export function useForking(props: ForkingProps): { button: ComponentChildren; fork: (() => void) | null; dialogs: ComponentChildren; follows: (opens: Opens) => (() => void) | null; stood: number } {
  const { name, project } = props;
  const [asking, setAsking] = useState<null | 'project' | { back: string }>(null);
  const [stood, setStood] = useState(0);
  // And when a package is added, which may bring another text of the default a fork was made from.
  useEvents(
    ['project.forked', 'project.unforked', 'project.blueprints', 'extension.added'],
    (event) => (event.topic === 'extension.added' || event.topic === 'project.blueprints' || event.data?.blueprint === name) && setStood((now) => now + 1),
    [name],
  );
  const scopes = me.value?.principal.scopes ?? [];
  const tables: Where['tables'] = {};
  for (const one of project ? (collections.value[project] ?? []) : []) tables[one.name] = one.name === 'board' ? 'board' : isLibrary(one) ? 'library' : isMockups(one) ? 'mockups' : 'table';

  // Makes the fork and opens it where it is written; what the daemon refuses comes back as its sentence.
  const fork = async (where: string): Promise<string | null> => {
    if (!name) return null;
    try {
      const made = await own(where, { blueprint: name });
      toast(forkedSaid(made, projectName(where)));
      setAsking(null);
      if (made.path) openTab({ kind: 'file', project: where, path: made.path });
      return null;
    } catch (err) {
      return said(err);
    }
  };
  const forking = () => {
    // For the project the page is of; one opened for none asks which.
    if (project) void fork(project).then((refused) => refused !== null && toast(refused, true));
    else setAsking('project');
  };
  const gone = (was: Owned, of: string) => {
    setAsking(null);
    toast(`${was.removed ?? 'The fork'} is taken away: ${was.blueprint} as it ships is what runs in ${projectName(of)} again, once what is running is applied.`);
    // The file is gone: its tab is closed wherever it is kept, and the default is shown in its place.
    if (was.removed) {
      const tab: Tab = { kind: 'file', project: of, path: was.removed };
      forgetThing(tab);
      forgetTab(tabKey(tab));
      closeTab(tabKey(tab), true);
    }
    openTab({ kind: 'blueprint', name: was.blueprint, project: of });
  };

  // Whether it is offered is known once the text has checked; until then a blueprint
  // that ships has the button closed in its place, so that the head does not move for
  // the answer under a pointer. Where it turns out not to be offered it goes then.
  const button =
    name && offersFork({ ships: props.ships, scopes, summary: props.summary }) ? (
      <button class="fork-it" onClick={forking} title="Copy its text into a project, where the copy is what runs under the name and is the project's to change">
        Fork
      </button>
    ) : props.ships && !props.summary && mayFork(scopes) ? (
      <button class="fork-it" disabled title="Once its text has been checked: copy it into a project">
        Fork
      </button>
    ) : null;
  const dialogs = (
    <>
      {asking === 'project' && name && <ForkFor name={name} onClose={() => setAsking(null)} onFork={fork} />}
      {asking !== null && asking !== 'project' && project && (
        <GoBack name={asking.back} project={project} onClose={() => setAsking(null)} onGone={(was) => gone(was, project)} />
      )}
    </>
  );
  return {
    button,
    // What the button does, for what else on the page offers it.
    fork: button ? forking : null,
    dialogs,
    stood,
    follows: (opens) => {
      const found = follows(opens, { project, scopes, tables });
      if (!found) return null;
      if (found.does === 'open') return () => openTab(found.tab);
      if (found.does === 'apply') return props.apply;
      if (found.does === 'fork') return forking;
      return () => setAsking({ back: found.name });
    },
  };
}
