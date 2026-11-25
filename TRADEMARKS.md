# The name

The code here is offered under MIT or Apache 2.0, as whoever uses it
chooses. The name is not part of that offer.

"Mythwright", and the icon drawn in `desktop/icon/`, are Adam Grgic's.
Neither licence gives anybody the use of them: Apache 2.0 says so in its
sixth section, and MIT says nothing of names at all.

## What anybody may do

- Run it, change it, hand it on and sell what they build with it, as the
  two licences say.
- Say truthfully what something is: "built from Mythwright", "works with
  Mythwright", "a package for Mythwright".
- Hand on the program unchanged, under its name, for a system or a
  distribution.
- Write about it, teach it and review it.

## What takes asking first

- Calling a changed version by this name, or by one easily taken for it.
- Running a service for other people under this name or one easily taken
  for it: a hosted factory, a relay, a directory, a place to sign in.
- Using the icon for either.
- A name, an address or a page that suggests whatever it is comes from this
  project or is vouched for by it.

Ask by opening an issue in this repository.

## Why

Somebody who signs in to something called this should know who answers for
it. A service somebody else runs that goes down, loses what it kept or is
broken into is theirs to answer for, and should not be taken for this
project's.

## A copy that is to be handed on or hosted is called something else

The name is written in one file, `brand.toml`, and nowhere in the code, so
that this is one command:

```sh
cargo xtask rename --display "Another Name"
```

It rewrites that file and the notes, and leaves this one as it is: what a
renamed copy says here is still true of the name it no longer has. The icon
is one drawing, `desktop/icon/icon.svg`: put another there and run
`node desktop/icon/make.mjs`. And where the product says to ask for support
is `support` in `brand.toml`, which is empty here: a copy that is run for
other people says its own.
