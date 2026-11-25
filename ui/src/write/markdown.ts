// What the block editor reads a document as and writes it back as: the
// schema of Markdown that `prosemirror-markdown` has, with a footnote's
// marker (`[^1]`) and a footnote's source (`[^1]: …` on a line of its own)
// beside it, so that a topic's sources go through the editor as they were
// written (`cited.ts` says the form). A link to a page (`[words](topic:…)`,
// `pagelinks.ts`) is a link as any other and needs nothing here. A table
// (`tables.ts`) is kept as the text it was written as, as a box's block of
// code is: no editor of tables is built in, and one that read it into cells
// would write it back laid out its own way. It is edited as its lines.
//
// Nothing of the page is in it, so that what goes in and comes out can be
// tried as plain text (`markdown.test.ts`).

import type MarkdownIt from 'markdown-it';
import type StateBlock from 'markdown-it/lib/rules_block/state_block.mjs';
import type StateInline from 'markdown-it/lib/rules_inline/state_inline.mjs';
import { MarkdownParser, MarkdownSerializer, defaultMarkdownParser, defaultMarkdownSerializer, schema as plain } from 'prosemirror-markdown';
import type { MarkdownSerializerState } from 'prosemirror-markdown';
import { Schema } from 'prosemirror-model';
import type { Node } from 'prosemirror-model';
import { tableAt } from '../tables.ts';

/** The labels a footnote's marker may have: anything but a space or a bracket. */
const LABEL = String.raw`[^\]\s]+`;
const MARKER = new RegExp(String.raw`^\[\^(${LABEL})\]`);
const SOURCE = new RegExp(String.raw`^\[\^(${LABEL})\]:[ \t]*`);

type Named<T> = T extends Schema<infer N, infer M> ? [N, M] : never;
/** The kinds of node and of mark there are: Markdown's, the two of a footnote, and a table kept as its text. */
type Nodes = Named<typeof plain>[0] | 'footnote_ref' | 'footnote_def' | 'pipe_table';
type Marks = Named<typeof plain>[1];

export const schema: Schema<Nodes, Marks> = new Schema<Nodes, Marks>({
  nodes: plain.spec.nodes.append({
    footnote_ref: {
      inline: true,
      group: 'inline',
      atom: true,
      attrs: { label: { default: '1' } },
      toDOM: (node) => ['sup', { class: 'cite writer-cite', 'data-cite': node.attrs.label }],
      parseDOM: [{ tag: 'sup[data-cite]', getAttrs: (dom) => ({ label: (dom as HTMLElement).getAttribute('data-cite') ?? '1' }) }],
    },
    footnote_def: {
      content: 'inline*',
      group: 'block',
      defining: true,
      attrs: { label: { default: '1' } },
      toDOM: (node) => ['div', { class: 'writer-note', 'data-note-label': node.attrs.label }, 0],
      parseDOM: [{ tag: 'div[data-note-label]', getAttrs: (dom) => ({ label: (dom as HTMLElement).getAttribute('data-note-label') ?? '1' }) }],
    },
    pipe_table: {
      content: 'text*',
      marks: '',
      group: 'block',
      code: true,
      defining: true,
      toDOM: () => ['pre', { class: 'writer-table', 'data-table': '' }, ['code', 0]],
      parseDOM: [{ tag: 'pre[data-table]', preserveWhitespace: 'full', priority: 60 }],
    },
  }),
  marks: plain.spec.marks,
});

/** A footnote's marker, read before a link could take its bracket. */
function marker(state: StateInline, silent: boolean): boolean {
  if (state.src.charCodeAt(state.pos) !== 0x5b || state.src.charCodeAt(state.pos + 1) !== 0x5e) return false;
  const found = MARKER.exec(state.src.slice(state.pos));
  if (!found) return false;
  if (!silent) state.push('footnote_ref', '', 0).meta = { label: found[1] };
  state.pos += found[0].length;
  return true;
}

