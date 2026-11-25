// A project's environments as the client shows them (`dev`, `staging`,
// `prod`: named sets of variables and secrets the project says for
// itself, `docs/environments.md`), worked out from what the daemon
// answers and nothing else: the page, its table and what may be done
// there. No value of a secret is ever in what the daemon answers, and so
// is in nothing here.

import type { Acted, Rules } from './envrules.ts';
import type { Last, Ran } from './envrun.ts';
import { covers } from './holds.ts';

/** A variable as the daemon answers it: a plain value, and whether an agent is handed it. */
export interface Variable {
  value: string | number | boolean;
  agents: boolean;
}

/** A secret as the daemon answers it: never its value. */
export interface Secret {
  set: number;
  by_name: string;
  /** The variable an agent working under the environment is handed it as; nothing for none. */
  agents: string | null;
  /** Whether the factory keeps a value for it: a copy of the factory carries none. */
  kept?: boolean;
}

/** One environment of a project, as the daemon answers it. */
export interface Environment {
  name: string;
  vars: Record<string, Variable>;
  /** Its secrets by name, where whoever asks may see their names; nothing where they may not. */
  secrets?: Record<string, Secret>;
  made: number;
  by_name: string;
  /** The blueprint it is loaded by, for a branch; nothing where it says none, or to an agent. */
  deploy?: string | null;
  /** What it was last loaded with, where it ever was. */
  ran?: Ran | null;
  /** What it last ran, however that was started, in the daemon's words. */
  last?: Last;
  /** What it holds to: who approves what is started under it, and which branches run under it. */
  rules?: Rules;
  /** The runs that acted under it, the newest first: nothing where whoever reads may not be told what runs. */
  runs?: Acted[];
}

/** What is said of each environment in the page's table. */
export interface Row {
  name: string;
  vars: number;
  /** How many secrets it lists; nothing where whoever looks may not see their names. */
  secrets: number | null;
  /** How many of its variables and secrets an agent working under it is handed. */
  agents: number;
  /** How many things that are running run under it. */
  running: number;
}

/** Something running, as much of it as the table goes by. */
export interface Running {
  id?: string;
  environment?: string;
}

/** The table of a project's environments: one row each, in the daemon's order, with how many run under each. */
export function rowsOf(environments: Environment[], running: Running[]): Row[] {
  return environments.map((environment) => {
    const secrets = environment.secrets ? Object.values(environment.secrets) : null;
    const agents = Object.values(environment.vars).filter((one) => one.agents).length + (secrets ?? []).filter((one) => one.agents).length;
    return {
      name: environment.name,
      vars: Object.keys(environment.vars).length,
      secrets: secrets ? secrets.length : null,
      agents,
      running: running.filter((one) => one.environment === environment.name).length,
    };
  });
}

/** A count in words: `none`, `1 variable`, `3 variables`. */
export function counted(many: number, one: string, more = `${one}s`): string {
  return many === 0 ? 'none' : `${many} ${many === 1 ? one : more}`;
}

/** What may be done on the page by whoever holds `scopes`, as the daemon holds it to. */
export interface May {
  /** Make an environment, set, take away or mark a variable, take one away. */
  change: boolean;
  /** Set, replace, take away or mark a secret. */
  secrets: boolean;
}

export function mayOf(scopes: string[]): May {
  return {
    change: ['projects:write', 'shell', 'secrets:use'].every((scope) => covers(scopes, scope)),
    secrets: covers(scopes, 'environments:secrets'),
  };
}

/** A secret's line: whether a value is kept for it, and who set it. */
export function secretSaid(secret: Secret, ago: (at: number) => string): string {
  if (secret.kept === false) return 'Not set: no value is kept for it here';
  return `set ${ago(secret.set)} by ${secret.by_name}`;
}

/** What an agent working under the environment is handed a secret as, in words. */
export function handedAs(secret: Secret): string {
  return secret.agents ? `Handed to agents as ${secret.agents}` : 'Not handed to agents';
}

/** A variable's value as it is written in its box. */
export function written(value: Variable['value']): string {
  return typeof value === 'string' ? value : String(value);
}

/**
 * What an apply is asked of an environment, for a project that has
 * `names`, where the instance it would make or apply over runs under
 * `under` (nothing where there is none yet): nothing to ask where the
 * project has none, or where what is running already says; else the
 * names to choose from.
 */
export function asked(names: string[], under: string | null | undefined): string[] | null {
  if (names.length === 0 || (under !== null && under !== undefined)) return null;
  return names;
}

/** Why Apply is closed in a drawer that asks an environment, where it is; nothing where it is open. */
export function applyClosed(names: string[] | null, chosen: string): string | null {
  if (!names || names.length === 0) return null;
  return names.includes(chosen) ? null : 'Choose the environment it runs under: this project runs everything under one of them.';
}

/** What something running is said to run under, where its project has environments or it runs under one. */
export function underSaid(environment: string | undefined, hasAny: boolean): string | null {
  if (environment) return `under ${environment}`;
  return hasAny ? 'under no environment' : null;
}

/** Whether `name` reads as an environment's name: lowercase letters, digits and `-`, at most 40 long, beginning with a letter or a digit. */
export function nameRefused(name: string): string | null {
  if (name === '') return 'Give it a name, such as `staging`.';
  return /^[a-z0-9][a-z0-9-]{0,39}$/.test(name) ? null : 'A name is lowercase letters, digits and `-`, such as `staging`.';
}

/** What is typed in a variable's box, as it is sent: true or false, a number, or else the text as it is. */
export function valueOf(text: string): string | number | boolean {
  const said = text.trim();
  if (said === 'true' || said === 'false') return said === 'true';
  if (said !== '' && /^-?\d+(\.\d+)?$/.test(said)) return Number(said);
  return text;
}
