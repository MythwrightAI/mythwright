// A project in the side panel, with what it has under it: drawn as its
// outline says where somebody arranged it (`outline.ts`, read by
// `outlined.ts`), in folders of the outline's own, and as it always was
// where nobody did. A thing is drawn the same wherever it stands: its
// line, what folds under it and what it opens are as they always were.

import type { ComponentChildren } from 'preact';
import { useContext } from 'preact/hooks';
import type { Entry, Project } from '../api.ts';
import { openedByName, outlineFold, projectFold } from '../folds.ts';
import { GIT_GLYPH } from '../gitpage.ts';
import { keysOf, laidOut, usualOf } from '../outline.ts';
import type { Laid } from '../outline.ts';
import { outlineOf, thingShown } from '../outlined.ts';
import { drawnFirst, filesBounded, holds, moreSaid, ofProject, shows } from '../panelsought.ts';
import { active, collections, isBlueprint, lookAt, openTab, tabKey, trees, turn, turned, turnedTo } from '../store.ts';
import type { Tab } from '../store.ts';
import { tableCalled } from '../called.ts';
import { titled } from '../table.ts';
import { NotTrusted, Twist } from '../ui.tsx';
import { classes, parentOf } from '../util.ts';
import { names } from '../inplace.ts';
import { More } from './grouped.tsx';
import { Fold, NavItem } from './navtree.tsx';
import { Blueprints } from './ownblueprints.tsx';
import { drawMore, filesMost, Sought, typedIn } from './panelseek.tsx';
import { makeLines, mayMake } from './repomade.tsx';
import { projectMenu } from './spaces.tsx';
import { folderLines, listingOf, mayArrange, thingLines } from './outline.tsx';
import { useCarried } from './outlinedrag.tsx';

/**
 * A project in the sidebar: its name, which opens its own page, and under
 * it what the project has. Its board, once a blueprint has made it one,
 * and whatever else a blueprint keeps for it (tables, libraries,
 * mockups); its environments; its pipelines; the blueprints it uses, once it uses any;
 * its Git page and its files where its folder is one whoever added it
 * said. Each is directly under the project, or in a folder of its
 * outline where somebody put it there. The arrow beside the name folds
 * all of it away.
 */
export function ProjectTree({ project, more }: { project: Project; more?: ComponentChildren }) {
  const { id, name } = project;
  const found = useContext(Sought);
  // Its lines carried to another place, by whoever may arrange it (`outlinedrag.tsx`).
  const carried = useCarried(project);
  // What was typed in the box holds it open where something under it was found, whatever its fold was left as.
  const hold = holds(found, ofProject.project(id));
  // It starts folded, as everything of the panel does: what is kept is that it was unfolded.
  const open = hold || turned.value.has(projectFold(id));
  const home: Tab = { kind: 'project', id };
  // What it lists today, in the order it always did, and those laid out as its outline says.
  const listing = listingOf(project);
  const laid = laidOut(keysOf(listing), outlineOf(id));
  // Whether anything is listed under it: one added alone, in a folder of the factory's making, has nothing yet, and no arrow that would unfold nothing.
  const any = laid.length > 0;
  // Its name opens its page and unfolds it, so that what it has is in view under it: the row stays where it is and
  // what it has comes below it. Its arrow is what folds it again.
  const opened = () => {
    const next = any ? openedByName(turned.value, projectFold(id), hold) : null;
    if (next) turnedTo(next);
    openTab(home);
  };
  return (
    <div data-project={id}>
      <div class={classes('nav-row', active.value === tabKey(home) && 'current')} onContextMenu={projectMenu(project)}>
        {any ? (
          <button class="nav-twist" aria-label={open ? `Fold ${name}` : `Unfold ${name}`} aria-expanded={open} aria-disabled={hold || undefined} onClick={() => !hold && turn(projectFold(id))}>
            <Twist open={open} />
          </button>
        ) : (
          <span class="nav-twist" />
        )}
        <button class="nav-item" aria-current={active.value === tabKey(home)} data-names={names(home)} onClick={() => opened()}>
          <span class="truncate" style="font-weight:600;color:var(--text)">
            {name}
          </span>
          <NotTrusted project={id} brief />
        </button>
        {more}
      </div>
      {open && any && (
        <div class="tree" {...carried}>
          {/* Nothing here is gathered under a word for what the project is: one may be a shop's code or the research
              for a book, and its board is of its work whichever it is. */}
          <Laids project={project} laid={laid} />
        </div>
      )}
    </div>
  );
}

