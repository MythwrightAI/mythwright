// What a blueprint's page offers under Testing, in its head (`views/testing.tsx`),
// as words: which lines there are, what each says it does and why one is
// closed, before anything is drawn or done.
//
// The head was a row of buttons for a blueprint that ships (Stop, Fork,
// Preview, Sandbox test, Side by side, Compare ways, Apply changes), which
// was too many: what tries a text
// without changing what runs is one button, Testing, and a menu of four
// lines, each with a few words on what it does under it. This holds nothing
// of the page, so that its tests can read it: a line says what pressing it
// does (`does`), and whoever draws the menu does that.

/** What a line of Testing opens. */
export type Tried = 'preview' | 'sandbox' | 'beside' | 'compare';

/** A line of Testing, as words: what it is called, what it does, why it cannot be pressed where it cannot. */
export interface TestingLine {
  label: string;
  /** Its word on what it does, read whole under it. */
  note: string;
  /** Why it is not pressed as things stand; nothing where it is. */
  closed?: string;
  does: Tried;
}

/** How the page stands, as far as Testing is concerned. */
export interface Standing {
  /** Whoever reads may read blueprints: what plays or runs a text in a sandbox is theirs only then. */
  reads: boolean;
  /** The page is of something running, and not of a text: only its preview is offered. */
  instance: boolean;
  /** Its text has been checked. */
  checked: boolean;
  /** How many errors the check found. */
  errors: number;
  /** The name it calls itself, once it checks; a comparison is of a blueprint by its name. */
  named: boolean;
  /** Why it is not previewed, in the check's own words, where it does not check. */
  unpreviewed: string | null;
  /** Its preview is showing in the drawer now. */
  previewing: boolean;
}

/** What each line says it does: one line each, read under its name. */
export const SAYS: Record<Tried, string> = {
  preview: 'Plays what it does on its graph; runs nothing.',
  sandbox: 'Runs this text in a factory of its own, with stand-ins.',
  beside: 'The same played against each text there is of it.',
  compare: 'Splits real work between ways in this factory, and counts what each comes to.',
};

/**
 * The lines of Testing, in their one order: Preview, Sandbox test, Side by
 * side, Compare on real work. A line whoever reads may not use is not
 * there; one that cannot be pressed as things stand says why. Nothing at
 * all where none is offered: then there is no Testing.
 */
export function testingLines(standing: Standing): TestingLine[] {
  const { reads, instance, checked, errors, named, unpreviewed, previewing } = standing;
  const lines: TestingLine[] = [];
  // A line that is closed says why; one that is not says nothing of it.
  const line = (label: string, does: Tried, closed: string | null, note = SAYS[does]): TestingLine => (closed ? { label, note, closed, does } : { label, note, does });
  if (reads) {
    const why = !checked ? 'Once its text has been checked.' : unpreviewed ? `It is previewed once its text checks: ${unpreviewed.replaceAll('`', '')}` : null;
    lines.push(line('Preview', 'preview', why, previewing ? 'Showing beside the page: pressed, it is put away.' : SAYS.preview));
  }
  if (reads && !instance) lines.push(line('Sandbox test', 'sandbox', null), line('Side by side', 'beside', null));
  if (!instance) lines.push(line('Compare on real work', 'compare', !checked || !named || errors > 0 ? 'Once its text checks.' : null));
  return lines;
}
