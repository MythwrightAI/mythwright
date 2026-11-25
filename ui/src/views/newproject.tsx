// The dialog that adds a project.
//
// It asks no workspace: a project is made in the workspace the side panel
// is of, the one showing, which its head says in a line, so that what is
// added while looking at a workspace is not added out of sight. It is put
// in another afterwards by its row's Move to…, as any project is. Where the
// panel is of none (the workspaces are not read yet), nothing is said and
// the daemon puts it in the usual one. A terminal still says one
// (`project create --workspace`).
//
// Setting a project up is a blueprint, `std/project`, applied for it as
// whoever adds it, and what it starts with is its rows: those of the
// factory's default (`project_starts` of `/api/meta`), or of a setup the
// factory keeps under a name, chosen at the top where it keeps any
// (`/api/setups`). They are laid out as a table: a row for each thing,
// what it is and its switch, on unless whoever adds it would rather not,
// and under the row, folded until it is asked for, what it is set up
// with. That is its blueprint's own params, read by the daemon
// (`/api/starts`), drawn as the drawer that applies a blueprint draws
// them (`ParamField`); what is written there is handed to the blueprint
// as the project is added, and a box nobody wrote in says nothing
// (`startset.ts`). Whoever may apply a blueprint adds a row of one of
// their own under the table, and saves the table as a setup with a name,
// or as the factory's default where they manage it.

import { useEffect, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Described, GraphParam } from '../api.ts';
import { startOf, written as writtenAs } from '../applyasked.ts';
import { held } from '../chosen.ts';
import { covers } from '../holds.ts';
import { workspaces } from '../spaces.ts';
import { ADDING, had, offeredAt, usual, wantedAfter, whyNot } from '../starts.ts';
import type { Start } from '../starts.ts';
import { DEFAULT, askedWith, changedOf, nameOf, ownRow, savedRows, writtenAfter } from '../startset.ts';
import type { Settable, Setup, Written } from '../startset.ts';
import { attempt, me, meta, openTab, toast } from '../store.ts';
import { Modal, Twist, useFocus } from '../ui.tsx';
import { classes } from '../util.ts';
import { ParamField } from './applying.tsx';
import { FolderField } from './folder.tsx';

/** The workspace a project made now is made in: the one the panel is of, where it is of one. */
export function madeIn(): { id: string; name: string } | null {
  const choice = held.value;
  if (choice.kind !== 'space') return null;
  return workspaces.value.find((one) => one.id === choice.id) ?? null;
}

/** Reads what each row of a setup may be set up with: nothing where it cannot be read, and then nothing is offered to be set. */
async function readSettable(setup: string): Promise<Settable> {
  try {
    const asked = setup === DEFAULT ? '/starts' : `/starts?setup=${encodeURIComponent(setup)}`;
    const read = await call<{ starts: { name: string; params: GraphParam[] }[] }>('GET', asked);
    return Object.fromEntries(read.starts.map((one) => [one.name, one.params]));
  } catch {
    return {};
  }
}

/** The setups the factory keeps, and the rows of its default as it is now: nothing where they cannot be read. */
async function readSetups(): Promise<{ setups: Setup[]; usual: Start[] } | null> {
  try {
    return await call<{ setups: Setup[]; usual: Start[] }>('GET', '/setups');
  } catch {
    return null;
  }
}

/** What a box says before anybody writes in it: what its row says, else what its blueprint says. */
const startsAs = (row: Start | undefined) => (param: GraphParam) => writtenAs(startOf(param, (row?.with ?? {}) as Record<string, never>));

/** What whoever is signed in holds. */
const mine = (): string[] => me.value?.principal.scopes ?? [];

