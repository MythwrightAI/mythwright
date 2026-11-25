// A file of a project named in what somebody wrote, drawn as a link on
// the words that name it (what is a path and which
// file it names is `pathlinked.ts`, what the page knows of it
// `pathsknown.ts`). Pressed, it opens the file as a tab, as the Repo page
// does; its address is the file's, so that it is opened in a page of its
// own as any address of the factory is.
//
// Which project words are of is said by what draws them (`WordsOf`: a
// channel's, a record's), and a path is looked for in no other unless
// the words name one before it (`shop:docs/x.md`).

import { createContext } from 'preact';
import type { ComponentChildren } from 'preact';
import { useContext, useEffect } from 'preact/hooks';
import { asked, pathPieces, projectOfId } from '../pathlinked.ts';
import type { File } from '../pathlinked.ts';
import { ask, knownFor } from '../pathsknown.ts';
import { usePeople } from '../peopleknown.ts';
import { openTab, projects } from '../store.ts';
import { addressOf } from '../tab.ts';
import { withMentions } from './mentioned.tsx';

/** The project the words drawn under it are of; none where they are of no project. */
export const WordsOf = createContext<string | null>(null);

/** A file named in the words, pressed to open it as a tab. */
export function PathLink({ file, children }: { file: File; children: ComponentChildren }) {
  const tab = { kind: 'file', project: file.project, path: file.path } as const;
  return (
    <a
      class="path-link"
      href={addressOf(tab) ?? undefined}
      title={`Open ${file.path}`}
      data-path={file.path}
      onClick={(event) => {
        // Pressed as a link is pressed to open it elsewhere, it is the browser's.
        if (event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
        event.preventDefault();
        openTab(tab);
      }}
    >
      {children}
    </a>
  );
}

/**
 * Words with each file they name a link, and the rest as `rest` draws it
 * (as they are, unless said). Reads what the page knows as it is drawn,
 * so a path the daemon has since said is a file is drawn as one.
 */
export function withPaths(words: string, own: string | null, rest: (text: string) => ComponentChildren = (text) => text): ComponentChildren[] {
  return pathPieces(words, knownFor(own)).map((piece) =>
    piece.file ? (
      <PathLink file={piece.file}>{piece.text}</PathLink>
    ) : (
      rest(piece.text)
    ),
  );
}

/** Words as they are, with each file they name a link; and, where somebody wrote them (`named`), each person or agent named with `@` a mention. */
export function PathsIn({ text, named }: { text: string; named?: boolean }) {
  // Who a name is, read once for the page.
  usePeople();
  return <>{withPaths(text, useContext(WordsOf), named ? (words) => withMentions(words) : undefined)}</>;
}

/**
 * Asks the daemon about the paths of something somebody wrote that the
 * page knows nothing of yet, once as it is drawn: a few at most
 * (`asked`), each once for the page.
 */
export function useAskedPaths(text: string): void {
  const own = useContext(WordsOf);
  useEffect(() => ask(asked(text, knownFor(own))), [text, own]);
}

/** What somebody wrote, as it is, with each file it names a link and each person or agent it names a mention: the paths the page knows nothing of are asked about as it is drawn. */
export function Written({ text }: { text: string }) {
  useAskedPaths(text);
  return <PathsIn text={text} named />;
}

/** The words under it as of the project a record or a channel is of, by its id: the project its id begins with. */
export function OfRecord({ record, children }: { record: string; children: ComponentChildren }) {
  return <WordsOf.Provider value={projectOfId(record, projects.value.map((project) => project.id))}>{children}</WordsOf.Provider>;
}
