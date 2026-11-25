// What the factory has to say for itself: when the daemon started and
// stopped, why, and what it is doing without. Each is an event the daemon
// keeps in its journal; this says one in words.
//
// A thing that is true of the factory and not of anything being looked at
// goes here, where it can be found later, and not in front of whoever
// happens to have a window open.

import type { FleetEvent } from './api.ts';

export interface Line {
  /** What happened, in a word or two. */
  what: string;
  /** The rest of it: where, which version, why. */
  more: string;
}

/** An event of the factory's own, as a line of its log. */
export function line(event: FleetEvent): Line {
  const data = (event.data ?? {}) as Record<string, unknown>;
  const text = (key: string) => (typeof data[key] === 'string' ? (data[key] as string) : '');
  switch (event.topic) {
    case 'system.started': {
      // Which program it was: somebody's installed copy, or one run from
      // the checkout it was built in, which reads that checkout as it is.
      const from = { installed: 'a copy on its own', checkout: 'run from its checkout' }[text('runs_from')] ?? '';
      const at = [text('version') && `version ${text('version')}`, from, text('address') && `at ${text('address')}`].filter(Boolean).join(', ');
      // Whoever started it may have left a word on why.
      return { what: 'Started', more: [at && `${at[0]!.toUpperCase()}${at.slice(1)}.`, text('note')].filter(Boolean).join(' ') };
    }
    case 'system.stopping':
      return { what: 'Stopping', more: text('note') };
    case 'system.stopped':
      return { what: 'Stopped', more: text('note') || 'What was running was left running for the next start.' };
    case 'system.degraded': {
      const without = text('what') ? `Running without ${text('what')}` : 'Running with less';
      const why = [text('reason'), text('effect')].filter(Boolean).join('; ');
      return { what: without, more: why ? `${why[0]!.toUpperCase()}${why.slice(1)}.` : '' };
    }
    default: {
      // One this does not know: its topic's last word, and whatever it says of itself.
      const word = event.topic.split('.').pop() ?? event.topic;
      return { what: `${word[0]?.toUpperCase() ?? ''}${word.slice(1)}`, more: text('note') };
    }
  }
}
