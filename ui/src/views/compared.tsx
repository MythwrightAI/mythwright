// Comparisons, where the measurements are: what is being compared, each
// way with what it starts and hands, and one table a person can decide
// from: a column for each text of each way with how many things it was
// given, and under it what came of them, what was chosen, what was added
// up (in all, and for each thing) and what else was measured. One is ended
// here, saying which way everything goes from then on; one is started
// from a blueprint's page.
//
// The figures are counts, times and amounts as the daemon measured and
// laid them out. No way is marked, ordered or coloured as the better one.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { api, call } from '../api.ts';
import type { Beside, Comparison, Counts, Described } from '../api.ts';
import { asked, byKind, counted, counting, does, figures, inForce, KINDS, mayRule, readsOf, rolledOf, ruleAsked, ruleWritten, saidOf, stands, thing, unmeasured, unruled } from '../compared.ts';
import type { Written, WrittenRule } from '../compared.ts';
import type { Rule } from '../comparing.ts';
import { coalesce } from '../events.ts';
import { askingKey, askingOf, keepAsking, nextName, way } from '../waysasked.ts';
import type { Asking } from '../waysasked.ts';
import { attempt, drawer, me, toast } from '../store.ts';
import { Modal, useEvents } from '../ui.tsx';
import { ago, count } from '../util.ts';

interface Shown {
  comparison: Comparison;
  beside: Beside;
}

/** Every comparison there is, of some projects where it is of some (`projects`; nothing is everything); nothing at all where there are none. */
export function Comparisons({ projects }: { projects: string[] | null }) {
  const [all, setAll] = useState<Shown[]>([]);
  const reload = useMemo(
    () => () =>
      void api
        // Of one project the daemon is asked for that one's; of several, for all there are, which are few, and those of the others are left out here.
        .comparisons(projects?.length === 1 ? projects[0] : undefined)
        .then((found) => found.filter((one) => !projects || (!!one.project && projects.includes(one.project))))
        .then((found) => Promise.all(found.map((one) => api.comparison(one.name))))
        .then(setAll)
        .catch(() => {}),
    [projects],
  );
  useEffect(reload, [reload]);
  useEvents('system.comparison.*', reload, [reload]);
  // What was measured under each is read again when work finishes, at most every fifteen seconds.
  const later = useMemo(() => coalesce(reload, 15_000), [reload]);
  useEvents(['run.finished', 'agent.turn.finished'], later, [later], later);
  if (all.length === 0) return null;
  // Those that still say how something is started first, and the newest of each first.
  const ordered = [...all].sort((a, b) => Number(inForce(b.comparison)) - Number(inForce(a.comparison)) || b.comparison.started - a.comparison.started);
  return (
    <>
      {ordered.map((one) => (
        <One key={one.comparison.name} shown={one} />
      ))}
    </>
  );
}

