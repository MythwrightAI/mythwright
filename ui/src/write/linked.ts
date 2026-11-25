// Links to other pages in the block editor: `[[` typed offers the pages by
// title, as `@` offers people in a reply's box, and the page chosen is put
// in as a link to its id (`pagelinks.ts`), with its title as the words;
// and a link to a page, pressed, opens it, as one drawn on the page does.
// Only a topic's documents link so (`linking`). In a block of code that is
// an article's box (`boxes.ts`), or a table kept as its text, where nothing
// is marked, the link is put in as the text it is written as.

import type { EditorState } from 'prosemirror-state';
import { Plugin, PluginKey } from 'prosemirror-state';
import type { EditorView } from 'prosemirror-view';
import { isBoxLanguage } from '../boxes.ts';
import { linkWords, pageAddress, pageLinkOf, pagesOffered, pageTyping, writtenLink } from '../pagelinks.ts';
import type { Offered } from '../pagelinks.ts';
import { openTab } from '../store.ts';
import { topics, wantTopics } from '../topicsread.ts';
import { showReply } from '../topicshown.ts';
import { treeOfLines } from '../wikipage.ts';
import { schema } from './markdown.ts';

/** What the page tells the editor of the pages it links to: the page being written, which is not offered. */
export interface Linking {
  page: string | null;
}

/** Whether the cursor is in a block of code that is an article's box, or in a table, whose links are their text. */
const inBox = (state: EditorState): boolean => {
  const { parent } = state.selection.$from;
  if (parent.type === schema.nodes.pipe_table) return true;
  return Boolean(parent.type.spec.code) && isBoxLanguage(String(parent.attrs.params ?? '').trim().toLowerCase());
};

/** The `[[` being typed before the cursor, in a line of words or a box: where it begins in the document, and what was typed after it. */
function asking(state: EditorState): { from: number; typed: string } | null {
  const { $from, empty } = state.selection;
  if (!empty || !$from.parent.isTextblock || ($from.parent.type.spec.code && !inBox(state))) return null;
  // Each thing in a line that is not words (a marker, a picture) is one place long, as its stand-in here is.
  const before = $from.parent.textBetween(0, $from.parentOffset, undefined, '￼');
  const found = pageTyping(before, before.length);
  return found ? { from: $from.start() + found.from, typed: found.typed } : null;
}

/** Opens what a link to a page leads to: the page, or the page at its reply. */
export function openLinked(href: string): boolean {
  const link = pageLinkOf(href);
  if (!link) return false;
  if (link.reply !== null) showReply(link.topic, link.reply);
  else openTab({ kind: 'topic', id: link.topic, title: topics.peek().find((line) => line.id === link.topic)?.title });
  return true;
}

const key = new PluginKey('page links');

export function pageLinks(host: HTMLElement, linking: Linking): Plugin {
  wantTopics();
  const box = document.createElement('div');
  box.className = 'block-menu page-menu';
  box.setAttribute('role', 'listbox');
  box.setAttribute('aria-label', 'Link a page');
  box.hidden = true;
  host.append(box);
  let chosen = 0;
  let shown: Offered[] = [];
  let putAway = -1;

  const pick = (view: EditorView, offered: Offered) => {
    const now = asking(view.state);
    if (!now) return;
    const { page } = offered;
    const link = schema.marks.link.create({ href: pageAddress(page.id), title: null });
    const at = view.state.selection.from;
    // What closed it, typed before it was chosen, goes with it.
    const closed = view.state.doc.textBetween(at, Math.min(at + 2, view.state.selection.$from.end())) === ']]' ? 2 : 0;
    const tr = inBox(view.state)
      ? view.state.tr.replaceWith(now.from, at + closed, schema.text(writtenLink(page.title, page.id)))
      : view.state.tr.replaceWith(now.from, at + closed, schema.text(linkWords(page.title), [link])).removeStoredMark(link);
    view.dispatch(tr.scrollIntoView());
    view.focus();
  };

  const draw = (view: EditorView) => {
    const now = asking(view.state);
    if (!now || now.from !== putAway) putAway = now ? putAway : -1;
    const lines = topics.peek();
    shown = now && now.from !== putAway ? pagesOffered(lines, treeOfLines(lines), now.typed, linking.page) : [];
    box.hidden = shown.length === 0;
    if (box.hidden) return;
    chosen = Math.min(chosen, shown.length - 1);
    box.replaceChildren(
      ...shown.map((offered, index) => {
        const item = document.createElement('button');
        item.type = 'button';
        item.setAttribute('role', 'option');
        item.setAttribute('aria-selected', String(index === chosen));
        item.className = index === chosen ? 'chosen' : '';
        item.innerHTML = '<strong></strong><span></span>';
        item.firstElementChild!.textContent = offered.page.title;
        item.lastElementChild!.textContent = offered.wiki ? `in ${offered.wiki}` : 'a page';
        // Before the editor loses the cursor to the click.
        item.addEventListener('mousedown', (event) => {
          event.preventDefault();
          pick(view, offered);
        });
        return item;
      }),
    );
    const at = view.coordsAtPos(now!.from);
    const within = host.getBoundingClientRect();
    box.style.left = `${Math.max(0, Math.min(at.left - within.left, within.width - 260))}px`;
    box.style.top = `${at.bottom - within.top + host.scrollTop + 6}px`;
  };

  return new Plugin({
    key,
    view: (view) => {
      draw(view);
      return { update: draw, destroy: () => box.remove() };
    },
    props: {
      handleKeyDown(view, event) {
        if (shown.length === 0) return false;
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          chosen = (chosen + (event.key === 'ArrowDown' ? 1 : shown.length - 1)) % shown.length;
          draw(view);
          return true;
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
          pick(view, shown[chosen]!);
          return true;
        }
        if (event.key === 'Escape') {
          putAway = asking(view.state)?.from ?? -1;
          draw(view);
          return true;
        }
        return false;
      },
      handleDOMEvents: {
        // A link to a page, pressed, opens it: words in it are changed from the keyboard, as in a page's mention.
        click(_view, event) {
          if (event.button !== 0) return false;
          const link = (event.target as Element | null)?.closest?.('a[href^="topic:"]');
          if (!link || !openLinked(link.getAttribute('href') ?? '')) return false;
          event.preventDefault();
          return true;
        },
      },
    },
  });
}
