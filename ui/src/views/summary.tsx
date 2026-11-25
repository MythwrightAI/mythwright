// A blueprint in plain words: the view of a blueprint's page that somebody
// who does not write blueprints can read.
//
// What it says is the daemon's, made from the text and from what each
// action and event says of itself, and handed over with the check's
// answer: so it is of the same text as the graph, and follows what is
// being written as the graph does. It is laid out to be read down
// (`summarylaid.ts`): a head that says what it is, where its text stands
// and its facts in a few words each; then what it does when it is
// applied, what it hears (each a card with its steps folded away, and
// where they are heard from said once where most say the same), what it
// does as it ends, and the rest, what it makes a table of its own with
// what each table it makes is made of (`summarymodel.tsx`). A line that is about a step or a
// handler is pressed to see where that is written; one that leads
// somewhere (`forked.ts`) is pressed to go there, where the page it is
// on can. What is unfolded is kept by the tab (`summaryfolds.ts`). What
// its runs have cost is read from measurements, once when this is shown
// and again when it has run, and said under the rest as counts: it is no
// part of the summary, and where nothing was measured nothing is said.

import { Fragment } from 'preact';
import { useLayoutEffect, useEffect, useRef, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Checked, Diagnostic } from '../api.ts';
import { costLabels, costOf, costSaid, lineOf } from '../summary.ts';
import type { Cost, Opens, SummaryLine, SummaryPart } from '../summary.ts';
import { dataModel, wordy, handlers, headOf, isHandler, ownLines, readOrder, sharedFrom } from '../summarylaid.ts';
import type { Facts, Head } from '../summarylaid.ts';
import { keepOpened, openedIn } from '../summaryfolds.ts';
import { Handler, Lines, MARKS, Made, Steps, Words } from './summarylines.tsx';
import type { Drawing } from './summarylines.tsx';
import { DataModel } from './summarymodel.tsx';

/** How far back what its runs have cost is counted, as it is asked and as it is said. */
const SINCE = { asked: '7d', said: 'the last 7 days' } as const;

export interface SummaryViewProps {
  /** The check's answer for the text that is showing; nothing while it is first being read. */
  checked: Checked | null;
  /** What is wrong with the text, as the page lists it. */
  problems: Diagnostic[];
  project: string | null;
  /** Something that is another thing once the blueprint has run again. */
  ran: string;
  /** Shows a line of the text. */
  onShow: (line: number) => void;
  onProblems: () => void;
  /** What pressing a line that leads somewhere does, where it can be followed from this page; nothing where it cannot, and the line is then words. */
  follows?: (opens: Opens) => (() => void) | null;
}

/** Facts of one label: the label, and each one's value or each thing it lists; one about a step is pressed to see where that is written. */
function Fact({ facts, draw }: { facts: Facts; draw: Drawing }) {
  const value = (line: SummaryLine) => {
    const leads = lineOf(draw.checked.graph, line.at);
    const said = line.items && line.items.length > 0 ? line.items : [line.title ?? line.text];
    return said.map((item) =>
      leads !== null ? (
        <button key={item} class="summary-item summary-words" title="See where this is written" onClick={() => draw.onShow(leads)}>
          <Words text={item} />
        </button>
      ) : (
        <span key={item} class="summary-item">
          <Words text={item} />
        </span>
      ),
    );
  };
  return (
    <div class={`summary-fact${wordy(facts) && facts.lines.length > 1 ? ' summary-wordy' : ''}`} data-label={facts.label}>
      <dt class="summary-label">{facts.label}</dt>
      <dd class="summary-value">{facts.lines.flatMap(value)}</dd>
    </div>
  );
}

/** What it is: where its text stands, what it says it is, its facts, and what it may be told. */
function TheHead({ head, part, draw }: { head: Head; part: SummaryPart | null; draw: Drawing }) {
  return (
    <section class="summary-part summary-head" data-part={part?.kind ?? 'is'}>
      <h2 class="summary-hidden">{part?.title}</h2>
      {head.standing.length > 0 && <Lines lines={head.standing} kind="standing" ordered={false} leads={false} draw={draw} />}
      {head.lead && (
        <ul class="summary-lines">
          <li class="summary-line summary-lead">
            <div class="summary-said">
              <span class="summary-words">
                <Words text={head.lead.text} />
              </span>
            </div>
          </li>
        </ul>
      )}
      {(head.facts.length > 0 || head.marks.length > 0) && (
        <dl class="summary-facts">
          {head.facts.map((facts) => (
            <Fact key={facts.label} facts={facts} draw={draw} />
          ))}
          {head.marks.map((mark) => (
            <div key={mark} class="summary-fact summary-marked">
              <dd class="summary-value">
                <span class="summary-mark">{MARKS[mark] ?? mark}</span>
              </dd>
            </div>
          ))}
        </dl>
      )}
      {head.rest.length > 0 && <Lines lines={head.rest} kind="is" ordered={false} leads draw={draw} />}
    </section>
  );
}

