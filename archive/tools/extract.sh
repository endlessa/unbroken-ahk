#!/usr/bin/env bash
# extract.sh — render archived documents to plain text for analysis.
#
#   usage: archive/tools/extract.sh [source-id]
#
# PDFs go through pdftotext -layout, HTML through a stdlib parser, and text-like
# formats are copied. Output mirrors the raw tree under archive/text/.
# Extraction never executes archived content.
set -uo pipefail

ARCHIVE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scope="${1:-}"
root="$ARCHIVE/raw${scope:+/$scope}"
[ -d "$root" ] || { echo "nothing to extract at $root" >&2; exit 2; }

done_n=0; skip_n=0; ocr_n=0
while IFS= read -r -d '' src; do
  rel="${src#"$ARCHIVE/raw/"}"
  out="$ARCHIVE/text/${rel%.*}.txt"
  mkdir -p "$(dirname "$out")"
  [ -f "$out" ] && [ "$out" -nt "$src" ] && { skip_n=$((skip_n+1)); continue; }

  case "$(file -b --mime-type "$src")" in
    application/pdf)
      pdftotext -layout -enc UTF-8 "$src" "$out" 2>/dev/null || { skip_n=$((skip_n+1)); continue; }
      # A scan has no text layer; leaving the empty result behind would look
      # like a successful extraction. Drop it and let ocr.py pick the file up.
      if [ "$(tr -d '[:space:]\f' < "$out" | wc -c)" -lt 40 ]; then
        rm -f "$out"; ocr_n=$((ocr_n+1)); continue
      fi ;;
    text/html|application/xhtml+xml|text/xml|application/xml)
      python3 -I "$ARCHIVE/tools/html2txt.py" "$src" > "$out" || { skip_n=$((skip_n+1)); continue; } ;;
    text/*|application/json|application/csv)
      cp "$src" "$out" ;;
    *)
      skip_n=$((skip_n+1)); continue ;;
  esac
  done_n=$((done_n+1))
done < <(find "$root" -type f ! -name '.gitkeep' -print0)

echo "extracted $done_n, skipped $skip_n, needing OCR $ocr_n"
if [ "$ocr_n" -gt 0 ]; then
  echo "run: python3 -I archive/tools/ocr.py triage${scope:+ $scope}"
fi
