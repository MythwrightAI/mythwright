// Which view of a table each reader looks at: theirs, kept in this browser
// (`client.view.<project>.<table>`, as it always was), and said here as
// well, so that what shows a table follows it and so does the address
// bar (`address.ts`: an address says the view after its `#`, `#newest`),
// and an address opened at a view shows that view in a page that is
// already open.

import { signal } from '@preact/signals';

const asked = signal<Record<string, string>>({});
const keyOf = (project: string, table: string) => `client.view.${project}.${table}`;

/** The view of a table this reader looks at, by its name; nothing where they never chose one. */
export function viewShown(project: string, table: string): string | null {
  const key = keyOf(project, table);
  const here = asked.value[key];
  if (here !== undefined) return here;
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/** Looks at a view of a table, by its name: kept for this reader, and drawn wherever the table is. */
export function showView(project: string, table: string, name: string): void {
  const key = keyOf(project, table);
  if (asked.peek()[key] !== name) asked.value = { ...asked.peek(), [key]: name };
  try {
    localStorage.setItem(key, name);
  } catch {
    /* not remembered */
  }
}
