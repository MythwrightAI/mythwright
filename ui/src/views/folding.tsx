// The two buttons at the head of the Resources panel, and of the Channels
// panel, that fold, and unfold, every fold of the panel at once
// (`folds.ts`).
//
// Collapse all forgets every fold of the panel that is kept, which is
// everything folded, since all of it starts folded. Expand all unfolds
// every fold of what was read: the panel's parts, each project it lists,
// what each of those has that folds, and the factory's blueprints. A
// project's files whose folder was not looked at on this page stay
// folded, so that it asks the daemon for nothing: unfolded, each would
// have the daemon look at its project's folder (`lookAt`).
//
// In the Channels panel Collapse all forgets every fold of that panel,
// which folds every branch and every channel, and Expand all unfolds every
// branch it draws (what is pinned, each group, each project's channels and
// what is archived) and every channel it lists with who is in it, so that
// all of it is in view: both apply to the channels too.

import { blueprintFolds } from '../blueprintlist.ts';
import type { Listed } from '../blueprintlist.ts';
import { held } from '../chosen.ts';
import { channelsUnfolded, foldedAll, ofChannels, ofTopicsPanel, resourcesFolds, topicsUnfolded, unfoldedAll } from '../folds.ts';
import { foldersOf } from '../outline.ts';
import { outlineOf } from '../outlined.ts';
import { workspaces } from '../spaces.ts';
import { callable, hasLooked, projects, trees, turned, turnedTo, used } from '../store.ts';
import { listed } from '../uses.ts';
import { listed as listedIn } from '../workspaces.ts';
import { channelsDrawn } from './grouped.tsx';
import { topicsListedNow } from './topics.tsx';
import { holding } from '../wikis.ts';
import { signs } from './signs.tsx';

/** Every page of the wikis the Topics panel lists that holds pages: what its Expand all unfolds. */
function topicsHolding(): string[] {
  const { listed, closed } = topicsListedNow();
  return holding([...listed, ...closed]);
}

function expandAll(): void {
  const read = listedIn(projects.value, workspaces.value, held.value).map((project) => ({
    id: project.id,
    blueprints: listed(used.value[project.id]).length > 0,
    files: project.folder !== 'own' && hasLooked(project.id) ? (trees.value[project.id] ?? null) : null,
    folders: foldersOf(outlineOf(project.id)?.items ?? []),
  }));
  turnedTo(unfoldedAll(turned.value, resourcesFolds(read, blueprintFolds((callable.value[''] ?? []) as Listed[]))));
}

/** Collapse all and Expand all, beside the panel's name: of the Resources panel unless it is said to be of the Channels panel or the Topics panel. */
export function FoldAll(props: { of?: 'channels' | 'topics' }) {
  const collapse = () => turnedTo(foldedAll(turned.value, props.of === 'channels' ? ofChannels : props.of === 'topics' ? ofTopicsPanel : undefined));
  const expand = () =>
    props.of === 'channels'
      ? turnedTo(channelsUnfolded(turned.value, channelsDrawn()))
      : props.of === 'topics'
        ? turnedTo(topicsUnfolded(turned.value, topicsListedNow().closed.length > 0, topicsHolding()))
        : expandAll();
  return (
    <>
      <button class="ghost icon brand-act" aria-label="Collapse all" title="Collapse all" onClick={collapse}>
        {signs.foldAll()}
      </button>
      <button class="ghost icon brand-act" aria-label="Expand all" title="Expand all" onClick={expand}>
        {signs.unfoldAll()}
      </button>
    </>
  );
}
