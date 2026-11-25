// How long the detail of runs' steps is kept and where their output is
// written: the factory's two settings, in Settings, for whoever manages
// the factory. Each is a choice with a usual, said in a sentence under it.

import { useEffect, useState } from 'preact/hooks';
import { type LogsKept, howLong, keepLogs, logsKept, wentNow, where } from '../logskept.ts';
import { attempt, toast } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { ago } from '../util.ts';

/** How many days each choice keeps for; `always` keeps everything. */
const DAYS = [1, 7, 14, 30, 90, 365];

export function LogsPanel() {
  const [now, setNow] = useState<LogsKept | null>(null);
  const [folder, setFolder] = useState('');
  const read = () => void logsKept().then(setNow).catch(() => {});
  useEffect(read, []);
  useEvents('system.logs', read, []);
  if (!now) return null;
  const set = async (asked: { days?: number | 'always'; folder?: string }) => {
    const said = await attempt(() => keepLogs(asked));
    if (!said) return;
    setNow(said);
    setFolder('');
    toast(wentNow(said.let_go) ?? 'Said.');
  };
  const days = now.days === null ? 'always' : String(now.days);
  const choices = now.days === null || DAYS.includes(now.days) ? DAYS : [...DAYS, now.days].sort((a, b) => a - b);
  return (
    <div class="panel logs-setting">
      <div class="panel-head">
        <h2>Logs of runs</h2>
        <span class="muted grow">how long what steps printed is kept, and where it is written</span>
      </div>
      <div class="stack" style="gap:10px;padding:12px 16px">
        <label class="stack" style="gap:4px">
          <span class="muted">Kept</span>
          <select aria-label="Logs are kept" value={days} onChange={(event) => void set({ days: event.currentTarget.value === 'always' ? 'always' : Number(event.currentTarget.value) })}>
            <option value="always">For ever</option>
            {choices.map((one) => (
              <option key={one} value={String(one)}>
                For {one === 1 ? 'a day' : `${one} days`}
              </option>
            ))}
          </select>
        </label>
        <span class="muted logs-how-long">{howLong(now)}</span>
        <form
          class="stack"
          style="gap:4px"
          onSubmit={(event) => {
            event.preventDefault();
            if (folder.trim()) void set({ folder: folder.trim() });
          }}
        >
          <span class="muted">Written in</span>
          <div class="row" style="flex-wrap:wrap;gap:8px">
            <input aria-label="Folder for logs" style="flex:1 1 16rem;min-width:0" placeholder={now.folder ?? now.usual} value={folder} onInput={(event) => setFolder(event.currentTarget.value)} />
            <button type="submit" disabled={!folder.trim()}>
              Write there
            </button>
            {now.folder && (
              <button type="button" onClick={() => void set({ folder: 'usual' })}>
                Back to the data folder
              </button>
            )}
          </div>
        </form>
        <span class="muted logs-where" style="overflow-wrap:anywhere">
          {where(now)}
        </span>
        <span class="faint">The databases stay in the data folder.</span>
        {now.by && now.at && (
          <span class="faint">
            said by {now.by}, {ago(now.at)}
          </span>
        )}
      </div>
    </div>
  );
}
