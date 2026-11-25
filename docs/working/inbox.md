# Topics, the inbox and conversations

This file covers the forum's topics and wikis, everybody's inbox and its six
kinds of line (somebody named in a message, a conversation, a line a
blueprint told, somebody named in a comment on a record, a page somebody
follows, anything else somebody follows and a digest), conversations and
direct messages, the Members line of the side panel, and the parts a told
line shows beside its words. Read it before changing `crates/topics`,
`talk::inbox`, `talk::conversed`, `told`, or the client's inbox, topic and
member views. Channels themselves are in `docs/working/channels.md`.

## Topics and wikis

### Where topics are kept

- Topics are a table of the factory's own. A table's place is a project id or
  the factory's own place, `@` (`kernel::place::FACTORY`, which no project slug
  can be). Such a table is declared by a blueprint applied for no project,
  which takes `admin` beside what declaring any table takes; it is kept in the
  factory's database only, never as files (`studio::factory::placed`); its
  records are `@-topics-12`; its events carry `project: "@"`, which keeps them
  in the factory's journal and makes them nobody's to whatever hears by project
  (`conductor::whose`), so a handler of any project hears them and narrows by
  what the event names.
- `std/topics` declares `topics` (the model `topic`, `studio::models::TOPIC`,
  struct `Topic`) and `categories`; it is applied when the first topic is made,
  by whoever manages the factory (`topics::Topics::make`); a member is told so.
- A topic's body and summary are `document` fields (Markdown in the row,
  512 KB at most). Topics are readable by whoever reads records; per-topic
  view permissions are not built. `mw topic`, `/api/topics`.
- A topic's status is optional: a row whose `status` is empty has none
  (`kernel::topics::status_of`), which is how one is made where nothing is
  said (`Topics::make`, `make_under`, `POST /api/topics` `status`,
  `mw topic make --status`, only `open` or `standing`: `made_status`). Use
  one for a question; a wiki's page or a gathering thread needs none. None
  is handed as `status: null`, named in no agent's prompt, and answering is
  refused (`not_a_question`, checked under the topic's turn in
  `Topics::answered`). Clearing it (`record set <topic> status=`, Not
  tracked in the client) says `topic.untracked`; a topic made with a status
  says it. A row an earlier version left empty (only by `record set
  status=` or a records route, since the keeper always wrote `open`) now
  reads as none.
- Every write of a topic's row, by any route, is one write with what goes with
  it: the topics keeper installs into the studio what a write of its model
  also does (`studio::factory::Written`, called in `save_also` with the old
  row): a revision (`topic_revisions`, the `topics` crate's own migrations)
  where the text changed, and the status event (`topic.revised`,
  `topic.answered`, `.closed`, `.opened`, `.standing`, `.untracked`, on
  `journal::HERE`).
  Something new that goes with a topic change is written there, never by one
  route.
- What a model keeps for itself (`own`: a topic's `workspace`, `project`,
  `followers`, `parent`) is refused at every route that writes records
  (`models::own_refused`, in `Records::add` and `checked`) and written only
  by the model's own methods (`Topics::place`, `follow`, `make_under`,
  `move_to`). A new `own` field says which method writes it (`Model::doors`).
- A topic belongs to a workspace or a project, exactly one
  (`kernel::topics::Place`), and is moved somewhere that exists when a node
  starts and when a project or workspace is removed (`Topics::settle`,
  `routes::topics::follow_places`).

### Pages and the tree

- A topic is a page of a wiki: it may hold topics under it to any depth, and
  one with no parent is a wiki. There is no wiki table and no folder kind. The
  parent is `parent` (`kernel::topics::PARENT`); the order among siblings is
  the record's own `position`, written only by the keeper for topics.
- How the tree is read (a parent naming no page, and every page of a ring,
  read as none), whether a page may move somewhere and where it lands are
  `kernel::topics::tree` (`Tree`, `landing`, `between`), checked case for case
  with the client's `ui/src/wikis.ts` against `ui/src/wikicases.json`.
- The keeper's methods `Topics::make_under`, `move_to` and `place` (a whole
  wiki; a page under another is refused) hold one turn for the whole tree
  (`ARRANGING` in `crates/topics/src/tree.rs`) and each written page's turn,
  and write every changed page in one write (`Collections::put_many_saying`),
  emitting `topic.moved` and `topic.placed`. New changes of where a page is go
  through them.
- Every page has its wiki's place, written so on make, move, place and settle,
  so reading a page's place needs no tree walk. The tree is read from a few
  cells and no body (`Collections::cells`). A topic an earlier version made
  has no `parent` and is a wiki; nothing is written for it.
- `POST /api/topics` takes `under`; `POST /api/topics/{id}/move` takes
  `{ under: <page>|null, before? }`; `GET /api/topics/{id}` returns `path` and
  `pages`; `mw topic make --under`, `mw topic move --under|--top [--before]`.
