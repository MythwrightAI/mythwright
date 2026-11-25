// How what is running comes by new blueprints: the factory's setting, in
// Settings, for whoever runs the factory. Two parts, each a choice with a
// usual: how texts are come by, and how one that is taken lands. Having
// them followed gives something up, which is said in words before it is
// turned on and not after.

import { useEffect, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { TextsSetting } from '../comparing.ts';
import { COME, standing } from '../textsetting.ts';
import { attempt, toast } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { ago } from '../util.ts';

export function TextsPanel() {
  const [now, setNow] = useState<TextsSetting | null>(null);
  // Following is asked for and then said to be meant, with what it gives up between the two.
  const [asking, setAsking] = useState(false);
  const read = () => void api.texts().then(setNow).catch(() => {});
  useEffect(read, []);
  useEvents('system.texts', read, []);
  if (!now) return null;
  const set = async (asked: { come?: string; land?: string }) => {
    const said = await attempt(() => api.setTexts(asked));
    setAsking(false);
    if (!said) return;
    setNow(said);
    const left = said.not_taken ?? [];
    toast(left.length > 0 ? `Said. ${left.map((one) => `${one.blueprint} does not check as it now is, so ${one.key} goes on with the text it has`).join('. ')}.` : 'Said.');
  };
  return (
    <div class="panel texts-setting">
      <div class="panel-head">
        <h2>Blueprints that are running</h2>
        <span class="muted grow">how they come by a blueprint that has changed</span>
      </div>
      <div class="stack" style="gap:10px;padding:12px 16px">
        <div class="row" style="flex-wrap:wrap;align-items:flex-end;gap:14px">
          <label class="stack" style="gap:4px">
            <span class="muted">Blueprints are</span>
            <select aria-label="Blueprints are" value={asking ? 'follow' : now.come} onChange={(event) => (event.currentTarget.value === 'follow' ? setAsking(true) : void set({ come: event.currentTarget.value }))}>
              {(Object.keys(COME) as TextsSetting['come'][]).map((come) => (
                <option key={come} value={come}>
                  {COME[come]}
                </option>
              ))}
            </select>
          </label>
          <label class="stack" style="gap:4px">
            <span class="muted">One that is taken lands</span>
            <select aria-label="One that is taken lands" value={now.land} onChange={(event) => void set({ land: event.currentTarget.value })}>
              <option value="in_place">In place: carried over, or taken where it is</option>
              <option value="again">Again: its steps run again</option>
            </select>
          </label>
        </div>
        <span class="muted texts-standing">{standing(now)}</span>
        {asking && (
          <div class="stack texts-asking" style="gap:8px">
            <span>
              Having them followed gives something up: {now.gives_up}. It takes what taking a wall away takes, and is for where blueprints are being written.
            </span>
            <div class="row">
              <button class="primary" onClick={() => void set({ come: 'follow' })}>
                Have them followed
              </button>
              <button onClick={() => setAsking(false)}>Not now</button>
            </div>
          </div>
        )}
        {now.by && now.at && (
          <span class="faint">
            said by {now.by}, {ago(now.at)}
          </span>
        )}
      </div>
    </div>
  );
}
