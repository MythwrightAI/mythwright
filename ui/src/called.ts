// What a project and a table are called for people, as the page says it
// and as a dialog asks for another: a project's name, and a table's title
// beside its name. What everything else knows either by (a project's id,
// a table's name) never changes; only what people read it as does. The
// daemon holds the same rules (`studio::projects::named`,
// `studio::shown::titled`) and says each refusal in its own sentence; the
// page says these first, before anything is asked.
//
// Holds nothing of the page, so that its tests can read it.

import { titled } from './table.ts';

/** The most letters a project's name has, as the daemon has it. */
export const LONGEST_PROJECT = 80;
/** The most letters a table's title has, as the daemon has it. */
export const LONGEST_TITLE = 80;

const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;
const letters = (text: string) => [...text].length;

/** What a table is called for people: what somebody called it, else what its blueprint says, else its name as it reads. */
export function tableCalled(table: { name: string; title?: string | null; title_chosen?: string | null }): string {
  return table.title_chosen || table.title || titled(table.name);
}

/** What a project is to be called, or why not: never nothing, one line, and no other project's name whatever its capitals. */
export function projectNamed(typed: string, id: string, projects: { id: string; name: string }[]): { name: string } | { wrong: string } {
  const name = typed.trim();
  if (name === '') return { wrong: 'A project is called something: say what it is to be called.' };
  if (CONTROL.test(name)) return { wrong: "A project's name is one line of words." };
  if (letters(name) > LONGEST_PROJECT) return { wrong: `A project's name is ${LONGEST_PROJECT} letters at most, and this one is ${letters(name)}.` };
  const other = projects.find((one) => one.id !== id && one.name.trim().toLowerCase() === name.toLowerCase());
  if (other) return { wrong: `Another project is called ${name}: call this one something else.` };
  return { name };
}

/** What a table is to be called, or why not; nothing (`null`), where it is to be called by its name again. */
export function tableTitled(typed: string): { title: string | null } | { wrong: string } {
  const title = typed.trim();
  if (CONTROL.test(title)) return { wrong: 'What a table is called is one line of words.' };
  if (letters(title) > LONGEST_TITLE) return { wrong: `What a table is called is ${LONGEST_TITLE} letters at most, and this is ${letters(title)}.` };
  return { title: title === '' ? null : title };
}
