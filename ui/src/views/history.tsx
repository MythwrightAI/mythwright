// The history of a blueprint as it was applied: every definition an
// instance has been given, the latest first. Who applied it and when, what
// it changed from the one before, and whether it held. Any of them can be
// read as it was, and applied again.
//
// Each is called by the version its text says, which is the blueprint's,
// and by which of the times the instance was applied it is. The second
// was once called a version too, and a heading that said "Version 1"
// stood over a text that said `version: 2`.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Instance, Version } from '../api.ts';
import { CodeEditor } from '../editor/editor.tsx';
import { coalesce } from '../events.ts';
import { attempt, toast } from '../store.ts';
import { Badge, Empty, Twist, useEvents } from '../ui.tsx';
import { handedOf, headingOf } from '../handed.ts';
import type { Handed } from '../handed.ts';
import { openIn, turnIn } from '../handedopen.ts';
import { when } from '../util.ts';
import { called, changes, nth } from '../versions.ts';

/** What the two things each line is called by are, for whoever points at one. */
const SAYS = 'The version is what the blueprint\'s text says of itself. Which time it was applied is counted for what is running here: applied again with other params, the same text is another time.';

/** How the one that is shown stands, after what it is called: which time it was applied where its heading is the blueprint's version, and what came of it. */
function stood(shown: Version, current: number | undefined): string {
  const time = called(shown).version ? `the ${nth(shown.version)} time it was applied` : null;
  if (shown.outcome === 'reverted') return time ? `${time}, which failed and was undone` : 'failed when it was applied, and was undone';
  if (shown.version === current) return time ? `${time}, which is what is running` : 'what is running';
  return time ? `as it was ${time}` : 'as it was applied';
}

export function History({ instance, tab = '' }: { instance: Instance; tab?: string }) {
  const [versions, setVersions] = useState<Version[] | null>(null);
  const [chosen, setChosen] = useState<number | null>(null);
  const reload = useMemo(() => coalesce(() => void api.versions(instance.id).then(setVersions).catch(() => setVersions([])), 80), [instance.id]);
  useEffect(reload, [reload]);
  // A version is added when the instance is given a definition, and one is marked undone when an update fails.
  useEvents(['instance.started', 'instance.updated', 'instance.update_failed', 'instance.active'], (event) => event.source?.instance === instance.id && reload(), [instance.id]);

  if (versions === null) return <div class="empty">Reading its history</div>;
  if (versions.length === 0) return <Empty title="Nothing is kept of how this was applied" />;
  // What it is running: the latest that held.
  const current = versions.find((one) => one.outcome === 'applied')?.version;
  const shown = versions.find((one) => one.version === chosen) ?? versions[0]!;
  const before = (one: Version) => versions.find((other) => other.version === one.version - 1);

  const again = async (version: Version) => {
    const applied = await attempt(() => api.apply({ project: instance.project, source: version.source, key: instance.key, params: version.params }));
    if (applied) toast(applied.changed ? `Applied ${instance.blueprint} again as it was: ${called(version).whole}.` : `${instance.blueprint} is already running that.`);
  };

  return (
    <div class="history">
      <div class="history-list">
        {versions.map((one) => (
          <button key={one.version} class="nav-item history-row" aria-current={one.version === shown.version} title={SAYS} onClick={() => setChosen(one.version)}>
            <span class="row">
              <strong class="history-called">{called(one).head}</strong>
              {one.version === current && <Badge status="ok">running</Badge>}
              {one.outcome === 'reverted' && <Badge status="failed">undone</Badge>}
            </span>
            <span class="faint history-when">
              {called(one).under ? `${called(one).under} · ` : ''}
              {when(one.applied)}
              {one.by_name ? ` · ${one.by_name}` : ''}
            </span>
            {changes(before(one), one).map((said) => (
              <span key={said} class="history-change">
                {said}
              </span>
            ))}
          </button>
        ))}
      </div>
      <div class="history-shown">
        <div class="row history-head">
          <strong class="history-called" title={SAYS}>
            {called(shown).head}
          </strong>
          <span class="muted">{stood(shown, current)}</span>
          <span class="grow" />
          {/* Only what was applied by somebody can be applied again by them: a child is its parent's to start. */}
          {!instance.parent && shown.version !== current && (
            <button title="Apply the text and the params it was applied with then" onClick={() => void again(shown)}>
              Apply it like this again
            </button>
          )}
        </div>
        <AppliedWith params={shown.params} tab={tab} />
        <div class="history-text">
          <CodeEditor key={shown.version} value={shown.source} language="blueprint" readOnly />
        </div>
      </div>
    </div>
  );
}

/**
 * What the time that is shown was applied with: a heading in words with
 * how many, folded until it is pressed (kept by the tab, as what is
 * unfolded under it is), and under it each name with its value, a
 * mapping or a list folded under its name. Nothing is drawn for none, so
 * that the text begins where it always did; folded, it is one line over
 * the text.
 */
function AppliedWith({ params, tab }: { params: Record<string, unknown>; tab: string }) {
  const [open, setOpen] = useState(() => openIn(tab));
  const heading = headingOf(params);
  if (heading === null) return null;
  const turn = (path: string) => setOpen(turnIn(tab, path));
  const lines = (all: Handed[]) =>
    all.map((one) => (
      <div key={one.path} class="handed-line">
        <span class="handed-name faint">{one.name}</span>
        {one.under ? (
          <span class="handed-inside">
            <button class="ghost handed-fold" aria-expanded={open.has(one.path)} disabled={one.under.length === 0} onClick={() => turn(one.path)}>
              {one.under.length > 0 && <Twist open={open.has(one.path)} />}
              <span>{one.holds}</span>
            </button>
            {open.has(one.path) && <div class="handed-under">{lines(one.under)}</div>}
          </span>
        ) : (
          <span class="handed-value">{one.said}</span>
        )}
      </div>
    ));
  return (
    <div class="handed">
      <button class="ghost handed-head" aria-expanded={open.has('')} onClick={() => turn('')}>
        <Twist open={open.has('')} />
        <span>{heading}</span>
      </button>
      {open.has('') && <div class="handed-lines">{lines(handedOf(params))}</div>}
    </div>
  );
}
