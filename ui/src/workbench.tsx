// The middle of the window: the open tabs, in groups that can be put side by
// side or one above the other, and the view showing in each group.
//
// Everything is drawn flat and placed by `places`: a row of tabs for each
// group, then every view that is showing, then the dividers. A view is a
// child of the whole area and not of its group, so moving a tab to another
// group moves its view on the screen without making it again.
//
// In a browser there is no row of tabs: one thing shows at a time, in
// place, and the browser's own tabs are the tabs (`inplace.ts`). What is
// over the thing there is a bar that holds what brings the side panel back
// while it is folded away, with what is shown, and nothing else.
//
// A tab is dragged with the pointer and not with the browser's own drag and
// drop: that one behaves differently in each of the windows the desktop app
// is drawn in, and cannot be followed over a framed page at all.

import { effect, signal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { lazy, Suspense } from 'preact/compat';
import { useEffect, useLayoutEffect, useRef } from 'preact/hooks';
import { api } from './api.ts';
import { aim, groups, holding, landing, places } from './panes.ts';
import type { Box, Divider, Group, Zone } from './panes.ts';
import { inView, oneRow, phone } from './phone.ts';
import {
  attempt,
  channels,
  closeTab,
  closing,
  dockTab,
  draftOf,
  edited,
  focusGroup,
  keepDraft,
  collections,
  focused,
  isBlueprint,
  layout,
  leaveFor,
  leaving,
  meta,
  partMissing,
  placeTab,
  projects,
  rememberLayout,
  resizeSplit,
  select,
  side,
  tabKey,
  tabs,
  unread,
} from './store.ts';
import type { Tab } from './store.ts';
import { CORNER, cornered, grab } from './titlebar.tsx';
import { Empty, Modal, SideFold } from './ui.tsx';
import { titled } from './running.ts';
import { whatIs } from './views/running.tsx';
import { Returned } from './views/returned.tsx';
import { forgetReturned } from './returned.ts';
import { channelsWait } from './conversed.ts';
import { lineSentence, lineTitle, readsInbox, waiting as inboxWaits } from './inboxed.ts';
import { classes } from './util.ts';
import { barShown, inPlace } from './inplace.ts';
import { titleShowing } from './profiles.ts';
import { Activity } from './views/activity.tsx';
import { BoardView } from './views/board.tsx';
import { LibraryView } from './views/library.tsx';
import { MockupsView } from './views/mockups.tsx';
import { RecordsView } from './views/records.tsx';
import { ChannelView } from './views/channel.tsx';
import { ExtensionView } from './views/extensions.tsx';
import { FileView, ShippedBlueprint } from './views/file.tsx';
import { Home } from './views/home.tsx';
import { InstanceView } from './views/instance.tsx';
import { PipelineRun } from './views/pipelinerun.tsx';
import { PipelinesView } from './views/pipelines.tsx';
import { runTitle } from './runread.ts';
import { Metrics } from './views/metrics.tsx';
import { ProjectHome } from './views/project.tsx';
import { GIT_GLYPH } from './gitpage.ts';
import { GitView } from './views/gitpage.tsx';
import { RepoView } from './views/repo.tsx';
import { ProjectEnvironments } from './views/projectenvs.tsx';
import { Profile, Settings } from './views/settings.tsx';
import { MembersView } from './views/members.tsx';
import { Secrets } from './views/secrets.tsx';
import { Releases } from './views/releases.tsx';
import { AboutView } from './views/about.tsx';
import { InboxPage } from './views/inboxpage.tsx';
import { TopicPage } from './views/topicpage.tsx';
import { TOPIC_GLYPH } from './views/topics.tsx';
import { titleOf } from './topicsread.ts';
import { TrashView } from './views/trash.tsx';
import { calledOf } from './kept.ts';
import { signs } from './views/signs.tsx';
import { headWord } from './views/thingwas.tsx';

// The Reference is read by whoever asks what a word means, and by nobody else: it is fetched when opened.
const Reference = lazy(() =>
  import('./views/reference.tsx').catch((err) => {
    partMissing(err);
    return { default: () => <Empty title="The Reference could not be loaded">Reload the page to carry on.</Empty> };
  }),
);

/** How tall a row of tabs is, in pixels. The stylesheet is told, as `--strip`. */
const STRIP = 37;
/** How far a pressed tab must be moved before it is being dragged and not clicked. */
const SLACK = 5;
/** The least a divider leaves a group, in pixels across and down. */
const NARROWEST = 320;
const SHORTEST = 160;
/** Nearer the edge of the area than this, as a share of it, is at the edge. */
const EDGE = 1e-6;
/** How wide the name carried beside the pointer is, in pixels. */
const GHOST = 180;

/** A tab being dragged: where the pointer is in the window, and what letting go there would do. */
interface Drag {
  key: string;
  x: number;
  y: number;
  to: { group: string; over: 'tabs' | Zone; before: string | null } | null;
}
const drag = signal<Drag | null>(null);
/** The tab the pointer went down on, and where. */
let pressed: { key: string; x: number; y: number } | null = null;
/** A drag ends with a click on the tab that was dragged, which is not a click on it. */
let swallow = false;
/** The element everything here is placed in. */
let area: HTMLElement | null = null;

const percent = (share: number) => `${share * 100}%`;

/** What letting go of a tab at a point in the window would do. */
function aimed(x: number, y: number, key: string): Drag['to'] {
  const rect = area?.getBoundingClientRect();
  if (!rect) return null;
  const hit = aim(layout.value, (x - rect.left) / rect.width, (y - rect.top) / rect.height, STRIP / rect.height);
  if (!hit) return null;
  if (hit.over === 'tabs') {
    // Put down on a tab it goes before that tab; past the last of them, at the end.
    const under = document.elementFromPoint(x, y)?.closest<HTMLElement>('.tab');
    return { ...hit, before: under?.dataset.key ?? null };
  }
  // In the middle of its own group a tab is already where it would go, and
  // a group's only tab has no one to be put beside.
  const home = holding(layout.value, key);
  if (home?.id === hit.group && (hit.over === 'center' || home.tabs.length === 1)) return null;
  return { ...hit, before: null };
}

function letGo(apply: boolean): void {
  const was = drag.value;
  pressed = null;
  if (!was) return;
  drag.value = null;
  swallow = true;
  if (!apply || !was.to) return;
  if (was.to.over === 'tabs') placeTab(was.key, was.to.group, was.to.before);
  else dockTab(was.key, was.to.group, was.to.over);
}

export function Workbench() {
  const dragging = drag.value;
  useEffect(() => {
    if (!dragging) return;
    const onKey = (event: KeyboardEvent) => event.key === 'Escape' && letGo(false);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dragging !== null]);

  // A phone has room for one group: every tab is in its row, and the groups are kept for a wider screen.
  const arranged = phone.value ? oneRow(layout.value, focused.value) : layout.value;
  const placed = places(arranged);
  const open = new Map(tabs.value.map((tab) => [tabKey(tab), tab]));
  const all = groups(arranged);
  // In the order of their keys, which moving a tab does not change: a view
  // that only changes place is then left alone on the page.
  const views = all
    .flatMap((entry) => {
      const tab = entry.active ? open.get(entry.active) : undefined;
      return tab ? [{ key: entry.active!, tab, box: placed.groups.find((box) => box.id === entry.id)! }] : [];
    })
    .sort((a, b) => (a.key < b.key ? -1 : 1));

  // In a browser the bar over what is shown is drawn only where it holds something (`barShown`); where it is not, nothing is over it.
  const bar = !inPlace || barShown(side.value !== null, phone.value);
  return (
    <div
      class={classes('panes', dragging && 'dragging', inPlace && 'in-place', inPlace && !bar && 'barless')}
      style={{ '--strip': `${bar ? STRIP : 0}px` }}
      ref={(element) => {
        area = element;
      }}
    >
      {inPlace
        ? bar && <PlaceBar shown={views[0]?.tab} />
        : all.map((entry) => <Strip key={entry.id} entry={entry} box={placed.groups.find((box) => box.id === entry.id)!} open={open} />)}
      {views.map(({ key, tab, box }) => (
        <div
          key={key}
          class="view"
          // Which tab it is of, for what is kept by the tab while its view is away.
          data-tab={key}
          // Which of the panes has the keyboard, for a key that goes to what is in it.
          data-focused={focused.value === box.id}
          style={{
            left: percent(box.x),
            width: percent(box.w),
            top: `calc(${percent(box.y)} + var(--strip))`,
            height: `calc(${percent(box.h)} - var(--strip))`,
          }}
          onPointerDownCapture={() => focusGroup(box.id)}
        >
          {/* What it showed is kept by its tab, and it is drawn again as it was left when its tab shows again. */}
          <Returned scope={key} find={() => document.querySelector(`.view[data-tab="${CSS.escape(key)}"]`)}>
            <View tab={tab} />
          </Returned>
        </div>
      ))}
      {placed.dividers.map((divider) => (
        <Between key={divider.path} divider={divider} />
      ))}
      {cornered && <div class="tabs-corner" style={{ width: CORNER }} onMouseDown={grab} />}
      {dragging && <Landing dragging={dragging} title={open.get(dragging.key)} />}
      <Closing />
      <Leaving />
    </div>
  );
}

/**
 * In a browser, the bar over the one thing that shows: what brings the side panel back while it is folded away, and
 * what is shown, said as its tab would say it. It is no row of tabs: there is nothing in it to choose or close.
 */
function PlaceBar({ shown }: { shown: Tab | undefined }) {
  const said = shown ? tabTitle(shown) : null;
  // With the panel away so is the rail, on a phone, and with it the mark that something waits: it is here instead.
  const waiting = phone.value && (channelsWait(channels.value, unread.value) || (readsInbox() && inboxWaits.value > 0));
  return (
    <div class="placebar">
      {side.value === null && <SideFold class={classes('tabs-side', waiting && 'tabs-side-waiting')} />}
      {said && shown?.kind !== 'home' && (
        <span class="placebar-shown" title={said.title}>
          <span class="faint">{said.glyph}</span>
          <span class="truncate">{said.title}</span>
          {edited.value.has(tabKey(shown!)) && <span class="tab-edited" role="img" aria-label="not saved" title="Not saved" />}
        </span>
      )}
    </div>
  );
}

// In a browser the browser's tab says what it shows, after the factory's profile and before the product (`titleShowing`).
if (inPlace)
  effect(() => {
    const said = meta.value;
    if (!said) return;
    const key = groups(layout.value).find((entry) => entry.id === focused.value)?.active;
    const shown = tabs.value.find((tab) => tabKey(tab) === key);
    document.title = titleShowing(said.name, said.profile, shown && shown.kind !== 'home' ? tabTitle(shown).title : null);
  });

/** One group's row of tabs. */
function Strip({ entry, box, open }: { entry: Group; box: Box; open: Map<string, Tab> }) {
  const to = drag.value?.to;
  const here = to?.over === 'tabs' && to.group === entry.id ? to : null;
  // In the desktop app the first row is also the top of the window: its
  // empty parts move the window, and the row that reaches the right-hand
  // corner stops short of the window's buttons.
  const top = cornered && box.y < EDGE;
  const width = top && box.x + box.w > 1 - EDGE ? `calc(${percent(box.w)} - ${CORNER}px)` : percent(box.w);
  // On a phone the row is scrolled with a finger and its tabs do not shrink
  // to fit, so the one showing may be off the end of it: it is brought in.
  const row = useRef<HTMLDivElement>(null);
  const narrow = phone.value;
  // Wherever the row is wider than its group, the tab showing is brought in, and clear of Split at its end.
  useLayoutEffect(() => {
    const strip = row.current;
    const tab = strip?.querySelector<HTMLElement>('.tab[aria-selected="true"]');
    const split = strip?.querySelector<HTMLElement>('.tabs-split')?.offsetWidth ?? 0;
    if (strip && tab) strip.scrollLeft = inView(strip.scrollLeft, strip.clientWidth - split, tab.offsetLeft, tab.offsetLeft + tab.offsetWidth);
  }, [entry.active, entry.tabs.length, narrow]);
  // A wheel turned over a row that scrolls moves it sideways: a mouse has no other way to, the row's bar being hidden.
  useEffect(() => {
    const strip = row.current;
    if (!strip) return;
    const turned = (event: WheelEvent) => {
      if (strip.scrollWidth <= strip.clientWidth || Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
      strip.scrollLeft += event.deltaY;
      event.preventDefault();
    };
    strip.addEventListener('wheel', turned, { passive: false });
    return () => strip.removeEventListener('wheel', turned);
  }, []);
  // With the panel away so is the rail, on a phone, and with it the mark that something unread is waiting in a channel or in the inbox: it is here instead.
  const waiting = narrow && (channelsWait(channels.value, unread.value) || (readsInbox() && inboxWaits.value > 0));
  return (
    <div
      class={classes('tabs', focused.value === entry.id && 'focused')}
      role="tablist"
      ref={row}
      style={{ left: percent(box.x), top: percent(box.y), width }}
      onPointerDownCapture={() => focusGroup(entry.id)}
      onMouseDown={top ? grab : undefined}
    >
      {/* With the side panel folded away, what brings it back is here, before the first tab of the first row. */}
      {side.value === null && box.x < EDGE && box.y < EDGE && <SideFold class={classes('tabs-side', waiting && 'tabs-side-waiting')} />}
      {entry.tabs.map((key) => {
        const tab = open.get(key);
        if (!tab) return null;
        const { glyph, title } = tabTitle(tab);
        // A private channel's tab has its lock where an open one's has its `#`, as its row and its head do; a conversation's its envelope.
        const of = tab.kind === 'channel' ? channels.value.find((channel) => channel.id === tab.id) : undefined;
        const [locked, conversed] = [of?.kept === true && !of.conversation, of?.conversation === true];
        return (
          <button
            key={key}
            class={classes('tab', drag.value?.key === key && 'dragged', here?.before === key && 'drop-before')}
            role="tab"
            data-key={key}
            title={tabTip(tab)}
            aria-selected={entry.active === key}
            onClick={() => {
              if (swallow) swallow = false;
              else select(key);
            }}
            onAuxClick={(event) => event.button === 1 && closeTab(key)}
            onPointerDown={(event) => {
              swallow = false;
              if (event.button !== 0 || (event.target as Element).closest('.close')) return;
              pressed = { key, x: event.clientX, y: event.clientY };
              // From here every movement comes to this tab, wherever the
              // pointer goes: over another group, or a page in a frame.
              event.currentTarget.setPointerCapture(event.pointerId);
            }}
            onPointerMove={(event) => {
              // A finger moved along the row is scrolling it. There is nowhere on a phone to carry a tab to.
              if (pressed?.key !== key || phone.value) return;
              if (!drag.value && Math.hypot(event.clientX - pressed.x, event.clientY - pressed.y) < SLACK) return;
              drag.value = { key, x: event.clientX, y: event.clientY, to: aimed(event.clientX, event.clientY, key) };
            }}
            onPointerUp={() => letGo(true)}
            onPointerCancel={() => letGo(false)}
          >
            {locked || conversed ? (
              <span class="faint kept-lock" role="img" aria-label={conversed ? 'Conversation' : 'Private'} title={conversed ? 'A conversation' : 'Private: read by whoever is in it and nobody else'}>
                {conversed ? signs.envelope(11) : signs.lock(11)}
              </span>
            ) : (
              <span class="faint">{glyph}</span>
            )}
            <span class="truncate">{title}</span>
            {edited.value.has(key) && <span class="tab-edited" role="img" aria-label="not saved" title="Not saved" />}
            <span
              class="close"
              role="button"
              aria-label={`Close ${title}`}
              onClick={(event) => {
                event.stopPropagation();
                closeTab(key);
              }}
            >
              ✕
            </span>
          </button>
        );
      })}
      <span class={classes('tabs-end', here && here.before === null && 'drop-before')} />
      {entry.tabs.length > 1 && entry.active && !narrow && (
        <button
          class="ghost icon tabs-split"
          aria-label="Split"
          title="Show this tab beside the others"
          onClick={() => dockTab(entry.active!, entry.id, 'right')}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M2.5 3h11v10h-11zM8 3v10" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" />
          </svg>
        </button>
      )}
    </div>
  );
}

/** The line between the two halves of a split, which is dragged to share the space differently. */
function Between({ divider }: { divider: Divider }) {
  const row = divider.dir === 'row';
  const { within } = divider;
  const start = row ? within.x : within.y;
  const span = row ? within.w : within.h;
  const style = row
    ? { left: percent(start + span * divider.share), top: percent(within.y), height: percent(within.h) }
    : { top: percent(start + span * divider.share), left: percent(within.x), width: percent(within.w) };
  return (
    <div
      class={`divider divider-${divider.dir}`}
      role="separator"
      aria-orientation={row ? 'vertical' : 'horizontal'}
      style={style}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
      }}
      onPointerMove={(event) => {
        const rect = area?.getBoundingClientRect();
        if (!rect || !event.currentTarget.hasPointerCapture(event.pointerId)) return;
        const at = row ? (event.clientX - rect.left) / rect.width : (event.clientY - rect.top) / rect.height;
        // Neither side is made too small to use, where there is room for both not to be.
        const least = Math.min(0.5, (row ? NARROWEST : SHORTEST) / (span * (row ? rect.width : rect.height)));
        resizeSplit(divider.path, Math.min(1 - least, Math.max(least, (at - start) / span)));
      }}
      onPointerUp={rememberLayout}
      onDblClick={() => {
        resizeSplit(divider.path, 0.5);
        rememberLayout();
      }}
    />
  );
}

