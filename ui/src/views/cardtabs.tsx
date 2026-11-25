// The tabs of a card's form, one row of them: Details, the card itself (its form, what a card that was left
// says, what it cost); Comments, what was said on it, with the box
// something is said in; History, what happened to it; and then a tab for
// each table that sits beside the card's board and whose rows are each
// about a card (`cardtabs.ts`). A tab of rows lists the rows about this
// card, each with what it says, a way to open it whole, and under it the
// rows of other tables that name it, each with what it says; a value that
// names a branch of the project's repository is drawn as that branch,
// which opens to say where it is and what was last committed there. A
// card's A/B tests are one such tab (`std/ab`): each test, and what each
// of its contenders did. Nothing here knows a test, a contender or a judge.

import type { ComponentChildren } from 'preact';
import { useEffect, useMemo, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Collection, HistoryLine } from '../api.ts';
import { aboutCards, cellsOf, linkOf, namedIn, namingFields, rowsAbout, tabOf } from '../cardtabs.ts';
import { offered } from '../byhand.ts';
import type { Question } from '../journey.ts';
import type { Cell } from '../cardtabs.ts';
import type { Branch, Branches } from '../envrun.ts';
import { covers } from '../holds.ts';
import { rowWords } from '../sits.ts';
import { tableCalled } from '../called.ts';
import { attempt, collections, drawer, me } from '../store.ts';
import { RENAMING, knownBy, naming, renames } from '../table.ts';
import type { Field, Row } from '../table.ts';
import { useEvents } from '../ui.tsx';
import { ago, classes } from '../util.ts';
import { openRow } from './records.tsx';
import { RecordTabs } from './recordtabs.tsx';
import type { MoreTab } from './recordtabs.tsx';
import { rowsOf } from './within.tsx';

/**
 * A card's form (`children`) under the one row of its tabs: the tabs every
 * record's form has (`RecordTabs`), and after them a tab for each table
 * whose rows are about a card. `asking` is what the card waits on, whose
 * asker's row is marked in its table's tab. Which tab shows is kept by the
 * tab the drawer is beside, as what is typed in the form is.
 */
export function CardTabs({ project, card, lines, box, at, asking = null, may = false, children }: { project: string; card: string; lines: HistoryLine[] | null; box?: ComponentChildren; at?: number | null; asking?: Question | null; may?: boolean; children: ComponentChildren }) {
  const tables = aboutCards('board', collections.value[project] ?? []);
  const more: MoreTab[] = tables.map((table) => ({ which: table.name, label: tabOf(table), table: table.name, panel: () => <AboutCard project={project} table={table} card={card} asking={asking} /> }));
  return (
    <RecordTabs record={card} kept={`card-tab:${card}`} of="What is kept of this card" lines={lines} box={box} at={at} may={may} project={project} more={more} class="card-tabs">
      {children}
    </RecordTabs>
  );
}

