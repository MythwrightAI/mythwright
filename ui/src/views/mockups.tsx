// A project's mockups: pages made to try an idea with, each a folder of
// plain web files. The gallery shows every one as it is now, small, with
// its variants beside it; one is opened to be looked at properly, as wide
// as a phone, a tablet or the room there is, alone or beside another.
//
// A mockup's page is somebody's, or an agent's, and its scripts run. It
// is shown in a frame that is a place of its own, from an address that
// reads the mockup's own folder and nothing else (`api.way`): it is given
// neither this page's session nor any way to this page.

import { signal } from '@preact/signals';
import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { coalesce } from '../events.ts';
import { CHANGES, WIDTHS, addressOf, changes, fileOf, folderOf, groups, isMockups, nameOf, statusOf, titleOf, variantOf, variantTitle, widthCalled } from '../mockups.ts';
import type { Kept } from '../mockups.ts';
import { attempt, collections, drawer, openTab, projectName } from '../store.ts';
import type { Field as Column, Row } from '../table.ts';
import { columns as columnsOf } from '../table.ts';
import { Empty, Field, Modal, useEvents, useFocus } from '../ui.tsx';
import { Commented, CommentIcon } from './comments.tsx';
import { Laid, ViewsHead, useViews } from './laid.tsx';
import { TableActs } from './tablehead.tsx';
import { Titled } from './pagehead.tsx';
import { headWord, ThingWas } from './thingwas.tsx';

/** What of a page of mockups is kept while its tab is not showing: which is open, what is beside it, how wide. */
interface Shown {
  open: string | null;
  beside: string | null;
  width: string;
}
const NOTHING: Shown = { open: null, beside: null, width: 'whole' };
const showing = signal<Record<string, Shown>>({});

