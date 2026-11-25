// A topic's replies as a tree: each reply is something said
// on the topic's row, which may say which reply it answers (`parent`, the
// number that one was said at). What the daemon hands is a list, oldest
// first; here is the tree drawn from it, the chain of replies one answers,
// and which chains start folded. It holds nothing of the page, so that its
// cases can read it.

/** The deepest a reply is drawn, as the daemon nests them (`kernel::topics::DEPTH`): one answering a reply there is drawn beside it. */
export const DEPTH = 8;

/** A chain longer than this starts folded past `FOLDED_PAST` of its replies. */
export const LONG = 5;
export const FOLDED_PAST = 3;

/** A reply as the daemon hands it (`topics::reply_of`). */
export interface Reply {
  seq: number;
  parent: number | null;
  by: string | null;
  who: string;
  body: string;
  at: number;
}

/** A reply in the tree: how deep it is drawn (the topic's own replies at 1), and what answers it, oldest first. */
export interface Node {
  reply: Reply;
  depth: number;
  under: Node[];
}

/** A reply from what the daemon handed, whatever it handed of it. */
export function replyOf(handed: Record<string, unknown>): Reply {
  const parent = Number(handed.parent);
  return {
    seq: Number(handed.seq) || 0,
    parent: handed.parent === null || handed.parent === undefined || !Number.isFinite(parent) ? null : parent,
    by: typeof handed.by === 'string' ? handed.by : null,
    who: typeof handed.who === 'string' ? handed.who : '',
    body: typeof handed.body === 'string' ? handed.body : '',
    at: Number(handed.at) || 0,
  };
}

/**
 * The tree of replies: each under the one it answers, oldest first at each
 * level, the topic's own at the top. One whose parent is not among them
 * (taken away, or not read yet) is the topic's own; and one answering a
 * reply that is drawn as deep as replies go is drawn beside that one, as a
 * reply of the same, so that no reply is ever drawn deeper than `DEPTH`.
 * Each reply is drawn once, whatever order they were handed in.
 */
export function tree(replies: readonly Reply[]): Node[] {
  const sorted = [...new Map(replies.map((reply) => [reply.seq, reply])).values()].sort((one, other) => one.seq - other.seq);
  const by = new Map(sorted.map((reply) => [reply.seq, { reply, depth: 0, under: [] as Node[] }]));
  const top: Node[] = [];
  // Oldest first, so that a reply's parent, said before it, is placed before it is.
  for (const reply of sorted) {
    const node = by.get(reply.seq)!;
    let parent = reply.parent !== null && reply.parent !== reply.seq ? by.get(reply.parent) : undefined;
    if (parent && parent.reply.seq > reply.seq) parent = undefined;
    if (!parent) {
      node.depth = 1;
      top.push(node);
      continue;
    }
    // As deep as replies go: beside its parent, under what that answers.
    let at: Node | undefined = parent;
    while (at && at.depth >= DEPTH) at = at.reply.parent === null ? undefined : by.get(at.reply.parent);
    if (!at) {
      node.depth = 1;
      top.push(node);
      continue;
    }
    node.depth = at.depth + 1;
    at.under.push(node);
  }
  return top;
}

/** The chain of replies `seq` is in, from the topic's own down to it, by their numbers; nothing where it is no reply listed. */
export function chainTo(replies: readonly Reply[], seq: number): number[] {
  const by = new Map(replies.map((reply) => [reply.seq, reply]));
  const chain: number[] = [];
  let at = by.get(seq);
  while (at && !chain.includes(at.seq)) {
    chain.unshift(at.seq);
    at = at.parent === null ? undefined : by.get(at.parent);
  }
  return chain;
}

/** How many replies the longest chain under a node holds, the node among them. */
function longest(node: Node): number {
  return 1 + Math.max(0, ...node.under.map(longest));
}

/**
 * Which replies start with what answers them folded: in a chain longer than
 * `LONG`, the reply `FOLDED_PAST` deep, so that a long exchange is read as
 * its first three and how many more there are. Never one whose fold would
 * hide a reply that must be seen (`kept`: the one that answered the topic,
 * and the one an inbox line opened the topic at) or that is one.
 */
export function startsFolded(top: readonly Node[], kept: readonly number[]): Set<number> {
  const folded = new Set<number>();
  const holds = (node: Node): boolean => kept.includes(node.reply.seq) || node.under.some(holds);
  const visit = (node: Node, above: number) => {
    const chain = above + longest(node);
    if (node.depth === FOLDED_PAST && chain > LONG && node.under.length > 0 && !node.under.some(holds)) folded.add(node.reply.seq);
    for (const under of node.under) visit(under, above + 1);
  };
  for (const node of top) visit(node, 0);
  return folded;
}

/** How many replies are under a node, all the way down: what a folded one says it holds. */
export function counted(node: Node): number {
  return node.under.reduce((sum, under) => sum + 1 + counted(under), 0);
}

/** The reply that answered the topic, to draw first under its summary; nothing where none is listed. */
export function answerOf(replies: readonly Reply[], answer: number | null): Reply | null {
  return answer === null ? null : (replies.find((reply) => reply.seq === answer) ?? null);
}
