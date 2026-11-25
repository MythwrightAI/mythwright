// One instance: the blueprint it is running, drawn with how its steps
// went, and what it currently holds.

import { namesOf } from '../envsheld.ts';
import { underSaid } from '../projectenvs.ts';
import { useEffect, useMemo, useState } from 'preact/hooks';
import { api, ApiError } from '../api.ts';
import type { Instance, InstanceDetail } from '../api.ts';
import { coalesce } from '../events.ts';
import { noteOf } from '../noted.ts';
import { tell } from '../running.ts';
import { attempt, drawer, isBlueprint, openTab, toast } from '../store.ts';
import { Badge, Empty, Field, useEvents } from '../ui.tsx';
import { ago, hasContent, show } from '../util.ts';
import { applied as appliedAs, lag, running } from '../versions.ts';
import { BlueprintDoc } from './blueprint.tsx';
import { Running, WithRunning } from './running.tsx';
import { useKept } from './returned.tsx';
import { goneBy } from '../drawerkept.ts';

export function InstanceView({ id }: { id: string }) {
  const [detail, setDetail] = useKept<InstanceDetail | null>('instance.detail', null);
  const [missing, setMissing] = useState('');
  const reload = useMemo(
    () =>
      coalesce(
        () =>
          void api
            .instance(id)
            .then((read) => {
              // What was read says what this id and its children's are, wherever else they are written.
              for (const one of [read.instance, ...read.children]) tell(one.id, { blueprint: one.blueprint, key: one.key, project: one.project });
              setDetail(read);
            })
            .catch((err) => {
              if (err instanceof ApiError && err.status === 404) tell(id, 'gone');
              setMissing(err instanceof Error ? err.message : String(err));
            }),
        80,
      ),
    [id],
  );
  useEffect(reload, [reload]);
  useEvents('instance.*', (event) => (event.source?.instance === id || event.data?.parent === id) && reload(), [id]);
  // Whether it is behind is worked out when it is read, so it is read again when a blueprint's file is said to have changed.
  useEvents(['project.file.changed', 'project.rescanned'], (event) => (!event.data?.path || isBlueprint(event.data.path)) && reload(), [id]);

  if (missing) return <Empty title="This instance cannot be shown">{missing}</Empty>;
  if (!detail) return <div class="empty">Reading the instance</div>;
  const { instance, children } = detail;
  const behind = lag(instance);
  const about = () => {
    drawer.value = { title: titleOf(instance), body: () => <Details instance={instance} children={children} />, gone: goneBy(() => api.instance(instance.id)) };
  };

  // Which environment it runs under, said in its head where its project has any or it runs under one.
  const under = underSaid(instance.environment, namesOf(instance.project).length > 0);
  return (
    <BlueprintDoc title={instance.blueprint} source={detail.source} project={instance.project} instanceId={id} readOnly>
      <span class="faint truncate">
        {instance.key}
        {children.length ? ` · ${children.length} ${children.length === 1 ? 'child' : 'children'}` : ''}
        {under ? ` · ${under}` : ''}
      </span>
      {behind && (
        <button class="lag-mark" title={behind.why} onClick={about}>
          <Badge status="warning">{behind.mark}</Badge>
        </button>
      )}
      <button onClick={about}>Details</button>
    </BlueprintDoc>
  );
}

const titleOf = (instance: Instance) => `${instance.blueprint} (${instance.id})`;

/** What is said of an instance in the drawer: how it stands, the whole of what its row says, and what it was given and holds. */
function Details({ instance, children }: { instance: Instance; children: Instance[] }) {
  const behind = lag(instance);
  // Red only for a failure: what it went on past is a warning (`noted.ts`).
  const note = noteOf(instance);
  const again = async () => {
    const applied = await attempt(() => api.applyAgain(instance.id));
    // What the details said was of the text it had.
    if (applied) drawer.value = null;
    if (applied) toast(appliedAs(instance.blueprint, instance.key, applied.took, applied.changed));
  };
  return (
    <>
      <div class="row">
        <Badge status={instance.status} />
        <span class="muted">updated {ago(instance.updated)}</span>
      </div>
      {note && <pre class={`code-box said-whole ${note.is === 'failure' ? 'error-text' : 'warn-text'}`}>{note.says}</pre>}
      <Field label="Text">
        <code>{running(instance)}</code>
      </Field>
      {behind && (
        <Field label={`Behind ${behind.there}`}>
          <div class="stack" style="gap:6px">
            <span class="muted lag-why">{behind.why}</span>
            {behind.moved && <span class="lag-moved">{behind.moved}</span>}
            {behind.again && (
              <div class="row">
                <button class="lag-again" onClick={() => void again()}>
                  {behind.again}
                </button>
              </div>
            )}
            <span class="faint lag-leave">{behind.leave}</span>
          </div>
        </Field>
      )}
      <Field label="Key">
        <code>{instance.key}</code>
      </Field>
      {instance.project && (
        <Field label="Project">
          <span>{instance.project}</span>
        </Field>
      )}
      {underSaid(instance.environment, namesOf(instance.project).length > 0) && (
        <Field label="Environment">
          <span data-under>{underSaid(instance.environment, namesOf(instance.project).length > 0)}</span>
        </Field>
      )}
      {instance.parent && (
        <Field label="Started by">
          <span>
            <Running id={instance.parent} />
          </span>
        </Field>
      )}
      {hasContent(instance.params) && (
        <Field label="Parameters">
          <pre class="code-box">
            <WithRunning text={show(instance.params)} />
          </pre>
        </Field>
      )}
      {hasContent(instance.state) && (
        <Field label="State">
          <pre class="code-box">
            <WithRunning text={show(instance.state)} />
          </pre>
        </Field>
      )}
      {instance.holds.length > 0 && (
        <Field label="Holds">
          <div class="stack" style="gap:4px">
            {instance.holds.map((hold) => (
              <code key={hold}>{hold}</code>
            ))}
          </div>
        </Field>
      )}
      {children.length > 0 && (
        <Field label="Children">
          <div class="stack" style="gap:4px">
            {children.map((child) => (
              <button key={child.id} style="justify-content:flex-start" onClick={() => openTab({ kind: 'instance', id: child.id })}>
                <Badge status={child.status} />
                <span class="truncate">
                  {child.blueprint} · {child.key}
                </span>
                {child.behind && <Badge status="warning">behind</Badge>}
              </button>
            ))}
          </div>
        </Field>
      )}
    </>
  );
}

/** The drawer, opened from where an instance is listed: its details, read and followed while it is open. */
export function showDetails(instance: Instance): void {
  drawer.value = { title: titleOf(instance), body: () => <ReadDetails key={instance.id} id={instance.id} />, gone: goneBy(() => api.instance(instance.id)) };
}

function ReadDetails({ id }: { id: string }) {
  const [detail, setDetail] = useState<InstanceDetail | null>(null);
  const [missing, setMissing] = useState('');
  const reload = useMemo(
    () =>
      coalesce(
        () =>
          void api
            .instance(id)
            .then(setDetail)
            .catch((err) => setMissing(err instanceof Error ? err.message : String(err))),
        80,
      ),
    [id],
  );
  useEffect(reload, [reload]);
  useEvents('instance.*', (event) => (event.source?.instance === id || event.data?.parent === id) && reload(), [id]);
  if (missing) return <Empty title="This instance cannot be shown">{missing}</Empty>;
  if (!detail) return <div class="empty">Reading the instance</div>;
  return <Details instance={detail.instance} children={detail.children} />;
}
