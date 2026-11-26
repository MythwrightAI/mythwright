# Jira

Keep the cards of a project that are **linked** with an issue of a
[Jira](https://www.atlassian.com/software/jira) project in step with it,
a ticket at a time. It is the GitHub package's `github/issues` done for
Jira, and works the same way: read *Issues: tickets kept in step* in
`extensions/github/README.md` beside this one for the whole of it, and
this page for where Jira differs. How such a package is written is in
*Keeping a tracker in step* in `docs/extensions.md`.

```sh
printf '%s' 'bot@example.com:ATATT...' | mw secret set jira-key   # an account's address and an API token
mw extension add jira
mw apply jira/issues --project shop --param site=https://acme.atlassian.net --param jira_project=SHOP --param key=jira-key
mw record add shop links card=shop-12        # an issue is made for the card
mw record add shop links issue=SHOP-48       # a card is made for the issue
mw record add shop links card=shop-7 issue=SHOP-31   # two that were there already
```

Of a linked ticket:

- **its title and body**: the card's title is the issue's summary, and
  its body the issue's description, both ways, as `takes_in` says below;
- **whether it is done**: a card moved to the board's last column has its
  issue moved on, by the first way its workflow offers from where it is,
  to a status Jira counts as done; one moved out of it is moved back to a
  status Jira counts as not begun. An issue moved to done in Jira has its
  card moved to the last column, and one moved out of done, to whatever
  status, has it moved to the first. Moves between statuses that are not
  done, and between columns that are not the last, stay where they are
  made. `closed` and `reopened` say other columns;
- **what is said on the issue** is said on the card, once, with who said
  it and where it is, and never marked public;
- **what is said on the card stays here** unless whoever says it marks it
  public and holds `extensions:jira`; then it is said on the issue, once.

## The program, and why

Its actions run **curl** (8.3 or later), asking the second version of
Jira's web API. Atlassian's own command line (`acli`) and the command
line most people use with Jira (`jira`) were the first choice, and both
hand an issue's description and what was said on it back in Atlassian's
document format: a tree of JSON that a package's expressions cannot read
back into the words a card holds. The second version of the API takes
and gives both as text, Jira's wiki markup, which is what a card's body
and a comment are written in here too as far as either side is concerned.
curl is what asks it the same way on any device.

The key is handed to curl in the environment (`JIRA_KEY`, from the secret
`key` names), and curl writes it into the request's header itself
(`--variable`, `--expand-header`, which came with 8.3): it is in no
argument, and what curl prints is written down with the secret's name in
its place.

## What it needs

- curl 8.3 or later on the daemon's device.
- An account at the Jira site for it to act as, best one of its own (a
  bot's), with leave in the one project to browse, make, change,
  transition and comment on its issues; and an
  [API token](https://id.atlassian.com/manage-profile/security/api-tokens)
  of that account's. The secret is the account's address and the token
  with a colon between (`bot@example.com:ATATT...`), which is what Jira
  takes as a password.
- The project's key (`SHOP`) and the site's address
  (`https://acme.atlassian.net`).

An issue made for a card is of the kind `issue_type` says (`Task` unless
said), and carries a label that marks it, `card-<card>`, by which it is
found again where the daemon stopped as it was made. Leave that label on
the issue.

## Who can write what an agent works from

A Jira site lets in only whom its administrator lets in, and only
somebody given leave to edit a project's issues can change one; a
customer of a service desk can report an issue and say something on one,
and is nobody else the site knows. So `takes_in` goes by that:

| `takes_in` | A change of a linked issue's summary or description | What is said on the issue |
|---|---|---|
| `trusted`, unless said | Comes into the card, unless a customer of a service desk reported the issue: then it is **held**, as GitHub's is for somebody with no standing. | Is said on the card, a customer's said to be a customer's. Answers an agent only where whoever said it is not a customer. |
| `everyone` | Comes into the card. | Is said on the card, and may answer an agent. |
| `nobody` | Is not taken and not written over. | Is said on the card, and answers nothing. |

A change that is held waits in the link (`held: waiting`, `held_by` who
reported the issue), and the card is told once what to do:
`mw record set shop-links-3 held=take` or `held=keep`.

Who is a customer is what Jira says of each account (`accountType`,
`customer`). That is read from what the API answers and has not been
seen against a real service desk yet.

## Hearing what changed in Jira

**On a timer**, where Jira cannot reach this factory:

```sh
mw apply jira/issues --project shop --param site=... --param jira_project=SHOP --param key=jira-key --param timer=true
```

It asks once a minute which issues of the project changed last, and
reads those whose links have not seen them as they now are.
`mw emit jira.look project=shop` has it ask at once. Either way it looks
of its own accord when it is applied and each time the daemon comes back.

**As it happens**, where Jira can reach the factory at an address: a
webhook of the site's (Jira settings, *System*, *WebHooks*) for the
events *Issue updated* and *Comment created*, with the JQL `project =
SHOP`, sent to `https://<where this factory answers>/in/jira`, and a
caller that takes what it sends only where it is signed with a secret the
two share:

```sh
mw secret set jira-signing < signing-secret.txt
mw caller set jira --topic jira.called --secret jira-signing --header X-Hub-Signature --prefix sha256=
```

Atlassian says its webhooks with a secret are signed so, an HMAC-SHA256
of the body in `X-Hub-Signature` after `sha256=`; that has not been seen
against a real site. A webhook with no secret is no caller this factory
takes. Of what a call says only the issue's key and its project are
taken, and the issue is read again as it is.

| Param | |
|---|---|
| `site` | The address of the Jira site, `https://acme.atlassian.net`. Needed. |
| `jira_project` | The key of the Jira project, `SHOP`. Needed. |
| `key` | The secret the program acts as: `address:token`. Needed. |
| `issue_type` | The kind of issue one made for a card is: `Task` unless said. |
| `channel` | Where it says what it did or could not do. The channel called what the project is, unless said. |
| `out_label`, `in_label` | A label that links a card, and one that links an issue in Jira. Left out, only a row of `links` links. |
| `closed`, `reopened` | The columns of a card whose issue is done, and of one whose issue is not begun. The board's last and first, unless said. |
| `prefer` | Which side is taken where both changed: `here`, unless `there` is said. |
| `takes_in` | As above: `trusted`, unless `everyone` or `nobody` is said. |
| `timer` | `true` to ask Jira once a minute what changed. |

It needs `extensions:jira`, `board`, `records`, `projects`,
`channels:write`, `blueprints`, `emit` and `secrets`.

## What is not held, or not built

- **A daemon stopped as an issue is being made** is held: the issue is
  looked for by its label when it is back, and made only where it is not
  there. But Jira's search may not list an issue made a moment before, so
  a daemon that stopped in the instant after Jira made one, and came back
  within that moment, may make a second. A card added for an issue is
  added by a step that is the same done twice, as GitHub's is; a link
  left saying `bringing` in its `state` by a run cut short another way is
  left for a person.
- **Features are not built**: a feature here (`std/feature`) is not an
  epic there. A feature's plan comes to cards, and each of those is
  linked as any card is; an epic's issues have no card made by being in
  the epic.
- **Labels, who an issue is given to, its priority and its sprint** do
  not cross. A card's labels say nothing in Jira but the one that links.
- **A description's formatting** crosses as the wiki markup Jira's second
  API writes it in, and what is written on a card is put into Jira as it
  is.
- **The package has not been run against a real Jira.** What curl prints
  was written into the stand-in from Atlassian's documentation of the
  API. The first trial is to apply it to a project of a site with an
  account of its own, link one card and one issue, and see that each
  comes across whole, before anything is left to it.

## The examples

`examples/` holds a stand-in for curl asking Jira (`stand-in.sh`) and
cases, each a sentence that is true of this package and the steps that
show it, played by the application's tests against a daemon that can find
the stand-in under the program's name and nothing else.