/** A footnote's source: a line of its own, read before a link's definition could take it, and its words read as a line's are. */
function source(state: StateBlock, start: number, _end: number, silent: boolean): boolean {
  if (state.sCount[start]! - state.blkIndent >= 4) return false;
  const line = state.src.slice(state.bMarks[start]! + state.tShift[start]!, state.eMarks[start]);
  const found = SOURCE.exec(line);
  if (!found) return false;
  if (silent) return true;
  const open = state.push('footnote_def_open', 'div', 1);
  open.meta = { label: found[1] };
  open.map = [start, start + 1];
  const words = state.push('inline', '', 0);
  words.content = line.slice(found[0].length).trim();
  words.map = [start, start + 1];
  words.children = [];
  state.push('footnote_def_close', 'div', -1);
  state.line = start + 1;
  return true;
}

/** A table, read where `tables.ts` reads one, as the lines it was written as. */
function table(state: StateBlock, start: number, end: number, silent: boolean): boolean {
  if (state.sCount[start]! - state.blkIndent >= 4) return false;
  const line = (at: number) => state.src.slice(state.bMarks[at]! + state.tShift[at]!, state.eMarks[at]);
  const lines = Array.from({ length: Math.min(end, start + 2) - start }, (_, at) => line(start + at));
  if (!tableAt(lines, 0)) return false;
  if (silent) return true;
  const all = Array.from({ length: end - start }, (_, at) => line(start + at));
  const found = tableAt(all, 0)!;
  const token = state.push('pipe_table', 'pre', 0);
  token.content = all.slice(0, found.lines).join('\n');
  token.map = [start, start + found.lines];
  state.line = start + found.lines;
  return true;
}

/**
 * Footnotes, as markdown-it reads them for this editor: no package of them
 * is taken in for two rules. And a link to a page is kept as it was
 * written: read as an address of the web, its `@` would be taken for the
 * name of somebody signing in, and dropped.
 */
function footnotes(md: MarkdownIt): void {
  md.inline.ruler.before('link', 'footnote_ref', marker);
  md.block.ruler.before('reference', 'footnote_def', source, { alt: ['paragraph'] });
  md.block.ruler.before('paragraph', 'pipe_table', table, { alt: ['paragraph'] });
  const normal = md.normalizeLink.bind(md);
  md.normalizeLink = (url) => (url.startsWith('topic:') ? url : normal(url));
}

// A reader of its own, made as the library makes its own, so that the library's is left as it was.
const Reader = defaultMarkdownParser.tokenizer.constructor as typeof MarkdownIt;

export const parser = new MarkdownParser(schema, new Reader('commonmark', { html: false }).use(footnotes), {
  ...defaultMarkdownParser.tokens,
  footnote_ref: { node: 'footnote_ref', getAttrs: (token) => ({ label: token.meta?.label ?? '1' }) },
  footnote_def: { block: 'footnote_def', getAttrs: (token) => ({ label: token.meta?.label ?? '1' }) },
  pipe_table: { block: 'pipe_table', noCloseToken: true },
});

/** What the library's writer keeps of the block it closed last, which it does not offer by name. */
type Closing = MarkdownSerializerState & { closed: Node | false; flushClose(size: number): void };

// Lists are written with dashes, as the files here already are; the
// library's own choice of `*` would rewrite every list it touched. Sources
// one after another are written on lines one after another, as they are
// written by hand.
export const serializer = new MarkdownSerializer(
  {
    ...defaultMarkdownSerializer.nodes,
    bullet_list(state, node) {
      state.renderList(node, '  ', () => '- ');
    },
    footnote_ref(state, node) {
      state.write(`[^${node.attrs.label}]`);
    },
    footnote_def(state, node) {
      const closing = state as Closing;
      if (closing.closed && closing.closed.type.name === 'footnote_def') closing.flushClose(1);
      state.write(`[^${node.attrs.label}]: `);
      state.renderInline(node);
      state.closeBlock(node);
    },
    pipe_table(state, node) {
      state.text(node.textContent, false);
      state.closeBlock(node);
    },
  },
  defaultMarkdownSerializer.marks,
);

/** A document read from its Markdown. */
export const parsed = (markdown: string): Node => parser.parse(markdown) ?? schema.node('doc', null, [schema.node('paragraph')]);

/** A document written as Markdown, ending with one line's end, as files here do. */
export const written = (doc: Node): string => `${serializer.serialize(doc)}\n`;
