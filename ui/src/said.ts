// What is said in channels, as the client holds it: a channel's messages
// and what is under one of them are each read once, when first shown,
// and added to from then on as things are said.
import { signal } from '@preact/signals';
import { api } from './api.ts';
import type { Message } from './api.ts';
import { Arriving, withArrived } from './arrived.ts';
import { placedIn } from './posted.ts';

/** What has been said in each channel that has been read, by the channel's id. */
export const messages = signal<Record<string, Message[]>>({});
/** What has been said under a message, by that message's id, for the threads that have been opened. */
export const threads = signal<Record<string, Message[]>>({});

// What is said in a channel, and under a message, while either is being
// read: the reading was taken before it and comes in after it, and without
// this it was in neither. A channel opened as something was said there
// never showed it.
const saidMeanwhile = new Arriving<Message>();
const repliedMeanwhile = new Arriving<Message>();

export async function loadMessages(channel: string): Promise<void> {
  const [loaded, arrived] = await saidMeanwhile.during(channel, () => api.messages(channel));
  const said = withArrived(loaded, arrived.filter((message) => !message.thread));
  messages.value = { ...messages.value, [channel]: said };
  // What was said under a message meanwhile may or may not be in the count
  // that was read with it: what is under it says.
  const under = new Set(arrived.flatMap((message) => (message.thread ? [message.thread] : [])));
  // A count that could not be read again stays as it was read: nothing
  // is lost by that, and the thread says how many when it is opened.
  for (const root of under) void recount(channel, root).catch(() => {});
}

/** What has been said under a message, with what was said there while it was read. */
async function repliesTo(channel: string, root: string): Promise<Message[]> {
  const [all, arrived] = await repliedMeanwhile.during(root, () => api.thread(channel, root));
  return withArrived(all.filter((message) => message.id !== root), arrived);
}

/** Has a message say how many are under it, from what is under it. */
async function recount(channel: string, root: string): Promise<void> {
  const replies = (await repliesTo(channel, root)).length;
  const known = messages.value[channel];
  if (!known) return;
  const counted = known.map((entry) => (entry.id === root ? { ...entry, replies } : entry));
  messages.value = { ...messages.value, [channel]: counted };
}

/** Takes in something said, as the daemon said it happened: into its channel if that has been read, and under its message. */
export function addMessage(message: Message): void {
  const known = messages.value[message.channel];
  saidMeanwhile.arrive(message.channel, message);
  if (message.thread) {
    repliedMeanwhile.arrive(message.thread, message);
    // Said under a message: it joins that thread, if the thread is open,
    // and the message it is under counts one more.
    const under = threads.value[message.thread];
    if (under?.some((entry) => entry.id === message.id)) return;
    if (under) threads.value = { ...threads.value, [message.thread]: [...under, message] };
    if (known) {
      const counted = known.map((entry) => (entry.id === message.thread ? { ...entry, replies: (entry.replies ?? 0) + 1 } : entry));
      messages.value = { ...messages.value, [message.channel]: counted };
    }
    return;
  }
  if (!known || known.some((entry) => entry.id === message.id)) return;
  messages.value = { ...messages.value, [message.channel]: [...known, message] };
}

/** Reads a thread: what has been said under a message. */
export async function loadThread(channel: string, root: string): Promise<void> {
  const under = await repliesTo(channel, root);
  threads.value = { ...threads.value, [root]: under };
}

/** Takes in that something attached to a message was made a file of a project: the message names that file from then on, wherever it is held. */
export function placedFile(data: { message?: string; attachment?: string; project?: string; path?: string; channel?: string }): void {
  const { message, attachment, project, path, channel } = data;
  if (!message || !attachment || !project || !path) return;
  const changed = (said: Message[]) => said.map((one) => (one.id === message ? placedIn(one, attachment, project, path) : one));
  if (channel && messages.value[channel]) messages.value = { ...messages.value, [channel]: changed(messages.value[channel]!) };
  threads.value = Object.fromEntries(Object.entries(threads.value).map(([root, said]) => [root, changed(said)]));
}
