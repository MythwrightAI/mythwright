// Taking the package that came with this version in place of the installed
// copy: the question asked first, and the taking.
//
// It is asked from the side panel's line of a package and from the
// package's own page, and drawn once, by the frame. What is said of a
// package that differs, and to whom this is offered, is `came.ts`.

import { signal } from '@preact/signals';
import { useState } from 'preact/hooks';
import { api } from '../api.ts';
import type { Extension } from '../api.ts';
import { asked } from '../came.ts';
import { meta, toast } from '../store.ts';
import { Modal, Prose } from '../ui.tsx';

/** The package somebody is being asked about, as it was when they pressed. */
const asking = signal<Extension | null>(null);

/** Asks whether the one that came is to be taken in place of `extension`. */
export function askUpdate(extension: Extension): void {
  asking.value = extension;
}

/** The question, where one is being asked. */
export function AskUpdate() {
  const extension = asking.value;
  return extension ? <Update key={extension.name} extension={extension} /> : null;
}

function Update({ extension }: { extension: Extension }) {
  const [busy, setBusy] = useState(false);
  const [wrong, setWrong] = useState('');
  const title = extension.title || extension.name;
  const close = () => {
    asking.value = null;
  };
  const take = async () => {
    setBusy(true);
    try {
      // The lists follow what the daemon says of a package installed: nothing is read again here.
      const taken = await api.updateExtension(extension.name);
      toast(taken.updated ? `${title} is now the one that came with this version.` : `${title} was already the one that came with this version.`);
      close();
    } catch (err) {
      // Said where it was asked, with the question still there.
      setWrong(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  };
  return (
    <Modal onClose={close}>
      <div class="stack ext-asked" data-extension={extension.name}>
        <h2>Update {title}?</h2>
        {asked(extension, meta.value?.name ?? 'the program').map((line) => (
          <p key={line}>
            <Prose text={line} />
          </p>
        ))}
        {wrong && <p class="error-text">{wrong}</p>}
        <div class="row modal-foot">
          <button type="button" onClick={close}>
            Cancel
          </button>
          <button class="primary" disabled={busy} onClick={() => void take()}>
            Update
          </button>
        </div>
      </div>
    </Modal>
  );
}
