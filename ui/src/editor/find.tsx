// Finding in the editor's text: the keys that ask for it, and the bar.
//
// The searching is the library's: its query, its state, its commands and
// what marks the matches. The bar is drawn here, and lies over the top
// right of the text without moving a line of it: opening it and putting
// it away change nothing of where the text is. Only a match that would
// lie under the bar with the text scrolled to its top, where no scrolling
// can bring it clear, is given room above the first line, and that room
// goes when the bar does.
//
// Nothing of it is in the editor until the bar is first asked for: an
// editor nobody searches in holds the keys below and no more, so no key
// typed in it is looked at by anything here.

import { completionStatus } from '@codemirror/autocomplete';
import { closeSearchPanel, findNext, findPrevious, getSearchQuery, openSearchPanel, replaceAll, replaceNext, search, searchPanelOpen, SearchQuery, setSearchQuery } from '@codemirror/search';
import { EditorSelection, Prec, StateEffect, StateField } from '@codemirror/state';
import type { EditorState, Extension, SelectionRange } from '@codemirror/state';
import { EditorView, keymap, runScopeHandlers } from '@codemirror/view';
import type { Command, Panel } from '@codemirror/view';
import { render } from 'preact';
import { signs } from '../views/signs.tsx';
import { counted, oneLine, placed, whichOf } from './finding.ts';
import type { Asked } from './finding.ts';
import { UNSOUGHT } from './sought.ts';
import type { Sought } from './sought.ts';

/** Where what is searched for is kept while the editor is away. */
export interface Keeping {
  kept: () => Sought | null;
  keep: (sought: Sought) => void;
}

/** What the bar has of its own: whether its second line is open, and the room it gave a match above the text. */
interface Own {
  replacing: boolean;
  /** How far the text was moved down, and what stands above its first line with that. */
  room: number;
  above: number | null;
}

const owns = StateEffect.define<Partial<Own>>();
const own = StateField.define<Own>({
  create: () => ({ replacing: false, room: 0, above: null }),
  update(value, tr) {
    for (const effect of tr.effects) if (effect.is(owns)) value = { ...value, ...effect.value };
    return value;
  },
  provide: (field) => EditorView.contentAttributes.from(field, (value): Record<string, string> => (value.above === null ? {} : { style: `padding-top: ${value.above}px` })),
});

/** No more matches than this are counted: past it the bar says there are more. */
const MOST = 10000;
/** What is left between the bar and a match under it, and between a match and the edge it is brought in from. */
const GAP = 6;
const LINES_ABOUT = 4;

interface Match {
  from: number;
  to: number;
}

const queryOf = (sought: Sought): SearchQuery => new SearchQuery({ ...sought, literal: true });
const soughtOf = (query: SearchQuery): Sought => ({ search: query.search, caseSensitive: query.caseSensitive, wholeWord: query.wholeWord, regexp: query.regexp, replace: query.replace });

/** Every match in the text, in order, as far as `MOST`. */
function all(state: EditorState, query: SearchQuery): { matches: Match[]; more: boolean } {
  const matches: Match[] = [];
  if (!query.valid) return { matches, more: false };
  const cursor = query.getCursor(state);
  for (let next = cursor.next(); !next.done; next = cursor.next()) {
    if (matches.length === MOST) return { matches, more: true };
    matches.push({ from: next.value.from, to: next.value.to });
  }
  return { matches, more: false };
}

/** The first match at or after a place, going round to the start of the text. */
function firstFrom(state: EditorState, query: SearchQuery, at: number): Match | null {
  for (const from of [at, 0]) {
    const next = query.getCursor(state, from).next();
    if (!next.done) return { from: next.value.from, to: next.value.to };
  }
  return null;
}

const barOf = (view: EditorView): HTMLElement | null => view.dom.querySelector<HTMLElement>('.finder');

/**
 * Gives a match room where the bar would lie over it whatever is
 * scrolled: it is in the first lines of the text, under where the bar is.
 * The text is moved down by as much as brings the match clear, and where
 * the text is scrolled that is taken up by the scrolling, so that nothing
 * is seen to move but what had to.
 */
