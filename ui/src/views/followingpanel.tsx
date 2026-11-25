// Settings → Following: everything whoever is signed in follows, each
// with its kind, how often they are told (and for Daily at what time of
// their day), and Unfollow. What it is called opens it.

import { useState } from 'preact/hooks';
import { OFTEN, follow, kindSaid, tabOf, useFollowing } from '../following.ts';
import type { Follow, Often } from '../following.ts';
import { attempt, openTab } from '../store.ts';
import { mayFollow } from './followbutton.tsx';

/** One thing followed: what it is, how often, and Unfollow. */
function Followed({ one }: { one: Follow }) {
  const [at, setAt] = useState(one.at);
  const tab = tabOf(one);
  const set = (how: { follows?: boolean; often?: Often; at?: string }) => void attempt(() => follow({ kind: one.kind, thing: one.thing }, how));
  return (
    <li class="following-line" data-thing={`${one.kind}:${one.thing}`}>
      <span class="following-kind faint">{kindSaid(one.kind, one.pages)}</span>
      {tab ? (
        <button type="button" class="ghost following-title truncate" title={`Open ${one.title}`} onClick={() => openTab(tab)}>
          {one.title}
        </button>
      ) : (
        <span class="following-title truncate">{one.title}</span>
      )}
      <span class="following-how">
        <select aria-label={`How often you are told of ${one.title}`} value={one.often} onChange={(event) => set({ often: event.currentTarget.value as Often })}>
          {OFTEN.map(([often, said]) => (
            <option key={often} value={often}>
              {said}
            </option>
          ))}
        </select>
        {one.often === 'daily' && (
          <input
            type="time"
            aria-label={`The time of day you are told of ${one.title}`}
            value={at}
            onInput={(event) => setAt(event.currentTarget.value)}
            onChange={(event) => set({ at: event.currentTarget.value })}
          />
        )}
        <button type="button" class="following-unfollow" onClick={() => set({ follows: false })}>
          Unfollow
        </button>
      </span>
    </li>
  );
}

export function FollowingPanel() {
  const all = useFollowing();
  if (!mayFollow()) return null;
  return (
    <div class="panel following-panel">
      <div class="panel-head">
        <h2>Following</h2>
        <span class="muted grow">what tells you in your inbox when it changes, and how often</span>
      </div>
      <div class="stack" style="gap:8px;padding:12px 16px">
        {all === null ? (
          <span class="faint">Reading it</span>
        ) : all.length === 0 ? (
          <span class="faint following-none">You follow nothing yet. A page, a board, a table, a library, a channel or a project has Follow at the top of its page.</span>
        ) : (
          <ul class="following-list">
            {all.map((one) => (
              <Followed key={`${one.kind}:${one.thing}`} one={one} />
            ))}
          </ul>
        )}
        <span class="faint">Hourly and daily are one line for each thing that changed in that hour or day, and none where nothing did. Daily comes at its time in the zone under Time.</span>
      </div>
    </div>
  );
}