/** A part under its heading, drawn as what it holds: steps, what is heard, what is done as it ends, or lines. */
function Part({ part, draw }: { part: SummaryPart; draw: Drawing }) {
  const lines = ownLines(part);
  if (lines.length === 0) return null;
  const count = part.kind === 'hears' ? handlers(part) : 0;
  let body;
  if (part.kind === 'does') body = <Steps lines={lines} draw={draw} />;
  else if (part.kind === 'makes') body = <Made lines={lines} draw={draw} />;
  else if (part.kind === 'hears') {
    const shared = sharedFrom(lines);
    const notes = lines.filter((line) => !isHandler(line));
    body = (
      <>
        {(notes.length > 0 || shared) && (
          <ul class="summary-lines summary-notes">
            {notes.map((line, at) => (
              <li key={at} class="summary-line">
                <div class="summary-said">
                  <span class="summary-words">
                    <Words text={line.text} />
                  </span>
                </div>
              </li>
            ))}
            {shared && (
              <li class="summary-from">
                <Words text={shared} />
              </li>
            )}
          </ul>
        )}
        <ul class="summary-cards">
          {lines.map((line, at) => (isHandler(line) ? <Handler key={at} line={line} at={at} draw={draw} shared={shared} /> : null))}
        </ul>
      </>
    );
  } else if (part.kind === 'ends')
    body = (
      <ul class="summary-lines summary-endings">
        {lines.map((line, at) => (
          <li key={at} class="summary-line summary-ending">
            <div class="summary-said">
              <span class="summary-words">
                <Words text={line.text} />
              </span>
            </div>
            {line.under.length > 0 && <Steps lines={line.under} draw={draw} />}
          </li>
        ))}
      </ul>
    );
  else body = <Lines lines={lines} kind={part.kind} ordered={part.ordered} leads={false} draw={draw} />;
  return (
    <section class="summary-part" data-part={part.kind}>
      <h2>
        {part.title}
        {count > 0 && <span class="summary-count">{count}</span>}
      </h2>
      {body}
    </section>
  );
}

/** Which folds are open, kept by the tab the view is in so that they are as they were left when it comes back. */
function useFolds(root: { current: HTMLElement | null }, drawn: boolean) {
  const tab = useRef<string | null>(null);
  const [opened, setOpened] = useState<ReadonlySet<string>>(new Set());
  useLayoutEffect(() => {
    tab.current = root.current?.closest<HTMLElement>('[data-tab]')?.dataset.tab ?? null;
    if (tab.current) setOpened(openedIn(tab.current));
  }, [drawn]);
  const turn = (fold: string) =>
    setOpened((now) => {
      const next = new Set(now);
      const open = !next.has(fold);
      if (open) next.add(fold);
      else next.delete(fold);
      if (tab.current) keepOpened(tab.current, fold, open);
      return next;
    });
  return { opened: (fold: string) => opened.has(fold), turn };
}

export function SummaryView(props: SummaryViewProps) {
  const { checked, project } = props;
  const summary = checked?.summary ?? null;
  const name = summary?.name ?? null;
  const [cost, setCost] = useState<{ of: string; cost: Cost | null } | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const folds = useFolds(root, !!summary);
  useEffect(() => {
    if (!name) return;
    let stale = false;
    const of = costLabels(name, project);
    api
      .metrics({ name: '*', since: SINCE.asked, labels: of })
      .then((series) => !stale && setCost({ of, cost: costOf(series) }))
      // Somebody who may not read measurements is told nothing of them, and the summary is still theirs to read.
      .catch(() => !stale && setCost({ of, cost: null }));
    return () => {
      stale = true;
    };
  }, [name, project, props.ran]);

  if (!checked) return <div class="empty">Reading the blueprint</div>;
  if (!summary) {
    const errors = props.problems.filter((problem) => problem.severity === 'error');
    return (
      <div class="summary-none">
        <strong>This cannot be said in plain words until it checks</strong>
        <ul class="summary-wrong">
          {errors.map((problem, at) => (
            <li key={at}>
              {/* What is wrong with the whole of a text is on no line of it. */}
              {problem.span.start.line > 0 && <span class="faint">line {problem.span.start.line} </span>}
              <Words text={problem.message} />
            </li>
          ))}
        </ul>
        <button onClick={props.onProblems}>See them in the text</button>
      </div>
    );
  }
  // What was read for another blueprint, or for this one of another project, is not this one's.
  const spent = cost && cost.of === costLabels(summary.name, project) ? cost.cost : null;
  const draw: Drawing = { checked, onShow: props.onShow, follows: props.follows, ...folds };
  const parts = readOrder(summary.parts);
  return (
    <div class="summary" data-of={summary.name} ref={root}>
      <div class="summary-page">
        <TheHead head={headOf(parts)} part={parts.find((part) => part.kind === 'is') ?? null} draw={draw} />
        {parts.map((part) =>
          part.kind === 'is' ? null : (
            <Fragment key={part.kind}>
              <Part part={part} draw={draw} />
              {/* What the tables it makes are made of, together, where it makes two or more. */}
              {part.kind === 'makes' && dataModel(part.lines).length > 0 && <DataModel tables={dataModel(part.lines)} />}
            </Fragment>
          ),
        )}
        {spent && (
          <section class="summary-part summary-cost" data-part="cost">
            <h2>What its runs have cost</h2>
            <ul class="summary-lines">
              {costSaid(spent, SINCE.said).map((said) => (
                <li key={said} class="summary-line">
                  <div class="summary-said">
                    <span class="summary-words">{said}</span>
                  </div>
                </li>
              ))}
            </ul>
          </section>
        )}
      </div>
    </div>
  );
}