- Deleting a page deletes every page under it, by every route, through the
  keeper (`Written::goes_with`, inside `Collections::trash_saying`'s write and
  once before it, holding the arranging turn, `Written::arranging`, then each
  page's turn via `Collections::changing_all`): one Trash entry holding the
  pages whole (`kept.with`), each emitting `record.removed` with `with` and
  `trash`. Restored in one write (`Written::given_back`) under its parent
  where that exists in its workspace, else as a wiki, reported in the answer
  and `trash.restored`. Deleted for good with each page's attachments and
  revisions (`Written::forgotten`). New per-record keeper state goes in
  `forgotten`; taking several records' turns uses `changing_all`.

### Replies and agents

- Replies are the record's comments, each of which may name what it answers
  (`parent`, `Records::reply`, `studio::replied`,
  `kernel::topics::placed_under`: eight deep, then placed beside the eighth).
  Any record's comments may.
- An agent named in a reply, or talked with in its chain, gets a turn of the
  topic (`agents::topical`: `kind: message`, no channel, conversation
  `topic:<topic>:<chain top>`, answering `<topic>#<number>`, prompt from
  `topics::context::prompt` with the topic and chain only). Its answer is
  posted under that reply once, even after a stop (`say_in_topic`), as the
  agent, by `Records::answer`: the daemon says it on the asker's leave, and
  the agent's own scopes are not asked (it may hold no `records:write`); its
  own commands still are, so the prompt is made for each agent asked and
  offers only the commands its scopes run (`context::May`: `topic show` with
  `records:read`, `topic reply` and `topic make --under` with that and
  `records:write`), says its answer is said for it and not to post it, and
  where it may not read says that what it is given is all it has. An error, an empty answer, or an answer that could
  not be said is said there by the factory instead ("`<agent>` could not
  answer: …", "… gave no answer.", "… answered, and its answer could not be
  said here: …"), each once, and counted as `agent.topic.answer`. An agent's
  reply in a topic gives no other agent a turn.
- A page is always named by its path: an agent in a reply is told its path and
  the titles of the pages directly under it, never their text (`About::path`,
  `pages`, twenty named and the rest counted, from `Topics::whereabouts`, also
  in `GET /api/topics/{id}`). An inbox line of a comment on a page shows its
  current path (`OnRecord::path`, in `studio::named`); the client re-reads the
  box on `topic.moved` and `topic.revised` where it has such a line.

### Links and sources

- A link to a page is a Markdown link to its id, `[words](topic:<id>)`, or
  `topic:<id>#<reply>` for a reply: read by `kernel::topics::linked`
  (`links_in`, `link_of`) and by the client's `prose.ts` through
  `pageLinkOf` in `ui/src/pagelinks.ts`, case for case with
  `ui/src/linkcases.json`. Nothing in code is a link, nor a picture's marks.
  Something new that reads links reads them there.
- Which pages link to a page is read, not kept: `Topics::linked`
  (`crates/topics/src/linked.rs`, `GET /api/topics/{id}/linked`) reads every
  topic's `body` and `summary` in one `Collections::cells` query and looks
  for the id (2,000 pages of 4 KB: about 25 ms, about what listing them
  takes). It asks `records:read` through `find`, as reading the page does;
  when topics come to be kept from some readers, it leaves out what the
  reader may not read. No table step was added for it.
- In the client a link is `PageLink` (`views/pagelink.tsx`), found among
  the topics read (`topicsread.ts`) by its id, its title as it is now in
  its `title`. One the reader cannot find (removed, or they hold no
  `records:read`) is `CANNOT_OPEN`, and the words written for it, most
  often the page's title, are never drawn; until the topics are read it is
  `a page`. A hover preview finds a link by `a.page-link[data-topic]` and
  the page's line (summary's first line) in `topics.value`, or reads it
  whole by `GET /api/topics/<id>`.
- Footnotes (`[^label]`, and `[^label]: …` on a line of its own) are read
  by `prose.ts` as `cite` spans and `note` blocks and numbered by
  `notesOf` in `ui/src/cited.ts` (first use, then unused); drawn by `Cite`
  and `NotesList` (`views/cited.tsx`). `Prose` lists a text's sources at
  its end, unless `notes="apart"`: a topic's post, whose References are
  drawn by `ArticleEnd` (`views/articleend.tsx`), with Linked from, mounted
  once at the end of the article in `views/topicpage.tsx`.
- The block editor keeps both: links as any link (`normalizeLink` leaves a
  `topic:` address whole), footnotes as two nodes of its own
  (`write/markdown.ts`, with two markdown-it rules, no package), typed by
  input rules and numbered by decorations (`write/noted.ts`); `[[` in it is
  `write/linked.ts`, and in a reply's box `TagBox` (`page`).
- The agent's prompt always asks for footnoted sources and links by id
  (`context::cite`); a topic has no setting to turn it off.

### Topics in the client

- Topics are a rail place of their own after Channels (`side` is `'topics'`,
  `SideView` in `ui/src/sidepanel.ts`; Alt and 5, `RAIL` in
  `ui/src/shortcuts.ts`; icon `signs.topic`). The channels' panel is called
  Channels (`CHANNELS_CALLED`); its store key is still `channels`.
- The Topics panel (`TopicsSide` in `ui/src/app.tsx`, `views/topics.tsx`,
  `views/wikitree.tsx`) is one tree of wikis in `position` order with their
  pages (`listedWikis` over `readTree` in `ui/src/wikis.ts`), each with its
  status and, on a wiki, its project. A page holding pages is a fold
  (`TOPICS.page(id)`, dropped once the page is gone, `topicFoldsGone`); closed
  wikis are under a folded Closed (`TOPICS.closed`). Each panel's Collapse all
  keeps the other's folds (`ofTopicsPanel`); `topicsUnfolded` is its Expand
  all. Fold names from the older two-node panel are dropped when read
  (`NODES_BEFORE`, `turnedNow`).
- It lists the panel workspace's topics (`spaceOfTopic`, `holdsTopic` in
  `ui/src/workspaces.ts`). Its search is `topicsFound` (over `topicItems` in
  `ui/src/panelsought.ts`): title, summary's first line, status, project, and
  the titles above it.
