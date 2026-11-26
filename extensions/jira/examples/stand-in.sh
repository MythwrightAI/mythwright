#!/bin/sh
# A stand-in for the program, curl asking Jira's web API, for this
# package's examples: it answers the way Jira does, changes nothing outside
# its own folder, and writes down how it was called, one call a line. The
# examples put it first on the path of a daemon of their own, under the
# program's name, so the real one is never run and nothing here reaches
# any service.
#
# What it was asked is read from its arguments as curl reads them: the
# method after `--request` (GET where none is said), what is sent after
# `--data-binary`, and the address, which is the argument that begins with
# `https://`. It answers as curl does where it is told `--fail-with-body`:
# what Jira answered on its output, and for an answer that is not well
# curl's line on its other stream, and 22.
#
# Handed a key (`JIRA_KEY`), it writes the key down as `acted/as` each
# time it is called. Jira takes no key where it is handed none, or where
# `bad-key` is beside it.
#
# The issues it has are folders in `issues/`, one a key (`SHOP-1`), each
# holding its `summary`, its `description`, its `status` (`new`,
# `indeterminate` or `done`, as Jira's categories of status are called;
# `new` where nothing says), its `labels` (one a line) and its `updated`,
# which says when it last changed. Who reported it is its `reporter`
# (`Ada` where nothing says), and `customer` beside it makes them a
# customer of a service desk. What was said on one is a file of
# `comments/<key>/`, one a thing said, in order: who said it on its first
# line, with `customer` after a space where they are one, and what they
# said after. A case writes these to say what is there, and what somebody
# did there meanwhile.
#
# The daemon the examples run can find this under the program's name and
# nothing else, not even a shell's tools. What this needs of them it looks
# for itself, where a system keeps them.
PATH=/usr/bin:/bin
here=$(dirname "$0")
# One call a line, whatever is in it: the end of a line within an argument
# is written as `\n`.
printf '%s' "$*" | tr '\n' '\036' | sed 's/\x1e/\\n/g' >> "$here/calls"
echo >> "$here/calls"
site=https://acme.example
rm -rf "$here/acted"
if [ -n "${JIRA_KEY:-}" ]; then
  mkdir -p "$here/acted"
  printf '%s' "$JIRA_KEY" > "$here/acted/as"
fi

method=GET
data=""
address=""
while [ $# -gt 0 ]; do
  case "$1" in
    --request) method=$2; shift 2 ;;
    --data-binary) data=$2; shift 2 ;;
    --variable | --expand-header | --header) shift 2 ;;
    https://*) address=$1; shift ;;
    *) shift ;;
  esac
done
path=${address#"$site"}
route=${path%%\?*}

# Answers as curl does for what Jira did not answer well.
refused() {
  printf '{"errorMessages":["%s"]}\n' "$2"
  printf 'curl: (22) The requested URL returned error: %s\n' "$1" >&2
  exit 22
}

if [ -z "${JIRA_KEY:-}" ] || [ -f "$here/bad-key" ]; then
  refused 401 "Client must be authenticated to access this resource."
fi

# A text as it is written inside JSON.
json() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/\t/\\t/g' -e 's/\r/\\r/g' | awk 'BEGIN { ORS = "" } NR > 1 { print "\\n" } { print }'
}

# The text a key of what was sent holds, as it is written inside JSON
# read back: the first `"<key>":"` and what follows to its closing mark.
sent() {
  printf '%s' "$data" | awk -v key="$1" '
    { text = text $0 }
    END {
      mark = "\"" key "\":\""
      at = index(text, mark)
      if (at == 0) exit 1
      text = substr(text, at + length(mark))
      out = ""
      while (length(text) > 0) {
        c = substr(text, 1, 1)
        if (c == "\\") {
          d = substr(text, 2, 1)
          if (d == "n") out = out "\n"; else if (d == "t") out = out "\t"; else if (d == "r") out = out "\r"; else out = out d
          text = substr(text, 3)
          continue
        }
        if (c == "\"") break
        out = out c
        text = substr(text, 2)
      }
      printf "%s", out
    }'
}

# What a file of an issue holds, or nothing.
of() {
  [ -f "$here/issues/$1/$2" ] && cat "$here/issues/$1/$2"
}

# Whether there is such an issue; answered as Jira answers where not.
is_there() {
  [ -d "$here/issues/$1" ] && return 0
  refused 404 "Issue does not exist or you do not have permission to see it."
}

