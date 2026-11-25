// A `mermaid` block of code drawn as the diagram it says, where somebody's
// words are drawn (`RENDERERS` in `codeblock.tsx`).
//
// The drawing library is large, and most pages have no diagram: it is a
// part of its own, loaded the first time a page draws one. Until it has
// drawn, the block is its code. It is drawn in the page's light or dark,
// and again when that changes (`themechange`, `wearing.ts`). One it cannot
// read is its code with a faint line under it saying what is wrong, the
// library's own words on one line (`diagrams.ts`).
//
// What it draws is the library's SVG, made with nothing of it run and
// nothing it says taken as markup (`strict`), and cleaned by the library
// before it is handed over, in the page's own colours (`diagramColours`).
// Its box is fitted to what it draws (`trimmedBox`). It is as wide as the
// column where its words still read so, and drawn where they read and
// scrolled sideways in its box past that, with a shade at the edge that has
// more (`diagramWidth`, `moreAt`). Pressed, or by Open under it, it opens
// whole over the page, as large as it was drawn, as a picture opens whole;
// nothing is fetched for it. Full screen under it has it fill the screen,
// as large as the screen holds it whole (`fullscreen.tsx`). It is called by its title or first line for
// whoever hears it read out, and Code under it shows what it was drawn
// from, with its Copy button.

import type { ComponentChildren } from 'preact';
import { createPortal } from 'preact/compat';
import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { diagramName, diagramSettings, diagramWidth, notDrawnSaid, trimmedBox, typicalText } from '../diagrams.ts';
import type { PageColours } from '../diagrams.ts';
import { Modal } from '../ui.tsx';
import { FullBox, FullButton, useFullScreen } from './fullscreen.tsx';
import { settle } from './viewframe.tsx';
import { useMoreShown } from './wider.tsx';

type Mermaid = (typeof import('mermaid'))['default'];

let loading: Promise<Mermaid> | null = null;
/** The drawing library, loaded once: asked again after it could not be, since the page may have been offline. */
function library(): Promise<Mermaid> {
  loading ??= import('mermaid').then(
    (loaded) => loaded.default,
    (err) => {
      loading = null;
      throw err;
    },
  );
  return loading;
}

// The library is set up for the whole page, so diagrams are drawn one at a
// time, each set up for its own light or dark just before it is drawn.
let drawing: Promise<unknown> = Promise.resolve();
let drawn = 0;
const isLight = () => 'light' in document.documentElement.dataset;

/** The page's colours as the browser computes its tokens, in whatever form they were written: what the library can read. */
function pageColours(): PageColours {
  const probe = document.createElement('span');
  probe.hidden = true;
  document.body.append(probe);
  const token = (name: string) => {
    probe.style.color = `var(--${name})`;
    return getComputedStyle(probe).color;
  };
  const colours = { background: token('bg0'), node: token('bg2'), text: token('text'), dim: token('dim'), line: token('line-strong'), faint: token('faint') };
  probe.remove();
  return colours;
}

/** A diagram drawn as SVG, in the light or dark it is asked for. */
function draw(text: string, light: boolean): Promise<string> {
  // An id of its own in the page: the SVG's styles are held to it.
  const id = `diagram-${++drawn}`;
  const turn = drawing.then(async () => {
    const mermaid = await library();
    mermaid.initialize(diagramSettings(light, getComputedStyle(document.body).fontFamily, pageColours()));
    return (await mermaid.render(id, text)).svg;
  });
  drawing = turn.catch(() => {});
  return turn;
}

type Drawn = { svg: string } | { failed: string } | null;

/** The sizes the words of a drawn diagram are drawn at, each as the page computes it. */
function textSizes(svg: SVGSVGElement): number[] {
  return [...svg.querySelectorAll('text, foreignObject div, foreignObject span')].map((one) => parseFloat(getComputedStyle(one).fontSize));
}

/**
 * Fits a drawn diagram's box to what it draws, once, and says the width it
 * was drawn at and the size of its words. Measured where it is drawn, as
 * nothing else can say how wide its words came out.
 */
function fitted(svg: SVGSVGElement): { natural: number; text: number } {
  // Measured once it is laid out: a diagram in a tab not showing has no size yet. A gantt's line for today is left
  // out of what it draws, since today may be months past its last bar.
  const today = [...svg.querySelectorAll<SVGElement>('.today')];
  today.forEach((one) => (one.style.display = 'none'));
  const drawn = svg.dataset.fitted === undefined ? svg.getBBox() : null;
  today.forEach((one) => (one.style.display = ''));
  if (drawn && drawn.width > 0) {
    svg.dataset.fitted = '';
    const given = svg.viewBox.baseVal;
    const trimmed = given ? trimmedBox(given, drawn) : null;
    if (trimmed) svg.setAttribute('viewBox', `${trimmed.x} ${trimmed.y} ${trimmed.width} ${trimmed.height}`);
  }
  const box = svg.viewBox.baseVal;
  return { natural: box?.width ?? 0, text: typicalText(textSizes(svg)) };
}

