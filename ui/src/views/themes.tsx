// The themes in Settings: the ones that ship, those offered to everybody,
// somebody's own, and the making of one.
//
// A theme of somebody's own is what it is called and eight colours. The
// rest of its list is worked out from those (`themes.ts`), so a whole
// theme comes of a few choices, and it is drawn beside the form as it is
// chosen, each in the client's own picker with the colour as it was beside
// it (`colourpick.tsx`). It may have a light version, eight colours more, worn where the
// system asks for light as the ones that ship are; its colours begin as
// worked out from the dark ones (`lightFrom`) until somebody changes one,
// and each version is held to being read on its own. It is kept with their account, with the choice of theme, so
// both are there in any browser they sign in to (`themed.ts`); and
// whoever may manage the factory offers one of theirs to everybody, and
// withdraws one. Any theme is saved as a small file, and one is read back
// in from one as a theme of the reader's own (`themefile.ts`). A theme of
// somebody's own may say how the client looks beyond its colours, under
// them in the form (`looks.ts`): the page wears each as it is chosen there,
// and goes back to the theme it is in where the form is left. Nothing is
// drawn that whoever looks may not do.

import { useRef, useState } from 'preact/hooks';
import { madeThemes, offeredThemes, prefersLight, setTheme, theme, themesRead, wearLook } from '../store.ts';
import { LOOKS, keptLook, lookSaid, wordOf } from '../looks.ts';
import type { Look, Setting } from '../looks.ts';
import { chooseTheme, offersThemes, readsThemes, themes, writesThemes } from '../themed.ts';
import { picks } from '../picking.ts';
import { readThemeFile, themeFile } from '../themefile.ts';
import { COLOURS, LIGHTS, STARTS, THEMES, lightFrom, listOf, unreadableEither, wornOf } from '../themes.ts';
import type { Colours, KeptTheme } from '../themes.ts';
import { Field, Modal, useFocus } from '../ui.tsx';
import { ColourPick } from './colourpick.tsx';