/** The lines of a project as they are laid out: each thing, and each folder of its outline with what is in it. */
function Laids({ project, laid }: { project: Project; laid: Laid[] }) {
  const found = useContext(Sought);
  const { id } = project;
  return (
    <>
      {laid.map((one) =>
        'folder' in one ? (
          shows(found, ofProject.folder(id, one.folder)) && (
            <div key={`folder:${one.folder}`} data-outline={one.folder} data-place={one.folder}>
              <Fold
                at={outlineFold(id, one.folder)}
                label={one.name}
                held={holds(found, ofProject.folder(id, one.folder))}
                more={mayArrange() && <More label={`${one.name}: more`} items={() => folderLines(project, { id: one.folder, name: one.name })} />}
              >
                <Laids project={project} laid={one.items} />
                {one.items.length === 0 && !found && <span class="nav-note">Nothing here yet</span>}
              </Fold>
            </div>
          )
        ) : (
          <Thing key={one.thing} project={project} thing={one.thing} />
        ),
      )}
    </>
  );
}

/** One thing of a project, by its key, drawn as it always was, with what can be done with it at its line's end for whoever may arrange the project. */
function Thing({ project, thing }: { project: Project; thing: string }) {
  const line = <Line project={project} thing={thing} />;
  // Blueprints and the Repo fold, and their menus are their own lines' (`more`).
  if (thing === 'blueprints' || thing === 'repo' || !mayArrange()) return line;
  return <Lined project={project} thing={thing} line={line} />;
}

/** A thing's line with its menu after it: only where the line is drawn. */
function Lined({ project, thing, line }: { project: Project; thing: string; line: ComponentChildren }) {
  const found = useContext(Sought);
  if (!shows(found, ofProject.thing(project.id, thing))) return null;
  return (
    <div class="nav-line" data-place={thing}>
      {line}
      <More label={`${thingCalled(project, thing)}: more`} items={() => thingLines(project, thing)} />
    </div>
  );
}

/** What a thing's line says, for what its menu is called: what somebody named it, else what it always said. */
function thingCalled(project: Project, thing: string): string {
  if (thing.startsWith('table:')) return tableOf(project.id, thing.slice('table:'.length));
  return said(project.id, thing);
}

/** What a thing's line says that is not a table's: what somebody named it for everybody, else what it always said. */
const said = (project: string, thing: string): string => thingShown(project, thing) ?? usualOf(thing);

/** What a table of a project is called for people (`called.ts`). */
function tableOf(project: string, name: string): string {
  const table = (collections.value[project] ?? []).find((one) => one.name === name);
  return table ? tableCalled(table) : titled(name);
}

function Line({ project, thing }: { project: Project; thing: string }) {
  const found = useContext(Sought);
  const { id, name } = project;
  const shown = shows(found, ofProject.thing(id, thing));
  const [kind, called] = [thing.slice(0, thing.indexOf(':')), thing.slice(thing.indexOf(':') + 1)];
  if (thing === 'board') return shown ? <NavItem tab={{ kind: 'board', project: id }} glyph="▦" label={said(id, thing)} /> : null;
  // Whatever else a blueprint keeps for it as a table. One that says it sits beside another is not listed: it is reached from that one's page (`sits.ts`).
  if (kind === 'table') return shown ? <NavItem tab={{ kind: 'table', project: id, name: called }} glyph="▥" label={tableOf(id, called)} /> : null;
  // What is written about it, where a blueprint has made its documents a library.
  if (kind === 'library') return shown ? <NavItem tab={{ kind: 'library', project: id, name: called }} glyph="❏" label={said(id, thing)} /> : null;
  // Pages to try an idea with, where a blueprint has made a folder of them its mockups.
  if (kind === 'mockups') return shown ? <NavItem tab={{ kind: 'mockups', project: id, name: called }} glyph="◰" label={said(id, thing)} /> : null;
  // Its environments, where it says any: one it has none of is made from its own page.
  if (thing === 'environments') return shown ? <NavItem tab={{ kind: 'environments', project: id }} glyph="◇" label={said(id, thing)} /> : null;
  // Its pipelines, where it has any of its own: one with none is offered them from its own page's gear.
  if (thing === 'pipelines') return shown ? <NavItem tab={{ kind: 'pipelines', project: id }} glyph="▶" label={said(id, thing)} /> : null;
  if (thing === 'blueprints') return <Blueprints project={id} place="blueprints" label={said(id, thing)} more={mayArrange() ? <More label={`${name} blueprints: more`} items={() => thingLines(project, 'blueprints')} /> : undefined} />;
  // Its repository, where its folder is a git repository of its own: what git says of it and what the packages installed show of it.
  if (thing === 'git') return shown ? <NavItem tab={{ kind: 'git', project: id }} glyph={GIT_GLYPH} label={said(id, thing)} /> : null;
  if (thing === 'repo') return shown ? <Repo project={project} /> : null;
  return null;
}

