#!/bin/sh
# A stand-in for our tracker's command, for this example's cases: it
# answers as the command the manifest describes does, changes nothing
# outside its own folder, and writes down how it was called, one call a
# line. The cases put it on the path of a daemon of their own, under the
# command's name, and nothing else.
#
# The tickets it has are folders in `tickets/`, one an id (`T-1`), each
# holding its `title`, its `body`, its `state` (`closed`, or open where it
# says nothing), the `mark` of the card it was made for, and its `stamp`,
# which says when it last changed. A case writes these to say what is
# there, and what somebody did there meanwhile.
PATH=/usr/bin:/bin
here=$(dirname "$0")
printf '%s' "$*" | tr '\n' '\036' | sed 's/\x1e/\\n/g' >> "$here/calls"
echo >> "$here/calls"
there=https://tracker.example/tickets

json() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/\t/\\t/g' | awk 'BEGIN { ORS = "" } NR > 1 { print "\\n" } { print }'
}

of() {
  [ -f "$here/tickets/$1/$2" ] && cat "$here/tickets/$1/$2"
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

changed() {
  count=$(($(cat "$here/changes" 2>/dev/null || echo 0) + 1))
  printf '%s' "$count" > "$here/changes"
  printf 't%s' "$count" > "$here/tickets/$1/stamp"
}

ticket() {
  closed=false
  [ "$(of "$1" state)" = closed ] && closed=true
  printf '{"id":"%s","url":"%s/%s","title":"%s","body":"%s","closed":%s,"stamp":"%s"}\n' \
    "$1" "$there" "$1" "$(json "$(of "$1" title)")" "$(json "$(of "$1" body)")" "$closed" "$(of "$1" stamp)"
}

is_there() {
  [ -d "$here/tickets/$1" ] && return 0
  echo "there is no ticket $1" >&2
  exit 1
}

case "$1" in
  whoami)
    echo "ada"
    ;;
  changed)
    list=""
    for id in $(ls "$here/tickets" 2>/dev/null | sort -t- -k2 -n); do
      list="$list${list:+,}{\"id\":\"$id\",\"stamp\":\"$(of "$id" stamp)\"}"
    done
    printf '[%s]\n' "$list"
    ;;
  find)
    mark=$(after --mark "$@")
    for id in $(ls "$here/tickets" 2>/dev/null | sort -t- -k2 -n); do
      if [ "$(of "$id" mark)" = "$mark" ]; then
        printf '{"id":"%s","url":"%s/%s"}\n' "$id" "$there" "$id"
        exit 0
      fi
    done
    echo '{}'
    ;;
  show)
    is_there "$2"
    ticket "$2"
    ;;
  new)
    mkdir -p "$here/tickets"
    id=T-$(($(ls "$here/tickets" | wc -l) + 1))
    making="$here/making"
    rm -rf "$making"
    mkdir -p "$making"
    after --title "$@" > "$making/title"
    after --body "$@" > "$making/body"
    after --mark "$@" > "$making/mark"
    mv "$making" "$here/tickets/$id"
    changed "$id"
    printf '{"id":"%s","url":"%s/%s"}\n' "$id" "$there" "$id"
    ;;
  edit)
    is_there "$2"
    if title=$(after --title "$@"); then printf '%s' "$title" > "$here/tickets/$2/title"; fi
    if body=$(after --body "$@"); then printf '%s' "$body" > "$here/tickets/$2/body"; fi
    changed "$2"
    ;;
  close | reopen)
    is_there "$2"
    if [ "$1" = close ]; then printf 'closed' > "$here/tickets/$2/state"; else rm -f "$here/tickets/$2/state"; fi
    changed "$2"
    ;;
  *)
    echo "the stand-in does not know how to: $*" >&2
    exit 2
    ;;
esac