/** Draws a diagram `width` wide, as high as that makes it. */
function sized(svg: SVGSVGElement, width: number): void {
  const box = svg.viewBox.baseVal;
  if (!box || !(box.width > 0) || !(width > 0)) return;
  svg.setAttribute('width', String(width));
  svg.setAttribute('height', String(Math.round((width * box.height) / box.width)));
  svg.style.maxWidth = 'none';
}

/**
 * A diagram opened whole over the page: as large as it was drawn, scrolled in its box where that is larger than the
 * window. On the page's body, since the article it is in holds what is fixed in it to itself (a container).
 */
function DiagramWhole({ svg, name, onClose }: { svg: string; name: string; onClose: () => void }) {
  return createPortal(
    <Modal wide class="diagram-whole" onClose={onClose}>
      <div class="diagram-whole-head">
        <h2 class="diagram-whole-name">{name}</h2>
        <button type="button" class="ghost diagram-whole-close" onClick={onClose} autoFocus>
          Close
        </button>
      </div>
      <div class="diagram-whole-box" role="img" aria-label={name} tabIndex={0} dangerouslySetInnerHTML={{ __html: svg }} />
    </Modal>,
    document.body,
  );
}

/** A diagram from its text; `code` is the block as its code, shown until it is drawn, where it cannot be, and by Code. */
export function Diagram({ text, code }: { text: string; code: ComponentChildren }) {
  const [light, setLight] = useState(isLight);
  const [drawnAs, setDrawnAs] = useState<Drawn>(null);
  const [showCode, setShowCode] = useState(false);
  const [whole, setWhole] = useState<string | null>(null);
  const shown = useRef<HTMLDivElement>(null);
  const framed = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const was = useRef(0);
  const full = useFullScreen();
  useEffect(() => {
    const told = () => setLight(isLight());
    window.addEventListener('themechange', told);
    return () => window.removeEventListener('themechange', told);
  }, []);
  useEffect(() => {
    let stale = false;
    // What was drawn in the other light stays until this one is.
    draw(text, light).then(
      (svg) => !stale && setDrawnAs({ svg }),
      (err) => !stale && setDrawnAs({ failed: notDrawnSaid(err) }),
    );
    return () => {
      stale = true;
    };
  }, [text, light]);
  useLayoutEffect(() => {
    const height = shown.current?.offsetHeight ?? 0;
    settle(shown.current, Math.max(0, height - was.current));
    was.current = height;
  }, [drawnAs, showCode]);
  useLayoutEffect(() => {
    const [svg, frame, scroller] = [box.current?.querySelector('svg'), framed.current, box.current];
    if (!svg || !frame || !scroller) return;
    // Drawn again to the column whenever that changes width.
    const fit = () => {
      const { natural, text: words } = fitted(svg);
      const drawnTo = diagramWidth(natural, scroller.clientWidth, words);
      sized(svg, drawnTo.width);
      frame.toggleAttribute('data-scrolls', drawnTo.scrolls);
    };
    fit();
    const watched = new ResizeObserver(fit);
    watched.observe(scroller);
    return () => watched.disconnect();
  }, [drawnAs, showCode]);
  useMoreShown(framed, box, [drawnAs, showCode]);

  /** Opens it whole: a copy of it as large as it was drawn. */
  const openWhole = () => {
    const svg = box.current?.querySelector('svg');
    if (!svg) return;
    const copy = svg.cloneNode(true) as SVGSVGElement;
    sized(copy, Math.round(copy.viewBox.baseVal?.width ?? 0));
    setWhole(copy.outerHTML);
  };

  if (drawnAs && 'failed' in drawnAs) {
    return (
      <div class="code-undrawn diagram-failed" data-lang="mermaid" ref={shown}>
        {code}
        <p class="faint code-undrawn-said">{drawnAs.failed}</p>
      </div>
    );
  }
  if (!drawnAs) {
    return (
      <div class="diagram" data-drawn="not-yet" ref={shown}>
        {code}
      </div>
    );
  }
  const name = diagramName(text);
  return (
    <div class="diagram" data-drawn={light ? 'light' : 'dark'} ref={shown}>
      {showCode ? (
        code
      ) : (
        <FullBox full={full} class="diagram-full">
          <div class="diagram-frame wider" ref={framed}>
            {/* In full screen it is already whole, and what would open it whole would open under it. */}
            <div class="diagram-box" role="img" aria-label={name} title={full.on ? undefined : 'Open it whole'} ref={box} onClick={full.on ? undefined : openWhole} dangerouslySetInnerHTML={{ __html: drawnAs.svg }} />
          </div>
        </FullBox>
      )}
      <div class="diagram-foot">
        {!showCode && (
          <button type="button" class="diagram-code diagram-open" onClick={openWhole} aria-label={`Open ${name} whole`}>
            Open
          </button>
        )}
        {!showCode && <FullButton full={full} class="diagram-code" />}
        <button type="button" class="diagram-code" aria-pressed={showCode} onClick={() => setShowCode(!showCode)}>
          Code
        </button>
      </div>
      {whole !== null && <DiagramWhole svg={whole} name={name} onClose={() => setWhole(null)} />}
    </div>
  );
}
