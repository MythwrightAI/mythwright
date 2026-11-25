// The lines of a blueprint's Summary, as they are drawn: a line of words,
// a step with what holds it under it, and what something hears as a card
// of its own, with its steps folded away behind one press.
//
// Every word is the daemon's. A line handed taken apart (`title`, `only`,
// `from`, `takes`: `summarylaid.ts`) is drawn by its parts, and any other
// by its words, as it always was. A line about a step or a handler is
// pressed to see where that is written; one that leads somewhere is
// pressed to go there, where the page it is on can.

import { lineOf, pieces } from '../summary.ts';
import type { Opens, SummaryLine } from '../summary.ts';
import type { Checked } from '../api.ts';
import { Fragment } from 'preact';
import { MOST_ROWS, fieldsSaid, foldKey, foldsRows, madeOf, modelFold, startedFrom, folds, fromOf, isHandler, rows } from '../summarylaid.ts';
import { ModelName, ModelView } from './summarymodel.tsx';

/** What a marked line is marked as, in a word. */
export const MARKS: Record<string, string> = { command: 'runs a command', secret: 'reaches a secret' };

/** What every line is drawn with: the text's graph, what pressing does, and which folds are open. */
export interface Drawing {
  checked: Checked;
  onShow: (line: number) => void;
  /** What pressing a line that leads somewhere does, where it can be followed from this page; nothing where it cannot, and the line is then words. */
  follows?: (opens: Opens) => (() => void) | null;
  /** Whether a fold is open, by what it is known by within the tab. */
  opened: (fold: string) => boolean;
  turn: (fold: string) => void;
}

/** A line's words, with what was set apart as written drawn as that. */
export function Words({ text }: { text: string }) {
  return (
    <>
      {pieces(text).map((piece, at) => (piece.code ? <code key={at}>{piece.text}</code> : <span key={at}>{piece.text}</span>))}
    </>
  );
}

/** A line's own words, pressed to go where it leads, or to see where it is written, where it is either. */
function Said({ line, words, draw, extra }: { line: SummaryLine; words: string; draw: Drawing; extra?: string }) {
  const goes = line.opens ? (draw.follows?.(line.opens) ?? null) : null;
  const leads = lineOf(draw.checked.graph, line.at);
  const kind = `summary-words${extra ? ` ${extra}` : ''}`;
  if (goes)
    return (
      <button class={`${kind} summary-goes`} data-opens={line.opens?.what} onClick={goes}>
        <Words text={words} />
      </button>
    );
  if (leads !== null)
    return (
      <button class={kind} title="See where this is written" onClick={() => draw.onShow(leads)}>
        <Words text={words} />
      </button>
    );
  return (
    <span class={kind}>
      <Words text={words} />
    </span>
  );
}

/** What holds a line to when, for what or where, and where it hears from where that is its own: one under another, quietly. */
function Only({ only, from }: { only: string[]; from?: string | null }) {
  if (only.length === 0 && !from) return null;
  return (
    <ul class="summary-only">
      {from && (
        <li data-from="">
          <Words text={from} />
        </li>
      )}
      {only.map((said) => (
        <li key={said}>
          <Words text={said} />
        </li>
      ))}
    </ul>
  );
}

/** A step, or something done as a blueprint ends: its words, what the text calls it at the right, what holds it under them, and its own steps under that. */
function Step({ line, draw }: { line: SummaryLine; draw: Drawing }) {
  return (
    <li class="summary-line summary-step" data-at={line.at ?? undefined} data-mark={line.mark ?? undefined}>
      <div class="summary-said">
        <Said line={line} words={line.title ?? line.text} draw={draw} />
        {line.key && <code class="summary-key">{line.key}</code>}
      </div>
      <Only only={line.title ? (line.only ?? []) : []} />
      {line.under.length > 0 && <Steps lines={line.under} draw={draw} />}
    </li>
  );
}

/** Steps one after another, numbered. */
export function Steps({ lines, draw }: { lines: SummaryLine[]; draw: Drawing }) {
  return (
    <ol class="summary-lines summary-steps">
      {lines.map((line, at) => (
        <Step key={at} line={line} draw={draw} />
      ))}
    </ol>
  );
}

