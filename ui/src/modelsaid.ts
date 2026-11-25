// The name a model is said by where a message says which model wrote it
// (`views/channel.tsx`): the name its provider gives it, wherever the
// client knows that name, and the id its program reported only where no
// name is known. One function, so that the head of a message and whether
// it reads as one with the message before it (`continued`) go by the
// same words: two messages of one model said once by its id and once by
// its name were headed twice, each differently.
//
// Where no name is known, an id its program reported is said in a
// readable form of itself (`readable`: `maker-big-5-5` as `Maker Big
// 5.5`); one somebody chose is said as they wrote it. Why there is often
// none: a provider's program lists the models its own picker offers, and
// the picker offers them by what is typed to choose them (`default`, a
// family's short name), each with its name, while the program says of a
// turn the whole id of the model it used, which no entry of that list is.
// The list is as its program has it, and is not out of date: an agent
// told the short name is said by the name the list gives it, and one
// left to its provider's choice is said by the whole id its program
// reported, which only this makes readable.
//
// It holds nothing of the page, so that its tests can read it.

/**
 * What is known of the agent that wrote a message: what it was told to
 * run on, what that is called, and the id its program last said it used
 * (`model_reported`, which the agents' list hands beside the rest), which
 * is the model it was told to run on by the id its provider gives it.
 */
export interface RunsOn {
  adapter: string;
  model?: string | null;
  model_name?: string | null;
  model_reported?: string | null;
}

/** The models each provider was asked for, by the provider's name: each model's id and what the provider calls it. */
export type Lists = Readonly<Record<string, readonly { id: string; label: string }[]>>;

/**
 * What a model is said as: `id` is what the message says wrote it, or
 * nothing where it says nothing (it was posted by the agent's program
 * itself, or before models were put down), which is said as what the
 * agent runs on now (`now`). An id is said by the name the slot gives the
 * model it was told to run on, where that is this id or the id its
 * program said it used; else by what its provider's list calls it; else
 * as it is.
 */
export function modelSaid(id: string | null | undefined, slot: RunsOn | undefined, lists: Lists, now: string | null): string | null {
  if (!id) return now;
  const its = slot && (slot.model === id || slot.model_reported === id) ? slot.model_name : null;
  if (its && its !== id) return its;
  const listed = slot ? lists[slot.adapter]?.find((model) => model.id === id) : undefined;
  // What somebody chose for it is said as they wrote it; only an id its program reported is made readable.
  return listed?.label || (slot?.model === id ? id : readable(id));
}

/**
 * An id said so that it reads as a name, where nothing gives it one: each
 * word with a capital, and numbers that follow one another joined by a
 * dot, as a version is written (`maker-big-5-5` is `Maker Big 5.5`). Only
 * an id written as an id is: small letters and numbers, words joined by
 * dashes; anything else (a name, an alias of one word) is as it is.
 */
export function readable(id: string): string {
  if (!/^[a-z][a-z0-9]*(?:-[a-z0-9.]+)+$/.test(id)) return id;
  const words: string[] = [];
  for (const part of id.split('-')) {
    const last = words.at(-1);
    if (/^\d+$/.test(part) && last !== undefined && /^\d+(?:\.\d+)*$/.test(last)) words[words.length - 1] = `${last}.${part}`;
    else words.push(/^[a-z]/.test(part) ? part[0]!.toUpperCase() + part.slice(1) : part);
  }
  return words.join(' ');
}

/**
 * Whether the client has a name to say a model by: false for an id the
 * slot was not told and that no list it has holds, which its provider is
 * then asked for once (`learnModels`).
 */
export function knownBy(id: string, slot: RunsOn, lists: Lists): boolean {
  const its = (slot.model === id || slot.model_reported === id) && !!slot.model_name && slot.model_name !== id;
  return its || !!lists[slot.adapter]?.some((model) => model.id === id);
}

/**
 * The agents as they are once one of them is heard to have said which
 * model its program used (`agent.slot.updated` with `model`): known at
 * once, before the list is read again, since its answer is heard next and
 * names that model by its id. Read only later, the answer was said for
 * the moment between by its id made readable (`Stand Otter 5.5`) where
 * its provider's name for it (`Otter 5.5`) was there to be had.
 */
export function reportedNow<T extends { name: string }>(agents: T[], slot: unknown, model: unknown): T[] {
  if (typeof model !== 'string' || !model) return agents;
  return agents.map((one) => (one.name === slot ? { ...one, model_reported: model } : one));
}
