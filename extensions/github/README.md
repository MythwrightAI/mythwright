# GitHub

Open a pull request on [GitHub](https://github.com) for each branch a
project's lanes send there, and keep the cards of a project that are
linked with an issue there in step with it (*Issues*, below).

A lane keeps what its agent changed as a commit on a branch of its own,
`lane/<card>`, and with `push` sends that branch to the repository's
remote. That is as far as the application goes: it knows no service that
hosts repositories. When the branch has been sent, the lane says so, as an
event of its card (`card.pushed`). This package brings one blueprint,
`github/pull-requests`, which hears that for one project and asks the `gh`
program on the device to open a pull request for the branch:

- its **title** is the card's title;
- its **body** is the card's body, and under it which card it was;
- it asks to be merged into the repository's default branch, unless `base`
  says another;
- a branch that **already has an open pull request is not given another**.
  A card that is sent back from review and pushed again is in the pull
  request it had, and nothing more is done.

Where the pull request is, is said in the project's channel, and of the
card (`card.proposed`), so that the card's history keeps it. One that could
not be opened is said in the channel too, with why. Nothing here merges,
closes or deletes anything, on the device or on GitHub.

It also brings the actions that blueprint does, each one way of running the
program (`mw actions github.find` says what one takes and gives):

| Action | |
|---|---|
| `github.account` | Sees that the program is installed and signed in. |
| `github.find` | Finds the open pull request for a branch, if it has one. |
| `github.open` | Opens a pull request for a branch, with a title and a body. It does not look whether there is one already. |

The application runs the program itself for each, with the arguments
written in `extension.yaml` and no shell, in the project's folder: the
program finds the repository, and which of its remotes is on GitHub, from
where it is run. What is written on a card is given to it as one argument
each, after the `=` of the option it belongs to, so that none of it can be
taken for an option or run.

## What it needs

The `gh` program ([cli.github.com](https://cli.github.com)) installed on
the daemon's device, and somebody for it to act as at GitHub with leave to
open pull requests in the repository: whoever runs the daemon, signed in
there (`gh auth login`), or a key the factory keeps (*As a bot*, below). A
project that is a git repository with a remote on GitHub, and a fleet
started with `push`, so that its lanes send their branches.

If the repository has more than one remote, `gh` decides which it means;
`gh repo set-default`, run once in the project's folder, says which.

## Turning it on

```sh
mw extension add github
mw apply fleet --project shop --param push=true
mw apply github/pull-requests --project shop
```

The last is applied once and then lives for as long as pull requests are
wanted; `mw stop <instance>` is how that ends. Applying it says at once if
the program is not installed or not signed in.

| Param | |
|---|---|
| `channel` | Where to say that a pull request was opened, or could not be. Left out, the channel called what the project is, which is where its fleet reports. It is made if it is not there. |
| `base` | The branch each pull request asks to be merged into. Left out, the repository's default branch. |
| `key` | A secret the factory keeps that the program acts as at GitHub: a token of a bot's account or an application's. Left out, it acts as whoever is signed in to it on the device. |

It needs `extensions:github`, which is leave to run this package's program
in the ways this package says, with `board:read` to read the card and
`channels:write` to say where the pull request is. No `shell`. People have
those unless they were taken away; an agent has `extensions` only if it was
given it. Told a `key`, whoever applies it also holds `secrets:use` and
`shell`, as for any program handed a secret in its environment.

## As a bot

Left as it is, the branches are sent and the pull requests opened by
whoever runs the daemon: with their sign-in, under their name, and with
everything their account may do. To have a bot do both, make a token for
it at GitHub that may do what this needs in the one repository and no more
(a fine-grained token with *Contents* and *Pull requests* set to read and
write is enough), and give it to the factory to keep:

```sh
mw secret set shop-bot < token.txt
mw apply fleet --project shop --param push=true --param key=shop-bot
mw apply github/pull-requests --project shop --param key=shop-bot
```

The first `key` is the fleet's: git sends each branch as the token, and
asks nobody else who the remote is reached as. It is given only to the
address the repository has for the remote, which has to be an `https` one
(`git remote set-url origin https://github.com/<owner>/<repository>.git`).
An application's installation token is given under the user
`x-access-token`: write that into the address,
`https://x-access-token@github.com/...`. The second is this package's: the
program is handed the token as `GH_TOKEN`, which it goes by in place of
anything it has kept, so each pull request is opened by the token's
account. A token GitHub will not take is said when the blueprint is
applied, and sends and opens nothing as anybody else.

What the token allows is GitHub's to hold to, and the factory does not
read it. The commits themselves are still by whoever git is set up as in
the project (`git config user.name` and `user.email` there).

## Issues: tickets kept in step, one at a time

`github/issues` keeps a card of a project's board and an issue of its
repository the same, for the tickets that are **linked** and for no
others. Nothing is copied wholesale: a card is here only, and an issue is
on GitHub only, until somebody links the two.

```sh
mw extension add github
mw apply github/issues --project shop
mw record add shop links card=shop-12      # an issue is made for the card
mw record add shop links issue=48          # a card is made for the issue
mw record add shop links card=shop-7 issue=31   # two that were there already
```

A link is a row of the project's table `links`, which the client shows as
any table, and a card that is opened shows the link that names it. A row
that names a card has an issue made for it, with the card's title and body;
one that names an issue's number has a card made for it, filled as the
issue stands; one that names both ties two tickets that were there, and
where they differ the card's words are put on the issue unless `prefer`
says `there`.

**Who may make a ticket public.** Adding a row of `links` takes
`extensions:github`: it has the program write a card's words on GitHub as
whoever the program acts as. People hold that unless it was taken from
them; an agent holds it only if it was given it. Somebody who may not read
a table cannot link its rows either: naming a record takes being able to
read it.

Of a linked ticket:

- **its title and body** are the same on both sides. Changed on the card,
  they are changed on the issue. Changed on the issue, they are changed on
  the card as `takes_in` says, below. Changed on both since they last
  agreed, the card's are taken, unless `prefer` says `there`;
- **whether it is open**: a card moved to the board's last column has its
  issue closed, and one moved out of it has it opened again; an issue
  closed on GitHub has its card moved to the last column, and one opened
  again to the first. `closed` and `reopened` say other columns. Moves
  between other columns stay here;
- **what is said on the issue** is said on the card, once, with who said
  it and where it is, and is never marked public;
- **what is said on the card stays here**, unless whoever says it marks it
  public (`mw record comment shop-12 --public ...`, or the switch beside
  sending in the card's drawer) and holds `extensions:github`. Then it is
  said on the issue, once, with their name.

A link that is taken away stops its ticket being kept in step, and leaves
the card and the issue as they are. Nothing here removes anything, on the
board or on GitHub.

### Who can write what an agent works from

A repository may be public, and whoever can write an issue or say
something on one is nobody the factory knows. A card's title and body are
what an agent works from, and what somebody says on a card whose agent
asked something is taken for the answer. So what comes in from GitHub is
held to one setting, `takes_in`:

| `takes_in` | A change of a linked issue's title or body | What is said on the issue |
|---|---|---|
| `trusted`, unless said | Comes into the card where GitHub says whoever changed it has standing in the repository: its owner, a member of its organisation, a collaborator. A change by anybody else is **held**. | Is said on the card. Answers an agent only where whoever said it has that standing. |
| `everyone` | Comes into the card, whoever changed it: anybody who can write there writes what the card's agent works from. | Is said on the card, and may answer an agent, whoever said it. |
| `nobody` | Is not taken and not written over. The card's words are changed only here, and go out when they are. | Is said on the card, and answers nothing. |

Whatever the setting, what is said on an issue is said on the card with
whether GitHub says its author has standing (`bo said on GitHub:`, or `bo,
who has no standing in the repository, said on GitHub:`), so that a person
or an agent reading the card knows whose words they are.

**A change that is held** is not lost. The link's `held` says `waiting`
and `held_by` says whose change it is, and the card is told once, in
words nobody outside sees, what was changed and what to do. While it
waits, the card's title and body and the issue's are kept apart: nothing
here is written over what is there, and nothing there comes in. Whether
the ticket is open, and what is said, go on crossing. Somebody who may
change the link then says which stands:

```sh
mw record set shop-links-3 held=take    # what is on GitHub comes into the card
mw record set shop-links-3 held=keep    # the card's words are put back on the issue
```

**An issue that is linked is filled as it stands**, whoever wrote it: a
card made for an issue takes the issue's title and body at that moment.
Whoever linked it, by a row or by putting the label on it at GitHub, has
looked at it; and only somebody with standing in the repository can put a
label on an issue there.

How GitHub says who has standing: the author's association with the
repository (`OWNER`, `MEMBER`, `COLLABORATOR`), and who last changed an
issue's body and its title. An issue is trusted as it stands where its
author has standing, or where somebody other than its author last changed
both its body and its title, since only somebody with standing can change
another's issue.

### Linking by a label

Applied with a label for either side, tickets are linked without a row
being written by hand:

```sh
mw apply github/issues --project shop --param out_label=public --param in_label=factory
```

- `out_label`: a card given that label has an issue made for it, where
  whoever labelled it holds `extensions:github`. Where they do not, the
  card stays here, and that is said in the project's channel.
- `in_label`: an issue that has that label on GitHub has a card made for
  it. Whether it has is read from the issue itself, as it is, and never
  from what a call says of it.

With neither said, nothing is ever linked but by a row.

### Hearing what changed on GitHub

What changes on the board is heard as it happens. What changes on GitHub
is heard one of two ways, and either way the issue is then read whole as
it is at that moment: nothing a call says of an issue is taken but which
issue it was.

**As it happens**, where GitHub can reach this factory at an address.
GitHub signs what it sends with a secret the two share, and the factory
takes a call only when the signature is one that secret makes:

```sh
mw secret set github-signing < signing-secret.txt
mw caller set github --topic issues.called --secret github-signing --header X-Hub-Signature-256 --prefix sha256=
```

Then, in the repository's settings on GitHub, add a webhook: its payload
URL is `https://<where this factory answers>/in/github`, its content type
`application/json`, its secret the one in `signing-secret.txt`, and of
the events it can send only **Issues** and **Issue comments**. A call
about another repository, or about a pull request, is passed over.
GitHub signs no time, so a call somebody saw could be sent again; since
only the number of the issue is taken from it, sending one again has the
issue read again and nothing more.

**On a timer**, where it cannot:

```sh
mw apply github/issues --project shop --param timer=true
```

It then asks GitHub once a minute which issues changed last, and reads
those whose links have not seen them as they now are. Anybody may have it
ask at once: `mw emit issues.look project=shop`.

Either way it looks once of its own accord when it is applied, and each
time the daemon comes back: what changed on GitHub while nothing here was
listening is seen then, and so is an issue that carried the label that
links before this was told of it.

| Param | |
|---|---|
| `key` | A secret the factory keeps that the program acts as at GitHub. Left out, it acts as whoever is signed in to it on the device. |
| `channel` | Where to say that a ticket was linked, or that something could not be done. Left out, the channel called what the project is. It is made if it is not there: a project needs a board for this, and no fleet. |
| `out_label`, `in_label` | The labels that link, as above. Left out, nothing is linked but by a row. |
| `closed`, `reopened` | The columns of a card whose issue is closed, and of one whose issue is open again. Left out, the board's last and its first. |
| `prefer` | Which side is taken where a ticket changed on both since they last agreed: `here`, unless `there` is said. |
| `takes_in` | Whose changes on GitHub are taken, as above: `trusted`, unless `everyone` or `nobody` is said. |
| `timer` | `true` to ask GitHub once a minute what changed. |

**What each secret needs leave for at GitHub.** The signing secret is no
credential there: it is a word GitHub signs with, and lets nobody do
anything. The `key`, where one is said, is a token of a bot's account or
an application's: a fine-grained token for the one repository with
*Issues* set to read and write, and *Metadata* to read, which GitHub adds
by itself, is enough for all of this. It needs nothing of the
repository's contents. Left out, the program acts as whoever runs the
daemon, with everything their account may do.

It needs `extensions:github`, `board`, `records`, `projects` to declare
its table, `channels:write`, `emit` to say its own events, and
`blueprints` to start what asks on a timer. Told a `key`, whoever applies it also holds `secrets:use` and
`shell`, as for any program handed a secret in its environment.

**Stopped in the middle.** What the program does at GitHub that cannot
be done twice (making an issue, saying something on one) carries a mark
by which it is found again: the card's id under an issue's body, the
number of the event here under a thing said. A daemon that stops as one
of them is being done does not do it again when it is back, and cannot
say whether it was done; so it looks by the mark, goes on from what it
finds, and does it then where it is not there. A card has one issue, and
a thing said is said once, wherever the daemon was stopped.

**What is not held.** A card is added for an issue, and what is said on
an issue is said on its card, by steps that are the same done twice: a
daemon that stops as either is being done does it again when it is back
and finds what it left, so the issue has one card and a thing said is on
the card once. A link found saying `bringing` in its `state` and naming
no card is left for a person, since a card may have been added for it;
the person names the card in the link or empties `state` to have one
made. That it asks on a timer is the runner's own to hold (a handler's
`poll`, played where the runner is tested); what the timer says here,
`issues.look`, is what the examples drive, since an example cannot have
a minute pass.

## The examples

`examples/` holds a stand-in for the program (`stand-in.sh`) and cases,
each a sentence that is true of this package and the steps that show it.
The application's tests play every case against a daemon of its own that
can find the stand-in under the program's name and nothing else, so the
real program is never run and nothing reaches GitHub. What the real
program prints was written into the stand-in from its documentation.

Keeping issues in step has also been tried by hand against GitHub
itself, once (2026-11-24), on one private repository with a factory made
for the trial: issues linked out by a label (`public`) and in by another
(`factory`), cards and issues that were not linked kept on their own
side, a title and a body changed both ways, an issue closed and opened
again both ways, things said both ways with what was said on a card and
not marked public kept here, a label given to an issue later bringing it
in, and an issue made on GitHub coming in by the timer within a minute.
Not tried against GitHub: the signed call (the webhook), pull requests,
the Git page's parts and the Git table, and a repository with many
issues. Nor what GitHub's API answers about an author's standing in a
repository, which the stand-in only stands for: the next trial is to read
one issue by an outsider and one by a member (`mw actions github.issue`
says how it is asked) and see that `trusted` says so of each, before
anything is left to it.

## On a project's Git page

A project whose folder is a git repository with a remote on GitHub shows,
on its **Git** page beside what git says of it, the repository's open pull
requests (who opened each, its branch, what was decided of it and how its
checks stand), its open issues and its last workflow runs, each with the
address it is read at on GitHub, each a tab of the page after git's own
(`#pulls`, `#issues`, `#checks` in its address). Each is one of this
package's actions (`github.pulls`, `github.issues-open`, `github.checks`),
read when its tab is first shown or asked to read again, as whoever runs the
daemon is signed in to `gh`, for somebody who holds `extensions:github`.
Nothing is changed there. `mw project git <project> --parts` says the same
from a terminal.

## The Git table: each piece of work, as GitHub knows it

`github/work` keeps a table of the project's, `git` (called Git), with a
row for each piece of work: a card whose branch reached the repository's
remote. A row says the card, its branch, the commit it is at and how many
commits it has, its pull request (where it is read, how it stands, how its
checks stand and what was decided of it), the issues the pull request
closes, and when the branch and the pull request last changed. The table
sits beside the board, each row about a card, and is written only from
here: nobody adds or changes a row by hand (`by_hand: false`). Pressed, a
card says its row in its Details, and in the table's own tab beside them.

```sh
mw apply github/work --project shop
```

Applied where the program is not signed in it is refused, saying how to
sign it in, and the project is given no table. A row is made when a card's
branch is sent (`card.pushed`), and read again as it is then: where its
branch is, from where the card was worked, and its pull request, from
GitHub (`github.work`, which asks `gh pr list --head <branch> --state=all`).
It is read again when GitHub says something of a pull request, where
GitHub can reach the factory (a caller that says `work.called`, described
as the issues' caller is, with **Pull requests** among the events the
webhook sends), and when it starts or the daemon comes back. Of what a call
says only the branch is taken. Nothing is changed on GitHub.

Not yet: it is applied by hand. A project whose program comes to be signed
in later is not given it by itself, as a project is given what it starts
with later; and the issues are those the pull request says it closes,
without when each last changed.
