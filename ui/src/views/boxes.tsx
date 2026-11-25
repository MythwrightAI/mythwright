// The boxes of an article, drawn (`boxes.ts` reads them): an info box of
// key facts with its picture, a series of the pages that go together with
// the page it is drawn on marked, and a note at the article's top.
//
// On a topic's article they are drawn apart from its text: the info boxes
// and series at its side (`sideBoxes`, in the article's `boxes`), a series
// placed at the end after the text, and the first note over everything
// (`topicarticle.tsx`); the text then leaves them out (`BoxesApart`).
// Anywhere else (a reply, a thread's post, a message) a box is drawn where
// it was written, through the renderers of a block of code
// (`RENDERERS` in `codeblock.tsx`), and so is a block that cannot be read,
// as its code with a faint line under it saying what is wrong.
//
// What a box says is drawn as any line is (`Prose`): a link to a page
// opens it, and a plain link opens in a window of its own and is never
// fetched. Its picture is the topic's, drawn as any picture of it is.

import { createContext } from 'preact';
import type { ComponentChildren } from 'preact';
import { boxedOf, boxOf, lists } from '../boxes.ts';
import type { Box, BoxLanguage, InfoBox, Note, Series } from '../boxes.ts';
import { read } from '../prose.ts';
import { Headed, Prose } from '../ui.tsx';
import { classes } from '../util.ts';
import { Pictured } from './pictured.tsx';

/** Whether the text being drawn is an article's post, whose boxes are drawn apart from it and not again in it. */
export const BoxesApart = createContext(false);

/**
 * A line of a box, drawn as a line of text is; `record` is the topic whose
 * pictures and pages it names. A box is no part of an article's text, so
 * what a line says as a heading is no heading of the article (`Headed`):
 * not in its Contents, and with no address or Edit of its own.
 */
function Line({ text, record }: { text: string; record: string | null }) {
  return (
    <Headed.Provider value={null}>
      <Prose text={text} {...(record ? { attached: record } : {})} />
    </Headed.Provider>
  );
}

export function InfoBoxDrawn({ box, record }: { box: InfoBox; record: string | null }) {
  return (
    <section class="info-box" aria-label={box.title}>
      <div class="box-title">
        <Line text={box.title} record={record} />
      </div>
      {box.picture && (
        <div class="info-box-picture">
          <Pictured picture={{ caption: box.picture.caption, to: box.picture.to, place: null }} record={record} figure />
        </div>
      )}
      {box.rows.length > 0 && (
        <table class="info-box-rows">
          <tbody>
            {box.rows.map((row, at) => (
              <tr key={at}>
                <th scope="row">
                  <Line text={row.key} record={record} />
                </th>
                <td>
                  <Line text={row.value} record={record} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

/** A series, with the page it is drawn on (`page`) marked among its pages, as where the reader is. */
export function SeriesDrawn({ box, record }: { box: Series; record: string | null }) {
  const here = lists(box, record);
  return (
    <nav class={classes('series-box', `place-${box.place}`)} aria-label={box.title} data-here={here ? '' : undefined}>
      <div class="box-title">
        <Line text={box.title} record={record} />
      </div>
      {box.groups.map((group, at) => (
        <div class="series-group" key={at}>
          {group.name && (
            <div class="series-group-name">
              <Line text={group.name} record={record} />
            </div>
          )}
          <ul class="series-pages">
            {group.pages.map((one, nth) => (
              <li key={nth} class={classes(one.topic === record && 'here')} aria-current={one.topic === record ? 'page' : undefined}>
                <Line text={one.line} record={record} />
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  );
}

/** A note pointing elsewhere: at an article's top, or where it was written. */
export function NoteDrawn({ box, record, head }: { box: Note; record: string | null; head?: boolean }) {
  return (
    <div class={classes('box-note', head && 'head')} role="note">
      {box.lines.map((line, at) => (
        <p key={at}>
          <Line text={line} record={record} />
        </p>
      ))}
    </div>
  );
}

/** A box, drawn as its kind is. */
function Drawn({ box, record }: { box: Box; record: string | null }) {
  if (box.is === 'infobox') return <InfoBoxDrawn box={box} record={record} />;
  if (box.is === 'series') return <SeriesDrawn box={box} record={record} />;
  return <NoteDrawn box={box} record={record} />;
}

/** A block of code in a box's language, drawn where it was written: its box, or its code and what is wrong with it. */
export function BoxBlock({ lang, text, code, record }: { lang: BoxLanguage; text: string; code: ComponentChildren; record: string | null }) {
  const found = boxOf(lang, text);
  if ('wrong' in found) {
    return (
      <div class="code-undrawn box-wrong" data-lang={lang}>
        {code}
        <p class="faint code-undrawn-said">{found.wrong}</p>
      </div>
    );
  }
  return (
    <div class="box-inline" data-lang={lang}>
      <Drawn box={found.box} record={record} />
    </div>
  );
}

/** The boxes an article's post has at its side, for its `boxes`: nothing where it has none. */
export function sideBoxes(body: string, page: string): ComponentChildren | undefined {
  const { side } = boxedOf(read(body));
  if (side.length === 0) return undefined;
  return side.map((box, at) => <Drawn key={at} box={box} record={page} />);
}
