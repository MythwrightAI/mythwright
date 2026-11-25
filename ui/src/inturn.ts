// Changes to one thing, made one after another.
//
// A change is worked out from the thing as it is, sent, and answered with
// what the thing is from then on. Two asked for in the time one takes
// were each worked out from the thing as the page had drawn it: the
// second was sent without the first in it and took the first away. Two
// columns of a view switched in the time one save takes, which is no time
// at all on a busy machine, came back as a view with only the second
// switched. So a change is a function of the thing, done when every
// change asked before it has been answered, and handed what the last of
// them came to.
//
// And the thing is read as well as changed, by whoever follows what is
// said of it. A reading taken before a change was answered and coming in
// after it is of the thing as it was: kept, it would undo on the page
// what the change did until something else was said. Such a reading is
// not taken, and whoever asked reads again.

/** One thing that is read, and changed a change at a time. Nothing of the page is in it. */
export class InTurn<T> {
  private last: Promise<unknown> = Promise.resolve();
  private landed = 0;
  private thing: T;

  constructor(thing: T) {
    this.thing = thing;
  }

  /** What it is, as far as anybody here was told: what was last read of it, or what the last change came to. */
  get now(): T {
    return this.thing;
  }

  /** Said by whoever is about to read it: handed back with the reading (`read`), it says whether a change overtook it. */
  asked(): number {
    return this.landed;
  }

  /**
   * It was read. With no word of when the reading was asked for it is
   * taken as it is; with one (`asked`), only where no change has been
   * answered since. Whether it was taken: one that was not is read again.
   */
  read(thing: T, asked?: number): boolean {
    if (asked !== undefined && asked !== this.landed) return false;
    this.thing = thing;
    return true;
  }

  /**
   * Asks for a change. `change` is handed the thing as it is once every
   * change asked before this one has been answered, and answers with what
   * it then is; with nothing where it was refused, and the thing is as it
   * was. One that fails is no reason for those asked after it not to be
   * done.
   */
  change(change: (now: T) => Promise<T | null | undefined>): Promise<T | null> {
    const done = this.last.then(async () => {
      const came = await change(this.thing);
      if (came === null || came === undefined) return null;
      this.thing = came;
      this.landed += 1;
      return came;
    });
    this.last = done.catch(() => {});
    return done;
  }
}
