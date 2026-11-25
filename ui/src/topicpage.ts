// A forum's topic, as the page has it: what the daemon hands of
// one (`GET /api/topics`, `GET /api/topics/<id>`) read into plain values,
// and what the Topics panel and a topic's page work out from them. It
// holds nothing of the page, so that its cases can read it.
//
// A topic is a row of the factory's own table of topics (`@-topics-3`),
// known to people by its title, which is unique whatever its capitals
// (`titleKey`, the daemon's `kernel::topics::title_key`). It is a
// workspace's or a project's, as a channel is (`spaceOfTopic` in
// `workspaces.ts`), and, where it is tracked as a question, stands where
// its status says: open until it is answered or closed, or standing (kept
// up to date, never answered). A wiki's page or a thread that gathers what
// is known has no status at all.

/** Where a topic stands, in the order the daemon offers them. */
export const STATUSES = ['open', 'answered', 'closed', 'standing'] as const;
export type Status = (typeof STATUSES)[number];

/** A topic as the panel lists it: never its body, which only its page reads. */
export interface TopicLine {
  id: string;
  number: number;
  title: string;
  /** Where it stands: nothing for a topic that is not tracked as a question. */
  status: Status | null;
  /** The first line of its summary, which the panel's box finds it by; nothing where it has none. */
  summary: string | null;
  workspace: string | null;
  project: string | null;
  /** The page it is under, by its id; nothing for a wiki. */
  parent: string | null;
  /** Where it stands among the pages beside it. */
  position: number;
  replies: number;
  /** When it was made, and when its row was last written. */
  created: number;
  updated: number;
}

/** A topic whole, as its page reads it. */
export interface TopicWhole extends TopicLine {
  body: string;
  summaryWhole: string;
  /** The reply that answered it, by the number it was said at. */
  answer: number | null;
  followers: string[];
  createdBy: string | null;
}

/** What the daemon hands of a topic, as it is handed (`topics::shown`). */
export interface Handed {
  id: string;
  number?: number;
  position?: number;
  data?: Record<string, unknown>;
  created?: number;
  created_by?: string | null;
  updated?: number;
  replies?: number;
}

const text = (value: unknown): string => (typeof value === 'string' ? value : '');
const placeOf = (value: unknown): string | null => (typeof value === 'string' && value !== '' ? value : null);

/** A status as the daemon says it, or none where it says nothing the page knows. */
export function statusOf(value: unknown): Status | null {
  return (STATUSES as readonly unknown[]).includes(value) ? (value as Status) : null;
}

/** Whether a reply may be marked as what answered a topic standing so: only one tracked as a question, as the daemon holds it. */
export function mayAnswer(status: Status | null): boolean {
  return status !== null;
}

/** A status offered at a topic's head: what it is called, what is written, whether it is the one it has, and why it cannot be chosen where it cannot. */
export interface StatusChoice {
  label: string;
  value: Status | null;
  chosen: boolean;
  closed?: string;
}

/** The statuses a topic's head offers, in the order the daemon offers them, then none (Not tracked). Answered is come to by marking a reply, unless it is so already. */
export function statusChoices(status: Status | null): StatusChoice[] {
  const named: Record<Status, string> = { open: 'Open', answered: 'Answered', closed: 'Closed', standing: 'Standing' };
  const choices: StatusChoice[] = STATUSES.map((value) => ({
    label: named[value],
    value,
    chosen: status === value,
    ...(value === 'answered' && status !== 'answered' ? { closed: 'A topic is answered by Mark as the answer under one of its replies' } : {}),
  }));
  return [...choices, { label: 'Not tracked', value: null, chosen: status === null }];
}

