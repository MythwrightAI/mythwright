// Which icons the client is built with.
//
// A table's column may be drawn with an icon of somebody's choosing, and
// the drawings there are to choose from are a set that is read when the
// client is built and never fetched. Not all of a set is offered: the
// application names no agent's program and no service that hosts source
// (the `providers` gate), so an icon that is such a name's, or is found
// by one, is left out of what is built, and no brand's mark is offered.
// Which names those are is read where the gate reads them, the packages
// under `extensions/` and the list kept beside them, and written nowhere
// in the client.

/** One part of a drawing: what it is, and what it is drawn with. */
export type IconNode = [string, Record<string, string>];

/** What is built: each icon by its name, its drawing and the words it is found by. */
export type Built = Record<string, [IconNode[], string]>;

/** What a line of a package's manifest says for a key, as the gate reads it. */
function manifestValue(text: string, key: string): string | null {
  const line = text.split('\n').find((one) => one.trimStart().startsWith(`${key}:`));
  if (!line) return null;
  const value = line
    .slice(line.indexOf(':') + 1)
    .trim()
    .replace(/^["']+|["']+$/g, '');
  return value === '' ? null : value.toLowerCase();
}

/**
 * The names the application may not spell: what each package is called
 * and the program it runs, the folders the packages are in, and what is
 * kept for packages not written yet.
 */
export function reserved(folders: string[], manifests: string[], kept: string): string[] {
  const names = new Set<string>(folders.map((folder) => folder.toLowerCase()));
  for (const manifest of manifests) {
    for (const key of ['extension', 'program']) {
      const value = manifestValue(manifest, key);
      if (value) names.add(value);
    }
  }
  for (const line of kept.split('\n')) {
    const name = line.trim();
    if (name !== '' && !name.startsWith('#')) names.add(name.toLowerCase());
  }
  return [...names].sort();
}

/** A name shorter than this is looked for as a word of its own, as the gate looks: two letters are inside half the words there are. */
const WHOLE_WORD_UNDER = 4;

/** Whether words, in lowercase, spell `name`. */
export function spells(words: string, name: string): boolean {
  const letter = (one: string | undefined) => one !== undefined && /[\p{L}\p{N}]/u.test(one);
  for (let at = words.indexOf(name); at >= 0; at = words.indexOf(name, at + 1)) {
    if (name.length >= WHOLE_WORD_UNDER) return true;
    if (!letter(words[at - 1]) && !letter(words[at + name.length])) return true;
  }
  return false;
}

/**
 * The icons of a set that are built, and the names of those left out: an
 * icon whose own name or any word it is found by spells a name the
 * application may not is no part of the client.
 */
export function built(nodes: Record<string, IconNode[]>, tags: Record<string, string[]>, names: string[]): { icons: Built; left: string[] } {
  const icons: Built = {};
  const left: string[] = [];
  for (const name of Object.keys(nodes).sort()) {
    const words = (tags[name] ?? []).map((word) => word.toLowerCase());
    if ([name.toLowerCase(), ...words].some((word) => names.some((one) => spells(word, one)))) left.push(name);
    else icons[name] = [nodes[name]!, words.join(',')];
  }
  return { icons, left };
}

/**
 * The module the client loads its icons from: the set's notice, kept
 * where a build keeps such comments, and the icons. A notice that would
 * end the comment it is written in is not written as it is.
 */
export function moduleOf(icons: Built, notice: string): string {
  return `/*!\n${notice.replaceAll('*/', '* /')}\n*/\nexport const icons = ${JSON.stringify(icons)};\n`;
}
