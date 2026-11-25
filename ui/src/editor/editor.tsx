// The editor as a component. The editor itself is loaded on first use.

import { useEffect, useLayoutEffect, useRef } from 'preact/hooks';
import type { OnWords } from '../anchored.ts';
import type { Anchor } from '../anchors.ts';
import type { Diagnostic } from '../api.ts';
import type { World } from '../blueprint/complete.ts';
import { showEntry } from '../referenced.ts';
import { blueprintLanguage, drawer, partMissing } from '../store.ts';
import type { Editor } from './codemirror.ts';
import { isEcho, say } from './echo.ts';
import { answers, asks } from './finding.ts';
import type { Keyboard } from './finding.ts';
import { languageOfPath } from './language.ts';
import type { Language } from './language.ts';
import { keepSought, soughtIn } from './sought.ts';
import { useKeptRef } from '../views/returned.tsx';

export interface CodeEditorProps {
  value: string;
  language: Language;
  readOnly?: boolean;
  problems?: Diagnostic[];
  /** Go to this line; change `lineTick` to go there again. */
  line?: number;
  lineTick?: number;
  onChange?: (text: string) => void;
  onSave?: () => void;
  /** Asked for by its own key: lay the text out. */
  onFormat?: () => void;
  /** For a blueprint: what there is to offer that is in neither the language nor the text. */
  world?: () => World;
  /** For a document: the comments on a few of its words, marked where they are found, and a mark pressed. */
  marked?: OnWords[];
  /** The comments whose card is open, whose mark is drawn as the open one. */
  opened?: number[];
  onMark?: (seqs: number[], anchor: Anchor, box: DOMRect) => void;
}

export function CodeEditor(props: CodeEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<Editor | null>(null);
  const latest = useRef(props);
  latest.current = props;
  // What this editor has said its text is, until it is handed back.
  const said = useRef<string[]>([]);
  // Where its text was scrolled to, kept by its tab (`views/returned.tsx`): the frame keeps the rest of a view's.
  const scrolled = useKeptRef<unknown>(`editor.scrolled:${props.language}`, null);

  // Read as it is taken off the page, while it is still there to be read: what an effect cleans up after is done once
  // the page is drawn without it.
  useLayoutEffect(
    () => () => {
      if (editor.current) scrolled.current = editor.current.scrolled();
    },
    [],
  );
  useEffect(() => {
    let gone = false;
    void import('./codemirror.ts').then(({ createEditor }) => {
      if (gone || !host.current) return;
      const now = latest.current;
      // What is searched for is kept by the tab, which outlives the view.
      const tab = host.current.closest<HTMLElement>('[data-tab]')?.dataset.tab;
      editor.current = createEditor({
        parent: host.current,
        doc: now.value,
        language: now.language,
        readOnly: now.readOnly ?? false,
        onChange: (text) => {
          say(said.current, text);
          latest.current.onChange?.(text);
        },
        onSave: () => latest.current.onSave?.(),
        onFormat: () => latest.current.onFormat?.(),
        onMark: (seqs, anchor, box) => latest.current.onMark?.(seqs, anchor, box),
        blueprint: {
          language: blueprintLanguage,
          world: () => latest.current.world?.() ?? { blueprints: [], agents: [], devices: [] },
          reference: showEntry,
        },
        sought: {
          kept: () => (tab ? soughtIn(tab) : null),
          keep: (sought) => {
            if (tab) keepSought(tab, sought);
          },
        },
      });
      editor.current.setProblems(now.problems ?? []);
      if (now.marked) editor.current.mark(now.marked);
      if (now.opened) editor.current.opened(now.opened);
      // Where its text was scrolled to when its tab was last shown, put back by the editor itself, which knows how
      // tall each line is only once it has drawn it: a line it is asked to go to stands over that.
      if (scrolled.current !== null) editor.current.scrollTo(scrolled.current);
      if (now.line) editor.current.goToLine(now.line);
    }, partMissing);
    return () => {
      gone = true;
      editor.current?.destroy();
      editor.current = null;
    };
  }, []);

  // The keys that find are the editor's own while the keyboard is in its
  // text or its bar. A text that is only read cannot hold the keyboard, and
  // somebody looking at a page whose main thing is this text means it
  // wherever they last pressed: so the page hears those keys for as long as
  // this view is on it, and `answers` says which are this editor's. It is
  // asked as the key comes down, before the frame has put a drawer away.
  useEffect(() => {
    const mac = /Mac|iPhone|iPad/.test(navigator.platform);
    const onKey = (event: KeyboardEvent) => {
      const asked = asks(event, mac);
      const here = host.current;
      if (!asked || !here || !editor.current) return;
      const on = event.target instanceof HTMLElement ? event.target : null;
      const within = on?.closest('.cm-editor');
      const pane = here.closest<HTMLElement>('.view');
      const keyboard: Keyboard = within
        ? here.contains(within)
          ? 'here'
          : 'another'
        : !on || on === document.body
          ? 'nowhere'
          : on.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(on.tagName)
            ? 'field'
            : on.closest('.view') && on.closest('.view') !== pane
              ? 'pane'
              : 'page';
      const pressed = {
        keyboard,
        focused: pane?.dataset.focused === 'true',
        over: document.querySelector('[aria-modal="true"]') !== null,
        open: editor.current.finds(),
        drawer: drawer.value !== null,
      };
      if (!answers(asked, pressed)) return;
      event.preventDefault();
      event.stopPropagation();
      editor.current.find(asked);
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, []);

  // The text is put in when somebody else changed it, and never when it is
  // only the editor's own words coming back, which by then may be old.
  useEffect(() => {
    if (!isEcho(said.current, props.value)) editor.current?.replace(props.value);
  }, [props.value]);
  useEffect(() => editor.current?.setReadOnly(props.readOnly ?? false), [props.readOnly]);
  useEffect(() => editor.current?.setProblems(props.problems ?? []), [props.problems]);
  useEffect(() => editor.current?.mark(props.marked ?? []), [props.marked]);
  useEffect(() => editor.current?.opened(props.opened ?? []), [props.opened]);
  useEffect(() => {
    if (props.line) editor.current?.goToLine(props.line);
  }, [props.line, props.lineTick]);

  // It keeps where it was scrolled to itself, and the frame leaves it to (`views/returned.tsx`).
  return <div class="editor" ref={host} data-scrolls-itself />;
}

export function languageOf(path: string, blueprint: boolean): Language {
  return blueprint ? 'blueprint' : languageOfPath(path);
}
