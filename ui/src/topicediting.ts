// Which of a topic's documents (its post, the article, and its summary) are
// open to be written on its page, as the page keeps it by its tab, and what
// pressing Edit, a summary's own Edit, or opening the article at one of its
// headings comes to (`TopicPage`'s `edit`, `views/topicdoc.tsx`). It holds
// nothing of the page, so that its cases can read it.

/** A topic's documents that are written in place. */
export type DocName = 'body' | 'summary';

/**
 * Which of a topic's documents are open to be written, as its page keeps
 * it: each until it is saved or left. Each time one is opened by a press
 * (`asked` counts them) the cursor is put in the one said (`focus`), and
 * the article opened at a heading (`at`, the heading's slug: `contents.ts`)
 * has it at the start of that heading's section.
 */
export interface Editing {
  body?: boolean;
  summary?: boolean;
  at?: string | null;
  focus?: DocName;
  asked?: number;
}

/** What a document is handed of what is open to be written. */
export function openOf(editing: Editing, name: DocName): { open: boolean; focus?: number; at?: string | null } {
  return { open: editing[name] === true, focus: editing.focus === name ? editing.asked : undefined, at: name === 'body' ? (editing.at ?? null) : null };
}

/** What is open to be written once one document is opened by a press, or left. */
export function opened(editing: Editing, name: DocName, open: boolean, at: string | null = null): Editing {
  if (!open) return { ...editing, [name]: false, ...(name === 'body' ? { at: null } : {}) };
  return { ...editing, [name]: true, focus: name, asked: (editing.asked ?? 0) + 1, ...(name === 'body' ? { at } : {}) };
}

/**
 * What is open once Edit is pressed at the head, or the article is opened at
 * one of its headings (`at`, its slug): the article, the cursor in it where no heading
 * is said; and its summary with it where it has one and no heading is said.
 */
export function editAll(editing: Editing, at: string | null, summary: boolean): Editing {
  const body = opened(editing, 'body', true, at);
  return at === null && summary ? { ...body, summary: true } : body;
}