# The stamp of the next change: one more than any said.
next_change() {
  count=$(cat "$here/changes" 2>/dev/null || echo 0)
  for stamp in "$here"/issues/*/updated; do
    [ -f "$stamp" ] || continue
    said=$(sed 's/^t//' "$stamp")
    case $said in '' | *[!0-9]*) continue ;; esac
    [ "$said" -gt "$count" ] && count=$said
  done
  echo $((count + 1))
}

# Says that an issue changed.
changed() {
  count=$(next_change)
  printf '%s' "$count" > "$here/changes"
  printf 't%s' "$count" > "$here/issues/$1/updated"
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

# Somebody as Jira says who they are.
person() {
  kind=atlassian
  [ "$2" = customer ] && kind=customer
  printf '{"accountId":"id-%s","displayName":"%s","accountType":"%s"}' "$(json "$1")" "$(json "$1")" "$kind"
}

# What was said on an issue, as a list in JSON.
comments() {
  list=""
  for said in $(ls "$here/comments/$1" 2>/dev/null | sort -n); do
    first=$(head -n 1 "$here/comments/$1/$said")
    by=${first%% *}
    kind=""
    case "$first" in *" customer") kind=customer ;; esac
    words=$(tail -n +2 "$here/comments/$1/$said")
    list="$list${list:+,}{\"id\":\"100$said\",\"author\":$(person "$by" "$kind"),\"body\":\"$(json "$words")\"}"
  done
  printf '[%s]' "$list"
}

# The status of a category, as Jira says it.
status() {
  case "$1" in
    done) printf '{"name":"Done","statusCategory":{"key":"done"}}' ;;
    indeterminate) printf '{"name":"In Progress","statusCategory":{"key":"indeterminate"}}' ;;
    *) printf '{"name":"To Do","statusCategory":{"key":"new"}}' ;;
  esac
}

case "$method $route" in
  "GET /rest/api/2/myself")
    printf '{"accountId":"id-ada","displayName":"Ada","accountType":"atlassian"}\n'
    ;;
  "GET /rest/api/2/search/jql")
    case "$path" in
      *labels%3D%22card-*)
        # The issue that carries the label of a card, or none.
        card=$(printf '%s' "$path" | sed 's/.*labels%3D%22card-\([^%]*\)%22.*/\1/')
        for key in $(ls "$here/issues" 2>/dev/null | sort -t- -k2 -n); do
          if grep -qx -- "card-$card" "$here/issues/$key/labels" 2>/dev/null; then
            printf '{"issues":[{"key":"%s","fields":{"summary":"%s"}}]}\n' "$key" "$(json "$(of "$key" summary)")"
            exit 0
          fi
        done
        printf '{"issues":[]}\n'
        ;;
      *)
        list=""
        for key in $(ls "$here/issues" 2>/dev/null | sort -t- -k2 -n); do
          list="$list${list:+,}{\"key\":\"$key\",\"fields\":{\"updated\":\"$(of "$key" updated)\",\"labels\":$(labels "$key")}}"
        done
        printf '{"issues":[%s]}\n' "$list"
        ;;
    esac
    ;;
  "POST /rest/api/2/issue")
    mkdir -p "$here/issues"
    key=SHOP-$(($(ls "$here/issues" | wc -l) + 1))
    count=$(next_change)
    making="$here/making"
    rm -rf "$making"
    mkdir -p "$making"
    sent summary > "$making/summary"
    sent description > "$making/description"
    printf '%s\n' "$data" | sed 's/.*"labels":\["\([^"]*\)"\].*/\1/' > "$making/labels"
    printf 't%s' "$count" > "$making/updated"
    mv "$making" "$here/issues/$key"
    printf '%s' "$count" > "$here/changes"
    printf '{"id":"1000%s","key":"%s","self":"%s/rest/api/2/issue/%s"}\n' "${key#SHOP-}" "$key" "$site" "$key"
    ;;
  "GET /rest/api/2/issue/"*/transitions)
    key=${route#/rest/api/2/issue/}
    key=${key%/transitions}
    is_there "$key"
    printf '{"transitions":[{"id":"11","name":"To Do","to":%s},{"id":"21","name":"In Progress","to":%s},{"id":"31","name":"Done","to":%s}]}\n' "$(status new)" "$(status indeterminate)" "$(status done)"
    ;;
  "POST /rest/api/2/issue/"*/transitions)
    key=${route#/rest/api/2/issue/}
    key=${key%/transitions}
    is_there "$key"
    case "$data" in
      *'"id":"31"'*) printf 'done' > "$here/issues/$key/status" ;;
      *'"id":"21"'*) printf 'indeterminate' > "$here/issues/$key/status" ;;
      *) printf 'new' > "$here/issues/$key/status" ;;
    esac
    changed "$key"
    ;;
  "GET /rest/api/2/issue/"*/comment)
    key=${route#/rest/api/2/issue/}
    key=${key%/comment}
    is_there "$key"
    printf '{"comments":%s}\n' "$(comments "$key")"
    ;;
  "POST /rest/api/2/issue/"*/comment)
    key=${route#/rest/api/2/issue/}
    key=${key%/comment}
    is_there "$key"
    mkdir -p "$here/comments/$key"
    said=$(($(ls "$here/comments/$key" | wc -l) + 1))
    { echo "Ada"; sent body; } > "$here/comments/$key/$said"
    changed "$key"
    printf '{"id":"100%s","body":"%s"}\n' "$said" "$(json "$(sent body)")"
    ;;
  "GET /rest/api/2/issue/"*)
    key=${route#/rest/api/2/issue/}
    is_there "$key"
    reporter=$(of "$key" reporter)
    reporter=${reporter:-Ada}
    kind=""
    [ -f "$here/issues/$key/customer" ] && kind=customer
    printf '{"key":"%s","fields":{"summary":"%s","description":"%s","status":%s,"labels":%s,"updated":"%s","reporter":%s,"comment":{"comments":%s}}}\n' \
      "$key" "$(json "$(of "$key" summary)")" "$(json "$(of "$key" description)")" "$(status "$(of "$key" status)")" \
      "$(labels "$key")" "$(of "$key" updated)" "$(person "$reporter" "$kind")" "$(comments "$key")"
    ;;
  "PUT /rest/api/2/issue/"*)
    key=${route#/rest/api/2/issue/}
    is_there "$key"
    if summary=$(sent summary); then printf '%s' "$summary" > "$here/issues/$key/summary"; fi
    if description=$(sent description); then printf '%s' "$description" > "$here/issues/$key/description"; fi
    changed "$key"
    ;;
  *)
    echo "the stand-in does not know how to: $method $address" >&2
    exit 2
    ;;
esac