/** What something hears, as a card: the event first, what holds it, how it takes what it hears, and its steps behind one press. */
export function Handler({ line, at, draw, shared }: { line: SummaryLine; at: number; draw: Drawing; shared: string | null }) {
  const fold = foldKey('hears', line, at);
  const open = draw.opened(fold);
  const steps = line.under.length;
  const id = `steps-${fold.replace(/[^\w-]/g, '-')}`;
  return (
    <li class="summary-line summary-card" data-at={line.at ?? undefined}>
      <div class="summary-said">
        <Said line={line} words={line.title ?? line.text} draw={draw} extra="summary-heard" />
        {line.key && <code class="summary-key">{line.key}</code>}
      </div>
      <Only only={line.title ? (line.only ?? []) : []} from={line.title ? fromOf(line, shared) : null} />
      {line.title && (line.tags?.length ?? 0) > 0 && (
        <div class="summary-tags">
          {line.tags!.map((one) => (
            <span key={one} class="summary-tag">
              <Words text={one} />
            </span>
          ))}
        </div>
      )}
      {steps > 0 && (
        <button class="ghost summary-fold summary-unfolds" aria-expanded={open} aria-controls={id} onClick={() => draw.turn(fold)}>
          {steps === 1 ? '1 step' : `${steps} steps`}
        </button>
      )}
      {steps > 0 && open && (
        <div id={id} class="summary-under">
          <Steps lines={line.under} draw={draw} />
        </div>
      )}
    </li>
  );
}

/** A line of words: a fact, a scope, a table it makes, a way to change it. One with a long list under it folds it away. */
export function OneLine({ line, kind, at, leads, draw }: { line: SummaryLine; kind: string; at: number; leads: boolean; draw: Drawing }) {
  const folded = folds(line) && !isHandler(line);
  const fold = foldKey(kind, line, at);
  const open = !folded || draw.opened(fold);
  // A param's name is what its line is known by, and goes before it; a step's key, a handler's name and a scope go after.
  const key = line.key && <code class="summary-key">{line.key}</code>;
  const goes = line.opens ? (draw.follows?.(line.opens) ?? null) : null;
  // A way to change it: what is done, pressed where it leads, and what is said of it under that, quietly.
  if (line.label && line.title)
    return (
      <li class="summary-line summary-action" data-at={line.at ?? undefined}>
        <div class="summary-said">
          <Said line={line} words={line.label} draw={draw} extra="summary-act" />
        </div>
        <div class="summary-detail">
          <Words text={line.title} />
        </div>
        {line.under.length > 0 && <Lines lines={line.under} kind={`${kind}:${at}`} ordered={false} leads={leads} draw={draw} />}
      </li>
    );
  return (
    <li class="summary-line" data-at={line.at ?? undefined} data-mark={line.mark ?? undefined}>
      <div class="summary-said">
        {line.mark && <span class="summary-mark">{MARKS[line.mark] ?? line.mark}</span>}
        {leads && key}
        {folded && !goes ? (
          <button class="summary-words summary-fold" aria-expanded={open} onClick={() => draw.turn(fold)}>
            <Words text={line.text} />
          </button>
        ) : (
          <Said line={line} words={line.text} draw={draw} />
        )}
        {!leads && key}
      </div>
      {line.under.length > 0 && open && <Lines lines={line.under} kind={`${kind}:${at}`} ordered={line.at !== null} leads={leads} draw={draw} />}
    </li>
  );
}

/** Lines of words, one after another where they are things done in order. */
export function Lines({ lines, kind, ordered, leads, draw }: { lines: SummaryLine[]; kind: string; ordered: boolean; leads: boolean; draw: Drawing }) {
  const List = ordered ? 'ol' : 'ul';
  return (
    <List class="summary-lines">
      {lines.map((line, at) => (
        <OneLine key={at} line={line} kind={kind} at={at} leads={leads} draw={draw} />
      ))}
    </List>
  );
}