export function ThemePanel() {
  // A theme being made, or one of somebody's own being changed.
  const [making, setMaking] = useState<KeptTheme | 'new' | null>(null);
  const [wrong, setWrong] = useState<string | null>(null);
  const [offering, mayWrite] = [offersThemes(), writesThemes()];
  const ask = (doing: Promise<unknown>) => {
    setWrong(null);
    doing.catch((error: Error) => setWrong(error.message));
  };
  const picked = useRef<HTMLInputElement>(null);
  const chosen = (one: { id: string; name: string; says: string }, style?: Record<string, string>) => (
    // Drawn in its own colours, whichever the page is in.
    <button key={one.id} class="theme-sample" data-theme={style ? undefined : one.id} style={style} role="radio" aria-checked={theme.value === one.id} onClick={() => void chooseTheme(one.id)}>
      <Sample name={one.name} says={one.says} used={theme.value === one.id} />
    </button>
  );
  /** The button that saves a theme as a file, made here by the browser. */
  const saving = (name: string, colours: Colours, light: Colours | null, look: Look = {}) => (
    <button class="ghost link" aria-label={`Save ${name} as a file`} onClick={() => saveAs(themeFile(name, colours, light, look))}>
      Save as a file
    </button>
  );
  const readIn = async (event: Event) => {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    setWrong(null);
    let text: string;
    try {
      text = await file.text();
    } catch {
      setWrong(`${file.name} was not read: the browser could not read the file.`);
      return;
    }
    const read = readThemeFile(text);
    if ('wrong' in read) {
      setWrong(`${file.name} was not read. ${read.wrong}`);
      return;
    }
    try {
      // One the reader has already, by its name and its colours, is that one.
      const kept = await themes.make(read.theme.name, read.theme.colours, read.theme.light, read.theme.look);
      await chooseTheme(kept.id);
    } catch (error) {
      // The daemon's own sentence.
      setWrong(`${file.name} was not read. ${(error as Error).message}`);
    }
  };
  return (
    <div class="panel">
      <div class="panel-head">
        <h2 class="grow">Theme</h2>
        <span class="faint">{readsThemes() ? 'Kept with your account.' : 'Kept in this browser.'} Each with a light version wears it where the page is light: as the system asks, or as Colour under Appearance says.</span>
      </div>
      <div class="stack theme-groups" role="radiogroup" aria-label="Theme">
        <div class="theme-samples">
          {THEMES.map((one) => (
            <div key={one.id} class="theme-made">
              {chosen(one)}
              <span class="row">{saving(one.name, STARTS[one.id]!, LIGHTS[one.id]!)}</span>
            </div>
          ))}
        </div>
        {offeredThemes.value.length > 0 && (
          <section class="theme-group" aria-label="Offered to everybody">
            <h3 class="theme-heading">Offered to everybody</h3>
            <div class="theme-samples">
              {offeredThemes.value.map((one) => (
                <div key={one.id} class="theme-made">
                  {chosen({ id: one.id, name: one.name, says: withLook(`Offered by ${one.offered?.name || one.owner_name}.`, one.look) }, listOf(wornOf(one, prefersLight.value)))}
                  <span class="row">
                    {saving(one.name, one.colours, one.light, one.look)}
                    {offering && (
                      <button class="ghost link" aria-label={`Withdraw ${one.name}`} onClick={() => ask(themes.withdraw(one.id))}>
                        Withdraw
                      </button>
                    )}
                  </span>
                </div>
              ))}
            </div>
          </section>
        )}
        {readsThemes() && (themesRead.value || madeThemes.value.length > 0) && (
          <section class="theme-group" aria-label="Your own">
            <h3 class="theme-heading">Your own</h3>
            <div class="theme-samples">
              {madeThemes.value.map((one) => (
                <div key={one.id} class="theme-made">
                  {chosen({ id: one.id, name: one.name, says: withLook(one.offered ? 'Yours, offered to everybody.' : 'A theme of your own.', one.look) }, listOf(wornOf(one, prefersLight.value)))}
                  <span class="row">
                    {saving(one.name, one.colours, one.light, one.look)}
                    {mayWrite && (
                      <>
                        <button class="ghost link" aria-label={`Change ${one.name}`} onClick={() => setMaking(one)}>
                          Change
                        </button>
                        <button class="ghost link" aria-label={`Remove ${one.name}`} onClick={() => ask(themes.takeAway(one.id))}>
                          Remove
                        </button>
                        {offering && !one.offered && (
                          <button class="ghost link" aria-label={`Offer ${one.name} to everybody`} onClick={() => ask(themes.offer(one.id))}>
                            Offer to everybody
                          </button>
                        )}
                        {offering && one.offered && (
                          <button class="ghost link" aria-label={`Withdraw ${one.name}`} onClick={() => ask(themes.withdraw(one.id))}>
                            Withdraw
                          </button>
                        )}
                      </>
                    )}
                  </span>
                </div>
              ))}
            </div>
            {/* What makes one is a line of its own under them, and no card among the themes. */}
            {mayWrite && themesRead.value && (
              <div class="theme-actions">
                <button type="button" title="A theme of your own: a few colours, and the rest worked out from them" onClick={() => setMaking('new')}>
                  + Add a theme
                </button>
                <input ref={picked} type="file" accept=".json,application/json" hidden aria-label="A theme's file to read" onChange={(event) => void readIn(event)} />
                <button type="button" title="A theme somebody saved as a file, here or in another factory, made a theme of your own" onClick={() => picked.current?.click()}>
                  Read a theme from a file
                </button>
              </div>
            )}
          </section>
        )}
        {wrong && <p class="bad theme-wrong">{wrong}</p>}
      </div>
      {making && <ThemeForm was={making === 'new' ? null : making} onClose={() => setMaking(null)} />}
    </div>
  );
}

/** What a theme's card says, with what it says of its look after it where it says anything. */
function withLook(says: string, look: Look | undefined): string {
  const said = lookSaid(look);
  return said ? `${says} ${said}.` : says;
}

/** Hands the browser a theme's file to save: nothing is asked of the daemon. */
function saveAs({ file, text }: { file: string; text: string }): void {
  const address = URL.createObjectURL(new Blob([text], { type: 'application/json;charset=utf-8' }));
  const link = document.createElement('a');
  link.href = address;
  link.download = file;
  link.click();
  // Let go of once the browser has begun to save it.
  setTimeout(() => URL.revokeObjectURL(address), 10_000);
}

/** What a theme's card says, drawn in whatever colours the card is given. */
function Sample({ name, says, used }: { name: string; says: string; used: boolean }) {
  return (
    <>
      <span class="sample-bar">
        {['accent', 'ok', 'info', 'agent', 'card', 'bad'].map((colour) => (
          <i key={colour} style={{ background: `var(--${colour})` }} />
        ))}
      </span>
      <span class="sample-body">
        <strong>{name}</strong>
        <span>{says}</span>
        <em>{used ? 'In use' : 'Use it'}</em>
      </span>
    </>
  );
}

