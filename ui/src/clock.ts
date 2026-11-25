// How a time is written for whoever reads the page: in their zone, on a
// clock of twelve hours or twenty-four, with the zone's short name after
// it as it is on that date (`10:07 PM MDT` in summer, `MST` in winter).
// Every time the client draws goes through here (`clock` and `when` of
// `util.ts` are these, in the setting the page wears).
//
// What the setting is, and what is kept of it, is the account's
// (`people::clocked`, read and followed by `clockkept.ts`). The daemon
// keeps a zone as text and knows no zones: the browser does
// (`Intl.supportedValuesOf`), so a zone is held to those here before it is
// sent, and every conversion is `Intl`'s, with nothing written out.

import { signal } from '@preact/signals';

export type Hours = '12h' | '24h';
export type Setting = { zone: string; hours: Hours };
export const HOURS: Hours[] = ['12h', '24h'];
/** How a time is written where nothing is said. */
export const USUAL_HOURS: Hours = '12h';
/** The most letters a zone's name has, as the daemon holds it. */
export const LONGEST_ZONE = 64;

/** The zone the browser says it is in, or UTC where it says none. */
export function browserZone(): string {
  try {
    return new Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  } catch {
    return 'UTC';
  }
}

/** The setting the page wears: the account's once it is read, and until then the browser's zone. */
export const worn = signal<Setting>({ zone: browserZone(), hours: USUAL_HOURS });

/** Every zone the browser knows, by name, in order: the list a zone is chosen from. */
export function zones(): string[] {
  let all: string[] = [];
  try {
    all = (Intl as unknown as { supportedValuesOf?: (what: string) => string[] }).supportedValuesOf?.('timeZone') ?? [];
  } catch {
    all = [];
  }
  return [...new Set([...all, 'UTC'])].sort();
}

/** Whether the browser knows a zone of that name. */
export function zoneKnown(zone: string): boolean {
  try {
    new Intl.DateTimeFormat('en-US', { timeZone: zone });
    return true;
  } catch {
    return false;
  }
}

/** The zones whose name says every word of what was typed, as somebody looks for one. */
export function zonesFound(typed: string, all: string[] = zones()): string[] {
  const words = typed.toLowerCase().split(/[\s/_]+/).filter(Boolean);
  return all.filter((zone) => {
    const name = zone.toLowerCase().replace(/_/g, ' ');
    return words.every((word) => name.includes(word));
  });
}

/** What a moment is in a zone: its day, the hour and minute, and the zone's short name as it is then. */
export type Parts = { year: number; month: number; day: number; weekday: string; hour: number; minute: number; name: string };

const formats = new Map<string, Intl.DateTimeFormat>();
function formatOf(zone: string): Intl.DateTimeFormat {
  let format = formats.get(zone);
  if (!format) {
    format = new Intl.DateTimeFormat('en-US', {
      timeZone: zone,
      hourCycle: 'h23',
      year: 'numeric',
      month: 'numeric',
      day: 'numeric',
      weekday: 'short',
      hour: '2-digit',
      minute: '2-digit',
      timeZoneName: 'short',
    });
    formats.set(zone, format);
  }
  return format;
}

/** What `ms` is in `zone`; in UTC where the browser knows no such zone, so that a time is always written. */
export function partsIn(ms: number, zone: string): Parts {
  let format: Intl.DateTimeFormat;
  try {
    format = formatOf(zone);
  } catch {
    format = formatOf('UTC');
  }
  const parts: Record<string, string> = {};
  for (const part of format.formatToParts(ms)) parts[part.type] = part.value;
  return {
    year: Number(parts.year),
    month: Number(parts.month),
    day: Number(parts.day),
    weekday: parts.weekday ?? '',
    hour: Number(parts.hour) % 24,
    minute: Number(parts.minute),
    name: parts.timeZoneName ?? zone,
  };
}

const two = (n: number) => String(n).padStart(2, '0');

