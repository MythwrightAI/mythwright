#!/bin/sh
# A stand-in for the program, for this package's examples: it answers the
# way the program does, changes nothing outside its own folder, and writes
# down how it was called, one call a line. The examples put it first on the
# path of a daemon of their own, under the program's name, so the real one
# is never run and nothing here reaches any service.
#
# The pull requests it has are files in `pulls/`, one a branch (with `-`
# where the branch has `/`), each holding its number. Files beside it make
# it another program: `signed-out`, one that nobody has signed in to;
# `refuses`, one that will not open a pull request, and says that text;
# `garbled`, one that answers a question about pull requests with that text.
#
# Handed a key (`GH_TOKEN`), it writes the key down as `acted/as` each time
# it is called, and leaves no such folder where it is handed none: who it
# would have acted as. `bad-key` makes the key one GitHub will not take.
#
# The issues it has are folders in `issues/`, one a number, each holding
# its `title`, its `body` as it was written there (the mark of the card it
# was made for included), its `state` (`OPEN` or `CLOSED`), its `labels`
# (one a line) and its `stamp`, which says when it last changed. Who wrote
# it is its `author`, with their standing in the repository as its
# `association` (`ada`, who owns the repository, where a case says
# neither), and who last changed its body and its title, where somebody
# did, are its `editor` and its `renamer`. What was said on one is a file
# of `said/<number>/`, one a thing said, in order: who said it on its
# first line, with their standing after a space where they have any
# (`bo MEMBER`), and what they said after. A case writes these to say
# what is there, and what somebody did there meanwhile.
# `refuses-issues` makes it a program that will not make an issue, and
# says that text. An issue is made whole or not at all, as the service
# makes one: a program ended as it makes one leaves none.
#
# Where a pipe called `held` is beside it, it says down the pipe what it
# has just done, once it has made an issue or said something on one, and
# then waits there for ever: a case stops the daemon while the program is
# so held, having done the thing and not yet answered. With `hold-making`
# beside it as well, it is held as it makes an issue, before the issue is
# made (`issue making`): the daemon stopped there ends the program having
# made nothing.
#
# What it says of one branch's pull request for a project's Git table,
# whatever the pull request's state, is what a case wrote in
# `work/<branch>` (with `-` where the branch has `/`), as `gh` answers once
# its `--jq` has shaped it, and that there is none where there is no such
# file.
#
# What it lists for a project's Git page (its open pull requests, its open
# issues, its workflow runs) is what a case wrote in `listed/pulls`,
# `listed/issues` and `listed/runs`, as `gh` answers once its `--jq` has
# shaped it.
#
# The daemon the examples run can find this under the program's name and
# nothing else, not even a shell's tools. What this needs of them it looks
# for itself, where a system keeps them.
PATH=/usr/bin:/bin
here=$(dirname "$0")
# One call a line, whatever is in it: the end of a line within an argument
# is written as `\n`. The line is added in one write: the daemon may call
# twice at once, and a call's words and its line's end written apart came
# out as one line of both calls and an empty one.
call=$(printf '%s' "$*" | tr '\n' '\036' | sed 's/\x1e/\\n/g')
printf '%s\n' "$call" >> "$here/calls"
where=https://github.example/acme/shop/pull
rm -rf "$here/acted"
if [ -n "${GH_TOKEN:-}" ]; then
  mkdir -p "$here/acted"
  printf '%s' "$GH_TOKEN" > "$here/acted/as"
fi

there=https://github.example/acme/shop/issues

# A text as it is written inside JSON.
json() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/\t/\\t/g' | awk 'BEGIN { ORS = "" } NR > 1 { print "\\n" } { print }'
}

# What a file of an issue holds, or nothing.
of() {
  [ -f "$here/issues/$1/$2" ] && cat "$here/issues/$1/$2"
}

# Says what was just done down the pipe a case holds it by, where there
# is one, and waits there until it is ended with the daemon.
held() {
  [ -p "$here/held" ] || return 0
  echo "$1" > "$here/held"
  read -r _ < "$here/held"
}

