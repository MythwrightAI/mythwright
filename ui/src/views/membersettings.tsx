// What may be set of somebody added to a channel, drawn from the sections
// `membersettings.ts` hands (`settingsOf`): each is drawn by what draws
// it elsewhere, the by-area table the Tools panel draws (`HoldsTable`)
// and the line of People in Settings that gives a person more
// (`Beyond`). Nothing here decides what is offered.

import { useState } from 'preact/hooks';
import type { Listed } from '../given.ts';
import { scopesAfter } from '../holds.ts';
import type { Section } from '../membersettings.ts';
import { me, meta } from '../store.ts';
import { Beyond } from './given.tsx';
import { HoldsTable } from './holds.tsx';

/**
 * What a changeable section is set to, kept while it is of the same one
 * (`key`, by `sectionKey`) and begun again on `begins` for another.
 */
export function useSet(key: string, begins: string[]): [string[], (scopes: string[]) => void] {
  const [kept, setKept] = useState<{ key: string; scopes: string[] } | null>(null);
  const now = kept && kept.key === key ? kept.scopes : begins;
  return [now, (scopes) => setKept({ key, scopes })];
}

/** The sections, in order. `scopes` is what a changeable one is set to; `onAccount` is told of a person given more. */
export function MemberSettings({ sections, scopes, onScopes, onAccount }: { sections: Section[]; scopes: string[]; onScopes: (scopes: string[]) => void; onAccount?: (account: Listed) => void }) {
  if (sections.length === 0) return null;
  const mine = me.value?.principal.scopes ?? [];
  const areas = meta.value?.scopes ?? [];
  return (
    <div class="stack member-settings">
      {sections.map((section) =>
        section.section === 'holds' ? (
          <div key="holds" class="member-setting" data-setting="holds">
            <strong>{section.title}</strong>
            <HoldsTable scopes={section.may ? scopes : section.scopes} giver={mine} onChange={section.may ? (area, verbs) => onScopes(scopesAfter(scopes, areas, area, verbs)) : undefined} />
            <p class="faint member-setting-said">{section.said}</p>
          </div>
        ) : (
          <div key="given" class="member-setting" data-setting="given">
            <strong>{section.title}</strong>
            <Beyond account={section.account} changed={(account) => onAccount?.(account)} />
          </div>
        ),
      )}
    </div>
  );
}
