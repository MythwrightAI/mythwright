// Who is signed in, on their Profile: what they are called, changed in place,
// and for an account that has no password (the owner the factory made as
// it first started) a way to give it one, with an email to sign in by
// where it has none, to sign in another way than the link `mw open` makes:
// from another device (`opened.ts`).

import { useState } from 'preact/hooks';
import { api } from '../api.ts';
import { me, meta } from '../store.ts';
import { opensWith, rename, setPassword } from '../opened.ts';

export function YouPanel() {
  const who = me.value!;
  const account = who.account;
  const [name, setName] = useState(account?.name ?? who.principal.name);
  const [email, setEmail] = useState('');
  const [password, setWords] = useState('');
  const [said, setSaid] = useState('');
  const [wrong, setWrong] = useState('');
  const [busy, setBusy] = useState(false);
  if (!account) return null;
  const read = async () => {
    me.value = await api.me();
  };
  const named = async () => {
    const asked = name.trim();
    if (!asked || asked === me.value?.account?.name) return;
    setWrong('');
    try {
      await rename(asked);
      await read();
    } catch (err) {
      setWrong(err instanceof Error ? err.message : String(err));
    }
  };
  const set = async (event: Event) => {
    event.preventDefault();
    setBusy(true);
    setWrong('');
    try {
      await setPassword(password, account.email ? undefined : email);
      setWords('');
      // Said once the account is read again, so that the form is gone by the time it is said.
      await read();
      setSaid('Set: you can sign in with your email and this password too, from anywhere that can reach the factory.');
    } catch (err) {
      setWrong(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div class="panel you-panel">
      <div class="panel-head">
        <h2 class="grow">Name and password</h2>
      </div>
      <div class="stack" style="padding:14px">
        <label class="field">
          <span>Name</span>
          <input
            value={name}
            maxLength={60}
            onInput={(event) => setName(event.currentTarget.value)}
            onKeyDown={(event) => event.key === 'Enter' && void named()}
            onBlur={() => void named()}
          />
        </label>
        {who.password === false && (
          <form class="stack" onSubmit={set}>
            <p class="muted">
              You have no password: <code>{opensWith(meta.value?.cli ?? '', meta.value?.profile)}</code> on this machine signs you in. Set one to sign
              in another way, from another device.
            </p>
            {!account.email && (
              <label class="field">
                <span>Email</span>
                <input type="email" name="email" autocomplete="username" required value={email} onInput={(event) => setEmail(event.currentTarget.value)} />
              </label>
            )}
            <label class="field">
              <span>Password</span>
              <input
                type="password"
                name="password"
                autocomplete="new-password"
                required
                minLength={8}
                value={password}
                onInput={(event) => setWords(event.currentTarget.value)}
              />
            </label>
            <div class="row">
              <button class="primary" type="submit" disabled={busy}>
                Set password
              </button>
            </div>
          </form>
        )}
        {said && <p class="muted">{said}</p>}
        {wrong && (
          <p class="error-text" role="alert">
            {wrong}
          </p>
        )}
      </div>
    </div>
  );
}
