// Mentioning someone in a channel: `@name`.
//
// The daemon decides what a mention means (it is how an agent or a bot is
// asked for something when a channel has more than one). This is only the
// typing of one: noticing that an `@` is being typed, offering who is in the
// channel, and putting the chosen name in.

import type { Member } from './api.ts';
import { actorKind } from './util.ts';

/** Someone who can be mentioned: what follows the `@`, and who that is. */
export interface Mentionable {
  handle: string;
  name: string;
  kind: 'agent' | 'bot' | 'user' | 'system';
}

/**
 * What is said of somebody under their name where they are offered: how
 * an agent stands and what it runs on, that a bot is one and how it
 * answers, a person's name. Whole, on a line of its own, so that none of
 * it is cut off where the list is narrow.
 *
 * `state` is how an agent stands just now and `on` what it runs on, as
 * it is said everywhere else; `takes` is whether a bot is the factory's
 * own, which takes commands.
 */
export function under(one: Mentionable, about: { state?: 'working' | 'idle' | 'stopped' | null; on?: string | null; takes?: boolean } = {}): string {
  if (one.kind === 'agent') {
    const stands = about.state === 'stopped' ? 'not running' : about.state;
    return [stands ?? 'agent', about.on].filter(Boolean).join(' · ');
  }
  if (one.kind === 'bot') return about.takes ? 'bot · takes commands' : 'bot · answers by itself';
  return one.name;
}

