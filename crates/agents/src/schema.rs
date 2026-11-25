//! What the agents' part of the database is made of, and each change to
//! it since: applied in order, once each.

pub(crate) const SCHEMA: &[&str] = &[
    "
    CREATE TABLE slots (
        name TEXT PRIMARY KEY,
        adapter TEXT NOT NULL,
        model TEXT,
        project TEXT,
        instructions TEXT NOT NULL,
        scopes TEXT NOT NULL,
        permission_mode TEXT NOT NULL,
        active INTEGER NOT NULL,
        created INTEGER NOT NULL,
        updated INTEGER NOT NULL
    );

    CREATE TABLE turns (
        id TEXT PRIMARY KEY,
        slot TEXT NOT NULL,
        kind TEXT NOT NULL,
        thread TEXT NOT NULL,
        prompt TEXT NOT NULL,
        channel TEXT,
        reply_to TEXT,
        options TEXT NOT NULL,
        status TEXT NOT NULL,
        reply TEXT NOT NULL DEFAULT '',
        error TEXT,
        decision TEXT,
        cost_usd REAL NOT NULL DEFAULT 0,
        input_tokens INTEGER NOT NULL DEFAULT 0,
        output_tokens INTEGER NOT NULL DEFAULT 0,
        created INTEGER NOT NULL,
        started INTEGER,
        finished INTEGER
    );
    CREATE INDEX turns_slot ON turns (slot, status);
    -- One turn per agent per message, however many times the message is seen.
    CREATE UNIQUE INDEX turns_message ON turns (slot, reply_to) WHERE kind = 'message';
",
    "
    -- How much an agent may do is said in the product's own three words now,
    -- not in those of one agent's program. Anything else becomes the middle one.
    UPDATE slots SET permission_mode = 'edit' WHERE permission_mode NOT IN ('read', 'edit', 'full');
",
    "
    -- What a turn is measured with besides its own agent and kind, as JSON:
    -- the labels of the step it was taken for.
    ALTER TABLE turns ADD COLUMN labels TEXT NOT NULL DEFAULT '{}';
",
    "
    -- The directory of its project a turn is taken in, where it is not the
    -- project's own: a lane's worktree.
    ALTER TABLE turns ADD COLUMN cwd TEXT;
",
    "
    -- What a slot draws its sign-in from, in order, as JSON: nothing, for
    -- one whose program signs in as whoever runs the daemon.
    ALTER TABLE slots ADD COLUMN signins TEXT NOT NULL DEFAULT '[]';

    -- The sign-ins the factory holds. Never the value of one: that is in a
    -- file of its own, which no copy of the factory carries.
    CREATE TABLE signins (
        name TEXT PRIMARY KEY,
        package TEXT NOT NULL,
        kind TEXT NOT NULL,
        at_once INTEGER NOT NULL,
        folder TEXT,
        last_used INTEGER,
        held_by TEXT NOT NULL DEFAULT '[]',
        aside_until INTEGER,
        aside_why TEXT,
        created INTEGER NOT NULL
    );
",
    "
    -- The role a slot was brought up in, if any: a word of its fleet's
    -- own, by which slots that do the same kind of work are told from the
    -- rest. Nothing here knows what any role is for.
    ALTER TABLE slots ADD COLUMN role TEXT NOT NULL DEFAULT '';
",
    "
    -- A slot that is leaving is given no more work, and is stopped when
    -- what it has in hand is finished.
    ALTER TABLE slots ADD COLUMN leaving INTEGER NOT NULL DEFAULT 0;

    -- Whose a conversation is, is asked each time work is given.
    CREATE INDEX turns_thread ON turns (thread);
",
    "
    -- How a slot is ranked among those of its role: a number its fleet
    -- says, by which the higher is chosen before the lower where whoever
    -- asks says to choose so. Nothing here knows what makes one rank higher.
    ALTER TABLE slots ADD COLUMN rank INTEGER NOT NULL DEFAULT 0;
",
    "
    -- Whether a slot's program is started behind a wall, and what it is
    -- shown there besides its work: folders to read and folders to change,
    -- each as JSON. A slot that was here before there were walls goes on
    -- as it was, with none, until somebody says otherwise; one made from
    -- now on has one unless it is told not to.
    ALTER TABLE slots ADD COLUMN wall INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE slots ADD COLUMN reads TEXT NOT NULL DEFAULT '[]';
    ALTER TABLE slots ADD COLUMN changes TEXT NOT NULL DEFAULT '[]';
",
    "
    -- What a slot's program may do with its project's repository behind
    -- its wall: nothing (`off`), read it, commit its own work, or write
    -- to any of it but what git would run.
    ALTER TABLE slots ADD COLUMN git TEXT NOT NULL DEFAULT 'read';
",
    "
    -- The secrets a slot's program is handed, as JSON: each variable of
    -- its environment with the name the factory keeps the secret under.
    -- Never a value.
    ALTER TABLE slots ADD COLUMN secrets TEXT NOT NULL DEFAULT '{}';
",
    "
    -- Whether a slot's program goes on without the daemon: started so that
    -- it does not end with it, and taken up where it is by the next one.
    -- Off unless somebody says so: such a program works with nobody
    -- watching while the daemon is down, which is its owner's to choose.
    ALTER TABLE slots ADD COLUMN outlives INTEGER NOT NULL DEFAULT 0;

    -- The same of one turn, as it was when the turn was asked for.
    ALTER TABLE turns ADD COLUMN outlives INTEGER NOT NULL DEFAULT 0;
    -- How many times a daemon was started and found the turn in hand.
    ALTER TABLE turns ADD COLUMN restarts INTEGER NOT NULL DEFAULT 0;
    -- What whoever asked for it said becomes of it then: begun again (1)
    -- or left as it was for a person (0), and after how many such starts
    -- at most it is begun again whatever was said.
    ALTER TABLE turns ADD COLUMN again INTEGER NOT NULL DEFAULT 1;
    ALTER TABLE turns ADD COLUMN most INTEGER NOT NULL DEFAULT 3;
    -- When the time it was given runs out, in milliseconds since 1970, where
    -- whoever asked for it gave it a time: held to by a daemon that comes
    -- back and finds its program still at work by itself.
    ALTER TABLE turns ADD COLUMN until INTEGER;
",
    "
    -- What an asking is, in a word of whoever asked, and who that was (who
    -- applied it, and the key it is known by). Asked the same again by
    -- the same asker in the same word
    -- (a restart took away what waited for the answer), the turn that was
    -- asked for then is this asking's, whatever has become of it, and the
    -- agent is not asked a second time. Nothing, where no word was said.
    ALTER TABLE turns ADD COLUMN asking TEXT;
    ALTER TABLE turns ADD COLUMN asked_by TEXT;
",
    "
    -- Whether a slot's program may use the web by itself: look things up
    -- and fetch pages with whatever of its own does that, unasked. Off
    -- unless somebody says so, for a slot from before as for a new one:
    -- a page may say anything to an agent, and what an agent looks for
    -- or asks an address for carries whatever it puts in it.
    ALTER TABLE slots ADD COLUMN web INTEGER NOT NULL DEFAULT 0;
",
    "
    -- Whose a turn is: the project its slot was of when the turn was
    -- queued, and the empty text where the slot was no project's. Said
    -- in the write that queues it and never another afterwards, since an
    -- agent may be given to another project and what it did before is
    -- not that project's. A turn kept before this says nothing, and is
    -- reached through its slot: the column is added and no row is
    -- written, so this reads none of a journal from before.
    ALTER TABLE turns ADD COLUMN project TEXT;
    CREATE TRIGGER turns_project_stays BEFORE UPDATE OF slot, project ON turns
    WHEN NEW.slot IS NOT OLD.slot OR NEW.project IS NOT OLD.project
    BEGIN
        SELECT RAISE(ABORT, 'a turn is of the agent it was asked of, and of the project that agent was of then, and of no other afterwards');
    END;
",
    "
    -- Which turn it is of all the factory's agents were ever asked for,
    -- whichever journal's file keeps it: a project's turns are kept in
    -- the project's own file, and where a row comes in one file says
    -- nothing of a row in another. Given as the turn is queued, from one
    -- count for the factory that only goes up. A turn kept before this
    -- says nothing, and is ordered by where it came in the one file there
    -- was, before any that says: the column is added and no row is
    -- written, so this reads none of a journal from before.
    ALTER TABLE turns ADD COLUMN nth INTEGER;
",
    "
    -- An agent that is taken away leaves its turns, as what it was asked
    -- and what it answered, and a turn's slot is never written again. So
    -- that nothing of them is a later agent's of the same name, a slot
    -- says which turn of all was the last its name had before it was
    -- made: the turns numbered above that are its own. Nought for a slot
    -- whose name nobody had before it, and for every slot from before:
    -- the column is added and no row is written, in a table of a line
    -- for each agent.
    ALTER TABLE slots ADD COLUMN since INTEGER NOT NULL DEFAULT 0;
",
    "
    -- The channel kept to its members that a turn is of, by its id: the
    -- one it was asked in, says its answer in, or goes on with a
    -- conversation of. Said in the write that queues the turn. What is
    -- said of such a turn takes being a member of that channel, its
    -- agent's key opens the channel while it is in hand, and its words
    -- are shown to whoever reads the channel and to nobody else, the
    -- channel being there or gone. Nothing for a turn of no such
    -- channel, and for every turn from before: the column is added and
    -- no row is written.
    ALTER TABLE turns ADD COLUMN kept TEXT;
",
    "
    -- The environment of its project's an agent works under, by name:
    -- what of it its program is handed (what is marked for agents) and
    -- where the keys its slot names are looked for first. The empty text
    -- for none, which every slot from before is: the column is added
    -- with a default that is the same for every row, in a table of a line
    -- for each agent.
    ALTER TABLE slots ADD COLUMN environment TEXT NOT NULL DEFAULT '';
",
    "
    -- An agent from before agents had a text of their own (`std/agent`),
    -- which a start gives each once, over its slot as it stands, where
    -- nothing running brings it up and it is in: noted on every row there
    -- is as the column is added, and taken off when it has been looked
    -- at. The empty text for none, which every agent made since is.
    ALTER TABLE slots ADD COLUMN owes TEXT NOT NULL DEFAULT '';
    UPDATE slots SET owes = 'text';
",
    "
    -- What an agent's program said it was refused as its turn went, each
    -- thing by a word, as a list in JSON, where its package says how its
    -- program tells it: shown with what the turn came to, and deciding
    -- nothing by itself. Nothing for a turn that says none, and for every
    -- turn from before: the column is added and no row is written.
    ALTER TABLE turns ADD COLUMN refused TEXT;
",
    "
    -- Folders of its project a turn's program is not shown, from the
    -- project's top, as a list in JSON: the places other pieces of work
    -- on the same item are done in, where several are done at once.
    -- Nothing for a turn that hides nothing, and for every turn from
    -- before: the column is added and no row is written.
    ALTER TABLE turns ADD COLUMN hidden TEXT;
",
    "
    -- Whether its program said what the turn cost: 1 where it did, 0
    -- where it said nothing of it (a package that reports only tokens),
    -- so that a cost never reported is not read as nought. Nothing for
    -- every turn from before, whose nought cannot be told from one never
    -- reported, and is read as reported only where it is more than
    -- nought: the column is added and no row is written.
    ALTER TABLE turns ADD COLUMN cost_said INTEGER;
",
];