function makeRoom(view: EditorView, range: SelectionRange): void {
  const bar = barOf(view);
  if (!bar) return;
  const within = (at: number) => Math.min(at, view.state.doc.length);
  const [from, to] = [within(range.from), within(range.to)];
  const scroller = view.scrollDOM.getBoundingClientRect();
  const over = bar.getBoundingClientRect();
  const kept = view.state.field(own);
  const above = kept.above ?? (parseFloat(getComputedStyle(view.contentDOM).paddingTop) || 0);
  const [start, end] = [view.coordsAtPos(from), view.coordsAtPos(to, -1)];
  // Where the match is with the text scrolled to its top: read where it is drawn, and worked out where it is not.
  const top = start ? start.top - scroller.top + view.scrollDOM.scrollTop : above + view.lineBlockAt(from).top;
  const clear = over.bottom - scroller.top + GAP;
  if (top >= clear) return;
  // On one line and wholly to the left of the bar, it is beside it and not under it.
  if (start && end && Math.abs(start.top - end.top) < 1 && end.right < over.left - GAP) return;
  const more = Math.ceil(clear - top);
  view.dispatch({ effects: owns.of({ room: kept.room + more, above: above + more }) });
  view.scrollDOM.scrollTop += more;
}

/** Brings a match into view with a few lines about it, clear of the bar. */
function bring(range: SelectionRange, view: EditorView): StateEffect<unknown> {
  makeRoom(view, range);
  return EditorView.scrollIntoView(range, { y: 'nearest', yMargin: Math.min(LINES_ABOUT * view.defaultLineHeight, view.scrollDOM.clientHeight / 4) });
}

/** Searches for something else: the matches are marked as it is typed, and the first from where the caret is, is gone to. */
function seek(view: EditorView, sought: Sought, go: boolean): void {
  const query = queryOf(sought);
  const first = go && query.valid ? firstFrom(view.state, query, view.state.selection.main.from) : null;
  const selection = first ? EditorSelection.single(first.from, first.to) : undefined;
  view.dispatch({ effects: [setSearchQuery.of(query), ...(selection ? [bring(selection.main, view)] : [])], selection, userEvent: 'select.search' });
}

function Bar({ view, matches, more }: { view: EditorView; matches: Match[]; more: boolean }) {
  const query = getSearchQuery(view.state);
  const sought = soughtOf(query);
  const { replacing } = view.state.field(own);
  const writes = !view.state.readOnly;
  const { from, to } = view.state.selection.main;
  const said = counted({ asked: query.search !== '', reads: query.valid, matches: matches.length, more, at: whichOf(matches, from, to) });
  // What does nothing with no match says so and is still there to be pressed: a button that is taken away from under the
  // keyboard, as the last match is replaced, leaves the keyboard nowhere.
  const none = matches.length === 0;
  const flip = (which: 'caseSensitive' | 'wholeWord' | 'regexp') => () => seek(view, { ...sought, [which]: !sought[which] }, true);
  const turn = (which: 'caseSensitive' | 'wholeWord' | 'regexp', name: string, mark: string) => (
    <button type="button" class="ghost finder-switch" aria-pressed={sought[which]} aria-label={name} title={name} onClick={flip(which)}>
      <span class={`finder-${which}`}>{mark}</span>
    </button>
  );
  return (
    <>
      <div class="finder-line">
        <input
          class="finder-words"
          type="text"
          aria-label="Find"
          placeholder="Find"
          spellcheck={false}
          autocomplete="off"
          aria-invalid={query.regexp && query.search !== '' && !query.valid}
          value={query.search}
          onInput={(event) => seek(view, { ...sought, search: event.currentTarget.value }, true)}
        />
        {turn('caseSensitive', 'Match case', 'Aa')}
        {turn('wholeWord', 'Whole word', 'ab')}
        {turn('regexp', 'Regular expression', '.*')}
        <span class="finder-count" role="status" aria-live="polite">
          {said}
        </span>
        <button type="button" class="ghost icon" aria-label="Next match" title="Next match (Enter)" aria-disabled={none} onClick={() => findNext(view)}>
          {signs.below()}
        </button>
        <button type="button" class="ghost icon" aria-label="Previous match" title="Previous match (Shift Enter)" aria-disabled={none} onClick={() => findPrevious(view)}>
          {signs.above()}
        </button>
        <button type="button" class="ghost icon" aria-label="Close" title="Close (Esc)" onClick={() => close(view)}>
          {signs.cross()}
        </button>
      </div>
      {writes && (
        <button type="button" class="ghost icon finder-turn" aria-expanded={replacing} aria-label="Toggle replace" title="Replace" onClick={() => view.dispatch({ effects: owns.of({ replacing: !replacing }) })}>
          {signs.chevron()}
        </button>
      )}
      {writes && replacing && (
        <div class="finder-line">
          <input
            class="finder-with"
            type="text"
            aria-label="Replace with"
            placeholder="Replace with"
            spellcheck={false}
            autocomplete="off"
            value={query.replace}
            onInput={(event) => seek(view, { ...sought, replace: event.currentTarget.value }, false)}
          />
          <span class="finder-does">
            <button type="button" aria-disabled={none} title="Replace this match and go to the next (Enter)" onClick={() => replaceNext(view)}>
              Replace
            </button>
            <button type="button" aria-disabled={none} title="Replace every match" onClick={() => replaceAll(view)}>
              Replace all
            </button>
          </span>
        </div>
      )}
    </>
  );
}

