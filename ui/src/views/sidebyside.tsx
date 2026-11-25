// Sandbox tests side by side, as a view of a blueprint's page: the same
// things done to two texts of the blueprint or more, each in a sandbox of
// its own, and what each came to in a column beside the others.
//
// Which texts there are is what the page knows of its blueprint: the text
// as it stands on the page, the file as it is saved, the default as it
// ships and the project's fork of it (`sidebyside.ts`). What is played
// against each is what the sandbox test of this tab was run with and had
// done to it, so that what was tried by hand on one text is tried on the
// others. The daemon plays them (`/api/sandboxes/beside`) and lays what
// each came to out as a comparison's columns are: counts, never scores,
// in the order the texts were chosen. Under each column's counts is its
// own sandbox as the Sandbox view draws one (`Happened`, `Foot` of
// `sandboxed.tsx`), its foot the daemon's own sentence that it was a
// sandbox test. Then one press each: the column's text applied, the
// default forked where the page offers a fork, and the texts compared on
// real work, which opens the drawer that starts a comparison with each as
// a way.
//
// What was played is kept by the tab (`sandboxkept.ts`), and its
// sandboxes are ended when it is played again and when the tab is let go.
// Nothing of any of it is drawn outside this view.

import { useEffect, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Checked, Fields, Graph } from '../api.ts';
import { covers } from '../holds.ts';
import { NOTHING, NONE_BESIDE, besides, endBeside, keepBeside, sandboxOf } from '../sandboxkept.ts';
import { needsFirst, outcome, paramsOf } from '../sandbox.ts';
import { allTheSame, asWays, asking, cannotPlay, chosenFirst, countsOf, forksOf, offered, playedSays, ranOf, standing, toggled } from '../sidebyside.ts';
import type { Column, SideBySide, Text } from '../sidebyside.ts';
import { me, meta, projectName } from '../store.ts';
import { Empty } from '../ui.tsx';
import { classes } from '../util.ts';
import { compareTexts } from './compared.tsx';
import { Foot, Happened } from './sandboxed.tsx';
import { toldOf } from './sandboxacts.tsx';

export interface SideBySideProps {
  tab: string;
  source: string;
  project: string | null;
  checked: Checked | null;
  errors: number;
  /** What the blueprint was last applied with in the factory, where it runs there. */
  known: Fields;
  /** The file the page is, where it is one, and whether what the page holds is other than what is saved. */
  path?: string;
  dirty?: boolean;
  /** The page is what ships under its name, as it ships. */
  isDefault: boolean;
  /** The blueprint ships: the page is the default, or a fork of it. */
  ships: boolean;
  /** Applies a text of it with what it is told, as the page's own Apply would. */
  onApply: (source: string, params: Fields) => void;
  /** Forks the default for the project, where the page offers that. */
  onFork: (() => void) | null;
  /** Shows the page's text to be changed, where whoever looks may change it. */
  onText: (() => void) | null;
}

const why = (err: unknown): string => (err instanceof Error ? err.message : String(err));

