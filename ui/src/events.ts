// The event stream: the one way the client learns that anything changed.
//
// One connection for the whole app. Views subscribe to topics here and read
// what they need once when they open; nothing in the client asks "anything
// new?" on a timer. (On the system this replaces, forgotten browser tabs
// doing exactly that were most of the load on the hub.)

import { shown } from './api.ts';
import type { FleetEvent } from './api.ts';
import { standingAfter } from './standing.ts';
import type { Heard, Standing } from './standing.ts';

export type Handler = (event: FleetEvent) => void;

/**
 * Each pattern and topic in its words, split once: every event is matched
 * against every pattern of every subscription, and with a fleet at work
 * that is some hundreds of events a second.
 */
const words = new Map<string, string[]>();
function wordsOf(said: string): string[] {
  let split = words.get(said);
  if (!split) {
    // Kept for the patterns and topics there are, and begun again where there have come to be a great many.
    if (words.size > 2000) words.clear();
    words.set(said, (split = said.split('.')));
  }
  return split;
}

/** Whether `topic` matches `pattern`: `*` is one word, a final `**` is the rest. */
export function topicMatches(pattern: string, topic: string): boolean {
  if (pattern === '**' || pattern === topic) return true;
  const want = wordsOf(pattern);
  const have = wordsOf(topic);
  for (let i = 0; i < want.length; i++) {
    if (want[i] === '**') return i === want.length - 1;
    if (i >= have.length) return false;
    if (want[i] !== '*' && want[i] !== have[i]) return false;
  }
  return want.length === have.length;
}

interface Subscription {
  patterns: string[];
  handler: Handler;
  /** Reads again what it shows of what it follows, where the stream says it may have missed some of it. */
  again?: () => void;
}

/**
 * Whether a subscription to `patterns` follows any of `topics`: one of the
 * topics the stream said it may have missed (`behind`), which the daemon
 * passes on as it says them and does not keep, so that nothing can hand
 * them over again.
 */
export function followsAny(patterns: string[], topics: string[]): boolean {
  return patterns.some((pattern) => topics.some((topic) => topicMatches(pattern, topic)));
}

export class Stream {
  private source: EventSource | null = null;
  private subscriptions = new Set<Subscription>();
  private statusListeners = new Set<(up: boolean) => void>();
  private standingListeners = new Set<(standing: Standing) => void>();
  /** Not yet asked for, arriving, or asked for and not arriving (`standing.ts`). */
  standing: Standing = 'waiting';
  /** The place after the last stored event seen, as the daemon wrote it: text handed back and never read. A new connection goes on from it. */
  at = '';
  up = false;

  /** Starts listening. `after` is the place to go on from, when one was handed out (`cursor`). */
  open(after?: string): void {
    this.close();
    if (after !== undefined) this.at = after;
    const since = this.at ? `?since=${encodeURIComponent(this.at)}` : '';
    // Its address says which page asks: the browser opens it by itself, and can send no key with it.
    const source = new EventSource(shown(`/api/events${since}`), { withCredentials: true });
    this.source = source;
    source.onopen = () => this.heard('opened');
    // The browser reconnects by itself and sends the last id it saw, so
    // nothing is missed; all there is to do here is say so.
    source.onerror = () => this.heard('failed');
    // What the daemon passed on as it said it and did not keep is in
    // nothing a stream that goes on from a place can hand over: where it
    // may have missed some, it says which topics, and whatever shows
    // something it follows on one of them reads it again.
    source.addEventListener('behind', (message) => {
      let topics: string[] = [];
      try {
        topics = JSON.parse((message as MessageEvent).data).topics ?? [];
      } catch {
        return;
      }
      this.behind(topics);
    });
    source.onmessage = (message) => {
      let event: FleetEvent;
      try {
        event = JSON.parse(message.data);
      } catch {
        return;
      }
      // Only a stored event says a place; a passing line says none.
      if (message.lastEventId) this.at = message.lastEventId;
      this.deliver(event);
    };
  }

  close(): void {
    this.source?.close();
    this.source = null;
    this.heard('closed');
  }

  /**
   * Calls `handler` for every event whose topic matches one of `patterns`,
   * and `again` where the stream says it may have missed some of them.
   */
  on(patterns: string | string[], handler: Handler, again?: () => void): () => void {
    const subscription = { patterns: Array.isArray(patterns) ? patterns : [patterns], handler, again };
    this.subscriptions.add(subscription);
    return () => this.subscriptions.delete(subscription);
  }

  onStatus(listener: (up: boolean) => void): () => void {
    this.statusListeners.add(listener);
    return () => this.statusListeners.delete(listener);
  }

  deliver(event: FleetEvent): void {
    for (const { patterns, handler } of this.subscriptions) {
      if (patterns.some((pattern) => topicMatches(pattern, event.topic))) {
        try {
          handler(event);
        } catch (err) {
          console.error('an event handler failed', event.topic, err);
        }
      }
    }
  }

  /** Tells each subscription that follows any of `topics`, and says how to read again, to read again. */
  behind(topics: string[]): void {
    for (const { patterns, again } of this.subscriptions) {
      if (!again || !followsAny(patterns, topics)) continue;
      try {
        again();
      } catch (err) {
        console.error('reading again failed', err);
      }
    }
  }

  /** How the stream stands is told to whoever asked, each time it changes. */
  onStanding(listener: (standing: Standing) => void): () => void {
    this.standingListeners.add(listener);
    return () => this.standingListeners.delete(listener);
  }

  private heard(heard: Heard): void {
    const standing = standingAfter(heard);
    if (standing !== this.standing) {
      this.standing = standing;
      for (const listener of this.standingListeners) listener(standing);
    }
    this.setUp(standing === 'live');
  }

  private setUp(up: boolean): void {
    if (this.up === up) return;
    this.up = up;
    for (const listener of this.statusListeners) listener(up);
  }
}

export const stream = new Stream();

/**
 * Runs `work` once after a burst of calls: an agent writing twenty files is
 * one refresh of the tree, not twenty.
 */
export function coalesce(work: () => void, wait = 60): () => void {
  let pending: ReturnType<typeof setTimeout> | null = null;
  return () => {
    if (pending !== null) return;
    pending = setTimeout(() => {
      pending = null;
      work();
    }, wait);
  };
}
