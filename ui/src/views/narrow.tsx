// Whose things the factory's views show: all of the workspace the side
// panel is of, one project of it, or every workspace.
//
// The same choice is in the head of the activity, the measurements and
// the agents (the Members page's Agents tab), and choosing in one chooses in all three (`narrowed.ts`),
// so that they are always of the same thing. It is of the workspace the
// panel is of: another workspace is chosen at the panel's foot, and what
// was chosen here is kept for each.

import { chosenBy, valueOf } from '../narrowing.ts';
import type { Line } from '../narrowing.ts';
import { choice, choose, offered } from '../narrowed.ts';

const line = (one: Line) => (
  <option key={one.value} value={one.value}>
    {one.label}
  </option>
);

/**
 * `more` is whether the page has something to show that is no project's
 * of this workspace (what was applied for no project, the factory's own
 * log and figures): with one workspace, every workspace is a line only
 * where it would show that.
 */
export function Narrow({ more = true }: { more?: boolean }) {
  const lines = offered(more);
  // Until it is known which workspace the panel is of there is nothing to choose within, and with one line there is nothing to choose.
  if (!lines || (lines.projects.length === 0 && !lines.everything)) return null;
  const called = lines.space.label;
  return (
    <select
      class="narrow"
      aria-label={`What is shown of ${called}`}
      title={`All of ${called}, one project of it${lines.everything ? ', or every workspace' : ''}. Another workspace is chosen at the foot of the side panel.`}
      value={valueOf(choice.value)}
      onChange={(event) => choose(chosenBy(event.currentTarget.value, lines))}
    >
      {line(lines.space)}
      {lines.projects.length > 0 && <optgroup label="Projects">{lines.projects.map(line)}</optgroup>}
      {/* Set apart: it is the one line that is of more than this workspace. */}
      {lines.everything && <hr />}
      {lines.everything && line(lines.everything)}
    </select>
  );
}