/** While a tab is dragged: its name beside the pointer, and the place it would take. */
function Landing({ dragging, title }: { dragging: Drag; title: Tab | undefined }) {
  const rect = area?.getBoundingClientRect();
  const to = dragging.to;
  const box = to && to.over !== 'tabs' ? places(layout.value).groups.find((entry) => entry.id === to.group) : null;
  // The body of the group, under its tabs, in pixels; then the half of it a tab dropped here would take.
  const place =
    rect && box && to && to.over !== 'tabs'
      ? landing(
          { x: box.x * rect.width, y: box.y * rect.height + STRIP, w: box.w * rect.width, h: box.h * rect.height - STRIP },
          to.over,
        )
      : null;
  return (
    <>
      {place && <div class="landing" style={{ left: place.x, top: place.y, width: place.w, height: place.h }} />}
      {title && (
        <div class="tab-ghost" style={{ width: GHOST, left: Math.min(dragging.x + 14, window.innerWidth - GHOST - 8), top: dragging.y + 12 }}>
          {tabTitle(title).title}
        </div>
      )}
    </>
  );
}

/** Asked when a tab holding unsaved changes is closed. */
function Closing() {
  const key = closing.value;
  const tab = key ? tabs.value.find((open) => tabKey(open) === key) : undefined;
  if (!key || !tab) return null;
  const keep = () => (closing.value = null);
  const save = async () => {
    const draft = draftOf(key);
    if (tab.kind === 'file' && draft) {
      const done = await attempt(() => api.writeFile(tab.project, tab.path, draft.text));
      if (done === undefined) return;
    }
    closeTab(key, true);
  };
  return (
    <Modal onClose={keep}>
      <h2>{tabTitle(tab).title} has changes that are not saved</h2>
      <p class="muted">Closing it without saving loses them.</p>
      <div class="row" style="justify-content:flex-end;flex-wrap:wrap">
        <button type="button" onClick={keep}>
          Keep it open
        </button>
        <button type="button" onClick={() => closeTab(key, true)}>
          Discard them
        </button>
        <button class="primary" type="button" onClick={() => void save()}>
          Save and close
        </button>
      </div>
    </Modal>
  );
}