- For whoever may write records (`mayMakeTopic`) each line has a `+` (makes an
  Untitled page under it, `UNTITLED`, with an inline title field, `TitleField`)
  and a menu: New page, Rename…, Move to… (`views/wikimoved.tsx`, places from
  `movesFor`; for a wiki also other workspaces, behind `askMoving` with
  `wikiQuestion`), Open in a new tab, Move to Trash (with Undo). New topic at
  the head makes one in the panel's workspace.
- Lines are carried by the general tree carry (`useTreeCarry`; each page sets
  `data-place` and `data-outline`); a landing is one
  `POST /api/topics/{id}/move`, allowed only where `landing` says the daemon
  would accept it. Nothing is drawn ahead of the answer.
- Topics are read when the panel is first shown and again on `record.*` of
  `@`'s table and `topic.*` (`ui/src/topicsread.ts`), never with a body.
- A topic is a tab (`{ kind: 'topic', id }`, `views/topicpage.tsx`,
  `views/replies.tsx`, over `ui/src/topicpage.ts`, `ui/src/replies.ts`; not
  `topic.tsx`/`topic.ts`/`topic.css`, which are a channel's description). Its
  address is `/topic/<wiki>/<page>` (each title whole, so a move within its
  wiki keeps it) and `/topic/<wiki>` for a wiki; ambiguous titles fall back to
  ids (`addressNames` in `ui/src/wikipage.ts`). Addresses resolve among the
  topics read (`namedIn`: in its wiki first, then any page of that title), so
  older `/topic/<title>` and `/topic/<id>` still open; several matches are
  listed as links. Topics are kept in Recents (`lately.ts`).
- A topic's page is an article or a thread, decided from the topic alone
  with nothing stored (`partsOf` in `ui/src/topicpage.ts`, with its cases):
  a tracked question (status open, answered or closed) is a thread
  whatever its body says, its body drawn between the answer and the
  pages (`ThreadPost`; Add post where it has none). Any other is an
  article where its body says something or it has a parent, or where
  Write an article was pressed (kept by the tab until something is saved).
  An article has three tabs in the head (`Tabs`): Article (`Article` in
  `views/topicarticle.tsx`: summary, body, pages), Talk (`Talk` in
  `views/topictalk.tsx`: answer, reply box, replies) and History (`History`
  in `views/topichistory.tsx`). A thread draws the summary and `Talk`
  untabbed with a question's post and the pages between, and History at its
  head (a toggle, `aria-pressed`) draws `History` in their place (`partsOf`
  shows `history` for a thread only where it is chosen). A reply asked for
  by an address or an inbox line is kept in view while what is above it is
  laid out (a post's editor loads after the replies), until the reader acts
  on the page. The summary is drawn
  only where it says something or Add summary was pressed; saved empty, it
  goes. A stored choice (replies off, say) would be a field of `std/topics`,
  which factories hold until applied again: avoid one.
- The tab shown is kept by the page's tab (`useKept('part')`), said in the
  bar as `#talk` or `#history/<n>` (`topicShowing`, `shownNow`), and asked
  for by an address (`#talk`, `#talk/r-<seq>`, `#history`, `#history/<n>`;
  none asks for Article) and by `showReply`, through `topicAsked` in
  `ui/src/topicshown.ts`. A `hashchange` is taken at the address it changed
  to (`event.newURL`), since by then the bar may already have been put back
  to the tab showing.
- Body and summary are read by everybody (`Prose`) and, for whoever may
  change the topic, written in the block editor once opened, and saved
  through `PATCH /api/records/<id>` (`DocField` in `views/topicdoc.tsx`).
  What is open is the page's, kept by its tab (`Editing`, `opened`,
  `editAll` in `ui/src/topicediting.ts`, with cases): Edit at the head
  (on Article, or on a thread whose question has a post, `ThreadPost`)
  opens the body and summary, the summary's small Edit opens it alone, and
  `edit(at)` in `TopicPage` opens the body at a heading (brought
  into view as the editor draws it; the cursor is put there only by Edit).
  A document that says nothing is open anyway, and typing in it opens it.
  Save keeps it and closes it; Ctrl+S keeps it and leaves it open; Cancel
  asks before typing goes (`draft:<name>`, kept by the tab, reopens it). The
  page re-reads the topic before a save resolves, so the read text that
  follows is what was saved.
- History reads `GET /api/topics/<id>/revisions` (each with `who`, named as
  a record's history names people, and `through`, a blueprint's instance)
  again whenever the topic's `updated` changes. What it works out is plain
  values with cases in `ui/src/revisions.ts` (`historyOf`: newest first,
  what changed, which earlier one a revision says all of again, shown as
  restored), and the difference in `ui/src/worddiff.ts` (lines first, then
  the words of each run that changed, Myers' way, written here; a run with
  nothing in common but spaces and marks is said removed whole, then added
  whole). Restore sets title, body and summary through the same `PATCH`,
  which makes a new revision: no route of its own. Replies are
  a tree with a box under each (`tree`, `startsFolded`: chains longer than
  five fold after the third, never the one with the answer); the answer is
  shown first on Talk while the topic stands answered.
- A page shows its path above its title (`crumbsOf`), an editable title for
  whoever may change it, and the pages directly under it with *+ New page*
  (`freshPages` in `ui/src/topicshown.ts`). It is re-read for events of itself,
  of pages above or under it, of pages moved to or away (`movedHereOrAway`),
  and where the read topics show other children (`underOf`). An inbox line of
  a reply opens it on Talk at that reply (`showReply`).
- A page's status is chosen at its head by whoever may change it
  (`StatusPick` in `views/topicpage.tsx`, a `RowMenu` of `statusChoices`:
  the four, Answered only by marking a reply, and Not tracked for none);
  with none it shows Add status, and its replies offer no Mark as the
  answer (`mayAnswer`). The panel says a status beside a line only where it
  is answered, closed or standing (`statusSaid`).
- New fields the panel lists are in `TopicLine` read in `lineOf`; new things
  its page shows are read in `wholeOf`. Checked by `ui/scripts/topics.mjs`,
  `wikis.mjs`, `wikidrag.mjs`, `topicpage.mjs`, `wikipage.mjs`, `cited.mjs`.

## The inbox

### What a line is

- Everybody has an inbox only they read (`talk::inbox`, `talk::inboxed`). A
  line is a row of `inbox`, the factory's own table (own migration list;
  `Told::Factory`; copied, moved with no project): an owner (`user:<account>`
  or `agent:<name>`, never a bot), a kind (`named`, `message`, `told`,
  `commented`, `followed`), its subject, what opens it (`place`), who it is from, how
  often it was told, and when it was read and put away.
