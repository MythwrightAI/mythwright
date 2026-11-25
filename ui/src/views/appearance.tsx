// How what is read looks, offered where it is read and in Settings.
//
// `Appearance` is one button in the head of a page that is read (a topic,
// a document of a library), as reading sites have one: it opens a small
// menu of the text's size, the width the page is read at, light or
// dark, whether a long article's Contents show, and its typeface, each chosen at once
// and kept with the account (`appearancekept.ts`). The page says which element it is read in by the
// class `READING` (`appearance.ts`), and the width and the text are that
// element's (`appearance.css`). `AppearancePanel` is the same five in
// Settings. The menu is on the page's body (`Floating`), so a head that
// scrolls or clips does not cut it off; it takes the arrows, Home and End,
// and Escape gives the keyboard back to its button.

import { useEffect, useRef, useState } from 'preact/hooks';
import { APPEARANCE, changed } from '../appearance.ts';
import type { AppearanceSetting } from '../appearance.ts';
import { appearance, keepAppearance, writesAppearance } from '../appearancekept.ts';
import { classes } from '../util.ts';
import { besideBox, Floating } from './offered.tsx';

const choose = (setting: AppearanceSetting, word: string) => void keepAppearance(changed(appearance.peek(), setting, word));

/** The button in a reading page's head, and the menu it opens. */
export function Appearance() {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    // The chosen size first has the keyboard, as a list of choices gives it: once the menu is placed, which it is
    // as it is drawn, and drawn till then where it cannot take the keyboard.
    const give = () => {
      const chosen = menu.current?.querySelector<HTMLElement>('[aria-checked="true"]');
      chosen?.focus();
      return !chosen || document.activeElement === chosen;
    };
    if (!give()) requestAnimationFrame(give);
    const away = (event: Event) => {
      const inside = button.current?.contains(event.target as Node) || menu.current?.contains(event.target as Node);
      if (event instanceof KeyboardEvent) {
        if (event.key !== 'Escape') return;
        event.stopPropagation();
        setOpen(false);
        button.current?.focus();
      } else if (!inside) setOpen(false);
    };
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', away, true);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', away, true);
    };
  }, [open]);
  const step = (event: KeyboardEvent) => {
    const lines = [...(menu.current?.querySelectorAll<HTMLElement>('[role="menuitemradio"]') ?? [])];
    const at = lines.indexOf(document.activeElement as HTMLElement);
    const to = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: lines.length - 1 }[event.key];
    if (to === undefined || lines.length === 0) return;
    event.preventDefault();
    lines[(to + lines.length) % lines.length]?.focus();
  };
  const worn = appearance.value;
  return (
    <>
      <button
        ref={button}
        type="button"
        class={classes('ghost appearance-button', open && 'on')}
        aria-label="Appearance"
        data-tip="Appearance: text, width, colour and typeface"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <span aria-hidden="true">Aa</span>
      </button>
      {open && (
        <Floating
          beside={() => {
            const box = button.current?.getBoundingClientRect();
            // Its right edge under the button's, which stands at the right of a head.
            return box ? { ...besideBox(box, 'under'), left: box.right - (menu.current?.offsetWidth ?? 0), across: undefined } : null;
          }}
          own={menu}
          class="appearance-menu"
          role="menu"
          label="Appearance"
        >
          <div onKeyDown={step}>
            {APPEARANCE.map(({ setting, label, words }) => (
              <div key={setting} class="appearance-group" role="group" aria-label={label}>
                <div class="appearance-label" aria-hidden="true">
                  {label}
                </div>
                {words.map((one) => (
                  <button key={one.word} type="button" role="menuitemradio" aria-checked={worn[setting] === one.word} data-appearance={`${setting}=${one.word}`} onClick={() => choose(setting, one.word)}>
                    <span class="appearance-tick" aria-hidden="true" />
                    {one.label}
                  </button>
                ))}
              </div>
            ))}
            {!writesAppearance() && <p class="faint appearance-here">Kept in this browser only: keeping it with your account needs `themes:write`.</p>}
          </div>
        </Floating>
      )}
    </>
  );
}

/** The same five in Settings, each kept with the account as it is chosen. */
export function AppearancePanel() {
  const worn = appearance.value;
  return (
    <div class="panel appearance-setting">
      <div class="panel-head">
        <h2>Appearance</h2>
        <span class="muted grow">how what you read looks, and light or dark, in every browser</span>
      </div>
      <div class="stack" style="gap:10px;padding:12px 16px">
        {APPEARANCE.map(({ setting, label, words }) => (
          <div key={setting} class="appearance-line">
            <span class="appearance-label" id={`appearance-${setting}`}>
              {label}
            </span>
            <div class="pills" role="radiogroup" aria-labelledby={`appearance-${setting}`}>
              {words.map((one) => (
                <button type="button" key={one.word} role="radio" aria-checked={worn[setting] === one.word} onClick={() => choose(setting, one.word)}>
                  {one.label}
                </button>
              ))}
            </div>
          </div>
        ))}
        <span class="faint">
          Text and width are those of what is read: a topic, a document; Typeface is an article's and a document's letters. They are also offered from the button at the head of such a page. Colour is the whole client's: Automatic is light or dark as your
          system asks.
        </span>
      </div>
    </div>
  );
}
