// What is done to a sandbox, in the order somebody does it.
//
// First what it is run with: what goes first (other blueprints applied
// before this one, offered from what the daemon says a start cannot be
// had without) and what the text is told, asked as applying it asks.
// Then, once it is up: an event said, something done to its tables by
// hand, the clock moved on, and each stand-in that waits answered. Every
// form is made from what the daemon describes (an event's topic and what
// it carries, an action's arguments), and each thing asked is its label,
// its box under that and what it means under the box.
//
// Nothing is worked out here but what a box holds: each act is plain
// values (`sandbox.ts`), sent one at a time and kept with the tab
// (`sandboxed.ts`).

import { useEffect, useState } from 'preact/hooks';
import type { ActionSpec, EventSpec, Fields, Graph } from '../api.ts';
import { written, startOf } from '../applyasked.ts';
import { UNASKED, actSays, answerRead, asksOf, carries, nth, valuesOf } from '../sandbox.ts';
import type { Act, Asked, Asking } from '../sandbox.ts';
import { actOn, dropAct } from '../sandboxed.ts';
import { keepSandbox } from '../sandboxkept.ts';
import type { Kept } from '../sandboxkept.ts';
import { blueprintLanguage, callable } from '../store.ts';
import { show } from '../util.ts';
import { ParamField } from './applying.tsx';

export interface ActsProps {
  tab: string;
  kept: Kept;
  project: string | null;
  graph: Graph | null;
  /** What the blueprint was last applied with in the factory, where it runs there: what its boxes start as. */
  known: Fields;
  /** Why it cannot be run as it stands, where it cannot: its text has problems. */
  cannot: string | null;
  /** Runs it, with what is written here. */
  onRun: () => void;
}

/** What is written in a param's box: what somebody wrote, else what the param starts as. */
export function toldOf(kept: Kept, graph: Graph | null, known: Fields): Record<string, string> {
  return Object.fromEntries((graph?.params ?? []).map((param) => [param.name, kept.plan.told[param.name] ?? written(startOf(param, known))]));
}

export function Acts(props: ActsProps) {
  const { tab, kept, graph } = props;
  const [described, setDescribed] = useState<{ actions: ActionSpec[]; events: EventSpec[] }>({ actions: [], events: [] });
  useEffect(() => {
    void blueprintLanguage().then((language) => language && setDescribed({ actions: language.actions, events: language.events ?? [] }));
  }, []);
  const busy = kept.busy !== null;
  const up = kept.id !== null && kept.shown !== null && !kept.shown.over;
  const written = toldOf(kept, graph, props.known);
  // Which project it is for is not asked: the daemon tells it the sandbox's one.
  const told = (graph?.params ?? []).filter((param) => !UNASKED.includes(param.name));
  const plan = (change: Partial<Kept['plan']>) => keepSandbox(tab, { plan: { ...kept.plan, ...change } });
  const act = (one: Act) => actOn(tab, one);

  return (
    <div class="sandbox-acts-list">
      <section class="sandbox-form" aria-label="What it is run with">
        <h3>Run it</h3>
        <First first={kept.plan.first} offered={(callable.value[props.project ?? ''] ?? []).map((one) => one.name)} onChange={(first) => plan({ first })} />
        {told.length > 0 && <p class="comparing-label sandbox-told">What it is told</p>}
        {told.length < (graph?.params ?? []).length && <p class="comparing-said sandbox-told">Which project it is for is not asked: a sandbox has one, made up for it, and the text is told that one.</p>}
        {told.map((param) => (
          <ParamField key={param.name} param={param} now={written[param.name] ?? ''} known={props.known} onSet={(text) => plan({ told: { ...kept.plan.told, [param.name]: text } })} />
        ))}
        <label class="comparing-field">
          <span class="comparing-label">What is stood in</span>
          <select value={kept.plan.answers} onChange={(chosen) => plan({ answers: chosen.currentTarget.value === 'default' ? 'default' : 'asked' })}>
            <option value="asked">waits to be answered here</option>
            <option value="default">answers at once, as usual</option>
          </select>
          <span class="comparing-said">Whatever reaches outside the factory's tables (a command, an agent, a file) is stood in. It waits for you to say what it answers, or answers at once what a stand-in for it usually answers.</span>
        </label>
        {props.cannot && <p class="error-text comparing-wrong">{props.cannot}</p>}
        <div class="sandbox-go">
          <button class="primary" disabled={busy || !!props.cannot} onClick={props.onRun} title="Run the text as it stands on this page in a sandbox of its own, with what was done to the last one done again">
            {kept.shown || kept.gone ? 'Run again' : 'Run'}
          </button>
          {busy && (
            <span class="sandbox-busy" role="status">
              {kept.busy}…
            </span>
          )}
        </div>
      </section>

      {up && kept.shown!.waiting.map((asking) => <Answer key={asking.id} asking={asking} busy={busy} onAnswer={act} />)}
      {up && kept.shown!.waiting.length > 0 && (
        <div class="sandbox-go">
          <button disabled={busy} onClick={() => void act({ kind: 'answers', answers: 'default' })} title="Answer this one, and every stand-in asked from here on, what a stand-in for its action usually answers">
            Answer every one as usual from here on
          </button>
        </div>
      )}

      {up && (
        <>
          <Say events={described.events} heard={heardBy(graph)} busy={busy} onSay={act} />
          <Do does={kept.shown!.does} actions={described.actions} busy={busy} onDo={act} />
          <Advance busy={busy} onAdvance={act} />
        </>
      )}

      {kept.plan.acts.length > 0 && (
        <section class="sandbox-form" aria-label="What was done to it">
          <h3>What was done to it</h3>
          <p class="comparing-said">In this order, and done again in it when the text is run again.</p>
          <ol class="sandbox-done">
            {kept.plan.acts.map((one, index) => (
              <li key={index}>
                <span>{actSays(one)}</span>
                <button class="ghost" disabled={busy} title="Leave this out the next time it is run" aria-label={`Leave out: ${actSays(one)}`} onClick={() => dropAct(tab, index)}>
                  ×
                </button>
              </li>
            ))}
          </ol>
        </section>
      )}
    </div>
  );
}

