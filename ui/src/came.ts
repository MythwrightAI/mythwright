// A package that is installed and is not the one of its name that came
// with this version: what is said of it, to whom the way to take the one
// that came is offered, and what is asked before it is taken.
//
// A package is installed as a copy, and the daemon can only say that the
// copy differs from the one that came, and where. Whether the copy is an
// earlier one or one somebody changed is known to nobody, so nothing here
// says "newer": it says that they differ, and that taking the one that
// came replaces the copy whatever was changed in it.

/** What the daemon says of a package that differs from the one that came. */
export interface Differs {
  /** Where, in a sentence's worth of clauses that follow "the one that came". */
  words: string;
}

/** As much of a package as is asked about here. */
export interface Installed {
  name: string;
  title: string;
  /** Whether it is not the one of its name that came. A daemon from before says nothing. */
  behind?: boolean;
  differs?: Differs;
}

/** Said beside one that differs, in the few words a line of the side panel has room for. */
export const DIFFERS = 'Differs from the one that came';

/** Whether a package is installed and is not the one that came. */
export const isBehind = (extension: Installed | undefined): boolean => extension?.behind === true;

/** Whether somebody holding `scopes` is let take the one that came: what installing a package takes, which is the owner's. */
export const mayUpdate = (scopes: readonly string[] | undefined): boolean => (scopes ?? []).some((scope) => scope === 'admin' || scope === '*');

/** Where the one that came differs from the copy, as a sentence, or nothing where the daemon did not say. */
export function against(extension: Installed): string {
  const words = extension.differs?.words ?? '';
  return words ? `Against the installed copy, the one that came ${words}.` : '';
}

/** What a package's own page says of one that differs: that it does, where, and that nothing says which is the earlier. */
export function said(extension: Installed, product: string): string[] {
  return [
    `The installed copy is not the one that came with this version of ${product}.`,
    against(extension),
    'A package is installed as a copy, and nothing says whether this copy is an earlier one or one somebody changed.',
  ].filter(Boolean);
}

/** What is asked before the one that came is taken: what is replaced, where the two differ, and what becomes of the copy. */
export function asked(extension: Installed, product: string): string[] {
  return [
    `The installed copy of this package is replaced by the one that came with this version of ${product}.`,
    against(extension),
    'Changes made to the installed copy are lost from what is installed. The copy is kept once beside the new one, as extension.was.yaml in the package’s folder under the factory’s data.',
  ].filter(Boolean);
}