- It stores no message text: words are read from the message as the box's owner may
  read it now (`Channels::inbox_of`).
- One method fills it: `Inbox::tell`, or `Inbox::tell_in` inside the caller's
  write; the row and `inbox.told` are one write, and it is idempotent. A
  telling is identified by its source (`Tell::said_by`; none where the subject
  is itself the telling): the same source again writes nothing; another source
  raises the existing line (unread again, back from put away, counted, with
  the newer text), never a second row. A line is unique per owner, kind,
  subject and `why`. New things that put anything in a box call that method,
  in the write that stores the subject, and say what distinguishes tellings.

### Who owns and who hears

- Whose box is never taken from a request: it is the caller's
  (`Principal::actor`; a key's is its maker's), with `inbox:read` and
  `inbox:write` of what the caller holds. Somebody else's line answers exactly
  as a missing one, for the factory's owner and `*` too. No route, command or
  action may take an owner.
- One line is read by id (`GET /api/inbox/<id>`, `Inbox::one`,
  `Channels::inbox_line`); a read line is made unread again by its owner
  (`POST /api/inbox/<id>/unread`, `Inbox::unread_again`, `mw inbox unread`,
  `inbox.unread`).
- `inbox.told`, `inbox.read`, `inbox.unread` and `inbox.away` carry ids and
  counts, no words, and take `member:inbox:<owner>`, which no scope satisfies;
  `talk` answers it with kept-channel membership (`inbox::tells`, first in
  `kept::Memory::tells`): the box's owner, holding `inbox:read`. They are on
  `journal::HERE`, and `inbox` is in `RESERVED_TOPICS`.
- A line of a channel its owner cannot read now is skipped
  (`inbox::passed_over`: kept without them, left, or message gone): not
  listed, counted, found or marked read, and nothing is emitted about it, so
  no box hints at what its owner may not read. It returns when they can read
  the channel again. So the count is computed from what the box's owner holds in the
  emitting write (`people::held`, installed as `Inbox::holds_with`), and the
  client re-reads the box on `channel.kept`, `channel.joined`, `channel.left`.
- A removed channel takes its lines in the write that takes its rows
  (`Inbox::forget_place`); a removed agent takes its box
  (`Inbox::forget_owner`). New things a line can open are removed the same way.

### Somebody named in a message

- A message naming a person with `@` tells each named account that is not the
  author and reads the channel, in the write that stores the message
  (`inboxed::tell_named`, called by `Channels::insert`). Which account a word
  names is `people::handle` (checked with the client's `handleOf` against
  `ui/src/handles.json`), installed into `talk` (`Channels::names_with`); the
  read check is the kept-channel rule (`reads_as`). A message naming nobody
  costs nothing extra. Two accounts with the same name are both told.
- An external agent is told where messages are heard for agents
  (`Agents::tell_named`, from `dispatch`), through the same method, where it
  may read the channel, once however often the message is heard.
- An agent the factory starts gets a turn and no line, and holds no `inbox`
  scope: `AGENT_SCOPES` is also what an empty fleet `agents` row gives
  (`unsaid` in `std/fleet`, checked by `scoped.rs`). Giving such agents a box
  means adding the scope to both, a new `std/fleet` version, contract files,
  and noting that older keys and agents without it are then refused such a
  row. External agents invited with defaults already hold it
  (`agents::external::usual`, `external_scopes` of `/api/meta`).

### The inbox in the client

- The box is the reader's own in every workspace (`ui/src/inbox.ts`,
  `inboxed.ts`, `views/inbox.tsx`): a rail place below Resources (`InboxRail`)
  with a count from events, hidden for somebody with no box; pressed, the side
  panel is the box (`side` is `'inbox'`; Alt and 3).
- It is read one read at a time and again per line told; the list holds still
  while pointed at, counting arrivals at its head. A channel coming into view
  marks its lines read (`readBySeeing`, `POST /api/inbox/read` with `place`).
- A line of something said opens its channel at the message (`openTab`;
  `showThread` in a thread) in the current workspace, without moving the
  panel.