export default function SideBySideView(props: SideBySideProps) {
  const { tab, project, checked } = props;
  const name = checked?.name ?? null;
  const graph = checked?.graph ?? null;
  const kept = besides.value[tab] ?? NONE_BESIDE;
  // What else of the blueprint the page has: its file as saved, what ships, and the forks of it. Each is
  // nothing until it has been read, and what is played waits for all three: played before, the texts would
  // be those the page had read so far.
  const [saved, setSaved] = useState<string | null | undefined>(undefined);
  const [shipped, setShipped] = useState<string | null | undefined>(undefined);
  const [forks, setForks] = useState<{ project: string; called: string; source: string }[] | undefined>(undefined);
  useEffect(() => {
    let here = true;
    if (props.path && project) void api.readFile(project, props.path).then((text) => here && setSaved(text), () => here && setSaved(null));
    else setSaved(null);
    return () => void (here = false);
    // Read again when what the page holds is saved, or comes to be other than what is.
  }, [props.path, project, props.dirty]);
  useEffect(() => {
    let here = true;
    if (name && props.ships && !props.isDefault) void api.blueprintSource(name).then((text) => here && setShipped(text), () => here && setShipped(null));
    else setShipped(null);
    return () => void (here = false);
  }, [name, props.ships, props.isDefault]);
  useEffect(() => {
    let here = true;
    if (!name || !props.ships) {
      setForks([]);
      return;
    }
    // Every project's fork of it that whoever looks may read, each read as it is saved.
    void call<{ own: { project: string; name: string; file: string }[] }>('GET', '/blueprints/stands')
      .then((stands) => Promise.all(forksOf(stands.own, name, { project, path: props.path ?? null }).map((one) => api.readFile(one.project, one.file).then((source) => ({ project: one.project, called: projectName(one.project), source })))))
      .then(
        (read) => here && setForks(read),
        () => here && setForks([]),
      );
    return () => void (here = false);
  }, [name, props.ships, project, props.path]);
  const read = checked !== null && saved !== undefined && shipped !== undefined && forks !== undefined;
  const texts = offered({
    source: props.source,
    saved: props.path ? (saved ?? null) : null,
    ships: props.ships,
    shipped: props.isDefault ? props.source : (shipped ?? null),
    isDefault: props.isDefault,
    forks: forks ?? [],
  });
  const stands = standing({ read, texts, file: !!props.path, changes: props.onText !== null, forks: props.onFork !== null });
  const chosen = (kept.chosen ?? chosenFirst(texts)).filter((word) => texts.some((text) => text.word === word));
  const picked: Text[] = texts.filter((text) => chosen.includes(text.word));
  // What is played against each: what the sandbox test of this tab was run with, and each thing done to it.
  const test = sandboxOf(tab);
  const plan = test?.plan ?? { first: needsFirst(name, meta.value?.project_starts ?? []), told: {}, answers: 'default' as const, acts: [] };
  const told = graph ? paramsOf(graph.params, toldOf(test ?? NOTHING, graph, props.known)) : { wrong: 'The text is being read.' };
  const cannot = props.errors > 0 ? 'This text has problems, so nothing is played: what is wrong with it is said under the text.' : 'wrong' in told ? told.wrong : cannotPlay(chosen);

  const play = async () => {
    if (cannot || 'wrong' in told || kept.busy) return;
    const had = keepBeside(tab, { busy: 'Playing them side by side', refused: null, pressed: false });
    // The sandboxes it had are ended first: a daemon has only so many at once.
    endBeside(had.answer);
    keepBeside(tab, { answer: null });
    let answer: SideBySide;
    try {
      answer = await call<SideBySide>('POST', '/sandboxes/beside', asking({ name, project, texts: picked, first: plan.first, params: told.params, told: plan.told, answers: plan.answers, acts: plan.acts }));
    } catch (err) {
      keepBeside(tab, { busy: null, refused: why(err), answered: (besides.value[tab]?.answered ?? 0) + 1 });
      return;
    }
    // The tab was let go of meanwhile: what was made for it is ended.
    if (!besides.value[tab]) return endBeside(answer);
    keepBeside(tab, { busy: null, answer, answered: (besides.value[tab]?.answered ?? 0) + 1 });
  };
  // Pressed in the page's head, it is played, once for each press. A press that cannot be played (one text, or
  // a text that does not check) is answered by what the view says of it, and counted as answered all the same.
  useEffect(() => {
    if (!kept.pressed || !read) return;
    if (graph && !cannot) void play();
    else keepBeside(tab, { pressed: false, answered: (besides.value[tab]?.answered ?? 0) + 1 });
  }, [kept.pressed, read, graph !== null, cannot]);

  const answer = kept.answer;
  const columns = answer?.ok ? (answer.sandboxes ?? []) : [];
  const mayCompare = !!name && covers(me.value?.principal.scopes ?? [], 'blueprints:run');
  const ways = asWays(picked.filter((text) => columns.some((column) => column.called === text.called)), props.isDefault ? props.source : (shipped ?? null));
  return (
    <div class="beside-view" data-busy={kept.busy !== null} data-answered={kept.answered} data-stands={'reading' in stands ? 'reading' : 'one' in stands ? 'one' : 'several'}>
      <section class="beside-choose" aria-label="What is played side by side">
        <h2>Side by side</h2>
        {'reading' in stands ? (
          <p class="beside-reading faint" role="status">
            Reading which texts of this blueprint there are.
          </p>
        ) : 'one' in stands ? (
          <div class="beside-one" role="status">
            <p>{stands.says}</p>
            {stands.seconds.length > 0 && (
              <ul class="beside-seconds">
                {stands.seconds.map((second) => (
                  <li key={second.word}>
                    <button onClick={() => (second.word === 'change' ? props.onText?.() : props.onFork?.())}>{second.called}</button>
                    <span class="faint">{second.then}</span>
                  </li>
                ))}
              </ul>
            )}
            {stands.otherwise && <p class="faint">{stands.otherwise}</p>}
          </div>
        ) : (
          <>
            <fieldset class="beside-texts">
              <legend>The texts</legend>
              {texts.map((text) => (
                <label key={text.word} class="beside-text">
                  <input type="checkbox" checked={chosen.includes(text.word)} onChange={() => keepBeside(tab, { chosen: toggled(chosen, text.word, texts) })} />
                  <span>
                    {text.called} <span class="faint">{text.says}</span>
                    {text.same && <span class="faint">, and the same to the letter as {text.same.join(' and ')}</span>}
                  </span>
                </label>
              ))}
            </fieldset>
            <p class="beside-played faint">{playedSays(plan.acts, plan.answers)}</p>
            <div class="beside-go">
              <button class="primary" disabled={!!cannot || kept.busy !== null} onClick={() => void play()}>
                {answer ? 'Play them again' : 'Play them'}
              </button>
              {mayCompare && columns.length >= 2 && ways.length >= 2 && (
                <button
                  class="beside-compare"
                  onClick={() => name && compareTexts(name, project, ways)}
                  title="Start a comparison on real work with these texts as its ways: each thing goes one of them, and what comes of each is counted side by side"
                >
                  Compare on real work
                </button>
              )}
              {kept.busy && <span class="sandbox-busy">{kept.busy}</span>}
              {cannot && !kept.busy && <span class="faint">{cannot}</span>}
            </div>
          </>
        )}
        {kept.refused && (
          <p class="sandbox-note bad" role="alert">
            {kept.refused}
          </p>
        )}
        {answer && !answer.ok && (
          <div class="sandbox-note bad" role="alert">
            <p>{answer.said}</p>
            <ul>
              {(answer.texts ?? []).map((one) => (
                <li key={one.called}>
                  {one.called}: {one.diagnostics.map((problem) => problem.message).join(' ')}
                </li>
              ))}
            </ul>
          </div>
        )}
        {answer?.ok && answer.beside && allTheSame(answer.beside) && <p class="sandbox-note">Every column ran the same text, to the letter.</p>}
      </section>
      {answer?.ok && answer.beside ? (
        <div class="beside-columns">
          {columns.map((column, at) => (
            <Played
              key={column.sandbox}
              column={column}
              at={at}
              answer={answer}
              text={picked.find((text) => text.called === column.called) ?? null}
              project={project}
              source={(picked.find((text) => text.called === column.called)?.source ?? shipped) || null}
              mayApply={covers(me.value?.principal.scopes ?? [], 'blueprints:run') && 'params' in told}
              onApply={(source) => 'params' in told && props.onApply(source, told.params)}
              onFork={props.onFork}
            />
          ))}
        </div>
      ) : (
        !kept.busy &&
        'several' in stands && (
          <Empty title="Nothing has been played side by side here yet">
            <span>Each text chosen is run in a factory of its own. Nothing in them touches the factory in use.</span>
          </Empty>
        )
      )}
    </div>
  );
}

