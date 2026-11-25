// The Reference: everything a blueprint can say, as a page in a tab.
//
// The words of the language, the actions with what each takes down to
// what is written inside an argument, and the events with what each
// carries. Every name and every sentence here is the daemon's (what it
// hands an editor of the language): the page is drawn from the model that
// is made of that (`reference.ts`) and from nothing else, and writes only
// the few words that say what a mark is.
//
// What is typed in the box at its top finds what says it, in a name or a
// sentence, across the three parts; the parts fold; a thing is unfolded
// for what it takes, gives and carries. Each entry has an address, which
// its mark copies, and the page opened at an address shows that entry
// unfolded and in view.
//
// An entry is shown by its example, a whole blueprint coloured as the
// editor colours it with a mark that copies it, and by where the
// blueprints there are use it, each a line that opens that blueprint's
// text there. Both are the daemon's. An example the daemon has a
// scenario of is run by Run it beside it, in a sandbox in the drawer
// (`scenario.tsx`).
//
// It is read once, and again when a package is added or taken away,
// which is all that changes what a program describes while it runs.

import { signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useEffect, useLayoutEffect } from 'preact/hooks';
import { linkTo } from '../address.ts';
import type { BlueprintLanguage } from '../api.ts';
import { classOf, coloured } from '../blueprint/coloured.ts';
import { readScenarios, readUsed } from '../blueprint/told.ts';
import { stream } from '../events.ts';
import { PARTS, asked as askedFor, found, matches, partOf, pathTo, reference } from '../reference.ts';
import type { Entry, List, Mark, Part, PartName } from '../reference.ts';
import { shownEntry } from '../referenced.ts';
import { scrollsItself } from '../returned.ts';
import { blueprintLanguage, openTab } from '../store.ts';
import { Twist } from '../ui.tsx';
import { classes } from '../util.ts';
import { copy } from './pagehead.tsx';
import { openScenario } from './scenario.tsx';
import { signs } from './signs.tsx';

/** What the daemon describes, as the page's parts: nothing until it is read, and `null` where it could not be. */
const told = signal<Part[] | null | undefined>(undefined);
/** The language as it was read, by which an example is coloured. */
let known: BlueprintLanguage | null = null;

async function read(): Promise<void> {
  const [language, used, scenarios] = await Promise.all([blueprintLanguage(), readUsed(), readScenarios()]);
  known = language;
  told.value = language ? reference(language, used, scenarios) : null;
}

// A package brings actions with it, and takes them away again. What was
// read is let go of by whoever keeps it for the editor, which hears the
// same: so this asks once that has been done.
stream.on(['extension.added', 'extension.removed'], () => void Promise.resolve().then(read));

// What somebody did with the page, kept while it is away: a view is on the page only while its tab is showing.
const typed = signal('');
const folded = signal<ReadonlySet<PartName>>(new Set());
const unfolded = signal<ReadonlySet<string>>(new Set());
/** The entries inside another whose example somebody asked to see. */
const exampled = signal<ReadonlySet<string>>(new Set());
/** The entry the page was last opened at, which is drawn marked. */
const lit = signal<string | null>(null);
/** Which asking the page was last unfolded for, and which was last brought into view: each is, once, in that order. */
let unfoldedFor = 0;
let brought = 0;

const turned = <T,>(set: ReadonlySet<T>, one: T): Set<T> => {
  const next = new Set(set);
  if (!next.delete(one)) next.add(one);
  return next;
};

// The part of what a blueprint's text may say is called Keywords on the page, and `words` everywhere
// else: an entry's address (`word/<mapping>/<word>`) and the daemon's own sentences are as they were.
const CALLED: Record<PartName, string> = { words: 'Keywords', actions: 'Actions', events: 'Events' };
const LISTED: Record<List['what'], string> = { 'one-of': 'One of', follows: 'Followed by', gives: 'Gives', carries: 'Carries' };

/** A sentence with `code` in backticks, as it should look. */
function Prose({ text }: { text: string }) {
  return (
    <>
      {text.split('`').map((part, index) => (index % 2 === 1 ? <code key={index}>{part}</code> : part))}
    </>
  );
}

/** A mark in the few words that say what it is; what it says of itself is the daemon's. */
function Marked({ mark, of }: { mark: Mark; of: Entry }) {
  const code = <code>{mark.says}</code>;
  const said = (): ComponentChildren => {
    switch (mark.what) {
      case 'needed':
        return 'needed';
      case 'either':
        return <>one of {code}, never two</>;
      case 'needs':
        return <>needs {code}</>;
      case 'makes':
        return <>makes {mark.says}</>;
      case 'named':
        return <>named by {code}</>;
      case 'setting':
        return <>its rows set what something does where {code} is said</>;
      case 'runs':
        return <>runs {code} as a command</>;
      case 'reaches':
        return <>reaches {code} by itself</>;
      case 'again':
        return 'may be done again';
      case 'open':
        return of.kind === 'event' ? 'carries more than is listed' : 'takes more than is listed';
      case 'each':
        return (
          <>
            each: <Prose text={mark.says ?? ''} />
          </>
        );
      case 'hears':
        return mark.says ? <>heard by whoever holds {code}</> : 'heard by anybody signed in';
      case 'passing':
        return 'passed on, not kept: no handler hears it';
    }
  };
  return (
    <span class="ref-mark" data-mark={mark.what}>
      {said()}
    </span>
  );
}

