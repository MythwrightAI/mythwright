// The blueprints a project uses, under its line in the side panel: its
// own first, each the file it is where the factory keeps it
// (`@blueprints/`) or in a folder of its repository it said, with the
// files of its own it links under it; then what else is applied there and
// alive. Whoever may write the project's files makes a new one of its own
// from there (a line of its own under them) and from the gear of the
// project's page,
// by a dialog that names it (`ownmade.ts`), and it opens to be written
// in. A file of `@blueprints/` says where it is by leading back here
// (`showOwnBlueprints`): it has no folder of the repository to open.

import type { ComponentChildren } from 'preact';
import { signal } from '@preact/signals';
import { Fragment } from 'preact';
import { useContext, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { projectFold, SECTIONS, unfoldedAll } from '../folds.ts';
import { blueprintNamed, firstBlueprint, linkCalled } from '../ownmade.ts';
import { holds, ofProject, shows } from '../panelsought.ts';
import { callable, learnUsed, openTab, projectName, side, turned, turnedTo, used } from '../store.ts';
import { Modal, useFocus } from '../ui.tsx';
import { listed } from '../uses.ts';
import type { Offered } from './grouped.tsx';
import { Fold, NavItem } from './navtree.tsx';
import { Sought } from './panelseek.tsx';
import { mayMake } from './repomade.tsx';
import { signs } from './signs.tsx';

/** What the project's Blueprints line is kept as, folded or not. */
const foldOf = (project: string) => `project:${project}:blueprints`;

/** The project a new blueprint of its own is being named for, while the dialog is open. */
const naming = signal<string | null>(null);

/** Where a new blueprint of `project`'s own is written, for whoever may write its files: nothing where it keeps none, or they may not. */
function writtenIn(project: string): string | null {
  if (mayMake().length === 0) return null;
  return used.value[project]?.written ?? null;
}

/** The line that makes a new blueprint of `project`'s own, wherever it is offered; none where it cannot be made. */
export function newBlueprintLine(project: string): Offered | null {
  if (!writtenIn(project)) return null;
  return { label: 'New blueprint…', mark: signs.plus(), act: () => (naming.value = project) };
}

/** Shows `project`'s blueprints in the side panel, unfolded and in view: where a file of the factory's place for them says it is. */
export function showOwnBlueprints(project: string): void {
  side.value = 'resources';
  turnedTo(unfoldedAll(turned.value, [SECTIONS.projects, projectFold(project), foldOf(project)]));
  requestAnimationFrame(() => {
    const lines = document.querySelectorAll<HTMLElement>(`[data-project="${CSS.escape(project)}"] .tree button.nav-item`);
    const line = [...lines].find((one) => one.textContent?.trim() === 'Blueprints');
    line?.scrollIntoView({ block: 'nearest' });
    line?.focus();
  });
}

/** The project's Blueprints line, where it uses any. */
export function Blueprints({ project, more, place, label = 'Blueprints' }: { project: string; more?: ComponentChildren; place?: string; label?: string }) {
  const found = useContext(Sought);
  const lines = listed(used.value[project]).filter((one) => shows(found, ofProject.blueprint(project, one.name)));
  if (lines.length === 0 || !shows(found, ofProject.blueprints(project))) return null;
  // What each is for, where the factory's list has been read and says.
  const said = new Map((callable.value[''] ?? []).map((one) => [one.name, one.description]));
  // A line that does something and is no thing of the project's: not among what a search finds.
  const made = found === null ? newBlueprintLine(project) : null;
  return (
    <Fold at={foldOf(project)} label={label} place={place} held={holds(found, ofProject.blueprints(project))} more={more}>
      {lines.map((one) => (
        <Fragment key={one.name}>
          <NavItem
            tab={one.file ? { kind: 'file', project, path: one.file } : { kind: 'blueprint', name: one.name, project }}
            glyph="◆"
            label={one.name}
            title={said.get(one.name) ?? undefined}
          />
          {one.links.length > 0 && (
            <div class="tree">
              {one.links.map((link) => (
                <NavItem key={link} tab={{ kind: 'file', project, path: link }} glyph="¶" label={linkCalled(one.name, link)} title={link} />
              ))}
            </div>
          )}
        </Fragment>
      ))}
      {/* Its own line, apart from the blueprints listed, so that it is never taken for one of them. */}
      {made && (
        <div class="own-new">
          <button class="nav-item" onClick={() => made.act()}>
            <span class="glyph">+</span>
            <span class="truncate">{made.label}</span>
          </button>
        </div>
      )}
    </Fold>
  );
}

/** The dialog that names a new blueprint of a project's own, where one is being made. */
export function AskNewBlueprint() {
  const project = naming.value;
  return project ? <Naming key={project} project={project} onClose={() => (naming.value = null)} /> : null;
}

function Naming({ project, onClose }: { project: string; onClose: () => void }) {
  const [name, setName] = useState('');
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const written = used.value[project]?.written ?? '@blueprints';
  const has = listed(used.value[project]).filter((one) => one.file).map((one) => one.name);
  const ships = (callable.value[''] ?? []).map((one) => one.name);
  const came = blueprintNamed(name, written, has, ships);
  const make = async (event: Event) => {
    event.preventDefault();
    if (busy || !('path' in came)) return;
    setBusy(true);
    setWrong(null);
    try {
      const entry = await api.newFile(project, came.path, firstBlueprint(came.name));
      await learnUsed();
      if (!turned.value.has(foldOf(project))) turnedTo(new Set([...turned.value, foldOf(project)]));
      onClose();
      openTab({ kind: 'file', project, path: entry.path });
    } catch (refused) {
      setBusy(false);
      setWrong((refused instanceof Error ? refused.message : String(refused)).replaceAll('`', ''));
    }
  };
  const typed = name.trim() !== '';
  return (
    <Modal onClose={onClose}>
      <h2>A new blueprint of {projectName(project)}'s own</h2>
      <form class="stack repo-made" onSubmit={(event) => void make(event)}>
        <label>
          Name
          <input
            ref={input}
            value={name}
            placeholder="release"
            aria-describedby="own-made-says"
            onInput={(event) => {
              setName(event.currentTarget.value);
              setWrong(null);
            }}
          />
        </label>
        <p id="own-made-says" class="faint repo-made-says">
          {typed && 'wrong' in came
            ? came.wrong
            : `It is kept by the factory for ${projectName(project)}, with one step that does nothing, and opens to be written in. A name may have folders: tools/build.`}
        </p>
        {wrong && (
          <p role="alert" class="repo-made-wrong">
            {wrong}
          </p>
        )}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy || !('path' in came)}>
            {busy ? 'Making it' : 'Make the blueprint'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