/** One text as it was played: what it ran, how it stands, its counts in the daemon's order, what can be done with it, and its own sandbox under that. */
function Played(props: {
  column: Column;
  at: number;
  answer: SideBySide;
  text: Text | null;
  project: string | null;
  source: string | null;
  mayApply: boolean;
  onApply: (source: string) => void;
  onFork: (() => void) | null;
}) {
  const { column, at, answer } = props;
  const beside = answer.beside!;
  const stands = outcome(column.shown);
  // Its own text's graph, to be lit by what its own sandbox did.
  const [graph, setGraph] = useState<Graph | null>(null);
  useEffect(() => {
    let here = true;
    if (props.source) void api.check(props.source, props.project).then((checked) => here && setGraph(checked.graph ?? null), () => {});
    return () => void (here = false);
  }, [props.source, props.project]);
  return (
    <article class="beside-column" data-called={column.called} data-sandbox={column.sandbox} aria-label={column.called}>
      <header class="beside-head">
        <h3>{column.called}</h3>
        {ranOf(beside, at) && <span class="beside-ran faint">{ranOf(beside, at)}</span>}
      </header>
      <p class={classes('sandbox-outcome', stands.how)} role="status">
        {stands.says}
      </p>
      {column.refused && <p class="sandbox-note bad">{column.refused}</p>}
      {column.skipped.map((said, index) => (
        <p key={index} class="sandbox-note">
          {said}
        </p>
      ))}
      <div class="beside-presses">
        {props.mayApply && props.source && (
          <button onClick={() => props.source && props.onApply(props.source)} title="Apply this text in the factory, with what it was told here">
            Apply
          </button>
        )}
        {props.text?.word === 'default' && props.onFork && (
          <button onClick={props.onFork} title="Copy the default into the project, where the copy is what runs under the name and is the project's to change">
            Fork
          </button>
        )}
      </div>
      <table class="beside-counts" aria-label={`What was counted of ${column.called}`}>
        <colgroup>
          <col class="beside-what" />
          <col class="beside-figure" />
          <col class="beside-after" />
        </colgroup>
        <tbody>
          {countsOf(beside, at).flatMap((row, index) => [
            ...(row.head
              ? [
                  <tr key={`${index} head`} class="beside-kind">
                    <th scope="colgroup" colSpan={3}>
                      {row.head}
                    </th>
                  </tr>,
                ]
              : []),
            <tr key={index} data-kind={row.kind} data-what={row.what}>
              <td>{row.what}</td>
              <td class="num">{row.figures ? row.figures[0] : '-'}</td>
              <td class="num faint">{row.figures ? row.figures[1] : ''}</td>
            </tr>,
          ])}
          {beside.rows.length === 0 && (
            <tr>
              <td class="faint" colSpan={3}>
                Nothing was counted.
              </td>
            </tr>
          )}
        </tbody>
      </table>
      <details class="beside-happened">
        {/* How it stands is said over its counts: the rest of what its own sandbox came to is here. */}
        <summary>What happened in its sandbox</summary>
        <Happened kept={{ before: null }} shown={column.shown} graph={graph} />
      </details>
      <Foot shown={column.shown} />
    </article>
  );
}