- A told line opens a page of its own (`opensItsPage`;
  `{ kind: 'inbox', id }`, `views/inboxpage.tsx`), titled by its first words
  (`titleOf`, `lineTitle`) and headed by its first sentence (`sentenceOf`,
  `firstSentence`): who told it (the bot), when (`toldWhen`), its words with
  links, what it is about (`thingOf`, `showThing`, `recordSaid`), and Mark
  unread/read and Put away (`offeredOf`). Re-read on events of the line
  (`isOf`). Address `/inbox/<id>` (owner only); not kept in Recents
  (`isThing`).
- A told line's tooltip is beside the box (`placeBeside`, `data-tip-beside`),
  long text cut at a word (`tipOf`, `TIP_LETTERS`) with a note that the whole
  is on its page (`data-tip-more`).
- Web addresses in a told line (only that kind) are links (`piecesOf` in
  `ui/src/linked.ts`: `http://` and `https://` that `supportLink` would open,
  trailing punctuation excluded), opened in a new tab (`noopener noreferrer`)
  with no prefetch (`x-dns-prefetch-control` in `ui/index.html`). Such a line
  is not a button: an empty press layer covers it (`inbox-press`) with links
  above; on touch, addresses repeat under the words (`inbox-links`).
- Checked by `ui/scripts/inbox.mjs`, `told.mjs`; routes by `inbox.rs` (API
  suite, with a key with fewer scopes); the keeper by `talk`'s suite.

## Conversations

- A conversation is a message to one or several people and agents, stored as
  a kept channel marked as a conversation (`talk::conversed`;
  `channels.conversation`). Its subject is the title; writer and recipients
  are members. Nothing a channel has is rebuilt: threads, attachments,
  search, the read rule, agent turns and answers all come from the kept
  channel and its routes.
- It is created in one write (`Channels::converse` via `Agents::converse`,
  `POST /api/inbox/conversations`, `mw to`): channel row, members, first
  message and each recipient's line with their events, or nothing. Agents are
  checked first as for any kept channel (`kept_out`). The usual channel and
  message code is handed one write (`Channels::making`, `saying`).
- Membership memory is filled before that write (and cleared if it fails), so
  the first message, emitted in the write, reaches members by memory. New
  events emitted in the write that makes a kept channel rely on that.
- It is one line in each member's box (`Kind::Message`, told by each message:
  `inboxed::tell_of`), raised by each message from somebody else, for people
  and external agents; skipped while its owner cannot read it (`passed_over`).
  Agents the factory starts get turns, not lines. A member named there is told
  only by that line. Somebody added gets their line in the adding write, from
  the latest message by another (`inboxed::tell_joined`, via `join_as`).
  Putting the line away does not leave the conversation.
- It is in no list of channels: `GET /api/channels` hands it to members with
  its mark, and every channel list leaves it out by the mark
  (`Channels::unlisted_for`, `mw channel list`, client `holdsChannel`,
  `leftOut`, `waitingIn`, `channelsWait`; `ui/src/conversed.ts`). New channel
  lists and channel unread counts leave it out the same way (the inbox counts
  it). `ensure_as` never finds one.
- Opened as a channel (`Channels::open_as_channel`,
  `POST /api/inbox/conversations/<id>/channel`, `mw channel from`) only by a
  member: placed first, then the mark removed in one write with
  `channel.kept` (`conversation` false). It is then an ordinary kept channel;
  its lines remain and still open it; it cannot be undone. `open_again`
  refuses a conversation.
- An external agent is a member as a person is, of a conversation or any kept
  channel: nothing is started for it. `talk` asks the agents keeper which
  agents it does not start (`Channels::outside_with`, beside `asked_with`),
  `Memory::of` and `Memory::reads_as` use it, and `agents` answers from its
  slots (`Hand::outside`, cleared on removal). `kept_out` and `not_brought`
  say nothing of it, and being named tells it nothing. So such a channel is
  only as private as that agent's program (stated in the README).
- Client: written from the icon at the inbox's head (`views/conversed.tsx`,
  draft kept); its page is a channel's with an envelope for the name; its tab
  is a channel tab.
- Not built: whether a message to an agent the factory starts waits instead of
  starting a turn, and any cap on conversations between agents.
- Tests: `conversed.rs` in `talk`, the API and command line suites;
  `external.rs` in agents; `ui/scripts/conversed.mjs`.

## The Members line and direct messages

- Everybody is under a Members line of the Factory part of the Resources panel
  (`ui/src/direct.ts`, `views/direct.tsx`): every person with an account
  (`GET /people`, re-read on registration and on unfolding), the reader first
  as `you`, and the agents of the panel's workspace (its projects' via
  `holdsProject`, and the factory's own). Never a bot.
- It is a fold (`FACTORY_FOLDS.members`, `factory:members`), folded at first;
  the older stored `channels:members` is dropped. Its name opens the Members
  page (`views/members.tsx`: People and Agents tabs), its arrow folds, as a
  project's line does. Panel search finds by name
  and description (`memberItems`, `ofMembers`).
- A member's line opens a menu (`RowMenu`), never an action directly:
  `memberActs` offers Details; `Message` or `Message…`; for a person, to
  whoever manages the factory, `Permissions…` (`showPermissions` in
  `views/given.tsx`); and for an agent its line's on the Agents tab (`agentActs`,
  dialogs drawn once as `AgentAsked`). Nothing the reader may not do is
  offered.