/** The small mark that copies an entry's address. */
function Address({ entry }: { entry: Entry }) {
  const link = linkTo({ kind: 'reference', at: entry.id });
  if (!link) return null;
  return (
    <button class="ghost icon ref-address" aria-label={`Copy the address of ${entry.name}`} title="Copy an address that opens the Reference here" onClick={() => void copy(link, 'The address was copied.')}>
      {signs.link()}
    </button>
  );
}

/** A blueprint's text as the editor colours it, read only. */
export function Code({ text }: { text: string }) {
  return (
    <pre class="ref-code">
      {coloured(text, known).map((line, at) => (
        <span key={at} class="ref-code-line">
          {line.map((piece, index) => (piece.type ? <span key={index} class={classOf(piece)}>{piece.text}</span> : piece.text))}
          {'\n'}
        </span>
      ))}
    </pre>
  );
}

/** Where in a blueprint something is, as its graph names it: a step, a handler, a param. */
function placeOf(at: string): string {
  if (at === '') return 'at its top';
  if (at.startsWith('on.')) return at.includes('/') ? `step ${at.slice(at.indexOf('/') + 1)} of handler ${at.slice(3, at.indexOf('/'))}` : `handler ${at.slice(3)}`;
  return at.includes('.') ? at.replace('.', ' ') : `step ${at}`;
}

/** An entry's example, with what copies it, and the blueprints that use it, each opened at the place. */
function Example({ entry }: { entry: Entry }) {
  const used = entry.used;
  return (
    <div class="ref-example">
      {entry.example && (
        <div class="ref-code-box">
          <h4>Example</h4>
          <Code text={entry.example} />
          <button class="ghost icon ref-copy" aria-label={`Copy the example of ${entry.name}`} title="Copy this example" onClick={() => void copy(entry.example!, 'The example was copied.')}>
            {signs.copy()}
          </button>
          {entry.scenario && (
            <div class="ref-run-line">
              <button class="ref-run" title="Run this example in a sandbox, with what its scenario does to it, and see what comes of it" onClick={() => openScenario(entry.name, entry.scenario!)}>
                Run it
              </button>
            </div>
          )}
        </div>
      )}
      {used && (
        <div class="ref-used">
          <h4>Used in</h4>
          <ul>
            {used.places.map((place) => (
              <li key={place.blueprint}>
                <button class="ref-used-line" onClick={() => openTab({ kind: 'blueprint', name: place.blueprint }, { line: place.line })}>
                  <code>{place.blueprint}</code>
                  <span>{placeOf(place.at)}</span>
                  <span class="ref-is">line {place.line}</span>
                </button>
              </li>
            ))}
          </ul>
          {used.more > 0 && <p class="ref-more">and {used.more === 1 ? 'one more' : `${used.more} more`}</p>}
        </div>
      )}
    </div>
  );
}

