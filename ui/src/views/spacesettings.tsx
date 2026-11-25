// In Settings: the workspace the side panel is of, and its settings. Its name is changed where it is written, in the
// part near the top (`SpaceSettings`); and it is taken away from the very
// foot of Settings, after every other part, set apart as what cannot be
// undone (`SpaceRemoval`), behind the question that says where what is in
// it goes and asks for its name to be typed (`RemoveSpace` in
// `spaces.tsx`).
//
// Both are of the workspace the panel is of when Settings is drawn, and
// follow the panel: chosen at the foot, renamed from a terminal or taken
// away, what is shown is what the store follows (`held`, `workspaces`),
// and nothing here changes what the panel is of or which tabs are open.
// Opened from the workspace's menu the part is brought into view
// (`settingsPart`). Whoever the daemon would refuse (it asks
// `projects:write` for both) is shown the name and offered neither.

import type { RefObject } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { held } from '../chosen.ts';
import { counted } from '../help.ts';
import { covers } from '../holds.ts';
import { workspaces, spaces } from '../spaces.ts';
import { me } from '../store.ts';
import type { Workspace } from '../workspaces.ts';
import { Face } from './signs.tsx';
import { askRemove } from './spaces.tsx';

/** The part of Settings for the workspace the side panel is of; nothing while it is of none. */
export function SpaceSettings({ at }: { at: RefObject<HTMLDivElement | null> }) {
  const now = held.value;
  const workspace = now.kind === 'space' ? workspaces.value.find((one) => one.id === now.id) : undefined;
  if (!workspace) return null;
  const may = covers(me.value?.principal.scopes ?? [], 'projects:write');
  const inside = `${counted(workspace.projects.length, 'project', 'projects')} and ${counted(workspace.channels.length, 'channel', 'channels')} of its own`;
  const others = workspaces.value.some((one) => one.id !== workspace.id);
  return (
    <div class="panel space-settings" ref={at} role="region" aria-label="Workspace">
      <div class="panel-head">
        <h2 class="grow">Workspace</h2>
      </div>
      <div class="stack" style="padding:14px">
        {/* Another workspace is another field: what was being written of one is not carried to the next. */}
        {may ? <SpaceNamed key={workspace.id} workspace={workspace} /> : <SpaceSaid workspace={workspace} />}
        <p class="faint space-settings-about">
          It holds {inside}. A workspace is where projects are listed, for everybody here; it changes nothing of who may read them. This is the workspace the side panel shows, and another is chosen at its foot.
        </p>
        {/* With no other, the foot of Settings offers nothing that would take it away, and the part says why. */}
        {may && !others && <p class="faint space-settings-about">The last workspace stays: a factory always has one. Call it something else instead.</p>}
      </div>
    </div>
  );
}

/** The workspace the side panel is of, where whoever is signed in may take it away; else nothing. */
function removable(): Workspace | null {
  const now = held.value;
  const workspace = now.kind === 'space' ? workspaces.value.find((one) => one.id === now.id) : undefined;
  if (!workspace || !covers(me.value?.principal.scopes ?? [], 'projects:write')) return null;
  // The last workspace stays: a factory always has one.
  return workspaces.value.some((one) => one.id !== workspace.id) ? workspace : null;
}

/**
 * At the very foot of Settings, after every other part: the workspace the
 * panel is of taken away, set apart as what cannot be undone. Drawn only
 * while there is another and only for whoever the daemon lets.
 */
export function SpaceRemoval() {
  const workspace = removable();
  if (!workspace) return null;
  return (
    <div class="panel space-removal" role="region" aria-label="Remove this workspace">
      <div class="panel-head">
        <h2 class="grow">Remove this workspace</h2>
      </div>
      <div class="space-settings-grave">
        <div class="space-settings-grave-said">
          <strong>{workspace.name}</strong>
          <span class="muted">Only the workspace goes: what is in it is put in another, which you choose, and its name is typed to say it is this one. It cannot be undone.</span>
        </div>
        <button class="danger" type="button" onClick={() => askRemove(workspace)}>
          Remove {workspace.name}…
        </button>
      </div>
    </div>
  );
}

/** For whoever may not change it: what it is called, and nothing to change it by. */
function SpaceSaid({ workspace }: { workspace: Workspace }) {
  return (
    <div class="space-settings-name">
      <Face name={workspace.name} />
      <strong class="truncate">{workspace.name}</strong>
    </div>
  );
}

/**
 * Its name, changed where it is written: said when Enter is pressed or
 * the field is left, and only where it is something else than it is
 * called and than was said last, so that Enter and leaving the field
 * after it ask once. What the daemon refuses is said under it in the
 * daemon's sentence, with what was written left as it was.
 */
function SpaceNamed({ workspace }: { workspace: Workspace }) {
  const [name, setName] = useState(workspace.name);
  const [wrong, setWrong] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const field = useRef<HTMLInputElement>(null);
  // What was last asked for, answered or still on its way.
  const asked = useRef<string | null>(null);
  // Called something else from elsewhere (a terminal, another page): the field says so, unless it is being written in.
  useEffect(() => {
    if (document.activeElement !== field.current) setName(workspace.name);
    asked.current = null;
  }, [workspace.name]);
  const save = async () => {
    const wanted = name.trim();
    if (wanted === workspace.name || wanted === asked.current) return;
    asked.current = wanted;
    setBusy(true);
    setWrong(null);
    try {
      await spaces.rename(workspace.id, wanted);
    } catch (refused) {
      asked.current = null;
      setWrong(refused instanceof Error ? refused.message : String(refused));
    } finally {
      setBusy(false);
    }
  };
  return (
    <form
      class="stack space-settings-form"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <label class="space-settings-label" for="space-settings-name">
        Name
      </label>
      <div class="space-settings-name">
        <Face name={workspace.name} />
        <input
          ref={field}
          id="space-settings-name"
          value={name}
          aria-invalid={wrong ? 'true' : undefined}
          aria-busy={busy ? 'true' : undefined}
          onInput={(event) => {
            setName(event.currentTarget.value);
            setWrong(null);
          }}
          onBlur={() => void save()}
        />
      </div>
      {wrong && (
        <p role="alert" class="error-text" style="margin:0">
          {wrong}
        </p>
      )}
    </form>
  );
}