- Details shows who they are where anybody is read about (`readAbout` in
  `views/seen.tsx`: the Tools panel beside the channel showing where relevant,
  else the drawer), one drawing for both (`AgentSeen`, `PersonSeen`,
  `BotSeen`).
- A direct message is a conversation of exactly two; the daemon knows no such
  thing. Which conversation it is comes from the channels read (`directWith`:
  unarchived first, then most recent by its inbox line). With none,
  `Message…` asks for the first message and writes a conversation through the
  usual route. Unread is shown on the line (`waitsIn`) and on the folded
  Members line. Lines are `.member-line`, never `.grouped`. Checked by
  `ui/scripts/members.mjs`, `folding.mjs`, `sought.mjs`.

## Somebody named in a comment on a record

- A comment naming somebody with `@` puts a `commented` line in their box
  (`talk::commented`), a separate kind because whether it is skipped depends
  on the record, not a channel.
- It is told by the history keeper (`studio::named`, in
  `Collections::keep_history`) as it writes the comment's history line, before
  storing how far it has read, so every comment route names the same way
  (`Records::comment`, the route, `mw record comment`, `record.comment`). A
  re-read after a stop is the same telling (`about` is the comment's journal
  number, `place` the record), so nothing is told twice. It cannot share a
  write with the comment, which is in the project's file.
