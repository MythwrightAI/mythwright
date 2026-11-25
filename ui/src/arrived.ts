// What arrives while something is being read.
//
// The client reads a thing once and then follows what is said of it. The
// two cross: something said after the daemon took the reading and before
// the reading is here is in neither, unless it is kept meanwhile. Kept, it
// is put with what was read, which is told from what it already holds by
// each thing's id.

/** What was read, and after it whatever arrived meanwhile that is not in it. */
export function withArrived<M extends { id: string }>(read: M[], arrived: M[]): M[] {
  const held = new Set(read.map((entry) => entry.id));
  const more: M[] = [];
  for (const entry of arrived) {
    if (held.has(entry.id)) continue;
    held.add(entry.id);
    more.push(entry);
  }
  // What arrived was said after the reading was taken, so it comes after.
  return more.length ? [...read, ...more] : read;
}

/** Keeps what arrives for a thing, by the thing's key, while a reading of it is on its way. */
export class Arriving<M> {
  private readonly readings = new Map<string, { under_way: number; arrived: M[] }>();

  /** Something arrived for `key`: kept if it is being read, and otherwise whoever follows it has it already. */
  arrive(key: string, entry: M): void {
    this.readings.get(key)?.arrived.push(entry);
  }

  /**
   * Reads `key`, and hands back what was read with everything that arrived
   * for it since the earliest reading of it still on its way: of two
   * readings side by side the later to come in may have been taken first.
   * Whoever asked puts the two together before anything else can arrive.
   */
  async during<T>(key: string, read: () => Promise<T>): Promise<[T, M[]]> {
    const reading = this.readings.get(key) ?? { under_way: 0, arrived: [] };
    this.readings.set(key, reading);
    reading.under_way += 1;
    try {
      const got = await read();
      return [got, [...reading.arrived]];
    } finally {
      reading.under_way -= 1;
      if (!reading.under_way) this.readings.delete(key);
    }
  }
}