/** `10:07 PM MDT`, or `22:07 MDT` on a clock of twenty-four hours. */
export function timeIn(ms: number, setting: Setting): string {
  const at = partsIn(ms, setting.zone);
  if (setting.hours === '24h') return `${two(at.hour)}:${two(at.minute)} ${at.name}`;
  const hour = at.hour % 12 === 0 ? 12 : at.hour % 12;
  return `${hour}:${two(at.minute)} ${at.hour < 12 ? 'AM' : 'PM'} ${at.name}`;
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** Which day a moment is in a zone, counted from the epoch: what tells today from yesterday there. */
export function dayIn(ms: number, zone: string): number {
  const at = partsIn(ms, zone);
  return Math.round(Date.UTC(at.year, at.month - 1, at.day) / 86_400_000);
}

/** The time today, the weekday before it this week, and the date before it otherwise, all in the reader's zone. */
export function whenIn(ms: number, setting: Setting, now = Date.now()): string {
  const days = dayIn(now, setting.zone) - dayIn(ms, setting.zone);
  const time = timeIn(ms, setting);
  if (days <= 0) return time;
  const at = partsIn(ms, setting.zone);
  if (days < 7) return `${at.weekday} ${time}`;
  return `${at.day} ${MONTHS[at.month - 1]} ${time}`;
}

/** The date a moment is in a zone, as a heading says it: `3 Nov`, with its year where that is not this one. */
export function dateIn(ms: number, zone: string, now = Date.now()): string {
  const at = partsIn(ms, zone);
  const year = partsIn(now, zone).year;
  return `${at.day} ${MONTHS[at.month - 1]}${at.year === year ? '' : ` ${at.year}`}`;
}

/** The zone's name as somebody reads it in a list: its parts with spaces for `_`, and what it is called now. */
export function zoneSaid(zone: string, now = Date.now()): string {
  return `${zone.replace(/_/g, ' ')} (${partsIn(now, zone).name})`;
}

/**
 * Why `said` is not a setting the daemon keeps, in its own sentence, or
 * nothing where it is one: the same rule as `people::clocked::checked`,
 * held to it case for case by `clockcases.json`. It holds a zone to the
 * form its name has; whether the browser knows it is `zoneKnown`.
 */
export function clockUnfit(said: unknown): string | null {
  if (typeof said !== 'object' || said === null || Array.isArray(said)) return 'How a time is written is a mapping of `zone` and `hours`.';
  const keys = Object.keys(said);
  const other = keys.find((key) => key !== 'zone' && key !== 'hours');
  if (other !== undefined) return `\`${other}\` is nothing said of how a time is written: it says \`zone\` and \`hours\`.`;
  const { zone, hours } = said as { zone?: unknown; hours?: unknown };
  if (typeof zone !== 'string') return 'A zone is said by its name, such as `America/Denver`, which is text.';
  if (zone === '') return 'Say which zone, by its name, such as `America/Denver`.';
  const letters = [...zone].length;
  if (letters > LONGEST_ZONE) return `A zone's name is ${LONGEST_ZONE} letters at most, and this one is ${letters}.`;
  if (/[\u0000-\u001f\u007f-\u009f]/.test(zone)) return "A zone's name is one line.";
  const stray = [...zone].find((c) => !/[A-Za-z0-9/_+-]/.test(c));
  if (stray !== undefined) return `A zone's name is made of letters, digits, \`_\`, \`-\` and \`+\`, in parts between \`/\`, and \`${stray}\` is none of them.`;
  if (zone.split('/').some((part) => !/^[A-Za-z]/.test(part))) {
    return `\`${zone}\` is no zone's name: each of its parts between \`/\` begins with a letter, such as \`America/Denver\`.`;
  }
  if (hours !== undefined && hours !== null && hours !== '12h' && hours !== '24h') {
    return `\`hours\` is \`12h\` or \`24h\`, and ${JSON.stringify(hours)} is neither.`;
  }
  return null;
}

/** What `said` comes to as a setting the page wears, or nothing where it is none the browser can write a time in. */
export function settingOf(said: unknown): Setting | null {
  if (clockUnfit(said) !== null) return null;
  const { zone, hours } = said as { zone: string; hours?: Hours | null };
  if (!zoneKnown(zone)) return null;
  return { zone, hours: hours ?? USUAL_HOURS };
}
