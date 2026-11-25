// Settings: the workspace the side panel is of (`spacesettings.tsx`, and at the foot its removal), who you are, who else is here, and keys for scripts. The agents are a tab of the Members page (`members.tsx`).

import { ExperimentsPanel } from './experiments.tsx';
import { Fragment } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Credential, Device, HubLink } from '../api.ts';
import { attempt, me, meta, openTab, settingsPart, signOut, signedIn, toast } from '../store.ts';
import { Beyond } from './given.tsx';
import { PeopleGroups } from './peoplegroups.tsx';
import type { Listed } from '../given.ts';
import { Badge, Field, useEvents } from '../ui.tsx';
import { ago } from '../util.ts';
import { SpaceRemoval, SpaceSettings } from './spacesettings.tsx';
import { ThemePanel } from './themes.tsx';
import { ClockPanel } from './clocked.tsx';
import { FollowingPanel } from './followingpanel.tsx';
import { AppearancePanel } from './appearance.tsx';
import { LogsPanel } from './logskept.tsx';
import { TrashPanel } from './trash.tsx';
import { TextsPanel } from './texts.tsx';
import { WherePanel } from './where.tsx';
import { KeyActs } from './keyrevoked.tsx';
import { YouPanel } from './you.tsx';
import { useInviting } from './invite.tsx';

const TIERS: [string, string][] = [
  ['free', 'One person and their agents.'],
  ['premium', 'A team: invite people to this installation.'],
  ['enterprise', 'A team, with sign-in through your identity provider.'],
];

/** Who is signed in. For now that is all a profile is. */
export function Profile() {
  const who = me.value!;
  return (
    <div>
      <div class="view-head">
        <h1>Profile</h1>
      </div>
      <div class="view-body stack" style="gap:20px;max-width:860px">
        <div class="panel">
          <div class="panel-head">
            <h2 class="grow">You</h2>
            <button
              title="Ends every session of yours: this one, and any in another browser or a terminal. Keys you made go on until each is withdrawn."
              onClick={() => void attempt(() => signOut(true))}
            >
              Sign out everywhere
            </button>
            <button onClick={() => void signOut()}>Sign out</button>
          </div>
          <div class="stack" style="padding:14px">
            <div class="row">
              <strong>{who.principal.name}</strong>
              <span class="muted">{who.account?.email}</span>
              <span class="badge plain">{who.account?.role ?? who.principal.kind}</span>
            </div>
            <p class="faint">
              Signed in to {meta.value?.name} {meta.value?.version} on {meta.value?.device}
              {meta.value?.profile ? `, the ${meta.value.profile} factory` : ''}.
            </p>
          </div>
        </div>
        <YouPanel />
      </div>
    </div>
  );
}

