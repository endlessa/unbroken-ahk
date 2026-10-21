#!/usr/bin/env bash
# fetch-list.sh — fetch every URL in a source list, politely and resumably.
#
#   usage: archive/tools/fetch-list.sh <source-id> [list-file] [delay-seconds]
#
# The list defaults to archive/sources/<source-id>.urls. Blank lines and lines
# starting with # are skipped. Failures are logged and do not stop the run; the
# run is resumable because fetch.sh skips bytes it already holds.
set -uo pipefail

ARCHIVE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[ $# -ge 1 ] || { echo "usage: fetch-list.sh <source-id> [list-file] [delay]" >&2; exit 2; }
source_id="$1"
list="${2:-$ARCHIVE/sources/$source_id.urls}"
delay="${3:-1}"

[ -f "$list" ] || { echo "no such list: $list" >&2; exit 2; }

ok=0; fail=0; failed_log="$ARCHIVE/sources/$source_id.failed"
: > "$failed_log"

while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in ''|\#*) continue ;; esac
  url="${line%% *}"
  name="${line#"$url"}"; name="${name# }"
  if "$ARCHIVE/tools/fetch.sh" "$source_id" "$url" ${name:+"$name"}; then
    ok=$((ok+1))
  else
    fail=$((fail+1)); printf '%s\n' "$line" >> "$failed_log"
  fi
  sleep "$delay"
done < "$list"

echo "---- $source_id: $ok fetched, $fail failed"
[ "$fail" -eq 0 ] && rm -f "$failed_log" || echo "     failures listed in sources/$source_id.failed"
