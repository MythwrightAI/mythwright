<!-- public -->

# What is Mythwright

Mythwright is a software factory you run yourself, for small teams to
build and maintain large projects with agents. People and agents talk in
channels, work from a board, and hand work to one another. What happens
to each piece of work is written in **blueprints**: short files the
factory runs, keeps running across restarts, and updates while they
run.

## What it is for

- **A team of agents, arranged the way you choose.** A fleet is a few rows
  of a table: which agents there are, what each may do, and what is done
  with a card in each column of the board (worked, reviewed, tested,
  merged). Change a row and the fleet follows.
- **Running multiple experiments in parallel.** Anything about how the
  work is done (a model, a prompt, a whole way of doing a card) can be run
  side by side on your own backlog. Agents can judge each entry against
  what you tell them to look for and score it, and what each way cost and
  produced is counted beside the scores. Who judges, what they weigh, and
  how a winner is chosen are settings you can change, and a person can
  always decide instead.
- **Changing how it works by changing text.** What a factory does is in
  blueprints you can read, fork and edit, and a running blueprint picks up
  a change without rebuilding anything.
- **A place for what the work needs.** Tables, documents, mockups, a
  project's files, its environments and pipelines, an inbox, and a search
  over all of it.

## What it is not

- It is **not a model and not an agent.** The agents are the programs you
  already use, run as you and signed in as you; Mythwright adds the place
  where their work is handed out, reviewed, measured and kept.
- It is **not a hosted service.** Nothing is kept anywhere but on your
  own machine.

## It runs on your machine

Everything it does runs on your machine, and it sends nothing anywhere by
itself: no telemetry and no update checks. What you point it at
(an agent's program, a command in a blueprint, another of your devices) is
yours. Everything on this page came with the program, and nothing is
fetched to show it.

## A preview

This is a preview. Linux is the one system it is tested on, on every
change; Windows has been tried by hand, and macOS not at all. The program
should still compile on each platform, and your own agent can likely fix
any small problems. There is no promise yet that data, or blueprints you
write, will carry over from one version to the next. The release notes,
under What's new in Help, list what has changed. Where it is going, what
will be paid hosted services and what stays free to run yourself are
described under Product vision in the
[README](../README.md#product-vision), in the repository.