/**
 * Asked in a browser when something is opened in place of a thing holding unsaved changes, as closing a tab with
 * them asks: stay, throw them away and go, or save and go. Back and Forward ask nothing, and what is half written is
 * kept by the thing's key to come back to.
 */
function Leaving() {
  const asked = leaving.value;
  const key = asked ? (groups(layout.value).find((entry) => entry.id === focused.value)?.active ?? null) : null;
  const tab = key ? tabs.value.find((open) => tabKey(open) === key) : undefined;
  if (!asked || !key || !tab) return null;
  const stay = () => (leaving.value = null);
  // What it showed is let go of with what was written, so that Back finds it read anew, as a tab closed and opened again is.
  const discard = () => {
    keepDraft(key, null);
    forgetReturned(key);
    leaveFor(asked);
  };
  const save = async () => {
    const draft = draftOf(key);
    if (tab.kind === 'file' && draft) {
      const done = await attempt(() => api.writeFile(tab.project, tab.path, draft.text));
      if (done === undefined) return;
    }
    keepDraft(key, null);
    forgetReturned(key);
    leaveFor(asked);
  };
  return (
    <Modal onClose={stay}>
      <h2>{tabTitle(tab).title} has changes that are not saved</h2>
      <p class="muted">Leaving it without saving loses them.</p>
      <div class="row" style="justify-content:flex-end;flex-wrap:wrap">
        <button type="button" onClick={stay}>
          Stay
        </button>
        <button type="button" onClick={discard}>
          Discard them
        </button>
        <button class="primary" type="button" onClick={() => void save()}>
          Save and go
        </button>
      </div>
    </Modal>
  );
}