export function Settings() {
  const who = me.value!;
  const admin = who.principal.scopes.includes('*') || who.principal.scopes.includes('admin');
  const [accounts, setAccounts] = useState<Listed[]>([]);
  const [keys, setKeys] = useState<Credential[]>([]);
  // Inviting somebody, as the Members page's People tab has it too (`invite.tsx`).
  const inviting = useInviting();
  const [issued, setIssued] = useState('');
  // What the key just made holds, as the daemon said.
  const [held, setHeld] = useState<string[]>([]);

  const reload = () => {
    if (!admin) return;
    void api.accounts().then((answer) => setAccounts(answer.accounts)).catch(() => {});
    void api.credentials().then(setKeys).catch(() => {});
  };
  useEffect(reload, [admin]);
  // Asked for from elsewhere (the workspace's menu): the workspace, or who is here to invite somebody, is brought into view, once.
  const people = useRef<HTMLDivElement>(null);
  const space = useRef<HTMLDivElement>(null);
  const part = settingsPart.value;
  useEffect(() => {
    if (!part) return;
    (part === 'people' ? people : space).current?.scrollIntoView({ block: 'start' });
    settingsPart.value = null;
  }, [part]);

  return (
    <div>
      <div class="view-head">
        <h1>Settings</h1>
      </div>
      <div class="view-body stack settings-panels" style="gap:20px">
        <WherePanel />
        <SpaceSettings at={space} />
        <ThemePanel />
        <AppearancePanel />
        <ClockPanel />
        <FollowingPanel />

        <div class="panel">
          <div class="panel-head">
            <h2 class="grow">Plan</h2>
            <span class="badge plain">{who.tier}</span>
          </div>
          <div class="stack" style="padding:14px">
            {TIERS.map(([tier, meaning]) => (
              <div key={tier} class="row">
                <strong style="width:90px;text-transform:capitalize">{tier}</strong>
                <span class="muted grow">{meaning}</span>
                {admin && (
                  <button
                    disabled={who.tier === tier}
                    onClick={async () => {
                      if (await attempt(() => api.setTier(tier))) await signedIn();
                    }}
                  >
                    {who.tier === tier ? 'Current' : 'Switch'}
                  </button>
                )}
              </div>
            ))}
            <p class="faint">
              The plan is recorded on this installation. When the account service is connected it decides the plan instead.
            </p>
          </div>
        </div>

        {admin && (
          <div class="panel people-panel" ref={people}>
            <div class="panel-head">
              <h2 class="grow">People</h2>
              {inviting.button}
            </div>
            {inviting.under}
            <table>
              <tbody>
                {accounts.map((account) => (
                  <Fragment key={account.id}>
                    <tr class="account-line">
                      <td>
                        <strong>{account.name}</strong>
                      </td>
                      <td class="muted">{account.email}</td>
                      <td>{account.role}</td>
                      <td class="muted">joined {ago(account.created)}</td>
                    </tr>
                    {/* What they hold, beyond their role, on a line of its own under theirs. */}
                    <tr class="account-holds">
                      <td colSpan={4}>
                        <Beyond account={account} changed={(now) => setAccounts((all) => all.map((one) => (one.id === now.id ? now : one)))} />
                      </td>
                    </tr>
                  </Fragment>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {admin && <PeopleGroups people={accounts} changed={reload} />}

        <Devices admin={admin} />

        {/* The owner's to say, so nobody else is shown it. */}
        {admin && <TextsPanel />}
        {admin && <LogsPanel />}
        {admin && <TrashPanel />}

        {/* Managing them is the owner's, so nobody else is offered the page. */}
        {admin && (
          <div class="panel">
            <div class="panel-head">
              <h2>Secrets</h2>
              <span class="muted grow">keys and tokens handed to the programs that name them</span>
              <button onClick={() => openTab({ kind: 'secrets' })}>Manage secrets</button>
            </div>
          </div>
        )}

        {admin && (
          <div class="panel credentials-panel">
            <div class="panel-head">
              <h2>Credentials</h2>
              <span class="muted grow">what agents and scripts sign in with</span>
              <button
                onClick={async () => {
                  const answer = await attempt(() => api.issueKey('script'));
                  if (answer) {
                    setIssued(answer.token);
                    setHeld(answer.scopes ?? []);
                    reload();
                  }
                }}
              >
                Make a key for scripts
              </button>
            </div>
            {issued && (
              <div style="padding:10px 14px 0">
                <Field label="Copy this now: it is shown once and only its hash is kept">
                  <code class="code-box" onClick={() => void navigator.clipboard?.writeText(issued).then(() => toast('Copied.'))}>
                    {issued}
                  </code>
                </Field>
                {/* A key made here names no scopes, so it holds its maker's. */}
                <p class="faint" style="margin:6px 0 0;font-size:calc(12px * var(--size))">
                  It can do what you can{held.length > 0 ? ` (${held.join(', ')})` : ''}. For one that can do less, name what it needs:{' '}
                  <code>{meta.value?.cli} key ci --scope board:read</code>
                </p>
              </div>
            )}
            <table>
              <thead>
                <tr>
                  <th>Kind</th>
                  <th>Speaks for</th>
                  <th>Name</th>
                  <th>Made</th>
                  <th />
                  <th />
                </tr>
              </thead>
              <tbody>
                {keys
                  .filter((key) => !key.revoked)
                  .map((key) => (
                  <tr key={key.id}>
                    <td>{key.kind}</td>
                    <td>
                      <code>{key.subject}</code>
                    </td>
                    <td>{key.name}</td>
                    <td class="muted">{ago(key.created)}</td>
                    <td>
                      <Badge status="active">in use</Badge>
                    </td>
                    <td class="key-line-acts">{key.kind === 'key' && <KeyActs credential={key} onChanged={reload} />}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {keys.some((key) => key.revoked) && (
              <p class="faint" style="padding:8px 14px">
                {keys.filter((key) => key.revoked).length} earlier credentials have been revoked and no longer work.
              </p>
            )}
          </div>
        )}

        <ExperimentsPanel admin={admin} />

        {/* Last of all, set apart as what cannot be undone. */}
        <SpaceRemoval />
      </div>
    </div>
  );
}

/** The other devices of this fleet, and this device's own hub if it has one. */
function Devices({ admin }: { admin: boolean }) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [hub, setHub] = useState<HubLink | null>(null);
  const [own, setOwn] = useState('');
  const [added, setAdded] = useState<{ name: string; join: string; note: string | null } | null>(null);
  const [name, setName] = useState('');
  const [at, setAt] = useState('');
  const [invitation, setInvitation] = useState('');

  const reload = () =>
    void api
      .devices()
      .then((answer) => {
        setOwn(answer.id);
        setDevices(answer.devices);
        setHub(answer.hub);
      })
      .catch(() => {});
  useEffect(reload, []);
  useEvents(
    ['device.connected', 'device.disconnected', 'device.changed', 'device.invited', 'device.added', 'device.removed', 'device.joined', 'device.left', 'device.unjoined', 'device.unproven'],
    reload,
  );

  return (
    <div class="panel">
      <div class="panel-head">
        <h2>Devices</h2>
        <span class="muted grow">other machines that do this fleet's work</span>
      </div>
      <div class="stack" style="padding:14px">
        <p class="muted">
          This device runs the blueprints. Invite another and a step marked <code>device: its-name</code> runs there. The other device
          calls this one, so only this one's address needs to be reachable from it; what crosses between them is sealed, and each holds
          the other to its key. To another, this device is <code>{own}</code>.
        </p>
        {admin && (
          <form
            class="row"
            onSubmit={async (event) => {
              event.preventDefault();
              const answer = await attempt(() => api.inviteDevice(name.trim(), at.trim() || undefined));
              if (answer) {
                setAdded(answer);
                setName('');
                reload();
              }
            }}
          >
            <input required placeholder="a name, e.g. builder-1" value={name} onInput={(event) => setName(event.currentTarget.value)} style="max-width:260px" />
            <input
              placeholder="where it reaches this one, if not here"
              title="The address the other device reaches this one at, as http://host:port. Left empty, it is the address this page was opened at."
              value={at}
              onInput={(event) => setAt(event.currentTarget.value)}
              style="max-width:260px"
            />
            <button type="submit">Invite a device</button>
          </form>
        )}
        {added && (
          <Field label={`Run this on ${added.name}. The invitation in it is shown once, and lets one device in, once: pass it on as a password is`}>
            <code class="code-box" onClick={() => void navigator.clipboard?.writeText(added.join).then(() => toast('Copied.'))}>
              {added.join}
            </code>
            {added.note && <span class="faint">Note: {added.note}.</span>}
          </Field>
        )}
      </div>
      {devices.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>Device</th>
              <th>Link</th>
              <th>Its key</th>
              <th>It is</th>
              <th>Running</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {devices.map((device) => (
              <tr key={device.name}>
                <td>
                  <strong>{device.name}</strong>
                </td>
                <td>
                  {device.invited ? (
                    <>
                      <Badge status="idle">invited</Badge>
                      <span class="faint"> has yet to use its invitation</span>
                    </>
                  ) : (
                    <>
                      <Badge status={device.connected ? 'active' : 'failed'}>{device.connected ? 'linked' : 'not linked'}</Badge>
                      {!device.connected && (
                        <span class="faint"> {device.last_seen ? `last heard from ${ago(device.last_seen)}` : 'never heard from'}</span>
                      )}
                    </>
                  )}
                </td>
                <td class="muted">
                  <code title="The short name of the key this device proved. It shows the same one as its own.">{device.id ?? ''}</code>
                </td>
                <td class="muted">{device.info ? `${device.info.os} ${device.info.arch}, ${device.info.cpus} CPU` : ''}</td>
                <td class="muted">{device.info ? `${device.info.instances} instances, ${device.info.services.length} services` : ''}</td>
                <td style="text-align:right">
                  {admin && (
                    <button class="danger" onClick={() => void attempt(() => api.removeDevice(device.name)).then(reload)}>
                      Remove
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <div class="stack" style="padding:14px;border-top:1px solid var(--line)">
        {hub ? (
          <div class="row" style="flex-wrap:wrap">
            <Badge status={hub.connected ? 'active' : 'failed'}>{hub.connected ? 'linked' : 'not linked'}</Badge>
            <span class="grow">
              This device is a member of the fleet at <code>{hub.url}</code>
              {hub.device ? `, as ${hub.device}` : ''}, and holds its hub to the key <code>{hub.id}</code>.
              {!hub.connected && hub.error ? ` ${hub.error}; trying again.` : ''}
            </span>
            {admin && (
              <button class="danger" onClick={() => void attempt(() => api.leaveFleet()).then(reload)}>
                Leave
              </button>
            )}
          </div>
        ) : (
          admin && (
            <form
              class="row"
              style="flex-wrap:wrap"
              onSubmit={async (event) => {
                event.preventDefault();
                if (await attempt(() => api.joinFleet(invitation.trim()))) {
                  setInvitation('');
                  reload();
                }
              }}
            >
              <span class="muted">Or make this device a member of another fleet:</span>
              <input
                required
                type="password"
                autocomplete="off"
                placeholder="the invitation its hub made"
                value={invitation}
                onInput={(event) => setInvitation(event.currentTarget.value)}
                style="max-width:320px"
              />
              <button type="submit">Join</button>
            </form>
          )
        )}
      </div>
    </div>
  );
}