# The stamp of the next change: one more than any said, whether or not
# the count of them was written down before the program was ended.
next_change() {
  count=$(cat "$here/changes" 2>/dev/null || echo 0)
  for stamp in "$here"/issues/*/stamp; do
    [ -f "$stamp" ] || continue
    said=$(sed 's/^t//' "$stamp")
    # One a case wrote is not one of these.
    case $said in '' | *[!0-9]*) continue ;; esac
    [ "$said" -gt "$count" ] && count=$said
  done
  echo $((count + 1))
}

# Says that an issue changed: what was last said to have, and one more.
changed() {
  count=$(next_change)
  printf '%s' "$count" > "$here/changes"
  printf 't%s' "$count" > "$here/issues/$1/stamp"
}

# An issue's labels, as a list in JSON.
labels() {
  list=""
  if [ -f "$here/issues/$1/labels" ]; then
    while IFS= read -r label || [ -n "$label" ]; do
      [ -n "$label" ] && list="$list${list:+,}\"$(json "$label")\""
    done < "$here/issues/$1/labels"
  fi
  printf '[%s]' "$list"
}

# Whether there is such an issue; said as the program says it where not.
is_there() {
  [ -d "$here/issues/$1" ] && return 0
  printf '%s\n' "GraphQL: Could not resolve to an issue or pull request with the number of $1. (repository.issue)" >&2
  exit 1
}

# What follows `=` in the first argument that begins with a word.
after() {
  word=$1
  shift
  for arg in "$@"; do
    case "$arg" in
      "$word="*) printf '%s' "${arg#"$word"=}"; return 0 ;;
    esac
  done
  return 1
}

# What a list a project's Git page asks for comes to, as the program
# answers it once its `--jq` has shaped it: a file of `listed/`, written by
# a case, and none where there is none.
listed() {
  if [ -f "$here/listed/$1" ]; then cat "$here/listed/$1"; else echo "[]"; fi
}

bad_key() {
  printf '%s\n' "HTTP 401: Bad credentials (https://api.github.example/graphql)" "Try authenticating with:  gh auth login" >&2
  exit 1
}

signed_out() {
  printf '%s\n' "You are not logged into any GitHub hosts. To log in, run: gh auth login" >&2
  exit 1
}

case "$1 $2" in
  "auth status")
    if [ -f "$here/bad-key" ]; then
      printf '%s\n' "github.example" "  X Failed to log in to github.example using token (GH_TOKEN)" "  - The token in GH_TOKEN is invalid." >&2
      exit 1
    fi
    [ -f "$here/signed-out" ] && signed_out
    printf '%s\n' "github.example" "  ✓ Logged in to github.example account ada (keyring)"
    ;;
  "pr list")
    [ -f "$here/bad-key" ] && bad_key
    [ -f "$here/signed-out" ] && signed_out
    if [ -f "$here/garbled" ]; then cat "$here/garbled"; exit 0; fi
    # The open pull requests as a project's Git page asks for them.
    if [ "$3" = "--state=open" ]; then listed pulls; exit 0; fi
    branch=$(printf '%s' "$3" | sed 's/^--head=//; s/\//-/g')
    # A branch's pull request as a project's Git table reads it, whatever
    # its state, as the program answers once its `--jq` has shaped it.
    if [ "$4" = "--state=all" ]; then
      if [ -f "$here/work/$branch" ]; then cat "$here/work/$branch"; else echo '{"found": false}'; fi
      exit 0
    fi
    if [ -f "$here/pulls/$branch" ]; then
      number=$(cat "$here/pulls/$branch")
      printf '[{"number":%s,"url":"%s/%s"}]\n' "$number" "$where" "$number"
    else
      echo "[]"
    fi
    ;;
  "pr create")
    [ -f "$here/signed-out" ] && signed_out
    if [ -f "$here/refuses" ]; then cat "$here/refuses" >&2; exit 1; fi
    branch=$(printf '%s' "$3" | sed 's/^--head=//; s/\//-/g')
    mkdir -p "$here/pulls"
    number=$(($(ls "$here/pulls" | wc -l) + 1))
    printf '%s' "$number" > "$here/pulls/$branch"
    # What the program says besides the address goes to its other stream.
    printf '\nCreating pull request for %s into main in acme/shop\n\n' "$3" >&2
    printf '%s/%s\n' "$where" "$number"
    ;;
  "repo view")
    [ -f "$here/bad-key" ] && bad_key
    [ -f "$here/signed-out" ] && signed_out
    echo "acme/shop"
    ;;
  "issue view")
    [ -f "$here/bad-key" ] && bad_key
    [ -f "$here/signed-out" ] && signed_out
    number=$3
    is_there "$number"
    if [ "$4" = "--json=comments" ]; then
      # Which thing said there carries a mark: its name, or nothing.
      mark=$(after --jq "$@" | sed 's/.*<!-- said \([^ ]*\) -->.*/\1/')
      for said in $(ls "$here/said/$number" 2>/dev/null | sort -n); do
        if grep -q -- "<!-- said $mark -->" "$here/said/$number/$said"; then
          printf '{"id":"c%s-%s"}\n' "$number" "$said"
          exit 0
        fi
      done
      echo "{}"
      exit 0
    fi
    echo "the stand-in reads an issue whole only as the API gives it" >&2
    exit 2
    ;;
  "api graphql")
    [ -f "$here/bad-key" ] && bad_key
    [ -f "$here/signed-out" ] && signed_out
    number=$(after number "$@")
    is_there "$number"
    # Its body without the mark of the card it was made for, and with
    # the ends of its lines as they are written here.
    body=$(of "$number" body | tr -d '\r' | sed '/^<!-- card [^>]* -->$/d')
    closed=false
    [ "$(of "$number" state)" = "CLOSED" ] && closed=true
    # Trusted as it stands: its author has standing in the repository,
    # or somebody else last changed both its body and its title.
    author=$(of "$number" author)
    author=${author:-ada}
    association=$(of "$number" association)
    [ -f "$here/issues/$number/author" ] || association=${association:-OWNER}
    editor=$(of "$number" editor)
    renamer=$(of "$number" renamer)
    trusted=false
    case "$association" in OWNER | MEMBER | COLLABORATOR) trusted=true ;; esac
    [ "${editor:-$author}" != "$author" ] && [ "${renamer:-$author}" != "$author" ] && trusted=true
    list=""
    for said in $(ls "$here/said/$number" 2>/dev/null | sort -n); do
      first=$(head -n 1 "$here/said/$number/$said")
      by=${first%% *}
      known=false
      case "$first" in *" OWNER" | *" MEMBER" | *" COLLABORATOR") known=true ;; esac
      who=$by
      [ "$known" = false ] && who="$by, who has no standing in the repository,"
      words=$(tail -n +2 "$here/said/$number/$said" | tr -d '\r')
      ours=false
      case "$words" in *"<!-- said "*) ours=true ;; esac
      list="$list${list:+,}{\"id\":\"c$number-$said\",\"by\":\"$(json "$by")\",\"who\":\"$(json "$who")\",\"trusted\":$known,\"body\":\"$(json "$words")\",\"url\":\"$there/$number#issuecomment-$said\",\"ours\":$ours}"
    done
    printf '{"number":%s,"url":"%s/%s","title":"%s","body":"%s","closed":%s,"labels":%s,"stamp":"%s","author":"%s","trusted":%s,"said":[%s]}\n' \
      "$number" "$there" "$number" "$(json "$(of "$number" title)")" "$(json "$body")" "$closed" "$(labels "$number")" "$(of "$number" stamp)" "$(json "$author")" "$trusted" "$list"
    ;;
  "issue list")
    [ -f "$here/bad-key" ] && bad_key
    [ -f "$here/signed-out" ] && signed_out
    if [ -f "$here/garbled" ]; then cat "$here/garbled"; exit 0; fi
    if [ "$3" = "--state=open" ]; then listed issues; exit 0; fi
    # The issue that carries the mark of a card, or nothing.
    card=$(after --jq "$@" | sed 's/.*<!-- card \([^ ]*\) -->.*/\1/')
    for number in $(ls "$here/issues" 2>/dev/null | sort -n); do
      if grep -q -- "<!-- card $card -->" "$here/issues/$number/body" 2>/dev/null; then
        printf '{"number":%s,"url":"%s/%s"}\n' "$number" "$there" "$number"
        exit 0
      fi
    done
    echo '{"number":null,"url":null}'
    ;;
  "issue create")
    [ -f "$here/signed-out" ] && signed_out
    if [ -f "$here/refuses-issues" ]; then cat "$here/refuses-issues" >&2; exit 1; fi
    mkdir -p "$here/issues"
    number=$(($(ls "$here/issues" | wc -l) + 1))
    # Made whole or not at all, as the service makes one: written aside and
    # put in its place in one move. The daemon going as it asked ends the
    # program, and one ended half way through used to leave an issue with
    # no stamp, which the daemon then found by its mark.
    count=$(next_change)
    making="$here/making"
    rm -rf "$making"
    mkdir -p "$making"
    after --title "$@" > "$making/title"
    after --body "$@" > "$making/body"
    printf 'OPEN' > "$making/state"
    printf 't%s' "$count" > "$making/stamp"
    [ -f "$here/hold-making" ] && held "issue making"
    mv "$making" "$here/issues/$number"
    printf '%s' "$count" > "$here/changes"
    held "issue create"
    printf '\nCreating issue in acme/shop\n\n' >&2
    printf '%s/%s\n' "$there" "$number"
    ;;
  "issue edit")
    [ -f "$here/signed-out" ] && signed_out
    number=$3
    is_there "$number"
    # What is not sent is left as it is; and whoever the program acts as
    # has changed what is, which says so of an issue that is somebody
    # else's.
    if title=$(after --title "$@"); then
      printf '%s' "$title" > "$here/issues/$number/title"
      [ -f "$here/issues/$number/author" ] && printf 'ada' > "$here/issues/$number/renamer"
    fi
    if body=$(after --body "$@"); then
      printf '%s' "$body" > "$here/issues/$number/body"
      [ -f "$here/issues/$number/author" ] && printf 'ada' > "$here/issues/$number/editor"
    fi
    changed "$number"
    printf '%s/%s\n' "$there" "$number"
    ;;
  "issue close" | "issue reopen")
    [ -f "$here/signed-out" ] && signed_out
    number=$3
    is_there "$number"
    if [ "$2" = "close" ]; then printf 'CLOSED' > "$here/issues/$number/state"; else printf 'OPEN' > "$here/issues/$number/state"; fi
    changed "$number"
    ;;
  "issue comment")
    [ -f "$here/signed-out" ] && signed_out
    number=$3
    is_there "$number"
    mkdir -p "$here/said/$number"
    said=$(($(ls "$here/said/$number" | wc -l) + 1))
    { echo "ada OWNER"; after --body "$@"; } > "$here/said/$number/$said"
    changed "$number"
    held "issue comment"
    printf '%s/%s#issuecomment-%s\n' "$there" "$number" "$said"
    ;;
  "api repos/{owner}/{repo}/issues?state=all&sort=updated&direction=desc&per_page=30")
    [ -f "$here/bad-key" ] && bad_key
    [ -f "$here/signed-out" ] && signed_out
    list=""
    for number in $(ls "$here/issues" 2>/dev/null | sort -n); do
      list="$list${list:+,}{\"number\":$number,\"stamp\":\"$(of "$number" stamp)\",\"labels\":$(labels "$number")}"
    done
    printf '[%s]\n' "$list"
    ;;
  "run list")
    [ -f "$here/bad-key" ] && bad_key
    [ -f "$here/signed-out" ] && signed_out
    if [ -f "$here/garbled" ]; then cat "$here/garbled"; exit 0; fi
    listed runs
    ;;
  *)
    echo "the stand-in does not know how to: $*" >&2
    exit 2
    ;;
esac
