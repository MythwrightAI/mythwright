// Move to… of a wiki's page: a small dialog with a box, and
// under it where the page could go, as `movesFor` says (`wikis.ts`): the
// top, where it is a wiki of its own, and every page of its workspace's
// wikis at its depth, narrowed by what is typed, the page itself and what
// is under it not offered, and where it is now marked and nothing to
// press. Pressed, it is one move to the end of that page. A list of every
// page is too long for a menu beside a line once a wiki has some hundreds,
// so it is a dialog with a box.
//
// A wiki may go to another workspace too, with every page under it: each
// other workspace is a line under a heading, and pressing one asks the
// question a project's move asks (`MoveTo` of `spaces.tsx`), which says
// what goes with it. What the daemon refuses is said in its sentence, with
// the dialog staying.

import { useState } from 'preact/hooks';
import { says, wordsOf } from '../blueprintlist.ts';
import { TOPICS } from '../folds.ts';
import { workspaces } from '../spaces.ts';
import { projects } from '../store.ts';
import { whereSaid } from '../topicpage.ts';
import type { TopicLine } from '../topicpage.ts';
import { readTopics, topics, topicsApi } from '../topicsread.ts';
import { Modal, useFocus } from '../ui.tsx';
import { movesFor, subtreeOf, wikiQuestion } from '../wikis.ts';
import type { Move } from '../wikis.ts';
import { spaceOfTopic } from '../workspaces.ts';
import { unfolded } from './outline.tsx';
import { askMoving } from './spaces.tsx';
import { topicsListedNow } from './topics.tsx';

/** What the daemon said, as a page says it: without the marks a terminal reads commands by. */
const said = (refused: unknown) => (refused instanceof Error ? refused.message : String(refused)).replaceAll('`', '');

/** What the top is said as among the places a page could go. */
const TOP = 'A wiki of its own';

export function WikiMoved({ page, onClose }: { page: TopicLine; onClose: () => void }) {
  const [typed, setTyped] = useState('');
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useFocus<HTMLInputElement>();
  const { tree } = topicsListedNow();
  const titleOf = (id: string) => topics.value.find((one) => one.id === id)?.title ?? id;
  const words = wordsOf(typed);
  const offered = movesFor(tree, page.id).filter((move) => words.length === 0 || says(words, [move.kind === 'top' ? TOP : titleOf(move.id)]));
  const wiki = (tree.above.get(page.id) ?? null) === null;
  const under = Math.max(0, subtreeOf(tree, page.id).length - 1);
  const here = spaceOfTopic(page, workspaces.value, projects.value)?.id ?? null;
  const elsewhere = wiki ? workspaces.value.filter((one) => one.id !== here && (words.length === 0 || says(words, [one.name]))) : [];
  const go = async (move: Move) => {
    if (move.here || busy) return;
    setBusy(true);
    setWrong(null);
    try {
      const into = move.kind === 'top' ? null : move.id;
      await topicsApi.move(page.id, into, null);
      await readTopics();
      // Where it went is unfolded, so that it is in view there.
      if (into) unfolded(TOPICS.page(into));
      onClose();
    } catch (refused) {
      setBusy(false);
      setWrong(said(refused));
    }
  };
  const toSpace = (space: { id: string; name: string }) => {
    onClose();
    askMoving(wikiQuestion(page.title, under, space.name, whereSaid(page, projects.value)), async () => {
      await topicsApi.place(page.id, space.id);
      await readTopics();
    });
  };
  const first = offered.find((move) => !move.here);
  return (
    <Modal onClose={onClose}>
      <h2>Move {page.title} to…</h2>
      <div class="stack wiki-moved">
        <input
          ref={input}
          type="search"
          aria-label="Find a page"
          placeholder="Find a page"
          value={typed}
          onInput={(event) => {
            setTyped(event.currentTarget.value);
            setWrong(null);
          }}
          onKeyDown={(event) => {
            // Enter takes the first place offered.
            if (event.key === 'Enter' && first) {
              event.preventDefault();
              void go(first);
            }
          }}
        />
        <div class="wiki-moves">
          {offered.map((move) => (
            <button
              key={move.kind === 'top' ? 'top' : move.id}
              type="button"
             
              class="wiki-move"
              style={move.kind === 'page' ? `--depth:${move.depth}` : undefined}
              aria-current={move.here || undefined}
              aria-disabled={move.here || busy || undefined}
              title={move.here ? 'It is here already' : undefined}
              onClick={() => void go(move)}
            >
              <span class="truncate">{move.kind === 'top' ? TOP : titleOf(move.id)}</span>
              {move.here && <span class="nav-hint">here now</span>}
            </button>
          ))}
          {elsewhere.length > 0 && <p class="faint wiki-moves-head">Another workspace{under === 1 ? ', with the page under it' : under > 1 ? `, with the ${under} pages under it` : ''}</p>}
          {elsewhere.map((space) => (
            <button key={`space-${space.id}`} type="button" class="wiki-move" onClick={() => toSpace(space)}>
              <span class="truncate">{space.name}</span>
            </button>
          ))}
          {offered.length === 0 && elsewhere.length === 0 && <p class="faint">Nothing here says “{typed.trim()}”.</p>}
        </div>
        {wrong && (
          <p role="alert" class="kept-wrong">
            {wrong}
          </p>
        )}
        <div class="row modal-foot" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </Modal>
  );
}
