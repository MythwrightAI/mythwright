// Small pieces every view uses.

import { createContext, Fragment } from 'preact';
import type { ComponentChildren } from 'preact';
import { useContext, useEffect, useLayoutEffect, useRef } from 'preact/hooks';
import { headingsOf } from './contents.ts';
import type { Heading } from './contents.ts';
import type { Handler } from './events.ts';
import { stream } from './events.ts';
import { usePeople } from './peopleknown.ts';
import { phone } from './phone.ts';
import { notesOf } from './cited.ts';
import type { Notes } from './cited.ts';
import { cardsIn, read } from './prose.ts';
import type { Block, Span } from './prose.ts';
import { foldSide, hasBoard, meta, projects, showCard, side } from './store.ts';
import { classes } from './util.ts';
import { Cite, NotesList } from './views/cited.tsx';
import { boxedOf } from './boxes.ts';
import { BoxesApart } from './views/boxes.tsx';
import { CodeBlock } from './views/codeblock.tsx';
import { withMentions } from './views/mentioned.tsx';
import { PageLink } from './views/pagelink.tsx';
import { ProseTable } from './views/prosetable.tsx';
import { Pictured } from './views/pictured.tsx';
import { useAskedPaths, withPaths, WordsOf } from './views/pathlink.tsx';

/** A word in a coloured pill; `says` is what it means, read where it is pointed at or reached by the keyboard. */
export function Badge({ status, children, says }: { status: string; children?: ComponentChildren; says?: string }) {
  return (
    <span class={classes('badge', status)} title={says} tabIndex={says ? 0 : undefined} aria-label={says ? `${typeof children === 'string' ? children : status}: ${says}` : undefined}>
      {children ?? status}
    </span>
  );
}

/**
 * Said of a project wherever it is shown, while it is not trusted: it was
 * made of a folder that was already there, and what somebody else put in
 * that folder does not get to be `std/board` here until the owner says so.
 */
export function NotTrusted({ project, brief }: { project: string; brief?: boolean }) {
  const found = projects.value.find((one) => one.id === project);
  if (!found || found.trusted !== false) return null;
  const why = `${found.name} was made of a folder that was already there. A file of its own with the name of a blueprint that ships is not used in place of that one until somebody who manages this factory says so: Trust this project, in the gear of its page, or ${meta.value?.cli ?? ''} project trust ${found.id}`;
  return (
    <span class={classes('badge plain warning not-trusted', brief && 'brief')} title={why} aria-label={`${found.name} is not trusted`}>
      not trusted
    </span>
  );
}

export function Pills<T extends string>(props: { value: T; options: readonly (T | [T, string])[]; onChange: (value: T) => void }) {
  return (
    <div class="pills" role="tablist">
      {props.options.map((option) => {
        const [value, label] = Array.isArray(option) ? option : [option, option];
        return (
          <button key={value} role="tab" aria-pressed={props.value === value} onClick={() => props.onChange(value)}>
            {label}
          </button>
        );
      })}
    </div>
  );
}

/**
 * A dialog. `plain` is for one with nothing behind it (signing in): it sits
 * on the bare page instead of dimming an empty one.
 */
export function Modal(props: { onClose?: () => void; wide?: boolean; plain?: boolean; class?: string; children: ComponentChildren }) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') props.onClose?.();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [props.onClose]);
  return (
    <div
      class={classes('scrim', props.plain && 'plain')}
      onMouseDown={(event) => event.target === event.currentTarget && props.onClose?.()}
    >
      <div class={classes('modal', props.wide && 'wide', props.class)} role="dialog" aria-modal="true">
        {props.children}
        {/* A wide one has no button of its own to close it: it is left by pressing beside it, and on a phone there is no beside. */}
        {phone.value && props.wide && props.onClose && (
          <button class="ghost icon modal-away" aria-label="Close" onClick={props.onClose}>
            ✕
          </button>
        )}
      </div>
    </div>
  );
}

