// The page about the program, as functions of what
// it read, with nothing of the page in them so that their tests can read
// them: its tabs, which shows, which guide is read under Docs, where a
// link in a guide leads, and what is said of the program under Technical.
//
// Everything on the page came with the program and is handed by its
// daemon (`/api/about`): what it is for (`docs/about.md`), the guides
// marked public and where its data is kept, which only whoever may
// manage the factory is told. The program's name is never written here:
// it is what the daemon says it is called (`/api/meta`).
//
// A tab is known by a word, which is what the page keeps as the tab that
// shows and what its address says after the `#` (`/about#technical`); a
// guide read under Docs is `docs/<guide>` (`/about#docs/blueprints`).

/** The tabs of the page, in the order they are drawn. */
export const ABOUT_TABS = ['about', 'docs', 'technical'] as const;

export type AboutTab = (typeof ABOUT_TABS)[number];

/** What a tab is called: the first says what the program is, by the name it is called. */
export function tabCalled(tab: AboutTab, name: string): string {
  if (tab === 'about') return `What is ${name || 'this'}`;
  return tab === 'docs' ? 'Docs' : 'Technical';
}

/** A guide as the daemon lists it: by its name, and what it calls itself. */
export interface Guide {
  name: string;
  title: string;
}

/** What is asked for, by what an address says after its `#`: a tab, and under Docs a guide. Anything else is the first tab. */
export function asked(at: string | null | undefined): { tab: AboutTab; guide?: string } {
  const [first = '', ...rest] = (at ?? '').split('/');
  const guide = rest.join('/');
  if (first === 'docs') return guide ? { tab: 'docs', guide } : { tab: 'docs' };
  if (first === 'technical') return { tab: 'technical' };
  return { tab: 'about' };
}

/** What the address says after its `#` for a tab and the guide read under Docs: nothing for the first tab. */
export function atOf(tab: AboutTab, guide?: string | null): string {
  if (tab === 'docs') return guide ? `docs/${guide}` : 'docs';
  return tab === 'technical' ? 'technical' : '';
}

/** The guide read under Docs: the one chosen where it is one there is, else the first. Nothing while there are none. */
export function guideShown(guides: Guide[], chosen: string | null | undefined): Guide | null {
  return guides.find((one) => one.name === chosen) ?? guides[0] ?? null;
}

/**
 * The guide a link in a guide leads to (`blueprints.md`, `./views.md#a-table`):
 * one of those listed, by its file's name; nothing for anything else
 * (`../README.md`, which no copy of the program carries).
 */
export function guideOfLink(href: string, guides: Guide[]): string | null {
  const file = /^(?:\.\/)?([a-z0-9-]+)\.md(?:#.*)?$/i.exec(href.trim());
  if (!file) return null;
  return guides.some((one) => one.name === file[1]) ? file[1]! : null;
}

/** What the daemon hands of the program for the page. */
export interface Told {
  title?: string;
  html?: string;
  guides?: Guide[];
  /** The packages installed: told only to whoever may read them. */
  packages?: Package[];
  /** Where the factory keeps its data: told only to whoever may manage it. */
  data?: string;
}

/** The guides a reading tells of: only those that say their name and what they are called. */
export function guidesOf(read: Told | null | undefined): Guide[] {
  const listed = read?.guides;
  if (!Array.isArray(listed)) return [];
  return listed.flatMap((one) => (one && typeof one.name === 'string' && one.name && typeof one.title === 'string' ? [{ name: one.name, title: one.title || one.name }] : []));
}

/** One installed package as the Technical tab says it. */
export interface Package {
  name: string;
  title: string;
  /** The one of its name that came with this version, another than that one (`came.ts`), or one of somebody's own that came with none. */
  stands: 'came' | 'differs' | 'own';
}

/** One line of the Technical tab: what it is, and what it is. */
export interface Fact {
  label: string;
  said: string;
}

/** What is known of the program for the Technical tab. Whatever is not known, or not to be told the reader, is left out. */
export interface Known {
  version?: string;
  /** Which build of the client is being served. */
  build?: string | null;
  /** `installed` or `checkout`, as the daemon says where it runs from. */
  runsFrom?: string | null;
  /** The system of the device the daemon is on, and what it is built for. */
  os?: string | null;
  arch?: string | null;
  /** The factory's profile, where it has one. */
  profile?: string | null;
  data?: string | null;
}

/** The lines of what is technical of the program, as plain words, each only where it is known. */
export function facts(known: Known): Fact[] {
  const lines: Fact[] = [];
  if (known.version) lines.push({ label: 'Version', said: known.version });
  if (known.build) lines.push({ label: 'Client build', said: known.build });
  if (known.runsFrom === 'installed') lines.push({ label: 'Runs from', said: 'a copy on its own' });
  else if (known.runsFrom === 'checkout') lines.push({ label: 'Runs from', said: 'the checkout it was built in' });
  const system = [known.os, known.arch].filter(Boolean).join(' ');
  if (system) lines.push({ label: 'System', said: system });
  if (known.profile) lines.push({ label: 'Profile', said: known.profile });
  if (known.data) lines.push({ label: 'Data folder', said: known.data });
  return lines;
}

/** The packages a reading tells of, or nothing where the reader was not told of them. */
export function packagesOf(read: Told | null | undefined): Package[] | null {
  const listed = read?.packages;
  if (!Array.isArray(listed)) return null;
  return listed.flatMap((one) =>
    one && typeof one.name === 'string' && ['came', 'differs', 'own'].includes(one.stands) ? [{ name: one.name, title: typeof one.title === 'string' && one.title ? one.title : one.name, stands: one.stands }] : [],
  );
}

/** What is said beside an installed package: whether it is the one that came with this version. */
export function packageSaid(one: Package): string {
  if (one.stands === 'differs') return 'Differs from the one that came';
  return one.stands === 'came' ? 'The one that came with this version' : 'Installed from its own manifest';
}
