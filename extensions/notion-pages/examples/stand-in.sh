#!/bin/sh
# A stand-in for the program, curl asking Notion's web API, for this
# package's examples: it answers the way Notion does, changes nothing
# outside its own folder, and writes down how it was called, one call a
# line. The examples put it first on the path of a daemon of their own,
# under the program's name, so the real one is never run and nothing here
# reaches any service.
#
# What it was asked is read from its arguments as curl reads them: the
# method after `--request` (GET where none is said), what is sent after
# `--data-binary`, and the address, which is the argument that begins with
# `https://`. It answers as curl does where it is told `--fail-with-body`:
# what Notion answered on its output, and for an answer that is not well
# curl's line on its other stream, and 22.
#
# Handed a secret (`NOTION_KEY`), it writes it down as `acted/as` each time
# it is called. Notion takes no secret where it is handed none, or where
# `bad-key` is beside it.
#
# The database's pages are folders in `pages/`, one an id, each holding
# its `title`, its `body` (the property `Description`), its `card` (the
# property `Card`, which marks a page made for a card), `done` and
# `ticked` (the checkboxes `Done` and `Factory`, ticked where the file
# says `true`) and its `edited`, which says when it was last edited. What
# was said on one is a file of `comments/<id>/`, one a thing said, in
# order: who said it on its first line, and what they said after, each
# line of it a piece of the text as Notion keeps them. A case writes these
# to say what is there, and what somebody did there meanwhile.
#
# The daemon the examples run can find this under the program's name and
# nothing else, not even a shell's tools. What this needs of them it looks
# for itself, where a system keeps them.
PATH=/usr/bin:/bin
here=$(dirname "$0")
printf '%s' "$*" | tr '\n' '\036' | sed 's/\x1e/\\n/g' >> "$here/calls"
echo >> "$here/calls"
api=https://api.notion.com/v1
rm -rf "$here/acted"
if [ -n "${NOTION_KEY:-}" ]; then
  mkdir -p "$here/acted"
  printf '%s' "$NOTION_KEY" > "$here/acted/as"
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
path=${address#"$api"}
route=${path%%\?*}

refused() {
  printf '{"object":"error","status":%s,"code":"%s","message":"%s"}\n' "$1" "$2" "$3"
  printf 'curl: (22) The requested URL returned error: %s\n' "$1" >&2
  exit 22
}

if [ -z "${NOTION_KEY:-}" ] || [ -f "$here/bad-key" ]; then
  refused 401 unauthorized "API token is invalid."
fi

json() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/\t/\\t/g' -e 's/\r/\\r/g' | awk 'BEGIN { ORS = "" } NR > 1 { print "\\n" } { print }'
}

# The text of the first `"content":"` of what was sent after `$1`, as it
# reads once what JSON writes in a text is read back.
sent() {
  printf '%s' "$data" | awk -v after="$1" '
    { text = text $0 }
    END {
      at = index(text, after)
      if (at == 0) exit 1
      text = substr(text, at + length(after))
      mark = "\"content\":\""
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

of() {
  [ -f "$here/pages/$1/$2" ] && cat "$here/pages/$1/$2"
}

is_there() {
  [ -d "$here/pages/$1" ] && return 0
  refused 404 object_not_found "Could not find page with ID: $1."
}

next_change() {
  count=$(cat "$here/changes" 2>/dev/null || echo 0)
  echo $((count + 1))
}

edited() {
  count=$(next_change)
  printf '%s' "$count" > "$here/changes"
  printf 't%s' "$count" > "$here/pages/$1/edited"
}

# A text as Notion keeps it: a list of pieces, none where it is empty.
pieces() {
  if [ -z "$1" ]; then printf '[]'; else printf '[{"plain_text":"%s"}]' "$(json "$1")"; fi
}

flag() {
  if [ "$(of "$1" "$2")" = true ]; then printf true; else printf false; fi
}

page() {
  printf '{"object":"page","id":"%s","url":"https://notion.example/%s","last_edited_time":"%s","properties":{"Name":{"type":"title","title":%s},"Description":{"type":"rich_text","rich_text":%s},"Card":{"type":"rich_text","rich_text":%s},"Done":{"type":"checkbox","checkbox":%s},"Factory":{"type":"checkbox","checkbox":%s}}}' \
    "$1" "$1" "$(of "$1" edited)" "$(pieces "$(of "$1" title)")" "$(pieces "$(of "$1" body)")" "$(pieces "$(of "$1" card)")" "$(flag "$1" done)" "$(flag "$1" ticked)"
}

# Writes what a page's properties were sent as into the folder `$1`.
written() {
  case "$data" in *'"Name":{'*) sent '"Name":{' > "$1/title" ;; esac
  case "$data" in
    *'"Description":{"rich_text":[]}'*) printf '' > "$1/body" ;;
    *'"Description":{'*) sent '"Description":{' > "$1/body" ;;
  esac
  case "$data" in *'"Card":{'*) sent '"Card":{' > "$1/card" ;; esac
  case "$data" in
    *'"Done":{"checkbox":true}'*) printf 'true' > "$1/done" ;;
    *'"Done":{"checkbox":false}'*) printf 'false' > "$1/done" ;;
  esac
}