/**
 * What it makes and starts, as a table: a row for
 * each thing made, saying what it is, what it is called (pressed to open
 * it where it is another blueprint; a table's name pointed at shows what
 * it is made of in a card, and pressed unfolds that under the row), what
 * it is started as and how, how many fields a table has, and from where,
 * each pressed to see where that is written. A long list shows its first
 * rows until the rest is asked for. Under a phone's width its cells stand
 * one under another.
 */
export function Made({ lines, draw }: { lines: SummaryLine[]; draw: Drawing }) {
  const laid = rows(lines);
  const fold = 'makes:rows';
  const open = !foldsRows(laid.length) || draw.opened(fold);
  const shown = open ? laid : laid.slice(0, MOST_ROWS);
  const anyModel = laid.some(({ line }) => !!line.model);
  const anyWhat = laid.some(({ line }) => !!line.made);
  return (
    <>
      <table class="summary-made">
        <thead>
          <tr>
            {anyWhat && <th scope="col">What</th>}
            <th scope="col">Called</th>
            {anyModel && <th scope="col">Made of</th>}
            <th scope="col">From</th>
          </tr>
        </thead>
        <tbody>
          {shown.map(({ line, lines: each }, at) => {
            const made = madeOf(line);
            const unfold = modelFold(line.at, at);
            const unfolded = !!line.model && draw.opened(unfold);
            const span = 2 + (anyWhat ? 1 : 0) + (anyModel ? 1 : 0);
            return (
              <Fragment key={at}>
                <tr class="summary-line summary-made-row" data-mark={line.mark ?? undefined} data-what={line.made?.what ?? undefined}>
                  {anyWhat && (
                    <td class="summary-made-what" data-col="What">
                      {made.what}
                    </td>
                  )}
                  <td class="summary-made-name" data-col="Called">
                    <div class="summary-said">
                      {line.model ? (
                        <ModelName name={made.name} model={line.model} open={unfolded} onPress={() => draw.turn(unfold)}>
                          <Words text={made.name} />
                        </ModelName>
                      ) : (
                        <Said line={{ ...line, at: null }} words={made.name} draw={draw} extra="summary-what" />
                      )}
                      {line.named && (
                        <span class="summary-named">
                          <span class="summary-as">as</span>{' '}
                          <span class="summary-name">
                            <Words text={line.named} />
                          </span>
                        </span>
                      )}
                      {(line.tags ?? []).map((tag) => (
                        <span key={tag} class="summary-tag">
                          <Words text={tag} />
                        </span>
                      ))}
                    </div>
                    <Only only={line.title ? (line.only ?? []) : []} />
                  </td>
                  {anyModel && (
                    <td class="summary-made-of" data-col="Made of">
                      {line.model && (
                        <button class="ghost summary-fold summary-unfolds" aria-expanded={unfolded} onClick={() => draw.turn(unfold)}>
                          {fieldsSaid(line.model)}
                        </button>
                      )}
                    </td>
                  )}
                  <td class="summary-by" data-col="From">
                    {each.map((one) => {
                      const leads = lineOf(draw.checked.graph, one.at);
                      const said = startedFrom(one);
                      return leads !== null && said ? (
                        <button key={one.at ?? said} class="summary-step-of" data-at={one.at ?? undefined} title="See where this is written" onClick={() => draw.onShow(leads)}>
                          {said}
                        </button>
                      ) : (
                        <span key={one.at ?? said} class="summary-step-of" data-at={one.at ?? undefined}>
                          {said}
                        </span>
                      );
                    })}
                  </td>
                </tr>
                {unfolded && line.model && (
                  <tr class="summary-model-row">
                    <td colSpan={span}>
                      <ModelView model={line.model} />
                    </td>
                  </tr>
                )}
              </Fragment>
            );
          })}
        </tbody>
      </table>
      {foldsRows(laid.length) && (
        <button class="ghost summary-fold summary-unfolds summary-more-rows" aria-expanded={open} onClick={() => draw.turn(fold)}>
          {open ? 'Fewer' : `${laid.length - MOST_ROWS} more`}
        </button>
      )}
    </>
  );
}
