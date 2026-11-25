// A card's form: what a card is called, what it says and holds, where it stands, what was said on it and what
// it offers to be pressed. It is the one form of a card, drawn wherever a card is opened: in the drawer beside
// its board (`board.tsx`), and in the Tools panel beside a channel (`context.tsx`), which reads what the form
// has to know of the board for itself (`useAbout`).

import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Board, Item } from '../api.ts';
import { coalesce } from '../events.ts';
import { covers } from '../holds.ts';
import { placesFor } from '../phone.ts';
import { Select } from '../select.tsx';
import { agents, attempt, draftOf, keepDraft, me, speaker } from '../store.ts';
import { givable } from '../givable.ts';
import { workspaces } from '../spaces.ts';
import { leftFor } from '../left.ts';
import { RENAMING, asks, extras, flagColumn, inUse, names, planned, renames, sentBack, waitsFor } from '../table.ts';
import type { Collection } from '../api.ts';
import type { Field, Named, Row } from '../table.ts';
import { useEvents } from '../ui.tsx';
import { saidDeleted } from './trash.tsx';
import { ago } from '../util.ts';
import { addedBy, asker, questionOf, waitingOn } from '../journey.ts';
import { CardCost } from './cardcost.tsx';
import { CardRows } from './cardrows.tsx';
import { CardTabs } from './cardtabs.tsx';
import { answerIn } from '../cardasked.ts';
import { assignedTo } from '../assigned.ts';
import { usePeople } from '../peopleknown.ts';
import { assignedWords } from './assignee.tsx';
import { useJourney } from './journey.tsx';
import { TagBox } from './tagbox.tsx';
import { SendBack, useStages } from './sendback.tsx';
import { among } from '../cardactions.ts';
import type { Action } from '../cardactions.ts';
import { CardActions } from './cardactions.tsx';
import { useBeside } from './sits.tsx';
import { FieldInput } from './table.tsx';
import { useKept } from './returned.tsx';

/** A card as a row of the table it is in. */
export const rowOf = (item: Item): Row => ({
  id: item.id,
  number: item.number,
  // What else its board keeps of it first: a card's own are what they are.
  data: { ...item.more, title: item.title, body: item.body, status: item.status, assignee: item.assignee, labels: item.labels, due: item.due ?? null },
  position: item.position,
  created: item.created,
  updated: item.updated,
});

/** What a card's form has to know of its board: its columns, and what else the board keeps of a card. */
export interface About {
  columns: string[];
  /** The board keeps a day for a card. */
  dated: boolean;
  /** The fields its blueprint declared beyond a card's own, each changed by what its kind is changed with. */
  extras: Field[];
  people: { value: string; name: string }[];
  others: Named;
  /** The rows of the project's `stages`, where it has a fleet: which says where a card that is sent back goes. */
  stages: Row[] | null;
  /** What a column offers to be pressed on a card there (`cardactions.ts`). */
  actions: Action[];
  /** The labels there are to pick: those its table declared, then those already on its cards (`inUse`). */
  labels: string[];
}

/** What a card's form is told of the board it is on. */
export function aboutOf(board: Board, fields: Field[], others: Named, stages: Row[] | null, actions: Action[]): About {
  const rows = board.items.map(rowOf);
  return {
    stages,
    actions,
    columns: board.columns,
    dated: fields.some((field) => field.name === 'due' && field.kind === 'date'),
    extras: extras(fields),
    people: agents.value.map((slot) => ({ value: `agent:${slot.name}`, name: slot.name })),
    others: { ...others, board: { fields, rows } },
    labels: inUse(fields, 'labels', rows),
  };
}

/**
 * What is asked on a card and not yet answered, as it is now. The form is
 * of the card as it was when it was opened; this is read again whenever
 * the card is changed, so that a question that has just been answered
 * leaves, and one that has just been asked arrives.
 */
function useQuestion(item: Item): string {
  const [question, setQuestion] = useState(asks(item.more));
  useEvents(
    ['board.item.updated'],
    (event) => {
      if (event.data?.id !== item.id) return;
      void api.record(item.id).then(
        (card) => setQuestion(asks(card.data)),
        () => undefined,
      );
    },
    [item.id],
  );
  return question;
}