export function Empty({ title, children }: { title: string; children?: ComponentChildren }) {
  return (
    <div class="empty">
      <strong>{title}</strong>
      {children}
    </div>
  );
}

/**
 * The button that folds the side panel away for room, or brings it back
 * showing what it last showed. It is at the panel's head while there is a
 * panel, and at the start of the tabs while there is not.
 */
export function SideFold({ class: name }: { class: string }) {
  const shown = side.value !== null;
  const says = shown ? 'Collapse the side panel' : 'Expand the side panel';
  return (
    // Where it is put moves the window in the desktop app; pressing this does not.
    <button class={name} aria-label={says} aria-expanded={shown} title={`${says} (Ctrl \\)`} onMouseDown={(event) => event.stopPropagation()} onClick={foldSide}>
      <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
        {/* On a phone the panel is not beside the page to be pushed back from: what brings it over the page is three lines, as everywhere. */}
        <path d={shown ? 'M9 4.5 4.5 9 9 13.5M13.5 4.5 9 9l4.5 4.5' : phone.value ? 'M3 5h12M3 9h12M3 13h12' : 'M9 4.5 13.5 9 9 13.5M4.5 4.5 9 9l-4.5 4.5'} fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" stroke-linecap="round" />
      </svg>
    </button>
  );
}

export function Field({ label, children }: { label: string; children: ComponentChildren }) {
  return (
    <div class="field">
      <span>{label}</span>
      {children}
    </div>
  );
}

/** Subscribes to events for as long as the component is shown. */
/**
 * Follows `patterns` for as long as the component is there. `again` reads
 * again what it shows of them, and is asked where the stream says it may
 * have missed some (`behind`): what the runner says of its runs and steps
 * is passed on and not kept, so a stream that goes on after a reconnect,
 * or that fell behind, cannot hand it over. Whatever follows `run.*` or
 * `step.*` says `again` (a test holds it: `events.test.ts`).
 */
export function useEvents(patterns: string | string[], handler: Handler, deps: unknown[] = [], again?: () => void): void {
  const latest = useRef(handler);
  latest.current = handler;
  const reread = useRef(again);
  reread.current = again;
  useEffect(() => stream.on(patterns, (event) => latest.current(event), again ? () => reread.current?.() : undefined), deps);
}

/**
 * Puts the cursor in a field when it appears: in the commit that draws it,
 * and not a moment after it is painted, so that whoever sees the field sees
 * it with the keyboard (asked where the keyboard was as a dialog was drawn,
 * the checks of the invitation and of removing a workspace found it
 * nowhere, now and then).
 */
export function useFocus<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  useLayoutEffect(() => ref.current?.focus(), []);
  return ref;
}

/**
 * Where the text drawn is an article whose headings are known (`contents.ts`):
 * what stands beside each heading (its link, its section's Edit), which
 * puts each heading in a line with it, known by its slug (`data-section`).
 */
export const Headed = createContext<((heading: Heading) => ComponentChildren) | null>(null);

/** A heading of an article, and what stands beside it. */
type HeadingAt = { heading: Heading; beside: ComponentChildren };

