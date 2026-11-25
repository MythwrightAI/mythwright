// A new blueprint of a project's own, made from the side panel's list of
// what it uses and from the gear of its page: a file where the factory
// keeps the project's own (`@blueprints/`), named as the blueprint is,
// which opens to be written in. Functions of plain values, with their
// cases in `ownmade.test.ts`; what is drawn is `views/ownblueprints.tsx`.

/** What a name comes to: the blueprint and its file, or why it cannot be one. */
export type Made = { name: string; path: string } | { wrong: string };

/**
 * The blueprint a name written in the dialog makes, in `written` (where
 * the project's own are written, as a path of the project's): the name
 * with its folders, `tools/ship`, as anything calls it; never one the
 * project has already, one that ships (that is forked from its page,
 * which keeps what it was forked from), or a name the daemon would not
 * read (a part that is empty, a dot, or one that begins with one).
 */
export function blueprintNamed(name: string, written: string, has: string[], ships: string[]): Made {
  const called = name.trim().replace(/\.bp$/, '');
  if (called === '') return { wrong: 'Give it a name.' };
  if (/\s/.test(called)) return { wrong: 'A name has no spaces: write tools/ship, or ship-it.' };
  if (called.includes('\\')) return { wrong: 'Folders are written with /, as in tools/ship.' };
  if (called.split('/').some((part) => part === '' || part.startsWith('.'))) return { wrong: 'Each part of a name is a word, and begins with no dot.' };
  if (called.startsWith('@')) return { wrong: 'A name begins with a letter or a digit.' };
  if (has.includes(called)) return { wrong: `The project has a blueprint called ${called} already.` };
  if (ships.includes(called)) return { wrong: `${called} ships with the factory: it is forked from its own page, which says what it was forked from.` };
  return { name: called, path: `${written}/${called}.bp` };
}

/** What a new blueprint says as it is made: its name and one step that does nothing, so that it checks and can be applied as it is. */
export function firstBlueprint(name: string): string {
  return `blueprint: ${name}\nsteps:\n  first: { do: noop }\n`;
}

/** What a file a blueprint links is called under it in the list: its path from the folder the blueprint is in. */
export function linkCalled(blueprint: string, link: string): string {
  const folder = blueprint.split('/').slice(0, -1).join('/');
  const within = link.replace(/^@blueprints\//, '');
  return folder && within.startsWith(`${folder}/`) ? within.slice(folder.length + 1) : within;
}
