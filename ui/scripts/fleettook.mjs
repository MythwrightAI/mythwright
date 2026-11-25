// A fleet having taken what was last written to one of its tables, for
// the checks that change a fleet's rows from the terminal and then go on.
//
// A change to a row asks the fleet to bring itself to its tables, and what
// does that says `fleet.arranged` with what it was asked for: `row` and the
// number of the change. Counting arrangements said since does not do: one
// asked for by an earlier change may end after the write, having read the
// tables before it, and of two changes made together the first may be
// called off by the second, which then takes both, so one arrangement is
// said for two. So a check waits for one asked for by the last change, or
// by a later one of the same project (the API suite's `taken.rs`).

const TABLES = ['agents', 'stages', 'segments', 'actions'];

/** Waits for `project`'s fleet to have taken the last change written to one of its tables. */
export async function fleetTook(cli, project) {
  const said = async (topics, limit) =>
    (await cli('events', '--topics', topics, '--limit', String(limit), '--json')).stdout
      .trim()
      .split('\n')
      .filter(Boolean)
      .map((line) => JSON.parse(line));
  const written = (await said('record.*', 200)).filter((event) => event.data.project === project && TABLES.includes(event.data.collection)).at(-1);
  if (!written) throw new Error(`nothing was written to a table of ${project}'s fleet`);
  const asked = (event) => (event.data.asked ?? '').startsWith('row ') && Number(event.data.asked.slice(4)) >= written.seq;
  for (let tries = 0; ; tries += 1) {
    if ((await said('fleet.arranged', 100)).some((event) => event.data.project === project && asked(event))) return;
    if (tries > 300) throw new Error(`${project}'s fleet never took the change of its rows numbered ${written.seq}`);
    await new Promise((later) => setTimeout(later, 200));
  }
}
