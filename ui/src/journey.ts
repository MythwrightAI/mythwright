// What a record's history says beyond its sentences: which lines are
// something said (and by whom), which of those was the answer to what had
// been asked, and who asked what a card is waiting on.
//
// The daemon keeps each line with what its event said of itself. This reads
// that, so that what was said on a record is drawn as what was said. That
// a card is waiting on an answer is not worked out here: the card says so
// itself, in its `question`, which what looks after the board's cards puts
// there and takes away. A history only says who asked it.

import type { Anchor } from './anchors.ts';
import type { HistoryLine } from './api.ts';

/** Something said on a record: who said it, the words, and whether it was a question. */
export interface Spoken {
  who: string;
  words: string;
  asks: boolean;
  /** For what was said: that it answered what had been asked. */
  answers: boolean;
  /** For what was said: that whoever said it marked it public. Private, without. */
  public: boolean;
  /** For what was said on a few words of a document: those words, as they were kept with it (`anchors.ts`). */
  on?: Anchor;
}

/** A line as it is drawn: something that happened, or something that was said. */
export interface Drawn {
  line: HistoryLine;
  said: Spoken | null;
}

const text = (value: unknown): string => (typeof value === 'string' ? value : '');

// What was asked on a card, and what anyone said on a record. The second
// of each pair is what the same thing was called before anything but a
// card could be spoken on: lines written then are kept as they were.
const ASKED = ['card.asked', 'board.item.asked'];
const SAID = ['record.commented', 'board.item.commented'];
/** That what was said just before was the answer, which what looks after the board's cards says. */
const ANSWERED = 'card.answered';

/** What a line is, when it is something that was said and not something that happened. */
export function spoken(line: HistoryLine): Spoken | null {
  const data = line.data ?? {};
  if (ASKED.includes(line.topic)) {
    return { who: text(data.asker) || line.who, words: text(data.question), asks: true, answers: false, public: false };
  }
  if (SAID.includes(line.topic)) {
    // A question a step said for a record that asks (an A/B test, on its card) is the record's, which the bot speaks for.
    const asks = isQuestion(line);
    const said: Spoken = { who: (asks && text(data.asker)) || line.who, words: text(data.body), asks, answers: data.answers === true, public: data.public === true };
    const on = data.anchor as Anchor | undefined;
    if (on && typeof on === 'object' && typeof on.quote === 'string') said.on = on;
    return said;
  }
  return null;
}

/**
 * Whether a line is a question a blueprint's step said on the record
 * (`record.ask`): said there by whoever asked, naming whom it is put to,
 * and kept with which asking it is.
 */
export function isQuestion(line: HistoryLine): boolean {
  return SAID.includes(line.topic) && typeof line.data?.once === 'string' && viaOf(line) !== null;
}

/**
 * The lines that say something was asked (`card.asked`) and that a
 * question said on the record after it says again, in its own words and
 * by whoever asked: those are drawn where what happened is, and the
 * question where what was said is, once.
 */
function saidAgain(lines: HistoryLine[]): Set<number> {
  const again = new Set<number>();
  let open: number | null = null;
  for (const line of lines) {
    if (ASKED.includes(line.topic)) open = line.seq;
    else if (open !== null && isQuestion(line)) {
      again.add(open);
      open = null;
    }
  }
  return again;
}

/**
 * A history as it is drawn. What was said that turned out to be the answer
 * is marked as that, and the line that only says so is left out: it was
 * said of the words above it, which now say it themselves. One whose words
 * are not to be found stays, as the sentence it is.
 */
export function drawn(lines: HistoryLine[]): Drawn[] {
  const read: Drawn[] = [];
  const again = saidAgain(lines);
  for (const line of lines) {
    if (again.has(line.seq)) continue;
    if (line.topic === ANSWERED) {
      const body = text(line.data?.body);
      const answer = [...read].reverse().find((one) => one.said && !one.said.asks && !one.said.answers && one.said.words === body);
      if (answer?.said) {
        answer.said = { ...answer.said, answers: true };
        continue;
      }
    }
    read.push({ line, said: spoken(line) });
  }
  return read;
}

