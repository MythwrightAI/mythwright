// The frame: the rail, the side panel, the workbench of tabs, and the drawer.

import type { ComponentChildren } from 'preact';
import { useEffect, useLayoutEffect, useState } from 'preact/hooks';
import { api } from './api.ts';
import type { Channel } from './api.ts';
import { groups } from './panes.ts';
import { phone } from './phone.ts';
import { drawerWidth, drawn } from './drawer.ts';
import './drawerleaves.ts';
import { back, drawerOwner } from './drawerkept.ts';
import { drawerScope } from './returned.ts';
import { Returned } from './views/returned.tsx';
import { sideWidth } from './side.ts';
import { saidOf } from './standing.ts';
import { DRAWER_WIDTH, DrawerEdge } from './views/draweredge.tsx';
import { SideEdge } from './views/sideedge.tsx';
import { Draining } from './views/draining.tsx';
import { SIDE_COMING, useSideSlid } from './views/sideslide.ts';
import {
  active,
  asking,
  attempt,
  channels,
  daemonBehind,
  drawer,
  edited,
  focused,
  foldSide,
  layout,
  me,
  meta,
  openTab,
  paletteOpen,
  projects,
  ready,
  reload,
  select,
  shortcutsOpen,
  showSide,
  side,
  standing,
  tabKey,
  toasts,
  turn,
  turned,
  unread,
} from './store.ts';
import type { SideView, Tab } from './store.ts';
import { cornered, grab, TitleBar } from './titlebar.tsx';
import { names } from './inplace.ts';
import { Modal, SideFold, Twist, useFocus } from './ui.tsx';
import { AskGroup, channelItemsNow, GroupedChannels, grouping, madeInto, NewGroup, outOfGroup, putMade, RowMenu } from './views/grouped.tsx';
import { AskToMake } from './views/repomade.tsx';
import { AskNewBlueprint } from './views/ownblueprints.tsx';
import { ProjectTree } from './views/projecttree.tsx';
import { OutlineAsked } from './views/outline.tsx';
import { NewProject } from './views/newproject.tsx';
import { AskSpace, ProjectsListed, SpaceChoice } from './views/spaces.tsx';
import { AskDoor, PrivateChoice } from './views/keptdoor.tsx';
import { AskConversed } from './views/conversed.tsx';
import { AskDirect, MembersFold } from './views/direct.tsx';
import { AgentAsked } from './views/agentacts.tsx';
import { ColumnAsked } from './views/columnset.tsx';
import { channelsWait } from './conversed.ts';
import { signs } from './views/signs.tsx';
import { calledOf } from './kept.ts';
import { SpaceFoot } from './views/chosen.tsx';
import { readsInbox } from './inboxed.ts';
import { InboxRail, InboxSide } from './views/inbox.tsx';
import { Lately, overPanel } from './views/lately.tsx';
import { Fold, NavItem, Section } from './views/navtree.tsx';
import { AllBlueprints, useBlueprintsRead } from './views/blueprintlist.tsx';
import { FoldAll } from './views/folding.tsx';
import { SECTIONS } from './folds.ts';
import { FACTORY, NothingFound, PanelSeek, REFERENCE_SAYS, resourcesFound, Sought, typedIn } from './views/panelseek.tsx';
import { narrowed, ofMembers, ofProject, shows } from './panelsought.ts';
import { topicsFound } from './views/topics.tsx';
import { NewWikiButton, WikiAsked, WikiTree } from './views/wikitree.tsx';
import { CHANNELS_CALLED } from './sidepanel.ts';
import { workspaces } from './spaces.ts';
import { listed as listedIn } from './workspaces.ts';
import { held } from './chosen.ts';
import { actorKind, classes, initials } from './util.ts';
import { Standing, Who } from './views/who.tsx';
import { Auth } from './views/auth.tsx';
import { AskUpdate } from './views/came.tsx';
import { ExtensionsSide } from './views/extensions.tsx';
import { Palette } from './views/search.tsx';
import { Shortcuts } from './views/shortcuts.tsx';
import { Where } from './views/where.tsx';
import { Across } from './views/across.tsx';
import { asksForComposer, asksForSearch, asksForShortcuts, foldsSide, railView, tabAfter, turnsTabs } from './shortcuts.ts';
import { tabTitle, Workbench } from './workbench.tsx';