/**
 * Eight colours to be chosen, each by its own swatch: of the dark version, or with `of` before each, of the light
 * one. A swatch opens the client's own picker (`colourpick.tsx`), which hands on each colour as it is chosen and, left
 * by Escape or a press outside, the colour as it was (`onBack`, for whoever has to put back more than the colour).
 */
function ColourBoxes({
  colours,
  of = '',
  shipped,
  wrong,
  onChange,
  onOpen,
  onBack,
}: {
  colours: Colours;
  of?: string;
  shipped: Record<string, Colours>;
  wrong: string | null;
  onChange: (key: keyof Colours, value: string) => void;
  onOpen?: () => void;
  onBack?: (key: keyof Colours, value: string) => void;
}) {
  const [open, setOpen] = useState<{ key: keyof Colours; was: string } | null>(null);
  const opener = useRef<HTMLElement | null>(null);
  const ships = THEMES.map((one) => ({ name: one.name, colours: shipped[one.id]! }));
  const back = () => {
    if (open && onBack) onBack(open.key, open.was);
    else if (open && colours[open.key] !== open.was) onChange(open.key, open.was);
    setOpen(null);
  };
  return (
    <div class="theme-colours">
      {COLOURS.map(([key, label, means]) => (
        <label key={key} title={means}>
          <button
            type="button"
            class="colour-swatch"
            style={{ background: colours[key] }}
            data-colour={colours[key]}
            aria-label={`${of}${label}`}
            aria-haspopup="dialog"
            aria-expanded={open?.key === key}
            onClick={(event) => {
              if (open?.key === key) return back();
              opener.current = event.currentTarget;
              onOpen?.();
              setOpen({ key, was: colours[key] });
            }}
          />
          <span>
            <strong>{label}</strong>
            <span class="faint">{means}</span>
          </span>
        </label>
      ))}
      {open && (
        <ColourPick
          key={open.key}
          called={`${of}${COLOURS.find(([one]) => one === open.key)![1]}`}
          was={open.was}
          offered={picks(colours, open.key, ships)}
          wrong={wrong}
          opener={opener}
          onChange={(value) => onChange(open.key, value)}
          onDone={() => setOpen(null)}
          onBack={() => {
            if (onBack) onBack(open.key, open.was);
            setOpen(null);
          }}
        />
      )}
    </div>
  );
}

/** What a theme says of how the client looks beyond its colours: a choice for each setting, the usual one where it says nothing. */
function LookBoxes({ look, onChange }: { look: Look; onChange: (setting: Setting, word: string) => void }) {
  return (
    <section class="theme-looks" aria-label="How it looks beyond its colours">
      {LOOKS.map(({ setting, label, says, words }) => (
        <label key={setting} class="theme-look">
          <span>
            <strong>{label}</strong>
            <span class="faint"> {says}</span>
          </span>
          <select aria-label={label} value={wordOf(look, setting)} onChange={(event) => onChange(setting, event.currentTarget.value)}>
            {words.map((one) => (
              <option key={one.word} value={one.word}>
                {one.label}
              </option>
            ))}
          </select>
        </label>
      ))}
    </section>
  );
}

