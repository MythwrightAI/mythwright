// The text is read for its colours ahead of where it is shown.
//
// A language colours what it has parsed, and left to itself it parses
// little further than what is in view: scrolled quickly, lines came into
// view as bare text and were coloured a moment later. So the whole text is
// parsed while nothing else is asked of the page, as it is opened, as its
// language comes and after each change, a few milliseconds at a time so
// that typing never waits for it.
import { forceParsing } from '@codemirror/language';
import { ViewPlugin } from '@codemirror/view';
import type { EditorView, ViewUpdate } from '@codemirror/view';

/** How long one turn of reading ahead may take, in milliseconds. */
const TURN = 20;

type Waiting = { cancel: () => void };

/** Asks for `run` when the page has nothing else to do, or soon where the browser cannot say when that is. */
function whenIdle(run: () => void): Waiting {
  if (typeof requestIdleCallback === 'function') {
    const id = requestIdleCallback(run, { timeout: 200 });
    return { cancel: () => cancelIdleCallback(id) };
  }
  const id = setTimeout(run, 16);
  return { cancel: () => clearTimeout(id) };
}

export const readAhead = ViewPlugin.fromClass(
  class {
    waiting: Waiting | null = null;
    view: EditorView;

    constructor(view: EditorView) {
      this.view = view;
      this.ask();
    }

    update(update: ViewUpdate) {
      if (update.docChanged || update.transactions.some((tr) => tr.reconfigured)) this.ask();
    }

    ask() {
      if (this.waiting) return;
      this.waiting = whenIdle(() => {
        this.waiting = null;
        // Done when the whole text is parsed; otherwise another turn.
        if (!forceParsing(this.view, this.view.state.doc.length, TURN)) this.ask();
      });
    }

    destroy() {
      this.waiting?.cancel();
      this.waiting = null;
    }
  },
);