/**
 * Its folder, whatever is in it, where whoever added it said a folder: one
 * the factory made for it is no line here, since nobody asked for files,
 * and is reached from the project's own page. The fold is remembered under
 * the word it was first kept by (`folds.ts`).
 */
function Repo({ project }: { project: Project }) {
  const { id, name } = project;
  const found = useContext(Sought);
  const key = `project:${id}`;
  // While something is typed, the files found are drawn so many at a time.
  const text = typedIn('resources');
  // What can be made in its repo, by whoever may write its files: offered on the Repo line and on each folder's.
  const making = mayMake().length > 0;
  const bounded = filesBounded(id, trees.value[id] ?? [], found, filesMost(id, text), drawnFirst(isBlueprint));
  return (
    <Fold
      at={`${key}:repo`}
      place="repo"
      label={said(id, 'repo')}
      tab={{ kind: 'repo', project: id }}
      called={`${name} repo`}
      held={holds(found, ofProject.files(id))}
      onOpen={() => lookAt(id)}
      more={(making || mayArrange()) && <More label={`More for ${name} repo`} items={() => [...makeLines(id, '', `${key}:repo`), ...thingLines(project, 'repo').map((line, at) => (at === 0 && making ? { ...line, apart: true } : line))]} />}
    >
      <Entries project={id} entries={trees.value[id] ?? []} under="" kept={bounded?.kept ?? null} />
      {/* A search that finds a great many files draws so many of them, and says how many more there are. */}
      {bounded && bounded.more > 0 && (
        <button class="nav-item nav-more" onClick={() => drawMore(id, text)}>
          <span class="glyph">…</span>
          <span class="truncate">{moreSaid(bounded.more)}</span>
        </button>
      )}
    </Fold>
  );
}

function Entries({ project, entries, under, kept }: { project: string; entries: Entry[]; under: string; kept: Set<string> | null }) {
  const found = useContext(Sought);
  const making = mayMake().length > 0;
  // While something is typed, only what was found and is within the bound is drawn (`filesBounded`).
  const here = entries.filter((entry) => parentOf(entry.path) === under && (kept ? kept.has(ofProject.entry(project, entry.path)) : shows(found, ofProject.entry(project, entry.path))));
  // Blueprints first: they are what runs the project.
  here.sort(drawnFirst(isBlueprint));
  return (
    <>
      {here.map((entry) =>
        entry.dir ? (
          <Fold
            key={entry.path}
            at={`project:${project}:folder:${entry.path}`}
            label={entry.name}
            held={holds(found, ofProject.entry(project, entry.path))}
            more={making && <More label={`More for ${entry.name}`} items={() => makeLines(project, entry.path, `project:${project}:folder:${entry.path}`)} />}
          >
            <Entries project={project} entries={entries} under={entry.path} kept={kept} />
          </Fold>
        ) : (
          <NavItem
            key={entry.path}
            tab={{ kind: 'file', project, path: entry.path }}
            glyph={isBlueprint(entry.path) ? '◆' : '·'}
            label={entry.name}
          />
        ),
      )}
      {here.length === 0 && !found && <span class="nav-note">Nothing here yet</span>}
    </>
  );
}