function ThemeForm({ was, onClose: closed }: { was: KeptTheme | null; onClose: () => void }) {
  const [name, setName] = useState(was?.name ?? '');
  const [from, setFrom] = useState(THEMES.some((one) => one.id === theme.value) ? theme.value : 'grid-iron');
  const [colours, setColours] = useState<Colours>(was?.colours ?? STARTS[from]!);
  // The light version: whether there is one, its colours, and whether somebody chose any of them. Until they do it is
  // worked out from the dark one, and follows it.
  const [lit, setLit] = useState(Boolean(was?.light));
  const [light, setLight] = useState<Colours>(was?.light ?? lightFrom(colours));
  const [ownLight, setOwnLight] = useState(Boolean(was?.light));
  // Whether the light version was chosen here as its picker was opened, which is what a picker left unkept puts back.
  const ownBefore = useRef(ownLight);
  const [refused, setRefused] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  // What it says of how the client looks: the page wears it as it is chosen, so that what it comes to is seen whole.
  const [look, setLook] = useState<Look>(keptLook(was?.look));
  const lookIs = (setting: Setting, word: string) => {
    const next = keptLook({ ...look, [setting]: word });
    setLook(next);
    wearLook(next);
  };
  // Left however it is left, the page is in the theme it is in again, with that theme's look.
  const onClose = () => {
    closed();
    setTheme(theme.value);
  };
  const focus = useFocus<HTMLInputElement>();
  const wrong = unreadableEither(colours, lit ? light : null);
  const darkIs = (next: Colours) => {
    setColours(next);
    if (!ownLight) setLight(lightFrom(next));
  };
  const keep = async (event: Event) => {
    event.preventDefault();
    if (!name.trim() || wrong || asking) return;
    setAsking(true);
    setRefused(null);
    const said = lit ? light : null;
    try {
      const kept = was ? await themes.change(was.id, name.trim(), colours, said, look) : await themes.make(name.trim(), colours, said, look);
      await chooseTheme(kept.id);
      closed();
    } catch (error) {
      // The daemon's own sentence, with what was written left as it was.
      setRefused((error as Error).message);
    } finally {
      setAsking(false);
    }
  };
  return (
    <Modal onClose={onClose}>
      <form class="stack" onSubmit={keep}>
        <h2>{was ? `Change ${was.name}` : 'A theme of your own'}</h2>
        <Field label="What it is called">
          <input ref={focus} value={name} maxLength={60} aria-label="What it is called" onInput={(event) => setName(event.currentTarget.value)} />
        </Field>
        {!was && (
          <Field label="Starting from">
            <select
              aria-label="Starting from"
              value={from}
              onChange={(event) => {
                setFrom(event.currentTarget.value);
                darkIs(STARTS[event.currentTarget.value]!);
              }}
            >
              {THEMES.map((one) => (
                <option key={one.id} value={one.id}>
                  {one.name}
                </option>
              ))}
            </select>
          </Field>
        )}
        <ColourBoxes colours={colours} shipped={STARTS} wrong={wrong} onChange={(key, value) => darkIs({ ...colours, [key]: value })} />
        <label class="row theme-lit">
          <input type="checkbox" checked={lit} aria-label="A light version" onChange={(event) => setLit(event.currentTarget.checked)} />
          <span>
            <strong>A light version</strong>
            <span class="faint"> worn where the page is light, as the themes that ship are. Without one, the theme is the same in both.</span>
          </span>
        </label>
        {lit && (
          <section class="stack theme-light" aria-label="The light version">
            <div class="row">
              <span class="faint grow">{ownLight ? 'Chosen here.' : 'Worked out from the colours above, until you change one.'}</span>
              {ownLight && (
                <button
                  type="button"
                  class="ghost link"
                  onClick={() => {
                    setOwnLight(false);
                    setLight(lightFrom(colours));
                  }}
                >
                  Work it out again
                </button>
              )}
            </div>
            <ColourBoxes
              colours={light}
              of="Light "
              shipped={LIGHTS}
              wrong={wrong}
              onOpen={() => (ownBefore.current = ownLight)}
              onChange={(key, value) => {
                setOwnLight(true);
                setLight((had) => ({ ...had, [key]: value }));
              }}
              onBack={(key, value) => {
                // Put back as it was: a light version that was still worked out from the dark one goes on being so.
                setOwnLight(ownBefore.current);
                setLight((had) => (ownBefore.current ? { ...had, [key]: value } : lightFrom(colours)));
              }}
            />
          </section>
        )}
        <LookBoxes look={look} onChange={lookIs} />
        <div class="theme-trials">
          <div class="theme-sample theme-trial" style={listOf(colours)} aria-label="How it looks">
            <Sample name={name.trim() || 'How it looks'} says="Text on its ground, a quieter line, and what is pressed." used />
          </div>
          {lit && (
            <div class="theme-sample theme-trial" style={listOf(light)} aria-label="How it looks in light">
              <Sample name={name.trim() || 'How it looks'} says="Where the system asks for light." used />
            </div>
          )}
        </div>
        {(wrong ?? refused) && <p class="bad">{wrong ?? refused}</p>}
        {/* Kept in reach at the foot, since with a light version the dialog is taller than a window has room for. */}
        <div class="row modal-foot">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" disabled={!name.trim() || wrong !== null || asking}>
            {was ? 'Keep it' : 'Add it'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