/** The topics the text's own handlers hear, as its graph says them. */
function heardBy(graph: Graph | null): string[] {
  const topics = (graph?.nodes ?? []).filter((node) => node.kind === 'trigger' && node.target.startsWith('event ')).map((node) => node.target.slice('event '.length));
  return [...new Set(topics)];
}

/** What goes first: blueprints applied before this one, each taken away by its mark, and one added by name. */
function First({ first, offered, onChange }: { first: string[]; offered: string[]; onChange: (first: string[]) => void }) {
  const [adding, setAdding] = useState('');
  const add = () => {
    const name = adding.trim();
    if (name && !first.includes(name)) onChange([...first, name]);
    setAdding('');
  };
  return (
    <div class="comparing-field">
      <span class="comparing-label">What goes first</span>
      {first.length > 0 && (
        <ul class="sandbox-first">
          {first.map((name) => (
            <li key={name}>
              <code>{name}</code>
              <button class="ghost" aria-label={`Do not apply ${name} first`} title="Do not apply this first" onClick={() => onChange(first.filter((one) => one !== name))}>
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
      <span class="sandbox-add">
        <input
          aria-label="A blueprint to apply first"
          list="sandbox-callable"
          placeholder="std/board"
          value={adding}
          onInput={(event) => setAdding(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key !== 'Enter') return;
            event.preventDefault();
            add();
          }}
        />
        <button type="button" onClick={add} disabled={adding.trim() === ''}>
          Add
        </button>
      </span>
      <datalist id="sandbox-callable">
        {offered.map((name) => (
          <option key={name} value={name} />
        ))}
      </datalist>
      <span class="comparing-said">Other blueprints applied in the sandbox before this one, in this order: what it needs to be there, such as a board before a fleet. A sandbox starts with nothing in it.</span>
    </div>
  );
}

/** The boxes of a form made from what something says it takes, and what is written in them. */
function Boxes({ asks, written: values, onSet }: { asks: Asked[]; written: Record<string, string>; onSet: (name: string, text: string) => void }) {
  return (
    <>
      {asks.map((ask) => (
        <label key={ask.name} class="comparing-field">
          <span class="comparing-label">
            {ask.name}
            {ask.needed && <span class="faint"> (needed)</span>}
          </span>
          {ask.box === 'bool' ? (
            <select value={values[ask.name] ?? ''} onChange={(event) => onSet(ask.name, event.currentTarget.value)}>
              <option value="">not said</option>
              <option value="true">true</option>
              <option value="false">false</option>
            </select>
          ) : ask.box === 'json' ? (
            <textarea rows={2} spellcheck={false} placeholder="words, or JSON" value={values[ask.name] ?? ''} onInput={(event) => onSet(ask.name, event.currentTarget.value)} />
          ) : (
            <input inputMode={ask.box === 'number' ? 'numeric' : undefined} value={values[ask.name] ?? ''} onInput={(event) => onSet(ask.name, event.currentTarget.value)} />
          )}
          {ask.help && <span class="comparing-said">{ask.help}</span>}
        </label>
      ))}
    </>
  );
}

/** An event said to the sandbox: its topic, and a box for each thing it says it carries. */
function Say({ events, heard, busy, onSay }: { events: EventSpec[]; heard: string[]; busy: boolean; onSay: (act: Act) => Promise<boolean> }) {
  const [topic, setTopic] = useState(heard[0] ?? '');
  const [values, setValues] = useState<Record<string, string>>({});
  const [more, setMore] = useState('');
  const [wrong, setWrong] = useState('');
  const event = events.find((one) => one.topic === topic.trim());
  const asks = carries(event?.fields ?? []);
  const say = async (submitted: Event) => {
    submitted.preventDefault();
    const told = valuesOf(asks, values);
    if ('wrong' in told) return setWrong(told.wrong);
    const beside = answerRead(more);
    if ('wrong' in beside) return setWrong(beside.wrong);
    if (beside.value === null || typeof beside.value !== 'object' || Array.isArray(beside.value)) return setWrong('What else it carries is a mapping: `{ "version": 2 }`.');
    setWrong('');
    if (await onSay({ kind: 'say', topic: topic.trim(), data: { ...beside.value, ...told.values } })) setValues({});
  };
  return (
    <form class="sandbox-form" aria-label="Say an event" onSubmit={(submitted) => void say(submitted)}>
      <h3>Say an event</h3>
      <label class="comparing-field">
        <span class="comparing-label">Topic</span>
        <input required list="sandbox-topics" placeholder="board.item.created" value={topic} onInput={(typed) => setTopic(typed.currentTarget.value)} />
        <datalist id="sandbox-topics">
          {heard.map((one) => (
            <option key={`heard:${one}`} value={one} label="this text hears it" />
          ))}
          {events.map((one) => (
            <option key={one.topic} value={one.topic} label={one.summary} />
          ))}
        </datalist>
        <span class="comparing-said">{event ? event.summary : 'What the event is called. One this program describes is offered with what it carries; one of your own is said as it is written.'}</span>
      </label>
      <Boxes asks={asks} written={values} onSet={(name, text) => setValues({ ...values, [name]: text })} />
      <label class="comparing-field">
        <span class="comparing-label">{asks.length > 0 ? 'What else it carries' : 'What it carries'}</span>
        <textarea rows={2} spellcheck={false} placeholder='{ "version": 2 }' value={more} onInput={(typed) => setMore(typed.currentTarget.value)} />
        <span class="comparing-said">A mapping, written as JSON. Left empty, it carries nothing more.</span>
      </label>
      {wrong && <p class="error-text comparing-wrong">{wrong}</p>}
      <div class="sandbox-go">
        <button type="submit" disabled={busy || topic.trim() === ''}>
          Say it
        </button>
      </div>
    </form>
  );
}

/** What each thing done by hand is, in words: the sandbox's tables are changed as the made-up person who applied the text. */
const BY_HAND: Record<string, string> = {
  'board.add': 'Add a card',
  'board.move': 'Move a card',
  'board.update': 'Change a card',
  'record.add': 'Add a row',
  'record.update': 'Change a row',
  'record.remove': 'Remove a row',
  'record.comment': 'Say something on a record',
  'channel.post': 'Say something in a channel',
};

/** Something done to the sandbox's tables by hand: which, and a box for each thing its action says it takes. */
function Do({ does, actions, busy, onDo }: { does: string[]; actions: ActionSpec[]; busy: boolean; onDo: (act: Act) => Promise<boolean> }) {
  const [action, setAction] = useState(does[0] ?? '');
  const [values, setValues] = useState<Record<string, string>>({});
  const [wrong, setWrong] = useState('');
  const spec = actions.find((one) => one.name === action);
  const asks = spec ? asksOf(spec) : [];
  const done = async (submitted: Event) => {
    submitted.preventDefault();
    const told = valuesOf(asks, values);
    if ('wrong' in told) return setWrong(told.wrong);
    setWrong('');
    if (await onDo({ kind: 'do', action, with: told.values })) setValues({});
  };
  return (
    <form class="sandbox-form" aria-label="Do something by hand" onSubmit={(submitted) => void done(submitted)}>
      <h3>Do something by hand</h3>
      <label class="comparing-field">
        <span class="comparing-label">What</span>
        <select
          value={action}
          onChange={(chosen) => {
            setAction(chosen.currentTarget.value);
            setValues({});
            setWrong('');
          }}
        >
          {does.map((one) => (
            <option key={one} value={one}>
              {BY_HAND[one] ?? one}
            </option>
          ))}
        </select>
        <span class="comparing-said">{spec ? `${spec.summary} Done as the made-up person who applied the text, to the sandbox's own tables.` : 'Done to the sandbox\'s own tables, as the made-up person who applied the text.'}</span>
      </label>
      <Boxes asks={asks} written={values} onSet={(name, text) => setValues({ ...values, [name]: text })} />
      {wrong && <p class="error-text comparing-wrong">{wrong}</p>}
      <div class="sandbox-go">
        <button type="submit" disabled={busy || action === ''}>
          Do it
        </button>
      </div>
    </form>
  );
}

const A_WHILE = ['1m', '5m', '1h', '1d'];

/** The sandbox's clock moved on: nothing else moves it. */
function Advance({ busy, onAdvance }: { busy: boolean; onAdvance: (act: Act) => Promise<boolean> }) {
  const [by, setBy] = useState('5m');
  return (
    <form
      class="sandbox-form"
      aria-label="Move the clock on"
      onSubmit={(submitted) => {
        submitted.preventDefault();
        void onAdvance({ kind: 'advance', by: by.trim() });
      }}
    >
      <h3>Move the clock on</h3>
      <div class="comparing-field">
        <span class="comparing-label">By how long</span>
        <span class="sandbox-add">
          <input required aria-label="How long" value={by} onInput={(typed) => setBy(typed.currentTarget.value)} />
          <span class="pills" role="group" aria-label="A while">
            {A_WHILE.map((one) => (
              <button type="button" key={one} aria-pressed={by === one} onClick={() => setBy(one)}>
                {one}
              </button>
            ))}
          </span>
        </span>
        <span class="comparing-said">A step that waits goes on, a time that was given runs out, and what is done every so often is done. The sandbox's clock stands still until it is moved.</span>
      </div>
      <div class="sandbox-go">
        <button type="submit" disabled={busy || by.trim() === ''}>
          Move it on
        </button>
      </div>
    </form>
  );
}

/** A stand-in that waits: what it was asked, in words, and the three ways it is answered. */
function Answer({ asking, busy, onAnswer }: { asking: Asking; busy: boolean; onAnswer: (act: Act) => Promise<boolean> }) {
  const [how, setHow] = useState<'usual' | 'fails' | 'gives'>('usual');
  const [gives, setGives] = useState('');
  const [why, setWhy] = useState('');
  const [wrong, setWrong] = useState('');
  const asked = asking.asked !== null && typeof asking.asked === 'object' && !Array.isArray(asking.asked) ? Object.entries(asking.asked) : [];
  const answer = async (submitted: Event) => {
    submitted.preventDefault();
    if (how === 'gives') {
      const read = answerRead(gives);
      if ('wrong' in read) return setWrong(read.wrong);
      setWrong('');
      await onAnswer({ kind: 'answer', asked: asking.id, how, gives: read.value });
    } else {
      setWrong('');
      await onAnswer({ kind: 'answer', asked: asking.id, how, why });
    }
  };
  const group = `sandbox-answer-${asking.id}`;
  return (
    <form class="sandbox-form sandbox-waits" aria-label={`Answer the ${nth(asking.id)} stand-in`} onSubmit={(submitted) => void answer(submitted)}>
      <h3>A stand-in waits to be answered</h3>
      <p class="sandbox-asked">
        The step <code>{asking.step}</code> of {asking.blueprint} asked <code>{asking.action}</code>, which was stood in. {asking.summary}
      </p>
      {asked.length > 0 && (
        <dl class="sandbox-handed">
          {asked.map(([name, value]) => (
            <div key={name}>
              <dt>{name}</dt>
              <dd>{typeof value === 'string' ? value : show(value)}</dd>
            </div>
          ))}
        </dl>
      )}
      <div class="comparing-field" role="radiogroup" aria-label="How it answers">
        <span class="comparing-label">How it answers</span>
        <label class="sandbox-choice">
          <input type="radio" name={group} checked={how === 'usual'} onChange={() => setHow('usual')} />
          <span>
            What a stand-in for it usually answers: <code>{show(asking.usual)}</code>
          </span>
        </label>
        <label class="sandbox-choice">
          <input type="radio" name={group} checked={how === 'fails'} onChange={() => setHow('fails')} />
          <span>It fails</span>
        </label>
        <label class="sandbox-choice">
          <input type="radio" name={group} checked={how === 'gives'} onChange={() => setHow('gives')} />
          <span>An answer written out</span>
        </label>
      </div>
      {how === 'fails' && (
        <label class="comparing-field">
          <span class="comparing-label">Why it fails</span>
          <input value={why} placeholder="It was told to fail." onInput={(typed) => setWhy(typed.currentTarget.value)} />
          <span class="comparing-said">The words its step fails with.</span>
        </label>
      )}
      {how === 'gives' && (
        <label class="comparing-field">
          <span class="comparing-label">What it gives</span>
          <textarea rows={3} spellcheck={false} placeholder='{ "reply": "left" }' value={gives} onInput={(typed) => setGives(typed.currentTarget.value)} />
          <span class="comparing-said">Written as JSON, and read before it is sent. It is what the step is handed back.</span>
        </label>
      )}
      {wrong && <p class="error-text comparing-wrong">{wrong}</p>}
      <div class="sandbox-go">
        <button class="primary" type="submit" disabled={busy}>
          Answer it
        </button>
      </div>
    </form>
  );
}
