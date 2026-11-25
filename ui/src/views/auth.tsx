// What anyone sees at the address with no session: how to be signed in.
//
// Nobody registers to begin. The factory made its owner as it first
// started, and a link from the command line signs them in (`mw open`,
// `opened.ts`), which is the one line said first. Under it is a sign-in for
// an account that has a password (an owner who set one, somebody invited),
// and registering with an invitation.

import { useEffect, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { meta, signedIn } from '../store.ts';
import { onceRefused, opensWith, spendOnce } from '../opened.ts';
import { Modal, Pills, useFocus } from '../ui.tsx';
import { WhereChip } from './where.tsx';

export function Auth() {
  const [mode, setMode] = useState<'login' | 'register'>('login');
  const [email, setEmail] = useState('');
  const [name, setName] = useState('');
  const [password, setPassword] = useState('');
  const [invite, setInvite] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const first = useFocus<HTMLInputElement>();
  // A link written into the bar of this very page changes only what is after its `#`: it is spent here.
  useEffect(() => {
    const taken = () => void spendOnce().then((spent) => (spent ? signedIn() : undefined));
    window.addEventListener('hashchange', taken);
    return () => window.removeEventListener('hashchange', taken);
  }, []);

  const submit = async (event: Event) => {
    event.preventDefault();
    setBusy(true);
    setError('');
    try {
      if (mode === 'register') await api.register({ email, name, password, invite: invite || undefined });
      else await api.login({ email, password });
      await signedIn();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal plain>
      <div class="row">
        <div class="brand-mark">{meta.value?.name.slice(0, 1)}</div>
        <h1>{meta.value?.name}</h1>
        {/* Which factory is being signed in to: each has accounts of its own. */}
        <WhereChip />
      </div>
      <p class="opens-with">
        To begin, run <code>{opensWith(meta.value?.cli ?? '', meta.value?.profile)}</code> in a terminal on this machine: it opens
        this page signed in.
      </p>
      {onceRefused.value && (
        <p class="error-text" role="alert">
          {onceRefused.value}
        </p>
      )}
      <div class="row">
        <p class="muted grow">
          {mode === 'login'
            ? `Or sign in to the ${meta.value?.profile ? `${meta.value.profile} ` : ''}factory on ${meta.value?.device} with an email and a password.`
            : 'Or register with an invitation from the owner.'}
        </p>
        <Pills
          value={mode}
          options={[
            ['login', 'Sign in'],
            ['register', 'Register'],
          ]}
          onChange={(next) => {
            setMode(next);
            setError('');
          }}
        />
      </div>
      <form class="stack" onSubmit={submit}>
        <label>
          Email
          <input ref={first} type="email" name="email" autocomplete="username" required value={email} onInput={(e) => setEmail(e.currentTarget.value)} />
        </label>
        {mode === 'register' && (
          <label>
            Name
            <input name="name" autocomplete="name" required value={name} onInput={(e) => setName(e.currentTarget.value)} />
          </label>
        )}
        <label>
          Password
          <input
            type="password"
            name="password"
            autocomplete={mode === 'register' ? 'new-password' : 'current-password'}
            required
            minLength={mode === 'register' ? 8 : undefined}
            value={password}
            onInput={(e) => setPassword(e.currentTarget.value)}
          />
        </label>
        {mode === 'register' && (
          <label>
            Invitation code
            <input name="invite" required value={invite} onInput={(e) => setInvite(e.currentTarget.value)} />
          </label>
        )}
        {error && <p class="error-text" role="alert">{error}</p>}
        <button class="primary" type="submit" disabled={busy} style="justify-content:center;height:34px">
          {busy ? 'One moment' : mode === 'register' ? 'Create account' : 'Sign in'}
        </button>
      </form>
    </Modal>
  );
}
