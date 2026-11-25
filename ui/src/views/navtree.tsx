// The lines of the side panel's trees: one that opens something, and one
// that folds what is under it. A line that opens something says what
// (`data-names`), so that pressed with Ctrl or ⌘ or the middle button it is
// opened in a tab of the browser's own (`address.ts`).
//
// They are apart from the frame (`app.tsx`) because more than one part of
// the panel draws them: a project's tree, and the factory's blueprints
// (`views/blueprintlist.tsx`).

import type { ComponentChildren, HTMLAttributes } from 'preact';
import { useEffect } from 'preact/hooks';
import { startsUnfolded } from '../folds.ts';
import { names } from '../inplace.ts';
import { active, openTab, tabKey, turn, turned } from '../store.ts';
import type { Tab } from '../store.ts';
import { Twist } from '../ui.tsx';
import { classes } from '../util.ts';

/** A line that opens something: its mark, what it is called, and a count or a few faint words at its end where it has either. */
export function NavItem(props: { tab: Tab; glyph: ComponentChildren; label: string; count?: number; title?: string; hint?: string }) {
  const key = tabKey(props.tab);
  return (
    <button class="nav-item" aria-current={active.value === key} data-names={names(props.tab)} title={props.title} onClick={() => openTab(props.tab)}>
      <span class="glyph">{props.glyph}</span>
      <span class="truncate">{props.label}</span>
      {props.count ? <span class="count">{props.count}</span> : null}
      {props.hint ? <span class="nav-hint">{props.hint}</span> : null}
    </button>
  );
}

/**
 * Something in a tree that folds: an arrow, a name, and what is
 * under it. It starts folded unless it is said to start `unfolded`, and what it was last left as is remembered
 * under `at`. One that has a page of its own (`tab`) is like a project:
 * its name opens the page, and the arrow beside it does the folding.
 * One that is `held` open is open whatever was remembered, and its arrow
 * does nothing and changes nothing that is kept: what is under it is
 * there because something else says so (what was typed in a box), and is
 * as it was left when that is over. A few faint words may stand at the
 * end of its line (`hint`), and a menu after it (`more`: what can be made in a
 * folder of a project's repo). One a project's outline can place says
 * what it is there (`place`), which is what carrying it goes by.
 */
export function Fold(props: {
  at: string;
  label: string;
  tab?: Tab;
  called?: string;
  unfolded?: boolean;
  held?: boolean;
  hint?: string;
  /** How many wait unread under it, said beside its name while it is folded, whether or not it has a page. */
  count?: number;
  title?: string;
  onOpen?: () => void;
  more?: ComponentChildren;
  place?: string;
  children: ComponentChildren;
}) {
  // What is remembered is that it was turned from how it starts.
  const open = props.held || turned.value.has(props.at) !== !!props.unfolded;
  // What is under it may need reading, which is done when it is first
  // shown: now, or when the page comes back with it already unfolded. One that is only held open by what was typed
  // reads nothing: a box narrows what was read, and asks for nothing more.
  useEffect(() => {
    if (open && !props.held) props.onOpen?.();
  }, [open]);
  const fold = () => {
    if (!props.held) turn(props.at);
  };
  const under = open && <div class="tree">{props.children}</div>;
  const hint = props.hint ? <span class="nav-hint">{props.hint}</span> : null;
  if (props.tab) {
    const tab = props.tab;
    const current = active.value === tabKey(tab);
    const called = props.called ?? props.label;
    return (
      <div data-place={props.place}>
        <div class={classes('nav-row', 'inner', current && 'current')}>
          <button class="nav-twist" aria-label={open ? `Fold ${called}` : `Unfold ${called}`} aria-expanded={open} aria-disabled={props.held || undefined} onClick={fold}>
            <Twist open={open} />
          </button>
          <button class="nav-item" aria-current={current} data-names={names(tab)} title={props.title} onClick={() => openTab(tab)}>
            <span class="truncate">{props.label}</span>
            {hint}
            {!open && props.count ? <span class="count">{props.count}</span> : null}
          </button>
          {props.more}
        </div>
        {under}
      </div>
    );
  }
  const line = (
    <button class="nav-item" aria-expanded={open} aria-disabled={props.held || undefined} title={props.title} onClick={fold}>
      <span class="glyph">
        <Twist open={open} />
      </span>
      <span class="truncate">{props.label}</span>
      {hint}
      {!open && props.count ? <span class="count">{props.count}</span> : null}
    </button>
  );
  return (
    <div data-place={props.place}>
      {props.more ? (
        <div class="nav-line">
          {line}
          {props.more}
        </div>
      ) : (
        line
      )}
      {under}
    </div>
  );
}

/**
 * A part of a panel under its heading, folded as anything in the tree
 * is: an arrow beside its name, which folds what is under it, kept under
 * `at` as having been turned from how it starts (folded, but for the
 * three that start unfolded: `startsUnfolded`). One that is
 * `held` open is open whatever was kept, as a `Fold` is. What else stands
 * in the heading (`act`, a button that adds) is beside the name and is
 * no part of the fold. What is under it is a tree hung from the heading's
 * arrow, as what is under a project hangs from the project's:
 * the same lines, one step in. A part whose lines are
 * held together for a reason of its own (what was lately opened, held
 * still under the pointer) says what the tree is besides (`under`), so
 * that each line is still a thing of the tree.
 */
export function Section(props: {
  at: string;
  label: string;
  held?: boolean;
  act?: ComponentChildren;
  under?: Omit<HTMLAttributes<HTMLDivElement>, 'class'> & { class?: string };
  children?: ComponentChildren;
}) {
  // Kept is that it was turned from how it starts: Projects, Factory and Blueprints start unfolded (`folds.ts`).
  const open = props.held || turned.value.has(props.at) !== startsUnfolded(props.at);
  return (
    <>
      <h3 class="nav-section">
        <button
          class="section-fold"
          aria-label={open ? `Fold ${props.label}` : `Unfold ${props.label}`}
          aria-expanded={open}
          aria-disabled={props.held || undefined}
          onClick={() => !props.held && turn(props.at)}
        >
          <Twist open={open} />
          <span class="truncate">{props.label}</span>
        </button>
        {props.act}
      </h3>
      {open && (
        <div {...props.under} class={classes('tree', props.under?.class)}>
          {props.children}
        </div>
      )}
    </>
  );
}
