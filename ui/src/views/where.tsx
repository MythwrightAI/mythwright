// Where the page is: the profile of the factory it is in, said in words.
//
// Two factories look alike. What tells a person that they are about to
// change production and not development is this, so it is said where it
// can be read without crowding anything: first in the browser tab's title
// (`titleOf`, set as the page starts), across the top of a narrow screen,
// beside the product's name before anyone has signed in, and in Settings
// with the others on the device. Not at the head of a side panel, which
// says what the panel is (beside the profile, `Channels` was cut down
// to `Ch…`), and not on the rail. A
// factory with no profile says nothing.
//
// It is words and nothing is pressed: an upright mark filled with the
// accent, on the rail among the buttons, was taken for one of them. The
// way to the others on the device is in Settings, with what a
// profile is. They are read when that is shown, not kept: which
// factories are on the device changes rarely, and nothing here asks
// again on a timer.

import { useEffect, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { alone, places } from '../profiles.ts';
import type { Profile } from '../profiles.ts';
import { meta } from '../store.ts';
import { Prose } from '../ui.tsx';
import { classes } from '../util.ts';

/**
 * What the factory's profile is called, as the line across the top of a
 * narrow screen. Only there is it shown: that is the stylesheet's to say.
 */
export function Where() {
  const name = meta.value?.profile;
  if (!name) return null;
  return (
    <span class="where where-band" role="note" aria-label={`Profile: ${name}`} title={`This is the ${name} factory. The others on this device are listed in Settings.`}>
      <span class="where-name">{name}</span>
    </span>
  );
}

/** The same beside a name, as a word in a pill: before anyone has signed in, and in the list. */
export function WhereChip() {
  const name = meta.value?.profile;
  if (!name) return null;
  return (
    <span class="where where-chip" title={`This is the ${name} factory.`}>
      <span class="where-name">{name}</span>
    </span>
  );
}

/** In Settings: which factory this is, what that means, and the others on this device with the way to each. */
export function WherePanel() {
  const name = meta.value?.profile;
  if (!name) return null;
  return (
    <div class="panel where-panel">
      <div class="panel-head">
        <h2 class="grow">Profile</h2>
      </div>
      <div class="stack" style="padding:14px">
        <p class="muted where-about">
          This is the <strong>{name}</strong> factory. Each profile on this device is a factory of its own, with its own projects, channels, accounts and data. Nothing is shared between them, so each is
          signed in to separately.
        </p>
        <Listed />
      </div>
    </div>
  );
}

function Listed() {
  const [found, setFound] = useState<Profile[] | null>(null);
  const [problem, setProblem] = useState('');
  useEffect(() => {
    let stale = false;
    api.profiles().then(
      (profiles) => !stale && setFound(profiles),
      (err) => !stale && setProblem(err instanceof Error ? err.message : String(err)),
    );
    return () => {
      stale = true;
    };
  }, []);
  return (
    <>
      {problem && (
        <p class="error-text" role="alert">
          {problem}
        </p>
      )}
      {!found && !problem && <p class="faint">Reading what there is…</p>}
      {found && (
        <ul class="where-list">
          {places(found).map((place) => {
            const inside = (
              <>
                <span class="where where-chip">
                  <span class="where-name">{place.name}</span>
                </span>
                {/* Why there is no way to one is the daemon's sentence, drawn as what is said in a channel is. */}
                <span class={classes('where-says', !place.here && !place.to && 'muted')}>{place.here || place.to ? place.says : <Prose text={place.says} />}</span>
                {place.to && <span class="where-go">Open ›</span>}
              </>
            );
            return (
              <li key={place.key} aria-current={place.here ? 'true' : undefined}>
                {/* Another factory is another address: it is gone to, as any page is, and asks who is there. */}
                {place.to ? (
                  <a class="where-place" href={place.to}>
                    {inside}
                  </a>
                ) : (
                  <div class={classes('where-place', place.here && 'here')}>{inside}</div>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {found && alone(found) && (
        <p class="faint">
          There is no other on this device. <code>{meta.value?.cli} up --profile &lt;name&gt;</code> starts one.
        </p>
      )}
    </>
  );
}
