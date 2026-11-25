// What a paste into the composer brings: which files are attached, what
// each is called, and whether the text the browser would put in the box
// is let through. Browsers differ in what they fill when a picture is
// copied: some hand it as a file, some as an item of the clipboard, some
// both, and some only as the HTML of the page it was on. What a picture
// copied on Windows is handed as through WSLg may be a BMP or a TIFF,
// which no browser shows: such a picture is drawn again as a PNG in the
// page before it is attached (`views/attaching.tsx`), and these functions
// say when and what it is then called.
//
// Nothing is fetched for a paste: a picture that is on the clipboard only
// as the address of somebody's page is no picture of the clipboard's, and
// the client fetches from its own daemon alone.

/** The kinds of picture the client shows as they are; any other picture is drawn again as a PNG before it goes. */
export const PICTURES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif'];

/** What is known of a file without reading it. */
export interface Like {
  name: string;
  type: string;
  size: number;
}

/** What the paste carried, as the page read it while the paste was being said. */
export interface Carried<F extends Like> {
  files: F[];
  /** Each item of the clipboard by its kind, and for one of kind `file` the file it is. */
  items: { kind: string; file: F | null }[];
  html: string;
  text: string;
}

/** What is done with a paste. */
export type Paste<F> =
  // These are attached, and no text goes into the box.
  | { take: 'files'; files: F[] }
  // The pictures the HTML holds in itself, each a `data:` address, are attached, and no text goes into the box.
  | { take: 'inline'; sources: string[] }
  // The browser puts the text in as it would. `remote` where the HTML names a picture somewhere else, which is not here to attach.
  | { take: 'text'; remote: boolean }
  // Nothing came: no file, no picture and no text.
  | { take: 'nothing' };

const key = (file: Like) => `${file.name}\n${file.type}\n${file.size}`;

/**
 * Every file the paste carried, once: those it lists as files and those it
 * lists as items. The two are often the same file twice, so a file is
 * taken as many times as the more of the two lists has it.
 */
export function filesOf<F extends Like>(carried: Pick<Carried<F>, 'files' | 'items'>): F[] {
  const taken = [...carried.files];
  const counted = new Map<string, number>();
  for (const file of taken) counted.set(key(file), (counted.get(key(file)) ?? 0) + 1);
  for (const item of carried.items) {
    if (item.kind !== 'file' || !item.file) continue;
    const left = counted.get(key(item.file)) ?? 0;
    if (left > 0) counted.set(key(item.file), left - 1);
    else taken.push(item.file);
  }
  return taken;
}

/** The address of each picture the HTML holds, in its order, with `&amp;` read as what it stands for. */
export function picturesIn(html: string): string[] {
  const found: string[] = [];
  for (const tag of html.matchAll(/<img\b[^>]*>/gi)) {
    const src = /\ssrc\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+))/i.exec(tag[0]);
    const said = src?.[1] ?? src?.[2] ?? src?.[3];
    if (said?.trim()) found.push(said.trim().replace(/&amp;/g, '&'));
  }
  return found;
}

/** Whether an address is a picture held in itself, which is decoded where it is and asks nothing of anywhere. */
export const isInline = (src: string) => /^data:image\/[\w.+-]+[;,]/i.test(src);

/** What is done with a paste that carried this. */
export function pasteOf<F extends Like>(carried: Carried<F>): Paste<F> {
  const files = filesOf(carried);
  if (files.length > 0) return { take: 'files', files };
  const pictures = picturesIn(carried.html);
  const sources = pictures.filter(isInline);
  if (sources.length > 0) return { take: 'inline', sources };
  const remote = pictures.length > 0;
  if (remote || carried.text !== '') return { take: 'text', remote };
  return { take: 'nothing' };
}

/** The line said under the box for a paste, where one is: what was not there to be attached. */
export function pasteSaid(paste: Paste<unknown>): string | null {
  if (paste.take === 'text' && paste.remote) return 'The picture itself was not on the clipboard: copy it with Copy image to attach it.';
  if (paste.take === 'nothing') return 'Nothing came with that paste: the clipboard held no picture, file or text.';
  return null;
}

/** The picture a `data:` address holds, as its kind and its bytes; nothing where it is no picture or cannot be read. */
export function dataOf(src: string): { type: string; bytes: Uint8Array } | null {
  const at = /^data:(image\/[\w.+-]+)((?:;[\w-]+=[^;,]*)*)(;base64)?,(.*)$/is.exec(src.trim());
  if (!at) return null;
  const [, type, , base64, body] = at;
  try {
    if (base64) {
      const binary = atob(body!.replace(/\s+/g, ''));
      return { type: type!.toLowerCase(), bytes: Uint8Array.from(binary, (one) => one.charCodeAt(0)) };
    }
    return { type: type!.toLowerCase(), bytes: new TextEncoder().encode(decodeURIComponent(body!)) };
  } catch {
    return null;
  }
}

/** Whether a pasted file is a picture of a kind the client does not show, to be drawn again as a PNG before it goes. */
export const drawnAgain = (type: string) => type.toLowerCase().startsWith('image/') && !PICTURES.includes(type.toLowerCase());

const ENDINGS: Record<string, string> = { 'image/jpeg': 'jpg', 'image/svg+xml': 'svg', 'image/x-icon': 'ico', 'image/vnd.microsoft.icon': 'ico', 'image/x-ms-bmp': 'bmp' };

/** What a file of a kind ends in. */
export function endingOf(type: string): string {
  const said = type.toLowerCase();
  return ENDINGS[said] ?? (said.split('/')[1]?.replace(/[^a-z0-9]/g, '') || 'bin');
}

/** What browsers and systems call something pasted that had no name of its own. */
const UNNAMED = /^(?:|image|images|blob|untitled|clipboard|clipboard image|unknown|file)$/i;

const two = (figure: number) => String(figure).padStart(2, '0');

/** Whether a name's ending is the one a file of the kind has, or one it is as often called by. */
const fits = (ending: string, type: string) => {
  const said = ending.toLowerCase();
  const kind = endingOf(type);
  return said === kind || (kind === 'jpg' && said === 'jpeg') || (kind === 'tiff' && said === 'tif');
};

/**
 * What a pasted file is called, now that it is of `type`: its own name
 * where it has one worth keeping, a picture's ending as its kind is (one
 * drawn again as a PNG ends in `.png`); otherwise `pasted-` and when it
 * was pasted. One called what another of the same paste is already called
 * is numbered after it.
 */
export function pastedName(name: string, type: string, at: Date, taken: string[] = []): string {
  const dot = name.lastIndexOf('.');
  const stem = (dot > 0 ? name.slice(0, dot) : name).trim();
  const own = dot > 0 ? name.slice(dot + 1) : '';
  const picture = type.toLowerCase().startsWith('image/');
  const ending = picture && !fits(own, type) ? endingOf(type) : own;
  const base = UNNAMED.test(stem)
    ? `pasted-${at.getFullYear()}${two(at.getMonth() + 1)}${two(at.getDate())}-${two(at.getHours())}${two(at.getMinutes())}${two(at.getSeconds())}`
    : stem;
  const first = ending ? `${base}.${ending}` : base;
  if (!taken.includes(first)) return first;
  for (let count = 2; ; count++) {
    const next = ending ? `${base}-${count}.${ending}` : `${base}-${count}`;
    if (!taken.includes(next)) return next;
  }
}