/** How each kind of block `prose.ts` reads is drawn: one entry for each kind, which the types hold to every kind there is. */
const DRAWN: { [K in Block['is']]: (block: Extract<Block, { is: K }>, drawn: (spans: Span[]) => ComponentChildren[], record: string | null, at?: HeadingAt) => ComponentChildren } = {
  text: (block, drawn) => <div class="prose-text">{drawn(block.spans)}</div>,
  heading: (block, drawn, _record, at) => {
    const heading = (
      <div class={`prose-heading level-${block.level}`} role="heading" aria-level={block.level + 2}>
        {drawn(block.spans)}
      </div>
    );
    return at ? (
      <div class={`prose-heading-line level-${block.level}`} data-section={at.heading.slug}>
        {heading}
        {at.beside}
      </div>
    ) : (
      heading
    );
  },
  quote: (block, drawn) => <blockquote class="prose-quote">{drawn(block.spans)}</blockquote>,
  list: (block, drawn) => (
    <div class="prose-list" role="list">
      {block.items.map((item, nth) => (
        <div class="prose-item" role="listitem" key={nth} style={item.depth > 0 ? { marginLeft: item.depth * 20 } : undefined}>
          <span class="prose-mark">{item.mark}</span>
          <span>{drawn(item.spans)}</span>
        </div>
      ))}
    </div>
  ),
  table: (block, drawn) => <ProseTable block={block} drawn={drawn} />,
  code: (block, _drawn, record) => <CodeBlock text={block.text} lang={block.lang} record={record} />,
  figure: (block, _drawn, record) => <Pictured picture={block} record={record} figure />,
  // A source is drawn in the list of them, at the end of what it is of, and not where it was written.
  note: () => null,
};

/** One block, drawn as its kind is. */
function drawnBlock<K extends Block['is']>(block: Extract<Block, { is: K }>, drawn: (spans: Span[]) => ComponentChildren[], record: string | null, at?: HeadingAt): ComponentChildren {
  return (DRAWN[block.is] as (block: Extract<Block, { is: K }>, drawn: (spans: Span[]) => ComponentChildren[], record: string | null, at?: HeadingAt) => ComponentChildren)(block, drawn, record, at);
}

/**
 * Text as people write it in a channel, drawn as it is meant to look: what
 * `prose.ts` reads in it, and the rest as it was written. `attached` is
 * the record the text is of (a topic), whose attachments its pictures name.
 * Its sources are listed at its end, numbered as its markers are, unless
 * they are drawn `apart` (a topic's post, whose sources are its article's
 * References); and its boxes are left out where the article draws them
 * apart from it (`BoxesApart`, `views/boxes.tsx`).
 * Inside `Headed` (an article read) each heading is known by its slug and
 * has what the page hands beside it.
 */
export function Prose({ text, attached, notes = 'here' }: { text: string; attached?: string; notes?: 'here' | 'apart' }) {
  const own = useContext(WordsOf);
  const beside = useContext(Headed);
  useAskedPaths(text);
  // Who a name is, read once for the page: a name of somebody known is drawn as a mention.
  usePeople();
  const record = attached ?? null;
  const all = read(text);
  const apart = useContext(BoxesApart) ? boxedOf(all).apart : null;
  const blocks = apart?.size ? all.filter((_block, at) => !apart.has(at)) : all;
  // Where a box was taken out from between two blocks, they keep the gap of the blank line that stood
  // around it, and do not close up to the gap between blocks that were written together.
  const gapped = new Set<Block>();
  if (apart?.size) all.forEach((block, at) => !apart.has(at) && at > 0 && apart.has(at - 1) && blocks[0] !== block && gapped.add(block));
  const noted = notesOf(blocks);
  const drawn = (spans: Span[]) => drawnOf(spans, own, record, noted);
  // Words and nothing else stay in the line they are put in, as they always have.
  const only = blocks.length === 1 ? blocks[0] : undefined;
  if (only?.is === 'text' && noted.list.length === 0) return <span>{drawn(only.spans)}</span>;
  // What stands at the side has the words after it go round it, and nothing after the text.
  const aside = blocks.some((block) => block.is === 'figure' && block.place === 'side');
  // An article's headings, each with what stands beside it, in the order they stand.
  const headings = beside ? headingsOf(blocks) : [];
  let nth = 0;
  const atOf = (block: Block): HeadingAt | undefined => {
    const heading = block.is === 'heading' && beside ? headings[nth++] : undefined;
    return heading && beside ? { heading, beside: beside(heading) } : undefined;
  };
  return (
    <>
      {blocks.map((block, index) => (
        <Fragment key={index}>
          {gapped.has(block) && <div class="prose-gap" aria-hidden="true" />}
          {drawnBlock(block, drawn, record, atOf(block))}
        </Fragment>
      ))}
      {aside && <div class="prose-after-side" aria-hidden="true" />}
      {notes === 'here' && noted.list.length > 0 && <NotesList notes={noted} drawn={(note) => drawn(note.spans ?? [])} />}
    </>
  );
}