function View({ tab }: { tab: Tab }) {
  switch (tab.kind) {
    case 'channel':
      return <ChannelView id={tab.id} />;
    case 'file':
      return <FileView project={tab.project} path={tab.path} />;
    case 'board':
      return <BoardView project={tab.project} />;
    case 'library':
      return <LibraryView project={tab.project} name={tab.name} />;
    case 'mockups':
      return <MockupsView project={tab.project} name={tab.name} />;
    case 'table':
      return <RecordsView project={tab.project} name={tab.name} />;
    case 'project':
      return <ProjectHome id={tab.id} />;
    case 'repo':
      return <RepoView project={tab.project} />;
    case 'git':
      return <GitView project={tab.project} />;
    case 'environments':
      return <ProjectEnvironments project={tab.project} />;
    case 'instance':
      return <InstanceView id={tab.id} />;
    case 'run':
      return <PipelineRun id={tab.id} />;
    case 'pipelines':
      return <PipelinesView project={tab.project} />;
    case 'activity':
      return <Activity />;
    case 'metrics':
      return <Metrics />;
    case 'members':
      return <MembersView at={tab.at} />;
    case 'extension':
      return <ExtensionView name={tab.name} />;
    case 'blueprint':
      return <ShippedBlueprint name={tab.name} project={tab.project} />;
    case 'settings':
      return <Settings />;
    case 'secrets':
      return <Secrets />;
    case 'trash':
      return <TrashView />;
    case 'profile':
      return <Profile />;
    case 'releases':
      return <Releases />;
    case 'about':
      return <AboutView />;
    case 'inbox':
      return <InboxPage id={tab.id} />;
    case 'topic':
      return <TopicPage id={tab.id} wiki={tab.wiki} />;
    case 'reference':
      return (
        <Suspense fallback={<div class="empty">Opening the Reference</div>}>
          <Reference />
        </Suspense>
      );
    default:
      return <Home />;
  }
}