/** A comment: a line of a record's history that is something said, with what it said. */
export interface Comment {
  line: HistoryLine;
  said: Spoken;
}

/**
 * The comments on a record, oldest first: the lines of its history that
 * are something said, and none of what happened to it. What an agent
 * asked on a card is one, as it is drawn as one in the history.
 */
export function comments(lines: HistoryLine[]): Comment[] {
  return drawn(lines).flatMap(({ line, said }) => (said ? [{ line, said }] : []));
}

/** What happened to a record, oldest first, with what was said on it left out: the rest of its history, where the comments are shown apart. */
export function happened(lines: HistoryLine[]): HistoryLine[] {
  const again = saidAgain(lines);
  return lines.filter((line) => spoken(line) === null || again.has(line.seq));
}

/**
 * Whom the last question said on a record was put to, with whoever asked
 * it: each named with @ before its words, as it was said (`record.ask`);
 * none, where it was asked of nobody. Nothing where no question was said
 * there: a card a program from before asked on says only that it was.
 */
export function questionOf(lines: HistoryLine[]): Question | null {
  const line = [...lines].reverse().find(isQuestion);
  if (!line) return null;
  const words = text(line.data?.body);
  const whom: string[] = [];
  for (const word of words.split(/\s+/)) {
    if (!word.startsWith('@') || word.length < 2) break;
    whom.push(word.slice(1));
  }
  return { who: line.who, whom, asker: text(line.data?.asker), words };
}

/** The last question said on a record: whoever said it, whom it was put to, who asked (an agent's name, or a record that asks) and its words. */
export interface Question {
  who: string;
  whom: string[];
  asker: string;
  words: string;
}

/** What a card's Details says of a question it waits on the answer to: whom it waits on, and where it is answered. */
export function waitingOn(whom: string[] | null): string {
  if (whom === null) return 'Waiting on an answer: answer in Comments';
  if (whom.length === 0) return 'Waiting on an answer, asked of nobody: answer in Comments';
  const named = whom.map((one) => `@${one}`);
  const said = named.length === 1 ? named[0] : `${named.slice(0, -1).join(', ')} and ${named[named.length - 1]}`;
  return `Waiting on an answer from ${said}: answer in Comments`;
}

/** Who asked the last thing asked on a record, for a card that says it is waiting on an answer. */
export function asker(lines: HistoryLine[]): string | null {
  for (const line of [...lines].reverse()) {
    const said = spoken(line);
    if (said?.asks) return said.who;
  }
  return null;
}

/** The blueprint whose step did what a line is about, which is said after the line as detail (`via`) and never as who did it; nothing for anybody else's doing. */
export function viaOf(line: HistoryLine): string | null {
  const through = line.data?.through;
  return typeof through === 'string' && through !== '' ? through : null;
}

/** Whose work a blueprint's step was told it did what a line is about from, as the line says it after its sentence: ` from what shop-architect-1 wrote`. */
export function fromWhat(line: HistoryLine): string {
  const wrote = text(line.data?.wrote);
  return wrote.startsWith('agent:') && wrote.length > 6 ? ` from what ${wrote.slice(6)} wrote` : '';
}

/**
 * Who a line says did what it is about, as it is read now: what a
 * blueprint's step did is the factory's doing, said as its bot by the name
 * it has now (`bot`, which the page has from the daemon), whatever it was
 * called when the line was written. Anybody else's is as it was written.
 */
export function whoNow(line: HistoryLine, bot: string | null): string {
  if (viaOf(line) && decidedBy(line)) return decidedBy(line) as string;
  return viaOf(line) && bot ? bot : line.who;
}

/** The agent whose decision what a blueprint's step did was (a rung of a project's chain), by its name: such a line is that agent's, with the blueprint after it as detail. */
export function decidedBy(line: HistoryLine): string | null {
  const decided = text(line.data?.decided);
  return decided.startsWith('agent:') && decided.length > 6 ? decided.slice(6) : null;
}

/** A line's sentence as it is read now: of what a blueprint's step did, with the bot by the name it has now in place of the one it was written with. */
export function saidNow(line: HistoryLine, bot: string | null): string {
  const who = whoNow(line, bot);
  return who === line.who || line.who === '' ? line.said : line.said.replace(line.who, who);
}

