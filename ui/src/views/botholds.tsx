// What the factory's bot may ask, by area, where it is read about: the
// table an agent's page has (`HoldsTable`), changed a line at a time by
// whoever manages the factory, giving only what they hold. It is a
// ceiling: the bot holds nothing of its own, and what it is asked is done
// as whoever asked, held to no more than this; nothing said is everything
// its commands reach. What a blueprint says as the bot is whoever applied
// it, and this narrows none of it.

import { call } from '../api.ts';
import { scopesAfter } from '../holds.ts';
import { attempt, me, meta, speaker } from '../store.ts';
import { HoldsTable } from './holds.tsx';

/** Narrows what the bot may ask, or gives it the usual again with nothing. */
const hold = (holds: string[] | null) => call<{ holds: string[] | null }>('PUT', '/system/speaker/holds', { holds });

export function BotHolds() {
  const about = speaker.value;
  if (!about?.reaches) return null;
  const scopes = about.holds ?? about.reaches;
  const mine = me.value?.principal.scopes ?? [];
  const may = mine.includes('*') || mine.includes('admin');
  const change = (area: string, verbs: string[]) => void attempt(() => hold(scopesAfter(scopes, meta.value?.scopes ?? [], area, verbs)));
  return (
    <>
      <h3>What it may ask of the factory</h3>
      <HoldsTable scopes={scopes} onChange={may ? change : undefined} giver={mine} />
      <p class="faint holds-where">
        {about.holds ? 'Narrowed from everything its commands reach. ' : 'Everything its commands reach, which nobody has narrowed. '}
        It holds nothing of its own: what it is asked is done as whoever asks, and never more than they hold or than this. What a blueprint says as it is whoever applied that blueprint, and is not narrowed by this.
        {may ? '' : ' Whoever manages the factory changes it.'}
      </p>
      {may && about.holds && (
        <button class="ghost" onClick={() => void attempt(() => hold(null))}>
          Give it everything its commands reach again
        </button>
      )}
    </>
  );
}