function panel(view: EditorView, keeping: Keeping): Panel {
  const dom = document.createElement('div');
  dom.className = 'finder';
  dom.setAttribute('role', 'search');
  dom.setAttribute('aria-label', 'Find in this text');
  let found = all(view.state, getSearchQuery(view.state));
  const draw = () => render(<Bar view={view} matches={found.matches} more={found.more} />, dom);
  dom.addEventListener('keydown', (event) => {
    const field = event.target instanceof HTMLInputElement ? event.target : null;
    let done = true;
    if (event.key === 'Escape') close(view);
    else if (event.key === 'Enter' && field?.classList.contains('finder-words')) (event.shiftKey ? findPrevious : findNext)(view);
    else if (event.key === 'Enter' && field) replaceNext(view);
    else done = runScopeHandlers(view, event, 'search-panel');
    if (!done) return;
    // A key answered in the bar is the bar's alone: Escape here puts nothing else away.
    event.preventDefault();
    event.stopPropagation();
  });
  draw();
  // Where it stands is worked out from the editor's box and a drawer's, and again when the browser says either
  // is another size or a drawer has come or gone.
  const main = view.dom.closest('.main');
  const drawer = () => main?.querySelector<HTMLElement>(':scope > .drawer') ?? null;
  const put = () => {
    const holder = dom.parentElement;
    if (!holder) return;
    const box = view.dom.getBoundingClientRect();
    const beside = drawer();
    // Where the drawer is laid out, which is where it comes to rest: it is drawn on its way there as it opens.
    const edge = beside?.offsetParent ? beside.offsetParent.getBoundingClientRect().left + beside.offsetParent.clientLeft + beside.offsetLeft : null;
    const at = placed({ width: box.width, gutters: view.dom.querySelector<HTMLElement>('.cm-gutters')?.offsetWidth ?? 0, covered: edge === null ? 0 : box.right - edge });
    holder.style.right = `${at.right}px`;
    holder.style.maxWidth = `${at.widest}px`;
  };
  const sizes = new ResizeObserver(put);
  const watch = () => {
    sizes.disconnect();
    sizes.observe(view.dom);
    const beside = drawer();
    if (beside) sizes.observe(beside);
    put();
  };
  const drawers = new MutationObserver(watch);
  return {
    dom,
    top: true,
    mount() {
      if (main) drawers.observe(main, { childList: true });
      watch();
    },
    update(update) {
      const query = getSearchQuery(update.state);
      const asked = !query.eq(getSearchQuery(update.startState));
      if (asked) keeping.keep(soughtOf(query));
      if (asked || update.docChanged) found = all(update.state, query);
      if (update.transactions.length > 0) draw();
    },
    destroy() {
      sizes.disconnect();
      drawers.disconnect();
      render(null, dom);
    },
  };
}

const installed = (view: EditorView): boolean => view.state.field(own, false) !== undefined;

/** Whether what is selected in the text is a match of what is searched for: what going to a match selects. */
function atMatch(state: EditorState, query: SearchQuery): boolean {
  const { from, to } = state.selection.main;
  if (!query.valid || from === to) return false;
  const next = query.getCursor(state, from, to).next();
  return !next.done && next.value.from === from && next.value.to === to;
}

