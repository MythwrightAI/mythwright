// The line across the top of every page of a factory that is not the one
// somebody would take it for (`ui/src/across.ts`): a copy of
// another, what it was told to say, or its profile. In the colour a
// warning is drawn in, said until whoever manages the factory says it may
// stop, which every open page hears (`system.across`) with nothing read
// again.

import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { call } from '../api.ts';
import { acrossOf, acrossSaid, mayQuiet } from '../across.ts';
import { worn } from '../clock.ts';
import { stream } from '../events.ts';
import { me, meta } from '../store.ts';

stream.on(['system.across'], (event) => {
  if (meta.value) meta.value = { ...meta.value, across: event.data.across ?? null };
});

export function Across() {
  const across = acrossOf(meta.value?.across);
  const [asking, setAsking] = useState(false);
  const [refused, setRefused] = useState('');
  // How tall it is drawn, which a phone's layout keeps clear of under it (`--across-h` in `phone.css`); followed as it wraps.
  const box = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const line = box.current;
    const root = document.documentElement;
    if (!line) return;
    const said = () => root.style.setProperty('--across-h', `${line.offsetHeight}px`);
    said();
    const sized = new ResizeObserver(said);
    sized.observe(line);
    return () => {
      sized.disconnect();
      root.style.removeProperty('--across-h');
    };
  }, [!!across]);
  if (!across) return null;
  const quiet = async () => {
    setAsking(true);
    setRefused('');
    await call('POST', '/system/across', { quiet: true }).catch((err) => setRefused(err instanceof Error ? err.message : String(err)));
    setAsking(false);
  };
  return (
    <div class="across-line" role="note" data-kind={across.kind} ref={box}>
      <span class="across-said">{acrossSaid(across, worn.value)}</span>
      {refused && <span class="across-refused">{refused}</span>}
      {mayQuiet(me.value?.principal.scopes ?? []) && (
        <button type="button" class="ghost across-quiet" disabled={asking} title="Every page of this factory stops saying this, for everybody" onClick={() => void quiet()}>
          Stop saying this
        </button>
      )}
    </div>
  );
}