/** A name as it is written after an `@`: lowercase words joined by dashes. */
export function handleOf(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/** Who in a channel can be mentioned. Agents and bots first: they are who a mention does something to. */
export function mentionable(members: Member[]): Mentionable[] {
  const order = { agent: 0, bot: 1, user: 2, system: 3 };
  return members
    .map((member) => {
      const kind = actorKind(member.actor);
      // An agent and a bot answer to the name they take part under; a person to their name.
      const handle = kind === 'agent' || kind === 'bot' ? member.actor.slice(member.actor.indexOf(':') + 1) : handleOf(member.name);
      return { handle, name: member.name, kind };
    })
    .filter((one) => one.handle !== '')
    .sort((a, b) => order[a.kind] - order[b.kind]);
}

/**
 * The mention being typed where the caret is: where its `@` is and what has
 * been typed after it. `null` when the caret is not in one. An `@` in the
 * middle of a word is an address, not a mention.
 */
export function typing(text: string, caret: number): { from: number; typed: string } | null {
  const found = /(^|[^A-Za-z0-9])@([A-Za-z0-9_-]*)$/.exec(text.slice(0, caret));
  if (!found) return null;
  const typed = found[2]!;
  return { from: caret - typed.length - 1, typed: typed.toLowerCase() };
}

/** Those whose handle or name starts with what was typed, then those that only contain it. */
export function matching(all: Mentionable[], typed: string): Mentionable[] {
  const starts = (one: Mentionable) => one.handle.startsWith(typed) || one.name.toLowerCase().startsWith(typed);
  const holds = (one: Mentionable) => one.handle.includes(typed) || one.name.toLowerCase().includes(typed);
  return [...all.filter(starts), ...all.filter((one) => !starts(one) && holds(one))];
}

/** The text with the mention being typed finished as `@handle `, and where the caret then is. */
export function complete(text: string, caret: number, from: number, handle: string): { text: string; caret: number } {
  const rest = text.slice(caret);
  const put = `@${handle}${rest.startsWith(' ') ? '' : ' '}`;
  return { text: text.slice(0, from) + put + rest, caret: from + put.length + (rest.startsWith(' ') ? 1 : 0) };
}

/**
 * The text with `@handle ` put where the caret is, for someone chosen
 * without typing their name: apart from the word before it, and finishing
 * the mention being typed there if one is.
 */
export function mention(text: string, caret: number, handle: string): { text: string; caret: number } {
  const begun = typing(text, caret);
  if (begun) return complete(text, caret, begun.from, handle);
  const apart = caret > 0 && !/\s/.test(text[caret - 1]!) ? ' ' : '';
  const at = caret + apart.length;
  return complete(text.slice(0, caret) + apart + text.slice(caret), at, at, handle);
}

/** Something the factory's bot takes: how it is written, and what it does. */
export interface Taken {
  written: string;
  does: string;
}

/**
 * The words of a command that are always written as they are: what comes
 * before the first place it leaves open (`<card>`, `[project]`).
 */
export function fixedWords(written: string): string {
  const words = written.split(/\s+/);
  const open = words.findIndex((word) => /^[<[]/.test(word));
  return (open === -1 ? words : words.slice(0, open)).join(' ');
}

/**
 * What is being written to the bot called `bot` where the caret is: where
 * the words after its name begin, and those words. `null` when the caret
 * is not just after its name on the same line, or the bot has no name yet.
 */
export function commanding(text: string, caret: number, bot: string | null | undefined): { from: number; typed: string } | null {
  if (!bot) return null;
  const named = bot.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const found = new RegExp(`(^|[^A-Za-z0-9])@${named}[ \\t]+([^\\n]*)$`, 'i').exec(text.slice(0, caret));
  if (!found) return null;
  const typed = found[2]!;
  return { from: caret - typed.length, typed: typed.toLowerCase().replace(/\s+/g, ' ').trimStart() };
}

/**
 * The commands that what was typed could still become: those whose fixed
 * words begin with it. None once something has been typed past them, which
 * is what the command is about and not which command it is.
 */
export function taking(all: Taken[], typed: string): Taken[] {
  const so = typed.trimEnd();
  return all.filter((one) => fixedWords(one.written).startsWith(so) && (typed === so || fixedWords(one.written) !== so));
}

/** The text with what was typed to the bot made the fixed words of `written`, and where the caret then is. */
export function command(text: string, caret: number, from: number, written: string): { text: string; caret: number } {
  const rest = text.slice(caret);
  const put = `${fixedWords(written)}${rest.startsWith(' ') ? '' : ' '}`;
  return { text: text.slice(0, from) + put + rest, caret: from + put.length + (rest.startsWith(' ') ? 1 : 0) };
}


/**
 * Who can be named in what is said on a record: each person with an
 * account and each agent the factory does not start, those whom being
 * named tells, in their box; never whoever is writing, and never an agent
 * the factory starts, which is given nothing for being named on a record.
 * People first, by name.
 */
export function taggable(people: Member[], agents: readonly { name: string; adapter: string }[], me: string | null): Mentionable[] {
  const persons = people
    .filter((one) => actorKind(one.actor) === 'user' && one.actor !== me)
    .map((one) => ({ handle: handleOf(one.name), name: one.name, kind: 'user' as const }))
    .filter((one) => one.handle !== '')
    .sort((a, b) => a.name.localeCompare(b.name));
  const outside = agents
    .filter((slot) => slot.adapter === 'external' && `agent:${slot.name}` !== me)
    .map((slot) => ({ handle: slot.name, name: slot.name, kind: 'agent' as const }));
  return [...persons, ...outside];
}

/**
 * Where what is offered as somebody writes (who can be named, what the
 * bot takes) is put on the screen: it lies over the page, put on its body,
 * so that nothing it is drawn in cuts it off or lies over it (the tools
 * panel beside a channel cut a thread's list off under its tabs).
 * `above` is how far down its foot may come where it stands over the box,
 * `below` how far up its head may come where it is under, and `first`
 * which it tries first: over the box under the channel and in a thread,
 * under the tools where a message is written at length. It goes where it
 * fits whole, the other way where only that fits, and otherwise on the side
 * with more room, as tall as that room and scrolling in itself; never past
 * the window by less than `room`, nor under what is fixed at its top
 * (`top`, the desktop window's own bar). `size` is how big it is with
 * nothing held, measured as wide as it may be.
 */
export function offeredAt(
  at: { left: number; above: number; below: number; first: 'over' | 'under' },
  size: { width: number; height: number },
  window: { width: number; height: number; top?: number },
  room = 8,
): { left: number; top: number; most: number | null } {
  const left = Math.max(room, Math.min(at.left, window.width - size.width - room));
  const overRoom = at.above - (window.top ?? 0) - room;
  const underRoom = window.height - room - at.below;
  const over = { left, top: at.above - size.height, most: null };
  const under = { left, top: at.below, most: null };
  const fits = { over: size.height <= overRoom, under: size.height <= underRoom };
  const tried = at.first === 'over' ? (['over', 'under'] as const) : (['under', 'over'] as const);
  for (const way of tried) if (fits[way]) return way === 'over' ? over : under;
  const more = overRoom === underRoom ? at.first : overRoom > underRoom ? 'over' : 'under';
  if (more === 'over') return { left, top: (window.top ?? 0) + room, most: Math.max(0, overRoom) };
  return { left, top: at.below, most: Math.max(0, underRoom) };
}

/**
 * Who can be named in a reply in a topic: each person with an account,
 * whom being named tells where they may read records (and so topics),
 * and each agent, which answers there where the factory starts it and is
 * told in its box where it does not; never whoever is writing. People
 * first, by name, then agents.
 */
export function topical(people: Member[], agents: readonly { name: string }[], me: string | null): Mentionable[] {
  const persons = taggable(people, [], me);
  const all = agents
    .filter((slot) => `agent:${slot.name}` !== me)
    .map((slot) => ({ handle: slot.name, name: slot.name, kind: 'agent' as const }));
  return [...persons, ...all];
}

/**
 * What is said under somebody offered to be named in a topic: as
 * everywhere (`under`), and of an agent the factory starts and is running,
 * that it answers there when named.
 */
export function offeredUnder(one: Mentionable, about: Parameters<typeof under>[1] & { answers?: boolean } = {}): string {
  const said = under(one, about);
  return one.kind === 'agent' && about.answers ? `${said} · answers when named` : said;
}

/** A piece of what somebody wrote: words as they are, or `@handle` as it was written (`written`), lowercase in `handle`. */
export type Said = string | { handle: string; written: string };

/**
 * Words split where they name somebody with `@`, as the daemon reads a
 * name (`talk::channels::mentions`): an `@` not after a letter or digit,
 * then letters, digits, `-` and `_`. `ada@example.com` names nobody.
 * Whether the name is anybody's is not asked here.
 */
export function saidIn(text: string): Said[] {
  const pieces: Said[] = [];
  const names = /@([A-Za-z0-9_-]+)/g;
  let at = 0;
  for (let found = names.exec(text); found; found = names.exec(text)) {
    const before = text.slice(0, found.index).at(-1);
    if (before !== undefined && /[\p{L}\p{N}]/u.test(before)) continue;
    if (found.index > at) pieces.push(text.slice(at, found.index));
    pieces.push({ handle: found[1]!.toLowerCase(), written: found[0] });
    at = found.index + found[0].length;
  }
  if (at < text.length) pieces.push(text.slice(at));
  return pieces;
}

/** Somebody a name is: as `Mentionable`, with who they are (`actor`). */
export type Named = Mentionable & { actor: string };

/**
 * Who `@handle` names, of those known: an agent by its name, the
 * factory's bot by its name, else a person by the word their name is
 * written as (`handleOf`); `null` where it names nobody known.
 */
export function namedBy(handle: string, known: { people: readonly Member[]; agents: readonly { name: string }[]; bot: string | null | undefined }): Named | null {
  const agent = known.agents.find((slot) => slot.name.toLowerCase() === handle);
  if (agent) return { handle, name: agent.name, kind: 'agent', actor: `agent:${agent.name}` };
  if (known.bot && known.bot.toLowerCase() === handle) return { handle, name: known.bot, kind: 'bot', actor: `bot:${known.bot}` };
  const person = known.people.find((one) => actorKind(one.actor) === 'user' && handleOf(one.name) === handle);
  return person ? { handle, name: person.name, kind: 'user', actor: person.actor } : null;
}

/**
 * Who somebody named is, in a few words, for whoever cannot see the card
 * shown as they are pointed at: their name, whether a person, an agent or
 * a bot, and of an agent how it stands and what it runs on (`under`).
 */
export function whoIs(one: Mentionable, about: Parameters<typeof under>[1] = {}): string {
  if (one.kind === 'user') return `${one.name}, person`;
  const said = under(one, about);
  if (one.kind === 'agent') return `${one.name}, ${said.startsWith('agent') ? said : `agent · ${said}`}`;
  return `${one.name}, ${said}`;
}