/** The first line of a document that says anything, without the marks that make it a heading or an item. */
export function firstLine(markdown: string): string | null {
  for (const line of markdown.split('\n')) {
    const said = line.replace(/^\s*(#{1,6}\s+|[-*+]\s+|>\s*|\d+[.)]\s+)/, '').trim();
    if (said !== '') return said;
  }
  return null;
}

/** A topic as the panel lists it, from what the daemon handed. */
export function lineOf(handed: Handed): TopicLine {
  const data = handed.data ?? {};
  return {
    id: handed.id,
    number: handed.number ?? 0,
    title: text(data.title) || handed.id,
    status: statusOf(data.status),
    summary: firstLine(text(data.summary)),
    workspace: placeOf(data.workspace),
    project: placeOf(data.project),
    replies: handed.replies ?? 0,
    created: handed.created ?? 0,
    updated: handed.updated ?? handed.created ?? 0,
    parent: placeOf(data.parent),
    position: typeof handed.position === 'number' && Number.isFinite(handed.position) ? handed.position : (handed.number ?? 0),
  };
}

/** A topic whole, from what its page read of it. */
export function wholeOf(handed: Handed): TopicWhole {
  const data = handed.data ?? {};
  const answer = Number(data.answer);
  return {
    ...lineOf(handed),
    body: text(data.body),
    summaryWhole: text(data.summary),
    answer: data.answer === null || data.answer === undefined || data.answer === '' || !Number.isFinite(answer) ? null : answer,
    followers: Array.isArray(data.followers) ? data.followers.filter((one): one is string => typeof one === 'string') : [],
    createdBy: handed.created_by ?? null,
  };
}

/** A title as it is told from another, as the daemon tells them: its words with the spaces between folded and no capitals. */
export function titleKey(title: string): string {
  return title.split(/\s+/).filter(Boolean).join(' ').toLowerCase();
}

/** Whether what names a topic is its id, and not its title: what the factory's table numbers its rows by. */
export function isTopicId(named: string): boolean {
  return /^@-topics-[0-9]+$/.test(named);
}

/** The topic a name means, of those listed: by its id, or by its title whatever its capitals; nothing where none, or several, are. */
export function named(lines: readonly TopicLine[], name: string): TopicLine | null {
  if (isTopicId(name)) return lines.find((line) => line.id === name) ?? null;
  const key = titleKey(name);
  const called = lines.filter((line) => titleKey(line.title) === key);
  return called.length === 1 ? called[0]! : null;
}

/** What a status is said as beside a topic's title in a list, faintly: nothing for one that is open, the usual way a question stands, nor for one with none. */
export function statusSaid(status: Status | null): string | null {
  return status === 'open' ? null : status;
}

/** Where a topic is, said faintly beside it: the project it is of, by its name; nothing for a workspace's own, which is the workspace showing. */
export function whereSaid(line: Pick<TopicLine, 'project'>, projects: readonly { id: string; name: string }[]): string | null {
  if (!line.project) return null;
  return projects.find((project) => project.id === line.project)?.name ?? line.project;
}

/** Whether an event is of the topics' table or of a topic, and of which: so that what follows topics reads them again, and a topic's page itself. */
export function topicOfEvent(event: { topic: string; data?: unknown }): { topic: string | null } | null {
  const data = (event.data ?? {}) as Record<string, unknown>;
  if (event.topic.startsWith('topic.')) return { topic: typeof data.record === 'string' ? data.record : null };
  if (!event.topic.startsWith('record.')) return null;
  const id = typeof data.id === 'string' ? data.id : null;
  if (data.collection === 'topics' && data.project === '@') return { topic: id };
  // What is said on a record carries its id alone, which a topic's begins with the place and table it is of.
  if (id && isTopicId(id)) return { topic: id };
  return null;
}

/** What a title would come to as a topic is made, and why it would be refused, as the daemon refuses it (`kernel::topics::titled`). */
export function titled(typed: string): { title: string; refused: string | null } {
  const title = typed.split(/\s+/).filter(Boolean).join(' ');
  if (title === '') return { title, refused: 'A topic is called something: write its title.' };
  if ([...title].length > 200) return { title, refused: "A topic's title is 200 characters at most." };
  return { title, refused: null };
}

/** The tabs of a topic that is an article: the article itself, its talk (its replies), and its history (its revisions). */
export type TopicPart = 'article' | 'talk' | 'history';

/** What a topic's page draws of it, and what its head offers to add. */
export interface PartsShown {
  /**
   * Whether it is an article, with its talk on a tab of its own beside it; a
   * topic that is not is a thread, its replies drawn directly with no tabs.
   */
  article: boolean;
  /** The tab that shows: nothing for a thread, unless its history is asked for. */
  showing: TopicPart | null;
  /** Whether its summary is drawn: where it says something, or is being written. */
  summary: boolean;
  /** Whether a thread draws its post, over its replies: a tracked question's, where it says something or is being written. */
  post: boolean;
  /** What the head offers whoever may change it: a summary where it has none, an article where it is a thread that may be one, a post where it is a question that has none. */
  addSummary: boolean;
  writeArticle: boolean;
  writePost: boolean;
}

/**
 * What a topic's page draws, from the topic alone and what this reader
 * chose on it, with nothing stored. A question that is tracked (open,
 * answered or closed) is talked out, so it is a thread whatever its post
 * says, its post drawn over its replies where it says something. Any other
 * is an article where its post says something or it is a page under
 * another (a wiki's page is an article, written or not), or where whoever
 * may change it pressed Write an article; else a thread. A summary is drawn
 * where it says something, or where whoever may change it pressed Add
 * summary. Each part not drawn is offered to whoever may change it, and
 * never to anybody else: Write an article on a thread that may be one, Add
 * post on a question. A thread's history shows in its place where it is
 * asked for.
 */
export function partsOf(
  topic: Pick<TopicWhole, 'body' | 'summaryWhole' | 'parent'> & { status?: Status | null },
  how: { mayChange: boolean; chosen: TopicPart | null; writing: boolean; summing: boolean },
): PartsShown {
  const question = topic.status === 'open' || topic.status === 'answered' || topic.status === 'closed';
  const says = topic.body.trim() !== '';
  const writing = how.mayChange && how.writing;
  const article = !question && (says || topic.parent !== null || writing);
  const post = question && (says || writing);
  const summary = topic.summaryWhole.trim() !== '' || (how.mayChange && how.summing);
  return {
    article,
    showing: article ? (how.chosen ?? 'article') : how.chosen === 'history' ? 'history' : null,
    summary,
    post,
    addSummary: how.mayChange && !summary,
    writeArticle: how.mayChange && !question && !article,
    writePost: how.mayChange && question && !post,
  };
}