/**
 * A card's form, wherever a card is opened: in the drawer beside its board, and in the Tools panel beside a
 * channel. `onDone` is what is done once it is saved, deleted or sent back (the drawer put away; the panel
 * back at its board). Each part is drawn as far as whoever reads it may see and change it: what changing a
 * card takes (`board:write`), and what a field's table says giving it a value takes beside, is asked of what
 * they hold, and a part they may not change is shown and closed; one they may not read is not drawn.
 */
export function CardForm({ item, about, at, onChanged, onDone }: { item: Item; about: About; at?: number; onChanged: () => void; onDone: () => void }) {
  const { columns, dated } = about;
  const held = me.value?.principal.scopes ?? [];
  const changes = covers(held, 'board:write');
  // A field of what else the board keeps may take more than changing the card does.
  // Whom each it may be given to is, said as a card says it wherever it is drawn (`assigned.ts`): never by an account's id.
  const people = usePeople();
  const whoIs = (assignee: string) => assignedWords(assignedTo(assignee, agents.value, me.value ? { actor: me.value.actor, name: me.value.account?.name ?? null } : null, people), false);
  const mayGive = (field: Field) => changes && [field.takes ?? []].flat().every((scope) => covers(held, scope));
  // What is written in the form is kept by the tab it was opened beside, with the drawer: given back, it is as it was left.
  const [due, setDue] = useKept(`card.due:${item.id}`, item.due ?? '');
  // Where it stands. A card is carried from column to column with a mouse;
  // a finger cannot carry one, so it is also said here, for either.
  const [status, setStatus] = useKept(`card.status:${item.id}`, item.status);
  const [title, setTitle] = useKept(`card.title:${item.id}`, item.title);
  const [body, setBody] = useKept(`card.body:${item.id}`, item.body);
  // A label at a time, so that one may hold a comma.
  const [labels, setLabels] = useKept<string[]>(`card.labels:${item.id}`, item.labels);
  // What else its board keeps of it: only what was changed here, so that
  // saving the form says nothing of a field nobody touched.
  const [more, setMore] = useKept<Record<string, unknown>>(`card.more:${item.id}`, {});
  const [assignee, setAssignee] = useKept(`card.assignee:${item.id}`, item.assignee ?? '');
  // What has happened to the card and what was said on it; and what it
  // is waiting on an answer to, if anything, which the card says itself
  // and its history says whose.
  const lines = useJourney(item.id);
  const added = addedBy(lines, speaker.value?.name ?? null);
  const question = useQuestion(item);
  const asked = question ? { who: lines ? asker(lines) : null, words: question } : null;
  // Whether its round left it for a person, and why: what its row keeps, said and not filled in.
  const left = leftFor(item.more?.again, item.moved, item.more?.taken);
  // What is being said on the card is kept like what is wrong with it.
  const unsaid = `say:${item.id}`;
  const [saying, setSaying] = useState(draftOf(unsaid)?.text ?? '');
  // Whether it is marked public: it is not unless whoever says it turns
  // that on, and it is off again for the next thing said. It is kept with
  // the words, so that words written to be public do not go private, or
  // the other way, for having looked at another tab meanwhile.
  const [open, setOpen] = useState(draftOf(unsaid)?.public === true);
  const keep = (text: string, marked: boolean) => keepDraft(unsaid, text.trim() ? { text, base: '', public: marked } : null);
  const write = (text: string) => {
    setSaying(text);
    keep(text, open);
  };
  const mark = (marked: boolean) => {
    setOpen(marked);
    keep(saying, marked);
  };
  const sayIt = async () => {
    const words = saying.trim();
    if (!words) return;
    if (await attempt(() => api.say(item.id, words, open))) {
      setSaying('');
      setOpen(false);
      keepDraft(unsaid, null);
      onChanged();
    }
  };
  const sayBox = (
    // Where what is asked on the card is answered, outlined in the accent while it waits on one.
    <div class={asked ? 'card-say card-answering' : 'card-say'}>
      {/* Somebody named with @ is told in their box, where they may read the card (`tagbox.tsx`). */}
      <TagBox
        rows={asked ? 3 : 2}
        value={saying}
        label={asked ? 'Your answer' : 'Say something on this card'}
        placeholder={asked ? `What ${asked.who ?? 'whoever asked'} needs to know. It is given the card again with what you say.` : 'Say something on this card. Name somebody with @ and they are told in their inbox.'}
        onInput={write}
      />
      <div class="row">
        <label class="say-public" title="What is said on a card stays in the factory unless it is marked public. The mark is kept with what was said and shown in the history, and is what anything that keeps this card somewhere else as well goes by.">
          <input type="checkbox" checked={open} onChange={(event) => mark(event.currentTarget.checked)} />
          Public
        </label>
        <span class="grow" />
        <button type="button" disabled={!saying.trim()} onClick={sayIt}>
          {asked ? 'Answer' : 'Say'}
        </button>
      </div>
    </div>
  );
  const sent = () => {
    onChanged();
    onDone();
  };

  const save = async (event: Event) => {
    event.preventDefault();
    const done = await attempt(() =>
      api.changeItem(item.id, {
        title,
        body,
        labels,
        ...(Object.keys(more).length ? { fields: more } : {}),
        assignee: assignee || null,
        // Only when it was moved: said again, where it stands would put it at the foot of its own column.
        ...(status !== item.status ? { status } : {}),
        // Only where the board keeps a day for a card: one made before it did has nowhere to put one.
        ...(dated ? { due: due || null } : {}),
      }),
    );
    if (done) {
      onChanged();
      onDone();
    }
  };

  return (
    // One row of tabs: Details, which is this form; what was said on it, with the box to say more, apart from what
    // happened to it; and a tab for each table whose rows are about a card (a card's
    // A/B tests): `cardtabs.tsx`.
    <CardTabs project={item.project} card={item.id} lines={lines} box={changes && sayBox} at={at} asking={asked && lines ? questionOf(lines) : null} may={changes}>
    <form class="stack card-form" onSubmit={save} style={flagColumn(about.extras) ? { '--flag-column': flagColumn(about.extras) } : undefined}>
      {asked && (
        // The question is in Comments, said there by whoever asked and naming whom it was put to; it is answered
        // there, and here is only a line that says so and leads there with the box having the keyboard.
        <button type="button" class="card-asks card-waits" onClick={() => answerIn(item.id)}>
          {waitingOn(lines ? (questionOf(lines)?.whom ?? null) : null)}
        </button>
      )}
      {planned(item.more) && (
        // What whoever planned the card said: read here, and changed by saying something on the card.
        <div class="card-plan" role="group" aria-label="How it is to be done">
          <span class="faint">How it is to be done, as was said when it was planned</span>
          <p>{planned(item.more)}</p>
        </div>
      )}
      {left && !asked && (
        <div class="card-asks card-left" role="group" aria-label="Left for a person">
          <span class="faint">
            Nobody is working this card: it was left here for a person{left.from ? `, and is taken up again when it is put back in ${left.from}` : ''}
          </span>
          {left.why && <p>{left.why}</p>}
        </div>
      )}
      {waitsFor(item.more) && <p class="faint">It waits its turn: {waitsFor(item.more)}.</p>}
      {sentBack(item.more) > 0 && (
        <p class="faint">
          It has been sent back {sentBack(item.more) === 1 ? 'once' : `${sentBack(item.more)} times`}.
        </p>
      )}
      {/* A card that is being looked over can be sent back to whoever worked on it, saying what is wrong: where to is its stage's to say. */}
      <CardActions item={item} actions={about.actions} />
      {changes && <SendBack item={item} columns={columns} stages={about.stages} onSent={sent} />}
      {/* What it holds, shown and closed to whoever may not change a card. */}
      <fieldset class="card-fields" disabled={!changes}>
      <label>
        Title
        <input required value={title} onInput={(event) => setTitle(event.currentTarget.value)} />
      </label>
      <label>
        Details
        <textarea rows={8} value={body} onInput={(event) => setBody(event.currentTarget.value)} />
      </label>
      <div class="field-many">
        <span>Labels</span>
        <Select label="Labels" chosen={labels} options={about.labels} many makes onChange={(next) => setLabels(next.map(String))} />
      </div>
      <label>
        Column
        <select value={status} onChange={(event) => setStatus(event.currentTarget.value)}>
          {placesFor(columns, item.status).map((column) => (
            <option key={column} value={column}>
              {column}
            </option>
          ))}
        </select>
      </label>
      <label>
        Assigned to
        <select value={assignee} onChange={(event) => setAssignee(event.currentTarget.value)}>
          <option value="">nobody</option>
          {item.assignee && !agents.value.some((slot) => `agent:${slot.name}` === item.assignee) && (
            <option value={item.assignee}>{whoIs(item.assignee)}</option>
          )}
          {/* The agents of its project, of its workspace and of the factory: never those of another workspace's projects. */}
          {givable(agents.value, item.project, workspaces.value, item.assignee).map((slot) => (
            <option key={slot.name} value={`agent:${slot.name}`}>
              {whoIs(`agent:${slot.name}`)}
            </option>
          ))}
        </select>
      </label>
      {dated && (
        <label>
          Due
          <input type="date" value={due} onInput={(event) => setDue(event.currentTarget.value)} />
        </label>
      )}
      {/* What else this board keeps of a card, as its blueprint declared it. */}
      {about.extras.map((field) => (
        <fieldset key={field.name} class="card-fields" disabled={!mayGive(field)}>
        <FieldInput
          key={field.name}
          field={field}
          value={field.name in more ? more[field.name] : item.more?.[field.name]}
          people={about.people}
          others={about.others}
          self={item.id}
          onChange={(next) => setMore({ ...more, [field.name]: next })}
        />
        </fieldset>
      ))}
      </fieldset>
      <p class="faint">
        {/* By whoever its history says added it: the factory's bot where a blueprint did, the blueprint after it, and never whoever applied it. */}
        In {item.status} · added {ago(item.created)} by {added?.who ?? item.created_by.split(':')[0]}
        {added?.via && ` · via ${added.via}`}
      </p>
      {changes && (
      <div class="row">
        <button
          type="button"
          class="danger"
          onClick={async () => {
            const gone = await attempt(() => api.removeItem(item.id));
            if (gone !== undefined) {
              // Into the Trash, with Undo, which puts it back in its column.
              saidDeleted(gone as { trash?: string; kept?: number | null }, item.title || item.id);
              onChanged();
              onDone();
            }
          }}
        >
          Delete
        </button>
        <span class="grow" />
        <button class="primary" type="submit">
          Save
        </button>
      </div>
      )}
      <CardRows project={item.project} card={item.id} />
      <CardCost id={item.id} />
    </form>
    </CardTabs>
  );
}