/** A card's id, pressed to see the card. */
function CardLink({ card, project, children }: { card: string; project: string; children: ComponentChildren }) {
  return (
    <button type="button" class="card-link" title={`Show ${card}`} onClick={() => showCard(card, project)}>
      {children}
    </button>
  );
}

/** Spans as they are drawn: a link to a page as `views/pagelink.tsx` draws it, a footnote's marker as `views/cited.tsx` does, a picture as `views/pictured.tsx` draws it, somebody named a mention, a card's id a card, a file of a project (of `own`, or one named before it) a link to it, and the rest as it is marked. */
function drawnOf(spans: Span[], own: string | null, record: string | null, noted: Notes): ComponentChildren[] {
  // The projects that have a board: a card is one of theirs.
  const boards = projects.value.filter((project) => hasBoard(project.id)).map((project) => project.id);
  const cards = (text: string) => cardsIn(text, boards).map((part) => (typeof part === 'string' ? part : <CardLink {...part}>{part.card}</CardLink>));
  // Somebody named with `@` is drawn as who it names, and the words around them as the rest are.
  const named = (text: string) => withMentions(text, cards);
  const drawn = (inside: Span[]) => drawnOf(inside, own, record, noted);
  return spans.flatMap((span): ComponentChildren[] => {
    if (span.is === 'text') return withPaths(span.text, own, named);
    if (span.is === 'code') {
      // An id written as code, as agents write them, is still the card.
      const [only, ...rest] = cardsIn(span.text, boards);
      if (rest.length === 0 && only && typeof only !== 'string') {
        return [
          <CardLink {...only}>
            <code>{span.text}</code>
          </CardLink>,
        ];
      }
      // And a path written as code, as it usually is, is still the file.
      return [<code>{withPaths(span.text, own)}</code>];
    }
    if (span.is === 'bold') return [<strong>{drawn(span.inside)}</strong>];
    if (span.is === 'italic') return [<em>{drawn(span.inside)}</em>];
    if (span.is === 'underline') return [<u>{drawn(span.inside)}</u>];
    if (span.is === 'strike') return [<s>{drawn(span.inside)}</s>];
    if (span.is === 'image') return [<Pictured picture={span} record={record} figure={false} />];
    if (span.is === 'page') {
      return [
        <PageLink topic={span.topic} reply={span.reply}>
          {drawn(span.inside)}
        </PageLink>,
      ];
    }
    if (span.is === 'cite') {
      const note = noted.byLabel.get(span.label);
      return note ? [<Cite note={note} source={note.spans ? drawn(note.spans) : null} />] : [];
    }
    if (span.is !== 'link') return [];
    // Somewhere else, so in a window of its own, which is told nothing of this one.
    return [
      <a href={span.to} target="_blank" rel="noopener noreferrer">
        {drawn(span.inside)}
      </a>,
    ];
  });
}

/**
 * What folds and unfolds a branch of a tree: a small box with a plus in it
 * while what is under it is folded away, and a minus while it is shown.
 * The lines of the tree run from box to box (`.tree` in the styles).
 */
export function Twist({ open }: { open: boolean }) {
  return (
    <span class={classes('twist', open && 'open')} aria-hidden="true">
      <svg viewBox="0 0 11 11">
        <rect x="0.5" y="0.5" width="10" height="10" rx="1.5" />
        <path d={open ? 'M3 5.5h5' : 'M3 5.5h5M5.5 3v5'} />
      </svg>
    </span>
  );
}
