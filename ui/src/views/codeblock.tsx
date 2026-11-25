// A block of code as what somebody wrote draws it, with a button at its
// top right that puts what it holds on the clipboard.
//
// The button lies over the block's corner, in room the block always
// keeps at its right, so that no line of the code is ever under it and
// nothing moves as it is shown: it is drawn while the block is pointed at
// or the keyboard is in it, and always under a finger (`code.css`). It
// is reached by Tab, is called Copy code for whoever hears it read out,
// says Copied for a moment once it has, and what it put there is the
// block's text as it was written (`codeblocks.ts`), never what the page
// drew of it.
//
// A block written in a language something draws (a diagram) is drawn by
// that, from `RENDERERS`, the one place a renderer is plugged in: a
// `mermaid` block is drawn as its diagram (`diagram.tsx`), and an
// `infobox`, `series` or `note` block as its box (`boxes.tsx`). A block in a
// language that is a diagram nothing here draws stays its code, with a
// faint line under it saying so (`undrawnSaid`).

import type { ComponentChildren } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { copySaid, copyWord, drawerOf, partsOf, undrawnSaid } from '../codeblocks.ts';
import type { Copied } from '../codeblocks.ts';
import { BoxBlock } from './boxes.tsx';
import { Diagram } from './diagram.tsx';
import { PathsIn, useAskedPaths } from './pathlink.tsx';

// How long the button says what it did before it says Copy again. Nothing
// waits on it: a check reads what the button says it has done (`data-copied`).
const SAYS_FOR = 1600;
// The widest of what it says, which it keeps room for whichever it says.
const WIDEST = (['idle', 'copied', 'failed'] as const).map(copyWord).sort((a, b) => b.length - a.length)[0];

/** Puts text on the clipboard: by the browser's own way where the page may, and by a selection copied where it may not. */
async function onClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) return navigator.clipboard.writeText(text);
  // A page served at a name that is not this machine's is not one the browser hands the clipboard to.
  const area = document.createElement('textarea');
  area.value = text;
  area.setAttribute('readonly', '');
  area.style.position = 'fixed';
  area.style.opacity = '0';
  // Selecting it takes the keyboard, which is given back to what had it (the button).
  const had = document.activeElement as HTMLElement | null;
  document.body.append(area);
  area.select();
  const done = document.execCommand('copy');
  area.remove();
  had?.focus();
  if (!done) throw new Error('not copied');
}

/**
 * What draws a block of code written in a language as what it means, by
 * the language's name (as `codeblocks.ts` keeps it, in small letters). A
 * renderer is added here and nowhere else. Each is handed the block as its
 * code too, for while it draws, where it cannot, and to be shown on asking;
 * and the record the text is of (a topic), whose pictures and page it names.
 */
export const RENDERERS: Record<string, (text: string, code: ComponentChildren, record: string | null) => ComponentChildren> = {
  mermaid: (text, code) => <Diagram text={text} code={code} />,
  infobox: (text, code, record) => <BoxBlock lang="infobox" text={text} code={code} record={record} />,
  series: (text, code, record) => <BoxBlock lang="series" text={text} code={code} record={record} />,
  note: (text, code, record) => <BoxBlock lang="note" text={text} code={code} record={record} />,
};

export function CodeBlock({ text, lang, record = null }: { text: string; lang?: string; record?: string | null }) {
  const draw = drawerOf(lang, RENDERERS);
  if (draw) return <>{draw(text, <CodeBox text={text} />, record)}</>;
  const undrawn = undrawnSaid(lang, Object.keys(RENDERERS));
  if (!undrawn) return <CodeBox text={text} />;
  return (
    <div class="code-undrawn" data-lang={lang}>
      <CodeBox text={text} />
      <p class="faint code-undrawn-said">{undrawn}</p>
    </div>
  );
}

/** The block as its code, with its Copy button. */
function CodeBox({ text }: { text: string }) {
  const [copied, setCopied] = useState<Copied>('idle');
  const back = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(back.current), []);
  const copy = () => {
    window.clearTimeout(back.current);
    const said = (now: Copied) => {
      setCopied(now);
      window.clearTimeout(back.current);
      back.current = window.setTimeout(() => setCopied('idle'), SAYS_FOR);
    };
    onClipboard(text).then(
      () => said('copied'),
      () => said('failed'),
    );
  };
  return (
    <div class="code-box code-block">
      {/* A file of a project named in it is a link to it; what is copied is the text as it was written. */}
      <pre class="code-block-text">
        <PathsIn text={text} />
      </pre>
      {/* What it says is drawn from its marks and is no text of the page's, so that the block's words read, and are selected, without it. The widest it may say is there unseen, so that it is as wide whichever it says. */}
      <button type="button" class="code-copy" aria-label="Copy code" data-copied={copied} data-word={copyWord(copied)} data-widest={WIDEST} onClick={copy} />
      <span class="code-copy-said" role="status">
        {copySaid(copied)}
      </span>
    </div>
  );
}

/**
 * Words as they are drawn where they are (`words`), with each block of
 * code in them drawn as one: for what is not drawn by `Prose` and still
 * holds what somebody wrote (a line of a record's history, a told line).
 * The paths of files in it are asked about here, once for the whole
 * (`useAskedPaths`); `words` draws them as links where it says so.
 */
export function WithCode({ text, words }: { text: string; words: (text: string) => ComponentChildren }) {
  useAskedPaths(text);
  const parts = partsOf(text);
  if (!parts.some((part) => part.is === 'code')) return <>{words(text)}</>;
  return <>{parts.map((part, at) => (part.is === 'code' ? <CodeBlock key={at} text={part.text} lang={part.lang} /> : <span key={at}>{words(part.text)}</span>))}</>;
}
