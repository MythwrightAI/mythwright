// The keys the client answers to, in a panel over the page: asked for with
// Ctrl (or ⌘), Shift and `/`.

import { desktop } from '../desktop.ts';
import { SHORTCUTS, shown } from '../shortcuts.ts';
import { shortcutsOpen } from '../store.ts';
import { Modal } from '../ui.tsx';

export function Shortcuts() {
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  return (
    <Modal wide onClose={() => (shortcutsOpen.value = false)}>
      <div class="shortcuts-head">Keyboard shortcuts</div>
      <div class="shortcuts">
        {SHORTCUTS.map((section) => (
          <section key={section.title}>
            <h3>{section.title}</h3>
            {section.rows.filter((row) => !row.desktop || desktop !== null).map((row) => (
              <div key={row.does} class="shortcut">
                <span>{row.does}</span>
                <span class="keys">
                  {row.keys.map((key) => (
                    <kbd key={key} class="key">
                      {shown(key, mac)}
                    </kbd>
                  ))}
                </span>
              </div>
            ))}
          </section>
        ))}
      </div>
      <div class="palette-foot">
        <span class="grow">Keys that are held together are side by side.</span>
        <span class="kbd">Esc</span>
        <span>close</span>
      </div>
    </Modal>
  );
}