function One({ shown }: { shown: Shown }) {
  const { comparison: one, beside } = shown;
  const across = beside.columns;
  const counts = counted(one);
  const [ending, setEnding] = useState(false);
  return (
    <div class="panel compare compared" data-comparison={one.name}>
      <div class="panel-head">
        <h2>{one.name}</h2>
        <span class="muted truncate">
          ways of {one.of}
          {one.project ? ` in ${one.project}` : ''}, {thing(one)} to one way{counts.length > 0 ? ` · counting ${counts.join(', ')}` : ''} · {stands(one)} · started {ago(one.started)}
        </span>
        <span class="grow" />
        {inForce(one) && <button onClick={() => setEnding(true)}>{one.ended === null ? 'End it' : 'Say again which way goes on'}</button>}
      </div>
      <div class="compare-scroll">
        <table>
          <thead>
            <tr>
              <th>Way</th>
              <th class="num">Share</th>
              <th class="num">Went</th>
              <th>What it does</th>
            </tr>
          </thead>
          <tbody>
            {one.ways.map((way) => (
              <tr key={way.name} class="compared-way">
                <td>{way.name}</td>
                <td class="num">{way.share}</td>
                <td class="num">{count(way.went)}</td>
                <td class="compared-does">{does(way)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <Shares one={one} rolled={rolledOf(beside.rolled)} />
        <table class="compared-counts">
          <thead>
            <tr>
              <th />
              {across.map((column) => (
                <th key={`${column.way} ${column.text}`} colSpan={2} class="compare-value">
                  {column.way}
                  {saidOf(column) && <div class="compare-which compared-text">{saidOf(column)}</div>}
                  {readsOf(column).map((reads) => (
                    <div key={reads} class="compare-which compared-reads">
                      {reads}
                    </div>
                  ))}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            <tr class="compared-given">
              <td>Things given</td>
              {across.map((column) => (
                <td key={`${column.way} ${column.text}`} colSpan={2} class="num compare-edge">
                  {column.given === null ? '-' : count(column.given)}
                </td>
              ))}
            </tr>
            {byKind(beside.rows).map(({ kind, rows }) => [
              <tr key={kind} class="compared-kind">
                <th>{KINDS[kind].head}</th>
                {across.map((column) =>
                  KINDS[kind].figures.map((figure, index) => (
                    <th key={`${column.way} ${column.text} ${figure}`} class={index === 0 ? 'num compare-edge' : 'num'}>
                      {figure}
                    </th>
                  )),
                )}
              </tr>,
              ...rows.map((row) => (
                <tr key={`${kind} ${row.what}`} data-kind={kind}>
                  <td>{row.what}</td>
                  {row.by.map((cell, index) =>
                    cell ? (
                      figures(row, cell).map((figure, at) => (
                        <td key={`${index} ${at}`} class={at === 0 ? 'num compare-edge' : 'num'}>
                          {figure}
                        </td>
                      ))
                    ) : (
                      <td key={index} colSpan={2} class="num faint compare-edge">
                        {unmeasured(beside.rows, row, index)}
                      </td>
                    ),
                  )}
                </tr>
              )),
            ])}
            {beside.rows.length === 0 && (
              <tr>
                <td class="faint" colSpan={1 + across.length * 2}>
                  Nothing has been measured under it yet.
                </td>
              </tr>
            )}
          </tbody>
        </table>
        {beside.rows.length > 0 && (
          <p class="muted compared-legend">
            After is how long after a thing was given its way, on average. A thing is what was added up, for each thing the way was given.
            {beside.rows.some((row) => row.kind === 'accepted') &&
              ' Of those given is how many of the things a way was given were accepted, as what it counts says; each accepted is what the way spent in all, its work that was not accepted with it, over how many were.'}
          </p>
        )}
      </div>
      {ending && <End one={one} onClose={() => setEnding(false)} />}
    </div>
  );
}

/** Words of the daemon's, with what it writes between backticks as code. */
function Coded({ text }: { text: string }) {
  return <>{text.split('`').map((part, index) => (index % 2 === 1 ? <code key={index}>{part}</code> : part))}</>;
}

/** Its shares as they are, the rule they change by with what it has read, and each time they were said again with why: the daemon's sentences, written down; and for whoever the daemon would let, the rule given, changed or taken away. */
function Shares({ one, rolled }: { one: Comparison; rolled: ReturnType<typeof rolledOf> }) {
  const [ruling, setRuling] = useState(false);
  const may = mayRule(me.value?.principal.scopes ?? [], one);
  if (!rolled && !may) return null;
  return (
    <div class="compared-rolled">
      {may && (
        <div class="row compared-ruling">
          <button type="button" onClick={() => setRuling(true)}>
            {one.rule ? 'Change its rule…' : 'Give it a rule…'}
          </button>
        </div>
      )}
      {ruling && <RuleDialog one={one} onClose={() => setRuling(false)} />}
      {rolled && <p class="compared-parts">Shared out {rolled.parts}.</p>}
      {rolled?.rule && (
        <p class="compared-rule">
          Its rule: <Coded text={rolled.rule} /> <span class="muted">{rolled.read}</span>
        </p>
      )}
      {rolled && rolled.changes.length > 0 && (
        <ol class="compared-changes">
          {rolled.changes.map((change) => (
            <li key={change.at + change.by + change.why}>
              <span class="muted">
                {ago(change.at)} · {change.by}
              </span>{' '}
              <Coded text={change.why} />
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

/** The parts of a rule, as they are written: the way that grows and the one it is held against among `ways`, what it reads, and how far it goes up and back. Each is its label, its box and what it means. */
export function RuleFields({ rule, ways, onChange }: { rule: WrittenRule; ways: string[]; onChange: (rule: WrittenRule) => void }) {
  const set = (part: keyof WrittenRule, value: string | boolean) => onChange({ ...rule, [part]: value });
  const way = (part: 'grows' | 'against', label: string, said: string) => (
    <label class="comparing-field">
      <span class="comparing-label">{label}</span>
      <select aria-label={label} value={rule[part]} onChange={(event) => set(part, event.currentTarget.value)}>
        {ways.map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </select>
      <span class="comparing-said">{said}</span>
    </label>
  );
  const number = (part: 'over' | 'step' | 'most' | 'least', label: string, said: string) => (
    <label class="comparing-field">
      <span class="comparing-label">{label}</span>
      <input aria-label={label} inputMode="numeric" value={rule[part]} onInput={(event) => set(part, event.currentTarget.value)} />
      <span class="comparing-said">{said}</span>
    </label>
  );
  return (
    <div class="comparing-rule">
      <div class="comparing-pair">
        {way('grows', 'Grows', 'The way given more of the things.')}
        {way('against', 'Held against', 'The way it is held against.')}
      </div>
      <div class="comparing-pair">
        <label class="comparing-field">
          <span class="comparing-label">By the count</span>
          <input aria-label="By the count" value={rule.count} placeholder="verdict" onInput={(event) => set('count', event.currentTarget.value)} />
          <span class="comparing-said">One the comparison counts.</span>
        </label>
        <label class="comparing-field">
          <span class="comparing-label">Coming to</span>
          <input aria-label="Coming to" value={rule.came} placeholder="failed" onInput={(event) => set('came', event.currentTarget.value)} />
          <span class="comparing-said">What it came to that is held.</span>
        </label>
      </div>
      <label class="row comparing-check">
        <input type="checkbox" aria-label="At least as often" checked={rule.atLeast} onChange={(event) => set('atLeast', event.currentTarget.checked)} />
        <span>At least as often as the other, for what is wanted; else no more often.</span>
      </label>
      <div class="comparing-pair">
        {number('over', 'Read over', 'Things of each before anything is decided.')}
        {number('step', 'Step', 'Of a hundred, each time it holds.')}
        {number('most', 'Most', 'Of a hundred it grows to.')}
        {number('least', 'Least', 'Of a hundred it goes back to a step at a time where it does not hold; empty, it stays.')}
      </div>
    </div>
  );
}

/** Giving a comparison a rule, changing it or taking it away: asked of the daemon, which says what is wrong in its sentence. */
function RuleDialog({ one, onClose }: { one: Comparison; onClose: () => void }) {
  const ways = one.ways.map((way) => way.name);
  const [rule, setRule] = useState<WrittenRule>(() => (one.rule ? ruleWritten(one.rule) : unruled(ways)));
  const [error, setError] = useState('');
  const send = async (told: Rule | null) => {
    try {
      await call<Comparison>('PUT', `/comparisons/${encodeURIComponent(one.name)}/rule`, { rule: told });
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };
  return (
    <Modal onClose={onClose}>
      <form
        class="stack comparing"
        onSubmit={(event) => {
          event.preventDefault();
          const asked = ruleAsked(rule, ways);
          if ('wrong' in asked) return setError(asked.wrong);
          void send(asked.rule);
        }}
      >
        <h2>{one.rule ? `Change the rule of ${one.name}` : `Give ${one.name} a rule`}</h2>
        <p class="muted">Its shares are said again by the rule as what it counts comes to, each time with why. What it read so far is read afresh from now.</p>
        {/* Over the fields, so that it is in sight however far down they were written. */}
        {error && <p class="error-text comparing-wrong">{error}</p>}
        <RuleFields rule={rule} ways={ways} onChange={setRule} />
        <div class="row modal-foot">
          {one.rule && (
            <button type="button" class="danger" onClick={() => void send(null)}>
              Take the rule away
            </button>
          )}
          <span class="grow" />
          <button type="button" onClick={onClose}>
            Not now
          </button>
          <button class="primary" type="submit">
            {one.rule ? 'Change it' : 'Give it'}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** Ending one: which way everything goes from now on. */
function End({ one, onClose }: { one: Comparison; onClose: () => void }) {
  const [goes, setGoes] = useState(one.goes ?? '');
  const end = async () => {
    const ended = await attempt(() => api.endComparison(one.name, goes || undefined));
    if (!ended) return;
    toast(`${one.name}: ${stands(ended)}.`);
    onClose();
  };
  return (
    <Modal onClose={onClose}>
      <form
        class="stack"
        onSubmit={(event) => {
          event.preventDefault();
          void end();
        }}
      >
        <h2>End {one.name}</h2>
        <p class="muted">
          From now on nothing more is counted under it. Say which way {one.of} is started from here: as it is written, or one of the ways for everything. That stands until it is said
          again here.
        </p>
        <label class="stack" style="gap:4px">
          <span class="muted">From now on</span>
          <select aria-label="From now on" value={goes} onChange={(event) => setGoes(event.currentTarget.value)}>
            <option value="">as it is written</option>
            {one.ways.map((way) => (
              <option key={way.name} value={way.name}>
                {way.name}: {does(way)}
              </option>
            ))}
          </select>
        </label>
        <div class="row modal-foot">
          <button type="button" onClick={onClose}>
            Not now
          </button>
          <button class="primary" type="submit">
            {one.ended === null ? 'End it' : 'Say so'}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/**
 * Opens the drawer on texts of a blueprint as the ways it is compared by
 * on real work: each a way of its own, called by the word it is known
 * by and starting its text as it was sent. What was written there before
 * is replaced by them; the rest of what is asked is as it was.
 */
export function compareTexts(of: string, project: string | null, texts: { word: string; called: string; source: string }[]): void {
  const key = askingKey(of, project);
  keepAsking(key, { ...askingOf(key), ways: texts.map((text) => ({ ...way(text.word), sent: { called: text.called, source: text.source } })) });
  // Drawn anew, so that what is asked is these and not what an open drawer held.
  sentTimes += 1;
  const opened = sentTimes;
  drawer.value = {
    title: `Compare ways of ${of}`,
    body: () => <Compare key={`${key}\n${opened}`} of={of} project={project} onClose={() => (drawer.value = null)} />,
  };
}

/** How many times texts were sent to be compared: each is drawn anew. */
let sentTimes = 0;

/**
 * Opens the drawer on the ways a blueprint's starting is to be compared by.
 * What was written there before and not started is there again.
 */
export function compareWays(of: string, project: string | null): void {
  drawer.value = {
    title: `Compare ways of ${of}`,
    body: () => <Compare key={askingKey(of, project)} of={of} project={project} onClose={() => (drawer.value = null)} />,
  };
}

/**
 * Starting one, from the page of the blueprint whose starting is compared.
 * It is asked in the drawer, so that the blueprint's text stays readable
 * beside it; each thing asked has its label above it and what it means
 * under it, and each way is a block of its own.
 */
export function Compare(props: { of: string; project: string | null; onClose: () => void }) {
  const key = askingKey(props.of, props.project);
  const [form, setForm] = useState<Asking>(() => askingOf(key));
  const [others, setOthers] = useState<Described[]>([]);
  const [error, setError] = useState('');
  useEffect(() => {
    void api
      .blueprints(props.project)
      .then(setOthers)
      .catch(() => {});
  }, [props.project]);
  // Kept as it is written: the drawer is put away when another tab is shown.
  const write = (next: Asking) => {
    keepAsking(key, next);
    setForm(next);
  };
  const { name, per, also, counts, events, ways, rule } = form;
  // What the blueprint is handed: one of those says which thing a start is for.
  const params = others.find((one) => one.name === props.of)?.params ?? [];
  const change = (index: number, field: keyof Written, value: string) => write({ ...form, ways: ways.map((one, at) => (at === index ? { ...one, [field]: value } : one)) });
  const start = async () => {
    const told = asked(ways);
    if ('wrong' in told) return setError(told.wrong);
    if (!name.trim()) return setError('Give the comparison a name: a word, such as models. What is measured under it is labelled with it.');
    setError('');
    const listed = also
      .split(/[\s,]+/)
      .map((one) => one.trim())
      .filter(Boolean);
    let counted: Counts | undefined = counts === 'cards' ? 'cards' : undefined;
    if (counts === 'said') {
      const said = counting(events);
      if ('wrong' in said) return setError(said.wrong);
      counted = said.counts;
    }
    let ruled: Rule | undefined;
    if (rule) {
      const said = ruleAsked(rule, told.ways.map((one) => one.name));
      if ('wrong' in said) return setError(said.wrong);
      ruled = said.rule;
    }
    const comparing = { name: name.trim(), project: props.project, of: props.of, per: per || undefined, also: listed, counts: counted, ways: told.ways, ...(ruled ? { rule: ruled } : {}) };
    const made = await attempt(() => api.compare(comparing));
    if (!made) return;
    keepAsking(key, null);
    toast(`Comparing ways of ${props.of} as ${made.name}. What is measured under each is in Measurements.`);
    props.onClose();
  };
  return (
    <form
      class="comparing"
      onSubmit={(event) => {
        event.preventDefault();
        void start();
      }}
    >
      <p class="muted comparing-about">
        From when this is started, wherever a step{props.project ? ` in ${props.project}` : ''} calls or spawns {props.of}, it is started one of these ways. Each thing goes one way and
        keeps to it, and everything under a way is measured with it. Nothing is written in any blueprint, and what is counted is shown side by side: which is better is yours to say.
      </p>
      <div class="comparing-pair">
        <label class="comparing-field">
          <span class="comparing-label">Called</span>
          <input aria-label="Called" value={name} placeholder="models" onInput={(event) => write({ ...form, name: event.currentTarget.value })} />
          <span class="comparing-said">What is measured under it is labelled with this.</span>
        </label>
        <label class="comparing-field">
          <span class="comparing-label">A thing is</span>
          <select
            aria-label="A thing is"
            value={per}
            onChange={(event) => {
              const said = event.currentTarget.value;
              // A thing that is an item is a card where the work is a board's: what comes of it is counted unless somebody says otherwise.
              write({ ...form, per: said, counts: counts === 'said' ? counts : said === 'item' ? 'cards' : '' });
            }}
          >
            <option value="">each place a step starts it</option>
            {params.map((param) => (
              <option key={param.name} value={param.name}>
                what {param.name} says
              </option>
            ))}
          </select>
          <span class="comparing-said">Each thing goes one way and keeps to it.</span>
        </label>
      </div>
      <label class="comparing-field">
        <span class="comparing-label">Also counted with a thing's way</span>
        <input aria-label="Also counted" value={also} placeholder="std/review" onInput={(event) => write({ ...form, also: event.currentTarget.value })} />
        <span class="comparing-said">Blueprints started for the same things.</span>
      </label>
      <label class="comparing-field">
        <span class="comparing-label">Counted of each thing</span>
        <select aria-label="Counted of each thing" value={counts} onChange={(event) => write({ ...form, counts: event.currentTarget.value as Asking['counts'] })}>
          <option value="">only what is measured under it</option>
          <option value="cards">what comes of a card: its moves, reviews, questions, work left</option>
          <option value="said">events said here</option>
        </select>
      </label>
      {counts === 'said' && (
        <label class="comparing-field">
          <span class="comparing-label">Events</span>
          <textarea aria-label="Events counted" rows={2} value={events} placeholder="made=shop.made:id:tone" onInput={(event) => write({ ...form, events: event.currentTarget.value })} />
          <span class="comparing-said">A line each: name=event:field of the thing:fields it came to.</span>
        </label>
      )}
      {ways.map((one, index) => (
        <div class="comparing-way" key={index}>
          {/* What takes the way out is in the block's head, where no field is. */}
          <div class="comparing-way-head">
            <span class="comparing-way-called">Way {index + 1}</span>
            {ways.length > 1 && (
              <button type="button" class="ghost icon" aria-label={`Remove way ${index + 1}`} title="Remove this way" onClick={() => write({ ...form, ways: ways.filter((_, at) => at !== index) })}>
                ✕
              </button>
            )}
          </div>
          <div class="comparing-way-line">
            <label class="comparing-field comparing-way-name">
              <span class="comparing-label">Name</span>
              <input aria-label={`Way ${index + 1}`} value={one.name} onInput={(event) => change(index, 'name', event.currentTarget.value)} />
            </label>
            <label class="comparing-field comparing-way-share">
              <span class="comparing-label">Share</span>
              <input aria-label={`Share of way ${index + 1}`} value={one.share} inputMode="numeric" onInput={(event) => change(index, 'share', event.currentTarget.value)} />
            </label>
            {one.sent ? (
              <div class="comparing-field comparing-way-starts comparing-way-sent">
                <span class="comparing-label">Starts</span>
                <span class="comparing-sent" data-sent={one.sent.called}>
                  {props.of} as it was played side by side: {one.sent.called}
                </span>
                <button type="button" class="ghost" onClick={() => write({ ...form, ways: ways.map((other, at) => (at === index ? { ...other, sent: undefined } : other)) })}>
                  Start it as it is written instead
                </button>
              </div>
            ) : (
            <label class="comparing-field comparing-way-starts">
              <span class="comparing-label">Starts</span>
              <select aria-label={`What way ${index + 1} starts`} value={one.blueprint} onChange={(event) => change(index, 'blueprint', event.currentTarget.value)}>
                <option value="">{props.of}, as it is written</option>
                {others
                  .filter((other) => other.name !== props.of)
                  .map((other) => (
                    <option key={other.name} value={other.name}>
                      {other.name}
                    </option>
                  ))}
              </select>
            </label>
            )}
          </div>
          <label class="comparing-field">
            <span class="comparing-label">Hands</span>
            <textarea aria-label={`What way ${index + 1} hands`} rows={2} value={one.hands} placeholder="param=value" onInput={(event) => change(index, 'hands', event.currentTarget.value)} />
            <span class="comparing-said">What it is handed, a line each.</span>
          </label>
          <label class="comparing-field">
            <span class="comparing-label">Reads</span>
            <textarea
              aria-label={`What way ${index + 1} reads`}
              rows={2}
              value={one.reads ?? ''}
              placeholder="std/prompts/review-seat.md=prompts/strict.md"
              onInput={(event) => change(index, 'reads', event.currentTarget.value)}
            />
            <span class="comparing-said">Another prompt read where one is linked, a line each: prompt=other prompt.</span>
          </label>
        </div>
      ))}
      <div class="comparing-more">
        <button type="button" onClick={() => write({ ...form, ways: [...ways, way(nextName(ways))] })}>
          Another way
        </button>
      </div>
      <label class="row comparing-check">
        <input
          type="checkbox"
          aria-label="Its shares grow by a rule"
          checked={rule !== null}
          onChange={(event) => write({ ...form, rule: event.currentTarget.checked ? unruled(ways.map((one) => one.name)) : null })}
        />
        <span>Its shares grow by a rule: one way is given more of the things while what is counted of them holds against another's.</span>
      </label>
      {rule && <RuleFields rule={rule} ways={ways.map((one) => one.name.trim()).filter(Boolean)} onChange={(next) => write({ ...form, rule: next })} />}
      {error && <p class="error-text comparing-wrong">{error}</p>}
      <div class="comparing-foot">
        <button type="button" onClick={props.onClose}>
          Not now
        </button>
        <button class="primary" type="submit">
          Start comparing
        </button>
      </div>
    </form>
  );
}