export function NewProject({ onClose }: { onClose: () => void }) {
  const [name, setName] = useState('');
  // Which setup it is set up with: the factory's default, or one it keeps under a name.
  const [setups, setSetups] = useState<Setup[]>([]);
  const [setup, setSetup] = useState(DEFAULT);
  // The default's rows as the daemon read them as the page opened, until they are read again here.
  const [theUsual, setTheUsual] = useState<Start[]>(() => meta.value?.project_starts ?? []);
  const named = setups.filter((one) => !one.default);
  const rowsOf = (which: string) => (which === DEFAULT ? theUsual : (setups.find((one) => one.name === which)?.rows ?? []));
  // Rows of whoever adds it, beside the setup's, by their names.
  const [own, setOwn] = useState<Start[]>([]);
  const all = [...rowsOf(setup), ...own];
  const starts = offeredAt(all, ADDING);
  // What whoever adds it turned on themselves is kept apart from what the project then has: a thing that went off
  // only because what it needs did comes back when that does.
  const [wanted, setWanted] = useState(() => usual(starts));
  const chosen = had(starts, wanted);
  const [busy, setBusy] = useState(false);
  // The folder it is made of, where whoever adds it says one: said nothing, the factory makes it one of its own.
  const [folder, setFolder] = useState('');
  // Where it goes, as the dialog was opened: the panel may be put on another workspace meanwhile, and what the
  // head said is where it is made.
  const [space] = useState(madeIn);
  // What each may be set up with, and what was written of it; which rows are unfolded; what cannot be read.
  const [settable, setSettable] = useState<Settable>({});
  const [written, setWritten] = useState<Written>({});
  const [open, setOpen] = useState<string[]>([]);
  const [wrong, setWrong] = useState<{ wrong: string; at: string } | null>(null);
  // What can be added as a row of one's own, and what is being written of one.
  const [blueprints, setBlueprints] = useState<Described[]>([]);
  const [adding, setAdding] = useState('');
  // Saving the table: under which name, or as the factory's default.
  const [saving, setSaving] = useState<string | null>(null);
  const [savedAs, setSavedAs] = useState('');
  // What was last saved, said in the dialog: a toast would lie over its foot on a phone.
  const [saidSaved, setSaidSaved] = useState('');
  const mayApply = covers(mine(), 'blueprints:run');
  const mayName = mayApply && covers(mine(), 'projects:write');
  const mayDefault = covers(mine(), 'admin');
  useEffect(() => {
    let gone = false;
    void readSettable(DEFAULT).then((read) => !gone && setSettable(read));
    void readSetups().then((read) => {
      if (gone || !read) return;
      setSetups(read.setups);
      // A default saved since the page read it is what is offered, each row as it says.
      if (JSON.stringify(read.usual) !== JSON.stringify(theUsual)) setWanted(usual(offeredAt(read.usual, ADDING)));
      setTheUsual(read.usual);
    });
    if (mayApply) void attempt(() => api.blueprints()).then((read) => !gone && read && setBlueprints(read));
    return () => {
      gone = true;
    };
  }, []);
  const input = useFocus<HTMLInputElement>();
  const titled = (one: string) => all.find((start) => start.name === one)?.title ?? one;
  // Another setup chosen: its rows, each as its setup says, and what may be set of each.
  const choose = (which: string) => {
    setSetup(which);
    setWritten({});
    setOpen([]);
    setWrong(null);
    setWanted(usual(offeredAt([...rowsOf(which), ...own], ADDING)));
    void readSettable(which).then((read) => setSettable((was) => ({ ...read, ...Object.fromEntries(own.map((row) => [row.name, was[row.name] ?? []])) })));
  };
  const addOwn = (event: Event) => {
    event.preventDefault();
    const blueprint = blueprints.find((one) => one.name === adding.trim());
    if (!blueprint) return;
    const row = ownRow(blueprint.name, all.map((one) => one.name));
    setOwn([...own, row]);
    setSettable({ ...settable, [row.name]: blueprint.params });
    setWanted([...wanted, row.name]);
    setAdding('');
  };
  const dropOwn = (row: string) => {
    setOwn(own.filter((one) => one.name !== row));
    setWanted(wanted.filter((one) => one !== row));
  };
  const submit = async (event: Event) => {
    event.preventDefault();
    if (busy) return;
    // A daemon that lists nothing is asked for the usual, which is its own to say.
    const ownNames = own.map((row) => row.name);
    const asked = starts.length > 0 ? askedWith(chosen, all, ownNames, settable, written, titled) : { asked: true as const };
    if ('wrong' in asked) {
      // What cannot be read is said, and its settings are unfolded to be put right.
      setWrong(asked);
      if (!open.includes(asked.at)) setOpen([...open, asked.at]);
      return;
    }
    setWrong(null);
    // Its fleet is brought up before the answer comes: the button says so, and is not pressed twice.
    setBusy(true);
    const setUp = setup === DEFAULT || asked.asked === true ? asked.asked : { setup, rows: asked.asked };
    const made = await attempt(() => api.createProject(name, setUp, space?.id, folder.trim() || undefined));
    // A folder the daemon refused is said why, and what was written is still there to put right.
    if (!made && folder.trim()) return setBusy(false);
    // What it could not be given is said, with what to do: the project is there all the same.
    for (const left of made?.set_up?.left ?? []) toast(left, true);
    if (made) openTab({ kind: 'project', id: made.id });
    onClose();
  };
  const save = async () => {
    const as = saving === DEFAULT ? DEFAULT : nameOf(savedAs);
    if (!as) return;
    const rows = savedRows(all, chosen, settable, written, titled);
    if ('wrong' in rows) {
      setWrong(rows);
      if (!open.includes(rows.at)) setOpen([...open, rows.at]);
      return;
    }
    const kept = await attempt(() => call<Setup>('PUT', `/setups/${encodeURIComponent(as)}`, { rows: rows.rows }));
    if (!kept) return;
    setSaidSaved(as === DEFAULT ? 'Saved as the factory’s default for new projects.' : `Saved as ${as}.`);
    const read = await readSetups();
    if (read) {
      setSetups(read.setups);
      setTheUsual(read.usual);
    }
    setSaving(null);
    setSavedAs('');
    // What was saved is what the dialog now shows, with nothing of one's own beside it.
    setOwn([]);
    setSetup(as);
    void readSettable(as).then(setSettable);
  };
  return (
    <Modal onClose={onClose} class="setting-up">
      <div class="ask-head">
        <h2>New project</h2>
        {space && <p class="faint ask-where">In {space.name}</p>}
      </div>
      <form class="stack" onSubmit={submit}>
        {named.length > 0 && (
          <label class="setup-choice">
            Set up as
            <select value={setup} onChange={(event) => choose(event.currentTarget.value)}>
              <option value={DEFAULT}>The factory’s default</option>
              {named.map((one) => (
                <option key={one.name} value={one.name}>
                  {one.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <label>
          Name
          <input ref={input} required value={name} placeholder="What the project is called" onInput={(event) => setName(event.currentTarget.value)} />
        </label>
        <FolderField value={folder} onChange={setFolder} />
        {starts.length > 0 && (
          <fieldset class="starts">
            <legend>It starts with</legend>
            <div class="setup-table">
              {starts.map((start) => {
                // What it cannot be had without is off: it is off too, and says why.
                const why = whyNot(starts, start, chosen);
                const params = settable[start.name] ?? [];
                const unfolded = open.includes(start.name);
                const begins = startsAs(start);
                const changed = changedOf(params, written[start.name], begins);
                const isOwn = own.some((row) => row.name === start.name);
                const fold = () => setOpen(unfolded ? open.filter((one) => one !== start.name) : [...open, start.name]);
                return (
                  <div key={start.name} class={classes('setup-row', unfolded && 'open')}>
                    <div class={classes('start-with', why && 'lacking')}>
                      <span class="grow">
                        {start.title}
                        <span class="muted">{start.says}</span>
                        {why && <span class="start-why">{why}</span>}
                        {isOwn && (
                          <button type="button" class="ghost setup-drop" aria-label={`Remove ${start.title}`} onClick={() => dropOwn(start.name)}>
                            Remove this row
                          </button>
                        )}
                      </span>
                      <input
                        type="checkbox"
                        role="switch"
                        class="switch"
                        aria-label={start.title}
                        checked={chosen.includes(start.name)}
                        disabled={why !== null}
                        onChange={(event) => setWanted(wantedAfter(starts, wanted, start.name, event.currentTarget.checked))}
                      />
                      {params.length > 0 ? (
                        <button
                          type="button"
                          class="ghost setup-fold"
                          aria-expanded={unfolded}
                          aria-controls={`setup-${start.name}`}
                          aria-label={`Settings of ${start.title}`}
                          title={unfolded ? 'Fold its settings away' : 'What it is set up with'}
                          onClick={fold}
                        >
                          {changed > 0 && <span class="setup-changed">{changed} set</span>}
                          <Twist open={unfolded} />
                        </button>
                      ) : (
                        // Room kept, so that every switch stands in one column.
                        <span class="setup-fold" aria-hidden="true" />
                      )}
                    </div>
                    {unfolded && (
                      <div id={`setup-${start.name}`} class="setup-settings" role="group" aria-label={`Settings of ${start.title}`}>
                        {params.map((param) => (
                          <ParamField
                            key={param.name}
                            param={param}
                            now={written[start.name]?.[param.name] ?? begins(param)}
                            known={{}}
                            onSet={(text) => {
                              setWritten(writtenAfter(written, start.name, param.name, text));
                              if (wrong?.at === start.name) setWrong(null);
                            }}
                          />
                        ))}
                        {wrong?.at === start.name && <p class="error-text comparing-wrong">{wrong.wrong}</p>}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
            {mayApply && (
              <div class="setup-add">
                <label>
                  A blueprint of your own
                  <input list="setup-blueprints" value={adding} placeholder="Its name, such as std/mockups" onInput={(event) => setAdding(event.currentTarget.value)} />
                </label>
                <datalist id="setup-blueprints">
                  {blueprints.map((one) => (
                    <option key={one.name} value={one.name} />
                  ))}
                </datalist>
                <button type="button" disabled={!blueprints.some((one) => one.name === adding.trim())} onClick={addOwn}>
                  Add row
                </button>
              </div>
            )}
            <span class="faint">Each is a blueprint, applied now or later; stopping one keeps what it made.</span>
            {(mayName || mayDefault) && saving === null && (
              <div class="setup-kept">
                <button type="button" class="ghost setup-save-as" onClick={() => setSaving(mayName ? '' : DEFAULT)}>
                  Save as…
                </button>
                {saidSaved && (
                  <span class="faint setup-saved" role="status">
                    {saidSaved}
                  </span>
                )}
              </div>
            )}
            {saving !== null && (
              <div class="setup-save" role="group" aria-label="Saving the table">
                {mayDefault && (
                  <label>
                    Save as
                    <select value={saving === DEFAULT ? DEFAULT : ''} onChange={(event) => setSaving(event.currentTarget.value)}>
                      <option value="">A setup with a name</option>
                      <option value={DEFAULT}>The factory’s default for new projects</option>
                    </select>
                  </label>
                )}
                {saving !== DEFAULT && (
                  <label>
                    Its name
                    <input value={savedAs} placeholder="research" onInput={(event) => setSavedAs(event.currentTarget.value)} />
                  </label>
                )}
                <button type="button" disabled={saving !== DEFAULT && !nameOf(savedAs)} onClick={() => void save()}>
                  Save
                </button>
              </div>
            )}
          </fieldset>
        )}
        <div class="row modal-foot" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit" disabled={busy}>
            {busy ? 'Creating' : 'Create'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
