// What a line of something followed told of, on the line's own page: what
// it is of, pressed to open it, and the changes it told of, oldest first,
// each who made it, what it was and what that is called now. Read with the
// line, and again as it is told again.

import { useEffect, useState } from 'preact/hooks';
import { changeSaid, changesOf, placeOf, tabOf } from '../following.ts';
import type { Change } from '../following.ts';
import type { Line } from '../inbox.ts';
import { openTab } from '../store.ts';
import { ago } from '../util.ts';

export function FollowedChanges({ line }: { line: Line }) {
  const [changes, setChanges] = useState<Change[] | null>(null);
  useEffect(() => {
    let live = true;
    void changesOf(line.id).then(
      (read) => live && setChanges(read),
      () => live && setChanges([]),
    );
    return () => void (live = false);
  }, [line.id, line.raised]);
  const thing = placeOf(line.place);
  const tab = thing ? tabOf(thing) : null;
  const title = (line.says ?? '').split(': ')[0] || line.place;
  return (
    <section class="inbox-page-about followed-changes">
      <h2 class="inbox-page-label">What changed</h2>
      {tab && (
        <button type="button" class="ghost inbox-page-opens" title={`Open ${title}`} onClick={() => openTab(tab)}>
          <span class="inbox-page-opens-says">{title}</span>
          <span class="faint">{line.why === 'digest' ? 'what it sums up' : 'what you follow'}</span>
        </button>
      )}
      {changes === null ? (
        <span class="faint">Reading it</span>
      ) : changes.length === 0 ? (
        <span class="faint">Nothing it told of is there to list now.</span>
      ) : (
        <ol class="followed-change-list">
          {changes.map((change) => (
            <li key={change.seq} class="followed-change">
              <span class="truncate">{changeSaid(change)}</span>
              <span class="faint">{ago(change.at)}</span>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