/** What a tab says where it is pointed at, where that is more than what it is called: a told line's first sentence whole. */
function tabTip(tab: Tab): string | undefined {
  return tab.kind === 'inbox' ? lineSentence(tab.id) : undefined;
}

/** What a tab says: a project's thing that somebody named something else in the side panel says that name (`headWord`). */
export function tabTitle(tab: Tab): { glyph: ComponentChildren; title: string } {
  switch (tab.kind) {
    case 'channel':
      return { glyph: '#', title: calledOf(channels.value.find((channel) => channel.id === tab.id) ?? { name: 'channel' }) };
    case 'file':
      return { glyph: isBlueprint(tab.path) ? '◆' : '≡', title: tab.path.split('/').pop() ?? tab.path };
    case 'board':
      return { glyph: '▦', title: `${projects.value.find((p) => p.id === tab.project)?.name ?? tab.project} ${headWord(tab.project, 'board', 'board')}` };
    case 'library':
      return { glyph: '❏', title: `${projects.value.find((p) => p.id === tab.project)?.name ?? tab.project} ${headWord(tab.project, `library:${tab.name}`, tab.name)}` };
    case 'mockups':
      return { glyph: '◰', title: `${projects.value.find((p) => p.id === tab.project)?.name ?? tab.project} ${headWord(tab.project, `mockups:${tab.name}`, tab.name)}` };
    case 'table': {
      // What people call it, where it is called something beside its name (`called.ts`).
      const table = (collections.value[tab.project] ?? []).find((one) => one.name === tab.name);
      return { glyph: '▥', title: `${projects.value.find((p) => p.id === tab.project)?.name ?? tab.project} ${table?.title_chosen || table?.title || tab.name}` };
    }
    case 'project':
      return { glyph: '▣', title: projects.value.find((p) => p.id === tab.id)?.name ?? tab.id };
    case 'repo':
      return { glyph: '▤', title: `${projects.value.find((p) => p.id === tab.project)?.name ?? tab.project} ${headWord(tab.project, 'repo', 'repo')}` };
    case 'git':
      return { glyph: GIT_GLYPH, title: `${projects.value.find((p) => p.id === tab.project)?.name ?? tab.project} ${headWord(tab.project, 'git', 'git')}` };
    case 'environments':
      return { glyph: '◇', title: `${projects.value.find((p) => p.id === tab.project)?.name ?? tab.project} ${headWord(tab.project, 'environments', 'environments')}` };
    case 'instance':
      // What it runs, and under which key: the id alone says nothing, and is what the tab is known by underneath.
      return { glyph: '◈', title: titled(tab.id, whatIs(tab.id)) };
    case 'run':
      // What it is a run of, once its page has read it: the id alone says nothing.
      return { glyph: '▷', title: runTitle(tab.id) };
    case 'pipelines':
      return { glyph: '▶', title: `${projects.value.find((p) => p.id === tab.project)?.name ?? tab.project} ${headWord(tab.project, 'pipelines', 'pipelines')}` };
    case 'activity':
      return { glyph: '↻', title: 'Activity' };
    case 'metrics':
      return { glyph: '◔', title: 'Measurements' };
    case 'members':
      return { glyph: signs.person(), title: 'Members' };
    case 'extension':
      return { glyph: '⬡', title: tab.name };
    case 'blueprint':
      return { glyph: '◆', title: tab.name };
    case 'settings':
      return { glyph: '⚙', title: 'Settings' };
    case 'secrets':
      return { glyph: '✱', title: 'Secrets' };
    case 'trash':
      // A bin, drawn: no letter says it as plainly.
      return { glyph: signs.bin(), title: 'Trash' };
    case 'profile':
      return { glyph: '◉', title: 'Profile' };
    case 'releases':
      return { glyph: '✦', title: "What's new" };
    case 'about':
      // By what the program is called, which the daemon says: before that is read, what the page is.
      return { glyph: signs.mark(12), title: meta.value?.name ? `About ${meta.value.name}` : 'About' };
    case 'inbox':
      // The message's first words: what the tab is known by is the line's id, which says nothing.
      return { glyph: signs.envelope(12), title: lineTitle(tab.id) };
    case 'reference':
      return { glyph: '§', title: 'Reference' };
    case 'topic':
      // What it is called as last read, or as it was opened: its id says nothing.
      return { glyph: TOPIC_GLYPH, title: titleOf(tab.id) ?? tab.title ?? 'Topic' };
    default:
      return { glyph: '⌂', title: 'Start' };
  }
}
