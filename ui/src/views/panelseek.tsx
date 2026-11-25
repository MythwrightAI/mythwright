// The box under the head of each side panel that lists things (Resources,
// Channels, Topics, the inbox), and what the Resources panel hands it.
//
// What is typed narrows everything the panel lists, and is worked out in
// `panelsought.ts` from what the panel has already read: nothing is asked
// of the daemon for it. It is kept here, outside the panel's view, so
// that it is there when the panel comes back after another was shown;
// for each panel apart, and only for whoever typed it, and in no browser
// store, so that it is gone when the page is read again. No fold is
// changed by it: emptied, by hand or by Escape, every fold is as it was.
//
// It is the panel's one box, and searching everything is reached from it:
// the keys that open that search are said at its right while it is empty,
// and pressed open it; what is typed is offered under it to be searched
// for everywhere, which opens that search with it already written, as
// Enter does. Ctrl and K open the same search from anywhere, and from
// here with what is typed in it.

import { pipelinesOf } from '../pipelines.ts';
import { signal } from '@preact/signals';
import { createContext } from 'preact';
import { held } from '../chosen.ts';
import { linesOf, sought } from '../blueprintlist.ts';
import type { Listed as Blueprint } from '../blueprintlist.ts';
import { pinnedNow, projectOf, recentNow } from '../lately.ts';
import { knownNow, lately } from '../latelykept.ts';
import { namesOf } from '../envsheld.ts';
import { gitListed } from '../gitpage.ts';
import { outlineOf } from '../outlined.ts';
import { anyShown, everywhere, keptItems, memberItems, MOST_FILES, narrowed, nothingFound, projectItem, seekHint } from '../panelsought.ts';
import { everybody } from './direct.tsx';
import { touched } from '../phone.ts';
import type { Found, Item } from '../panelsought.ts';
import { workspaces } from '../spaces.ts';
import { callable, collections, hasBoard, librariesOf, me, mockupsOf, projects, searchEverything, tablesOf, trees, used } from '../store.ts';
import { tableCalled } from '../called.ts';
import { tabKey as tabKeyOf } from '../tab.ts';
import { titled } from '../table.ts';
import { listed as usedBy } from '../uses.ts';
import { tabTitle } from '../workbench.tsx';
import { listed as listedIn } from '../workspaces.ts';

export type Panel = 'resources' | 'channels' | 'topics' | 'inbox';

/** What was typed in each panel's box, and by whom. */
const typed = signal<Partial<Record<Panel, { by: string; text: string }>>>({});

/** What is typed in a panel's box now: nothing where somebody else typed it, who was signed in on this page before. */
/** What a table is called for people (`called.ts`): it is found by that and by its name. */
function calledOf(project: string, name: string): string {
  const table = (collections.value[project] ?? []).find((one) => one.name === name);
  return table ? tableCalled(table) : titled(name);
}

export function typedIn(panel: Panel): string {
  const one = typed.value[panel];
  return one && one.by === (me.value?.actor ?? '') ? one.text : '';
}

/** How many of each project's files found are drawn, for what is typed now: pressing the line after them draws more. */
const drawnMost = signal<{ text: string; of: Record<string, number> }>({ text: '', of: {} });

/** How many files found under a project's Repo are drawn while `text` is typed. */
export function filesMost(project: string, text: string): number {
  const now = drawnMost.value;
  return (now.text === text ? now.of[project] : undefined) ?? MOST_FILES;
}

/** Draws the next so many files found under a project. */
export function drawMore(project: string, text: string): void {
  const now = drawnMost.value;
  const of = now.text === text ? now.of : {};
  drawnMost.value = { text, of: { ...of, [project]: filesMost(project, text) + MOST_FILES } };
}

function type(panel: Panel, text: string): void {
  typed.value = { ...typed.value, [panel]: { by: me.value?.actor ?? '', text } };
}

/** What each panel's box says it looks in, which is no more than what the panel has read. */
const LOOKS: Record<Panel, string> = {
  resources: 'Search this panel',
  channels: 'Search these channels',
  topics: 'Search these topics',
  inbox: 'Search the inbox',
};

/** Whether the keys in front of whoever reads are a Mac's, which say ⌘ for Ctrl. */
const mac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

