// A still copy of something on the page, for it to be seen going away.
//
// What is put away is off the page at once, with whatever it held: what
// it showed was of something the page has let go of, and nothing of it
// may go on being drawn from that. So what glides out is a still copy of
// it, taken in the moment it is put away, while it is still on the page:
// the same boxes and words, with nothing in it that runs, is pressed,
// takes the keyboard or is read aloud. The style sheet moves the copy,
// and it is taken off the page when that is over; where nothing moves it
// (less movement was asked for) it is never put there (`goesOut`).
//
// The copy is not called what the thing is called: whatever asks whether
// it is there finds none from the moment it is put away. And nothing that
// looks for something on the page finds anything inside it: what it holds
// is a closed tree of its own (a shadow root nobody is handed), drawn in
// the page's own style sheet. The drawer held a table, and a table opened
// as a tab from the drawer was found twice while the copy went out, once
// on its page and once in the picture of it, and a check read the
// picture's rows for the page's on a busy machine; nor could any other
// question the page asks of itself by a selector take the picture for
// the real thing. Only its own box is there to be found, called what the
// style sheet moves (`called`), with what it was a picture of said by
// `data-of`. What each field of it holds is written onto the copy, which
// would otherwise show each as it was first drawn (`holdsAsItStands`).
// The drawer goes out so (`drawerleaves.ts`), and so do the Tools panel
// beside a channel (`toolsslide.ts`) and the side panel (`sideslide.ts`).

/** The page's own rules, made once into a sheet a copy's closed tree can wear; made again when the page's sheets are others. */
let worn: { from: StyleSheet[]; sheet: CSSStyleSheet } | null = null;
function pageSheet(): CSSStyleSheet | null {
  if (typeof CSSStyleSheet === 'undefined') return null;
  const from = [...document.styleSheets];
  if (worn && worn.from.length === from.length && worn.from.every((one, at) => one === from[at])) return worn.sheet;
  let text = '';
  for (const one of from) {
    try {
      for (const rule of one.cssRules) text += `${rule.cssText}\n`;
    } catch {
      /* a sheet whose rules cannot be read is left out */
    }
  }
  // What is said of a part under the page's root holds of it in the copy too, which has no root of the page's above it.
  text = text.replace(/:root\s+(?=[a-z.#[*])/gi, '');
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(text);
  worn = { from, sheet };
  return sheet;
}

export interface Still {
  /** What the copy's box is called, which the style sheet gives its look and its way out. */
  called: string;
  /** What it is a picture of, said by its box. */
  of?: string | null;
  /** Whether the thing itself is copied into the tree, its own box with it, or only what is in it: the drawer's box is the copy's own. */
  whole?: boolean;
  /** Further things copied whole after it, which go out together with it: the rail beside the side panel on a phone. */
  more?: HTMLElement[];
  /**
   * Where the copy's own box begins on the page, for each thing copied whole
   * to stand inside it where it stood, whatever the style sheet says of
   * where such a thing is put (the side panel is a column of the page's, or
   * fixed over a phone's screen, and its copy is neither).
   */
  placed?: { left: number; top: number };
}

/** A still copy of `live` as it stands: its box, and in a closed tree of its own a picture of what was in it. */
export function stillOf(live: HTMLElement, { called, of, whole = false, more = [], placed }: Still): HTMLElement {
  const copy = document.createElement('div');
  copy.className = called;
  copy.setAttribute('aria-hidden', 'true');
  copy.inert = true;
  if (of) copy.dataset.of = of;
  const tree = copy.attachShadow({ mode: 'closed' });
  const sheet = pageSheet();
  if (sheet) tree.adoptedStyleSheets = [sheet];
  const wholes = whole ? [live, ...more] : [];
  const parts = whole ? wholes : [...live.childNodes];
  for (const part of parts) {
    const cloned = part.cloneNode(true);
    tree.append(cloned);
    if (placed && cloned instanceof HTMLElement && part instanceof HTMLElement) {
      const box = part.getBoundingClientRect();
      Object.assign(cloned.style, { position: 'absolute', inset: 'auto', margin: '0', boxSizing: 'border-box', left: `${box.left - placed.left}px`, top: `${box.top - placed.top}px`, width: `${box.width}px`, height: `${box.height}px` });
    }
  }
  for (const named of tree.querySelectorAll('[id]')) named.removeAttribute('id');
  // A frame would be fetched again to be shown going away.
  for (const frame of tree.querySelectorAll('iframe')) {
    frame.removeAttribute('src');
    frame.removeAttribute('srcdoc');
  }
  // How far each part was scrolled is not copied with it.
  const from = whole ? wholes.flatMap((one) => [one, ...one.querySelectorAll<HTMLElement>('*')]) : [...live.querySelectorAll<HTMLElement>('*')];
  const to = [...tree.querySelectorAll<HTMLElement>('*')];
  from.forEach((one, at) => holdsAsItStands(one, to[at]));
  const scrolled = from.map((one, at) => [at, one.scrollTop, one.scrollLeft] as const).filter(([, top, left]) => top !== 0 || left !== 0);
  queueMicrotask(() => {
    for (const [at, top, left] of scrolled) to[at]?.scrollTo(left, top);
  });
  return copy;
}

/**
 * What a field holds now, written onto its copy where a copy keeps it
 * (`selected`, `value`, `checked`, a text area's text): a copy keeps a
 * field's attributes and not what was chosen or typed in it since, and a
 * drawer going out showed each list at its first choice and each box as
 * it was first drawn.
 */
function holdsAsItStands(live: Element, copy: Element | undefined): void {
  if (!copy || live.tagName !== copy.tagName) return;
  if (live instanceof HTMLSelectElement) {
    const options = copy.querySelectorAll('option');
    [...live.options].forEach((option, at) => options[at]?.toggleAttribute('selected', option.selected));
  } else if (live instanceof HTMLTextAreaElement) {
    copy.textContent = live.value;
  } else if (live instanceof HTMLInputElement) {
    if (live.type === 'checkbox' || live.type === 'radio') copy.toggleAttribute('checked', live.checked);
    else if (live.type !== 'file' && live.type !== 'password') copy.setAttribute('value', live.value);
  }
}

/** Has a copy that was put on the page go out as the style sheet moves it, and takes it off when that is over: at once where nothing moves it. */
export function goesOut(copy: HTMLElement): void {
  const moving = typeof copy.getAnimations === 'function' ? copy.getAnimations() : [];
  if (moving.length === 0) return void copy.remove();
  void Promise.allSettled(moving.map((one) => one.finished)).then(() => copy.remove());
}
