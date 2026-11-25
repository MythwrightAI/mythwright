// What a blueprint is applied with, asked before it is applied.
//
// It is asked in the drawer, so that the blueprint's text stays readable
// beside it: each param is its name, its box under that and what the
// blueprint says of it under the box, and what the form is left by is
// kept at the drawer's foot. For a blueprint opened for no project it
// also asks which project it is for. What is written is kept as it is
// written (`applyasked.ts`): the drawer is put away when another tab is
// shown, and has what was typed when it is opened again.

import { useState } from 'preact/hooks';
import type { Fields, GraphParam } from '../api.ts';
import { answered, applyingOf, keepApplying, startOf, written } from '../applyasked.ts';
import type { Applying as Written } from '../applyasked.ts';
import { asItems } from '../pills.ts';
import { applyClosed } from '../projectenvs.ts';
import { Select } from '../select.tsx';
import { projects } from '../store.ts';
import { Prose } from '../ui.tsx';

/** What is chosen in the list of projects for a blueprint that is for none of them. */
const NO_PROJECT = '-';

export interface ApplyingProps {
  /** What the draft is kept by: the blueprint and the project it is of. */
  kept: string;
  params: GraphParam[];
  /** What it was last applied with, where it is running. */
  known: Fields;
  asksProject?: boolean;
  /** The environments to choose one of for what it is applied for (a project, or none): nothing where none is asked (`projectenvs::asked`). */
  environments?: (where: string | null) => string[] | null;
  onClose: () => void;
  /** Applies it; `true` once it is applied, and the drawer is then put away. */
  onApply: (values: Fields, where?: string | null, under?: string) => Promise<boolean>;
}

