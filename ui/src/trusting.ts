// What a project's page offers of trusting the project, and what it says
// before it does.
//
// A project made of a folder that was already there is not trusted: a file
// of its own called after a blueprint that ships is passed over for the
// one that ships (*Identity* in `docs/architecture.md`). Trusting it, and
// taking that back, is the daemon's to do (`POST /api/projects/{id}/trust`)
// for whoever manages the factory (`admin`). This is the line the gear of
// the project's page draws for it and the question asked before, as plain
// values: it holds nothing of the page, so that its tests can read it.

import { covers } from './holds.ts';

/** Trusting a project, or taking that back: the gear's line, the question it opens and what is said once it is done. */
export interface Trusting {
  /** What the project is to be: trusted, or not. */
  to: boolean;
  /** The line of the gear's menu. */
  line: string;
  /** The question's heading. */
  asks: string;
  /** What it does, in plain sentences. */
  says: string;
  /** The button that does it. */
  button: string;
  /** Said once the daemon has done it. */
  done: string;
}

/**
 * What the gear of a project's page offers of trusting it, for whoever is
 * signed in: trusting one that is not trusted, and taking it back from one
 * that is, each only to whoever manages the factory, which is what the
 * daemon asks. Nothing for anybody else, nor where the daemon says nothing
 * of whether the project is trusted (a program from before).
 */
export function trusting(project: { name: string; trusted?: boolean }, scopes: readonly string[]): Trusting | null {
  if (typeof project.trusted !== 'boolean' || !covers([...scopes], 'admin')) return null;
  const name = project.name;
  if (!project.trusted)
    return {
      to: true,
      line: 'Trust this project…',
      asks: `Trust ${name}?`,
      says: `Its own files named like a blueprint that ships will be used in place of the one that ships. Anyone who can write to its folder can then change what those blueprints do.`,
      button: 'Trust it',
      done: `${name} is trusted.`,
    };
  return {
    to: false,
    line: 'Stop trusting…',
    asks: `Stop trusting ${name}?`,
    says: `The blueprints that ship will be used again in place of its own files of the same name. Its files are left as they are.`,
    button: 'Stop trusting',
    done: `${name} is no longer trusted.`,
  };
}
