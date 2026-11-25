// A picture in what somebody wrote: how it is written, and what it is drawn
// as. A function of plain text with nothing of the page in it, so that its
// tests can read it; `views/pictured.tsx` draws it.
//
// A picture is written as Markdown writes one, and stays text:
//
//   ![what it shows](attachment:ratt_1a2b)            across the column
//   ![what it shows](attachment:ratt_1a2b "side")     at the side, words beside it
//
// What is in the brackets is its caption, said under it. Where it leads is
// a file attached to what the text is of (a topic is a record), named by
// the attachment's id, which neither a revision nor a new title changes.
// An address somewhere else (`https://…`) is never fetched: the factory
// sends nothing anywhere unasked, so such a picture is drawn as a link
// with its caption. An attached page (`.html`) put in the same way is
// drawn as channels draw a view, and an SVG is how a diagram is shown.

/** Where a picture stands: at the side of the words, or across the column. */
export type Place = 'side' | 'across';

/** A picture as it is written. */
export interface Picture {
  caption: string;
  /** Where it leads, as it was written. */
  to: string;
  /** Where it was said to stand; a picture that says nothing stands across. */
  place: Place | null;
}

/** How the address of a file attached here begins. */
export const ATTACHED = 'attachment:';

// The marks of one picture: `![caption](address "how it stands")`. A `]` in a
// caption is written `\]`, as the editor writes it.
const CAPTION = String.raw`((?:\\.|[^\]\\\n])*)`;
const ADDRESS = String.raw`([^)\s]+)`;
const TITLE = String.raw`(?:\s+"([^"\n]*)")?`;
/** One picture, wherever it is in a line: what `prose.ts` looks for among the marks. */
export const PICTURE = `!\\[${CAPTION}\\]\\(${ADDRESS}${TITLE}\\)`;
const ONLY = new RegExp(`^\\s*${PICTURE}\\s*$`);

/** How a picture said it stands, from the words in its quotes. */
export function placeOf(said: string | undefined | null): Place | null {
  const word = (said ?? '').trim().toLowerCase();
  return word === 'side' || word === 'across' ? word : null;
}

/** A picture from what its marks held: the caption, the address, and the words in quotes. */
export function pictureOf(caption: string, to: string, said?: string | null): Picture {
  return { caption: caption.replace(/\\(.)/g, '$1'), to, place: placeOf(said) };
}

/** The picture a line is, where the line is nothing but one picture: it is then drawn as a figure of its own. */
export function figureOf(line: string): Picture | null {
  const found = ONLY.exec(line);
  return found ? pictureOf(found[1]!, found[2]!, found[3]) : null;
}

/** A picture written as it is kept: what `figureOf` reads back as the same picture. */
export function written(picture: Picture): string {
  const caption = picture.caption.replace(/[\\[\]]/g, '\\$&').replace(/\n/g, ' ');
  return `![${caption}](${picture.to}${picture.place ? ` "${picture.place}"` : ''})`;
}

/** The id of the attachment a picture leads to, or `null` where it leads somewhere else. */
export function attachmentOf(to: string): string | null {
  if (!to.startsWith(ATTACHED)) return null;
  const id = to.slice(ATTACHED.length);
  return /^[\w-]+$/.test(id) ? id : null;
}

/** The kinds of picture a browser draws without running anything, SVG among them. */
export const DRAWN_PICTURES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif', 'image/svg+xml'];

/** What is a view: a page attached, drawn in a box of its own. */
export const VIEW_MEDIA = 'text/html';

/** What a chooser of files to put in a text offers: pictures, SVG and pages. */
export const CHOSEN = 'image/png,image/jpeg,image/gif,image/webp,image/svg+xml,.svg,text/html,.html,.htm';

/** Whether a file of this kind is put in the text where it is pasted or dropped: a picture, an SVG, or a page. */
export const takenIn = (media: string) => [...DRAWN_PICTURES, VIEW_MEDIA].includes(media.split(';')[0]!.trim().toLowerCase());

/** What a picture is drawn as. */
export type Drawn =
  // The file, drawn; where it cannot be read, a line saying so.
  | { as: 'picture'; id: string }
  // An attached page, drawn as a view.
  | { as: 'view'; id: string }
  // Another kind of file, by its name, to be saved.
  | { as: 'file'; id: string; name: string }
  // An address somewhere else on the web, which is not fetched: a link.
  | { as: 'link'; to: string }
  // Anything else (a path, a script's address): the marks as they were written, and nothing to press.
  | { as: 'written'; text: string };

/**
 * What a picture is drawn as, by what is known to be attached where it is
 * (`null` where nothing is known: text of no record, or not read yet).
 * Where the attachment is listed, by what kind of file it is; where it is
 * not, it is tried as a picture, and the page says so where that fails (it
 * went, or it is another record's that the reader may not read).
 */
export function drawnAs(picture: Picture, attached: { id: string; name: string; media: string }[] | null): Drawn {
  const id = attachmentOf(picture.to);
  if (id === null) return /^https?:\/\/[^\s]+$/i.test(picture.to) ? { as: 'link', to: picture.to } : { as: 'written', text: written(picture) };
  const one = attached?.find((it) => it.id === id);
  const media = one?.media.split(';')[0]!.trim().toLowerCase();
  if (!one || DRAWN_PICTURES.includes(media!)) return { as: 'picture', id };
  if (media === VIEW_MEDIA) return { as: 'view', id };
  return { as: 'file', id, name: one.name };
}

/** What is said in place of a picture that could not be read. */
export function missingSaid(picture: Picture): string {
  const named = picture.caption.trim() ? `“${picture.caption.trim()}”` : 'A picture';
  return `${named} is not here: it was removed, or it is not yours to see.`;
}

/** What a link to a picture somewhere else says: its caption, and that it is not fetched. */
export function elsewhereSaid(picture: Picture): { words: string; title: string } {
  return {
    words: picture.caption.trim() || picture.to,
    title: `A picture at ${picture.to}. It is not fetched here; open it to see it.`,
  };
}

/** A picture's caption to begin with, from the name of the file it is: none for a picture pasted, whose name says nothing. */
export function captionOf(name: string, media: string): string {
  if (!media.startsWith(VIEW_MEDIA)) return '';
  return name.replace(/\.[^.]+$/, '');
}

/** What a file is, by what it says it is or else by the end of its name: an SVG or a page often arrives saying nothing. */
export function mediaOf(file: { name: string; type: string }): string {
  if (file.type) return file.type;
  if (/\.svg$/i.test(file.name)) return 'image/svg+xml';
  if (/\.html?$/i.test(file.name)) return VIEW_MEDIA;
  return '';
}

/**
 * Text with pictures put in where the caret is (`at`), each on a line of
 * its own, which is how one is read as a figure: the line the caret is in
 * is ended before them where it has words, and they are ended after. Where
 * the caret is then, after them.
 */
export function withPictures(text: string, at: number, pictures: Picture[]): { text: string; caret: number } {
  const where = Math.max(0, Math.min(at, text.length));
  const before = text.slice(0, where);
  const after = text.slice(where);
  const put = `${before === '' || before.endsWith('\n') ? '' : '\n'}${pictures.map(written).join('\n')}\n`;
  const rest = after.startsWith('\n') ? after.slice(1) : after;
  return { text: before + put + rest, caret: before.length + put.length };
}
