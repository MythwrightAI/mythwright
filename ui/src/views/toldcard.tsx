// A card of a told line's parts opened in the drawer beside the line's
// page: its own form and history, as the drawer beside its board has them
// (`cardform.tsx`), with the page staying where it is. What the form has
// to know of the board is read for itself (`useAbout`), as the Tools
// panel's is, and read again as the board's cards change.

import { useEffect, useMemo, useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Board } from '../api.ts';
import { goneBy } from '../drawerkept.ts';
import { coalesce } from '../events.ts';
import { drawer } from '../store.ts';
import { useEvents } from '../ui.tsx';
import { CardForm, useAbout } from './cardform.tsx';

/** Opens the card `id` of `project`'s board in the drawer. */
export function openToldCard(project: string, id: string): void {
  drawer.value = { title: id, body: () => <ToldCard key={id} project={project} id={id} />, gone: goneBy(() => api.record(id)) };
}

function ToldCard({ project, id }: { project: string; id: string }) {
  const [board, setBoard] = useState<Board | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const read = useMemo(
    () =>
      coalesce(
        () =>
          void api.board(project).then(setBoard, (error: unknown) => setFailed(error instanceof Error ? error.message : String(error))),
        50,
      ),
    [project],
  );
  useEffect(read, [read]);
  useEvents(['board.item.*'], (event) => event.data?.project === project && read(), [project, read]);
  const about = useAbout(project, board);
  if (failed) return <p class="faint" role="alert">{failed}</p>;
  const item = board?.items.find((one) => one.id === id);
  if (board && !item) return <p class="faint">There is no card {id} on the board of {project}. It may have been taken away.</p>;
  if (!item || !about) return <p class="faint">Reading the board</p>;
  return <CardForm key={item.id} item={item} about={about} onChanged={read} onDone={() => (drawer.value = null)} />;
}