/** The box, directly under a panel's head, and under it while something is typed the way to search everything for that. */
export function PanelSeek({ panel }: { panel: Panel }) {
  const text = typedIn(panel);
  const hint = seekHint(text, touched, mac);
  const offered = everywhere(text);
  return (
    <div class={touched ? 'panel-seek' : 'panel-seek keyed'}>
      <div class="panel-seek-box">
        <input
          type="search"
          aria-label={LOOKS[panel]}
          placeholder={LOOKS[panel]}
          title="Narrows what this panel lists to what says every word typed. Nothing more is read for it: a project's files are looked in as far as they were read. Enter searches everything for it."
          value={text}
          onInput={(event) => type(panel, event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === 'Escape' && text !== '') {
              // Emptying the box is all the key does while there is something in it.
              event.stopPropagation();
              type(panel, '');
            } else if (text.trim() !== '' && (event.key === 'Enter' || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k'))) {
              // What is typed here is handed on to the search of everything, which the frame would open empty.
              event.preventDefault();
              event.stopPropagation();
              searchEverything(text.trim());
            }
          }}
        />
        {/* Always in the same place, and room kept for it whether or not it is drawn, so that nothing moves as it comes and goes. */}
        {hint && (
          <button class="panel-seek-keys kbd" aria-label="Search everything" title={`Search everything (${hint})`} onClick={() => searchEverything()}>
            {hint}
          </button>
        )}
      </div>
      {offered && (
        <button class="nav-item panel-everywhere" onClick={() => searchEverything(text.trim())}>
          <span class="glyph" aria-hidden="true">⌕</span>
          {/* Wrapped, never cut: the words typed are what the line is about, and are shown whole however narrow the panel. */}
          <span class="panel-everywhere-says">
            Search everything for <span class="panel-everywhere-words">{offered.words}</span>
          </span>
        </button>
      )}
    </div>
  );
}

/** What a panel says where nothing it lists says what was typed. */
export function NothingFound({ text, files }: { text: string; files?: boolean }) {
  return (
    <p class="nav-note panel-none" role="status">
      {nothingFound(text)}
      {files && " A project's files are looked in as far as the page has read them."}
    </p>
  );
}

/** What the panel being drawn is held to: nothing while nothing is typed in its box. */
export const Sought = createContext<Found | null>(null);

/** The Factory's lines, as what is found knows them. */
export const FACTORY = { activity: 'factory:activity', metrics: 'factory:metrics', trash: 'factory:trash', reference: 'reference' };
export const REFERENCE_SAYS = 'Every word of the language, every action and every event, as this program describes them';

/** What the Resources panel lists, in the order it draws it (`SECTIONS`), held to what was typed: everything but the factory's blueprints, which `sought` holds. */
export function resourcesFound(text: string): { found: Found | null; blueprints: number; any: boolean } {
  // Nothing typed reads nothing: the panel is as it always was.
  if (text.trim() === '') return { found: null, blueprints: 0, any: true };
  const blueprints = (callable.value[''] ?? []) as Blueprint[];
  const listedBlueprints = linesOf(sought(blueprints, text));
  const [kept, known, now, all] = [lately.value, knownNow.value, held.value, workspaces.value];
  const named = (id: string | null | undefined) => projects.value.find((project) => project.id === id)?.name ?? id ?? null;
  const line = (thing: Parameters<typeof tabTitle>[0]) => ({ key: tabKeyOf(thing), title: tabTitle(thing).title, whose: named(projectOf(thing, known)) });
  const said = new Map(blueprints.map((one) => [one.name, one.description]));
  const items: Item[] = [
    ...keptItems(pinnedNow(kept, known, now, all).map(line), recentNow(kept, known, now, all).map(line)),
    ...listedIn(projects.value, all, now).map((project) =>
      projectItem({
        id: project.id,
        name: project.name,
        board: hasBoard(project.id),
        tables: tablesOf(project.id).map((name) => ({ name, label: calledOf(project.id, name) })),
        libraries: librariesOf(project.id).map((name) => ({ name, label: titled(name) })),
        mockups: mockupsOf(project.id).map((name) => ({ name, label: titled(name) })),
        blueprints: usedBy(used.value[project.id]).map((one) => ({ name: one.name, description: said.get(one.name) })),
        files: project.folder !== 'own' ? (trees.value[project.id] ?? []) : null,
        environments: namesOf(project.id),
        git: gitListed(project),
        pipelines: pipelinesOf(used.value[project.id]).length > 0,
        outline: outlineOf(project.id),
      }),
    ),
    { id: FACTORY.activity, says: ['Activity'] },
    { id: FACTORY.metrics, says: ['Measurements'] },
    // Everybody here and the agents of the workspace, under the Members line (`direct.tsx`), found by name and by the words beside each.
    ...memberItems(everybody.value),
    { id: FACTORY.trash, says: ['Trash', 'deleted'] },
    { id: FACTORY.reference, says: ['Reference', REFERENCE_SAYS] },
  ];
  const found = narrowed(items, text);
  return { found, blueprints: listedBlueprints, any: listedBlueprints > 0 || anyShown(found) };
}