# What was said on a page, as a list in JSON: each thing said with who
# said it and its pieces, which in a file of `comments/<id>/` are what
# follows its first line, a line holding only `|` between one and the next.
comments() {
  list=""
  for said in $(ls "$here/comments/$1" 2>/dev/null | sort -n); do
    by=$(head -n 1 "$here/comments/$1/$said")
    parts=$(tail -n +2 "$here/comments/$1/$said" | awk '
      function esc(t) { gsub(/\\/, "\\\\", t); gsub(/"/, "\\\"", t); gsub(/\t/, "\\t", t); return t }
      { lines[NR] = $0 }
      END {
        piece = ""; n = 0; out = ""
        for (i = 1; i <= NR; i++) {
          if (lines[i] == "|") { out = out (n++ ? "," : "") "{\"plain_text\":\"" piece "\"}"; piece = ""; continue }
          piece = piece (piece == "" || substr(piece, length(piece) - 1) == "\\n" && 0 ? "" : "") esc(lines[i]) (i < NR && lines[i + 1] != "|" ? "\\n" : "")
        }
        out = out (n++ ? "," : "") "{\"plain_text\":\"" piece "\"}"
        printf "%s", out
      }')
    list="$list${list:+,}{\"id\":\"c-$said\",\"created_by\":{\"object\":\"user\",\"id\":\"u-$(json "$by")\"},\"display_name\":{\"type\":\"user\",\"resolved_name\":\"$(json "$by")\"},\"rich_text\":[$parts]}"
  done
  printf '[%s]' "$list"
}

case "$method $route" in
  "GET /databases/"*)
    printf '{"object":"database","id":"%s","title":[{"plain_text":"Shop"}]}\n' "${route#/databases/}"
    ;;
  "POST /databases/"*/query)
    case "$data" in
      *'"filter"'*)
        card=$(printf '%s' "$data" | sed 's/.*"equals":"\([^"]*\)".*/\1/')
        for id in $(ls "$here/pages" 2>/dev/null | sort); do
          if [ "$(of "$id" card)" = "$card" ]; then
            printf '{"results":[%s]}\n' "$(page "$id")"
            exit 0
          fi
        done
        printf '{"results":[]}\n'
        ;;
      *)
        list=""
        for id in $(ls "$here/pages" 2>/dev/null | sort); do
          list="$list${list:+,}$(page "$id")"
        done
        printf '{"results":[%s]}\n' "$list"
        ;;
    esac
    ;;
  "POST /pages")
    mkdir -p "$here/pages"
    id=aaaaaaaa-0000-0000-0000-00000000000$(($(ls "$here/pages" | wc -l) + 1))
    # Made whole or not at all, as the service makes one.
    making="$here/making"
    rm -rf "$making"
    mkdir -p "$making"
    count=$(next_change)
    written "$making"
    printf 't%s' "$count" > "$making/edited"
    mv "$making" "$here/pages/$id"
    printf '%s' "$count" > "$here/changes"
    printf '%s\n' "$(page "$id")"
    ;;
  "GET /pages/"*)
    id=${route#/pages/}
    is_there "$id"
    printf '%s\n' "$(page "$id")"
    ;;
  "PATCH /pages/"*)
    id=${route#/pages/}
    is_there "$id"
    written "$here/pages/$id"
    edited "$id"
    printf '%s\n' "$(page "$id")"
    ;;
  "GET /comments")
    id=$(printf '%s' "$path" | sed 's/.*block_id=\([^&]*\).*/\1/')
    is_there "$id"
    printf '{"results":%s}\n' "$(comments "$id")"
    ;;
  "POST /comments")
    id=$(printf '%s' "$data" | sed 's/.*"page_id":"\([^"]*\)".*/\1/')
    is_there "$id"
    mkdir -p "$here/comments/$id"
    said=$(($(ls "$here/comments/$id" | wc -l) + 1))
    { echo "Factory"; sent '"rich_text"'; } > "$here/comments/$id/$said"
    printf '{"object":"comment","id":"c-%s"}\n' "$said"
    ;;
  *)
    echo "the stand-in does not know how to: $method $address" >&2
    exit 2
    ;;
esac
