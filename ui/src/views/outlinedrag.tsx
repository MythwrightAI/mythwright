// A project's things carried to another place in its tree, by the pointer
// and by the keyboard. The carry itself is a tree's and
// no project's (`treedrag.tsx`, over `landing` and `stepped` in
// `treedrag.ts`); this is what a project's outline hands it. Only for
// whoever may arrange the project, and only of what its outline can place:
// a thing's line, a folder's line; never what is under the Repo or
// Blueprints, which is listed by the disk and by what the project runs.
// Each landing is one `put`; a step by the keyboard goes by what is drawn
// (`drawnOf`), and where it lands in a folder that folder is unfolded so
// that the line keeping the keyboard is in view.

import type { Project } from '../api.ts';
import { outlineFold, projectFold } from '../folds.ts';
import { stepped } from '../outline.ts';
import type { At } from '../outline.ts';
import { drawnOf, mayArrange, moveTo, unfolded } from './outline.tsx';
import { useTreeCarry } from './treedrag.tsx';

/** Moves `node` where a carry or a step says, unfolding what it went into where the keyboard took it there. */
async function put(project: Project, node: string, to: At, unfold: boolean) {
  const came = await moveTo(drawnOf(project), node, to);
  if (came && unfold && to.into) unfolded(projectFold(project.id), outlineFold(project.id, to.into));
  return came;
}

/**
 * What a project's tree is handed so that its lines can be carried: the
 * press that may begin a carry, the click a carry swallows, and the
 * keyboard's steps. Nothing where whoever reads it may not arrange it.
 */
export function useCarried(project: Project) {
  return useTreeCarry(
    mayArrange()
      ? {
          land: (node, to, by) => put(project, node, to, by === 'keys'),
          step: (node, way) => stepped(drawnOf(project).laid, node, way),
        }
      : null,
  );
}
