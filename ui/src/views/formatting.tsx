// The composer's tools, in a row under what is being written, beside the
// button that attaches a file.
//
// Each changes the text in the box (`format.ts` says how) and leaves the
// caret in it: pressing one does not take the caret out, so what was
// selected is still selected when the tool is used on it.

import type { ComponentChildren, RefObject } from 'preact';
import { useEffect, useRef, useState } from 'preact/hooks';
import { format } from '../format.ts';
import type { Tool, Written } from '../format.ts';
import { shown } from '../shortcuts.ts';
import { classes } from '../util.ts';
import { besideTool, Floating } from './offered.tsx';

/**
 * Makes what is in the box `next`. As though it had been typed, where the
 * browser can do that: then undoing takes the tool's work back like any
 * other writing.
 */
export function write(area: HTMLTextAreaElement, next: Written): void {
  const was = area.value;
  // Only what differs is written: what is the same before it and after it stays.
  let head = 0;
  while (head < was.length && head < next.text.length && was[head] === next.text[head]) head++;
  let tail = 0;
  while (tail < was.length - head && tail < next.text.length - head && was[was.length - 1 - tail] === next.text[next.text.length - 1 - tail]) tail++;
  const put = next.text.slice(head, next.text.length - tail);
  area.focus();
  if (was !== next.text) {
    area.setSelectionRange(head, was.length - tail);
    const typed = put === '' ? document.execCommand('delete') : document.execCommand('insertText', false, put);
    if (!typed || area.value !== next.text) {
      area.value = next.text;
      area.dispatchEvent(new Event('input', { bubbles: true }));
    }
  }
  area.setSelectionRange(next.from, next.to);
}

/** Uses a tool on what is selected in the box. */
export function use(area: HTMLTextAreaElement | null, tool: Tool): void {
  if (area) write(area, format(tool, { text: area.value, from: area.selectionStart, to: area.selectionEnd }));
}

const line = { fill: 'none', stroke: 'currentColor', 'stroke-width': 1.4, 'stroke-linecap': 'round', 'stroke-linejoin': 'round' } as const;
const Mark = ({ children }: { children: ComponentChildren }) => (
  <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" {...line}>
    {children}
  </svg>
);

/** A tool: what it is called, the keys for it, and how it is drawn. */
const TOOLS: [Tool, string, string[], ComponentChildren][] = [
  ['bold', 'Bold', ['Mod', 'B'], <b>B</b>],
  ['italic', 'Italic', ['Mod', 'I'], <i>I</i>],
  ['underline', 'Underline', ['Mod', 'U'], <u>U</u>],
  ['strike', 'Strikethrough', ['Mod', 'Shift', 'X'], <s>S</s>],
  [
    'link',
    'Link',
    ['Mod', 'Shift', 'U'],
    <Mark>
      <path d="M6.8 9.2a2.6 2.6 0 0 0 3.7 0l2-2a2.6 2.6 0 0 0-3.7-3.7l-.7.7M9.2 6.8a2.6 2.6 0 0 0-3.7 0l-2 2a2.6 2.6 0 0 0 3.7 3.7l.7-.7" />
    </Mark>,
  ],
  [
    'numbers',
    'Numbered list',
    ['Mod', 'Shift', '7'],
    <Mark>
      <path d="M7 4h6.5M7 8h6.5M7 12h6.5M2.6 3.2l1-.6v3M2.4 10.4c.3-.5 1.7-.6 1.7.3 0 .7-1.7 1.3-1.7 2.1h1.8" />
    </Mark>,
  ],
  [
    'bullets',
    'Bulleted list',
    ['Mod', 'Shift', '8'],
    <Mark>
      <path d="M7 4h6.5M7 8h6.5M7 12h6.5" />
      <path d="M3.3 4h.01M3.3 8h.01M3.3 12h.01" stroke-width="2.4" />
    </Mark>,
  ],
  [
    'quote',
    'Quotation',
    ['Mod', 'Shift', '9'],
    <Mark>
      <path d="M3 3v10" stroke-width="2" />
      <path d="M6.5 4.5h7M6.5 8h7M6.5 11.5h4.5" />
    </Mark>,
  ],
  [
    'code',
    'Code',
    ['Mod', 'Shift', 'C'],
    <Mark>
      <path d="m5.2 4.5-3.4 3.5 3.4 3.5M10.8 4.5l3.4 3.5-3.4 3.5M9 3l-2 10" />
    </Mark>,
  ],
  [
    'block',
    'Block of code',
    ['Mod', 'Alt', 'Shift', 'C'],
    <Mark>
      <rect x="1.8" y="2.5" width="12.4" height="11" rx="2" />
      <path d="m6.4 6.2-1.9 1.8 1.9 1.8M9.6 6.2l1.9 1.8-1.9 1.8" />
    </Mark>,
  ],
];

/** What a line can be, behind one button. */
const LINES: [Tool, string][] = [
  ['text', 'Text'],
  ['h1', 'Heading 1'],
  ['h2', 'Heading 2'],
  ['h3', 'Heading 3'],
];

const keys = (held: string[]) => {
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  return held.map((key) => shown(key, mac)).join(mac ? ' ' : '+');
};
// Not to take the caret out of the box, which a press on a button would.
const stay = (event: Event) => event.preventDefault();

export function Formatting({ area }: { area: RefObject<HTMLTextAreaElement | null> }) {
  const [open, setOpen] = useState(false);
  const menu = useRef<HTMLSpanElement>(null);
  // The menu is on the page's body (`offered.tsx`): a press in it is not one outside.
  const kinds = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const away = (event: Event) => {
      const inside = menu.current?.contains(event.target as Node) || kinds.current?.contains(event.target as Node);
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !inside) setOpen(false);
    };
    window.addEventListener('pointerdown', away);
    window.addEventListener('keydown', away);
    return () => {
      window.removeEventListener('pointerdown', away);
      window.removeEventListener('keydown', away);
    };
  }, [open]);
  const button = ([tool, name, held, mark]: (typeof TOOLS)[number]) => (
    <button key={tool} type="button" class="ghost icon" aria-label={name} data-tip={`${name} (${keys(held)})`} onMouseDown={stay} onClick={() => use(area.current, tool)}>
      {mark}
    </button>
  );
  return (
    <div class="format" role="toolbar" aria-label="Formatting">
      {TOOLS.slice(0, 4).map(button)}
      <span class="format-lines" ref={menu}>
        <button type="button" class={classes('ghost icon wide', open && 'on')} aria-label="Kind of line" data-tip="Text, or a heading" aria-haspopup="menu" aria-expanded={open} onMouseDown={stay} onClick={() => setOpen(!open)}>
          ¶<span class="faint">▾</span>
        </button>
        {open && (
          // Over what is above it, moving nothing, and over whatever the composer is drawn in: the tools are at the foot of the window.
          <Floating beside={() => besideTool(menu.current?.querySelector('button') ?? null)} own={kinds} class="format-menu" role="menu" label="Kind of line">
            {LINES.map(([tool, name]) => (
              <button
                key={tool}
                type="button"
                role="menuitem"
                class={`ghost line-${tool}`}
                onMouseDown={stay}
                onClick={() => {
                  setOpen(false);
                  use(area.current, tool);
                }}
              >
                {name}
              </button>
            ))}
          </Floating>
        )}
      </span>
      <span class="format-rule" />
      {TOOLS.slice(4, 7).map(button)}
      <span class="format-rule" />
      {TOOLS.slice(7).map(button)}
    </div>
  );
}
