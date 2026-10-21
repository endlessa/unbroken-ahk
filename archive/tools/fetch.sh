#!/usr/bin/env bash
# fetch.sh — retrieve one URL into the offline archive and record provenance.
#
#   usage: archive/tools/fetch.sh <source-id> <url> [filename]
#
# Writes the body to archive/raw/<source-id>/<filename> and appends one JSON
# line to archive/manifest.jsonl describing what was fetched, when, and with
# what hash. Re-fetching a URL whose bytes are unchanged is a no-op.
set -euo pipefail

ARCHIVE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UA="unbroken-archive/1.0 (offline research corpus; curl)"

[ $# -ge 2 ] || { echo "usage: fetch.sh <source-id> <url> [filename]" >&2; exit 2; }
source_id="$1"; url="$2"

derive_name() {
  local n
  n="$(printf '%s' "$1" | sed -e 's/[?#].*$//' -e 's#/$##' -e 's#.*/##')"
  [ -n "$n" ] || n="index.html"
  printf '%s' "$n" | tr -c 'A-Za-z0-9._-' '_' | cut -c1-120
}
name="${3:-$(derive_name "$url")}"

dest_dir="$ARCHIVE/raw/$source_id"
mkdir -p "$dest_dir"
dest="$dest_dir/$name"
tmp="$(mktemp "${TMPDIR:-/tmp}/fetch.XXXXXX")"
hdr="$(mktemp "${TMPDIR:-/tmp}/fetchhdr.XXXXXX")"
trap 'rm -f "$tmp" "$hdr"' EXIT

# --retry-all-errors covers the transient proxy 5xx/resets seen in this container.
http=$(curl -sS -L \
  --retry 5 --retry-delay 2 --retry-all-errors \
  --connect-timeout 30 --max-time 1800 \
  -A "$UA" \
  -D "$hdr" \
  -o "$tmp" \
  -w '%{http_code}' \
  "$url") || { echo "FAIL  $url (curl exit $?)" >&2; exit 1; }

if [ "$http" -ge 400 ]; then
  echo "FAIL  $http  $url" >&2
  exit 1
fi

sha="$(sha256sum "$tmp" | cut -d' ' -f1)"
bytes="$(wc -c < "$tmp" | tr -d ' ')"
ctype="$(grep -i '^content-type:' "$hdr" | tail -1 | cut -d: -f2- | tr -d '\r' | sed 's/^ *//')"
final="$(grep -i '^location:' "$hdr" | tail -1 | cut -d: -f2- | tr -d '\r' | sed 's/^ *//')"
[ -n "$final" ] || final="$url"
sniffed="$(file -b --mime-type "$tmp")"

if [ -f "$dest" ] && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$sha" ]; then
  echo "SAME  $sha  $name"
  exit 0
fi

mv "$tmp" "$dest"
trap 'rm -f "$hdr"' EXIT

python3 -I -c '
import json, sys
rec = dict(zip(
  ["fetched_utc","source_id","path","sha256","bytes","content_type","sniffed_type","http_status","final_url","original_url"],
  sys.argv[1:]))
rec["bytes"] = int(rec["bytes"])
rec["http_status"] = int(rec["http_status"])
print(json.dumps(rec, sort_keys=True))
' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$source_id" "raw/$source_id/$name" "$sha" "$bytes" \
  "$ctype" "$sniffed" "$http" "$final" "$url" >> "$ARCHIVE/manifest.jsonl"

echo "GOT   $sha  $bytes  $name"