/**
 * What the cards of a board name, where the board's blueprint gave them a field that names a record of
 * another collection: a card's sprint. Another card is already there; anything else is read, and read again
 * when one of its records is added, changed or taken away, when that collection is declared again, or when a
 * file of the project changes, which is how one kept as files is added to by hand. Kept by the tab under `kept`.
 */
export function useOthers(project: string, fields: Field[], kept: string): Named {
  const naming = [...new Set(fields.filter((field) => names(field) && field.of[0] !== 'board').map((field) => field.of[0]!))].sort().join(' ');
  const [others, setOthers] = useKept<Named>(kept, {});
  const readOthers = useMemo(
    () =>
      coalesce(() => {
        if (naming === '') return;
        void api
          .collections(project)
          .then(async (all) => {
            const read: Named = {};
            for (const name of naming.split(' ')) {
              const of = all.find((one) => one.name === name);
              if (of) read[name] = { fields: of.fields, rows: await api.records(project, name) };
            }
            setOthers(read);
          })
          .catch(() => undefined);
      }, 50),
    [project, naming],
  );
  useEffect(readOthers, [readOthers]);
  useEvents(RENAMING, (event) => renames(event, project, naming.split(' ')) && readOthers(), [project, readOthers]);
  return others;
}

/**
 * What a card's form has to know of the board `board` of `project`, read for a card opened away from its
 * board's page (the Tools panel): the board's table as its blueprint declared it, what its cards name, where
 * a card sent back goes and what a column offers to be pressed. Nothing until all of it is read.
 */
export function useAbout(project: string, board: Board | null): About | null {
  const [table, setTable] = useKept<Collection | null>(`card.table:${project}`, null);
  const read = useMemo(() => coalesce(() => project && void api.collections(project).then((all) => setTable(all.find((one) => one.name === 'board') ?? null), () => undefined), 50), [project]);
  useEffect(read, [read]);
  useEvents(['collection.ensured', 'collection.fields'], (event) => event.data?.project === project && read(), [project, read]);
  const fields = table?.fields ?? [];
  const others = useOthers(project, fields, `card.others:${project}`);
  const stages = useStages(project);
  const sat = useBeside(project, 'board');
  if (!board || !table || stages === undefined) return null;
  return aboutOf(board, fields, others, stages, among(sat));
}