/** What is listed inside an entry and under it: the words it may be, what it gives or carries, and the entries it holds. */
function Inside({ entry, shows }: { entry: Entry; shows: boolean }) {
  return (
    <div class="ref-inside">
      {shows && <Example entry={entry} />}
      {entry.lists.map((list) => (
        <div class="ref-list" key={list.what} data-list={list.what}>
          <h4>{LISTED[list.what]}</h4>
          <dl>
            {list.rows.map((row) => (
              <div key={row.name}>
                <dt>{row.name}</dt>
                <dd>
                  {row.is && <span class="ref-is">{row.sometimes ? `${row.is}, sometimes` : row.is}</span>}
                  <Prose text={row.says} />
                </dd>
              </div>
            ))}
          </dl>
        </div>
      ))}
      {entry.under.length > 0 && (
        <div class="ref-under">
          {entry.under.map((one) => (
            <Shown key={one.id} entry={one} within />
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * One entry. One at the top of a part is a line that unfolds for what is
 * inside it; one inside another is drawn whole, to every level, since the
 * thing it is of was unfolded to read it.
 */
function Shown({ entry, within, open }: { entry: Entry; within?: boolean; open?: boolean }) {
  const shows = !!(entry.example || entry.used);
  const holds = entry.lists.length > 0 || entry.under.length > 0 || shows;
  const unfold = within || open || unfolded.value.has(entry.id);
  // One inside another shows its example where it was asked for: pressed, found, or opened at.
  // Found by what was typed is the entry itself saying it, and not only the one it is inside.
  const asked = lit.value === entry.id || (typed.value.trim() !== '' && matches(entry, askedFor(typed.value)));
  const example = !within || asked || exampled.value.has(entry.id);
  const head = (
    <>
      <code class="ref-name">{entry.name}</code>
      {entry.is && <span class="ref-is">{entry.is}</span>}
    </>
  );
  return (
    <div class={classes('ref-entry', lit.value === entry.id && 'ref-lit')} id={`ref:${entry.id}`} data-entry={entry.id} data-kind={entry.kind}>
      <div class="ref-line">
        {within || !holds ? (
          <span class="ref-head">{head}</span>
        ) : (
          <button class="ref-head" aria-expanded={unfold} onClick={() => (unfolded.value = turned(unfolded.value, entry.id))}>
            <Twist open={unfold} />
            {head}
          </button>
        )}
        {entry.marks.map((mark, index) => (
          <Marked key={index} mark={mark} of={entry} />
        ))}
        {within && shows && !asked && (
          <button class="ref-shows" aria-expanded={example} onClick={() => (exampled.value = turned(exampled.value, entry.id))}>
            {entry.example ? 'Example' : 'Used in'}
          </button>
        )}
        <Address entry={entry} />
      </div>
      {entry.says && (
        <p class="ref-says">
          <Prose text={entry.says} />
        </p>
      )}
      {holds && unfold && <Inside entry={entry} shows={shows && example} />}
    </div>
  );
}

export default function Reference() {
  const parts = told.value;
  // Asked for where it was never read, and again where an earlier reading did not come in.
  useEffect(() => {
    if (!told.peek()) void read();
  }, []);

  // Opened at an entry: what hides it is put away, it is unfolded with what it is of, and marked.
  const asked = shownEntry.value;
  useEffect(() => {
    if (!parts || !asked || asked.asked === unfoldedFor) return;
    unfoldedFor = asked.asked;
    const path = pathTo(parts, asked.at);
    const part = partOf(asked.at);
    if (!path || !part) {
      // It names nothing there is: the page is as it stands.
      brought = asked.asked;
      return;
    }
    typed.value = '';
    if (folded.peek().has(part)) folded.value = turned(folded.peek(), part);
    unfolded.value = new Set(unfolded.peek()).add(path[0]!.id);
    lit.value = path.at(-1)!.id;
  }, [parts, asked?.asked]);
  // And brought into view once it is drawn so: after the page was unfolded for this asking, and not before.
  // Read here so that the page is drawn again, and this asked again, when either changes.
  const marked = lit.value;
  const opened = unfolded.value;
  useLayoutEffect(() => {
    if (!asked || asked.asked !== unfoldedFor || asked.asked === brought || !marked || opened.size === 0) return;
    const drawn = document.getElementById(`ref:${marked}`);
    if (!drawn) return;
    brought = asked.asked;
    // Where the page was left scrolled, put back as its tab shows again, is let be (`returned.ts`).
    scrollsItself();
    drawn.scrollIntoView({ block: 'center' });
  });

  const result = parts ? found(parts, typed.value) : null;
  const searching = typed.value.trim() !== '';
  const open = new Set(result?.open);
  return (
    <div class="reference">
      <div class="view-head ref-top">
        <h1>Reference</h1>
        <input
          class="ref-find"
          type="search"
          aria-label="Find in the Reference"
          placeholder="Find a keyword, an action or an event"
          value={typed.value}
          onInput={(event) => (typed.value = event.currentTarget.value)}
        />
      </div>
      <div class="view-body ref-body">
        {parts === undefined && <div class="empty">Reading what a blueprint can say</div>}
        {parts === null && (
          <div class="empty">
            <strong>It could not be read</strong>
            <button onClick={() => void read()}>Read it again</button>
          </div>
        )}
        {result &&
          PARTS.map((name) => {
            const part = result.parts.find((one) => one.name === name)!;
            const shut = folded.value.has(name);
            const count = result.counts[name];
            return (
              <section class="ref-part" key={name} data-part={name} data-count={count}>
                <button class="ref-part-head" aria-expanded={!shut} onClick={() => (folded.value = turned(folded.value, name))}>
                  <Twist open={!shut} />
                  <h2>{CALLED[name]}</h2>
                  <span class="count">{searching ? `${count} found` : count}</span>
                </button>
                {!shut && (
                  <div class="ref-entries">
                    {part.entries.map((one) => (
                      <Shown key={one.id} entry={one} open={open.has(one.id)} />
                    ))}
                    {searching && part.entries.length === 0 && <p class="ref-none">Nothing here says that.</p>}
                  </div>
                )}
              </section>
            );
          })}
      </div>
    </div>
  );
}