export function App() {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (asksForSearch(event)) {
        event.preventDefault();
        if (me.value) paletteOpen.value = asksForSearch(event) === 'open' || !paletteOpen.value;
      } else if (asksForShortcuts(event)) {
        event.preventDefault();
        if (me.value) shortcutsOpen.value = !shortcutsOpen.value;
      } else if (foldsSide(event)) {
        event.preventDefault();
        if (me.value) foldSide();
      } else if (turnsTabs(event)) {
        // Through the tabs of the pane that has the keyboard, and round again.
        event.preventDefault();
        const pane = groups(layout.value).find((entry) => entry.id === focused.value);
        const next = pane && tabAfter(pane.tabs, pane.active ?? '', turnsTabs(event) as 1 | -1);
        if (me.value && next) select(next);
      } else if (railView(event)) {
        event.preventDefault();
        // Shown, and left shown if it already was: a key that sometimes hid it would have to be looked at. The inbox only for whoever has one.
        const view = railView(event)!;
        if (me.value && (view !== 'inbox' || readsInbox())) showSide(view, false);
      } else if (asksForComposer(event, typingIn(event.target))) {
        // The channel in the pane that has the keyboard, or the one there is.
        const box = document.querySelector<HTMLTextAreaElement>('.view[data-focused="true"] textarea[aria-label="Message"]') ?? (document.querySelectorAll('.view textarea[aria-label="Message"]').length === 1 ? document.querySelector<HTMLTextAreaElement>('.view textarea[aria-label="Message"]') : null);
        if (me.value && box) {
          event.preventDefault();
          box.focus();
        }
      } else if (event.key === 'Escape' && drawer.value && !paletteOpen.value && !shortcutsOpen.value) {
        drawer.value = null;
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  // Folded away, a copy of the side panel slides out over the room it gave back; brought back, it slides in (`sideslide.ts`).
  const slid = useSideSlid(ready.value && me.value ? side.value : undefined, phone.value);
  // And it says when it is where it will be, as the drawer does: at once where nothing slides it in, or another panel took its place.
  useLayoutEffect(() => drawn(slid.shell.current?.querySelector<HTMLElement>(':scope > .sidebar') ?? null));

  if (!ready.value) return null;
  if (!me.value) {
    return (
      <>
        <TitleBar />
        <Auth />
      </>
    );
  }
  return (
    <div
      ref={slid.shell}
      class={classes('shell', side.value === null && 'side-closed', slid.coming && SIDE_COMING, cornered && 'cornered')}
      // How wide the side panel was left; a phone lays it out its own way.
      style={phone.value ? undefined : { '--side-width': `${sideWidth.value}px` }}
    >
      <TitleBar corner />
      {/* Which factory this is, across the top of a screen too narrow to keep the side panel beside the page. */}
      <Where />
      {/* That it is a copy of another, or what it was told to say, across the top of every page. */}
      <Across />
      <Rail />
      {side.value === 'channels' && <ChannelsSide />}
      {side.value === 'topics' && <TopicsSide />}
      {side.value === 'resources' && <ResourcesSide />}
      {/* The inbox of whoever is signed in, a view of the rail as the others are (`views/inbox.tsx`). */}
      {side.value === 'inbox' && readsInbox() && <InboxSide />}
      {side.value === 'extensions' && <ExtensionsSide />}
      {side.value !== null && !phone.value && <SideEdge />}
      {asking.value === 'project' && <NewProject onClose={() => (asking.value = null)} />}
      {asking.value === 'channel' && (
        <Ask
          onClose={() => {
            asking.value = null;
            madeInto.value = null;
          }}
        />
      )}
      <AskGroup />
      <AskSpace />
      <AskToMake />
      <AskNewBlueprint />
      <AskDoor />
      <AskConversed />
      <AskDirect />
      <AgentAsked />
      <ColumnAsked />
      <AskUpdate />
      <RowMenu />
      <OutlineAsked />
      {/* On a phone the side panel lies over the page, and the page under it is pressed to put it away. */}
      {phone.value && side.value !== null && <div class="side-under" onClick={foldSide} />}
      {/* How wide the drawer was left; it is never wider than this, the workbench it lies over. */}
      <div class="main" style={{ [DRAWER_WIDTH]: `${drawerWidth.value}px` }}>
        <Workbench />
        {/* One element for as long as a drawer is open, whatever it is handed meanwhile: it slides in when it opens, and not when its contents change. */}
        {drawer.value && (
          // Given back to its tab, it is simply there: it does not come in again.
          <aside class={back.value ? 'drawer drawer-back' : 'drawer'} ref={drawn}>
            {!phone.value && <DrawerEdge />}
            <div class="drawer-head">
              <h2 class="grow truncate">{drawer.value.title}</h2>
              <button class="ghost icon" aria-label="Close" onClick={() => (drawer.value = null)}>
                ✕
              </button>
            </div>
            <div class="drawer-body">
              {/* What it holds is kept by its tab, as the tab's view is: drawn again as it was left when it is given back. */}
              {drawerOwner.value ? (
                <Returned key={drawerOwner.value} scope={drawerScope(drawerOwner.value)} find={() => document.querySelector('.main > .drawer > .drawer-body')}>
                  {drawer.value.body()}
                </Returned>
              ) : (
                drawer.value.body()
              )}
            </div>
          </aside>
        )}
      </div>
      {paletteOpen.value && <Palette />}
      {shortcutsOpen.value && <Shortcuts />}
      <div class="toasts" role="status">
        <Notices />
        {toasts.value.map((entry) => (
          <div key={entry.id} class={classes('toast', entry.bad && 'bad', entry.undo && 'undoable')}>
            {entry.text}
            {entry.undo && (
              <button
                type="button"
                class="ghost toast-undo"
                onClick={() => {
                  toasts.value = toasts.value.filter((one) => one.id !== entry.id);
                  entry.undo?.();
                }}
              >
                Undo
              </button>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

/**
 * What stays on screen until someone deals with it: the client was replaced
 * while this page was open, or it is newer than the daemon it talks to.
 * Neither interrupts; both say what to do.
 */
function Notices() {
  const why = reload.value;
  return (
    <>
      {why && (
        <div class="toast notice">
          <p>
            {why === 'ready'
              ? 'A new version of the client is ready.'
              : 'Part of the client could not be loaded; it has probably been updated since this page was opened.'}
            {edited.value.size > 0 && ' Save your edits first: reloading discards them.'}
          </p>
          <div class="row">
            <button class="primary" onClick={() => location.reload()}>
              Reload
            </button>
            <button class="ghost" onClick={() => (reload.value = null)}>
              Later
            </button>
          </div>
        </div>
      )}
      <Draining />
      {daemonBehind.value && (
        <div class="toast notice">
          <p>
            This client is newer than the daemon it is talking to, so some things will not work until the daemon is
            restarted. What the daemon runs keeps running while it does.
          </p>
          <div class="row">
            <button class="ghost" onClick={() => (daemonBehind.value = false)}>
              Dismiss
            </button>
          </div>
        </div>
      )}
    </>
  );
}

/** A cog, drawn to the same size and weight as the other icons on the rail. */
const COG =
  'M7.02 3.76L7.80 1.80L10.20 1.80L10.98 3.76L11.30 3.90L13.24 3.06L14.94 4.76L14.10 6.70L14.24 7.02L16.20 7.80L16.20 10.20L14.24 10.98L14.10 11.30L14.94 13.24L13.24 14.94L11.30 14.10L10.98 14.24L10.20 16.20L7.80 16.20L7.02 14.24L6.70 14.10L4.76 14.94L3.06 13.24L3.90 11.30L3.76 10.98L1.80 10.20L1.80 7.80L3.76 7.02L3.90 6.70L3.06 4.76L4.76 3.06L6.70 3.90zM9 6.5a2.5 2.5 0 1 0 0 5a2.5 2.5 0 1 0 0-5z';

/** A tray with something in it, on the rail's grid: the inbox. */
const INBOX = 'M2.5 10.5l2-6.5h9l2 6.5v4h-13zM2.5 10.5h3.6l.9 1.8h4l.9-1.8h3.6';

/** The far-left strip: which view the side panel shows, and at its foot the page about the program and the way to settings. */
function Rail() {
  // One size and one weight of line for everything on the rail. Drawn on
  // a grid of 18 and shown a little larger: at 18 they read as small beside
  // the panel's own text, and the button has the room.
  const icon = (d: string) => (
    <svg width="20" height="20" viewBox="0 0 18 18" aria-hidden="true">
      <path d={d} fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" stroke-linecap="round" />
    </svg>
  );
  const waiting = channelsWait(channels.value, unread.value);
  // What the program is called is what its daemon says: before that is read, what the place is.
  const about = meta.value?.name ? `About ${meta.value.name}` : 'About';
  const view = (name: SideView, label: string, drawing: ComponentChildren) => (
    <button
      class={classes('rail-button', name === 'channels' && waiting && side.value !== 'channels' && 'waiting')}
      aria-label={label}
      aria-pressed={side.value === name}
      data-tip={side.value === name ? `${label} (click to hide)` : label}
      onClick={() => showSide(name)}
    >
      {drawing}
    </button>
  );
  return (
    <div class="rail">
      {view('channels', CHANNELS_CALLED, icon('M2.5 3.5h13v8.5H8.2L5 14.8V12H2.5z'))}
      {/* Just after the channels: a forum's topics, a place of their own, drawn as the panel's own signs are. */}
      {view('topics', 'Topics', signs.topic(20))}
      {view('resources', 'Resources', icon('M2.5 4.5h4.5l1.5 1.5h7v8.5h-13z'))}
      {/* Directly below Resources: the reader's box, with how many lines wait on it. */}
      <InboxRail icon={icon(INBOX)} />
      {view('extensions', 'Extensions', icon('M2.5 2.5h5v5h-5zM2.5 10.5h5v5h-5zM10.5 10.5h5v5h-5zM13 1.6l3.4 3.4-3.4 3.4L9.6 5z'))}
      <span class="grow" />
      {/* The program itself, by its mark: the page about it, opened as a tab as the settings are. At the
          foot with what opens a page, and not among the places over it, which each show the side panel something. */}
      <button class="rail-button rail-about" aria-label={about} data-tip={about} onClick={() => openTab({ kind: 'about' })}>
        {signs.mark(20)}
      </button>
      {/* Who is signed in, over the settings they are changed in. */}
      {me.value && (
        <button class="rail-button rail-me" aria-label={`You: ${me.value.principal.name}`} data-tip={`${me.value.principal.name} · ${me.value.tier} plan`} onClick={() => openTab({ kind: 'profile' })}>
          <span class="avatar">{initials(me.value.principal.name)}</span>
        </button>
      )}
      <button class="rail-button" aria-label="Settings" data-tip="Settings" onClick={() => openTab({ kind: 'settings' })}>
        {icon(COG)}
      </button>
    </div>
  );
}

/** Whether a key pressed here is being typed into something. */
function typingIn(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));
}

/**
 * A channel in the side panel: its name, which opens it, and folded under
 * it who is in it. The people are in one place and the agents in another,
 * each agent with how it stands just now, so that a look at the panel says
 * who is at work where without opening anything.
 */
function ChannelTree({ channel, more }: { channel: Channel; more?: ComponentChildren }) {
  const key = `channel:${channel.id}`;
  const open = turned.value.has(key);
  const tab: Tab = { kind: 'channel', id: channel.id };
  const current = active.value === tabKey(tab);
  const count = unread.value[channel.id];
  const called = calledOf(channel);
  const of = (kind: string) => channel.members.filter((member) => actorKind(member.actor) === kind);
  const group = (label: string, kind: string) => {
    const members = of(kind);
    if (members.length === 0 && kind === 'bot') return null;
    return (
      <Fold at={`${key}:${kind}`} label={label} unfolded>
        {members.map((member) => (
          <Who key={member.actor} member={member} class={classes('nav-who', kind)}>
            {kind === 'agent' ? <Standing agent={member.actor.slice('agent:'.length)} /> : <span class="glyph">{kind === 'bot' ? '◇' : '·'}</span>}
            <span class="truncate">{member.name}</span>
            {member.actor === me.value?.actor && <span class="faint">you</span>}
          </Who>
        ))}
        {members.length === 0 && <span class="nav-who faint">nobody yet</span>}
      </Fold>
    );
  };
  return (
    <div>
      <div class={classes('nav-row', current && 'current')}>
        <button class="nav-twist" aria-label={open ? `Fold ${called}` : `Unfold ${called}`} aria-expanded={open} onClick={() => turn(key)}>
          <Twist open={open} />
        </button>
        <button class="nav-item" aria-current={current} data-names={names(tab)} title={channel.topic.trim() ? `${called}: ${channel.topic.trim()}` : undefined} onClick={() => openTab(tab)}>
          {/* A private channel is told from the rest by a lock where an open one has its `#`: shown only to whoever is in it. */}
          {channel.kept ? (
            <span class="glyph kept-lock" role="img" aria-label="Private" title="Private: read by whoever is in it and nobody else">
              {signs.lock(13)}
            </span>
          ) : (
            <span class="glyph">#</span>
          )}
          <span class="truncate">{called}</span>
          {count ? <span class="count">{count}</span> : null}
        </button>
        {more}
      </div>
      {open && (
        <div class="tree">
          {group('People', 'user')}
          {group('Agents', 'agent')}
          {group('Bots', 'bot')}
        </div>
      )}
    </div>
  );
}

/**
 * Said at a side panel's head only while what the page shows is not being
 * kept up: nothing is there while events arrive, which is how it is meant
 * to be, and nothing again once they are back.
 */
function StreamLost() {
  const said = saidOf(standing.value);
  if (!said) return null;
  return <span class="stream-lost" role="status" aria-label={said} title={said} />;
}

/**
 * The side panel for talking: the channels, as they were before there were
 * topics, which have a place of their own now. What is typed in
 * its box narrows them, and one line says where nothing says it.
 */
function ChannelsSide() {
  const text = typedIn('channels');
  const found = text.trim() === '' ? null : narrowed(channelItemsNow(), text);
  const out = outOfGroup();
  return (
    <nav class="sidebar" aria-label={CHANNELS_CALLED}>
      {/* Headed by what it is, as the other side panels are, and not by what the product is called nor which factory this is. It says so once: what makes a group or a channel is here, and no second heading is over the list. */}
      {/* A channel dragged onto it leaves its group: it is there before anything is dragged, and is outlined while one is over it. */}
      <div class={classes('brand', out.over && 'drop')} onMouseDown={grab} {...out.takes}>
        <span>{CHANNELS_CALLED}</span>
        <span class="brand-acts">
          <StreamLost />
          <FoldAll of="channels" />
          <NewGroup />
          <button class="ghost icon brand-act" aria-label="New channel" title="New channel" onClick={() => (asking.value = 'channel')}>
            +
          </button>
          <SideFold class="ghost icon side-fold" />
        </span>
      </div>
      {/* What is typed here narrows the channels listed; what was said in them is found by searching everything, which the box offers. */}
      <PanelSeek panel="channels" />
      <div class="nav">
        {/* The channels, with what is offered where there is none yet. */}
        <GroupedChannels found={found} row={(channel, more) => <ChannelTree channel={channel} more={more} />} />
        {found && found.shown.size === 0 && <NothingFound text={text} />}
      </div>
      {/* Which workspace the panel is of, where the factory has any: the same choice under Channels and under Resources (`chosen.tsx`). */}
      <SpaceFoot />
    </nav>
  );
}

/**
 * The side panel of a forum's topics, a place of the rail of
 * its own just after the channels: the wikis of the workspace it is of,
 * one tree of their pages, with those that are closed under Closed
 * (`views/wikitree.tsx`).
 */
function TopicsSide() {
  const text = typedIn('topics');
  const found = topicsFound(text);
  return (
    <nav class="sidebar" aria-label="Topics">
      <div class="brand" onMouseDown={grab}>
        <span>Topics</span>
        <span class="brand-acts">
          <StreamLost />
          <FoldAll of="topics" />
          <NewWikiButton />
          <SideFold class="ghost icon side-fold" />
        </span>
      </div>
      <PanelSeek panel="topics" />
      <div class="nav">
        <WikiTree found={found} text={text} />
      </div>
      <WikiAsked />
      {/* Which workspace the panel is of: the same choice as under the other panels that list by workspace (`chosen.tsx`). */}
      <SpaceFoot />
    </nav>
  );
}

/**
 * The side panel for what there is to work on, in three parts that are
 * not of a kind. The factory is the whole installation: what is running on
 * it, what has happened and how it is doing, across every project (each of
 * those views can be narrowed to one). The blueprints are what anything
 * here can be made to do, whichever project it is done for. The projects
 * are what is worked on.
 */
function ResourcesSide() {
  // What is typed in the box narrows everything listed here, to what the panel has read (`panelseek.tsx`).
  const text = typedIn('resources');
  const { found, blueprints, any } = resourcesFound(text);
  const searching = found !== null;
  const some = (...ids: string[]) => ids.some((id) => shows(found, id));
  const projectsShown = listedIn(projects.value, workspaces.value, held.value).some((project) => shows(found, ofProject.project(project.id)));
  useBlueprintsRead();
  return (
    <nav class="sidebar" aria-label="Resources">
      <div class="brand" onMouseDown={grab}>
        <span>Resources</span>
        <span class="brand-acts">
          <FoldAll />
          <SideFold class="ghost icon side-fold" />
        </span>
      </div>
      <PanelSeek panel="resources" />
      <div class="nav" {...overPanel}>
        <Sought.Provider value={found}>
          {/* What whoever is signed in keeps to hand: what they pinned, and what they lately opened. Neither is drawn while it is empty. */}
          <Lately />
          {/* Each part folds, as everything under it does, and starts folded (`folds.ts`); what was typed in the box holds open a part it finds something in.
              The parts stand in the order of `SECTIONS`: what is kept to hand, the projects that are worked on, the factory's, and then the blueprints. */}
          {(!searching || projectsShown) && (
            <Section
              at={SECTIONS.projects}
              label="Projects"
              held={searching}
              act={
                <button class="ghost icon" aria-label="New project" title="New project" onClick={() => (asking.value = 'project')}>
                  +
                </button>
              }
            >
              {/* Those of the workspace the panel is of, where there is any; and all of them, as they always were, where there is none. */}
              <ProjectsListed tree={(project, more) => (shows(found, ofProject.project(project.id)) ? <ProjectTree key={project.id} project={project} more={more} /> : null)} />
              {projects.value.length === 0 && !searching && (
                <p class="faint" style="padding:2px 8px;font-size:calc(12px * var(--size))">
                  Projects appear here. Ask an agent to set one up, or add one with +.
                </p>
              )}
            </Section>
          )}
          {some(FACTORY.activity, FACTORY.metrics, ofMembers.part, FACTORY.trash) && (
            <Section at={SECTIONS.factory} label="Factory" held={searching}>
              {some(FACTORY.activity) && <NavItem tab={{ kind: 'activity' }} glyph="↻" label="Activity" />}
              {some(FACTORY.metrics) && <NavItem tab={{ kind: 'metrics' }} glyph="◔" label="Measurements" />}
              {/* Everybody here and the agents of the workspace: the name opens the Members page, the people and the agents a tab
                  each (`members.tsx`); the arrow lists them, each pressed for what can be done with them (`direct.tsx`). */}
              <MembersFold found={found} />
              {/* What was deleted, to be given back: the factory's, whatever workspace the panel is of. */}
              {some(FACTORY.trash) && <NavItem tab={{ kind: 'trash' }} glyph={tabTitle({ kind: 'trash' }).glyph} label="Trash" />}
            </Section>
          )}
          {(!searching || some(FACTORY.reference) || blueprints > 0) && (
            <Section at={SECTIONS.blueprints} label="Blueprints" held={searching}>
              {/* Everything a blueprint can say: the factory's, whatever workspace the panel is of. */}
              {some(FACTORY.reference) && <NavItem tab={{ kind: 'reference' }} glyph="§" label="Reference" title={REFERENCE_SAYS} />}
              <AllBlueprints typed={text} />
            </Section>
          )}
          {!any && <NothingFound text={text} files />}
        </Sought.Provider>
      </div>
      {/* Which workspace the panel is of, where the factory has any: the same choice under Channels and under Resources (`chosen.tsx`). */}
      <SpaceFoot />
    </nav>
  );
}

function Ask({ onClose }: { onClose: () => void }) {
  const [name, setName] = useState('');
  // The workspace it is to be in: the one the lists are held to, so that what is added while looking at a workspace
  // is not added out of sight. Unsaid (the workspaces are not read yet), the daemon puts it in the usual one.
  const [space, setSpace] = useState(() => (held.value.kind === 'space' ? held.value.id : ''));
  // A channel may be private: read by whoever is in it and nobody else. And then, said under it, hold its agents to itself.
  const [kept, setKept] = useState(false);
  const [alone, setAlone] = useState(false);
  // The group a channel is to go in, where it is made from a group's menu: as it was when the dialog opened.
  const [into] = useState(() => madeInto.peek());
  const intoName = into ? (grouping.value.groups.find((group) => group.id === into)?.name ?? null) : null;
  const input = useFocus<HTMLInputElement>();
  const submit = async (event: Event) => {
    event.preventDefault();
    const channel = await attempt(() => api.createChannel(name, undefined, space || undefined, kept, kept && alone));
    // Made from a group's menu, it goes in that group, and is then opened as any is.
    if (channel && into) putMade(channel, into, space);
    if (channel) openTab({ kind: 'channel', id: channel.id });
    onClose();
  };
  return (
    <Modal onClose={onClose}>
      <h2>New channel</h2>
      <form class="stack" onSubmit={submit}>
        {/* Where it is to be comes first, and is filled already with the workspace showing: the caret begins in the name under it. */}
        <SpaceChoice value={space} onChange={setSpace} />
        <label>
          Name
          <input ref={input} required value={name} placeholder="general" onInput={(event) => setName(event.currentTarget.value)} />
        </label>
        <PrivateChoice kept={kept} alone={alone} onKept={setKept} onAlone={setAlone} />
        {intoName && <p class="faint" style="margin:0;font-size:calc(12px * var(--size))">It goes in {intoName}, a group of your own.</p>}
        <div class="row" style="justify-content:flex-end">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button class="primary" type="submit">
            Create
          </button>
        </div>
      </form>
    </Modal>
  );
}