export function MockupsView({ project, name }: { project: string; name: string }) {
  const kept = (collections.value[project] ?? []).find((one) => one.name === name && isMockups(one));
  const at = `${project}:${name}`;
  const shown = showing.value[at] ?? NOTHING;
  const show = (next: Partial<Shown>) => {
    showing.value = { ...showing.value, [at]: { ...shown, ...next } };
  };
  // `null` while they are first being read.
  const [rows, setRows] = useState<Row[] | null>(null);
  // How often each mockup's files have changed since this page was shown: a frame is drawn again when its count moves.
  const [drawn, setDrawn] = useState<Record<string, number>>({});
  const [making, setMaking] = useState<{ of: Row | null } | null>(null);
  const reload = useMemo(() => coalesce(() => void api.records(project, name).then(setRows, () => setRows([])), 120), [project, name]);
  useEffect(() => {
    setRows(null);
    reload();
  }, [reload]);
  // Read once, and again when something says they may have changed: one
  // made or changed here, or a file put in a mockup's folder by anything.
  useEvents(
    CHANGES,
    (event) => {
      if (!kept) return;
      const what = changes(event, project, kept);
      if (what.list) reload();
      const folder = what.drawn;
      if (folder) setDrawn((was) => ({ ...was, [folder]: (was[folder] ?? 0) + 1 }));
    },
    [project, name, kept?.at],
  );

  // Their own look is the gallery; other views of the same mockups are saved beside it.
  const views = useViews(project, kept, name, 'gallery');
  if (!kept) {
    return (
      <Empty title="There are no mockups here">
        A project has them once a blueprint makes a folder of it theirs: <code>std/mockups</code> does. A mockup is then a folder there that holds an <code>index.html</code>.
      </Empty>
    );
  }
  const all = rows ?? [];
  const open = all.find((row) => row.id === shown.open) ?? null;
  const count = (row: Row) => drawn[nameOf(row)] ?? 0;
  const stages = kept.fields.find((field) => field.name === 'status')?.of ?? [];
  const taken = all.map(titleOf);

  if (open) {
    const beside = all.find((row) => row.id === shown.beside && row.id !== open.id) ?? null;
    const width = widthCalled(shown.width);
    const of = variantOf(open);
    const varies = of ? all.find((row) => row.id === of) : undefined;
    const comments = () => {
      const body = () => (
        <Commented key={open.id} record={open.id} path={fileOf(kept, open)} empty="Nothing else has been kept of this mockup yet. What is made and changed through here is; what another program does to its files is not, since nobody saw who did it." />
      );
      drawer.value = { title: titleOf(open), body };
    };
    return (
      <div class="split">
        <div class="view-head mockup-head">
          <button class="ghost" title="Back to all the mockups" onClick={() => show({ open: null, beside: null })}>
            ‹ All
          </button>
          <h1 class="truncate">{titleOf(open)}</h1>
          {of && (
            <button class="ghost faint truncate" title="What this is another way of doing" disabled={!varies} onClick={() => varies && show({ open: varies.id, beside: open.id })}>
              a variant of {varies ? titleOf(varies) : 'one that is no longer here'}
            </button>
          )}
          <span class="grow" />
          <div class="pills" role="radiogroup" aria-label="How wide it is shown">
            {WIDTHS.map((one) => (
              <button key={one.name} role="radio" aria-checked={one.name === width.name} title={one.says} onClick={() => show({ width: one.name })}>
                {one.label}
              </button>
            ))}
          </div>
          <select aria-label="Beside it" title="Another mockup, shown beside this one at the same width" value={beside?.id ?? ''} onChange={(event) => show({ beside: event.currentTarget.value || null })}>
            <option value="">Alone</option>
            {all
              .filter((row) => row.id !== open.id)
              .map((row) => (
                <option key={row.id} value={row.id}>
                  Beside {titleOf(row)}
                </option>
              ))}
          </select>
          {stages.length > 0 && (
            <select aria-label="How far it has got" title="How far it has got" value={statusOf(open)} onChange={(event) => void attempt(() => api.setRecord(open.id, { status: event.currentTarget.value }))}>
              {stages.map((stage) => (
                <option key={stage} value={stage}>
                  {stage}
                </option>
              ))}
            </select>
          )}
          <button class="ghost" title="A copy of this one to change: another way of doing the same thing" onClick={() => setMaking({ of: open })}>
            Variant
          </button>
          <button class="ghost" title={`Its page, to write in: ${fileOf(kept, open)}`} onClick={() => openTab({ kind: 'file', project, path: fileOf(kept, open) })}>
            Edit
          </button>
          <button class="ghost" title="What has been said on it, with a line to add a comment, and after that what has happened to it" onClick={comments}>
            <CommentIcon />
            Comments
          </button>
          <button class="ghost" title={`Open it by itself, as on a phone: ${location.origin}${addressOf(open.id)} for whoever is signed in and may read this project`} onClick={() => window.open(addressOf(open.id), '_blank', 'noopener')}>
            Its address
          </button>
        </div>
        <div class={`mockup-stage${beside ? ' two' : ''}`}>
          <Pane row={open} across={width.across} drawn={count(open)} named={beside !== null} />
          {beside && <Pane row={beside} across={width.across} drawn={count(beside)} named />}
        </div>
        {making && <Making project={project} kept={kept} of={making.of} taken={taken} onClose={() => setMaking(null)} onMade={(made) => show({ open: made.id, beside: making.of?.id ?? null })} />}
      </div>
    );
  }

  const folder = folderOf(kept);
  const gallery = (
    <div class="mockup-gallery">
      {rows === null && <p class="faint">Reading</p>}
      {rows?.length === 0 && (
        <Empty title="No mockups yet">
          Begin one here, ask an agent for one in the project's channel, or put a folder with an <code>index.html</code> in {folder ? <code>{folder}/</code> : 'the project'}: it is a mockup as soon as it is there.
        </Empty>
      )}
      {groups(all).map((group) => (
        <section key={group.first.id} class="mockup-group" aria-label={titleOf(group.first)}>
          {[group.first, ...group.variants].map((row) => (
            <div key={row.id} class={`mockup-card${row.id === group.first.id ? '' : ' variant'}`}>
              <div class="mockup-thumb">
                <Frame id={row.id} title={titleOf(row)} drawn={count(row)} small />
                <button class="mockup-open" aria-label={`Open ${titleOf(row)}`} title={`Open ${titleOf(row)}`} onClick={() => show({ open: row.id, beside: null })} />
              </div>
              <div class="mockup-line">
                <span class="truncate">{titleOf(row)}</span>
                {row.id !== group.first.id && <span class="faint">variant</span>}
                <span class="grow" />
                {/* What a mockup says of itself is words, and never the name of a style: a class here is any class there is. */}
                {statusOf(row) && <span class="badge mockup-status" data-status={statusOf(row)}>{statusOf(row)}</span>}
              </div>
            </div>
          ))}
        </section>
      ))}
    </div>
  );
  return (
    <div class="split">
      <div class="view-head">
        <Titled>
          <h1>
            {projectName(project)} {headWord(project, `mockups:${name}`, name)}
          </h1>
          <ThingWas project={project} thing={`mockups:${name}`} />
          {rows && (
            <span class="faint">
              {rows.length} {rows.length === 1 ? 'mockup' : 'mockups'}
            </span>
          )}
          <span class="faint truncate" title="Each folder here that holds an index.html is a mockup">
            in {folder ? `${folder}/` : 'the project itself'}
          </span>
        </Titled>
        <span class="grow" />
        <TableActs tab={{ kind: 'mockups', project, name }} table={kept} rows={rows} />
        <button class="primary" onClick={() => setMaking({ of: null })}>
          New mockup
        </button>
        <ViewsHead views={views} />
      </div>
      <Laid
        views={views}
        own={{ gallery: () => gallery }}
        fields={columnsOf(kept.fields)}
        rows={all}
        people={[]}
        what="mockup"
        onOpen={(row) => show({ open: row.id, beside: null })}
        onChange={async (row: Row, field: Column, value: unknown) => {
          await attempt(() => api.setRecord(row.id, { [field.name]: value }));
          reload();
        }}
      />
      {making && <Making project={project} kept={kept} of={making.of} taken={taken} onClose={() => setMaking(null)} onMade={(made) => show({ open: made.id, beside: null })} />}
    </div>
  );
}

