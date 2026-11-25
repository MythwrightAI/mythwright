<!-- public -->

# Example: A/B testing a change to a service

Say you run an inventory service for a shop, and you have a change to how
restocking works. You want to try it on part of the traffic, see how it
does against what runs now, and then roll it out or roll it back. This is
how Mythwright carries that from the change itself to the decision. Each
part is a blueprint you can read and change.

## 1. Build the change, or two versions of it

The change is a card on the project's board, worked by an agent on a
branch of its own and reviewed before it is merged.

If you are not sure which way to make it, have two agents each build a
version of the same card on their own branches, and keep the one you
prefer: an A/B test of the work itself. [ab-testing.md](ab-testing.md)
says how.

## 2. Deploy it under environments

A project's environments (for example `dev`, `staging` and `prod`) each
hold their own variables and secrets. A pipeline can deploy a branch when
it moves, and an environment can say who has to approve before anything
runs there, so nothing reaches `prod` without a person's go-ahead.
[environments.md](environments.md) has the details.

## 3. Split the traffic

Two ways, depending on what you are changing.

**A feature flag with a percentage.** The project keeps the flag for each
environment, and the service is handed it as a variable
(`FLAG_RESTOCK_V2`), for example `10` in `prod`. Your service decides
which customers fall in that ten percent. Turning a flag on or wider in an
environment whose rule names who approves waits for one of them.

```sh
mw flag set shop restock-v2 prod 10
```

**Two versions of the service running side by side.** The factory can
host a service behind a stable address and roll a new version out to a
share of the requests (10%, then 50%, then all of them), watching each
step and rolling back if it misbehaves. The first-run guide's canary
rollout of a small key-value service shows it working.

## 4. Measure

Your service says what it measured by printing lines like this:

```
::measure {"name": "restock.stockouts", "value": 3, "labels": {"store": "north"}}
```

The factory keeps them with which version, environment and step they
came from, and shows them on the **Measurements** page. A percentage flag
can also be started as a comparison, so that each order (or whatever your
service handles) is counted under the side of the flag it fell on:

```sh
mw flag ways shop restock-v2 --environment prod --blueprint restock --per order --param restock
mw compare show restock-v2
```

## 5. Decide

The numbers for each version sit side by side. Mythwright counts and
never picks a winner for you: you decide, or you write the rule that
decides, such as widening the rollout while errors stay under a limit
and rolling back the moment they do not.

## What it does not do

- It does not put users in buckets. Your service does that, from the
  flag's value.
- It does not do statistics: no significance tests or confidence
  intervals. It counts.
- It is not a product-analytics tool. To see whether conversion went up,
  send your events to your analytics as well.