- Who is decided in the telling write (`Channels::tell_commented`): people by
  `people::handle`, external agents by name (`Memory::outside`), each only
  where what they hold covers reading the record (`Collection::reading`:
  `board:read` or `records:read`, plus the table's `reads`); never the author
  or a key's maker. Agents the factory starts get no line and no turn.
- Shown and skipped through `Inbox::records_with` (`commented::OnRecords`):
  current read requirements from the tables held in memory
  (`named::Declared`), so a line of a table made private since, or of an
  unowned record, is not listed or counted; words read from the history as the
  reader may read the record (`on_record`). The line stores no words.
- Client: it opens the record at the comment (`opensRecord` in
  `ui/src/inbox.ts`): a card in the Tools panel beside a channel the reader is
  in (`showCard` with `at`, `contextAt`) or on its board; a row in its form
  (`openRow`'s `at`); Comments shown, the comment marked.
- A card's and a row's form have one row of tabs at the top (`RecordTabs` in
  `views/recordtabs.tsx`; `CardTabs` in `views/cardtabs.tsx` adds the card's
  tables via `more`): Details first, then Comments (`SaidPanel`, with the box),
  History (`HappenedPanel`, no box), Attachments, and for a card a tab per
  table about cards. A table not written by hand keeps them, Details
  read-only. Flag switches line up in a column (`--flag-column`, `flagColumn`
  in `table.ts`, `FLAG_COLUMN_MOST`). The showing tab is kept per record
  (`useKept`: `card-tab:`, `row-tab:`); both open on Details, so checks press
  Comments or History first.
- The comment box is `TagBox` (`views/tagbox.tsx`, `taggable` in
  `mentions.ts`). New ways to comment emit `record.commented` with `body`, or
  nobody named is told. Checked by `ui/scripts/tagged.mjs`.
- A topic's reply is a comment, so a person named in one is told the same way
  (`a_person_named_in_a_reply_is_told…` in the API suite's `topics.rs`); an
  agent the factory starts is asked and given no line. Its reply boxes are
  `TagBox` with `among: 'topic'` (`topical`): every person, and every agent,
  one the factory starts and runs said to answer when named (`offeredUnder`).
- A name said is drawn as a mention wherever somebody's words are drawn
  (`Prose`, and a record's comments through `PathsIn`/`Written` with
  `named`): `withMentions` in `views/mentioned.tsx` over `saidIn` and
  `namedBy` (`mentions.ts`). A name of somebody known (a person by
  `handleOf`, an agent, the factory's bot) is underlined in the accent
  (`mentioned.css`) and is a `Who`: pointed at or focused it shows their
  card, and its name says who it is (`whoIs`); pressed it shows them only
  beside a channel (`still` elsewhere). A name of nobody stays plain.

## Somebody who follows a page

- Following is per page (`Topics::follow`, the `followers` cell); an agent
  follows nothing. A follower has one `followed` line a page (`talk::followed`,
  `about` and `place` the page, `why` `follows`), raised by each change:
  `said_by` is the change's number in the factory's journal, which is where
  every topic event and reply is.
- Which changes tell is `kernel::topics::tells_followers` (a reply, a
  revision, a status, a move to another page or the top, a wiki placed
  elsewhere; not a reorder, nor a page carried along). The words of what it
  did are `kernel::topics::change_said`, handed out as `OnRecord::did`.
- Told by the history keeper as a reply's mentions are (`history.rs`,
  `followed` beside `named`; `studio::named::change`, `followers`): the
  change's history line is written, then followers are told in one write,
  then the place is kept. A change heard again after a stop raises nothing:
  a line already of that change, or of a later one (`said_by` the greater),
  is left as it is (`followed::tell_in`). Checked across stops by the
  simulation in the API suite's `followers.rs`.
- Who: whoever follows the page as the keeper comes to the change and
  followed it then (`followed_at`: `topic.followed` said since, read back,
  `kernel::topics::followed_then`; the keeper may come to it late), but
  whoever made it (a key's maker included), where what
  they hold in the telling write covers reading the table
  (`Collection::reading`); shown and passed over as a `commented` line is
  (`passed_over`, `OnRecords::said` at `said_by`).
- A follower named in a reply is told by their `followed` line alone where
  they are told at once: `tell_commented` tells followers first in the same
  write (`followed::tell_in` hands back who follows at once) and names
  nobody of those, as a conversation's member named there is told by the
  conversation's line alone. One told hourly, daily or not at all is named
  as anybody is.
- **With its pages.** A follow's row (below) says `pages`; the keeper hands
  over the pages above the changed one, nearest first, each with its
  `followers` cell (`Collections::following`, walking `parent`, a ring ends
  where it comes round), and `followed::tell_in` tells each of those who
  follows that page with its pages and followed it before the change
  (`since`, the number of the `inbox.follows` that said so, in the factory's
  journal like every topic event) by the line of that page: `about` the
  followed page, `place` the page that changed. One line a change a
  follower: the page itself first, then the nearest above. A page made
  (`record.created`) tells only those above it; its first revision, said
  with its making, tells nobody (`kernel::following::page_change`).
- Client: `wordsOf` says `did` for a change that is no reply; `opensRecord`
  opens the page at the reply, or at its head (`at` null). Checked by
  `ui/scripts/tagged.mjs`.

## Following anything, and how often

- **Rows.** `follows` (talk's migration list `follows`, the factory's own,
  `talk::following`): who, `kind` (`page`, `board`, `table`, `library`,
  `channel`, `project`, `kernel::following::KINDS`), `thing` (a page's id,
  `<project>/<table>`, a channel's or project's id), `pages`, `often`
  (`once`, `hourly`, `daily`, `off`), `at` (`HH:MM`), `since`, `told`,
  `made`. A page is followed by its `followers` cell still; its row says
  how, written before the cell on follow and gone after it on unfollow, and
  a page followed by the cell alone (an earlier version) is at once and
  alone (`GET /api/following` fills it in). Each change of a row is one
  write with `inbox.follows`, heard by its owner alone. Whoever follows is
  asked whether they read the thing in that write (`reads_thing`: a page
  and a table by `OnRecords::takes`/`table_takes`, a channel by the one
  question every reading of one asks, a project by `projects:read`); an
  agent follows nothing. Routes: `GET`/`POST /api/following`, and
  `POST /api/topics/<page>/follow`, which goes through the same
  (`api::routes::follows::followed`). `mw follow|unfollow|following`.
- **What changed** in anything but a page is heard by `api::followers::tell`
  from every journal (`record.created|updated|removed|commented`,
  `channel.message`, `project.renamed|placed`; which things each is of is
  `kernel::following::thing_change`, how a row is called by its table's
  kind is `worded`). A change of nothing followed (`Held::anybody`, kept in
  memory as rows are written) costs no reading. One that is followed is
  told in one write that also keeps where the listener has heard to
  (`following` / `heard` in the factory's kv): told once whatever stops.
  Who may: `followers::may`, the journal's one question with the event's
  `takes` and `Engine::may_hear` of its topic, as the follower holds things
  in the write, and `reads_thing`; nobody of their own doing (a key's maker).
- **At once**, anything but a page: one `following` line a thing (`about`
  and `place` `<kind>:<thing>`, `why` `follows`), raised by each change;
  each change is a row of `follow_changes` naming the line, and `says` is
  what those rows sum to (`kernel::following::line_said`) since the line
  was last read (a read or put-away line's rows go as the next change comes).
- **Digests.** Hourly or daily, a change waits as a `follow_changes` row with
  no `line` (a page's from the history keeper, `followed::tell_in`, guarded
  by `told`; anything else's from the listener). `api::followers::digests`
  asks what waits (`Channels::waiting`), works out when each is due
  (`kernel::following::due`: the next whole hour after the first change, or
  the next `at` in the follower's zone from `people::Clocked::zone_of`,
  else UTC, by `reader::at`), tells what is due as one line in one write
  (`Channels::digest`: `about` `<kind>:<thing>#<due>`, `why` `digest`,
  `said_by` the due time; its rows take the line, those made after the due
  time wait for the next; what the follower may not read now goes untold),
  and sleeps on the daemon's clock until the next is due or a change comes
  to wait (`Held::wakes`). Nothing waiting, no line; stopped anywhere, what
  waits is told after the start, once.
- `following` lines are passed over while their owner does not read the
  thing (`passed_over`, `reads_thing`); `GET /api/inbox/<line>/changes`
  lists a line's rows with what each record is called now.
- Checked by the API suite's `following.rs` (a wiki with a page added later,
  a table and a channel, hourly and off on a `ManualClock`, once across a
  restart, what may not be read) and the kernel's `following.rs`.

## Blueprints telling somebody: `inbox.tell`

- A blueprint tells somebody with a `told` line (`inbox.tell`,
  `crates/talk/src/telling.rs`) through the one method, keeping the
  blueprint's words, from the bot.
- It runs as whoever applied the blueprint and reads nothing for the
  recipient: it takes the applier's `inbox:write`, and `about` and `place` are
  ids nobody resolves. So a told line is never skipped and its words are shown
  regardless; only opening the subject is checked, by existing routes
  (`showThing` in `ui/src/views/inboxpage.tsx`). Nothing may read a card,
  record or channel in order to tell of it as somebody else.
- Whom: whoever applied the instance (`applier`; a key is its maker), an
  actor, or a handle, resolved in the telling write (`Channels::names_with`,
  `Inbox::held` over `holds_with`). A word naming nobody, or a bot, refuses
  the whole telling first; somebody without `inbox:read` is skipped and listed
  in `passed` (agents the factory starts, for now).
- A re-run step tells nothing (`said_by` is run, step and `ActionCx::doing`);
  another telling of the same subject for the same `why` raises the line.
  Whatever tells the same thing from two places at once makes them take turns,
  as `std/told` does (`one_at_a_time`).

### A fleet's telling: `std/told`

- `std/told` is attached by the fleet (`uses`), not spawned: it hears the
  fleet's failures as the fleet does (`scope: target`, the guards of
  `trouble`, `let_be`, `slipped` with `target.id`), never its own.
- Its other handlers hear the project's events by `for:` (`card.asked`,
  `feature.asked`, `card.left`, `feature.planned`, and `card.planned` where it
  says `waits`). A new thing a fleet tells of is an emitted event, a handler
  and a block case, never a look at rows.
- Whom is the rows of `tells`, declared beside the board (`whom`, taking
  `inbox:write`; `of`); the recipient and sentence are a block
  (`deciding::told`, `tells.whom`).
- Handlers for what waits for a person let a failed telling fail their run
  (reported by `slipped`); handlers for failures let it pass.
- Where whoever applied the fleet may put nothing in an inbox, it says so once
  in the fleet's channel and stops itself (`gone`), to be attached again by
  the next apply. Both it and the fleet ensure the fleet's channel;
  `Channels::ensure_as` gives both the same one.
- A card that waits for nobody runs none of this (`tells.rs`). A fleet an
  earlier version started attaches nothing until applied again. Checked by
  `ui/scripts/told.mjs` and `telling.rs` (simulation).

### A card that asks is told by naming

- `asked` posts the question on the card (`record.ask`, `questioned.rs`) as
  whoever asked (an agent via `decided`; an A/B test via the bot, `asker`),
  naming with `@` each person or agent the `tells` rows give
  (`Channels::whom`, `people::handle` or agent name). The speaker is the bot or
  the agent, never whoever applied the fleet, who is told where named. The step
  records its asking (`once`) and posts once even after a stop.
- No told line is emitted for it: being named is the telling, limited to
  whoever may read the card. Asked of nobody, it says so (`otherwise`,
  `unasked`) and where the fleet reports (`untold`).
- A card's Details shows whom it waits on (`questionOf`, `waitingOn` in
  `ui/src/journey.ts`) and leads to Comments with the box focused (`answerIn`
  in `ui/src/cardasked.ts`); the repeated `card.asked` line is under History
  (`saidAgain`). `feature.asked` is still a told line. Checked by
  `review.mjs`, `tagged.mjs`.

## A told line's parts

- What a told line shows beside its words is the blueprint's choice (`parts`
  of `inbox.tell`), stored on the line as told, checked and filled in
  (`kernel::parts::read`). What a part evaluates to is computed when read and
  pressed, never stored (`GET /api/inbox/<line>/parts`, in
  `api/src/routes/reviewed.rs`, over the `told` crate).
- One thing is stored: a part listing a column's cards keeps which cards the
  column held when told (`kept`, `kernel::parts::keep`), so the line keeps
  listing them after they move. It is read just before the telling write
  through what the studio installs (`Channels::columns_with`), never in that
  write, because the factory's thread never waits on a project's. A raised
  line keeps the column as it is then. Kept ids are not handed out as such
  (`kernel::parts::shown`): each is read as the line's owner may read the board, and
  a card the line was not told of cannot be pressed from it.
- A `cards` part is read with the reader's authority (`board:read`). Pass or
  fail is `POST /api/inbox/<line>/parts/<n>/cards/<card>` as the presser:
  `board:write` first, then `kernel::parts::press` (still listed, a reason
  where asked for, and never an agent on a card it worked on or is assigned).
  Fail moves the card with the reason (`Board::move_said`). Pass records
  `card.approved` (a history line, not a comment) and then moves it to
  `pass_to`, presses `pass_press` through `Board::press`, or nothing more.
  Nothing is stored on the line. `card.approved` is described in `api`'s
  `events.rs`; `card` is not in `RESERVED_TOPICS` because blueprints emit it.
- Three more kinds are read as the reader may read what each is of, saying why
  in place where not (`told::read`): `rows` (a project table's rows by id or
  cell value, chosen by `kernel::rows::read` as `rows.read` does, `saying`
  taking `{ in: [...] }`; read via `Records::list`, so private tables show
  none); `runs` (of an instance, of a key, or of an environment,
  `environments::last::runs`; `blueprints:read`); `measures` (`fleet:read`,
  latest beside the median of the last few, `kernel::parts::measured`,
  counted, never scored).
- A part may name its `tab` (`kernel::parts::tabs`, `tabsOf` in
  `ui/src/toldread.ts`): untabbed parts go on the first, called Details where
  there are others; one tab draws no tab row. The page draws parts from what
  the daemon reports (`views/reviewed.tsx` over `ui/src/reviewed.ts`): Pass
  and Fail only where allowed. Checked by `ui/scripts/reviewed.mjs`,
  `toldtabs.mjs`.
- A new part kind is a kind of `kernel::parts` with cases, a row of `INSIDE`,
  and an arm in `told::read` with the authority reading it otherwise takes; a
  new press action is a case of `press`.
- What was said on a card for its agent is `record.said` (`studio::said`, over
  `kernel::parts::discussed`), which `std/work` appends after the note of
  `lane-round.md`; that prompt has no blank of its own for it, because a
  project's earlier round links it and is checked against its blanks.
