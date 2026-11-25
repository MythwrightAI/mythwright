// What the records of a table name: the records of the other tables its
// fields name, read so that a cell can say what it names by what that is
// called, and be chosen from them.
//
// Read once, and again when one of those is added, changed or taken away.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import { coalesce } from '../events.ts';
import { collections } from '../store.ts';
import { RENAMING, names, renames } from '../table.ts';
import type { Field, Named } from '../table.ts';
import { useEvents } from '../ui.tsx';

/** The records the fields of the table `own` name, by the table each is of; its own are left to whoever has them. */
export function useNamed(project: string, fields: Field[], own: string): Named {
  const naming = [...new Set(fields.filter((field) => names(field) && field.of[0] !== own).map((field) => field.of[0]!))].sort().join(' ');
  const [others, setOthers] = useState<Named>({});
  const read = useMemo(
    () =>
      coalesce(() => {
        if (naming === '') return;
        const all = collections.value[project] ?? [];
        void Promise.all(
          naming.split(' ').map(async (other) => {
            const of = all.find((each) => each.name === other);
            return of ? ([other, { fields: of.fields, rows: await api.records(project, other) }] as const) : null;
          }),
        )
          .then((found) => setOthers(Object.fromEntries(found.filter((each) => each !== null))))
          .catch(() => undefined);
      }, 50),
    [project, naming],
  );
  useEffect(read, [read]);
  useEvents(RENAMING, (event) => renames(event, project, naming.split(' ')) && read(), [project, read]);
  return others;
}