/**
 * Opens the bar, or goes to it: with what somebody selected in the text
 * where that is one line, and otherwise with what it last held, selected.
 * A match the search itself selected is not something somebody selected:
 * the bar that left the caret at one holds what found it, an expression
 * as it was written.
 */
function open(view: EditorView, keeping: Keeping, replacing = false): boolean {
  const first = !installed(view);
  if (first) view.dispatch({ effects: StateEffect.appendConfig.of([own, search({ top: true, literal: true, createPanel: (shown) => panel(shown, keeping), scrollToMatch: bring })]) });
  const was = first ? (keeping.kept() ?? UNSOUGHT) : soughtOf(getSearchQuery(view.state));
  const { from, to } = view.state.selection.main;
  const picked = atMatch(view.state, queryOf(was)) ? '' : oneLine(view.state.sliceDoc(from, to));
  const want = queryOf({ ...was, search: picked || was.search });
  openSearchPanel(view);
  const effects: StateEffect<unknown>[] = [];
  if (!getSearchQuery(view.state).eq(want)) effects.push(setSearchQuery.of(want));
  if (replacing && !view.state.field(own).replacing) effects.push(owns.of({ replacing: true }));
  if (effects.length > 0) view.dispatch({ effects });
  const words = barOf(view)?.querySelector<HTMLInputElement>('.finder-words');
  words?.focus();
  words?.select();
  return true;
}

/** Puts the bar away: the marks go, the text is where it was, and the keyboard is the text's again where it was the bar's. */
function close(view: EditorView): boolean {
  if (!searchPanelOpen(view.state)) return false;
  const { room } = view.state.field(own);
  // The library gives the keyboard back where the bar had it; where nothing has it, the text takes it too.
  const nowhere = view.root.activeElement === null || view.root.activeElement === document.body;
  closeSearchPanel(view);
  if (nowhere) view.focus();
  if (room > 0) {
    view.dispatch({ effects: owns.of({ room: 0, above: null }) });
    view.scrollDOM.scrollTop = Math.max(0, view.scrollDOM.scrollTop - room);
  }
  return true;
}

/** What the editor is given so that its text can be searched, and how the page asks the same of it. */
export function finding(keeping: Keeping): { extension: Extension; ask: (view: EditorView, asked: Asked) => void; shown: (view: EditorView) => boolean } {
  // A match is gone to with what is searched for, and where nothing is the bar is opened to ask.
  const step =
    (go: Command): Command =>
    (view) =>
      installed(view) && getSearchQuery(view.state).valid ? go(view) : open(view, keeping);
  const next = step(findNext);
  const before = step(findPrevious);
  const replace: Command = (view) => !view.state.readOnly && open(view, keeping, true);
  return {
    extension: [
      keymap.of([
        { key: 'Mod-f', run: (view) => open(view, keeping), scope: 'editor search-panel' },
        // Replacing is offered only where the text may be changed. A Mac keeps ⌘H for itself, so it has one key
        // for this and every other system two.
        { key: 'Mod-Alt-f', run: replace, scope: 'editor search-panel' },
        { key: 'Mod-h', mac: 'Mod-Alt-f', run: replace, scope: 'editor search-panel' },
        { key: 'F3', run: next, shift: before, scope: 'editor search-panel', preventDefault: true },
        { key: 'Mod-g', run: next, shift: before, scope: 'editor search-panel', preventDefault: true },
        { key: 'Escape', run: close },
      ]),
      // Escape in the text puts one thing away: what is offered there, and then this bar. The press is theirs
      // alone, so that whatever the page would put away by it (a drawer beside the text) waits for the next.
      Prec.highest(
        EditorView.domEventHandlers({
          keydown(event, view) {
            if (event.key === 'Escape' && (completionStatus(view.state) !== null || searchPanelOpen(view.state))) event.stopPropagation();
            return false;
          },
        }),
      ),
    ],
    ask(view, asked) {
      if (asked === 'find') open(view, keeping);
      else if (asked === 'away') close(view);
      else (asked === 'next' ? next : before)(view);
    },
    shown: (view) => searchPanelOpen(view.state),
  };
}
