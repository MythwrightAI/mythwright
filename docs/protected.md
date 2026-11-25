# What is protected, and what is not

Read this before running it anywhere that matters.

- **Everything is asked with a credential, and a credential can do only
  what it holds.** The daemon listens on this machine only unless told
  otherwise. It answers a request only when the request was addressed to
  it (an address, `localhost`, or a name its owner told it: `mw names add
  <name>`), so a page you merely visit cannot ask it things under a name
  of its own, and it refuses one that changes something and comes from
  another site. The first account, which owns the installation, is made by
  its first start, on the machine itself, with no password until its owner
  sets one.
- **Something outside can say an event, with a key for that topic.** A
  key made with `--scope emit:<topic>` lets its bearer say that one topic
  of a blueprint's own and do nothing else, and only such a key is taken
  in an address (`/in/<key>/<topic>`), for a service that can be given
  nothing but an address to call. Whatever a handler does on that topic,
  the bearer of that key can set off: give the handler no more to do than
  you would let them ask for. The event says which key said it, by an id
  the factory gave the key and nobody chose.
- **A service that signs what it sends is let in by its signature.** One
  you describe (`mw caller set <name>`: its topic, its header, which of
  the factory's secrets it signs with) is given an address with no key in
  it, and each call is taken only if it carries an HMAC-SHA256 of what was
  sent, made with that secret. Where the service signs the time as well, a
  call is refused once it is five minutes old and is said once however
  often it is sent in that time. Where it signs no time, a call that
  somebody saw can be sent again for as long as the secret stands: that is
  the service's choice, and a handler should be safe to run twice.
- **An agent has no key of yours, and is handed one made for it.** Behind
  its wall an agent's program has none of your sign-ins to other services.
  One that is to push, or to ask a service something, is handed a secret
  the factory keeps, under a variable its program reads (`mw agent add
  <name> --secret GH_TOKEN=github-bot`). What that key allows is the
  service's to hold to, so make it a narrow one; the agent can send what
  it holds anywhere. What the factory itself does at a remote (sending a
  branch, opening a pull request) it does as you, unless you tell it a
  key to go by (`key` of the fleet, and of the package that opens pull
  requests): a bot's token, which the factory keeps and git and that
  package's program then go by, and by nothing of yours.
- **A secret is handed to a program and kept out of everything else.**
  A key or a password a program needs is kept by the factory under a name
  (`mw secret set <name>`), and a blueprint says the name where the
  program reads it. It is in no event, log, database or copy of the
  factory, and a program that prints what it was handed has the secret's
  name written down in its place, before its output reaches the disk (a
  value shorter than six letters, or printed changed, is not caught). The
  client's *Secrets* page, the owner's, lists each with who set it and
  what names it; a value is typed once and nothing reads it back. Where
  it is kept it is sealed, as the sign-ins of agents'
  programs are, with a key made from the installation's own secret: a
  copy of the file, or of the folder it is in, is not the value, and does
  not open on another installation or under another name. It is not kept
  from the program it is handed to, nor from whoever can read the whole
  data folder as the daemon does, since the secret the key is made from
  is in it.
- **Part of a project can be kept private.** A table says what reading it
  takes (`reads: private:journal`), and its records, the files they are
  kept in, what searching finds of them and what is said as they change
  are then for whoever holds that: you, and whoever you give it to. An
  agent working in the project that was not given it is not shown them,
  from the factory or from behind its wall.
  [docs/library.md](library.md), *Kept private*.
- **A channel that is removed goes into the Trash, and once it is deleted
  from there it is gone from the factory, and from nowhere else.** Only the
  owner removes one. Its messages go with it, what was attached to them,
  what agents were asked there, and what the journal and the search index
  kept of what was said. Until it is deleted for good the Trash holds it
  (`mw trash restore <id>` gives it back), and so does a copy of the
  factory made meanwhile; once it is, a copy made afterwards holds none of
  it. Where the factory keeps nothing that is deleted (`mw trash --keep
  0`) it goes at once. What an agent's program remembers for itself, a
  copy made before, and whoever had read it are not reached.
- **A project made of a folder that was already there is not trusted.**
  None of its files is read as a blueprint of its own unless you say a
  folder of it is (`mw project blueprints <name> <folder>`), and even
  then a file there with the name of a blueprint that ships (`std/board`,
  say) is not used in place of the one that ships until you say so:
  Trust this project… in the gear of its page, or `mw project trust
  <name>`; Stop trusting… there takes it back. Read what somebody else's
  repository carries before you do. What the factory keeps for it (a
  fork you made) is used either way: only you wrote it.
- **An agent's program works behind a wall, on Linux.** What an agent asks
  of the daemon is held to what its slot allows. Its program is a process
  on your machine, started as you; so it is started behind a wall, held by
  the system's own bubblewrap, and of your machine it is shown its work, a
  home of its own, the folders its own program signs in with, and what you
  add (`mw help wall`; `mw agent try <name>` starts its program there and
  says whether it has what it needs, without giving it work). What it may
  do with the repository is yours to say of each: read it, commit its own
  work, or all of it, and never its settings or hooks. Your keys, your session, the factory's data and
  your other projects are not there for it to read, and `read` is held to
  reading. It still reaches the network, and it can read the sign-in it
  was shown: one made for agents costs least to lose. Where the system has
  no bubblewrap, or will not let it work, an agent is not started until
  you install it or tell that agent to work with no wall (`--no-wall`);
  such an agent can read what you can read, and is work you would give a
  person with your login. A command a blueprint runs, and a service it
  hosts, have no wall.
- **The client and the command line are not encrypted by the daemon.**
  Started with `--listen` on an address other machines reach, passwords
  and credentials cross the network in the clear, and it says so as it
  starts: put it behind something that encrypts (a tunnel, a proxy).
- **A device and its hub are sealed to each other.** Each proves a key to
  the other in a handshake, and what crosses is read and changed by
  nothing in between, on any network. What is still yours to mind: an
  invitation lets one device in, once, so until it is used it is passed
  on as a password is; a member does what its hub sends, as no more than
  it was joined to allow (`--allow`, everything unless narrowed); and the
  two have to be able to reach each other. Reaching an installation that cannot be reached directly is
  not something this application does.

[docs/architecture.md](architecture.md), *Identity*, has each of these
with its reason. A problem with any of it is reported privately:
[SECURITY.md](../SECURITY.md).
