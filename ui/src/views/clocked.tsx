// How a time is written for whoever is signed in, in Settings: the zone,
// chosen from the zones the browser knows by every word typed, and a
// clock of twelve hours or twenty-four. Each is kept with their account
// as it is chosen (`clockkept.ts`), and every time on the page is drawn
// again in it.

import { useState } from 'preact/hooks';
import { browserZone, settingOf, worn, zones, zonesFound, zoneSaid } from '../clock.ts';
import type { Hours } from '../clock.ts';
import { keepClock, writesClock } from '../clockkept.ts';
import { clock } from '../util.ts';

/** How many of the zones found are listed at once: typing narrows the rest. */
const LISTED = 80;

export function ClockPanel() {
  const [typed, setTyped] = useState('');
  const now = worn.value;
  const may = writesClock();
  const found = zonesFound(typed, zones());
  // With nothing typed the zone chosen is first, so that what is chosen is seen without looking for it.
  const listed = (typed.trim() ? found : [now.zone, ...found.filter((zone) => zone !== now.zone)]).slice(0, LISTED);
  const choose = (zone: string, hours: Hours = now.hours) => {
    const setting = settingOf({ zone, hours });
    if (setting) void keepClock(setting);
  };
  const here = browserZone();
  return (
    <div class="panel clock-setting">
      <div class="panel-head">
        <h2>Time</h2>
        <span class="muted grow">how a time is written for you, in every browser</span>
      </div>
      <div class="stack" style="gap:10px;padding:12px 16px">
        <p class="clock-now">
          It is <strong data-clock-now>{clock(Date.now())}</strong> in {zoneSaid(now.zone)}.
        </p>
        <div class="pills" role="radiogroup" aria-label="A clock of">
          {(['12h', '24h'] as Hours[]).map((hours) => (
            <button type="button" key={hours} role="radio" aria-checked={now.hours === hours} disabled={!may} onClick={() => choose(now.zone, hours)}>
              {hours === '12h' ? '12 hours, AM and PM' : '24 hours'}
            </button>
          ))}
        </div>
        {may ? (
          <div class="stack clock-zones" style="gap:6px">
            <input
              type="search"
              aria-label="Find a zone"
              placeholder="Find a zone: a city or a region"
              value={typed}
              onInput={(event) => setTyped(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && listed[0]) {
                  event.preventDefault();
                  choose(listed[0]);
                }
              }}
            />
            <ul class="clock-zone-list" role="listbox" aria-label="Zones">
              {listed.map((zone) => (
                <li key={zone} role="option" aria-selected={zone === now.zone}>
                  <button type="button" data-zone={zone} onClick={() => choose(zone)}>
                    {zoneSaid(zone)}
                  </button>
                </li>
              ))}
              {found.length === 0 && <li class="faint">No zone says that.</li>}
              {found.length > LISTED && <li class="faint">{found.length - LISTED} more: type more of the name.</li>}
            </ul>
            {here !== now.zone && (
              <button type="button" class="ghost clock-here" onClick={() => choose(here)}>
                Use this browser's zone, {zoneSaid(here)}
              </button>
            )}
          </div>
        ) : (
          <span class="faint">Changing it needs `clock:write`.</span>
        )}
        <span class="faint">The zone is the one this browser says it is in until you choose another. A window run under another system may say another zone than yours: choose yours here.</span>
      </div>
    </div>
  );
}