/** The rows of `table` about the card, each with the rows that name it. */
function AboutCard({ project, table, card, asking }: { project: string; table: Collection; card: string; asking: Question | null }) {
  const all = collections.value[project] ?? [];
  // The tables whose rows name a row of this one, and by which field.
  const under = useMemo(
    () => all.flatMap((other) => namingFields(other.fields, table.name).map((field) => ({ table: other, field }))),
    [all, table.name],
  );
  const held = rowsOf(project, table.name);
  const heldUnder = under.map(({ table: other }) => rowsOf(project, other.name));
  const read = () => [held, ...heldUnder].forEach((one) => one.read());
  const names = [table.name, ...under.map(({ table: other }) => other.name)];
  useEffect(read, [project, names.join(' ')]);
  useEvents(RENAMING, (event) => renames(event, project, names) && read(), [project, names.join(' ')]);
  const branches = useBranches(project);
  const scopes = me.value?.principal.scopes ?? [];
  // A table whose rows are written only by what keeps it is offered no new row, as it is nowhere (`byhand.ts`).
  const mayAdd = offered(table).adds && covers(scopes, 'records:write') && [table.takes ?? []].flat().every((scope) => covers(scopes, scope));
  const rows = held.rows.value;
  if (rows === null) return <p class="faint about-card-none">Reading {table.name}</p>;
  const mine = rowsAbout(table, rows, card);
  const one = rowWords(table).one;
  // Opened whole in the drawer, with the way back to the card.
  const openWhole = (name: string, row: Row) => {
    const was = drawer.value;
    openRow(project, name, row, was ? () => (drawer.value = was) : undefined);
  };
  return (
    <div class="about-card" aria-label={tabOf(table)}>
      {mine.length === 0 && <p class="faint">No {one} of this card yet.</p>}
      {mine.map((row) => (
        <section key={row.id} class={classes('about-card-row', asking?.asker === row.id && 'about-card-asks')} data-row={row.id}>
          <div class="about-card-head">
            <strong class="truncate">{knownBy(row, table.fields)}</strong>
            {/* The row that asked what the card waits on: what it names under it is marked. */}
            {asking?.asker === row.id && <span class="about-card-asking">asks</span>}
            <span class="grow" />
            <button type="button" onClick={() => openWhole(table.name, row)} title={`Open this ${one} whole, to see and change all of it`}>
              Open
            </button>
          </div>
          <Cells cells={cellsOf(row, table.fields, table.about ? [table.about] : [])} branches={branches} />
          {under.map(({ table: other, field }, index) => {
            const named = naming(heldUnder[index]!.rows.value ?? [], field, row.id);
            if (named.length === 0) return null;
            return (
              <div key={`${other.name}.${field}`} class="about-card-under" aria-label={other.name}>
                <div class="faint about-card-of">{tableCalled(other)}</div>
                {named.map((each) => {
                  // Named by what the row above it asks, which the card waits on: marked, as the question says it is.
                  const asked = asking?.asker === row.id && namedIn(asking.words, each, other.fields);
                  return (
                  <div key={each.id} class={classes('about-card-named', asked && 'about-card-named-asked')} data-row={each.id} data-asked={asked ? 'true' : undefined}>
                    <button type="button" class="about-card-name truncate" title={`Open this one of ${other.name} whole`} onClick={() => openWhole(other.name, each)}>
                      {knownBy(each, other.fields)}
                    </button>
                    {asked && <span class="about-card-asking">named in what is asked</span>}
                    <Cells cells={cellsOf(each, other.fields, [field])} branches={branches} />
                  </div>
                  );
                })}
              </div>
            );
          })}
        </section>
      ))}
      {mayAdd && table.about && (
        <button
          type="button"
          class="about-card-new"
          onClick={async () => {
            const added = await attempt(() => api.addRecord(project, table.name, { [table.about as string]: card }));
            if (added) {
              read();
              openWhole(table.name, added);
            }
          }}
        >
          + New {one}
        </button>
      )}
    </div>
  );
}

/** The project's branches, read once as the tab opens: they change by git, which says nothing. */
export function useBranches(project: string): Branch[] {
  const [branches, setBranches] = useState<Branch[]>([]);
  useEffect(() => {
    void call<Branches>('GET', `/projects/${encodeURIComponent(project)}/branches`).then(
      (read) => setBranches(read.branches.filter((branch) => !branch.remote)),
      () => setBranches([]),
    );
  }, [project]);
  return branches;
}

/** What a row says, a line a thing: a flag by its name, a list a line each, long words folded, a branch as itself, an address a link. */
export function Cells({ cells, branches }: { cells: Cell[]; branches: Branch[] }) {
  if (cells.length === 0) return null;
  return (
    <dl class="about-card-cells">
      {cells.map((cell) => (
        <div key={cell.name} class={cell.flag ? 'about-card-flag' : 'about-card-cell'}>
          {cell.flag ? (
            <dd class="about-card-mark">{cell.says}</dd>
          ) : (
            <>
              <dt class="faint">{cell.name}</dt>
              <dd>
                <Said cell={cell} branch={branches.find((branch) => branch.name === cell.says)} />
              </dd>
            </>
          )}
        </div>
      ))}
    </dl>
  );
}

function Said({ cell, branch }: { cell: Cell; branch: Branch | undefined }) {
  const [open, setOpen] = useState(false);
  if (branch) {
    return (
      <span class="about-card-branch">
        <button type="button" class="link" aria-expanded={open} title="The branch of the project's repository this names: where it is" onClick={() => setOpen(!open)}>
          <code>{branch.name}</code>
        </button>
        {open && (
          <span class="faint about-card-commit">
            at <code>{branch.commit.slice(0, 10)}</code> {ago(branch.at)}: {branch.subject}
          </span>
        )}
      </span>
    );
  }
  if (cell.lines) return <>{cell.lines.map((line, at) => <div key={at}><Linked words={line} /></div>)}</>;
  if (cell.long) {
    return (
      <details class="about-card-long">
        <summary class="truncate">{cell.says.split('\n')[0]}</summary>
        <div class="about-card-words">{cell.says}</div>
      </details>
    );
  }
  return <Linked words={cell.says} />;
}

/** Words as they are, or, where they are an address a browser opens as a page, a link that opens it in a tab of its own. */
function Linked({ words }: { words: string }) {
  const link = linkOf(words);
  return link ? (
    <a href={link} target="_blank" rel="noopener noreferrer">
      {words}
    </a>
  ) : (
    <>{words}</>
  );
}

export type { Field };
