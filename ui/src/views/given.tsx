// The line of *People* in Settings that says what somebody holds beyond
// their role, with what may be given them and what was (`given.ts`).
// The daemon holds the rule; what is offered is only what it would let
// through, and what it refuses is said in its sentence.

import { useEffect, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import { type Listed, offered, saidOf } from '../given.ts';
import { attempt, drawer, me, meta, toast } from '../store.ts';
import { HoldsTable } from './holds.tsx';

export function Beyond({ account, changed }: { account: Listed; changed: (account: Listed) => void }) {
  const mine = me.value?.principal.scopes ?? [];
  const offers = offered(meta.value?.scopes ?? [], mine, account);
  const [chosen, setChosen] = useState('');
  const at = (scope: string) => `/accounts/${encodeURIComponent(account.id)}/given/${encodeURIComponent(scope)}`;
  const given = account.given ?? [];
  const manages = mine.includes('*') || mine.includes('admin');
  return (
    <div class="beyond" data-account={account.email}>
      <span class="muted beyond-said">{saidOf(account)}</span>
      {manages &&
        given.map((scope) => (
          <span class="beyond-given" key={scope}>
            <code>{scope}</code>
            <button
              type="button"
              class="ghost icon"
              aria-label={`Take ${scope} away from ${account.name}`}
              title={`Take ${scope} away from ${account.name}, and from the keys they made`}
              onClick={async () => {
                const answer = await attempt(() => call<{ account: Listed; keys: number }>('DELETE', at(scope)));
                if (!answer) return;
                changed(answer.account);
                const keys = answer.keys === 1 ? 'one of their keys' : `${answer.keys} of their keys`;
                toast(`${account.name} holds ${scope} no longer${answer.keys > 0 ? `, nor do ${keys}` : ''}`);
              }}
            >
              ×
            </button>
          </span>
        ))}
      {offers.length > 0 && (
        <span class="beyond-give">
          <select aria-label={`A scope to give ${account.name}`} value={chosen} onChange={(event) => setChosen((event.target as HTMLSelectElement).value)}>
            <option value="">Give…</option>
            {offers.map((offer) => (
              <option key={offer.scope} value={offer.scope} title={offer.does}>
                {offer.scope}
              </option>
            ))}
          </select>
          <button
            type="button"
            disabled={!chosen}
            onClick={async () => {
              const answer = await attempt(() => call<{ account: Listed }>('PUT', at(chosen)));
              if (!answer) return;
              changed(answer.account);
              toast(`${account.name} holds ${chosen} from now on`);
              setChosen('');
            }}
          >
            Give
          </button>
        </span>
      )}
    </div>
  );
}

/**
 * What a person holds, in the drawer: what Settings' People says of them,
 * by area, with what may be given them beyond their role (`Beyond`). For
 * whoever manages the factory, who alone reads every account; it is the
 * line a person's menu in the side panel's Members offers them.
 */
export function showPermissions(actor: string, name: string): void {
  drawer.value = { title: `${name}: permissions`, body: () => <Permissions key={actor} actor={actor} name={name} /> };
}

function Permissions({ actor, name }: { actor: string; name: string }) {
  const [account, setAccount] = useState<Listed | null | undefined>(undefined);
  useEffect(() => {
    void api
      .accounts()
      .then((read) => setAccount((read.accounts as Listed[]).find((one) => `user:${one.id}` === actor) ?? null))
      .catch(() => setAccount(null));
  }, [actor]);
  if (account === undefined) return <p class="faint">Reading</p>;
  if (account === null) return <p class="faint">There is no account of {name} that you may read.</p>;
  return (
    <div class="context-about">
      <div class="row">
        <strong class="context-card-title">{account.name}</strong>
        <span class="grow" />
        <span class="faint">{account.role}</span>
      </div>
      <p class="muted">{account.email}</p>
      <h3>What they may ask of the factory</h3>
      <HoldsTable scopes={account.holds ?? []} />
      <Beyond account={account} changed={setAccount} />
    </div>
  );
}