/**
 * A line's words by their paragraphs, as whoever said them wrote them: a
 * card sent back holds each reviewer's words, apart from the others by an
 * empty line, and drawn as one run they read as one paragraph. A line
 * that is only spaces ends a paragraph as an empty one does; a single
 * break stays inside its paragraph.
 */
export function paragraphsOf(words: string): string[] {
  const all = words.split(/\n[ \t]*\n\s*/).map((one) => one.replace(/\s+$/, ''));
  const kept = all.filter((one) => one !== '');
  return kept.length > 0 ? kept : [words];
}

// What says a record was added: of a card, and of any other record.
const ADDED = ['board.item.created', 'record.created'];

/** Who added a record, as its history says it, and the blueprint whose step did where one did. */
export interface Added {
  /** In the history's own words: a person's name, an agent's, or the factory's bot with whose work it was added from (`conductor from what shop-architect-1 wrote`). */
  who: string;
  /** The blueprint whose step added it, whoever lent it the authority: said after, as detail. */
  via: string | null;
}

/**
 * Who added a record, from the line of its history that says so, which
 * says the factory's bot as it is called now (`bot`) where a blueprint's
 * step did and never whoever applied that blueprint; nothing where the
 * history does not say (it is being read, or was kept before the record's
 * table kept one). A record given back from the Trash was added before that.
 */
export function addedBy(lines: HistoryLine[] | null, bot: string | null): Added | null {
  const line = lines?.find((one) => ADDED.includes(one.topic) && one.data?.restored !== true);
  if (!line) return null;
  const via = viaOf(line);
  return { who: via ? `${whoNow(line, bot)}${fromWhat(line)}` : line.who, via };
}

/** Whoever a line is of, as it is drawn with their picture: who they are and what they are called, and a record where what asked is one. */
export interface Speaker {
  /** Who it is, as the journal has them (`agent:builder`, `user:u1`, `bot:conductor`); nothing where the page cannot say, which is drawn and opens nothing. */
  actor: string | null;
  name: string;
  /** What asked, where it is a record (an A/B test): the bot speaks for it, and it is named after the bot. */
  record?: { id: string; project: string; table: string };
}

/** A record of a project's table, by its id (`<project>-<table>-<number>`), as the daemon numbers one; nothing for anything else. */
export function recordOf(id: string, tables: { project: string; name: string }[]): { id: string; project: string; table: string } | null {
  const one = tables.find((table) => {
    const head = `${table.project}-${table.name}-`;
    return id.startsWith(head) && /^\d+$/.test(id.slice(head.length));
  });
  return one ? { id, project: one.project, table: one.name } : null;
}

/** An actor the page can read about: a person, an agent or a bot, and nothing else (a key, the system). */
const readable = (actor: string | null | undefined): string | null => (actor && /^(user|agent|bot):./.test(actor) ? actor : null);

/**
 * Whoever a line of a record's history is of, as it is drawn: of what was
 * said, whoever said or asked it; of what happened, whoever did it, which
 * is the factory's bot where a blueprint's step did (`whoNow`). An asker
 * that is an agent is that agent; one that is a record (an A/B test asks
 * on its card) is drawn as the bot that speaks for it, with the record
 * after. `agents` is the names of the agents there are, `tables` the
 * tables of the projects the page has read.
 */
export function speakerOf(line: HistoryLine, said: Spoken | null, bot: string | null, agents: ReadonlySet<string>, tables: { project: string; name: string }[]): Speaker {
  const asBot = bot && viaOf(line) && !decidedBy(line) ? `bot:${bot}` : null;
  if (said?.asks && said.who !== line.who) {
    if (agents.has(said.who)) return { actor: `agent:${said.who}`, name: said.who };
    const record = recordOf(said.who, tables);
    if (record) return { actor: bot ? `bot:${bot}` : null, name: bot ?? said.who, record };
    return { actor: null, name: said.who };
  }
  const name = whoNow(line, bot);
  const decided = decidedBy(line);
  if (decided) return { actor: `agent:${decided}`, name };
  return { actor: asBot ?? readable(line.actor), name };
}