/** One mockup on the stage, as wide as was chosen. */
function Pane({ row, across, drawn, named }: { row: Row; across: number | null; drawn: number; named: boolean }) {
  return (
    <div class="mockup-pane">
      {named && <div class="mockup-pane-name faint truncate">{titleOf(row)}</div>}
      <div class="mockup-holder" style={{ width: across ? `${across}px` : '100%' }}>
        <Frame id={row.id} title={titleOf(row)} drawn={drawn} />
      </div>
    </div>
  );
}

/**
 * A mockup, in a frame that is a place of its own. Its address is a way
 * in to its files that is asked for as it is shown; `drawn` moves when
 * one of its files has changed, and the frame is then drawn again.
 */
function Frame({ id, title, drawn, small }: { id: string; title: string; drawn: number; small?: boolean }) {
  const [way, setWay] = useState<string | null>(null);
  const [why, setWhy] = useState('');
  useEffect(() => {
    let live = true;
    setWhy('');
    api.way(id).then(
      (found) => live && setWay(found.path),
      (failed: unknown) => live && setWhy(failed instanceof Error ? failed.message : String(failed)),
    );
    return () => {
      live = false;
    };
  }, [id]);
  if (why) return <div class="mockup-missing faint">{why}</div>;
  if (!way) return <div class="mockup-missing faint">Opening</div>;
  return <iframe key={`${way}:${drawn}`} class={small ? 'mockup-small' : 'mockup-frame'} title={title} sandbox="allow-scripts allow-forms allow-modals" src={way} tabIndex={small ? -1 : 0} />;
}

/** Begins a mockup: a new one from a plain page, or a variant as a copy of another. */
function Making({ project, kept, of, taken, onClose, onMade }: { project: string; kept: Kept; of: Row | null; taken: string[]; onClose: () => void; onMade: (made: Row) => void }) {
  const [title, setTitle] = useState(of ? variantTitle(of, taken) : '');
  const [busy, setBusy] = useState(false);
  const focus = useFocus<HTMLInputElement>();
  const make = async (event: Event) => {
    event.preventDefault();
    if (!title.trim() || busy) return;
    setBusy(true);
    const fields = of ? { title: title.trim(), of: of.id } : { title: title.trim() };
    const made = await attempt(() => api.addRecord(project, kept.name, fields));
    setBusy(false);
    if (made) {
      onClose();
      onMade(made);
    }
  };
  const folder = folderOf(kept);
  return (
    <Modal onClose={onClose}>
      <form class="stack" onSubmit={make}>
        <h2>{of ? `A variant of ${titleOf(of)}` : 'A new mockup'}</h2>
        <Field label="What it is called">
          <input ref={focus} value={title} aria-label="What it is called" onInput={(event) => setTitle(event.currentTarget.value)} />
        </Field>
        <p class="faint">
          {of
            ? `It begins as a copy of everything in ${folder ? `${folder}/` : ''}${nameOf(of)}/, in a folder of its own, to be changed.`
            : `It is a folder in ${folder ? `${folder}/` : 'the project'} named by what it is called, with a plain page to begin from.`}
        </p>
        <div class="row">
          <span class="grow" />
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" disabled={!title.trim() || busy}>
            {busy ? 'Making it' : 'Make it'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
