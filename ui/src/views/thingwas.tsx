// What a page's head says of a project's thing that somebody showed as
// something else in the side panel (`outline.ts`, a thing's `name`):
// that name, where the head always said the usual word, and beside it,
// faintly, what the thing is, so that it can still be told (`Board ·
// table board`). Where nobody named it, the head is as it always was.

import { whatIs } from '../outline.ts';
import { thingShown } from '../outlined.ts';

/** What a page's head calls a thing of `project` (by its key): what somebody named it, else `usual`. */
export const headWord = (project: string, thing: string, usual: string): string => thingShown(project, thing) ?? usual;

/** Beside a page's head whose thing was named something else: what it is, faintly. Nothing where it was not. */
export function ThingWas({ project, thing }: { project: string; thing: string }) {
  if (!thingShown(project, thing)) return null;
  const was = whatIs(thing);
  return (
    <span class="faint thing-was" title={`${was}: renamed for everybody reading the project; blueprints and commands call it as before`}>
      {was}
    </span>
  );
}