export function Applying(props: ApplyingProps) {
  const [form, setForm] = useState<Written>(() => applyingOf(props.kept) ?? { where: '', values: {} });
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  // A box nobody has written in says what the param starts as, so a param the text has only just been given has its box.
  const said = (param: GraphParam): string => form.values[param.name] ?? written(startOf(param, props.known));
  const write = (next: Written) => {
    keepApplying(props.kept, next);
    setForm(next);
    // What was wrong is being put right.
    if (error) setError('');
  };
  const set = (name: string, text: string) => write({ ...form, values: { ...form.values, [name]: text } });

  const where = props.asksProject ? (form.where === NO_PROJECT || form.where === '' ? null : form.where) : null;
  const choose = props.environments?.(props.asksProject ? where : null) ?? null;
  const closed = applyClosed(choose, form.under ?? '');
  const submit = async (event: Event) => {
    event.preventDefault();
    if (closed) return setError(closed);
    const told = answered(props.params, Object.fromEntries(props.params.map((param) => [param.name, said(param)])));
    if ('wrong' in told) return setError(told.wrong);
    setError('');
    setBusy(true);
    const applied = await props.onApply(told.values, props.asksProject ? (form.where === NO_PROJECT ? null : form.where) : undefined, choose ? form.under : undefined);
    setBusy(false);
    // Kept as written if it was refused, with why said, to be put right.
    if (!applied) return;
    keepApplying(props.kept, null);
    props.onClose();
  };

  return (
    <form class="comparing applying" onSubmit={(event) => void submit(event)}>
      <p class="muted comparing-about">
        {props.params.length > 0
          ? 'What this blueprint is handed when it is applied. A box that is left empty says nothing, and the blueprint goes by what its text says there.'
          : 'A blueprint does what it does to a project, or to the factory as a whole.'}
      </p>
      {props.asksProject && (
        <label class="comparing-field">
          <span class="comparing-label">
            Project <span class="faint">(needed)</span>
          </span>
          {/* Nothing is chosen until somebody chooses: what a blueprint does, it does to a project. */}
          <select required aria-label="Project" value={form.where} onChange={(event) => write({ ...form, where: event.currentTarget.value })}>
            <option value="" disabled>
              Choose a project
            </option>
            {projects.value.map((one) => (
              <option key={one.id} value={one.id}>
                {one.name}
              </option>
            ))}
            <option value={NO_PROJECT}>No project: for the factory as a whole</option>
          </select>
          <span class="comparing-said">This blueprint was opened from the factory's list, so it is not any project's yet.</span>
        </label>
      )}
      {choose && (
        <label class="comparing-field">
          <span class="comparing-label">
            Environment <span class="faint">(needed)</span>
          </span>
          {/* Nothing is chosen until somebody chooses: the project runs everything under one of them. */}
          <select required aria-label="Environment" value={form.under ?? ''} onChange={(event) => write({ ...form, under: event.currentTarget.value })}>
            <option value="" disabled>
              Choose an environment
            </option>
            {choose.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
          <span class="comparing-said">What it reads as environment.vars, the secrets it is handed and what it starts run under it, for as long as it runs.</span>
        </label>
      )}
      {props.params.map((param) => (
        <ParamField key={param.name} param={param} now={said(param)} known={props.known} onSet={(text) => set(param.name, text)} />
      ))}
      {error && <p class="error-text comparing-wrong">{error}</p>}
      {closed && !error && <p class="faint comparing-wrong">{closed}</p>}
      <div class="comparing-foot">
        <button type="button" onClick={props.onClose}>
          Not now
        </button>
        <button class="primary" type="submit" disabled={busy || !!closed} title={closed ?? undefined}>
          Apply
        </button>
      </div>
    </form>
  );
}

/**
 * One thing a blueprint is told: its name, its box under that and what
 * the blueprint says of it under the box. `now` is what is written in
 * the box; `known` is what the blueprint was last applied with, where a
 * list starts from it.
 */
export function ParamField({ param, now, known, onSet }: { param: GraphParam; now: string; known: Fields; onSet: (text: string) => void }) {
  const name = (
    <span class="comparing-label">
      {param.name}{' '}
      <span class="faint">
        ({param.type}
        {param.required ? ', needed' : ''})
      </span>
    </span>
  );
  const says = param.description && (
    <span class="comparing-said">
      <Prose text={param.description} />
    </span>
  );
  // One of a few things: they are there to be chosen, not typed.
  // (A daemon older than this client does not say which; then they are typed.)
  const choices = param.choices ?? [];
  // A list of plain words: each a tag, in an order, with room to make more.
  // The ones it starts with stay to be chosen again after one is left out.
  const starts = param.type === 'list' && choices.length === 0 ? asItems(startOf(param, known) ?? []) : null;
  const field =
    choices.length > 0 ? (
      <Select label={param.name} chosen={now ? [now] : []} options={choices.map(String)} onChange={(chosen) => onSet(chosen.length > 0 ? String(chosen[0]) : '')} />
    ) : starts ? (
      <Select label={param.name} chosen={listed(now) ?? starts} options={starts} many makes ordered onChange={(list) => onSet(JSON.stringify(list))} />
    ) : param.type === 'bool' ? (
      <Choose label={param.name} value={now} options={['true', 'false']} onChange={(chosen) => onSet(chosen)} />
    ) : null;
  if (field) {
    // Not a label: pointing at its words would press what is first in it.
    return (
      <div class="comparing-field">
        {name}
        {field}
        {says}
      </div>
    );
  }
  return (
    <label class="comparing-field">
      {name}
      <input required={param.required} value={now} onInput={(event) => onSet(event.currentTarget.value)} />
      {says}
    </label>
  );
}

/** The plain words a box holds as a list, where it holds one. */
function listed(text: string) {
  try {
    return asItems(JSON.parse(text));
  } catch {
    return null;
  }
}

/** One of a few things, chosen by pressing it. */
function Choose(props: { label: string; value: string; options: string[]; onChange: (chosen: string) => void }) {
  return (
    <div class="pills applying-choice" role="radiogroup" aria-label={props.label}>
      {props.options.map((option) => (
        <button type="button" key={option} role="radio" aria-checked={props.value === option} onClick={() => props.onChange(option)}>
          {option}
        </button>
      ))}
    </div>
  );
}
