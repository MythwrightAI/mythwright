<!-- public -->

# Example: choosing a model, or a prompt, on your real work

Say your team's agents work the cards of a project called `shop`, and you
wonder whether another model would do better, or whether the worker would
do better told something else. A benchmark answers that for somebody
else's work. This answers it for yours: two setups side by side, each
given a share of the cards that come to be worked, everything each spends
and comes to counted under its name, and the numbers laid out together for
you to read. Nothing is written in any blueprint for it, and nothing is
scored: Mythwright counts, and which setup did better is yours to say.

## 1. Two setups, as two rows

Who works the cards is a row of the project's `agents` table: a role, how
many there are of it, what runs them and on which model, and what they are
told. A setup is a row of its own. Say the two models are called otter and
slate:

```sh
mw record list shop agents
mw record add shop agents title=otter role=otter model=otter permission=edit told="You work the cards of this project's board, one at a time. Do what the card asks and no more, and say plainly what you did and what is left."
mw record add shop agents title=slate role=slate model=slate permission=edit told="You work the cards of this project's board, one at a time. Do what the card asks and no more, and say plainly what you did and what is left."
```

Each row brings up an agent of its role (`shop-otter-1`, `shop-slate-1`),
run by the device's usual agent program unless the row says `adapter`.

To try two versions of the instructions instead, keep the model the same
and change what each row is told: one row says what the worker is told
now, the other the words you want to try. A row may also name a longer
file of the project's own words (`prompt`), read before `told`.

## 2. Have reviewers look

What is counted is most useful where every card is looked at the same way.
Give the reviewer row a count, and each card that is worked is reviewed by
an agent of that role before it is merged:

```sh
mw record set shop-agents-3 count=1
```

(`mw record list shop agents` says which row is the reviewers'. With a
person reviewing instead, the counts below still work: a card sent back is
counted as a move back from review.)

## 3. Start the comparison

```sh
mw compare start models --project shop --of std/work --per item --also std/review \
    --way otter --with otter.role=otter \
    --way slate --with slate.role=slate
```

From now on, each card that comes to be worked goes one way or the other,
and keeps to it however often it is worked again: `std/work` is what gives
a card to an agent, `--per item` says that a thing is a card, and `--with`
says which role each way gives it to. `--also std/review` has each card's
review counted under the same way. Both ways get an equal share unless you
say otherwise.

To try the new setup on a little of the work first, give it a small share:

```sh
mw compare start models --project shop --of std/work --per item --also std/review \
    --way otter --with otter.role=otter --share otter=9 \
    --way slate --with slate.role=slate --share slate=1
```

Then work goes on as it always does: cards are made ready, taken, worked,
reviewed, sent back and merged. Nothing about the cards says which way
they went, except the agent that worked them.

## 4. What is counted

For each card, under the way it went:

- **What came of it**: each move (worked to review, sent back to doing,
  passed to done), what each review said (passed, failed, undecided), a
  question asked in place of working, work that could not be kept or
  merged, and a card left for a person. Each with how long after the card
  was given its way.
- **What was spent**: every turn an agent took on it, with what its program
  said it took (tokens read and written, and what that cost) and how long
  it was at work.
- **What was accepted**: a card whose review passed, or that reached the
  board's last column. How many of each way's cards were, and what the way
  spent for each one that was, the work that was not accepted counted in.
- **What came after**: work that a later card reverted, or changed again
  within two weeks, or said it fixes.

## 5. Read the numbers

```sh
mw compare show models
```

One screen, a column for each way: how many cards each was given, and
under that what came of them, what each agent chose to end its turns with,
what was added up in all and for each card, and how many were accepted and
what each accepted card cost. In the client the same is under
**Measurements**, in the comparison's panel.

One card at a time, its cost by round:

```sh
mw board cost shop-14
```

A round is each time the card was worked and looked over: the turns taken
on it in that while, what each took, and what was decided at the round's
end, passed, sent back with what was said, or left for a person. The card's
drawer in the client shows the same.

Some things to look at:

- **What each accepted card cost.** A cheaper model that is sent back twice
  as often may cost more for each card that lands.
- **How often review sent a card back.** The moves from review back to
  doing, and the reviews that failed.
- **Rounds.** How many times a card went round before it passed.
- **What came after.** Work that was reverted, or soon changed again.

Wait for enough cards before you read much into it: a dozen each says
little, and the counts are counts, with no test of how sure they are.

## 6. Keep one, or let a rule widen it

When you have decided, make the setup you keep the worker's, and end the
comparison:

```sh
mw record set shop-agents-2 model=slate
mw compare end models
```

Or end it with everything going one way from then on, unmeasured, until
you end it again:

```sh
mw compare end models --goes slate
```

Or let a rule you write widen the new setup's share as it holds up, the way
a service is rolled out to more of its traffic:

```sh
mw compare rule models --grows slate --against otter --holds verdict=failed --over 20 --step 10 --most 50
```

Once each way has had 20 cards whose review said something since the shares
were last set, slate is given 10 more of every hundred cards, up to half,
where its reviews failed no more often than otter's. Where they failed more
often, the shares stay where they are (`--least 10` takes it back a step
instead). `mw compare show models` says each time the shares were said
again, and why. Shares can always be said by hand:

```sh
mw compare share models otter=1 slate=1 --why "slate held up on 40 cards"
```

The rule is yours: the daemon reads the counts and goes by it, and decides
nothing else.

## What it does not do

- It does not pick a winner, or say that one is better. It counts.
- It does no statistics: no significance test, no confidence interval.
- It compares on the cards that happen to come, which are not two of the
  same card. Where you want the same card done both ways and judged,
  that is a card's A/B test, which the board's page offers (A/B testing,
  in its gear).
- A turn whose program said nothing of what it took is said to be so on
  the card's cost, and is not counted as one that took nothing.
